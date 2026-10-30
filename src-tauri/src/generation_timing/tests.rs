use super::*;
use crate::codex_identity::CodexReviewRuntime;
use serde_json::{Value, json};
use std::time::Duration;

pub(super) struct Fixture(pub(super) PathBuf);
impl Fixture {
    pub(super) fn new() -> Self {
        let root = std::env::temp_dir().join(format!("atlas-timing-test-{}", uuid::Uuid::new_v4()));
        codex_agent::ensure_private_directory(&root).unwrap();
        Self(root)
    }
    pub(super) fn trace(&self, run: &str, started: Instant) -> Trace {
        Trace::new_with_preparation_and_prompt(
            &self.0,
            run,
            &"c".repeat(64),
            &runtime(),
            "gpt-6-luna",
            "max",
            started,
            Some("synthetic-contract-v1"),
            None,
            vec![],
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn runtime() -> CodexReviewRuntime {
    CodexReviewRuntime {
        connection_id: uuid::Uuid::new_v4().to_string(),
        executable_path: "/private/NEVER-EXPORT-EXECUTABLE".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 1,
    }
}
pub(super) fn run() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[test]
fn native_events_keep_one_clock_unknown_usage_and_no_sensitive_fields() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now() - Duration::from_millis(50));
    trace.event(EventKind::RuntimeLockAcquired);
    for _ in 0..20 {
        trace.progress("generating");
    }
    trace.progress("repository source NEVER-EXPORT-SOURCE");
    trace.context(ContextTiming {
        origin_elapsed_ms: Some(25),
        calls: vec![ContextCall {
            tool: crate::evidence_ledger::Tool::Read,
            state: crate::evidence_ledger::CallState::Delivered,
            started_ms: Some(1),
            prepared_ms: Some(2),
            finished_ms: Some(3),
            source_bytes: 12,
            metadata_bytes: 4,
            wire_bytes: 16,
            stages: Vec::new(),
        }],
    });
    trace.output(200);
    trace.finish(Ok(()));
    let record = inspect(&f.0, &id).unwrap().unwrap();
    assert_eq!(record.events.len(), 3);
    assert!(record.events[0].elapsed_ms >= 50);
    assert_eq!(record.context.as_ref().unwrap().origin_elapsed_ms, Some(25));
    assert_eq!(record.terminal.as_ref().unwrap().state, Terminal::Returned);
    let text = serde_json::to_string(&record).unwrap();
    for forbidden in [
        "NEVER-EXPORT",
        "executablePath",
        "accountIdentity",
        "contents",
        "question",
        "referenceId",
    ] {
        assert!(!text.contains(forbidden));
    }
    assert!(text.contains("\"tokenUsage\":null"));
    let before = fs::read(path(&f.0, &id).unwrap()).unwrap();
    trace.event(EventKind::PublicationStarted);
    drop(trace);
    assert_eq!(before, fs::read(path(&f.0, &id).unwrap()).unwrap());
}

#[test]
fn timing_diagnostics_preserve_the_selected_model_and_effort() {
    let f = Fixture::new();
    let id = run();
    let trace = Trace::new_with_preparation_and_prompt(
        &f.0,
        &id,
        &"c".repeat(64),
        &runtime(),
        "gpt-5.6-luna",
        "medium",
        Instant::now(),
        Some("file-chapter-test-v1"),
        None,
        vec![],
    );
    trace.finish(Ok(()));

    let record = inspect(&f.0, &id).unwrap().unwrap();
    assert_eq!(record.model_id, "gpt-5.6-luna");
    assert_eq!(record.reasoning_effort, "medium");
    assert_eq!(
        record.prompt_contract_version.as_deref(),
        Some("file-chapter-test-v1")
    );
}

#[test]
fn application_outcome_is_correlated_after_provider_return_and_is_idempotent() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now() - Duration::from_millis(30));
    trace.finish(Ok(()));
    drop(trace);
    let terminal = inspect(&f.0, &id)
        .unwrap()
        .unwrap()
        .terminal
        .unwrap()
        .elapsed_ms;
    let result = ApplicationResult {
        outcome: ApplicationOutcome::Complete,
        observation: ApplicationObservation::Live,
        observed_elapsed_ms: Some(terminal + 1),
        published_section_count: 1,
        first_useful_elapsed_ms: Some(terminal.saturating_sub(1)),
    };
    record_application_result(&f.0, &id, result.clone()).unwrap();
    let first = fs::read(path(&f.0, &id).unwrap()).unwrap();
    // Late reads do not overwrite the result recorded by the live worker.
    record_application_result(
        &f.0,
        &id,
        ApplicationResult {
            outcome: ApplicationOutcome::Interrupted,
            observation: ApplicationObservation::Reconciled,
            observed_elapsed_ms: None,
            published_section_count: 0,
            first_useful_elapsed_ms: None,
        },
    )
    .unwrap();
    assert_eq!(first, fs::read(path(&f.0, &id).unwrap()).unwrap());

    let saved = inspect(&f.0, &id).unwrap().unwrap();
    assert_eq!(saved.schema_version, SCHEMA_VERSION);
    assert_eq!(saved.terminal.as_ref().unwrap().state, Terminal::Returned);
    assert_eq!(saved.application_result, Some(result));
    let text = serde_json::to_string(&saved).unwrap();
    for forbidden in ["question", "source", "contents", "model reasoning"] {
        assert!(!text.contains(forbidden));
    }
}

#[test]
fn greenfield_current_format_rejects_legacy_timing_schema_without_rewriting() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now());
    trace.finish(Ok(()));
    drop(trace);
    let file = path(&f.0, &id).unwrap();
    let mut old: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    old["schemaVersion"] = json!(4);
    old.as_object_mut().unwrap().remove("promptContractVersion");
    fs::write(&file, serde_json::to_vec(&old).unwrap()).unwrap();
    let original = fs::read(&file).unwrap();

    assert!(inspect(&f.0, &id).is_err());
    assert!(record_application_result(
        &f.0,
        &id,
        ApplicationResult {
            outcome: ApplicationOutcome::Interrupted,
            observation: ApplicationObservation::Reconciled,
            observed_elapsed_ms: None,
            published_section_count: 1,
            first_useful_elapsed_ms: Some(0),
        },
    )
    .is_err());
    assert_eq!(fs::read(&file).unwrap(), original);
}

#[test]
fn live_application_result_supersedes_a_reconciliation_race() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now() - Duration::from_millis(20));
    trace.finish(Ok(()));
    drop(trace);
    let terminal = inspect(&f.0, &id)
        .unwrap()
        .unwrap()
        .terminal
        .unwrap()
        .elapsed_ms;
    record_application_result(
        &f.0,
        &id,
        ApplicationResult {
            outcome: ApplicationOutcome::Complete,
            observation: ApplicationObservation::Reconciled,
            observed_elapsed_ms: None,
            published_section_count: 1,
            first_useful_elapsed_ms: Some(terminal),
        },
    )
    .unwrap();
    record_application_result(
        &f.0,
        &id,
        ApplicationResult {
            outcome: ApplicationOutcome::Complete,
            observation: ApplicationObservation::Live,
            observed_elapsed_ms: Some(terminal + 1),
            published_section_count: 1,
            first_useful_elapsed_ms: Some(terminal),
        },
    )
    .unwrap();
    let result = inspect(&f.0, &id)
        .unwrap()
        .unwrap()
        .application_result
        .unwrap();
    assert_eq!(result.observation, ApplicationObservation::Live);
    assert_eq!(result.observed_elapsed_ms, Some(terminal + 1));
}

#[test]
fn application_timing_rejects_cross_clock_and_out_of_order_values() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now() - Duration::from_millis(40));
    trace.finish(Ok(()));
    drop(trace);
    let terminal = inspect(&f.0, &id)
        .unwrap()
        .unwrap()
        .terminal
        .unwrap()
        .elapsed_ms;
    let file = path(&f.0, &id).unwrap();

    assert!(
        record_application_result(
            &f.0,
            &id,
            ApplicationResult {
                outcome: ApplicationOutcome::Complete,
                observation: ApplicationObservation::Live,
                observed_elapsed_ms: Some(terminal.saturating_sub(1)),
                published_section_count: 0,
                first_useful_elapsed_ms: None,
            },
        )
        .is_err()
    );
    assert!(
        inspect(&f.0, &id)
            .unwrap()
            .unwrap()
            .application_result
            .is_none()
    );

    // Reconciled outcomes can be stored on a trace with no terminal event,
    // but they cannot claim an elapsed value from this process's clock.
    let mut value: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    value["terminal"] = Value::Null;
    fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(
        record_application_result(
            &f.0,
            &id,
            ApplicationResult {
                outcome: ApplicationOutcome::Interrupted,
                observation: ApplicationObservation::Reconciled,
                observed_elapsed_ms: Some(terminal + 1),
                published_section_count: 0,
                first_useful_elapsed_ms: None,
            },
        )
        .is_err()
    );
    record_application_result(
        &f.0,
        &id,
        ApplicationResult {
            outcome: ApplicationOutcome::Interrupted,
            observation: ApplicationObservation::Reconciled,
            observed_elapsed_ms: None,
            published_section_count: 0,
            first_useful_elapsed_ms: None,
        },
    )
    .unwrap();
    assert!(
        inspect(&f.0, &id)
            .unwrap()
            .unwrap()
            .application_result
            .is_some()
    );
}

#[test]
fn pre_manager_preparation_stages_share_the_dispatch_clock_and_are_bounded() {
    let f = Fixture::new();
    let id = run();
    let started = Instant::now() - Duration::from_millis(40);
    let kinds = [
        PreparationKind::WorkerRegistration,
        PreparationKind::JobRegistration,
        PreparationKind::SnapshotLoad,
        PreparationKind::RequestPreparation,
        PreparationKind::RuntimeReview,
        PreparationKind::JobIdentityBinding,
        PreparationKind::RetrievalGrant,
        PreparationKind::SyntaxIndexLoad,
        PreparationKind::RetrievalSession,
        PreparationKind::OperationJournal,
    ];
    let stages = kinds
        .into_iter()
        .enumerate()
        .map(|(index, kind)| PreparationStage {
            kind,
            started_ms: (index * 2) as u64,
            finished_ms: (index * 2 + 1) as u64,
            outcome: PreparationOutcome::Complete,
        })
        .collect();
    let trace = Trace::new_with_preparation(
        &f.0,
        &id,
        &"c".repeat(64),
        &runtime(),
        "gpt-6-luna",
        "max",
        started,
        Some(17),
        stages,
    );
    trace.finish(Ok(()));
    let record = inspect(&f.0, &id).unwrap().unwrap();
    assert_eq!(record.renderer_queue_wait_ms, Some(17));
    assert_eq!(record.preparation_stages.len(), 10);
    assert_eq!(
        record.preparation_stages[0].kind,
        PreparationKind::WorkerRegistration
    );
    assert_eq!(
        record.preparation_stages[1].kind,
        PreparationKind::JobRegistration
    );
    assert_eq!(
        record.preparation_stages[7].kind,
        PreparationKind::SyntaxIndexLoad
    );
    assert_eq!(
        record.preparation_stages[7].outcome,
        PreparationOutcome::Complete
    );
    assert_eq!(
        record.preparation_stages[9].kind,
        PreparationKind::OperationJournal
    );
    assert!(record.preparation_stages[9].finished_ms <= record.events[0].elapsed_ms);
    let at_limit = (0..MAX_PREPARATION_STAGES)
        .map(|index| PreparationStage {
            kind: PreparationKind::RetrievalSession,
            started_ms: (index * 2) as u64,
            finished_ms: (index * 2 + 1) as u64,
            outcome: PreparationOutcome::Complete,
        })
        .collect::<Vec<_>>();
    assert!(validate_preparation_stages(&at_limit).is_ok());
    let over_limit = (0..=MAX_PREPARATION_STAGES)
        .map(|index| PreparationStage {
            kind: PreparationKind::RetrievalSession,
            started_ms: (index * 2) as u64,
            finished_ms: (index * 2 + 1) as u64,
            outcome: PreparationOutcome::Complete,
        })
        .collect::<Vec<_>>();
    assert!(validate_preparation_stages(&over_limit).is_err());

    // Current records must contain the complete v4 envelope, including fields
    // introduced when preparation and renderer wait measurements became part
    // of the diagnostic contract.
    let file = path(&f.0, &id).unwrap();
    let mut incomplete: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    incomplete.as_object_mut().unwrap().remove("preparationStages");
    incomplete.as_object_mut().unwrap().remove("rendererQueueWaitMs");
    fs::write(&file, serde_json::to_vec(&incomplete).unwrap()).unwrap();
    let original = fs::read(&file).unwrap();
    assert!(inspect(&f.0, &id).is_err());
    assert_eq!(fs::read(&file).unwrap(), original);
}

#[test]
fn preparation_records_only_fixed_stage_and_outcome_on_failure() {
    let mut preparation = Preparation::new(Instant::now());
    let error = preparation
        .run_stage(PreparationKind::SnapshotLoad, || {
            Err::<(), _>("private source and local path".to_owned())
        })
        .unwrap_err();
    assert!(error.contains("private source"));
    let stages = preparation.into_stages();
    assert_eq!(stages.len(), 1);
    assert_eq!(stages[0].kind, PreparationKind::SnapshotLoad);
    assert_eq!(stages[0].outcome, PreparationOutcome::Failed);
    assert!(stages[0].finished_ms >= stages[0].started_ms);
    let encoded = serde_json::to_string(&stages).unwrap();
    assert!(!encoded.contains("private source"));
    assert!(!encoded.contains("local path"));
}

#[test]
fn local_context_stages_are_bounded_ordered_and_source_free() {
    use crate::evidence_ledger::Tool;

    let stages = vec![
        ContextStage {
            kind: ContextStageKind::SearchValidation,
            started_ms: 10,
            finished_ms: 11,
            outcome: ContextStageOutcome::Complete,
        },
        ContextStage {
            kind: ContextStageKind::SyntaxIndexLoad,
            started_ms: 12,
            finished_ms: 14,
            outcome: ContextStageOutcome::Fallback,
        },
        ContextStage {
            kind: ContextStageKind::SearchIndexPreparation,
            started_ms: 15,
            finished_ms: 19,
            outcome: ContextStageOutcome::Complete,
        },
    ];
    assert!(
        validate_context_stages(Tool::Search, Some(10), Some(20), Some(22), &stages, Some(2),)
            .is_ok()
    );

    let encoded = serde_json::to_string(&stages).unwrap();
    assert!(!encoded.contains("query"));
    assert!(!encoded.contains("path"));
    assert!(!encoded.contains("source"));

    let mut unordered = stages.clone();
    unordered.swap(1, 2);
    assert!(validate_context_stages(Tool::Search, Some(10), None, None, &unordered, None).is_err());
    assert!(validate_context_stages(Tool::Read, Some(10), None, None, &stages, None).is_err());

    let mut invalid_fallback = stages.clone();
    invalid_fallback[0].outcome = ContextStageOutcome::Fallback;
    assert!(
        validate_context_stages(Tool::Search, Some(10), None, None, &invalid_fallback, None,)
            .is_err()
    );

    let too_many = (0..=MAX_CONTEXT_STAGES_PER_CALL)
        .map(|index| ContextStage {
            kind: ContextStageKind::SearchQuery,
            started_ms: 10 + (index as u64 * 2),
            finished_ms: 11 + (index as u64 * 2),
            outcome: ContextStageOutcome::Complete,
        })
        .collect::<Vec<_>>();
    assert!(validate_context_stages(Tool::Search, Some(10), None, None, &too_many, None).is_err());
}

#[test]
fn optional_syntax_index_load_distinguishes_text_fallback_from_availability() {
    let mut preparation = Preparation::new(Instant::now());
    let index = preparation.run_optional_stage(PreparationKind::SyntaxIndexLoad, || {
        Ok::<_, String>(Some("available"))
    });
    assert_eq!(index, Some("available"));
    let absent = preparation.run_optional_stage(PreparationKind::SyntaxIndexLoad, || {
        Ok::<Option<()>, String>(None)
    });
    assert_eq!(absent, None);
    let failed = preparation.run_optional_stage(PreparationKind::SyntaxIndexLoad, || {
        Err::<Option<()>, _>("private index path".into())
    });
    assert_eq!(failed, None);

    let stages = preparation.into_stages();
    assert_eq!(stages.len(), 3);
    assert_eq!(stages[0].outcome, PreparationOutcome::Complete);
    assert_eq!(stages[1].outcome, PreparationOutcome::Fallback);
    assert_eq!(stages[2].outcome, PreparationOutcome::Fallback);
    let encoded = serde_json::to_string(&stages).unwrap();
    assert!(!encoded.contains("private index path"));
}

#[test]
fn abandoned_and_failed_runs_are_retained_without_inference_or_success_claims() {
    let f = Fixture::new();
    let id = run();
    drop(f.trace(&id, Instant::now()));
    assert_eq!(
        inspect(&f.0, &id).unwrap().unwrap().terminal.unwrap().state,
        Terminal::Abandoned
    );
    for (error, expected) in [
        (crate::codex_identity::changed(), Terminal::RuntimeChanged),
        (
            "The Codex run exceeded ten minutes and was stopped. Review again before retrying."
                .into(),
            Terminal::TimedOut,
        ),
        (
            "Codex stopped because Code Atlas is closing.".into(),
            Terminal::Cancelled,
        ),
        ("private source in OS error".into(), Terminal::Failed),
    ] {
        let id = run();
        let trace = f.trace(&id, Instant::now());
        trace.finish(Err(&error));
        let record = inspect(&f.0, &id).unwrap().unwrap();
        assert_eq!(record.terminal.unwrap().state, expected);
        assert!(
            !fs::read_to_string(path(&f.0, &id).unwrap())
                .unwrap()
                .contains(&error)
        );
    }
}

#[test]
fn a_failed_diagnostic_write_recovers_without_touching_saved_work() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now());
    fs::write(f.0.join("saved-chapter.json"), b"immutable chapter").unwrap();
    let dir = f.0.join("generation-timings");
    let held = f.0.join("held-timings");
    fs::rename(&dir, &held).unwrap();
    fs::write(&dir, b"not a directory").unwrap();
    trace.event(EventKind::Starting);
    fs::remove_file(&dir).unwrap();
    fs::rename(&held, &dir).unwrap();
    trace.finish(Ok(()));
    let record = inspect(&f.0, &id).unwrap().unwrap();
    assert_eq!(record.write_failures, 1);
    assert_eq!(record.events.last().unwrap().kind, EventKind::Starting);
    assert_eq!(
        fs::read(f.0.join("saved-chapter.json")).unwrap(),
        b"immutable chapter"
    );
}

#[test]
fn diagnostics_reject_corruption_duplicates_unexpected_content_and_identity_changes() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now());
    trace.finish(Ok(()));
    let file = path(&f.0, &id).unwrap();
    let original = fs::read(&file).unwrap();
    let original_value: Value = serde_json::from_slice(&original).unwrap();
    for (key, bad) in [
        ("schemaVersion", json!(6)),
        ("modelId", json!("private source")),
        ("events", json!([])),
        ("clock", json!("wall clock")),
        ("tokenUsage", json!({"input":99})),
        ("source", json!("private source")),
        ("runId", json!(run())),
        ("outputBytes", json!(300 * 1024 + 1)),
    ] {
        let mut value = original_value.clone();
        value[key] = bad;
        fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(inspect(&f.0, &id).is_err(), "{key}");
    }
    let duplicate =
        String::from_utf8(original.clone())
            .unwrap()
            .replacen("{", "{\"schemaVersion\":1,", 1);
    fs::write(&file, duplicate).unwrap();
    assert!(inspect(&f.0, &id).is_err());
    fs::write(&file, vec![b' '; MAX_BYTES + 1]).unwrap();
    assert!(inspect(&f.0, &id).is_err());
    fs::write(&file, &original).unwrap();
    assert!(inspect(&f.0, "../outside").is_err());
    assert!(inspect(&f.0, &run()).unwrap().is_none());
    let second = f.trace(&id, Instant::now());
    second.event(EventKind::Starting);
    drop(second);
    assert_eq!(fs::read(&file).unwrap(), original);
}

#[test]
fn retention_is_bounded_preserves_unfinished_records_and_never_overwrites_a_run() {
    let f = Fixture::new();
    let active = run();
    let working = f.trace(&active, Instant::now());
    for _ in 1..MAX_RECORDS {
        f.trace(&run(), Instant::now()).finish(Ok(()));
    }
    let duplicate = f.trace(&active, Instant::now());
    drop(duplicate);
    assert_eq!(
        fs::read_dir(f.0.join("generation-timings"))
            .unwrap()
            .count(),
        MAX_RECORDS
    );
    f.trace(&run(), Instant::now()).finish(Ok(()));
    assert_eq!(
        fs::read_dir(f.0.join("generation-timings"))
            .unwrap()
            .count(),
        MAX_RECORDS
    );
    assert!(inspect(&f.0, &active).unwrap().unwrap().terminal.is_none());
    for index in 0..MAX_EVENTS + 20 {
        working.event(if index % 2 == 0 {
            EventKind::Starting
        } else {
            EventKind::Validating
        });
    }
    working.finish(Ok(()));
    let saved = inspect(&f.0, &active).unwrap().unwrap();
    assert_eq!(saved.events.len(), MAX_EVENTS);
    assert!(saved.truncated);
    assert!(fs::metadata(path(&f.0, &active).unwrap()).unwrap().len() < MAX_BYTES as u64);
}

#[test]
fn malformed_event_order_and_context_states_are_not_exported() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now());
    trace.event(EventKind::Starting);
    trace.finish(Ok(()));
    let file = path(&f.0, &id).unwrap();
    let original: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    for invalid in [
        json!({"originElapsedMs":0,"calls":[{"tool":"read","state":"delivered","startedMs":2,"preparedMs":1,"finishedMs":3,"sourceBytes":0,"metadataBytes":0,"wireBytes":0}]}),
        json!({"originElapsedMs":0,"calls":[{"tool":"read","state":"delivered","startedMs":0,"preparedMs":null,"finishedMs":3,"sourceBytes":0,"metadataBytes":0,"wireBytes":0}]}),
        json!({"originElapsedMs":0,"calls":[{"tool":"read","state":"started","startedMs":0,"preparedMs":1,"finishedMs":3,"sourceBytes":0,"metadataBytes":0,"wireBytes":0}]}),
        json!({"originElapsedMs":0,"calls":[{"tool":"read","state":"delivered","startedMs":0,"preparedMs":1,"finishedMs":3,"sourceBytes":250001,"metadataBytes":0,"wireBytes":250001}]}),
        json!({"originElapsedMs":0,"calls":[{"tool":"read","state":"delivered","startedMs":0,"preparedMs":1,"finishedMs":3,"sourceBytes":10,"metadataBytes":10,"wireBytes":19}]}),
        json!({"originElapsedMs":MAX_ELAPSED,"calls":[{"tool":"read","state":"delivered","startedMs":0,"preparedMs":0,"finishedMs":0,"sourceBytes":0,"metadataBytes":0,"wireBytes":0}]}),
    ] {
        let mut value = original.clone();
        value["context"] = invalid;
        fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(inspect(&f.0, &id).is_err());
    }
    let mut value = original.clone();
    value["events"][0]["elapsedMs"] = json!(MAX_ELAPSED + 1);
    fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(inspect(&f.0, &id).is_err());
    let mut value = original.clone();
    value["terminal"]["elapsedMs"] = json!(0);
    value["events"][0]["elapsedMs"] = json!(10);
    fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(inspect(&f.0, &id).is_err());
    fs::write(&file, serde_json::to_vec(&original).unwrap()).unwrap();
}

#[test]
fn greenfield_current_format_requires_stage_outcomes_and_valid_bounds() {
    let f = Fixture::new();
    let id = run();
    let started = Instant::now() - Duration::from_millis(30);
    let trace = Trace::new_with_preparation(
        &f.0,
        &id,
        &"c".repeat(64),
        &runtime(),
        "gpt-6-luna",
        "max",
        started,
        None,
        vec![PreparationStage {
            kind: PreparationKind::SnapshotLoad,
            started_ms: 0,
            finished_ms: 1,
            outcome: PreparationOutcome::Complete,
        }],
    );
    trace.finish(Ok(()));
    let file = path(&f.0, &id).unwrap();
    let original: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    let mut excessive_queue_wait = original.clone();
    excessive_queue_wait["rendererQueueWaitMs"] = json!(MAX_ELAPSED + 1);
    fs::write(&file, serde_json::to_vec(&excessive_queue_wait).unwrap()).unwrap();
    assert!(inspect(&f.0, &id).is_err());
    let after_manager = original["events"][0]["elapsedMs"].as_u64().unwrap() + 1;
    for stages in [
        json!([{"kind":"snapshotLoad","startedMs":2,"finishedMs":1}]),
        json!([{"kind":"snapshotLoad","startedMs":0,"finishedMs":1},
               {"kind":"requestPreparation","startedMs":0,"finishedMs":2}]),
        json!([{"kind":"snapshotLoad","startedMs":0,"finishedMs":MAX_ELAPSED + 1}]),
        json!([{"kind":"snapshotLoad","startedMs":0,"finishedMs":after_manager}]),
    ] {
        let mut value = original.clone();
        value["preparationStages"] = stages;
        fs::write(&file, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(inspect(&f.0, &id).is_err());
    }
    let mut missing_outcome = original.clone();
    missing_outcome["preparationStages"][0]
        .as_object_mut()
        .unwrap()
        .remove("outcome");
    fs::write(&file, serde_json::to_vec(&missing_outcome).unwrap()).unwrap();
    assert!(inspect(&f.0, &id).is_err());
}

#[test]
fn greenfield_current_format_requires_context_stages() {
    let call = ContextCall {
        tool: crate::evidence_ledger::Tool::Read,
        state: crate::evidence_ledger::CallState::Delivered,
        started_ms: Some(1),
        prepared_ms: Some(2),
        finished_ms: Some(3),
        source_bytes: 4,
        metadata_bytes: 5,
        wire_bytes: 9,
        stages: Vec::new(),
    };
    let mut value = serde_json::to_value(call).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("stages");
    assert!(serde_json::from_value::<ContextCall>(value).is_err());
}

#[test]
fn overfull_external_storage_disables_tracing_without_deleting_records() {
    let f = Fixture::new();
    let original = run();
    f.trace(&original, Instant::now()).finish(Ok(()));
    let template: Value =
        serde_json::from_slice(&fs::read(path(&f.0, &original).unwrap()).unwrap()).unwrap();
    for _ in 0..MAX_RECORDS {
        let id = run();
        let mut value = template.clone();
        value["runId"] = json!(id);
        fs::write(
            path(&f.0, &id).unwrap(),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }
    let count = fs::read_dir(f.0.join("generation-timings"))
        .unwrap()
        .count();
    let id = run();
    f.trace(&id, Instant::now()).finish(Ok(()));
    assert_eq!(
        fs::read_dir(f.0.join("generation-timings"))
            .unwrap()
            .count(),
        count
    );
    assert!(inspect(&f.0, &id).unwrap().is_none());
    assert!(inspect(&f.0, &original).unwrap().is_some());
}

#[cfg(unix)]
#[test]
fn symlinked_storage_disables_diagnostics_without_changing_the_target() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let elsewhere = Fixture::new();
    symlink(&elsewhere.0, f.0.join("generation-timings")).unwrap();
    let trace = f.trace(&run(), Instant::now());
    trace.event(EventKind::Starting);
    trace.finish(Ok(()));
    assert_eq!(fs::read_dir(&elsewhere.0).unwrap().count(), 0);
}
