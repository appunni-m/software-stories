//! Explicit continuation upgrades archive the exact previous record. Reading a
//! guide never upgrades it, reinterprets its scope or replenishes its attempts.

use super::*;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::repository_guide) struct Upgrade {
    pub schema_version: u32,
    pub contract: String,
    pub revision: u64,
    pub sha256: String,
}

pub(in crate::repository_guide) fn original_contract(record: &Record) -> (u32, &str) {
    match &record.upgrade {
        Some(prior) => (prior.schema_version, &prior.contract),
        None if record.schema_version == 5 && record.contract == CONTRACT => (4, SEPARATE_CONTRACT),
        None => (record.schema_version, &record.contract),
    }
}

fn history(storage: &Path) -> Result<std::path::PathBuf, String> {
    let directory = storage.join("repository-guide-history");
    codex_agent::ensure_private_directory(&directory)?;
    Ok(directory)
}

fn archive_path(
    storage: &Path,
    record: &Record,
    prior: &Upgrade,
) -> Result<std::path::PathBuf, String> {
    // Hash-addressed snapshots make an archive-before-write interruption
    // recoverable even if a later local reconciliation advances the revision.
    Ok(history(storage)?.join(format!("{}-{}.json", record.id, prior.sha256)))
}

pub(super) fn verify(storage: &Path, record: &Record) -> Result<(), String> {
    let Some(prior) = &record.upgrade else {
        return Ok(());
    };
    let bytes = crate::repository_plan_store::read_bounded_file(
        &archive_path(storage, record, prior)?,
        MAX_RECORD,
    )?;
    if context::digest(&bytes) != prior.sha256 {
        return Err("The guide's original record changed. Saved work was preserved.".into());
    }
    let original: Record = crate::strict_json::from_slice(&bytes)
        .and_then(serde_json::from_value)
        .map_err(|_| "The guide's original record is invalid. Saved work was preserved.")?;
    validate(&original)?;
    if original.upgrade.is_some()
        || !original.opening_continuations.is_empty()
        || original.schema_version != prior.schema_version
        || original.contract != prior.contract
        || original.revision != prior.revision
        || original.id != record.id
        || original.project_id != record.project_id
        || original.snapshot_id != record.snapshot_id
        || original.manifest_sha256 != record.manifest_sha256
        || original.grant_id != record.grant_id
        || original.question != record.question
        || original.planning_run_id != record.planning_run_id
        || original.opening_context_sha256 != record.opening_context_sha256
        || original.plan != record.plan
        || serde_json::to_value(&original.opening).ok()
            != serde_json::to_value(&record.opening).ok()
        || original.units.len() != record.units.len()
        || original.units.iter().zip(&record.units).any(|(old, new)| {
            old.key != new.key
                || new.attempts.len() < old.attempts.len()
                || serde_json::to_value(&old.attempts).ok()
                    != serde_json::to_value(&new.attempts[..old.attempts.len()]).ok()
        })
    {
        return Err("The guide's original scope, outline or spent investigations changed.".into());
    }
    Ok(())
}

fn archive(storage: &Path, record: &Record) -> Result<Upgrade, String> {
    let bytes =
        crate::repository_plan_store::read_bounded_file(&path(storage, &record.id)?, MAX_RECORD)?;
    let prior = Upgrade {
        schema_version: record.schema_version,
        contract: record.contract.clone(),
        revision: record.revision,
        sha256: context::digest(&bytes),
    };
    let destination = archive_path(storage, record, &prior)?;
    if destination
        .try_exists()
        .map_err(|_| "Guide history could not be inspected.")?
    {
        if crate::repository_plan_store::read_bounded_file(&destination, MAX_RECORD)? != bytes {
            return Err("The original guide archive conflicts. Saved work was preserved.".into());
        }
    } else {
        let directory = history(storage)?;
        let entries = scan(&directory)?;
        if entries.len() >= MAX_ENTRIES
            || entries.iter().map(|(_, size)| size).sum::<u64>() + bytes.len() as u64 > MAX_BYTES
        {
            return Err("Guide history storage is full. No continuation was started.".into());
        }
        codex_agent::create_private_file(&destination, &bytes)?;
        File::open(&directory)
            .and_then(|file| file.sync_all())
            .map_err(
                |_| "Guide history publication is uncertain. Inspect saved work before retrying.",
            )?;
    }
    Ok(prior)
}

/// Called only after explicit consent, the guide lease, reconciliation and
/// current runtime/grant checks. A failed compare never archives or dispatches.
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
        if matches!(current.schema_version, 2..=4) {
            current.upgrade = Some(archive(storage, current)?);
            current.schema_version = 5;
            current.contract = CONTRACT.into();
        }
        current.status = Status::Building;
        Ok(())
    })
}
