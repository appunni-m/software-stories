//! Explicit Claude inspection and offline operation review. Inference/login are
//! not exposed; native connection observations cannot authorize generation.

use crate::{
    ProjectSessions, app_data_directory, claude_connection, claude_connections, claude_generation,
    claude_worker, provider,
};
use std::path::Path;
use tauri::{AppHandle, State};

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn inspect_claude_agent_connection(
    app: AppHandle,
    manager: State<'_, claude_worker::ClaudeWorkerManager>,
    connections: State<'_, claude_connections::ClaudeConnections>,
    executable_path: String,
    run_id: String,
) -> Result<claude_connection::ClaudeConnectionObservation, claude_worker::ClaudeWorkerFailure> {
    use claude_worker::{ClaudeWorkerFailure as Failure, ClaudeWorkerFailureKind as Kind};
    let ticket = connections.begin_inspection(&run_id)?;
    let storage = app_data_directory(&app).map_err(|_| {
        connections.abandon_inspection(&ticket);
        Failure {
            kind: Kind::PreparationFailed,
            message: "Local app storage is unavailable. No Claude inspection was started."
                .to_owned(),
        }
    })?;
    let worker = manager.inner().clone();
    match tauri::async_runtime::spawn_blocking(move || {
        worker.inspect_connection(&storage, Path::new(&executable_path), &run_id)
    })
    .await
    {
        Ok(Ok(observation)) => {
            connections.finish_installation(&ticket)?;
            Ok(observation)
        }
        Ok(Err(error)) => {
            connections.abandon_inspection(&ticket);
            Err(error)
        }
        Err(_) => {
            connections.abandon_inspection(&ticket);
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
    connections: State<'_, claude_connections::ClaudeConnections>,
    run_id: String,
) -> bool {
    let invalidated = connections.cancel_operation(&run_id);
    manager.cancel(&run_id) || invalidated
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn inspect_claude_agent_model_catalog(
    app: AppHandle,
    manager: State<'_, claude_worker::ClaudeWorkerManager>,
    connections: State<'_, claude_connections::ClaudeConnections>,
    executable_path: String,
    node_executable_path: String,
    sdk_module_path: String,
    run_id: String,
) -> Result<
    crate::claude_discovery::ClaudeModelDiscoveryObservation,
    claude_worker::ClaudeWorkerFailure,
> {
    use claude_worker::{ClaudeWorkerFailure as Failure, ClaudeWorkerFailureKind as Kind};
    let ticket = connections.begin_inspection(&run_id)?;
    let storage = app_data_directory(&app).map_err(|_| {
        connections.abandon_inspection(&ticket);
        Failure {
            kind: Kind::PreparationFailed,
            message: "Local app storage is unavailable. No Claude catalog check was started."
                .to_owned(),
        }
    })?;
    let worker = manager.inner().clone();
    match tauri::async_runtime::spawn_blocking(move || {
        worker.inspect_model_catalog(
            &storage,
            Path::new(&executable_path),
            Path::new(&node_executable_path),
            Path::new(&sdk_module_path),
            &run_id,
        )
    })
    .await
    {
        Ok(Ok(mut observation)) => match connections.finish_catalog(&ticket, &observation) {
            Ok(connection) => {
                observation.review_connection = Some(connection);
                Ok(observation)
            }
            Err(error) => {
                connections.abandon_inspection(&ticket);
                Err(error)
            }
        },
        Ok(Err(error)) => {
            connections.abandon_inspection(&ticket);
            Err(error)
        }
        Err(_) => {
            connections.abandon_inspection(&ticket);
            manager.shutdown();
            Err(Failure {
                kind: Kind::CleanupFailed,
                message: "Claude catalog inspection stopped unexpectedly. Restart the app after checking native process/storage cleanup.".to_owned(),
            })
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn forget_claude_agent_connection(
    connections: State<'_, claude_connections::ClaudeConnections>,
    connection_id: String,
) -> Result<bool, claude_worker::ClaudeWorkerFailure> {
    connections.forget(&connection_id)
}

/// Offline only. The renderer selects settings and immutable evidence; the
/// native store supplies identity/capabilities from the current observation.
/// Preparing a review neither spawns a process nor accepts sending consent.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_claude_chapter_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    connections: State<'_, claude_connections::ClaudeConnections>,
    reviews: State<'_, provider::PendingProviderReviews>,
    connection_id: String,
    input: provider::BuildClaudeReviewInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    build_operation_review(
        app,
        &sessions,
        &connections,
        &reviews,
        connection_id,
        ReviewInput::Chapter(input),
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_claude_code_lens_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    connections: State<'_, claude_connections::ClaudeConnections>,
    reviews: State<'_, provider::PendingProviderReviews>,
    connection_id: String,
    input: provider::BuildClaudeCodeLensInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    build_operation_review(
        app,
        &sessions,
        &connections,
        &reviews,
        connection_id,
        ReviewInput::CodeLens(input),
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_claude_pr_guide_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    connections: State<'_, claude_connections::ClaudeConnections>,
    reviews: State<'_, provider::PendingProviderReviews>,
    connection_id: String,
    input: provider::BuildClaudePrGuideInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    build_operation_review(
        app,
        &sessions,
        &connections,
        &reviews,
        connection_id,
        ReviewInput::PrGuide(input),
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_claude_pr_change_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    connections: State<'_, claude_connections::ClaudeConnections>,
    reviews: State<'_, provider::PendingProviderReviews>,
    connection_id: String,
    input: provider::BuildClaudePrChangeInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    build_operation_review(
        app,
        &sessions,
        &connections,
        &reviews,
        connection_id,
        ReviewInput::PrChange(input),
    )
    .await
}

enum ReviewInput {
    Chapter(provider::BuildClaudeReviewInput),
    CodeLens(provider::BuildClaudeCodeLensInput),
    PrGuide(provider::BuildClaudePrGuideInput),
    PrChange(provider::BuildClaudePrChangeInput),
}

/// Every offline operation uses one live native connection. The renderer
/// cannot provide repository roots, runtime/account observations or approval
/// evidence, and this function neither starts inference nor grants consent.
async fn build_operation_review(
    app: AppHandle,
    sessions: &ProjectSessions,
    connections: &claude_connections::ClaudeConnections,
    reviews: &provider::PendingProviderReviews,
    connection_id: String,
    input: ReviewInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    use provider::{ProviderCommandResult as Envelope, ProviderFailureKind as Kind};
    let lease = match connections.review_lease(&connection_id) {
        Ok(lease) => lease,
        Err(_) => {
            return Envelope::error(
                Kind::ReviewChanged,
                "Check the Claude catalog and prepare a review using its current native connection.",
            );
        }
    };
    let project_id = match &input {
        ReviewInput::Chapter(input) => &input.project_id,
        ReviewInput::CodeLens(input) => &input.project_id,
        ReviewInput::PrGuide(input) => &input.project_id,
        ReviewInput::PrChange(input) => &input.project_id,
    };
    let project = match sessions.get(project_id) {
        Ok(project) => project,
        Err(_) => {
            return Envelope::error(
                Kind::InvalidReview,
                "Reopen the selected repository before preparing a Claude review.",
            );
        }
    };
    let storage = match app_data_directory(&app) {
        Ok(storage) => storage,
        Err(_) => {
            return Envelope::error(
                Kind::LocalStorageUnavailable,
                "Local snapshot storage is unavailable. No Claude request was prepared.",
            );
        }
    };
    let runtime = lease.runtime.clone();
    let projects = sessions.clone();
    let pending = match tauri::async_runtime::spawn_blocking(move || match input {
        ReviewInput::Chapter(input) => {
            claude_generation::prepare_chapter_review(&storage, &projects, &runtime, &input)
        }
        ReviewInput::CodeLens(input) => {
            provider::build_claude_code_lens_review(&storage, &input, &runtime)
        }
        ReviewInput::PrGuide(input) => provider::build_claude_pr_guide_review(
            Path::new(&project.repository_root),
            &input,
            &runtime,
        ),
        ReviewInput::PrChange(input) => provider::build_claude_pr_change_review(
            Path::new(&project.repository_root),
            &input,
            &runtime,
        ),
    })
    .await
    {
        Ok(pending) => pending,
        Err(_) => {
            return Envelope::error(
                Kind::ReviewUnavailable,
                "The Claude source review could not be prepared. No agent process was started.",
            );
        }
    };
    match pending.and_then(|pending| connections.insert_review(&lease, reviews, pending)) {
        Ok(review) => Envelope::ok(review),
        Err(error) => Envelope::failed(error),
    }
}
