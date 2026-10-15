//! Local repository planning preparation. This command starts no agent/model
//! request and grants no reusable source consent or generation authorization.

use crate::{ProjectSessions, app_data_directory, repository_plan, repository_plan_store};
use tauri::{AppHandle, State};

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn prepare_repository_plan_context(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    input: repository_plan::PrepareRepositoryPlanInput,
) -> Result<repository_plan::PreparedRepositoryPlanContext, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || repository_plan::prepare(&storage, input))
        .await
        .map_err(|_| "Local planning preparation stopped unexpectedly. Existing plans and chapters were preserved.".to_owned())?
}

/// Discover local metadata without source revalidation or agent/model access.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn list_repository_plan_drafts(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
) -> Result<Vec<repository_plan_store::RepositoryPlanDraftSummary>, String> {
    sessions.get(&project_id)?;
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || repository_plan_store::list(&storage, &project_id))
        .await
        .map_err(|_| {
            "Saved plan discovery stopped unexpectedly. Local records were preserved.".to_owned()
        })?
}

/// Revalidate the original proposal against its exact native snapshot and index.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn load_repository_plan_draft(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    draft_id: String,
) -> Result<repository_plan_store::RepositoryPlanDraftDetail, String> {
    sessions.get(&project_id)?;
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        repository_plan_store::load(&storage, &project_id, &draft_id)
    })
    .await
    .map_err(|_| {
        "Saved plan loading stopped unexpectedly. Local records were preserved.".to_owned()
    })?
}

/// Explicitly remove only this project's canonical regular plan record.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn discard_repository_plan_draft(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    draft_id: String,
) -> Result<(), String> {
    sessions.get(&project_id)?;
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || repository_plan_store::discard(&storage, &project_id, &draft_id))
        .await.map_err(|_| "Discarding the saved plan stopped unexpectedly. Refresh saved plans and recovery before trying again.".to_owned())?
}
