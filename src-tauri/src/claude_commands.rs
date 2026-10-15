//! Explicit Claude CLI metadata inspection. Generation/login are not exposed.

use crate::{app_data_directory, claude_connection, claude_worker};
use std::path::Path;
use tauri::{AppHandle, State};

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn inspect_claude_agent_connection(
    app: AppHandle,
    manager: State<'_, claude_worker::ClaudeWorkerManager>,
    executable_path: String,
    run_id: String,
) -> Result<claude_connection::ClaudeConnectionObservation, claude_worker::ClaudeWorkerFailure> {
    use claude_worker::{ClaudeWorkerFailure as Failure, ClaudeWorkerFailureKind as Kind};
    let storage = app_data_directory(&app).map_err(|_| Failure {
        kind: Kind::PreparationFailed,
        message: "Local app storage is unavailable. No Claude inspection was started.".to_owned(),
    })?;
    let worker = manager.inner().clone();
    match tauri::async_runtime::spawn_blocking(move || {
        worker.inspect_connection(&storage, Path::new(&executable_path), &run_id)
    })
    .await
    {
        Ok(result) => result,
        Err(_) => {
            manager.shutdown();
            Err(Failure {
                kind: Kind::CleanupFailed,
                message: "Claude inspection stopped unexpectedly. Restart the app after checking native process/storage cleanup.".to_owned(),
            })
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn cancel_claude_agent_operation(
    manager: State<'_, claude_worker::ClaudeWorkerManager>,
    run_id: String,
) -> bool {
    manager.cancel(&run_id)
}
