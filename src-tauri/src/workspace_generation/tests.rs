use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH};

fn input(f: &Fixture) -> Input {
    Input {
        run_id: Uuid::new_v4().to_string(),
        project_id: f.manifest.summary.project_id.clone(),
        snapshot_id: f.manifest.summary.snapshot_id.clone(),
        grant_id: Uuid::new_v4().to_string(),
        question: "How does greeting format its result?".into(),
        references: vec![],
        model_id: "gpt-6-luna".into(),
        effort: "max".into(),
    }
}
#[test]
fn initial_request_contains_intent_and_no_whole_source_or_index() {
    let f = Fixture::new();
    let input = input(&f);
    let (prompt, schema, digest) = request(&input, &f.manifest).unwrap();
    assert!(prompt.contains(&input.question));
    assert!(!prompt.contains("export function greeting"));
    assert!(!prompt.contains("selectedSymbols"));
    assert!(prompt.len() < 1000);
    assert_eq!(digest.len(), 64);
    assert_eq!(schema["additionalProperties"], false);
}
#[test]
fn attached_large_files_are_starting_points_and_stale_or_forged_references_fail() {
    let mut f = Fixture::new();
    let text = format!(
        "export function giant() {{\n{}\n}}\n",
        "// static filler\n".repeat(40000)
    );
    f.recapture_with_source(SIMPLE_PATH, &text);
    assert!(text.len() > 512000);
    let mut input = input(&f);
    let source = &f
        .manifest
        .files
        .iter()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap();
    input.references.push(Reference {
        path: SIMPLE_PATH.into(),
        sha256: Some(source.sha256.clone()),
        from: None,
        to: None,
    });
    let (prompt, _, _) = request(&input, &f.manifest).unwrap();
    assert!(prompt.len() < 1200);
    assert!(!prompt.contains("static filler"));
    input.references[0].sha256 = Some("f".repeat(64));
    assert!(request(&input, &f.manifest).is_err());
    input.references[0].sha256 = None;
    input.references[0].from = Some(2);
    assert!(request(&input, &f.manifest).is_err());
    input.references[0].from = None;
    input.references[0].path = "../live.ts".into();
    assert!(request(&input, &f.manifest).is_err());
}
#[test]
fn question_settings_and_bounds_are_part_of_the_contract() {
    let f = Fixture::new();
    let mut input = input(&f);
    let first = request(&input, &f.manifest).unwrap().2;
    input.question = "Which input becomes the greeting?".into();
    assert_ne!(request(&input, &f.manifest).unwrap().2, first);
    input.model_id = "another-model".into();
    assert!(request(&input, &f.manifest).is_err());
    input.model_id = "gpt-6-luna".into();
    input.effort = "high".into();
    assert!(request(&input, &f.manifest).is_err());
    input.effort = "max".into();
    input.question = "q".repeat(12001);
    assert!(request(&input, &f.manifest).is_err());
}
#[test]
#[ignore = "Explicit existing-account GPT-6 Luna/max synthetic investigation; consumes account quota."]
fn codex_account_retrieval_probe() {
    let f = Fixture::new();
    let mut input = input(&f);
    let manager = codex_agent::CodexAgentManager::default();
    manager.resume_connection(&f.storage).unwrap();
    let runtime = manager.review_runtime(&f.storage).unwrap();
    input.grant_id = workspace::save_retrieval_grant(
        &f.storage,
        &input.project_id,
        &input.snapshot_id,
        &runtime,
    )
    .unwrap();
    let result = run(
        &f.storage,
        input.clone(),
        &manager,
        RepositoryContextJobs::default(),
        &chapters::PendingChapterDrafts::default(),
        &recovery::InterruptionJournal::default(),
        |p| println!("RETRIEVAL_PHASE {}", p.phase),
    );
    if let Ok(record) = crate::evidence_ledger::inspect(&f.storage, &input.run_id) {
        let record = serde_json::to_value(record).unwrap();
        println!(
            "RETRIEVAL_DIAGNOSTICS {}",
            json!({"status":record["status"],"calls":record["calls"],"evidence":record["evidence"]})
        );
    }
    let preview = result.unwrap();
    assert!(!preview.revision.sections.is_empty());
    assert!(
        preview
            .revision
            .sections
            .iter()
            .flat_map(|s| &s.claims)
            .flat_map(|c| &c.anchors)
            .any(|a| a.path == SIMPLE_PATH)
    );
    crate::evidence_ledger::inspect(&f.storage, &input.run_id).unwrap();
    println!(
        "RETRIEVAL_RESULT {}",
        json!({"model":preview.model_id,"sections":preview.revision.sections.len(),"elapsedScope":"synthetic","origin":preview.revision.generation_origin})
    );
    manager.shutdown();
}

/// Explicitly authorized public-repository acceptance, never part of offline CI.
#[test]
#[ignore = "Existing-account GPT-6 Luna/max Fontdone investigation; consumes account quota and requires its saved snapshot."]
fn fontdone_codex_retrieval_probe() {
    let storage = std::path::PathBuf::from(
        std::env::var("CODE_ATLAS_FONTDONE_DATA_DIR").expect("set CODE_ATLAS_FONTDONE_DATA_DIR"),
    );
    let case = std::env::var("CODE_ATLAS_FONTDONE_CASE").expect("set CODE_ATLAS_FONTDONE_CASE");
    let (question, references) = match case.as_str() {
        "broad" => (
            "How does Fontdone turn font bytes into a rendered glyph bitmap? Trace the real entry points, parsing, outline loading, hinting, and rasterization. Explain the important branches and show a flow with source links.",
            vec![],
        ),
        "memory-face" => (
            "How does Font::memory_face work? Explain its inputs, selection and validation of the face, and the data it creates. Follow necessary implementation dependencies.",
            vec![Reference {
                path: "src/font.rs".into(),
                sha256: None,
                from: None,
                to: None,
            }],
        ),
        "outline" => (
            "When does Outline::is_empty return true? Start with the exact predicate. Explain both conditions and what happens if the fields disagree, and explain what makes the result false.",
            vec![],
        ),
        _ => panic!("unknown Fontdone case"),
    };
    let manifest = snapshot::load_snapshot_manifest(
        &storage,
        "/Users/lazytrot/work/fontdone",
        "snap-1792140879637485000-11155-0",
    )
    .unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    manager.resume_connection(&storage).unwrap();
    let runtime = manager.review_runtime(&storage).unwrap();
    let grant_id = workspace::save_retrieval_grant(
        &storage,
        &manifest.summary.project_id,
        &manifest.summary.snapshot_id,
        &runtime,
    )
    .unwrap();
    let input = Input {
        run_id: Uuid::new_v4().to_string(),
        project_id: manifest.summary.project_id.clone(),
        snapshot_id: manifest.summary.snapshot_id.clone(),
        grant_id,
        question: question.into(),
        references,
        model_id: "gpt-6-luna".into(),
        effort: "max".into(),
    };
    let started = Instant::now();
    let phases = Mutex::new(Vec::new());
    let result = run(
        &storage,
        input.clone(),
        &manager,
        RepositoryContextJobs::default(),
        &chapters::PendingChapterDrafts::default(),
        &recovery::InterruptionJournal::default(),
        |p| {
            phases
                .lock()
                .unwrap()
                .push(json!({"phase":p.phase,"elapsedMs":started.elapsed().as_millis()}));
            println!(
                "FONTDONE_AGENT_PHASE {} {}",
                p.phase,
                started.elapsed().as_millis()
            );
        },
    );
    let ledger = crate::evidence_ledger::inspect(&storage, &input.run_id)
        .map(|r| serde_json::to_value(r).unwrap())
        .unwrap_or(Value::Null);
    let artifact = json!({"case":case,"question":question,"projectId":input.project_id,"snapshotId":input.snapshot_id,"runId":input.run_id,"model":"gpt-6-luna","effort":"max","elapsedMs":started.elapsed().as_millis(),"phases":phases.into_inner().unwrap(),"status":ledger["status"],"calls":ledger["calls"],"evidence":ledger["evidence"],"preview":result.as_ref().ok(),"error":result.as_ref().err()});
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../docs/acceptance/artifacts/2026-10-17-fontdone-agent-{case}.json"
    ));
    std::fs::write(&path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
    println!("FONTDONE_AGENT_ARTIFACT {}", path.display());
    manager.shutdown();
    let preview = result.unwrap();
    assert!(!preview.revision.sections.is_empty());
    assert!(
        preview
            .revision
            .generation_origin
            .as_ref()
            .unwrap()
            .retrieval
            .is_some()
    );
    if case == "broad" || case == "memory-face" {
        assert!(
            preview
                .revision
                .sections
                .iter()
                .flat_map(|s| &s.claims)
                .flat_map(|c| &c.anchors)
                .any(|a| a.path == "src/font.rs"),
            "the core large implementation file was never cited"
        );
    }
}
