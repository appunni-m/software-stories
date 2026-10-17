use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};

pub(super) fn record(f: &Fixture) -> Record {
    let source = context::source(&f.storage, &f.manifest, SIMPLE_PATH).unwrap();
    let range = context::range_from_source(&source, 0, SIMPLE_SOURCE.len())
        .unwrap()
        .range;
    Record {
        revision: 0,
        schema_version: 1,
        contract: CONTRACT.into(),
        id: Uuid::new_v4().to_string(),
        project_id: f.manifest.summary.project_id.clone(),
        snapshot_id: f.manifest.summary.snapshot_id.clone(),
        manifest_sha256: context::digest(&serde_json::to_vec(&f.manifest).unwrap()),
        grant_id: Uuid::new_v4().to_string(),
        question: "How does this project work?".into(),
        planning_run_id: Uuid::new_v4().to_string(),
        plan: Some(Plan {
            title: "A grounded repository guide".into(),
            units: vec![
                plan::UnitPlan {
                    key: "input".into(),
                    question: "How does the name enter the greeting?".into(),
                    prerequisites: vec![],
                    starting_ranges: vec![range.clone()],
                },
                plan::UnitPlan {
                    key: "output".into(),
                    question: "How does the greeting become output?".into(),
                    prerequisites: vec!["input".into()],
                    starting_ranges: vec![range],
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
    }
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
        summary: Some(Summary {
            run_id,
            question,
            project_id: record.project_id.clone(),
            snapshot_id: record.snapshot_id.clone(),
            outcome,
            sections,
            incomplete_topics: vec![],
        }),
    }
}
#[test]
fn prerequisites_wait_for_a_complete_unit_and_completed_units_are_skipped() {
    let f = Fixture::new();
    let mut record = record(&f);
    assert!(prerequisite_complete(&record, 0));
    assert!(!prerequisite_complete(&record, 1));
    let next = attempt(&record, Outcome::Partial);
    record.units[0].attempts.push(next);
    assert!(!unit_complete(&record.units[0]));
    assert!(!prerequisite_complete(&record, 1));
    let next = attempt(&record, Outcome::Complete);
    record.units[0].attempts.push(next);
    assert!(unit_complete(&record.units[0]));
    assert!(prerequisite_complete(&record, 1));
    assert_eq!(MAX_UNITS, 6);
    assert_eq!(MAX_ATTEMPTS, 2);
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
        assert_eq!(value["researchBoundaries"], serde_json::json!(record.plan.as_ref().unwrap().omissions));
        assert_eq!(value["unitKey"], record.units[index].key);
        assert_eq!(value["unitNumber"], index + 1);
        assert_eq!(value["unitCount"], 2);
        assert!(continuation(&f.storage, &record, index).unwrap().starts_with(
            &record.plan.as_ref().unwrap().units[index].question
        ));
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
    assert!(view.answers.is_empty());
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
}

#[test]
fn saved_guide_listing_recovers_interruption_without_dispatch_or_replacing_the_plan() {
    let f = Fixture::new();
    let mut original = record(&f);
    original.active_run_id = Some(original.planning_run_id.clone());
    store::create(&f.storage, &original).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    let result = list_saved_guides(&f.storage, &original.project_id, &original.snapshot_id, &manager).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].status, Status::Interrupted);
    let saved = store::read(&f.storage, &original.id).unwrap();
    assert_eq!(saved.plan, original.plan);
    assert!(saved.active_run_id.is_none());
    assert!(saved.units.iter().all(|unit| unit.attempts.is_empty()));
    let revision = saved.revision;
    list_saved_guides(&f.storage, &original.project_id, &original.snapshot_id, &manager).unwrap();
    assert_eq!(store::read(&f.storage, &original.id).unwrap().revision, revision);
    assert!(!f.storage.join("evidence-ledgers").exists());
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
    let recovered = reconcile(
        &f.storage,
        &record,
        &codex_agent::CodexAgentManager::default(),
    )
    .unwrap();
    assert_eq!(recovered.status, Status::Stopped);
    assert_eq!(recovered.revision, 2);
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
    assert_eq!(summary(&f.storage, &r).unwrap().answers.len(), 1);
    let directory = f.storage.join("answer-sections");
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join(format!("{run}.json"));
    std::fs::write(&path, b"{}").unwrap();
    assert!(reading::exists(&f.storage, &run).unwrap());
    assert!(summary(&f.storage, &r).is_err());
    r.units[0].attempts[0].summary = None;
    assert!(reconcile(&f.storage, &r, &codex_agent::CodexAgentManager::default()).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"{}");
}

#[test]
#[ignore = "Explicit existing-account GPT-6 Luna/max whole Fontdone guide; at most seven bounded initial investigations, no automatic retries."]
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
        assert!(original.plan.is_some(), "Explicit continuation cannot repeat planning");
        assert!(!matches!(original.status, Status::Planning | Status::Building));
        workspace::check_retrieval_grant(&storage, &original.grant_id, project, snapshot_id, &runtime).unwrap();
        original
    } else {
        let grant_id = workspace::save_retrieval_grant(&storage, project, snapshot_id, &runtime).unwrap();
        Record{revision:0,schema_version:1,contract:CONTRACT.into(),id:Uuid::new_v4().to_string(),project_id:project.into(),snapshot_id:snapshot_id.into(),
        manifest_sha256:context::digest(&serde_json::to_vec(&manifest).unwrap()),grant_id,
        question:"How does Fontdone turn font bytes into a rendered glyph bitmap? Build a coherent guide through real entry points, parsing, outline loading, hinting, rasterization, output and important failure paths. Include alternate supported font representations where they change this journey; name any boundaries that remain unexplored.".into(),
        planning_run_id:Uuid::new_v4().to_string(),plan:None,units:vec![],status:Status::Planning,active_run_id:None}
    };
    let _lease = store::lease(&storage, &record.id).unwrap().unwrap();
    if resume_id.is_none() {
        store::create(&storage, &record).unwrap();
        store::update(&storage, &record.id, |r| {
            r.active_run_id = Some(r.planning_run_id.clone());
            Ok(())
        }).unwrap();
    }
    let started = Instant::now();
    let observations = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&observations);
    let saved_storage = storage.clone();
    let result = build(
        &storage,
        &record.id,
        &manager,
        RepositoryContextJobs::default(),
        &recovery::InterruptionJournal::default(),
        |p| println!("GUIDE_PHASE {} {}", p.phase, started.elapsed().as_millis()),
        Arc::new(move |notice| {
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
        }),
        |notice| {
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
        },
        |guide| {
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
        },
    );
    let saved = store::read(&storage, &record.id).unwrap();
    let runs = std::iter::once(saved.planning_run_id.as_str()).chain(
        saved
            .units
            .iter()
            .flat_map(|u| u.attempts.iter().map(|a| a.run_id.as_str())),
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
