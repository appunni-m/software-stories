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
    let r = runtime();
    let grant = workspace::save_retrieval_grant(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
        &r,
    )
    .unwrap();
    let ledger = Ledger::new(
        &f.storage,
        &Uuid::new_v4().to_string(),
        &f.manifest,
        &grant,
        &r,
        "gpt-6-luna",
        "max",
    )
    .unwrap();
    (ledger, r, AtomicBool::new(false))
}
fn request(f: &Fixture, from: usize, to: usize) -> RangeRequest {
    RangeRequest {
        path: SIMPLE_PATH.into(),
        source_sha256: f
            .manifest
            .files
            .iter()
            .find(|s| s.path == SIMPLE_PATH)
            .unwrap()
            .sha256
            .clone(),
        start_byte: from,
        end_byte: to,
    }
}
fn read(
    l: &mut Ledger,
    f: &Fixture,
    r: &CodexReviewRuntime,
    c: &AtomicBool,
    ranges: &[RangeRequest],
) -> Value {
    let call = l
        .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, r, c)
        .unwrap();
    let mut result = Value::Null;
    l.deliver(call, Context::Read {}, ranges, &f.manifest, r, c, |bytes| {
        result = serde_json::from_slice(bytes).unwrap();
        Ok(())
    })
    .unwrap();
    result
}

#[test]
fn quote_diagnostics_check_exact_unique_admitted_utf8_without_transmitting_source() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    read(&mut l, &f, &r, &c, &[request(&f, 0, 40)]);
    let before = serde_json::to_value(&l.record).unwrap();
    for (path, quote, expected) in [
        (
            SIMPLE_PATH,
            "// greeting formats UTF-8 text such as café.",
            Some("quoteNotAdmitted"),
        ),
        (
            SIMPLE_PATH,
            "export function greeting(name: string)",
            Some("quoteNotAdmitted"),
        ),
        (
            SIMPLE_PATH,
            "export  function greeting(name: string)",
            Some("quoteNotExact"),
        ),
        (SIMPLE_PATH, "// marker repeats.", Some("quoteAmbiguous")),
        (SIMPLE_PATH, "aaaaaaaa", Some("quoteAmbiguous")),
        (SIMPLE_PATH, "greet", Some("quoteLength")),
        (
            "src/unread.ts",
            "export function greeting(name: string)",
            Some("pathNotAdmitted"),
        ),
        (SIMPLE_PATH, "// greeting formats UTF-8 text", None),
    ] {
        let response =
            json!({"sections":[{"claims":[{"evidence":[{"path":path,"quote":quote}]}]}]})
                .to_string();
        let issues = l
            .quote_diagnostics(&response, &f.manifest, &r, &c)
            .unwrap()
            .unwrap();
        assert_eq!(issues.len(), usize::from(expected.is_some()), "{quote}");
        if let Some(code) = expected {
            assert_eq!(issues[0]["quoteProblem"], code);
            assert_eq!(issues[0]["location"], "/sections/0/claims/0/evidence/0");
            let encoded = serde_json::to_string(&issues).unwrap();
            assert!(!encoded.contains(quote));
            assert!(!encoded.contains(path));
        }
    }
    assert_eq!(before, serde_json::to_value(&l.record).unwrap());
    // Adjacent delivered fragments cover a quote without widening the grant.
    read(&mut l, &f, &r, &c, &[request(&f, 40, SIMPLE_SOURCE.len())]);
    let valid = json!({"sections":[{"claims":[{"evidence":[{"path":SIMPLE_PATH,
        "quote":"// greeting formats UTF-8 text such as café."}]}]}]})
    .to_string();
    assert!(
        l.quote_diagnostics(&valid, &f.manifest, &r, &c)
            .unwrap()
            .unwrap()
            .is_empty()
    );
    c.store(true, Ordering::Relaxed);
    assert!(l.quote_diagnostics(&valid, &f.manifest, &r, &c).is_err());
}

#[test]
fn tool_timings_are_monotonic_source_free_and_survive_restart() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    read(&mut l, &f, &r, &c, &[request(&f, 0, 20)]);
    read(&mut l, &f, &r, &c, &[request(&f, 20, 40)]);
    let operation = l.record.binding.operation_id.clone();
    let mut previous = 0;
    for call in &l.record.calls {
        let t = call.timing.as_ref().unwrap();
        assert!(t.started_ms >= previous);
        assert!(t.prepared_ms.unwrap() >= t.started_ms);
        assert!(t.finished_ms.unwrap() >= t.prepared_ms.unwrap());
        previous = t.finished_ms.unwrap();
    }
    assert!(
        !fs::read_to_string(&l.path)
            .unwrap()
            .contains("export function greeting")
    );
    drop(l);
    let saved = inspect(&f.storage, &operation).unwrap();
    assert_eq!(
        saved.context_clock,
        Some(ContextClock::CapturedCodeSessionV1)
    );
    assert_eq!(saved.calls.len(), 2);
    assert_eq!(
        saved
            .calls
            .last()
            .unwrap()
            .timing
            .as_ref()
            .unwrap()
            .finished_ms,
        Some(previous)
    );
}

#[test]
fn rejected_uncertain_and_interrupted_tools_record_their_terminal_boundary() {
    for fault in ["rejected", "uncertain", "interrupted"] {
        let f = Fixture::new();
        let (mut l, r, c) = setup(&f);
        let operation = l.record.binding.operation_id.clone();
        let token = l
            .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
            .unwrap();
        if fault != "interrupted" {
            let mut range = request(&f, 0, 20);
            if fault == "rejected" {
                // Out-of-bounds ranges produce a delivered readError reply.
                // A forbidden path instead fails before transport preparation.
                range.path = "../live.ts".into();
            }
            assert!(
                l.deliver(
                    token,
                    Context::Read {},
                    &[range],
                    &f.manifest,
                    &r,
                    &c,
                    |_| Err("transport failed".into())
                )
                .is_err()
            );
        }
        drop(l);
        let saved = inspect(&f.storage, &operation).unwrap();
        let call = &saved.calls[0];
        let timing = call.timing.as_ref().unwrap();
        assert!(timing.finished_ms.is_some(), "{fault}");
        assert_eq!(timing.prepared_ms.is_some(), fault == "uncertain");
        assert_eq!(
            call.state,
            if fault == "rejected" {
                CallState::Rejected
            } else {
                CallState::Uncertain
            }
        );
    }
}

#[test]
fn legacy_unknown_timings_remain_unknown_and_impossible_new_timelines_are_rejected() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    read(&mut l, &f, &r, &c, &[request(&f, 0, 20)]);
    read(&mut l, &f, &r, &c, &[request(&f, 20, 40)]);
    let mut original = serde_json::to_value(&l.record).unwrap();
    // Fixed synthetic times exercise ordering independently of machine speed.
    original["calls"][0]["timing"] = json!({"startedMs":1,"preparedMs":2,"finishedMs":3});
    original["calls"][1]["timing"] = json!({"startedMs":5,"preparedMs":6,"finishedMs":7});
    for fault in [
        "clock",
        "missing",
        "backwards",
        "unfinished",
        "unprepared",
        "bound",
        "overlap",
    ] {
        let mut bad = original.clone();
        match fault {
            "clock" => bad["contextClock"] = json!("unknownClock"),
            "missing" => {
                bad["calls"][0].as_object_mut().unwrap().remove("timing");
            }
            "backwards" => bad["calls"][0]["timing"]["finishedMs"] = json!(0),
            "unfinished" => bad["calls"][0]["timing"]["finishedMs"] = Value::Null,
            "unprepared" => bad["calls"][0]["timing"]["preparedMs"] = Value::Null,
            "bound" => bad["calls"][1]["timing"]["finishedMs"] = json!(86_400_001u64),
            "overlap" => bad["calls"][1]["timing"]["startedMs"] = json!(2),
            _ => unreachable!(),
        }
        fs::write(&l.path, serde_json::to_vec(&bad).unwrap()).unwrap();
        assert!(
            inspect(&f.storage, &l.record.binding.operation_id).is_err(),
            "{fault}"
        );
    }
    let mut legacy = original;
    legacy.as_object_mut().unwrap().remove("contextClock");
    for call in legacy["calls"].as_array_mut().unwrap() {
        call.as_object_mut().unwrap().remove("timing");
    }
    fs::write(&l.path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert!(saved.context_clock.is_none());
    assert!(saved.calls.iter().all(|call| call.timing.is_none()));
    assert_eq!(serde_json::to_value(saved).unwrap(), legacy);
}
#[test]
fn replies_report_capacity_after_their_own_cost() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let reply = read(&mut l, &f, &r, &c, &[request(&f, 0, 30)]);
    let remaining = &reply["budget"]["remaining"];
    let call = &l.record.calls[0];
    let reserved = reply["budget"]["feedbackBytesReserved"].as_u64().unwrap() as usize;
    assert_eq!(remaining["sourceBytes"], MAX_SOURCE - 30);
    assert_eq!(remaining["files"], MAX_FILES - 1);
    assert_eq!(remaining["calls"], MAX_CALLS - 1);
    assert_eq!(remaining["fragments"], MAX_FRAGMENTS - 1);
    assert!(
        remaining["metadataBytes"].as_u64().unwrap() as usize
            <= MAX_METADATA - call.metadata_bytes - reserved
    );
    assert!(
        remaining["wireBytes"].as_u64().unwrap() as usize <= MAX_WIRE - call.wire_bytes - reserved
    );
    assert_eq!(reply["budget"]["finishNow"], false);
}

#[test]
fn limit_feedback_is_charged_and_never_admits_more_code() {
    for reason in [
        LimitReason::SourceBytes,
        LimitReason::MetadataBytes,
        LimitReason::WireBytes,
    ] {
        let f = Fixture::new();
        let (mut l, r, c) = setup(&f);
        read(&mut l, &f, &r, &c, &[request(&f, 0, 30)]);
        // Boundary injection tests the native counter policy, not live recall.
        let reserved = (MAX_CALLS - 2) * MAX_LIMIT_NOTICE;
        match reason {
            LimitReason::SourceBytes => l.record.calls[0].source_bytes = MAX_SOURCE,
            LimitReason::MetadataBytes => {
                l.record.calls[0].metadata_bytes = MAX_METADATA - reserved
            }
            LimitReason::WireBytes => l.record.calls[0].wire_bytes = MAX_WIRE - reserved,
            _ => unreachable!(),
        }
        for _ in 0..3 {
            let reply = read(&mut l, &f, &r, &c, &[request(&f, 30, 50)]);
            assert_eq!(reply["budgetStop"], serde_json::to_value(reason).unwrap());
            assert_eq!(reply["budget"]["finishNow"], true);
            assert!(reply["context"].is_null());
            assert!(reply["added"].as_array().unwrap().is_empty());
            assert_eq!(l.record.evidence.len(), 1);
            let call = l.record.calls.last().unwrap();
            assert_eq!(call.state, CallState::Delivered);
            assert_eq!(call.source_bytes, 0);
            assert!(call.metadata_bytes > 0 && call.wire_bytes <= MAX_LIMIT_NOTICE);
        }
        assert!(l.budget_stopped());
        // These deliberately injected counters do not represent real delivery.
        // The persisted-record inspector must reject that inconsistency.
        assert!(inspect(&f.storage, &l.record.binding.operation_id).is_err());
    }
}

#[test]
fn real_source_exhaustion_retains_a_valid_receipt_and_prior_evidence() {
    let mut f = Fixture::new();
    f.recapture_with_source(SIMPLE_PATH, &"x".repeat(280_000));
    let (mut l, r, c) = setup(&f);
    for i in 0..8 {
        read(
            &mut l,
            &f,
            &r,
            &c,
            &[request(&f, i * 30_000, (i + 1) * 30_000)],
        );
    }
    let reply = read(&mut l, &f, &r, &c, &[request(&f, 240_000, 270_000)]);
    assert_eq!(reply["budgetStop"], "sourceBytes");
    assert_eq!(reply["budget"]["remaining"]["sourceBytes"], 10_000);
    assert!(reply["added"].as_array().unwrap().is_empty());
    assert_eq!(l.record.evidence.len(), 8);
    assert_eq!(
        l.record.calls.iter().map(|c| c.source_bytes).sum::<usize>(),
        240_000
    );
    l.complete(&f.manifest, &r, &c).unwrap();
    assert_eq!(
        inspect(&f.storage, &l.record.binding.operation_id)
            .unwrap()
            .status,
        Status::Complete
    );
}

#[test]
fn budget_notice_does_not_bypass_scope_stop_or_uncertain_delivery() {
    for fault in ["path", "cancel", "runtime", "transport"] {
        let f = Fixture::new();
        let (mut l, mut r, c) = setup(&f);
        for _ in 0..MAX_CALLS - 1 {
            read(&mut l, &f, &r, &c, &[request(&f, 0, 20)]);
        }
        let token = l
            .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
            .unwrap();
        let mut range = request(&f, 20, 30);
        match fault {
            "path" => range.path = "../private".into(),
            "cancel" => c.store(true, Ordering::Release),
            "runtime" => r.account_revision += 1,
            _ => (),
        }
        let mut sent = false;
        let result = l.deliver(
            token,
            Context::Read {},
            &[range],
            &f.manifest,
            &r,
            &c,
            |bytes| {
                assert_eq!(fault, "transport");
                let response: Value = serde_json::from_slice(bytes).unwrap();
                assert_eq!(response["budgetStop"], "calls");
                assert!(response["added"].as_array().unwrap().is_empty());
                sent = true;
                Err("uncertain write".into())
            },
        );
        assert!(result.is_err());
        assert_eq!(sent, fault == "transport");
        assert_eq!(l.record.evidence.len(), 1);
        if fault == "transport" {
            assert_eq!(l.record.status, Status::Interrupted);
            assert_eq!(l.record.calls.last().unwrap().state, CallState::Uncertain);
            assert!(
                l.begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
                    .is_err()
            );
            inspect(&f.storage, &l.record.binding.operation_id).unwrap();
        }
    }
}

#[test]
fn saved_budget_counters_must_match_possible_delivery_and_old_records_remain_readable() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    read(&mut l, &f, &r, &c, &[request(&f, 0, 30)]);
    let mut legacy = serde_json::to_value(&l.record).unwrap();
    legacy["calls"][0]
        .as_object_mut()
        .unwrap()
        .remove("budgetStop");
    fs::write(&l.path, serde_json::to_vec(&legacy).unwrap()).unwrap();
    inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    for field in ["sourceBytes", "metadataBytes"] {
        let mut forged = legacy.clone();
        forged["calls"][0][field] = json!(MAX_SOURCE);
        fs::write(&l.path, serde_json::to_vec(&forged).unwrap()).unwrap();
        assert!(inspect(&f.storage, &l.record.binding.operation_id).is_err());
    }
}

#[test]
fn final_call_keeps_prior_evidence_and_reserves_feedback() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    for i in 0..MAX_CALLS {
        let reply = read(&mut l, &f, &r, &c, &[request(&f, 0, 30)]);
        if i == MAX_CALLS - 1 {
            assert_eq!(reply["budgetStop"], "calls");
            assert_eq!(reply["budget"]["remaining"]["calls"], 0);
            assert!(reply["context"].is_null());
        }
    }
    assert_eq!(l.record.evidence.len(), 1);
    assert_eq!(
        l.record.calls.iter().map(|c| c.source_bytes).sum::<usize>(),
        30
    );
    assert!(
        l.begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
            .is_err()
    );
    l.complete(&f.manifest, &r, &c).unwrap();
    assert_eq!(
        inspect(&f.storage, &l.record.binding.operation_id)
            .unwrap()
            .status,
        Status::Complete
    );
}

#[test]
fn real_file_and_fragment_limits_return_no_new_evidence() {
    let mut f = Fixture::new();
    for i in 0..=MAX_FILES {
        f.recapture_with_source(
            &format!("src/part{i}.ts"),
            &format!("export const part{i} = 'captured source';\n"),
        );
    }
    let (mut l, r, c) = setup(&f);
    for i in 0..=MAX_FILES {
        let path = format!("src/part{i}.ts");
        let file = f
            .manifest
            .files
            .iter()
            .find(|file| file.path == path)
            .unwrap();
        let reply = read(
            &mut l,
            &f,
            &r,
            &c,
            &[RangeRequest {
                path,
                source_sha256: file.sha256.clone(),
                start_byte: 0,
                end_byte: 20,
            }],
        );
        if i == MAX_FILES {
            assert_eq!(reply["budgetStop"], "files");
        }
    }
    assert_eq!(l.record.evidence.len(), MAX_FILES);

    let mut f = Fixture::new();
    f.recapture_with_source(SIMPLE_PATH, &"x".repeat(1024));
    let (mut l, r, c) = setup(&f);
    for batch in 0..32 {
        let ranges = (0..8)
            .map(|i| {
                let start = (batch * 8 + i) * 2 + 1;
                request(&f, start, start + 1)
            })
            .collect::<Vec<_>>();
        read(&mut l, &f, &r, &c, &ranges);
    }
    assert_eq!(l.record.evidence.len(), 256);
    let reply = read(&mut l, &f, &r, &c, &[request(&f, 0, 513)]);
    assert_eq!(reply["budgetStop"], "fragments");
    assert!(reply["added"].as_array().unwrap().is_empty());
    assert_eq!(l.record.evidence.len(), 256);
}

#[test]
fn escaped_source_cannot_exceed_one_reply() {
    let mut f = Fixture::new();
    f.recapture_with_source(SIMPLE_PATH, &"\"\\\n".repeat(40_000));
    let (mut l, r, c) = setup(&f);
    let ranges = (0..4)
        .map(|i| request(&f, i * 30_000, (i + 1) * 30_000))
        .collect::<Vec<_>>();
    let reply = read(&mut l, &f, &r, &c, &ranges);
    assert_eq!(reply["budgetStop"], "resultBytes");
    assert!(l.record.evidence.is_empty());
    assert_eq!(l.record.calls[0].source_bytes, 0);
    assert!(l.record.calls[0].wire_bytes <= MAX_LIMIT_NOTICE);
}

#[test]
fn overlapping_reads_send_only_new_bytes_and_store_no_source() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let first = read(&mut l, &f, &r, &c, &[request(&f, 0, 40)]);
    let second = read(
        &mut l,
        &f,
        &r,
        &c,
        &[request(&f, 20, 80), request(&f, 70, 100)],
    );
    assert_eq!(first["added"][0]["contents"], &SIMPLE_SOURCE[..40]);
    assert_eq!(second["added"][0]["contents"], &SIMPLE_SOURCE[40..80]);
    assert_eq!(second["added"][1]["contents"], &SIMPLE_SOURCE[80..100]);
    assert_eq!(
        l.record.calls.iter().map(|c| c.source_bytes).sum::<usize>(),
        100
    );
    let again = read(&mut l, &f, &r, &c, &[request(&f, 0, 100)]);
    assert!(again["added"].as_array().unwrap().is_empty());
    assert_eq!(again["alreadyAvailable"].as_array().unwrap().len(), 3);
    let saved = fs::read(&l.path).unwrap();
    assert!(!String::from_utf8_lossy(&saved).contains("export function greeting"));
    assert_eq!(
        inspect(&f.storage, &l.record.binding.operation_id)
            .unwrap()
            .evidence
            .len(),
        3
    );
    l.complete(&f.manifest, &r, &c).unwrap();
    assert!(
        l.begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
            .is_err()
    );
}
#[test]
fn invalid_reads_return_accounted_metadata_and_never_substitute_source() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let mut wrong_hash = request(&f, 0, 20);
    wrong_hash.source_sha256 = "f".repeat(64);
    let utf8 = SIMPLE_SOURCE.find('é').unwrap() + 1;
    for req in [
        wrong_hash,
        request(&f, utf8, utf8 + 5),
        request(&f, 0, usize::MAX),
        request(&f, 0, SIMPLE_SOURCE.len() + 1),
        request(&f, 20, 10),
    ] {
        let response = read(&mut l, &f, &r, &c, &[request(&f, 0, 20), req]);
        assert!(response["added"].as_array().unwrap().is_empty());
        assert!(response["alreadyAvailable"].as_array().unwrap().is_empty());
        assert_eq!(response["readError"]["code"], "invalidRead");
        assert_eq!(response["readError"]["sizeBytes"], SIMPLE_SOURCE.len());
    }
    assert!(l.record.evidence.is_empty());
    assert!(
        l.record
            .calls
            .iter()
            .all(|c| c.state == CallState::Delivered
                && c.source_bytes == 0
                && c.metadata_bytes > 0
                && c.wire_bytes == c.metadata_bytes)
    );
    // Correction is an agent decision, not an automatic inference retry.
    let corrected = read(&mut l, &f, &r, &c, &[request(&f, 0, 20)]);
    assert_eq!(corrected["added"][0]["contents"], &SIMPLE_SOURCE[..20]);
}

#[test]
fn foreign_paths_and_wrong_tools_are_terminal_without_source_delivery() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let mut wrong_path = request(&f, 0, 20);
    wrong_path.path = "../live.ts".into();
    let call = l
        .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
        .unwrap();
    assert!(
        l.deliver(
            call,
            Context::Read {},
            &[wrong_path],
            &f.manifest,
            &r,
            &c,
            |_| panic!("foreign source escaped")
        )
        .is_err()
    );
    assert!(l.record.evidence.is_empty());
    assert_eq!(l.record.calls[0].state, CallState::Rejected);
    let call = l
        .begin_call(Tool::Map, &"c".repeat(64), &f.manifest, &r, &c)
        .unwrap();
    assert!(
        l.deliver(
            call,
            Context::Read {},
            &[request(&f, 0, 20)],
            &f.manifest,
            &r,
            &c,
            |_| panic!()
        )
        .is_err()
    );
    assert!(
        l.begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
            .is_err()
    );
}
#[test]
fn revocation_runtime_snapshot_deadline_and_cancellation_stop_delivery() {
    let f = Fixture::new();
    for mode in 0..5 {
        let (mut l, mut r, c) = setup(&f);
        let call = l
            .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
            .unwrap();
        let mut manifest = f.manifest.clone();
        match mode {
            0 => c.store(true, Ordering::Release),
            1 => r.account_revision += 1,
            2 => manifest.summary.snapshot_id.push_str("-changed"),
            3 => l.deadline = Instant::now(),
            _ => {
                fs::remove_file(
                    f.storage
                        .join("workspace-permissions")
                        .join(format!("{}.json", l.record.binding.grant_id)),
                )
                .unwrap();
            }
        }
        assert!(
            l.deliver(
                call,
                Context::Read {},
                &[request(&f, 0, 20)],
                &manifest,
                &r,
                &c,
                |_| panic!("scope escaped")
            )
            .is_err()
        );
    }
}
#[test]
fn uncertain_reply_is_terminal_and_no_existing_operation_can_restart() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let call = l
        .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
        .unwrap();
    assert!(
        l.deliver(
            call,
            Context::Read {},
            &[request(&f, 0, 20)],
            &f.manifest,
            &r,
            &c,
            |_| Err("raw transport should not appear".into())
        )
        .unwrap_err()
        .contains("uncertain")
    );
    assert_eq!(l.record.status, Status::Interrupted);
    assert_eq!(l.record.calls[0].state, CallState::Uncertain);
    assert!(
        l.begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
            .is_err()
    );
    assert!(
        Ledger::new(
            &f.storage,
            &l.record.binding.operation_id,
            &f.manifest,
            &l.record.binding.grant_id,
            &r,
            "gpt-6-luna",
            "max"
        )
        .is_err()
    );
}
#[test]
fn bounds_account_for_encoded_source_and_hostile_saved_counters() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    read(&mut l, &f, &r, &c, &[request(&f, 0, 30)]);
    let mut record = l.record.clone();
    record.calls[0].wire_bytes = usize::MAX;
    record.calls.push(record.calls[0].clone());
    fs::write(&l.path, serde_json::to_vec(&record).unwrap()).unwrap();
    assert!(inspect(&f.storage, &l.record.binding.operation_id).is_err());
    l.save(l.record.clone()).unwrap();
    l.record.calls[0].metadata_bytes = MAX_METADATA;
    let call = l
        .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &r, &c)
        .unwrap();
    assert!(
        l.deliver(
            call,
            Context::Read {},
            &[request(&f, 30, 50)],
            &f.manifest,
            &r,
            &c,
            |_| panic!()
        )
        .is_err()
    );
    assert_eq!(l.record.evidence.len(), 1);
}
#[test]
fn only_admitted_exact_quotes_can_back_a_validated_answer() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let proposal = json!({"title":"How the greeting is formatted","sections":[{"title":"What does greeting return?","prose":"The greeting function formats a name for the caller.","spokenText":"The greeting formats a name for the caller.","claims":[{"category":"codeBehavior","aspect":"responsibility","prose":"The greeting function accepts a string name and returns a string.","inferenceBasis":"","identifiers":["greeting"],"evidence":[{"path":SIMPLE_PATH,"quote":"export function greeting(name: string): string {"}],"relationshipIds":[]}],"visual":{"kind":"none","title":"","direction":"auto","nodes":[],"edges":[]}}]});
    let revision = crate::chapter_generation::canonicalize_workspace_overview(
        &f.manifest.summary.project_id,
        &f.manifest,
        Some(&f.index),
        &f.sources(&[SIMPLE_PATH]),
        &Default::default(),
        &proposal.to_string(),
    )
    .unwrap();
    assert!(l.verify_revision(&revision, &f.manifest, &r, &c).is_err());
    let a = &revision.sections[0].claims[0].anchors[0];
    read(
        &mut l,
        &f,
        &r,
        &c,
        &[request(
            &f,
            a.start_byte as usize,
            a.end_byte_inclusive as usize + 1,
        )],
    );
    l.verify_revision(&revision, &f.manifest, &r, &c).unwrap();
    let exact = request(&f, a.start_byte as usize, a.end_byte_inclusive as usize + 1);
    for _ in 1..MAX_CALLS {
        read(&mut l, &f, &r, &c, std::slice::from_ref(&exact));
    }
    assert!(l.budget_stopped());
    l.verify_revision(&revision, &f.manifest, &r, &c).unwrap();
    let mut changed = revision.clone();
    changed.sections[0].claims[0].anchors[0].excerpt.push('!');
    assert!(l.verify_revision(&changed, &f.manifest, &r, &c).is_err());
    changed = revision.clone();
    changed.sections[0].claims[0].anchors[0].start_column += 1;
    assert!(l.verify_revision(&changed, &f.manifest, &r, &c).is_err());
    c.store(true, Ordering::Release);
    assert!(l.verify_revision(&revision, &f.manifest, &r, &c).is_err());
}
#[test]
fn interval_subtraction_handles_order_overlap_adjacent_and_nested_ranges() {
    assert_eq!(
        gaps(
            0,
            100,
            vec![(70, 90), (10, 20), (15, 30), (30, 40), (12, 15)]
        ),
        vec![(0, 10), (40, 70), (90, 100)]
    );
    assert!(gaps(10, 20, vec![(0, 100)]).is_empty());
}
