use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};
use serde_json::{Value, json};

fn runtime() -> CodexReviewRuntime {
    CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 1,
    }
}

fn setup(f: &Fixture) -> (Ledger, CodexReviewRuntime, AtomicBool) {
    setup_with_model(f, "gpt-6-luna", "max")
}

fn setup_with_model(
    f: &Fixture,
    model: &str,
    effort: &str,
) -> (Ledger, CodexReviewRuntime, AtomicBool) {
    let runtime = runtime();
    let grant = workspace::save_retrieval_grant(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
        &runtime,
    )
    .unwrap();
    let ledger = Ledger::new_progressive(
        &f.storage,
        &Uuid::new_v4().to_string(),
        &f.manifest,
        &grant,
        &runtime,
        model,
        effort,
    )
    .unwrap();
    (ledger, runtime, AtomicBool::new(false))
}

#[test]
fn selected_model_and_effort_are_bound_to_saved_workspace_answers() {
    let f = Fixture::new();
    let (ledger, runtime, cancelled) = setup_with_model(&f, "gpt-5.6-luna", "medium");
    assert_eq!(ledger.record.binding.model, "gpt-5.6-luna");
    assert_eq!(ledger.record.binding.effort, "medium");
    let operation_id = ledger.record.binding.operation_id.clone();
    ledger
        .start_narration_answer_sections(
            "How does greeting produce its result?",
            &"d".repeat(64),
            &f.manifest,
            &runtime,
            &cancelled,
        )
        .unwrap();

    let answer = serde_json::to_value(sections::inspect(&f.storage, &operation_id).unwrap())
        .unwrap();
    assert_eq!(answer["binding"]["model"], "gpt-5.6-luna");
    assert_eq!(answer["binding"]["effort"], "medium");
}

fn request(f: &Fixture, start_byte: usize, end_byte: usize) -> RangeRequest {
    RangeRequest {
        path: SIMPLE_PATH.into(),
        source_sha256: f
            .manifest
            .files
            .iter()
            .find(|file| file.path == SIMPLE_PATH)
            .unwrap()
            .sha256
            .clone(),
        start_byte,
        end_byte,
    }
}

fn read(
    ledger: &mut Ledger,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
    ranges: &[RangeRequest],
) -> Value {
    let call = ledger
        .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, runtime, cancel)
        .unwrap();
    let mut result = Value::Null;
    ledger
        .deliver(
            call,
            Context::Read {},
            ranges,
            &f.manifest,
            runtime,
            cancel,
            |bytes| {
                result = serde_json::from_slice(bytes).unwrap();
                Ok(())
            },
        )
        .unwrap();
    result
}

fn citation_response(reference_id: &str) -> String {
    json!({
        "citationVersion":crate::native_citations::SHORT_VERSION,
        "title":"Formatting a greeting",
        "sections":[{"title":"Construct the message",
            "prose":"The `greeting` function formats the supplied name into a message.",
            "spokenText":"The greeting function formats the supplied name into a message.",
            "claims":[{"category":"codeBehavior","aspect":"responsibility",
                "prose":"The `greeting` function formats the supplied name into a message.",
                "inferenceBasis":"","evidence":[{"referenceId":reference_id}],"relationshipIds":[]}],
            "visual":{"kind":"none","title":"","direction":"auto","nodes":[],"edges":[]}}]
    })
    .to_string()
}

#[test]
fn current_reads_issue_only_short_operation_references_and_reuse_them() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let first = read(
        &mut ledger,
        &f,
        &runtime,
        &cancel,
        &[request(&f, 0, SIMPLE_SOURCE.len())],
    );
    assert_eq!(first["citationVersion"], crate::native_citations::SHORT_VERSION);
    assert_eq!(first["added"][0]["referenceId"], "code:1");
    assert!(first["added"][0].get("id").is_none());
    assert!(first.get("alreadyAvailable").is_none());
    assert_eq!(first["added"][0]["contents"], SIMPLE_SOURCE);

    let repeated = read(
        &mut ledger,
        &f,
        &runtime,
        &cancel,
        &[request(&f, 0, SIMPLE_SOURCE.len())],
    );
    assert_eq!(repeated["reusedReferences"][0]["referenceId"], "code:1");
    assert!(repeated["reusedReferences"][0].get("id").is_none());
    assert!(repeated.get("alreadyAvailable").is_none());
    assert_eq!(ledger.record.evidence.len(), 1);

    let response = citation_response("code:1");
    assert!(matches!(
        ledger
            .resolve_citations(&response, &f.manifest, &runtime, &cancel)
            .unwrap(),
        crate::native_citations::Resolution::Ready(_)
    ));
    let old_version = response.replace(
        crate::native_citations::SHORT_VERSION,
        "nativeEvidenceV1",
    );
    assert!(matches!(
        ledger
            .resolve_citations(&old_version, &f.manifest, &runtime, &cancel)
            .unwrap(),
        crate::native_citations::Resolution::Invalid(_)
    ));
    let long_alias = response.replace("code:1", &format!("evidence:{}", "f".repeat(64)));
    assert!(matches!(
        ledger
            .resolve_citations(&long_alias, &f.manifest, &runtime, &cancel)
            .unwrap(),
        crate::native_citations::Resolution::Invalid(_)
    ));
}

#[test]
fn current_ledger_records_bind_luna_max_and_require_current_wire_versions() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    assert_eq!(ledger.record.schema_version, LEDGER_SCHEMA_VERSION);
    assert_eq!(ledger.record.binding.model, "gpt-6-luna");
    assert_eq!(ledger.record.binding.effort, "max");
    assert_eq!(ledger.record.context_clock, ContextClock::CapturedCodeSessionV1);

    read(
        &mut ledger,
        &f,
        &runtime,
        &cancel,
        &[request(&f, 0, SIMPLE_SOURCE.len())],
    );
    let current = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(current.schema_version, LEDGER_SCHEMA_VERSION);
    assert_eq!(current.calls[0].state, CallState::Delivered);
    assert!(current.calls[0].timing.finished_ms.is_some());
    assert!(valid_context_timings(&current));
    assert!(
        !fs::read_to_string(&ledger.path)
            .unwrap()
            .contains(SIMPLE_SOURCE)
    );

    for version in [1, 2] {
        let mut old = current.clone();
        old.schema_version = version;
        fs::write(&ledger.path, serde_json::to_vec(&old).unwrap()).unwrap();
        assert!(inspect(&f.storage, &ledger.record.binding.operation_id).is_err());
    }
}

#[test]
fn timing_exports_only_current_native_tool_offsets_and_budget_metadata() {
    let run_started = Instant::now();
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    read(
        &mut ledger,
        &f,
        &runtime,
        &cancel,
        &[request(&f, 0, SIMPLE_SOURCE.len())],
    );
    let timing = ledger.timing_diagnostics(run_started);
    assert!(timing.origin_elapsed_ms.is_some());
    assert_eq!(timing.calls.len(), 1);
    assert_eq!(timing.calls[0].state, CallState::Delivered);
    assert!(timing.calls[0].started_ms <= timing.calls[0].prepared_ms);
    assert!(timing.calls[0].prepared_ms <= timing.calls[0].finished_ms);
    let text = serde_json::to_string(&timing).unwrap();
    for forbidden in [SIMPLE_SOURCE, SIMPLE_PATH, "sha256", "evidence", "arguments", "contents"] {
        assert!(!text.contains(forbidden));
    }
}

#[test]
fn invalid_read_batches_return_fixed_feedback_without_partial_evidence() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let valid = request(&f, 0, SIMPLE_SOURCE.len());
    let mut invalid = valid.clone();
    invalid.source_sha256 = "not-a-source-digest".into();
    let reply = read(&mut ledger, &f, &runtime, &cancel, &[valid, invalid]);
    assert_eq!(reply["readError"]["code"], "invalidRead");
    assert!(reply["added"].as_array().unwrap().is_empty());
    assert!(reply.get("alreadyAvailable").is_none());
    assert!(ledger.admitted_ranges().is_empty());
    assert_eq!(ledger.record.calls.last().unwrap().state, CallState::Delivered);
    let text = reply.to_string();
    for private in [SIMPLE_SOURCE, "not-a-source-digest"] {
        assert!(!text.contains(private));
    }
}

fn comfortable_response_budget() -> ResponseBudget {
    ResponseBudget {
        remaining: RemainingBudget {
            calls: FINALIZATION_CALL_RESERVE + 1,
            files: MAX_FILES,
            source_bytes: 4_000,
            metadata_bytes: 16_000,
            wire_bytes: MAX_WIRE,
            fragments: MAX_FRAGMENTS,
            time_ms: FINALIZATION_TIME_RESERVE.as_millis() as u64,
        },
        feedback_bytes_reserved: 0,
        finish_now: false,
    }
}

#[test]
fn finish_now_preserves_six_calls_or_sixty_seconds_for_final_manifest() {
    let budget = comfortable_response_budget();
    assert!(!requires_finalization_now(false, &budget));

    let mut calls_reserved = comfortable_response_budget();
    calls_reserved.remaining.calls = FINALIZATION_CALL_RESERVE;
    assert!(requires_finalization_now(false, &calls_reserved));

    let mut time_reserved = comfortable_response_budget();
    time_reserved.remaining.time_ms = FINALIZATION_TIME_RESERVE.as_millis() as u64 - 1;
    assert!(requires_finalization_now(false, &time_reserved));

    assert!(requires_finalization_now(true, &budget));
}
