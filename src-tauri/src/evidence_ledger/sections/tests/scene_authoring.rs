use super::*;

fn start_scene(
    ledger: &Ledger,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) -> ActiveAnswer {
    ledger
        .start_scene_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            runtime,
            cancel,
        )
        .unwrap()
}

#[test]
fn authored_rejections_preserve_prior_drafts_and_report_only_local_positions() {
    for case in [
        "foreignReference",
        "unknownNode",
        "forwardParent",
        "duplicateKey",
        "sourceValue",
        "unreadReference",
        "missingNarration",
        "legacyPointer",
    ] {
        let f = Fixture::new();
        let (mut ledger, runtime, cancel) = setup(&f);
        let reference = read(&mut ledger, &f, &runtime, &cancel);
        let mut answer = start_scene(&ledger, &f, &runtime, &cancel);
        let first = authoring::tests::proposal(&reference, "entry", 0);
        publish(
            &mut answer,
            &ledger,
            &first.to_string(),
            &f,
            &runtime,
            &cancel,
        );
        let original = serde_json::to_value(&answer.value.sections[0]).unwrap();
        let mut raw = authoring::tests::proposal(&reference, "deeper", 1);
        let claim = &mut raw["section"]["claims"][0];
        let (code, location) = match case {
            "foreignReference" => {
                claim["scene"]["nodes"][0]["references"] = json!(["code:99"]);
                (
                    "sceneAuthoringReference",
                    "/section/claims/0/scene/nodes/0/references",
                )
            }
            "unknownNode" => {
                claim["scene"]["elements"][1]["nodeKey"] = json!("foreign-node");
                (
                    "sceneAuthoringKey",
                    "/section/claims/0/scene/elements/1/nodeKey",
                )
            }
            "forwardParent" => {
                claim["scene"]["elements"][0]["parentKey"] = json!("message");
                (
                    "sceneAuthoringKey",
                    "/section/claims/0/scene/elements/0/parentKey",
                )
            }
            "duplicateKey" => {
                claim["scene"]["nodes"][1]["key"] = json!("input");
                ("sceneAuthoringKey", "/section/claims/0/scene/nodes/1/key")
            }
            "sourceValue" => {
                claim["scene"]["elements"][1]["value"] = json!("private_unread_source");
                (
                    "sceneSourceValue",
                    "/section/claims/0/scene/elements/1/value",
                )
            }
            "unreadReference" => {
                claim["evidence"][0]["referenceId"] = json!("code:99");
                for node in claim["scene"]["nodes"].as_array_mut().unwrap() {
                    node["references"] = json!(["code:99"]);
                }
                claim["scene"]["edges"][0]["references"] = json!(["code:99"]);
                ("citation", "/section/claims/0/evidence/0")
            }
            "missingNarration" => {
                claim["scene"]["narration"] = json!("");
                ("sceneNarration", "/section/claims/0/scene/narration")
            }
            "legacyPointer" => {
                claim["scene"]["elements"][1]["nodeIndex"] = json!(0);
                ("sceneAuthoringShape", "/section/claims/0/scene")
            }
            _ => unreachable!(),
        };
        let raw = raw.to_string();
        let prepared = answer
            .prepare(&ledger, &raw, &f.manifest, &runtime, &cancel)
            .unwrap();
        assert_eq!(
            prepared.findings,
            vec![json!({"location":location,"code":code})],
            "{case}"
        );
        assert!(valid_findings(&prepared.findings, 3));
        answer
            .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
            .unwrap();
        let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
        assert_eq!(saved.attempts.len(), 2);
        assert_eq!(saved.attempts[1].arguments, raw);
        assert_eq!(saved.sections.len(), 1);
        assert_eq!(serde_json::to_value(&saved.sections[0]).unwrap(), original);
        assert!(
            !serde_json::to_string(&saved.attempts[1].findings)
                .unwrap()
                .contains("private_unread_source")
        );
    }
}

#[test]
fn mixed_graph_and_composition_keep_claim_order_natural_speech_and_local_source_links() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_scene(&ledger, &f, &runtime, &cancel);
    let mut raw = authoring::tests::proposal(&reference, "entry", 0);
    let mut graph = raw["section"]["claims"][0].clone();
    graph["scene"]["mode"] = json!("graph");
    graph["scene"]["elements"] = json!([]);
    graph["scene"]["title"] = json!("名前 → message");
    let speech = "For example, a person's name can contain Unicode. It remains inside the returned greeting.";
    graph["scene"]["narration"] = json!(speech);
    raw["section"]["claims"].as_array_mut().unwrap().push(graph);
    publish(
        &mut answer,
        &ledger,
        &raw.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    let section = &saved.sections[0].preview.revision.sections[0];
    assert_eq!(section.claims.len(), 2);
    assert_eq!(
        section.spoken_text,
        format!(
            "{} {speech}",
            raw["section"]["claims"][0]["scene"]["narration"]
                .as_str()
                .unwrap()
        )
    );
    assert!(
        section
            .claims
            .iter()
            .all(|claim| claim.anchors[0].excerpt == SIMPLE_SOURCE)
    );
    let visual = serde_json::to_value(&section.visual).unwrap();
    assert_eq!(visual["nodes"].as_array().unwrap().len(), 4);
    assert_eq!(
        visual["presentation"]["scenes"][1]["title"],
        "名前 → message"
    );
    assert_eq!(visual["presentation"]["scenes"][1]["mode"], "graph");
    assert_eq!(visual["presentation"]["scenes"][1]["claimIndex"], 1);
}

#[test]
fn scene_checkpoint_policy_cannot_be_downgraded_or_borrowed_by_legacy_answers() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_scene(&ledger, &f, &runtime, &cancel);
    let raw = authoring::tests::proposal(&reference, "entry", 0).to_string();
    for version in [1, 2, 3, 4, 6] {
        assert!(
            answer
                .prepare_with_checkpoint_version(
                    &ledger,
                    &raw,
                    &f.manifest,
                    &runtime,
                    &cancel,
                    version
                )
                .is_err()
        );
    }
    publish(&mut answer, &ledger, &raw, &f, &runtime, &cancel);
    let bytes = fs::read(&answer.path).unwrap();
    for case in [
        "answerSchema",
        "checkpoint",
        "proposal",
        "narration",
        "element",
    ] {
        let mut saved: Value = serde_json::from_slice(&bytes).unwrap();
        match case {
            "answerSchema" => saved["schemaVersion"] = json!(2),
            "checkpoint" => saved["sections"][0]["checkpointVersion"] = json!(4),
            "proposal" => {
                saved["attempts"][0]["arguments"] = json!(proposal(&reference, "entry", 0))
            }
            "narration" => {
                saved["sections"][0]["preview"]["revision"]["sections"][0]["spokenText"] =
                    json!("Different narration cannot replace the owned scene.")
            }
            "element" => {
                saved["sections"][0]["preview"]["revision"]["sections"][0]["visual"]["presentation"]
                    ["scenes"][0]["elements"][1]["value"] = json!("other")
            }
            _ => unreachable!(),
        }
        let changed = serde_json::to_vec(&saved).unwrap();
        fs::write(&answer.path, &changed).unwrap();
        assert!(
            inspect(&f.storage, &ledger.record.binding.operation_id).is_err(),
            "{case}"
        );
        assert_eq!(fs::read(&answer.path).unwrap(), changed);
    }
    fs::write(&answer.path, &bytes).unwrap();
    inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    let (mut other, runtime, cancel) = setup(&f);
    let reference = read(&mut other, &f, &runtime, &cancel);
    let old = start(&other, &f, &runtime, &cancel);
    assert!(
        old.prepare_with_checkpoint_version(
            &other,
            &proposal(&reference, "entry", 0),
            &f.manifest,
            &runtime,
            &cancel,
            5
        )
        .is_err()
    );
    assert!(!valid_findings(
        &vec![json!({"location":"/section","code":"sceneAuthoringKey"})],
        2
    ));
}

#[test]
fn correction_and_explicit_revision_use_original_attempt_budget_and_preserve_claims() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_scene(&ledger, &f, &runtime, &cancel);
    let mut raw = authoring::tests::proposal(&reference, "entry", 0);
    let original_claim = raw["section"]["claims"][0].clone();
    raw["section"]["claims"][0]["scene"]["elements"][1]["nodeKey"] = json!("missing");
    let rejected = answer
        .prepare(&ledger, &raw.to_string(), &f.manifest, &runtime, &cancel)
        .unwrap();
    answer
        .commit(&ledger, rejected, &f.manifest, &runtime, &cancel)
        .unwrap();
    raw["section"]["claims"][0] = original_claim.clone();
    let first = publish(
        &mut answer,
        &ledger,
        &raw.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let prior = serde_json::to_value(&answer.value.sections[0]).unwrap();
    raw["section"]["claims"][0]["scene"]["title"] = json!("The name reaches the message");
    let conflict = answer
        .prepare(&ledger, &raw.to_string(), &f.manifest, &runtime, &cancel)
        .unwrap();
    assert_eq!(conflict.findings, finding("sectionConflict"));
    answer
        .commit(&ledger, conflict, &f.manifest, &runtime, &cancel)
        .unwrap();
    raw["previousGenerationId"] = json!(first);
    publish(
        &mut answer,
        &ledger,
        &raw.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.attempts.len(), 4);
    assert_eq!(saved.sections.len(), 2);
    assert_eq!(
        saved.sections[1].supersedes.as_deref(),
        Some(first.as_str())
    );
    assert_eq!(serde_json::to_value(&saved.sections[0]).unwrap(), prior);
    assert_eq!(
        saved.sections[1].preview.revision.sections[0].claims[0].prose,
        original_claim["prose"]
    );
    while answer.value.attempts.len() < MAX_ATTEMPTS {
        publish(
            &mut answer,
            &ledger,
            &raw.to_string(),
            &f,
            &runtime,
            &cancel,
        );
    }
    let bytes = fs::read(&answer.path).unwrap();
    assert!(
        answer
            .prepare(&ledger, &raw.to_string(), &f.manifest, &runtime, &cancel)
            .is_err()
    );
    assert_eq!(fs::read(&answer.path).unwrap(), bytes);
}

#[test]
fn combined_narration_limit_is_not_bypassed_by_individually_valid_scenes() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_scene(&ledger, &f, &runtime, &cancel);
    let mut raw = authoring::tests::proposal(&reference, "entry", 0);
    raw["section"]["claims"][0]["scene"]["narration"] = json!("meaning ".repeat(300).trim());
    let claim = raw["section"]["claims"][0].clone();
    raw["section"]["claims"].as_array_mut().unwrap().push(claim);
    let rejected = answer
        .prepare(&ledger, &raw.to_string(), &f.manifest, &runtime, &cancel)
        .unwrap();
    assert!(
        rejected
            .findings
            .iter()
            .any(|f| f["code"] == "sceneNarration" && f["location"] == "/section/claims")
    );
    answer
        .commit(&ledger, rejected, &f.manifest, &runtime, &cancel)
        .unwrap();
    assert!(
        inspect(&f.storage, &ledger.record.binding.operation_id)
            .unwrap()
            .sections
            .is_empty()
    );
}

#[test]
fn scene_local_proposals_publish_with_exact_source_and_replay_without_an_agent() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let evidence = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = ledger
        .start_scene_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            &runtime,
            &cancel,
        )
        .unwrap();
    let raw = authoring::tests::proposal(&evidence, "entry", 0).to_string();
    let generation = publish(&mut answer, &ledger, &raw, &f, &runtime, &cancel);
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.schema_version, 3);
    assert_eq!(saved.sections[0].checkpoint_version, 5);
    assert_eq!(saved.sections[0].preview.generation_id, generation);
    let section = &saved.sections[0].preview.revision.sections[0];
    assert_eq!(
        section.spoken_text,
        "Start with a person's name. The function places it inside a greeting, so that name appears in the returned message."
    );
    assert!(section.prose.contains("greeting"));
    assert_eq!(section.claims[0].anchors[0].excerpt, SIMPLE_SOURCE);
    let bytes = fs::read(&answer.path).unwrap();
    assert_eq!(
        publish(&mut answer, &ledger, &raw, &f, &runtime, &cancel),
        generation
    );
    assert_eq!(answer.value.sections.len(), 1);
    // Inspection preserves durable bytes and cannot connect to a model.
    let before = fs::read(&answer.path).unwrap();
    inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(fs::read(&answer.path).unwrap(), before);
    assert!(!bytes.is_empty());
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}
