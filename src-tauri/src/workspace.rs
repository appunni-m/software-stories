//! Snapshot-scoped exploration permissions and bounded, read-only Git history.

use crate::{
    ProjectSessions, app_data_directory, codex_agent, codex_identity::CodexReviewRuntime,
    repository_context_commands::RepositoryContextJobs, snapshot,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, State};
use uuid::Uuid;

mod git_history;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Grant {
    id: String,
    project_id: String,
    snapshot_id: String,
    manifest_sha256: String,
    account_sha256: String,
    executable_sha256: String,
    retrieval_policy: String,
}

fn grant_path(storage: &Path, id: &str) -> Result<PathBuf, String> {
    let parsed =
        Uuid::parse_str(id).map_err(|_| "Allow exploration of this snapshot first.".to_owned())?;
    if parsed.to_string() != id {
        return Err("The exploration permission is invalid.".to_owned());
    }
    Ok(storage
        .join("workspace-permissions")
        .join(format!("{id}.json")))
}

fn manifest_digest(storage: &Path, project: &str, snapshot: &str) -> Result<String, String> {
    let manifest = snapshot::load_snapshot_manifest(storage, project, snapshot)?;
    let bytes = serde_json::to_vec(&manifest)
        .map_err(|_| "The snapshot could not be checked.".to_owned())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

/// A grant is checked again during native dispatch, including after a restart.
pub(crate) fn check_grant(
    storage: &Path,
    id: &str,
    project: &str,
    snapshot: &str,
    runtime: &CodexReviewRuntime,
) -> Result<(), String> {
    checked_grant(storage, id, project, snapshot, runtime).map(|_| ())
}

fn checked_grant(
    storage: &Path,
    id: &str,
    project: &str,
    snapshot: &str,
    runtime: &CodexReviewRuntime,
) -> Result<Grant, String> {
    let path = grant_path(storage, id)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|_| "Allow Codex to explore this snapshot first.".to_owned())?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 8192 {
        return Err("The exploration permission is invalid.".to_owned());
    }
    let grant: Grant = serde_json::from_slice(
        &crate::bounded_file::read(&path, 8192)
            .map_err(|_| "The exploration permission is unavailable.".to_owned())?,
    )
    .map_err(|_| "The exploration permission is invalid.".to_owned())?;
    if grant.retrieval_policy != crate::evidence_ledger::POLICY
        || grant.id != id
        || grant.project_id != project
        || grant.snapshot_id != snapshot
        || grant.account_sha256 != runtime.account_identity_sha256
        || grant.executable_sha256 != runtime.executable_sha256
        || grant.manifest_sha256 != manifest_digest(storage, project, snapshot)?
    {
        return Err("The snapshot or Codex account changed. Allow exploration again.".to_owned());
    }
    Ok(grant)
}

/// Retrieval uses the same explicit current snapshot permission.
pub(crate) fn check_retrieval_grant(
    storage: &Path,
    id: &str,
    project: &str,
    snapshot: &str,
    runtime: &CodexReviewRuntime,
) -> Result<(), String> {
    check_grant(storage, id, project, snapshot, runtime)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn authorize_workspace_exploration(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    project_id: String,
    snapshot_id: String,
    consent: bool,
) -> Result<String, String> {
    sessions.get(&project_id)?;
    if !consent {
        return Err(
            "Permission is required before sharing repository context with Codex.".to_owned(),
        );
    }
    let storage = app_data_directory(&app)?;
    let agent = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let runtime = agent.review_runtime(&storage)?;
        save_retrieval_grant(&storage, &project_id, &snapshot_id, &runtime)
    })
    .await
    .map_err(|_| "Permission setup stopped unexpectedly.".to_owned())?
}

pub(crate) fn save_retrieval_grant(
    storage: &Path,
    project_id: &str,
    snapshot_id: &str,
    runtime: &CodexReviewRuntime,
) -> Result<String, String> {
    if !runtime.valid_shape() {
        return Err(crate::codex_identity::changed());
    }
    let id = Uuid::new_v4().to_string();
    let grant = Grant {
        id: id.clone(),
        project_id: project_id.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        manifest_sha256: manifest_digest(&storage, &project_id, &snapshot_id)?,
        account_sha256: runtime.account_identity_sha256.clone(),
        executable_sha256: runtime.executable_sha256.clone(),
        retrieval_policy: crate::evidence_ledger::POLICY.to_owned(),
    };
    let path = grant_path(&storage, &id)?;
    codex_agent::ensure_private_directory(
        path.parent().ok_or("Permission storage is unavailable.")?,
    )?;
    let bytes =
        serde_json::to_vec(&grant).map_err(|_| "Permission could not be saved.".to_owned())?;
    codex_agent::create_private_file(&path, &bytes)
        .map_err(|_| "Permission could not be saved.".to_owned())?;
    Ok(id)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn revoke_workspace_exploration(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
    grant_id: String,
) -> Result<(), String> {
    let path = grant_path(&app_data_directory(&app)?, &grant_id)?;
    manager.serialize_permission_change(|| match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Permission could not be revoked. Try again.".to_owned()),
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GitCommit {
    oid: String,
    parents: Vec<String>,
    subject: String,
    author: String,
    timestamp: String,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn list_workspace_commits(
    sessions: State<'_, ProjectSessions>,
    jobs: State<'_, RepositoryContextJobs>,
    project_id: String,
    request_id: String,
) -> Result<Vec<GitCommit>, String> {
    let request = jobs.begin(&request_id).map_err(|_| "Git history could not start. Try again.".to_owned())?;
    let project = sessions.get(&project_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        git_history::list(Path::new(&project.repository_root), request.cancellation())
    })
    .await
    .map_err(|_| "Git history stopped unexpectedly.".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH};
    fn runtime() -> CodexReviewRuntime {
        CodexReviewRuntime {
            connection_id: Uuid::new_v4().to_string(),
            executable_path: "/usr/local/bin/codex".to_owned(),
            executable_sha256: "a".repeat(64),
            account_identity_sha256: "b".repeat(64),
            account_revision: 1,
        }
    }

    #[test]
    fn codex_file_and_overview_chapters_request_layout_correction_without_a_chat_question() {
        let fixture = Fixture::new();
        let runtime = runtime();
        for target_path in [Some(SIMPLE_PATH.to_owned()), None] {
            let input = crate::provider::BuildCodexReviewInput {
                project_id:fixture.manifest.summary.project_id.clone(),
                snapshot_id:fixture.manifest.summary.snapshot_id.clone(), target_path,
                selected_paths:vec![SIMPLE_PATH.to_owned()], model_id:"gpt-6-luna".into(),
                effort:"max".into(), approval_policy:"never".into(), section_update:None,
                question:None, focus:vec![], workspace_grant:None,
            };
            let review=crate::provider::build_codex_chapter_review(&fixture.storage,&input,&runtime).unwrap().review;
            assert_eq!(crate::claim_alignment::CorrectionPolicy::from_generation_config(&review.generation_config),
                crate::claim_alignment::CorrectionPolicy::ChapterScenesV5);
            assert!(review.generation_config.get("claimAlignmentCorrection").is_none());
            assert!(review.system_instruction.contains("one correction"));
            crate::provider::revalidate_codex_request_review(&fixture.storage,&review,&runtime).unwrap();
        }
    }
    fn save_grant(fixture: &Fixture, runtime: &CodexReviewRuntime) -> String {
        let id = Uuid::new_v4().to_string();
        let path = grant_path(&fixture.storage, &id).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let grant = Grant {
            id: id.clone(),
            project_id: fixture.manifest.summary.project_id.clone(),
            snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
            manifest_sha256: manifest_digest(
                &fixture.storage,
                &fixture.manifest.summary.project_id,
                &fixture.manifest.summary.snapshot_id,
            )
            .unwrap(),
            account_sha256: runtime.account_identity_sha256.clone(),
            executable_sha256: runtime.executable_sha256.clone(),
            retrieval_policy: crate::evidence_ledger::POLICY.to_owned(),
        };
        fs::write(path, serde_json::to_vec(&grant).unwrap()).unwrap();
        id
    }
    #[test]
    fn local_search_uses_captured_source_and_rejects_empty_queries() {
        let fixture = Fixture::new();
        let p = &fixture.manifest.summary.project_id;
        let snapshot = &fixture.manifest.summary.snapshot_id;
        let source = snapshot::read_snapshot_source_from_manifest(
            &fixture.storage,
            &fixture.manifest,
            SIMPLE_PATH,
        )
        .unwrap();
        let first = source
            .contents
            .lines()
            .find(|line| line.trim().len() > 8)
            .unwrap()
            .trim();
        let query = first.chars().take(12).collect::<String>();
        let result = search_snapshot(&fixture.storage, p, snapshot, &query).unwrap();
        assert!(
            result
                .matches
                .iter()
                .any(|hit| hit.range.path == SIMPLE_PATH && hit.line >= 1)
        );
        assert!(search_snapshot(&fixture.storage, p, snapshot, "").is_err());
        assert!(search_snapshot(&fixture.storage, p, snapshot, "bad\nquery").is_err());
    }
    #[test]
    fn permissions_are_bound_to_snapshot_account_runtime_and_revocation() {
        let fixture = Fixture::new();
        let runtime = runtime();
        let id = save_grant(&fixture, &runtime);
        let p = &fixture.manifest.summary.project_id;
        let s = &fixture.manifest.summary.snapshot_id;
        assert!(check_grant(&fixture.storage, &id, p, s, &runtime).is_ok());
        let mut changed = runtime.clone();
        changed.account_identity_sha256 = "c".repeat(64);
        assert!(check_grant(&fixture.storage, &id, p, s, &changed).is_err());
        changed = runtime.clone();
        changed.executable_sha256 = "d".repeat(64);
        assert!(check_grant(&fixture.storage, &id, p, s, &changed).is_err());
        assert!(check_grant(&fixture.storage, &id, p, "wrong-snapshot", &runtime).is_err());
        assert!(grant_path(&fixture.storage, "../../somewhere").is_err());
        fs::remove_file(grant_path(&fixture.storage, &id).unwrap()).unwrap();
        assert!(check_grant(&fixture.storage, &id, p, s, &runtime).is_err());
    }
    #[test]
    fn unsupported_permission_is_rejected_without_rewriting() {
        let fixture = Fixture::new();
        let runtime = runtime();
        let p = &fixture.manifest.summary.project_id;
        let s = &fixture.manifest.summary.snapshot_id;
        let old = save_grant(&fixture, &runtime);
        let path = grant_path(&fixture.storage, &old).unwrap();
        let mut legacy: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        legacy.as_object_mut().unwrap().remove("retrievalPolicy");
        fs::write(&path, serde_json::to_vec(&legacy).unwrap()).unwrap();
        let original = fs::read(&path).unwrap();
        assert!(check_grant(&fixture.storage, &old, p, s, &runtime).is_err());
        assert!(
            check_retrieval_grant(&fixture.storage, &old, p, s, &runtime)
                .unwrap_err()
                .contains("invalid")
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        let new = save_retrieval_grant(&fixture.storage, p, s, &runtime).unwrap();
        check_retrieval_grant(&fixture.storage, &new, p, s, &runtime).unwrap();
        assert!(check_retrieval_grant(&fixture.storage, &old, p, s, &runtime).is_err());
    }
    #[test]
    fn question_and_permission_are_fingerprinted_and_rechecked_before_send() {
        let fixture = Fixture::new();
        let runtime = runtime();
        let grant = save_grant(&fixture, &runtime);
        let mut input = crate::provider::BuildCodexReviewInput {
            project_id: fixture.manifest.summary.project_id.clone(),
            snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
            target_path: Some(SIMPLE_PATH.to_owned()),
            selected_paths: vec![SIMPLE_PATH.to_owned()],
            model_id: "gpt-6-luna".to_owned(),
            effort: "max".to_owned(),
            approval_policy: "never".to_owned(),
            section_update: None,
            question: Some("How does this implementation greet its caller?".to_owned()),
            focus: vec![],
            workspace_grant: Some(grant.clone()),
        };
        let first = crate::provider::build_codex_chapter_review(&fixture.storage, &input, &runtime)
            .unwrap()
            .review;
        assert!(
            first
                .user_prompt
                .contains("How does this implementation greet its caller?")
        );
        assert!(
            crate::provider::revalidate_codex_request_review(&fixture.storage, &first, &runtime)
                .is_ok()
        );
        assert_eq!(
            first.generation_config["candidateCorrection"],
            crate::claim_alignment::chapter_correction_config()
        );
        let mut tampered = first.clone();
        tampered.generation_config["candidateCorrection"]["maxAttempts"] =
            serde_json::json!(2);
        assert!(
            crate::provider::revalidate_codex_request_review(&fixture.storage, &tampered, &runtime)
                .is_err()
        );
        input.question = Some("Which inputs reach this implementation?".to_owned());
        let second =
            crate::provider::build_codex_chapter_review(&fixture.storage, &input, &runtime)
                .unwrap()
                .review;
        assert_ne!(first.fingerprint, second.fingerprint);
        fs::remove_file(grant_path(&fixture.storage, &grant).unwrap()).unwrap();
        assert!(
            crate::provider::revalidate_codex_request_review(&fixture.storage, &first, &runtime)
                .is_err()
        );
    }
}

#[cfg(test)]
fn search_snapshot(
    storage: &Path,
    project: &str,
    snapshot_id: &str,
    query: &str,
) -> Result<crate::repository_context::SearchResult, String> {
    let manifest = snapshot::load_snapshot_manifest(storage, project, snapshot_id)?;
    let syntax = crate::index::load_snapshot_index(storage, project, snapshot_id)?;
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let budget = crate::repository_context::Budget::new(&cancelled);
    let index = crate::repository_context::prepare(storage, &manifest, syntax.as_ref(), &budget)?;
    crate::repository_context::search(storage, &manifest, &index, query, None, 32, &budget)
}
