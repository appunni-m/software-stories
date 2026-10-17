use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};

fn setup(f: &Fixture) -> (Ledger, CodexReviewRuntime, AtomicBool) {
    let runtime = CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 1,
    };
    let grant = workspace::save_retrieval_grant(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
        &runtime,
    )
    .unwrap();
    let ledger = Ledger::new(
        &f.storage,
        &Uuid::new_v4().to_string(),
        &f.manifest,
        &grant,
        &runtime,
        "gpt-6-luna",
        "max",
    )
    .unwrap();
    (ledger, runtime, AtomicBool::new(false))
}

fn read(
    ledger: &mut Ledger,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) -> String {
    let call = ledger
        .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, runtime, cancel)
        .unwrap();
    let source = f
        .manifest
        .files
        .iter()
        .find(|x| x.path == SIMPLE_PATH)
        .unwrap();
    let mut result = Value::Null;
    ledger
        .deliver(
            call,
            Context::Read {},
            &[RangeRequest {
                path: SIMPLE_PATH.into(),
                source_sha256: source.sha256.clone(),
                start_byte: 0,
                end_byte: SIMPLE_SOURCE.len(),
            }],
            &f.manifest,
            runtime,
            cancel,
            |bytes| {
                result = serde_json::from_slice(bytes).unwrap();
                Ok(())
            },
        )
        .unwrap();
    result["added"]
        .as_array()
        .and_then(|a| a.first())
        .or_else(|| {
            result["alreadyAvailable"]
                .as_array()
                .and_then(|a| a.first())
        })
        .map(|v| v.get("id").unwrap_or(v).as_str().unwrap().to_owned())
        .unwrap()
}

fn proposal(id: &str, key: &str, order: usize) -> String {
    json!({"proposalVersion":PROPOSAL_VERSION,"sectionKey":key,"order":order,"previousGenerationId":null,
        "section":{"title":"Formatting a greeting","claims":[{
            "category":"codeBehavior","aspect":"inputOutput",
            "prose":"The greeting function formats the supplied name into a message. The returned greeting includes that name.",
            "inferenceBasis":"","identifiers":["greeting"],"evidence":[{"referenceId":id}],"relationshipIds":[]
        }],"visual":{"kind":"dataFlow","title":"Format the greeting","direction":"leftToRight","nodes":[
            {"label":"Name","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}]},
            {"label":"Message","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}]}
        ],"edges":[{"fromNodeIndex":0,"toNodeIndex":1,"label":"format","evidence":[{"claimIndex":0,"evidenceIndex":0}]}]}}}).to_string()
}

fn start(
    ledger: &Ledger,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) -> ActiveAnswer {
    ledger
        .start_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            runtime,
            cancel,
        )
        .unwrap()
}

fn publish(
    answer: &mut ActiveAnswer,
    ledger: &Ledger,
    raw: &str,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) -> String {
    let prepared = answer
        .prepare(ledger, raw, &f.manifest, runtime, cancel)
        .unwrap();
    assert!(prepared.findings.is_empty(), "{:?}", prepared.findings);
    let generation = prepared.generation_id.clone().unwrap();
    answer
        .commit(ledger, prepared, &f.manifest, runtime, cancel)
        .unwrap();
    generation
}

#[test]
fn a_published_section_replays_exact_source_visual_and_proposal_after_ledger_appends() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let id = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start(&ledger, &f, &runtime, &cancel);
    let generation = publish(
        &mut answer,
        &ledger,
        &proposal(&id, "entry", 0),
        &f,
        &runtime,
        &cancel,
    );
    let original = encode_answer(&answer.value).unwrap();
    read(&mut ledger, &f, &runtime, &cancel);
    assert_eq!(original, fs::read(&answer.path).unwrap());
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.sections.len(), 1);
    assert_eq!(saved.sections[0].preview.generation_id, generation);
    let section = &saved.sections[0].preview.revision.sections[0];
    assert_eq!(section.claims[0].anchors[0].excerpt, SIMPLE_SOURCE);
    assert!(!section.prose.contains("Presentation is derived"));
    assert!(!section.spoken_text.contains("Presentation is derived"));
    assert!(
        section
            .visual
            .as_ref()
            .unwrap()
            .typed_visual()
            .unwrap()
            .nodes
            .iter()
            .all(|n| n.evidence_anchor_ids == vec![section.claims[0].anchors[0].anchor_id.clone()])
    );
    // The checkpoint remains independently inspectable without the mutable
    // operation record. No grant or runtime connection is acquired by replay.
    fs::remove_file(&ledger.path).unwrap();
    fs::remove_file(
        f.storage
            .join("workspace-permissions")
            .join(format!("{}.json", ledger.record.binding.grant_id)),
    )
    .unwrap();
    inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
}

#[test]
fn a_later_rejected_claim_preserves_the_first_section_and_allows_a_partial_terminal_outcome() {
    let f = Fixture::new();
    let (mut ledger, r, c) = setup(&f);
    let id = read(&mut ledger, &f, &r, &c);
    let mut a = start(&ledger, &f, &r, &c);
    publish(&mut a, &ledger, &proposal(&id, "first", 0), &f, &r, &c);
    let original = serde_json::to_value(&a.value.sections[0]).unwrap();
    let bad = proposal(&id, "second", 1).replace(
        "\"identifiers\":[\"greeting\"]",
        "\"identifiers\":[\"not_read\"]",
    );
    let p = a.prepare(&ledger, &bad, &f.manifest, &r, &c).unwrap();
    assert_eq!(p.findings, finding("identifierAlignment"));
    assert!(p.generation_id.is_none());
    a.commit(&ledger, p, &f.manifest, &r, &c).unwrap();
    a.finish(
        Outcome::Partial,
        vec!["The deeper implementation could not be completed.".into()],
    )
    .unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.outcome, Outcome::Partial);
    assert_eq!(saved.sections.len(), 1);
    assert_eq!(serde_json::to_value(&saved.sections[0]).unwrap(), original);
    assert_eq!(saved.attempts[1].arguments, bad);
}

#[test]
fn exact_repeats_reuse_identity_and_changed_content_requires_the_previous_generation() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let raw = proposal(&id, "entry", 0);
    let first = publish(&mut a, &l, &raw, &f, &r, &c);
    assert_eq!(publish(&mut a, &l, &raw, &f, &r, &c), first);
    assert_eq!(a.value.sections.len(), 1);
    let changed = raw.replace("Formatting a greeting", "Reading the greeting");
    let p = a.prepare(&l, &changed, &f.manifest, &r, &c).unwrap();
    assert_eq!(p.findings, finding("sectionConflict"));
    a.commit(&l, p, &f.manifest, &r, &c).unwrap();
    let mut next: Value = serde_json::from_str(&changed).unwrap();
    next["previousGenerationId"] = json!(first);
    let second = publish(&mut a, &l, &next.to_string(), &f, &r, &c);
    assert_ne!(first, second);
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert_eq!(saved.sections.len(), 2);
    assert_eq!(
        saved.sections[1].supersedes.as_deref(),
        Some(first.as_str())
    );
}

#[test]
fn stop_revocation_runtime_and_snapshot_changes_refuse_prepared_publication() {
    for change in ["stop", "grant", "runtime", "snapshot", "deadline"] {
        let f = Fixture::new();
        let (mut l, r, c) = setup(&f);
        let id = read(&mut l, &f, &r, &c);
        let mut a = start(&l, &f, &r, &c);
        let p = a
            .prepare(&l, &proposal(&id, "entry", 0), &f.manifest, &r, &c)
            .unwrap();
        let before = fs::read(&a.path).unwrap();
        let mut runtime = r.clone();
        let mut manifest = f.manifest.clone();
        match change {
            "stop" => c.store(true, Ordering::Release),
            "grant" => fs::remove_file(
                f.storage
                    .join("workspace-permissions")
                    .join(format!("{}.json", l.record.binding.grant_id)),
            )
            .unwrap(),
            "runtime" => runtime.account_revision += 1,
            "snapshot" => manifest.summary.snapshot_id.push_str("-changed"),
            "deadline" => l.deadline = Instant::now(),
            _ => unreachable!(),
        }
        assert!(
            a.commit(&l, p, &manifest, &runtime, &c).is_err(),
            "{change}"
        );
        assert_eq!(fs::read(&a.path).unwrap(), before, "{change}");
    }
}

#[test]
fn invalid_proposals_are_bounded_and_never_replace_an_existing_section() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let valid = serde_json::to_value(&a.value.sections[0]).unwrap();
    let unknown = proposal(&format!("evidence:{}", "f".repeat(64)), "second", 1);
    let malformed = "{\"order\":0,\"order\":1}".to_owned();
    let bad_visual = proposal(&id, "second", 1).replace("\"toNodeIndex\":1", "\"toNodeIndex\":7");
    let wrong_order = proposal(&id, "second", 5);
    for raw in [
        unknown,
        malformed,
        bad_visual,
        wrong_order,
        "{}".into(),
        "[]".into(),
        "null".into(),
    ] {
        let p = a.prepare(&l, &raw, &f.manifest, &r, &c).unwrap();
        assert!(!p.findings.is_empty());
        a.commit(&l, p, &f.manifest, &r, &c).unwrap();
    }
    assert!(
        a.prepare(&l, &proposal(&id, "second", 1), &f.manifest, &r, &c)
            .is_err()
    );
    assert_eq!(serde_json::to_value(&a.value.sections[0]).unwrap(), valid);
    inspect(&f.storage, &l.record.binding.operation_id).unwrap();
}

#[test]
fn checkpoint_proposal_content_and_visual_tampering_fail_independent_inspection() {
    for change in [
        "source-range",
        "delivery",
        "checkpoint-hash",
        "proposal",
        "prose",
        "edge",
        "origin",
        "order",
        "identity",
    ] {
        let f = Fixture::new();
        let (mut l, r, c) = setup(&f);
        let id = read(&mut l, &f, &r, &c);
        let mut a = start(&l, &f, &r, &c);
        publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
        let mut saved: Value = serde_json::from_slice(&fs::read(&a.path).unwrap()).unwrap();
        match change {
            "source-range" => {
                saved["sections"][0]["checkpoint"]["evidence"][0]["range"]["startByte"] = json!(1)
            }
            "delivery" => {
                saved["sections"][0]["checkpoint"]["calls"][0]["state"] = json!("uncertain")
            }
            "checkpoint-hash" => saved["sections"][0]["checkpointSha256"] = json!("f".repeat(64)),
            "proposal" => saved["attempts"][0]["arguments"] = json!("{}"),
            "prose" => {
                saved["sections"][0]["preview"]["revision"]["sections"][0]["prose"] =
                    json!("This unrelated sentence was never generated by the owned proposal.")
            }
            "edge" => {
                saved["sections"][0]["preview"]["revision"]["sections"][0]["visual"]["edges"][0]["label"] =
                    json!("unproven relationship")
            }
            "origin" => {
                saved["sections"][0]["preview"]["revision"]["generationOrigin"]["retrieval"]["ledgerSha256"] =
                    json!("f".repeat(64))
            }
            "order" => saved["sections"][0]["order"] = json!(2),
            "identity" => saved["binding"]["runtime"]["accountRevision"] = json!(99),
            _ => unreachable!(),
        }
        codex_agent::atomic_private_write(&a.path, &serde_json::to_vec(&saved).unwrap()).unwrap();
        assert!(
            inspect(&f.storage, &l.record.binding.operation_id).is_err(),
            "{change}"
        );
    }
}

#[test]
fn saved_file_symlinks_capacity_and_stale_preparation_cannot_destroy_prior_work() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let p = a
        .prepare(&l, &proposal(&id, "entry", 0), &f.manifest, &r, &c)
        .unwrap();
    let stale = a
        .prepare(&l, &proposal(&id, "other", 0), &f.manifest, &r, &c)
        .unwrap();
    a.commit(&l, p, &f.manifest, &r, &c).unwrap();
    let before = fs::read(&a.path).unwrap();
    assert!(a.commit(&l, stale, &f.manifest, &r, &c).is_err());
    assert_eq!(fs::read(&a.path).unwrap(), before);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{PermissionsExt, symlink};
        assert_eq!(
            a.path.metadata().unwrap().permissions().mode() & 0o777,
            0o600
        );
        let moved = a.path.with_extension("preserved");
        fs::rename(&a.path, &moved).unwrap();
        symlink(&moved, &a.path).unwrap();
        assert!(inspect(&f.storage, &l.record.binding.operation_id).is_err());
        fs::remove_file(&a.path).unwrap();
        fs::rename(moved, &a.path).unwrap();
    }
    for _ in 1..MAX_ANSWERS {
        let (ledger, r, c) = setup(&f);
        start(&ledger, &f, &r, &c);
    }
    let (ledger, r, c) = setup(&f);
    assert!(
        ledger
            .start_answer_sections("Another question", &"e".repeat(64), &f.manifest, &r, &c)
            .is_err()
    );
    assert_eq!(fs::read(&a.path).unwrap(), before);
}

#[test]
fn six_unique_sections_are_allowed_and_an_existing_key_needs_an_explicit_revision() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    for order in 0..MAX_SECTIONS {
        publish(
            &mut a,
            &l,
            &proposal(&id, &format!("part-{order}"), order),
            &f,
            &r,
            &c,
        );
    }
    let raw = proposal(&id, "seventh", 0);
    let rejected = a.prepare(&l, &raw, &f.manifest, &r, &c).unwrap();
    assert_eq!(rejected.findings, finding("sectionLimit"));
    a.commit(&l, rejected, &f.manifest, &r, &c).unwrap();
    a.finish(
        Outcome::Partial,
        vec!["The remaining topic needs further exploration.".into()],
    )
    .unwrap();
    assert_eq!(
        inspect(&f.storage, &l.record.binding.operation_id)
            .unwrap()
            .sections
            .len(),
        6
    );
}

#[test]
fn oversized_proposals_and_corrupt_storage_preserve_the_previous_bytes() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let before = fs::read(&a.path).unwrap();
    assert!(
        a.prepare(&l, &"x".repeat(MAX_PROPOSAL_BYTES + 1), &f.manifest, &r, &c)
            .is_err()
    );
    assert_eq!(fs::read(&a.path).unwrap(), before);
    let prepared = a
        .prepare(&l, &proposal(&id, "later", 1), &f.manifest, &r, &c)
        .unwrap();
    fs::write(&a.path, b"damaged local answer").unwrap();
    assert!(a.commit(&l, prepared, &f.manifest, &r, &c).is_err());
    assert!(a.finish(Outcome::Partial, vec![]).is_err());
    assert_eq!(fs::read(&a.path).unwrap(), b"damaged local answer");
    let (other, runtime, cancel) = setup(&f);
    assert!(
        other
            .start_answer_sections(
                "Another question",
                &"e".repeat(64),
                &f.manifest,
                &runtime,
                &cancel
            )
            .is_err()
    );
    assert_eq!(fs::read(&a.path).unwrap(), b"damaged local answer");
}

#[test]
fn incomplete_or_uncertain_source_delivery_cannot_publish_a_section() {
    for state in [CallState::Prepared, CallState::Uncertain] {
        let f = Fixture::new();
        let (mut l, r, c) = setup(&f);
        let id = read(&mut l, &f, &r, &c);
        let a = start(&l, &f, &r, &c);
        l.record.calls[0].state = state;
        let before = fs::read(&a.path).unwrap();
        let rejected = a
            .prepare(&l, &proposal(&id, "entry", 0), &f.manifest, &r, &c)
            .unwrap();
        assert_eq!(rejected.findings, finding("citation"));
        assert!(rejected.generation_id.is_none());
        assert_eq!(fs::read(&a.path).unwrap(), before);
    }
}

#[test]
fn a_proposal_in_progress_is_excluded_from_its_immutable_delivery_checkpoint() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let _pending = l
        .begin_call(Tool::Map, &"f".repeat(64), &f.manifest, &r, &c)
        .unwrap();
    publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert_eq!(saved.sections[0].checkpoint.calls.len(), 1);
    assert_eq!(l.record.calls.len(), 2);
    assert_eq!(
        saved.sections[0].checkpoint.calls[0].state,
        CallState::Delivered
    );
}

#[test]
fn a_failed_final_manifest_cannot_clear_sections_and_terminal_records_cannot_publish_again() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let first = publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let before = fs::read(&a.path).unwrap();
    assert!(a.finish(Outcome::Failed, vec![]).is_err());
    assert!(
        a.finish(Outcome::Complete, vec!["Unfinished".into()])
            .is_err()
    );
    assert_eq!(fs::read(&a.path).unwrap(), before);
    a.finish(Outcome::Stopped, vec![]).unwrap();
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert_eq!(saved.outcome, Outcome::Stopped);
    assert_eq!(saved.sections[0].preview.generation_id, first);
    assert!(
        a.prepare(&l, &proposal(&id, "later", 1), &f.manifest, &r, &c)
            .is_err()
    );
    assert!(a.finish(Outcome::Complete, vec![]).is_err());
}

#[test]
fn even_empty_saved_answers_require_the_original_native_binding() {
    for change in ["runtime", "model", "grant", "manifest"] {
        let f = Fixture::new();
        let (l, r, c) = setup(&f);
        let a = start(&l, &f, &r, &c);
        let mut raw: Value = serde_json::from_slice(&fs::read(&a.path).unwrap()).unwrap();
        match change {
            "runtime" => raw["binding"]["runtime"]["accountIdentitySha256"] = json!("invalid"),
            "model" => raw["binding"]["model"] = json!("different-model"),
            "grant" => raw["binding"]["grantId"] = json!("invalid"),
            "manifest" => raw["binding"]["manifestSha256"] = json!("f".repeat(64)),
            _ => unreachable!(),
        }
        fs::write(&a.path, serde_json::to_vec(&raw).unwrap()).unwrap();
        assert!(
            inspect(&f.storage, &l.record.binding.operation_id).is_err(),
            "{change}"
        );
    }
}

#[test]
fn saved_answer_bounds_and_proposal_versions_fail_without_inference_or_replacement() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let mut raw: Value = serde_json::from_str(&proposal(&id, "entry", 0)).unwrap();
    raw["proposalVersion"] = json!("future-version");
    let p = a
        .prepare(&l, &raw.to_string(), &f.manifest, &r, &c)
        .unwrap();
    assert_eq!(p.findings, finding("proposalShape"));
    a.commit(&l, p, &f.manifest, &r, &c).unwrap();
    publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let original = fs::read(&a.path).unwrap();
    let mut too_large = a.value.clone();
    too_large.question = "x".repeat(MAX_RECORD_BYTES);
    assert!(encode_answer(&too_large).is_err());
    for _ in 0..MAX_DIRECTORY_ENTRIES {
        fs::write(
            a.path
                .parent()
                .unwrap()
                .join(format!("{}.preserved", Uuid::new_v4())),
            b"preserved",
        )
        .unwrap();
    }
    let (other, runtime, cancel) = setup(&f);
    assert!(
        other
            .start_answer_sections(
                "Another question",
                &"e".repeat(64),
                &f.manifest,
                &runtime,
                &cancel
            )
            .is_err()
    );
    assert_eq!(fs::read(&a.path).unwrap(), original);
    inspect(&f.storage, &l.record.binding.operation_id).unwrap();
}
