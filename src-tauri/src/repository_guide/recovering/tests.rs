use super::*;
use crate::chapter_generation::tests::Fixture;
use crate::repository_guide::tests::{attempt, record};

mod saved_failure;

#[test]
fn failed_build_retires_its_owner_without_changing_attempts_or_the_saved_outline() {
    let f = Fixture::new();
    let mut original = record(&f);
    let failed = attempt(&original, Outcome::Failed);
    original.active_run_id = Some(failed.run_id.clone());
    original.units[0].attempts.push(failed);
    store::create(&f.storage, &original).unwrap();
    let units = serde_json::to_value(&original.units).unwrap();
    let plan = original.plan.clone();
    let manager = codex_agent::CodexAgentManager::default();
    let saved = settle_failed_build(&f.storage, &original.id, &manager).unwrap();
    assert_eq!(saved.status, Status::Partial);
    assert!(saved.active_run_id.is_none());
    assert_eq!(saved.revision, original.revision + 1);
    assert_eq!(saved.plan, plan);
    assert_eq!(serde_json::to_value(&saved.units).unwrap(), units);
    assert_eq!(saved.units[0].attempts.len(), 1);
    assert!(saved.units[1].attempts.is_empty());
    let bytes = record_bytes(&f, &saved);
    let again = settle_failed_build(&f.storage, &saved.id, &manager).unwrap();
    assert_eq!(again.revision, saved.revision);
    assert_eq!(record_bytes(&f, &saved), bytes);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn failed_build_cannot_retire_an_operation_that_still_has_a_native_worker() {
    let f = Fixture::new();
    let original = pending(&f);
    let manager = codex_agent::CodexAgentManager::default();
    let run = original.active_run_id.as_ref().unwrap();
    let worker = manager.hold_answer_worker(run).unwrap();
    let bytes = record_bytes(&f, &original);
    assert!(settle_failed_build(&f.storage, &original.id, &manager).is_err());
    assert_eq!(record_bytes(&f, &original), bytes);
    drop(worker);
    let retired = settle_failed_build(&f.storage, &original.id, &manager).unwrap();
    assert_eq!(retired.status, Status::Partial);
    assert!(retired.active_run_id.is_none());
    assert_eq!(retired.units[0].attempts.len(), 1);
    assert_eq!(retired.units[0].attempts[0].summary.as_ref().unwrap().outcome, Outcome::Interrupted);
}

#[test]
fn a_terminal_receipt_does_not_allow_failure_cleanup_before_worker_retirement() {
    let f = Fixture::new();
    let mut original = record(&f);
    let failed = attempt(&original, Outcome::Failed);
    original.active_run_id = Some(failed.run_id.clone());
    original.units[0].attempts.push(failed);
    store::create(&f.storage, &original).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    let worker = manager.hold_answer_worker(original.active_run_id.as_ref().unwrap()).unwrap();
    let bytes = record_bytes(&f, &original);
    assert!(settle_failed_build(&f.storage, &original.id, &manager).is_err());
    assert!(reconcile(&f.storage, &original, &manager).is_err());
    assert_eq!(record_bytes(&f, &original), bytes);
    drop(worker);
    let retired = settle_failed_build(&f.storage, &original.id, &manager).unwrap();
    assert_eq!(retired.status, Status::Partial);
    assert_eq!(retired.units[0].attempts.len(), 1);
    assert!(retired.active_run_id.is_none());
}

#[test]
fn failure_cleanup_keeps_stop_and_noop_terminal_bytes_at_the_revision_limit() {
    for status in [Status::Stopped, Status::Partial, Status::Interrupted, Status::Failed] {
        let f = Fixture::new();
        let mut original = record(&f);
        original.status = status.clone();
        original.revision = u64::MAX;
        store::create(&f.storage, &original).unwrap();
        let bytes = record_bytes(&f, &original);
        let saved = settle_failed_build(&f.storage, &original.id, &codex_agent::CodexAgentManager::default()).unwrap();
        assert_eq!(saved.status, status);
        assert_eq!(saved.revision, u64::MAX);
        assert_eq!(record_bytes(&f, &original), bytes);
    }
    let f = Fixture::new();
    let mut original = pending(&f);
    original.status = Status::Stopped;
    store::update(&f.storage, &original.id, |r| { r.status = Status::Stopped; Ok(()) }).unwrap();
    let saved = settle_failed_build(&f.storage, &original.id, &codex_agent::CodexAgentManager::default()).unwrap();
    assert_eq!(saved.status, Status::Stopped);
    assert!(saved.active_run_id.is_none());
    assert_eq!(saved.units[0].attempts.len(), 1);
}

#[test]
fn real_build_preflight_failure_notifies_partial_without_starting_or_replaying_an_attempt() {
    let f = Fixture::new();
    let original = record(&f);
    store::create(&f.storage, &original).unwrap();
    let observations = Mutex::new(Vec::new());
    let manager = codex_agent::CodexAgentManager::default();
    let result = build(
        &f.storage, &original.id, false, &manager,
        RepositoryContextJobs::default(), &recovery::InterruptionJournal::default(),
        |_| {}, Arc::new(|_| panic!("no section may publish during failed preflight")),
        |_| panic!("no section may be adopted during failed preflight"),
        |guide| observations.lock().unwrap().push(guide.status.clone()),
    );
    assert!(result.is_err());
    assert_eq!(*observations.lock().unwrap(), vec![Status::Partial]);
    let saved = store::read(&f.storage, &original.id).unwrap();
    assert_eq!(saved.status, Status::Partial);
    assert!(saved.active_run_id.is_none());
    assert!(saved.units.iter().all(|u| u.attempts.is_empty()));
    assert_eq!(saved.plan, original.plan);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn unchanged_terminal_reconciliation_preserves_bytes_and_revision_even_at_the_limit() {
    for revision in [17, u64::MAX] {
        let f = Fixture::new();
        let mut original = record(&f);
        original.status = Status::Partial;
        original.revision = revision;
        store::create(&f.storage, &original).unwrap();
        let path = f
            .storage
            .join("repository-guides")
            .join(format!("{}.json", original.id));
        let bytes = format!("{}\n", serde_json::to_string_pretty(&original).unwrap()).into_bytes();
        std::fs::write(&path, &bytes).unwrap();
        let manager = codex_agent::CodexAgentManager::default();
        for _ in 0..3 {
            let recovered = reconcile(&f.storage, &original, &manager).unwrap();
            assert_eq!(
                recovered.revision, revision,
                "opening unchanged work must not create a revision"
            );
            assert_eq!(record_bytes(&f, &original), bytes);
        }
        assert!(!f.storage.join("codex-agent/runtime.json").exists());
        assert!(!f.storage.join("repository-guide-history").exists());
    }
}


fn pending(f: &Fixture) -> Record {
    let mut record = record(f);
    let mut attempt = attempt(&record, Outcome::Interrupted);
    attempt.summary = None;
    record.active_run_id = Some(attempt.run_id.clone());
    record.units[0].attempts.push(attempt);
    store::create(&f.storage, &record).unwrap();
    record
}

fn record_bytes(f: &Fixture, record: &Record) -> Vec<u8> {
    std::fs::read(
        f.storage
            .join("repository-guides")
            .join(format!("{}.json", record.id)),
    )
    .unwrap()
}

fn empty_answer(f: &Fixture, record: &Record, question: &str) -> std::path::PathBuf {
    use crate::{codex_identity::CodexReviewRuntime, evidence_ledger::Ledger};
    let runtime = CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 1,
    };
    let grant = workspace::save_retrieval_grant(
        &f.storage,
        &record.project_id,
        &record.snapshot_id,
        &runtime,
    )
    .unwrap();
    let run = &record.units[0].attempts[0].run_id;
    let ledger = Ledger::new_progressive(&f.storage, run, &f.manifest, &grant, &runtime).unwrap();
    ledger
        .start_narration_answer_sections(
            question,
            &"d".repeat(64),
            &f.manifest,
            &runtime,
            &std::sync::atomic::AtomicBool::new(false),
        )
        .unwrap();
    f.storage
        .join("answer-sections")
        .join(format!("{run}.json"))
}

#[test]
fn stale_guide_recovery_cannot_erase_newer_completed_unit_metadata() {
    let f = Fixture::new();
    let original = record(&f);
    store::create(&f.storage, &original).unwrap();
    let complete = attempt(&original, Outcome::Complete);
    store::update(&f.storage, &original.id, |current| {
        current.units[0].attempts.push(complete);
        Ok(())
    })
    .unwrap();
    let newer = record_bytes(&f, &original);
    let result = crate::repository_guide::reconcile(
        &f.storage,
        &original,
        &codex_agent::CodexAgentManager::default(),
    );
    assert!(matches!(result, Err(message) if message.contains("changed while opening")));
    assert_eq!(record_bytes(&f, &original), newer);
    assert!(unit_complete(
        &store::read(&f.storage, &original.id).unwrap().units[0]
    ));
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn concurrent_stop_preserves_new_attempts_plan_and_owner_instead_of_copying_stale_state() {
    let f = Fixture::new();
    let original = record(&f);
    store::create(&f.storage, &original).unwrap();
    let mut added = attempt(&original, Outcome::Interrupted);
    added.summary = None;
    let run = added.run_id.clone();
    store::update(&f.storage, &original.id, |current| {
        current.units[0].attempts.push(added);
        current.active_run_id = Some(run.clone());
        current.plan.as_mut().unwrap().title = "A newer saved guide outline".into();
        current.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    let stopped_bytes = record_bytes(&f, &original);
    let recovered = reconcile(
        &f.storage,
        &original,
        &codex_agent::CodexAgentManager::default(),
    )
    .unwrap();
    assert_eq!(recovered.status, Status::Stopped);
    assert_eq!(recovered.units[0].attempts.len(), 1);
    assert_eq!(recovered.active_run_id.as_ref(), Some(&run));
    assert_eq!(recovered.plan.unwrap().title, "A newer saved guide outline");
    assert_eq!(recovered.revision, 1);
    assert_eq!(record_bytes(&f, &original), stopped_bytes);
}

#[test]
fn an_owned_worker_without_a_receipt_is_not_recovered_as_interrupted() {
    let f = Fixture::new();
    let record = pending(&f);
    let manager = codex_agent::CodexAgentManager::default();
    let run = &record.units[0].attempts[0].run_id;
    let worker = manager.hold_answer_worker(run).unwrap();
    let original = record_bytes(&f, &record);
    assert!(
        matches!(reconcile(&f.storage, &record, &manager), Err(message) if message.contains("still has an owned"))
    );
    assert_eq!(record_bytes(&f, &record), original);
    drop(worker);
    let recovered = reconcile(&f.storage, &record, &manager).unwrap();
    assert_eq!(
        recovered.units[0].attempts[0]
            .summary
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::Interrupted
    );
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn guide_source_validation_does_not_hold_stop_ownership() {
    use std::{sync::mpsc, time::Duration};
    let f = Fixture::new();
    let record = pending(&f);
    let original = record_bytes(&f, &record);
    let manager = codex_agent::CodexAgentManager::default();
    let stopping = manager.clone();
    let run = record.units[0].attempts[0].run_id.clone();
    let (ready, wait) = mpsc::channel();
    let (cancelled, result) = mpsc::channel();
    let stop = std::thread::spawn(move || {
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        cancelled.send(stopping.cancel(&run)).unwrap();
    });
    let mut stopped_during_validation = false;
    let recovered = reconcile_with_reader(&f.storage, &record, &manager, Status::Interrupted, |_| {
        ready.send(()).unwrap();
        stopped_during_validation = result.recv_timeout(Duration::from_secs(1)).unwrap_or(false);
        Err("Controlled source replay failure; saved bytes remain unchanged.".into())
    });
    stop.join().unwrap();
    assert!(recovered.is_err());
    assert!(
        stopped_during_validation,
        "Stop must acquire native ownership while guide source replay is pending"
    );
    assert_eq!(record_bytes(&f, &record), original);
}

#[test]
fn changed_answer_receipts_are_reread_with_a_fixed_bound_and_never_overwrite_stop() {
    let f = Fixture::new();
    let record = pending(&f);
    let attempt = &record.units[0].attempts[0];
    let path = empty_answer(&f, &record, &attempt.question);
    let observed = reading::observe(
        &f.storage,
        &attempt.run_id,
        &record.project_id,
        &record.snapshot_id,
    )
    .unwrap();
    reading::interrupt(&f.storage, &attempt.run_id, Outcome::Stopped).unwrap();
    let stopped = std::fs::read(&path).unwrap();
    let original = record_bytes(&f, &record);
    let manager = codex_agent::CodexAgentManager::default();
    let mut reads = 0;
    let changed = reconcile_with_reader(&f.storage, &record, &manager, Status::Interrupted, |_| {
        reads += 1;
        Ok(Some(observed.clone()))
    });
    assert_eq!(reads, 3);
    assert!(matches!(changed, Err(message) if message.contains("nothing was regenerated")));
    assert_eq!(std::fs::read(&path).unwrap(), stopped);
    assert_eq!(record_bytes(&f, &record), original);
    let mut reads = 0;
    let recovered = reconcile_with_reader(&f.storage, &record, &manager, Status::Interrupted, |_| {
        reads += 1;
        if reads == 1 {
            Ok(Some(observed.clone()))
        } else {
            reading::observe(
                &f.storage,
                &attempt.run_id,
                &record.project_id,
                &record.snapshot_id,
            )
            .map(Some)
        }
    })
    .unwrap();
    assert_eq!(reads, 2);
    assert_eq!(
        recovered.units[0].attempts[0]
            .summary
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::Stopped
    );
    assert_eq!(std::fs::read(&path).unwrap(), stopped);
}

#[test]
fn a_missing_observation_cannot_hide_a_new_receipt() {
    let f = Fixture::new();
    let record = pending(&f);
    let attempt = &record.units[0].attempts[0];
    empty_answer(&f, &record, &attempt.question);
    let manager = codex_agent::CodexAgentManager::default();
    let mut reads = 0;
    let recovered = reconcile_with_reader(&f.storage, &record, &manager, Status::Interrupted, |_| {
        reads += 1;
        if reads == 1 {
            Ok(None)
        } else {
            reading::observe(
                &f.storage,
                &attempt.run_id,
                &record.project_id,
                &record.snapshot_id,
            )
            .map(Some)
        }
    })
    .unwrap();
    assert_eq!(reads, 2);
    assert_eq!(
        recovered.units[0].attempts[0]
            .summary
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::Interrupted
    );
}

#[test]
fn changed_questions_fail_before_either_record_is_recovered() {
    let f = Fixture::new();
    let record = pending(&f);
    let path = empty_answer(&f, &record, "A different investigation question?");
    let answer_bytes = std::fs::read(&path).unwrap();
    let guide_bytes = record_bytes(&f, &record);
    assert!(
        matches!(reconcile(&f.storage, &record, &codex_agent::CodexAgentManager::default()), Err(message) if message.contains("question changed"))
    );
    assert_eq!(record_bytes(&f, &record), guide_bytes);
    assert_eq!(std::fs::read(&path).unwrap(), answer_bytes);
}
