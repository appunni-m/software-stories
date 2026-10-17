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
    let ledger = Ledger::new_progressive(
        &f.storage,
        &Uuid::new_v4().to_string(),
        &f.manifest,
        &grant,
        &runtime,
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
            result["reusedReferences"]
                .as_array()
                .and_then(|a| a.first())
        })
        .or_else(|| {
            result["alreadyAvailable"]
                .as_array()
                .and_then(|a| a.first())
        })
        .map(|v| {
            v.get("referenceId")
                .or_else(|| v.get("id"))
                .unwrap_or(v)
                .as_str()
                .unwrap()
                .to_owned()
        })
        .unwrap()
}

pub(crate) fn proposal(id: &str, key: &str, order: usize) -> String {
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
fn complete_presentation_keeps_every_condition_and_later_sentence() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let mut raw: Value = serde_json::from_str(&proposal(&id, "entry", 0)).unwrap();
    let complete = "The `greeting` function formats the supplied `name`. It uses the input directly. The greeting includes the supplied name. An empty name still reaches the same formatting operation. The function returns the resulting message.";
    raw["section"]["claims"][0]["prose"] = json!(complete);
    publish(&mut a, &l, &raw.to_string(), &f, &r, &c);
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert_eq!(saved.sections[0].checkpoint_version, 2);
    let section = &saved.sections[0].preview.revision.sections[0];
    assert_eq!(section.prose, complete);
    assert_eq!(section.spoken_text, complete);
}

#[test]
fn oversized_complete_presentation_is_rejected_without_publishing_a_summary() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let mut raw: Value = serde_json::from_str(&proposal(&id, "entry", 0)).unwrap();
    let mut claims = Vec::new();
    for index in 0..4 {
        let mut claim = raw["section"]["claims"][0].clone();
        claim["prose"] = json!(format!("The greeting function {} in case {index}.", "formats the supplied name into a message and ".repeat(30)));
        claims.push(claim);
    }
    raw["section"]["claims"] = json!(claims);
    let prepared = a.prepare(&l, &raw.to_string(), &f.manifest, &r, &c).unwrap();
    assert_eq!(prepared.findings, vec![json!({"location":"/section","code":"presentationLimit"})]);
    assert!(prepared.generation_id.is_none());
    a.commit(&l, prepared, &f.manifest, &r, &c).unwrap();
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert!(saved.sections.is_empty());
    assert_eq!(saved.attempts[0].arguments, raw.to_string());
}

#[test]
fn historical_summary_checkpoints_are_not_reinterpreted_as_complete_scripts() {
    for version in [1, 2] {
        let f = Fixture::new();
        let (mut l, r, c) = setup(&f);
        let id = read(&mut l, &f, &r, &c);
        let mut a = start(&l, &f, &r, &c);
        let mut raw: Value = serde_json::from_str(&proposal(&id, "entry", 0)).unwrap();
        raw["section"]["claims"][0]["prose"] = json!("The `greeting` function formats the supplied `name`. It returns the resulting message.");
        let raw = raw.to_string();
        let prepared = a.prepare_with_checkpoint_version(&l, &raw, &f.manifest, &r, &c, version).unwrap();
        assert!(prepared.findings.is_empty());
        a.commit(&l, prepared, &f.manifest, &r, &c).unwrap();
        let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
        let script = &saved.sections[0].preview.revision.sections[0].spoken_text;
        if version == 1 {
            assert_eq!(script, "It returns the resulting message.");
        } else {
            assert_eq!(script, "The `greeting` function formats the supplied `name`. It returns the resulting message.");
        }
        let before = fs::read(&a.path).unwrap();
        // Exact old proposals remain idempotent under the new write policy.
        publish(&mut a, &l, &raw, &f, &r, &c);
        assert_eq!(a.value.sections.len(), 1);
        assert_eq!(a.value.sections[0].checkpoint_version, version);
        let mut forged: Value = serde_json::from_slice(&before).unwrap();
        forged["sections"][0]["checkpointVersion"] = json!(3 - version);
        fs::write(&a.path, serde_json::to_vec(&forged).unwrap()).unwrap();
        assert!(inspect(&f.storage, &l.record.binding.operation_id).is_err());
    }
}

fn manifest(answer: &ActiveAnswer) -> String {
    json!({"manifestVersion":"answerManifestV1","sections":answer.value.latest_identities(),"incompleteTopics":[]}).to_string()
}

#[test]
fn proposal_feedback_locates_identifier_counts_prose_and_evidence_without_copying_data() {
    let claim = json!({"prose":"The helper returns a greeting string.","identifiers":["greeting","absent"],"evidence":[{"quote":"fn greeting() { private_source_marker(); }"},{"quote":"unrelated_source_marker"}]});
    let section = json!({"claims":[claim]});
    let text = json!({"sections":[section]}).to_string();
    let findings = alignment_findings(&section, &text);
    assert_eq!(
        findings,
        vec![
            json!({"location":"/section/claims/0/identifiers/1","code":"identifierProse"}),
            json!({"location":"/section/claims/0/identifiers/1","code":"identifierEvidence"}),
            json!({"location":"/section/claims/0/evidence/1","code":"evidenceIdentifier"}),
        ]
    );
    assert!(valid_findings(&findings));
    let encoded = serde_json::to_string(&findings).unwrap();
    for forbidden in [
        "greeting",
        "absent",
        "private_source_marker",
        "unrelated_source_marker",
    ] {
        assert!(!encoded.contains(forbidden));
    }
    let mut too_many = section.clone();
    too_many["claims"][0]["identifiers"] = json!(vec!["greeting"; 9]);
    assert_eq!(
        alignment_findings(&too_many, &json!({"sections":[too_many]}).to_string()),
        vec![json!({"location":"/section/claims/0/identifiers","code":"identifierCount"})]
    );
    assert!(!alignment_findings(&section, "{}").is_empty());
    assert_eq!(
        section_location(&json!({"location":"/sections/0/claims/2/evidence/3"})),
        "/section/claims/2/evidence/3"
    );
    assert_eq!(
        section_location(&json!({"location":"private/source.rs"})),
        "/section"
    );
}

#[test]
fn invalid_section_key_reports_its_exact_field_and_preserves_the_original_proposal() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let evidence = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start(&ledger, &f, &runtime, &cancel);
    let original = proposal(&evidence, "entry_flow", 0);
    let prepared = answer.prepare(&ledger, &original, &f.manifest, &runtime, &cancel).unwrap();
    assert_eq!(prepared.findings, vec![json!({"location":"/sectionKey","code":"sectionKeyFormat"})]);
    assert!(prepared.generation_id.is_none());
    answer.commit(&ledger, prepared, &f.manifest, &runtime, &cancel).unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.attempts[0].arguments, original);
    assert!(saved.sections.is_empty());
    assert!(valid_findings(&saved.attempts[0].findings));
    publish(
        &mut answer,
        &ledger,
        &proposal(&evidence, "entry-flow", 0),
        &f,
        &runtime,
        &cancel,
    );
    assert_eq!(answer.value.sections.len(), 1);
    // Changing a rejected key is not proof that all omitted claims were repaired.
    assert!(answer.value.unresolved_proposals());
}

#[test]
fn final_manifest_matches_latest_versions_and_cannot_rewrite_or_omit_sections() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let first = publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let old = manifest(&a);
    let mut updated: Value = serde_json::from_str(&proposal(&id, "entry", 0)).unwrap();
    updated["previousGenerationId"] = json!(first);
    updated["section"]["title"] = json!("Formatting the supplied name");
    let second = publish(&mut a, &l, &updated.to_string(), &f, &r, &c);
    assert!(a.finish_manifest(&old, false).is_err());
    assert_eq!(
        inspect(&f.storage, &l.record.binding.operation_id)
            .unwrap()
            .final_responses[0]
            .text,
        old
    );
    assert!(a.finish_manifest(&format!("{{\"manifestVersion\":\"answerManifestV1\",\"sections\":[],\"incompleteTopics\":[]}}"),false).is_err());
    assert_eq!(a.value.sections.len(), 2);
    let final_text = manifest(&a);
    a.finish_manifest(&final_text, false).unwrap();
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert_eq!(saved.outcome, Outcome::Complete);
    assert_eq!(saved.latest_identities()[0].generation_id, second);
    let summary = serde_json::to_string(&saved.summary()).unwrap();
    assert!(!summary.contains(SIMPLE_SOURCE));
    assert!(!summary.contains("arguments"));
    assert!(!summary.contains("accountIdentity"));
}

#[test]
fn unresolved_rejection_or_budget_stop_cannot_claim_a_complete_answer() {
    for budget in [false, true] {
        let f = Fixture::new();
        let (mut l, r, c) = setup(&f);
        let id = read(&mut l, &f, &r, &c);
        let mut a = start(&l, &f, &r, &c);
        publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
        if !budget {
            let p = a.prepare(&l, "{", &f.manifest, &r, &c).unwrap();
            a.commit(&l, p, &f.manifest, &r, &c).unwrap();
        }
        let text = manifest(&a);
        a.finish_manifest(&text, budget).unwrap();
        let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
        assert_eq!(saved.outcome, Outcome::Partial);
        assert_eq!(saved.sections.len(), 1);
        assert_eq!(saved.incomplete_topics.len(), 1);
    }
}

#[test]
fn repaired_proposal_can_complete_and_foreign_manifest_fields_fail_closed() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let mut bad: Value = serde_json::from_str(&proposal(&id, "entry", 0)).unwrap();
    bad["section"]["claims"][0]["identifiers"] = json!(["unread"]);
    let p = a
        .prepare(&l, &bad.to_string(), &f.manifest, &r, &c)
        .unwrap();
    a.commit(&l, p, &f.manifest, &r, &c).unwrap();
    publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let text = manifest(&a);
    let mut foreign: Value = serde_json::from_str(&text).unwrap();
    foreign["explanation"] = json!("unvalidated replacement");
    assert!(a.finish_manifest(&foreign.to_string(), false).is_err());
    a.finish_manifest(&text, false).unwrap();
    assert_eq!(
        inspect(&f.storage, &l.record.binding.operation_id)
            .unwrap()
            .outcome,
        Outcome::Complete
    );
}

#[test]
fn local_adoption_is_idempotent_after_stop_and_revocation_and_retention_preserves_drafts() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let generation = publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let run = &l.record.binding.operation_id;
    let project = &f.manifest.summary.project_id;
    let snapshot = &f.manifest.summary.snapshot_id;
    assert!(reading::remove(&f.storage, run, project, snapshot).is_err());
    a.finish(Outcome::Stopped, vec![]).unwrap();
    assert!(reading::remove(&f.storage, run, project, snapshot).is_err());
    fs::remove_file(
        f.storage
            .join("workspace-permissions")
            .join(format!("{}.json", l.record.binding.grant_id)),
    )
    .unwrap();
    let saved = reading::adopt(&f.storage, run, project, snapshot, &generation).unwrap();
    assert_eq!(
        reading::adopt(&f.storage, run, project, snapshot, &generation).unwrap(),
        saved
    );
    assert!(reading::adopt(&f.storage, run, "foreign", snapshot, &generation).is_err());
    assert_eq!(
        reading::list(&f.storage, project, snapshot).unwrap().len(),
        1
    );
    reading::remove(&f.storage, run, project, snapshot).unwrap();
    assert!(inspect(&f.storage, run).is_err());
    assert_eq!(
        crate::chapter_store::list_for_snapshot(&f.storage, project, snapshot).unwrap(),
        vec![saved]
    );
}

#[test]
fn recovery_marks_work_interrupted_without_reopening_or_replaying_the_investigation() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let summary = reading::interrupt(
        &f.storage,
        &l.record.binding.operation_id,
        Outcome::Interrupted,
    )
    .unwrap();
    assert_eq!(summary.outcome, Outcome::Interrupted);
    assert_eq!(summary.sections.len(), 1);
    assert_eq!(
        reading::interrupt(&f.storage, &l.record.binding.operation_id, Outcome::Failed)
            .unwrap()
            .outcome,
        Outcome::Interrupted
    );
    assert!(
        a.prepare(&l, &proposal(&id, "later", 1), &f.manifest, &r, &c)
            .and_then(|p| a.commit(&l, p, &f.manifest, &r, &c))
            .is_err()
    );
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
    assert_eq!(p.findings, vec![
        json!({"location":"/section/claims/0/identifiers/0","code":"identifierProse"}),
        json!({"location":"/section/claims/0/identifiers/0","code":"identifierEvidence"}),
        json!({"location":"/section/claims/0/evidence/0","code":"evidenceIdentifier"}),
    ]);
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
        assert_eq!(rejected.findings, vec![json!({"location":"/section/claims/0/evidence/0","code":"citation"})]);
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
        .begin_call(Tool::ProposeSection, &"f".repeat(64), &f.manifest, &r, &c)
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

#[test]
fn progressive_evidence_ids_and_receipts_cannot_reinterpret_historical_versions() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    assert_eq!(l.record.schema_version, 3);
    let range = &l.record.evidence[0].range;
    let expected = context::digest(
        &serde_json::to_vec(&(
            "codex-retrieval-v3-answer-sections",
            &l.record.binding,
            range,
        ))
        .unwrap(),
    );
    assert_eq!(id, "code:1");
    assert_eq!(l.record.evidence[0].id, format!("evidence:{expected}"));
    let mut historical = l.record.clone();
    historical.schema_version = 2;
    let old = context::digest(
        &serde_json::to_vec(&(
            "codex-retrieval-v2-native-citations",
            &historical.binding,
            range,
        ))
        .unwrap(),
    );
    assert_eq!(
        evidence_id(&historical, range).unwrap(),
        format!("evidence:{old}")
    );
    assert_ne!(old, expected);
    historical.schema_version = 1;
    let oldest =
        context::digest(&serde_json::to_vec(&(&historical.binding.project_id, range)).unwrap());
    assert_eq!(
        evidence_id(&historical, range).unwrap(),
        format!("evidence:{oldest}")
    );
    let mut a = start(&l, &f, &r, &c);
    publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    let origin = saved.sections[0]
        .preview
        .revision
        .generation_origin
        .as_ref()
        .unwrap();
    assert_eq!(origin.schema_version, 5);
    assert_eq!(
        origin.contract_version,
        "codex-retrieval-v3-answer-sections"
    );
    // A historical active session cannot acquire the new write tool or receipt.
    let grant = workspace::save_retrieval_grant(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
        &r,
    )
    .unwrap();
    let mut old = Ledger::new(
        &f.storage,
        &Uuid::new_v4().to_string(),
        &f.manifest,
        &grant,
        &r,
        "gpt-6-luna",
        "max",
    )
    .unwrap();
    assert!(
        old.begin_call(Tool::ProposeSection, &"f".repeat(64), &f.manifest, &r, &c)
            .is_err()
    );
    assert!(
        old.start_answer_sections("Another question", &"e".repeat(64), &f.manifest, &r, &c)
            .is_err()
    );
    assert!(old.record.calls.is_empty());
}

#[test]
fn ready_timing_is_persisted_once_and_independent_of_preparation_clock() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let generation = publish(&mut a, &l, &proposal(&id, "entry", 0), &f, &r, &c);
    assert_eq!(a.notice(&generation).unwrap().run_elapsed_ms, None);
    a.record_publication_time(&generation, 1000).unwrap();
    a.record_publication_time(&generation, 2000).unwrap();
    let summary = inspect(&f.storage, &l.record.binding.operation_id)
        .unwrap()
        .summary();
    assert_eq!(summary.sections[0].run_elapsed_ms, Some(1000));
    assert!(a.record_publication_time(&generation, 600001).is_err());
}

#[test]
fn visual_required_field_feedback_names_only_the_fixed_wire_position() {
    let value = json!({"visual":{"kind":"dataFlow","direction":"auto","nodes":[],"edges":[]}});
    assert_eq!(visual_shape_findings(&value), vec![json!({"location":"/section/visual/title","code":"visualField"})]);
    let malformed = json!({"visual":{"kind":23,"title":"private model text","direction":"auto","nodes":"bad","edges":[]}});
    let findings = visual_shape_findings(&malformed);
    assert_eq!(findings.len(), 2);
    assert!(valid_findings(&findings));
    assert!(!serde_json::to_string(&findings).unwrap().contains("private"));
}

#[test]
fn scene_evidence_feedback_locates_the_wrong_claim_and_preserves_the_rejected_proposal() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    let id = read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    let mut raw: Value = serde_json::from_str(&proposal(&id, "entry", 0)).unwrap();
    let mut second = raw["section"]["claims"][0].clone();
    second["prose"] = json!("The greeting function returns a formatted message containing the supplied name.");
    raw["section"]["claims"].as_array_mut().unwrap().push(second);
    raw["section"]["visual"]["nodes"][1]["evidence"][0]["claimIndex"] = json!(1);
    raw["section"]["visual"]["presentation"] = json!({"schemaVersion":1,"scenes":[
        {"claimIndex":0,"title":"Format a name","mode":"composition","elements":[
            {"kind":"card","parentIndex":null,"nodeIndex":0,"tone":"accent","value":"","valueOrigin":"none"}]},
        {"claimIndex":1,"title":"Return the message","mode":"composition","elements":[
            {"kind":"card","parentIndex":null,"nodeIndex":0,"tone":"positive","value":"","valueOrigin":"none"}]}
    ]});
    let rejected_raw = raw.to_string();
    let rejected = a.prepare(&l, &rejected_raw, &f.manifest, &r, &c).unwrap();
    assert_eq!(rejected.findings, vec![json!({"location":"/section/visual/presentation/scenes/1/elements/0/nodeIndex","code":"sceneEvidence"})]);
    assert!(valid_findings(&rejected.findings));
    assert!(rejected.generation_id.is_none());
    a.commit(&l, rejected, &f.manifest, &r, &c).unwrap();
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert!(saved.sections.is_empty());
    assert_eq!(saved.attempts[0].arguments, rejected_raw);
    raw["section"]["visual"]["presentation"]["scenes"][1]["elements"][0]["nodeIndex"] = json!(1);
    publish(&mut a, &l, &raw.to_string(), &f, &r, &c);
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert_eq!(saved.attempts.len(), 2);
    assert_eq!(saved.sections.len(), 1);
}

#[test]
fn scene_evidence_diagnostics_are_bounded_source_free_and_skip_unknown_shapes() {
    let mut section = json!({"visual":{"nodes":[{"label":"private label","evidence":[{"claimIndex":1,"evidenceIndex":0}]}],
        "presentation":{"schemaVersion":1,"scenes":[{"claimIndex":0,"title":"private heading","mode":"composition","elements":[
            {"kind":"code","parentIndex":null,"nodeIndex":0,"tone":"accent","value":"private source","valueOrigin":"source"}]}]}}});
    let findings = scene_evidence_findings(&section);
    assert_eq!(findings.len(), 1);
    assert!(valid_findings(&findings));
    assert!(!serde_json::to_string(&findings).unwrap().contains("private"));
    section["visual"]["nodes"][0]["evidence"].as_array_mut().unwrap().push(json!({"claimIndex":0,"evidenceIndex":0}));
    assert!(scene_evidence_findings(&section).is_empty());
    section["visual"]["nodes"][0]["evidence"] = json!([]);
    section["visual"]["presentation"]["scenes"][0]["mode"] = json!("graph");
    assert!(scene_evidence_findings(&section).is_empty());
    section["visual"]["presentation"]["scenes"][0]["mode"] = json!("unknown");
    assert!(scene_evidence_findings(&section).is_empty());
    assert!(scene_evidence_findings(&json!({})).is_empty());
    let elements: Vec<_> = (0..48).map(|_| json!({"kind":"card","parentIndex":null,"nodeIndex":0,"tone":"accent","value":"","valueOrigin":"none"})).collect();
    let scenes: Vec<_> = (0..20).map(|index| json!({"claimIndex":index,"title":"A bounded scene","mode":"composition","elements":elements})).collect();
    section["visual"]["presentation"]["scenes"] = json!(scenes);
    let bounded = scene_evidence_findings(&section);
    assert_eq!(bounded.len(), 64);
    assert!(valid_findings(&bounded));
}

#[test]
fn legacy_long_reference_answers_replay_without_becoming_short_reference_records() {
    let f = Fixture::new();
    let (mut l, r, c) = setup(&f);
    read(&mut l, &f, &r, &c);
    let mut a = start(&l, &f, &r, &c);
    a.value.schema_version = 1;
    fs::write(&a.path, encode_answer(&a.value).unwrap()).unwrap();
    let mut raw: Value =
        serde_json::from_str(&proposal(&l.record.evidence[0].id, "entry", 0)).unwrap();
    raw["proposalVersion"] = json!(LEGACY_PROPOSAL_VERSION);
    publish(&mut a, &l, &raw.to_string(), &f, &r, &c);
    let before = fs::read(&a.path).unwrap();
    let saved = inspect(&f.storage, &l.record.binding.operation_id).unwrap();
    assert_eq!(saved.schema_version, 1);
    assert_eq!(saved.sections.len(), 1);
    assert_eq!(fs::read(&a.path).unwrap(), before);
    // A record version cannot be changed to reinterpret its references.
    let mut tampered: Value = serde_json::from_slice(&before).unwrap();
    tampered["schemaVersion"] = json!(2);
    fs::write(&a.path, serde_json::to_vec(&tampered).unwrap()).unwrap();
    assert!(inspect(&f.storage, &l.record.binding.operation_id).is_err());
    tampered["schemaVersion"] = json!(3);
    fs::write(&a.path, serde_json::to_vec(&tampered).unwrap()).unwrap();
    assert!(inspect(&f.storage, &l.record.binding.operation_id).is_err());
}

#[test]
fn partial_answer_retains_fixed_actual_failure_without_losing_its_published_section() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancelled) = setup(&f);
    let id = read(&mut ledger, &f, &runtime, &cancelled);
    let mut answer = start(&ledger, &f, &runtime, &cancelled);
    publish(
        &mut answer,
        &ledger,
        &proposal(&id, "entry", 0),
        &f,
        &runtime,
        &cancelled,
    );
    let before = serde_json::to_value(&answer.value.sections).unwrap();
    let run = &ledger.record.binding.operation_id;
    let saved = reading::interrupt_with_failure(
        &f.storage,
        run,
        Outcome::Partial,
        Some(FailureCode::from_error(
            "The agent supplied invalid captured-code arguments.",
        )),
    )
    .unwrap();
    assert_eq!(saved.outcome, Outcome::Partial);
    assert_eq!(saved.sections.len(), 1);
    // Technical diagnostics stay in the private record, outside the UI summary.
    assert!(
        serde_json::to_value(saved)
            .unwrap()
            .get("failureCode")
            .is_none()
    );
    let replay = inspect(&f.storage, run).unwrap();
    assert_eq!(replay.failure_code, Some(FailureCode::InvalidToolArguments));
    assert_eq!(serde_json::to_value(&replay.sections).unwrap(), before);
    let bytes = fs::read(&answer.path).unwrap();
    reading::interrupt_with_failure(&f.storage, run, Outcome::Failed, Some(FailureCode::Unknown))
        .unwrap();
    assert_eq!(fs::read(&answer.path).unwrap(), bytes);
}

#[test]
fn interrupted_answer_does_not_invent_historical_causes_or_retain_raw_error_text() {
    let f = Fixture::new();
    let (ledger, runtime, cancelled) = setup(&f);
    let answer = start(&ledger, &f, &runtime, &cancelled);
    let run = &ledger.record.binding.operation_id;
    assert_eq!(
        FailureCode::from_error("private path /user/repo/secret; arbitrary provider error"),
        FailureCode::Unknown
    );
    assert_eq!(
        FailureCode::from_error("prefix: The agent supplied invalid captured-code arguments."),
        FailureCode::Unknown
    );
    let summary = reading::interrupt_with_failure(
        &f.storage,
        run,
        Outcome::Partial,
        Some(FailureCode::Unknown),
    )
    .unwrap();
    assert_eq!(summary.outcome, Outcome::Failed);
    let bytes = fs::read(&answer.path).unwrap();
    assert_eq!(
        inspect(&f.storage, run).unwrap().failure_code,
        Some(FailureCode::Unknown)
    );
    assert!(!String::from_utf8_lossy(&bytes).contains("private path"));
    // Original records lacking the optional diagnostic remain readable and
    // terminal recovery cannot backfill an unobserved cause.
    let mut historical: Value = serde_json::from_slice(&bytes).unwrap();
    historical.as_object_mut().unwrap().remove("failureCode");
    let original = serde_json::to_vec(&historical).unwrap();
    fs::write(&answer.path, &original).unwrap();
    assert_eq!(inspect(&f.storage, run).unwrap().failure_code, None);
    reading::interrupt_with_failure(
        &f.storage,
        run,
        Outcome::Failed,
        Some(FailureCode::ContextDelivery),
    )
    .unwrap();
    assert_eq!(fs::read(&answer.path).unwrap(), original);
}

#[test]
fn failure_diagnostics_reject_unknown_null_and_nonterminal_metadata() {
    let f = Fixture::new();
    let (ledger, runtime, cancelled) = setup(&f);
    let answer = start(&ledger, &f, &runtime, &cancelled);
    let run = &ledger.record.binding.operation_id;
    let original = fs::read(&answer.path).unwrap();
    assert!(reading::interrupt_with_failure(
        &f.storage, run, Outcome::Partial, Some(FailureCode::Stopped),
    ).is_err());
    assert_eq!(fs::read(&answer.path).unwrap(), original);
    for value in [
        json!(null),
        json!("arbitraryErrorText"),
        json!("contextDelivery"),
        json!("stopped"),
    ] {
        let mut invalid: Value = serde_json::from_slice(&original).unwrap();
        invalid["failureCode"] = value;
        fs::write(&answer.path, serde_json::to_vec(&invalid).unwrap()).unwrap();
        assert!(inspect(&f.storage, run).is_err());
    }
}

#[test]
fn known_terminal_failures_use_only_fixed_classifications() {
    for (message, code) in [
        (
            "The code context packet timed out.",
            FailureCode::ContextTimeout,
        ),
        (
            "The context peer closed an incomplete packet.",
            FailureCode::ContextPeerClosed,
        ),
        (
            "The context transport disconnected.",
            FailureCode::ContextTransport,
        ),
        (
            "The context transport could not use nonblocking I/O.",
            FailureCode::ContextTransport,
        ),
        (
            "Context delivery is uncertain. This operation stopped; no automatic retry is allowed.",
            FailureCode::ContextDelivery,
        ),
        (
            "The captured-code tool did not complete safely.",
            FailureCode::ToolEvent,
        ),
        (
            "The Codex run exceeded ten minutes and was stopped. Review again before retrying.",
            FailureCode::RunTimeout,
        ),
        (
            crate::codex_identity::changed().as_str(),
            FailureCode::RuntimeIdentity,
        ),
    ] {
        assert_eq!(FailureCode::from_error(message), code);
    }
}
