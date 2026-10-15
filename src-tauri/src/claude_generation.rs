//! Internal reviewed operation pipeline for a qualified native Claude runtime.
//!
//! This module neither qualifies nor constructs that runtime. A registered
//! command calls offline review preparation with native-owned observations.
//! Registered send commands require the release-owned runtime admission type.
//! The qualification registry is empty and app generation flows are unaccepted;
//! setup/catalog observations cannot authorize inference. No direct API is used.

use crate::{
    ProjectSessions, app_data_directory, chapters,
    claude_agent::ClaudeReviewRuntime,
    claude_connections::{ClaudeConnections, ClaudeOperationAccess, ClaudeOperationLease},
    claude_runtime::ClaudeGenerationRuntime,
    claude_worker::{
        ClaudeWorkerFailure, ClaudeWorkerFailureKind, ClaudeWorkerInput, ClaudeWorkerManager,
    },
    pr_guide, prepare_chapter_draft_preview, prepare_pr_guide_preview, provider, recovery,
};
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

/// Offline entry with a native-owned observed connection, never a WebView
/// runtime DTO. The command owner rechecks that connection before storing the
/// review. Run on the blocking pool; this creates no model turn or consent.
pub(crate) fn prepare_chapter_review(
    storage: &Path,
    sessions: &ProjectSessions,
    runtime: &ClaudeReviewRuntime,
    input: &provider::BuildClaudeReviewInput,
) -> Result<provider::PendingProviderRequest, provider::ProviderFailure> {
    sessions.get(&input.project_id).map_err(|_| {
        failure(
            provider::ProviderFailureKind::InvalidReview,
            "Reopen the selected repository before reviewing this Claude operation.",
        )
    })?;
    provider::build_claude_chapter_review(storage, input, runtime)
}

/// The native command supplies an admitted runtime, never the renderer. A
/// consumed request produces an unaccepted in-memory preview only. Generic
/// Save/Accept operations still own persistence and accepted history.
pub(crate) async fn send_chapter(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    connections: &ClaudeConnections,
    runtime: ClaudeGenerationRuntime,
    reviews: &provider::PendingProviderReviews,
    drafts: &chapters::PendingChapterDrafts,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    fingerprint: &str,
    consent: bool,
) -> provider::ProviderCommandResult<chapters::ChapterDraftPreview> {
    use provider::{ProviderCommandResult as Envelope, ProviderFailureKind as Kind};
    let completed = match execute(
        app,
        sessions,
        manager,
        connections,
        runtime,
        reviews,
        interruptions,
        review_id,
        fingerprint,
        consent,
        provider::ClaudeOperation::Chapter,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return Envelope::failed(error),
    };
    let access = completed.lease.access();
    let storage = completed.storage.clone();
    let repository = completed.repository.clone();
    let review = completed.review.clone();
    let validation_access = access.clone();
    let drafts = drafts.clone();
    let (result, clean_validation) = validate_result(
        completed.result,
        provider::ClaudeOperation::Chapter,
        move |response| {
            provider::rebuild_active_claude_review(
                &storage,
                &repository,
                &review,
                &validation_access,
            )?;
            validation_access.publish(&review, || {
                prepare_chapter_draft_preview(&storage, &review, response, &drafts)
            })
        },
    )
    .await;
    let mut result = result.map(Envelope::ok).unwrap_or_else(Envelope::failed);
    // A crash/uncertain native cleanup keeps its original backend/scope notice.
    // Recovery must never reopen that notice as a direct API request.
    if completed.clear_interruption && clean_validation {
        if let Err(message) = interruptions.discard(&completed.storage, review_id) {
            match &mut result {
                Envelope::Ok { data } => data.update_notes.push(format!(
                    "Your validated preview is available. Its interruption notice could not be cleared: {message} Save or discard this preview before generating again.")),
                Envelope::Error { .. } => return Envelope::error(Kind::LocalStorageUnavailable,
                    "Claude stopped, but its interruption notice could not be cleared. Check local recovery state before preparing another request."),
            }
        }
    }
    match result {
        Envelope::Ok { data } => access.publish(&completed.review, || Ok(data))
            .map(Envelope::ok).unwrap_or_else(|mut error| {
                error.message.push_str(" Check pending previews and local recovery before generating again; an unaccepted preview may already exist.");
                Envelope::failed(error)
            }),
        error => error,
    }
}

struct CompletedOperation {
    lease: ClaudeOperationLease,
    storage: PathBuf,
    repository: PathBuf,
    review: provider::ProviderRequestReview,
    result: Result<provider::ProviderSendResult, provider::ProviderFailure>,
    clear_interruption: bool,
}

/// Planning uses the same admitted runtime and revocable operation lease.
/// Its result is saved as an unaccepted plan under the publication lock. Retain
/// the exact journal notice until explicit app review/discard resolves it.
pub(crate) async fn send_repository_plan(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    connections: &ClaudeConnections,
    runtime: ClaudeGenerationRuntime,
    reviews: &provider::PendingProviderReviews,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    fingerprint: &str,
    consent: bool,
) -> provider::ProviderCommandResult<provider::RepositoryPlanPreview> {
    use provider::{ClaudeOperation, ProviderCommandResult as Envelope};
    let completed = match execute(
        app,
        sessions,
        manager,
        connections,
        runtime,
        reviews,
        interruptions,
        review_id,
        fingerprint,
        consent,
        ClaudeOperation::RepositoryPlan,
    )
    .await
    {
        Ok(completed) => completed,
        Err(error) => return Envelope::failed(error),
    };
    let access = completed.lease.access();
    let validation_access = access.clone();
    let storage = completed.storage.clone();
    let repository = completed.repository.clone();
    let review = completed.review.clone();
    let (result, _) = validate_result(
        completed.result,
        ClaudeOperation::RepositoryPlan,
        move |response| {
            provider::rebuild_active_claude_review(
                &storage,
                &repository,
                &review,
                &validation_access,
            )?;
            let candidate =
                crate::repository_plan_store::prepare_save(&storage, &review, &response)?;
            validation_access.publish(&review, || {
                crate::repository_plan_store::publish(&storage, candidate)
            })
        },
    )
    .await;
    match result {
        Ok(plan) => access
            .publish(&completed.review, || Ok(plan))
            .map(Envelope::ok)
            .unwrap_or_else(|mut error| {
                error.message.push_str(" An unaccepted plan may already be saved. Inspect saved plans and recovery before generating again.");
                Envelope::failed(error)
            }),
        Err(error) => Envelope::failed(error),
    }
}

async fn execute(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    connections: &ClaudeConnections,
    runtime: ClaudeGenerationRuntime,
    reviews: &provider::PendingProviderReviews,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    fingerprint: &str,
    consent: bool,
    operation: provider::ClaudeOperation,
) -> Result<CompletedOperation, provider::ProviderFailure> {
    use provider::ProviderFailureKind as Kind;
    if !consent {
        return Err(failure(
            Kind::ConsentRequired,
            "Review the exact source, effective Claude settings and account usage before explicitly starting this request.",
        ));
    }
    let runtime = runtime.into_review_runtime();
    let pending = reviews.get(review_id)?;
    if pending.review.fingerprint != fingerprint
        || provider::claude_review_operation(&pending.review)? != operation
    {
        return Err(failure(
            Kind::ReviewChanged,
            "Build a fresh Claude review for this exact operation with the selected source and settings.",
        ));
    }
    let project = sessions.get(&pending.review.project_id).map_err(|_| {
        failure(
            Kind::InvalidReview,
            "Reopen the selected repository before reviewing this Claude operation again.",
        )
    })?;
    let repository = PathBuf::from(project.repository_root);
    let storage = app_data_directory(&app).map_err(|_| {
        failure(
            Kind::LocalStorageUnavailable,
            "Local snapshot storage is unavailable. No Claude request was sent.",
        )
    })?;
    // Release-owned runtime admission has already succeeded. This reservation
    // additionally owns the revocable connection lifecycle and exact review.
    let lease = connections.begin_operation(&runtime, &pending.review)?;
    let access = lease.access();
    let storage_for_check = storage.clone();
    let runtime_for_check = runtime.clone();
    let review_for_check = pending.review.clone();
    let repository_for_check = repository.clone();
    let validated = tauri::async_runtime::spawn_blocking(move || {
        provider::rebuild_claude_request_review(
            &storage_for_check,
            &repository_for_check,
            &review_for_check,
            &runtime_for_check,
        )
        .map(|_| ())
    })
    .await
    .map_err(|_| {
        failure(
            Kind::ReviewUnavailable,
            "The Claude review could not be revalidated. Build a fresh review.",
        )
    })?;
    if let Err(error) = validated {
        reviews.remove(review_id);
        return Err(error);
    }
    // Effective settings come solely from the stored review. No per-send
    // model/effort/approval override or direct API body can enter this path.
    let settings = provider::claude_settings_from_review(&pending.review)?;
    runtime
        .validate_settings(&settings)
        .map_err(|message| failure(Kind::InvalidReview, message))?;
    let pending = access.consume_review(&pending.review, reviews, interruptions, &storage)?;
    let review = pending.review;
    let input = ClaudeWorkerInput {
        run_id: review.review_id.clone(),
        cancel_signal: access.cancel_signal(),
        started_at: access.started_at()?,
        settings,
        system_instruction: review.system_instruction.clone(),
        user_prompt: review.user_prompt.clone(),
        output_schema: review.generation_config["outputSchema"].clone(),
    };
    let worker = manager.clone();
    let storage_for_run = storage.clone();
    let review_for_run = review.clone();
    let repository_for_run = repository.clone();
    let access_for_run = access.clone();
    let run = tauri::async_runtime::spawn_blocking(move || {
        worker.run(&storage_for_run, &runtime, &input, || {
            provider::rebuild_active_claude_review(&storage_for_run, &repository_for_run, &review_for_run, &access_for_run)
                .map(|_| ())
                .map_err(|_| ClaudeWorkerFailure {
                    kind: ClaudeWorkerFailureKind::ReviewChanged,
                    message: "The reviewed Claude source, settings or operation changed before dispatch. Prepare a fresh review.".to_owned(),
                })
        }, |event| { let _ = app.emit("claude-agent-progress", event); })
    }).await;
    let (mut result, clear_interruption) = match run {
        Ok(Ok(response)) => (
            Ok(provider::ProviderSendResult {
                review_id: review.review_id.clone(),
                fingerprint: review.fingerprint.clone(),
                model_id: response.model_id,
                model_version: None,
                response_text: response.response_text,
                input_tokens: None,
                output_tokens: None,
            }),
            true,
        ),
        Ok(Err(error)) => {
            let clean = !matches!(error.kind, ClaudeWorkerFailureKind::CleanupFailed);
            (Err(worker_failure(error)), clean)
        }
        Err(_) => {
            manager.shutdown();
            (
                Err(failure(
                    Kind::ProviderRejectedRequest,
                    "The Claude worker stopped unexpectedly. Native cleanup is unconfirmed; resolve it before restarting the app and reviewing another request.",
                )),
                false,
            )
        }
    };
    // Native cleanup uncertainty takes priority. Otherwise a revoked owner
    // cannot validate/publish the worker's late result, even after clean exit.
    if clear_interruption {
        if let Err(error) = access.check_review(&review) {
            result = Err(error);
        }
    }
    Ok(CompletedOperation {
        lease,
        storage,
        repository,
        review,
        result,
        clear_interruption,
    })
}

/// Range sender reached only after native release-owned admission. The empty
/// qualification registry currently keeps every app invocation unavailable.
pub(crate) async fn send_code_lens(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    connections: &ClaudeConnections,
    runtime: ClaudeGenerationRuntime,
    reviews: &provider::PendingProviderReviews,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    fingerprint: &str,
    consent: bool,
) -> provider::ProviderCommandResult<provider::CodeLensExplanationPreview> {
    use provider::{ClaudeOperation, ProviderCommandResult as Envelope};
    let completed = match execute(
        app,
        sessions,
        manager,
        connections,
        runtime,
        reviews,
        interruptions,
        review_id,
        fingerprint,
        consent,
        ClaudeOperation::CodeLens,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return Envelope::failed(error),
    };
    let storage = completed.storage.clone();
    let repository = completed.repository.clone();
    let review = completed.review.clone();
    let access = completed.lease.access();
    let validation_access = access.clone();
    let (result, clean_validation) = validate_result(
        completed.result,
        provider::ClaudeOperation::CodeLens,
        move |response| {
            provider::rebuild_active_claude_review(
                &storage,
                &repository,
                &review,
                &validation_access,
            )?;
            provider::canonicalize_code_lens_explanation(&storage, &review, &response)
        },
    )
    .await;
    // Range validation reads exact source but creates no chapter/PR history.
    finish_result(
        &completed.storage,
        interruptions,
        &completed.review,
        &access,
        completed.clear_interruption && clean_validation,
        ClaudeOperation::CodeLens,
        result,
    )
}

pub(crate) async fn send_pr_guide(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    connections: &ClaudeConnections,
    runtime: ClaudeGenerationRuntime,
    reviews: &provider::PendingProviderReviews,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    fingerprint: &str,
    consent: bool,
) -> provider::ProviderCommandResult<pr_guide::PrGuidePreview> {
    use provider::{ClaudeOperation, ProviderCommandResult as Envelope};
    let completed = match execute(
        app,
        sessions,
        manager,
        connections,
        runtime,
        reviews,
        interruptions,
        review_id,
        fingerprint,
        consent,
        ClaudeOperation::PrGuide,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return Envelope::failed(error),
    };
    let storage = completed.storage.clone();
    let repository = completed.repository.clone();
    let review = completed.review.clone();
    let access = completed.lease.access();
    let validation_access = access.clone();
    let (result, clean_validation) = validate_result(
        completed.result,
        provider::ClaudeOperation::PrGuide,
        move |response| {
            provider::rebuild_active_claude_review(
                &storage,
                &repository,
                &review,
                &validation_access,
            )?;
            // This saves only separate unaccepted PR Guide history, never an
            // accepted guide or a file/overview chapter revision.
            validation_access.publish(&review, || {
                prepare_pr_guide_preview(&storage, &review, response)
            })
        },
    )
    .await;
    finish_result(
        &completed.storage,
        interruptions,
        &completed.review,
        &access,
        completed.clear_interruption && clean_validation,
        ClaudeOperation::PrGuide,
        result,
    )
}

pub(crate) async fn send_pr_change(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    connections: &ClaudeConnections,
    runtime: ClaudeGenerationRuntime,
    reviews: &provider::PendingProviderReviews,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    fingerprint: &str,
    consent: bool,
) -> provider::ProviderCommandResult<provider::PrChangeExplanationPreview> {
    use provider::{ClaudeOperation, ProviderCommandResult as Envelope};
    let completed = match execute(
        app,
        sessions,
        manager,
        connections,
        runtime,
        reviews,
        interruptions,
        review_id,
        fingerprint,
        consent,
        ClaudeOperation::PrChange,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => return Envelope::failed(error),
    };
    let storage = completed.storage.clone();
    let repository = completed.repository.clone();
    let review = completed.review.clone();
    let access = completed.lease.access();
    let validation_access = access.clone();
    let (result, clean_validation) = validate_result(
        completed.result,
        provider::ClaudeOperation::PrChange,
        move |response| {
            provider::canonicalize_claude_pr_change_explanation(
                &storage,
                &repository,
                &review,
                &response,
                &validation_access,
            )
        },
    )
    .await;
    // The common hunk canonicalizer writes no draft/history.
    finish_result(
        &completed.storage,
        interruptions,
        &completed.review,
        &access,
        completed.clear_interruption && clean_validation,
        ClaudeOperation::PrChange,
        result,
    )
}

/// Keep blocking Git/source validation and chapter/PR draft writes off the async
/// runtime. A panicked result preparation has uncertain persistence and keeps
/// its original interruption notice; it must not trigger automatic generation.
async fn validate_result<T: Send + 'static>(
    response: Result<provider::ProviderSendResult, provider::ProviderFailure>,
    operation: provider::ClaudeOperation,
    validate: impl FnOnce(provider::ProviderSendResult) -> Result<T, provider::ProviderFailure>
    + Send
    + 'static,
) -> (Result<T, provider::ProviderFailure>, bool) {
    let response = match response {
        Ok(response) => response,
        Err(error) => return (Err(error), true),
    };
    match tauri::async_runtime::spawn_blocking(move || validate(response)).await {
        Ok(result) => (result, true),
        Err(_) => (
            Err(failure(
                provider::ProviderFailureKind::LocalStorageUnavailable,
                match operation {
                    provider::ClaudeOperation::RepositoryPlan => {
                        "Claude plan validation stopped unexpectedly. The original planning question and source selection remain in recovery; no plan was saved."
                    }
                    provider::ClaudeOperation::Chapter => {
                        "Claude chapter preparation stopped unexpectedly. Check pending previews and local recovery before generating again; an unaccepted preview may already exist."
                    }
                    provider::ClaudeOperation::PrGuide => {
                        "Claude guide preparation stopped unexpectedly. Check PR Guide history and local recovery before generating again; an unaccepted draft may already be saved."
                    }
                    provider::ClaudeOperation::CodeLens | provider::ClaudeOperation::PrChange => {
                        "Claude explanation validation stopped unexpectedly. Keep the original recovery notice and reselect its exact source range or Git hunk before a new request."
                    }
                },
            )),
            false,
        ),
    }
}

fn finish_result<T>(
    storage: &Path,
    interruptions: &recovery::InterruptionJournal,
    review: &provider::ProviderRequestReview,
    access: &ClaudeOperationAccess,
    clear_interruption: bool,
    operation: provider::ClaudeOperation,
    result: Result<T, provider::ProviderFailure>,
) -> provider::ProviderCommandResult<T> {
    use provider::{ProviderCommandResult as Envelope, ProviderFailureKind as Kind};
    if clear_interruption && interruptions.discard(storage, &review.review_id).is_err() {
        return Envelope::error(
            Kind::LocalStorageUnavailable,
            if operation == provider::ClaudeOperation::PrGuide && result.is_ok() {
                "The validated PR Guide draft is saved in PR Guide history, but its interruption notice could not be cleared. Open that draft before considering another generation."
            } else {
                "Claude stopped, but its interruption notice could not be cleared. Check local recovery and reselect the exact source range or Git comparison before preparing another request."
            },
        );
    }
    match result {
        Ok(preview) => access.publish(review, || Ok(preview))
            .map(Envelope::ok).unwrap_or_else(|mut error| {
                if operation == provider::ClaudeOperation::PrGuide {
                    error.message.push_str(" Check PR Guide history before generating again; an unaccepted draft may already be saved.");
                }
                Envelope::failed(error)
            }),
        Err(error) => Envelope::failed(error),
    }
}

fn failure(
    kind: provider::ProviderFailureKind,
    message: impl Into<String>,
) -> provider::ProviderFailure {
    provider::ProviderFailure {
        kind,
        message: message.into(),
    }
}

fn worker_failure(error: ClaudeWorkerFailure) -> provider::ProviderFailure {
    use ClaudeWorkerFailureKind as Worker;
    use provider::ProviderFailureKind as Kind;
    let kind = match error.kind {
        Worker::Cancelled => Kind::RequestCancelled,
        Worker::TimedOut => Kind::RequestTimedOut,
        Worker::RuntimeChanged | Worker::ReviewChanged => Kind::ReviewChanged,
        Worker::Busy | Worker::PreparationFailed | Worker::UnsupportedPlatform => {
            Kind::ReviewUnavailable
        }
        Worker::OutputLimit => Kind::ProviderResponseTooLarge,
        Worker::ProtocolRejected => Kind::MalformedProviderResponse,
        Worker::ProcessFailed | Worker::CleanupFailed => Kind::ProviderRejectedRequest,
    };
    // Only the worker's fixed native messages cross this boundary, never its
    // raw process output, SDK errors, account responses or credential fields.
    failure(kind, error.message)
}
