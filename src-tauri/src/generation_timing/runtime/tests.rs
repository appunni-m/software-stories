use super::*;
use crate::generation_timing::tests::{Fixture, run};
use serde_json::{Value, json};

fn usage(total: u64) -> Value {
    let breakdown = |n| {
        json!({"inputTokens":n,"cachedInputTokens":0,"outputTokens":0,
        "reasoningOutputTokens":0,"totalTokens":n})
    };
    json!({"last":breakdown(10),"total":breakdown(total),"modelContextWindow":null})
}

#[test]
fn frequent_notifications_are_throttled_and_finalization_flushes_all_counts() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now());
    trace.turn_requested(1);
    let file = path(&f.0, &id).unwrap();
    let before = fs::read(&file).unwrap();
    trace.runtime_event(1, RuntimeEventKind::ReasoningActivity);
    assert_eq!(fs::read(&file).unwrap(), before);
    trace.state.lock().unwrap().as_mut().unwrap().last_write =
        Instant::now() - Duration::from_secs(2);
    trace.runtime_event(1, RuntimeEventKind::ReasoningActivity);
    assert_eq!(
        inspect(&f.0, &id).unwrap().unwrap().runtime_turns[0].observations[0].count,
        2
    );
    trace.runtime_event(1, RuntimeEventKind::ReasoningActivity);
    trace.finish(Err("synthetic failure"));
    let saved = inspect(&f.0, &id).unwrap().unwrap();
    assert_eq!(saved.runtime_turns[0].observations[0].count, 3);
    assert_eq!(saved.terminal.unwrap().state, Terminal::Failed);
}

#[test]
fn runtime_counters_keep_correction_separate_and_never_sum_cumulative_usage() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now());
    trace.turn_requested(1);
    trace.runtime_event(1, RuntimeEventKind::TurnStarted);
    for _ in 0..20_000 {
        trace.runtime_event(1, RuntimeEventKind::ReasoningActivity);
    }
    for total in [40, 80] {
        trace.runtime_event(1, RuntimeEventKind::TokenUsageUpdated);
        trace.reported_usage(
            1,
            TokenUsageSnapshot::from_notification(&usage(total)).unwrap(),
        );
    }
    trace.turn_requested(2);
    trace.runtime_event(2, RuntimeEventKind::TurnStarted);
    trace.runtime_event(2, RuntimeEventKind::TokenUsageUpdated);
    trace.reported_usage(
        2,
        TokenUsageSnapshot::from_notification(&usage(120)).unwrap(),
    );
    trace.finish(Ok(()));
    let record = inspect(&f.0, &id).unwrap().unwrap();
    assert_eq!(record.schema_version, 2);
    assert_eq!(record.runtime_turns.len(), 2);
    assert_eq!(record.runtime_turns[0].observations[1].count, 20_000);
    assert_eq!(record.runtime_turns[0].observations.len(), 3);
    assert_eq!(
        record.runtime_turns[0]
            .token_usage
            .as_ref()
            .unwrap()
            .snapshot
            .total
            .input_tokens,
        80
    );
    assert_eq!(
        record
            .token_usage
            .as_ref()
            .unwrap()
            .snapshot
            .total
            .input_tokens,
        120
    );
    assert_eq!(record.token_usage.as_ref().unwrap().turn_index, 2);
    assert!(fs::metadata(path(&f.0, &id).unwrap()).unwrap().len() < 4096);
    let before = fs::read(path(&f.0, &id).unwrap()).unwrap();
    trace.runtime_event(2, RuntimeEventKind::TerminalError);
    assert_eq!(fs::read(path(&f.0, &id).unwrap()).unwrap(), before);
}

#[test]
fn token_usage_accepts_only_reported_nonnegative_protocol_numbers() {
    let value = usage(80);
    let parsed = TokenUsageSnapshot::from_notification(&value).unwrap();
    assert_eq!(parsed.last.cache_write_input_tokens, 0);
    for (path, bad) in [
        ("/total/inputTokens", json!(-1)),
        ("/last/reasoningOutputTokens", json!(1.5)),
        ("/last/totalTokens", json!("80")),
        ("/last/cacheWriteInputTokens", json!(null)),
        ("/modelContextWindow", json!(u64::MAX)),
    ] {
        let mut invalid = value.clone();
        if path == "/last/cacheWriteInputTokens" {
            invalid["last"]["cacheWriteInputTokens"] = bad;
        } else {
            *invalid.pointer_mut(path).unwrap() = bad;
        }
        assert!(
            TokenUsageSnapshot::from_notification(&invalid).is_none(),
            "{path}"
        );
    }
    let mut incomplete = value.clone();
    incomplete["last"]
        .as_object_mut()
        .unwrap()
        .remove("cachedInputTokens");
    assert!(TokenUsageSnapshot::from_notification(&incomplete).is_none());
    let mut extra = value;
    extra["modelReasoning"] = json!("NEVER-EXPORT-REASONING");
    let encoded =
        serde_json::to_string(&TokenUsageSnapshot::from_notification(&extra).unwrap()).unwrap();
    assert!(!encoded.contains("NEVER-EXPORT"));
}

#[test]
fn unsupported_diagnostics_are_rejected_without_rewriting() {
    let f = Fixture::new();
    let id = run();
    f.trace(&id, Instant::now()).finish(Ok(()));
    let file = path(&f.0, &id).unwrap();
    let mut value: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    value["schemaVersion"] = json!(1);
    value.as_object_mut().unwrap().remove("runtimeTurns");
    let original = serde_json::to_vec(&value).unwrap();
    fs::write(&file, &original).unwrap();
    assert!(inspect(&f.0, &id).is_err());
    assert_eq!(fs::read(&file).unwrap(), original);
}

#[test]
fn diagnostic_validation_rejects_unbounded_unordered_or_invented_runtime_data() {
    let f = Fixture::new();
    let id = run();
    let trace = f.trace(&id, Instant::now());
    trace.turn_requested(1);
    trace.runtime_event(1, RuntimeEventKind::TokenUsageUpdated);
    trace.reported_usage(
        1,
        TokenUsageSnapshot::from_notification(&usage(80)).unwrap(),
    );
    trace.finish(Ok(()));
    let file = path(&f.0, &id).unwrap();
    let original: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    for (path, bad) in [
        ("/schemaVersion", json!(1)),
        ("/runtimeTurns/0/index", json!(2)),
        ("/runtimeTurns/0/requestedMs", json!(MAX_ELAPSED + 1)),
        ("/runtimeTurns/0/observations/0/count", json!(0)),
        (
            "/runtimeTurns/0/observations/0/lastMs",
            json!(MAX_ELAPSED + 1),
        ),
        (
            "/runtimeTurns/0/observations/0/kind",
            json!("private source"),
        ),
        (
            "/runtimeTurns/0/tokenUsage/observedMs",
            json!(MAX_ELAPSED + 1),
        ),
        ("/runtimeTurns/0/tokenUsage/scope", json!("estimated")),
        (
            "/runtimeTurns/0/tokenUsage/snapshot/total/inputTokens",
            json!(u64::MAX),
        ),
        ("/tokenUsage", json!(null)),
    ] {
        let mut invalid = original.clone();
        *invalid.pointer_mut(path).unwrap() = bad;
        fs::write(&file, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(inspect(&f.0, &id).is_err(), "{path}");
    }
    let mut invalid = original.clone();
    invalid["runtimeTurns"][0]["observations"]
        .as_array_mut()
        .unwrap()
        .push(original["runtimeTurns"][0]["observations"][0].clone());
    fs::write(&file, serde_json::to_vec(&invalid).unwrap()).unwrap();
    assert!(inspect(&f.0, &id).is_err());
    fs::write(&file, serde_json::to_vec(&original).unwrap()).unwrap();
}
