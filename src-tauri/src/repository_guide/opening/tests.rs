use super::*;
use crate::{chapter_generation::tests::Fixture, repository_guide::tests::record};

pub(in crate::repository_guide) fn pending(f: &Fixture) -> Record {
    let previous = record(f);
    let mut r = new_record(
        Start {
            id: previous.id,
            project_id: previous.project_id,
            snapshot_id: previous.snapshot_id,
            grant_id: previous.grant_id,
            question: previous.question,
            consent: true,
        },
        &f.manifest,
    )
    .unwrap();
    r.active_run_id = Some(r.planning_run_id.clone());
    r
}

#[test]
fn combined_contract_keeps_the_same_scene_and_outline_boundaries() {
    let schema = schema(super::super::plan::schema());
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["outline"]["properties"]["sections"]["maxItems"],
        6
    );
    assert_eq!(
        schema["properties"]["openingAnswer"],
        workspace_generation::progressive::schema()
    );
    let text = json!({"outline":{"marker":"not yet resolved"},"openingAnswer":{"manifestVersion":"answerManifestV1"}}).to_string();
    let (outline, manifest) = split(&text).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&outline).unwrap()["marker"],
        "not yet resolved"
    );
    assert!(manifest.contains("answerManifestV1"));
    let raw_manifest = "{\n  \"manifestVersion\" : \"answerManifestV1\"\n}";
    assert_eq!(
        split(&format!(
            "{{\"outline\":{{}},\"openingAnswer\":{raw_manifest}}}"
        ))
        .unwrap()
        .1,
        raw_manifest
    );
    for bad in [
        "{}",
        r#"{"outline":{},"openingAnswer":{},"extra":true}"#,
        r#"{"outline":{},"openingAnswer":{},"outline":{}}"#,
    ] {
        assert!(split(bad).is_err());
    }
    let instructions = instructions();
    assert!(instructions.contains("first real input-to-decision/result transition"));
    assert!(instructions.contains("SAME owned investigation and original budgets"));
    assert!(instructions.contains("sceneAuthoringReference"));
    assert!(instructions.contains("Choose the representation that teaches the behavior"));
    assert!(instructions.contains("Selecting a handler does not mean its work succeeds"));
    assert!(instructions.contains("Do not make the first unit an exhaustive"));
    assert!(!instructions.contains("Finish only with the supplied answerManifestV1"));
    assert!(!instructions.contains("then investigate and publish deeper sections"));
    assert!(instructions.contains("then focus on the remaining research outline"));
    assert!(instructions.contains("A comment or default constructor does not prove later consumer behavior"));
    let base = crate::agent_context::PROGRESSIVE_BASE_INSTRUCTIONS;
    assert!(!base.contains("then investigate and publish deeper supported sections"));
    assert!(base.contains("operation's role and final schema"));
    assert!(base.contains("Respect the original source, call, proposal and time limits"));
    assert!(base.contains("Do not run commands, edit code, browse, delegate or access a live checkout"));
    assert!(instructions.contains("including requested alternate paths and failures"));
    assert!(instructions.contains("Evidence proves the whole claim"));
    assert!(instructions.contains("At most six unique sections and eight total proposals"));
}

#[test]
fn opening_has_a_durable_owner_before_any_outline_or_sections_exist() {
    let f = Fixture::new();
    let r = pending(&f);
    store::create(&f.storage, &r).unwrap();
    let result = summary(&f.storage, &store::read(&f.storage, &r.id).unwrap()).unwrap();
    assert_eq!(result.opening_run_id.as_ref(), Some(&r.planning_run_id));
    assert_eq!(result.status, Status::Planning);
    assert!(result.units.is_empty() && result.answers.is_empty());
    assert_eq!(result.question, r.question);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn opening_cannot_be_forged_into_a_legacy_record_or_another_question() {
    let f = Fixture::new();
    let original = pending(&f);
    for kind in [
        "schema",
        "contract",
        "owner",
        "question",
        "missing",
        "ready",
        "duplicate",
    ] {
        let mut r = original.clone();
        match kind {
            "schema" => r.schema_version = 1,
            "contract" => r.contract = PLAN_ONLY_CONTRACT.into(),
            "owner" => r.opening.as_mut().unwrap().run_id = Uuid::new_v4().to_string(),
            "question" => r.opening.as_mut().unwrap().question.push_str("Changed"),
            "missing" => r.opening = None,
            "ready" => r.status = Status::Ready,
            "duplicate" => {
                let mut old = record(&f);
                old.schema_version = r.schema_version;
                old.contract = r.contract.clone();
                old.opening = r.opening.clone();
                old.planning_run_id = r.planning_run_id.clone();
                old.units[0].attempts.push(r.opening.clone().unwrap());
                r = old;
            }
            _ => unreachable!(),
        }
        assert!(store::create(&f.storage, &r).is_err(), "{kind}");
    }
}

#[test]
fn inactive_opening_recovers_locally_without_replanning_or_changing_stop() {
    let f = Fixture::new();
    let r = pending(&f);
    store::create(&f.storage, &r).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    let worker = manager.hold_answer_worker(&r.planning_run_id).unwrap();
    assert!(reconcile(&f.storage, &r, &manager).is_err());
    drop(worker);
    let recovered = reconcile(&f.storage, &r, &manager).unwrap();
    assert_eq!(recovered.status, Status::Interrupted);
    assert_eq!(
        recovered
            .opening
            .as_ref()
            .unwrap()
            .summary
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::Interrupted
    );
    assert!(recovered.plan.is_none() && recovered.units.is_empty());
    store::update(&f.storage, &r.id, |current| {
        current.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    let stopped = reconcile(&f.storage, &recovered, &manager).unwrap();
    assert_eq!(stopped.status, Status::Stopped);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

fn publish(f: &Fixture, r: &Record) -> crate::evidence_ledger::sections::ActiveAnswer {
    use crate::chapter_generation::tests::{SIMPLE_PATH, SIMPLE_SOURCE};
    use crate::{
        codex_identity::CodexReviewRuntime,
        evidence_ledger::{Context, Ledger, RangeRequest, Tool},
    };
    use std::sync::atomic::AtomicBool;
    let runtime = CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 1,
    };
    let grant =
        workspace::save_retrieval_grant(&f.storage, &r.project_id, &r.snapshot_id, &runtime)
            .unwrap();
    let cancel = AtomicBool::new(false);
    let mut ledger = Ledger::new_progressive(
        &f.storage,
        &r.planning_run_id,
        &f.manifest,
        &grant,
        &runtime,
    )
    .unwrap();
    let call = ledger
        .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, &runtime, &cancel)
        .unwrap();
    let source = f
        .manifest
        .files
        .iter()
        .find(|file| file.path == SIMPLE_PATH)
        .unwrap();
    let mut delivered = Value::Null;
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
            &runtime,
            &cancel,
            |bytes| {
                delivered = serde_json::from_slice(bytes).unwrap();
                Ok(())
            },
        )
        .unwrap();
    let reference = delivered["added"][0]["referenceId"].as_str().unwrap();
    let mut answer = ledger
        .start_native_manifest_answer_sections(&r.question, &"d".repeat(64), &f.manifest, &runtime, &cancel)
        .unwrap();
    let raw = crate::evidence_ledger::sections::authoring::tests::proposal(reference, "opening", 0)
        .to_string();
    let prepared = answer
        .prepare(&ledger, &raw, &f.manifest, &runtime, &cancel)
        .unwrap();
    assert!(prepared.findings.is_empty(), "{:?}", prepared.findings);
    answer
        .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
        .unwrap();
    answer
}

#[test]
fn a_validated_scene_is_readable_and_adoptable_before_the_outline_exists() {
    let f = Fixture::new();
    let r = pending(&f);
    store::create(&f.storage, &r).unwrap();
    let _answer = publish(&f, &r);
    let before = std::fs::read(
        f.storage
            .join("repository-guides")
            .join(format!("{}.json", r.id)),
    )
    .unwrap();
    let shown = summary(&f.storage, &r).unwrap();
    assert!(shown.units.is_empty());
    assert_eq!(shown.answers.len(), 1);
    assert_eq!(shown.answers[0].question, r.question);
    assert_eq!(shown.answers[0].outcome, Outcome::Working);
    let section = shown.answers[0].sections[0].clone();
    let revision = reading::adopt(
        &f.storage,
        &section.run_id,
        &r.project_id,
        &r.snapshot_id,
        &section.generation_id,
    )
    .unwrap();
    assert_eq!(revision.revision_id, section.revision_id);
    assert_eq!(
        before,
        std::fs::read(
            f.storage
                .join("repository-guides")
                .join(format!("{}.json", r.id))
        )
        .unwrap()
    );
    let recovered = reconcile(&f.storage, &r, &codex_agent::CodexAgentManager::default()).unwrap();
    let restored = summary(&f.storage, &recovered).unwrap();
    assert_eq!(restored.answers[0].outcome, Outcome::Interrupted);
    assert_eq!(
        restored.answers[0].sections[0].revision_id,
        section.revision_id
    );
    assert!(recovered.plan.is_none());
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn a_bad_final_result_preserves_published_opening_and_stop_without_replanning() {
    let f = Fixture::new();
    let r = pending(&f);
    store::create(&f.storage, &r).unwrap();
    let mut answer = publish(&f, &r);
    assert!(
        answer
            .finish_manifest(
                r#"{"manifestVersion":"answerManifestV1","sections":[],"incompleteTopics":[]}"#,
                false
            )
            .is_err()
    );
    let saved = reading::interrupt(&f.storage, &r.planning_run_id, Outcome::Partial).unwrap();
    store::update(&f.storage, &r.id, |current| {
        current.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    retain_failure(&f.storage, &r).unwrap();
    let recovered = store::read(&f.storage, &r.id).unwrap();
    assert_eq!(recovered.status, Status::Stopped);
    assert!(recovered.plan.is_none() && recovered.active_run_id.is_none());
    assert_eq!(
        recovered
            .opening
            .as_ref()
            .unwrap()
            .summary
            .as_ref()
            .unwrap()
            .sections[0]
            .revision_id,
        saved.sections[0].revision_id
    );
    assert_eq!(
        summary(&f.storage, &recovered).unwrap().answers[0].outcome,
        Outcome::Partial
    );
}

#[test]
fn completed_opening_does_not_complete_deeper_units_and_partial_opening_cannot_be_ready() {
    let f = Fixture::new();
    let mut r = pending(&f);
    store::create(&f.storage, &r).unwrap();
    let mut answer = publish(&f, &r);
    answer.finish_manifest(&json!({"manifestVersion":"answerManifestV2","incompleteTopics":[]}).to_string(), false).unwrap();
    let result = summary(&f.storage, &r).unwrap().answers.remove(0);
    assert_eq!(result.outcome, Outcome::Complete);
    let old = record(&f);
    save_outline(&f.storage, &r, old.plan.unwrap(), Some(result)).unwrap();
    r = store::read(&f.storage, &r.id).unwrap();
    assert_eq!(r.status, Status::Building);
    assert_eq!(
        r.opening
            .as_ref()
            .unwrap()
            .summary
            .as_ref()
            .unwrap()
            .outcome,
        Outcome::Complete
    );
    assert!(!guide_complete(&r));
    for i in 0..r.units.len() {
        let complete = crate::repository_guide::tests::attempt(&r, Outcome::Complete);
        r.units[i].attempts.push(complete);
    }
    assert!(guide_complete(&r));
    r.opening
        .as_mut()
        .unwrap()
        .summary
        .as_mut()
        .unwrap()
        .outcome = Outcome::Partial;
    assert!(!guide_complete(&r));
    r.status = Status::Ready;
    assert!(store::save(&f.storage, &r, false).is_err());
}

#[test]
fn both_plan_only_contracts_keep_their_original_bytes_and_have_no_opening() {
    let f = Fixture::new();
    for contract in [PLAN_ONLY_CONTRACT, LEGACY_CONTRACT] {
        let mut r = record(&f);
        r.contract = contract.into();
        store::create(&f.storage, &r).unwrap();
        let path = f
            .storage
            .join("repository-guides")
            .join(format!("{}.json", r.id));
        let before = std::fs::read(&path).unwrap();
        assert!(
            !String::from_utf8(before.clone())
                .unwrap()
                .contains("opening")
        );
        let restored = store::read(&f.storage, &r.id).unwrap();
        assert!(restored.opening.is_none());
        assert_eq!(serde_json::to_vec(&restored).unwrap(), before);
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}

#[test]
fn failure_retention_cannot_clear_a_newer_investigation_owner() {
    let f = Fixture::new();
    let mut r = pending(&f);
    let deeper = record(&f);
    r.plan = deeper.plan;
    r.units = deeper.units;
    let mut attempt = crate::repository_guide::tests::attempt(&r, Outcome::Interrupted);
    attempt.summary = None;
    r.active_run_id = Some(attempt.run_id.clone());
    r.units[0].attempts.push(attempt);
    store::create(&f.storage, &r).unwrap();
    let location = f
        .storage
        .join("repository-guides")
        .join(format!("{}.json", r.id));
    let original = std::fs::read(&location).unwrap();
    assert!(retain_failure(&f.storage, &r).is_err());
    assert_eq!(std::fs::read(location).unwrap(), original);
}

#[test]
fn outline_publication_cannot_overwrite_a_newer_parent_stop_or_foreign_opening() {
    for case in ["revision", "stop", "owner", "foreign", "missing"] {
        let f = Fixture::new();
        let r = pending(&f);
        store::create(&f.storage, &r).unwrap();
        let _answer = publish(&f, &r);
        let mut answer = summary(&f.storage, &r).unwrap().answers.remove(0);
        let plan = record(&f).plan.unwrap();
        match case {
            "revision" => {
                store::update(&f.storage, &r.id, |_| Ok(())).unwrap();
            }
            "stop" => {
                store::update(&f.storage, &r.id, |current| {
                    current.status = Status::Stopped;
                    Ok(())
                })
                .unwrap();
            }
            "owner" => {
                store::update(&f.storage, &r.id, |current| {
                    current.active_run_id = None;
                    Ok(())
                })
                .unwrap();
            }
            "foreign" => answer.question.push_str("Changed"),
            "missing" => {}
            _ => unreachable!(),
        }
        let location = f
            .storage
            .join("repository-guides")
            .join(format!("{}.json", r.id));
        let before = std::fs::read(&location).unwrap();
        assert!(
            save_outline(
                &f.storage,
                &r,
                plan,
                if case == "missing" {
                    None
                } else {
                    Some(answer)
                }
            )
            .is_err(),
            "{case}"
        );
        assert_eq!(std::fs::read(location).unwrap(), before, "{case}");
        assert_eq!(
            summary(&f.storage, &store::read(&f.storage, &r.id).unwrap())
                .unwrap()
                .answers[0]
                .sections
                .len(),
            1
        );
    }
}
