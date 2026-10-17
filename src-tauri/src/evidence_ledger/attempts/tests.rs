use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};
use serde_json::json;

fn setup(f: &Fixture) -> Ledger {
    let runtime = CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 0,
    };
    let grant = workspace::save_retrieval_grant(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
        &runtime,
    )
    .unwrap();
    let mut ledger = Ledger::new(
        &f.storage,
        &Uuid::new_v4().to_string(),
        &f.manifest,
        &grant,
        &runtime,
        "gpt-6-luna",
        "max",
    )
    .unwrap();
    let token = ledger
        .begin_call(
            Tool::Read,
            &"c".repeat(64),
            &f.manifest,
            &runtime,
            &AtomicBool::new(false),
        )
        .unwrap();
    let source = f
        .manifest
        .files
        .iter()
        .find(|s| s.path == SIMPLE_PATH)
        .unwrap();
    ledger
        .deliver(
            token,
            Context::Read {},
            &[RangeRequest {
                path: SIMPLE_PATH.into(),
                source_sha256: source.sha256.clone(),
                start_byte: 0,
                end_byte: SIMPLE_SOURCE.len(),
            }],
            &f.manifest,
            &runtime,
            &AtomicBool::new(false),
            |_| Ok(()),
        )
        .unwrap();
    ledger
}

fn candidate(corrected: bool) -> codex_agent::CodexRunResult {
    codex_agent::CodexRunResult {
        thread_id: "owned-thread".into(), turn_id: if corrected {"second"} else {"first"}.into(),
        model: "gpt-6-luna".into(), effort: "max".into(),
        response_text: json!({"sections":[{"claims":[{
            "prose":if corrected {"greeting formats a message."} else {"The helper formats a message."},
            "identifiers":["greeting"],"evidence":[{"quote":"export function greeting(name: string)"}]
        }]}]}).to_string(),
    }
}

fn record_path(f: &Fixture, l: &Ledger) -> PathBuf {
    f.storage
        .join("rejected-explanations")
        .join(format!("{}.json", l.record.binding.operation_id))
}

#[test]
fn initial_and_corrected_candidates_survive_restart_without_polluting_evidence() {
    let f = Fixture::new();
    let l = setup(&f);
    let operation = l.record.binding.operation_id.clone();
    let fingerprint = "d".repeat(64);
    let initial = candidate(false);
    let corrected = candidate(true);
    l.save_candidate(&fingerprint, &initial, 1, None).unwrap();
    l.save_candidate(&fingerprint, &corrected, 2, None).unwrap();
    assert!(
        !fs::read_to_string(&l.path)
            .unwrap()
            .contains("The helper formats")
    );
    assert!(
        !fs::read_to_string(&l.path)
            .unwrap()
            .contains("export function greeting")
    );
    drop(l);
    let saved = inspect(&f.storage, &operation).unwrap();
    assert_eq!(saved.request_fingerprint, fingerprint);
    assert_eq!(saved.candidates.len(), 2);
    assert_eq!(saved.candidates[0].response_text, initial.response_text);
    assert_eq!(saved.candidates[1].response_text, corrected.response_text);
    assert_eq!(
        saved.candidates[0].alignment_issues[0]["location"],
        "/sections/0/claims/0"
    );
    assert_eq!(
        saved.candidates[0].alignment_issues[0]["missingFromProse"],
        json!(["greeting"])
    );
    assert!(saved.candidates[1].alignment_issues.is_empty());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let directory = f.storage.join("rejected-explanations");
        assert_eq!(
            fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(directory.join(format!("{operation}.json")))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn candidate_identity_order_and_request_cannot_be_replaced() {
    let f = Fixture::new();
    let l = setup(&f);
    let fingerprint = "d".repeat(64);
    assert!(
        l.save_candidate(&fingerprint, &candidate(true), 2, None)
            .is_err()
    );
    l.save_candidate(&fingerprint, &candidate(false), 1, None)
        .unwrap();
    let original = fs::read(record_path(&f, &l)).unwrap();
    for fault in [
        "text",
        "thread",
        "turn",
        "model",
        "effort",
        "size",
        "fingerprint",
        "ordinal",
    ] {
        let mut changed = candidate(false);
        let mut fp = fingerprint.clone();
        let mut ordinal = 1;
        match fault {
            "text" => changed.response_text.push(' '),
            "thread" => changed.thread_id = "other-thread".into(),
            "turn" => changed.turn_id = "other-turn".into(),
            "model" => changed.model = "other-model".into(),
            "effort" => changed.effort = "high".into(),
            "size" => {
                changed.response_text = "x".repeat(chapters::MAX_GENERATED_RESPONSE_BYTES + 1)
            }
            "fingerprint" => fp = "e".repeat(64),
            "ordinal" => ordinal = 3,
            _ => unreachable!(),
        }
        assert!(
            l.save_candidate(&fp, &changed, ordinal, None).is_err(),
            "{fault}"
        );
        assert_eq!(fs::read(record_path(&f, &l)).unwrap(), original, "{fault}");
    }
    let mut second = candidate(true);
    second.thread_id = "other-thread".into();
    assert!(l.save_candidate(&fingerprint, &second, 2, None).is_err());
    second.thread_id = "owned-thread".into();
    second.turn_id = "first".into();
    assert!(l.save_candidate(&fingerprint, &second, 2, None).is_err());
    l.save_candidate(
        &fingerprint,
        &candidate(false),
        1,
        Some("Independent source validation rejected this candidate."),
    )
    .unwrap();
    assert!(
        l.save_candidate(
            &fingerprint,
            &candidate(false),
            1,
            Some("A different reason.")
        )
        .is_err()
    );
    assert!(
        inspect(&f.storage, &l.record.binding.operation_id)
            .unwrap()
            .candidates[0]
            .rejection
            .is_some()
    );
}

#[test]
fn damaged_candidate_records_and_evidence_mismatches_are_preserved() {
    let f = Fixture::new();
    let l = setup(&f);
    let fingerprint = "d".repeat(64);
    l.save_candidate(&fingerprint, &candidate(false), 1, None)
        .unwrap();
    let path = record_path(&f, &l);
    let original = fs::read(&path).unwrap();
    for fault in [
        "response",
        "diagnostics",
        "evidence",
        "binding",
        "json",
        "size",
    ] {
        let mut value: Value = serde_json::from_slice(&original).unwrap();
        match fault {
            "response" => value["candidates"][0]["responseText"] = json!("changed"),
            "diagnostics" => value["candidates"][0]["alignmentIssues"] = json!([]),
            "evidence" => value["evidenceSha256"] = json!("f".repeat(64)),
            "binding" => value["binding"]["model"] = json!("other-model"),
            _ => (),
        }
        let bytes = match fault {
            "json" => b"{".to_vec(),
            "size" => vec![b'x'; MAX_RECORD + 1],
            _ => serde_json::to_vec(&value).unwrap(),
        };
        fs::write(&path, &bytes).unwrap();
        assert!(
            inspect(&f.storage, &l.record.binding.operation_id).is_err(),
            "{fault}"
        );
        assert!(
            l.save_candidate(&fingerprint, &candidate(true), 2, None)
                .is_err(),
            "{fault}"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::write(&path, &original).unwrap();
    }
}

#[test]
fn capacity_preserves_all_failed_operations_and_reserves_their_second_attempt() {
    let f = Fixture::new();
    let fingerprint = "d".repeat(64);
    let mut ledgers = vec![];
    for _ in 0..MAX_OPERATIONS {
        let l = setup(&f);
        l.save_candidate(&fingerprint, &candidate(false), 1, None)
            .unwrap();
        ledgers.push(l);
    }
    let extra = setup(&f);
    assert!(
        extra
            .save_candidate(&fingerprint, &candidate(false), 1, None)
            .unwrap_err()
            .contains("full")
    );
    for l in ledgers {
        l.save_candidate(&fingerprint, &candidate(true), 2, None)
            .unwrap();
        assert_eq!(
            inspect(&f.storage, &l.record.binding.operation_id)
                .unwrap()
                .candidates
                .len(),
            2
        );
    }
}

#[test]
#[cfg(unix)]
fn symlink_directory_or_record_never_replaces_an_external_file() {
    use std::os::unix::fs::symlink;
    for directory_link in [true, false] {
        let f = Fixture::new();
        let l = setup(&f);
        let outside = f.storage.join("external");
        fs::create_dir(&outside).unwrap();
        let sentinel = outside.join("keep");
        fs::write(&sentinel, "keep unchanged").unwrap();
        let dir = f.storage.join("rejected-explanations");
        if directory_link {
            symlink(&outside, &dir).unwrap();
        } else {
            fs::create_dir(&dir).unwrap();
            symlink(&sentinel, record_path(&f, &l)).unwrap();
        }
        assert!(
            l.save_candidate(&"d".repeat(64), &candidate(false), 1, None)
                .is_err()
        );
        assert_eq!(fs::read_to_string(sentinel).unwrap(), "keep unchanged");
    }
}
