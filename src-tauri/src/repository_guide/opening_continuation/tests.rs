use super::*;
use crate::chapter_generation::tests::Fixture;
use crate::repository_guide::tests::{attempt, fresh_record};

fn partial(f: &Fixture) -> Record {
    let mut record = fresh_record(f);
    let mut opening = attempt(&record, Outcome::Partial);
    opening.run_id = record.planning_run_id.clone();
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
fn initial_build_never_automatically_retries_a_partial_opening() {
    let f = Fixture::new();
    let mut record = partial(&f);
    for index in 0..record.units.len() {
        let mut owner = record.clone();
        owner.schema_version = 2;
        owner.contract = OPENING_CONTRACT.into();
        owner.planning_run_id = record.units[index].attempts[0].run_id.clone();
        owner.question = record.units[index].attempts[0].question.clone();
        owner.opening.as_mut().unwrap().run_id = owner.planning_run_id.clone();
        owner.opening.as_mut().unwrap().question = owner.question.clone();
        owner.opening_context_sha256 = None;
        let mut answer = opening::tests::publish(&f, &owner);
        answer
            .finish_manifest(
                r#"{"manifestVersion":"answerManifestV2","incompleteTopics":[]}"#,
                false,
            )
            .unwrap();
        let saved = reading::scoped(
            &f.storage,
            &owner.planning_run_id,
            &record.project_id,
            &record.snapshot_id,
        )
        .unwrap()
        .summary();
        for section in &saved.sections {
            reading::adopt(
                &f.storage,
                &section.run_id,
                &record.project_id,
                &record.snapshot_id,
                &section.generation_id,
            )
            .unwrap();
        }
        record.units[index].attempts[0].summary = Some(saved);
    }
    store::create(&f.storage, &record).unwrap();
    let result = build(
        &f.storage,
        &record.id,
        false,
        &codex_agent::CodexAgentManager::default(),
        RepositoryContextJobs::default(),
        &recovery::InterruptionJournal::default(),
        |_| {},
        Arc::new(|_| {}),
        |_| Ok(()),
        |_| {},
    )
    .unwrap();
    assert_eq!(result.status, Status::Partial);
    assert!(result.can_continue);
    assert_eq!(result.continuation_operations, 1);
    assert!(
        store::read(&f.storage, &record.id)
            .unwrap()
            .opening_continuations
            .is_empty()
    );
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
    assert_eq!(MAX_UNITS + 1, 7);
}

#[test]
fn original_opening_is_spent_and_only_one_explicit_follow_up_is_admitted() {
    let f = Fixture::new();
    let record = partial(&f);
    store::create(&f.storage, &record).unwrap();
    assert!(!eligible(&record));
    assert_eq!(remaining_operations(&record), 1);
    let prepared = store::prepare_continuation(&f.storage, &record.id, record.revision).unwrap();
    assert!(eligible(&prepared));
    let begun = begin(&f.storage, &prepared, &Uuid::new_v4().to_string()).unwrap();
    assert_eq!(begun.opening_continuations.len(), 1);
    assert_eq!(
        begun.opening_continuations[0].question,
        opening::question(&record).unwrap()
    );
    assert_eq!(remaining_operations(&begun), 0);
    let before = bytes(&f, &record);
    assert!(begin(&f.storage, &prepared, &Uuid::new_v4().to_string()).is_err());
    assert!(begin(&f.storage, &begun, &Uuid::new_v4().to_string()).is_err());
    assert_eq!(bytes(&f, &record), before);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn completed_follow_up_preserves_original_drafts_outline_and_completed_units() {
    let f = Fixture::new();
    let record = partial(&f);
    store::create(&f.storage, &record).unwrap();
    let prepared = store::prepare_continuation(&f.storage, &record.id, 0).unwrap();
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
    let record = partial(&f);
    store::create(&f.storage, &record).unwrap();
    let prepared = store::prepare_continuation(&f.storage, &record.id, 0).unwrap();
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
    let record = partial(&f);
    store::create(&f.storage, &record).unwrap();
    let prepared = store::prepare_continuation(&f.storage, &record.id, 0).unwrap();
    let begun = begin(&f.storage, &prepared, &Uuid::new_v4().to_string()).unwrap();
    let recovered = reconcile(
        &f.storage,
        &begun,
        &codex_agent::CodexAgentManager::default(),
    )
    .unwrap();
    assert_eq!(recovered.status, Status::Interrupted);
    assert_eq!(
        recovered.opening_continuations[0]
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
fn legacy_upgrade_is_explicit_archives_exact_bytes_and_preserves_original_scope() {
    for (schema, contract) in [
        (2, OPENING_CONTRACT),
        (3, INDEPENDENT_CONTRACT),
        (4, SEPARATE_CONTRACT),
    ] {
        let f = Fixture::new();
        let mut record = partial(&f);
        record.schema_version = schema;
        record.contract = contract.into();
        if schema != 4 {
            record.opening_context_sha256 = None;
            let opening = record.opening.as_mut().unwrap();
            opening.question = record.question.clone();
            opening.summary.as_mut().unwrap().question = record.question.clone();
        }
        store::create(&f.storage, &record).unwrap();
        let location = f
            .storage
            .join("repository-guides")
            .join(format!("{}.json", record.id));
        // Preserve noncanonical whitespace as well as field values.
        let raw = serde_json::to_vec_pretty(&record).unwrap();
        std::fs::write(&location, &raw).unwrap();
        let read = store::read(&f.storage, &record.id).unwrap();
        assert_eq!(read.schema_version, schema);
        assert_eq!(std::fs::read(&location).unwrap(), raw);
        assert!(!f.storage.join("repository-guide-history").exists());
        assert!(store::prepare_continuation(&f.storage, &record.id, 1).is_err());
        assert!(!f.storage.join("repository-guide-history").exists());
        let prepared = store::prepare_continuation(&f.storage, &record.id, 0).unwrap();
        assert_eq!(prepared.schema_version, 5);
        assert_eq!(store::original_contract(&prepared), (schema, contract));
        assert_eq!(separate_opening_intent(&prepared), schema == 4);
        assert_eq!(independent_research(&prepared), schema >= 3);
        assert_eq!(
            opening::question(&prepared).unwrap(),
            record.opening.as_ref().unwrap().question
        );
        let archive = std::fs::read_dir(f.storage.join("repository-guide-history"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(std::fs::read(&archive).unwrap(), raw);
        assert_eq!(
            prepared.upgrade.as_ref().unwrap().sha256,
            context::digest(&raw)
        );
        let begun = begin(&f.storage, &prepared, &Uuid::new_v4().to_string()).unwrap();
        let finished = finish(&f.storage, &begun, completion(&begun)).unwrap();
        assert!(guide_complete(&finished));
        assert_eq!(std::fs::read(archive).unwrap(), raw);
        let before = bytes(&f, &record);
        store::read(&f.storage, &record.id).unwrap();
        assert_eq!(bytes(&f, &record), before);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(location).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }
}

#[test]
fn archive_integrity_rejects_removed_history_or_rewritten_old_attempts() {
    let f = Fixture::new();
    let mut record = partial(&f);
    record.schema_version = 4;
    record.contract = SEPARATE_CONTRACT.into();
    store::create(&f.storage, &record).unwrap();
    let prepared = store::prepare_continuation(&f.storage, &record.id, 0).unwrap();
    let before = bytes(&f, &record);
    assert!(
        store::update(&f.storage, &record.id, |current| {
            current.units[0].attempts.clear();
            Ok(())
        })
        .is_err()
    );
    assert!(
        store::update(&f.storage, &record.id, |current| {
            current.plan.as_mut().unwrap().title.push_str(" changed");
            Ok(())
        })
        .is_err()
    );
    assert_eq!(bytes(&f, &record), before);
    let archive = std::fs::read_dir(f.storage.join("repository-guide-history"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let raw = std::fs::read(&archive).unwrap();
    std::fs::write(&archive, b"{}").unwrap();
    assert!(store::read(&f.storage, &record.id).is_err());
    assert_eq!(bytes(&f, &record), before);
    std::fs::write(&archive, raw).unwrap();
    assert_eq!(
        store::read(&f.storage, &record.id).unwrap().revision,
        prepared.revision
    );
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
    notice.run_id = record.planning_run_id.clone();
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
    assert!(continuation.scoped);
    assert_eq!(continuation.previous_run_id, original.run_id);
    assert_eq!(
        continuation.incomplete_topics,
        original.summary.as_ref().unwrap().incomplete_topics
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
                &|_| {}
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
    let record = partial(&f);
    store::create(&f.storage, &record).unwrap();
    let prepared = store::prepare_continuation(&f.storage, &record.id, 0).unwrap();
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
    assert_eq!(stopped.opening_continuations.len(), 1);
    assert!(begin(&f.storage, &begun, &Uuid::new_v4().to_string()).is_err());
}

#[test]
fn legacy_consent_counts_units_that_a_completed_prerequisite_can_unlock() {
    let f = Fixture::new();
    let mut record = partial(&f);
    record.schema_version = 2;
    record.contract = OPENING_CONTRACT.into();
    record.opening_context_sha256 = None;
    record.opening.as_mut().unwrap().question = record.question.clone();
    record
        .opening
        .as_mut()
        .unwrap()
        .summary
        .as_mut()
        .unwrap()
        .question = record.question.clone();
    for unit in &mut record.units {
        unit.attempts.clear();
    }
    assert!(!prerequisite_complete(&record, 1));
    assert_eq!(
        remaining_operations(&record),
        3,
        "one opening and both possible unit operations"
    );
}

#[cfg(unix)]
#[test]
fn a_conflicting_history_entry_preserves_the_original_record_without_dispatch() {
    let f = Fixture::new();
    let mut record = partial(&f);
    record.schema_version = 4;
    record.contract = SEPARATE_CONTRACT.into();
    store::create(&f.storage, &record).unwrap();
    let before = bytes(&f, &record);
    let history = f.storage.join("repository-guide-history");
    std::fs::create_dir(&history).unwrap();
    std::os::unix::fs::symlink("/", history.join("conflicting-entry")).unwrap();
    assert!(store::prepare_continuation(&f.storage, &record.id, 0).is_err());
    assert_eq!(bytes(&f, &record), before);
    assert!(
        store::read(&f.storage, &record.id)
            .unwrap()
            .opening_continuations
            .is_empty()
    );
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
#[ignore = "Explicit offline qualification of an owned copy of a saved partial guide; never connects an account or runs inference."]
fn saved_partial_opening_recovery_probe() {
    use std::io::Write;
    let root = std::path::PathBuf::from(
        std::env::var_os("CODE_ATLAS_OPENING_RECOVERY_FIXTURE")
            .expect("provide an isolated owned fixture copy"),
    );
    let marker: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("owned-fixture.json")).unwrap()).unwrap();
    assert_eq!(
        marker["kind"],
        "code-atlas-owned-opening-recovery-fixture-v1"
    );
    let storage = root.join("storage");
    assert!(storage.is_dir());
    let id = marker["guideId"].as_str().unwrap();
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut report = options.open(root.join("native-review.json")).unwrap();
    let _lease = store::lease(&storage, id).unwrap().unwrap();
    let original = store::read(&storage, id).unwrap();
    assert!(matches!(original.schema_version, 2..=4));
    assert_eq!(original.status, Status::Partial);
    assert!(original.plan.is_some());
    assert!(original.active_run_id.is_none());
    assert_eq!(
        original
            .opening
            .as_ref()
            .unwrap()
            .summary
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::Partial
    );
    let before = summary(&storage, &original).unwrap();
    let raw = std::fs::read(storage.join("repository-guides").join(format!("{id}.json"))).unwrap();
    let prepared = store::prepare_continuation(&storage, id, original.revision).unwrap();
    assert_eq!(
        prepared.upgrade.as_ref().unwrap().sha256,
        context::digest(&raw)
    );
    let original_contract = store::original_contract(&prepared);
    assert_eq!(
        original_contract,
        (original.schema_version, original.contract.as_str())
    );
    let question = opening::question(&original).unwrap();
    let intent = intent(&prepared).unwrap();
    assert_eq!(intent.question, original.question);
    assert_eq!(
        intent
            .opening_continuation
            .as_ref()
            .unwrap()
            .incomplete_topics,
        original
            .opening
            .as_ref()
            .unwrap()
            .summary
            .as_ref()
            .unwrap()
            .incomplete_topics
    );
    let run = Uuid::new_v4().to_string();
    let begun = begin(&storage, &prepared, &run).unwrap();
    assert_eq!(begun.opening_continuations[0].question, question);
    assert!(begin(&storage, &prepared, &Uuid::new_v4().to_string()).is_err());
    // No agent was dispatched. Model a restart before the new operation had a
    // receipt: its admitted attempt remains spent and locally Interrupted.
    let recovered =
        reconcile(&storage, &begun, &codex_agent::CodexAgentManager::default()).unwrap();
    assert_eq!(recovered.opening_continuations.len(), 1);
    assert_eq!(
        recovered.opening_continuations[0]
            .summary
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::Interrupted
    );
    assert!(!eligible(&recovered));
    assert_eq!(recovered.plan, original.plan);
    assert_eq!(
        serde_json::to_value(&recovered.units).unwrap(),
        serde_json::to_value(&original.units).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&recovered.opening).unwrap(),
        serde_json::to_value(&original.opening).unwrap()
    );
    let after = summary(&storage, &recovered).unwrap();
    let before_sections = before
        .answers
        .iter()
        .flat_map(|answer| &answer.sections)
        .collect::<Vec<_>>();
    let after_sections = after
        .answers
        .iter()
        .flat_map(|answer| &answer.sections)
        .collect::<Vec<_>>();
    assert_eq!(
        serde_json::to_value(&before_sections).unwrap(),
        serde_json::to_value(&after_sections).unwrap()
    );
    let result = serde_json::json!({
        "originalGuideSchema":original.schema_version,"newGuideSchema":recovered.schema_version,
        "originalGuideSha256":context::digest(&raw),"guideId":id,
        "originalOpeningRunId":original.planning_run_id,"followUpRunId":run,
        "openingQuestionUnchanged":true,"originalGuideQuestionUnchanged":true,
        "outlineUnchanged":true,"priorUnitAttemptsUnchanged":true,"priorOpeningUnchanged":true,
        "priorSections":before_sections.len(),"originalBytesArchived":true,
        "admittedFollowUps":1,"recoveredOutcome":"interrupted","openingReplayPermitted":false,
        "inferenceDispatched":false,"completeAnswerQualified":false,
        "semanticReview":false,"nativeUiReview":false,"voiceReview":false,
    });
    report
        .write_all(&serde_json::to_vec_pretty(&result).unwrap())
        .unwrap();
    report.sync_all().unwrap();
}
