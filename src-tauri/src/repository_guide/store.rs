//! Bounded private job records. A cooperative file lease spans the entire
//! worker; short compare-and-write transactions protect stop and publication.

use super::*;
use std::fs::{self, File, OpenOptions};

const MAX_RECORD: usize = 256 * 1024;
const MAX_GUIDES: usize = 1_000;
const MAX_LOCK_FILES: usize = MAX_GUIDES + 256;
const MAX_BYTES: u64 = MAX_GUIDES as u64 * MAX_RECORD as u64;
pub(super) static STORE: Mutex<()> = Mutex::new(());

struct StoreEntries {
    records: Vec<(String, u64)>,
    lock_files: usize,
    bytes: u64,
}

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
    let value = crate::strict_json::from_slice(&bytes)
        .map_err(|_| "The saved guide is damaged or incompatible. It was preserved.")?;
    decode_record(id, value)
}

fn decode_record(id: &str, value: serde_json::Value) -> Result<Record, String> {
    let record: Record = serde_json::from_value(value)
        .map_err(|_| "The saved guide is damaged or incompatible. It was preserved.")?;
    validate(&record)?;
    if record.id != id {
        return Err("The saved guide identity changed.".into());
    }
    Ok(record)
}

/// Read only the ownership fields needed to scope a saved record. Unknown
/// fields are allowed here so an unsupported record from another repository
/// cannot prevent this repository from reopening; the complete current schema
/// is still required before a matching record is returned or used.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordScope {
    project_id: String,
    snapshot_id: String,
}

/// Version metadata is used only for capacity accounting. Unsupported records
/// remain opaque and are never returned as guides or rewritten.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RecordContract {
    schema_version: u32,
    contract: String,
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
    if entries.bytes.saturating_sub(previous) + bytes.len() as u64 > MAX_BYTES
        || (new && entries.records.len() >= MAX_GUIDES)
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
fn scan(directory: &Path) -> Result<StoreEntries, String> {
    let mut entries = StoreEntries {
        records: Vec::new(),
        lock_files: 0,
        bytes: 0,
    };
    for entry in fs::read_dir(directory).map_err(|_| "Saved guides could not be listed.")? {
        let entry = entry.map_err(|_| "A guide entry could not be inspected.")?;
        let meta = fs::symlink_metadata(entry.path())
            .map_err(|_| "A guide entry could not be inspected.")?;
        if !meta.is_file() || meta.file_type().is_symlink() {
            return Err(
                "The guide directory contains a nonregular entry. Existing work was preserved."
                    .into(),
            );
        }
        let name = entry
            .file_name()
            .to_str()
            .ok_or("A guide filename is invalid.")?
            .to_owned();
        if name.ends_with(".json") {
            if entries.records.len() >= MAX_GUIDES || meta.len() > MAX_RECORD as u64 {
                return Err(
                    "The saved guide storage limit was reached. Existing work was preserved."
                        .into(),
                );
            }
            entries.bytes = entries.bytes.saturating_add(meta.len());
            if entries.bytes > MAX_BYTES {
                return Err(
                    "Saved guide storage exceeds its limit. Existing work was preserved.".into(),
                );
            }
            entries.records.push((name, meta.len()));
        } else if name.ends_with(".lock") {
            if entries.lock_files >= MAX_LOCK_FILES || meta.len() != 0 {
                return Err(
                    "The guide lock-file limit was reached. Existing work was preserved.".into(),
                );
            }
            entries.lock_files += 1;
        } else {
            return Err(
                "The guide directory contains an unsupported entry. Existing work was preserved."
                    .into(),
            );
        }
    }
    Ok(entries)
}
pub(super) fn list(
    storage: &Path,
    project: &str,
    snapshot: &str,
) -> Result<(Vec<Record>, usize), String> {
    let mut records = Vec::new();
    let mut unavailable_count = 0;
    for (name, _) in scan(&directory(storage)?)?.records {
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        let bytes =
            crate::repository_plan_store::read_bounded_file(&path(storage, id)?, MAX_RECORD)?;
        let value = crate::strict_json::from_slice(&bytes)
            .map_err(|_| "The saved guide is damaged or incompatible. It was preserved.")?;
        let scope: RecordScope = serde_json::from_value(value.clone())
            .map_err(|_| "The saved guide is damaged or incompatible. It was preserved.")?;
        if scope.project_id == project && scope.snapshot_id == snapshot {
            match decode_record(id, value) {
                Ok(record) => records.push(record),
                Err(_) => unavailable_count += 1,
            }
        }
    }
    Ok((records, unavailable_count))
}

pub(super) fn answer_reservations(storage: &Path) -> Result<Vec<AnswerReservation>, String> {
    let directory = storage.join("repository-guides");
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        _ => return Err("Guide storage could not be checked. Existing work was preserved.".into()),
    }
    let mut reservations = Vec::new();
    for (name, _) in scan(&directory)?.records {
        let Some(id) = name.strip_suffix(".json") else {
            continue;
        };
        let bytes =
            crate::repository_plan_store::read_bounded_file(&path(storage, id)?, MAX_RECORD)?;
        let value = crate::strict_json::from_slice(&bytes)
            .map_err(|_| "The saved guide is damaged or incompatible. It was preserved.")?;
        let contract: RecordContract = serde_json::from_value(value.clone())
            .map_err(|_| "The saved guide is damaged or incompatible. It was preserved.")?;
        if contract.schema_version != RECORD_SCHEMA_VERSION {
            continue;
        }
        if contract.contract != CONTRACT {
            return Err("The saved repository guide has an incompatible contract.".into());
        }
        let record = decode_record(id, value)?;
        reservations.push(answer_reservation(&record));
    }
    Ok(reservations)
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
    if !lock_path.exists() && scan(&directory)?.lock_files >= MAX_LOCK_FILES {
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
        || record.planning_retry_count > 2
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
        || record.units.len() > MAX_TOTAL_UNITS
        || record.active_run_id.as_ref().is_some_and(|id| !uuid(id))
    {
        return Err("The saved repository guide has an incompatible contract.".into());
    }
    if record.opening_continuations.len() > MAX_OPENING_CONTINUATIONS
        || record
            .opening_continuations
            .iter()
            .filter(|attempt| attempt.automatic_retry)
            .count()
            > usize::from(MAX_AUTOMATIC_RETRIES_PER_UNIT)
        || record
            .opening_continuations
            .iter()
            .filter(|attempt| !attempt.automatic_retry)
            .count()
            > 1
    {
        return Err("The guide's opening continuation contract changed.".into());
    }
    let mut runs = std::collections::HashSet::from([record.planning_run_id.as_str()]);
    if let Some(opening) = &record.opening {
        if opening.automatic_retry
            || !uuid(&opening.run_id)
            || (opening.run_id != record.planning_run_id && !runs.insert(&opening.run_id))
        {
            return Err("The guide opening investigation identity or retry state changed.".into());
        }
        opening::question(record)?;
        validate_summary(record, opening)?;
    } else if record.opening_context_sha256.is_some() {
        return Err("The saved guide has learning intent without an opening.".into());
    }
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
        let original_units = plan
            .units
            .iter()
            .position(|unit| unit.continuation_topic.is_some())
            .unwrap_or(plan.units.len());
        let continuation_units = plan.units.len().saturating_sub(original_units);
        if !(2..=MAX_UNITS).contains(&original_units)
            || plan.units.len() > MAX_TOTAL_UNITS
            || continuation_units > MAX_TOTAL_UNITS.saturating_sub(original_units)
            || record.units.len() != plan.units.len()
            || !text(&plan.title, 8, 120)
            || !(1..=4).contains(&plan.opening_ranges.len())
            || !valid_omissions(&plan.omissions)
        {
            return Err("The saved guide outline is invalid.".into());
        }
        let mut keys = std::collections::HashSet::new();
        let mut continuation_topics = std::collections::HashSet::new();
        let mut continuation_seen = false;
        for (unit, progress) in plan.units.iter().zip(&record.units) {
            if unit.continuation_topic.is_some() {
                continuation_seen = true;
            }
            let valid_starting_ranges = if unit.continuation_topic.is_some() {
                unit.starting_ranges.len() <= 4
            } else {
                (1..=4).contains(&unit.starting_ranges.len())
            };
            let valid_continuation_topic = unit.continuation_topic.as_deref().is_none_or(|topic| {
                let normalized = topic
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase();
                text(topic, 1, 240)
                    && unit.question == super::extension_question(topic)
                    && unit.prerequisites.is_empty()
                    && continuation_topics.insert(normalized.clone())
                    && !plan.omissions.iter().any(|omission| {
                        omission
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .to_lowercase()
                            == normalized
                    })
            });
            if !key(&unit.key)
                || keys.contains(&unit.key)
                || unit.key != progress.key
                || (continuation_seen && unit.continuation_topic.is_none())
                || !valid_continuation_topic
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
                || !valid_starting_ranges
                || progress
                    .attempts
                    .iter()
                    .filter(|attempt| attempt.automatic_retry)
                    .count()
                    > usize::from(MAX_AUTOMATIC_RETRIES_PER_UNIT)
                || progress
                    .attempts
                    .first()
                    .is_some_and(|attempt| attempt.automatic_retry)
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
    let consumed: std::collections::HashSet<_> = record
        .consumed_slide_revisions
        .iter()
        .map(String::as_str)
        .collect();
    let published: std::collections::HashSet<_> = record
        .opening
        .iter()
        .chain(&record.opening_continuations)
        .chain(record.units.iter().flat_map(|unit| &unit.attempts))
        .filter_map(|attempt| attempt.summary.as_ref())
        .flat_map(|answer| &answer.sections)
        .map(|section| section.revision_id.as_str())
        .collect();
    if record.consumed_slide_revisions.len() > MAX_GUIDE_ANSWER_RECORDS
        || consumed.len() != record.consumed_slide_revisions.len()
        || consumed
            .iter()
            .any(|revision| !published.contains(revision))
    {
        return Err("The saved guide's slide progression receipts are invalid.".into());
    }
    if record.status == Status::WaitingForPlayback
        && (record.plan.is_none()
            || record.active_run_id.is_some()
            || crate::repository_guide::opening_continuation::remaining_operations(record) == 0)
    {
        return Err("The saved guide is waiting without unfinished slides.".into());
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
        if opening_continuation::remaining_operations(current) == 0
            && !super::plan_extension_eligible(current)
        {
            return Err("This guide has no remaining investigations. Its saved sections and unfinished boundaries remain available.".into());
        }
        current.status = Status::Building;
        Ok(())
    })
}

/// Admit one explicit fresh outline attempt after a failed or interrupted
/// planning run. It keeps the guide identity and source grant while giving the
/// Codex operation a new owner and the same bounded planning contract.
pub(in crate::repository_guide) fn prepare_plan_retry(
    storage: &Path,
    id: &str,
    revision: u64,
) -> Result<Record, String> {
    update(storage, id, |current| {
        if current.revision != revision
            || current.active_run_id.is_some()
            || current.plan.is_some()
            || current.status == Status::Stopped
            || current.planning_retry_count >= 2
            || current.units.len() != 0
            || !current.opening_continuations.is_empty()
            || current
                .opening
                .as_ref()
                .is_none_or(|opening| opening.summary.is_some())
        {
            return Err(
                "This outline cannot be retried safely. Existing guide progress was preserved."
                    .into(),
            );
        }
        current.planning_retry_count += 1;
        current.planning_run_id = uuid::Uuid::new_v4().to_string();
        current.active_run_id = Some(current.planning_run_id.clone());
        current.status = Status::Planning;
        Ok(())
    })
}
