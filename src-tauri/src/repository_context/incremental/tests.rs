use super::*;
use crate::{chapter_generation::tests::Fixture, repository_context::tests::add_source};

fn identities(result: SearchResult) -> Vec<(String, usize, usize, String, String)> {
    let mut identities = result
        .matches
        .into_iter()
        .map(|h| {
            (
                h.range.path,
                h.range.start_byte,
                h.range.end_byte,
                h.range.source_sha256,
                h.range.chunk_sha256,
            )
        })
        .collect::<Vec<_>>();
    identities.sort();
    identities
}

fn copy_blobs(from: &Path, to: &Path, manifest: &snapshot::SnapshotManifest) {
    for file in &manifest.files {
        let source = from.join(&file.blob_path);
        let destination = to.join(&file.blob_path);
        if !source.is_file() || destination.exists() {
            continue;
        }
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        fs::copy(source, destination).unwrap();
    }
}

fn generation_hashes(directory: &Path) -> HashMap<PathBuf, String> {
    fs::read_dir(directory)
        .unwrap()
        .map(|e| {
            let p = e.unwrap().path();
            (
                p.file_name().unwrap().into(),
                digest(&fs::read(&p).unwrap()),
            )
        })
        .collect()
}

#[test]
fn additions_deletions_renames_and_shifted_file_order_match_a_fresh_index() {
    let mut fixture = Fixture::new();
    add_source(
        &mut fixture,
        "lib/z.txt",
        "uniqueOldPathMarker preservedRenamedBody\n",
    );
    add_source(&mut fixture, "lib/remove.txt", "uniqueDeletedBodyMarker\n");
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let old_manifest = fixture.manifest.clone();
    let first = prepare(&fixture.storage, &old_manifest, None, &budget).unwrap();
    let old_files = generation_hashes(&first.generation);
    fixture.manifest.summary.snapshot_id.push_str("-delta");
    fixture
        .manifest
        .files
        .retain(|f| f.path != "lib/remove.txt");
    fixture
        .manifest
        .files
        .iter_mut()
        .find(|f| f.path == "lib/z.txt")
        .unwrap()
        .path = "lib/b.txt".into();
    add_source(&mut fixture, "lib/a.txt", "uniqueAddedBodyMarker\n");
    let updated = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert_eq!(updated.metadata.build.documents_added, 2);
    assert_eq!(updated.metadata.build.documents_deleted, 2);
    assert_eq!(
        updated.metadata.build.documents_reused + 2,
        updated.metadata.ranges.len()
    );
    assert_eq!(generation_hashes(&first.generation), old_files);
    let fresh_storage = Fixture::new();
    copy_blobs(&fixture.storage, &fresh_storage.storage, &fixture.manifest);
    let fresh = prepare(&fresh_storage.storage, &fixture.manifest, None, &budget).unwrap();
    assert!(fresh.metadata.build.incremental_base_snapshot.is_none());
    assert_eq!(updated.metadata.document_keys, fresh.metadata.document_keys);
    for query in [
        "uniqueOldPathMarker",
        "uniqueDeletedBodyMarker",
        "uniqueAddedBodyMarker",
        "greeting",
        "return",
        "const",
    ] {
        for scope in [None, Some("lib"), Some("src")] {
            assert_eq!(
                identities(
                    search(
                        &fixture.storage,
                        &fixture.manifest,
                        &updated,
                        query,
                        scope,
                        32,
                        &budget
                    )
                    .unwrap()
                ),
                identities(
                    search(
                        &fresh_storage.storage,
                        &fixture.manifest,
                        &fresh,
                        query,
                        scope,
                        32,
                        &budget
                    )
                    .unwrap()
                ),
                "{query} {scope:?}"
            );
        }
    }
    assert_eq!(
        search(
            &fixture.storage,
            &old_manifest,
            &first,
            "uniqueOldPathMarker",
            None,
            8,
            &budget
        )
        .unwrap()
        .matches[0]
            .range
            .path,
        "lib/z.txt"
    );
    assert_eq!(
        search(
            &fixture.storage,
            &fixture.manifest,
            &updated,
            "uniqueOldPathMarker",
            None,
            8,
            &budget
        )
        .unwrap()
        .matches[0]
            .range
            .path,
        "lib/b.txt"
    );
    let loaded = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert!(loaded.reused_generation);
    assert_eq!(
        loaded.metadata.document_keys,
        updated.metadata.document_keys
    );
}

#[test]
fn new_symbol_ids_reuse_postings_but_return_only_current_symbol_context() {
    let mut fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let first = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    let before = generation_hashes(&first.generation);
    fixture.manifest.summary.snapshot_id.push_str("-ids");
    fixture.index.snapshot_id = fixture.manifest.summary.snapshot_id.clone();
    for symbol in &mut fixture.index.symbols {
        symbol.symbol_id.push_str("-current");
    }
    let updated = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    assert_eq!(updated.metadata.build.documents_added, 0);
    assert_eq!(updated.metadata.build.documents_deleted, 0);
    assert_eq!(
        updated.metadata.build.documents_reused,
        updated.metadata.ranges.len()
    );
    assert_eq!(updated.metadata.document_keys, first.metadata.document_keys);
    assert_eq!(generation_hashes(&first.generation), before);
    let result = search(
        &fixture.storage,
        &fixture.manifest,
        &updated,
        "greeting",
        None,
        8,
        &budget,
    )
    .unwrap();
    assert!(
        result
            .matches
            .iter()
            .filter_map(|h| h.range.enclosing.as_ref())
            .all(|s| s.symbol_id.ends_with("-current"))
    );
    assert!(
        result
            .matches
            .iter()
            .all(|h| h.range.snapshot_id == fixture.manifest.summary.snapshot_id)
    );
}

#[test]
fn changed_exclusion_or_corrupt_current_source_deletes_old_documents() {
    let mut fixture = Fixture::new();
    add_source(&mut fixture, "lib/protected.txt", "protectedUniqueMarker\n");
    add_source(&mut fixture, "lib/damaged.txt", "damagedUniqueMarker\n");
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let first = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    fixture.manifest.summary.snapshot_id.push_str("-filtered");
    fixture
        .manifest
        .files
        .iter_mut()
        .find(|f| f.path == "lib/protected.txt")
        .unwrap()
        .secret_like = true;
    let damaged = fixture
        .manifest
        .files
        .iter()
        .find(|f| f.path == "lib/damaged.txt")
        .unwrap();
    fs::write(
        fixture.storage.join(&damaged.blob_path),
        b"not the captured content",
    )
    .unwrap();
    let updated = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert_eq!(updated.metadata.build.documents_added, 0);
    assert_eq!(updated.metadata.build.documents_deleted, 2);
    assert_eq!(
        updated.metadata.build.documents_reused,
        first.metadata.ranges.len() - 2
    );
    for query in ["protectedUniqueMarker", "damagedUniqueMarker"] {
        let result = search(
            &fixture.storage,
            &fixture.manifest,
            &updated,
            query,
            None,
            8,
            &budget,
        )
        .unwrap();
        assert!(result.matches.iter().all(|h| !["lib/protected.txt", "lib/damaged.txt"].contains(&h.range.path.as_str())));
        assert!(
            search_with_mode(
                &fixture.storage,
                &fixture.manifest,
                &updated,
                query,
                None,
                8,
                &budget,
                SearchMode::Regex
            )
            .unwrap()
            .matches
            .is_empty()
        );
    }
}

#[test]
fn invalid_document_map_and_legacy_generation_rebuild_without_discarding_layout_reuse() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let first = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let scope = first.generation.parent().unwrap();
    let mut pointer: Pointer =
        serde_json::from_slice(&fs::read(scope.join("current.json")).unwrap()).unwrap();
    let mut metadata: Metadata =
        serde_json::from_slice(&fs::read(first.generation.join("ranges.json")).unwrap()).unwrap();
    metadata.document_keys[0] = "f".repeat(64);
    let bytes = json(&metadata).unwrap();
    fs::write(first.generation.join("ranges.json"), &bytes).unwrap();
    pointer.metadata_sha256 = digest(&bytes);
    fs::write(scope.join("current.json"), json(&pointer).unwrap()).unwrap();
    let rebuilt = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert!(rebuilt.metadata.build.incremental_base_snapshot.is_none());
    assert_eq!(
        rebuilt.metadata.build.documents_added,
        rebuilt.metadata.ranges.len()
    );
    assert_eq!(rebuilt.metadata.build.files_chunked, 0);
    let scope = rebuilt.generation.parent().unwrap();
    let mut pointer: Pointer =
        serde_json::from_slice(&fs::read(scope.join("current.json")).unwrap()).unwrap();
    let mut metadata: Metadata =
        serde_json::from_slice(&fs::read(rebuilt.generation.join("ranges.json")).unwrap()).unwrap();
    metadata.version = CHUNK_VERSION.into();
    let bytes = json(&metadata).unwrap();
    fs::write(rebuilt.generation.join("ranges.json"), &bytes).unwrap();
    pointer.metadata_sha256 = digest(&bytes);
    fs::write(scope.join("current.json"), json(&pointer).unwrap()).unwrap();
    let legacy = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert!(legacy.metadata.build.incremental_base_snapshot.is_none());
    assert_eq!(
        legacy.metadata.build.documents_added,
        legacy.metadata.ranges.len()
    );
    assert_eq!(legacy.metadata.build.files_chunked, 0);
}

#[test]
fn another_project_can_reuse_layouts_but_cannot_supply_index_segments() {
    let mut fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    fixture
        .manifest
        .summary
        .project_id
        .push_str("-another-project");
    let other = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert!(other.metadata.build.incremental_base_snapshot.is_none());
    assert_eq!(other.metadata.build.documents_reused, 0);
    assert_eq!(
        other.metadata.build.documents_added,
        other.metadata.ranges.len()
    );
    assert_eq!(other.metadata.build.files_chunked, 0);
}

#[test]
fn corrupt_managed_paths_cannot_become_external_writer_deletions() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let first = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    let outside = fixture.storage.join("outside-kept");
    fs::write(&outside, b"never delete outside a generation").unwrap();
    let managed = first.generation.join(".managed.json");
    let mut paths: Vec<String> = serde_json::from_slice(&fs::read(&managed).unwrap()).unwrap();
    paths.push("../../../../outside-kept".into());
    fs::write(managed, json(&paths).unwrap()).unwrap();
    let rebuilt = prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert!(rebuilt.metadata.build.incremental_base_snapshot.is_none());
    assert_eq!(
        rebuilt.metadata.build.documents_added,
        rebuilt.metadata.ranges.len()
    );
    assert_eq!(
        fs::read(outside).unwrap(),
        b"never delete outside a generation"
    );
    for invalid in ["", ".", "..", "../outside", "nested/file", "/absolute"] {
        assert!(!safe_component(Path::new(invalid)), "{invalid}");
    }
}

/// Explicit local engine measurement. Only a named disposable copy may be extended.
#[test]
#[ignore = "Requires a disposable Fontdone copy and marker; never runs a model or repository code."]
fn fontdone_one_file_incremental_profile() {
    let storage = PathBuf::from(std::env::var("CODE_ATLAS_FONTDONE_DATA_DIR").unwrap())
        .canonicalize()
        .unwrap();
    assert_eq!(storage.parent(), Some(Path::new("/private/tmp")));
    assert!(
        storage
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("code-atlas-incremental-fontdone-")
    );
    assert_eq!(
        fs::read_to_string(storage.join(".atlas-disposable-index-probe")).unwrap(),
        "Code Atlas disposable indexing fixture\n"
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
    .unwrap()
    .unwrap();
    let cancelled = AtomicBool::new(false);
    let cold_started = Instant::now();
    let first = prepare(&storage, &manifest, Some(&syntax), &Budget::new(&cancelled)).unwrap();
    let cold_ms = cold_started.elapsed().as_millis();
    assert!(!first.reused_generation);
    assert!(first.metadata.build.incremental_base_snapshot.is_none());
    let before = generation_hashes(&first.generation);
    let changed_path = "src/font.rs";
    let mut contents = source(&storage, &manifest, changed_path).unwrap().contents;
    let suffix = "\n// code_atlas_disposable_incremental_probe: append-only comment.\n";
    contents.push_str(suffix);
    let mut updated = manifest.clone();
    updated.summary.parent_snapshot_id = Some(manifest.summary.snapshot_id.clone());
    updated.summary.snapshot_id.push_str("-indexdelta");
    updated.summary.total_bytes += suffix.len() as u64;
    let file = updated
        .files
        .iter_mut()
        .find(|f| f.path == changed_path)
        .unwrap();
    file.sha256 = digest(contents.as_bytes());
    file.size_bytes = contents.len() as u64;
    file.blob_path = format!("objects/sha256/{}/{}", &file.sha256[..2], &file.sha256[2..]);
    let blob = storage.join(&file.blob_path);
    fs::create_dir_all(blob.parent().unwrap()).unwrap();
    write_new(&blob, contents.as_bytes()).unwrap();
    assert_eq!(
        updated
            .files
            .iter()
            .zip(&manifest.files)
            .filter(|(a, b)| a != b)
            .count(),
        1
    );
    let syntax_started = Instant::now();
    let updated_syntax = index::build_snapshot_index(
        &storage,
        &updated,
        "disposable-incremental-profile",
        &cancelled,
        |_| {},
    );
    let syntax_ms = syntax_started.elapsed().as_millis();
    let incremental_started = Instant::now();
    let delta = prepare(
        &storage,
        &updated,
        Some(&updated_syntax),
        &Budget::new(&cancelled),
    )
    .unwrap();
    let incremental_ms = incremental_started.elapsed().as_millis();
    let old_changed = first
        .metadata
        .files
        .iter()
        .find(|f| f.path == changed_path)
        .unwrap();
    let new_changed = delta
        .metadata
        .files
        .iter()
        .find(|f| f.path == changed_path)
        .unwrap();
    assert_eq!(
        delta.metadata.build.incremental_base_snapshot.as_deref(),
        Some(manifest.summary.snapshot_id.as_str())
    );
    assert_eq!(
        delta.metadata.build.files_documents_reused + 1,
        delta.metadata.coverage.files_indexed
    );
    assert_eq!(delta.metadata.build.documents_added, new_changed.chunks);
    assert_eq!(delta.metadata.build.documents_deleted, old_changed.chunks);
    assert_eq!(
        delta.metadata.build.documents_reused + new_changed.chunks,
        delta.metadata.ranges.len()
    );
    assert_eq!(generation_hashes(&first.generation), before);
    let reference_storage = storage.join("cold-reference");
    fs::create_dir(&reference_storage).unwrap();
    copy_blobs(&storage, &reference_storage, &updated);
    let reference_started = Instant::now();
    let reference = prepare(
        &reference_storage,
        &updated,
        Some(&updated_syntax),
        &Budget::new(&cancelled),
    )
    .unwrap();
    let reference_ms = reference_started.elapsed().as_millis();
    assert_eq!(
        reference.metadata.document_keys,
        delta.metadata.document_keys
    );
    let corpus: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../docs/acceptance/fixtures/fontdone-retrieval-ranges.json"
    ))
    .unwrap();
    let expected = corpus["expected"].as_array().unwrap();
    let found = |result: &SearchResult, expected: &serde_json::Value| {
        let path = expected["path"].as_str().unwrap();
        let sha = updated
            .files
            .iter()
            .find(|f| f.path == path)
            .unwrap()
            .sha256
            .as_str();
        result.matches.iter().any(|h| {
            h.range.path == path
                && h.range.source_sha256 == sha
                && h.range.start_byte <= expected["startByte"].as_u64().unwrap() as usize
                && h.range.end_byte >= expected["endByte"].as_u64().unwrap() as usize
        })
    };
    let mut named = vec![];
    for expected in expected {
        let query = expected["query"].as_str().unwrap();
        let path = expected["path"].as_str().unwrap();
        let mut outcomes = vec![];
        for scope in [None, Some(path)] {
            let a = search(
                &storage,
                &updated,
                &delta,
                query,
                scope,
                32,
                &Budget::new(&cancelled),
            )
            .unwrap();
            let b = search(
                &reference_storage,
                &updated,
                &reference,
                query,
                scope,
                32,
                &Budget::new(&cancelled),
            )
            .unwrap();
            assert_eq!(
                found(&a, expected),
                found(&b, expected),
                "{} {scope:?}",
                expected["id"]
            );
            outcomes.push(found(&a, expected));
        }
        assert!(outcomes[1], "missing scoped {}", expected["id"]);
        named.push(serde_json::json!({"id":expected["id"],"globalFound":outcomes[0],"scopedFound":outcomes[1]}));
    }
    let mut broad = vec![];
    for query in [
        "font bytes rendered glyph bitmap entry parsing outline hinting rasterization",
        "font buffer glyph image format outline hinting pixel coverage",
    ] {
        let a = search(
            &storage,
            &updated,
            &delta,
            query,
            None,
            32,
            &Budget::new(&cancelled),
        )
        .unwrap();
        let b = search(
            &reference_storage,
            &updated,
            &reference,
            query,
            None,
            32,
            &Budget::new(&cancelled),
        )
        .unwrap();
        let recovered = expected
            .iter()
            .filter(|e| found(&a, e))
            .map(|e| e["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        let fresh = expected
            .iter()
            .filter(|e| found(&b, e))
            .map(|e| e["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(recovered, fresh);
        broad.push(serde_json::json!({"query":query,"retrieved":recovered,"expectedCount":expected.len(),"limited":a.limited}));
    }
    println!(
        "FONTDONE_INCREMENTAL {}",
        serde_json::json!({"baseSnapshot":manifest.summary.snapshot_id,"changedPath":changed_path,"appendBytes":suffix.len(),"coldMs":cold_ms,"syntaxRebuildMs":syntax_ms,"incrementalMs":incremental_ms,"freshReferenceMs":reference_ms,"oldGenerationUnchanged":true,"sameCurrentDocumentKeysAsFresh":true,"coverage":delta.metadata.coverage,"coldBuild":first.metadata.build,"incrementalBuild":delta.metadata.build,"freshBuild":reference.metadata.build,"named":named,"broad":broad})
    );
}
