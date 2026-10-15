//! Internal reviewed chapter pipeline for a qualified native Claude runtime.
//!
//! This module neither qualifies nor constructs that runtime. It has no Tauri
//! registration or app caller: the permitted account/install/isolation owner
//! and generation UI remain missing. A setup catalog cannot call this module
//! or become generation authorization. No direct API transport is used here.

use crate::{
    ProjectSessions, app_data_directory, chapters,
    claude_agent::ClaudeReviewRuntime,
    claude_worker::{
        ClaudeWorkerFailure, ClaudeWorkerFailureKind, ClaudeWorkerInput, ClaudeWorkerManager,
    },
    prepare_chapter_draft_preview, provider, recovery,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

/// Native-only entry. The caller must qualify the runtime/account route and
/// obtain fresh native capabilities first; no WebView runtime DTO is accepted.
/// Run on the blocking pool; source review creates no model turn or consent.
pub(crate) fn build_chapter_review(
    storage: &Path,
    sessions: &ProjectSessions,
    reviews: &provider::PendingProviderReviews,
    runtime: &ClaudeReviewRuntime,
    input: &provider::BuildClaudeReviewInput,
) -> Result<provider::ProviderRequestReview, provider::ProviderFailure> {
    sessions.get(&input.project_id).map_err(|_| {
        failure(
            provider::ProviderFailureKind::InvalidReview,
            "Reopen the selected repository before reviewing this Claude operation.",
        )
    })?;
    reviews.insert(provider::build_claude_chapter_review(
        storage, input, runtime,
    )?)
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
) -> Result<CompletedOperation, provider::ProviderFailure> {
    use provider::ProviderFailureKind as Kind;
    if !consent {
        return Err(failure(
            Kind::ConsentRequired,
            "Review the exact source, effective Claude settings and account usage before explicitly starting this request.",
        ));
    }
    let pending = reviews.get(review_id)?;
    if pending.review.fingerprint != fingerprint || !accepts_chapter(&pending.review) {
        return Err(failure(
            Kind::ReviewChanged,
            "Build a fresh Claude chapter, overview or one-section review with the selected source and settings.",
        ));
    }
    sessions.get(&pending.review.project_id).map_err(|_| {
        failure(
            Kind::InvalidReview,
            "Reopen the selected repository before reviewing this Claude operation again.",
        )
    })?;
    let storage = app_data_directory(&app).map_err(|_| {
        failure(
            Kind::LocalStorageUnavailable,
            "Local snapshot storage is unavailable. No Claude request was sent.",
        )
    })?;
    let storage_for_check = storage.clone();
    let runtime_for_check = runtime.clone();
    let review_for_check = pending.review.clone();
    let validated = tauri::async_runtime::spawn_blocking(move || {
        provider::revalidate_claude_chapter_review(
            &storage_for_check,
            &review_for_check,
            &runtime_for_check,
        )
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
    let run = tauri::async_runtime::spawn_blocking(move || {
        worker.run(&storage_for_run, &runtime, &input, || {
            provider::revalidate_claude_chapter_review(&storage_for_run, &review_for_run, &runtime)
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
        review,
        result,
        clear_interruption,
    })
}

fn accepts_chapter(review: &provider::ProviderRequestReview) -> bool {
    review.provider_id == "claude-agent"
        && review.pr_guide_context.is_none()
        && review.pr_change_context.is_none()
        && review.code_lens_context.is_none()
        && match review.purpose.as_str() {
            "fileChapter" => {
                review.target_path.is_some() && review.section_update_context.is_none()
            }
            "repositoryOverview" => {
                review.target_path.is_none() && review.section_update_context.is_none()
            }
            "sectionUpdate" => {
                review.target_path.is_some() && review.section_update_context.is_some()
            }
            _ => false,
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
