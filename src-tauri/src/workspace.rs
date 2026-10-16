//! Snapshot-scoped exploration permissions and bounded, read-only Git history.

use crate::{
    ProjectSessions, app_data_directory, bounded_process, codex_agent,
    codex_identity::CodexReviewRuntime, project, snapshot,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tauri::{AppHandle, State};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Grant {
    id: String,
    project_id: String,
    snapshot_id: String,
    manifest_sha256: String,
    account_sha256: String,
    executable_sha256: String,
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
    let path = grant_path(storage, id)?;
    if fs::metadata(&path)
        .map_err(|_| "Allow Codex to explore this snapshot first.".to_owned())?
        .len()
        > 8192
    {
        return Err("The exploration permission is invalid.".to_owned());
    }
    let grant: Grant = serde_json::from_slice(
        &fs::read(path).map_err(|_| "The exploration permission is unavailable.".to_owned())?,
    )
    .map_err(|_| "The exploration permission is invalid.".to_owned())?;
    if grant.id != id
        || grant.project_id != project
        || grant.snapshot_id != snapshot
        || grant.account_sha256 != runtime.account_identity_sha256
        || grant.executable_sha256 != runtime.executable_sha256
        || grant.manifest_sha256 != manifest_digest(storage, project, snapshot)?
    {
        return Err("The snapshot or Codex account changed. Allow exploration again.".to_owned());
    }
    Ok(())
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
        let id = Uuid::new_v4().to_string();
        let grant = Grant {
            id: id.clone(),
            project_id: project_id.clone(),
            snapshot_id: snapshot_id.clone(),
            manifest_sha256: manifest_digest(&storage, &project_id, &snapshot_id)?,
            account_sha256: runtime.account_identity_sha256,
            executable_sha256: runtime.executable_sha256,
        };
        let path = grant_path(&storage, &id)?;
        fs::create_dir_all(path.parent().ok_or("Permission storage is unavailable.")?)
            .map_err(|_| "Permission storage is unavailable.".to_owned())?;
        let bytes =
            serde_json::to_vec(&grant).map_err(|_| "Permission could not be saved.".to_owned())?;
        // A fresh UUID and create_new ensure an existing permission is never overwritten.
        use std::io::Write;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|_| "Permission could not be saved.".to_owned())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "Permission could not be saved.".to_owned())?;
        Ok(id)
    })
    .await
    .map_err(|_| "Permission setup stopped unexpectedly.".to_owned())?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn revoke_workspace_exploration(app: AppHandle, grant_id: String) -> Result<(), String> {
    let path = grant_path(&app_data_directory(&app)?, &grant_id)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("Permission could not be revoked. Try again.".to_owned()),
    }
}

#[derive(Serialize)]
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
    project_id: String,
) -> Result<Vec<GitCommit>, String> {
    let project = sessions.get(&project_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut command = project::git_command();
        command
            .current_dir(&project.repository_root)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_NO_LAZY_FETCH", "1")
            .args([
                "--no-pager",
                "log",
                "--all",
                "--date-order",
                "--max-count=80",
                "--format=%H%x00%P%x00%s%x00%an%x00%aI%x00",
                "--",
            ]);
        let output = bounded_process::run(
            &mut command,
            None,
            bounded_process::Limits {
                command_timeout: Duration::from_secs(8),
                operation_deadline: Instant::now() + Duration::from_secs(10),
                stdout_bytes: 512_000,
                stderr_bytes: 8192,
                stdin_bytes: 0,
            },
            None,
        )
        .map_err(|_| "Git history could not be read within its local limits.".to_owned())?;
        if !output.status.success() {
            return Err(
                "Git history is unavailable. This repository may have no commits yet.".to_owned(),
            );
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let fields: Vec<&str> = text.split('\0').collect();
        Ok(fields
            .chunks_exact(5)
            .filter_map(|row| {
                let oid = row[0].trim();
                if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return None;
                }
                Some(GitCommit {
                    oid: oid.to_owned(),
                    parents: row[1].split_whitespace().map(str::to_owned).collect(),
                    subject: row[2].chars().take(300).collect(),
                    author: row[3].chars().take(120).collect(),
                    timestamp: row[4].to_owned(),
                })
            })
            .collect())
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
                .any(|hit| hit.path == SIMPLE_PATH && hit.line >= 1)
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchHit {
    path: String,
    line: usize,
    text: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResult {
    matches: Vec<SearchHit>,
    limited: bool,
}
fn search_snapshot(
    storage: &Path,
    project: &str,
    snapshot_id: &str,
    query: &str,
) -> Result<SearchResult, String> {
    if !(2..=120).contains(&query.chars().count()) || query.chars().any(char::is_control) {
        return Err("Search with 2–120 characters.".to_owned());
    }
    let manifest = snapshot::load_snapshot_manifest(storage, project, snapshot_id)?;
    let needle = query.to_lowercase();
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut bytes = 0u64;
    let mut matches = vec![];
    let mut limited = false;
    let mut files = manifest
        .files
        .iter()
        .filter(|f| {
            !f.secret_like
                && matches!(
                    f.content_kind,
                    snapshot::CapturedContentKind::Text | snapshot::CapturedContentKind::Unknown
                )
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|f| {
        (
            !crate::source_discovery::is_code_path(&f.path),
            f.path.clone(),
        )
    });
    for file in files {
        if Instant::now() >= deadline
            || matches.len() >= 80
            || bytes.saturating_add(file.size_bytes) > 24_000_000
        {
            limited = true;
            break;
        }
        bytes += file.size_bytes;
        let source =
            match snapshot::read_snapshot_source_from_manifest(storage, &manifest, &file.path) {
                Ok(source) => source,
                Err(_) => {
                    limited = true;
                    continue;
                }
            };
        for (index, line) in source.contents.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                matches.push(SearchHit {
                    path: file.path.clone(),
                    line: index + 1,
                    text: line.trim().chars().take(220).collect(),
                });
                if matches.len() >= 80 {
                    limited = true;
                    break;
                }
            }
        }
    }
    Ok(SearchResult { matches, limited })
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn search_workspace_source(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    snapshot_id: String,
    query: String,
) -> Result<SearchResult, String> {
    sessions.get(&project_id)?;
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        search_snapshot(&storage, &project_id, &snapshot_id, &query)
    })
    .await
    .map_err(|_| "Source search stopped unexpectedly.".to_owned())?
}
