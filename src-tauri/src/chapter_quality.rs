//! Offline benchmark boundary. All supplied source is staged in owned storage;
//! this path never opens an account, invokes an agent, or reads a user repository.

use crate::{chapter_generation, chapters, codex_identity, index, provider, snapshot, strict_json};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use uuid::Uuid;

const FLAG: &str = "--chapter-quality-helper";
const MAX_INPUT_BYTES: usize = 2 * 1024 * 1024;
const PROJECT: &str = "chapter-quality-contract";
const SNAPSHOT: &str = "quality-contract-v1";
const FIXTURE_PATH: &str = "src/quality-contract.ts";
const FIXTURE_SOURCE: &str =
    "export function greeting(name: string): string {\n  return `Hello, ${name}`;\n}\n";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Profile {
    backend_id: String,
    model_id: String,
    reasoning_effort: Option<String>,
    approval_policy: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Source {
    path: String,
    sha256: String,
    text: String,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "camelCase", deny_unknown_fields)]
enum Request {
    Metadata {
        #[serde(rename = "protocolVersion")]
        protocol_version: u32,
        profile: Profile,
    },
    Validate {
        #[serde(rename = "protocolVersion")]
        protocol_version: u32,
        profile: Profile,
        #[serde(rename = "targetPath")]
        target_path: String,
        sources: Vec<Source>,
        proposal: String,
    },
}

/// Recognize the private entry point before initializing Tauri or its plugins.
pub(crate) fn run_if_requested() -> bool {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if !args.iter().any(|arg| arg.starts_with(FLAG)) {
        return false;
    }
    let result = if args == [FLAG] {
        let mut input = Vec::new();
        std::io::stdin()
            .take(MAX_INPUT_BYTES as u64 + 1)
            .read_to_end(&mut input)
            .map_err(|error| format!("Could not read quality input: {error}"))
            .and_then(|_| process(&input))
    } else {
        Err("The quality helper accepts exactly one private flag.".to_owned())
    };
    let response = match result {
        Ok(data) => {
            json!({"protocolVersion": 1, "nativeBuildSha256": build_hash(), "status": "ok", "data": data})
        }
        Err(message) => {
            json!({"protocolVersion": 1, "nativeBuildSha256": build_hash(), "status": "error", "message": message})
        }
    };
    if let Ok(bytes) = serde_json::to_vec(&response) {
        let _ = std::io::stdout().write_all(&bytes);
    }
    true
}

fn build_hash() -> &'static str {
    env!("CODE_ATLAS_CLAUDE_ADAPTER_SHA256")
}
fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn process(bytes: &[u8]) -> Result<Value, String> {
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("Quality input exceeds 2 MiB.".to_owned());
    }
    let value =
        strict_json::from_slice(bytes).map_err(|error| format!("Invalid quality JSON: {error}"))?;
    let request: Request = serde_json::from_value(value)
        .map_err(|error| format!("Invalid quality request: {error}"))?;
    match request {
        Request::Metadata {
            protocol_version: 1,
            profile,
        } => {
            let source = Source {
                path: FIXTURE_PATH.to_owned(),
                sha256: sha256(FIXTURE_SOURCE.as_bytes()),
                text: FIXTURE_SOURCE.to_owned(),
            };
            let fixture = Fixture::new(vec![source])?;
            let review = fixture.review(&profile, FIXTURE_PATH)?;
            let schema = response_schema(&review)?;
            let model_ids = if profile.backend_id == "codex" {
                vec!["gpt-6-luna".to_owned()]
            } else {
                provider::disclosure_for(&profile.backend_id)
                    .ok_or("Unknown direct API benchmark backend.")?
                    .models
                    .into_iter()
                    .map(|model| model.model_id)
                    .collect()
            };
            Ok(json!({
                "scope": "offline contract metadata; synthetic runtime identity; no model or account qualification",
                "backendId": profile.backend_id, "modelId": profile.model_id,
                "reasoningEffort": profile.reasoning_effort, "approvalPolicy": profile.approval_policy,
                "modelIds": model_ids, "promptVersion": provider::FILE_PROMPT_VERSION,
                "chapterSchemaVersion": chapters::CHAPTER_SCHEMA_VERSION,
                "responseSchemaVersion": 3,
                "systemInstruction": review.system_instruction, "userPrompt": review.user_prompt,
                "generationConfig": review.generation_config, "responseSchema": schema,
            }))
        }
        Request::Validate {
            protocol_version: 1,
            profile,
            target_path,
            sources,
            proposal,
        } => {
            // A bad corpus/profile is an infrastructure error, never a candidate failure.
            let fixture = Fixture::new(sources)?;
            let review = fixture.review(&profile, &target_path)?;
            let schema = response_schema(&review)?;
            let validator = jsonschema::validator_for(&schema)
                .map_err(|error| format!("Native schema could not compile: {error}"))?;
            let result = validate(&fixture, &review, &validator, &target_path, &proposal);
            Ok(match result {
                Ok(counts) => json!({"valid": true, "counts": counts}),
                Err(message) => json!({"valid": false, "message": message,
                    "diagnostics": crate::claim_alignment::fixed_source_chapter_diagnostics(&proposal,
                        fixture.sources.iter().map(|s| (s.path.as_str(), s.contents.as_str()))).unwrap_or_default()}),
            })
        }
        _ => Err("Unsupported quality protocol version.".to_owned()),
    }
}

fn response_schema(review: &provider::ProviderRequestReview) -> Result<Value, String> {
    let mut schema = review
        .generation_config
        .get("outputSchema")
        .or_else(|| review.generation_config.get("responseSchema"))
        .cloned()
        .ok_or("The native review has no structured response schema.")?;
    crate::provider_api::lowercase_schema_types(&mut schema);
    Ok(schema)
}

fn validate(
    fixture: &Fixture,
    review: &provider::ProviderRequestReview,
    validator: &jsonschema::Validator,
    target: &str,
    proposal: &str,
) -> Result<Value, String> {
    if proposal.len() > chapters::MAX_GENERATED_RESPONSE_BYTES {
        return Err("Candidate exceeds the 300 KiB generated response limit.".to_owned());
    }
    let value = strict_json::from_slice(proposal.as_bytes())
        .map_err(|error| format!("Candidate JSON: {error}"))?;
    // No network/file resolution features are compiled into jsonschema. The schema
    // comes only from the app's request builder, never the supplied candidate.
    validator
        .validate(&value)
        .map_err(|error| format!("Current response schema: {error}"))?;
    let allowed = review
        .selected_relationships
        .iter()
        .map(|relationship| relationship.relationship_id.clone())
        .collect::<HashSet<_>>();
    let mut revision = chapter_generation::canonicalize_file_chapter(
        PROJECT,
        &fixture.manifest,
        Some(&fixture.index),
        target,
        &fixture.sources,
        &allowed,
        proposal,
    )?;
    provider::visual_scenes::prepare_generated_sections(&mut revision, None)?;
    let report = chapters::validate_chapter_revision(
        &fixture.root,
        &fixture.manifest,
        Some(&fixture.index),
        &revision,
    );
    if report.status != chapters::ValidationReportStatus::Valid {
        return Err(format!(
            "Native source/scene validation: {}",
            serde_json::to_string(&report.issues).map_err(|error| error.to_string())?
        ));
    }
    let anchors: usize = revision
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .map(|claim| claim.anchors.len())
        .sum();
    Ok(json!({"sections": revision.sections.len(), "sourceAnchors": anchors}))
}

struct Fixture {
    root: PathBuf,
    manifest: snapshot::SnapshotManifest,
    index: index::SnapshotIndex,
    sources: Vec<snapshot::SnapshotSource>,
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl Fixture {
    fn new(supplied: Vec<Source>) -> Result<Self, String> {
        if !(1..=12).contains(&supplied.len())
            || supplied
                .iter()
                .map(|source| source.text.len())
                .sum::<usize>()
                > 250_000
        {
            return Err(
                "Quality source must contain 1–12 files and at most 250000 UTF-8 bytes.".to_owned(),
            );
        }
        let mut seen = HashSet::new();
        let mut files = Vec::new();
        let mut sources = Vec::new();
        for source in supplied {
            if source.path.is_empty()
                || source.path != source.path.trim()
                || source.path.len() > 1024
                || source.path.contains(['\\', ':'])
                || source.path.chars().any(char::is_control)
                || source
                    .path
                    .split('/')
                    .any(|part| part.is_empty() || part == "." || part == "..")
                || !seen.insert(source.path.clone())
                || source.text.contains('\0')
                || source.sha256 != sha256(source.text.as_bytes())
            {
                return Err("Quality source has a noncanonical path, duplicate, invalid text or changed hash.".to_owned());
            }
            let captured = snapshot::CapturedFile {
                path: source.path.clone(),
                kind: snapshot::InventoryPathKind::File,
                size_bytes: source.text.len() as u64,
                sha256: source.sha256.clone(),
                blob_path: format!(
                    "objects/sha256/{}/{}",
                    &source.sha256[..2],
                    &source.sha256[2..]
                ),
                content_kind: snapshot::CapturedContentKind::Text,
                secret_like: snapshot::looks_secret_like(&source.path, source.text.as_bytes()),
            };
            if crate::repository_context::exclusion(&captured).is_some() {
                return Err("Quality source includes an excluded or secret-like file.".to_owned());
            }
            sources.push(snapshot::SnapshotSource {
                snapshot_id: SNAPSHOT.to_owned(),
                path: source.path,
                sha256: source.sha256,
                size_bytes: captured.size_bytes,
                contents: source.text,
            });
            files.push(captured);
        }
        let summary = snapshot::SnapshotSummary {
            snapshot_id: SNAPSHOT.to_owned(),
            project_id: PROJECT.to_owned(),
            parent_snapshot_id: None,
            created_at_unix_ms: 0,
            branch: None,
            head: None,
            index_state: snapshot::GitChangeState::Clean,
            worktree_state: snapshot::GitChangeState::Clean,
            status: snapshot::CaptureStatus::Complete,
            files_captured: files.len(),
            paths_skipped: 0,
            total_bytes: files.iter().map(|file| file.size_bytes).sum(),
            paths_visited: files.len() as u64,
            policy_version: "quality-supplied-source-v1".to_owned(),
        };
        let manifest = snapshot::SnapshotManifest {
            schema_version: 2,
            summary,
            repository_root: "/quality-contract".to_owned(),
            selected_path: "/quality-contract".to_owned(),
            capture_limits: snapshot::CaptureLimits::default(),
            directories: Vec::new(),
            files,
            skipped: Vec::new(),
        };
        let source_map: HashMap<_, _> = sources
            .iter()
            .map(|source| (source.path.clone(), source.clone()))
            .collect();
        let native_index = index::build_snapshot_index_from_sources(
            &manifest,
            &source_map,
            &BTreeSet::new(),
            "quality-contract",
            &AtomicBool::new(false),
            |_| {},
        );
        let root = std::env::temp_dir().join(format!("code-atlas-quality-{}", Uuid::new_v4()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder
            .create(&root)
            .map_err(|error| format!("Could not create isolated quality storage: {error}"))?;
        let fixture = Self {
            root,
            manifest,
            index: native_index,
            sources,
        };
        fixture.write()?;
        Ok(fixture)
    }

    fn write(&self) -> Result<(), String> {
        let write = |path: PathBuf, bytes: Vec<u8>| -> Result<(), String> {
            let parent = path.parent().ok_or("Quality storage path has no parent.")?;
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            fs::write(path, bytes).map_err(|error| error.to_string())
        };
        for (file, source) in self.manifest.files.iter().zip(&self.sources) {
            write(
                self.root.join(&file.blob_path),
                source.contents.as_bytes().to_vec(),
            )?;
        }
        let directory = self.root.join("snapshots").join(sha256(PROJECT.as_bytes()));
        write(
            directory.join("catalog.json"),
            serde_json::to_vec(&json!({"schemaVersion": 1, "snapshots": [&self.manifest.summary]}))
                .map_err(|error| error.to_string())?,
        )?;
        write(
            directory.join("manifests").join(format!("{SNAPSHOT}.json")),
            serde_json::to_vec(&self.manifest).map_err(|error| error.to_string())?,
        )?;
        index::save_snapshot_index(&self.root, &self.index)
    }

    fn review(
        &self,
        profile: &Profile,
        target: &str,
    ) -> Result<provider::ProviderRequestReview, String> {
        if !self.sources.iter().any(|source| source.path == target) {
            return Err("Quality target is absent from supplied source.".to_owned());
        }
        let paths = self
            .sources
            .iter()
            .map(|source| source.path.clone())
            .collect();
        let pending = if profile.backend_id == "codex" {
            if profile.model_id != "gpt-6-luna"
                || profile.reasoning_effort.as_deref() != Some("max")
                || profile.approval_policy.as_deref() != Some("never")
            {
                return Err(
                    "The Codex benchmark profile requires GPT-6 Luna/max and approval never."
                        .to_owned(),
                );
            }
            // Fixed placeholders fingerprint the compiled contract. They do not
            // attest an installed executable, account, consent or actual run.
            let runtime = codex_identity::CodexReviewRuntime {
                connection_id: "00000000-0000-4000-8000-000000000001".to_owned(),
                executable_path: "/quality-contract/codex".to_owned(),
                executable_sha256: "a".repeat(64),
                account_identity_sha256: "b".repeat(64),
                account_revision: 1,
            };
            provider::build_codex_chapter_review(
                &self.root,
                &provider::BuildCodexReviewInput {
                    project_id: PROJECT.to_owned(),
                    snapshot_id: SNAPSHOT.to_owned(),
                    target_path: Some(target.to_owned()),
                    selected_paths: paths,
                    model_id: profile.model_id.clone(),
                    effort: "max".to_owned(),
                    approval_policy: "never".to_owned(),
                    section_update: None,
                    question: None,
                    focus: Vec::new(),
                    workspace_grant: None,
                },
                &runtime,
            )
        } else {
            if profile.reasoning_effort.is_some() || profile.approval_policy.is_some() {
                return Err("Direct API profile must leave agent settings null.".to_owned());
            }
            provider::build_request_review_for_provider(
                &self.root,
                &provider::BuildProviderReviewInput {
                    project_id: PROJECT.to_owned(),
                    snapshot_id: SNAPSHOT.to_owned(),
                    target_path: Some(target.to_owned()),
                    selected_paths: paths,
                    model_id: profile.model_id.clone(),
                    section_update: None,
                },
                &profile.backend_id,
            )
        }
        .map_err(|failure| failure.message)?;
        Ok(pending.review)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_generation::tests::{SIMPLE_PATH, SIMPLE_SOURCE};

    fn profile() -> Value {
        json!({"backendId":"codex","modelId":"gpt-6-luna","reasoningEffort":"max","approvalPolicy":"never"})
    }
    fn request(proposal: Value) -> Value {
        json!({"action":"validate","protocolVersion":1,"profile":profile(),"targetPath":SIMPLE_PATH,
            "sources":[{"path":SIMPLE_PATH,"text":SIMPLE_SOURCE,"sha256":sha256(SIMPLE_SOURCE.as_bytes())}],"proposal":proposal.to_string()})
    }
    fn run(request: &Value) -> Result<Value, String> {
        process(&serde_json::to_vec(request).unwrap())
    }

    #[test]
    fn current_composition_passes_the_same_native_preview_boundary() {
        let proposal = provider::visual_scenes::tests::composed_proposal(3);
        let data = run(&request(proposal)).unwrap();
        assert_eq!(data["valid"], true, "{data}");
        assert!(data["counts"]["sourceAnchors"].as_u64().unwrap() > 0);
    }

    #[test]
    fn failed_scene_candidates_expose_the_owned_correction_positions_without_admitting_content() {
        let mut proposal = provider::visual_scenes::tests::composed_proposal(3);
        proposal["sections"][0]["visual"]["nodes"][0]["evidence"][0]["claimIndex"] = json!(1);
        let data = run(&request(proposal)).unwrap();
        assert_eq!(data["valid"], false);
        assert_eq!(
            data["diagnostics"],
            json!([
                {"location":"/sections/0/visual/presentation/scenes/0/elements/1/nodeIndex","sceneProblem":"sceneEvidence"}
            ])
        );
        assert!(!data["diagnostics"].to_string().contains("Hello"));
    }

    #[test]
    fn malformed_future_foreign_and_downgraded_scenes_are_rejected() {
        let base = provider::visual_scenes::tests::composed_proposal(3);
        let mut missing_visual = base.clone();
        missing_visual["sections"][0]
            .as_object_mut()
            .unwrap()
            .remove("visual");
        let mut leaf_parent = base.clone();
        leaf_parent["sections"][0]["visual"]["presentation"]["scenes"][0]["elements"][3]["parentIndex"] =
            json!(1);
        let mut future = base.clone();
        future["sections"][0]["visual"]["presentation"]["schemaVersion"] = json!(99);
        let mut foreign = base.clone();
        foreign["sections"][0]["visual"]["nodes"][0]["evidence"][0]["claimIndex"] = json!(42);
        let mut false_source = base.clone();
        false_source["sections"][0]["visual"]["presentation"]["scenes"][0]["elements"][1]["value"] =
            json!("invented source");
        for candidate in [
            missing_visual,
            leaf_parent,
            future,
            foreign,
            false_source,
            provider::visual_scenes::tests::composed_proposal(1),
        ] {
            let data = run(&request(candidate)).unwrap();
            assert_eq!(data["valid"], false, "{data}");
        }
    }

    #[test]
    fn source_profile_and_protocol_failures_are_infrastructure_errors() {
        let base = request(provider::visual_scenes::tests::composed_proposal(3));
        for mutation in [
            "sourceHash",
            "path",
            "duplicate",
            "model",
            "protocol",
            "unknown",
            "missingTarget",
        ] {
            let mut input = base.clone();
            match mutation {
                "sourceHash" => input["sources"][0]["sha256"] = json!("a".repeat(64)),
                "path" => input["sources"][0]["path"] = json!("../greeting.ts"),
                "duplicate" => {
                    let source = input["sources"][0].clone();
                    input["sources"].as_array_mut().unwrap().push(source);
                }
                "model" => input["profile"]["modelId"] = json!("different-model"),
                "protocol" => input["protocolVersion"] = json!(2),
                "unknown" => input["unexpected"] = json!(true),
                _ => input["targetPath"] = json!("absent.ts"),
            }
            assert!(run(&input).is_err(), "accepted {mutation}");
        }
        assert!(process(br#"{"protocolVersion":1,"protocolVersion":2}"#).is_err());
        assert!(process(&vec![b' '; MAX_INPUT_BYTES + 1]).is_err());
        let mut input = base;
        input["proposal"] = json!("{\"title\":\"a\",\"title\":\"b\"}");
        assert_eq!(run(&input).unwrap()["valid"], false);
        input["proposal"] = json!("x".repeat(chapters::MAX_GENERATED_RESPONSE_BYTES + 1));
        assert_eq!(run(&input).unwrap()["valid"], false);
    }

    #[test]
    fn metadata_uses_actual_shared_prompts_and_is_deterministic() {
        let request = json!({"protocolVersion":1,"action":"metadata","profile":profile()});
        let first = run(&request).unwrap();
        assert_eq!(first, run(&request).unwrap());
        assert!(
            first["systemInstruction"]
                .as_str()
                .unwrap()
                .contains("Teaching policy v19:")
        );
        assert!(
            first["systemInstruction"]
                .as_str()
                .unwrap()
                .contains("ai-html-compositions-v12")
        );
        assert!(
            first["userPrompt"]
                .as_str()
                .unwrap()
                .contains("Hello, ${name}")
        );
        assert_eq!(
            first["generationConfig"]["candidateCorrection"]["maxAttempts"],
            1
        );
        assert_eq!(first["responseSchemaVersion"], 3);
        assert_eq!(first["responseSchema"]["additionalProperties"], false);
    }

    #[test]
    fn direct_api_contract_metadata_needs_no_key_and_rejects_agent_settings() {
        for backend in ["gemini", "openai", "anthropic"] {
            let disclosure = provider::disclosure_for(backend).unwrap();
            let mut input = json!({"protocolVersion":1,"action":"metadata","profile":{
                "backendId":backend,"modelId":disclosure.models[0].model_id,"reasoningEffort":null,"approvalPolicy":null}});
            let data = run(&input).unwrap();
            assert_eq!(data["backendId"], backend);
            assert!(!data["modelIds"].as_array().unwrap().is_empty());
            input["profile"]["reasoningEffort"] = json!("max");
            assert!(run(&input).is_err());
        }
    }

    #[test]
    fn owned_source_storage_is_removed_on_drop() {
        let fixture = Fixture::new(vec![Source {
            path: FIXTURE_PATH.into(),
            sha256: sha256(FIXTURE_SOURCE.as_bytes()),
            text: FIXTURE_SOURCE.into(),
        }])
        .unwrap();
        let root = fixture.root.clone();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&root).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        assert!(root.exists());
        drop(fixture);
        assert!(!root.exists());
    }
}
