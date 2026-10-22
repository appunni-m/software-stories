use super::*;
use crate::{chapter_generation::tests::Fixture, repository_guide::tests::record};
use serde_json::Value;

pub(in crate::repository_guide) fn pending(f: &Fixture) -> Record {
    let previous = record(f);
    let mut r = new_record(
        Start {
            id: previous.id,
            project_id: previous.project_id,
            snapshot_id: previous.snapshot_id,
            grant_id: previous.grant_id,
            question: previous.question,
            model_id: DEFAULT_MODEL_ID.into(),
            reasoning_effort: DEFAULT_REASONING_EFFORT.into(),
            consent: true,
            renderer_queue_wait_ms: None,
        },
        &f.manifest,
    )
    .unwrap();
    r.active_run_id = Some(r.planning_run_id.clone());
    r
}

fn planned(f: &Fixture) -> Record {
    let mut r = pending(f);
    let source = record(f);
    r.plan = source.plan;
    r.units = source.units;
    r.status = Status::Building;
    r.active_run_id = None;
    r
}

fn start_opening(r: &Record, f: &Fixture) -> Record {
    begin_initial_opening(&f.storage, r).unwrap()
}

#[test]
fn opening_uses_the_first_planned_slide_ranges_as_research_hints() {
    let f = Fixture::new();
    let r = planned(&f);
    let planned_ranges = r.plan.as_ref().unwrap().units[0].starting_ranges.as_slice();
    let opening_ranges = starting_ranges(&r).unwrap();
    assert_eq!(opening_ranges, planned_ranges);
    assert!(starting_ranges(&pending(&f)).is_err());
}

#[test]
fn opening_has_a_durable_owner_before_any_outline_or_sections_exist() {
    let f = Fixture::new();
    let r = pending(&f);
    store::create(&f.storage, &r).unwrap();
    let result = summary(&f.storage, &store::read(&f.storage, &r.id).unwrap()).unwrap();
    assert_eq!(
        result.opening_run_id.as_ref(),
        Some(&r.opening.as_ref().unwrap().run_id)
    );
    assert_eq!(result.status, Status::Planning);
    assert!(result.units.is_empty() && result.answers.is_empty());
    assert_eq!(result.question, r.question);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn fresh_opening_owns_a_separate_learning_question_without_losing_the_guide_question() {
    let f = Fixture::new();
    let r = pending(&f);
    let opening = r.opening.as_ref().unwrap();
    assert_ne!(
        opening.question, r.question,
        "the first transition and the entire guide must not share one answer scope"
    );
    store::create(&f.storage, &r).unwrap();
    let reopened = store::read(&f.storage, &r.id).unwrap();
    assert_eq!(reopened.question, r.question);
    assert_eq!(reopened.opening.unwrap().question, opening.question);
}

#[test]
fn opening_rejects_unsupported_records_and_changed_questions() {
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
            "contract" => r.contract = "unsupported-guide-contract".into(),
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
    assert!(
        recovered.opening.as_ref().unwrap().summary.is_none(),
        "planning recovery must not consume the opening slide before the outline exists"
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

pub(in crate::repository_guide) fn publish(
    f: &Fixture,
    r: &Record,
) -> crate::evidence_ledger::sections::ActiveAnswer {
    publish_for_question(f, r, question(r).unwrap())
}

pub(in crate::repository_guide) fn publish_for_question(
    f: &Fixture,
    r: &Record,
    learning_question: &str,
) -> crate::evidence_ledger::sections::ActiveAnswer {
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
    let run_id = r.opening.as_ref().unwrap().run_id.clone();
    let mut ledger = Ledger::new_progressive(
        &f.storage,
        &run_id,
        &f.manifest,
        &grant,
        &runtime,
        "gpt-6-luna",
        "max",
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
        .start_narration_answer_sections(
            learning_question,
            &"d".repeat(64),
            &f.manifest,
            &runtime,
            &cancel,
        )
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
fn independent_research_continues_after_slide_failures_but_not_a_changed_runtime() {
    use crate::evidence_ledger::sections::FailureCode;
    let f = Fixture::new();
    let r = planned(&f);
    let mut active = publish(&f, &r);
    active.finish_manifest(&json!({"manifestVersion":"answerManifestV2","incompleteTopics":["A later implementation remains unexplored."]}).to_string(), false).unwrap();
    let answer = reading::scoped(
        &f.storage,
        &r.opening.as_ref().unwrap().run_id,
        &r.project_id,
        &r.snapshot_id,
    )
    .unwrap();
    assert_eq!(answer.outcome, Outcome::Partial);
    assert!(!answer.has_recorded_failure());
    assert!(check_research_outcome(&r, &answer).is_ok());
    for code in [
        FailureCode::InvalidToolArguments,
        FailureCode::ToolEvent,
        FailureCode::ContextTimeout,
        FailureCode::ContextPeerClosed,
        FailureCode::ContextTransport,
        FailureCode::ContextDelivery,
        FailureCode::RunTimeout,
        FailureCode::RuntimeIdentity,
        FailureCode::Unknown,
    ] {
        let f = Fixture::new();
        let r = planned(&f);
        let _active = publish(&f, &r);
        reading::interrupt_with_failure(
            &f.storage,
            &r.opening.as_ref().unwrap().run_id,
            Outcome::Partial,
            Some(code.clone()),
        )
        .unwrap();
        let answer = reading::scoped(
            &f.storage,
            &r.opening.as_ref().unwrap().run_id,
            &r.project_id,
            &r.snapshot_id,
        )
        .unwrap();
        assert_eq!(answer.outcome, Outcome::Partial);
        assert!(
            !answer.sections.is_empty(),
            "useful recovery sections are preserved"
        );
        assert!(answer.has_recorded_failure());
        let must_halt = code == FailureCode::RuntimeIdentity;
        assert_eq!(
            operation_requires_halt(None, Some(&answer), &Status::Building),
            must_halt,
            "only a stop or authority/runtime change halts independent slide work"
        );
        assert_eq!(check_research_outcome(&r, &answer).is_err(), must_halt);
    }
}

#[test]
fn interrupted_or_working_receipts_never_authorize_later_research_and_stop_stays_stop() {
    let f = Fixture::new();
    let r = planned(&f);
    let _active = publish(&f, &r);
    let answer = reading::scoped(
        &f.storage,
        &r.opening.as_ref().unwrap().run_id,
        &r.project_id,
        &r.snapshot_id,
    )
    .unwrap();
    assert_eq!(answer.outcome, Outcome::Working);
    assert!(check_research_outcome(&r, &answer).is_err());
    reading::interrupt(
        &f.storage,
        &r.opening.as_ref().unwrap().run_id,
        Outcome::Interrupted,
    )
    .unwrap();
    let answer = reading::scoped(
        &f.storage,
        &r.opening.as_ref().unwrap().run_id,
        &r.project_id,
        &r.snapshot_id,
    )
    .unwrap();
    assert!(check_research_outcome(&r, &answer).is_err());
    let f = Fixture::new();
    let r = planned(&f);
    let _active = publish(&f, &r);
    reading::interrupt_with_failure(
        &f.storage,
        &r.opening.as_ref().unwrap().run_id,
        Outcome::Stopped,
        Some(crate::evidence_ledger::sections::FailureCode::Stopped),
    )
    .unwrap();
    let answer = reading::scoped(
        &f.storage,
        &r.opening.as_ref().unwrap().run_id,
        &r.project_id,
        &r.snapshot_id,
    )
    .unwrap();
    assert_eq!(answer.outcome, Outcome::Stopped);
    assert!(check_research_outcome(&r, &answer).is_ok());
}

#[test]
fn a_validated_scene_is_readable_and_adoptable_before_the_unit_jobs_finish() {
    let f = Fixture::new();
    let r = planned(&f);
    store::create(&f.storage, &r).unwrap();
    let active = start_opening(&r, &f);
    let _answer = publish(&f, &active);
    let before = std::fs::read(
        f.storage
            .join("repository-guides")
            .join(format!("{}.json", r.id)),
    )
    .unwrap();
    let shown = summary(&f.storage, &r).unwrap();
    assert_eq!(shown.units.len(), r.units.len());
    assert_eq!(shown.answers.len(), 1);
    assert_eq!(shown.answers[0].question, question(&r).unwrap());
    assert_eq!(shown.question, r.question);
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
    let recovered = reconcile(
        &f.storage,
        &active,
        &codex_agent::CodexAgentManager::default(),
    )
    .unwrap();
    let restored = summary(&f.storage, &recovered).unwrap();
    assert_eq!(restored.answers[0].outcome, Outcome::Interrupted);
    assert_eq!(
        restored.answers[0].sections[0].revision_id,
        section.revision_id
    );
    assert!(recovered.plan.is_some());
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn a_bad_final_result_preserves_published_opening_and_stop_without_replanning() {
    let f = Fixture::new();
    let r = planned(&f);
    store::create(&f.storage, &r).unwrap();
    let active = start_opening(&r, &f);
    let mut answer = publish(&f, &active);
    assert!(
        answer
            .finish_manifest(
                r#"{"manifestVersion":"answerManifestV2","sections":[],"incompleteTopics":[]}"#,
                false
            )
            .is_err()
    );
    let saved = reading::interrupt(
        &f.storage,
        &active.opening.as_ref().unwrap().run_id,
        Outcome::Partial,
    )
    .unwrap();
    store::update(&f.storage, &r.id, |current| {
        current.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    retain_failure(&f.storage, &active).unwrap();
    let recovered = store::read(&f.storage, &r.id).unwrap();
    assert_eq!(recovered.status, Status::Stopped);
    assert!(recovered.plan.is_some() && recovered.active_run_id.is_none());
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
    let mut r = planned(&f);
    store::create(&f.storage, &r).unwrap();
    let active = start_opening(&r, &f);
    let mut answer = publish(&f, &active);
    answer
        .finish_manifest(
            &json!({"manifestVersion":"answerManifestV2","incompleteTopics":[]}).to_string(),
            false,
        )
        .unwrap();
    let result = reading::scoped(
        &f.storage,
        &active.opening.as_ref().unwrap().run_id,
        &active.project_id,
        &active.snapshot_id,
    )
    .unwrap()
    .summary();
    assert_eq!(result.outcome, Outcome::Complete);
    r = finish_initial_opening(&f.storage, &active, result).unwrap();
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
        let r = planned(&f);
        store::create(&f.storage, &r).unwrap();
        let active = start_opening(&r, &f);
        let _answer = publish(&f, &active);
        let mut answer = summary(&f.storage, &active).unwrap().answers.remove(0);
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
