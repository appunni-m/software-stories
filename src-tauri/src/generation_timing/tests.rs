use super::*;
use crate::codex_identity::CodexReviewRuntime;
use serde_json::{Value, json};
use std::time::Duration;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("atlas-timing-test-{}", uuid::Uuid::new_v4()));
        codex_agent::ensure_private_directory(&root).unwrap();
        Self(root)
    }
    fn trace(&self, run: &str, started: Instant) -> Trace {
        Trace::new(
            &self.0,
            run,
            &"c".repeat(64),
            &runtime(),
            "gpt-6-luna",
            "max",
            started,
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
fn run() -> String {
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
        ("schemaVersion", json!(2)),
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
