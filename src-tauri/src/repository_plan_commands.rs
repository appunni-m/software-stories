//! Local repository planning preparation. This command starts no agent/model
//! request and grants no reusable source consent or generation authorization.

use crate::{
    ProjectSessions, app_data_directory, recovery, repository_plan, repository_plan_history,
    repository_plan_store,
};
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

/// Metadata discovery does not qualify accepted source evidence as current.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn list_repository_plan_revisions(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
) -> Result<Vec<repository_plan_history::RepositoryPlanRevisionSummary>, String> {
    sessions.get(&project_id)?;
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        repository_plan_history::list(&storage, &project_id)
    })
    .await
    .map_err(|_| {
        "Accepted plan discovery stopped unexpectedly. Existing history was preserved.".to_owned()
    })?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn load_repository_plan_revision(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    revision_id: String,
) -> Result<repository_plan_history::RepositoryPlanRevisionDetail, String> {
    sessions.get(&project_id)?;
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        repository_plan_history::load(&storage, &project_id, &revision_id)
    })
    .await
    .map_err(|_| {
        "Accepted plan loading stopped unexpectedly. Existing history was preserved.".to_owned()
    })?
}

/// Explicit acceptance appends a native immutable decision, never chapters or
/// coverage. Expected content and parent identity prevent stale review writes.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn accept_repository_plan_draft(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    input: repository_plan_history::AcceptRepositoryPlanInput,
) -> Result<repository_plan_history::RepositoryPlanRevisionDetail, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || repository_plan_history::accept(&storage, input))
        .await.map_err(|_| "Plan acceptance stopped unexpectedly. An immutable revision may already exist; refresh accepted history before retrying.".to_owned())?
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolvePlanNoticeInput {
    project_id: String,
    source: repository_plan_history::ReviewedPlanSource,
}

/// Native digest, source replay and exact unchanged-journal comparison all
/// precede resolution. The renderer cannot provide an arbitrary notice ID.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn resolve_repository_plan_notice(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    journal: State<'_, recovery::InterruptionJournal>,
    input: ResolvePlanNoticeInput,
) -> Result<bool, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    let journal = journal.inner().clone();
    tauri::async_runtime::spawn_blocking(move || repository_plan_history::resolve_notice(&storage, &journal, &input.project_id, input.source))
        .await.map_err(|_| "Plan notice resolution stopped unexpectedly. Refresh recovery before retrying; accepted plans and chapters were preserved.".to_owned())?
}
