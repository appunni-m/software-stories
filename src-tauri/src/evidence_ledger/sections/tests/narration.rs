use super::*;

fn narrated(reference: &str) -> Value {
    let mut raw = authoring::tests::grounded_names_proposal(reference, "entry", 0);
    raw["proposalVersion"] = json!(NARRATION_PROPOSAL_VERSION);
    raw["section"]["visual"] = json!({"kind":"none","title":"","direction":"auto"});
    raw["section"]["claims"][0]["scene"] = json!({
        "title":"A name becomes a greeting", "mode":"narration",
        "narration":"Start with a person's name. It is placed inside a greeting, so that name appears in the returned message.",
        "nodes":[],"edges":[],"elements":[]
    });
    raw
}

fn start_narrated(
    ledger: &Ledger,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) -> ActiveAnswer {
    ledger
        .start_narrated_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            runtime,
            cancel,
        )
        .unwrap()
}

#[test]
fn all_nonvisual_claim_speech_is_published_and_replayed_without_manufacturing_a_visual() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_narrated(&ledger, &f, &runtime, &cancel);
    let mut raw = narrated(&reference);
    let mut inference = raw["section"]["claims"][0].clone();
    inference["category"] = json!("inference");
    inference["inferenceBasis"] = json!(
        "The captured function returns a formatted greeting but does not show its consumers."
    );
    inference["prose"] = json!("The `greeting` result could be used for display, but the selected function does not prove a caller does so.");
    inference["scene"]["narration"] = json!("That message could be shown to someone, but this code alone does not tell us whether a caller displays it.");
    raw["section"]["claims"]
        .as_array_mut()
        .unwrap()
        .push(inference);
    publish(
        &mut answer,
        &ledger,
        &raw.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let path = record_path(&f.storage, &ledger.record.binding.operation_id).unwrap();
    let original = fs::read(&path).unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.schema_version, NARRATION_ANSWER_VERSION);
    let publication = &saved.sections[0];
    assert_eq!(publication.checkpoint_version, NARRATION_CHECKPOINT_VERSION);
    let revision = &publication.preview.revision;
    let section = &revision.sections[0];
    assert_eq!(revision.schema_version, 5);
    assert!(section.visual.is_none());
    assert_eq!(section.claims.len(), 2);
    assert_eq!(
        section.spoken_text,
        format!(
            "{} Inference: {}",
            raw["section"]["claims"][0]["scene"]["narration"]
                .as_str()
                .unwrap(),
            raw["section"]["claims"][1]["scene"]["narration"]
                .as_str()
                .unwrap()
        )
    );
    assert_ne!(section.prose, section.spoken_text);
    assert_eq!(
        section.claims[0].prose,
        raw["section"]["claims"][0]["prose"].as_str().unwrap()
    );
    assert_eq!(section.claims[0].anchors[0].excerpt, SIMPLE_SOURCE);
    assert_eq!(saved.attempts[0].arguments, raw.to_string());
    assert_eq!(original, fs::read(&path).unwrap());
    assert!(chapters::encode_chapter_revision(revision).is_ok());
}

#[test]
fn nonvisual_speech_is_required_at_every_claim_and_correction_keeps_spent_attempts() {
    for field in [
        "null",
        "empty",
        "visual-content",
        "wrong-mode",
        "extra",
        "utf8-limit",
    ] {
        let f = Fixture::new();
        let (mut ledger, runtime, cancel) = setup(&f);
        let reference = read(&mut ledger, &f, &runtime, &cancel);
        let mut answer = start_narrated(&ledger, &f, &runtime, &cancel);
        let good = narrated(&reference);
        let mut bad = good.clone();
        let scene = &mut bad["section"]["claims"][0]["scene"];
        match field {
            "null" => *scene = Value::Null,
            "empty" => scene["narration"] = json!(" "),
            "visual-content" => scene["nodes"] = json!([{}]),
            "wrong-mode" => scene["mode"] = json!("graph"),
            "extra" => scene["script"] = json!("untrusted()"),
            "utf8-limit" => scene["narration"] = json!("é".repeat(501)),
            _ => unreachable!(),
        }
        let prepared = answer
            .prepare(&ledger, &bad.to_string(), &f.manifest, &runtime, &cancel)
            .unwrap();
        assert!(prepared.generation_id.is_none(), "{field}");
        assert!(!prepared.findings.is_empty(), "{field}");
        answer
            .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
            .unwrap();
        assert!(answer.value.sections.is_empty());
        publish(
            &mut answer,
            &ledger,
            &good.to_string(),
            &f,
            &runtime,
            &cancel,
        );
        let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
        assert_eq!(saved.attempts.len(), 2);
        assert_eq!(saved.attempts[0].arguments, bad.to_string());
        assert_eq!(saved.sections.len(), 1);
    }
}

#[test]
fn changing_only_authored_speech_requires_a_new_immutable_section_revision() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_narrated(&ledger, &f, &runtime, &cancel);
    let raw = narrated(&reference);
    publish(
        &mut answer,
        &ledger,
        &raw.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let first = answer.value.sections[0].clone();
    let mut changed = raw.clone();
    changed["section"]["claims"][0]["scene"]["narration"] = json!("Imagine supplying a name. The returned greeting contains that same name, wrapped in a friendly message.");
    let rejected = answer
        .prepare(
            &ledger,
            &changed.to_string(),
            &f.manifest,
            &runtime,
            &cancel,
        )
        .unwrap();
    assert!(rejected.generation_id.is_none());
    answer
        .commit(&ledger, rejected, &f.manifest, &runtime, &cancel)
        .unwrap();
    changed["previousGenerationId"] = json!(first.preview.generation_id);
    publish(
        &mut answer,
        &ledger,
        &changed.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.sections.len(), 2);
    assert_eq!(
        serde_json::to_value(&saved.sections[0]).unwrap(),
        serde_json::to_value(first).unwrap()
    );
    assert_ne!(
        saved.sections[0].preview.revision.sections[0].spoken_text,
        saved.sections[1].preview.revision.sections[0].spoken_text
    );
}

#[test]
fn old_nonvisual_proposals_keep_their_original_claim_copy_script_and_schema_on_replay() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = ledger
        .start_grounded_names_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            &runtime,
            &cancel,
        )
        .unwrap();
    let mut raw = narrated(&reference);
    raw["proposalVersion"] = json!(GROUNDED_NAMES_PROPOSAL_VERSION);
    raw["section"]["claims"][0]["scene"] = Value::Null;
    publish(
        &mut answer,
        &ledger,
        &raw.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let before =
        fs::read(record_path(&f.storage, &ledger.record.binding.operation_id).unwrap()).unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    let revision = &saved.sections[0].preview.revision;
    assert_eq!(revision.schema_version, 4);
    assert_eq!(saved.sections[0].checkpoint_version, 7);
    assert_eq!(revision.sections[0].spoken_text, revision.sections[0].prose);
    assert_eq!(
        before,
        fs::read(record_path(&f.storage, &ledger.record.binding.operation_id).unwrap()).unwrap()
    );
}

#[test]
fn narration_limits_include_all_claims_and_publish_nothing_on_failure() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_narrated(&ledger, &f, &runtime, &cancel);
    let mut raw = narrated(&reference);
    let mut claim = raw["section"]["claims"][0].clone();
    claim["scene"]["narration"] = json!("é".repeat(500));
    raw["section"]["claims"] = json!(vec![claim; 5]);
    let prepared = answer
        .prepare(&ledger, &raw.to_string(), &f.manifest, &runtime, &cancel)
        .unwrap();
    assert!(prepared.generation_id.is_none());
    assert_eq!(prepared.findings, finding("presentationLimit"));
    answer
        .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
        .unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert!(saved.sections.is_empty());
    assert_eq!(saved.attempts.len(), 1);
    assert_eq!(saved.attempts[0].arguments, raw.to_string());
}
