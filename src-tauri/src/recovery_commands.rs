//! Local evidence recovery only. No runtime thread, settings or consent resume.

use crate::{ProjectSessions, app_data_directory, git_evidence, provider, recovery};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecoveredGenerationScope {
    operation: recovery::InterruptedOperation,
    comparison: Option<git_evidence::GitComparisonResult>,
}

/// Read the journal natively so the renderer cannot replace a recovered
/// chapter/section/range/Git scope with a broader selection. A request requires
/// fresh runtime configuration, evidence review and explicit sending consent.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn load_interrupted_generation_scope(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    operation_id: String,
) -> Result<RecoveredGenerationScope, String> {
    let storage = app_data_directory(&app)?;
    let operation = interruptions
        .list(&storage)?
        .into_iter()
        .find(|operation| operation.operation_id == operation_id)
        .filter(|operation| {
            operation.kind == recovery::InterruptedOperationKind::ProviderGeneration
        })
        .ok_or("This interruption notice is no longer available. No request was opened.")?;
    let project = sessions.get(&operation.project_id)?;
    let repository = PathBuf::from(project.repository_root);
    let for_check = operation.clone();
    let storage_for_check = storage.clone();
    let comparison = tauri::async_runtime::spawn_blocking(move || {
        provider::revalidate_interrupted_evidence(&storage_for_check, &repository, &for_check)
            .map_err(|error| error.message)
    })
    .await
    .map_err(|_| "The original evidence could not be checked. No request was opened.")??;
    // Discard/replacement during blocking reads cannot resurrect an old notice.
    if !interruptions
        .list(&storage)?
        .iter()
        .any(|current| current == &operation)
    {
        return Err(
            "This interruption notice changed while reading its evidence. No request was opened."
                .to_owned(),
        );
    }
    Ok(RecoveredGenerationScope {
        operation,
        comparison,
    })
}
