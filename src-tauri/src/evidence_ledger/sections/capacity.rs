//! Reserve answer attempts that already exist and capacity for work each guide
//! can still perform. No content is migrated, evicted, reclassified as
//! accepted, or granted inference authority here.

use super::*;
use crate::repository_guide::{AnswerReservation, MAX_GUIDE_ANSWER_RECORDS};
use std::collections::HashMap;

fn reserved_records(
    guide_slots: impl IntoIterator<Item = usize>,
    standalone_count: usize,
) -> Option<usize> {
    guide_slots
        .into_iter()
        .try_fold(standalone_count, |total, slots| total.checked_add(slots))
}

#[cfg(unix)]
pub(super) fn lease(storage: &Path) -> Result<crate::file_lease::Lease, String> {
    use std::os::unix::fs::OpenOptionsExt;
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(storage.join(".answer-capacity.lock"))
        .map_err(|_| "Explanation storage could not be leased. Existing work was preserved.")?;
    crate::file_lease::try_exclusive(file)
        .map_err(|_| "Explanation storage could not be leased. Existing work was preserved.")?
        .ok_or_else(|| {
            "Another explanation storage transaction is active. No new investigation was started."
                .into()
        })
}

#[cfg(not(unix))]
pub(super) fn lease(_storage: &Path) -> Result<crate::file_lease::Lease, String> {
    Err("Explanation storage requires a native Unix filesystem lease.".into())
}

/// Guide creation holds this same capacity transaction through publication.
/// A new guide reserves its full bounded allowance; saved guides reserve their
/// persisted attempts and only the additional attempts they can still publish.
pub(crate) fn with_guide_reservation<T>(
    storage: &Path,
    candidate: &AnswerReservation,
    publish: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
    let _lease = lease(storage)?;
    check(storage, Some(candidate), None)?;
    publish()
}

fn owned(binding: &Binding, owner: &AnswerReservation) -> Result<(), String> {
    if binding.project_id != owner.project_id
        || binding.snapshot_id != owner.snapshot_id
        || binding.manifest_sha256 != owner.manifest_sha256
    {
        return Err(
            "A saved guide answer belongs to another scope. Existing work was preserved.".into(),
        );
    }
    Ok(())
}

pub(super) fn check(
    storage: &Path,
    candidate: Option<&AnswerReservation>,
    incoming: Option<&Binding>,
) -> Result<(), String> {
    let mut reservations = crate::repository_guide::answer_reservations(storage)?;
    if let Some(candidate) = candidate {
        match reservations
            .iter()
            .find(|saved| saved.guide_id == candidate.guide_id)
        {
            Some(saved) if saved != candidate => {
                return Err(
                    "The guide changed while storage was checked. Existing work was preserved."
                        .into(),
                );
            }
            Some(_) => {}
            None => reservations.push(candidate.clone()),
        }
    }
    let mut owners = HashMap::new();
    for reservation in &reservations {
        if reservation.run_ids.len() > MAX_GUIDE_ANSWER_RECORDS
            || reservation.reserved_records < reservation.run_ids.len()
            || reservation.reserved_records > MAX_GUIDE_ANSWER_RECORDS
        {
            return Err(
                "A guide exceeds its investigation allowance. Existing work was preserved.".into(),
            );
        }
        for id in &reservation.run_ids {
            if owners.insert(id.as_str(), reservation).is_some() {
                return Err(
                    "Saved guides repeat an investigation identity. Existing work was preserved."
                        .into(),
                );
            }
        }
    }
    let mut standalone_records = 0usize;
    let directory = storage.join("answer-sections");
    match fs::symlink_metadata(&directory) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
            for (index, entry) in fs::read_dir(&directory)
                .map_err(|_| "Saved answers could not be inspected.")?
                .enumerate()
            {
                if index >= MAX_DIRECTORY_ENTRIES {
                    return Err(
                        "Saved answers reached their directory limit. Existing work was preserved."
                            .into(),
                    );
                }
                let path = entry
                    .map_err(|_| "Saved answers could not be inspected.")?
                    .path();
                if !path
                    .extension()
                    .is_some_and(|extension| extension == "json")
                {
                    continue;
                }
                let bytes = read_bytes(&path)?;
                let envelope: serde_json::Value = crate::strict_json::from_slice(&bytes)
                    .map_err(|_| "A saved answer is damaged; it was preserved.")?;
                let operation_id = path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .ok_or("A saved answer identity is invalid; it was preserved.")?;
                if !canonical_uuid(operation_id) {
                    return Err("A saved answer identity is invalid; it was preserved.".into());
                }
                let schema_version = envelope
                    .get("schemaVersion")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or("A saved answer has no valid schema version; it was preserved.")?;
                if schema_version < NARRATION_ANSWER_VERSION as u64 {
                    // Historical receipts stay byte-for-byte on disk and
                    // count toward the same total. Guide-owned paths are
                    // already covered by that guide's reserved lifetime slots.
                    if !owners.contains_key(operation_id) {
                        standalone_records = standalone_records
                            .checked_add(1)
                            .ok_or("Saved answer capacity could not be counted.")?;
                    }
                    continue;
                }
                if schema_version > NARRATION_ANSWER_VERSION as u64 {
                    return Err(
                        "A newer saved answer exists. Update Code Atlas before starting another investigation.".into(),
                    );
                }
                let answer: Answer = serde_json::from_value(envelope)
                    .map_err(|_| "A saved answer is damaged; it was preserved.")?;
                if answer.binding.operation_id != operation_id {
                    return Err(
                        "A saved answer identity changed. Existing work was preserved.".into(),
                    );
                }
                if incoming
                    .is_some_and(|binding| binding.operation_id == answer.binding.operation_id)
                {
                    return Err(
                        "This investigation already has a saved answer. It was preserved.".into(),
                    );
                }
                if let Some(owner) = owners.get(answer.binding.operation_id.as_str()) {
                    owned(&answer.binding, owner)?;
                } else {
                    standalone_records = standalone_records
                        .checked_add(1)
                        .ok_or("Saved answer capacity could not be counted.")?;
                }
            }
        }
        _ => {
            return Err(
                "Answer storage is not a regular local directory. Existing work was preserved."
                    .into(),
            );
        }
    }
    if let Some(binding) = incoming {
        if let Some(owner) = owners.get(binding.operation_id.as_str()) {
            owned(binding, owner)?;
        } else {
            standalone_records = standalone_records
                .checked_add(1)
                .ok_or("Saved answer capacity could not be counted.")?;
        }
    }
    // One thousand record slots total. Saved guides account for their attempts
    // and remaining bounded retries; standalone and historical receipts each
    // consume one slot. New guides reserve their full allowance before planning.
    let reserved = reserved_records(
        reservations
            .iter()
            .map(|reservation| reservation.reserved_records),
        standalone_records,
    )
    .ok_or("Saved answer capacity could not be counted.")?;
    if reserved > MAX_ANSWER_RECORDS {
        return Err("Saved explanations reached their storage limit of 1,000 answer records. Existing answers and guides were preserved; no new investigation was started.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guide_lifetime_allowances_and_standalone_receipts_share_one_thousand_slots() {
        let remaining = MAX_ANSWER_RECORDS - 2 * MAX_GUIDE_ANSWER_RECORDS;
        assert_eq!(
            reserved_records([MAX_GUIDE_ANSWER_RECORDS; 2], remaining),
            Some(MAX_ANSWER_RECORDS)
        );
        assert_eq!(
            reserved_records([MAX_GUIDE_ANSWER_RECORDS; 2], remaining + 1),
            Some(MAX_ANSWER_RECORDS + 1)
        );
    }

    #[test]
    fn answer_capacity_arithmetic_rejects_overflow() {
        assert_eq!(reserved_records([usize::MAX], 1), None);
    }
}
