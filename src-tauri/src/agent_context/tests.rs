use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH};
use serde_json::json;
use std::{io::Write, os::unix::net::UnixStream, thread, time::Instant};

#[test]
fn search_result_kind_defaults_to_ranges_and_accepts_file_discovery() {
    let arguments = json!({"query":"glyph rendering","scope":null,"limit":8,"offset":0});
    let default: Search = parse(arguments.clone()).unwrap();
    assert!(matches!(default.result_kind, SearchResultKind::Ranges));

    let files: Search = parse(json!({
        "query":"glyph rendering",
        "scope":null,
        "limit":8,
        "offset":0,
        "resultKind":"files"
    }))
    .unwrap();
    assert!(matches!(files.result_kind, SearchResultKind::Files));
}

#[test]
fn transport_setup_diagnostics_exclude_os_messages_and_private_paths() {
    let error = std::io::Error::new(
        std::io::ErrorKind::NotConnected,
        "private /source/account details",
    );
    assert_eq!(
        transport_setup_error("read", &error),
        "Code tool read setup failed (NotConnected, OS unavailable)."
    );
    assert!(
        !transport_setup_error("write", &std::io::Error::from_raw_os_error(57)).contains("/source")
    );
}

#[test]
fn only_literal_matcher_failures_get_correctable_query_feedback() {
    assert_eq!(
        recoverable_search_message(context::INVALID_LITERAL_SEARCH_QUERY),
        Some(INVALID_LITERAL_SEARCH_MESSAGE)
    );
    for terminal in [
        "Code search failed. Try rebuilding its local index.",
        "A code result belongs to a different source version.",
        "The captured source is unavailable.",
    ] {
        assert_eq!(recoverable_search_message(terminal), None);
    }
}

fn session(f: &Fixture) -> Session {
    session_with_model(f, "gpt-6-luna", "max").0
}

fn session_with_model(f: &Fixture, model: &str, effort: &str) -> (Session, String) {
    let runtime = CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 0,
    };
    let grant = crate::workspace::save_retrieval_grant(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
        &runtime,
    )
    .unwrap();
    let operation_id = Uuid::new_v4().to_string();
    let fingerprint = "d".repeat(64);
    let mut session = Session::new_progressive(
        &f.storage,
        &operation_id,
        f.manifest.clone(),
        &runtime,
        &grant,
        RepositoryContextJobs::default(),
        "How does a requested name become the greeting?",
        &fingerprint,
        model,
        effort,
        crate::codex_agent::RunDeadline::TenMinutes,
        Arc::new(|_| {}),
    )
    .unwrap();
    let (_manager, owner, _) =
        crate::codex_agent::section_owner_for_test(&operation_id, &fingerprint, &runtime);
    session.bind_section_owner(owner).unwrap();
    (session, operation_id)
}

#[test]
fn native_session_binds_the_selected_model_and_effort() {
    let f = Fixture::new();
    let (_session, operation_id) = session_with_model(&f, "gpt-5.6-luna", "medium");
    let answer = serde_json::to_value(
        crate::evidence_ledger::sections::inspect(&f.storage, &operation_id).unwrap(),
    )
    .unwrap();
    assert_eq!(answer["binding"]["model"], "gpt-5.6-luna");
    assert_eq!(answer["binding"]["effort"], "medium");
}

#[test]
fn session_exposes_only_the_current_progressive_tool_contract() {
    let f = Fixture::new();
    let s = session(&f);
    let config = s.configuration().unwrap();
    assert_eq!(config["args"][0], "--atlas-answer-mcp");
    assert_eq!(config["enabled_tools"], json!(mcp::tool_names()));
    assert_eq!(config["enabled_tools"].as_array().unwrap().len(), 6);
    assert!(s.checked_tool(mcp::PROPOSAL_TOOL).is_ok());
}

fn event(id: &str, name: &str, args: &Value) -> Value {
    json!({"id":id,"type":"mcpToolCall","server":mcp::SERVER,"tool":name,"arguments":args,"status":"inProgress"})
}
fn exchange(s: &mut Session, name: &str, args: Value) -> (Value, String) {
    exchange_ordered(s, name, args, false)
}
fn exchange_ordered(s: &mut Session, name: &str, args: Value, early: bool) -> (Value, String) {
    let cancelled = s
        .progressive
        .owner
        .as_ref()
        .expect("test session has a publication owner")
        .cancellation_token();
    exchange_owned_ordered(s, name, args, early, cancelled)
}
fn exchange_owned_ordered(
    s: &mut Session,
    name: &str,
    args: Value,
    mut early: bool,
    cancelled: Arc<AtomicBool>,
) -> (Value, String) {
    let started = event(&Uuid::new_v4().to_string(), name, &args);
    if !early {
        s.event(&started, false).unwrap();
    }
    let socket = s.directory.join("context.sock");
    let token = s.token.clone();
    let name = name.to_owned();
    let worker = thread::spawn(move || {
        let mut stream = UnixStream::connect(socket).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let payload = serde_json::to_vec(&mcp::Request {
            token,
            name,
            arguments: args,
        })
        .unwrap();
        // Exercise the actual owner through a split length header and body.
        // Darwin's inherited nonblocking stream used to reject this packet.
        let header = (payload.len() as u32).to_be_bytes();
        stream.write_all(&header[..2]).unwrap();
        thread::sleep(Duration::from_millis(20));
        stream.write_all(&header[2..]).unwrap();
        let split = payload.len() / 2;
        stream.write_all(&payload[..split]).unwrap();
        thread::sleep(Duration::from_millis(20));
        stream.write_all(&payload[split..]).unwrap();
        String::from_utf8(mcp::read_packet(&mut stream, mcp::MAX_REPLY).unwrap()).unwrap()
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    while !worker.is_finished() && Instant::now() < deadline {
        s.poll(&cancelled, || Ok(())).unwrap();
        if early && s.pending.is_some() {
            assert!(s.ledger.admitted_ranges().is_empty());
            assert!(!worker.is_finished());
            s.event(&started, false).unwrap();
            early = false;
        }
        thread::sleep(Duration::from_millis(2));
    }
    let text = worker.join().unwrap();
    let mut completed = started;
    completed["status"] = json!("completed");
    completed["result"] = json!({"content":[{"type":"text","text":text}]});
    s.event(&completed, true).unwrap();
    (completed, text)
}
#[test]
fn helper_can_arrive_before_the_app_server_started_notification() {
    let f = Fixture::new();
    let mut s = session(&f);
    let file = f
        .manifest
        .files
        .iter()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap();
    let (_, text) = exchange_ordered(
        &mut s,
        "read_code",
        json!({"ranges":[{"path":SIMPLE_PATH,"sourceSha256":file.sha256,"startByte":0,"endByte":file.size_bytes}]}),
        true,
    );
    assert!(text.contains("export function greeting"));
    s.check_complete().unwrap();
}

#[test]
fn a_nonce_without_a_started_event_cannot_read_code() {
    let f = Fixture::new();
    let mut s = session(&f);
    let mut stream = UnixStream::connect(s.directory.join("context.sock")).unwrap();
    let req = mcp::Request {
        token: s.token.clone(),
        name: "list_modules".into(),
        arguments: json!({"scope":null,"limit":4,"offset":0}),
    };
    mcp::write_packet(
        &mut stream,
        &serde_json::to_vec(&req).unwrap(),
        mcp::MAX_INPUT,
    )
    .unwrap();
    let cancelled = s
        .progressive
        .owner
        .as_ref()
        .expect("test session has a publication owner")
        .cancellation_token();
    s.poll(&cancelled, || Ok(())).unwrap();
    assert!(s.pending.is_some());
    assert!(s.ledger.admitted_ranges().is_empty());
    s.pending.as_mut().unwrap().received_at = Instant::now() - Duration::from_secs(4);
    assert!(s.poll(&cancelled, || Ok(())).is_err());
    assert!(s.pending.is_none());
}
#[test]
fn owned_mcp_search_read_and_result_events_share_native_evidence() {
    let f = Fixture::new();
    let mut s = session(&f);
    let (_, text) = exchange(
        &mut s,
        "search_code",
        json!({"query":"greeting","scope":null,"limit":4}),
    );
    let result: Value = serde_json::from_str(&text).unwrap();
    let range = &result["context"]["result"]["matches"][0]["range"];
    assert_eq!(range["path"], SIMPLE_PATH);
    assert!(
        result["context"]["result"]["matches"][0]
            .get("text")
            .is_none()
    );
    let (completed, text) = exchange(
        &mut s,
        "read_code",
        json!({"ranges":[{"path":range["path"],"sourceSha256":range["sourceSha256"],"startByte":range["startByte"],"endByte":range["endByte"]}]}),
    );
    assert!(text.contains("export function greeting"));
    assert!(!s.ledger.admitted_ranges().is_empty());
    s.check_complete().unwrap();
    assert!(s.event(&completed, true).is_err());
}
#[test]
fn invalid_range_returns_no_code_and_an_agent_can_correct_it() {
    let f = Fixture::new();
    let mut s = session(&f);
    let file = f
        .manifest
        .files
        .iter()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap();
    let (_, text) = exchange(
        &mut s,
        "read_code",
        json!({"ranges":[{"path":SIMPLE_PATH,"sourceSha256":file.sha256,"startByte":0,"endByte":32768}]}),
    );
    let result: Value = serde_json::from_str(&text).unwrap();
    assert!(result["added"].as_array().unwrap().is_empty());
    assert_eq!(result["readError"]["code"], "invalidRead");
    assert!(s.ledger.admitted_ranges().is_empty());
    let (_, text) = exchange(
        &mut s,
        "read_code",
        json!({"ranges":[{"path":SIMPLE_PATH,"sourceSha256":file.sha256,"startByte":0,"endByte":file.size_bytes}]}),
    );
    assert!(text.contains("export function greeting"));
    assert!(!s.ledger.admitted_ranges().is_empty());
    s.check_complete().unwrap();
}

#[test]
fn malformed_typed_read_arguments_return_owned_feedback_before_a_corrected_read() {
    let f = Fixture::new();
    let mut s = session(&f);
    let file = f
        .manifest
        .files
        .iter()
        .find(|file| file.path == SIMPLE_PATH)
        .unwrap();
    let valid = json!({"path":SIMPLE_PATH,"sourceSha256":file.sha256,"startByte":0,"endByte":file.size_bytes});
    for arguments in [
        Value::Null,
        json!({}),
        json!({"ranges":"secret_descriptor_payload"}),
        json!({"ranges":[{"path":SIMPLE_PATH,"sourceSha256":file.sha256,"startByte":"0","endByte":file.size_bytes}]}),
        json!({"ranges":[{"path":SIMPLE_PATH,"sourceSha256":file.sha256,"startByte":-1,"endByte":file.size_bytes}]}),
        json!({"ranges":[valid.clone()],"extra":"secret_descriptor_payload"}),
    ] {
        let (_, text) = exchange(&mut s, "read_code", arguments);
        let result: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(result["readError"]["code"], "invalidReadArguments");
        assert!(result["added"].as_array().unwrap().is_empty());
        assert!(s.ledger.admitted_ranges().is_empty());
        assert!(!text.contains("secret_descriptor_payload"));
        assert!(!text.contains("export function greeting"));
    }
    let (_, text) = exchange(&mut s, "read_code", json!({"ranges":[valid]}));
    assert!(text.contains("export function greeting"));
    assert!(!s.ledger.admitted_ranges().is_empty());
    s.check_complete().unwrap();
}
#[test]
fn invalid_read_batches_return_owned_feedback_then_allow_a_corrected_read() {
    let f = Fixture::new();
    let mut s = session(&f);
    let file = f
        .manifest
        .files
        .iter()
        .find(|file| file.path == SIMPLE_PATH)
        .unwrap();
    let range = json!({"path":SIMPLE_PATH,"sourceSha256":file.sha256,"startByte":0,"endByte":file.size_bytes});
    for ranges in [vec![], vec![range.clone(); 9]] {
        let (_, text) = exchange(&mut s, "read_code", json!({"ranges":ranges}));
        let result: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(result["readError"]["code"], "invalidRangeCount");
        assert!(result["added"].as_array().unwrap().is_empty());
        assert!(s.ledger.admitted_ranges().is_empty());
    }
    let (_, corrected) = exchange(&mut s, "read_code", json!({"ranges":[range]}));
    assert!(corrected.contains("export function greeting"));
    s.check_complete().unwrap();
}

#[test]
fn malformed_read_descriptors_keep_the_owned_tool_exchange_correctable() {
    let f = Fixture::new();
    let mut s = session(&f);
    let file = f
        .manifest
        .files
        .iter()
        .find(|file| file.path == SIMPLE_PATH)
        .unwrap();
    let range = json!({"path":SIMPLE_PATH,"sourceSha256":file.sha256,"startByte":0,"endByte":file.size_bytes});
    for (field, value) in [
        ("path", "../private-source.ts"),
        ("sourceSha256", "not-a-source-digest"),
        ("path", "uncaptured-source.ts"),
    ] {
        let mut invalid = range.clone();
        invalid[field] = json!(value);
        let (_, text) = exchange(
            &mut s,
            "read_code",
            json!({"ranges":[range.clone(), invalid]}),
        );
        let reply: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(reply["readError"]["code"], "invalidRead");
        assert!(reply["added"].as_array().unwrap().is_empty());
        assert!(!text.contains(value));
        assert!(!text.contains(crate::chapter_generation::tests::SIMPLE_SOURCE));
        assert!(s.ledger.admitted_ranges().is_empty());
    }
    let (_, corrected) = exchange(&mut s, "read_code", json!({"ranges":[range]}));
    assert!(corrected.contains("export function greeting"));
    assert!(!s.ledger.admitted_ranges().is_empty());
    s.check_complete().unwrap();
}

#[test]
fn excessive_agent_page_size_is_bounded_instead_of_aborting() {
    let f = Fixture::new();
    let mut s = session(&f);
    for (name, args) in [
        ("list_modules", json!({"scope":".","limit":50,"offset":0})),
        (
            "find_definitions",
            json!({"query":"greeting","scope":null,"limit":usize::MAX,"offset":0}),
        ),
        (
            "search_code",
            json!({"query":"greeting","scope":null,"limit":0}),
        ),
    ] {
        let (_, text) = exchange(&mut s, name, args);
        let result: Value = serde_json::from_str(&text).unwrap();
        let records = result
            .pointer("/context/result/items")
            .or_else(|| result.pointer("/context/result/matches"))
            .unwrap()
            .as_array()
            .unwrap();
        assert!(!records.is_empty() && records.len() <= 32);
    }
    s.check_complete().unwrap();
}
#[test]
fn invalid_query_returns_bounded_feedback_and_can_be_corrected() {
    let f = Fixture::new();
    let mut s = session(&f);
    for query in [
        "x".to_owned(),
        "\nprivate query".to_owned(),
        "x".repeat(241),
        "private\tquery".to_owned(),
        (0..33)
            .map(|i| format!("term{i}"))
            .collect::<Vec<_>>()
            .join(" "),
    ] {
        let (_, text) = exchange(
            &mut s,
            "search_code",
            json!({"query":query,"scope":null,"limit":4}),
        );
        let result: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(result["context"]["kind"], "searchError");
        assert_eq!(result["context"]["result"]["code"], "invalidQuery");
        assert!(result["added"].as_array().unwrap().is_empty());
        assert!(!text.contains("private query"));
        assert!(text.len() < 1536);
        assert!(s.ledger.admitted_ranges().is_empty());
    }
    let (_, text) = exchange(
        &mut s,
        "search_code",
        json!({"query":"greeting(","scope":null,"limit":4}),
    );
    let result: Value = serde_json::from_str(&text).unwrap();
    assert!(
        !result["context"]["result"]["matches"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    s.check_complete().unwrap();
}

#[test]
fn literal_punctuation_is_a_valid_bounded_agent_search() {
    let f = Fixture::new();
    let mut s = session(&f);
    for query in ["{}", "::", "()"] {
        let (_, text) = exchange(
            &mut s,
            "search_code",
            json!({"query":query,"scope":null,"limit":4}),
        );
        let result: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(result["context"]["kind"], "search");
        assert!(
            result["context"]["result"]["matches"]
                .as_array()
                .unwrap()
                .len()
                <= 4
        );
    }
    s.check_complete().unwrap();
}

#[test]
fn unsafe_scope_is_not_a_correctable_query_error() {
    let f = Fixture::new();
    let mut s = session(&f);
    let args = json!({"query":"x","scope":"../private","limit":4});
    s.event(&event("unsafe", "search_code", &args), false)
        .unwrap();
    let mut stream = UnixStream::connect(s.directory.join("context.sock")).unwrap();
    mcp::write_packet(
        &mut stream,
        &serde_json::to_vec(&mcp::Request {
            token: s.token.clone(),
            name: "search_code".into(),
            arguments: args,
        })
        .unwrap(),
        mcp::MAX_INPUT,
    )
    .unwrap();
    let cancelled = s
        .progressive
        .owner
        .as_ref()
        .expect("test session has a publication owner")
        .cancellation_token();
    assert!(
        s.poll(&cancelled, || Ok(()))
            .unwrap_err()
            .contains("captured file or folder")
    );
    assert!(s.ledger.admitted_ranges().is_empty());
    assert!(s.check_complete().is_err());
}

#[test]
fn code_punctuation_in_search_is_literal_instead_of_a_regex() {
    let f = Fixture::new();
    let mut s = session(&f);
    for query in ["greeting(", "greeting[", "greeting\\", "greeting.*"] {
        let (_, text) = exchange(
            &mut s,
            "search_code",
            json!({"query":query,"scope":null,"limit":4}),
        );
        let result: Value = serde_json::from_str(&text).unwrap();
        let matches = result
            .pointer("/context/result/matches")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(!matches.is_empty(), "no candidates for {query}");
        assert!(result["added"].as_array().unwrap().is_empty());
    }
    s.check_complete().unwrap();
}

#[test]
fn foreign_tools_forged_results_and_changed_arguments_are_rejected() {
    let f = Fixture::new();
    let mut s = session(&f);
    let a = json!({"scope":null,"limit":4,"offset":0});
    let mut unknown = event("x", "shell", &json!({}));
    assert!(s.event(&unknown, false).is_err());
    unknown = event("x", "list_modules", &a);
    unknown["server"] = json!("inherited");
    assert!(s.event(&unknown, false).is_err());
    let started = event("good", "list_modules", &a);
    s.event(&started, false).unwrap();
    let mut forged = started.clone();
    forged["status"] = json!("completed");
    forged["result"] = json!({"content":[{"type":"text","text":"forged evidence"}]});
    assert!(s.event(&forged, true).is_err());
    let started = event("args", "list_modules", &a);
    s.event(&started, false).unwrap();
    let mut changed = started;
    changed["arguments"]["scope"] = json!("other");
    changed["status"] = json!("completed");
    assert!(s.event(&changed, true).is_err());
}
#[test]
fn bad_nonce_and_extra_fields_cannot_read_source() {
    let f = Fixture::new();
    let mut s = session(&f);
    let args = json!({"query":"greeting","scope":null,"limit":4,"cwd":"/live"});
    s.event(&event("bad", "search_code", &args), false).unwrap();
    let mut stream = UnixStream::connect(s.directory.join("context.sock")).unwrap();
    let req = mcp::Request {
        token: s.token.clone(),
        name: "search_code".into(),
        arguments: args,
    };
    mcp::write_packet(
        &mut stream,
        &serde_json::to_vec(&req).unwrap(),
        mcp::MAX_INPUT,
    )
    .unwrap();
    let cancelled = s
        .progressive
        .owner
        .as_ref()
        .expect("test session has a publication owner")
        .cancellation_token();
    assert!(s.poll(&cancelled, || Ok(())).is_err());
    assert!(s.ledger.admitted_ranges().is_empty());
    let mut s = session(&f);
    let mut stream = UnixStream::connect(s.directory.join("context.sock")).unwrap();
    stream.write_all(&[255, 255, 255, 255]).unwrap();
    let cancelled = s
        .progressive
        .owner
        .as_ref()
        .expect("test session has a publication owner")
        .cancellation_token();
    assert!(s.poll(&cancelled, || Ok(())).is_err());
}
#[test]
fn private_socket_is_removed_and_catalog_cannot_include_inherited_tools() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let s = session(&f);
    let directory = s.directory.clone();
    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let own = json!({"name":mcp::SERVER,"tools":mcp::tools().into_iter().map(|t|(t.name.to_string(),serde_json::to_value(t).unwrap())).collect::<serde_json::Map<_,_>>()});
    s.verify_catalog(&json!({"data":[own.clone()]})).unwrap();
    assert!(
        s.verify_catalog(&json!({"data":[own,{"name":"other","tools":{"shell":{}}}]}))
            .is_err()
    );
    drop(s);
    assert!(!directory.exists());
    assert_eq!(
        arguments_digest(&json!({"b":2,"a":{"d":1,"c":2}})).unwrap(),
        arguments_digest(&json!({"a":{"c":2,"d":1},"b":2})).unwrap()
    );
}

#[test]
fn progressive_proposals_publish_before_the_reply_and_keep_sections_after_a_later_rejection() {
    use crate::evidence_ledger::sections;
    use std::sync::Mutex;
    let f = Fixture::new();
    let planning = session(&f);
    let runtime = planning.runtime.clone();
    let grant = crate::workspace::save_retrieval_grant(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
        &runtime,
    )
    .unwrap();
    let run_id = Uuid::new_v4().to_string();
    let fingerprint = "d".repeat(64);
    let notices = Arc::new(Mutex::new(Vec::new()));
    let output = Arc::clone(&notices);
    let storage = f.storage.clone();
    let mut s = Session::new_progressive(
        &f.storage,
        &run_id,
        f.manifest.clone(),
        &runtime,
        &grant,
        RepositoryContextJobs::default(),
        "How is the greeting created?",
        &fingerprint,
        "gpt-6-luna",
        "max",
        crate::codex_agent::RunDeadline::TenMinutes,
        Arc::new(move |notice| {
            // The callback can independently inspect an already durable section;
            // it is never given an uncommitted preview or raw source payload.
            let saved = sections::inspect(&storage, &notice.run_id).unwrap();
            assert_eq!(serde_json::to_value(&saved).unwrap()["schemaVersion"], 7);
            assert!(
                saved
                    .sections
                    .iter()
                    .any(|p| p.preview.generation_id == notice.generation_id)
            );
            output.lock().unwrap().push(notice);
        }),
    )
    .unwrap();
    assert!(
        s.poll(&Arc::new(AtomicBool::new(false)), || Ok(()))
            .is_err()
    );
    let (manager, owner, cancel) =
        crate::codex_agent::section_owner_for_test(&run_id, &fingerprint, &runtime);
    let mut changed_runtime = runtime.clone();
    changed_runtime.account_revision += 1;
    let (_, foreign, _) =
        crate::codex_agent::section_owner_for_test(&run_id, &fingerprint, &changed_runtime);
    assert!(s.bind_section_owner(foreign).is_err());
    s.bind_section_owner(owner.clone()).unwrap();
    assert!(s.bind_section_owner(owner).is_err());
    assert!(s.poll(&cancel, || Ok(())).is_ok());
    let file = f
        .manifest
        .files
        .iter()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap();
    let (_, read_reply) = exchange_owned_ordered(
        &mut s,
        "read_code",
        json!({"ranges":[{"path":SIMPLE_PATH,
        "sourceSha256":file.sha256,"startByte":0,"endByte":file.size_bytes}]}),
        false,
        Arc::clone(&cancel),
    );
    let read_reply: Value = serde_json::from_str(&read_reply).unwrap();
    let reference = read_reply["added"][0]["referenceId"].as_str().unwrap();
    let mut raw = sections::authoring::tests::proposal(reference, "entry", 0);
    raw["proposalVersion"] = json!(sections::NARRATION_PROPOSAL_VERSION);
    let raw = raw.to_string();
    let args: Value = serde_json::from_str(&raw).unwrap();
    let mut wrong_order = args.clone();
    wrong_order["order"] = json!(5);
    let (_, rejected) = exchange_owned_ordered(
        &mut s,
        mcp::PROPOSAL_TOOL,
        wrong_order,
        false,
        Arc::clone(&cancel),
    );
    let rejected: Value = serde_json::from_str(&rejected).unwrap();
    assert_eq!(
        rejected["context"]["result"]["findings"][0]["code"],
        "sectionOrder"
    );
    assert_eq!(rejected["context"]["result"]["nextNewSectionOrder"], 0);
    assert_eq!(rejected["context"]["result"]["remainingAttempts"], 7);
    assert_eq!(rejected["context"]["result"]["remainingCorrections"], 2);
    assert!(notices.lock().unwrap().is_empty());
    let mut out_of_bounds = args.clone();
    out_of_bounds["sectionKey"] = json!("another-key");
    out_of_bounds["order"] = json!(6);
    let (_, rejected) = exchange_owned_ordered(
        &mut s,
        mcp::PROPOSAL_TOOL,
        out_of_bounds,
        false,
        Arc::clone(&cancel),
    );
    let rejected: Value = serde_json::from_str(&rejected).unwrap();
    assert_eq!(
        rejected["context"]["result"]["findings"][0]["code"],
        "proposalShape"
    );
    assert_eq!(rejected["context"]["result"]["nextNewSectionOrder"], 0);
    assert_eq!(rejected["context"]["result"]["remainingAttempts"], 6);
    assert_eq!(rejected["context"]["result"]["remainingCorrections"], 2);
    assert!(notices.lock().unwrap().is_empty());
    let (_, text) = exchange_owned_ordered(
        &mut s,
        mcp::PROPOSAL_TOOL,
        args.clone(),
        false,
        Arc::clone(&cancel),
    );
    let response: Value = serde_json::from_str(&text).unwrap();
    let generation = response["context"]["result"]["generationId"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        response["context"]["result"]["findings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(response["context"]["result"]["remainingAttempts"], 5);
    assert_eq!(response["context"]["result"]["remainingCorrections"], 2);
    assert_eq!(response["context"]["result"]["nextNewSectionOrder"], 1);
    assert_eq!(notices.lock().unwrap()[0].generation_id, generation);
    assert!(notices.lock().unwrap()[0].run_elapsed_ms.is_some());
    // Same owned proposal after a lost/ignored reply reuses the durable identity.
    let (_, text) =
        exchange_owned_ordered(&mut s, mcp::PROPOSAL_TOOL, args, false, Arc::clone(&cancel));
    assert_eq!(
        serde_json::from_str::<Value>(&text).unwrap()["context"]["result"]["generationId"],
        generation
    );
    let mut bad = sections::authoring::tests::proposal(reference, "deeper", 1);
    bad["proposalVersion"] = json!(sections::NARRATION_PROPOSAL_VERSION);
    bad["section"]["claims"][0]["evidence"][0]["referenceId"] = json!("code:999");
    let (_, text) =
        exchange_owned_ordered(&mut s, mcp::PROPOSAL_TOOL, bad, false, Arc::clone(&cancel));
    assert!(
        serde_json::from_str::<Value>(&text).unwrap()["context"]["result"]["generationId"]
            .is_null()
    );
    s.check_complete().unwrap();
    assert_eq!(
        sections::inspect(&f.storage, &run_id)
            .unwrap()
            .sections
            .len(),
        1
    );
    let ledger: Value = serde_json::from_slice(
        &fs::read(
            f.storage
                .join("evidence-ledgers")
                .join(format!("{run_id}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(ledger["calls"].as_array().unwrap().len(), 6);
    // Simulate an observed scope change after durable commit, before notice or
    // reply. The section is inspectable, but no successful callback or reply
    // may escape and no automatic inference/tool replay is started.
    let mut late = sections::authoring::tests::proposal(reference, "late", 1);
    late["proposalVersion"] = json!(sections::NARRATION_PROPOSAL_VERSION);
    let started = event(&Uuid::new_v4().to_string(), mcp::PROPOSAL_TOOL, &late);
    s.event(&started, false).unwrap();
    let mut peer = UnixStream::connect(s.directory.join("context.sock")).unwrap();
    peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    mcp::write_packet(
        &mut peer,
        &serde_json::to_vec(&mcp::Request {
            token: s.token.clone(),
            name: mcp::PROPOSAL_TOOL.into(),
            arguments: late,
        })
        .unwrap(),
        mcp::MAX_PROPOSAL_INPUT,
    )
    .unwrap();
    let before_notices = notices.lock().unwrap().len();
    assert!(
        s.poll(&cancel, || {
            if sections::inspect(&f.storage, &run_id)?.sections.len() > 1 {
                Err("Synthetic scope change after persistence".into())
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    assert!(mcp::read_packet(&mut peer, mcp::MAX_REPLY).is_err());
    assert_eq!(notices.lock().unwrap().len(), before_notices);
    assert_eq!(
        sections::inspect(&f.storage, &run_id)
            .unwrap()
            .sections
            .len(),
        2
    );
    assert!(manager.cancel(&run_id));
    assert!(s.poll(&cancel, || Ok(())).is_err());
    assert_eq!(
        sections::inspect(&f.storage, &run_id).unwrap().sections[0]
            .preview
            .generation_id,
        generation
    );
}
