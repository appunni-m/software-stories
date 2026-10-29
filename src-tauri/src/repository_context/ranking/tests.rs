use super::*;
use tantivy::indexer::NoMergePolicy;

fn tied_index(same_path: bool, count: usize) -> (Index, Vec<String>) {
    let index = Index::create_in_ram(schema());
    let schema = index.schema();
    let key = schema.get_field("range_key").unwrap();
    let path = schema.get_field("source_path").unwrap();
    let start = schema.get_field("start_byte").unwrap();
    let end = schema.get_field("end_byte").unwrap();
    let body = schema.get_field("body").unwrap();
    let exact = schema.get_field("exact").unwrap();
    let declaration = schema.get_field("declaration").unwrap();
    let mut writer = index.writer_with_num_threads(1, 30_000_000).unwrap();
    writer.set_merge_policy(Box::new(NoMergePolicy));
    let mut expected = vec![];
    for ordinal in (0..count).rev() {
        let file = if same_path {
            "lib/one.rs".to_owned()
        } else {
            let prefix = ["Z", "a", "é", "字"][ordinal % 4];
            format!("lib/{prefix}-{ordinal:04}.rs")
        };
        let offset = if same_path { ordinal as u64 * 8192 } else { 0 };
        let identity = format!("{file}:{offset:010}");
        expected.push(identity.clone());
        let mut doc = TantivyDocument::default();
        doc.add_text(key, identity);
        doc.add_text(path, file);
        doc.add_u64(start, offset);
        doc.add_u64(end, offset + 1);
        doc.add_text(body, "render glyph bitmap");
        doc.add_text(exact, "renderglyphbitmap");
        doc.add_text(declaration, "renderglyphbitmap");
        writer.add_document(doc).unwrap();
        if ordinal % 100 == 0 {
            writer.commit().unwrap();
        }
    }
    writer.commit().unwrap();
    writer.wait_merging_threads().unwrap();
    expected.sort();
    expected.truncate(MAX_CANDIDATES);
    (index, expected)
}

#[test]
fn equal_scores_use_path_and_byte_before_each_candidate_cap() {
    for same_path in [false, true] {
        let (index, expected) = tied_index(same_path, MAX_CANDIDATES + 44);
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        assert!(searcher.segment_readers().len() > 1);
        for (field_name, word) in [
            ("body", "render"),
            ("exact", "renderglyphbitmap"),
            ("declaration", "renderglyphbitmap"),
        ] {
            let query = term_query(field(&index, field_name).unwrap(), word);
            let (found, count) = candidates(&searcher, &query).unwrap();
            assert_eq!(count, MAX_CANDIDATES + 44);
            assert_eq!(found.len(), MAX_CANDIDATES);
            assert!(found.iter().all(|h| h.0 == found[0].0));
            let range_key = field(&index, "range_key").unwrap();
            let mut actual = found
                .iter()
                .map(|(_, address)| {
                    searcher
                        .doc::<TantivyDocument>(*address)
                        .unwrap()
                        .get_first(range_key)
                        .unwrap()
                        .as_str()
                        .unwrap()
                        .to_owned()
                })
                .collect::<Vec<_>>();
            // Test membership as well as ordering: sorting after the cap cannot
            // recover the lower paths/offsets already discarded by the collector.
            let ordered = actual.clone();
            actual.sort();
            assert_eq!(actual, expected, "{field_name}, same_path={same_path}");
            assert_eq!(
                ordered, expected,
                "ordered {field_name}, same_path={same_path}"
            );
        }
    }
}

#[test]
fn empty_and_both_sides_of_candidate_boundary_keep_exact_counts() {
    for count in [0, MAX_CANDIDATES - 1, MAX_CANDIDATES, MAX_CANDIDATES + 1] {
        let (index, _) = tied_index(false, count);
        let reader = index.reader().unwrap();
        let searcher = reader.searcher();
        let query = term_query(field(&index, "body").unwrap(), "render");
        let (found, total) = candidates(&searcher, &query).unwrap();
        assert_eq!(total, count);
        assert_eq!(found.len(), count.min(MAX_CANDIDATES));
    }
}

#[test]
fn a_higher_score_beats_earlier_paths_and_missing_order_fields_fail() {
    let (index, _) = tied_index(false, MAX_CANDIDATES + 44);
    let mut writer = index.writer_with_num_threads(1, 30_000_000).unwrap();
    let mut doc = TantivyDocument::default();
    doc.add_text(field(&index, "range_key").unwrap(), "winner");
    doc.add_text(field(&index, "source_path").unwrap(), "z/winner.rs");
    doc.add_u64(field(&index, "start_byte").unwrap(), 0);
    doc.add_u64(field(&index, "end_byte").unwrap(), 1);
    doc.add_text(field(&index, "body").unwrap(), "render winner");
    writer.add_document(doc).unwrap();
    writer.commit().unwrap();
    writer.wait_merging_threads().unwrap();
    let reader = index.reader().unwrap();
    let searcher = reader.searcher();
    let query = BooleanQuery::union(vec![
        term_query(field(&index, "body").unwrap(), "render"),
        Box::new(BoostQuery::new(
            term_query(field(&index, "body").unwrap(), "winner"),
            50.0,
        )),
    ]);
    let (hits, count) = candidates(&searcher, &query).unwrap();
    assert_eq!(count, MAX_CANDIDATES + 45);
    assert_eq!(hits.len(), MAX_CANDIDATES);
    assert!(hits[0].0 > hits[1].0);
    assert_eq!(
        searcher
            .doc::<TantivyDocument>(hits[0].1)
            .unwrap()
            .get_first(field(&index, "range_key").unwrap())
            .unwrap()
            .as_str(),
        Some("winner")
    );

    let invalid = Index::create_in_ram(schema());
    let mut writer = invalid.writer_with_num_threads(1, 30_000_000).unwrap();
    let mut doc = TantivyDocument::default();
    doc.add_text(field(&invalid, "body").unwrap(), "render");
    writer.add_document(doc).unwrap();
    writer.commit().unwrap();
    writer.wait_merging_threads().unwrap();
    let reader = invalid.reader().unwrap();
    assert!(
        candidates(
            &reader.searcher(),
            &term_query(field(&invalid, "body").unwrap(), "render")
        )
        .is_err()
    );
}

#[test]
fn captured_search_ties_match_after_incremental_update_and_reopen() {
    use crate::{chapter_generation::tests::Fixture, repository_context::tests::add_source};
    let mut fixture = Fixture::new();
    let source = "pub fn renderGlyphBitmap() {}\n";
    for ordinal in 0..MAX_CANDIDATES + 44 {
        add_source(&mut fixture, &format!("lib/m{ordinal:04}.rs"), source);
    }
    let cancelled = AtomicBool::new(false);
    let syntax = index::build_snapshot_index(
        &fixture.storage,
        &fixture.manifest,
        "ties-base",
        &cancelled,
        |_| {},
    );
    let first = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&syntax),
        &Budget::new(&cancelled),
    )
    .unwrap();
    fixture.manifest.summary.snapshot_id.push_str("-ties");
    for path in ["lib/a0000.rs", "lib/a0001.rs"] {
        add_source(&mut fixture, path, source);
    }
    let syntax = index::build_snapshot_index(
        &fixture.storage,
        &fixture.manifest,
        "ties-delta",
        &cancelled,
        |_| {},
    );
    let updated = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&syntax),
        &Budget::new(&cancelled),
    )
    .unwrap();
    assert_eq!(updated.metadata.build.documents_added, 2);
    assert!(updated.metadata.build.documents_reused > MAX_CANDIDATES);
    assert_eq!(
        updated.metadata.build.incremental_base_snapshot.as_deref(),
        Some(first.metadata.snapshot_id.as_str())
    );

    let fresh_storage = Fixture::new();
    for file in &fixture.manifest.files {
        let from = fixture.storage.join(&file.blob_path);
        let to = fresh_storage.storage.join(&file.blob_path);
        if from.is_file() && !to.exists() {
            fs::create_dir_all(to.parent().unwrap()).unwrap();
            fs::copy(from, to).unwrap();
        }
    }
    let fresh = prepare(
        &fresh_storage.storage,
        &fixture.manifest,
        Some(&syntax),
        &Budget::new(&cancelled),
    )
    .unwrap();
    assert!(fresh.metadata.build.incremental_base_snapshot.is_none());
    let reopened = prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&syntax),
        &Budget::new(&cancelled),
    )
    .unwrap();
    assert!(reopened.reused_generation);
    let mut expected = fixture
        .manifest
        .files
        .iter()
        .filter(|f| f.path.starts_with("lib/"))
        .map(|f| f.path.clone())
        .collect::<Vec<_>>();
    expected.sort();
    expected.truncate(32);
    for (storage, prepared) in [
        (&fixture.storage, &updated),
        (&fixture.storage, &reopened),
        (&fresh_storage.storage, &fresh),
    ] {
        for query in ["render bitmap", "renderGlyphBitmap"] {
            let result = search(
                storage,
                &fixture.manifest,
                prepared,
                query,
                Some("lib"),
                32,
                &Budget::new(&cancelled),
            )
            .unwrap();
            assert!(result.limited);
            assert_eq!(
                result
                    .matches
                    .iter()
                    .map(|h| h.range.path.clone())
                    .collect::<Vec<_>>(),
                expected,
                "{query}"
            );
            assert!(
                result
                    .matches
                    .iter()
                    .all(|h| h.range.snapshot_id == fixture.manifest.summary.snapshot_id)
            );
            if query == "renderGlyphBitmap" {
                assert!(
                    result
                        .matches
                        .iter()
                        .all(|h| h.exact_identifier && h.declaration_match)
                );
            }
        }
    }
}

#[test]
fn missing_wrong_or_multivalued_sort_identity_rebuilds_the_derived_index() {
    use crate::chapter_generation::tests::Fixture;
    for fault in [
        "missingPath",
        "wrongPath",
        "multiplePaths",
        "missingStart",
        "wrongStart",
        "multipleStarts",
    ] {
        let fixture = Fixture::new();
        let cancelled = AtomicBool::new(false);
        let first = prepare(
            &fixture.storage,
            &fixture.manifest,
            None,
            &Budget::new(&cancelled),
        )
        .unwrap();
        let damaged = Index::open_in_dir(&first.generation).unwrap();
        let mut writer = damaged.writer_with_num_threads(1, 30_000_000).unwrap();
        writer.delete_all_documents().unwrap();
        for (key, range) in first
            .metadata
            .document_keys
            .iter()
            .zip(&first.metadata.ranges)
        {
            let mut doc = TantivyDocument::default();
            doc.add_text(field(&damaged, "range_key").unwrap(), key);
            if fault != "missingPath" {
                doc.add_text(
                    field(&damaged, "source_path").unwrap(),
                    if fault == "wrongPath" {
                        "wrong.rs"
                    } else {
                        &range.path
                    },
                );
            }
            if fault == "multiplePaths" {
                doc.add_text(field(&damaged, "source_path").unwrap(), "another.rs");
            }
            if fault != "missingStart" {
                doc.add_u64(
                    field(&damaged, "start_byte").unwrap(),
                    range.start_byte as u64 + u64::from(fault == "wrongStart"),
                );
            }
            if fault == "multipleStarts" {
                doc.add_u64(
                    field(&damaged, "start_byte").unwrap(),
                    range.start_byte as u64 + 1,
                );
            }
            doc.add_text(field(&damaged, "body").unwrap(), "render");
            writer.add_document(doc).unwrap();
        }
        writer.commit().unwrap();
        writer.wait_merging_threads().unwrap();
        let rebuilt = prepare(
            &fixture.storage,
            &fixture.manifest,
            None,
            &Budget::new(&cancelled),
        )
        .unwrap();
        assert!(!rebuilt.reused_generation, "{fault}");
        assert!(
            rebuilt.metadata.build.incremental_base_snapshot.is_none(),
            "{fault}"
        );
        assert_eq!(
            rebuilt.metadata.build.documents_added,
            rebuilt.metadata.ranges.len(),
            "{fault}"
        );
        assert_eq!(
            rebuilt.metadata.build.files_chunked, 0,
            "validated layouts still reusable: {fault}"
        );
    }
}
