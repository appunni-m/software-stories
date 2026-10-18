//! Flat conversation metadata. Source and generated chapter bytes stay in their stores.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};
use tauri::AppHandle;

const CONVERSATION_SCHEMA_VERSION: u32 = 1;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const UNAVAILABLE: &str =
    "Your saved conversation could not be opened. Existing work has been preserved.";
const CONFLICT: &str = "Another window saved this conversation. Your unsaved changes have been preserved in this window.";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Record {
    schema_version: u32,
    revision: u64,
    state: Value,
}

fn identifier(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 256
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
}
fn text(s: &str, max: usize) -> bool {
    s.len() <= max
        && !s
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
}
fn relative(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 4096
        && !s.starts_with('/')
        && !s.contains('\\')
        && !s.chars().any(char::is_control)
        && !s
            .split('/')
            .any(|p| p.is_empty() || matches!(p, "." | ".."))
}
fn object<'a>(v: &'a Value, fields: &[&str]) -> Result<&'a serde_json::Map<String, Value>, String> {
    let o = v.as_object().ok_or(UNAVAILABLE)?;
    if o.keys().any(|k| !fields.contains(&k.as_str())) {
        return Err(UNAVAILABLE.into());
    }
    Ok(o)
}
fn string<'a>(o: &'a serde_json::Map<String, Value>, k: &str) -> Result<&'a str, String> {
    o.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| UNAVAILABLE.into())
}
fn optional(
    o: &serde_json::Map<String, Value>,
    k: &str,
    valid: impl FnOnce(&Value) -> bool,
) -> bool {
    o.get(k).is_none_or(|v| v.is_null() || valid(v))
}
fn id(v: &Value) -> bool {
    v.as_str().is_some_and(identifier)
}
fn path(v: &Value) -> bool {
    v.as_str().is_some_and(relative)
}
fn hex(v: &Value, len: usize) -> bool {
    v.as_str().is_some_and(|s| {
        s.len() == len
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn number(v: &Value, max: u64) -> bool {
    v.as_f64()
        .is_some_and(|n| n.is_finite() && n >= 0.0 && n <= max as f64)
}
fn integer(v: &Value, max: u64) -> bool {
    v.as_u64().is_some_and(|n| n <= max)
}
fn uuid(v: &Value) -> bool {
    v.as_str()
        .is_some_and(|s| uuid::Uuid::parse_str(s).is_ok() && s.len() == 36)
}
fn generation(v: &Value) -> bool {
    v.as_str()
        .and_then(|s| s.strip_prefix("generation:"))
        .is_some_and(|s| uuid(&Value::String(s.into())))
}

fn context(v: &Value) -> Result<(), String> {
    let o = object(v, &["path", "sha256", "from", "to"])?;
    if !relative(string(o, "path")?)
        || !optional(o, "sha256", |v| hex(v, 64))
        || !optional(o, "from", |v| {
            v.as_u64().is_some_and(|n| n > 0 && n <= u32::MAX as u64)
        })
        || !optional(o, "to", |v| {
            v.as_u64().is_some_and(|n| n > 0 && n <= u32::MAX as u64)
        })
        || o.get("to").is_some_and(|v| {
            !v.is_null()
                && (o.get("from").and_then(Value::as_u64).is_none()
                    || v.as_u64() < o["from"].as_u64())
        })
    {
        return Err(UNAVAILABLE.into());
    }
    Ok(())
}

fn turn(v: &Value) -> Result<&str, String> {
    let o = object(
        v,
        &[
            "id",
            "question",
            "context",
            "status",
            "revisionId",
            "generationId",
            "error",
            "runIds",
            "publishedSections",
            "notes",
            "guideId",
            "reading",
            "outcomeAt",
            "outcomeSequence",
            "outcomeNotice",
        ],
    )?;
    let name = string(o, "id")?;
    if !identifier(name)
        || !text(string(o, "question")?, 65536)
        || ![
            "queued",
            "working",
            "ready",
            "partial",
            "failed",
            "stopped",
            "interrupted",
        ]
        .contains(&string(o, "status")?)
        || !optional(o, "revisionId", id)
        || !optional(o, "generationId", generation)
        || !optional(o, "guideId", uuid)
        || !optional(o, "error", |v| v.as_str().is_some_and(|s| text(s, 65536)))
        || !optional(o, "outcomeAt", |v| integer(v, 9_007_199_254_740_991))
        || !optional(o, "outcomeSequence", |v| integer(v, 9_007_199_254_740_991))
    {
        return Err(UNAVAILABLE.into());
    }
    let contexts = o
        .get("context")
        .and_then(Value::as_array)
        .ok_or(UNAVAILABLE)?;
    if contexts.len() > 64 {
        return Err(UNAVAILABLE.into());
    }
    for c in contexts {
        context(c)?;
    }
    let runs = o.get("runIds").and_then(Value::as_array);
    if runs.is_some_and(|r| {
        r.len() > 32
            || r.iter().any(|v| !uuid(v))
            || r.iter().collect::<HashSet<_>>().len() != r.len()
    }) {
        return Err(UNAVAILABLE.into());
    }
    if let Some(v) = o.get("runIds") {
        if !v.is_array() {
            return Err(UNAVAILABLE.into());
        }
    }
    if let Some(v) = o.get("publishedSections") {
        let sections = v.as_array().ok_or(UNAVAILABLE)?;
        if sections.len() > 256 {
            return Err(UNAVAILABLE.into());
        }
        let mut seen = HashSet::new();
        for s in sections {
            let p = object(
                s,
                &[
                    "runId",
                    "key",
                    "order",
                    "generationId",
                    "revisionId",
                    "supersedes",
                    "contextElapsedMs",
                    "runElapsedMs",
                ],
            )?;
            if !p.get("runId").is_some_and(uuid)
                || !runs.is_some_and(|r| r.contains(&p["runId"]))
                || !p.get("generationId").is_some_and(generation)
                || !p.get("revisionId").is_some_and(id)
                || !p.get("order").is_some_and(|v| integer(v, 5))
                || !string(p, "key")?
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
                || string(p, "key")?.is_empty()
                || string(p, "key")?.len() > 80
                || !optional(p, "supersedes", generation)
                || !p
                    .get("contextElapsedMs")
                    .is_some_and(|v| number(v, 9_007_199_254_740_991))
                || !optional(p, "runElapsedMs", |v| number(v, 9_007_199_254_740_991))
                || !seen.insert((p["runId"].clone(), p["generationId"].clone()))
            {
                return Err(UNAVAILABLE.into());
            }
        }
    }
    if let Some(v) = o.get("notes") {
        let a = v.as_array().ok_or(UNAVAILABLE)?;
        if a.len() > 8 || a.iter().any(|v| !v.as_str().is_some_and(|s| text(s, 4000))) {
            return Err(UNAVAILABLE.into());
        }
    }
    if let Some(v) = o.get("reading") {
        for p in object(v, &["guide", "flow"])?.values() {
            let p = object(p, &["revisionId", "sectionId"])?;
            if !p.get("revisionId").is_some_and(id) || !p.get("sectionId").is_some_and(id) {
                return Err(UNAVAILABLE.into());
            }
        }
    }
    if let Some(v) = o.get("outcomeNotice") {
        if object(v, &["read", "dismissed"])?
            .values()
            .any(|v| !hex(v, 16))
        {
            return Err(UNAVAILABLE.into());
        }
    }
    Ok(name)
}

fn validate(state: &Value, project: &str, snapshot: &str) -> Result<(), String> {
    let o = object(
        state,
        &[
            "version",
            "projectId",
            "snapshotId",
            "currentId",
            "file",
            "explorations",
            "grantId",
        ],
    )?;
    if project.is_empty()
        || project.len() > 4096
        || project.chars().any(char::is_control)
        || !identifier(snapshot)
        || o.get("version") != Some(&Value::from(1))
        || string(o, "projectId")? != project
        || string(o, "snapshotId")? != snapshot
        || !optional(o, "file", path)
        || !optional(o, "grantId", id)
    {
        return Err(UNAVAILABLE.into());
    }
    let explorations = o
        .get("explorations")
        .and_then(Value::as_array)
        .ok_or(UNAVAILABLE)?;
    if explorations.is_empty() || explorations.len() > 2000 {
        return Err(UNAVAILABLE.into());
    }
    let mut parents = HashMap::new();
    let mut owners = HashMap::new();
    let mut origins = Vec::new();
    for e in explorations {
        let e = object(
            e,
            &[
                "id",
                "parentId",
                "originTurnId",
                "title",
                "view",
                "turns",
                "focusedTurnId",
                "file",
                "reveal",
                "scrollPositions",
            ],
        )?;
        let name = string(e, "id")?;
        let parent = e.get("parentId").and_then(Value::as_str);
        if !identifier(name)
            || parents.insert(name, parent).is_some()
            || !text(string(e, "title")?, 65536)
            || !["guide", "flow", "history"].contains(&string(e, "view")?)
            || !optional(e, "file", path)
            || (name == "root"
                && (!e.get("parentId").is_some_and(Value::is_null)
                    || !e.get("originTurnId").is_some_and(Value::is_null)))
            || (name != "root"
                && (!parent.is_some_and(identifier) || !e.get("originTurnId").is_some_and(id)))
        {
            return Err(UNAVAILABLE.into());
        }
        if let Some(v) = e.get("reveal") {
            if !v.is_null() {
                context(v)?;
            }
        }
        if let Some(v) = e.get("scrollPositions") {
            if object(v, &["guide", "flow", "history"])?
                .values()
                .any(|v| !number(v, 10_000_000))
            {
                return Err(UNAVAILABLE.into());
            }
        }
        let turns = e
            .get("turns")
            .and_then(Value::as_array)
            .ok_or(UNAVAILABLE)?;
        for t in turns {
            if owners.insert(turn(t)?, name).is_some() || owners.len() > 20000 {
                return Err(UNAVAILABLE.into());
            }
        }
        if !optional(e, "focusedTurnId", |v| {
            v.as_str().is_some_and(|id| owners.get(id) == Some(&name))
        }) {
            return Err(UNAVAILABLE.into());
        }
        if name != "root" {
            origins.push((parent.ok_or(UNAVAILABLE)?, string(e, "originTurnId")?));
        }
    }
    if !parents.contains_key("root") || !parents.contains_key(string(o, "currentId")?) {
        return Err(UNAVAILABLE.into());
    }
    for (parent, origin) in origins {
        if owners.get(origin) != Some(&parent) {
            return Err(UNAVAILABLE.into());
        }
    }
    for name in parents.keys() {
        let mut cursor = *name;
        let mut seen = HashSet::new();
        while cursor != "root" {
            if !seen.insert(cursor) {
                return Err(UNAVAILABLE.into());
            }
            cursor = parents.get(cursor).copied().flatten().ok_or(UNAVAILABLE)?;
        }
    }
    if serde_json::to_vec(state).map_err(|_| UNAVAILABLE)?.len() > MAX_BYTES {
        return Err(UNAVAILABLE.into());
    }
    Ok(())
}

fn location(storage: &Path, project: &str, snapshot: &str) -> Result<PathBuf, String> {
    if project.is_empty()
        || project.len() > 4096
        || project.chars().any(char::is_control)
        || !identifier(snapshot)
    {
        return Err(UNAVAILABLE.into());
    }
    let digest = Sha256::digest(serde_json::to_vec(&(project, snapshot)).map_err(|_| UNAVAILABLE)?);
    Ok(storage
        .join("workspace-conversations")
        .join(format!("{digest:x}.json")))
}
fn load(storage: &Path, project: &str, snapshot: &str) -> Result<Option<Record>, String> {
    let path = location(storage, project, snapshot)?;
    match fs::symlink_metadata(path.parent().ok_or(UNAVAILABLE)?) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Ok(m) if m.is_dir() && !m.file_type().is_symlink() => {}
        _ => return Err(UNAVAILABLE.into()),
    }
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Ok(m) if m.is_file() && !m.file_type().is_symlink() && m.len() <= MAX_BYTES as u64 => {}
        _ => return Err(UNAVAILABLE.into()),
    }
    let bytes = crate::bounded_file::read(&path, MAX_BYTES).map_err(|_| UNAVAILABLE)?;
    let record: Record =
        serde_json::from_value(crate::strict_json::from_slice(&bytes).map_err(|_| UNAVAILABLE)?)
            .map_err(|_| UNAVAILABLE)?;
    if record.schema_version != CONVERSATION_SCHEMA_VERSION
        || record.revision == 0
        || record.revision > 9_007_199_254_740_991
    {
        return Err(UNAVAILABLE.into());
    }
    validate(&record.state, project, snapshot)?;
    Ok(Some(record))
}
fn save(
    storage: &Path,
    project: &str,
    snapshot: &str,
    expected_revision: Option<u64>,
    state: Value,
) -> Result<Record, String> {
    validate(&state, project, snapshot)?;
    let path = location(storage, project, snapshot)?;
    let directory = path.parent().ok_or(UNAVAILABLE)?;
    crate::codex_agent::ensure_private_directory(storage).map_err(|_| UNAVAILABLE)?;
    crate::codex_agent::ensure_private_directory(directory).map_err(|_| UNAVAILABLE)?;
    let mut options = fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let lock = options
        .open(path.with_extension("lock"))
        .map_err(|_| UNAVAILABLE)?;
    let _lease = crate::file_lease::try_exclusive(lock)
        .map_err(|_| UNAVAILABLE)?
        .ok_or("This conversation is being saved. Try again.")?;
    let previous = load(storage, project, snapshot)?;
    // An acknowledgment may be lost after the atomic write. An exact replay is
    // safe; a stale different value must never overwrite a newer window.
    if let Some(record) = &previous {
        if record.state == state {
            return Ok(record.clone());
        }
    }
    if previous.as_ref().map(|r| r.revision) != expected_revision {
        return Err(CONFLICT.into());
    }
    let revision = expected_revision
        .unwrap_or(0)
        .checked_add(1)
        .filter(|r| *r <= 9_007_199_254_740_991)
        .ok_or(UNAVAILABLE)?;
    let record = Record {
        schema_version: CONVERSATION_SCHEMA_VERSION,
        revision,
        state,
    };
    let bytes = serde_json::to_vec(&record).map_err(|_| UNAVAILABLE)?;
    if bytes.len() > MAX_BYTES {
        return Err(UNAVAILABLE.into());
    }
    crate::codex_agent::atomic_private_write(&path, &bytes).map_err(|_| UNAVAILABLE)?;
    Ok(record)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn load_workspace_conversation(
    app: AppHandle,
    project_id: String,
    snapshot_id: String,
) -> Result<Option<Record>, String> {
    let storage = crate::app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || load(&storage, &project_id, &snapshot_id))
        .await
        .map_err(|_| UNAVAILABLE)?
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn save_workspace_conversation(
    app: AppHandle,
    project_id: String,
    snapshot_id: String,
    expected_revision: Option<u64>,
    state: Value,
) -> Result<Record, String> {
    let storage = crate::app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        save(
            &storage,
            &project_id,
            &snapshot_id,
            expected_revision,
            state,
        )
    })
    .await
    .map_err(|_| UNAVAILABLE)?
}

#[cfg(test)]
mod tests;
