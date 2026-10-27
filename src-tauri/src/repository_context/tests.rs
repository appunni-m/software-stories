use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH};

pub(super) fn add_source(fixture: &mut Fixture, path: &str, text: &str) {
    let sha256 = digest(text.as_bytes());
    let blob_path = format!("objects/sha256/{}/{}", &sha256[..2], &sha256[2..]);
    let absolute = fixture.storage.join(&blob_path);
    fs::create_dir_all(absolute.parent().unwrap()).unwrap();
    fs::write(absolute, text).unwrap();
    fixture.manifest.files.retain(|f| f.path != path);
    fixture.manifest.files.push(snapshot::CapturedFile {
        path: path.to_owned(),
        kind: snapshot::InventoryPathKind::File,
        size_bytes: text.len() as u64,
        sha256,
        blob_path,
        content_kind: snapshot::CapturedContentKind::Text,
        secret_like: false,
    });
}

#[test]
fn covers_unicode_crlf_huge_lines_and_entire_large_declarations() {
    let fixture = Fixture::new();
    let mut symbol = fixture
        .index
        .symbols
        .iter()
        .find(|s| s.path == SIMPLE_PATH)
        .unwrap()
        .clone();
    let text = format!(
        "fn renderGlyphBitmap() {{\r\n{}\r\n{}\r\n}}\r\n",
        "    // naïve 字形 🦀\r\n".repeat(5000),
        "字".repeat(6000)
    );
    symbol.anchor.start_byte = 0;
    symbol.anchor.end_byte = text.len();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let ranges = chunks(&text, &[&symbol], &budget).unwrap();
    assert!(ranges.len() > 10);
    assert!(valid_chunks(&ranges, &text));
    assert!(
        ranges
            .iter()
            .all(|r| r.enclosing.as_ref().unwrap().symbol_id == symbol.symbol_id)
    );
    let reconstructed = ranges
        .iter()
        .map(|r| &text[r.start_byte..r.end_byte])
        .collect::<String>();
    assert_eq!(reconstructed, text);
    let mut bad = ranges.clone();
    bad[1].start_byte += 1;
    assert!(!valid_chunks(&bad, &text));
    let mut bad = ranges;
    bad[0].start_line = 999;
    assert!(!valid_chunks(&bad, &text));
}

#[test]
fn identifiers_keep_original_snake_camel_acronym_and_unicode_parts() {
    let words = terms("Font::memory_face renderGlyphBitmap HTTPServer naïve 字形");
    for expected in [
        "font",
        "memory_face",
        "memory",
        "face",
        "renderglyphbitmap",
        "render",
        "glyph",
        "bitmap",
        "http",
        "server",
        "naïve",
        "字形",
    ] {
        assert!(
            words.contains(&expected.to_owned()),
            "missing {expected}: {words:?}"
        );
    }
}

#[test]
fn compound_identifier_search_does_not_return_common_component_only_hits() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "lib/code.rs",
        "const CODE_OF_CONDUCT: &str = \"\";\n",
    );
    add_source(&mut fixture, "lib/atlas.rs", "struct Atlas;\n");
    add_source(
        &mut fixture,
        "lib/split.rs",
        "fn code() {}\nstruct AtlasAcceptanceNoMatch_20261020 {}\n",
    );
    add_source(&mut fixture, "lib/memory.rs", "fn memory_face() {}\n");
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();

    let missing = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "CodeAtlasAcceptanceNoMatch_20261020",
        Some("lib"),
        8,
        &budget,
    )
    .unwrap();
    assert!(
        missing.matches.is_empty(),
        "an identifier query assembled from separate identifiers returned: {:?}",
        missing
            .matches
            .iter()
            .map(|hit| (&hit.range.path, hit.text.as_str()))
            .collect::<Vec<_>>()
    );

    let component = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "MemoryFace",
        Some("lib/memory.rs"),
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(component.matches.len(), 1);
    assert_eq!(component.matches[0].range.path, "lib/memory.rs");
}

#[test]
fn natural_language_search_matches_common_english_inflections() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        SIMPLE_PATH,
        "pub fn render_glyph_bitmap() { rasterize_glyph(); }\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "rendered glyphs rasterization",
        None,
        8,
        &budget,
    )
    .unwrap();
    assert!(
        result
            .matches
            .iter()
            .any(|hit| hit.text.contains("render_glyph_bitmap")),
        "inflected query should find the source declaration: {:?}",
        result
            .matches
            .iter()
            .map(|hit| &hit.text)
            .collect::<Vec<_>>()
    );
}

#[test]
fn broad_natural_search_does_not_match_only_a_filename_but_explicit_paths_do() {
    let mut fixture = Fixture::new();
    let filename_only = "tests/fixtures/public-api/font-bytes-rendered-glyph-bitmap-entry-parsing-outline-hinting-rasterization.json";
    add_source(&mut fixture, filename_only, "unrelated fixture payload\n");
    add_source(
        &mut fixture,
        "src/render.rs",
        "fn render_font_bytes_to_bitmap() { parse_glyph_outline(); apply_hinting(); rasterize(); }\n",
    );
    add_source(
        &mut fixture,
        "src/renderer/path.rs",
        "unrelated module payload\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();

    let broad = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "font bytes rendered glyph bitmap entry parsing outline hinting rasterization",
        None,
        8,
        &budget,
    )
    .unwrap();
    assert!(
        broad
            .matches
            .iter()
            .any(|hit| hit.range.path == "src/render.rs"),
        "the implementation body should match the broad query"
    );
    assert!(
        broad
            .matches
            .iter()
            .all(|hit| hit.range.path != filename_only),
        "a filename-only match should not be presented as a source match"
    );

    let explicit_path = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "src/renderer/path.rs",
        None,
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(explicit_path.matches[0].range.path, "src/renderer/path.rs");
}

#[test]
fn natural_range_search_admits_and_reports_bounded_per_term_candidates() {
    let mut candidates = HashMap::new();
    let mut coverage = HashMap::new();
    merge_query_term_candidate(&mut candidates, &mut coverage, 17, 0.25, 1 << 0);
    merge_query_term_candidate(&mut candidates, &mut coverage, 23, 0.5, 1 << 1);
    merge_query_term_candidate(&mut candidates, &mut coverage, 17, 0.75, 1 << 2);

    assert_eq!(candidates.len(), 2, "term-only hits join the candidate set");
    assert_eq!(candidates[&17], (false, false, 0.75));
    assert_eq!(coverage[&17].count_ones(), 2);
    assert_eq!(coverage[&23].count_ones(), 1);
}

#[test]
fn natural_range_search_exposes_verified_query_coverage() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "src/render.rs",
        "fn render_font_bytes_to_bitmap() { parse_glyph_outline(); apply_hinting(); rasterize(); }\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "font bytes rendered glyph bitmap entry parsing outline hinting rasterization",
        None,
        8,
        &budget,
    )
    .unwrap();
    let hit = result
        .matches
        .iter()
        .find(|hit| hit.range.path == "src/render.rs")
        .expect("the implementation should be returned from its source body");
    let coverage = hit
        .query_coverage
        .as_ref()
        .expect("natural-language range search should disclose lexical coverage");
    assert_eq!(coverage.total_terms, 10);
    assert!(coverage.matched_terms.contains(&"font".to_owned()));
    assert!(coverage.matched_terms.contains(&"bytes".to_owned()));
    assert!(coverage.matched_terms.contains(&"bitmap".to_owned()));
}

#[test]
fn broad_question_coverage_ignores_prompt_scaffolding() {
    let query = "How does the font get from bytes into bitmap";
    let words = terms(query);
    assert_eq!(
        coverage_terms(query, None, &words),
        ["bitmap", "bytes", "font"]
    );
    assert!(coverage_terms("How does it work", None, &terms("How does it work")).is_empty());

    let code_query = "Font::memory_face";
    assert_eq!(
        coverage_terms(code_query, None, &terms(code_query)),
        terms(code_query),
        "identifier-focused searches retain every exact term"
    );
}

#[test]
fn broad_range_search_reports_content_terms_not_question_scaffolding() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "src/render.rs",
        "fn render_font_bytes_to_bitmap() { parse_glyph_outline(); rasterize(); }\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "How does the font get from bytes into bitmap",
        None,
        8,
        &budget,
    )
    .unwrap();
    let hit = result
        .matches
        .iter()
        .find(|hit| hit.range.path == "src/render.rs")
        .expect("the code range should remain searchable with the full question");
    let coverage = hit.query_coverage.as_ref().unwrap();
    assert_eq!(coverage.total_terms, 3);
    assert_eq!(coverage.matched_terms, ["bitmap", "bytes", "font"]);
}

#[test]
fn file_discovery_returns_one_verified_range_per_path_and_pages_stably() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "src/a.rs",
        "fn render_glyph() { cover_pixel(); }\n",
    );
    add_source(&mut fixture, "src/b.rs", "fn render_step() {}\n");
    add_source(
        &mut fixture,
        "src/sub/c.rs",
        "fn glyph_step() { cover_pixel(); }\n",
    );
    add_source(&mut fixture, "tests/unrelated.rs", "fn parse_header() {}\n");

    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let search = |scope: Option<&str>, offset| {
        search_files_with_mode_page(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "rendered glyphs covered",
            scope,
            1,
            offset,
            &budget,
            SearchMode::Code,
        )
        .unwrap()
    };

    let first = search(None, 0);
    assert_eq!(first.matches.len(), 1);
    assert_eq!(first.matches[0].range.path, "src/a.rs");
    let coverage = first.matches[0]
        .query_coverage
        .as_ref()
        .expect("file discovery should expose the terms that ranked this path");
    assert_eq!(coverage.total_terms, 3);
    assert_eq!(coverage.matched_terms, ["covered", "glyphs", "rendered"]);
    let serialized = serde_json::to_value(&first.matches[0]).unwrap();
    assert_eq!(
        serialized["queryCoverage"]["matchedTerms"],
        serde_json::json!(["covered", "glyphs", "rendered"])
    );
    assert_eq!(serialized["queryCoverage"]["totalTerms"], 3);
    assert!(first.next_offset.is_some());
    assert_eq!(
        first.matches[0].range.source_sha256,
        digest(b"fn render_glyph() { cover_pixel(); }\n")
    );

    let second = search(None, first.next_offset.unwrap());
    assert_eq!(second.matches.len(), 1);
    assert_eq!(second.matches[0].range.path, "src/sub/c.rs");
    let coverage = second.matches[0]
        .query_coverage
        .as_ref()
        .expect("each discovered path should explain its lexical coverage");
    assert_eq!(coverage.total_terms, 3);
    assert_eq!(coverage.matched_terms, ["covered", "glyphs"]);
    assert_ne!(first.matches[0].range.path, second.matches[0].range.path);

    let scoped = search(Some("src/sub"), 0);
    assert_eq!(scoped.matches.len(), 1);
    assert_eq!(scoped.matches[0].range.path, "src/sub/c.rs");
    assert!(scoped.next_offset.is_none());
}

#[test]
fn broad_file_discovery_does_not_fall_back_to_question_scaffolding() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "src/render.rs", "fn render_font_bytes() {}\n");

    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let result = search_files_with_mode_page(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "How does it work",
        None,
        8,
        0,
        &budget,
        SearchMode::Code,
    )
    .unwrap();

    assert!(result.matches.is_empty());
    assert!(result.next_offset.is_none());
}

#[test]
fn broad_file_discovery_prefers_distinct_query_coverage_over_one_rare_term() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "src/00-coverage.txt",
        "alpha beta gamma delta\n",
    );
    add_source(&mut fixture, "src/99-rare.txt", "quasar\n");
    for index in 0..8 {
        add_source(
            &mut fixture,
            &format!("src/common-{index:02}.txt"),
            "alpha beta gamma delta\n",
        );
    }

    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let result = search_files_with_mode_page(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "alpha beta gamma delta quasar",
        None,
        1,
        0,
        &budget,
        SearchMode::Code,
    )
    .unwrap();

    assert_eq!(result.matches.len(), 1);
    assert_eq!(
        result.matches[0].range.path, "src/00-coverage.txt",
        "a broad discovery query should lead with the file covering four distinct terms, not the file matching only the rarest term"
    );
    let coverage = result.matches[0]
        .query_coverage
        .as_ref()
        .expect("file discovery should expose the same coverage signal it ranks by");
    assert_eq!(coverage.matched_terms, ["alpha", "beta", "delta", "gamma"]);
    assert_eq!(coverage.total_terms, 5);
}

#[test]
fn broad_range_search_prefers_distinct_query_coverage_over_one_rare_term() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "src/00-coverage.txt",
        "alpha beta gamma delta\n",
    );
    add_source(&mut fixture, "src/99-rare.txt", "quasar\n");
    for index in 0..8 {
        add_source(
            &mut fixture,
            &format!("src/common-{index:02}.txt"),
            "alpha beta gamma delta\n",
        );
    }

    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "alpha beta gamma delta quasar",
        None,
        1,
        &budget,
    )
    .unwrap();

    assert_eq!(result.matches.len(), 1);
    assert_eq!(
        result.matches[0].range.path, "src/00-coverage.txt",
        "a broad code query should lead with a range covering four distinct terms, not a range matching only its rarest term"
    );
}

#[cfg(unix)]
#[test]
fn file_discovery_observes_cancellation_after_verifying_a_captured_blob() {
    use std::{
        ffi::CString,
        os::unix::ffi::OsStrExt,
        sync::{Arc, mpsc},
        thread,
    };

    let mut fixture = Fixture::new();
    let text = "fn render_glyph() {}\n";
    add_source(&mut fixture, "src/render.rs", text);
    let cancelled = Arc::new(AtomicBool::new(false));
    let initial_budget = Budget::new(cancelled.as_ref());
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &initial_budget).unwrap();
    let captured = fixture
        .manifest
        .files
        .iter()
        .find(|file| file.path == "src/render.rs")
        .unwrap();
    let blob_path = fixture.storage.join(&captured.blob_path);
    fs::remove_file(&blob_path).unwrap();
    let c_path = CString::new(blob_path.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);

    let result = thread::scope(|scope| {
        let (read_tx, read_rx) = mpsc::channel();
        let (continue_tx, continue_rx) = mpsc::channel();
        let contents = text.as_bytes().to_vec();
        let writer_path = blob_path.clone();
        let writer = scope.spawn(move || {
            use std::io::Write;

            let mut fifo = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(writer_path)
                .unwrap();
            fifo.write_all(&contents[..1]).unwrap();
            read_tx.send(()).unwrap();
            continue_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("search test should release the captured-source writer");
            fifo.write_all(&contents[1..]).unwrap();
        });

        let storage = fixture.storage.as_path();
        let manifest = &fixture.manifest;
        let prepared = &prepared;
        let search_cancelled = Arc::clone(&cancelled);
        let search = scope.spawn(move || {
            let budget = Budget::new(search_cancelled.as_ref());
            search_files_with_mode_page(
                storage,
                manifest,
                prepared,
                "render",
                None,
                8,
                0,
                &budget,
                SearchMode::Code,
            )
        });

        read_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("search should open the captured-source FIFO");
        cancelled.store(true, Ordering::Release);
        continue_tx.send(()).unwrap();
        let result = search.join().unwrap();
        writer.join().unwrap();
        result
    });

    match result {
        Err(error) => assert_eq!(error, "Code search stopped."),
        Ok(_) => panic!("cancelled file discovery must not publish a result"),
    }
}

#[test]
fn large_parserless_c_adapter_is_searchable_without_a_whole_file_prompt() {
    let mut fixture = Fixture::new();
    let text = format!(
        "{}void render_glyph_bitmap(void) {{}}\n",
        "/* filler with other code */\n".repeat(42_000)
    );
    assert!(text.len() > 1_032_893);
    assert!(text.len() > index::MAX_SOURCE_BYTES_PER_FILE as usize);
    add_source(&mut fixture, "src/adapter.c", &text);
    let repeated_calls = format!(
        "{}pub extern \"C\" fn FT_Load_Glyph(face: Face, glyph: u32) {{}}\n",
        "FT_Load_Glyph(face, glyph, flags);\n".repeat(42_000)
    );
    assert!(repeated_calls.len() > 1_032_893);
    assert!(repeated_calls.len() > index::MAX_SOURCE_BYTES_PER_FILE as usize);
    add_source(&mut fixture, "src/ffi-late.rs", &repeated_calls);
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "render_glyph_bitmap",
        None,
        8,
        &budget,
    )
    .unwrap();
    let hit = &result.matches[0];
    assert_eq!(hit.range.path, "src/adapter.c");
    assert!(hit.exact_identifier);
    assert!(hit.range.parser.is_none());
    assert!(hit.range.end_byte - hit.range.start_byte <= CHUNK_BYTES);
    let read = read_range(
        &fixture.storage,
        &fixture.manifest,
        &hit.range.path,
        &hit.range.source_sha256,
        hit.range.start_byte,
        hit.range.end_byte,
    )
    .unwrap();
    assert!(read.contents.contains("void render_glyph_bitmap"));
    assert_eq!(read.range.chunk_sha256, hit.range.chunk_sha256);
    assert_eq!(hit.line, 42_001);
    let literal = search_pattern(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "FT_Load_Glyph",
        None,
        8,
        &budget,
        Instant::now(),
        SearchMode::Code,
    )
    .unwrap();
    assert!(
        literal.matches.iter().any(|hit| hit.declaration_match),
        "literal fallback results: {:?}",
        literal
            .matches
            .iter()
            .map(|hit| (hit.line, hit.declaration_match, hit.text.as_str()))
            .collect::<Vec<_>>()
    );
    let named = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "FT_Load_Glyph",
        None,
        8,
        &budget,
    )
    .unwrap();
    let declaration = named
        .matches
        .iter()
        .find(|hit| hit.declaration_match)
        .unwrap_or_else(|| {
            panic!(
                "exact-name search should find the late parserless declaration: {:?}",
                named
                    .matches
                    .iter()
                    .map(|hit| (hit.line, hit.declaration_match, hit.text.as_str()))
                    .collect::<Vec<_>>()
            )
        });
    assert_eq!(declaration.range.path, "src/ffi-late.rs");
    assert_eq!(declaration.line, 42_001);
    assert!(declaration.range.parser.is_none());
    let component = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "glyph bitmap",
        Some("src"),
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(component.matches[0].range.path, "src/adapter.c");
    assert!(component.coverage.text_fallback_files > 0);
}

#[test]
fn repository_wide_candidates_round_robin_paths_without_reordering_each_path() {
    let paths = [
        "src/a.rs", "src/a.rs", "src/a.rs", "src/b.rs", "src/b.rs", "src/c.rs",
    ];
    let candidates = vec![
        (0, 0.99),
        (1, 0.95),
        (2, 0.90),
        (3, 0.85),
        (4, 0.80),
        (5, 0.75),
    ];
    let diversified = diversify_by_path_limited(candidates, &|id| paths[id].to_owned(), usize::MAX);
    assert_eq!(
        diversified
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        [0, 3, 5, 1, 4, 2]
    );
}

#[test]
fn global_path_limit_keeps_more_ranges_from_the_strongest_paths() {
    let paths = [
        "src/a.rs", "src/a.rs", "src/a.rs", "src/b.rs", "src/b.rs", "src/c.rs",
    ];
    let candidates = vec![
        (0, 0.99),
        (1, 0.95),
        (2, 0.90),
        (3, 0.85),
        (4, 0.80),
        (5, 0.75),
    ];
    let diversified = diversify_by_path_limited(candidates, &|id| paths[id].to_owned(), 2);
    assert_eq!(
        diversified
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>(),
        [0, 3, 1, 4, 2]
    );
}

#[test]
fn repository_wide_lexical_results_diversify_paths_before_the_result_limit() {
    let mut fixture = Fixture::new();
    let filler = "padding ".repeat(CHUNK_BYTES);
    add_source(
        &mut fixture,
        "lib/a-many-ranges.txt",
        &format!("render glyph padding\n{filler}\nrender glyph padding\n"),
    );
    let noise = (0..500)
        .map(|number| format!("noise{number}"))
        .collect::<Vec<_>>()
        .join(" ");
    add_source(
        &mut fixture,
        "lib/b-one-range.txt",
        &format!("render glyph padding {noise}\n"),
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();

    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "render glyph",
        None,
        2,
        &budget,
    )
    .unwrap();

    assert_eq!(result.matches.len(), 2);
    assert_eq!(result.matches[0].range.path, "lib/a-many-ranges.txt");
    assert_eq!(result.matches[1].range.path, "lib/b-one-range.txt");
    assert!(result.limited);
}

#[test]
fn repository_wide_search_oversamples_before_path_diversification() {
    let mut fixture = Fixture::new();
    let prefix = "sprocket raster ";
    let filler_bytes = CHUNK_BYTES - prefix.len() - 1;
    let filler = format!(
        "{}{}",
        "padding ".repeat(filler_bytes / "padding ".len()),
        "x".repeat(filler_bytes % "padding ".len())
    );
    let chunk = format!("{prefix}{filler}\n");
    assert_eq!(chunk.len(), CHUNK_BYTES);
    add_source(
        &mut fixture,
        "lib/a-many-ranges.txt",
        &chunk.repeat(MAX_CANDIDATES),
    );
    add_source(&mut fixture, "lib/b-one-range.txt", &chunk);
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let large_file = prepared
        .metadata
        .files
        .iter()
        .find(|file| file.path == "lib/a-many-ranges.txt")
        .unwrap();
    assert_eq!(
        large_file.chunks,
        MAX_CANDIDATES,
        "coverage: {}",
        serde_json::to_string(large_file).unwrap()
    );
    assert_eq!(
        prepared
            .metadata
            .ranges
            .iter()
            .filter(|range| range.path == "lib/a-many-ranges.txt")
            .count(),
        MAX_CANDIDATES
    );
    assert_eq!(
        prepared
            .metadata
            .ranges
            .iter()
            .filter(|range| range.path == "lib/b-one-range.txt")
            .count(),
        1
    );

    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "sprocket raster",
        None,
        2,
        &budget,
    )
    .unwrap();

    assert_eq!(
        result.matches.len(),
        2,
        "hits: {:?}",
        result
            .matches
            .iter()
            .map(|hit| (&hit.range.path, hit.range.start_byte, hit.range.end_byte))
            .collect::<Vec<_>>()
    );
    assert_eq!(result.matches[0].range.path, "lib/a-many-ranges.txt");
    assert_eq!(result.matches[1].range.path, "lib/b-one-range.txt");
    assert!(result.limited);
}

#[test]
fn fallback_identifier_declaration_recognizes_function_headers_not_calls() {
    let declaration = "pub extern \"C\" fn FT_Load_Glyph(face: Face) {}";
    let start = declaration.find("FT_Load_Glyph").unwrap();
    assert!(function_declaration_at(
        declaration,
        start,
        start + "FT_Load_Glyph".len()
    ));
    let call = "FT_Load_Glyph(face, glyph, flags);";
    let start = call.find("FT_Load_Glyph").unwrap();
    assert!(!function_declaration_at(
        call,
        start,
        start + "FT_Load_Glyph".len()
    ));
    let prose = "Use FT_Load_Glyph(face, glyph, flags) for this operation.";
    let start = prose.find("FT_Load_Glyph").unwrap();
    assert!(!function_declaration_at(
        prose,
        start,
        start + "FT_Load_Glyph".len()
    ));
    assert!(is_code_path("fontdone-c-abi/src/implementation.rs"));
    assert!(!is_code_path("docs/guide.md"));
}

#[test]
fn generation_reuse_corruption_rebuild_and_changed_snapshot_do_not_mix_results() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "lib/old.py",
        "def uniquelyoldrenderer():\n    return 1\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let first = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    assert!(!first.reused_generation);
    let again = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    assert!(again.reused_generation);
    let root = root(
        &fixture.storage,
        &fixture.manifest.summary.project_id,
        &fixture.manifest.summary.snapshot_id,
    );
    let pointer: Pointer =
        serde_json::from_slice(&fs::read(root.join("current.json")).unwrap()).unwrap();
    fs::write(
        root.join(pointer.generation).join("ranges.json"),
        b"corrupted",
    )
    .unwrap();
    let rebuilt = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    assert!(!rebuilt.reused_generation);
    assert_eq!(rebuilt.metadata.build.files_chunked, 0);
    assert_eq!(
        rebuilt.metadata.build.files_chunk_cache_reused,
        rebuilt.metadata.coverage.files_indexed
    );
    let old_manifest = fixture.manifest.clone();
    fixture.manifest.files.retain(|f| f.path != "lib/old.py");
    add_source(
        &mut fixture,
        "lib/new.py",
        "def uniquelynewrenderer():\n    return 2\n",
    );
    fixture.manifest.summary.snapshot_id.push_str("-next");
    let next = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert!(!next.reused_generation);
    assert!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &next,
            "uniquelyoldrenderer",
            None,
            8,
            &budget
        )
        .unwrap()
        .matches
        .is_empty()
    );
    assert_eq!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &next,
            "uniquelynewrenderer",
            None,
            8,
            &budget
        )
        .unwrap()
        .matches[0]
            .range
            .path,
        "lib/new.py"
    );
    assert!(
        search(
            &fixture.storage,
            &old_manifest,
            &next,
            "uniquelynewrenderer",
            None,
            8,
            &budget
        )
        .is_err()
    );
    // The old immutable snapshot remains independently searchable.
    assert_eq!(
        search(
            &fixture.storage,
            &old_manifest,
            &first,
            "uniquelyoldrenderer",
            None,
            8,
            &budget
        )
        .unwrap()
        .matches[0]
            .range
            .path,
        "lib/old.py"
    );
}

#[test]
fn one_changed_file_reuses_other_chunk_layouts_but_rehashes_all_source() {
    let mut fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    let mut contents = fixture.sources(&[SIMPLE_PATH])[0].contents.clone();
    contents.push_str("\nexport function incrementalNewThing() { return 42; }\n");
    fixture.recapture_with_source(SIMPLE_PATH, &contents);
    let updated = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    assert_eq!(updated.metadata.build.files_chunked, 1);
    assert_eq!(updated.metadata.build.documents_added, 1);
    assert_eq!(updated.metadata.build.documents_deleted, 1);
    assert_eq!(
        updated.metadata.build.files_documents_reused + 1,
        updated.metadata.coverage.files_indexed
    );
    assert_eq!(
        updated.metadata.build.documents_reused + 1,
        updated.metadata.coverage.chunks
    );
    assert!(updated.metadata.build.incremental_base_snapshot.is_some());
    assert!(updated.metadata.build.segment_bytes_copied > 0);
    assert_eq!(
        updated.metadata.build.files_chunk_cache_reused + 1,
        updated.metadata.coverage.files_indexed
    );
    assert_eq!(
        updated.metadata.build.source_bytes_verified,
        updated.metadata.coverage.bytes_indexed
    );
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &updated,
        "incrementalNewThing",
        None,
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(result.matches[0].range.path, SIMPLE_PATH);
    assert!(result.matches[0].declaration_match);
}

#[test]
fn rename_reuses_text_chunks_and_removes_the_old_path() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "lib/old.py",
        "def zzxuniquerender():\n    return 1\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let file = fixture
        .manifest
        .files
        .iter_mut()
        .find(|f| f.path == "lib/old.py")
        .unwrap();
    file.path = "lib/renamed.py".to_owned();
    fixture.manifest.summary.snapshot_id.push_str("-renamed");
    let renamed = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert_eq!(renamed.metadata.build.files_chunked, 0);
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &renamed,
        "zzxuniquerender",
        None,
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].range.path, "lib/renamed.py");
}

#[test]
fn exact_identifiers_rank_before_components_and_ties_are_repeatable() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/a.py", "renderGlyphBitmap()\n");
    add_source(&mut fixture, "lib/b.py", "render_glyph_bitmap()\n");
    add_source(&mut fixture, "lib/c.py", "render_glyph_bitmap()\n");
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let first = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "render_glyph_bitmap",
        Some("lib"),
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(first.matches[0].range.path, "lib/b.py");
    assert_eq!(first.matches[1].range.path, "lib/c.py");
    assert_eq!(first.matches[2].range.path, "lib/a.py");
    assert!(!first.matches[2].exact_identifier);
    let repeat = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "render_glyph_bitmap",
        Some("lib"),
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(
        json(&first.matches).unwrap(),
        json(&repeat.matches).unwrap()
    );
    assert!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "render_glyph_bitmap",
            Some("../lib"),
            8,
            &budget
        )
        .is_err()
    );
    for bad in ["", "\n", "  ", "x", "ok\nno"] {
        assert!(
            search(
                &fixture.storage,
                &fixture.manifest,
                &prepared,
                bad,
                None,
                8,
                &budget
            )
            .is_err()
        );
    }
    assert!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "render",
            None,
            33,
            &budget
        )
        .is_err()
    );
}

#[test]
fn mixed_search_keeps_literal_code_matches_when_lexical_results_exist() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/a-exact.txt", "needle\n");
    add_source(
        &mut fixture,
        "lib/b-partial.txt",
        "embeddedneedlefragment\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "needle",
        Some("lib"),
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(result.matches.len(), 2);
    assert_eq!(result.matches[0].range.path, "lib/a-exact.txt");
    assert!(
        result
            .matches
            .iter()
            .any(|hit| hit.range.path == "lib/b-partial.txt"
                && hit.text.contains("embeddedneedlefragment"))
    );
}

#[test]
fn ranked_lexical_results_precede_unranked_substring_fallbacks() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/a-literal.txt", "prerender_marker\n");
    add_source(&mut fixture, "lib/z-ranked.rs", "fn render_glyph() {}\n");
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();

    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "render glyph",
        None,
        1,
        &budget,
    )
    .unwrap();

    assert_eq!(result.matches.len(), 1);
    assert_eq!(result.matches[0].range.path, "lib/z-ranked.rs");
    assert!(!result.matches[0].exact_identifier);
}

#[test]
fn literal_code_operators_are_searchable_without_index_tokens() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "lib/operators.txt",
        "fn sample() -> bool {\r\n    left == right && item::ready()\r\n}\r\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    for (query, line) in [("::", 2), ("->", 1), ("==", 2), ("()", 1), ("&&", 2)] {
        let result = search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            query,
            Some("lib/operators.txt"),
            8,
            &budget,
        )
        .unwrap();
        assert_eq!(result.matches.len(), 1, "{query}");
        let hit = &result.matches[0];
        assert_eq!(hit.line, line, "{query}");
        assert!(hit.text.contains(query));
        assert!(!hit.exact_identifier && !hit.declaration_match);
        let read = read_range(
            &fixture.storage,
            &fixture.manifest,
            &hit.range.path,
            &hit.range.source_sha256,
            hit.range.start_byte,
            hit.range.end_byte,
        )
        .unwrap();
        assert_eq!(read.range.chunk_sha256, hit.range.chunk_sha256);
        assert!(read.contents.contains(query));
    }
}

#[test]
fn literal_fallback_finds_partial_identifiers_and_unicode_in_large_unsupported_text() {
    let mut fixture = Fixture::new();
    let text = format!(
        "{}def renderGlyphBitmap():\r\n    return '字形🦀Ünicode'\r\n",
        "# unrelated filler padding\r\n".repeat(26_000)
    );
    assert!(text.len() > 512 * 1024);
    add_source(&mut fixture, "lib/renderer.py", &text);
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    for (query, expected_line, snippet) in [
        ("lyphBit", 26_001, "renderGlyphBitmap"),
        ("🦀ü", 26_002, "字形🦀Ünicode"),
    ] {
        let result = search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            query,
            Some("lib"),
            8,
            &budget,
        )
        .unwrap();
        let hit = &result.matches[0];
        assert_eq!(hit.range.path, "lib/renderer.py");
        assert_eq!(hit.line, expected_line);
        assert!(hit.text.contains(snippet));
        assert!(hit.range.parser.is_none());
        assert!(result.coverage.text_fallback_files > 0);
        assert!(hit.range.end_byte - hit.range.start_byte <= MAX_RANGE_BYTES);
    }
}

#[test]
fn literal_matches_cross_chunk_boundaries_without_overlapping_returned_ranges() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "lib/long.txt",
        &format!("{}:: y :: 字形", "x".repeat(CHUNK_BYTES - 1)),
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "::",
        Some("lib/long.txt"),
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(result.matches.len(), 2);
    assert_eq!(result.matches[0].range.end_byte, CHUNK_BYTES + 1);
    assert!(result.matches[0].range.end_byte <= result.matches[1].range.start_byte);
    for hit in result.matches {
        assert_eq!(hit.line, 1);
        assert!(hit.text.contains("::"));
        let read = read_range(
            &fixture.storage,
            &fixture.manifest,
            &hit.range.path,
            &hit.range.source_sha256,
            hit.range.start_byte,
            hit.range.end_byte,
        )
        .unwrap();
        assert!(read.contents.contains("::"));
        assert_eq!(read.range.chunk_sha256, hit.range.chunk_sha256);
    }
}

#[test]
fn literal_fallback_is_deterministic_scoped_bounded_and_never_interprets_regex() {
    let mut fixture = Fixture::new();
    for path in ["library/first.txt", "lib/b.txt", "lib/a.txt"] {
        add_source(&mut fixture, path, "left::right\n");
    }
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let mut prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let first = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "::",
        Some("lib"),
        1,
        &budget,
    )
    .unwrap();
    assert_eq!(first.matches.len(), 1);
    assert_eq!(first.matches[0].range.path, "lib/a.txt");
    assert!(first.limited);
    assert_eq!(first.result_bytes, json(&first.matches).unwrap().len());
    prepared.metadata.files.reverse();
    prepared.metadata.ranges.reverse();
    let repeat = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "::",
        Some("lib"),
        1,
        &budget,
    )
    .unwrap();
    assert_eq!(
        json(&first.matches).unwrap(),
        json(&repeat.matches).unwrap()
    );
    for (query, scope) in [(".*", "lib"), ("::", "missing")] {
        let empty = search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            query,
            Some(scope),
            8,
            &budget,
        )
        .unwrap();
        assert!(empty.matches.is_empty());
        assert!(!empty.limited);
    }
}

#[test]
fn literal_fallback_retains_snapshot_filtering_cancellation_deadline_and_hash_checks() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/visible.txt", "left::right\n");
    add_source(&mut fixture, "vendor/private.txt", "left::right\n");
    add_source(&mut fixture, "secrets.env", "left::right\n");
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    for scope in ["vendor", "secrets.env"] {
        assert!(
            search(
                &fixture.storage,
                &fixture.manifest,
                &prepared,
                "::",
                Some(scope),
                8,
                &budget
            )
            .unwrap()
            .matches
            .is_empty()
        );
    }
    cancelled.store(true, Ordering::Release);
    assert_eq!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "::",
            None,
            8,
            &budget
        )
        .err()
        .unwrap(),
        "Code search stopped."
    );
    cancelled.store(false, Ordering::Release);
    let expired = Budget {
        cancelled: &cancelled,
        deadline: Instant::now() - Duration::from_secs(1),
    };
    assert_eq!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "::",
            None,
            8,
            &expired
        )
        .err()
        .unwrap(),
        "Code search reached its local time limit."
    );
    fixture.manifest.summary.snapshot_id.push_str("-other");
    assert!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "::",
            None,
            8,
            &budget
        )
        .is_err()
    );
    fixture
        .manifest
        .summary
        .snapshot_id
        .truncate(fixture.manifest.summary.snapshot_id.len() - 6);
    let file = fixture
        .manifest
        .files
        .iter()
        .find(|file| file.path == "lib/visible.txt")
        .unwrap();
    fs::write(fixture.storage.join(&file.blob_path), "tampered::text\n").unwrap();
    assert!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "::",
            Some("lib/visible.txt"),
            8,
            &budget
        )
        .is_err()
    );
}

#[test]
fn literal_fallback_rejects_corrupted_matching_range_metadata() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/visible.txt", "left::right\n");
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let mut prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    prepared
        .metadata
        .ranges
        .iter_mut()
        .find(|range| range.path == "lib/visible.txt")
        .unwrap()
        .chunk_sha256 = "0".repeat(64);
    assert!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "::",
            Some("lib/visible.txt"),
            8,
            &budget
        )
        .is_err()
    );
}

#[test]
fn excluded_secret_invalid_utf8_and_tampered_blobs_do_not_escape_native_reads() {
    let mut fixture = Fixture::new();
    for path in [
        "vendor/private.py",
        "../escape.py",
        "absolute//bad.py",
        "/absolute.py",
        "lib/token.py",
    ] {
        add_source(&mut fixture, path, "zzxexcludedmarker\n");
    }
    add_source(
        &mut fixture,
        "lib/config.py",
        "-----BEGIN PRIVATE KEY-----\nzzxexcludedmarker\n",
    );
    add_source(&mut fixture, "lib/visible.py", "visible_safe_identifier\n");
    let invalid = [0xf0, 0x28, 0x8c, 0x28];
    let sha256 = digest(&invalid);
    let blob_path = format!("objects/sha256/{}/{}", &sha256[..2], &sha256[2..]);
    let absolute = fixture.storage.join(&blob_path);
    fs::create_dir_all(absolute.parent().unwrap()).unwrap();
    fs::write(absolute, invalid).unwrap();
    fixture.manifest.files.push(snapshot::CapturedFile {
        path: "lib/invalid.py".to_owned(),
        kind: snapshot::InventoryPathKind::File,
        size_bytes: 4,
        sha256: sha256.clone(),
        blob_path,
        content_kind: snapshot::CapturedContentKind::Unknown,
        secret_like: false,
    });
    assert!(
        read_range(
            &fixture.storage,
            &fixture.manifest,
            "lib/invalid.py",
            &sha256,
            0,
            4
        )
        .is_err()
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "zzxexcludedmarker",
            None,
            8,
            &budget
        )
        .unwrap()
        .matches
        .is_empty()
    );
    let file = fixture
        .manifest
        .files
        .iter()
        .find(|f| f.path == "lib/visible.py")
        .unwrap();
    assert!(
        read_range(
            &fixture.storage,
            &fixture.manifest,
            "lib/token.py",
            &file.sha256,
            0,
            5
        )
        .is_err()
    );
    assert!(
        read_range(
            &fixture.storage,
            &fixture.manifest,
            &file.path,
            &"0".repeat(64),
            0,
            5
        )
        .is_err()
    );
    fs::write(fixture.storage.join(&file.blob_path), "tampered source").unwrap();
    assert!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "visible_safe_identifier",
            None,
            8,
            &budget
        )
        .is_err()
    );
}

#[test]
fn range_reads_enforce_utf8_bounds_hash_and_byte_budget() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/unicode.py", "é\r\n字\n");
    let file = fixture
        .manifest
        .files
        .iter()
        .find(|f| f.path == "lib/unicode.py")
        .unwrap();
    let read = read_range(
        &fixture.storage,
        &fixture.manifest,
        &file.path,
        &file.sha256,
        4,
        7,
    )
    .unwrap();
    assert_eq!(read.contents, "字");
    assert_eq!(read.range.start_line, 2);
    assert_eq!(read.range.end_line, 2);
    for (start, end) in [(1, 2), (0, 500), (0, 0), (0, MAX_RANGE_BYTES + 1)] {
        assert!(
            read_range(
                &fixture.storage,
                &fixture.manifest,
                &file.path,
                &file.sha256,
                start,
                end
            )
            .is_err()
        );
    }
}

#[test]
fn cancellation_keeps_last_complete_generation_and_never_publishes_staging() {
    let mut fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    let root = root(
        &fixture.storage,
        &fixture.manifest.summary.project_id,
        &fixture.manifest.summary.snapshot_id,
    );
    let pointer = fs::read(root.join("current.json")).unwrap();
    add_source(
        &mut fixture,
        "lib/changed.py",
        "def changed_function(): pass\n",
    );
    cancelled.store(true, Ordering::Release);
    assert!(
        prepare(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            &budget
        )
        .is_err()
    );
    assert_eq!(fs::read(root.join("current.json")).unwrap(), pointer);
    assert!(fs::read_dir(root).unwrap().all(|e| {
        !e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("staging-")
    }));
}

#[test]
fn corrupt_tantivy_data_is_rebuilt_and_expired_deadlines_do_not_start_builds() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let expired = Budget {
        cancelled: &cancelled,
        deadline: Instant::now() - Duration::from_millis(1),
    };
    assert!(
        prepare(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            &expired
        )
        .is_err()
    );
    let budget = Budget::new(&cancelled);
    prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    let directory = root(
        &fixture.storage,
        &fixture.manifest.summary.project_id,
        &fixture.manifest.summary.snapshot_id,
    );
    let pointer: Pointer =
        serde_json::from_slice(&fs::read(directory.join("current.json")).unwrap()).unwrap();
    fs::write(directory.join(pointer.generation).join("meta.json"), b"{}").unwrap();
    let rebuilt = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    assert!(!rebuilt.reused_generation);
    assert!(
        !search(
            &fixture.storage,
            &fixture.manifest,
            &rebuilt,
            "greeting",
            None,
            8,
            &budget
        )
        .unwrap()
        .matches
        .is_empty()
    );
}

/// Explicit local acceptance probe: never sends Fontdone source to a model or runs its code.
#[test]
#[ignore = "Requires the saved public Fontdone snapshot; run explicitly with CODE_ATLAS_FONTDONE_DATA_DIR."]
fn fontdone_local_index_profile() {
    let storage = PathBuf::from(
        std::env::var("CODE_ATLAS_FONTDONE_DATA_DIR").expect("set CODE_ATLAS_FONTDONE_DATA_DIR"),
    );
    let manifest = snapshot::load_snapshot_manifest(
        &storage,
        "/Users/lazytrot/work/fontdone",
        "snap-1792140879637485000-11155-0",
    )
    .unwrap();
    let syntax = index::load_snapshot_index(
        &storage,
        &manifest.summary.project_id,
        &manifest.summary.snapshot_id,
    )
    .unwrap();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let started = Instant::now();
    let prepared = prepare(&storage, &manifest, syntax.as_ref(), &budget).unwrap();
    println!("FONTDONE_INDEX {}", json(&serde_json::json!({"elapsedMs":started.elapsed().as_millis(),"reusedGeneration":prepared.reused_generation,"coverage":prepared.metadata.coverage,"build":prepared.metadata.build})).map(|b| String::from_utf8(b).unwrap()).unwrap());
    for (query, path) in [
        ("memory_face", "src/font.rs"),
        ("handle", "src/ffi/handles.rs"),
        ("font", "fontdone-c-abi/src/implementation.rs"),
    ] {
        let result = search(
            &storage,
            &manifest,
            &prepared,
            query,
            Some(path),
            8,
            &budget,
        )
        .unwrap();
        assert!(!result.matches.is_empty(), "missing {query} inside {path}");
        assert_eq!(result.matches[0].range.path, path);
        let first = &result.matches[0].range;
        let read = read_range(
            &storage,
            &manifest,
            path,
            &first.source_sha256,
            first.start_byte,
            first.end_byte,
        )
        .unwrap();
        assert!(read.contents.to_lowercase().contains(query));
        println!(
            "FONTDONE_SEARCH {}",
            String::from_utf8(json(&serde_json::json!({"query":query,"coverage":result.coverage,"elapsedMs":result.elapsed_ms,"resultBytes":result.result_bytes,"limited":result.limited,"ranges":result.matches.iter().map(|h| &h.range).collect::<Vec<_>>()})).unwrap()).unwrap()
        );
    }
    let warm = Instant::now();
    assert!(
        prepare(&storage, &manifest, syntax.as_ref(), &budget)
            .unwrap()
            .reused_generation
    );
    println!("FONTDONE_WARM_MS {}", warm.elapsed().as_millis());
}

#[test]
fn regex_search_matches_captured_unicode_crlf_and_preserves_case_sensitive_syntax() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "lib/patterns.txt",
        "fn first() {}\r\nlet naïve_字形 = value::Ready;\r\nskip\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let query = r"(?m)^let\s+naïve_字形\s*=\s*value::\S+;\r?$";
    let result = search_with_mode(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        query,
        Some("lib"),
        8,
        &budget,
        SearchMode::Regex,
    )
    .unwrap();
    assert_eq!(result.matches.len(), 1);
    let hit = &result.matches[0];
    assert_eq!(hit.line, 2);
    assert!(hit.text.contains("naïve_字形"));
    let read = read_range(
        &fixture.storage,
        &fixture.manifest,
        &hit.range.path,
        &hit.range.source_sha256,
        hit.range.start_byte,
        hit.range.end_byte,
    )
    .unwrap();
    assert_eq!(read.range.chunk_sha256, hit.range.chunk_sha256);
    for (query, count) in [(r"(?-i:Ready)", 1), (r"(?-i:ready)", 0), (".", 1)] {
        assert_eq!(
            search_with_mode(
                &fixture.storage,
                &fixture.manifest,
                &prepared,
                query,
                Some("lib/patterns.txt"),
                8,
                &budget,
                SearchMode::Regex
            )
            .unwrap()
            .matches
            .len(),
            count
        );
    }
}

#[test]
fn regex_search_rejects_malformed_empty_byte_splitting_and_oversized_patterns_safely() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/unicode.txt", "a café value\n");
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let mut patterns = vec![
        "(".to_owned(),
        r"(?=value)".into(),
        r"(a)\1".into(),
        "a*".into(),
        r"\b".into(),
        r"(?-u:\xA9)".into(),
        "a{999999999}".into(),
        "x".repeat(241),
    ];
    patterns.push(format!("{}a{}", "(".repeat(70), ")".repeat(70)));
    for query in patterns {
        let error = search_with_mode(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            &query,
            Some("lib"),
            8,
            &budget,
            SearchMode::Regex,
        )
        .err()
        .expect("pattern must fail safely");
        assert!(!error.contains("café"));
        assert!(error.len() < 240);
    }
    assert!(
        search_with_mode(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "value",
            Some("../outside"),
            8,
            &budget,
            SearchMode::Regex
        )
        .is_err()
    );
}

#[test]
fn regex_is_explicit_and_literal_search_never_interprets_metacharacters() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "lib/code.txt",
        "value::Ready\nvalue::Busy\nvalue::(Ready|Busy)\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let literal = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "::(Ready|Busy)",
        Some("lib"),
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(literal.matches.len(), 1);
    assert_eq!(literal.matches[0].line, 3);
    let regex = search_with_mode(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "::(Ready|Busy)",
        Some("lib"),
        8,
        &budget,
        SearchMode::Regex,
    )
    .unwrap();
    assert_eq!(regex.matches.len(), 1);
    assert_eq!(regex.matches[0].line, 1);
    assert!(regex.matches[0].text.contains("Ready"));
}

#[test]
fn search_pages_are_stable_complete_and_bounded_by_the_candidate_window() {
    assert_eq!(validate_search_page(32, 224).unwrap(), 256);
    assert_eq!(validate_search_page(1, 255).unwrap(), 256);
    assert!(validate_search_page(32, 225).is_err());
    assert!(validate_search_page(0, 0).is_err());
    assert!(validate_search_page(33, 0).is_err());

    let mut fixture = Fixture::new();
    for ordinal in 0..40 {
        add_source(
            &mut fixture,
            &format!("src/module-{ordinal:02}.txt"),
            "needle\n",
        );
    }
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();

    let first = search_with_mode_page(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "needle",
        None,
        16,
        0,
        &budget,
        SearchMode::Code,
    )
    .unwrap();
    let second = search_with_mode_page(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "needle",
        None,
        16,
        first.next_offset.unwrap(),
        &budget,
        SearchMode::Code,
    )
    .unwrap();
    let third = search_with_mode_page(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "needle",
        None,
        16,
        second.next_offset.unwrap(),
        &budget,
        SearchMode::Code,
    )
    .unwrap();

    assert_eq!(first.matches.len(), 16);
    assert_eq!(first.next_offset, Some(16));
    assert!(first.limited);
    assert_eq!(second.matches.len(), 16);
    assert_eq!(second.next_offset, Some(32));
    assert!(second.limited);
    assert_eq!(third.matches.len(), 8);
    assert_eq!(third.next_offset, None);
    assert!(!third.limited);
    let paths = first
        .matches
        .iter()
        .chain(&second.matches)
        .chain(&third.matches)
        .map(|hit| hit.range.path.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(paths.len(), 40);
}

#[test]
fn overlapping_ranked_and_literal_windows_expand_until_a_page_sentinel_or_cap() {
    let source_sha256 = digest(b"captured source");
    let range = |start_byte, end_byte| CodeRange {
        snapshot_id: "snapshot".to_owned(),
        path: "src/module.rs".to_owned(),
        source_sha256: source_sha256.clone(),
        start_byte,
        end_byte,
        start_line: 1,
        end_line: 1,
        chunk_sha256: digest(format!("{start_byte}:{end_byte}").as_bytes()),
        enclosing: None,
        parent: None,
        parser: None,
    };

    // Each bounded search returns three windows. Their union contains only
    // two unique windows because the ranked and literal prefixes overlap.
    let ranked = [range(0, 80), range(40, 120), range(120, 200)];
    let literal = [range(10, 70), range(70, 110), range(130, 190)];
    let mut unique = Vec::<CodeRange>::new();
    for candidate in ranked.into_iter().chain(literal) {
        if !unique
            .iter()
            .any(|existing| search_windows_overlap(existing, &candidate))
        {
            unique.push(candidate);
        }
    }

    assert_eq!(unique.len(), 2);
    let different_snapshot = CodeRange {
        snapshot_id: "another-snapshot".to_owned(),
        ..range(10, 70)
    };
    let different_file = CodeRange {
        path: "src/other.rs".to_owned(),
        ..range(10, 70)
    };
    let changed_source = CodeRange {
        source_sha256: digest(b"changed source"),
        ..range(10, 70)
    };
    assert!(!search_windows_overlap(&unique[0], &different_snapshot));
    assert!(!search_windows_overlap(&unique[0], &different_file));
    assert!(!search_windows_overlap(&unique[0], &changed_source));

    let page_end = 2;
    let has_page_sentinel = unique.len() > page_end;
    assert!(!has_page_sentinel);
    assert_eq!(
        expanded_search_fetch_limit(3, true, has_page_sentinel),
        Some(6)
    );
    assert_eq!(expanded_search_fetch_limit(6, true, true), None);
    assert_eq!(
        expanded_search_fetch_limit(MAX_CANDIDATES, true, false),
        None
    );
    assert_eq!(expanded_search_fetch_limit(6, false, false), None);
}

#[test]
fn regex_cross_chunk_matches_keep_exact_nonoverlapping_ranges_and_reject_huge_matches() {
    let mut fixture = Fixture::new();
    let text = format!(
        "{}Start\nFinish {}Start\nFinish\n",
        "x".repeat(CHUNK_BYTES - 3),
        "z".repeat(CHUNK_BYTES)
    );
    add_source(&mut fixture, "lib/boundary.txt", &text);
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let result = search_with_mode(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        r"Start\s+Finish",
        Some("lib"),
        8,
        &budget,
        SearchMode::Regex,
    )
    .unwrap();
    assert_eq!(result.matches.len(), 2);
    for hit in &result.matches {
        let read = read_range(
            &fixture.storage,
            &fixture.manifest,
            &hit.range.path,
            &hit.range.source_sha256,
            hit.range.start_byte,
            hit.range.end_byte,
        )
        .unwrap();
        assert!(read.contents.contains("Start\nFinish"));
        assert_eq!(read.range.chunk_sha256, hit.range.chunk_sha256);
    }
    assert!(result.matches[0].range.end_byte <= result.matches[1].range.start_byte);
    let large = "x".repeat(MAX_RANGE_BYTES + 1);
    add_source(&mut fixture, "lib/huge.txt", &large);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let error = search_with_mode(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        r"(?s)^.+$",
        Some("lib/huge.txt"),
        8,
        &budget,
        SearchMode::Regex,
    )
    .err()
    .unwrap();
    assert!(error.contains("32 KiB"));
}

#[test]
fn mixed_search_prioritizes_exact_identifiers_and_reports_discarded_literal_results() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/z-exact.txt", "needle\n");
    add_source(
        &mut fixture,
        "lib/a-literal.txt",
        "embeddedneedlefragment\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "needle",
        Some("lib"),
        1,
        &budget,
    )
    .unwrap();
    assert_eq!(result.matches.len(), 1);
    assert!(result.matches[0].exact_identifier);
    assert_eq!(result.matches[0].range.path, "lib/z-exact.txt");
    assert!(result.limited);
    for _ in 0..3 {
        let result = search(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "needle",
            Some("lib"),
            8,
            &budget,
        )
        .unwrap();
        assert_eq!(result.matches.len(), 2);
        assert!(!result.limited);
        assert_eq!(result.matches[0].range.path, "lib/z-exact.txt");
        assert_eq!(result.result_bytes, json(&result.matches).unwrap().len());
    }
}

#[test]
fn regex_search_retains_exclusions_cancellation_deadlines_snapshot_and_object_integrity() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/visible.txt", "value::Ready\n");
    add_source(&mut fixture, "vendor/hidden.txt", "value::Ready\n");
    add_source(&mut fixture, "secrets.env", "value::Ready\n");
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let prepared = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    for scope in ["vendor", "secrets.env"] {
        assert!(
            search_with_mode(
                &fixture.storage,
                &fixture.manifest,
                &prepared,
                "::Ready",
                Some(scope),
                8,
                &budget,
                SearchMode::Regex
            )
            .unwrap()
            .matches
            .is_empty()
        );
    }
    cancelled.store(true, Ordering::Release);
    assert_eq!(
        search_with_mode(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "::Ready",
            None,
            8,
            &budget,
            SearchMode::Regex
        )
        .err()
        .unwrap(),
        "Code search stopped."
    );
    cancelled.store(false, Ordering::Release);
    let expired = Budget {
        cancelled: &cancelled,
        deadline: Instant::now() - Duration::from_secs(1),
    };
    assert!(
        search_with_mode(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "::Ready",
            None,
            8,
            &expired,
            SearchMode::Regex
        )
        .is_err()
    );
    let mut changed = fixture.manifest.clone();
    changed.summary.snapshot_id.push_str("-different");
    assert!(
        search_with_mode(
            &fixture.storage,
            &changed,
            &prepared,
            "::Ready",
            None,
            8,
            &budget,
            SearchMode::Regex
        )
        .is_err()
    );
    let file = fixture
        .manifest
        .files
        .iter()
        .find(|file| file.path == "lib/visible.txt")
        .unwrap();
    fs::write(fixture.storage.join(&file.blob_path), "damaged").unwrap();
    assert!(
        search_with_mode(
            &fixture.storage,
            &fixture.manifest,
            &prepared,
            "::Ready",
            None,
            8,
            &budget,
            SearchMode::Regex
        )
        .is_err()
    );
}
