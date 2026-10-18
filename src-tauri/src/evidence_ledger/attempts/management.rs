//! Explicit local review and reversible removal. Never resumes an agent or
//! turns rejected model text into a trusted chapter or rendered HTML.

use super::*;
use std::{fs::OpenOptions, io::Read, time::UNIX_EPOCH};

const UNAVAILABLE: &str = "Saved attempts could not be read. Existing explanations are unchanged.";
const CHANGED: &str = "This saved attempt changed. Refresh the list before trying again.";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Entry {
    pub(crate) id: String,
    pub(crate) review_token: Option<String>,
    bytes: u64,
    project_id: Option<String>,
    snapshot_id: Option<String>,
    started_at_unix_ms: Option<u64>,
    candidate_count: usize,
    available: bool,
    question: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Inventory {
    pub(crate) entries: Vec<Entry>,
    limited: bool,
    operation_limit: usize,
    pub(crate) busy: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Attempt {
    ordinal: u8,
    response_text: String,
    rejection: Option<String>,
    findings: Vec<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Review {
    entry: Entry,
    attempts: Vec<Attempt>,
}

pub(super) fn lease(directory: &Path) -> Result<crate::file_lease::Lease, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(directory.join("store.lock"))
        .map_err(|_| UNAVAILABLE)?;
    if !file.metadata().map_err(|_| UNAVAILABLE)?.is_file() {
        return Err(UNAVAILABLE.into());
    }
    crate::file_lease::try_exclusive(file)
        .map_err(|_| UNAVAILABLE)?
        .ok_or_else(|| "Another saved attempt is being updated. Try again.".into())
}

fn entry_name(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 255
        && id != "store.lock"
        && !id.chars().any(char::is_control)
        && Path::new(id).components().count() == 1
        && matches!(
            Path::new(id).components().next(),
            Some(std::path::Component::Normal(_))
        )
}

fn operation(id: &str) -> Option<&str> {
    id.strip_suffix(".json").filter(|id| canonical_uuid(id))
}

fn identity(path: &Path) -> Result<String, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    if fs::symlink_metadata(path)
        .map_err(|_| CHANGED)?
        .file_type()
        .is_symlink()
    {
        return Err(UNAVAILABLE.into());
    }
    let file = options.open(path).map_err(|_| CHANGED)?;
    let meta = file.metadata().map_err(|_| CHANGED)?;
    if !meta.is_file() {
        return Err(UNAVAILABLE.into());
    }
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| (d.as_secs(), d.subsec_nanos()));
    #[cfg(unix)]
    let native = {
        use std::os::unix::fs::MetadataExt;
        (meta.dev(), meta.ino(), meta.ctime(), meta.ctime_nsec())
    };
    #[cfg(not(unix))]
    let native = (0u64, 0u64, 0i64, 0i64);
    // Valid-size records also bind exact bytes. Oversized damaged records bind
    // inode/change-time/size, so they can be moved without an unbounded read.
    let contents = if meta.len() <= MAX_RECORD as u64 {
        let mut bytes = Vec::new();
        (&file)
            .take(MAX_RECORD as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| UNAVAILABLE)?;
        if bytes.len() as u64 != meta.len() {
            return Err(CHANGED.into());
        }
        Some(context::digest(&bytes))
    } else {
        None
    };
    // Detect replacement and ordinary in-place writes during the bounded read.
    // The private directory and shared lease serialize cooperating app processes;
    // this is not protection against a separate same-user process rewriting it.
    let after = fs::symlink_metadata(path).map_err(|_| CHANGED)?;
    if !after.is_file()
        || after.file_type().is_symlink()
        || after.len() != meta.len()
        || after.modified().ok() != meta.modified().ok()
    {
        return Err(CHANGED.into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if (after.dev(), after.ino(), after.ctime(), after.ctime_nsec()) != native {
            return Err(CHANGED.into());
        }
    }
    Ok(context::digest(
        &serde_json::to_vec(&(meta.len(), modified, native, contents)).map_err(|_| UNAVAILABLE)?,
    ))
}

fn summarize(storage: &Path, path: &Path) -> Result<Entry, String> {
    let id = path
        .file_name()
        .and_then(|s| s.to_str())
        .filter(|s| entry_name(s))
        .ok_or(UNAVAILABLE)?
        .to_owned();
    let meta = fs::symlink_metadata(path).map_err(|_| UNAVAILABLE)?;
    let token = identity(path).ok();
    let saved = operation(&id).and_then(|op| inspected(storage, op).ok());
    Ok(Entry {
        id,
        review_token: token,
        bytes: meta.len(),
        project_id: saved.as_ref().map(|(_, r)| r.binding.project_id.clone()),
        snapshot_id: saved.as_ref().map(|(_, r)| r.binding.snapshot_id.clone()),
        started_at_unix_ms: saved.as_ref().map(|(_, r)| r.started_at_unix_ms),
        candidate_count: saved.as_ref().map_or(0, |(r, _)| r.candidates.len()),
        available: saved.is_some(),
        question: None,
    })
}

pub(crate) fn list(storage: &Path) -> Result<Inventory, String> {
    let _guard = STORE.lock().map_err(|_| UNAVAILABLE)?;
    let directory = directory(storage)?;
    let _lease = lease(&directory)?;
    let mut entries = vec![];
    let mut limited = false;
    for item in fs::read_dir(&directory).map_err(|_| UNAVAILABLE)? {
        let path = item.map_err(|_| UNAVAILABLE)?.path();
        if path.file_name().and_then(|s| s.to_str()) == Some("store.lock") {
            continue;
        }
        if entries.len() == MAX_ENTRIES {
            limited = true;
            break;
        }
        entries.push(summarize(storage, &path)?);
    }
    // Read each relevant conversation once, after candidate/receipt validation.
    // Question labels are metadata, never inferred from untrusted model output.
    let mut groups = std::collections::HashMap::<(String, String), Vec<&str>>::new();
    for entry in &entries {
        if let (Some(project), Some(snapshot), Some(run)) =
            (&entry.project_id, &entry.snapshot_id, operation(&entry.id))
        {
            groups
                .entry((project.clone(), snapshot.clone()))
                .or_default()
                .push(run);
        }
    }
    let questions = groups
        .into_iter()
        .flat_map(|((project, snapshot), runs)| {
            crate::workspace_conversation::questions_for_runs(storage, &project, &snapshot, &runs)
        })
        .collect::<std::collections::HashMap<_, _>>();
    for entry in &mut entries {
        entry.question = operation(&entry.id).and_then(|run| questions.get(run).cloned());
    }
    entries.sort_by(|a, b| {
        b.started_at_unix_ms
            .cmp(&a.started_at_unix_ms)
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(Inventory {
        entries,
        limited,
        operation_limit: MAX_OPERATIONS,
        busy: false,
    })
}

pub(crate) fn review(storage: &Path, id: &str, token: &str) -> Result<Review, String> {
    let _guard = STORE.lock().map_err(|_| UNAVAILABLE)?;
    if !entry_name(id) {
        return Err(UNAVAILABLE.into());
    }
    let directory = directory(storage)?;
    let _lease = lease(&directory)?;
    let path = directory.join(id);
    if identity(&path)? != token {
        return Err(CHANGED.into());
    }
    let (saved, _) = inspected(storage, operation(id).ok_or(UNAVAILABLE)?)?;
    let mut entry = summarize(storage, &path)?;
    if entry.review_token.as_deref() != Some(token) {
        return Err(CHANGED.into());
    }
    if let (Some(project), Some(snapshot), Some(run)) =
        (&entry.project_id, &entry.snapshot_id, operation(id))
    {
        entry.question =
            crate::workspace_conversation::questions_for_runs(storage, project, snapshot, &[run])
                .remove(run);
    }
    Ok(Review {
        entry,
        attempts: saved
            .candidates
            .into_iter()
            .map(|c| Attempt {
                ordinal: c.ordinal,
                response_text: c.response_text,
                rejection: c.rejection,
                findings: c.preflight_findings.unwrap_or(c.alignment_issues),
            })
            .collect(),
    })
}

fn move_with(
    storage: &Path,
    id: &str,
    token: &str,
    move_file: impl FnOnce(&Path) -> Result<(), String>,
) -> Result<(), String> {
    let _guard = STORE.lock().map_err(|_| UNAVAILABLE)?;
    if !entry_name(id) {
        return Err(UNAVAILABLE.into());
    }
    let directory = directory(storage)?;
    let _lease = lease(&directory)?;
    let path = directory.join(id);
    if identity(&path)? != token {
        return Err(CHANGED.into());
    }
    // The operation/worker guard and the cross-process store lease remain held
    // through the OS move. No byte overwrite, recursive removal or hard-delete fallback.
    move_file(&path)
}

pub(crate) fn move_to_trash(storage: &Path, id: &str, token: &str) -> Result<(), String> {
    move_with(storage, id, token, |path| {
        #[cfg(target_os = "macos")]
        {
            use trash::macos::{DeleteMethod, TrashContextExtMacos};
            let mut context = trash::TrashContext::new();
            // The default Finder backend uses AppleScript and may request
            // Automation permission. Use the native file-manager API instead.
            context.set_delete_method(DeleteMethod::NsFileManager);
            context.delete(path).map_err(|_| {
                "The attempt could not be moved to Trash. Refresh the list before trying again."
                    .into()
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = path;
            Err("Moving saved attempts to Trash is supported in the Mac app.".into())
        }
    })
}

#[cfg(test)]
mod tests;
