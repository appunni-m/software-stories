//! Local, unaccepted repository plans. Saving never accepts chapters, credits
//! coverage, renews consent or qualifies an agent. Loading replays the original
//! proposal against immutable native evidence while retaining saved identities.

use crate::{
    pending_chapter_previews::{ensure_real_directory, open_without_following_symlinks},
    provider::{self, ProviderFailure, ProviderFailureKind, RepositoryPlanPreview},
    repository_plan::{self, PrepareRepositoryPlanInput, RepositoryPlanReviewScope},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use uuid::Uuid;

mod wire;

pub(crate) const SAVED_PLAN_RECORD_SCHEMA_VERSION: u32 = 2;
const LEGACY_SAVED_PLAN_RECORD_SCHEMA_VERSION: u32 = 1;

const MAX_RECORD_BYTES: usize = 2 * 1024 * 1024;
const MAX_PROJECT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORDS: usize = 128;
const MAX_ENTRIES: usize = 512;
// All native plan store operations share this lock. It is never acquired before
// an agent ownership lock: readers/discarders do not call an agent runtime.
static STORE_LOCK: Mutex<()> = Mutex::new(());

/// Recorded dispatch settings, not tamper-proof account/model provenance.
/// No executable path, connection token, account identity or API key is saved.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "backend", deny_unknown_fields)]
pub(crate) enum SavedPlanAgentSettings {
    #[serde(rename = "codex", rename_all = "camelCase")]
    Codex {
        effort: String,
        approval_policy: String,
    },
    #[serde(rename = "claude-agent", rename_all = "camelCase")]
    Claude {
        effort: Option<String>,
        approval_policy: String,
        max_turns: u32,
    },
}

#[derive(Clone)]
pub(crate) struct SavedPlanRecord {
    pub schema_version: u32,
    pub preview: RepositoryPlanPreview,
    pub scope: RepositoryPlanReviewScope,
    pub selected_paths: Vec<String>,
    pub settings: SavedPlanAgentSettings,
    // Retain the strict proposal because canonical claims intentionally omit
    // model-declared identifiers/aspects needed for independent replay.
    pub response_text: String,
    // A source-compatible enum field rename must not rehash immutable records.
    // This is derived from strictly decoded native fields, never authorization.
    encoding: wire::RecordEncoding,
}

/// Does not expose the original model proposal or any reusable authorization.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryPlanDraftDetail {
    preview: RepositoryPlanPreview,
    scope: RepositoryPlanReviewScope,
    selected_paths: Vec<String>,
    settings: SavedPlanAgentSettings,
    record_sha256: String,
}

/// Listing is metadata discovery only; load is required before using evidence.
/// Damaged regular records stay visible and explicitly discardable.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryPlanDraftSummary {
    draft_id: String,
    title: String,
    question: Option<String>,
    snapshot_id: Option<String>,
    review_id: Option<String>,
    provider_id: Option<String>,
    model_id: Option<String>,
    created_at_unix_ms: Option<u64>,
    unavailable_reason: Option<String>,
}

/// Only the native sender can construct this after validating its own response.
/// Preparation is pure; publication is a separate operation under agent ownership.
pub(crate) struct PreparedPlanSave {
    record: SavedPlanRecord,
    bytes: Vec<u8>,
}

/// Shared only with native accepted history and notice resolution. Callers
/// must not acquire any agent ownership lock while holding this guard.
pub(crate) fn lock_store() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    STORE_LOCK
        .lock()
        .map_err(|_| "Local plan storage is busy.".to_owned())
}

pub(crate) fn record_digest(record: &SavedPlanRecord) -> Result<String, String> {
    let bytes = serde_json::to_vec(record)
        .map_err(|_| "The local plan record could not be encoded.".to_owned())?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub(crate) fn record_detail(record: &SavedPlanRecord) -> Result<RepositoryPlanDraftDetail, String> {
    Ok(RepositoryPlanDraftDetail {
        preview: record.preview.clone(),
        scope: record.scope.clone(),
        selected_paths: record.selected_paths.clone(),
        settings: record.settings.clone(),
        record_sha256: record_digest(record)?,
    })
}

/// The caller holds the shared store lock and independently revalidates this
/// bounded record before acceptance, source actions or notice resolution.
pub(crate) fn read_draft_record(
    storage: &Path,
    project_id: &str,
    draft_id: &str,
) -> Result<SavedPlanRecord, String> {
    let filename = record_name(draft_id)?;
    let directory =
        plan_directory(storage, project_id, false)?.ok_or("The saved plan is unavailable.")?;
    read_record(&directory.join(filename), project_id, draft_id)
}

pub(crate) fn prepare_save(
    storage: &Path,
    review: &provider::ProviderRequestReview,
    response: &provider::ProviderSendResult,
) -> Result<PreparedPlanSave, ProviderFailure> {
    let mut preview = provider::canonicalize_repository_plan_response(storage, review, response)?;
    let scope = provider::repository_plan_review_scope(review)?;
    let config = &review.generation_config;
    let settings = match review.provider_id.as_str() {
        "codex" => SavedPlanAgentSettings::Codex {
            effort: config["effort"].as_str().unwrap_or("").to_owned(),
            approval_policy: config["approvalPolicy"].as_str().unwrap_or("").to_owned(),
        },
        "claude-agent" => {
            let settings = provider::claude_settings_from_review(review)?;
            SavedPlanAgentSettings::Claude {
                effort: settings.effort,
                approval_policy: settings.approval_policy,
                max_turns: settings.max_turns,
            }
        }
        _ => {
            return Err(local_failure(
                "Choose the original planning agent. No plan was saved.",
            ));
        }
    };
    preview.saved_locally = true;
    let record = SavedPlanRecord {
        schema_version: SAVED_PLAN_RECORD_SCHEMA_VERSION,
        preview,
        scope,
        selected_paths: review
            .selected_files
            .iter()
            .map(|file| file.path.clone())
            .collect(),
        settings,
        response_text: response.response_text.clone(),
        encoding: wire::RecordEncoding::Current,
    };
    validate_record_shape(&record, &review.project_id, &record.preview.plan.draft_id)
        .map_err(local_failure)?;
    revalidate(storage, &record).map_err(|message| ProviderFailure {
        kind: ProviderFailureKind::ReviewChanged,
        message,
    })?;
    let bytes = serde_json::to_vec(&record).map_err(|_| {
        local_failure("The plan could not be encoded. Its interruption notice remains available.")
    })?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(local_failure(
            "The saved plan would exceed 2 MiB. Its interruption notice remains available; reduce the reviewed selection.",
        ));
    }
    Ok(PreparedPlanSave { record, bytes })
}

/// Atomic no-overwrite publication. Call only inside the original agent's
/// guarded publication boundary, after its cancellation/deadline check.
pub(crate) fn publish(
    storage: &Path,
    candidate: PreparedPlanSave,
) -> Result<RepositoryPlanPreview, ProviderFailure> {
    // A publisher already holds its agent's ownership guard. Never queue
    // behind a source-revalidating reader while cancellation is serialized.
    let _guard = STORE_LOCK.try_lock().map_err(|_| {
        local_failure("Local plan storage is busy. Its interruption notice remains available.")
    })?;
    let project_id = &candidate.record.preview.plan.project_id;
    let directory = plan_directory(storage, project_id, true)
        .map_err(local_failure)?
        .ok_or_else(|| local_failure("The local plan directory is unavailable."))?;
    let target = directory
        .join(record_name(&candidate.record.preview.plan.draft_id).map_err(local_failure)?);
    if target
        .try_exists()
        .map_err(|_| local_failure("The saved plan destination could not be inspected."))?
    {
        if read_bytes(&target).map_err(local_failure)? == candidate.bytes {
            return Ok(candidate.record.preview);
        }
        return Err(local_failure(
            "A different local record already uses this plan identity. It was preserved; inspect saved plans and recovery before generating again.",
        ));
    }
    let entries = scan(&directory).map_err(local_failure)?;
    let count = entries
        .iter()
        .filter(|entry| entry.draft_id.is_some())
        .count();
    let total = entries
        .iter()
        .try_fold(0_u64, |total, entry| total.checked_add(entry.size))
        .ok_or_else(|| local_failure("The saved plan folder has an unsupported size."))?;
    if count >= MAX_RECORDS
        || total.saturating_add(candidate.bytes.len() as u64) > MAX_PROJECT_BYTES
        || entries.len() >= MAX_ENTRIES
    {
        return Err(local_failure(
            "Local plan storage is full (128 plans or 64 MiB per project). Explicitly discard unwanted drafts before generating again; existing plans and recovery were preserved.",
        ));
    }
    let stage = directory.join(format!(".plan-{}.tmp", Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&stage).map_err(|_| {
        local_failure(
            "The plan could not be staged locally. Its interruption notice remains available.",
        )
    })?;
    let cleanup = StagedFile(stage);
    file.write_all(&candidate.bytes).and_then(|_| file.sync_all())
        .map_err(|_| local_failure("The staged plan could not be written completely. Its interruption notice remains available."))?;
    fs::hard_link(&cleanup.0, &target).map_err(|_| local_failure("The plan could not be published without overwriting local data. Inspect saved plans and recovery before generating again."))?;
    drop(file);
    drop(cleanup);
    File::open(&directory).and_then(|directory| directory.sync_all()).map_err(|_| local_failure("The plan may already be saved, but directory durability could not be confirmed. Inspect saved plans and recovery before generating again."))?;
    Ok(candidate.record.preview)
}

pub(crate) fn list(
    storage: &Path,
    project_id: &str,
) -> Result<Vec<RepositoryPlanDraftSummary>, String> {
    let _guard = STORE_LOCK
        .lock()
        .map_err(|_| "Local plan storage is busy.")?;
    let Some(directory) = plan_directory(storage, project_id, false)? else {
        return Ok(Vec::new());
    };
    let mut result = Vec::new();
    for entry in scan(&directory)? {
        let Some(draft_id) = entry.draft_id else {
            continue;
        };
        let parsed = read_record(&entry.path, project_id, &draft_id);
        result.push(match parsed {
            Ok(record) => RepositoryPlanDraftSummary {
                draft_id, title: record.preview.plan.title,
                question: Some(record.preview.plan.question), snapshot_id: Some(record.preview.plan.snapshot_id),
                review_id: Some(record.preview.review_id), provider_id: Some(record.preview.provider_id), model_id: Some(record.preview.model_id),
                created_at_unix_ms: Some(record.preview.plan.created_at_unix_ms), unavailable_reason: None,
            },
            Err(_) => RepositoryPlanDraftSummary {
                draft_id, title: "Unavailable saved plan".to_owned(), question: None, snapshot_id: None,
                review_id: None, provider_id: None, model_id: None, created_at_unix_ms: None,
                unavailable_reason: Some("This local record is damaged, oversized or incompatible. Its bytes were preserved; discard it explicitly if no longer needed.".to_owned()),
            },
        });
    }
    result.sort_by(|a, b| {
        b.created_at_unix_ms
            .cmp(&a.created_at_unix_ms)
            .then_with(|| a.draft_id.cmp(&b.draft_id))
    });
    Ok(result)
}

pub(crate) fn load(
    storage: &Path,
    project_id: &str,
    draft_id: &str,
) -> Result<RepositoryPlanDraftDetail, String> {
    let _guard = STORE_LOCK
        .lock()
        .map_err(|_| "Local plan storage is busy.")?;
    let record = read_draft_record(storage, project_id, draft_id)?;
    revalidate(storage, &record)?;
    record_detail(&record)
}

/// Explicit deletion accepts a damaged regular record without trusting its JSON.
pub(crate) fn discard(storage: &Path, project_id: &str, draft_id: &str) -> Result<(), String> {
    let _guard = STORE_LOCK
        .lock()
        .map_err(|_| "Local plan storage is busy.")?;
    let filename = record_name(draft_id)?;
    let Some(directory) = plan_directory(storage, project_id, false)? else {
        return Ok(());
    };
    let path = directory.join(filename);
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() => {}
        Ok(_) => return Err("The saved plan is not a regular file. It was preserved.".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("The saved plan could not be inspected. It was preserved.".to_owned()),
    }
    fs::remove_file(path).map_err(|_| {
        "The saved plan could not be discarded. Check saved plans before trying again.".to_owned()
    })?;
    File::open(directory).and_then(|directory| directory.sync_all()).map_err(|_| "The plan was removed, but directory durability could not be confirmed. Refresh saved plans before trying again.".to_owned())
}

pub(crate) fn revalidate(storage: &Path, record: &SavedPlanRecord) -> Result<(), String> {
    repository_plan::revalidate_saved(
        storage,
        PrepareRepositoryPlanInput {
            project_id: record.preview.plan.project_id.clone(),
            snapshot_id: record.preview.plan.snapshot_id.clone(),
            question: record.scope.question.clone(),
            selected_paths: record.selected_paths.clone(),
        },
        &record.scope,
        record.response_text.as_bytes(),
        &record.preview.plan,
    )
}

pub(crate) fn validate_record_shape(
    record: &SavedPlanRecord,
    project_id: &str,
    draft_id: &str,
) -> Result<(), String> {
    let preview = &record.preview;
    let valid_settings = match &record.settings {
        SavedPlanAgentSettings::Codex {
            effort,
            approval_policy,
        } => {
            preview.provider_id == "codex"
                && !effort.trim().is_empty()
                && effort.len() <= 32
                && approval_policy == "never"
        }
        SavedPlanAgentSettings::Claude {
            effort,
            approval_policy,
            max_turns,
        } => {
            preview.provider_id == "claude-agent"
                && approval_policy == "dontAsk"
                && (1..=8).contains(max_turns)
                && effort.as_deref().is_none_or(|effort| {
                    matches!(effort, "low" | "medium" | "high" | "xhigh" | "max")
                })
        }
    };
    let valid_review_id =
        Uuid::parse_str(&preview.review_id).is_ok_and(|id| id.to_string() == preview.review_id);
    let fingerprint_valid = preview.fingerprint.len() == 64
        && preview
            .fingerprint
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !matches!(record.schema_version, LEGACY_SAVED_PLAN_RECORD_SCHEMA_VERSION | SAVED_PLAN_RECORD_SCHEMA_VERSION)
        || !preview.saved_locally
        || preview.plan.schema_version != 1
        || preview.plan.project_id != project_id
        || preview.plan.draft_id != draft_id
        || !repository_plan::canonical_uuid_id(draft_id, "plan")
        || !(8..=120).contains(&preview.plan.title.chars().count())
        || preview.plan.title.chars().any(char::is_control)
        || preview.plan.snapshot_id.is_empty()
        || preview.plan.snapshot_id.len() > 512
        || preview.plan.created_at_unix_ms == 0
        || preview.plan.created_at_unix_ms > (1_u64 << 53) - 1
        || preview.plan.question != record.scope.question
        || !record
            .scope
            .valid_recovery_shape(record.selected_paths.len())
        || record
            .selected_paths
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != record.selected_paths.len()
        || !valid_review_id
        || preview.recovery_operation_id != preview.review_id
        || !fingerprint_valid
        || !valid_settings
        || preview.model_id.trim().is_empty()
        || preview.model_id.len() > 200
        || preview
            .model_version
            .as_ref()
            .is_some_and(|value| value.len() > 200)
        || record.response_text.is_empty()
        || record.response_text.len() > 1_000_000
        || crate::snapshot::looks_secret_like(
            "repository-plan.json",
            record.response_text.as_bytes(),
        )
    {
        return Err("The saved plan has invalid identity, ownership, settings or bounded content. Its local copy was preserved.".to_owned());
    }
    Ok(())
}

fn read_record(path: &Path, project_id: &str, draft_id: &str) -> Result<SavedPlanRecord, String> {
    let bytes = read_bytes(path)?;
    let record: SavedPlanRecord = serde_json::from_slice(&bytes).map_err(|_| {
        "The saved plan is damaged or incompatible. Its local copy was preserved.".to_owned()
    })?;
    validate_record_shape(&record, project_id, draft_id)?;
    Ok(record)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    read_bounded_file(path, MAX_RECORD_BYTES)
}

pub(crate) fn read_bounded_file(path: &Path, max_bytes: usize) -> Result<Vec<u8>, String> {
    let file = open_without_following_symlinks(path)
        .map_err(|_| "The saved plan could not be opened as a regular local file.".to_owned())?;
    let metadata = file
        .metadata()
        .map_err(|_| "The saved plan could not be inspected.".to_owned())?;
    if !metadata.is_file() || metadata.len() > max_bytes as u64 {
        return Err("The saved plan is not a bounded regular file. It was preserved.".to_owned());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(max_bytes.saturating_add(1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "The saved plan could not be read completely.".to_owned())?;
    if bytes.len() != metadata.len() as usize || bytes.len() > max_bytes {
        return Err(
            "The saved plan changed while reading it. Its local copy was preserved.".to_owned(),
        );
    }
    Ok(bytes)
}

struct Entry {
    path: PathBuf,
    draft_id: Option<String>,
    size: u64,
}

fn scan(directory: &Path) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|_| "The saved plan folder could not be read.".to_owned())?
    {
        if entries.len() >= MAX_ENTRIES {
            return Err(
                "The saved plan folder contains too many entries. Existing data was preserved."
                    .to_owned(),
            );
        }
        let entry = entry.map_err(|_| "A saved plan entry could not be inspected.".to_owned())?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| "A saved plan entry could not be inspected.".to_owned())?;
        if !metadata.is_file() {
            return Err(
                "The saved plan folder contains a nonregular entry. Existing data was preserved."
                    .to_owned(),
            );
        }
        let name = entry.file_name();
        let draft_id = name
            .to_str()
            .and_then(|name| name.strip_suffix(".json"))
            .and_then(|uuid| {
                let id = format!("plan:{uuid}");
                repository_plan::canonical_uuid_id(&id, "plan").then_some(id)
            });
        entries.push(Entry {
            path,
            draft_id,
            size: metadata.len(),
        });
    }
    Ok(entries)
}

fn plan_directory(
    storage: &Path,
    project_id: &str,
    create: bool,
) -> Result<Option<PathBuf>, String> {
    project_directory(storage, project_id, "repository-plans", create)
}

pub(crate) fn project_directory(
    storage: &Path,
    project_id: &str,
    folder: &'static str,
    create: bool,
) -> Result<Option<PathBuf>, String> {
    if project_id.trim().is_empty() || project_id.len() > 512 {
        return Err("Reopen the repository before using saved plans.".to_owned());
    }
    let root = storage.join(folder);
    let project = root.join(format!("{:x}", Sha256::digest(project_id.as_bytes())));
    for directory in [storage, root.as_path(), project.as_path()] {
        match fs::symlink_metadata(directory) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {},
            Err(error) if error.kind() == std::io::ErrorKind::NotFound && !create => return Ok(None),
            _ => ensure_real_directory(directory).map_err(|_| "The saved plan directory is unavailable or nonregular. Existing data was preserved.".to_owned())?,
        }
    }
    Ok(Some(project))
}

fn record_name(draft_id: &str) -> Result<String, String> {
    if !repository_plan::canonical_uuid_id(draft_id, "plan") {
        return Err("Select a saved native plan identity.".to_owned());
    }
    Ok(format!("{}.json", &draft_id[5..]))
}

fn local_failure(message: impl Into<String>) -> ProviderFailure {
    ProviderFailure {
        kind: ProviderFailureKind::LocalStorageUnavailable,
        message: message.into(),
    }
}

struct StagedFile(PathBuf);
impl Drop for StagedFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
