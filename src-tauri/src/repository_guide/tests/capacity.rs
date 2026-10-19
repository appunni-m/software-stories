//! Storage state fixtures only: no account, inference or accepted chapter.

use super::*;
use crate::{codex_identity::CodexReviewRuntime, evidence_ledger::Ledger};

fn runtime(f: &Fixture) -> (CodexReviewRuntime, String) {
    let runtime = CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 1,
    };
    let grant = workspace::save_retrieval_grant(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
        &runtime,
    )
    .unwrap();
    (runtime, grant)
}

fn retain(f: &Fixture, run: &str, runtime: &CodexReviewRuntime, grant: &str) -> Result<(), String> {
    let ledger = Ledger::new_progressive(
        &f.storage,
        run,
        &f.manifest,
        grant,
        runtime,
        "gpt-6-luna",
        "max",
    )?;
    ledger.start_narration_answer_sections(
        "How does the captured implementation transform its input?",
        &"d".repeat(64),
        &f.manifest,
        runtime,
        &std::sync::atomic::AtomicBool::new(false),
    )?;
    Ok(())
}

fn answer_bytes(f: &Fixture) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    std::fs::read_dir(f.storage.join("answer-sections"))
        .unwrap()
        .map(|entry| {
            let path = entry.unwrap().path();
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect()
}

#[test]
fn a_guide_retains_all_fourteen_owned_investigations_with_seven_other_answers() {
    let f = Fixture::new();
    let (runtime, grant) = runtime(&f);
    for _ in 0..7 {
        retain(&f, &Uuid::new_v4().to_string(), &runtime, &grant).unwrap();
    }
    let prior = answer_bytes(&f);
    let mut guide = fresh_record(&f);
    guide.grant_id = grant.clone();
    let plan = guide.plan.as_mut().unwrap();
    let starting_ranges = plan.units[0].starting_ranges.clone();
    for index in 2..MAX_UNITS {
        let key = format!("stage-{index}");
        plan.units.push(plan::UnitPlan {
            key: key.clone(),
            question: format!("How does implementation stage {index} transform the input?"),
            prerequisites: vec![],
            starting_ranges: starting_ranges.clone(),
        });
        guide.units.push(Unit {
            key,
            attempts: vec![],
        });
    }
    let opening = guide.opening.as_mut().unwrap();
    opening.summary = Some(Summary {
        run_id: opening.run_id.clone(),
        project_id: guide.project_id.clone(),
        snapshot_id: guide.snapshot_id.clone(),
        question: opening.question.clone(),
        outcome: Outcome::Partial,
        sections: vec![],
        incomplete_topics: vec!["The initial transition remains unexplored.".into()],
    });
    guide.opening_continuations.push(Attempt {
        run_id: Uuid::new_v4().to_string(),
        question: opening::question(&guide).unwrap().into(),
        summary: None,
    });
    for (unit, plan) in guide
        .units
        .iter_mut()
        .zip(&guide.plan.as_ref().unwrap().units)
    {
        for _ in 0..MAX_ATTEMPTS {
            unit.attempts.push(Attempt {
                run_id: Uuid::new_v4().to_string(),
                question: plan.question.clone(),
                summary: None,
            });
        }
    }
    store::create(&f.storage, &guide).unwrap();
    let runs = guide
        .opening
        .iter()
        .chain(&guide.opening_continuations)
        .chain(guide.units.iter().flat_map(|unit| &unit.attempts))
        .map(|attempt| attempt.run_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(runs.len(), 14);
    for run in &runs {
        retain(&f, run, &runtime, &grant).unwrap();
    }
    let after = answer_bytes(&f);
    assert_eq!(after.len(), 21);
    for (path, bytes) in prior {
        assert_eq!(after.get(&path), Some(&bytes));
    }
    retain(&f, &Uuid::new_v4().to_string(), &runtime, &grant).unwrap();
    assert_eq!(answer_bytes(&f).len(), after.len() + 1);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn guide_capacity_reservation_is_persisted_before_any_account_operation() {
    let f = Fixture::new();
    let (runtime, grant) = runtime(&f);
    for _ in 0..8 {
        retain(&f, &Uuid::new_v4().to_string(), &runtime, &grant).unwrap();
    }
    let prior = answer_bytes(&f);
    let mut guide = fresh_record(&f);
    guide.grant_id = grant;
    store::create(&f.storage, &guide).unwrap();
    assert!(store::exists(&f.storage, &guide.id).unwrap());
    assert_eq!(answer_bytes(&f), prior);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn a_new_guide_reserves_its_future_children_before_the_first_answer_exists() {
    let f = Fixture::new();
    let (runtime, grant) = runtime(&f);
    for _ in 0..7 {
        retain(&f, &Uuid::new_v4().to_string(), &runtime, &grant).unwrap();
    }
    let prior = answer_bytes(&f);
    let mut guide = fresh_record(&f);
    guide.grant_id = grant.clone();
    store::create(&f.storage, &guide).unwrap();
    retain(&f, &Uuid::new_v4().to_string(), &runtime, &grant).unwrap();
    assert_eq!(answer_bytes(&f).len(), prior.len() + 1);
    retain(&f, &guide.planning_run_id, &runtime, &grant).unwrap();
    assert_eq!(answer_bytes(&f).len(), prior.len() + 2);
}

#[test]
fn a_guide_slot_cannot_be_reused_with_another_captured_inventory() {
    let f = Fixture::new();
    let (runtime, grant) = runtime(&f);
    let mut guide = fresh_record(&f);
    guide.grant_id = grant.clone();
    guide.manifest_sha256 = "f".repeat(64);
    store::create(&f.storage, &guide).unwrap();
    let answer = retain(&f, &guide.planning_run_id, &runtime, &grant);
    assert!(matches!(answer, Err(message) if message.contains("another scope")));
    assert!(answer_bytes(&f).is_empty());
    assert_eq!(store::read(&f.storage, &guide.id).unwrap().revision, 0);
}

#[cfg(unix)]
#[test]
fn a_competing_capacity_lease_prevents_guide_creation_without_spending_an_attempt() {
    let f = Fixture::new();
    let guide = fresh_record(&f);
    let path = f.storage.join(".answer-capacity.lock");
    let original = b"controlled existing lease bytes";
    std::fs::write(&path, original).unwrap();
    let owner = crate::file_lease::try_exclusive(std::fs::File::open(&path).unwrap())
        .unwrap()
        .unwrap();
    let started = std::time::Instant::now();
    let result = store::create(&f.storage, &guide);
    assert!(matches!(result, Err(message) if message.contains("transaction is active")));
    assert!(started.elapsed() < std::time::Duration::from_secs(1));
    assert!(!store::exists(&f.storage, &guide.id).unwrap());
    assert!(!f.storage.join("answer-sections").exists());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    drop(owner);
    store::create(&f.storage, &guide).unwrap();
    assert_eq!(store::read(&f.storage, &guide.id).unwrap().revision, 0);
}

#[test]
fn simultaneous_standalone_answers_keep_their_independent_receipts() {
    let f = Fixture::new();
    let (runtime, grant) = runtime(&f);
    for _ in 0..7 {
        retain(&f, &Uuid::new_v4().to_string(), &runtime, &grant).unwrap();
    }
    let prior = answer_bytes(&f);
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut workers = vec![];
    for _ in 0..2 {
        let runtime = runtime.clone();
        let grant = grant.clone();
        let storage = f.storage.clone();
        let manifest = f.manifest.clone();
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            let ledger = Ledger::new_progressive(
                &storage,
                &Uuid::new_v4().to_string(),
                &manifest,
                &grant,
                &runtime,
                "gpt-6-luna",
                "max",
            )
            .unwrap();
            barrier.wait();
            ledger
                .start_narration_answer_sections(
                    "How does the implementation transform this input?",
                    &"d".repeat(64),
                    &manifest,
                    &runtime,
                    &std::sync::atomic::AtomicBool::new(false),
                )
                .is_ok()
        }));
    }
    barrier.wait();
    let successes = workers
        .into_iter()
        .map(|worker| worker.join().unwrap())
        .filter(|success| *success)
        .count();
    assert_eq!(successes, 2);
    let after = answer_bytes(&f);
    assert_eq!(after.len(), 9);
    for (path, bytes) in prior {
        assert_eq!(after.get(&path), Some(&bytes));
    }
}
