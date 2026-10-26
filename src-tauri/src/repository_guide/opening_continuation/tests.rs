use super::*;
use crate::chapter_generation::tests::Fixture;
use crate::repository_guide::tests::{attempt, fresh_record};

fn partial(f: &Fixture) -> Record {
    let mut record = fresh_record(f);
    let mut opening = attempt(&record, Outcome::Partial);
    opening.run_id = record.opening.as_ref().unwrap().run_id.clone();
    opening.question = opening::question(&record).unwrap().into();
    let summary = opening.summary.as_mut().unwrap();
    summary.run_id = opening.run_id.clone();
    summary.question = opening.question.clone();
    summary.incomplete_topics =
        vec!["The first transition's rejection result remains unverified.".into()];
    record.opening = Some(opening);
    for index in 0..record.units.len() {
        let complete = attempt(&record, Outcome::Complete);
        record.units[index].attempts.push(complete);
    }
    record.status = Status::Partial;
    record
}

fn after_automatic_retry(f: &Fixture) -> Record {
    let record = partial(f);
    store::create(&f.storage, &record).unwrap();
    let prepared = store::prepare_continuation(&f.storage, &record.id, record.revision).unwrap();
    let automatic =
        begin_with_retry(&f.storage, &prepared, &Uuid::new_v4().to_string(), true).unwrap();
    let owner = automatic.opening_continuations.last().unwrap();
    let mut failed = attempt(&record, Outcome::Partial).summary.unwrap();
    failed.run_id = owner.run_id.clone();
    failed.question = owner.question.clone();
    failed.incomplete_topics =
        vec!["The failed attempt still leaves this exact transition unclear.".into()];
    finish(&f.storage, &automatic, failed).unwrap()
}

fn completion(record: &Record) -> Summary {
    let owner = record.opening_continuations.last().unwrap();
    let mut result = attempt(record, Outcome::Complete).summary.unwrap();
    result.run_id = owner.run_id.clone();
    result.question = owner.question.clone();
    result.sections[0].run_id = owner.run_id.clone();
    result
}

fn bytes(f: &Fixture, record: &Record) -> Vec<u8> {
    std::fs::read(
        f.storage
            .join("repository-guides")
            .join(format!("{}.json", record.id)),
    )
    .unwrap()
}

#[test]
fn an_empty_partial_opening_gets_one_auto_recovery_and_one_explicit_retry() {
    let f = Fixture::new();
    let record = partial(&f);
    store::create(&f.storage, &record).unwrap();
    assert!(automatic_retry_eligible(&record));
    assert!(!eligible(&record));
    assert_eq!(remaining_operations(&record), 1);

    let prepared = store::prepare_continuation(&f.storage, &record.id, record.revision).unwrap();
    let automatic =
        begin_with_retry(&f.storage, &prepared, &Uuid::new_v4().to_string(), true).unwrap();
    assert!(automatic.opening_continuations[0].automatic_retry);
    assert!(!automatic_retry_eligible(&automatic));
    assert!(!eligible(&automatic));
    assert_eq!(remaining_operations(&automatic), 0);

    let owner = automatic.opening_continuations.last().unwrap();
    let mut failed = attempt(&record, Outcome::Partial).summary.unwrap();
    failed.run_id = owner.run_id.clone();
    failed.question = owner.question.clone();
    let terminal = finish(&f.storage, &automatic, failed).unwrap();
    let explicit = begin(&f.storage, &terminal, &Uuid::new_v4().to_string()).unwrap();
    assert_eq!(explicit.opening_continuations.len(), 2);
    assert!(!explicit.opening_continuations[1].automatic_retry);
    assert_eq!(remaining_operations(&explicit), 0);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn automatic_recovery_precedes_one_explicit_follow_up() {
    let f = Fixture::new();
    let record = partial(&f);
    store::create(&f.storage, &record).unwrap();
    assert!(!eligible(&record));
    assert_eq!(remaining_operations(&record), 1);
    let prepared = store::prepare_continuation(&f.storage, &record.id, record.revision).unwrap();
    assert!(!eligible(&prepared));
    let automatic =
        begin_with_retry(&f.storage, &prepared, &Uuid::new_v4().to_string(), true).unwrap();
    assert_eq!(automatic.opening_continuations.len(), 1);
    assert!(automatic.opening_continuations[0].automatic_retry);
    assert_eq!(
        automatic.opening_continuations[0].question,
        opening::question(&record).unwrap()
    );
    assert_eq!(remaining_operations(&automatic), 0);
    assert!(begin(&f.storage, &automatic, &Uuid::new_v4().to_string()).is_err());
    let owner = automatic.opening_continuations.last().unwrap();
    let mut failed = attempt(&record, Outcome::Partial).summary.unwrap();
    failed.run_id = owner.run_id.clone();
    failed.question = owner.question.clone();
    let terminal = finish(&f.storage, &automatic, failed).unwrap();
    assert!(eligible(&terminal));
    assert_eq!(remaining_operations(&terminal), 1);
    let explicit = begin(&f.storage, &terminal, &Uuid::new_v4().to_string()).unwrap();
    assert_eq!(explicit.opening_continuations.len(), 2);
    assert!(!explicit.opening_continuations[1].automatic_retry);
    assert!(begin(&f.storage, &explicit, &Uuid::new_v4().to_string()).is_err());
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn completed_follow_up_preserves_original_drafts_outline_and_completed_units() {
    let f = Fixture::new();
    let record = after_automatic_retry(&f);
    let prepared = store::prepare_continuation(&f.storage, &record.id, record.revision).unwrap();
    let begun = begin(&f.storage, &prepared, &Uuid::new_v4().to_string()).unwrap();
    assert!(!guide_complete(&begun));
    let finished = finish(&f.storage, &begun, completion(&begun)).unwrap();
    // State-machine metadata only; synthetic sections are not accepted output.
    assert!(guide_complete(&finished));
    assert_eq!(
        serde_json::to_value(&finished.opening).unwrap(),
        serde_json::to_value(&record.opening).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&finished.units).unwrap(),
        serde_json::to_value(&record.units).unwrap()
    );
    assert_eq!(finished.plan, record.plan);
    assert!(finished.active_run_id.is_none());
    assert_eq!(remaining_operations(&finished), 0);
    let before = bytes(&f, &record);
    assert!(finish(&f.storage, &begun, completion(&begun)).is_err());
    assert!(store::prepare_continuation(&f.storage, &record.id, finished.revision).is_err());
    assert_eq!(bytes(&f, &record), before);
}

#[test]
fn stop_and_stale_revision_prevent_admission_and_late_results_cannot_resume_work() {
    let f = Fixture::new();
    let record = after_automatic_retry(&f);
    assert!(eligible(&record));
    let prepared = store::prepare_continuation(&f.storage, &record.id, record.revision).unwrap();
    let stopped = store::update(&f.storage, &record.id, |current| {
        current.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    assert!(begin(&f.storage, &prepared, &Uuid::new_v4().to_string()).is_err());
    assert!(begin(&f.storage, &stopped, &Uuid::new_v4().to_string()).is_err());
    let resumed = store::prepare_continuation(&f.storage, &record.id, stopped.revision).unwrap();
    let begun = begin(&f.storage, &resumed, &Uuid::new_v4().to_string()).unwrap();
    let stopped = store::update(&f.storage, &record.id, |current| {
        current.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    let mut foreign = completion(&begun);
    foreign.run_id = Uuid::new_v4().to_string();
    assert!(finish(&f.storage, &begun, foreign).is_err());
    let mut saved = completion(&begun);
    saved.outcome = Outcome::Stopped;
    let finished = finish(&f.storage, &begun, saved).unwrap();
    assert_eq!(finished.status, Status::Stopped);
    assert!(finished.revision > stopped.revision);
    assert!(!guide_complete(&finished));
    assert_eq!(remaining_operations(&finished), 0);
}

#[test]
fn interrupted_follow_up_is_recovered_without_replay_or_a_budget_reset() {
    let f = Fixture::new();
    let record = after_automatic_retry(&f);
    let prepared = store::prepare_continuation(&f.storage, &record.id, record.revision).unwrap();
    let begun = begin(&f.storage, &prepared, &Uuid::new_v4().to_string()).unwrap();
    let recovered = reconcile(
        &f.storage,
        &begun,
        &codex_agent::CodexAgentManager::default(),
    )
    .unwrap();
    assert_eq!(recovered.status, Status::Interrupted);
    assert_eq!(
        recovered.opening_continuations[1]
            .summary
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::Interrupted
    );
    assert_eq!(remaining_operations(&recovered), 0);
    assert_eq!(recovered.plan, record.plan);
    assert_eq!(
        serde_json::to_value(&recovered.units).unwrap(),
        serde_json::to_value(&record.units).unwrap()
    );
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
    let before = bytes(&f, &record);
    assert!(store::prepare_continuation(&f.storage, &record.id, recovered.revision).is_err());
    assert_eq!(bytes(&f, &record), before);
}

#[test]
fn rejected_contracts_and_complete_openings_cannot_obtain_an_extra_attempt() {
    let f = Fixture::new();
    let mut record = partial(&f);
    let original = record.opening.as_mut().unwrap().summary.as_mut().unwrap();
    original.outcome = Outcome::Complete;
    original.incomplete_topics.clear();
    let mut notice = record.units[0].attempts[0]
        .summary
        .as_ref()
        .unwrap()
        .sections[0]
        .clone();
    notice.run_id = record.opening.as_ref().unwrap().run_id.clone();
    record
        .opening
        .as_mut()
        .unwrap()
        .summary
        .as_mut()
        .unwrap()
        .sections
        .push(notice);
    store::create(&f.storage, &record).unwrap();
    assert_eq!(remaining_operations(&record), 0);
    assert!(!eligible(&record));
    assert!(store::prepare_continuation(&f.storage, &record.id, 0).is_err());
    assert!(!f.storage.join("repository-guide-history").exists());
}

#[test]
fn opening_intent_keeps_both_questions_and_exact_gaps_outside_source_evidence() {
    let f = Fixture::new();
    let record = partial(&f);
    let intent = intent(&record).unwrap();
    assert_eq!(intent.question, record.question);
    assert_eq!(intent.unit_number, 0);
    assert_eq!(intent.unit_count, record.units.len());
    let original = record.opening.as_ref().unwrap();
    let continuation = intent.opening_continuation.unwrap();
    assert_eq!(continuation.previous_run_id, original.run_id);
    assert_eq!(
        continuation.incomplete_topics,
        original.summary.as_ref().unwrap().incomplete_topics
    );
}

#[test]
fn a_recovery_uses_the_latest_failed_opening_attempt_as_its_context() {
    let f = Fixture::new();
    let record = after_automatic_retry(&f);
    let intent = intent(&record).unwrap();
    let continuation = intent.opening_continuation.unwrap();
    let latest = record.opening_continuations.last().unwrap();
    assert_eq!(continuation.previous_run_id, latest.run_id);
    assert_eq!(
        continuation.incomplete_topics,
        latest.summary.as_ref().unwrap().incomplete_topics
    );
}

#[test]
fn a_missing_current_connection_or_changed_snapshot_cannot_spend_the_opening_follow_up() {
    let f = Fixture::new();
    let record = partial(&f);
    store::create(&f.storage, &record).unwrap();
    let prepared = store::prepare_continuation(&f.storage, &record.id, 0).unwrap();
    let before = bytes(&f, &record);
    let manager = codex_agent::CodexAgentManager::default();
    let journal = recovery::InterruptionJournal::default();
    for changed in [false, true] {
        let mut candidate = prepared.clone();
        if changed {
            candidate.manifest_sha256 = "f".repeat(64);
        }
        assert!(
            run(
                &f.storage,
                &candidate,
                &manager,
                RepositoryContextJobs::default(),
                &journal,
                &|_| {},
                Arc::new(|_| panic!("no publication was authorized")),
                &|_| Ok(()),
                &|_| {},
                false,
                None,
                std::time::Instant::now(),
            )
            .is_err()
        );
        assert_eq!(bytes(&f, &record), before);
    }
    assert!(!f.storage.join("answer-sections").exists());
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn stop_selects_the_durable_current_owner_and_checks_scope_in_the_transaction() {
    let f = Fixture::new();
    let record = after_automatic_retry(&f);
    let prepared = store::prepare_continuation(&f.storage, &record.id, record.revision).unwrap();
    let run = Uuid::new_v4().to_string();
    let begun = begin(&f.storage, &prepared, &run).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    let before = bytes(&f, &record);
    assert!(
        stop_saved_guide(
            &f.storage,
            &manager,
            &record.id,
            "foreign",
            &record.snapshot_id
        )
        .is_err()
    );
    assert_eq!(bytes(&f, &record), before);
    stop_saved_guide(
        &f.storage,
        &manager,
        &record.id,
        &record.project_id,
        &record.snapshot_id,
    )
    .unwrap();
    let stopped = store::read(&f.storage, &record.id).unwrap();
    assert_eq!(stopped.status, Status::Stopped);
    assert_eq!(stopped.active_run_id.as_deref(), Some(run.as_str()));
    assert_eq!(stopped.opening_continuations.len(), 2);
    assert!(begin(&f.storage, &begun, &Uuid::new_v4().to_string()).is_err());
}
