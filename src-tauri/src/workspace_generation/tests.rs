use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH};

// libtest cannot serve the application's MCP subcommand. Fail before opening
// an account/session or reserving a measurement artifact when a live probe
// omitted its built application helper.
fn require_live_context_helper() {
    let path = std::path::PathBuf::from(std::env::var_os("CODE_ATLAS_CONTEXT_HELPER_BIN")
        .expect("set CODE_ATLAS_CONTEXT_HELPER_BIN to the built Code Atlas executable"));
    assert!(path.is_absolute() && path.is_file(), "the live probe requires an absolute built application executable path");
    assert_ne!(std::fs::canonicalize(&path).unwrap(), std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap(), "the Rust test executable cannot serve the MCP helper");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(path.metadata().unwrap().permissions().mode() & 0o111, 0, "the live context helper is not executable");
    }
}

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
    assert_eq!(schema["properties"]["citationVersion"]["enum"],json!([crate::native_citations::VERSION]));
    let evidence=&schema["properties"]["sections"]["items"]["properties"]["claims"]["items"]["properties"]["evidence"]["items"];
    assert_eq!(evidence["additionalProperties"],false);
    assert_eq!(evidence["required"],json!(["referenceId"]));
    assert!(evidence["properties"].get("quote").is_none());
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
    require_live_context_helper();
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
#[ignore = "Existing-account GPT-6 Luna/max synthetic correction; consumes account quota."]
fn codex_owned_correction_probe() {
    require_live_context_helper();
    use std::sync::atomic::{AtomicUsize, Ordering};
    let f = Fixture::new();
    let manager = codex_agent::CodexAgentManager::default();
    let result = (|| -> Result<(), String> {
        manager.resume_connection(&f.storage)?;
        let runtime = manager.review_runtime(&f.storage)?;
        let operation = Uuid::new_v4().to_string();
        let grant = workspace::save_retrieval_grant(
            &f.storage,
            &f.manifest.summary.project_id,
            &f.manifest.summary.snapshot_id,
            &runtime,
        )?;
        let session = Arc::new(Mutex::new(agent_context::Session::new(
            &f.storage,
            &operation,
            f.manifest.clone(),
            &runtime,
            &grant,
            RepositoryContextJobs::default(),
        )?));
        let file = f
            .manifest
            .files
            .iter()
            .find(|file| file.path == SIMPLE_PATH)
            .unwrap();
        let prompt = json!({"question":"How does greeting format its result?",
            "startingPoint":{"path":SIMPLE_PATH,"sourceSha256":file.sha256,
                "startByte":0,"endByte":file.size_bytes}})
        .to_string();
        assert!(!prompt.contains("export function"));
        let fingerprint = context::digest(prompt.as_bytes());
        let corrections = AtomicUsize::new(0);
        let started = Instant::now();
        let outcome = manager.run_with_result_and_publish(&f.storage, codex_agent::CodexRunInput {
            run_id:operation.clone(), expected_runtime:runtime.clone(), model:"gpt-6-luna".into(),
            effort:"max".into(), approval_policy:"never".into(),
            system_instruction:concat!("This is an isolated synthetic integration test. First read the complete startingPoint range using read_code. ",
                "Return one section with one codeBehavior/responsibility claim and visual kind none. Use a specific 8-120 character chapter title, ",
                "a section title of at most 120 bytes, substantive section prose of 20-4000 bytes and non-empty spokenText of 1-1000 bytes. ",
                "Use inferenceBasis empty and relationshipIds empty. Set citationVersion nativeEvidenceV1 and cite a citeable:true fragment using {referenceId:fragment.id}. Do not copy source quotes. ",
                "The claim's identifiers must be [\"greeting\"]. For the FIRST response only, intentionally omit the literal identifier greeting from that claim's prose; ",
                "use this prose exactly: The helper formats the supplied name into a Hello message. ",
                "Do not correct that deliberate mismatch in the first turn. If a SECOND turn requests correction, obey it and explain the read implementation accurately ",
                "using only source already delivered. Do not call tools in the second turn. No extra claims, sections, source, commands, or network.").into(),
            user_prompt:prompt, output_schema:provider::workspace_retrieval_schema(),
            repair_claim_alignment:true, retrieval:Some(Arc::clone(&session)), sources:vec![],
        }, &fingerprint, started, |event| {
            if event.message == "Refining the explanation against its cited code." {
                corrections.fetch_add(1, Ordering::Relaxed);
            }
            println!("OWNED_CORRECTION_PHASE {}", event.phase);
        }, |response, access| -> Result<chapters::ChapterRevision, String> {
            access.check_review(&operation, &fingerprint, &runtime.executable_path)?;
            let mut owned = session.lock().map_err(|_| "Synthetic context lock failed.")?;
            owned.check_complete()?;
            let sources = owned.ledger.validation_sources(&f.manifest, access.runtime(), access.cancelled())?;
            let resolved=match owned.ledger.resolve_citations(&response.response_text,&f.manifest,access.runtime(),access.cancelled())? {
                crate::native_citations::Resolution::Ready(resolved)=>resolved,
                crate::native_citations::Resolution::Invalid(_)=>return Err("Synthetic references are unavailable.".into()),
            };
            let revision = chapter_generation::canonicalize_cited_workspace_overview(&f.manifest.summary.project_id,
                &f.manifest, Some(&f.index), &sources, &resolved)?;
            owned.ledger.verify_revision(&revision, &f.manifest, access.runtime(), access.cancelled())?;
            let validation = chapters::validate_chapter_revision(&f.storage, &f.manifest, Some(&f.index), &revision);
            if validation.status != chapters::ValidationReportStatus::Valid {
                return Err("Corrected synthetic explanation failed independent validation.".into());
            }
            owned.ledger.complete(&f.manifest, access.runtime(), access.cancelled())?;
            Ok(revision)
        }, |validated:Result<chapters::ChapterRevision,String>| {
            if let Err(reason) = &validated {
                session.lock().map_err(|_| "Synthetic context lock failed.")?
                    .preserve_rejection(&fingerprint, reason)?;
            }
            validated
        });
        // Preserve failed synthetic output before Fixture cleans its temporary
        // storage. Normal diagnostics never export private source/candidates.
        let error = match &outcome {
            Err(error) => Some(error.clone()),
            Ok(Err(error)) => Some(error.clone()),
            _ => None,
        };
        let recovered = crate::evidence_ledger::attempts::inspect(&f.storage, &operation).ok();
        let record = crate::evidence_ledger::inspect(&f.storage, &operation)
            .ok()
            .map(|record| serde_json::to_value(record).unwrap());
        let artifact = json!({"fixture":"public synthetic greeting; not user source", "operationId":operation,
            "model":"gpt-6-luna","effort":"max","elapsedMs":started.elapsed().as_millis(),
            "corrections":corrections.load(Ordering::Relaxed),"error":error,
            "candidates":recovered.map(|saved|saved.candidates),"calls":record.as_ref().map(|r|&r["calls"]),
            "revision":outcome.as_ref().ok().and_then(|r|r.as_ref().ok())});
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../docs/acceptance/artifacts/2026-10-17-codex-owned-correction.json");
        std::fs::write(path, serde_json::to_vec_pretty(&artifact).unwrap()).unwrap();
        let _revision = outcome??;
        assert_eq!(
            corrections.load(Ordering::Relaxed),
            1,
            "must exercise exactly one correction"
        );
        let saved = crate::evidence_ledger::attempts::inspect(&f.storage, &operation)?;
        assert_eq!(saved.candidates.len(), 2);
        assert!(!saved.candidates[0].preflight_findings.as_ref().unwrap().is_empty());
        assert!(saved.candidates[1].preflight_findings.as_ref().unwrap().is_empty());
        assert_eq!(saved.candidates[0].thread_id, saved.candidates[1].thread_id);
        assert_ne!(saved.candidates[0].turn_id, saved.candidates[1].turn_id);
        let ledger =
            serde_json::to_value(crate::evidence_ledger::inspect(&f.storage, &operation)?).unwrap();
        assert_eq!(
            ledger["calls"].as_array().unwrap().len(),
            1,
            "correction cannot request new context"
        );
        assert_eq!(ledger["status"], "complete");
        println!(
            "OWNED_CORRECTION_RESULT {}",
            json!({"operationId":operation,"corrections":1,"sourceCalls":1})
        );
        Ok(())
    })();
    manager.shutdown();
    result.unwrap();
}

/// Explicitly authorized public-repository acceptance, never part of offline CI.
#[test]
#[ignore = "Existing-account GPT-6 Luna/max Fontdone investigation; consumes account quota and requires its saved snapshot."]
fn fontdone_codex_retrieval_probe() {
    require_live_context_helper();
    use std::io::Write;
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
    let tag = std::env::var("CODE_ATLAS_FONTDONE_ARTIFACT_TAG").unwrap_or_default();
    assert!(
        tag.len() <= 48
            && tag
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
        "use a short lowercase artifact tag"
    );
    let suffix = if tag.is_empty() {
        String::new()
    } else {
        format!("-{tag}")
    };
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../docs/acceptance/artifacts/2026-10-17-fontdone-agent-{case}{suffix}.json"
    ));
    // Reserve before using account quota. An existing observation is immutable,
    // even if the later request fails or another probe starts concurrently.
    let mut artifact_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("artifact exists: choose a fresh CODE_ATLAS_FONTDONE_ARTIFACT_TAG");
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
    // This explicit public-source acceptance exports failed candidates for
    // review. Production diagnostics remain source-free.
    let rejected = crate::evidence_ledger::attempts::inspect(&storage, &input.run_id)
        .ok()
        .map(|saved| saved.candidates);
    let artifact = json!({"case":case,"question":question,"projectId":input.project_id,"snapshotId":input.snapshot_id,"runId":input.run_id,"model":"gpt-6-luna","effort":"max","elapsedMs":started.elapsed().as_millis(),"phases":phases.into_inner().unwrap(),"status":ledger["status"],"contextClock":ledger["contextClock"],"calls":ledger["calls"],"evidence":ledger["evidence"],"rejectedCandidates":rejected,"preview":result.as_ref().ok(),"error":result.as_ref().err()});
    artifact_file
        .write_all(&serde_json::to_vec_pretty(&artifact).unwrap())
        .unwrap();
    artifact_file.sync_all().unwrap();
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
