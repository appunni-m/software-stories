//! Bounded private job records. A cooperative file lease spans the entire
//! worker; short compare-and-write transactions protect stop and publication.

use super::*;
use std::fs::{self, File, OpenOptions};

const MAX_RECORD: usize = 256 * 1024;
const MAX_ENTRIES: usize = 64;
const MAX_BYTES: u64 = 8 * 1024 * 1024;
pub(super) static STORE: Mutex<()> = Mutex::new(());

fn directory(storage: &Path) -> Result<std::path::PathBuf, String> {
    let directory = storage.join("repository-guides");
    codex_agent::ensure_private_directory(&directory)?;
    Ok(directory)
}
fn path(storage: &Path, id: &str) -> Result<std::path::PathBuf, String> {
    if !uuid(id) {
        return Err("The repository guide identity is invalid.".into());
    }
    Ok(directory(storage)?.join(format!("{id}.json")))
}
pub(super) fn read(storage: &Path, id: &str) -> Result<Record, String> {
    let bytes = crate::repository_plan_store::read_bounded_file(&path(storage, id)?, MAX_RECORD)?;
    let record: Record = crate::strict_json::from_slice(&bytes)
        .and_then(serde_json::from_value)
        .map_err(|_| "The saved guide is damaged or incompatible. It was preserved.")?;
    validate(&record)?;
    if record.id != id {
        return Err("The saved guide identity changed.".into());
    }
    Ok(record)
}
pub(super) fn exists(storage: &Path, id: &str) -> Result<bool, String> {
    match fs::symlink_metadata(path(storage, id)?) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("The saved guide could not be inspected.".into()),
    }
}
pub(super) fn save(storage: &Path, record: &Record, new: bool) -> Result<(), String> {
    if new {
        validate(record)?;
        return crate::evidence_ledger::sections::capacity::with_guide_reservation(
            storage,
            &answer_reservation(record),
            || save_record(storage, record, true),
        );
    }
    save_record(storage, record, false)
}

fn save_record(storage: &Path, record: &Record, new: bool) -> Result<(), String> {
    validate(record)?;
    let bytes = serde_json::to_vec(record).map_err(|_| "The guide could not be encoded.")?;
    if bytes.len() > MAX_RECORD {
        return Err("The guide record exceeds its local storage limit.".into());
    }
    let directory = directory(storage)?;
    let entries = scan(&directory)?;
    let destination = path(storage, &record.id)?;
    let previous = fs::symlink_metadata(&destination)
        .ok()
        .map_or(0, |m| m.len());
    if entries
        .iter()
        .map(|(_, size)| size)
        .sum::<u64>()
        .saturating_sub(previous)
        + bytes.len() as u64
        > MAX_BYTES
        || (new && entries.len() >= MAX_ENTRIES)
    {
        return Err("Repository guide storage is full. Existing work was preserved.".into());
    }
    if new {
        codex_agent::create_private_file(&destination, &bytes)?;
        File::open(&directory).and_then(|f| f.sync_all()).map_err(
            |_| "Guide publication could not be confirmed; inspect saved work before retrying.",
        )?;
        Ok(())
    } else {
        codex_agent::atomic_private_write(&destination, &bytes)
    }
}
fn scan(directory: &Path) -> Result<Vec<(String, u64)>, String> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory).map_err(|_| "Saved guides could not be listed.")? {
        if entries.len() >= MAX_ENTRIES {
            return Err(
                "The guide directory limit was reached. Existing work was preserved.".into(),
            );
        }
        let entry = entry.map_err(|_| "A guide entry could not be inspected.")?;
        let meta = fs::symlink_metadata(entry.path())
            .map_err(|_| "A guide entry could not be inspected.")?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(
                "The guide directory contains a nonregular entry. Existing work was preserved."
                    .into(),
            );
        }
        entries.push((
            entry
                .file_name()
                .to_str()
                .ok_or("A guide filename is invalid.")?
                .to_owned(),
            meta.len(),
        ));
    }
    if entries.iter().map(|(_, size)| size).sum::<u64>() > MAX_BYTES {
        return Err("Saved guide storage exceeds its limit.".into());
    }
    Ok(entries)
}
pub(super) fn list(storage: &Path, project: &str, snapshot: &str) -> Result<Vec<Record>, String> {
    let mut records = Vec::new();
    for (name, _) in scan(&directory(storage)?)? {
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        let record = read(storage, id)?;
        if record.project_id == project && record.snapshot_id == snapshot {
            records.push(record);
        }
    }
    Ok(records)
}

pub(super) fn answer_reservations(storage: &Path) -> Result<Vec<AnswerReservation>, String> {
    let directory = storage.join("repository-guides");
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        _ => return Err("Guide storage could not be checked. Existing work was preserved.".into()),
    }
    scan(&directory)?
        .into_iter()
        .filter_map(|(name, _)| {
            name.strip_suffix(".json")
                .map(|id| read(storage, id).map(|record| answer_reservation(&record)))
        })
        .collect()
}

/// Native owner explicitly releases the lease on return or unwind.
#[cfg(unix)]
pub(super) fn lease(storage: &Path, id: &str) -> Result<Option<crate::file_lease::Lease>, String> {
    if !uuid(id) {
        return Err("The guide worker identity is invalid.".into());
    }
    lock_named(storage, &format!("{id}.lock"))
}
#[cfg(unix)]
fn lock_named(storage: &Path, name: &str) -> Result<Option<crate::file_lease::Lease>, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let directory = directory(storage)?;
    let lock_path = directory.join(name);
    if !lock_path.exists() && scan(&directory)?.len() >= MAX_ENTRIES {
        return Err("The guide directory limit was reached. No new worker was admitted.".into());
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(lock_path)
        .map_err(|_| "The guide worker lease could not be opened.")?;
    if !file
        .metadata()
        .map_err(|_| "The guide lease could not be inspected.")?
        .is_file()
    {
        return Err("The guide lease is not a regular local file.".into());
    }
    crate::file_lease::try_exclusive(file).map_err(|_| {
        "The filesystem cannot lease a guide worker. Existing work was preserved.".into()
    })
}
#[cfg(not(unix))]
fn lock_named(_storage: &Path, _name: &str) -> Result<Option<crate::file_lease::Lease>, String> {
    Err("Guide transactions require a native Unix filesystem lease.".into())
}
#[cfg(not(unix))]
pub(super) fn lease(
    _storage: &Path,
    _id: &str,
) -> Result<Option<crate::file_lease::Lease>, String> {
    Err("Repository guide workers require a native Unix filesystem lease.".into())
}

pub(super) fn update(
    storage: &Path,
    id: &str,
    change: impl FnOnce(&mut Record) -> Result<(), String>,
) -> Result<Record, String> {
    update_if_changed(storage, id, |record| {
        change(record)?;
        Ok(true)
    })
}

/// The callback returns false when the transaction should preserve the existing
/// bytes and revision. Callers must leave the record unchanged in that case.
pub(super) fn update_if_changed(
    storage: &Path,
    id: &str,
    change: impl FnOnce(&mut Record) -> Result<bool, String>,
) -> Result<Record, String> {
    let _guard = STORE.lock().map_err(|_| "Guide storage is unavailable.")?;
    let _transaction=lock_named(storage,".transactions.lock")?.ok_or("Another native guide transaction is active. Retry the local action; no inference was started.")?;
    let mut record = read(storage, id)?;
    let revision = record.revision;
    if !change(&mut record)? {
        return Ok(record);
    }
    record.revision = revision
        .checked_add(1)
        .ok_or("The guide revision limit was reached. Existing work was preserved.")?;
    save(storage, &record, false)?;
    Ok(record)
}
pub(super) fn create(storage: &Path, record: &Record) -> Result<(), String> {
    let _guard = STORE.lock().map_err(|_| "Guide storage is unavailable.")?;
    let _transaction = lock_named(storage, ".transactions.lock")?
        .ok_or("Another native guide transaction is active. No guide was started.")?;
    save(storage, record, true)
}

fn validate(record: &Record) -> Result<(), String> {
    if record.schema_version != RECORD_SCHEMA_VERSION
        || record.contract != CONTRACT
        || record.opening.is_none()
        || !uuid(&record.id)
        || !uuid(&record.planning_run_id)
        || !uuid(&record.grant_id)
        || !text(&record.question, 1, 12000)
        || record.project_id.is_empty()
        || record.snapshot_id.is_empty()
        || !model_text(&record.model_id, 200)
        || !model_text(&record.reasoning_effort, 32)
        || record.manifest_sha256.len() != 64
        || !record
            .manifest_sha256
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
        || record.units.len() > MAX_UNITS
        || record.active_run_id.as_ref().is_some_and(|id| !uuid(id))
    {
        return Err("The saved repository guide has an incompatible contract.".into());
    }
    if record.opening_continuations.len() > 1 {
        return Err("The guide's opening continuation contract changed.".into());
    }
    if let Some(opening) = &record.opening {
        opening::question(record)?;
        validate_summary(record, opening)?;
    } else if record.opening_context_sha256.is_some() {
        return Err("The saved guide has learning intent without an opening.".into());
    }
    let mut runs = std::collections::HashSet::from([record.planning_run_id.as_str()]);
    for attempt in &record.opening_continuations {
        if record.plan.is_none()
            || record
                .opening
                .as_ref()
                .and_then(|a| a.summary.as_ref())
                .is_none_or(|s| matches!(s.outcome, Outcome::Complete | Outcome::Working))
            || !uuid(&attempt.run_id)
            || !runs.insert(&attempt.run_id)
            || attempt.question != opening::question(record)?
        {
            return Err("The guide's opening continuation owner or spent budget changed.".into());
        }
        validate_summary(record, attempt)?;
    }
    if let Some(plan) = &record.plan {
        if !(2..=MAX_UNITS).contains(&plan.units.len())
            || record.units.len() != plan.units.len()
            || !text(&plan.title, 8, 120)
            || plan.omissions.len() > 8
            || plan.omissions.iter().any(|s| !text(s, 1, 240))
        {
            return Err("The saved guide outline is invalid.".into());
        }
        let mut keys = std::collections::HashSet::new();
        for (unit, progress) in plan.units.iter().zip(&record.units) {
            if !key(&unit.key)
                || keys.contains(&unit.key)
                || unit.key != progress.key
                || !text(&unit.question, 16, 800)
                || !unit.question.ends_with('?')
                || unit.prerequisites.iter().any(|k| !keys.contains(k))
                || unit.prerequisites.len() > keys.len()
                || unit
                    .prerequisites
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != unit.prerequisites.len()
                || !(1..=4).contains(&unit.starting_ranges.len())
                || progress.attempts.len() > MAX_ATTEMPTS
            {
                return Err("The saved guide's units or prerequisites changed.".into());
            }
            keys.insert(unit.key.clone());
            for attempt in &progress.attempts {
                if !uuid(&attempt.run_id)
                    || !runs.insert(attempt.run_id.as_str())
                    || attempt.question.is_empty()
                    || attempt.question.len() > 12000
                {
                    return Err("The saved guide repeats an investigation identity.".into());
                }
                validate_summary(record, attempt)?;
            }
        }
    } else if !record.units.is_empty() {
        return Err("The saved guide has progress without an outline.".into());
    }
    if record
        .active_run_id
        .as_ref()
        .is_some_and(|id| !runs.contains(id.as_str()))
    {
        return Err("The guide's active investigation is unknown.".into());
    }
    if record.status == Status::Ready && (record.plan.is_none() || !guide_complete(record)) {
        return Err("The saved guide is marked ready without complete units.".into());
    }
    Ok(())
}

fn validate_summary(record: &Record, attempt: &Attempt) -> Result<(), String> {
    if let Some(summary) = &attempt.summary {
        if summary.run_id != attempt.run_id
            || summary.question != attempt.question
            || summary.project_id != record.project_id
            || summary.snapshot_id != record.snapshot_id
            || summary.sections.len() > 8
            || (summary.outcome == Outcome::Complete
                && (summary.sections.is_empty() || !summary.incomplete_topics.is_empty()))
            || summary.incomplete_topics.len() > 6
            || summary.incomplete_topics.iter().any(|t| !text(t, 1, 1000))
            || summary.sections.iter().any(|s| {
                s.run_id != attempt.run_id
                    || s.project_id != record.project_id
                    || s.snapshot_id != record.snapshot_id
                    || !key(&s.key)
                    || s.order > 5
                    || !s
                        .generation_id
                        .strip_prefix("generation:")
                        .is_some_and(uuid)
                    || s.revision_id.len() > 256
            })
        {
            return Err("The saved guide's answer lineage or bounds changed.".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;

/// Continue an existing current guide without converting stored formats.
pub(in crate::repository_guide) fn prepare_continuation(
    storage: &Path,
    id: &str,
    revision: u64,
) -> Result<Record, String> {
    update(storage, id, |current| {
        if current.revision != revision || current.active_run_id.is_some() {
            return Err("The guide changed while continuation was being prepared. Inspect its saved progress before continuing.".into());
        }
        if opening_continuation::remaining_operations(current) == 0 {
            return Err("This guide has no remaining investigations. Its saved sections and unfinished boundaries remain available.".into());
        }
        current.status = Status::Building;
        Ok(())
    })
}
