//! Local, user-operated agent handoffs. No runtime or authentication is managed
//! here; importing still uses the same native snapshot/evidence validator.

use super::{
    ProjectSessions, app_data_directory, chapters, prepare_chapter_draft_preview, provider,
};
use serde::Deserialize;
use serde_json::Value;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) async fn build_agent_handoff_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    input: provider::BuildAgentHandoffInput,
) -> Result<provider::ProviderCommandResult<provider::ProviderRequestReview>, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    let pending = tauri::async_runtime::spawn_blocking(move || {
        provider::build_agent_handoff_review(&storage, &input)
    })
    .await
    .map_err(|_| "The local handoff could not be prepared.".to_owned())?;
    Ok(match pending.and_then(|pending| reviews.insert(pending)) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn export_agent_handoff(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    review_id: String,
    fingerprint: String,
    consent: bool,
) -> Result<provider::ProviderCommandResult<String>, String> {
    use provider::{ProviderCommandResult as Envelope, ProviderFailureKind as Kind};
    if !consent {
        return Ok(Envelope::error(
            Kind::ConsentRequired,
            "Review the source and confirm copying it to an external agent.",
        ));
    }
    let pending = match reviews.get(&review_id) {
        Ok(pending)
            if pending.review.provider_id == "agent-handoff"
                && pending.review.fingerprint == fingerprint =>
        {
            pending
        }
        Ok(_) => {
            return Ok(Envelope::error(
                Kind::ReviewChanged,
                "Select the matching manual handoff review.",
            ));
        }
        Err(error) => return Ok(Envelope::failed(error)),
    };
    sessions.get(&pending.review.project_id)?;
    let storage = app_data_directory(&app)?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        provider::revalidate_agent_handoff(&storage, &pending.review)?;
        provider::agent_handoff_text(&pending.review)
    })
    .await
    .map_err(|_| "The local handoff could not be exported.".to_owned())?;
    Ok(match result {
        Ok(text) => Envelope::ok(text),
        Err(error) => Envelope::failed(error),
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AgentResponse {
    handoff_id: String,
    fingerprint: String,
    chapter: Value,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn import_agent_handoff_result(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    drafts: State<'_, chapters::PendingChapterDrafts>,
    review_id: String,
    fingerprint: String,
    response_text: String,
) -> Result<provider::ProviderCommandResult<chapters::ChapterDraftPreview>, String> {
    use provider::{ProviderCommandResult as Envelope, ProviderFailureKind as Kind};
    if response_text.len() > provider::MAX_PROVIDER_RESPONSE_BYTES {
        return Ok(Envelope::error(
            Kind::ProviderResponseTooLarge,
            "The imported response exceeds 1 MB.",
        ));
    }
    let response: AgentResponse = match serde_json::from_str(&response_text) {
        Ok(response) => response,
        Err(_) => {
            return Ok(Envelope::error(
                Kind::MalformedProviderResponse,
                "Paste the complete JSON object with handoffId, fingerprint and chapter, without Markdown fences.",
            ));
        }
    };
    if response.handoff_id != review_id
        || response.fingerprint != fingerprint
        || !response.chapter.is_object()
    {
        return Ok(Envelope::error(
            Kind::ReviewChanged,
            "This response belongs to another handoff or has no chapter object. Run the current copied task and import its complete result.",
        ));
    }
    let pending = match reviews.get(&review_id) {
        Ok(pending)
            if pending.review.provider_id == "agent-handoff"
                && pending.review.fingerprint == fingerprint =>
        {
            pending
        }
        Ok(_) => {
            return Ok(Envelope::error(
                Kind::ReviewChanged,
                "The response does not match a manual handoff review.",
            ));
        }
        Err(error) => return Ok(Envelope::failed(error)),
    };
    sessions.get(&pending.review.project_id)?;
    let storage = app_data_directory(&app)?;
    let reviewed = pending.review.clone();
    let storage_for_check = storage.clone();
    let checked = tauri::async_runtime::spawn_blocking(move || {
        provider::revalidate_agent_handoff(&storage_for_check, &reviewed)
    })
    .await
    .map_err(|_| "The handoff source could not be revalidated.".to_owned())?;
    if let Err(error) = checked {
        return Ok(Envelope::failed(error));
    }
    let pending = match reviews.consume(&review_id, &fingerprint) {
        Ok(pending) => pending,
        Err(error) => return Ok(Envelope::failed(error)),
    };
    let result = provider::ProviderSendResult {
        review_id,
        fingerprint,
        model_id: pending.review.model_id.clone(),
        model_version: None,
        response_text: response.chapter.to_string(),
        input_tokens: None,
        output_tokens: None,
    };
    match prepare_chapter_draft_preview(&storage, &pending.review, result, &drafts) {
        Ok(preview) => Ok(Envelope::ok(preview)),
        Err(error) => {
            // Permit correction against the same exact task, without extending
            // its expiry. Accepted history has not been modified.
            let _ = reviews.insert(pending);
            Ok(Envelope::failed(error))
        }
    }
}
