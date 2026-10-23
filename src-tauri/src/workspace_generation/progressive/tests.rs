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
        renderer_queue_wait_ms: None,
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
        json!(["answerManifestV2"])
    );
    assert!(schema["properties"].get("citationVersion").is_none());
    assert!(
        schema["properties"]["sections"]["items"]["properties"]
            .get("claims")
            .is_none()
    );
    input.question.push_str(" Include failure paths.");
    assert_ne!(request(&input, &f.manifest).unwrap().2, digest);
    input.model_id = "gpt-5.6-luna".into();
    input.effort = "medium".into();
    assert_ne!(request(&input, &f.manifest).unwrap().2, digest);
}

#[test]
fn timing_report_names_the_first_native_validated_publication_by_section_order() {
    let publications = vec![
        PublicationTiming {
            generation_id: "generation:later".into(),
            order: 1,
            candidate_prepared_context_elapsed_ms: 90,
            native_ready_run_elapsed_ms: Some(105),
        },
        PublicationTiming {
            generation_id: "generation:first".into(),
            order: 0,
            candidate_prepared_context_elapsed_ms: 42,
            native_ready_run_elapsed_ms: None,
        },
    ];
    let first = first_useful_answer(&publications).unwrap();
    let report = TimingReport {
        schema_version: 3,
        kind: "code-atlas-generation-timing",
        scope: "synthetic scope",
        run_id: "synthetic-run".into(),
        answer_outcome: Outcome::Partial,
        worker_active: false,
        trace: None,
        first_agent_message_delta_elapsed_ms: Some(12),
        first_useful_answer: Some(first),
        publications,
        unmeasured: [
            "repositoryGuidePlanningDuration",
            "connectionPreparationBeforeManager",
            "rendererAdoption",
            "narrationReadiness",
            "providerWaitVersusModelReasoning",
            "firstModelToken",
        ],
    };
    let value = serde_json::to_value(report).unwrap();
    assert_eq!(value["schemaVersion"], 3);
    assert_eq!(value["firstAgentMessageDeltaElapsedMs"], 12);
    assert_eq!(
        value["firstUsefulAnswer"]["generationId"],
        "generation:first"
    );
    assert_eq!(value["firstUsefulAnswer"]["order"], 0);
    assert_eq!(
        value["firstUsefulAnswer"]["candidatePreparedContextElapsedMs"],
        42
    );
    // Old or incomplete timing observations remain unknown; they are not
    // reconstructed from wall-clock timestamps or later publications.
    assert!(value["firstUsefulAnswer"]["nativeReadyRunElapsedMs"].is_null());
    assert!(
        value["unmeasured"]
            .as_array()
            .unwrap()
            .contains(&json!("firstModelToken"))
    );
    assert!(
        value["unmeasured"]
            .as_array()
            .unwrap()
            .contains(&json!("repositoryGuidePlanningDuration"))
    );
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

fn empty_saved_answer(f: &Fixture, input: &Input) -> std::path::PathBuf {
    use crate::{codex_identity::CodexReviewRuntime, evidence_ledger::Ledger};
    use std::sync::atomic::AtomicBool;
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
    let ledger = Ledger::new_progressive(
        &f.storage,
        &input.run_id,
        &f.manifest,
        &grant,
        &runtime,
        &input.model_id,
        &input.effort,
    )
    .unwrap();
    ledger
        .start_narration_answer_sections(
            &input.question,
            &"d".repeat(64),
            &f.manifest,
            &runtime,
            &AtomicBool::new(false),
        )
        .unwrap();
    f.storage
        .join("answer-sections")
        .join(format!("{}.json", input.run_id))
}

#[test]
fn saved_answer_loading_keeps_active_workers_and_recovers_without_connecting_or_replaying() {
    let f = Fixture::new();
    let input = input(&f);
    let record_path = empty_saved_answer(&f, &input);
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
fn native_manifest_schema_leaves_all_published_identities_under_native_ownership() {
    // An actual Fontdone final response omitted a third published section.
    // A fresh agent reports unfinished topics; it never recopies owned IDs.
    let contract = schema();
    assert_eq!(
        contract["required"],
        json!(["manifestVersion", "incompleteTopics"])
    );
    assert_eq!(contract["properties"].as_object().unwrap().len(), 2);
    assert!(contract["properties"].get("sections").is_none());
    assert_eq!(contract["additionalProperties"], false);
    assert_eq!(
        contract["properties"]["manifestVersion"]["enum"],
        json!(["answerManifestV2"])
    );
    assert_eq!(contract["properties"]["incompleteTopics"]["maxItems"], 6);
    let instructions = system_instructions();
    assert!(instructions.contains("retains every latest published section automatically"));
    assert!(instructions.contains("Do not copy sectionKey or generationId"));
    assert!(!instructions.contains("sections lists the latest accepted"));
}

#[test]
fn slow_saved_answer_validation_does_not_block_stop_ownership() {
    use std::sync::mpsc;
    use std::time::Duration;
    let manager = codex_agent::CodexAgentManager::default();
    let run = Uuid::new_v4().to_string();
    let (release, wait) = mpsc::channel();
    let (cancelled, observed) = mpsc::channel();
    let stopping = manager.clone();
    let stop_run = run.clone();
    let stop = std::thread::spawn(move || {
        wait.recv_timeout(Duration::from_secs(3)).unwrap();
        cancelled.send(stopping.cancel(&stop_run)).unwrap();
    });
    let mut stopped_during_read = false;
    let result = load_saved_answer_with_reader(Path::new("unused"), &run, &manager, || {
        release.send(()).unwrap();
        stopped_during_read = observed
            .recv_timeout(Duration::from_secs(1))
            .unwrap_or(false);
        Err("A controlled slow read failed without changing saved data.".into())
    });
    stop.join().unwrap();
    assert!(result.is_err());
    assert!(
        stopped_during_read,
        "Stop must acquire ownership while source validation is pending"
    );
}

#[test]
fn saved_answer_rereads_changed_receipts_locally_with_a_fixed_retry_bound() {
    let f = Fixture::new();
    let input = input(&f);
    let path = empty_saved_answer(&f, &input);
    let old = reading::observe(
        &f.storage,
        &input.run_id,
        &input.project_id,
        &input.snapshot_id,
    )
    .unwrap();
    reading::interrupt(&f.storage, &input.run_id, Outcome::Stopped).unwrap();
    let stopped = std::fs::read(&path).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    let mut reads = 0;
    let stale = load_saved_answer_with_reader(&f.storage, &input.run_id, &manager, || {
        reads += 1;
        Ok(old.clone())
    });
    assert_eq!(reads, 3);
    assert!(matches!(stale, Err(message) if message.contains("nothing was regenerated")));
    assert_eq!(std::fs::read(&path).unwrap(), stopped);
    let mut reads = 0;
    let current = load_saved_answer_with_reader(&f.storage, &input.run_id, &manager, || {
        reads += 1;
        if reads == 1 {
            Ok(old.clone())
        } else {
            reading::observe(
                &f.storage,
                &input.run_id,
                &input.project_id,
                &input.snapshot_id,
            )
        }
    })
    .unwrap();
    assert_eq!(reads, 2);
    assert_eq!(current.outcome, Outcome::Stopped);
    assert_eq!(std::fs::read(&path).unwrap(), stopped);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
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
    let timing = timing_report(
        &f.storage,
        &input.run_id,
        &input.project_id,
        &input.snapshot_id,
        false,
    );
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
    for forbidden in [
        "accountIdentitySha256",
        "executablePath",
        "Hello, ${name}",
        "referenceId",
        "excerpt",
    ] {
        assert!(!timing_text.contains(forbidden));
    }
    let result = result.unwrap();
    assert_eq!(result.outcome, Outcome::Complete);
    assert!(!result.sections.is_empty());
    let saved = sections::inspect(&f.storage, &input.run_id).unwrap();
    assert!(
        saved
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
                    .is_some_and(|presentation| presentation.scenes.iter().any(|scene| scene
                        .mode
                        == crate::scene_presentation::SceneMode::Composition))))
    );
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
        references: vec![], model_id: "gpt-6-luna".into(), effort: "max".into(), renderer_queue_wait_ms: None,
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
    assert!(instructions.contains("Teaching policy v27:"));
    assert!(instructions.contains("visible cue"));
    assert!(instructions.contains("Narration tells the subject's journey in direct speech"));
    assert!(instructions.starts_with(ANSWER_ROLE));
    assert!(instructions.contains("then investigate and publish deeper sections"));
    assert!(!instructions.contains("then focus on the remaining research outline"));
    assert!(
        instructions.contains(
            "A comment, import, identifier or default constructor does not prove later consumer behavior"
        )
    );
    assert!(instructions.contains("Trace the guards and failures"));
    assert!(!instructions.contains("parser fallback starts after those checks"));
    assert!(instructions.contains("Start with the real input or situation"));
    assert!(instructions.contains("show code only when syntax is the lesson"));
    assert!(instructions.contains(
        "Identify the real thing the reader should watch: input, decision, state/data change, or output"
    ));
    assert!(instructions.contains("verified order → timeline"));
    assert!(
        instructions.contains("Use a graph only for relations those forms cannot show clearly")
    );
    assert!(instructions.contains("one app-supported representation per scene"));
    assert!(instructions.contains("Across the section use 2–12 nodes and at most 20 edges"));
    assert!(instructions.contains("The app's typed renderer owns safe HTML and source links"));
    assert!(instructions.contains("Pair every alternative with its actual guard"));
    assert!(instructions.contains("Point to a visible cue only as a brief landmark"));
    assert!(instructions.contains("State the decision rule before examples"));
    assert!(instructions.contains("explain every outcome"));
    assert!(!instructions.contains("The separate prose/spokenText fields remain required"));
    assert!(
        instructions.contains(
            "sceneAuthoringReference identifies a duplicate, absent or ambiguous reference"
        )
    );
    assert!(
        instructions.contains("preserving the claim, supported behavior and chosen representation")
    );
    assert!(instructions.contains(
        "sceneSourceValue must occur exactly once in that node's same-claim cited excerpt"
    ));
    assert!(instructions.contains("fix from delivered evidence, never borrow or relabel a copy"));
}

#[test]
fn teaching_policy_is_bound_to_the_new_request_without_changing_retrieval_limits() {
    let f = Fixture::new();
    let input = input(&f);
    let (prompt, schema, fingerprint) = request(&input, &f.manifest).unwrap();
    let contract = generation_contract(&input, &prompt, &schema).unwrap();
    assert_eq!(
        contract["storyPolicy"],
        provider::visual_scenes::STORY_POLICY_VERSION
    );
    assert_eq!(contract["teachingPolicy"], "meaning-first-v27");
    assert_eq!(contract["systemInstructions"], system_instructions());
    assert!(system_instructions().contains(
        "If no visual helps the section, choose visual.kind:\"none\" and give every claim one mode:\"narration\" scene"
    ));
    assert!(system_instructions().contains(
        "Otherwise give every claim a nonempty visual scene with at least one source-linked node"
    ));
    assert!(system_instructions().contains("Never mix narration-only and visual scenes"));
    assert!(
        system_instructions().contains("Selector is the root for first-match/priority/fallback")
    );
    assert!(system_instructions().contains(
        "selectorShape means the root selector has 2+ source-ordered direct branch groups"
    ));
    assert!(system_instructions().contains("Never combine guard and outcome in one card"));
    assert!(system_instructions().contains("Visual design method v33:"));
    assert!(system_instructions().contains("VISUAL STORY CONTRACT — ai-html-compositions-v35"));
    assert!(system_instructions().contains("each branch's parentKey names the selector key"));
    assert!(
        system_instructions().contains("each guard/outcome leaf's parentKey names its branch key")
    );
    assert!(!system_instructions().contains("presentation:{schemaVersion:3"));
    assert!(!system_instructions().contains("Elements have exactly kind,parentIndex,nodeIndex"));
    assert!(system_instructions().contains("under 45 visible characters per card"));
    assert!(system_instructions().contains("Identify the real thing the reader should watch"));
    assert!(
        system_instructions()
            .contains("Follow a real input through source-grounded changes and outcomes")
    );
    assert!(system_instructions().contains(
        "Identify the real thing the reader should watch: input, decision, state/data change, or output"
    ));
    assert!(
        system_instructions().contains("Across the section use 2–12 nodes and at most 20 edges")
    );
    assert!(system_instructions().contains("never a LIFO stack"));
    assert_eq!(
        fingerprint,
        context::digest(&serde_json::to_vec(&contract).unwrap())
    );
    assert_eq!(contract["sectionAttempts"], 8);
    assert_eq!(contract["uniqueSections"], 6);
    assert_eq!(contract["runDeadlinePolicy"], "user-controlled-stop-v1");
    assert!(
        contract["systemInstructions"]
            .as_str()
            .unwrap()
            .contains("No wall-clock deadline; null budget.remaining.timeMs is intentional")
    );
    assert!(
        !contract["systemInstructions"]
            .as_str()
            .unwrap()
            .contains("ten-minute wall-clock deadline")
    );
    assert_eq!(contract["checkpointVersion"], 8);
    assert_eq!(contract["answerSchemaVersion"], 7);
    assert_eq!(contract["finalManifestVersion"], "answerManifestV2");
    assert_eq!(
        contract["finalManifestPolicy"],
        "all-latest-published-sections-v1"
    );
    assert_eq!(contract["proposalVersion"], "answerSectionV6");
    assert_eq!(
        contract["sceneAuthoringPolicy"],
        "claim-owned-scene-local-identities-v3"
    );
    assert!(system_instructions().contains("at most eight nodes"));
    assert!(system_instructions().contains("one edge of context"));
    assert_eq!(
        contract["identifierCompilationPolicy"],
        "explicit-prose-grounded-suffixes-v2"
    );
    assert_eq!(
        contract["publicationOrderPolicy"],
        "investigation-local-next-order-v1"
    );
    assert!(system_instructions().contains("proposalVersion:\"answerSectionV6\""));
    assert!(!system_instructions().contains("every marked name must occur"));
    assert!(system_instructions().contains("Start its first new key at order:0"));
    assert!(system_instructions().contains("nextNewSectionOrder"));
    assert_eq!(
        contract["contractVersion"],
        crate::evidence_ledger::PROGRESSIVE_CONTRACT
    );
    assert_eq!(contract["policy"], crate::evidence_ledger::POLICY);
    assert_eq!(
        contract["tools"],
        serde_json::to_value(context_mcp::tools()).unwrap()
    );
    assert_eq!(contract["outputSchema"], schema);
    let mut without_teaching_policy = contract.clone();
    without_teaching_policy
        .as_object_mut()
        .unwrap()
        .remove("teachingPolicy");
    without_teaching_policy["systemInstructions"] = json!(INSTRUCTIONS);
    assert_ne!(
        fingerprint,
        context::digest(&serde_json::to_vec(&without_teaching_policy).unwrap())
    );
    let contract_bytes = serde_json::to_vec(&contract).unwrap().len();
    assert!(
        contract_bytes <= INITIAL_REQUEST_LIMIT_BYTES,
        "contract is {contract_bytes} bytes"
    );
}

#[test]
fn guide_progressive_contract_has_no_wall_clock_deadline() {
    let f = Fixture::new();
    let input = input(&f);
    let prompt = question_prompt(&input, &f.manifest).unwrap();
    let contract = generation_contract_for(&input, &prompt, &schema(), true).unwrap();
    let instructions = system_instructions_for(true);
    assert_eq!(contract["runDeadlinePolicy"], "user-controlled-stop-v1");
    assert_eq!(contract["uniqueSections"], 1);
    assert!(instructions.contains("no wall-clock deadline"));
    assert!(instructions.contains("null budget.remaining.timeMs is intentional"));
    assert!(instructions.contains("This operation owns exactly one planned slide"));
    assert!(instructions.contains("a final manifest cannot replace a slide"));
    assert!(instructions.contains("Do not finish with an incomplete-topic notice"));
    assert!(
        contract["tools"]
            .to_string()
            .contains("Do not return a final manifest as a substitute for an unsubmitted slide")
    );
    assert!(instructions.contains("make no further tool calls"));
    assert!(instructions.contains("the next slide is a separate operation"));
    assert!(!instructions.contains("ten-minute wall-clock deadline"));
    assert_eq!(contract["systemInstructions"], instructions);
}

#[test]
fn every_workspace_code_investigation_uses_a_user_controlled_runtime() {
    let f = Fixture::new();
    let input = input(&f);
    let prompt = question_prompt(&input, &f.manifest).unwrap();
    let contract = generation_contract(&input, &prompt, &schema()).unwrap();
    assert_eq!(contract["runDeadlinePolicy"], "user-controlled-stop-v1");
}

#[test]
fn progressive_story_rules_cover_representation_and_fallible_result_meaning() {
    let instructions = system_instructions();
    assert!(instructions.contains(
        "Identify the real thing the reader should watch: input, decision, state/data change, or output"
    ));
    assert!(instructions.contains("changed data → beforeAfter/transform"));
    assert!(instructions.contains("not language, file type or a favorite template"));
    assert!(instructions.contains("Selecting a handler does not mean its work succeeds"));
    assert!(instructions.contains("Cross-check claim, narration, labels and values"));
    assert!(instructions.contains("Show every supported outcome with its guard and destination"));
    assert!(!instructions.contains("spokenText copies the complete scene narration"));
}

#[test]
fn narration_pacing_preserves_all_conditions_and_is_bound_to_the_request() {
    let f = Fixture::new();
    let input = input(&f);
    let (prompt, schema, fingerprint) = request(&input, &f.manifest).unwrap();
    let mut contract = generation_contract(&input, &prompt, &schema).unwrap();
    let instructions = system_instructions();
    assert!(instructions.contains("One consequential decision or transition per claim/scene"));
    assert!(
        instructions
            .contains("two to four claims/scenes and target at most eight distinct evidence nodes")
    );
    assert!(instructions.contains("at most eight nodes"));
    assert!(instructions.contains("Split stacked guards, broad stages and dense branch sets into consecutive slides at handoffs"));
    assert!(instructions.contains("Preserve all guards, outcomes and edges"));
    assert!(instructions.contains("Never claim measured timings"));
    assert!(instructions.contains("State the decision rule before examples"));
    assert!(instructions.contains("explain every outcome"));
    assert_eq!(contract["systemInstructions"], instructions);
    assert_eq!(
        fingerprint,
        context::digest(&serde_json::to_vec(&contract).unwrap())
    );
    contract["systemInstructions"] =
        json!(instructions.replace(&format!("\n\n{SCENE_PACING_INSTRUCTIONS}"), ""));
    assert_ne!(
        fingerprint,
        context::digest(&serde_json::to_vec(&contract).unwrap())
    );
    assert_eq!(contract["sectionAttempts"], 8);
    assert_eq!(contract["uniqueSections"], 6);
}

#[test]
fn progressive_teaching_rules_fit_the_existing_request_metadata_limit() {
    let f = Fixture::new();
    let input = input(&f);
    let prompt = question_prompt(&input, &f.manifest).unwrap();
    let contract = generation_contract(&input, &prompt, &schema()).unwrap();
    let bytes = serde_json::to_vec(&contract).unwrap();
    assert!(
        bytes.len() <= INITIAL_REQUEST_LIMIT_BYTES,
        "request metadata is {} bytes",
        bytes.len()
    );
}
