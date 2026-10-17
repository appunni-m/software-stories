//! Fresh final responses declare unfinished topics; native publication owns
//! every section identity. Historical manifests still require their exact list.

use super::*;

const COMPLETE: &str = r#"{"manifestVersion":"answerManifestV2","incompleteTopics":[]}"#;

fn start_native(
    ledger: &Ledger,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) -> ActiveAnswer {
    ledger
        .start_native_manifest_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            runtime,
            cancel,
        )
        .unwrap()
}

#[test]
fn native_manifest_keeps_every_published_section_and_exact_superseded_bytes() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_native(&ledger, &f, &runtime, &cancel);
    let mut generations = Vec::new();
    for (order, key) in ["entry", "transform", "result"].into_iter().enumerate() {
        generations.push(publish(
            &mut answer,
            &ledger,
            &authoring::tests::proposal(&reference, key, order).to_string(),
            &f,
            &runtime,
            &cancel,
        ));
    }
    let mut revision = authoring::tests::proposal(&reference, "entry", 0);
    revision["previousGenerationId"] = json!(generations[0]);
    revision["section"]["title"] = json!("Formatting the supplied name");
    let updated = publish(
        &mut answer,
        &ledger,
        &revision.to_string(),
        &f,
        &runtime,
        &cancel,
    );
    let original = serde_json::to_value(&answer.value.sections).unwrap();
    let attempts = serde_json::to_value(&answer.value.attempts).unwrap();
    answer.finish_manifest(COMPLETE, false).unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.schema_version, 4);
    assert_eq!(saved.outcome, Outcome::Complete);
    assert_eq!(
        saved.latest_identities(),
        vec![
            reading::SectionIdentity {
                section_key: "entry".into(),
                generation_id: updated
            },
            reading::SectionIdentity {
                section_key: "transform".into(),
                generation_id: generations[1].clone()
            },
            reading::SectionIdentity {
                section_key: "result".into(),
                generation_id: generations[2].clone()
            },
        ]
    );
    assert_eq!(serde_json::to_value(&saved.sections).unwrap(), original);
    assert_eq!(serde_json::to_value(&saved.attempts).unwrap(), attempts);
    assert!(saved.sections.iter().all(|s| s.checkpoint_version == 5));
    assert_eq!(saved.final_responses.len(), 1);
    assert_eq!(saved.final_responses[0].text, COMPLETE);
    assert_eq!(
        saved.final_responses[0].sha256,
        context::digest(COMPLETE.as_bytes())
    );
    let bytes = fs::read(&answer.path).unwrap();
    reading::scoped(
        &f.storage,
        &ledger.record.binding.operation_id,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
    )
    .unwrap();
    assert_eq!(fs::read(&answer.path).unwrap(), bytes);
    assert!(answer.finish_manifest(COMPLETE, false).is_err());
    assert_eq!(fs::read(&answer.path).unwrap(), bytes);
}

#[test]
fn native_manifest_cannot_hide_budget_stop_rejections_or_explicit_unfinished_topics() {
    for cause in ["budget", "rejection", "topic"] {
        let f = Fixture::new();
        let (mut ledger, runtime, cancel) = setup(&f);
        let reference = read(&mut ledger, &f, &runtime, &cancel);
        let mut answer = start_native(&ledger, &f, &runtime, &cancel);
        publish(
            &mut answer,
            &ledger,
            &authoring::tests::proposal(&reference, "entry", 0).to_string(),
            &f,
            &runtime,
            &cancel,
        );
        if cause == "rejection" {
            let mut bad = authoring::tests::proposal(&reference, "deeper", 1);
            bad["section"]["claims"][0]["scene"]["nodes"][0]["references"] = json!(["code:99"]);
            let prepared = answer
                .prepare(&ledger, &bad.to_string(), &f.manifest, &runtime, &cancel)
                .unwrap();
            assert!(!prepared.findings.is_empty());
            answer
                .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
                .unwrap();
        }
        let text = if cause == "topic" {
            json!({"manifestVersion":"answerManifestV2","incompleteTopics":["The caller's failure handling was not investigated."]}).to_string()
        } else {
            COMPLETE.into()
        };
        let sections = serde_json::to_value(&answer.value.sections).unwrap();
        answer.finish_manifest(&text, cause == "budget").unwrap();
        let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
        assert_eq!(saved.outcome, Outcome::Partial, "{cause}");
        assert_eq!(saved.incomplete_topics.len(), 1);
        assert_eq!(serde_json::to_value(&saved.sections).unwrap(), sections);
        assert_eq!(saved.final_responses[0].text, text);
        if cause == "topic" {
            assert_eq!(
                saved.incomplete_topics[0],
                "The caller's failure handling was not investigated."
            );
        }
    }
}

#[test]
fn native_manifest_requires_a_section_and_never_turns_an_empty_run_into_an_answer() {
    let f = Fixture::new();
    let (ledger, runtime, cancel) = setup(&f);
    let mut answer = start_native(&ledger, &f, &runtime, &cancel);
    assert!(answer.finish_manifest(COMPLETE, false).is_err());
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert!(saved.sections.is_empty());
    assert_eq!(saved.final_responses[0].text, COMPLETE);
    assert_eq!(saved.outcome, Outcome::Working);
}

#[test]
fn native_manifest_strictly_rejects_foreign_identity_lists_and_preserves_bad_final_bytes() {
    let bad = [
        r#"{"manifestVersion":"answerManifestV1","sections":[],"incompleteTopics":[]}"#.into(),
        r#"{"manifestVersion":"answerManifestV2","sections":[],"incompleteTopics":[]}"#.into(),
        r#"{"manifestVersion":"answerManifestV2","incompleteTopics":null}"#.into(),
        r#"{"manifestVersion":"answerManifestV2","incompleteTopics":[],"prose":"rewrite"}"#.into(),
        r#"{"manifestVersion":"answerManifestV2","incompleteTopics":[],"incompleteTopics":["hidden"]}"#.into(),
        r#"{"manifestVersion":"answerManifestV2","incompleteTopics":[""]}"#.into(),
        json!({"manifestVersion":"answerManifestV2","incompleteTopics":["x".repeat(241)]}).to_string(),
        json!({"manifestVersion":"answerManifestV2","incompleteTopics":["é".repeat(121)]}).to_string(),
        json!({"manifestVersion":"answerManifestV2","incompleteTopics":["line\nbreak"]}).to_string(),
        json!({"manifestVersion":"answerManifestV2","incompleteTopics":vec!["boundary"; 7]}).to_string(),
        format!("{}{}", COMPLETE, " ".repeat(4096)),
    ];
    for text in bad {
        let f = Fixture::new();
        let (mut ledger, runtime, cancel) = setup(&f);
        let reference = read(&mut ledger, &f, &runtime, &cancel);
        let mut answer = start_native(&ledger, &f, &runtime, &cancel);
        publish(
            &mut answer,
            &ledger,
            &authoring::tests::proposal(&reference, "entry", 0).to_string(),
            &f,
            &runtime,
            &cancel,
        );
        let before = serde_json::to_value(&answer.value.sections).unwrap();
        assert!(answer.finish_manifest(&text, false).is_err());
        let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
        assert_eq!(saved.outcome, Outcome::Working);
        assert_eq!(saved.final_responses[0].text, text);
        assert_eq!(serde_json::to_value(&saved.sections).unwrap(), before);
        answer.finish(Outcome::Partial, vec![]).unwrap();
        assert_eq!(
            inspect(&f.storage, &ledger.record.binding.operation_id)
                .unwrap()
                .outcome,
            Outcome::Partial
        );
    }
}

#[test]
fn historical_scene_manifest_still_requires_every_exact_latest_identity() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = ledger
        .start_scene_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            &runtime,
            &cancel,
        )
        .unwrap();
    for (order, key) in ["entry", "transform", "result"].into_iter().enumerate() {
        publish(
            &mut answer,
            &ledger,
            &authoring::tests::proposal(&reference, key, order).to_string(),
            &f,
            &runtime,
            &cancel,
        );
    }
    let before = serde_json::to_value(&answer.value.sections).unwrap();
    assert!(answer.finish_manifest(COMPLETE, false).is_err());
    let mut omitted = answer.value.latest_identities();
    omitted.pop();
    let old =
        json!({"manifestVersion":"answerManifestV1","sections":omitted,"incompleteTopics":[]})
            .to_string();
    assert!(answer.finish_manifest(&old, false).is_err());
    answer.finish_manifest(&manifest(&answer), false).unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.schema_version, 3);
    assert_eq!(saved.outcome, Outcome::Complete);
    assert_eq!(serde_json::to_value(&saved.sections).unwrap(), before);
    assert_eq!(saved.final_responses[1].text, old);
}

#[test]
fn native_manifest_completion_requires_its_exact_valid_receipt_on_write_and_read() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_native(&ledger, &f, &runtime, &cancel);
    publish(
        &mut answer,
        &ledger,
        &authoring::tests::proposal(&reference, "entry", 0).to_string(),
        &f,
        &runtime,
        &cancel,
    );
    assert!(answer.finish(Outcome::Complete, vec![]).is_err());
    answer.finish_manifest(COMPLETE, false).unwrap();
    let original = fs::read(&answer.path).unwrap();
    for bad in [
        None,
        Some(r#"{"manifestVersion":"answerManifestV1","sections":[],"incompleteTopics":[]}"#),
        Some(
            r#"{"manifestVersion":"answerManifestV2","incompleteTopics":["Unexplored failure path"]}"#,
        ),
        Some(r#"{"manifestVersion":"answerManifestV2","incompleteTopics":[],"sections":[]}"#),
    ] {
        let mut value: Value = serde_json::from_slice(&original).unwrap();
        value["finalResponses"] = match bad {
            None => json!([]),
            Some(text) => json!([{"text":text,"sha256":context::digest(text.as_bytes())}]),
        };
        assert!(
            replay_answer(
                &f.storage,
                &ledger.record.binding.operation_id,
                &serde_json::to_vec(&value).unwrap()
            )
            .is_err()
        );
        assert_eq!(fs::read(&answer.path).unwrap(), original);
    }
}

#[test]
fn native_manifest_repairs_use_the_same_original_proposal_budget_and_key() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_native(&ledger, &f, &runtime, &cancel);
    let mut bad = authoring::tests::proposal(&reference, "entry", 0);
    bad["section"]["claims"][0]["scene"]["nodes"][0]["references"] = json!(["code:99"]);
    let bad = bad.to_string();
    let prepared = answer
        .prepare(&ledger, &bad, &f.manifest, &runtime, &cancel)
        .unwrap();
    assert!(
        prepared
            .findings
            .iter()
            .any(|f| f["code"] == "sceneAuthoringReference")
    );
    answer
        .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
        .unwrap();
    assert_eq!(answer.remaining_attempts(), 7);
    publish(
        &mut answer,
        &ledger,
        &authoring::tests::proposal(&reference, "entry", 0).to_string(),
        &f,
        &runtime,
        &cancel,
    );
    assert_eq!(answer.remaining_attempts(), 6);
    answer.finish_manifest(COMPLETE, false).unwrap();
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.attempts[0].arguments, bad);
    assert_eq!(saved.attempts.len(), 2);
    assert_eq!(saved.outcome, Outcome::Complete);
}

#[test]
fn native_manifest_cannot_append_after_another_owner_stops_the_saved_answer() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start_native(&ledger, &f, &runtime, &cancel);
    publish(
        &mut answer,
        &ledger,
        &authoring::tests::proposal(&reference, "entry", 0).to_string(),
        &f,
        &runtime,
        &cancel,
    );
    reading::interrupt(
        &f.storage,
        &ledger.record.binding.operation_id,
        Outcome::Stopped,
    )
    .unwrap();
    let stopped = fs::read(&answer.path).unwrap();
    assert!(answer.finish_manifest(COMPLETE, false).is_err());
    assert_eq!(fs::read(&answer.path).unwrap(), stopped);
    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.outcome, Outcome::Stopped);
    assert_eq!(saved.sections.len(), 1);
    assert!(saved.final_responses.is_empty());
}
