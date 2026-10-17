use super::*;
use crate::chapter_generation::tests::Fixture;

fn input(f: &Fixture) -> Input {
    Input {
        run_id: Uuid::new_v4().to_string(),
        project_id: f.manifest.summary.project_id.clone(),
        snapshot_id: f.manifest.summary.snapshot_id.clone(),
        grant_id: Uuid::new_v4().to_string(),
        question: "How does greeting create its result?".into(),
        references: vec![],
        model_id: "gpt-6-luna".into(),
        effort: "max".into(),
    }
}

#[test]
fn progressive_contract_is_small_binds_intent_and_never_requests_a_final_rewrite() {
    let f = Fixture::new();
    let mut input = input(&f);
    let (prompt, schema, digest) = request(&input, &f.manifest).unwrap();
    assert!(prompt.len() < 1000);
    assert!(!prompt.contains("export function greeting"));
    assert_eq!(
        schema["properties"]["manifestVersion"]["enum"],
        json!(["answerManifestV1"])
    );
    assert!(schema["properties"].get("citationVersion").is_none());
    assert!(schema["properties"]["sections"]["items"]["properties"]
        .get("claims")
        .is_none());
    assert_ne!(
        digest,
        super::super::request(&input, &f.manifest).unwrap().2
    );
    input.question.push_str(" Include failure paths.");
    assert_ne!(request(&input, &f.manifest).unwrap().2, digest);
    input.model_id = "gpt-5.6-luna".into();
    assert!(request(&input, &f.manifest).is_err());
}

#[test]
fn native_worker_registration_prevents_false_restart_interruption() {
    let manager = codex_agent::CodexAgentManager::default();
    let run = Uuid::new_v4().to_string();
    assert!(!manager.with_operation_state(&run, Ok).unwrap());
    let worker = manager.hold_answer_worker(&run).unwrap();
    assert!(manager.with_operation_state(&run, Ok).unwrap());
    assert!(manager.hold_answer_worker(&run).is_err());
    drop(worker);
    assert!(!manager.with_operation_state(&run, Ok).unwrap());
}

#[test]
fn saved_answer_loading_keeps_active_workers_and_recovers_without_connecting_or_replaying() {
    use crate::{codex_identity::CodexReviewRuntime, evidence_ledger::Ledger};
    use std::sync::atomic::AtomicBool;
    let f = Fixture::new();
    let input = input(&f);
    let runtime = CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 1,
    };
    let grant = workspace::save_retrieval_grant(
        &f.storage,
        &input.project_id,
        &input.snapshot_id,
        &runtime,
    )
    .unwrap();
    let ledger =
        Ledger::new_progressive(&f.storage, &input.run_id, &f.manifest, &grant, &runtime).unwrap();
    ledger
        .start_answer_sections(
            &input.question,
            &"d".repeat(64),
            &f.manifest,
            &runtime,
            &AtomicBool::new(false),
        )
        .unwrap();
    let record_path = f
        .storage
        .join("answer-sections")
        .join(format!("{}.json", input.run_id));
    let original = std::fs::read(&record_path).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    let worker = manager.hold_answer_worker(&input.run_id).unwrap();
    let active = load_saved_answer(
        &f.storage,
        &input.run_id,
        &input.project_id,
        &input.snapshot_id,
        &manager,
    )
    .unwrap();
    assert_eq!(active.outcome, Outcome::Working);
    assert!(active.sections.is_empty());
    assert_eq!(original, std::fs::read(&record_path).unwrap());
    assert!(
        load_saved_answer(
            &f.storage,
            &input.run_id,
            "another-project",
            &input.snapshot_id,
            &manager
        )
        .is_err()
    );
    assert_eq!(original, std::fs::read(&record_path).unwrap());
    drop(worker);
    let recovered = load_saved_answer(
        &f.storage,
        &input.run_id,
        &input.project_id,
        &input.snapshot_id,
        &manager,
    )
    .unwrap();
    assert_eq!(recovered.outcome, Outcome::Interrupted);
    assert!(recovered.sections.is_empty());
    let saved = std::fs::read(&record_path).unwrap();
    assert_ne!(original, saved);
    assert_eq!(
        load_saved_answer(
            &f.storage,
            &input.run_id,
            &input.project_id,
            &input.snapshot_id,
            &manager
        )
        .unwrap()
        .outcome,
        Outcome::Interrupted
    );
    assert_eq!(saved, std::fs::read(&record_path).unwrap());
    let listed = reading::list(&f.storage, &input.project_id, &input.snapshot_id).unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].outcome, Outcome::Interrupted);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}

#[test]
fn final_manifest_schema_preserves_the_entire_native_generation_identity() {
    // Native publication returns a prefixed UUID, not a bare 36-byte UUID.
    // A 36-character output constraint made real Codex truncate this identity
    // and omit the second published section from its terminal manifest.
    let generation_id = format!("generation:{}", Uuid::new_v4());
    let contract = schema();
    let bound = contract["properties"]["sections"]["items"]["properties"]["generationId"]
        ["maxLength"]
        .as_u64()
        .unwrap();
    assert_eq!(generation_id.len(), 47);
    assert_eq!(bound, generation_id.len() as u64);
}

#[test]
#[ignore = "Explicit existing-account GPT-6 Luna/max progressive synthetic answer; consumes quota and requires the rebuilt app helper."]
fn codex_progressive_account_probe() {
    use std::io::Write;
    super::super::tests::require_live_context_helper();
    let output = std::path::PathBuf::from(
        std::env::var_os("CODE_ATLAS_PROGRESSIVE_PROBE_OUTPUT")
            .expect("set a fresh absolute CODE_ATLAS_PROGRESSIVE_PROBE_OUTPUT artifact path"),
    );
    assert!(output.is_absolute());
    let mut artifact = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .unwrap();
    let f = Fixture::new();
    let mut input = input(&f);
    input.question = "Explain only the implementation of greeting, from the incoming name to its returned string. Use one short AI-authored HTML composition (visual.presentation), showing the transformation with a clearly labeled illustrative name and greeting; a dependency graph would not help here. Narrate what the visual means. The function body is the complete requested scope; unrequested call sites are outside this question.".into();
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
    let observations = Arc::new(Mutex::new(vec![]));
    let observed = Arc::clone(&observations);
    let storage = f.storage.clone();
    let project = input.project_id.clone();
    let snapshot = input.snapshot_id.clone();
    let started = Instant::now();
    let result = run(
        &f.storage,
        input.clone(),
        &manager,
        RepositoryContextJobs::default(),
        &recovery::InterruptionJournal::default(),
        |p| println!("PROGRESSIVE_PHASE {}", p.phase),
        Arc::new(move |notice| {
            let saved = reading::adopt(
                &storage,
                &notice.run_id,
                &project,
                &snapshot,
                &notice.generation_id,
            );
            observed.lock().unwrap().push(
                json!({"notice":notice,"readyElapsedMs":started.elapsed().as_millis(),
                "savedRevisionId":saved.as_ref().ok().map(|r|&r.revision_id),"error":saved.err()}),
            );
        }),
    );
    let answer = sections::inspect(&f.storage, &input.run_id)
        .ok()
        .map(|v| serde_json::to_value(v).unwrap());
    let ledger = crate::evidence_ledger::inspect(&f.storage, &input.run_id)
        .ok()
        .map(|v| serde_json::to_value(v).unwrap());
    let timing = timing_report(&f.storage, &input.run_id, &input.project_id, &input.snapshot_id, false);
    let record = json!({"fixture":"public synthetic greeting; not user source","operationId":input.run_id,
        "model":"gpt-6-luna","effort":"max","totalElapsedMs":started.elapsed().as_millis(),
        "result":result.as_ref().ok(),"error":result.as_ref().err(),"observations":*observations.lock().unwrap(),
        "answer":answer,"ledger":ledger,"timing":timing.as_ref().ok(),"timingError":timing.as_ref().err()});
    artifact
        .write_all(&serde_json::to_vec_pretty(&record).unwrap())
        .unwrap();
    artifact.sync_all().unwrap();
    manager.shutdown();
    let timing = serde_json::to_value(timing.unwrap()).unwrap();
    assert_eq!(timing["trace"]["terminal"]["state"], "returned");
    assert!(timing["trace"]["tokenUsage"].is_null());
    assert_eq!(timing["trace"]["modelId"], "gpt-6-luna");
    assert_eq!(timing["trace"]["reasoningEffort"], "max");
    assert!(!timing["trace"]["events"].as_array().unwrap().is_empty());
    let timing_text = timing.to_string();
    for forbidden in ["accountIdentitySha256", "executablePath", "Hello, ${name}", "referenceId", "excerpt"] {
        assert!(!timing_text.contains(forbidden));
    }
    let result = result.unwrap();
    assert_eq!(result.outcome, Outcome::Complete);
    assert!(!result.sections.is_empty());
    let saved = sections::inspect(&f.storage, &input.run_id).unwrap();
    assert!(saved
        .sections
        .iter()
        .any(|published| published
            .preview
            .revision
            .sections
            .iter()
            .any(|section| section
                .visual
                .as_ref()
                .and_then(chapters::ChapterVisualPayload::typed_visual)
                .and_then(|visual| visual.presentation.as_ref())
                .is_some_and(|presentation| presentation.scenes.iter().any(
                    |scene| scene.mode == crate::scene_presentation::SceneMode::Composition
                )))));
    let observations = observations.lock().unwrap();
    assert!(observations.iter().all(|v| v["error"].is_null()));
    assert!(
        observations.first().unwrap()["readyElapsedMs"]
            .as_u64()
            .unwrap()
            <= 120_000
    );
}

#[test]
#[ignore = "Explicit GPT-6 Luna/max frozen Fontdone broad-answer qualification; consumes existing account quota."]
fn fontdone_progressive_account_probe() {
    use std::io::Write;
    super::super::tests::require_live_context_helper();
    let storage = std::path::PathBuf::from(
        std::env::var_os("CODE_ATLAS_FONTDONE_DATA_DIR")
            .expect("set isolated CODE_ATLAS_FONTDONE_DATA_DIR"),
    );
    let output = std::path::PathBuf::from(
        std::env::var_os("CODE_ATLAS_PROGRESSIVE_PROBE_OUTPUT")
            .expect("set fresh CODE_ATLAS_PROGRESSIVE_PROBE_OUTPUT"),
    );
    assert!(storage.is_absolute() && output.is_absolute());
    let mut artifact = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .expect("never overwrite an earlier attempt");
    let project = "/Users/lazytrot/work/fontdone";
    let snapshot_id = "snap-1792140879637485000-11155-0";
    let manifest = snapshot::load_snapshot_manifest(&storage, project, snapshot_id).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    manager.resume_connection(&storage).unwrap();
    let runtime = manager.review_runtime(&storage).unwrap();
    let grant_id =
        workspace::save_retrieval_grant(&storage, project, snapshot_id, &runtime).unwrap();
    let input = Input {
        run_id: Uuid::new_v4().to_string(), project_id: project.into(), snapshot_id: snapshot_id.into(), grant_id,
        question: "How does Fontdone turn font bytes into a rendered glyph bitmap? Trace the real entry points, parsing, outline loading, hinting, and rasterization. Explain the important branches and show useful visual scenes with source links.".into(),
        references: vec![], model_id: "gpt-6-luna".into(), effort: "max".into(),
    };
    let started = Instant::now();
    let observations = Arc::new(Mutex::new(vec![]));
    let observed = Arc::clone(&observations);
    let saved_storage = storage.clone();
    let result = run(
        &storage,
        input.clone(),
        &manager,
        RepositoryContextJobs::default(),
        &recovery::InterruptionJournal::default(),
        |p| {
            println!(
                "FONTDONE_PROGRESSIVE_PHASE {} {}",
                p.phase,
                started.elapsed().as_millis()
            )
        },
        Arc::new(move |notice| {
            let saved = reading::adopt(
                &saved_storage,
                &notice.run_id,
                project,
                snapshot_id,
                &notice.generation_id,
            );
            println!(
                "FONTDONE_SECTION {} {}",
                notice.order,
                started.elapsed().as_millis()
            );
            observed.lock().unwrap().push(
                json!({"notice":notice,"readyElapsedMs":started.elapsed().as_millis(),
                "savedRevisionId":saved.as_ref().ok().map(|r|&r.revision_id),"error":saved.err()}),
            );
        }),
    );
    let answer = sections::inspect(&storage, &input.run_id)
        .ok()
        .map(|a| serde_json::to_value(a).unwrap());
    let ledger = crate::evidence_ledger::inspect(&storage, &input.run_id)
        .ok()
        .map(|l| serde_json::to_value(l).unwrap());
    let record = json!({"fixture":"Frozen user-selected Fontdone snapshot; isolated app storage, no accepted chapter decisions",
        "sourceRevision":std::env::var("CODE_ATLAS_QUALIFICATION_REVISION").ok(),"operationId":input.run_id,
        "evidenceLedgerSourceSha256":context::digest(include_bytes!("../../evidence_ledger.rs")),
        "probeSourceSha256":context::digest(include_bytes!("tests.rs")),
        "implementationSourceSha256":{
            "nativeCitations":context::digest(include_bytes!("../../native_citations.rs")),
            "answerSections":context::digest(include_bytes!("../../evidence_ledger/sections.rs")),
            "scenePresentation":context::digest(include_bytes!("../../scene_presentation.rs")),
            "sceneQuality":context::digest(include_bytes!("../../scene_presentation/quality.rs")),
            "contextCatalog":context::digest(include_bytes!("../../context_mcp.rs")),
            "progressiveRequest":context::digest(include_bytes!("../progressive.rs")),
            "ownedContext":context::digest(include_bytes!("../../agent_context.rs"))
        },
        "model":"gpt-6-luna","effort":"max","question":input.question,
        "snapshotSummary":manifest.summary,"totalElapsedMs":started.elapsed().as_millis(),
        "result":result.as_ref().ok(),"error":result.as_ref().err(),"answer":answer,"ledger":ledger,
        "observations":*observations.lock().unwrap(),"humanQualityReview":false,"humanListeningPass":false});
    artifact
        .write_all(&serde_json::to_vec_pretty(&record).unwrap())
        .unwrap();
    artifact.sync_all().unwrap();
    manager.shutdown();
    assert!(
        observations
            .lock()
            .unwrap()
            .iter()
            .all(|o| o["error"].is_null()),
        "section adoption failed; inspect the preserved artifact"
    );
    assert_eq!(
        result.unwrap().outcome,
        Outcome::Complete,
        "a partial answer is not broad completion; inspect the preserved artifact"
    );
}

#[test]
fn progressive_story_uses_behavior_examples_and_distinct_outcomes_before_syntax() {
    let instructions = system_instructions();
    assert!(instructions.contains("Teaching policy v4:"));
    assert!(instructions.contains("open with the problem and a useful input-to-result example"));
    assert!(instructions.contains("use at most one code element in a scene"));
    assert!(instructions.contains("give each outcome its own accurately named node or table row"));
    assert!(instructions.contains("scene narration is a separate natural-language explanation"));
    assert!(instructions.contains("state the general decision rule"));
    assert!(instructions.contains("an example alone cannot replace the rule"));
    assert!(!instructions.contains("The separate prose/spokenText fields remain required"));
    assert!(instructions.contains("visualEvidenceDuplicate identifies the redundant entry"));
    assert!(instructions.contains("preserving the claim, supported behavior and chosen representation"));
}

#[test]
fn teaching_policy_is_bound_to_the_new_request_without_changing_retrieval_limits() {
    let f = Fixture::new();
    let input = input(&f);
    let (prompt, schema, fingerprint) = request(&input, &f.manifest).unwrap();
    let contract = generation_contract(&input, &prompt, &schema).unwrap();
    assert_eq!(contract["teachingPolicy"], "meaning-first-v4");
    assert_eq!(contract["systemInstructions"], system_instructions());
    assert_eq!(
        fingerprint,
        context::digest(&serde_json::to_vec(&contract).unwrap())
    );
    assert_eq!(contract["sectionAttempts"], 8);
    assert_eq!(contract["uniqueSections"], 6);
    assert_eq!(contract["checkpointVersion"], 4);
    assert_eq!(
        contract["contractVersion"],
        crate::evidence_ledger::PROGRESSIVE_CONTRACT
    );
    assert_eq!(contract["policy"], crate::evidence_ledger::POLICY);
    assert_eq!(
        contract["tools"],
        serde_json::to_value(context_mcp::tools_for(true)).unwrap()
    );
    assert_eq!(contract["outputSchema"], schema);
    let mut historical = contract.clone();
    historical.as_object_mut().unwrap().remove("teachingPolicy");
    historical["systemInstructions"] = json!(INSTRUCTIONS);
    assert_ne!(
        fingerprint,
        context::digest(&serde_json::to_vec(&historical).unwrap())
    );
    assert!(serde_json::to_vec(&contract).unwrap().len() < 32000);
}
