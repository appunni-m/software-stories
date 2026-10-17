//! Native account-backed Codex commands. Immutable explanation evidence and
//! local validators are shared with API workflows; authentication is separate.

use super::{
    PreviewPublication, ProjectSessions, app_data_directory, chapters, codex_agent, pr_guide,
    provider, publish_chapter_candidate, publish_pr_guide_candidate, recovery,
    validate_chapter_draft_preview, validate_pr_guide_preview,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tauri::{AppHandle, Emitter, State};

async fn on_runtime<T: Send + 'static>(
    app: AppHandle,
    manager: codex_agent::CodexAgentManager,
    operation: impl FnOnce(&codex_agent::CodexAgentManager, &Path) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || operation(&manager, &storage))
        .await
        .map_err(|_| {
            "The Codex operation stopped unexpectedly. Existing chapters are unchanged.".to_owned()
        })?
}

#[tauri::command]
pub(crate) async fn get_codex_agent_status(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
) -> Result<codex_agent::CodexAgentStatus, String> {
    on_runtime(app, manager.inner().clone(), |manager, storage| {
        manager.status(storage)
    })
    .await
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn configure_codex_agent(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
    executable_path: String,
) -> Result<codex_agent::CodexAgentStatus, String> {
    on_runtime(app, manager.inner().clone(), move |manager, storage| {
        manager.configure(storage, &executable_path)
    })
    .await
}

/// Reuse installed credentials without starting OAuth or opening a browser.
#[tauri::command]
pub(crate) async fn resume_codex_connection(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
) -> Result<codex_agent::CodexAgentStatus, String> {
    on_runtime(app, manager.inner().clone(), |manager, storage| {
        manager.resume_connection(storage)
    })
    .await
}

#[tauri::command]
pub(crate) async fn start_codex_login(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
) -> Result<codex_agent::CodexLoginStart, String> {
    on_runtime(app, manager.inner().clone(), |manager, storage| {
        let login = manager.start_login(storage)?;
        // The runtime validates the OAuth destination. Open only that returned
        // URL in the system browser, never in the repository WebView.
        #[cfg(target_os = "macos")]
        if let (Some(auth_url), Some(login_id)) = (&login.auth_url, &login.login_id) {
            let result = open_login_browser(auth_url);
            if result.is_err() {
                let _ = manager.cancel_login(storage, login_id);
                return Err("The system browser could not open Codex sign-in. Retry Connect after choosing a default browser.".to_owned());
            }
        }
        Ok(login)
    }).await
}

#[cfg(target_os = "macos")]
fn open_login_browser(url: &str) -> Result<(), ()> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};
    let mut child = Command::new("/usr/bin/open")
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| ())?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return if status.success() { Ok(()) } else { Err(()) },
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(());
            }
        }
    }
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn cancel_codex_login(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
    login_id: String,
) -> Result<(), String> {
    on_runtime(app, manager.inner().clone(), move |manager, storage| {
        manager.cancel_login(storage, &login_id)
    })
    .await
}

#[tauri::command]
pub(crate) async fn logout_codex_agent(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
) -> Result<(), String> {
    on_runtime(app, manager.inner().clone(), |manager, storage| {
        manager.logout(storage)
    })
    .await
}

#[tauri::command]
pub(crate) async fn list_codex_models(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
) -> Result<Vec<codex_agent::CodexModel>, String> {
    on_runtime(app, manager.inner().clone(), |manager, storage| {
        manager.models(storage)
    })
    .await
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_codex_chapter_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    input: provider::BuildCodexReviewInput,
) -> Result<provider::ProviderCommandResult<provider::ProviderRequestReview>, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    let runtime = manager.inner().clone();
    let pending = tauri::async_runtime::spawn_blocking(move || {
        let identity =
            runtime
                .review_runtime(&storage)
                .map_err(|message| provider::ProviderFailure {
                    kind: provider::ProviderFailureKind::ReviewUnavailable,
                    message,
                })?;
        provider::build_codex_chapter_review(&storage, &input, &identity)
    })
    .await
    .map_err(|_| "Codex request review could not be prepared.".to_owned())?;
    Ok(match pending.and_then(|pending| reviews.insert(pending)) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_codex_planned_chapter_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    input: provider::BuildCodexPlannedChapterInput,
) -> Result<provider::ProviderCommandResult<provider::ProviderRequestReview>, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    let runtime = manager.inner().clone();
    let pending = tauri::async_runtime::spawn_blocking(move || {
        let identity =
            runtime
                .review_runtime(&storage)
                .map_err(|message| provider::ProviderFailure {
                    kind: provider::ProviderFailureKind::ReviewUnavailable,
                    message,
                })?;
        provider::build_codex_planned_chapter_review(&storage, &input, &identity)
    })
    .await
    .map_err(|_| "Codex plan-step chapter review could not be prepared.".to_owned())?;
    Ok(match pending.and_then(|pending| reviews.insert(pending)) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_codex_repository_plan_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    input: provider::BuildCodexRepositoryPlanInput,
) -> Result<provider::ProviderCommandResult<provider::ProviderRequestReview>, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    let runtime = manager.inner().clone();
    let pending = tauri::async_runtime::spawn_blocking(move || {
        let identity = runtime.review_runtime(&storage).map_err(|message| {
            command_failure(provider::ProviderFailureKind::ReviewUnavailable, message)
        })?;
        provider::build_codex_repository_plan_review(
            &storage, &input, &identity,
        )
    }).await.map_err(|_| "The Codex planning review could not be prepared. Existing plans and chapters were preserved.".to_owned())?;
    Ok(match pending.and_then(|pending| reviews.insert(pending)) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_codex_code_lens_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    input: provider::BuildCodexCodeLensInput,
) -> Result<provider::ProviderCommandResult<provider::ProviderRequestReview>, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    let runtime = manager.inner().clone();
    let pending = tauri::async_runtime::spawn_blocking(move || {
        let identity = runtime.review_runtime(&storage).map_err(|message| {
            command_failure(provider::ProviderFailureKind::ReviewUnavailable, message)
        })?;
        provider::build_codex_code_lens_review(&storage, &input, &identity)
    })
    .await
    .map_err(|_| "Codex source range review could not be prepared.".to_owned())?;
    Ok(match pending.and_then(|pending| reviews.insert(pending)) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_codex_pr_guide_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    input: provider::BuildCodexPrGuideInput,
) -> Result<provider::ProviderCommandResult<provider::ProviderRequestReview>, String> {
    build_git_review(
        app,
        &sessions,
        &manager,
        &reviews,
        GitReviewInput::Guide(input),
    )
    .await
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn build_codex_pr_change_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    input: provider::BuildCodexPrChangeInput,
) -> Result<provider::ProviderCommandResult<provider::ProviderRequestReview>, String> {
    build_git_review(
        app,
        &sessions,
        &manager,
        &reviews,
        GitReviewInput::Change(input),
    )
    .await
}

enum GitReviewInput {
    Guide(provider::BuildCodexPrGuideInput),
    Change(provider::BuildCodexPrChangeInput),
}

async fn build_git_review(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &codex_agent::CodexAgentManager,
    reviews: &provider::PendingProviderReviews,
    input: GitReviewInput,
) -> Result<provider::ProviderCommandResult<provider::ProviderRequestReview>, String> {
    let project_id = match &input {
        GitReviewInput::Guide(input) => &input.project_id,
        GitReviewInput::Change(input) => &input.project_id,
    };
    let project = sessions.get(project_id)?;
    let storage = app_data_directory(&app)?;
    let runtime = manager.clone();
    let pending = tauri::async_runtime::spawn_blocking(move || {
        let identity = runtime.review_runtime(&storage).map_err(|message| {
            command_failure(provider::ProviderFailureKind::ReviewUnavailable, message)
        })?;
        match input {
            GitReviewInput::Guide(input) => provider::build_codex_pr_guide_review(
                Path::new(&project.repository_root),
                &input,
                &identity,
            ),
            GitReviewInput::Change(input) => provider::build_codex_pr_change_review(
                Path::new(&project.repository_root),
                &input,
                &identity,
            ),
        }
    })
    .await
    .map_err(|_| "The Codex Git-change review could not be prepared.".to_owned())?;
    Ok(match pending.and_then(|pending| reviews.insert(pending)) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn cancel_codex_chapter_request(
    manager: State<'_, codex_agent::CodexAgentManager>,
    run_id: String,
) -> bool {
    manager.cancel(&run_id)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn send_codex_chapter_request(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    drafts: State<'_, chapters::PendingChapterDrafts>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    review_id: String,
    fingerprint: String,
    consent: bool,
) -> Result<provider::ProviderCommandResult<chapters::ChapterDraftPreview>, String> {
    use provider::{ProviderCommandResult as ResultEnvelope, ProviderFailureKind as Kind};
    let chapter_drafts = drafts.inner().clone();
    let completed = match execute_codex_operation(
        app,
        &sessions,
        &manager,
        &reviews,
        &interruptions,
        &review_id,
        &fingerprint,
        consent,
        CodexOperation::Chapter,
        |storage, _, review, response, _| {
            let mut candidate = validate_chapter_draft_preview(storage, review, response)?;
            super::attach_agent_generation_origin(&mut candidate, review)?;
            Ok(candidate)
        },
        move |storage, candidate| {
            publish_chapter_candidate(
                storage,
                candidate,
                &chapter_drafts,
                PreviewPublication::AgentOwned,
            )
        },
    )
    .await
    {
        Ok(completed) => completed,
        Err(error) => return Ok(ResultEnvelope::failed(error)),
    };
    let mut result = completed
        .result
        .map(ResultEnvelope::ok)
        .unwrap_or_else(ResultEnvelope::failed);
    if completed.clear_interruption {
        if let Err(message) = interruptions.discard(&completed.storage, &review_id) {
            match &mut result {
            ResultEnvelope::Ok { data } => data.update_notes.push(format!(
                "Your validated preview is available. Its interruption notice could not be cleared: {message} Save or discard this preview before generating again.")),
            ResultEnvelope::Error { .. } => return Ok(ResultEnvelope::error(Kind::LocalStorageUnavailable,
                format!("Codex stopped, but its recovery notice could not be cleared: {message} Check saved previews before generating again."))),
        }
        }
    }
    Ok(result)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn send_codex_code_lens_request(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    review_id: String,
    fingerprint: String,
    consent: bool,
) -> Result<provider::ProviderCommandResult<provider::CodeLensExplanationPreview>, String> {
    use provider::{ProviderCommandResult as ResultEnvelope, ProviderFailureKind as Kind};
    let completed = match execute_codex_operation(
        app,
        &sessions,
        &manager,
        &reviews,
        &interruptions,
        &review_id,
        &fingerprint,
        consent,
        CodexOperation::CodeLens,
        |storage, _, review, response, _| {
            provider::canonicalize_code_lens_explanation(storage, review, &response)
        },
        |_, preview| Ok(preview),
    )
    .await
    {
        Ok(completed) => completed,
        Err(error) => return Ok(ResultEnvelope::failed(error)),
    };
    let result = completed.result;
    if completed.clear_interruption
        && interruptions
            .discard(&completed.storage, &review_id)
            .is_err()
    {
        return Ok(ResultEnvelope::error(
            Kind::LocalStorageUnavailable,
            "Codex stopped, but its interruption notice could not be cleared. Select the exact source range and review a new request before trying again.",
        ));
    }
    Ok(match result {
        Ok(preview) => ResultEnvelope::ok(preview),
        Err(error) => ResultEnvelope::failed(error),
    })
}

/// A separate planning operation never enters chapter storage or an API
/// credential path. Its journal notice remains until explicit resolution.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SendCodexPlanningRequest {
    review_id: String,
    fingerprint: String,
    consent: bool,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn send_codex_repository_plan_request(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    input: SendCodexPlanningRequest,
) -> Result<provider::ProviderCommandResult<provider::RepositoryPlanPreview>, String> {
    use provider::{ProviderCommandResult as Envelope, ProviderFailureKind as Kind};
    let SendCodexPlanningRequest {
        review_id,
        fingerprint,
        consent,
    } = input;
    let prepared = match prepare_codex_operation(
        &app,
        &sessions,
        &manager,
        &reviews,
        &interruptions,
        &review_id,
        &fingerprint,
        consent,
        CodexOperation::RepositoryPlan,
    )
    .await
    {
        Ok(prepared) => prepared,
        Err(error) => return Ok(Envelope::failed(error)),
    };
    let mut owner = CodexCommandOwner {
        manager: manager.inner().clone(),
        run_id: review_id,
        armed: true,
    };
    let runtime = manager.inner().clone();
    let run = tauri::async_runtime::spawn_blocking(move || {
        let PreparedCodexOperation { storage, review, input, started_at, .. } = prepared;
        runtime.run_with_result_and_publish(
            &storage, input, &review.fingerprint, started_at,
            |event| { let _ = app.emit("codex-agent-progress", event); },
            |response, access| {
                let _ = app.emit("codex-agent-progress", codex_agent::CodexProgress {
                    run_id: review.review_id.clone(), phase: "validating".to_owned(),
                    thread_id: Some(response.thread_id.clone()), turn_id: Some(response.turn_id.clone()),
                    message: "Checking the plan against the original learning question and captured source.".to_owned(),
                });
                provider::rebuild_active_codex_repository_plan_review(&storage, &review, access)?;
                crate::repository_plan_store::prepare_save(
                    &storage, &review,
                    &provider::ProviderSendResult {
                        review_id: review.review_id.clone(),
                        fingerprint: review.fingerprint.clone(),
                        model_id: response.model,
                        model_version: None,
                        response_text: response.response_text,
                        input_tokens: None,
                        output_tokens: None,
                    },
                )
            },
            |candidate| candidate.and_then(|candidate| crate::repository_plan_store::publish(&storage, candidate)),
        )
    }).await;
    owner.armed = false;
    Ok(match run {
        Ok(Ok(Ok(plan))) => Envelope::ok(plan),
        Ok(Ok(Err(error))) => Envelope::failed(error),
        Ok(Err(message)) => {
            Envelope::failed(command_failure(codex_run_failure_kind(&message), message))
        }
        Err(_) => {
            manager.shutdown();
            Envelope::error(
                Kind::ProviderRejectedRequest,
                "The Codex planning worker stopped unexpectedly. Its original question and source selection remain in recovery; restart the app before reviewing another request.",
            )
        }
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn send_codex_pr_guide_request(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    review_id: String,
    fingerprint: String,
    consent: bool,
) -> Result<provider::ProviderCommandResult<pr_guide::PrGuidePreview>, String> {
    use provider::{ProviderCommandResult as Envelope, ProviderFailureKind as Kind};
    let completed = match execute_codex_operation(
        app,
        &sessions,
        &manager,
        &reviews,
        &interruptions,
        &review_id,
        &fingerprint,
        consent,
        CodexOperation::PrGuide,
        |_, _, review, response, _| validate_pr_guide_preview(review, response),
        |storage, candidate| {
            publish_pr_guide_candidate(storage, candidate, PreviewPublication::AgentOwned)
        },
    )
    .await
    {
        Ok(completed) => completed,
        Err(error) => return Ok(Envelope::failed(error)),
    };
    // A guide remains in its separate unaccepted revision family.
    let result = completed.result;
    if completed.clear_interruption
        && interruptions
            .discard(&completed.storage, &review_id)
            .is_err()
    {
        return Ok(Envelope::error(
            Kind::LocalStorageUnavailable,
            if result.is_ok() {
                "The validated PR Guide draft is saved in PR Guide history, but its interruption notice could not be cleared. Open that draft before considering another generation."
            } else {
                "Codex stopped, but its interruption notice could not be cleared. Check recovery state and select the exact Git comparison before preparing another request."
            },
        ));
    }
    Ok(match result {
        Ok(preview) => Envelope::ok(preview),
        Err(error) => Envelope::failed(error),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn send_codex_pr_change_request(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    manager: State<'_, codex_agent::CodexAgentManager>,
    reviews: State<'_, provider::PendingProviderReviews>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    review_id: String,
    fingerprint: String,
    consent: bool,
) -> Result<provider::ProviderCommandResult<provider::PrChangeExplanationPreview>, String> {
    use provider::{ProviderCommandResult as Envelope, ProviderFailureKind as Kind};
    let completed = match execute_codex_operation(
        app,
        &sessions,
        &manager,
        &reviews,
        &interruptions,
        &review_id,
        &fingerprint,
        consent,
        CodexOperation::PrChange,
        |storage, repository, review, response, access| {
            provider::canonicalize_codex_pr_change_explanation(
                storage, repository, review, &response, access,
            )
        },
        |_, preview| Ok(preview),
    )
    .await
    {
        Ok(completed) => completed,
        Err(error) => return Ok(Envelope::failed(error)),
    };
    let result = completed.result;
    if completed.clear_interruption
        && interruptions
            .discard(&completed.storage, &review_id)
            .is_err()
    {
        return Ok(Envelope::error(
            Kind::LocalStorageUnavailable,
            "Codex stopped, but its interruption notice could not be cleared. Select the exact Git comparison and hunk before preparing another request.",
        ));
    }
    // Hunk claims remain transient. No chapter or PR Guide draft is written.
    Ok(match result {
        Ok(preview) => Envelope::ok(preview),
        Err(error) => Envelope::failed(error),
    })
}

#[derive(Clone, Copy)]
enum CodexOperation {
    Chapter,
    RepositoryPlan,
    CodeLens,
    PrGuide,
    PrChange,
}
impl CodexOperation {
    fn accepts(self, review: &provider::ProviderRequestReview) -> bool {
        let no_git = review.pr_guide_context.is_none() && review.pr_change_context.is_none();
        match self {
            Self::RepositoryPlan => provider::repository_plan_review_scope(review).is_ok(),
            Self::Chapter => {
                matches!(
                    review.purpose.as_str(),
                    "fileChapter" | "repositoryOverview" | "sectionUpdate"
                ) && review.code_lens_context.is_none()
                    && no_git
            }
            Self::CodeLens => {
                review.purpose == "codeLensExplanation"
                    && review.code_lens_context.is_some()
                    && review.section_update_context.is_none()
                    && no_git
            }
            Self::PrGuide => {
                review.purpose == "prGuide"
                    && review.pr_guide_context.is_some()
                    && review.pr_change_context.is_none()
                    && review.target_path.is_none()
                    && review.code_lens_context.is_none()
                    && review.section_update_context.is_none()
            }
            Self::PrChange => {
                review.purpose == "prChangeExplanation"
                    && review.pr_change_context.is_some()
                    && review.pr_guide_context.is_none()
                    && review.target_path.is_some()
                    && review.code_lens_context.is_none()
                    && review.section_update_context.is_none()
            }
        }
    }
}

struct PreparedCodexOperation {
    storage: PathBuf,
    repository: PathBuf,
    review: provider::ProviderRequestReview,
    input: codex_agent::CodexRunInput,
    started_at: Instant,
}

/// Dropping any generation command revokes its queued/running blocking worker.
/// The journal remains available even if the async caller disappears.
struct CodexCommandOwner {
    manager: codex_agent::CodexAgentManager,
    run_id: String,
    armed: bool,
}

impl Drop for CodexCommandOwner {
    fn drop(&mut self) {
        if self.armed {
            self.manager.cancel(&self.run_id);
        }
    }
}

struct CompletedCodexOperation<T> {
    storage: PathBuf,
    result: Result<T, provider::ProviderFailure>,
    clear_interruption: bool,
}

fn command_failure(
    kind: provider::ProviderFailureKind,
    message: impl Into<String>,
) -> provider::ProviderFailure {
    provider::ProviderFailure {
        kind,
        message: message.into(),
    }
}

/// Shared native consent, scope, source/settings revalidation and journal
/// admission. Planning keeps its own result lifecycle and never becomes a
/// chapter. No worker starts until this preparation succeeds.
async fn prepare_codex_operation(
    app: &AppHandle,
    sessions: &ProjectSessions,
    manager: &codex_agent::CodexAgentManager,
    reviews: &provider::PendingProviderReviews,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    fingerprint: &str,
    consent: bool,
    operation: CodexOperation,
) -> Result<PreparedCodexOperation, provider::ProviderFailure> {
    use provider::ProviderFailureKind as Kind;
    if !consent {
        return Err(command_failure(
            Kind::ConsentRequired,
            "Review the selected source and confirm sharing it with your Codex account before generating. Plan limits apply.",
        ));
    }
    let pending = reviews.get(review_id)?;
    if pending.review.provider_id != "codex"
        || pending.review.fingerprint != fingerprint
        || !operation.accepts(&pending.review)
    {
        return Err(command_failure(
            Kind::ReviewChanged,
            "Build a fresh Codex review for this operation with the selected source and settings.",
        ));
    }
    let project = sessions.get(&pending.review.project_id).map_err(|_| {
        command_failure(
            Kind::InvalidReview,
            "Reopen the selected repository before reviewing this Codex operation again.",
        )
    })?;
    let repository = PathBuf::from(project.repository_root);
    let storage = app_data_directory(app).map_err(|_| {
        command_failure(
            Kind::LocalStorageUnavailable,
            "Local snapshot storage is unavailable. No Codex request was sent.",
        )
    })?;
    let runtime = manager.clone();
    let reviewed = pending.review.clone();
    let storage_for_review = storage.clone();
    let repository_for_review = repository.clone();
    let validated = tauri::async_runtime::spawn_blocking(move || {
        let identity = runtime
            .review_runtime(&storage_for_review)
            .map_err(|message| command_failure(Kind::ReviewUnavailable, message))?;
        match operation {
            CodexOperation::RepositoryPlan => provider::rebuild_codex_repository_plan_review(
                &storage_for_review,
                &reviewed,
                &identity,
            )
            .map(|_| ()),
            CodexOperation::PrGuide | CodexOperation::PrChange => {
                provider::revalidate_codex_git_review(&repository_for_review, &reviewed, &identity)
                    .map(|_| ())
            }
            CodexOperation::Chapter | CodexOperation::CodeLens => {
                provider::revalidate_codex_request_review(&storage_for_review, &reviewed, &identity)
            }
        }
    })
    .await
    .map_err(|_| {
        command_failure(
            Kind::ReviewUnavailable,
            "The Codex review could not be revalidated. Build a fresh review.",
        )
    })?;
    if let Err(error) = validated {
        reviews.remove(review_id);
        return Err(error);
    }
    let started_at = Instant::now();
    let pending = reviews.consume(review_id, fingerprint)?;
    interruptions.record(&storage, recovery::new_reviewed_provider_operation(&pending.review))
        .map_err(|_| command_failure(Kind::LocalStorageUnavailable,
            "Local interruption state could not be saved. No Codex request was sent; build a fresh review before retrying."))?;
    let review = pending.review;
    let input = codex_agent::CodexRunInput {
        run_id: review_id.to_owned(),
        model: review.model_id.clone(),
        expected_runtime: provider::codex_runtime_from_review(&review)?,
        effort: review.generation_config["effort"]
            .as_str()
            .unwrap_or("")
            .to_owned(),
        approval_policy: review.generation_config["approvalPolicy"]
            .as_str()
            .unwrap_or("")
            .to_owned(),
        system_instruction: review.system_instruction.clone(),
        user_prompt: review.user_prompt.clone(),
        output_schema: review.generation_config["outputSchema"].clone(),
        correction_policy: if matches!(operation, CodexOperation::Chapter) {
            crate::claim_alignment::CorrectionPolicy::from_generation_config(&review.generation_config)
        } else {
            crate::claim_alignment::CorrectionPolicy::Disabled
        },
        retrieval: None,
        sources: review
            .selected_files
            .iter()
            .enumerate()
            .map(|(index, file)| codex_agent::CodexSourceFile {
                path: if matches!(
                    operation,
                    CodexOperation::PrGuide | CodexOperation::PrChange
                ) {
                    provider::agent_diff_source_name(index)
                } else {
                    file.path.clone()
                },
                // Code Lens stages only the reviewed excerpt. Its integrity digest
                // differs from the original file identity retained in the review.
                sha256: format!("{:x}", Sha256::digest(file.contents.as_bytes())),
                contents: file.contents.clone(),
            })
            .collect(),
    };
    Ok(PreparedCodexOperation {
        storage,
        repository,
        review,
        input,
        started_at,
    })
}

/// Retain process/run ownership through pure validation and serialize only
/// the final publication with cancellation. No raw response escapes this worker.
async fn execute_codex_operation<C: Send + 'static, T: Send + 'static>(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &codex_agent::CodexAgentManager,
    reviews: &provider::PendingProviderReviews,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    fingerprint: &str,
    consent: bool,
    operation: CodexOperation,
    validate: impl FnOnce(
        &Path,
        &Path,
        &provider::ProviderRequestReview,
        provider::ProviderSendResult,
        &codex_agent::CodexResultAccess<'_>,
    ) -> Result<C, provider::ProviderFailure>
    + Send
    + 'static,
    publish: impl FnOnce(&Path, C) -> Result<T, provider::ProviderFailure> + Send + 'static,
) -> Result<CompletedCodexOperation<T>, provider::ProviderFailure> {
    use provider::ProviderFailureKind as Kind;
    let prepared = prepare_codex_operation(
        &app,
        sessions,
        manager,
        reviews,
        interruptions,
        review_id,
        fingerprint,
        consent,
        operation,
    )
    .await?;
    let storage = prepared.storage.clone();
    let mut owner = CodexCommandOwner {
        manager: manager.clone(),
        run_id: review_id.to_owned(),
        armed: true,
    };
    let runtime = manager.clone();
    let run = tauri::async_runtime::spawn_blocking(move || {
        let PreparedCodexOperation {
            storage,
            repository,
            review,
            input,
            started_at,
        } = prepared;
        runtime.run_with_result_and_publish(
            &storage,
            input,
            &review.fingerprint,
            started_at,
            |event| {
                let _ = app.emit("codex-agent-progress", event);
            },
            |response, access| {
                let _ = app.emit(
                    "codex-agent-progress",
                    codex_agent::CodexProgress {
                        run_id: review.review_id.clone(),
                        phase: "validating".to_owned(),
                        thread_id: Some(response.thread_id.clone()),
                        turn_id: Some(response.turn_id.clone()),
                        message:
                            "Checking the explanation against the original source and operation."
                                .to_owned(),
                    },
                );
                provider::rebuild_active_codex_review(&storage, &repository, &review, access)?;
                validate(
                    &storage,
                    &repository,
                    &review,
                    provider::ProviderSendResult {
                        review_id: review.review_id.clone(),
                        fingerprint: review.fingerprint.clone(),
                        model_id: response.model,
                        model_version: None,
                        response_text: response.response_text,
                        input_tokens: None,
                        output_tokens: None,
                    },
                    access,
                )
            },
            |candidate| candidate.and_then(|candidate| publish(&storage, candidate)),
        )
    })
    .await;
    owner.armed = false;
    let result = match run {
        Ok(Ok(result)) => result,
        Ok(Err(message)) => Err(command_failure(codex_run_failure_kind(&message), message)),
        Err(_) => {
            manager.shutdown();
            Err(command_failure(
                Kind::ProviderRejectedRequest,
                "The Codex worker stopped unexpectedly. Its original operation remains in recovery; inspect saved previews and PR Guide history, then restart the app before reviewing another request.",
            ))
        }
    };
    // Failures (including validation/publication, cancel and timeout) retain
    // exact recovery scope. Only a published success can resolve this notice.
    let clear_interruption = result.is_ok();
    Ok(CompletedCodexOperation {
        storage,
        result,
        clear_interruption,
    })
}

// These are fixed, sanitized adapter messages, never arbitrary model text.
// Preserve actionable categories at the command boundary; unknown runtime
// failures remain explicit rejection with the adapter's safe diagnostic.
fn codex_run_failure_kind(message: &str) -> provider::ProviderFailureKind {
    use provider::ProviderFailureKind as Kind;
    if message.starts_with("The Codex run was cancelled locally.") {
        Kind::RequestCancelled
    } else if message.contains("timed out") || message.contains("exceeded ten minutes") {
        Kind::RequestTimedOut
    } else if message.starts_with("Your Codex account reached a usage limit.") {
        Kind::QuotaExceeded
    } else if message.starts_with("The selected model is not in")
        || message.starts_with("The selected model does not advertise")
    {
        Kind::UnavailableModel
    } else {
        Kind::ProviderRejectedRequest
    }
}
