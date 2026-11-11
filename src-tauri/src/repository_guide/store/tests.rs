use super::*;
use crate::{
    chapter_generation::tests::Fixture,
    repository_guide::tests::{attempt, record},
};
use uuid::Uuid;
#[test]
fn private_records_reopen_and_conflicting_creates_preserve_original_bytes() {
    let f = Fixture::new();
    let record = record(&f);
    save(&f.storage, &record, true).unwrap();
    let before = std::fs::read(path(&f.storage, &record.id).unwrap()).unwrap();
    assert!(save(&f.storage, &record, true).is_err());
    assert_eq!(
        std::fs::read(path(&f.storage, &record.id).unwrap()).unwrap(),
        before
    );
    assert_eq!(read(&f.storage, &record.id).unwrap().id, record.id);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(path(&f.storage, &record.id).unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[test]
fn opening_only_narrow_guide_can_be_saved_and_reopened_ready() {
    let f = Fixture::new();
    let mut guide = record(&f);
    let plan = guide.plan.as_mut().unwrap();
    plan.units.clear();
    plan.omissions.clear();
    guide.units.clear();

    let run_id = guide.planning_run_id.clone();
    let project_id = guide.project_id.clone();
    let snapshot_id = guide.snapshot_id.clone();
    let question = guide.question.clone();
    let opening = guide.opening.as_mut().unwrap();
    let section = SectionNotice {
        run_id: run_id.clone(),
        project_id: project_id.clone(),
        snapshot_id: snapshot_id.clone(),
        key: "opening".into(),
        order: 0,
        generation_id: format!("generation:{}", Uuid::new_v4()),
        revision_id: format!("revision:{}", Uuid::new_v4()),
        supersedes: None,
        visual_forms: Vec::new(),
        context_elapsed_ms: 1,
        run_elapsed_ms: Some(2),
    };
    opening.summary = Some(Summary {
        run_id,
        project_id,
        snapshot_id,
        question,
        outcome: Outcome::Complete,
        failure_code: None,
        sections: vec![section],
        incomplete_topics: Vec::new(),
    });
    guide.status = Status::Ready;

    save(&f.storage, &guide, true).unwrap();
    let reopened = read(&f.storage, &guide.id).unwrap();

    assert_eq!(reopened.status, Status::Ready);
    assert!(reopened.plan.as_ref().unwrap().units.is_empty());
    assert!(reopened.plan.as_ref().unwrap().omissions.is_empty());
    assert!(super::super::guide_complete(&reopened));
}

#[test]
fn prerequisite_receipt_must_reference_a_saved_complete_prerequisite_version() {
    let f = Fixture::new();
    let mut guide = record(&f);
    let prerequisite_attempt = attempt(&guide, Outcome::Complete);
    let prerequisite_revision = prerequisite_attempt.summary.as_ref().unwrap().sections[0]
        .revision_id
        .clone();
    guide.units[0].attempts.push(prerequisite_attempt);
    let mut dependent_attempt = Attempt {
        run_id: Uuid::new_v4().to_string(),
        question: guide.plan.as_ref().unwrap().units[1].question.clone(),
        automatic_retry: false,
        prerequisite_fingerprints: vec![PrerequisiteFingerprint {
            key: "input".into(),
            revision_ids: vec![prerequisite_revision],
        }],
        summary: None,
    };
    validate_prerequisite_fingerprints(
        &guide,
        &guide.plan.as_ref().unwrap().units[1],
        &dependent_attempt,
    )
    .unwrap();

    dependent_attempt.prerequisite_fingerprints[0].revision_ids[0] = "revision:foreign".into();
    assert!(
        validate_prerequisite_fingerprints(
            &guide,
            &guide.plan.as_ref().unwrap().units[1],
            &dependent_attempt,
        )
        .is_err()
    );
}

#[test]
fn missing_current_prerequisite_receipt_is_rejected_without_rewriting_saved_bytes() {
    let f = Fixture::new();
    let mut guide = record(&f);
    let prerequisite_fingerprints = unit_prerequisite_fingerprints(&guide, 1).unwrap();
    guide.units[1].attempts.push(Attempt {
        run_id: Uuid::new_v4().to_string(),
        question: guide.plan.as_ref().unwrap().units[1].question.clone(),
        automatic_retry: false,
        prerequisite_fingerprints,
        summary: None,
    });
    save(&f.storage, &guide, true).unwrap();

    let location = path(&f.storage, &guide.id).unwrap();
    let current = serde_json::to_value(&guide).unwrap();
    for (version, contract) in [
        (9, "repository-guide-v10-five-initial-units"),
        (
            RECORD_SCHEMA_VERSION,
            "repository-guide-v11-five-initial-units",
        ),
    ] {
        let mut incompatible = current.clone();
        incompatible["schemaVersion"] = serde_json::json!(version);
        incompatible["contract"] = serde_json::json!(contract);
        incompatible["units"][1]["attempts"][0]
            .as_object_mut()
            .unwrap()
            .remove("prerequisiteFingerprints");
        let bytes = serde_json::to_vec(&incompatible).unwrap();
        std::fs::write(&location, &bytes).unwrap();

        let error = read(&f.storage, &guide.id)
            .err()
            .expect("a missing prerequisite receipt is incompatible");
        assert!(error.contains("damaged or incompatible"), "{error}");
        assert_eq!(std::fs::read(&location).unwrap(), bytes);
    }
}

#[test]
fn worker_lease_excludes_a_second_worker_until_the_original_is_joined() {
    let f = Fixture::new();
    let id = Uuid::new_v4().to_string();
    let worker = lease(&f.storage, &id).unwrap().unwrap();
    assert!(lease(&f.storage, &id).unwrap().is_none());
    drop(worker);
    assert!(lease(&f.storage, &id).unwrap().is_some());
}

#[test]
fn replacing_a_guide_grant_is_scoped_revision_checked_and_idempotent() {
    let f = Fixture::new();
    let guide = record(&f);
    let id = guide.id.clone();
    let original_grant = guide.grant_id.clone();
    create(&f.storage, &guide).unwrap();

    let replacement = Uuid::new_v4().to_string();
    let updated = replace_grant(&f.storage, &id, guide.revision, &replacement).unwrap();
    assert_eq!(updated.grant_id, replacement);
    assert_eq!(updated.revision, guide.revision + 1);
    assert_eq!(updated.question, guide.question);
    assert_eq!(
        updated.opening.as_ref().map(|opening| &opening.run_id),
        guide.opening.as_ref().map(|opening| &opening.run_id)
    );
    assert_ne!(updated.grant_id, original_grant);

    let unchanged = replace_grant(&f.storage, &id, updated.revision, &replacement).unwrap();
    assert_eq!(unchanged.revision, updated.revision);

    let path = path(&f.storage, &id).unwrap();
    let before_stale = fs::read(&path).unwrap();
    let stale_replacement = Uuid::new_v4().to_string();
    assert!(replace_grant(&f.storage, &id, guide.revision, &stale_replacement).is_err());
    assert_eq!(fs::read(&path).unwrap(), before_stale);

    let active = update(&f.storage, &id, |current| {
        current.active_run_id = Some(current.planning_run_id.clone());
        Ok(())
    })
    .unwrap();
    let before_active = fs::read(&path).unwrap();
    assert!(replace_grant(&f.storage, &id, active.revision, &stale_replacement).is_err());
    assert_eq!(fs::read(&path).unwrap(), before_active);
    assert!(replace_grant(&f.storage, &id, active.revision, "not-a-uuid").is_err());
    assert_eq!(fs::read(&path).unwrap(), before_active);
}

#[test]
fn persistent_worker_lock_files_do_not_consume_saved_guide_capacity() {
    assert_eq!(MAX_GUIDES, 1_000);
    assert_eq!(MAX_BYTES, MAX_GUIDES as u64 * MAX_RECORD as u64);

    let f = Fixture::new();
    for _ in 0..30 {
        let guide = record(&f);
        create(&f.storage, &guide).unwrap();
        drop(lease(&f.storage, &guide.id).unwrap().unwrap());
    }
    for _ in 0..3 {
        drop(
            lease(&f.storage, &Uuid::new_v4().to_string())
                .unwrap()
                .unwrap(),
        );
    }

    let directory = directory(&f.storage).unwrap();
    assert_eq!(fs::read_dir(&directory).unwrap().count(), 64);
    let next = record(&f);
    create(&f.storage, &next).unwrap();
    assert!(exists(&f.storage, &next.id).unwrap());

    let entries = scan(&directory).unwrap();
    assert_eq!(entries.records.len(), 31);
    assert_eq!(entries.lock_files, 34);
}

#[cfg(unix)]
#[test]
fn ending_the_owner_releases_a_lease_even_if_an_inherited_descriptor_remains() {
    let f = Fixture::new();
    let id = Uuid::new_v4().to_string();
    let owner = lease(&f.storage, &id).unwrap().unwrap();
    // dup and fork retain the same open-file-description lock. Simulate the
    // inherited descriptor without forking a multithreaded Rust test process.
    let inherited = owner.try_clone().unwrap();
    assert!(lease(&f.storage, &id).unwrap().is_none());
    drop(owner);
    let next = lease(&f.storage, &id).unwrap();
    assert!(
        next.is_some(),
        "an inherited descriptor must not extend the ended owner's lease"
    );
    drop(inherited);
    assert!(lease(&f.storage, &id).unwrap().is_none());
    drop(next);
    assert!(lease(&f.storage, &id).unwrap().is_some());
}
#[test]
fn damaged_contract_duplicate_runs_and_excess_attempts_fail_without_rewriting() {
    let f = Fixture::new();
    let mut r = record(&f);
    let next = attempt(&r, Outcome::Complete);
    r.units[0].attempts.push(next);
    assert!(validate(&r).is_ok());
    let mut a = r.units[0].attempts[0].clone();
    a.prerequisite_fingerprints = unit_prerequisite_fingerprints(&r, 1).unwrap();
    r.units[1].attempts.push(a);
    assert!(validate(&r).is_err());
    r.units[1].attempts.clear();
    r.contract = "unknown".into();
    assert!(validate(&r).is_err());
    r.contract = CONTRACT.into();
    for _ in 0..3 {
        let next = attempt(&r, Outcome::Partial);
        r.units[0].attempts.push(next);
    }
    assert!(validate(&r).is_err());
    assert!(read(&f.storage, "../../outside").is_err());
}

#[test]
fn fresh_independent_research_requires_its_own_schema_and_opening() {
    let f = Fixture::new();
    let fresh = crate::repository_guide::tests::fresh_record(&f);
    assert_eq!(fresh.schema_version, RECORD_SCHEMA_VERSION);
    assert_eq!(fresh.contract, CONTRACT);
    assert!(validate(&fresh).is_ok());
    for (schema, contract) in [
        (0, CONTRACT),
        (1, CONTRACT),
        (2, CONTRACT),
        (3, CONTRACT),
        (4, CONTRACT),
        (5, CONTRACT),
        (6, CONTRACT),
        (7, CONTRACT),
        (8, CONTRACT),
        (9, CONTRACT),
        (11, CONTRACT),
    ] {
        let mut invalid = fresh.clone();
        invalid.schema_version = schema;
        invalid.contract = contract.into();
        assert!(validate(&invalid).is_err(), "{schema}/{contract}");
    }
    let mut missing = fresh.clone();
    missing.opening = None;
    assert!(validate(&missing).is_err());
    let mut invalid_edge = fresh;
    invalid_edge.plan.as_mut().unwrap().units[0].prerequisites = vec!["output".into()];
    assert!(
        validate(&invalid_edge).is_err(),
        "independent research still preserves the reading DAG"
    );
}

#[test]
fn automatic_retry_markers_cannot_replace_the_original_slide_attempt() {
    let f = Fixture::new();
    let mut invalid_opening = crate::repository_guide::tests::fresh_record(&f);
    invalid_opening.opening.as_mut().unwrap().automatic_retry = true;
    assert!(validate(&invalid_opening).is_err());

    let mut invalid_topic = crate::repository_guide::tests::fresh_record(&f);
    invalid_topic.units[0].attempts.push(Attempt {
        run_id: Uuid::new_v4().to_string(),
        question: invalid_topic.plan.as_ref().unwrap().units[0]
            .question
            .clone(),
        automatic_retry: true,
        prerequisite_fingerprints: vec![],
        summary: None,
    });
    assert!(validate(&invalid_topic).is_err());
}

#[test]
fn playback_wait_state_keeps_only_unique_receipts_for_published_slides() {
    let f = Fixture::new();
    let mut waiting = crate::repository_guide::tests::fresh_record(&f);
    let completed = attempt(&waiting, Outcome::Complete);
    let revision_id = completed.summary.as_ref().unwrap().sections[0]
        .revision_id
        .clone();
    waiting.units[0].attempts.push(completed);
    waiting.status = Status::WaitingForPlayback;
    waiting.consumed_slide_revisions.push(revision_id.clone());
    assert!(validate(&waiting).is_ok());

    let mut duplicate = waiting.clone();
    duplicate.consumed_slide_revisions.push(revision_id);
    assert!(validate(&duplicate).is_err());

    let mut foreign = waiting;
    foreign.consumed_slide_revisions = vec!["revision:not-published".into()];
    assert!(validate(&foreign).is_err());
}

#[test]
fn a_failed_outline_gets_one_automatic_and_one_explicit_fresh_owner_retry() {
    let f = Fixture::new();
    let pending = crate::repository_guide::opening::tests::pending(&f);
    assert_eq!(
        super::super::opening_continuation::remaining_operations(&pending),
        0,
        "an active planner cannot be continued in parallel"
    );
    create(&f.storage, &pending).unwrap();
    let first_run = pending.planning_run_id.clone();
    let failed = update(&f.storage, &pending.id, |current| {
        current.active_run_id = None;
        current.status = Status::Partial;
        Ok(())
    })
    .unwrap();
    assert_eq!(
        super::super::opening_continuation::remaining_operations(&failed),
        1,
        "the first failed outline has one bounded automatic retry"
    );

    let retry = prepare_plan_retry(&f.storage, &pending.id, failed.revision).unwrap();
    assert_eq!(retry.status, Status::Planning);
    assert_eq!(retry.planning_retry_count, 1);
    assert_ne!(retry.planning_run_id, first_run);
    assert_eq!(
        retry.active_run_id.as_deref(),
        Some(retry.planning_run_id.as_str())
    );
    assert!(retry.opening.as_ref().unwrap().summary.is_none());

    let failed_again = update(&f.storage, &pending.id, |current| {
        current.active_run_id = None;
        current.status = Status::Partial;
        Ok(())
    })
    .unwrap();
    let explicit_retry =
        prepare_plan_retry(&f.storage, &pending.id, failed_again.revision).unwrap();
    assert_eq!(explicit_retry.planning_retry_count, 2);
    assert_ne!(explicit_retry.planning_run_id, retry.planning_run_id);
    let exhausted = update(&f.storage, &pending.id, |current| {
        current.active_run_id = None;
        current.status = Status::Partial;
        Ok(())
    })
    .unwrap();
    let before = std::fs::read(path(&f.storage, &pending.id).unwrap()).unwrap();
    assert!(prepare_plan_retry(&f.storage, &pending.id, exhausted.revision).is_err());
    assert_eq!(
        std::fs::read(path(&f.storage, &pending.id).unwrap()).unwrap(),
        before
    );
}

#[test]
fn opening_binding_rejects_changed_context() {
    let f = Fixture::new();
    let fresh = crate::repository_guide::tests::fresh_record(&f);
    for kind in [
        "missing",
        "digest",
        "guideQuestion",
        "openingQuestion",
        "project",
        "snapshot",
        "run",
    ] {
        let mut invalid = fresh.clone();
        match kind {
            "missing" => invalid.opening_context_sha256 = None,
            "digest" => invalid.opening_context_sha256 = Some("a".repeat(64)),
            "guideQuestion" => invalid.question.push_str(" Changed intent."),
            "openingQuestion" => {
                invalid
                    .opening
                    .as_mut()
                    .unwrap()
                    .question
                    .push_str(" Changed intent.");
                invalid.opening_context_sha256 =
                    Some(crate::repository_guide::opening::context_sha256(&invalid).unwrap());
            }
            "project" => invalid.project_id.push_str("-other"),
            "snapshot" => invalid.snapshot_id.push_str("-other"),
            "run" => {
                invalid.planning_run_id = Uuid::new_v4().to_string();
                invalid.opening.as_mut().unwrap().run_id = invalid.planning_run_id.clone();
            }
            _ => unreachable!(),
        }
        assert!(validate(&invalid).is_err(), "{kind}");
    }
    let mut foreign = crate::repository_guide::tests::record(&f);
    foreign.opening_context_sha256 = fresh.opening_context_sha256;
    assert!(validate(&foreign).is_err());
}
#[test]
fn extended_guide_records_above_the_previous_limit_reopen_and_list() {
    let f = Fixture::new();
    let guide = record(&f);
    save(&f.storage, &guide, true).unwrap();

    let location = path(&f.storage, &guide.id).unwrap();
    let mut bytes = std::fs::read(&location).unwrap();
    let previous_limit = 512 * 1024;
    assert!(MAX_RECORD > previous_limit);
    bytes.resize(previous_limit + 1, b' ');
    std::fs::write(&location, bytes).unwrap();

    assert_eq!(read(&f.storage, &guide.id).unwrap().id, guide.id);
    let listing = list(&f.storage, &guide.project_id, &guide.snapshot_id).unwrap();
    assert_eq!(listing.records.len(), 1);
    assert_eq!(listing.records[0].id, guide.id);
}

#[test]
fn nonregular_records_and_oversized_data_are_preserved() {
    let f = Fixture::new();
    let r = record(&f);
    save(&f.storage, &r, true).unwrap();
    let location = path(&f.storage, &r.id).unwrap();
    std::fs::write(&location, vec![b'x'; MAX_RECORD + 1]).unwrap();
    assert!(read(&f.storage, &r.id).is_err());
    #[cfg(unix)]
    {
        std::fs::remove_file(&location).unwrap();
        std::os::unix::fs::symlink(f.storage.join("missing"), &location).unwrap();
        assert!(read(&f.storage, &r.id).is_err());
        assert!(list(&f.storage, &r.project_id, &r.snapshot_id).is_err());
    }
}
#[test]
fn transactions_exclude_competing_writes_and_revisions_cannot_go_backwards() {
    let f = Fixture::new();
    let r = record(&f);
    create(&f.storage, &r).unwrap();
    let transaction = lock_named(&f.storage, ".transactions.lock")
        .unwrap()
        .unwrap();
    assert!(
        update(&f.storage, &r.id, |r| {
            r.status = Status::Stopped;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(read(&f.storage, &r.id).unwrap().revision, 0);
    drop(transaction);
    let first = update(&f.storage, &r.id, |r| {
        r.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    assert_eq!(first.revision, 1);
    let second = update(&f.storage, &r.id, |r| {
        r.revision = 0;
        Ok(())
    })
    .unwrap();
    assert_eq!(second.revision, 2);
}
#[test]
fn complete_without_sections_and_foreign_cached_answer_lineage_are_rejected() {
    let f = Fixture::new();
    let mut r = record(&f);
    let mut a = attempt(&r, Outcome::Complete);
    a.summary.as_mut().unwrap().sections.clear();
    r.units[0].attempts.push(a);
    assert!(validate(&r).is_err());
    r.units[0].attempts.clear();
    let mut a = attempt(&r, Outcome::Partial);
    a.summary.as_mut().unwrap().project_id = "another repository".into();
    r.units[0].attempts.push(a);
    assert!(validate(&r).is_err());
}

#[test]
fn unsupported_guide_versions_fail_without_rewriting_saved_bytes() {
    let f = Fixture::new();
    let record = record(&f);
    create(&f.storage, &record).unwrap();
    let location = path(&f.storage, &record.id).unwrap();
    let current = serde_json::to_value(&record).unwrap();
    for version in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 11] {
        let mut unsupported = current.clone();
        unsupported["schemaVersion"] = serde_json::json!(version);
        let bytes = serde_json::to_vec(&unsupported).unwrap();
        std::fs::write(&location, &bytes).unwrap();
        assert!(read(&f.storage, &record.id).is_err(), "version {version}");
        assert_eq!(std::fs::read(&location).unwrap(), bytes);
    }
    let mut prior_contract = current;
    prior_contract["schemaVersion"] = serde_json::json!(5);
    prior_contract["contract"] =
        serde_json::json!("repository-guide-v6-bounded-opening-continuation");
    prior_contract.as_object_mut().unwrap().remove("modelId");
    prior_contract
        .as_object_mut()
        .unwrap()
        .remove("reasoningEffort");
    let prior_bytes = serde_json::to_vec(&prior_contract).unwrap();
    std::fs::write(&location, &prior_bytes).unwrap();
    assert!(read(&f.storage, &record.id).is_err());
    assert_eq!(std::fs::read(&location).unwrap(), prior_bytes);
    assert!(!f.storage.join("repository-guides/history").exists());
    assert!(!f.storage.join("evidence-ledgers").exists());
}
