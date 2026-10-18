//! Repository inspection runs outside the UI. Only the still-owned request may
//! publish Recents and session metadata; inspection itself has no side effects.

use crate::{
    ProjectSessions, app_data_directory,
    project::{self, ProjectOpenResult},
    repository_context_commands::{RepositoryContextJobs, Request},
};
use std::{path::Path, sync::atomic::Ordering};
use tauri::{AppHandle, State};
use tauri_plugin_dialog::DialogExt;

fn publish(
    storage: &Path,
    inspected: ProjectOpenResult,
    request: &Request,
    sessions: &ProjectSessions,
) -> Result<ProjectOpenResult, String> {
    let prepared = project::prepare_inspected_project(storage, inspected, request.cancellation());
    match request.commit(|| {
        let result = prepared.publish();
        sessions.remember_result(&result);
        result
    }) {
        Ok(result) => Ok(result),
        Err(_) if request.cancellation().load(Ordering::Acquire) => {
            Ok(ProjectOpenResult::cancelled())
        }
        Err(error) => Err(error),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn pick_project_folder(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    jobs: State<'_, RepositoryContextJobs>,
    request_id: String,
) -> Result<ProjectOpenResult, String> {
    let request = jobs.begin(&request_id)?;
    let sessions = sessions.inner().clone();
    let storage = app_data_directory(&app)?;
    let (send, receive) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose a local Git repository")
        .pick_folder(move |selected| {
            let _ = send.send(selected);
        });
    let selected = receive
        .await
        .map_err(|_| "The folder picker closed before returning a selection.".to_owned())?;
    if request.cancellation().load(Ordering::Acquire) {
        return Ok(ProjectOpenResult::cancelled());
    }
    let Some(selected) = selected else {
        return Ok(ProjectOpenResult::cancelled());
    };
    let path = selected
        .into_path()
        .map_err(|_| "The selected folder path could not be read.".to_owned())?;
    tauri::async_runtime::spawn_blocking(move || {
        let inspected =
            project::inspect_selected_project(&path.display().to_string(), request.cancellation());
        publish(&storage, inspected, &request, &sessions)
    })
    .await
    .map_err(|_| {
        "The repository inspection worker stopped. Your previous workspace remains available."
            .to_owned()
    })?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn open_recent_project(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    jobs: State<'_, RepositoryContextJobs>,
    request_id: String,
    project_id: String,
) -> Result<ProjectOpenResult, String> {
    let request = jobs.begin(&request_id)?;
    let sessions = sessions.inner().clone();
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let inspected =
            project::inspect_recent_project(&storage, &project_id, request.cancellation());
        publish(&storage, inspected, &request, &sessions)
    })
    .await
    .map_err(|_| {
        "The repository inspection worker stopped. Your previous workspace remains available."
            .to_owned()
    })?
}

#[cfg(test)]
mod tests;
