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
const FLOW_DIAGRAM_LAYOUT_VERSION: u64 = 4;
const FOCUSED_STORY_LAYOUT_VERSION: u64 = 4;
const MAX_BYTES: usize = 8 * 1024 * 1024;
const UNAVAILABLE: &str =
    "Your saved conversation could not be opened. Existing work has been preserved.";
const CONFLICT: &str = "Another window saved this conversation. Your unsaved changes have been preserved in this window.";
const SAVE_STORAGE: &str = "Local conversation storage could not be written. Check available disk space and folder access; your changes remain open in this window.";
const SAVE_BUSY: &str = "A conversation save is still in progress. Wait a moment, then retry.";
const SAVE_INVALID: &str = "Code Atlas rejected this conversation update. Your saved work is intact and the unsaved changes remain open.";
const SAVE_TOO_LARGE: &str = "This conversation exceeds the local save limit. Your saved work is intact and the unsaved changes remain open.";

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
        .is_some_and(|s| uuid::Uuid::parse_str(s).is_ok_and(|id| id.to_string() == s))
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

fn guide_unit(v: &Value) -> Result<(), String> {
    let o = object(v, &["key", "question", "prerequisites", "status", "runIds"])?;
    let key = string(o, "key")?;
    let prerequisites = o
        .get("prerequisites")
        .and_then(Value::as_array)
        .ok_or(UNAVAILABLE)?;
    let runs = o
        .get("runIds")
        .and_then(Value::as_array)
        .ok_or(UNAVAILABLE)?;
    if key.is_empty()
        || key.len() > 80
        || !key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        || !text(string(o, "question")?, 65536)
        || prerequisites.len() > 6
        || prerequisites.iter().any(|v| {
            v.as_str().is_none_or(|key| {
                key.is_empty()
                    || key.len() > 80
                    || !key
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            })
        })
        || prerequisites.iter().collect::<HashSet<_>>().len() != prerequisites.len()
        || ![
            "pending",
            "working",
            "complete",
            "partial",
            "stopped",
            "interrupted",
            "failed",
        ]
        .contains(&string(o, "status")?)
        || runs.len() > 3
        || runs.iter().any(|v| !uuid(v))
        || runs.iter().collect::<HashSet<_>>().len() != runs.len()
    {
        return Err(UNAVAILABLE.into());
    }
    Ok(())
}

fn guide_metadata(v: &Value, expected_id: &str) -> Result<(), String> {
    let o = object(
        v,
        &[
            "revision",
            "id",
            "projectId",
            "snapshotId",
            "question",
            "title",
            "status",
            "openingRunId",
            "openingRunIds",
            "canContinue",
            "continuationOperations",
            "units",
            "omissions",
            "capturedFiles",
            "startingFiles",
        ],
    )?;
    let id = string(o, "id")?;
    let opening_runs = o
        .get("openingRunIds")
        .and_then(Value::as_array)
        .ok_or(UNAVAILABLE)?;
    let units = o
        .get("units")
        .and_then(Value::as_array)
        .ok_or(UNAVAILABLE)?;
    let omissions = o
        .get("omissions")
        .and_then(Value::as_array)
        .ok_or(UNAVAILABLE)?;
    if !uuid(&Value::String(id.to_owned()))
        || id != expected_id
        || !integer(o.get("revision").ok_or(UNAVAILABLE)?, u64::MAX)
        || !text(string(o, "projectId")?, 4096)
        || !identifier(string(o, "snapshotId")?)
        || !text(string(o, "question")?, 65536)
        || !text(string(o, "title")?, 512)
        || ![
            "planning",
            "building",
            "waitingForPlayback",
            "ready",
            "partial",
            "stopped",
            "interrupted",
            "failed",
        ]
        .contains(&string(o, "status")?)
        || !optional(o, "openingRunId", uuid)
        || opening_runs.len() > 3
        || opening_runs.iter().any(|v| !uuid(v))
        || opening_runs.iter().collect::<HashSet<_>>().len() != opening_runs.len()
        || !o.get("canContinue").is_some_and(Value::is_boolean)
        || !o
            .get("continuationOperations")
            .is_some_and(|v| integer(v, 7))
        || units.len() > 6
        || units.iter().any(|unit| guide_unit(unit).is_err())
        || omissions.len() > 32
        || omissions
            .iter()
            .any(|v| v.as_str().is_none_or(|s| !text(s, 4096)))
        || !o
            .get("capturedFiles")
            .is_some_and(|v| integer(v, 1_000_000))
        || !o.get("startingFiles").is_some_and(|v| integer(v, 1024))
    {
        return Err(UNAVAILABLE.into());
    }
    Ok(())
}

fn viewport(v: &Value) -> Result<(), String> {
    let o = object(v, &["x", "y", "zoom"])?;
    if ["x", "y"].iter().any(|key| {
        !o.get(*key)
            .and_then(Value::as_f64)
            .is_some_and(|n| n.is_finite() && n.abs() <= 10_000_000.0)
    }) || !o
        .get("zoom")
        .and_then(Value::as_f64)
        .is_some_and(|n| n.is_finite() && (0.1..=4.0).contains(&n))
    {
        return Err(UNAVAILABLE.into());
    }
    Ok(())
}

fn original_revision(owner: &serde_json::Map<String, Value>, revision: &str) -> bool {
    owner.get("revisionId").and_then(Value::as_str) == Some(revision)
        || owner
            .get("publishedSections")
            .and_then(Value::as_array)
            .is_some_and(|s| {
                s.iter()
                    .any(|s| s.get("revisionId").and_then(Value::as_str) == Some(revision))
            })
}

// These are bounded reader preferences, never chapter payloads or generation
// receipts. The reader separately verifies accepted ancestry against history.
fn answer_versions(v: &Value, owner: &serde_json::Map<String, Value>) -> Result<(), String> {
    let entries = v.as_array().ok_or(UNAVAILABLE)?;
    if entries.len() > 256 {
        return Err(UNAVAILABLE.into());
    }
    let mut seen = HashSet::new();
    let mut selected = HashSet::new();
    for entry in entries {
        let o = object(
            entry,
            &["originalRevisionId", "revisionId", "selected", "reading"],
        )?;
        let original = string(o, "originalRevisionId")?;
        let revision = string(o, "revisionId")?;
        let chosen = o
            .get("selected")
            .and_then(Value::as_bool)
            .ok_or(UNAVAILABLE)?;
        if !identifier(original)
            || !identifier(revision)
            || !original_revision(owner, original)
            || !seen.insert((original, revision))
            || (chosen && !selected.insert(original))
        {
            return Err(UNAVAILABLE.into());
        }
        if let Some(reading) = o.get("reading") {
            if object(reading, &["guide", "flow"])?
                .values()
                .any(|v| !id(v))
            {
                return Err(UNAVAILABLE.into());
            }
        }
    }
    Ok(())
}

fn presentation(v: &Value, owner: &serde_json::Map<String, Value>) -> Result<(), String> {
    let o = object(v, &["stories", "diagrams"])?;
    for key in ["stories", "diagrams"] {
        let Some(values) = o.get(key) else { continue };
        let values = values.as_array().ok_or(UNAVAILABLE)?;
        if values.len() > 256 {
            return Err(UNAVAILABLE.into());
        }
        let mut seen = HashSet::new();
        for p in values {
            let p = object(
                p,
                if key == "stories" {
                    &[
                        "revisionId",
                        "sectionId",
                        "sectionRevisionId",
                        "sceneId",
                        "positions",
                        "fullMap",
                        "viewport",
                        "focusLayoutVersion",
                        "focusedNodeId",
                        "focusedEdgeId",
                    ]
                } else {
                    &[
                        "revisionId",
                        "sectionId",
                        "sectionRevisionId",
                        "viewport",
                        "layoutVersion",
                    ]
                },
            )?;
            let revision = string(p, "revisionId")?;
            let section = string(p, "sectionId")?;
            let section_revision = string(p, "sectionRevisionId")?;
            let owned = original_revision(owner, revision)
                || owner
                    .get("answerVersions")
                    .and_then(Value::as_array)
                    .is_some_and(|s| {
                        s.iter()
                            .any(|s| s.get("revisionId").and_then(Value::as_str) == Some(revision))
                    });
            if !identifier(revision)
                || !identifier(section)
                || !identifier(section_revision)
                || !owned
                || !seen.insert((revision, section, section_revision))
            {
                return Err(UNAVAILABLE.into());
            }
            if key == "stories" {
                if !p.get("sceneId").is_some_and(id)
                    || !p.get("fullMap").is_some_and(Value::is_boolean)
                    // Layout 3 is a supported on-disk migration source. The
                    // renderer upgrades it to layout 4 while retaining scene
                    // and narration positions and discarding stale geometry.
                    || !p.get("focusLayoutVersion").is_some_and(|v| {
                        matches!(v.as_u64(), Some(3) | Some(FOCUSED_STORY_LAYOUT_VERSION))
                    })
                    || !optional(p, "focusedNodeId", id)
                    || !optional(p, "focusedEdgeId", id)
                {
                    return Err(UNAVAILABLE.into());
                }
                let positions = p
                    .get("positions")
                    .and_then(Value::as_object)
                    .ok_or(UNAVAILABLE)?;
                if positions.len() > 128
                    || positions
                        .iter()
                        .any(|(k, v)| !identifier(k) || !number(v, 3600))
                {
                    return Err(UNAVAILABLE.into());
                }
                if let Some(v) = p.get("viewport") {
                    viewport(v)?;
                }
            } else {
                if p.get("layoutVersion").and_then(Value::as_u64)
                    != Some(FLOW_DIAGRAM_LAYOUT_VERSION)
                {
                    return Err(UNAVAILABLE.into());
                }
                viewport(p.get("viewport").ok_or(UNAVAILABLE)?)?;
            }
        }
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
            "guide",
            "reading",
            "presentation",
            "answerVersions",
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
            "waiting",
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
    if let Some(guide) = o.get("guide") {
        guide_metadata(guide, string(o, "guideId")?)?;
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
                || !p.contains_key("supersedes")
                || !p.contains_key("runElapsedMs")
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
        if a.len() > 8
            || a.iter().any(|v| {
                !v.as_str().is_some_and(|s| {
                    !s.trim().is_empty()
                        && s.encode_utf16().count() <= 1000
                        && !s.chars().any(char::is_control)
                })
            })
        {
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
    if let Some(v) = o.get("answerVersions") {
        answer_versions(v, o)?;
    }
    if let Some(v) = o.get("presentation") {
        presentation(v, o)?;
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
                "codeOpen",
                "fileFlow",
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
            || e.get("codeOpen").is_some_and(|v| !v.is_boolean())
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
        if let Some(v) = e.get("fileFlow") {
            let v = object(v, &["path", "showAll"])?;
            if !v.get("path").is_some_and(path) || !v.get("showAll").is_some_and(Value::is_boolean)
            {
                return Err(UNAVAILABLE.into());
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

/// Reader positions from unsupported layouts or malformed optional fields are
/// transient. Drop or repair only those positions in memory so they cannot make
/// the conversation unavailable. Keep story layout 3 as a supported migration
/// input; the renderer upgrades it to layout 4. The durable record is untouched.
fn discard_stale_reader_positions(state: &mut Value) {
    let Some(explorations) = state.get_mut("explorations").and_then(Value::as_array_mut) else {
        return;
    };
    for exploration in explorations {
        if let Some(scroll) = exploration.get_mut("scrollPositions") {
            if let Some(positions) = scroll.as_object_mut() {
                positions.retain(|view, value| {
                    ["guide", "flow", "history"].contains(&view.as_str())
                        && number(value, 10_000_000)
                });
            } else {
                exploration
                    .as_object_mut()
                    .expect("exploration is an object after validation inputs are parsed")
                    .remove("scrollPositions");
            }
        }
        let Some(turns) = exploration.get_mut("turns").and_then(Value::as_array_mut) else {
            continue;
        };
        for turn in turns {
            discard_reading_positions(turn);
            discard_presentation_positions(turn);
        }
    }
}

/// Keep a saved Guide position only when its answer/section ownership fields
/// are syntactically usable. Ownership itself remains checked by `validate`.
/// Optional progress fields are disposable and are repaired independently so
/// one malformed scene offset does not hide the saved conversation.
fn sanitize_story_position(story: &mut Value) -> Option<(String, String, String)> {
    let story = story.as_object_mut()?;
    if !matches!(
        story.get("focusLayoutVersion").and_then(Value::as_u64),
        Some(3) | Some(FOCUSED_STORY_LAYOUT_VERSION)
    ) {
        return None;
    }

    let revision = story.get("revisionId").and_then(Value::as_str)?.to_owned();
    let section = story.get("sectionId").and_then(Value::as_str)?.to_owned();
    let section_revision = story
        .get("sectionRevisionId")
        .and_then(Value::as_str)?
        .to_owned();
    if !identifier(&revision) || !identifier(&section) || !identifier(&section_revision) {
        return None;
    }
    if !story.get("sceneId").is_some_and(id) {
        return None;
    }

    if !story.get("fullMap").is_some_and(Value::is_boolean) {
        story.insert("fullMap".into(), Value::Bool(false));
    }
    match story.get_mut("positions") {
        Some(Value::Object(positions)) => {
            positions.retain(|claim, position| identifier(claim) && number(position, 3600));
            if positions.len() > 128 {
                positions.clear();
            }
        }
        Some(positions) => *positions = Value::Object(Default::default()),
        None => {
            story.insert("positions".into(), Value::Object(Default::default()));
        }
    }
    if story
        .get("viewport")
        .is_some_and(|viewport_value| viewport(viewport_value).is_err())
    {
        story.remove("viewport");
    }
    for key in ["focusedNodeId", "focusedEdgeId"] {
        if story
            .get(key)
            .is_some_and(|value| !value.is_null() && !id(value))
        {
            story.remove(key);
        }
    }

    Some((revision, section, section_revision))
}

/// Keep a saved Flow selection when its answer/section identity is usable,
/// and reset only its malformed viewport. The later validator still enforces
/// that the selected revision belongs to this answer.
fn sanitize_diagram_position(diagram: &mut Value) -> Option<(String, String, String)> {
    let diagram = diagram.as_object_mut()?;
    if diagram.get("layoutVersion").and_then(Value::as_u64) != Some(FLOW_DIAGRAM_LAYOUT_VERSION) {
        return None;
    }

    let revision = diagram
        .get("revisionId")
        .and_then(Value::as_str)?
        .to_owned();
    let section = diagram.get("sectionId").and_then(Value::as_str)?.to_owned();
    let section_revision = diagram
        .get("sectionRevisionId")
        .and_then(Value::as_str)?
        .to_owned();
    if !identifier(&revision) || !identifier(&section) || !identifier(&section_revision) {
        return None;
    }
    if diagram
        .get("viewport")
        .is_none_or(|viewport_value| viewport(viewport_value).is_err())
    {
        diagram.insert(
            "viewport".into(),
            serde_json::json!({"x": 0, "y": 0, "zoom": 1}),
        );
    }

    Some((revision, section, section_revision))
}

/// These presentation values are reader preferences. Drop malformed optional
/// collections and individual stale entries in memory while leaving valid
/// locations and all answer data untouched. The durable record is not changed.
fn discard_presentation_positions(turn: &mut Value) {
    let Some(turn) = turn.as_object_mut() else {
        return;
    };
    if turn
        .get("presentation")
        .is_some_and(|presentation| !presentation.is_object())
    {
        turn.remove("presentation");
        return;
    }
    let Some(presentation) = turn.get_mut("presentation").and_then(Value::as_object_mut) else {
        return;
    };

    let sanitizers: [(&str, fn(&mut Value) -> Option<(String, String, String)>); 2] = [
        ("stories", sanitize_story_position),
        ("diagrams", sanitize_diagram_position),
    ];
    for (key, sanitize) in sanitizers {
        if presentation
            .get(key)
            .is_some_and(|entries| !entries.is_array())
        {
            presentation.remove(key);
            continue;
        }
        let Some(entries) = presentation.get_mut(key).and_then(Value::as_array_mut) else {
            continue;
        };
        entries.truncate(256);
        let mut seen = HashSet::new();
        entries.retain_mut(|entry| sanitize(entry).is_some_and(|identity| seen.insert(identity)));
    }
}

/// Saved reading locations are disposable UI state. Keep every valid entry
/// while dropping malformed values so corrupt scroll/bookmark progress cannot
/// make the answer journal unavailable. This is an in-memory recovery only;
/// load never rewrites the user's durable record.
fn discard_reading_positions(turn: &mut Value) {
    let Some(object) = turn.as_object_mut() else {
        return;
    };
    if let Some(reading) = object.get_mut("reading") {
        let valid = if let Some(views) = reading.as_object_mut() {
            views.retain(|view, location| {
                if !["guide", "flow"].contains(&view.as_str()) {
                    return false;
                }
                let Some(location) = location.as_object() else {
                    return false;
                };
                location
                    .keys()
                    .all(|key| ["revisionId", "sectionId"].contains(&key.as_str()))
                    && location.get("revisionId").is_some_and(id)
                    && location.get("sectionId").is_some_and(id)
            });
            true
        } else {
            false
        };
        if !valid {
            object.remove("reading");
        }
    }
    if let Some(versions) = object
        .get_mut("answerVersions")
        .and_then(Value::as_array_mut)
    {
        for version in versions {
            let Some(fields) = version.as_object_mut() else {
                continue;
            };
            if let Some(reading) = fields.get_mut("reading") {
                let valid = if let Some(views) = reading.as_object_mut() {
                    views.retain(|view, section| {
                        ["guide", "flow"].contains(&view.as_str()) && id(section)
                    });
                    true
                } else {
                    false
                };
                if !valid {
                    fields.remove("reading");
                }
            }
        }
    }
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
    let mut record: Record =
        serde_json::from_value(crate::strict_json::from_slice(&bytes).map_err(|_| UNAVAILABLE)?)
            .map_err(|_| UNAVAILABLE)?;
    if record.schema_version != CONVERSATION_SCHEMA_VERSION
        || record.revision == 0
        || record.revision > 9_007_199_254_740_991
    {
        return Err(UNAVAILABLE.into());
    }
    discard_stale_reader_positions(&mut record.state);
    validate(&record.state, project, snapshot)?;
    Ok(Some(record))
}
/// Read only the question labels for exact owned runs. Missing, damaged or
/// ambiguous associations remain unknown; they never prevent record recovery.
pub(crate) fn questions_for_runs(
    storage: &Path,
    project: &str,
    snapshot: &str,
    runs: &[&str],
) -> HashMap<String, String> {
    let Some(record) = load(storage, project, snapshot).ok().flatten() else {
        return HashMap::new();
    };
    let wanted: HashSet<&str> = runs.iter().copied().collect();
    let mut found = HashMap::new();
    let mut ambiguous = HashSet::new();
    for exploration in record.state["explorations"]
        .as_array()
        .into_iter()
        .flatten()
    {
        for turn in exploration["turns"].as_array().into_iter().flatten() {
            for run in turn["runIds"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                if wanted.contains(run) {
                    if found
                        .insert(
                            run.to_owned(),
                            turn["question"].as_str().unwrap_or_default().to_owned(),
                        )
                        .is_some()
                    {
                        ambiguous.insert(run.to_owned());
                    }
                }
            }
        }
    }
    found.retain(|run, question| !ambiguous.contains(run) && !question.trim().is_empty());
    found
}

fn save(
    storage: &Path,
    project: &str,
    snapshot: &str,
    expected_revision: Option<u64>,
    state: Value,
) -> Result<Record, String> {
    validate(&state, project, snapshot).map_err(|_| SAVE_INVALID.to_owned())?;
    let path = location(storage, project, snapshot)?;
    let directory = path.parent().ok_or(UNAVAILABLE)?;
    crate::codex_agent::ensure_private_directory(storage).map_err(|_| SAVE_STORAGE)?;
    crate::codex_agent::ensure_private_directory(directory).map_err(|_| SAVE_STORAGE)?;
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
        .map_err(|_| SAVE_STORAGE)?;
    let _lease = crate::file_lease::try_exclusive(lock)
        .map_err(|_| SAVE_STORAGE)?
        .ok_or(SAVE_BUSY)?;
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
        return Err(SAVE_TOO_LARGE.into());
    }
    crate::codex_agent::atomic_private_write(&path, &bytes).map_err(|_| SAVE_STORAGE)?;
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
