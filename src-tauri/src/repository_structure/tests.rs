use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};
use std::{fs, sync::atomic::AtomicBool};

fn add_source(fixture: &mut Fixture, path: &str, text: &str) {
    let sha256 = context::digest(text.as_bytes());
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
fn exact_symbols_return_immutable_absolute_coordinates_and_parser_identity() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let result = definitions(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        "greeting",
        None,
        8,
        0,
        &context::Budget::new(&cancelled),
    )
    .unwrap();
    let found = &result.items[0];
    assert_eq!(found.range.path, SIMPLE_PATH);
    assert_eq!(found.name, "greeting");
    assert!(found.match_kind.starts_with("exact"));
    assert!(found.range.parser.is_some());
    assert_eq!(
        found.range.snapshot_id,
        fixture.manifest.summary.snapshot_id
    );
    assert_eq!(found.range.start_line, 2);
    let read = context::read_range(
        &fixture.storage,
        &fixture.manifest,
        SIMPLE_PATH,
        &found.range.source_sha256,
        found.range.start_byte,
        found.range.end_byte,
    )
    .unwrap();
    assert!(read.contents.contains("function greeting("));
    assert_eq!(read.range.chunk_sha256, found.range.chunk_sha256);
    assert_eq!(
        result.indexer_version.as_deref(),
        Some(fixture.index.indexer_version.as_str())
    );
    assert!(result.interpretation.contains("not compiler or runtime"));
}

#[test]
fn name_matching_prefers_exact_qualified_names_and_keeps_partial_matches_last() {
    let fixture = Fixture::new();
    let mut symbol = fixture.index.symbols[0].clone();
    symbol.name = "loadGlyph".to_owned();
    symbol.qualified_name = "Face::loadGlyph".to_owned();
    assert_eq!(
        match_rank(&symbol, "Face::loadGlyph"),
        Some((0, "exactQualified"))
    );
    assert_eq!(match_rank(&symbol, "loadGlyph"), Some((1, "exactName")));
    assert_eq!(match_rank(&symbol, "LOADGLYPH"), Some((2, "caseFolded")));
    assert_eq!(
        match_rank(&symbol, "Other::loadGlyph"),
        Some((3, "unqualifiedName"))
    );
    assert_eq!(match_rank(&symbol, "Glyph"), Some((4, "partialName")));
    assert_eq!(match_rank(&symbol, "unrelated"), None);
}

#[test]
fn huge_unicode_crlf_declarations_return_a_bounded_prefix_with_the_full_extent() {
    let mut fixture = Fixture::new();
    let text = format!(
        "// café 🦀\r\nexport function greeting() {{\r\n{}return 'done';\r\n}}\r\n",
        "// 字形 🦀\r\n".repeat(4000)
    );
    fixture.recapture_with_source(SIMPLE_PATH, &text);
    let cancelled = AtomicBool::new(false);
    let result = definitions(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        "greeting",
        None,
        8,
        0,
        &context::Budget::new(&cancelled),
    )
    .unwrap();
    let symbol = &result.items[0];
    assert!(symbol.range.end_byte - symbol.range.start_byte <= context::CHUNK_BYTES);
    assert!(symbol.declaration_end_byte > symbol.range.end_byte);
    assert_eq!(
        symbol.range.enclosing.as_ref().unwrap().end_byte,
        symbol.declaration_end_byte
    );
    assert!(text.is_char_boundary(symbol.range.end_byte));
    assert_eq!(symbol.range.start_line, 2);
}

#[test]
fn invalid_hash_coordinates_and_corrupted_blobs_do_not_become_definitions() {
    let mut fixture = Fixture::new();
    let index = fixture.index.clone();
    let cancelled = AtomicBool::new(false);
    for variant in 0..4 {
        fixture.index = index.clone();
        let anchor = &mut fixture
            .index
            .symbols
            .iter_mut()
            .find(|s| s.name == "greeting")
            .unwrap()
            .anchor;
        match variant {
            0 => anchor.source_sha256 = "0".repeat(64),
            1 => anchor.start_line += 1,
            2 => anchor.end_column += 1,
            _ => anchor.end_byte = SIMPLE_SOURCE.len() + 1,
        }
        let page = definitions(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            "greeting",
            None,
            8,
            0,
            &context::Budget::new(&cancelled),
        )
        .unwrap();
        assert!(page.items.is_empty());
        assert_eq!(page.unavailable, 1);
    }
    fixture.index = index;
    let file = fixture
        .manifest
        .files
        .iter()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap();
    fs::write(fixture.storage.join(&file.blob_path), "tampered").unwrap();
    assert!(
        definitions(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            "greeting",
            None,
            8,
            0,
            &context::Budget::new(&cancelled)
        )
        .unwrap()
        .items
        .is_empty()
    );
}

#[test]
fn definition_pages_are_repeatable_and_reject_unbounded_inputs() {
    let mut fixture = Fixture::new();
    let template = fixture
        .index
        .symbols
        .iter()
        .find(|s| s.name == "greeting")
        .unwrap()
        .clone();
    fixture.index.symbols = (0..40)
        .rev()
        .map(|i| {
            let mut s = template.clone();
            s.symbol_id = format!("symbol-{i:03}");
            s
        })
        .collect();
    let cancelled = AtomicBool::new(false);
    let budget = context::Budget::new(&cancelled);
    let first = definitions(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        "greeting",
        None,
        32,
        0,
        &budget,
    )
    .unwrap();
    let second = definitions(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        "greeting",
        None,
        32,
        first.next_offset.unwrap(),
        &budget,
    )
    .unwrap();
    assert_eq!(first.items.len(), 32);
    assert_eq!(second.items.len(), 8);
    assert_eq!(second.next_offset, None);
    assert_eq!(first.items[0].symbol_id, "symbol-000");
    assert_eq!(second.items[0].symbol_id, "symbol-032");
    fixture.index.symbols.reverse();
    let repeated = definitions(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        "greeting",
        None,
        32,
        0,
        &budget,
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(&first).unwrap(),
        serde_json::to_value(&repeated).unwrap()
    );
    for (query, scope, limit, offset) in [
        ("x", None, 8, 0),
        ("greeting", Some("../src"), 8, 0),
        ("greeting", None, 33, 0),
        ("greeting", None, 0, 0),
        ("greeting", None, 8, MAX_OFFSET + 1),
        ("hello\nthere", None, 8, 0),
    ] {
        assert!(
            definitions(
                &fixture.storage,
                &fixture.manifest,
                Some(&fixture.index),
                query,
                scope,
                limit,
                offset,
                &budget
            )
            .is_err()
        );
    }
}

#[test]
fn missing_syntax_is_explicit_and_foreign_snapshot_context_is_rejected() {
    let mut fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = context::Budget::new(&cancelled);
    let page = definitions(
        &fixture.storage,
        &fixture.manifest,
        None,
        "greeting",
        None,
        8,
        0,
        &budget,
    )
    .unwrap();
    assert!(!page.syntax_available);
    assert!(page.items.is_empty());
    assert!(page.interpretation.contains("captured text"));
    fixture.index.snapshot_id.push_str("-foreign");
    assert!(
        definitions(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            "greeting",
            None,
            8,
            0,
            &budget
        )
        .is_err()
    );
    assert!(
        module_map(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            None,
            8,
            0,
            &budget
        )
        .is_err()
    );
}

#[test]
fn scoped_parser_gaps_are_disclosed_without_exposing_excluded_file_metadata() {
    let mut fixture = Fixture::new();
    fixture.index.symbols.retain(|s| s.path != SIMPLE_PATH);
    fixture
        .index
        .files
        .iter_mut()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap()
        .status = index::IndexFileStatus::FileTooLarge;
    let cancelled = AtomicBool::new(false);
    let run = |fixture: &Fixture| {
        definitions(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            "greeting",
            Some(SIMPLE_PATH),
            8,
            0,
            &context::Budget::new(&cancelled),
        )
        .unwrap()
    };
    let page = run(&fixture);
    assert!(page.items.is_empty());
    assert_eq!(
        page.scoped_syntax_status,
        Some(index::IndexFileStatus::FileTooLarge)
    );
    fixture
        .manifest
        .files
        .iter_mut()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap()
        .secret_like = true;
    assert!(run(&fixture).scoped_syntax_status.is_none());
}

#[test]
fn relationships_follow_both_directions_and_label_static_candidates() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = context::Budget::new(&cancelled);
    let call = fixture
        .index
        .relationships
        .iter()
        .find(|r| {
            r.kind == index::RelationshipKind::Calls
                && r.confidence == index::RelationshipConfidence::High
        })
        .unwrap();
    let outgoing = relationships(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &call.source_id,
        Direction::Outgoing,
        None,
        32,
        0,
        &budget,
    )
    .unwrap();
    let edge = outgoing
        .items
        .iter()
        .find(|r| r.relationship_id == call.relationship_id)
        .unwrap();
    assert_eq!(edge.resolution, "staticCandidate");
    assert!(edge.source.is_some());
    assert!(edge.target.is_some());
    let incoming = relationships(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        call.target_id.as_deref().unwrap(),
        Direction::Incoming,
        None,
        32,
        0,
        &budget,
    )
    .unwrap();
    assert!(
        incoming
            .items
            .iter()
            .any(|r| r.relationship_id == call.relationship_id)
    );
    assert!(incoming.interpretation.contains("do not prove runtime"));
    let unresolved = outgoing
        .items
        .iter()
        .filter(|r| r.confidence == index::RelationshipConfidence::Unresolved)
        .collect::<Vec<_>>();
    assert!(!unresolved.is_empty());
    assert!(
        unresolved
            .iter()
            .all(|r| r.resolution == "unresolved" && r.unresolved_reason.is_some())
    );
}

#[test]
fn excluded_targets_are_redacted_and_excluded_requested_endpoints_disclose_no_links() {
    let mut fixture = Fixture::new();
    let call = fixture
        .index
        .relationships
        .iter()
        .find(|r| {
            r.kind == index::RelationshipKind::Calls
                && r.confidence == index::RelationshipConfidence::High
        })
        .unwrap()
        .clone();
    let target_id = call.target_id.clone().unwrap();
    let target_path = fixture
        .index
        .symbols
        .iter()
        .find(|s| s.symbol_id == target_id)
        .unwrap()
        .path
        .clone();
    fixture
        .manifest
        .files
        .iter_mut()
        .find(|f| f.path == target_path)
        .unwrap()
        .secret_like = true;
    let raw = fixture
        .index
        .relationships
        .iter_mut()
        .find(|r| r.relationship_id == call.relationship_id)
        .unwrap();
    raw.target_label = Some("DO_NOT_DISCLOSE".to_owned());
    raw.unresolved_reason = Some("DO_NOT_DISCLOSE".to_owned());
    let cancelled = AtomicBool::new(false);
    let budget = context::Budget::new(&cancelled);
    let outgoing = relationships(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &call.source_id,
        Direction::Outgoing,
        None,
        32,
        0,
        &budget,
    )
    .unwrap();
    let edge = outgoing
        .items
        .iter()
        .find(|r| r.relationship_id == call.relationship_id)
        .unwrap();
    assert_eq!(edge.resolution, "unresolved");
    assert!(edge.target.is_none());
    assert!(edge.target_label.is_none());
    assert!(
        !serde_json::to_string(&outgoing)
            .unwrap()
            .contains("DO_NOT_DISCLOSE")
    );
    let hidden = relationships(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &target_id,
        Direction::Both,
        None,
        32,
        0,
        &budget,
    )
    .unwrap();
    assert!(hidden.items.is_empty());
    assert_eq!(hidden.unavailable, 1);
}

#[test]
fn relationship_pages_are_bounded_and_invalid_evidence_is_omitted() {
    let mut fixture = Fixture::new();
    let template = fixture
        .index
        .relationships
        .iter()
        .find(|r| {
            r.kind == index::RelationshipKind::Calls
                && r.confidence == index::RelationshipConfidence::High
        })
        .unwrap()
        .clone();
    fixture.index.relationships = (0..35)
        .rev()
        .map(|i| {
            let mut r = template.clone();
            r.relationship_id = format!("relation-{i:03}");
            r
        })
        .collect();
    fixture.index.relationships[0].evidence.start_column += 1;
    let cancelled = AtomicBool::new(false);
    let budget = context::Budget::new(&cancelled);
    let first = relationships(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &template.source_id,
        Direction::Outgoing,
        None,
        32,
        0,
        &budget,
    )
    .unwrap();
    let second = relationships(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &template.source_id,
        Direction::Outgoing,
        None,
        32,
        first.next_offset.unwrap(),
        &budget,
    )
    .unwrap();
    assert_eq!(first.items.len() + second.items.len(), 34);
    assert_eq!(second.unavailable, 1);
    assert_eq!(second.next_offset, None);
    assert!(
        relationships(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            "",
            Direction::Both,
            None,
            8,
            0,
            &budget
        )
        .is_err()
    );
}

#[test]
fn module_map_groups_immediate_children_preserves_text_fallback_and_is_deterministic() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "src2/unsupported.py",
        "def render_bitmap():\n    return 1\n",
    );
    let cancelled = AtomicBool::new(false);
    let budget = context::Budget::new(&cancelled);
    let root = module_map(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        None,
        32,
        0,
        &budget,
    )
    .unwrap();
    assert_eq!(root.items[0].path, "src");
    assert!(root.items[0].directory);
    assert!(root.items[0].public_symbols.len() <= 4);
    let python = root.items.iter().find(|m| m.path == "src2").unwrap();
    assert_eq!(python.captured_text_files, 1);
    assert_eq!(python.code_files, 1);
    assert_eq!(python.syntax_files, 0);
    fixture.manifest.files.reverse();
    fixture.index.symbols.reverse();
    assert_eq!(
        serde_json::to_value(&root).unwrap(),
        serde_json::to_value(
            module_map(
                &fixture.storage,
                &fixture.manifest,
                Some(&fixture.index),
                None,
                32,
                0,
                &budget
            )
            .unwrap()
        )
        .unwrap()
    );
    let children = module_map(
        &fixture.storage,
        &fixture.manifest,
        None,
        Some("src"),
        32,
        0,
        &budget,
    )
    .unwrap();
    assert!(
        children
            .items
            .iter()
            .all(|m| m.path.starts_with("src/") && !m.directory)
    );
    assert!(!children.syntax_available);
    let file = module_map(
        &fixture.storage,
        &fixture.manifest,
        None,
        Some(SIMPLE_PATH),
        32,
        0,
        &budget,
    )
    .unwrap();
    assert_eq!(file.items.len(), 1);
    assert_eq!(file.items[0].path, SIMPLE_PATH);
    assert!(!file.items[0].directory);
}

#[test]
fn module_map_omits_excluded_secret_and_corrupt_sources_and_paginates() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "vendor/private.ts", "export const no = 1;");
    add_source(&mut fixture, ".env", "SECRET=private");
    let file = fixture
        .manifest
        .files
        .iter()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap();
    fs::write(fixture.storage.join(&file.blob_path), "tampered").unwrap();
    let cancelled = AtomicBool::new(false);
    let budget = context::Budget::new(&cancelled);
    let root = module_map(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        None,
        1,
        0,
        &budget,
    )
    .unwrap();
    assert_eq!(root.items.len(), 1);
    assert!(root.unavailable >= 3);
    assert!(root.next_offset.is_some());
    let full = module_map(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        None,
        32,
        0,
        &budget,
    )
    .unwrap();
    assert!(
        full.items
            .iter()
            .all(|m| m.path != "vendor" && m.path != ".env")
    );
    assert!(!full.items[0].public_symbols.iter().any(|s| s == "greeting"));
}

#[test]
fn all_structural_operations_honor_cancellation_and_source_byte_budget() {
    let mut fixture = Fixture::new();
    let cancelled = AtomicBool::new(true);
    let budget = context::Budget::new(&cancelled);
    assert!(
        definitions(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            "greeting",
            None,
            8,
            0,
            &budget
        )
        .unwrap_err()
        .contains("stopped")
    );
    assert!(
        module_map(
            &fixture.storage,
            &fixture.manifest,
            None,
            None,
            8,
            0,
            &budget
        )
        .unwrap_err()
        .contains("stopped")
    );
    assert!(
        relationships(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            "unknown",
            Direction::Both,
            None,
            8,
            0,
            &budget
        )
        .unwrap_err()
        .contains("stopped")
    );
    fixture
        .manifest
        .files
        .iter_mut()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap()
        .size_bytes = MAX_VERIFIED_BYTES + 1;
    let mut sources = Sources::new(&fixture.storage, &fixture.manifest);
    assert!(sources.get(SIMPLE_PATH).is_err());
    assert_eq!(sources.admitted, 0);
    assert!(sources.get(SIMPLE_PATH).is_err());
    assert_eq!(sources.admitted, 0);
}

/// Reads only the frozen public snapshot. It neither runs repository code nor calls a model.
#[test]
#[ignore = "Explicit Fontdone structural-context measurement; requires CODE_ATLAS_FONTDONE_DATA_DIR."]
fn fontdone_structural_profile() {
    use std::{path::PathBuf, time::Instant};
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Expected {
        id: String,
        path: String,
        query: String,
        source_sha256: String,
        start_byte: usize,
        end_byte: usize,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Corpus {
        snapshot_id: String,
        expected: Vec<Expected>,
    }
    let corpus: Corpus = serde_json::from_str(include_str!(
        "../../../docs/acceptance/fixtures/fontdone-retrieval-ranges.json"
    ))
    .unwrap();
    let storage = PathBuf::from(
        std::env::var("CODE_ATLAS_FONTDONE_DATA_DIR").expect("set CODE_ATLAS_FONTDONE_DATA_DIR"),
    );
    let manifest = snapshot::load_snapshot_manifest(
        &storage,
        "/Users/lazytrot/work/fontdone",
        &corpus.snapshot_id,
    )
    .unwrap();
    let syntax =
        index::load_snapshot_index(&storage, &manifest.summary.project_id, &corpus.snapshot_id)
            .unwrap()
            .unwrap();
    let cancelled = AtomicBool::new(false);
    let mut recovered = 0;
    for expected in &corpus.expected {
        let started = Instant::now();
        let page = definitions(
            &storage,
            &manifest,
            Some(&syntax),
            &expected.query,
            Some(&expected.path),
            32,
            0,
            &context::Budget::new(&cancelled),
        )
        .unwrap();
        let hit = page.items.iter().find(|s| {
            s.range.path == expected.path
                && s.range.source_sha256 == expected.source_sha256
                && s.range.start_byte <= expected.start_byte
                && s.range.end_byte >= expected.end_byte
        });
        recovered += usize::from(hit.is_some());
        println!(
            "FONTDONE_DEFINITION {}",
            serde_json::json!({"id":expected.id,"found":hit.is_some(),"elapsedMs":started.elapsed().as_millis(),"returned":page.items.len(),"serializedBytes":serde_json::to_vec(&page).unwrap().len(),"syntaxStatus":page.scoped_syntax_status})
        );
        if expected.id == "format-branch" {
            let symbol = hit.expect("missing independently reviewed memory_face declaration");
            let started = Instant::now();
            let links = relationships(
                &storage,
                &manifest,
                Some(&syntax),
                &symbol.symbol_id,
                Direction::Outgoing,
                None,
                32,
                0,
                &context::Budget::new(&cancelled),
            )
            .unwrap();
            println!(
                "FONTDONE_RELATIONSHIPS {}",
                serde_json::json!({"symbol":"memory_face","returned":links.items.len(),"staticCandidates":links.items.iter().filter(|r| r.resolution=="staticCandidate").count(),"unresolved":links.items.iter().filter(|r| r.resolution=="unresolved").count(),"unavailable":links.unavailable,"nextOffset":links.next_offset,"elapsedMs":started.elapsed().as_millis(),"serializedBytes":serde_json::to_vec(&links).unwrap().len()})
            );
        }
    }
    let started = Instant::now();
    let map = module_map(
        &storage,
        &manifest,
        Some(&syntax),
        None,
        32,
        0,
        &context::Budget::new(&cancelled),
    )
    .unwrap();
    assert!(map.items.iter().any(|m| m.path == "src" && m.directory));
    println!(
        "FONTDONE_MODULE_MAP {}",
        serde_json::json!({"elapsedMs":started.elapsed().as_millis(),"serializedBytes":serde_json::to_vec(&map).unwrap().len(),"map":map,"definitionsRecovered":recovered,"expectedCount":corpus.expected.len()})
    );
}
