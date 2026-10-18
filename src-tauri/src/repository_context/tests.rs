use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH};

fn add_source(fixture: &mut Fixture, path: &str, text: &str) {
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
fn large_unsupported_text_is_searchable_without_parser_or_whole_file_prompt() {
    let mut fixture = Fixture::new();
    let text = format!(
        "{}\ndef enormous_memory_face(buffer):\n    return renderGlyphBitmap(buffer)\n",
        "# filler with other code\n".repeat(26_000)
    );
    assert!(text.len() > 512 * 1024);
    add_source(&mut fixture, "lib/renderer.py", &text);
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
        "enormous_memory_face",
        None,
        8,
        &budget,
    )
    .unwrap();
    let hit = &result.matches[0];
    assert_eq!(hit.range.path, "lib/renderer.py");
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
    assert!(read.contents.contains("def enormous_memory_face"));
    assert_eq!(read.range.chunk_sha256, hit.range.chunk_sha256);
    assert_eq!(hit.line, 26_002);
    let component = search(
        &fixture.storage,
        &fixture.manifest,
        &prepared,
        "glyph bitmap",
        Some("lib"),
        8,
        &budget,
    )
    .unwrap();
    assert_eq!(component.matches[0].range.path, "lib/renderer.py");
    assert!(component.coverage.text_fallback_files > 0);
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
