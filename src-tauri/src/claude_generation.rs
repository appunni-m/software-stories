//! Internal reviewed operation pipeline for a qualified native Claude runtime.
//!
//! This module neither qualifies nor constructs that runtime. A registered
//! command calls offline review preparation with native-owned observations.
//! The senders have no registered command or app caller: the permitted account,
//! installation/isolation owner and generation UI remain missing. A setup
//! catalog cannot become generation authorization. No direct API is used here.

use crate::{
    ProjectSessions, app_data_directory, chapters,
    claude_agent::ClaudeReviewRuntime,
    claude_worker::{
        ClaudeWorkerFailure, ClaudeWorkerFailureKind, ClaudeWorkerInput, ClaudeWorkerManager,
    },
    pr_guide, prepare_chapter_draft_preview, prepare_pr_guide_preview, provider, recovery,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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

/// Future qualified native caller supplies the runtime, not the renderer. A
/// consumed request produces an unaccepted in-memory preview only. Generic
/// Save/Accept operations still own persistence and accepted history.
pub(crate) async fn send_chapter(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    runtime: Arc<ClaudeReviewRuntime>,
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
    let mut result = match completed.result {
        Ok(response) => {
            prepare_chapter_draft_preview(&completed.storage, &completed.review, response, drafts)
                .map(Envelope::ok)
                .unwrap_or_else(Envelope::failed)
        }
        Err(error) => Envelope::failed(error),
    };
    // A crash/uncertain native cleanup keeps its original backend/scope notice.
    // Recovery must never reopen that notice as a direct API request.
    if completed.clear_interruption {
        if let Err(message) = interruptions.discard(&completed.storage, review_id) {
            match &mut result {
                Envelope::Ok { data } => data.update_notes.push(format!(
                    "Your validated preview is available. Its interruption notice could not be cleared: {message} Save or discard this preview before generating again.")),
                Envelope::Error { .. } => return Envelope::error(Kind::LocalStorageUnavailable,
                    "Claude stopped, but its interruption notice could not be cleared. Check local recovery state before preparing another request."),
            }
        }
    }
    result
}

struct CompletedOperation {
    storage: PathBuf,
    repository: PathBuf,
    review: provider::ProviderRequestReview,
    result: Result<provider::ProviderSendResult, provider::ProviderFailure>,
    clear_interruption: bool,
}

async fn execute(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    runtime: Arc<ClaudeReviewRuntime>,
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
    let pending = reviews.consume(review_id, fingerprint)?;
    interruptions.record(&storage, recovery::new_reviewed_provider_operation(&pending.review))
        .map_err(|_| failure(Kind::LocalStorageUnavailable,
            "Local interruption state could not be saved. No Claude request was sent; build a fresh review before retrying."))?;
    let review = pending.review;
    let input = ClaudeWorkerInput {
        run_id: review.review_id.clone(),
        settings,
        system_instruction: review.system_instruction.clone(),
        user_prompt: review.user_prompt.clone(),
        output_schema: review.generation_config["outputSchema"].clone(),
    };
    let worker = manager.clone();
    let storage_for_run = storage.clone();
    let review_for_run = review.clone();
    let repository_for_run = repository.clone();
    let run = tauri::async_runtime::spawn_blocking(move || {
        worker.run(&storage_for_run, &runtime, &input, || {
            provider::rebuild_claude_request_review(&storage_for_run, &repository_for_run, &review_for_run, &runtime)
                .map(|_| ())
                .map_err(|_| ClaudeWorkerFailure {
                    kind: ClaudeWorkerFailureKind::ReviewChanged,
                    message: "The reviewed Claude source, settings or operation changed before dispatch. Prepare a fresh review.".to_owned(),
                })
        }, |event| { let _ = app.emit("claude-agent-progress", event); })
    }).await;
    let (result, clear_interruption) = match run {
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
    Ok(CompletedOperation {
        storage,
        repository,
        review,
        result,
        clear_interruption,
    })
}

/// Candidate internal range sender. No registered command can reach it until
/// the native connection/account/isolation owner is qualified.
pub(crate) async fn send_code_lens(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    runtime: Arc<ClaudeReviewRuntime>,
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
        runtime.clone(),
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
    let (result, clean_validation) = validate_result(completed.result, move |response| {
        provider::rebuild_claude_request_review(&storage, &repository, &review, &runtime)?;
        provider::canonicalize_code_lens_explanation(&storage, &review, &response)
    })
    .await;
    // Range validation reads exact source but creates no chapter/PR history.
    finish_result(
        &completed.storage,
        interruptions,
        review_id,
        completed.clear_interruption && clean_validation,
        ClaudeOperation::CodeLens,
        result,
    )
}

pub(crate) async fn send_pr_guide(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    runtime: Arc<ClaudeReviewRuntime>,
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
        runtime.clone(),
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
    let (result, clean_validation) = validate_result(completed.result, move |response| {
        provider::rebuild_claude_request_review(&storage, &repository, &review, &runtime)?;
        // This saves only separate unaccepted PR Guide history, never an
        // accepted guide or a file/overview chapter revision.
        prepare_pr_guide_preview(&storage, &review, response)
    })
    .await;
    finish_result(
        &completed.storage,
        interruptions,
        review_id,
        completed.clear_interruption && clean_validation,
        ClaudeOperation::PrGuide,
        result,
    )
}

pub(crate) async fn send_pr_change(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &ClaudeWorkerManager,
    runtime: Arc<ClaudeReviewRuntime>,
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
        runtime.clone(),
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
    let (result, clean_validation) = validate_result(completed.result, move |response| {
        provider::canonicalize_claude_pr_change_explanation(
            &storage,
            &repository,
            &review,
            &response,
            &runtime,
        )
    })
    .await;
    // The common hunk canonicalizer writes no draft/history.
    finish_result(
        &completed.storage,
        interruptions,
        review_id,
        completed.clear_interruption && clean_validation,
        ClaudeOperation::PrChange,
        result,
    )
}

/// Keep blocking Git/source validation and any PR draft write off the async
/// runtime. A panicked result preparation has uncertain persistence and keeps
/// its original interruption notice; it must not trigger automatic generation.
async fn validate_result<T: Send + 'static>(
    response: Result<provider::ProviderSendResult, provider::ProviderFailure>,
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
                "Claude result preparation stopped unexpectedly. Check local recovery and PR Guide history before generating again; a draft may already be saved.",
            )),
            false,
        ),
    }
}

fn finish_result<T>(
    storage: &Path,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    clear_interruption: bool,
    operation: provider::ClaudeOperation,
    result: Result<T, provider::ProviderFailure>,
) -> provider::ProviderCommandResult<T> {
    use provider::{ProviderCommandResult as Envelope, ProviderFailureKind as Kind};
    if clear_interruption && interruptions.discard(storage, review_id).is_err() {
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
        Ok(preview) => Envelope::ok(preview),
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
