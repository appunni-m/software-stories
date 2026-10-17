use super::*;

fn authored(reference: &str) -> Value {
    let mut raw = authoring::tests::native_names_proposal(reference, "entry", 0);
    raw["proposalVersion"] = json!("answerSectionV5");
    raw["section"]["claims"][0]["prose"] = json!(
        "The `Library::greeting` function formats the supplied name into a returned message."
    );
    raw
}

#[test]
fn qualified_prose_uses_a_grounded_literal_suffix_without_rewriting_claims_or_source() {
    // Metadata alignment proves the literal name, not namespace resolution or
    // semantic truth. The complete unchanged claim still needs quality review.
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
    let raw = authored(&reference);
    publish(
        &mut answer,
        &ledger,
        &raw.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.schema_version, 6);
    assert_eq!(saved.sections[0].checkpoint_version, 7);
    assert_eq!(saved.attempts[0].arguments, raw.to_string());
    assert_eq!(
        saved.sections[0].preview.revision.sections[0].claims[0].prose,
        raw["section"]["claims"][0]["prose"]
    );
    assert_eq!(
        saved.sections[0].preview.revision.sections[0].claims[0].anchors[0].excerpt,
        SIMPLE_SOURCE
    );
}

#[test]
fn older_name_policy_keeps_rejecting_qualified_names_missing_from_own_fragments() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = ledger
        .start_native_names_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            &runtime,
            &cancel,
        )
        .unwrap();
    let mut raw = authored(&reference);
    raw["proposalVersion"] = json!(NATIVE_NAMES_PROPOSAL_VERSION);
    let prepared = answer
        .prepare(&ledger, &raw.to_string(), &f.manifest, &runtime, &cancel)
        .unwrap();
    assert_eq!(
        prepared.findings,
        vec![json!({"location":"/section/claims/0/evidence/0","code":"namedIdentifierEvidence"})]
    );
    answer
        .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
        .unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.schema_version, NATIVE_NAMES_ANSWER_VERSION);
    assert_eq!(saved.attempts[0].arguments, raw.to_string());
    assert!(saved.sections.is_empty());
}

#[test]
fn grounded_manifest_and_checkpoint_replay_preserve_the_new_policy_and_all_publications() {
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
    let raw = authored(&reference).to_string();
    let generation = publish(&mut answer, &ledger, &raw, &f, &runtime, &cancel);
    assert_eq!(
        publish(&mut answer, &ledger, &raw, &f, &runtime, &cancel),
        generation
    );
    let mut revision: Value = serde_json::from_str(&raw).unwrap();
    revision["previousGenerationId"] = json!(generation);
    revision["section"]["title"] = json!("From name to message");
    let latest = publish(
        &mut answer,
        &ledger,
        &revision.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    assert!(
        answer
            .value
            .manifest_topics(
                r#"{"manifestVersion":"answerManifestV1","sections":[],"incompleteTopics":[]}"#
            )
            .is_err()
    );
    answer
        .finish_manifest(
            r#"{"manifestVersion":"answerManifestV2","incompleteTopics":[]}"#,
            false,
        )
        .unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.outcome, Outcome::Complete);
    assert_eq!(saved.sections.len(), 2);
    assert_eq!(saved.latest_identities()[0].generation_id, latest);
    let bytes = fs::read(&answer.path).unwrap();
    for mutation in ["checkpoint", "schema", "content"] {
        let mut damaged: Value = serde_json::from_slice(&bytes).unwrap();
        match mutation {
            "checkpoint" => damaged["sections"][0]["checkpointVersion"] = json!(6),
            "schema" => damaged["schemaVersion"] = json!(5),
            "content" => {
                damaged["sections"][0]["preview"]["revision"]["sections"][0]["claims"][0]["prose"] =
                    json!("Changed content is not a name repair.")
            }
            _ => unreachable!(),
        }
        assert!(
            replay_answer(
                &f.storage,
                &ledger.record.binding.operation_id,
                &serde_json::to_vec(&damaged).unwrap()
            )
            .is_err(),
            "{mutation}"
        );
        assert_eq!(fs::read(&answer.path).unwrap(), bytes);
    }
}

#[test]
fn grounded_proposals_preserve_authority_rejections_raw_arguments_and_the_attempt_budget() {
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
    let raw = authored(&reference);
    let mut changed = runtime.clone();
    changed.account_revision += 1;
    assert!(
        answer
            .prepare(&ledger, &raw.to_string(), &f.manifest, &changed, &cancel)
            .is_err()
    );
    cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    assert!(
        answer
            .prepare(&ledger, &raw.to_string(), &f.manifest, &runtime, &cancel)
            .is_err()
    );
    cancel.store(false, std::sync::atomic::Ordering::SeqCst);
    assert!(answer.value.attempts.is_empty());
    publish(
        &mut answer,
        &ledger,
        &raw.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let original = serde_json::to_value(&answer.value.sections).unwrap();
    for case in [
        "oldVersion",
        "unknownName",
        "unknownEvidence",
        "providerMetadata",
        "unknownName",
        "unknownName",
        "unknownName",
    ] {
        let mut rejected = authored(&reference);
        rejected["sectionKey"] = json!("deeper");
        rejected["order"] = json!(1);
        match case {
            "oldVersion" => rejected["proposalVersion"] = json!(NATIVE_NAMES_PROPOSAL_VERSION),
            "unknownName" => {
                rejected["section"]["claims"][0]["prose"] =
                    json!("The `Other::unread_helper` returns the message.")
            }
            "unknownEvidence" => {
                rejected["section"]["claims"][0]["evidence"][0]["referenceId"] = json!("code:99")
            }
            "providerMetadata" => {
                rejected["section"]["claims"][0]["identifiers"] = json!(["greeting"])
            }
            _ => unreachable!(),
        }
        let text = rejected.to_string();
        let prepared = answer
            .prepare(&ledger, &text, &f.manifest, &runtime, &cancel)
            .unwrap();
        assert!(!prepared.findings.is_empty(), "{case}");
        assert!(valid_findings(
            &prepared.findings,
            GROUNDED_NAMES_ANSWER_VERSION
        ));
        answer
            .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
            .unwrap();
        assert_eq!(answer.value.attempts.last().unwrap().arguments, text);
    }
    assert_eq!(answer.value.attempts.len(), 8);
    assert!(
        answer
            .prepare(&ledger, &raw.to_string(), &f.manifest, &runtime, &cancel)
            .is_err()
    );
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(serde_json::to_value(&saved.sections).unwrap(), original);
    assert_eq!(saved.attempts.len(), 8);
}
