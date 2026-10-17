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
