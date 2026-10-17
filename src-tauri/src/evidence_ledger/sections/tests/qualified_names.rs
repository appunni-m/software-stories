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
