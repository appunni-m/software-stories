use super::*;
use crate::chapter_generation::tests::Fixture;
use crate::evidence_ledger::attempts::tests::{candidate, record_path, setup};
use serde_json::json;

fn saved(f: &Fixture) -> (Ledger, PathBuf, Vec<u8>) {
    let l = setup(f);
    l.save_candidate(
        &"d".repeat(64),
        &candidate(false),
        1,
        Some("Check the cited identifier."),
    )
    .unwrap();
    let path = record_path(f, &l);
    let bytes = fs::read(&path).unwrap();
    (l, path, bytes)
}

#[test]
fn inventory_is_metadata_only_and_explicit_review_retains_both_inert_candidates() {
    let f = Fixture::new();
    let (l, path, _) = saved(&f);
    l.save_candidate(&"d".repeat(64), &candidate(true), 2, None)
        .unwrap();
    let ledger_before = fs::read(&l.path).unwrap();
    let bytes_before = fs::read(&path).unwrap();
    let result = list(&f.storage).unwrap();
    assert!(!result.limited && !result.busy);
    assert_eq!(result.operation_limit, 8);
    assert_eq!(result.entries.len(), 1);
    let entry = &result.entries[0];
    assert!(entry.available);
    assert_eq!(entry.candidate_count, 2);
    assert_eq!(
        entry.project_id.as_deref(),
        Some(l.record.binding.project_id.as_str())
    );
    let metadata = serde_json::to_string(&result).unwrap();
    for private in [
        "responseText",
        "owned-thread",
        "threadId",
        "executable",
        "account",
        "gpt-6-luna",
        "greeting",
    ] {
        assert!(!metadata.contains(private), "{private}");
    }
    let reviewed = review(
        &f.storage,
        &entry.id,
        entry.review_token.as_deref().unwrap(),
    )
    .unwrap();
    assert_eq!(reviewed.attempts.len(), 2);
    assert_eq!(
        reviewed.attempts[0].response_text,
        candidate(false).response_text
    );
    assert_eq!(
        reviewed.attempts[0].rejection.as_deref(),
        Some("Check the cited identifier.")
    );
    assert_eq!(
        reviewed.attempts[1].response_text,
        candidate(true).response_text
    );
    assert_eq!(fs::read(&l.path).unwrap(), ledger_before);
    assert_eq!(fs::read(&path).unwrap(), bytes_before);
    assert!(f.storage.join("rejected-explanations/store.lock").is_file());
}

#[test]
fn stale_review_and_move_refuse_a_changed_candidate_without_calling_the_move() {
    let f = Fixture::new();
    let (l, path, _) = saved(&f);
    let entry = list(&f.storage).unwrap().entries.remove(0);
    l.save_candidate(&"d".repeat(64), &candidate(true), 2, None)
        .unwrap();
    let updated = fs::read(&path).unwrap();
    let token = entry.review_token.unwrap();
    assert!(review(&f.storage, &entry.id, &token).is_err());
    assert!(
        move_with(&f.storage, &entry.id, &token, |_| panic!(
            "must not move stale record"
        ))
        .is_err()
    );
    assert_eq!(fs::read(path).unwrap(), updated);
}

#[test]
fn failed_move_preserves_bytes_and_successful_fixture_move_frees_capacity_without_eviction() {
    let f = Fixture::new();
    let mut ledgers = Vec::new();
    for _ in 0..MAX_OPERATIONS {
        ledgers.push(saved(&f).0);
    }
    let extra = setup(&f);
    assert!(
        extra
            .save_candidate(&"d".repeat(64), &candidate(false), 1, None)
            .unwrap_err()
            .contains("Notifications")
    );
    let inventory = list(&f.storage).unwrap();
    let entry = &inventory.entries[0];
    let path = directory(&f.storage).unwrap().join(&entry.id);
    let before = fs::read(&path).unwrap();
    let journal_before = ledgers
        .iter()
        .map(|l| fs::read(&l.path).unwrap())
        .collect::<Vec<_>>();
    assert!(
        move_with(
            &f.storage,
            &entry.id,
            entry.review_token.as_deref().unwrap(),
            |_| Err("injected OS failure".into())
        )
        .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    let fixture_trash = f.storage.join("fixture-owned-trash");
    fs::create_dir(&fixture_trash).unwrap();
    move_with(
        &f.storage,
        &entry.id,
        entry.review_token.as_deref().unwrap(),
        |p| fs::rename(p, fixture_trash.join(&entry.id)).map_err(|e| e.to_string()),
    )
    .unwrap();
    assert!(!path.exists());
    assert_eq!(fs::read(fixture_trash.join(&entry.id)).unwrap(), before);
    assert_eq!(list(&f.storage).unwrap().entries.len(), MAX_OPERATIONS - 1);
    extra
        .save_candidate(&"d".repeat(64), &candidate(false), 1, None)
        .unwrap();
    assert_eq!(list(&f.storage).unwrap().entries.len(), MAX_OPERATIONS);
    for (l, before) in ledgers.iter().zip(journal_before) {
        assert_eq!(fs::read(&l.path).unwrap(), before);
    }
}

#[test]
fn damaged_and_oversized_regular_records_can_be_moved_but_never_reviewed_as_answers() {
    let f = Fixture::new();
    let (l, path, original) = saved(&f);
    for bytes in [b"{broken".to_vec(), vec![b'x'; MAX_RECORD + 1], {
        let mut v: Value = serde_json::from_slice(&original).unwrap();
        v["evidenceSha256"] = json!("f".repeat(64));
        serde_json::to_vec(&v).unwrap()
    }] {
        fs::write(&path, &bytes).unwrap();
        let entry = list(&f.storage).unwrap().entries.remove(0);
        assert!(!entry.available);
        assert_eq!(entry.project_id, None);
        let token = entry.review_token.unwrap();
        assert!(review(&f.storage, &entry.id, &token).is_err());
        let dest = f.storage.join("fixture-owned-trash-record");
        move_with(&f.storage, &entry.id, &token, |p| {
            fs::rename(p, &dest).map_err(|e| e.to_string())
        })
        .unwrap();
        assert_eq!(fs::read(&dest).unwrap(), bytes);
        fs::rename(dest, &path).unwrap();
    }
    drop(l);
}

#[test]
fn traversal_lock_and_nonregular_entries_never_reach_a_move_callback() {
    let f = Fixture::new();
    let (_, _, _) = saved(&f);
    for name in [
        "",
        "..",
        ".",
        "../outside.json",
        "/tmp/outside.json",
        "a/b",
        "store.lock",
        "bad\nname",
    ] {
        assert!(
            move_with(&f.storage, name, "token", |_| panic!("unsafe path")).is_err(),
            "{name:?}"
        );
    }
    let dir = directory(&f.storage).unwrap();
    fs::create_dir(dir.join("folder")).unwrap();
    assert!(move_with(&f.storage, "folder", "token", |_| panic!("directory")).is_err());
    #[cfg(unix)]
    {
        let target = f.storage.join("unrelated-file");
        fs::write(&target, b"unchanged").unwrap();
        std::os::unix::fs::symlink(&target, dir.join("link")).unwrap();
        assert!(move_with(&f.storage, "link", "token", |_| panic!("link")).is_err());
        assert_eq!(fs::read(target).unwrap(), b"unchanged");
        let inventory = list(&f.storage).unwrap();
        assert!(
            inventory
                .entries
                .iter()
                .filter(|e| e.id == "folder" || e.id == "link")
                .all(|e| e.review_token.is_none() && !e.available)
        );
    }
}

#[test]
fn cross_process_lease_blocks_read_review_move_and_save_until_released() {
    let f = Fixture::new();
    let (l, path, bytes) = saved(&f);
    let entry = list(&f.storage).unwrap().entries.remove(0);
    let guard = lease(&directory(&f.storage).unwrap()).unwrap();
    assert!(list(&f.storage).is_err());
    assert!(
        review(
            &f.storage,
            &entry.id,
            entry.review_token.as_deref().unwrap()
        )
        .is_err()
    );
    assert!(
        move_with(
            &f.storage,
            &entry.id,
            entry.review_token.as_deref().unwrap(),
            |_| panic!("leased")
        )
        .is_err()
    );
    assert!(
        l.save_candidate(&"d".repeat(64), &candidate(true), 2, None)
            .is_err()
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
    drop(guard);
    assert_eq!(list(&f.storage).unwrap().entries.len(), 1);
    l.save_candidate(&"d".repeat(64), &candidate(true), 2, None)
        .unwrap();
}

#[test]
fn directory_inventory_is_bounded_and_preserves_all_unknown_files() {
    let f = Fixture::new();
    let dir = directory(&f.storage).unwrap();
    for i in 0..MAX_ENTRIES + 1 {
        fs::write(dir.join(format!("unknown-{i}.json")), b"unverified").unwrap();
    }
    let inventory = list(&f.storage).unwrap();
    assert!(inventory.limited);
    assert_eq!(inventory.entries.len(), MAX_ENTRIES);
    assert!(
        inventory
            .entries
            .iter()
            .all(|e| !e.available && e.review_token.is_some())
    );
    for i in 0..MAX_ENTRIES + 1 {
        assert_eq!(
            fs::read(dir.join(format!("unknown-{i}.json"))).unwrap(),
            b"unverified"
        );
    }
}

#[test]
fn question_labels_come_only_from_the_matching_saved_conversation_and_never_model_output() {
    let f = Fixture::new();
    let (l, path, before) = saved(&f);
    let project = &l.record.binding.project_id;
    let snapshot = &l.record.binding.snapshot_id;
    let question = "Where does the actual request go? <b>plain text</b>";
    let state = json!({"version":1,"projectId":project,"snapshotId":snapshot,"currentId":"root","file":null,
        "explorations":[{"id":"root","parentId":null,"originTurnId":null,"title":"Repository","view":"guide",
        "turns":[{"id":"question","question":question,"context":[],"status":"failed","runIds":[l.record.binding.operation_id]}]}]});
    let dir = f.storage.join("workspace-conversations");
    fs::create_dir_all(&dir).unwrap();
    let name = context::digest(&serde_json::to_vec(&(project, snapshot)).unwrap());
    let conversation = dir.join(format!("{name}.json"));
    fs::write(
        &conversation,
        serde_json::to_vec(&json!({"schemaVersion":1,"revision":1,"state":state})).unwrap(),
    )
    .unwrap();
    let journal = fs::read(&conversation).unwrap();
    let inventory = list(&f.storage).unwrap();
    let entry = &inventory.entries[0];
    assert_eq!(entry.question.as_deref(), Some(question));
    assert_eq!(
        review(
            &f.storage,
            &entry.id,
            entry.review_token.as_deref().unwrap()
        )
        .unwrap()
        .entry
        .question
        .as_deref(),
        Some(question)
    );
    assert_eq!(fs::read(conversation).unwrap(), journal);
    assert_eq!(fs::read(path).unwrap(), before);
}
