//! Local repository planning preparation. This command starts no agent/model
//! request and grants no reusable source consent or generation authorization.

use crate::{ProjectSessions, app_data_directory, repository_plan};
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
