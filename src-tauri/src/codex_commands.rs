//! Native account-backed Codex commands. Immutable explanation evidence and
//! local validators are shared with API workflows; authentication is separate.

use super::{
    ProjectSessions, app_data_directory, chapters, codex_agent, pr_guide,
    prepare_chapter_draft_preview, prepare_pr_guide_preview, provider, recovery,
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
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
        {
            let result = open_login_browser(&login.auth_url);
            if result.is_err() {
                let _ = manager.cancel_login(storage, &login.login_id);
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
        let status = runtime
            .status(&storage)
            .map_err(|message| provider::ProviderFailure {
                kind: provider::ProviderFailureKind::ReviewUnavailable,
                message,
            })?;
        provider::build_codex_chapter_review(
            &storage,
            &input,
            status.executable_path.as_deref().unwrap_or(""),
        )
    })
    .await
    .map_err(|_| "Codex request review could not be prepared.".to_owned())?;
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
        let status = runtime.status(&storage).map_err(|message| {
            command_failure(provider::ProviderFailureKind::ReviewUnavailable, message)
        })?;
        provider::build_codex_code_lens_review(
            &storage,
            &input,
            status.executable_path.as_deref().unwrap_or(""),
        )
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
        let status = runtime.status(&storage).map_err(|message| {
            command_failure(provider::ProviderFailureKind::ReviewUnavailable, message)
        })?;
        let executable = status.executable_path.as_deref().unwrap_or("");
        match input {
            GitReviewInput::Guide(input) => provider::build_codex_pr_guide_review(
                Path::new(&project.repository_root),
                &input,
                executable,
            ),
            GitReviewInput::Change(input) => provider::build_codex_pr_change_review(
                Path::new(&project.repository_root),
                &input,
                executable,
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
    )
    .await
    {
        Ok(completed) => completed,
        Err(error) => return Ok(ResultEnvelope::failed(error)),
    };
    let mut result = match completed.result {
        Ok(response) => {
            prepare_chapter_draft_preview(&completed.storage, &completed.review, response, &drafts)
                .map(ResultEnvelope::ok)
                .unwrap_or_else(ResultEnvelope::failed)
        }
        Err(error) => ResultEnvelope::failed(error),
    };
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
    )
    .await
    {
        Ok(completed) => completed,
        Err(error) => return Ok(ResultEnvelope::failed(error)),
    };
    let result = completed.result.and_then(|response| {
        provider::canonicalize_code_lens_explanation(
            &completed.storage,
            &completed.review,
            &response,
        )
    });
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
    )
    .await
    {
        Ok(completed) => completed,
        Err(error) => return Ok(Envelope::failed(error)),
    };
    let result = completed.result.and_then(|response| {
        provider::revalidate_codex_git_review(
            &completed.repository,
            &completed.review,
            completed.review.generation_config["executablePath"]
                .as_str()
                .unwrap_or(""),
        )?;
        // PR Guides retain their separate unaccepted revision family. Never
        // route this result through chapter history or accept it automatically.
        prepare_pr_guide_preview(&completed.storage, &completed.review, response)
    });
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
    )
    .await
    {
        Ok(completed) => completed,
        Err(error) => return Ok(Envelope::failed(error)),
    };
    let result = completed.result.and_then(|response| {
        provider::canonicalize_pr_change_explanation(
            &completed.repository,
            &completed.review,
            &response,
        )
    });
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
    CodeLens,
    PrGuide,
    PrChange,
}
impl CodexOperation {
    fn accepts(self, review: &provider::ProviderRequestReview) -> bool {
        let no_git = review.pr_guide_context.is_none() && review.pr_change_context.is_none();
        match self {
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

struct CompletedCodexOperation {
    storage: PathBuf,
    repository: PathBuf,
    review: provider::ProviderRequestReview,
    result: Result<provider::ProviderSendResult, provider::ProviderFailure>,
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

/// One reviewed lifecycle for automatic Codex operations. The command-specific
/// caller validates the response and clears recovery only after preparing its
/// result; range results never enter chapter draft/history storage.
async fn execute_codex_operation(
    app: AppHandle,
    sessions: &ProjectSessions,
    manager: &codex_agent::CodexAgentManager,
    reviews: &provider::PendingProviderReviews,
    interruptions: &recovery::InterruptionJournal,
    review_id: &str,
    fingerprint: &str,
    consent: bool,
    operation: CodexOperation,
) -> Result<CompletedCodexOperation, provider::ProviderFailure> {
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
    let storage = app_data_directory(&app).map_err(|_| {
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
        let status = runtime
            .status(&storage_for_review)
            .map_err(|message| command_failure(Kind::ReviewUnavailable, message))?;
        let executable = status.executable_path.as_deref().unwrap_or("");
        match operation {
            CodexOperation::PrGuide | CodexOperation::PrChange => {
                provider::revalidate_codex_git_review(&repository_for_review, &reviewed, executable)
                    .map(|_| ())
            }
            CodexOperation::Chapter | CodexOperation::CodeLens => {
                provider::revalidate_codex_request_review(
                    &storage_for_review,
                    &reviewed,
                    executable,
                )
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
    let pending = reviews.consume(review_id, fingerprint)?;
    interruptions.record(&storage, recovery::new_reviewed_provider_operation(&pending.review))
        .map_err(|_| command_failure(Kind::LocalStorageUnavailable,
            "Local interruption state could not be saved. No Codex request was sent; build a fresh review before retrying."))?;
    let review = pending.review;
    let input = codex_agent::CodexRunInput {
        run_id: review_id.to_owned(),
        model: review.model_id.clone(),
        expected_executable_path: review.generation_config["executablePath"]
            .as_str()
            .unwrap_or("")
            .to_owned(),
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
    let runtime = manager.clone();
    let storage_for_run = storage.clone();
    let run = tauri::async_runtime::spawn_blocking(move || {
        runtime.run(&storage_for_run, input, |event| {
            let _ = app.emit("codex-agent-progress", event);
        })
    })
    .await;
    let (result, clear_interruption) = match run {
        Ok(Ok(response)) => (
            Ok(provider::ProviderSendResult {
                review_id: review_id.to_owned(),
                fingerprint: fingerprint.to_owned(),
                model_id: response.model,
                model_version: None,
                response_text: response.response_text,
                input_tokens: None,
                output_tokens: None,
            }),
            true,
        ),
        Ok(Err(message)) => (
            Err(command_failure(codex_run_failure_kind(&message), message)),
            true,
        ),
        Err(_) => {
            manager.shutdown();
            (
                Err(command_failure(
                    Kind::ProviderRejectedRequest,
                    "The Codex worker stopped unexpectedly. Restart the app and review a new request before retrying.",
                )),
                false,
            )
        }
    };
    Ok(CompletedCodexOperation {
        storage,
        repository,
        review,
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
