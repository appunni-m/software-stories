use super::*;

fn authored(reference: &str, key: &str, order: usize) -> Value {
    authoring::tests::native_names_proposal(reference, key, order)
}

fn start_native_names(
    ledger: &Ledger,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) -> ActiveAnswer {
    ledger
        .start_native_names_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            runtime,
            cancel,
        )
        .unwrap()
}

#[test]
fn fresh_metadata_failures_preserve_raw_proposals_publications_and_attempt_limits() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let ledger_before = ledger.record_sha256().unwrap();
    let mut answer = start_native_names(&ledger, &f, &runtime, &cancel);
    let raw = authored(&reference, "entry", 0).to_string();
    let generation = publish(&mut answer, &ledger, &raw, &f, &runtime, &cancel);
    let original = serde_json::to_value(&answer.value.sections).unwrap();
    assert_eq!(
        publish(&mut answer, &ledger, &raw, &f, &runtime, &cancel),
        generation
    );
    let mut rejected = Vec::new();
    for case in [
        "unmarked",
        "unknownName",
        "unknownReference",
        "oldMetadata",
        "unmarked",
        "unmarked",
    ] {
        let mut raw = authored(&reference, "deeper", 1);
        let claim = &mut raw["section"]["claims"][0];
        let (code, location) = match case {
            "unmarked" => {
                claim["prose"] = json!("The greeting function formats a name into a message.");
                ("namedIdentifier", "/section/claims/0/prose")
            }
            "unknownName" => {
                claim["prose"] = json!("The `unread_helper` returns the formatted message.");
                ("namedIdentifierEvidence", "/section/claims/0/evidence/0")
            }
            "unknownReference" => {
                claim["evidence"][0]["referenceId"] = json!("code:99");
                for node in claim["scene"]["nodes"].as_array_mut().unwrap() {
                    node["references"] = json!(["code:99"]);
                }
                claim["scene"]["edges"][0]["references"] = json!(["code:99"]);
                ("citation", "/section/claims/0/evidence/0")
            }
            "oldMetadata" => {
                claim["identifiers"] = json!(["not_in_prose"]);
                ("namedIdentifier", "/section/claims/0/identifiers")
            }
            _ => unreachable!(),
        };
        let raw = raw.to_string();
        let prepared = answer
            .prepare(&ledger, &raw, &f.manifest, &runtime, &cancel)
            .unwrap();
        assert_eq!(
            prepared.findings,
            vec![json!({"location":location,"code":code})]
        );
        assert!(valid_findings(&prepared.findings, 5));
        answer
            .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
            .unwrap();
        rejected.push(raw);
    }
    assert_eq!(answer.value.attempts.len(), 8);
    assert!(
        answer
            .prepare(&ledger, &raw, &f.manifest, &runtime, &cancel)
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(&answer.value.sections).unwrap(),
        original
    );
    for (attempt, raw) in answer.value.attempts[2..].iter().zip(rejected) {
        assert_eq!(attempt.arguments, raw);
    }
    assert_eq!(ledger.record_sha256().unwrap(), ledger_before);
    answer
        .finish_manifest(
            r#"{"manifestVersion":"answerManifestV2","incompleteTopics":[]}"#,
            false,
        )
        .unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.outcome, Outcome::Partial);
    assert_eq!(saved.sections.len(), 1);
}

#[test]
fn fresh_revisions_and_native_manifest_keep_all_content_and_strict_checkpoint_identity() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_native_names(&ledger, &f, &runtime, &cancel);
    let raw = authored(&reference, "entry", 0);
    let original = publish(
        &mut answer,
        &ledger,
        &raw.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let mut revision = raw.clone();
    revision["previousGenerationId"] = json!(original);
    revision["section"]["title"] = json!("Formatting the supplied name");
    let latest = publish(
        &mut answer,
        &ledger,
        &revision.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    publish(
        &mut answer,
        &ledger,
        &authored(&reference, "result", 1).to_string(),
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
    let complete = r#"{"manifestVersion":"answerManifestV2","incompleteTopics":[]}"#;
    answer.finish_manifest(complete, false).unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.outcome, Outcome::Complete);
    assert_eq!(saved.sections.len(), 3);
    assert_eq!(saved.latest_identities()[0].generation_id, latest);
    assert_eq!(saved.attempts[0].arguments, raw.to_string());
    assert_eq!(saved.attempts[1].arguments, revision.to_string());
    let bytes = fs::read(&answer.path).unwrap();
    for mutation in ["checkpoint", "schema", "content"] {
        let mut damaged: Value = serde_json::from_slice(&bytes).unwrap();
        match mutation {
            "checkpoint" => damaged["sections"][0]["checkpointVersion"] = json!(5),
            "schema" => damaged["schemaVersion"] = json!(4),
            "content" => {
                damaged["sections"][0]["preview"]["revision"]["sections"][0]["claims"][0]["prose"] =
                    json!("Changed content must not be admitted.")
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
fn new_metadata_cannot_bypass_cancellation_runtime_identity_or_historical_alignment() {
    for schema in [3, 4, 5] {
        let f = Fixture::new();
        let (mut ledger, runtime, cancel) = setup(&f);
        let reference = read(&mut ledger, &f, &runtime, &cancel);
        let answer = ledger
            .start_versioned_answer_sections(
                "How is the greeting created?",
                &"d".repeat(64),
                &f.manifest,
                &runtime,
                &cancel,
                schema,
            )
            .unwrap();
        let mut raw = if schema == 5 {
            authored(&reference, "entry", 0)
        } else {
            authoring::tests::proposal(&reference, "entry", 0)
        };
        let bytes = fs::read(&answer.path).unwrap();
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
        if schema != 5 {
            raw["section"]["claims"][0]["identifiers"] =
                json!(["greeting", "absent_extra_metadata"]);
            let prepared = answer
                .prepare(&ledger, &raw.to_string(), &f.manifest, &runtime, &cancel)
                .unwrap();
            assert!(
                prepared
                    .findings
                    .iter()
                    .any(|f| f["code"] == "identifierProse")
            );
            assert!(!valid_findings(
                &[json!({"location":"/section/claims/0/prose","code":"namedIdentifier"})],
                schema
            ));
        }
        assert_eq!(fs::read(&answer.path).unwrap(), bytes);
    }
}

#[test]
fn native_names_never_admit_undelivered_or_changed_captured_source() {
    for state in ["prepared", "uncertain", "corrupt"] {
        let f = Fixture::new();
        let (mut ledger, runtime, cancel) = setup(&f);
        let reference = read(&mut ledger, &f, &runtime, &cancel);
        let answer = start_native_names(&ledger, &f, &runtime, &cancel);
        let before = fs::read(&answer.path).unwrap();
        match state {
            "prepared" => ledger.record.calls[0].state = CallState::Prepared,
            "uncertain" => ledger.record.calls[0].state = CallState::Uncertain,
            "corrupt" => {
                let file = f
                    .manifest
                    .files
                    .iter()
                    .find(|file| file.path == SIMPLE_PATH)
                    .unwrap();
                fs::write(f.storage.join(&file.blob_path), b"changed immutable object").unwrap();
            }
            _ => unreachable!(),
        }
        let result = answer.prepare(
            &ledger,
            &authored(&reference, "entry", 0).to_string(),
            &f.manifest,
            &runtime,
            &cancel,
        );
        if state == "corrupt" {
            assert!(result.is_err());
        } else {
            let prepared = result.unwrap();
            assert_eq!(
                prepared.findings,
                vec![json!({"location":"/section/claims/0/evidence/0","code":"citation"})]
            );
            assert!(prepared.generation_id.is_none());
        }
        assert_eq!(fs::read(&answer.path).unwrap(), before);
    }
}

#[test]
fn native_named_prose_publishes_without_duplicate_metadata_and_replays_exactly() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_native_names(&ledger, &f, &runtime, &cancel);
    let raw = authored(&reference, "entry", 0).to_string();
    publish(&mut answer, &ledger, &raw, &f, &runtime, &cancel);
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.attempts[0].arguments, raw);
    assert_eq!(saved.sections[0].checkpoint_version, 6);
    let claim = &saved.sections[0].preview.revision.sections[0].claims[0];
    assert!(claim.prose.contains("`greeting`"));
    assert_eq!(claim.anchors[0].excerpt, SIMPLE_SOURCE);
    assert_eq!(
        answer.value.sections[0].preview.revision,
        saved.sections[0].preview.revision
    );
}
