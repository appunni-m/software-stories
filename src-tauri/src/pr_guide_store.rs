use crate::git_evidence::EvidenceClassification;
use crate::pr_guide::{EvidenceSide, PrGuideClaimCategory, PrGuideLifecycle, PrGuideRevision};
use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

const MAX_REVISIONS_PER_COMPARISON: usize = 2_000;
const MAX_STAGING_FILES_PER_COMPARISON: usize = 32;
const MAX_REVISION_BYTES: usize = 2 * 1024 * 1024;
const MAX_TOTAL_BYTES_PER_COMPARISON: usize = 64 * 1024 * 1024;
const STORE_SCHEMA_VERSION: u32 = 1;
static PR_GUIDE_STORE_LOCK: Mutex<()> = Mutex::new(());

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RevisionFile {
    schema_version: u32,
    revision: PrGuideRevision,
}

pub fn list_for_comparison(
    storage_dir: &Path,
    project_id: &str,
    comparison_id: &str,
) -> Result<Vec<PrGuideRevision>, String> {
    let _guard = lock_store()?;
    let Some(directory) = existing_comparison_directory(storage_dir, project_id, comparison_id)?
    else {
        return Ok(Vec::new());
    };
    let mut revisions = read_all(&directory)?;
    revisions.sort_by_key(|revision| revision.revision_number);
    Ok(revisions)
}

/// Saves a newly generated and locally validated PR Guide as an immutable draft.
pub fn save_draft(
    storage_dir: &Path,
    mut revision: PrGuideRevision,
) -> Result<PrGuideRevision, String> {
    let _guard = lock_store()?;
    if revision.lifecycle != PrGuideLifecycle::Draft || revision.based_on_revision_id.is_some() {
        return Err("Only a new, unaccepted PR Guide draft can be saved by generation.".to_owned());
    }
    validate_revision(&revision)?;
    let directory = create_comparison_directory(
        storage_dir,
        &revision.project_id,
        &revision.comparison.comparison_id,
    )?;
    let existing = read_all(&directory)?;
    if existing.len() >= MAX_REVISIONS_PER_COMPARISON {
        return Err("This comparison reached the local PR Guide revision limit.".to_owned());
    }
    if existing
        .iter()
        .any(|item| item.revision_id == revision.revision_id)
    {
        return Err("This immutable PR Guide revision ID already exists.".to_owned());
    }
    revision.revision_number = existing
        .iter()
        .map(|item| item.revision_number)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "The PR Guide revision number is exhausted.".to_owned())?;
    publish_revision(&directory, &revision, &existing)
}

/// Appends an accepted revision containing only the claims selected in review.
pub fn accept_draft(
    storage_dir: &Path,
    project_id: &str,
    comparison_id: &str,
    revision_id: &str,
    included_claim_ids: &[String],
) -> Result<PrGuideRevision, String> {
    let _guard = lock_store()?;
    if included_claim_ids.is_empty() {
        return Err("Keep at least one validated claim in the accepted PR Guide.".to_owned());
    }
    let directory = existing_comparison_directory(storage_dir, project_id, comparison_id)?
        .ok_or_else(|| "The selected PR Guide history is no longer available.".to_owned())?;
    let existing = read_all(&directory)?;
    let draft = existing
        .iter()
        .find(|revision| revision.revision_id == revision_id)
        .filter(|revision| revision.lifecycle == PrGuideLifecycle::Draft)
        .ok_or_else(|| {
            "The selected PR Guide draft is no longer available for review.".to_owned()
        })?;
    if has_decision(&existing, &draft.revision_id) {
        return Err("This PR Guide draft already has an immutable review decision.".to_owned());
    }
    let mut selected = std::collections::HashSet::new();
    if included_claim_ids
        .iter()
        .any(|claim_id| !selected.insert(claim_id.as_str()))
    {
        return Err("A PR Guide claim was selected more than once.".to_owned());
    }
    let mut accepted = draft.clone();
    let all_claims = accepted
        .sections
        .iter()
        .flat_map(|section| section.claims.iter())
        .map(|claim| claim.claim_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    if selected
        .iter()
        .any(|claim_id| !all_claims.contains(claim_id))
    {
        return Err("The accepted PR Guide selection contains an unknown claim.".to_owned());
    }
    for section in &mut accepted.sections {
        section
            .claims
            .retain(|claim| selected.contains(claim.claim_id.as_str()));
    }
    accepted
        .sections
        .retain(|section| !section.claims.is_empty());
    accepted.based_on_revision_id = Some(draft.revision_id.clone());
    accepted.revision_id = format!("pr-guide:{}", Uuid::new_v4());
    accepted.revision_number = next_revision_number(&existing)?;
    accepted.lifecycle = PrGuideLifecycle::Accepted;
    accepted.decision_note = None;
    accepted.created_at_unix_ms = unix_time_ms();
    validate_revision(&accepted)?;
    publish_revision(&directory, &accepted, &existing)
}

/// Appends a rejected review decision without rewriting the generated draft.
pub fn reject_draft(
    storage_dir: &Path,
    project_id: &str,
    comparison_id: &str,
    revision_id: &str,
    reason: &str,
) -> Result<PrGuideRevision, String> {
    let _guard = lock_store()?;
    if reason.trim().is_empty() || reason.len() > 4_000 {
        return Err("A rejection reason must contain 1–4,000 characters.".to_owned());
    }
    let directory = existing_comparison_directory(storage_dir, project_id, comparison_id)?
        .ok_or_else(|| "The selected PR Guide history is no longer available.".to_owned())?;
    let existing = read_all(&directory)?;
    let draft = existing
        .iter()
        .find(|revision| revision.revision_id == revision_id)
        .filter(|revision| revision.lifecycle == PrGuideLifecycle::Draft)
        .ok_or_else(|| {
            "The selected PR Guide draft is no longer available for review.".to_owned()
        })?;
    if has_decision(&existing, &draft.revision_id) {
        return Err("This PR Guide draft already has an immutable review decision.".to_owned());
    }
    let mut rejected = draft.clone();
    rejected.based_on_revision_id = Some(draft.revision_id.clone());
    rejected.revision_id = format!("pr-guide:{}", Uuid::new_v4());
    rejected.revision_number = next_revision_number(&existing)?;
    rejected.lifecycle = PrGuideLifecycle::Rejected;
    rejected.decision_note = Some(reason.to_owned());
    rejected.created_at_unix_ms = unix_time_ms();
    publish_revision(&directory, &rejected, &existing)
}

fn next_revision_number(existing: &[PrGuideRevision]) -> Result<u32, String> {
    existing
        .iter()
        .map(|revision| revision.revision_number)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or_else(|| "The PR Guide revision number is exhausted.".to_owned())
}

fn has_decision(existing: &[PrGuideRevision], draft_id: &str) -> bool {
    existing
        .iter()
        .any(|revision| revision.based_on_revision_id.as_deref() == Some(draft_id))
}

fn lock_store() -> Result<MutexGuard<'static, ()>, String> {
    PR_GUIDE_STORE_LOCK
        .lock()
        .map_err(|_| "The local PR Guide history is temporarily unavailable.".to_owned())
}

fn publish_revision(
    directory: &Path,
    revision: &PrGuideRevision,
    existing: &[PrGuideRevision],
) -> Result<PrGuideRevision, String> {
    validate_revision(revision)?;
    let bytes = serde_json::to_vec(&RevisionFile {
        schema_version: STORE_SCHEMA_VERSION,
        revision: revision.clone(),
    })
    .map_err(|_| "The PR Guide revision could not be serialized.".to_owned())?;
    if bytes.len() > MAX_REVISION_BYTES {
        return Err("The PR Guide revision exceeds its 2 MB storage limit.".to_owned());
    }
    let current_bytes = existing.iter().try_fold(0_usize, |total, item| {
        total
            .checked_add(serde_json::to_vec(item).map_or(usize::MAX, |value| value.len()))
            .ok_or_else(|| "The PR Guide history exceeded its local size limit.".to_owned())
    })?;
    if current_bytes.saturating_add(bytes.len()) > MAX_TOTAL_BYTES_PER_COMPARISON {
        return Err("The PR Guide history exceeds its 64 MB local storage limit.".to_owned());
    }
    let filename = revision_uuid(&revision.revision_id)?;
    let final_path = directory.join(format!("{filename}.json"));
    match fs::symlink_metadata(&final_path) {
        Ok(_) => return Err("This immutable PR Guide revision ID already exists.".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err("The PR Guide revision destination could not be inspected.".to_owned());
        }
    }
    let temporary = directory.join(format!(".draft-{}.tmp", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "The PR Guide draft could not be staged locally.".to_owned())?;
    let staged = StagedFile(temporary);
    file.write_all(&bytes)
        .map_err(|_| "The PR Guide draft could not be written locally.".to_owned())?;
    file.sync_all()
        .map_err(|_| "The PR Guide draft could not be synced locally.".to_owned())?;
    fs::hard_link(&staged.0, &final_path)
        .map_err(|_| "The immutable PR Guide revision could not be published.".to_owned())?;
    drop(file);
    drop(staged);
    Ok(revision.clone())
}

fn read_all(directory: &Path) -> Result<Vec<PrGuideRevision>, String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|_| "The local PR Guide history could not be listed.".to_owned())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "The local PR Guide history could not be read.".to_owned())?;
    entries.sort_by_key(|entry| entry.file_name());
    if entries.len() > MAX_REVISIONS_PER_COMPARISON + MAX_STAGING_FILES_PER_COMPARISON {
        return Err("The local PR Guide history contains too many entries.".to_owned());
    }
    let mut total_bytes = 0_usize;
    let mut staging_files = 0_usize;
    let mut revisions = Vec::new();
    for entry in entries {
        let filename = entry.file_name();
        let filename = filename.to_string_lossy();
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|_| "A local PR Guide history entry could not be inspected.".to_owned())?;
        if !metadata.file_type().is_file() || metadata.file_type().is_symlink() {
            return Err("The local PR Guide history contains a non-file entry.".to_owned());
        }
        let len = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        if is_staging_filename(&filename) {
            staging_files += 1;
            if staging_files > MAX_STAGING_FILES_PER_COMPARISON || len > MAX_REVISION_BYTES {
                return Err(
                    "The local PR Guide history contains too many or oversized staging files."
                        .to_owned(),
                );
            }
            continue;
        }
        if !filename.ends_with(".json") {
            return Err("The local PR Guide history contains an unexpected file.".to_owned());
        }
        if len > MAX_REVISION_BYTES
            || total_bytes.saturating_add(len) > MAX_TOTAL_BYTES_PER_COMPARISON
        {
            return Err("The local PR Guide history exceeds its bounded storage size.".to_owned());
        }
        total_bytes = total_bytes.saturating_add(len);
        let mut bytes = Vec::with_capacity(len);
        fs::File::open(entry.path())
            .and_then(|mut file| file.read_to_end(&mut bytes))
            .map_err(|_| "A local PR Guide revision could not be read.".to_owned())?;
        let record: RevisionFile = serde_json::from_slice(&bytes)
            .map_err(|_| "A local PR Guide revision has malformed JSON.".to_owned())?;
        if record.schema_version != STORE_SCHEMA_VERSION {
            return Err("A local PR Guide revision uses an unsupported schema version.".to_owned());
        }
        validate_revision(&record.revision)?;
        let expected_filename = format!("{}.json", revision_uuid(&record.revision.revision_id)?);
        if filename != expected_filename || record.revision.revision_number == 0 {
            return Err(
                "A local PR Guide revision filename or sequence number is invalid.".to_owned(),
            );
        }
        revisions.push(record.revision);
    }
    revisions.sort_by_key(|revision| revision.revision_number);
    if revisions.len() > MAX_REVISIONS_PER_COMPARISON
        || revisions.iter().enumerate().any(|(index, revision)| {
            revision.revision_number != u32::try_from(index + 1).unwrap_or(u32::MAX)
        })
    {
        return Err(
            "The local PR Guide history has a missing or duplicate revision number.".to_owned(),
        );
    }
    let mut decided_drafts = std::collections::HashSet::new();
    for revision in &revisions {
        if let Some(parent_id) = revision.based_on_revision_id.as_deref() {
            let parent = revisions
                .iter()
                .find(|candidate| candidate.revision_id == parent_id);
            let Some(parent) = parent.filter(|candidate| {
                candidate.lifecycle == PrGuideLifecycle::Draft
                    && candidate.project_id == revision.project_id
                    && candidate.comparison == revision.comparison
                    && candidate.title == revision.title
            }) else {
                return Err(
                    "The local PR Guide history contains an invalid decision link.".to_owned(),
                );
            };
            let preserves_reviewed_content = match &revision.lifecycle {
                PrGuideLifecycle::Accepted => revision.sections.iter().all(|section| {
                    parent
                        .sections
                        .iter()
                        .find(|candidate| {
                            candidate.section_id == section.section_id
                                && candidate.title == section.title
                        })
                        .is_some_and(|parent_section| {
                            section.claims.iter().all(|claim| {
                                parent_section
                                    .claims
                                    .iter()
                                    .any(|parent_claim| parent_claim == claim)
                            })
                        })
                }),
                PrGuideLifecycle::Rejected => revision.sections == parent.sections,
                PrGuideLifecycle::Draft => false,
            };
            if !preserves_reviewed_content || !decided_drafts.insert(parent_id) {
                return Err(
                    "The local PR Guide history contains an invalid decision link.".to_owned(),
                );
            }
        }
    }
    Ok(revisions)
}

fn is_staging_filename(filename: &str) -> bool {
    filename
        .strip_prefix(".draft-")
        .and_then(|value| value.strip_suffix(".tmp"))
        .is_some_and(|value| Uuid::parse_str(value).is_ok())
}

fn validate_revision(revision: &PrGuideRevision) -> Result<(), String> {
    let lifecycle_is_valid = match &revision.lifecycle {
        PrGuideLifecycle::Draft => {
            revision.based_on_revision_id.is_none() && revision.decision_note.is_none()
        }
        PrGuideLifecycle::Accepted => {
            revision
                .based_on_revision_id
                .as_deref()
                .is_some_and(|id| revision_uuid(id).is_ok())
                && revision.decision_note.is_none()
        }
        PrGuideLifecycle::Rejected => {
            revision
                .based_on_revision_id
                .as_deref()
                .is_some_and(|id| revision_uuid(id).is_ok())
                && revision
                    .decision_note
                    .as_deref()
                    .is_some_and(|note| !note.trim().is_empty() && note.len() <= 4_000)
        }
    };
    let comparison = &revision.comparison;
    let valid_object_id = |value: &str| {
        matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
    };
    let valid_anchor = |anchor: &crate::git_evidence::GitSourceAnchor, revision_id: &str| {
        anchor.revision_id == revision_id
            && valid_object_id(&anchor.revision_id)
            && valid_object_id(&anchor.blob_id)
            && !anchor.path.trim().is_empty()
            && anchor.path.len() <= 4_096
            && anchor.start_line > 0
            && anchor.end_line >= anchor.start_line
    };
    let patch_bytes = comparison
        .selected_changes
        .iter()
        .flat_map(|change| &change.hunks)
        .fold(0_usize, |total, hunk| {
            total.saturating_add(hunk.patch_excerpt.len())
        });
    let expected_comparison_id = stable_comparison_id(
        &comparison.base_ref,
        &comparison.base.object_id,
        &comparison.head_ref,
        &comparison.head.object_id,
    );
    if revision.schema_version != 1
        || revision.project_id.trim().is_empty()
        || revision.project_id.len() > 1_024
        || !valid_comparison_id(&comparison.comparison_id)
        || comparison.comparison_id != expected_comparison_id
        || comparison.base_ref.trim().is_empty()
        || comparison.head_ref.trim().is_empty()
        || comparison.base_ref.len() > 1_024
        || comparison.head_ref.len() > 1_024
        || !valid_object_id(&comparison.base.object_id)
        || !valid_object_id(&comparison.head.object_id)
        || comparison.base.requested != comparison.base_ref
        || comparison.head.requested != comparison.head_ref
        || comparison.selected_changes.is_empty()
        || comparison.selected_changes.len() > 12
        || patch_bytes > 250_000
        || revision.sections.is_empty()
        || revision.sections.len() > 12
        || revision.title.trim().len() < 8
        || revision.title.len() > 120
        || revision_uuid(&revision.revision_id).is_err()
        || !lifecycle_is_valid
    {
        return Err("The local PR Guide revision identity or structure is invalid.".to_owned());
    }
    if !valid_object_id(&comparison.base.object_id)
        || !valid_object_id(&comparison.head.object_id)
        || comparison.selected_changes.iter().any(|change| {
            change.hunks.is_empty()
                || change.hunks.len() > 128
                || !matches!(
                    change.classification,
                    EvidenceClassification::Text | EvidenceClassification::ParserUncertain
                )
                || change.skipped_reason.is_some()
                || (change.base_path.is_none() && change.head_path.is_none())
                || change
                    .base_path
                    .as_deref()
                    .is_some_and(|path| path.is_empty() || path.len() > 4_096)
                || change
                    .head_path
                    .as_deref()
                    .is_some_and(|path| path.is_empty() || path.len() > 4_096)
                || change.hunks.iter().any(|hunk| {
                    hunk.patch_excerpt.is_empty()
                        || hunk.patch_excerpt.len() > 8 * 1024
                        || hunk.base_anchor.as_ref().is_some_and(|anchor| {
                            !valid_anchor(anchor, &comparison.base.object_id)
                                || change.base_path.as_deref() != Some(anchor.path.as_str())
                                || change.base_blob_id.as_deref() != Some(anchor.blob_id.as_str())
                        })
                        || hunk.head_anchor.as_ref().is_some_and(|anchor| {
                            !valid_anchor(anchor, &comparison.head.object_id)
                                || change.head_path.as_deref() != Some(anchor.path.as_str())
                                || change.head_blob_id.as_deref() != Some(anchor.blob_id.as_str())
                        })
                })
        })
    {
        return Err("The local PR Guide comparison evidence is outside its bounds.".to_owned());
    }
    let mut claim_ids = std::collections::HashSet::new();
    let mut change_ids = std::collections::HashSet::new();
    if comparison.selected_changes.iter().any(|change| {
        !change_ids.insert((change.base_path.as_deref(), change.head_path.as_deref()))
    }) {
        return Err("The local PR Guide comparison repeats a selected file.".to_owned());
    }
    let claim_count: usize = revision
        .sections
        .iter()
        .map(|section| section.claims.len())
        .sum();
    if claim_count == 0 || claim_count > 240 {
        return Err("The local PR Guide contains an unsupported number of claims.".to_owned());
    }
    for section in &revision.sections {
        if section.section_id.trim().is_empty()
            || section.section_id.len() > 120
            || section.title.trim().is_empty()
            || section.title.len() > 120
            || section.claims.is_empty()
            || section.claims.len() > 20
        {
            return Err("The local PR Guide contains an empty section.".to_owned());
        }
        for claim in &section.claims {
            if claim.prose.trim().len() < 24
                || claim.prose.len() > 2_000
                || claim.claim_id.trim().is_empty()
                || claim.claim_id.len() > 120
                || !claim_ids.insert(claim.claim_id.as_str())
                || claim.citations.is_empty()
                || claim.citations.len() > 8
                || (claim.category == PrGuideClaimCategory::Inference
                    && claim
                        .inference_basis
                        .as_deref()
                        .is_none_or(|basis| basis.trim().len() < 12 || basis.len() > 500))
                || (claim.category != PrGuideClaimCategory::Inference
                    && claim.inference_basis.is_some())
            {
                return Err("The local PR Guide contains an invalid or uncited claim.".to_owned());
            }
            for citation in &claim.citations {
                let change = revision
                    .comparison
                    .selected_changes
                    .get(citation.change_index)
                    .ok_or_else(|| {
                        "A PR Guide citation points outside its selected files.".to_owned()
                    })?;
                let hunk = change.hunks.get(citation.hunk_index).ok_or_else(|| {
                    "A PR Guide citation points outside its selected hunks.".to_owned()
                })?;
                let expected_anchor = match citation.side {
                    EvidenceSide::Base => hunk.base_anchor.as_ref(),
                    EvidenceSide::Head => hunk.head_anchor.as_ref(),
                };
                if expected_anchor != Some(&citation.anchor)
                    || citation.quote.trim().len() < 8
                    || citation.quote.len() > 1_000
                    || !crate::pr_guide::quote_matches_side(
                        &hunk.patch_excerpt,
                        &citation.quote,
                        citation.side,
                    )
                {
                    return Err(
                        "A PR Guide citation no longer matches its immutable Git hunk.".to_owned(),
                    );
                }
            }
        }
    }
    Ok(())
}

fn comparison_directory(
    storage_dir: &Path,
    project_id: &str,
    comparison_id: &str,
) -> Result<PathBuf, String> {
    if project_id.trim().is_empty() || !valid_comparison_id(comparison_id) {
        return Err("The PR Guide project or comparison identity is invalid.".to_owned());
    }
    let root = storage_dir.join("pr-guides");
    let project_hash = hex_digest(Sha256::digest(project_id.as_bytes()).as_slice());
    let comparison_hash = comparison_id
        .strip_prefix("cmp:")
        .ok_or_else(|| "The PR Guide comparison identity is invalid.".to_owned())?;
    Ok(root.join(project_hash).join(comparison_hash.to_owned()))
}

fn existing_comparison_directory(
    storage_dir: &Path,
    project_id: &str,
    comparison_id: &str,
) -> Result<Option<PathBuf>, String> {
    ensure_directory(storage_dir)?;
    let path = comparison_directory(storage_dir, project_id, comparison_id)?;
    let guides = path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "The local PR Guide storage path is invalid.".to_owned())?;
    let project = path
        .parent()
        .ok_or_else(|| "The local PR Guide storage path is invalid.".to_owned())?;
    for directory in [guides, project, path.as_path()] {
        match fs::symlink_metadata(directory) {
            Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            }
            Ok(_) => {
                return Err("The local PR Guide storage path is not a real directory.".to_owned());
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => {
                return Err("The local PR Guide storage path could not be inspected.".to_owned());
            }
        }
    }
    Ok(Some(path))
}

fn create_comparison_directory(
    storage_dir: &Path,
    project_id: &str,
    comparison_id: &str,
) -> Result<PathBuf, String> {
    ensure_directory(storage_dir)?;
    let path = comparison_directory(storage_dir, project_id, comparison_id)?;
    let guides = path
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "The local PR Guide storage path is invalid.".to_owned())?;
    let project = path
        .parent()
        .ok_or_else(|| "The local PR Guide storage path is invalid.".to_owned())?;
    ensure_directory_path(guides)?;
    ensure_directory_path(project)?;
    ensure_directory_path(&path)?;
    Ok(path)
}

fn ensure_directory(root: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|_| "The local application storage directory is unavailable.".to_owned())?;
    if !metadata.file_type().is_dir() || metadata.file_type().is_symlink() {
        return Err("The local application storage directory is not a real directory.".to_owned());
    }
    Ok(())
}

fn ensure_directory_path(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            Ok(())
        }
        Ok(_) => Err("The local PR Guide storage path is not a real directory.".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::create_dir(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => {
                    return Err(
                        "The local PR Guide storage directory could not be created.".to_owned()
                    );
                }
            }
            ensure_directory_path(path)
        }
        Err(_) => Err("The local PR Guide storage directory could not be inspected.".to_owned()),
    }
}

fn valid_comparison_id(value: &str) -> bool {
    value
        .strip_prefix("cmp:")
        .is_some_and(|hash| hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn stable_comparison_id(base_ref: &str, base_id: &str, head_ref: &str, head_id: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"code-atlas-local-git-comparison-v1\0");
    digest.update(base_ref.as_bytes());
    digest.update([0]);
    digest.update(base_id.as_bytes());
    digest.update([0]);
    digest.update(head_ref.as_bytes());
    digest.update([0]);
    digest.update(head_id.as_bytes());
    format!("cmp:{}", hex_digest(digest.finalize().as_slice()))
}

fn revision_uuid(value: &str) -> Result<Uuid, String> {
    value
        .strip_prefix("pr-guide:")
        .ok_or_else(|| "The PR Guide revision ID is invalid.".to_owned())
        .and_then(|value| {
            Uuid::parse_str(value).map_err(|_| "The PR Guide revision ID is invalid.".to_owned())
        })
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(u64::MAX as u128) as u64
        })
}

struct StagedFile(PathBuf);

impl Drop for StagedFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git_evidence::{
        EvidenceClassification, GitChangeKind, GitDiffHunk, GitFileChange, GitSourceAnchor,
        ResolvedGitRef,
    };
    use crate::pr_guide::{
        ComparisonContext, EvidenceSide, PrGuideCitation, PrGuideClaim, PrGuideClaimAspect,
        PrGuideSection,
    };
    struct Temp(PathBuf);

    impl Temp {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("code-atlas-pr-guide-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for Temp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn draft() -> PrGuideRevision {
        let base = "a".repeat(40);
        let head = "b".repeat(40);
        let comparison_id = stable_comparison_id("main", &base, "feature", &head);
        let anchor = GitSourceAnchor {
            revision_id: head.clone(),
            path: "src/lib.rs".to_owned(),
            blob_id: "d".repeat(40),
            start_line: 1,
            end_line: 2,
        };
        let change = GitFileChange {
            kind: GitChangeKind::Modified,
            base_path: Some("src/lib.rs".to_owned()),
            head_path: Some("src/lib.rs".to_owned()),
            classification: EvidenceClassification::Text,
            base_blob_id: Some("c".repeat(40)),
            head_blob_id: Some("d".repeat(40)),
            base_size_bytes: Some(20),
            head_size_bytes: Some(21),
            hunks: vec![GitDiffHunk {
                base_anchor: Some(GitSourceAnchor {
                    revision_id: base.clone(),
                    path: "src/lib.rs".to_owned(),
                    blob_id: "c".repeat(40),
                    start_line: 1,
                    end_line: 2,
                }),
                head_anchor: Some(anchor.clone()),
                base_changed_ranges: vec![],
                head_changed_ranges: vec![],
                base_symbols: vec![],
                head_symbols: vec![],
                patch_excerpt: "-    old_value()\n+    new_value()\n".to_owned(),
            }],
            skipped_reason: None,
        };
        PrGuideRevision {
            schema_version: 1,
            revision_id: format!("pr-guide:{}", Uuid::new_v4()),
            based_on_revision_id: None,
            revision_number: 0,
            project_id: "project:test".to_owned(),
            comparison: ComparisonContext {
                base_ref: "main".to_owned(),
                head_ref: "feature".to_owned(),
                comparison_id,
                base: ResolvedGitRef {
                    requested: "main".to_owned(),
                    object_id: base,
                },
                head: ResolvedGitRef {
                    requested: "feature".to_owned(),
                    object_id: head,
                },
                selected_changes: vec![change],
            },
            lifecycle: PrGuideLifecycle::Draft,
            decision_note: None,
            created_at_unix_ms: 1,
            title: "Updated call behavior".to_owned(),
            sections: vec![PrGuideSection {
                section_id: "section:1".to_owned(),
                title: "Behavior".to_owned(),
                claims: vec![
                    PrGuideClaim {
                        claim_id: "claim:1".to_owned(),
                        category: PrGuideClaimCategory::CodeBehavior,
                        aspect: PrGuideClaimAspect::CodeChange,
                        prose: "The handler now calls new_value instead of old_value.".to_owned(),
                        inference_basis: None,
                        citations: vec![PrGuideCitation {
                            change_index: 0,
                            hunk_index: 0,
                            side: EvidenceSide::Head,
                            anchor,
                            quote: "+    new_value()".to_owned(),
                        }],
                    },
                    PrGuideClaim {
                        claim_id: "claim:2".to_owned(),
                        category: PrGuideClaimCategory::Inference,
                        aspect: PrGuideClaimAspect::Risk,
                        prose: "This may change the persisted result for existing users."
                            .to_owned(),
                        inference_basis: Some(
                            "The selected hunk changes the call but shows no migration.".to_owned(),
                        ),
                        citations: vec![PrGuideCitation {
                            change_index: 0,
                            hunk_index: 0,
                            side: EvidenceSide::Head,
                            anchor: GitSourceAnchor {
                                revision_id: "b".repeat(40),
                                path: "src/lib.rs".to_owned(),
                                blob_id: "d".repeat(40),
                                start_line: 1,
                                end_line: 2,
                            },
                            quote: "+    new_value()".to_owned(),
                        }],
                    },
                ],
            }],
        }
    }

    #[test]
    fn guide_history_is_separate_and_acceptance_keeps_only_reviewed_claims() {
        let temp = Temp::new();
        let draft = save_draft(&temp.0, draft()).unwrap();
        let accepted = accept_draft(
            &temp.0,
            &draft.project_id,
            &draft.comparison.comparison_id,
            &draft.revision_id,
            &["claim:1".to_owned()],
        )
        .unwrap();
        assert_eq!(accepted.lifecycle, PrGuideLifecycle::Accepted);
        assert_eq!(
            accepted.based_on_revision_id.as_deref(),
            Some(draft.revision_id.as_str())
        );
        assert_eq!(
            accepted
                .sections
                .iter()
                .map(|section| section.claims.len())
                .sum::<usize>(),
            1
        );
        assert_eq!(
            list_for_comparison(&temp.0, &draft.project_id, &draft.comparison.comparison_id)
                .unwrap()
                .len(),
            2
        );
        assert!(!temp.0.join("chapters").exists());
    }

    #[test]
    fn guide_history_rejects_unknown_claims_and_keeps_rejected_draft_immutable() {
        let temp = Temp::new();
        let draft = save_draft(&temp.0, draft()).unwrap();
        assert!(
            accept_draft(
                &temp.0,
                &draft.project_id,
                &draft.comparison.comparison_id,
                &draft.revision_id,
                &["not-a-claim".to_owned()]
            )
            .is_err()
        );
        let rejected = reject_draft(
            &temp.0,
            &draft.project_id,
            &draft.comparison.comparison_id,
            &draft.revision_id,
            "Inference lacks enough evidence.",
        )
        .unwrap();
        assert!(
            accept_draft(
                &temp.0,
                &draft.project_id,
                &draft.comparison.comparison_id,
                &draft.revision_id,
                &["claim:1".to_owned()],
            )
            .is_err()
        );
        assert!(
            reject_draft(
                &temp.0,
                &draft.project_id,
                &draft.comparison.comparison_id,
                &draft.revision_id,
                "A second decision must not replace the first.",
            )
            .is_err()
        );
        let history =
            list_for_comparison(&temp.0, &draft.project_id, &draft.comparison.comparison_id)
                .unwrap();
        assert_eq!(history[0].lifecycle, PrGuideLifecycle::Draft);
        assert_eq!(history[1].lifecycle, PrGuideLifecycle::Rejected);
        assert_eq!(
            rejected.decision_note.as_deref(),
            Some("Inference lacks enough evidence.")
        );
    }

    #[test]
    fn history_ignores_only_well_formed_crash_leftover_staging_files() {
        let temp = Temp::new();
        let draft = save_draft(&temp.0, draft()).unwrap();
        let directory =
            comparison_directory(&temp.0, &draft.project_id, &draft.comparison.comparison_id)
                .unwrap();
        fs::write(
            directory.join(format!(".draft-{}.tmp", Uuid::new_v4())),
            b"interrupted write",
        )
        .unwrap();
        let history =
            list_for_comparison(&temp.0, &draft.project_id, &draft.comparison.comparison_id)
                .unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].revision_id, draft.revision_id);
    }

    #[cfg(unix)]
    #[test]
    fn read_and_decision_paths_reject_a_symlinked_history_root() {
        use std::os::unix::fs::symlink;

        let storage = Temp::new();
        let outside = Temp::new();
        let draft = save_draft(&outside.0, draft()).unwrap();
        symlink(outside.0.join("pr-guides"), storage.0.join("pr-guides")).unwrap();

        assert!(
            list_for_comparison(
                &storage.0,
                &draft.project_id,
                &draft.comparison.comparison_id,
            )
            .is_err()
        );
        assert!(
            accept_draft(
                &storage.0,
                &draft.project_id,
                &draft.comparison.comparison_id,
                &draft.revision_id,
                &["claim:1".to_owned()],
            )
            .is_err()
        );
        assert!(
            reject_draft(
                &storage.0,
                &draft.project_id,
                &draft.comparison.comparison_id,
                &draft.revision_id,
                "This must not write through a symlink.",
            )
            .is_err()
        );
        assert_eq!(
            list_for_comparison(
                &outside.0,
                &draft.project_id,
                &draft.comparison.comparison_id,
            )
            .unwrap()
            .len(),
            1
        );
    }
}
