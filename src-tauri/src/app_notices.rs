//! Local UI outcomes only. This history cannot execute work or restore consent.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::AppHandle;

const MAX_ITEMS: usize = 100;
const NOTICE_HISTORY_SCHEMA_VERSION: u32 = 1;
const MAX_DISMISSED: usize = 2048;
const MAX_BYTES: usize = 1024 * 1024;
const UNAVAILABLE: &str = "Saved updates are unavailable. Existing work has been preserved.";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Tone {
    Info,
    Warning,
    Error,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Target {
    OpenRepository {},
    RetryRecents {},
    RetryConversation {},
    OpenExploration {
        #[serde(rename = "explorationId")]
        exploration_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Input {
    title: String,
    message: String,
    tone: Tone,
    project_id: Option<String>,
    snapshot_id: Option<String>,
    target: Option<Target>,
    occurrence_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Item {
    id: String,
    input: Input,
    created_at_unix_ms: u64,
    read: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct History {
    schema_version: u32,
    items: Vec<Item>,
    dismissed: Vec<String>,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum Operation {
    Publish { input: Input },
    Read { id: String },
    Dismiss { id: String },
}

impl Default for History {
    fn default() -> Self {
        Self {
            schema_version: NOTICE_HISTORY_SCHEMA_VERSION,
            items: vec![],
            dismissed: vec![],
        }
    }
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
}

fn validate(input: &Input) -> Result<(), String> {
    let text = |s: &str, max| {
        !s.trim().is_empty() && s.chars().count() <= max && !s.chars().any(char::is_control)
    };
    if !text(&input.title, 100)
        || !text(&input.message, 700)
        || input.project_id.as_deref().is_some_and(|id| {
            id.trim().is_empty() || id.len() > 4096 || id.chars().any(char::is_control)
        })
        || input
            .snapshot_id
            .as_deref()
            .is_some_and(|id| !identifier(id))
        || input
            .occurrence_id
            .as_deref()
            .is_some_and(|id| !identifier(id))
        || (input.snapshot_id.is_some() && input.project_id.is_none())
    {
        return Err("This update has an invalid display context.".into());
    }
    if let Some(Target::OpenExploration { exploration_id }) = &input.target {
        if !identifier(exploration_id) || input.project_id.is_none() || input.snapshot_id.is_none()
        {
            return Err("This update has no saved exploration context.".into());
        }
    }
    if matches!(input.target, Some(Target::RetryConversation {}))
        && (input.project_id.is_none() || input.snapshot_id.is_none())
    {
        return Err("This update has no saved conversation context.".into());
    }
    Ok(())
}

fn key(input: &Input) -> Result<String, String> {
    validate(input)?;
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(input).map_err(|_| UNAVAILABLE)?)
    ))
}

fn valid_key(id: &str) -> bool {
    id.len() == 64
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn list(storage: &Path) -> Result<History, String> {
    let directory = storage.join("app-notices");
    match fs::symlink_metadata(&directory) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(History::default()),
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
        _ => return Err(UNAVAILABLE.into()),
    }
    let path = directory.join("history.json");
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(History::default()),
        Ok(m) if m.is_file() && !m.file_type().is_symlink() && m.len() <= MAX_BYTES as u64 => {}
        _ => return Err(UNAVAILABLE.into()),
    }
    let bytes = crate::bounded_file::read(&path, MAX_BYTES).map_err(|_| UNAVAILABLE)?;
    let value = crate::strict_json::from_slice(&bytes).map_err(|_| UNAVAILABLE)?;
    let history: History = serde_json::from_value(value).map_err(|_| UNAVAILABLE)?;
    if history.schema_version != NOTICE_HISTORY_SCHEMA_VERSION
        || history.items.len() > MAX_ITEMS
        || history.dismissed.len() > MAX_DISMISSED
    {
        return Err(UNAVAILABLE.into());
    }
    let mut ids = HashSet::new();
    for id in &history.dismissed {
        if !valid_key(id) || !ids.insert(id.clone()) {
            return Err(UNAVAILABLE.into());
        }
    }
    for item in &history.items {
        if !valid_key(&item.id)
            || key(&item.input)? != item.id
            || !ids.insert(item.id.clone())
            || item.created_at_unix_ms == 0
            || item.created_at_unix_ms > 9_007_199_254_740_991
        {
            return Err(UNAVAILABLE.into());
        }
    }
    Ok(history)
}

fn apply(storage: &Path, operation: Operation) -> Result<History, String> {
    crate::codex_agent::ensure_private_directory(storage).map_err(|_| UNAVAILABLE)?;
    let directory = storage.join("app-notices");
    crate::codex_agent::ensure_private_directory(&directory).map_err(|_| UNAVAILABLE)?;
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(directory.join("history.lock"))
        .map_err(|_| UNAVAILABLE)?;
    let _lease = crate::file_lease::try_exclusive(file)
        .map_err(|_| UNAVAILABLE)?
        .ok_or("Another local update is being saved. Try again.")?;
    let mut history = list(storage)?;
    match operation {
        Operation::Publish { input } => {
            let id = key(&input)?;
            if history.dismissed.contains(&id) || history.items.iter().any(|item| item.id == id) {
                return Ok(history);
            }
            if history.items.len() >= MAX_ITEMS {
                return Err(
                    "Saved updates are full. Dismiss an older update before saving more.".into(),
                );
            }
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| UNAVAILABLE)?
                .as_millis();
            history.items.push(Item {
                id,
                input,
                created_at_unix_ms: u64::try_from(now).map_err(|_| UNAVAILABLE)?,
                read: false,
            });
        }
        Operation::Read { id } => {
            if !valid_key(&id) {
                return Err(UNAVAILABLE.into());
            }
            let Some(item) = history.items.iter_mut().find(|item| item.id == id) else {
                return Ok(history);
            };
            if item.read {
                return Ok(history);
            }
            item.read = true;
        }
        Operation::Dismiss { id } => {
            if !valid_key(&id) {
                return Err(UNAVAILABLE.into());
            }
            if !history.items.iter().any(|item| item.id == id) {
                return Ok(history);
            }
            if history.dismissed.len() >= MAX_DISMISSED {
                return Err(
                    "Saved update acknowledgments are full. Existing updates have been preserved."
                        .into(),
                );
            }
            history.items.retain(|item| item.id != id);
            history.dismissed.push(id);
        }
    }
    let bytes = serde_json::to_vec(&history).map_err(|_| UNAVAILABLE)?;
    if bytes.len() > MAX_BYTES {
        return Err(UNAVAILABLE.into());
    }
    crate::codex_agent::atomic_private_write(&directory.join("history.json"), &bytes)
        .map_err(|_| UNAVAILABLE)?;
    Ok(history)
}

#[tauri::command]
pub(crate) async fn list_app_notices(app: AppHandle) -> Result<History, String> {
    let storage = crate::app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || list(&storage))
        .await
        .map_err(|_| UNAVAILABLE)?
}

#[tauri::command]
pub(crate) async fn update_app_notices(
    app: AppHandle,
    operation: Operation,
) -> Result<History, String> {
    let storage = crate::app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || apply(&storage, operation))
        .await
        .map_err(|_| UNAVAILABLE)?
}

#[cfg(test)]
mod tests;
