//! Native account-backed Codex commands. Chapter evidence and local draft
//! validation are shared with the API workflow; authentication is separate.

use super::{
    ProjectSessions, app_data_directory, chapters, codex_agent, prepare_chapter_draft_preview,
    provider, recovery,
};
use std::path::Path;
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
    if !consent {
        return Ok(ResultEnvelope::error(
            Kind::ConsentRequired,
            "Review the selected source and confirm sharing it with your Codex account before generating. Plan limits apply.",
        ));
    }
    let pending = match reviews.get(&review_id) {
        Ok(pending)
            if pending.review.provider_id == "codex"
                && pending.review.fingerprint == fingerprint =>
        {
            pending
        }
        Ok(_) => {
            return Ok(ResultEnvelope::error(
                Kind::ReviewChanged,
                "Build a fresh Codex review with the selected source and settings.",
            ));
        }
        Err(error) => return Ok(ResultEnvelope::failed(error)),
    };
    sessions.get(&pending.review.project_id)?;
    let storage = app_data_directory(&app)?;
    let runtime = manager.inner().clone();
    let reviewed = pending.review.clone();
    let storage_for_review = storage.clone();
    let validated = tauri::async_runtime::spawn_blocking(move || {
        let status =
            runtime
                .status(&storage_for_review)
                .map_err(|message| provider::ProviderFailure {
                    kind: Kind::ReviewUnavailable,
                    message,
                })?;
        provider::revalidate_codex_chapter_review(
            &storage_for_review,
            &reviewed,
            status.executable_path.as_deref().unwrap_or(""),
        )
    })
    .await
    .map_err(|_| "The Codex review could not be revalidated.".to_owned())?;
    if let Err(error) = validated {
        reviews.remove(&review_id);
        return Ok(ResultEnvelope::failed(error));
    }
    let pending = match reviews.consume(&review_id, &fingerprint) {
        Ok(pending) => pending,
        Err(error) => return Ok(ResultEnvelope::failed(error)),
    };
    interruptions.record(
        &storage,
        recovery::new_provider_operation(
            pending.review.project_id.clone(),
            pending.review.snapshot_id.clone(),
            pending.review.target_path.clone(),
            review_id.clone(),
        ),
    )?;
    let review = pending.review;
    let input = codex_agent::CodexRunInput {
        run_id: review_id.clone(),
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
            .map(|file| codex_agent::CodexSourceFile {
                path: file.path.clone(),
                sha256: file.sha256.clone(),
                contents: file.contents.clone(),
            })
            .collect(),
    };
    let runtime = manager.inner().clone();
    let storage_for_run = storage.clone();
    let run = tauri::async_runtime::spawn_blocking(move || {
        runtime.run(&storage_for_run, input, |event| {
            let _ = app.emit("codex-agent-progress", event);
        })
    })
    .await;
    let mut result = match run {
        Ok(Ok(response)) => prepare_chapter_draft_preview(
            &storage,
            &review,
            provider::ProviderSendResult {
                review_id: review_id.clone(),
                fingerprint,
                model_id: response.model,
                model_version: None,
                response_text: response.response_text,
                input_tokens: None,
                output_tokens: None,
            },
            &drafts,
        )
        .map(ResultEnvelope::ok)
        .unwrap_or_else(ResultEnvelope::failed),
        Ok(Err(message)) => ResultEnvelope::error(codex_run_failure_kind(&message), message),
        Err(_) => {
            manager.shutdown();
            ResultEnvelope::error(
                Kind::ProviderRejectedRequest,
                "The Codex worker stopped unexpectedly. Restart the app and review a new request before retrying.",
            )
        }
    };
    if let Err(message) = interruptions.discard(&storage, &review_id) {
        match &mut result {
            ResultEnvelope::Ok { data } => data.update_notes.push(format!(
                "Your validated preview is available. Its interruption notice could not be cleared: {message} Save or discard this preview before generating again.")),
            ResultEnvelope::Error { .. } => return Ok(ResultEnvelope::error(Kind::LocalStorageUnavailable,
                format!("Codex stopped, but its recovery notice could not be cleared: {message} Check saved previews before generating again."))),
        }
    }
    Ok(result)
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
