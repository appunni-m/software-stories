use super::*;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("atlas-runtime-events-{}", Uuid::new_v4()));
        ensure_private_directory(&root).unwrap();
        Self(root)
    }
    fn input(&self) -> CodexRunInput {
        CodexRunInput {
            run_id: Uuid::new_v4().to_string(),
            expected_runtime: CodexReviewRuntime {
                connection_id: Uuid::new_v4().to_string(),
                executable_path: "/unused/codex".into(),
                executable_sha256: "a".repeat(64),
                account_identity_sha256: "b".repeat(64),
                account_revision: 1,
            },
            prompt_contract_version: "synthetic-runtime-events-v1".into(),
            model: "gpt-6-luna".into(),
            effort: "max".into(),
            run_deadline: super::super::RunDeadline::BoundedProbe,
            approval_policy: "never".into(),
            system_instruction: "Synthetic protocol fixture, no model.".into(),
            user_prompt: "Synthetic question.".into(),
            output_schema: json!({}),
            correction_policy: crate::claim_alignment::CorrectionPolicy::Disabled,
            retrieval: None,
            sources: vec![],
        }
    }
    fn trace(&self, input: &CodexRunInput, started: Instant) -> Trace {
        Trace::new(
            &self.0,
            &input.run_id,
            &"c".repeat(64),
            &input.expected_runtime,
            "gpt-6-luna",
            "max",
            started,
        )
    }
    fn saved(&self, input: &CodexRunInput) -> Value {
        serde_json::to_value(
            crate::generation_timing::inspect(&self.0, &input.run_id)
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn event(method: &str, mut extra: Value) -> Value {
    extra["threadId"] = json!("owned-thread");
    extra["turnId"] = json!("owned-turn");
    json!({"method":method,"params":extra})
}
fn error(retry: bool, code: Value) -> Value {
    event(
        "error",
        json!({"willRetry":retry,"error":{"message":"NEVER-EXPORT-SOURCE OR CREDENTIALS",
        "additionalDetails":"NEVER-EXPORT-DETAILS","codexErrorInfo":code}}),
    )
}
fn completed() -> Value {
    event(
        "turn/completed",
        json!({"turn":{"id":"owned-turn","status":"completed",
        "items":[{"id":"final","type":"agentMessage","text":"{\"ok\":true}"}]}}),
    )
}
fn usage(total: u64) -> Value {
    let count = |n| {
        json!({"inputTokens":n,"cachedInputTokens":0,"outputTokens":0,
        "reasoningOutputTokens":0,"totalTokens":n})
    };
    json!({"last":count(10),"total":count(total),"modelContextWindow":null,
        "unknownMetadata":"NEVER-EXPORT-METADATA"})
}

// The real turn loop, IO acknowledgements and receive queue run against an
// in-memory protocol peer. No Codex process, account or network is used.
fn process(events: Vec<Value>, before_ack: Vec<Value>) -> (ServerProcess, Arc<Mutex<Vec<Value>>>) {
    let child = Command::new(std::env::current_exe().unwrap())
        .arg("--list")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let (writer_sender, writer_receiver) = mpsc::sync_channel::<WriteFrame>(1);
    let (sender, receiver) = mpsc::sync_channel(32);
    let requests = Arc::new(Mutex::new(vec![]));
    let observed = Arc::clone(&requests);
    let writer = thread::spawn(move || {
        let frame = writer_receiver.recv().unwrap();
        let request: Value = serde_json::from_slice(&frame.bytes).unwrap();
        observed.lock().unwrap().push(request.clone());
        frame.acknowledged.send(Ok(())).unwrap();
        for value in before_ack {
            if sender.send(Ok(value)).is_err() {
                return;
            }
        }
        if sender
            .send(Ok(
                json!({"id":request["id"],"result":{"turn":{"id":"owned-turn"}}}),
            ))
            .is_err()
        {
            return;
        }
        for value in events {
            if sender.send(Ok(value)).is_err() {
                return;
            }
        }
    });
    let process = ServerProcess {
        process_id: Arc::new(AtomicU32::new(child.id())),
        child,
        writer_sender: Some(writer_sender),
        writer: Some(writer),
        receiver: Some(receiver),
        reader: None,
        next_id: 1,
        shutdown: Arc::new(AtomicBool::new(false)),
        cancelled: None,
        executable: "/unused/codex".into(),
        executable_sha256: "a".repeat(64),
        connection_id: Uuid::new_v4().to_string(),
        account_revision: 1,
        scope_epoch: Arc::new(AtomicU64::new(1)),
        reaped: false,
        disabled_mcp: json!({}),
    };
    (process, requests)
}

#[test]
fn terminal_error_ends_the_real_turn_loop_without_waiting_for_completion_or_retry() {
    for early in [false, true] {
        let f = Fixture::new();
        let input = f.input();
        let started = Instant::now();
        let trace = f.trace(&input, started);
        let failure = error(false, json!("usageLimitExceeded"));
        let (mut process, requests) = if early {
            process(vec![completed()], vec![failure])
        } else {
            process(vec![failure, completed()], vec![])
        };
        let result = process.generate_turn(
            &input,
            &f.0,
            "owned-thread",
            "Synthetic prompt",
            &Arc::new(AtomicBool::new(false)),
            started,
            &|_| {},
            &trace,
            1,
        );
        let failure = result.unwrap_err();
        assert!(failure.contains("usage limit"), "{failure}");
        assert!(!failure.contains("NEVER-EXPORT"));
        trace.finish(Err(&failure));
        drop(process);
        assert_eq!(requests.lock().unwrap().len(), 1);
        assert_eq!(requests.lock().unwrap()[0]["method"], "turn/start");
        let saved = f.saved(&input);
        assert_eq!(saved["terminal"]["state"], "failed");
        assert_eq!(
            saved["runtimeTurns"][0]["observations"][0]["kind"],
            "terminalError"
        );
        assert!(!saved.to_string().contains("NEVER-EXPORT"));
    }
}

#[test]
fn retryable_error_keeps_the_same_turn_and_records_only_safe_activity_and_reported_usage() {
    let f = Fixture::new();
    let input = f.input();
    let started = Instant::now();
    let trace = f.trace(&input, started);
    let (mut process, requests) = process(
        vec![
            event(
                "turn/started",
                json!({"turn":{"id":"owned-turn","status":"inProgress"}}),
            ),
            error(
                true,
                json!({"responseStreamDisconnected":{"httpStatusCode":503}}),
            ),
            event(
                "item/reasoning/textDelta",
                json!({"delta":"NEVER-EXPORT-REASONING"}),
            ),
            event("thread/tokenUsage/updated", json!({"tokenUsage":usage(40)})),
            event("thread/tokenUsage/updated", json!({"tokenUsage":usage(80)})),
            completed(),
        ],
        vec![],
    );
    let response = process
        .generate_turn(
            &input,
            &f.0,
            "owned-thread",
            "Synthetic prompt",
            &Arc::new(AtomicBool::new(false)),
            started,
            &|_| {},
            &trace,
            1,
        )
        .unwrap();
    trace.finish(Ok(()));
    drop(process);
    assert_eq!(response.response_text, "{\"ok\":true}");
    assert_eq!(requests.lock().unwrap().len(), 1);
    let saved = f.saved(&input);
    assert_eq!(saved["tokenUsage"]["scope"], "appServerThreadCumulative");
    assert_eq!(saved["tokenUsage"]["snapshot"]["total"]["inputTokens"], 80);
    assert_eq!(saved["runtimeTurns"][0]["observations"][3]["count"], 2);
    assert!(!saved.to_string().contains("NEVER-EXPORT"));
}

#[test]
fn stale_and_unowned_errors_and_usage_cannot_fail_or_annotate_the_current_turn() {
    let f = Fixture::new();
    let input = f.input();
    let started = Instant::now();
    let trace = f.trace(&input, started);
    trace.turn_requested(1);
    for (thread_id, turn_id) in [
        (Some("other-thread"), Some("owned-turn")),
        (Some("owned-thread"), Some("old-turn")),
        (None, Some("owned-turn")),
        (Some("owned-thread"), None),
    ] {
        for mut message in [
            error(false, json!("unauthorized")),
            event("thread/tokenUsage/updated", json!({"tokenUsage":usage(80)})),
        ] {
            let params = message.get_mut("params").unwrap();
            params["threadId"] = json!(thread_id);
            params["turnId"] = json!(turn_id);
            observe(
                &trace,
                1,
                message["method"].as_str().unwrap(),
                &message["params"],
                "owned-thread",
                Some("owned-turn"),
            )
            .unwrap();
        }
    }
    observe(
        &trace,
        1,
        "error",
        &error(false, json!("unauthorized"))["params"],
        "owned-thread",
        None,
    )
    .unwrap();
    trace.finish(Ok(()));
    let saved = f.saved(&input);
    assert!(saved["tokenUsage"].is_null());
    assert!(
        saved["runtimeTurns"][0]["observations"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn events_before_ack_are_replayed_only_after_the_acknowledged_turn_matches() {
    let f = Fixture::new();
    let input = f.input();
    let started = Instant::now();
    let trace = f.trace(&input, started);
    let mut stale_error = error(false, json!("unauthorized"));
    stale_error["params"]["turnId"] = json!("previous-correction-turn");
    let mut stale_usage = event(
        "thread/tokenUsage/updated",
        json!({"tokenUsage":usage(999)}),
    );
    stale_usage["params"]["turnId"] = json!("previous-correction-turn");
    let (mut process, requests) = process(
        vec![completed()],
        vec![
            stale_error,
            stale_usage,
            event(
                "item/reasoning/summaryTextDelta",
                json!({"delta":"NEVER-EXPORT-EARLY-REASONING"}),
            ),
            event("thread/tokenUsage/updated", json!({"tokenUsage":usage(80)})),
        ],
    );
    process
        .generate_turn(
            &input,
            &f.0,
            "owned-thread",
            "Synthetic prompt",
            &Arc::new(AtomicBool::new(false)),
            started,
            &|_| {},
            &trace,
            1,
        )
        .unwrap();
    trace.finish(Ok(()));
    drop(process);
    assert_eq!(requests.lock().unwrap().len(), 1);
    let saved = f.saved(&input);
    assert_eq!(saved["tokenUsage"]["snapshot"]["total"]["inputTokens"], 80);
    let events = saved["runtimeTurns"][0]["observations"].as_array().unwrap();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["kind"], "reasoningActivity");
    assert!(!saved.to_string().contains("NEVER-EXPORT"));
}

#[test]
fn unavailable_diagnostics_cannot_mask_a_terminal_runtime_error() {
    let f = Fixture::new();
    fs::write(f.0.join("generation-timings"), b"existing unrelated file").unwrap();
    let input = f.input();
    let started = Instant::now();
    let trace = f.trace(&input, started);
    let (mut process, requests) = process(
        vec![error(false, json!("contextWindowExceeded")), completed()],
        vec![],
    );
    let failure = process
        .generate_turn(
            &input,
            &f.0,
            "owned-thread",
            "Synthetic prompt",
            &Arc::new(AtomicBool::new(false)),
            started,
            &|_| {},
            &trace,
            1,
        )
        .unwrap_err();
    assert!(failure.contains("context limit"));
    trace.finish(Err(&failure));
    drop(process);
    assert_eq!(requests.lock().unwrap().len(), 1);
    assert_eq!(
        fs::read(f.0.join("generation-timings")).unwrap(),
        b"existing unrelated file"
    );
}

#[test]
fn malformed_usage_stays_unknown_and_error_classification_never_reads_private_message_text() {
    let f = Fixture::new();
    let input = f.input();
    let trace = f.trace(&input, Instant::now());
    trace.turn_requested(1);
    observe(
        &trace,
        1,
        "thread/tokenUsage/updated",
        &event(
            "thread/tokenUsage/updated",
            json!({"tokenUsage":{"totalTokens":80}}),
        )["params"],
        "owned-thread",
        Some("owned-turn"),
    )
    .unwrap();
    let mut unknown = error(false, json!("futureErrorCode"));
    unknown["params"]["error"]["message"] = json!("login quota usage_limit NEVER-EXPORT");
    let failure = observe(
        &trace,
        1,
        "error",
        &unknown["params"],
        "owned-thread",
        Some("owned-turn"),
    )
    .unwrap_err();
    assert_eq!(failure, rpc_failure("generation", &Value::Null));
    trace.finish(Err(&failure));
    assert!(f.saved(&input)["tokenUsage"].is_null());
    for code in [
        json!("usageLimitExceeded"),
        json!("rateLimitExceeded"),
        json!("sessionBudgetExceeded"),
        json!("unauthorized"),
        json!("contextWindowExceeded"),
        json!({"responseStreamDisconnected":{"httpStatusCode":503}}),
    ] {
        let value = json!({"error":{"codexErrorInfo":code,"message":"NEVER-EXPORT"}});
        assert_eq!(
            rpc_failure("turn/completed", &value),
            classified_failure(&value["error"]).unwrap()
        );
    }
    let mut malformed = error(false, json!("other"));
    malformed["params"]
        .as_object_mut()
        .unwrap()
        .remove("willRetry");
    assert_eq!(
        observe(
            &trace,
            1,
            "error",
            &malformed["params"],
            "owned-thread",
            Some("owned-turn")
        )
        .unwrap_err(),
        UPGRADE
    );
}
