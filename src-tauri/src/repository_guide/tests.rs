use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};

mod capacity;

pub(super) fn record(f: &Fixture) -> Record {
    let source = context::source(&f.storage, &f.manifest, SIMPLE_PATH).unwrap();
    let range = context::range_from_source(&source, 0, SIMPLE_SOURCE.len())
        .unwrap()
        .range;
    let mut result = Record {
        revision: 0,
        schema_version: RECORD_SCHEMA_VERSION,
        contract: CONTRACT.into(),
        id: Uuid::new_v4().to_string(),
        project_id: f.manifest.summary.project_id.clone(),
        snapshot_id: f.manifest.summary.snapshot_id.clone(),
        manifest_sha256: context::digest(&serde_json::to_vec(&f.manifest).unwrap()),
        grant_id: Uuid::new_v4().to_string(),
        question: "How does this project work?".into(),
        model_id: DEFAULT_MODEL_ID.into(),
        reasoning_effort: DEFAULT_REASONING_EFFORT.into(),
        planning_run_id: Uuid::new_v4().to_string(),
        planning_retry_count: 0,
        opening: None,
        opening_context_sha256: None,
        opening_continuations: vec![],
        consumed_slide_revisions: vec![],
        plan: Some(Plan {
            title: "A grounded repository guide".into(),
            opening_ranges: vec![range.clone()],
            units: vec![
                plan::UnitPlan {
                    key: "input".into(),
                    question: "How does the name enter the greeting?".into(),
                    prerequisites: vec![],
                    starting_ranges: vec![range.clone()],
                    continuation_topic: None,
                },
                plan::UnitPlan {
                    key: "output".into(),
                    question: "How does the greeting become output?".into(),
                    prerequisites: vec!["input".into()],
                    starting_ranges: vec![range],
                    continuation_topic: None,
                },
            ],
            omissions: vec!["No runtime execution was captured.".into()],
        }),
        units: vec![
            Unit {
                key: "input".into(),
                attempts: vec![],
            },
            Unit {
                key: "output".into(),
                attempts: vec![],
            },
        ],
        status: Status::Building,
        active_run_id: None,
    };
    result.opening = Some(Attempt {
        run_id: result.planning_run_id.clone(),
        question: opening::LEARNING_QUESTION.into(),
        automatic_retry: false,
        summary: Some(Summary {
            run_id: result.planning_run_id.clone(),
            project_id: result.project_id.clone(),
            snapshot_id: result.snapshot_id.clone(),
            question: opening::LEARNING_QUESTION.into(),
            outcome: Outcome::Interrupted,
            failure_code: None,
            sections: vec![],
            incomplete_topics: vec!["Synthetic terminal opening for unit-state tests.".into()],
        }),
    });
    result.opening_context_sha256 = Some(opening::context_sha256(&result).unwrap());
    result
}
pub(super) fn attempt(record: &Record, outcome: Outcome) -> Attempt {
    let run_id = Uuid::new_v4().to_string();
    let question = "How does the name enter the greeting?".to_owned();
    // Synthetic identity metadata for state-machine tests, never a published
    // or accepted chapter. Real results are independently admitted by Codex.
    let sections = if outcome == Outcome::Complete {
        vec![SectionNotice {
            run_id: run_id.clone(),
            project_id: record.project_id.clone(),
            snapshot_id: record.snapshot_id.clone(),
            key: "input".into(),
            order: 0,
            generation_id: format!("generation:{}", Uuid::new_v4()),
            revision_id: format!("revision:{}", Uuid::new_v4()),
            supersedes: None,
            context_elapsed_ms: 1,
            run_elapsed_ms: Some(2),
        }]
    } else {
        vec![]
    };
    Attempt {
        run_id: run_id.clone(),
        question: question.clone(),
        automatic_retry: false,
        summary: Some(Summary {
            run_id,
            question,
            project_id: record.project_id.clone(),
            snapshot_id: record.snapshot_id.clone(),
            outcome,
            failure_code: None,
            sections,
            incomplete_topics: vec![],
        }),
    }
}
pub(super) fn fresh_record(f: &Fixture) -> Record {
    let previous = record(f);
    let mut fresh = new_record(
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
    fresh.plan = previous.plan;
    fresh.units = previous.units;
    fresh.status = Status::Building;
    fresh
}

#[test]
fn playback_receipt_is_durable_while_a_guide_worker_owns_the_runway() {
    let f = Fixture::new();
    let mut record = record(&f);
    let completed = attempt(&record, Outcome::Complete);
    let revision_id = completed.summary.as_ref().unwrap().sections[0]
        .revision_id
        .clone();
    record.units[0].attempts.push(completed);
    store::create(&f.storage, &record).unwrap();
    let _owner = store::lease(&f.storage, &record.id).unwrap().unwrap();

    let saved = record_consumed_slide(&f.storage, &record.id, &revision_id).unwrap();
    assert!(saved.consumed_slide_revisions.contains(&revision_id));
    assert!(store::lease(&f.storage, &record.id).unwrap().is_none());

    let revision = saved.revision;
    let repeated = record_consumed_slide(&f.storage, &record.id, &revision_id).unwrap();
    assert_eq!(repeated.revision, revision, "a replayed receipt is a no-op");
    assert_eq!(repeated.consumed_slide_revisions.len(), 1);
    assert_eq!(slide_run_limit(5, 0, &repeated, true), 6);
    assert_eq!(slide_run_limit(5, 0, &repeated, false), 5);
}

#[tokio::test]
async fn terminal_playback_receipt_waits_for_the_owner_lease_then_claims_refill() {
    let f = Fixture::new();
    let mut record = record(&f);
    let completed = attempt(&record, Outcome::Complete);
    let revision_id = completed.summary.as_ref().unwrap().sections[0]
        .revision_id
        .clone();
    record.units[0].attempts.push(completed);
    record.status = Status::WaitingForPlayback;
    store::create(&f.storage, &record).unwrap();

    // Model the finalization/lease-drop interval: the guide is terminal, but
    // its native worker still owns the lease when the reader records playback.
    let owner = store::lease(&f.storage, &record.id).unwrap().unwrap();
    let receipt = record_consumed_slide(&f.storage, &record.id, &revision_id).unwrap();
    assert_eq!(receipt.status, Status::WaitingForPlayback);
    assert!(guide_buffer_gap(&receipt) > 0);

    let storage = f.storage.clone();
    let id = record.id.clone();
    let handoff = tokio::spawn(async move {
        wait_for_terminal_guide_handoff(&storage, &id, &receipt).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(45)).await;
    assert!(
        !handoff.is_finished(),
        "a second worker cannot start before the original lease is released"
    );
    drop(owner);

    let refill = handoff
        .await
        .unwrap()
        .unwrap()
        .expect("the terminal guide should hand off its refill lease");
    assert!(
        store::lease(&f.storage, &record.id).unwrap().is_none(),
        "the refill worker must own the exclusive lease"
    );
    drop(refill);
    assert!(store::lease(&f.storage, &record.id).unwrap().is_some());
}

#[test]
fn playback_saved_after_the_unit_boundary_extends_runway_during_finalization() {
    let f = Fixture::new();
    let mut record = record(&f);
    let completed = attempt(&record, Outcome::Complete);
    let revision_id = completed.summary.as_ref().unwrap().sections[0]
        .revision_id
        .clone();
    record.units[0].attempts.push(completed);
    store::create(&f.storage, &record).unwrap();

    // The reader finishes this slide after the worker's last ordinary unit
    // boundary, but before its transactional terminal-state decision.
    let receipt = record_consumed_slide(&f.storage, &record.id, &revision_id).unwrap();
    let mut keep_building = false;
    let saved = store::update_if_changed(&f.storage, &record.id, |current| {
        let finalized = finalize_slide_run(current, None, 1, 0, 0, 1, true);
        keep_building = !finalized;
        Ok(finalized)
    })
    .unwrap();

    assert!(keep_building, "the newly heard slide extends the runway");
    assert_eq!(saved.status, Status::Building);
    assert_eq!(saved.revision, receipt.revision);
    assert!(saved.active_run_id.is_none());

    let mut without_receipt = receipt;
    without_receipt.consumed_slide_revisions.clear();
    assert!(finalize_slide_run(&mut without_receipt, None, 1, 0, 0, 1, true));
    assert_eq!(without_receipt.status, Status::WaitingForPlayback);
}

#[test]
fn one_slide_retry_requires_a_terminal_failure_and_a_free_runway_slot() {
    let f = Fixture::new();
    let mut record = record(&f);
    record.status = Status::Partial;
    let first_failure = attempt(&record, Outcome::Partial);
    record.units[0].attempts.push(first_failure);

    assert!(unit_retryable(&record, 0));
    assert!(!unit_retryable(&record, 1), "a pending unit is not a retry");

    let mut active = record.clone();
    active.active_run_id = Some(Uuid::new_v4().to_string());
    assert!(
        !unit_retryable(&active, 0),
        "a retry cannot overlap its owner"
    );

    let mut stopped = record.clone();
    stopped.status = Status::Stopped;
    assert!(!unit_retryable(&stopped, 0), "Stop remains final");

    let mut exhausted = record.clone();
    for _ in 1..MAX_ATTEMPTS {
        let extra_failure = attempt(&exhausted, Outcome::Partial);
        exhausted.units[0].attempts.push(extra_failure);
    }
    assert!(
        !unit_retryable(&exhausted, 0),
        "the attempt budget is retained"
    );

    let mut full_runway = record;
    for _ in 0..GUIDE_SLIDE_BUFFER {
        let completed_slide = attempt(&full_runway, Outcome::Complete);
        full_runway.opening_continuations.push(completed_slide);
    }
    assert!(!unit_retryable(&full_runway, 0));
    let consumed_revision = full_runway.opening_continuations[0]
        .summary
        .as_ref()
        .unwrap()
        .sections[0]
        .revision_id
        .clone();
    full_runway.consumed_slide_revisions.push(consumed_revision);
    assert!(
        unit_retryable(&full_runway, 0),
        "a consumed slide frees one slot"
    );
}

#[test]
fn explicit_slide_retry_appends_a_fresh_attempt_without_replaying_other_work() {
    let f = Fixture::new();
    let mut record = record(&f);
    record.status = Status::Partial;
    let question = record.plan.as_ref().unwrap().units[0].question.clone();
    let mut failed = attempt(&record, Outcome::Partial);
    failed.question = question.clone();
    failed.summary.as_mut().unwrap().question = question.clone();
    record.units[0].attempts.push(failed);

    let failed_attempt = serde_json::to_value(&record.units[0].attempts[0]).unwrap();
    let untouched_unit = serde_json::to_value(&record.units[1]).unwrap();
    let saved_plan = serde_json::to_value(&record.plan).unwrap();
    let saved_opening = serde_json::to_value(&record.opening).unwrap();
    store::create(&f.storage, &record).unwrap();
    let current = store::read(&f.storage, &record.id).unwrap();
    let retry_run_id = Uuid::new_v4().to_string();

    let retried = begin_unit(&f.storage, &current, 0, &retry_run_id, &question).unwrap();

    assert_eq!(
        retried.active_run_id.as_deref(),
        Some(retry_run_id.as_str())
    );
    assert_eq!(retried.units[0].attempts.len(), 2);
    assert_eq!(
        serde_json::to_value(&retried.units[0].attempts[0]).unwrap(),
        failed_attempt
    );
    assert_eq!(retried.units[0].attempts[1].run_id, retry_run_id);
    assert!(retried.units[0].attempts[1].summary.is_none());
    assert_eq!(
        serde_json::to_value(&retried.units[1]).unwrap(),
        untouched_unit
    );
    assert_eq!(serde_json::to_value(&retried.plan).unwrap(), saved_plan);
    assert_eq!(
        serde_json::to_value(&retried.opening).unwrap(),
        saved_opening
    );
}

#[test]
fn uncovered_topics_extend_the_same_saved_plan_without_replaying_completed_units() {
    let f = Fixture::new();
    let mut record = fresh_record(&f);
    let opening_run_id = record.opening.as_ref().unwrap().run_id.clone();
    let opening_question = opening::LEARNING_QUESTION.to_owned();
    let mut opening = attempt(&record, Outcome::Complete);
    let opening_owner = opening_run_id.clone();
    opening.run_id = opening_run_id.clone();
    opening.question = opening_question.clone();
    let summary = opening.summary.as_mut().unwrap();
    summary.run_id = opening_run_id;
    summary.question = opening_question;
    summary.sections[0].run_id = opening_owner;
    record.opening = Some(opening);
    record.opening_context_sha256 = Some(opening::context_sha256(&record).unwrap());
    let completed_units = record
        .plan
        .as_ref()
        .unwrap()
        .units
        .iter()
        .map(|unit| {
            let mut completed = attempt(&record, Outcome::Complete);
            completed.question = unit.question.clone();
            completed.summary.as_mut().unwrap().question = unit.question.clone();
            completed
        })
        .collect::<Vec<_>>();
    for (unit, completed) in record.units.iter_mut().zip(completed_units) {
        unit.attempts.push(completed);
    }
    let omitted_topic_count = MAX_TOTAL_UNITS.saturating_sub(record.units.len());
    record.plan.as_mut().unwrap().omissions =
        std::iter::once("No runtime execution was captured.".to_owned())
            .chain(
                (1..omitted_topic_count)
                    .map(|index| format!("Additional public route {index} remains unexplored.")),
            )
            .collect();
    record.status = Status::Ready;
    assert!(plan_extension_eligible(&record));
    assert!(guide_summary(&record, &f.manifest, Vec::new()).can_extend);
    store::create(&f.storage, &record).unwrap();

    let resumed = store::prepare_continuation(&f.storage, &record.id, record.revision).unwrap();
    assert_eq!(resumed.id, record.id);
    assert_eq!(resumed.plan, record.plan);
    assert_eq!(
        resumed.units[..2]
            .iter()
            .map(|unit| unit.attempts[0].run_id.as_str())
            .collect::<Vec<_>>(),
        record.units[..2]
            .iter()
            .map(|unit| unit.attempts[0].run_id.as_str())
            .collect::<Vec<_>>()
    );

    let extended = append_omission_units(&f.storage, &resumed).unwrap();

    let plan = extended.plan.as_ref().unwrap();
    assert_eq!(plan.units.len(), MAX_TOTAL_UNITS);
    assert_eq!(extended.units.len(), MAX_TOTAL_UNITS);
    assert_eq!(plan.omissions, Vec::<String>::new());
    assert!(
        plan.units[..2]
            .iter()
            .all(|unit| unit.continuation_topic.is_none())
    );
    assert!(extended.units[..2].iter().all(unit_complete));
    assert_eq!(
        plan.units[2].continuation_topic.as_deref(),
        Some("No runtime execution was captured.")
    );
    assert!(
        plan.units[2..]
            .iter()
            .all(|unit| { unit.continuation_topic.is_some() && unit.starting_ranges.is_empty() })
    );
    assert!(
        extended.units[2..]
            .iter()
            .all(|unit| unit.attempts.is_empty())
    );
    assert!(!plan_extension_eligible(&extended));
    assert!(!guide_summary(&extended, &f.manifest, Vec::new()).can_extend);
    let reopened = store::read(&f.storage, &record.id).unwrap();
    assert_eq!(reopened.plan.as_ref(), Some(plan));
    for index in 0..2 {
        assert_eq!(
            reopened.units[index].attempts[0].run_id,
            extended.units[index].attempts[0].run_id
        );
    }
}

#[test]
fn an_exhausted_outline_with_unresolved_omissions_finishes_partial() {
    let f = Fixture::new();
    let mut record = fresh_record(&f);
    let plan_unit = record.plan.as_ref().unwrap().units[0].clone();
    let units = (0..MAX_TOTAL_UNITS)
        .map(|index| plan::UnitPlan {
            key: format!("unit-{index}"),
            question: format!("How does unit {index} work?"),
            prerequisites: Vec::new(),
            starting_ranges: plan_unit.starting_ranges.clone(),
            continuation_topic: None,
        })
        .collect::<Vec<_>>();
    let completed_attempts = units
        .iter()
        .map(|unit| {
            let mut attempt = attempt(&record, Outcome::Complete);
            attempt.question = unit.question.clone();
            attempt.summary.as_mut().unwrap().question = unit.question.clone();
            attempt
        })
        .collect::<Vec<_>>();
    record.plan.as_mut().unwrap().units = units.clone();
    record.units = units
        .into_iter()
        .zip(completed_attempts)
        .map(|(unit, attempt)| Unit {
            key: unit.key,
            attempts: vec![attempt],
        })
        .collect();
    let complete_opening = attempt(&record, Outcome::Complete);
    record.opening.as_mut().unwrap().summary = complete_opening.summary;
    record.plan.as_mut().unwrap().omissions = vec!["One public route remains unexplored.".into()];

    assert!(guide_complete(&record));
    assert!(pending_extension_topics(record.plan.as_ref().unwrap()).is_empty());
    assert!(!plan_extension_eligible(&record));
    assert_eq!(terminal_guide_status(&record), Status::Partial);

    record.plan.as_mut().unwrap().omissions.clear();
    assert_eq!(terminal_guide_status(&record), Status::Ready);
}

#[test]
fn renderer_queue_wait_accepts_bounded_values_and_ignores_invalid_diagnostics() {
    assert_eq!(bounded_renderer_queue_wait_ms(None), None);
    assert_eq!(bounded_renderer_queue_wait_ms(Some(0)), Some(0));
    assert_eq!(
        bounded_renderer_queue_wait_ms(Some(crate::generation_timing::MAX_ELAPSED)),
        Some(crate::generation_timing::MAX_ELAPSED)
    );
    assert_eq!(
        bounded_renderer_queue_wait_ms(Some(crate::generation_timing::MAX_ELAPSED + 1)),
        None
    );
}

#[test]
fn fresh_guide_researches_later_units_after_a_partial_reading_prerequisite() {
    let f = Fixture::new();
    let mut r = fresh_record(&f);
    let partial = attempt(&r, Outcome::Partial);
    r.units[0].attempts.push(partial);
    assert!(
        unit_research_eligible(&r, 1),
        "a reading prerequisite is not permission to research captured code independently"
    );
    assert!(!guide_complete(&r));
    assert_eq!(r.units[0].attempts.len(), 1);
    assert!(r.units[1].attempts.is_empty());
}

#[test]
fn published_guide_slides_are_counted_by_stable_revision_identity() {
    let f = Fixture::new();
    let mut record = fresh_record(&f);
    let first = attempt(&record, Outcome::Complete);
    let revision_id = first.summary.as_ref().unwrap().sections[0]
        .revision_id
        .clone();
    record.units[0].attempts.push(first.clone());
    record.units[1].attempts.push(first);
    assert_eq!(published_slide_count(&record), 1);
    assert!(contains_slide(&record, &revision_id));
    assert!(!contains_slide(&record, "revision:not-published"));
}

#[test]
fn selected_account_model_is_persisted_and_bound_to_the_guide_contract() {
    let f = Fixture::new();
    let previous = record(&f);
    let selected = new_record(
        Start {
            id: Uuid::new_v4().to_string(),
            project_id: previous.project_id.clone(),
            snapshot_id: previous.snapshot_id.clone(),
            grant_id: previous.grant_id.clone(),
            question: previous.question.clone(),
            model_id: "gpt-5.6-luna".into(),
            reasoning_effort: "medium".into(),
            consent: true,
            renderer_queue_wait_ms: None,
        },
        &f.manifest,
    )
    .unwrap();

    assert_eq!(selected.model_id, "gpt-5.6-luna");
    assert_eq!(selected.reasoning_effort, "medium");
    let mut default_settings = selected.clone();
    default_settings.model_id = DEFAULT_MODEL_ID.into();
    default_settings.reasoning_effort = DEFAULT_REASONING_EFFORT.into();
    default_settings.opening_context_sha256 =
        Some(opening::context_sha256(&default_settings).unwrap());
    let (_, _, default_fingerprint, _) = plan::request(&default_settings).unwrap();
    let (_, _, selected_fingerprint, _) = plan::request(&selected).unwrap();
    assert_ne!(default_fingerprint, selected_fingerprint);
}

#[test]
fn selected_model_must_match_the_live_codex_catalog_and_effort() {
    let models = [codex_agent::CodexModel {
        model_id: "gpt-5.6-luna".into(),
        display_name: "GPT-5.6 Luna".into(),
        default_reasoning_effort: Some("medium".into()),
        supported_reasoning_efforts: vec![
            "low".into(),
            "medium".into(),
            "high".into(),
            "max".into(),
        ],
        is_default: false,
    }];

    assert!(validate_model_selection(&models, "gpt-5.6-luna", "medium").is_ok());
    assert!(validate_model_selection(&models, "gpt-5.6-luna", "ultra").is_err());
    assert!(validate_model_selection(&models, "gpt-6-luna", "max").is_err());
}

#[test]
fn independent_research_preserves_outline_order_attempt_limits_and_reading_edges() {
    let f = Fixture::new();
    let mut r = fresh_record(&f);
    let range = r.plan.as_ref().unwrap().units[0].starting_ranges[0].clone();
    r.plan.as_mut().unwrap().units = (0..MAX_INITIAL_UNITS)
        .map(|i| plan::UnitPlan {
            key: format!("unit-{i}"),
            question: format!("How does behavior number {i} transform its input?"),
            prerequisites: if i == 0 {
                vec![]
            } else {
                vec![format!("unit-{}", i - 1)]
            },
            starting_ranges: vec![range.clone()],
            continuation_topic: None,
        })
        .collect();
    r.units = r
        .plan
        .as_ref()
        .unwrap()
        .units
        .iter()
        .map(|u| Unit {
            key: u.key.clone(),
            attempts: vec![],
        })
        .collect();
    let outline = serde_json::to_vec(&r.plan).unwrap();
    let mut visited = vec![];
    // This is the production loop's bounded, ordered eligibility decision.
    for index in 0..r.units.len() {
        assert!(unit_research_eligible(&r, index));
        visited.push(r.units[index].key.clone());
        let intent = unit_intent(&r, index).unwrap();
        assert_eq!(intent.unit_number, index + 2);
        assert_eq!(intent.unit_count, MAX_INITIAL_UNITS + 1);
        let question = continuation(&f.storage, &r, index).unwrap();
        assert!(question.starts_with(&r.plan.as_ref().unwrap().units[index].question));
        assert!(question.contains(RESEARCH_INSTRUCTIONS));
        assert!(!question.contains(SIMPLE_SOURCE));
        let partial = attempt(&r, Outcome::Partial);
        r.units[index].attempts.push(partial);
    }
    assert_eq!(
        visited,
        (0..MAX_INITIAL_UNITS)
            .map(|i| format!("unit-{i}"))
            .collect::<Vec<_>>()
    );
    assert!(r.units.iter().all(|u| u.attempts.len() == 1));
    assert_eq!(serde_json::to_vec(&r.plan).unwrap(), outline);
    assert!(!guide_complete(&r));
    let second = attempt(&r, Outcome::Partial);
    r.units[0].attempts.push(second);
    assert!(unit_research_eligible(&r, 0));
    let third = attempt(&r, Outcome::Partial);
    r.units[0].attempts.push(third);
    assert!(!unit_research_eligible(&r, 0));
    let complete = attempt(&r, Outcome::Complete);
    r.units[1].attempts.push(complete);
    assert!(!unit_research_eligible(&r, 1));
    r.status = Status::Stopped;
    assert!((0..r.units.len()).all(|i| !unit_research_eligible(&r, i)));
    assert_eq!(MAX_INITIAL_UNITS + 1, 6);
    assert_eq!(MAX_ATTEMPTS, 3);
}

#[test]
fn research_admission_preserves_stop_owner_revision_and_spent_attempts() {
    let f = Fixture::new();
    let r = fresh_record(&f);
    store::create(&f.storage, &r).unwrap();
    let question = continuation(&f.storage, &r, 1).unwrap();
    let run_id = Uuid::new_v4().to_string();
    let begun = begin_unit(&f.storage, &r, 1, &run_id, &question).unwrap();
    assert_eq!(begun.units[1].attempts.len(), 1);
    assert_eq!(begun.active_run_id.as_deref(), Some(run_id.as_str()));
    let before = serde_json::to_vec(&begun).unwrap();
    assert!(begin_unit(&f.storage, &r, 1, &Uuid::new_v4().to_string(), &question).is_err());
    assert!(
        begin_unit(
            &f.storage,
            &begun,
            1,
            &Uuid::new_v4().to_string(),
            &question
        )
        .is_err()
    );
    assert_eq!(
        serde_json::to_vec(&store::read(&f.storage, &r.id).unwrap()).unwrap(),
        before
    );
    let stopped = store::update(&f.storage, &r.id, |current| {
        current.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    assert!(
        begin_unit(
            &f.storage,
            &begun,
            0,
            &Uuid::new_v4().to_string(),
            &question
        )
        .is_err()
    );
    let after = store::read(&f.storage, &r.id).unwrap();
    assert_eq!(after.status, Status::Stopped);
    assert_eq!(after.revision, stopped.revision);
    assert!(after.units[0].attempts.is_empty());
    assert_eq!(after.units[1].attempts.len(), 1);

    let mut exhausted = fresh_record(&f);
    for _ in 0..MAX_ATTEMPTS {
        let partial = attempt(&exhausted, Outcome::Partial);
        exhausted.units[0].attempts.push(partial);
    }
    store::create(&f.storage, &exhausted).unwrap();
    assert!(
        begin_unit(
            &f.storage,
            &exhausted,
            0,
            &Uuid::new_v4().to_string(),
            &question
        )
        .is_err()
    );
    assert_eq!(
        store::read(&f.storage, &exhausted.id).unwrap().units[0]
            .attempts
            .len(),
        MAX_ATTEMPTS
    );
}

#[test]
fn a_slide_gets_one_automatic_recovery_and_keeps_one_explicit_continuation() {
    let f = Fixture::new();
    let record = fresh_record(&f);
    store::create(&f.storage, &record).unwrap();

    let first_question = continuation(&f.storage, &record, 0).unwrap();
    let first_run = Uuid::new_v4().to_string();
    let first = begin_unit(&f.storage, &record, 0, &first_run, &first_question).unwrap();
    let failed = store::update(&f.storage, &record.id, |current| {
        let project_id = current.project_id.clone();
        let snapshot_id = current.snapshot_id.clone();
        let attempt = current.units[0].attempts.last_mut().unwrap();
        attempt.summary = Some(Summary {
            run_id: attempt.run_id.clone(),
            project_id,
            snapshot_id,
            question: attempt.question.clone(),
            outcome: Outcome::Failed,
            failure_code: None,
            sections: vec![],
            incomplete_topics: vec!["The first bounded attempt failed.".into()],
        });
        current.active_run_id = None;
        current.status = Status::Partial;
        Ok(())
    })
    .unwrap();
    assert_eq!(first.units[0].attempts.len(), 1);

    let recovery_question = continuation(&f.storage, &failed, 0).unwrap();
    let recovery_run = Uuid::new_v4().to_string();
    let recovered = begin_unit_with_retry(
        &f.storage,
        &failed,
        0,
        &recovery_run,
        &recovery_question,
        true,
    )
    .unwrap();
    assert!(recovered.units[0].attempts[1].automatic_retry);
    assert_eq!(recovered.units[0].attempts.len(), 2);

    let failed_again = store::update(&f.storage, &record.id, |current| {
        let project_id = current.project_id.clone();
        let snapshot_id = current.snapshot_id.clone();
        let attempt = current.units[0].attempts.last_mut().unwrap();
        attempt.summary = Some(Summary {
            run_id: attempt.run_id.clone(),
            project_id,
            snapshot_id,
            question: attempt.question.clone(),
            outcome: Outcome::Failed,
            failure_code: None,
            sections: vec![],
            incomplete_topics: vec!["The automatic recovery also failed.".into()],
        });
        current.active_run_id = None;
        current.status = Status::Partial;
        Ok(())
    })
    .unwrap();
    let explicit_question = continuation(&f.storage, &failed_again, 0).unwrap();
    let explicit_run = Uuid::new_v4().to_string();
    let explicit = begin_unit(
        &f.storage,
        &failed_again,
        0,
        &explicit_run,
        &explicit_question,
    )
    .unwrap();
    assert!(explicit.units[0].attempts[1].automatic_retry);
    assert!(!explicit.units[0].attempts[2].automatic_retry);
    assert_eq!(explicit.units[0].attempts.len(), 3);
    assert!(
        begin_unit(
            &f.storage,
            &explicit,
            0,
            &Uuid::new_v4().to_string(),
            &explicit_question,
        )
        .is_err()
    );
}

#[test]
fn reading_prerequisites_do_not_block_research_and_completed_units_are_skipped() {
    let f = Fixture::new();
    let mut record = record(&f);
    assert!(unit_research_eligible(&record, 0));
    assert!(unit_research_eligible(&record, 1));
    let next = attempt(&record, Outcome::Partial);
    record.units[0].attempts.push(next);
    assert!(!unit_complete(&record.units[0]));
    assert!(unit_research_eligible(&record, 1));
    let next = attempt(&record, Outcome::Complete);
    record.units[0].attempts.push(next);
    assert!(unit_complete(&record.units[0]));
    assert!(unit_research_eligible(&record, 1));
    assert!(!unit_research_eligible(&record, 0));
    assert_eq!(MAX_INITIAL_UNITS, 5);
    assert_eq!(MAX_ATTEMPTS, 3);
}

#[test]
fn each_unit_receives_the_exact_native_guide_intent_without_changing_its_question() {
    let f = Fixture::new();
    let mut record = record(&f);
    record.question = "How does café 🦀 turn a name into output, including failures?".into();
    record.plan.as_mut().unwrap().omissions = vec![
        "No runtime execution was captured.".into(),
        "Detailed encoding diagnostics remain unexplored.".into(),
    ];
    for index in 0..2 {
        let intent = unit_intent(&record, index).unwrap();
        let value = serde_json::to_value(intent).unwrap();
        assert_eq!(value["guideId"], record.id);
        assert_eq!(value["question"], record.question);
        assert_eq!(
            value["researchBoundaries"],
            serde_json::json!(record.plan.as_ref().unwrap().omissions)
        );
        assert_eq!(value["unitKey"], record.units[index].key);
        assert_eq!(value["unitNumber"], index + 2);
        assert_eq!(value["unitCount"], 3);
        let question = continuation(&f.storage, &record, index).unwrap();
        assert!(question.starts_with(&record.plan.as_ref().unwrap().units[index].question));
        assert!(question.contains(&format!(
            "This is unit {} of a repository guide.",
            index + 2
        )));
    }
    assert!(unit_intent(&record, 2).is_err());
    record.plan = None;
    assert!(unit_intent(&record, 0).is_err());
}
#[test]
fn guide_reopens_without_research_and_source_scope_is_revalidated() {
    let f = Fixture::new();
    let record = record(&f);
    store::save(&f.storage, &record, true).unwrap();
    let opened = scoped(
        &f.storage,
        &record.id,
        &record.project_id,
        &record.snapshot_id,
    )
    .unwrap();
    let view = summary(&f.storage, &opened).unwrap();
    assert_eq!(view.answers.len(), 1);
    assert_eq!(view.answers[0].run_id, record.planning_run_id);
    assert!(view.answers[0].sections.is_empty());
    assert_eq!(view.units.len(), 2);
    assert_eq!(view.starting_files, 1);
    assert!(view.captured_files >= 1);
    assert!(scoped(&f.storage, &record.id, "other", &record.snapshot_id).is_err());
    let mut changed = record.clone();
    changed.manifest_sha256 = "f".repeat(64);
    assert!(checked_manifest(&f.storage, &changed).is_err());
    changed = record.clone();
    changed.plan.as_mut().unwrap().units[0].starting_ranges[0].start_byte += 1;
    assert!(checked_manifest(&f.storage, &changed).is_err());
    changed = record.clone();
    changed.plan.as_mut().unwrap().opening_ranges[0].start_byte += 1;
    assert!(checked_manifest(&f.storage, &changed).is_err());
}

#[test]
fn saved_guide_listing_recovers_interruption_without_dispatch_or_replacing_the_plan() {
    let f = Fixture::new();
    let mut original = record(&f);
    original.active_run_id = Some(original.planning_run_id.clone());
    store::create(&f.storage, &original).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    let result = list_saved_guides(
        &f.storage,
        &original.project_id,
        &original.snapshot_id,
        &manager,
    )
    .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].status, Status::Interrupted);
    let saved = store::read(&f.storage, &original.id).unwrap();
    assert_eq!(saved.plan, original.plan);
    assert!(saved.active_run_id.is_none());
    assert!(saved.units.iter().all(|unit| unit.attempts.is_empty()));
    let revision = saved.revision;
    list_saved_guides(
        &f.storage,
        &original.project_id,
        &original.snapshot_id,
        &manager,
    )
    .unwrap();
    assert_eq!(
        store::read(&f.storage, &original.id).unwrap().revision,
        revision
    );
    assert!(!f.storage.join("evidence-ledgers").exists());
}

#[test]
fn an_unsupported_guide_in_another_scope_does_not_block_selected_guide_listing() {
    let f = Fixture::new();
    let selected = record(&f);
    store::create(&f.storage, &selected).unwrap();

    let mut unrelated = record(&f);
    unrelated.id = Uuid::new_v4().to_string();
    unrelated.project_id = "/another/repository".into();
    unrelated.snapshot_id = "another-snapshot".into();
    unrelated.schema_version = 6;
    unrelated.contract = "repository-guide-v6-bounded-opening-continuation".into();
    let bytes = serde_json::to_vec(&unrelated).unwrap();
    let location = f
        .storage
        .join("repository-guides")
        .join(format!("{}.json", unrelated.id));
    std::fs::write(&location, &bytes).unwrap();

    let manager = codex_agent::CodexAgentManager::default();
    let guides = list_saved_guides(
        &f.storage,
        &selected.project_id,
        &selected.snapshot_id,
        &manager,
    )
    .expect("an unrelated unsupported guide must not break this repository's restore");
    assert_eq!(guides.len(), 1);
    assert_eq!(guides[0].id, selected.id);
    assert_eq!(std::fs::read(&location).unwrap(), bytes);

    let reservations = answer_reservations(&f.storage)
        .expect("unsupported saved guides keep their capacity reserved");
    assert_eq!(reservations.len(), 2);
    assert!(reservations
        .iter()
        .any(|reservation| reservation.guide_id == selected.id));
    let opaque = reservations
        .iter()
        .find(|reservation| reservation.guide_id == unrelated.id)
        .expect("the unsupported guide retains an opaque capacity reservation");
    assert!(opaque.run_ids.is_empty());
    assert_eq!(opaque.reserved_records, MAX_GUIDE_ANSWER_RECORDS);
    assert_eq!(std::fs::read(&location).unwrap(), bytes);

    let (unsupported, unavailable_count) = list_saved_guides_with_count(
        &f.storage,
        &unrelated.project_id,
        &unrelated.snapshot_id,
        &manager,
    )
    .expect("an unsupported saved guide is reported as unavailable");
    assert!(unsupported.is_empty());
    assert_eq!(unavailable_count, 1);
    assert_eq!(std::fs::read(&location).unwrap(), bytes);
}

#[test]
fn restart_recovers_only_metadata_and_explicit_continuation_uses_saved_progress() {
    let f = Fixture::new();
    let mut record = record(&f);
    let next = attempt(&record, Outcome::Complete);
    record.units[0].attempts.push(next);
    record.units[1].attempts.push(Attempt {
        run_id: Uuid::new_v4().to_string(),
        question: "Which output remains unexplored?".into(),
        automatic_retry: false,
        summary: None,
    });
    record.active_run_id = Some(record.units[1].attempts[0].run_id.clone());
    store::save(&f.storage, &record, true).unwrap();
    let recovered = reconcile(
        &f.storage,
        &record,
        &codex_agent::CodexAgentManager::default(),
    )
    .unwrap();
    assert_eq!(recovered.status, Status::Interrupted);
    assert!(recovered.active_run_id.is_none());
    assert!(unit_complete(&recovered.units[0]));
    assert_eq!(unit_status(&recovered.units[1]), "interrupted");
    let question = continuation(&f.storage, &recovered, 1).unwrap();
    assert!(question.contains("Explicit continuation"));
    assert!(question.contains("do not repeat"));
    assert!(question.contains("first new section MUST use order:0 and previousGenerationId:null"));
    assert!(question.contains("Earlier saved sections are outside this investigation"));
    assert!(question.contains("nextNewSectionOrder"));
    assert_eq!(recovered.units[0].attempts.len(), 1);
    assert_eq!(recovered.units[1].attempts.len(), 1);
}
#[test]
fn recovery_preserves_a_concurrent_stop() {
    let f = Fixture::new();
    let record = record(&f);
    store::save(&f.storage, &record, true).unwrap();
    store::update(&f.storage, &record.id, |r| {
        r.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    let path = f
        .storage
        .join("repository-guides")
        .join(format!("{}.json", record.id));
    let stopped_bytes = std::fs::read(&path).unwrap();
    let recovered = reconcile(
        &f.storage,
        &record,
        &codex_agent::CodexAgentManager::default(),
    )
    .unwrap();
    assert_eq!(recovered.status, Status::Stopped);
    assert_eq!(recovered.revision, 1);
    assert_eq!(std::fs::read(path).unwrap(), stopped_bytes);
}

#[test]
fn cached_metadata_cannot_hide_a_damaged_existing_answer_receipt() {
    let f = Fixture::new();
    let mut r = record(&f);
    let a = attempt(&r, Outcome::Interrupted);
    let run = a.run_id.clone();
    r.units[0].attempts.push(a);
    store::create(&f.storage, &r).unwrap();
    assert!(!reading::exists(&f.storage, &run).unwrap());
    let summaries = summary(&f.storage, &r).unwrap().answers;
    assert_eq!(summaries.len(), 2); // Current guides always retain their opening.
    assert!(summaries.iter().any(|answer| answer.run_id == run));
    let directory = f.storage.join("answer-sections");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{run}.json"));
    std::fs::write(&path, b"{}").unwrap();
    assert!(reading::exists(&f.storage, &run).unwrap());
    // Listing a saved guide uses its already-published summary and must stay
    // responsive; opening/reconciling the receipt still performs full replay.
    let listed = summary_for_listing(&f.storage, &r).unwrap();
    assert!(listed.answers.iter().any(|answer| answer.run_id == run));
    assert!(summary(&f.storage, &r).is_err());
    r.units[0].attempts[0].summary = None;
    assert!(reconcile(&f.storage, &r, &codex_agent::CodexAgentManager::default()).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"{}");
}

#[test]
#[ignore = "Explicit existing-account GPT-6 Luna/max whole Fontdone guide; one bounded automatic retry for a failed outline or unpublished slide; no live run during regression testing."]
fn fontdone_repository_guide_account_probe() {
    use serde_json::json;
    use std::io::Write;
    use std::time::Instant;
    let helper = std::path::PathBuf::from(
        std::env::var_os("CODE_ATLAS_CONTEXT_HELPER_BIN")
            .expect("set the rebuilt application helper"),
    );
    assert!(helper.is_absolute() && helper.is_file());
    assert_ne!(
        std::fs::canonicalize(helper).unwrap(),
        std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap()
    );
    let storage = std::path::PathBuf::from(
        std::env::var_os("CODE_ATLAS_FONTDONE_DATA_DIR")
            .expect("set isolated frozen Fontdone storage"),
    );
    let output = std::path::PathBuf::from(
        std::env::var_os("CODE_ATLAS_GUIDE_PROBE_OUTPUT").expect("set a fresh private output path"),
    );
    assert!(storage.is_absolute() && output.is_absolute());
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut artifact = options
        .open(&output)
        .expect("never overwrite an earlier guide attempt");
    let project = "/Users/lazytrot/work/fontdone";
    let snapshot_id = "snap-1792140879637485000-11155-0";
    let manifest = snapshot::load_snapshot_manifest(&storage, project, snapshot_id).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    manager.resume_connection(&storage).unwrap();
    let runtime = manager.review_runtime(&storage).unwrap();
    let resume_id = std::env::var("CODE_ATLAS_GUIDE_RESUME_ID").ok();
    let record = if let Some(id) = &resume_id {
        let original = scoped(&storage, id, project, snapshot_id).unwrap();
        assert!(
            original.plan.is_some(),
            "Explicit continuation cannot repeat planning"
        );
        assert!(!matches!(
            original.status,
            Status::Planning | Status::Building
        ));
        workspace::check_retrieval_grant(
            &storage,
            &original.grant_id,
            project,
            snapshot_id,
            &runtime,
        )
        .unwrap();
        original
    } else {
        let grant_id =
            workspace::save_retrieval_grant(&storage, project, snapshot_id, &runtime).unwrap();
        new_record(Start { id:Uuid::new_v4().to_string(), project_id:project.into(), snapshot_id:snapshot_id.into(),
            grant_id, question:"How does Fontdone turn font bytes into a rendered glyph bitmap? Build a coherent guide through real entry points, parsing, outline loading, hinting, rasterization, output and important failure paths. Include alternate supported font representations where they change this journey; name any boundaries that remain unexplored.".into(),
            model_id:DEFAULT_MODEL_ID.into(), reasoning_effort:DEFAULT_REASONING_EFFORT.into(), consent:true, renderer_queue_wait_ms:None }, &manifest).unwrap()
    };
    let _lease = store::lease(&storage, &record.id).unwrap().unwrap();
    if resume_id.is_none() {
        store::create(&storage, &record).unwrap();
        store::update(&storage, &record.id, |r| {
            r.active_run_id = Some(r.planning_run_id.clone());
            Ok(())
        })
        .unwrap();
    }
    let record = if resume_id.is_some() {
        let reconciled = reconcile(&storage, &record, &manager).unwrap();
        store::prepare_continuation(&storage, &record.id, reconciled.revision).unwrap()
    } else {
        record
    };
    let started = Instant::now();
    let observations = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&observations);
    let saved_storage = storage.clone();
    let progress = |p: codex_agent::CodexProgress| {
        println!("GUIDE_PHASE {} {}", p.phase, started.elapsed().as_millis());
    };
    let notify: Arc<dyn Fn(SectionNotice) + Send + Sync> = Arc::new(
        move |notice: SectionNotice| {
            let saved = reading::adopt(
                &saved_storage,
                &notice.run_id,
                project,
                snapshot_id,
                &notice.generation_id,
            );
            observed.lock().unwrap().push(json!({"notice":notice,"elapsedMs":started.elapsed().as_millis(),"saved":saved.is_ok(),"error":saved.err()}));
            println!(
                "GUIDE_SECTION {} {}",
                notice.order,
                started.elapsed().as_millis()
            );
        },
    );
    let adopt = |notice: &SectionNotice| {
        if !saved_section(&storage, notice)? {
            reading::adopt(
                &storage,
                &notice.run_id,
                project,
                snapshot_id,
                &notice.generation_id,
            )?;
        }
        Ok(())
    };
    let changed = |guide: &GuideSummary| {
        let progress = json!({"guide":guide,"elapsedMs":started.elapsed().as_millis(),"accepted":false,"semanticReview":false,"voiceReview":false});
        codex_agent::atomic_private_write(
            &storage.join("progress.json"),
            &serde_json::to_vec_pretty(&progress).unwrap(),
        )
        .unwrap();
        println!(
            "GUIDE_PROGRESS {:?} {}",
            guide.status,
            started.elapsed().as_millis()
        );
    };
    // Exercise the production runway/refill cadence. This acceptance probe
    // treats each ready slide as heard; the packaged reader supplies these
    // receipts from narration completion or explicit Next actions.
    let mut first_build = true;
    let result = loop {
        let pass = build(
            &storage,
            &record.id,
            first_build && resume_id.is_some(),
            &manager,
            RepositoryContextJobs::default(),
            &recovery::InterruptionJournal::default(),
            GUIDE_SLIDES_PER_OPERATION,
            None,
            None,
            Instant::now(),
            &progress,
            Arc::clone(&notify),
            &adopt,
            &changed,
        );
        if pass.is_err() {
            break pass;
        }
        first_build = false;
        let saved = store::read(&storage, &record.id).unwrap();
        if saved.status != Status::WaitingForPlayback {
            break pass;
        }
        let next_revision = saved
            .opening
            .iter()
            .chain(&saved.opening_continuations)
            .chain(saved.units.iter().flat_map(|unit| &unit.attempts))
            .filter_map(|attempt| attempt.summary.as_ref())
            .flat_map(|answer| &answer.sections)
            .find(|section| {
                !saved
                    .consumed_slide_revisions
                    .contains(&section.revision_id)
            })
            .map(|section| section.revision_id.clone())
            .expect("a playback-waiting guide has an unheard slide");
        let saved = store::update(&storage, &record.id, |current| {
            if !current.consumed_slide_revisions.contains(&next_revision) {
                current.consumed_slide_revisions.push(next_revision.clone());
            }
            Ok(())
        })
        .unwrap();
        store::prepare_continuation(&storage, &record.id, saved.revision).unwrap();
    };
    let saved = store::read(&storage, &record.id).unwrap();
    let runs = std::iter::once(saved.planning_run_id.as_str())
        .chain(
            saved
                .opening_continuations
                .iter()
                .map(|attempt| attempt.run_id.as_str()),
        )
        .chain(
            saved
                .units
                .iter()
                .flat_map(|unit| unit.attempts.iter().map(|attempt| attempt.run_id.as_str())),
        );
    let ledgers = runs
        .map(|run| {
            (
                run.to_owned(),
                crate::evidence_ledger::inspect(&storage, run)
                    .ok()
                    .and_then(|l| serde_json::to_value(l).ok()),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let receipt = json!({"sourceRevision":std::env::var("CODE_ATLAS_QUALIFICATION_REVISION").unwrap_or_else(|_|"unrecorded".into()),
        "elapsedMs":started.elapsed().as_millis(),"guideId":record.id,"explicitContinuation":resume_id.is_some(),"model":"gpt-6-luna","effort":"max", "accepted":false,
        "humanReview":false,"voiceReview":false,"result":result.as_ref().ok(),"error":result.as_ref().err(),
        "record":saved,"observations":*observations.lock().unwrap(),"ledgers":ledgers});
    artifact
        .write_all(&serde_json::to_vec_pretty(&receipt).unwrap())
        .unwrap();
    artifact.sync_all().unwrap();
    assert_eq!(
        result.unwrap().status,
        Status::Ready,
        "A partial guide is not a complete guide qualification"
    );
}
