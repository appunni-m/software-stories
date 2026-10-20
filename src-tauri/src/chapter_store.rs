use crate::chapters::{self, ChapterRevision, RevisionLifecycle, ValidationStatus};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_REVISION_COUNT: usize = 10_000;
const MAX_REVISION_JSON_BYTES: usize = 4 * 1024 * 1024;
const MAX_TOTAL_JSON_BYTES: usize = 128 * 1024 * 1024;
// Count unrelated entries too so a damaged folder cannot cause an unbounded scan.
const MAX_STORAGE_ENTRY_COUNT: usize = MAX_REVISION_COUNT * 4;

struct ProjectRevisions {
    revisions: Vec<ChapterRevision>,
    total_bytes: usize,
    entries_visited: usize,
}

pub fn list_for_snapshot(
    storage_dir: &Path,
    project_id: &str,
    snapshot_id: &str,
) -> Result<Vec<ChapterRevision>, String> {
    if !valid_id(snapshot_id) {
        return Err("The chapter snapshot ID is invalid.".to_owned());
    }
    Ok(list_for_project(storage_dir, project_id)?
        .into_iter()
        .filter(|revision| revision.snapshot_id == snapshot_id)
        .collect())
}

pub fn list_for_project(
    storage_dir: &Path,
    project_id: &str,
) -> Result<Vec<ChapterRevision>, String> {
    Ok(load_project_revisions(storage_dir, project_id)?.revisions)
}

/// The caller serializes saves so assigning the next revision number and publishing
/// the immutable draft form one operation. Evidence validation happens before this call.
pub fn save_new_draft(
    storage_dir: &Path,
    mut revision: ChapterRevision,
) -> Result<ChapterRevision, String> {
    if revision.lifecycle != RevisionLifecycle::Draft
        || revision.validation_status != ValidationStatus::Valid
    {
        return Err(
            "Only a locally validated, unaccepted draft can be saved by generation.".to_owned(),
        );
    }
    validate_revision_identity(&revision)?;
    append_revision(storage_dir, &mut revision, false)
}

/// Append an immutable review revision. The caller must validate its source evidence
/// while holding the shared write lock before using this entry point.
pub fn append_review_revision(
    storage_dir: &Path,
    mut revision: ChapterRevision,
) -> Result<ChapterRevision, String> {
    if revision.schema_version != chapters::CHAPTER_SCHEMA_VERSION
        || revision.based_on_revision_id.is_none()
    {
        return Err("A review revision needs the current schema and a parent revision.".to_owned());
    }
    match revision.lifecycle {
        RevisionLifecycle::Draft => {}
        RevisionLifecycle::Accepted if revision.validation_status == ValidationStatus::Valid => {}
        RevisionLifecycle::Rejected
            if revision
                .decision_note
                .as_deref()
                .is_some_and(|note| !note.trim().is_empty() && note.len() <= 4_000) => {}
        _ => {
            return Err(
                "The review revision has an invalid lifecycle or decision reason.".to_owned(),
            );
        }
    }
    append_revision(storage_dir, &mut revision, true)
}

fn append_revision(
    storage_dir: &Path,
    revision: &mut ChapterRevision,
    review: bool,
) -> Result<ChapterRevision, String> {
    if revision.schema_version != chapters::CHAPTER_SCHEMA_VERSION {
        return Err("The chapter revision uses an unsupported schema version.".to_owned());
    }
    if !review && revision.based_on_revision_id.is_some() {
        return Err("Generated chapter revisions cannot claim a review parent.".to_owned());
    }
    let existing = load_project_revisions(storage_dir, &revision.project_id)?;
    if existing.revisions.len() >= MAX_REVISION_COUNT {
        return Err("This project already reached the local revision limit.".to_owned());
    }
    if existing
        .revisions
        .iter()
        .any(|item| item.revision_id == revision.revision_id)
    {
        return Err(
            "This immutable revision ID already exists. Generate a fresh draft.".to_owned(),
        );
    }
    let mut last_number = 0_u32;
    let mut target_exists = false;
    let mut latest_for_snapshot: Option<&ChapterRevision> = None;
    for item in &existing.revisions {
        if item.target.target_id == revision.target.target_id {
            target_exists = true;
            if item.chapter_id != revision.chapter_id || item.target.scope != revision.target.scope
            {
                return Err(
                    "The chapter target identity conflicts with an existing local chapter."
                        .to_owned(),
                );
            }
            last_number = last_number.max(item.revision_number);
            if item.snapshot_id == revision.snapshot_id
                && latest_for_snapshot
                    .is_none_or(|latest| item.revision_number > latest.revision_number)
            {
                latest_for_snapshot = Some(item);
            }
        } else if item.chapter_id == revision.chapter_id {
            return Err(
                "The chapter identity is already assigned to a different local target.".to_owned(),
            );
        }
    }
    if review {
        let parent_id = revision
            .based_on_revision_id
            .as_deref()
            .ok_or_else(|| "The review revision is missing its parent.".to_owned())?;
        let parent = existing
            .revisions
            .iter()
            .find(|item| item.revision_id == parent_id)
            .ok_or_else(|| "The review parent is no longer available.".to_owned())?;
        if parent.chapter_id != revision.chapter_id
            || parent.target.target_id != revision.target.target_id
            || parent.target.scope != revision.target.scope
        {
            return Err("This chapter changed since it was opened. Reload the latest revision before saving.".to_owned());
        }
        if parent.snapshot_id == revision.snapshot_id {
            if latest_for_snapshot.map(|latest| latest.revision_id.as_str()) != Some(parent_id) {
                return Err("This chapter changed since it was opened. Reload the latest revision before saving.".to_owned());
            }
        } else {
            // Moving an accepted chapter into a newer captured snapshot is an
            // explicit review branch. It may only create the first draft for
            // that target/snapshot and must start from the latest accepted
            // revision of its source snapshot. Acceptance later follows the
            // ordinary same-snapshot append-only path from this draft.
            let latest_parent_snapshot = existing
                .revisions
                .iter()
                .filter(|item| {
                    item.target.target_id == parent.target.target_id
                        && item.snapshot_id == parent.snapshot_id
                })
                .max_by_key(|item| item.revision_number);
            if revision.lifecycle != RevisionLifecycle::Draft
                || parent.lifecycle != RevisionLifecycle::Accepted
                || latest_parent_snapshot.map(|latest| latest.revision_id.as_str())
                    != Some(parent_id)
                || latest_for_snapshot.is_some()
            {
                return Err("A cross-snapshot update must start from the latest accepted revision and create the first draft for the selected snapshot.".to_owned());
            }
        }
        if matches!(
            revision.lifecycle,
            RevisionLifecycle::Accepted | RevisionLifecycle::Rejected
        ) && parent.lifecycle != RevisionLifecycle::Draft
        {
            return Err("Only a saved draft can be accepted or rejected.".to_owned());
        }
    }
    revision.revision_number = last_number
        .checked_add(1)
        .ok_or_else(|| "The chapter revision number is exhausted.".to_owned())?;
    validate_revision_identity(revision)?;
    let bytes = chapters::encode_chapter_revision(revision)?;
    if existing.total_bytes.saturating_add(bytes.len()) > MAX_TOTAL_JSON_BYTES {
        return Err("Local chapter revisions exceed the supported storage size.".to_owned());
    }
    // Reserve a revision, its staging file, and a directory for a new target.
    if existing
        .entries_visited
        .saturating_add(if target_exists { 2 } else { 3 })
        > MAX_STORAGE_ENTRY_COUNT
    {
        return Err("The local chapter folder contains too many entries.".to_owned());
    }

    create_storage_directories(
        storage_dir,
        &revision.project_id,
        &revision.target.target_id,
    )?;
    let target_dir = target_directory(
        storage_dir,
        &revision.project_id,
        &revision.target.target_id,
    );
    let final_path = target_dir.join(format!("{}.json", revision.revision_id));
    match fs::symlink_metadata(&final_path) {
        Ok(_) => {
            return Err(
                "This immutable revision ID already exists. Generate a fresh draft.".to_owned(),
            );
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("The immutable draft destination could not be inspected.".to_owned()),
    }

    let temporary = target_dir.join(format!(".draft-{}.tmp", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| "The local draft could not be staged.".to_owned())?;
    let staged = StagedDraft(temporary);
    file.write_all(&bytes)
        .map_err(|_| "The local draft could not be written.".to_owned())?;
    file.sync_all()
        .map_err(|_| "The local draft could not be synced.".to_owned())?;
    // Publishing a hard link never replaces an existing file or dangling symbolic link.
    fs::hard_link(&staged.0, &final_path).map_err(|_| {
        "The immutable draft could not be added without replacing another revision.".to_owned()
    })?;
    drop(file);
    drop(staged);
    Ok(revision.clone())
}

struct StagedDraft(PathBuf);

impl Drop for StagedDraft {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn load_project_revisions(
    storage_dir: &Path,
    project_id: &str,
) -> Result<ProjectRevisions, String> {
    if project_id.trim().is_empty() {
        return Err("The chapter project identity is empty.".to_owned());
    }
    let mut result = ProjectRevisions {
        revisions: Vec::new(),
        total_bytes: 0,
        entries_visited: 0,
    };
    for directory in storage_directories(storage_dir, project_id) {
        if !inspect_directory(&directory)? {
            return Ok(result);
        }
    }
    let entries = fs::read_dir(project_targets_directory(storage_dir, project_id))
        .map_err(|_| "The local chapter revision folder could not be read.".to_owned())?;
    for entry in entries {
        visit_entry(&mut result)?;
        let entry =
            entry.map_err(|_| "A local chapter target entry could not be read.".to_owned())?;
        let file_type = entry
            .file_type()
            .map_err(|_| "A local chapter target type could not be read.".to_owned())?;
        if file_type.is_symlink() {
            return Err("A local chapter target folder cannot be a symbolic link.".to_owned());
        }
        if !file_type.is_dir() {
            continue;
        }
        let target_id = entry
            .file_name()
            .into_string()
            .map_err(|_| "A local chapter target has an invalid storage identity.".to_owned())?;
        if !valid_id(&target_id) {
            return Err("A local chapter target has an invalid storage identity.".to_owned());
        }
        let target_path = entry.path();
        if !inspect_directory(&target_path)? {
            return Err("A local chapter target folder disappeared while being read.".to_owned());
        }
        for file in fs::read_dir(&target_path)
            .map_err(|_| "A local chapter target folder could not be read.".to_owned())?
        {
            visit_entry(&mut result)?;
            let file =
                file.map_err(|_| "A local chapter revision entry could not be read.".to_owned())?;
            let path = file.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }
            if result.revisions.len() >= MAX_REVISION_COUNT {
                return Err(
                    "The local chapter revision count exceeds the supported limit.".to_owned(),
                );
            }
            let file_type = file
                .file_type()
                .map_err(|_| "A local chapter revision type could not be read.".to_owned())?;
            if !file_type.is_file() {
                return Err("A local chapter revision must be a regular file, not a symbolic link or folder.".to_owned());
            }
            let bytes = read_revision_bytes(&path, MAX_TOTAL_JSON_BYTES - result.total_bytes)?;
            result.total_bytes += bytes.len();
            let revision = chapters::decode_chapter_revision(&bytes)?;
            validate_revision_identity(&revision)?;
            if revision.project_id != project_id
                || revision.target.target_id != target_id
                || Some(revision.revision_id.as_str())
                    != path.file_stem().and_then(|stem| stem.to_str())
            {
                return Err("A local chapter revision's project or identity does not match its storage path.".to_owned());
            }
            result.revisions.push(revision);
        }
    }
    result.revisions.sort_by(|left, right| {
        left.target
            .target_id
            .cmp(&right.target.target_id)
            .then_with(|| left.revision_number.cmp(&right.revision_number))
            .then_with(|| left.snapshot_id.cmp(&right.snapshot_id))
            .then_with(|| left.created_at_unix_ms.cmp(&right.created_at_unix_ms))
            .then_with(|| left.revision_id.cmp(&right.revision_id))
    });
    validate_project_revision_order(&result.revisions)?;
    Ok(result)
}

fn read_revision_bytes(path: &Path, remaining_bytes: usize) -> Result<Vec<u8>, String> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path).map_err(|_| {
        "A local chapter revision could not be read without following a symbolic link.".to_owned()
    })?;
    let metadata = file
        .metadata()
        .map_err(|_| "A local chapter revision could not be inspected.".to_owned())?;
    if !metadata.is_file() {
        return Err("A local chapter revision must be a regular file.".to_owned());
    }
    let limit = MAX_REVISION_JSON_BYTES.min(remaining_bytes);
    if metadata.len() > limit as u64 {
        return Err("Local chapter revisions exceed the supported storage size.".to_owned());
    }
    // Bound the read too in case a file grows after its size was inspected.
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "A local chapter revision could not be read.".to_owned())?;
    if bytes.len() > limit {
        return Err("Local chapter revisions exceed the supported storage size.".to_owned());
    }
    Ok(bytes)
}

fn validate_revision_identity(revision: &ChapterRevision) -> Result<(), String> {
    if revision.schema_version != chapters::CHAPTER_SCHEMA_VERSION {
        return Err("The chapter revision uses an unsupported schema version.".to_owned());
    }
    if revision.project_id.trim().is_empty()
        || !valid_id(&revision.snapshot_id)
        || !valid_id(&revision.target.target_id)
        || !valid_id(&revision.chapter_id)
        || !valid_id(&revision.revision_id)
        || revision.chapter_id == revision.revision_id
        || revision.chapter_id == revision.target.target_id
        || revision.revision_id == revision.target.target_id
    {
        return Err("The chapter revision uses an invalid local identity.".to_owned());
    }
    if revision.revision_number == 0
        || revision.created_at_unix_ms == 0
        || revision.title.trim().is_empty()
        || (revision.lifecycle == RevisionLifecycle::Accepted
            && revision.validation_status != ValidationStatus::Valid)
    {
        return Err("The local chapter revision has invalid revision metadata.".to_owned());
    }
    if matches!(
        &revision.target.scope,
        crate::chapters::ChapterTargetScope::Overview
    ) && (revision.target.target_id != "target:overview"
        || revision.chapter_id != "chapter:overview")
    {
        return Err("An overview chapter must use its stable overview identities.".to_owned());
    }
    crate::chapter_origin::validate_revision_origin(revision)?;
    chapters::validate_overview_content(revision)?;
    if revision.lifecycle != RevisionLifecycle::Draft && revision.based_on_revision_id.is_none() {
        return Err("A reviewed revision must identify the revision it follows.".to_owned());
    }
    if revision
        .based_on_revision_id
        .as_deref()
        .is_some_and(|parent| !valid_id(parent) || parent == revision.revision_id)
    {
        return Err("The chapter revision has an invalid parent identity.".to_owned());
    }
    if revision.lifecycle == RevisionLifecycle::Rejected
        && revision
            .decision_note
            .as_deref()
            .is_none_or(|note| note.trim().is_empty() || note.len() > 4_000)
    {
        return Err("A rejected revision needs a reason of at most 4,000 bytes.".to_owned());
    }
    if revision.lifecycle != RevisionLifecycle::Rejected && revision.decision_note.is_some() {
        return Err("Only a rejected revision can carry a decision reason.".to_owned());
    }
    Ok(())
}

fn validate_project_revision_order(revisions: &[ChapterRevision]) -> Result<(), String> {
    let mut revision_ids = HashSet::new();
    let mut chapter_targets = HashMap::new();
    let mut previous: Option<&ChapterRevision> = None;
    for revision in revisions {
        if !revision_ids.insert(revision.revision_id.as_str()) {
            return Err("The local chapter store repeats an immutable revision ID.".to_owned());
        }
        if let Some(target_id) = chapter_targets.insert(
            revision.chapter_id.as_str(),
            revision.target.target_id.as_str(),
        ) {
            if target_id != revision.target.target_id {
                return Err("A local chapter identity is assigned to multiple targets.".to_owned());
            }
        }
        if let Some(previous) =
            previous.filter(|item| item.target.target_id == revision.target.target_id)
        {
            if previous.chapter_id != revision.chapter_id
                || previous.target.scope != revision.target.scope
            {
                return Err(
                    "A local chapter target has inconsistent chapter identity or scope.".to_owned(),
                );
            }
            if previous.revision_number == revision.revision_number {
                return Err("A local chapter target repeats a revision number.".to_owned());
            }
        }
        previous = Some(revision);
    }
    for revision in revisions {
        if let Some(parent_id) = revision.based_on_revision_id.as_deref() {
            let Some(parent) = revisions.iter().find(|item| item.revision_id == parent_id) else {
                return Err(
                    "A local chapter review revision refers to a missing parent.".to_owned(),
                );
            };
            if parent.chapter_id != revision.chapter_id
                || parent.target.target_id != revision.target.target_id
                || parent.target.scope != revision.target.scope
                || parent.revision_number >= revision.revision_number
            {
                return Err(
                    "A local chapter review revision has inconsistent parent history.".to_owned(),
                );
            }
            if parent.snapshot_id == revision.snapshot_id {
                let latest_prior = revisions
                    .iter()
                    .filter(|item| {
                        item.target.target_id == revision.target.target_id
                            && item.snapshot_id == revision.snapshot_id
                            && item.revision_number < revision.revision_number
                    })
                    .max_by_key(|item| item.revision_number);
                if latest_prior.map(|item| item.revision_id.as_str()) != Some(parent_id) {
                    return Err(
                        "A local chapter review revision does not follow the latest prior revision for its snapshot."
                            .to_owned(),
                    );
                }
            } else {
                // A selective update starts a new snapshot history with a draft
                // based on the latest accepted revision from the old snapshot.
                // Later acceptance/rejection is then an ordinary same-snapshot
                // child of that saved draft.
                let latest_parent_snapshot = revisions
                    .iter()
                    .filter(|item| {
                        item.target.target_id == parent.target.target_id
                            && item.snapshot_id == parent.snapshot_id
                    })
                    .max_by_key(|item| item.revision_number);
                let has_prior_destination_revision = revisions.iter().any(|item| {
                    item.target.target_id == revision.target.target_id
                        && item.snapshot_id == revision.snapshot_id
                        && item.revision_number < revision.revision_number
                });
                if revision.lifecycle != RevisionLifecycle::Draft
                    || parent.lifecycle != RevisionLifecycle::Accepted
                    || latest_parent_snapshot.map(|item| item.revision_id.as_str())
                        != Some(parent_id)
                    || has_prior_destination_revision
                {
                    return Err(
                        "A cross-snapshot update must be the first draft in its snapshot and follow the latest accepted source revision."
                            .to_owned(),
                    );
                }
            }
            if matches!(
                revision.lifecycle,
                RevisionLifecycle::Accepted | RevisionLifecycle::Rejected
            ) && parent.lifecycle != RevisionLifecycle::Draft
            {
                return Err(
                    "A local chapter acceptance or rejection must follow a saved draft.".to_owned(),
                );
            }
        }
    }
    Ok(())
}

fn visit_entry(result: &mut ProjectRevisions) -> Result<(), String> {
    if result.entries_visited >= MAX_STORAGE_ENTRY_COUNT {
        return Err("The local chapter folder contains too many entries.".to_owned());
    }
    result.entries_visited += 1;
    Ok(())
}

fn inspect_directory(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(true),
        Ok(_) => Err(
            "A local chapter storage folder must be a directory, not a symbolic link.".to_owned(),
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("A local chapter storage folder could not be inspected.".to_owned()),
    }
}

fn create_storage_directories(
    storage_dir: &Path,
    project_id: &str,
    target_id: &str,
) -> Result<(), String> {
    if !inspect_directory(storage_dir)? {
        fs::create_dir_all(storage_dir)
            .map_err(|_| "The local chapter storage folder could not be created.".to_owned())?;
        inspect_directory(storage_dir)?;
    }
    let mut directories = storage_directories(storage_dir, project_id);
    directories.push(target_directory(storage_dir, project_id, target_id));
    // Create children individually so create_dir_all cannot traverse links in the chapter tree.
    for directory in directories.into_iter().skip(1) {
        match fs::create_dir(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err("The local chapter folder could not be created.".to_owned()),
        }
        if !inspect_directory(&directory)? {
            return Err(
                "The local chapter folder disappeared before the draft could be saved.".to_owned(),
            );
        }
    }
    Ok(())
}

fn storage_directories(storage_dir: &Path, project_id: &str) -> Vec<PathBuf> {
    let chapters = current_chapter_store_directory(storage_dir);
    let project = chapters.join(digest(project_id.as_bytes()));
    vec![
        storage_dir.to_owned(),
        chapters,
        project.clone(),
        project.join("targets"),
    ]
}

fn project_targets_directory(storage_dir: &Path, project_id: &str) -> PathBuf {
    current_chapter_store_directory(storage_dir)
        .join(digest(project_id.as_bytes()))
        .join("targets")
}

/// Each strict chapter schema owns a fresh store. Unsupported records from an
/// earlier product schema remain untouched in `chapters`; current operations
/// never decode them, migrate them, or let them block new local work.
fn current_chapter_store_directory(storage_dir: &Path) -> PathBuf {
    storage_dir.join(format!("chapters-v{}", chapters::CHAPTER_SCHEMA_VERSION))
}

fn target_directory(storage_dir: &Path, project_id: &str, target_id: &str) -> PathBuf {
    project_targets_directory(storage_dir, project_id).join(target_id)
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_id(value: &str) -> bool {
    (1..=128).contains(&value.len())
        && value.as_bytes()[0].is_ascii_alphanumeric()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapters::{ChapterTarget, ChapterTargetScope};

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "code-atlas-chapter-store-{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn draft(snapshot: &str, revision_id: &str) -> ChapterRevision {
        ChapterRevision {
            schema_version: chapters::CHAPTER_SCHEMA_VERSION,
            chapter_id: "chapter:file:test".to_owned(),
            revision_id: revision_id.to_owned(),
            revision_number: 1,
            project_id: "project-test".to_owned(),
            snapshot_id: snapshot.to_owned(),
            target: ChapterTarget {
                target_id: "target:file:test".to_owned(),
                scope: ChapterTargetScope::File {
                    path: "src/lib.rs".to_owned(),
                },
            },
            title: "How this module computes the answer".to_owned(),
            lifecycle: RevisionLifecycle::Draft,
            validation_status: ValidationStatus::Valid,
            created_at_unix_ms: 1,
            based_on_revision_id: None,
            decision_note: None,
            generation_origin: None,
            sections: Vec::new(),
        }
    }

    #[test]
    fn current_schema_store_ignores_unsupported_old_chapters_without_modifying_them() {
        let temp = TempDir::new();
        let legacy = temp
            .0
            .join("chapters")
            .join(digest(b"project-test"))
            .join("targets")
            .join("target:file:test")
            .join("revision:legacy.json");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        let legacy_bytes = br#"{"schemaVersion":4,"unsupported":"preserve verbatim"}"#;
        fs::write(&legacy, legacy_bytes).unwrap();

        assert!(
            list_for_project(&temp.0, "project-test")
                .unwrap()
                .is_empty()
        );
        let current =
            save_new_draft(&temp.0, draft("snapshot-current", "revision:current")).unwrap();
        assert_eq!(
            list_for_project(&temp.0, "project-test").unwrap(),
            vec![current.clone()]
        );
        assert_eq!(
            fs::read(
                target_directory(&temp.0, "project-test", "target:file:test")
                    .join("revision:current.json")
            )
            .unwrap(),
            chapters::encode_chapter_revision(&current).unwrap(),
        );
        assert_eq!(fs::read(&legacy).unwrap(), legacy_bytes);
    }

    #[test]
    fn immutable_drafts_append_with_monotonic_revision_numbers_and_snapshot_scoped_listing() {
        let temp = TempDir::new();
        let first = save_new_draft(&temp.0, draft("snapshot-z", "revision:first")).unwrap();
        let second = save_new_draft(&temp.0, draft("snapshot-a", "revision:second")).unwrap();
        assert_eq!(first.revision_number, 1);
        assert_eq!(second.revision_number, 2);
        assert_eq!(
            list_for_snapshot(&temp.0, "project-test", "snapshot-z").unwrap(),
            vec![first.clone()]
        );
        assert_eq!(
            list_for_snapshot(&temp.0, "project-test", "snapshot-a").unwrap(),
            vec![second.clone()]
        );
        assert_eq!(
            list_for_project(&temp.0, "project-test").unwrap(),
            vec![first, second]
        );
    }

    #[test]
    fn overview_identity_is_fixed_and_current_schema_only() {
        let temp = TempDir::new();
        let mut overview = draft("snapshot-overview", "revision:overview");
        overview.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
        overview.chapter_id = "chapter:overview".to_owned();
        overview.target = ChapterTarget {
            target_id: "target:overview".to_owned(),
            scope: ChapterTargetScope::Overview,
        };
        let overview_copy = "main defines the repository request entry point.".to_owned();
        overview.sections = vec![chapters::ChapterSection {
            section_id: "section:overview".to_owned(),
            section_revision_id: "sectionrev:overview".to_owned(),
            order: 0,
            title: "Repository entry point".to_owned(),
            prose: overview_copy.clone(),
            spoken_text: overview_copy.clone(),
            claims: vec![chapters::ChapterClaim {
                claim_id: "claim:overview".to_owned(),
                category: chapters::ClaimCategory::CodeBehavior,
                prose: overview_copy,
                anchors: vec![chapters::ChapterSourceAnchor {
                    anchor_id: "anchor:overview".to_owned(),
                    path: "src/lib.rs".to_owned(),
                    symbol_id: None,
                    snapshot_id: overview.snapshot_id.clone(),
                    source_sha256: "a".repeat(64),
                    start_byte: 0,
                    end_byte_inclusive: 3,
                    start_line: 1,
                    end_line: 1,
                    start_column: 1,
                    end_column: 4,
                    excerpt: "main".to_owned(),
                }],
                dependencies: Vec::new(),
            }],
            dependencies: Vec::new(),
            visual: None,
            hotspots: Vec::new(),
        }];
        let saved = save_new_draft(&temp.0, overview.clone()).unwrap();
        assert_eq!(
            list_for_project(&temp.0, "project-test").unwrap(),
            vec![saved]
        );

        for (chapter_id, target_id, schema_version) in [
            (
                "chapter:other",
                "target:overview",
                chapters::CHAPTER_SCHEMA_VERSION,
            ),
            (
                "chapter:overview",
                "target:other",
                chapters::CHAPTER_SCHEMA_VERSION,
            ),
            ("chapter:overview", "target:overview", 4),
        ] {
            let mut invalid = overview.clone();
            invalid.revision_id = format!("revision:invalid-{schema_version}-{chapter_id}");
            invalid.chapter_id = chapter_id.to_owned();
            invalid.target.target_id = target_id.to_owned();
            invalid.schema_version = schema_version;
            let isolated = TempDir::new();
            assert!(save_new_draft(&isolated.0, invalid).is_err());
        }
    }

    #[test]
    fn duplicate_revision_ids_and_non_draft_or_unvalidated_values_cannot_replace_saved_revisions() {
        let temp = TempDir::new();
        let original = save_new_draft(&temp.0, draft("snapshot-1", "revision:same")).unwrap();
        let mut duplicate = draft("snapshot-1", "revision:same");
        duplicate.title = "Changed attempt".to_owned();
        assert!(save_new_draft(&temp.0, duplicate).is_err());
        let listed = list_for_project(&temp.0, "project-test").unwrap();
        assert_eq!(listed, vec![original]);
        let mut accepted = draft("snapshot-1", "revision:accepted");
        accepted.lifecycle = RevisionLifecycle::Accepted;
        assert!(save_new_draft(&temp.0, accepted).is_err());
        let mut unvalidated = draft("snapshot-1", "revision:unvalidated");
        unvalidated.validation_status = ValidationStatus::Unvalidated;
        assert!(save_new_draft(&temp.0, unvalidated).is_err());
    }

    fn write_revision(storage: &Path, revision: &ChapterRevision) -> PathBuf {
        let directory = target_directory(storage, &revision.project_id, &revision.target.target_id);
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(format!("{}.json", revision.revision_id));
        fs::write(&path, chapters::encode_chapter_revision(revision).unwrap()).unwrap();
        path
    }

    fn reviewed_child(
        base: &ChapterRevision,
        revision_id: &str,
        revision_number: u32,
        lifecycle: RevisionLifecycle,
    ) -> ChapterRevision {
        let mut revision = base.clone();
        revision.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
        revision.revision_id = revision_id.to_owned();
        revision.revision_number = revision_number;
        revision.lifecycle = lifecycle.clone();
        revision.validation_status = if lifecycle == RevisionLifecycle::Accepted {
            ValidationStatus::Valid
        } else {
            ValidationStatus::Unvalidated
        };
        revision.created_at_unix_ms = u64::from(revision_number) + 1;
        revision.based_on_revision_id = Some(base.revision_id.clone());
        revision.decision_note = if lifecycle == RevisionLifecycle::Rejected {
            Some("The chapter needs revision.".to_owned())
        } else {
            None
        };
        revision
    }

    #[test]
    fn loading_rejects_acceptance_or_rejection_from_non_draft_history() {
        // Accepted and rejected revisions may only follow a saved draft.
        let temp = TempDir::new();
        let seed = draft("snapshot-1", "revision:seed");
        write_revision(&temp.0, &seed);
        let accepted_parent = reviewed_child(
            &seed,
            "revision:accepted-parent",
            2,
            RevisionLifecycle::Accepted,
        );
        write_revision(&temp.0, &accepted_parent);
        let accepted_child = reviewed_child(
            &accepted_parent,
            "revision:accepted-child",
            3,
            RevisionLifecycle::Accepted,
        );
        write_revision(&temp.0, &accepted_child);
        assert!(
            list_for_project(&temp.0, "project-test")
                .unwrap_err()
                .contains("must follow a saved draft")
        );

        let temp = TempDir::new();
        let seed = draft("snapshot-1", "revision:seed");
        write_revision(&temp.0, &seed);
        let rejected_parent = reviewed_child(
            &seed,
            "revision:rejected-parent",
            2,
            RevisionLifecycle::Rejected,
        );
        write_revision(&temp.0, &rejected_parent);
        let rejected_child = reviewed_child(
            &rejected_parent,
            "revision:rejected-child",
            3,
            RevisionLifecycle::Rejected,
        );
        write_revision(&temp.0, &rejected_child);
        assert!(
            list_for_project(&temp.0, "project-test")
                .unwrap_err()
                .contains("must follow a saved draft")
        );
    }

    #[test]
    fn loading_rejects_review_branches_that_skip_a_newer_same_snapshot_revision() {
        let temp = TempDir::new();
        let first = draft("snapshot-1", "revision:first");
        write_revision(&temp.0, &first);
        let mut newer = draft("snapshot-1", "revision:newer");
        newer.revision_number = 2;
        write_revision(&temp.0, &newer);
        let branch = reviewed_child(&first, "revision:branch", 3, RevisionLifecycle::Draft);
        write_revision(&temp.0, &branch);
        assert!(
            list_for_project(&temp.0, "project-test")
                .unwrap_err()
                .contains("latest prior revision")
        );
    }

    #[test]
    fn latest_parent_is_scoped_to_the_same_snapshot() {
        let temp = TempDir::new();
        let first_snapshot =
            save_new_draft(&temp.0, draft("snapshot-1", "revision:first-snapshot")).unwrap();
        let other_snapshot =
            save_new_draft(&temp.0, draft("snapshot-2", "revision:other-snapshot")).unwrap();
        let mut review = first_snapshot.clone();
        review.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
        review.revision_id = "revision:review".to_owned();
        review.revision_number = 0;
        review.based_on_revision_id = Some(first_snapshot.revision_id.clone());
        let saved = append_review_revision(&temp.0, review).unwrap();
        assert_eq!(saved.revision_number, 3);
        assert_eq!(
            list_for_project(&temp.0, "project-test").unwrap(),
            vec![first_snapshot, other_snapshot, saved]
        );
    }

    #[test]
    fn separate_targets_have_independent_numbers_and_global_immutable_revision_ids() {
        let temp = TempDir::new();
        let first = save_new_draft(&temp.0, draft("snapshot-1", "revision:first")).unwrap();
        let mut other = draft("snapshot-1", "revision:other");
        other.chapter_id = "chapter:file:other".to_owned();
        other.target.target_id = "target:file:other".to_owned();
        other.target.scope = ChapterTargetScope::File {
            path: "src/other.rs".to_owned(),
        };
        assert_eq!(
            save_new_draft(&temp.0, other.clone())
                .unwrap()
                .revision_number,
            1
        );
        other.revision_id = first.revision_id;
        assert!(
            save_new_draft(&temp.0, other)
                .unwrap_err()
                .contains("revision ID already exists")
        );
    }

    #[test]
    fn target_and_chapter_identity_conflicts_leave_existing_drafts_unchanged() {
        let temp = TempDir::new();
        let saved = save_new_draft(&temp.0, draft("snapshot-1", "revision:first")).unwrap();
        for field in ["chapter", "scope", "target"] {
            let mut changed = draft("snapshot-2", "revision:changed");
            match field {
                "chapter" => changed.chapter_id = "chapter:replacement".to_owned(),
                "scope" => {
                    changed.target.scope = ChapterTargetScope::File {
                        path: "src/other.rs".to_owned(),
                    }
                }
                "target" => changed.target.target_id = "target:other".to_owned(),
                _ => unreachable!(),
            }
            assert!(save_new_draft(&temp.0, changed).is_err(), "{field}");
        }
        assert_eq!(
            list_for_project(&temp.0, "project-test").unwrap(),
            vec![saved]
        );
    }

    #[test]
    fn malformed_revision_or_filename_identity_prevents_a_new_write() {
        let temp = TempDir::new();
        let existing = draft("snapshot-1", "revision:first");
        let path = write_revision(&temp.0, &existing);
        fs::write(&path, b"{malformed").unwrap();
        assert!(list_for_project(&temp.0, "project-test").is_err());
        assert!(save_new_draft(&temp.0, draft("snapshot-2", "revision:next")).is_err());
        fs::write(&path, chapters::encode_chapter_revision(&existing).unwrap()).unwrap();
        fs::rename(&path, path.with_file_name("revision:renamed.json")).unwrap();
        assert!(
            list_for_project(&temp.0, "project-test")
                .unwrap_err()
                .contains("storage path")
        );
        assert!(save_new_draft(&temp.0, draft("snapshot-2", "revision:next")).is_err());
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
    }

    #[test]
    fn wrong_project_or_target_payload_is_rejected_without_being_exposed() {
        for wrong_project in [true, false] {
            let temp = TempDir::new();
            let revision = draft("snapshot-1", "revision:first");
            let path = write_revision(&temp.0, &revision);
            let mut changed = revision;
            if wrong_project {
                changed.project_id = "another-project".to_owned();
            } else {
                changed.target.target_id = "target:another".to_owned();
            }
            fs::write(path, chapters::encode_chapter_revision(&changed).unwrap()).unwrap();
            assert!(
                list_for_project(&temp.0, "project-test")
                    .unwrap_err()
                    .contains("storage path")
            );
            assert!(save_new_draft(&temp.0, draft("snapshot-2", "revision:next")).is_err());
        }
    }

    #[test]
    fn repeated_numbers_inconsistent_scope_and_reused_revision_ids_are_rejected() {
        for conflict in ["number", "scope", "revision", "chapter"] {
            let temp = TempDir::new();
            write_revision(&temp.0, &draft("snapshot-1", "revision:first"));
            let mut second = draft("snapshot-2", "revision:second");
            second.revision_number = 2;
            match conflict {
                "number" => second.revision_number = 1,
                "scope" => {
                    second.target.scope = ChapterTargetScope::File {
                        path: "src/other.rs".to_owned(),
                    }
                }
                "revision" => {
                    second.revision_id = "revision:first".to_owned();
                    second.chapter_id = "chapter:other".to_owned();
                    second.target.target_id = "target:other".to_owned();
                }
                "chapter" => second.target.target_id = "target:other".to_owned(),
                _ => unreachable!(),
            }
            write_revision(&temp.0, &second);
            assert!(
                list_for_project(&temp.0, "project-test").is_err(),
                "{conflict}"
            );
            assert!(
                save_new_draft(&temp.0, draft("snapshot-3", "revision:next")).is_err(),
                "{conflict}"
            );
        }
    }

    #[test]
    fn revision_gaps_use_the_greatest_number_and_exhaustion_does_not_create_a_file() {
        let temp = TempDir::new();
        let mut existing = draft("snapshot-1", "revision:old");
        existing.revision_number = 10;
        write_revision(&temp.0, &existing);
        assert_eq!(
            save_new_draft(&temp.0, draft("snapshot-2", "revision:next"))
                .unwrap()
                .revision_number,
            11
        );
        existing.revision_id = "revision:max".to_owned();
        existing.revision_number = u32::MAX;
        write_revision(&temp.0, &existing);
        assert!(
            save_new_draft(&temp.0, draft("snapshot-3", "revision:overflow"))
                .unwrap_err()
                .contains("exhausted")
        );
        assert_eq!(
            fs::read_dir(target_directory(
                &temp.0,
                "project-test",
                "target:file:test"
            ))
            .unwrap()
            .count(),
            3
        );
    }

    #[test]
    fn invalid_storage_ids_and_metadata_cannot_create_directories() {
        let temp = TempDir::new();
        let too_long = "x".repeat(129);
        for value in [
            "",
            "../escape",
            "id/child",
            ".hidden",
            "é",
            too_long.as_str(),
        ] {
            for field in ["chapter", "revision", "target", "snapshot"] {
                let mut revision = draft("snapshot-1", "revision:first");
                match field {
                    "chapter" => revision.chapter_id = value.to_owned(),
                    "revision" => revision.revision_id = value.to_owned(),
                    "target" => revision.target.target_id = value.to_owned(),
                    "snapshot" => revision.snapshot_id = value.to_owned(),
                    _ => unreachable!(),
                }
                assert!(
                    save_new_draft(&temp.0, revision).is_err(),
                    "{field}={value}"
                );
            }
        }
        for field in [
            "project", "title", "created", "number", "same-id", "schema", "rejected", "invalid",
        ] {
            let mut revision = draft("snapshot-1", "revision:first");
            match field {
                "project" => revision.project_id = " ".to_owned(),
                "title" => revision.title = " ".to_owned(),
                "created" => revision.created_at_unix_ms = 0,
                "number" => revision.revision_number = 0,
                "same-id" => revision.chapter_id = revision.revision_id.clone(),
                "schema" => revision.schema_version = chapters::CHAPTER_SCHEMA_VERSION + 1,
                "rejected" => revision.lifecycle = RevisionLifecycle::Rejected,
                "invalid" => revision.validation_status = ValidationStatus::Invalid,
                _ => unreachable!(),
            }
            assert!(save_new_draft(&temp.0, revision).is_err(), "{field}");
        }
        assert_eq!(fs::read_dir(&temp.0).unwrap().count(), 0);
    }

    #[test]
    fn canonical_repository_paths_are_hashed_and_kept_as_project_identity() {
        let temp = TempDir::new();
        let mut revision = draft("snapshot-1", "revision:first");
        revision.project_id = "/Users/example/projects/路径 with spaces".to_owned();
        let saved = save_new_draft(&temp.0, revision).unwrap();
        assert_eq!(
            list_for_project(&temp.0, &saved.project_id).unwrap(),
            vec![saved.clone()]
        );
        assert!(
            list_for_project(&temp.0, "another-project")
                .unwrap()
                .is_empty()
        );
        assert!(project_targets_directory(&temp.0, &saved.project_id).exists());
    }

    #[test]
    fn accepted_revision_bytes_survive_new_drafts_and_failed_generation_saves() {
        let temp = TempDir::new();
        let seed = save_new_draft(&temp.0, draft("snapshot-1", "revision:seed")).unwrap();
        let accepted = append_review_revision(
            &temp.0,
            reviewed_child(&seed, "revision:accepted", 2, RevisionLifecycle::Accepted),
        )
        .unwrap();
        let path = target_directory(&temp.0, &accepted.project_id, &accepted.target.target_id)
            .join(format!("{}.json", accepted.revision_id));
        let bytes = fs::read(&path).unwrap();
        let next = save_new_draft(&temp.0, draft("snapshot-2", "revision:next")).unwrap();
        assert_eq!(next.revision_number, 3);
        assert!(save_new_draft(&temp.0, accepted.clone()).is_err());
        assert_eq!(fs::read(path).unwrap(), bytes);
        assert_eq!(
            list_for_snapshot(&temp.0, "project-test", "snapshot-1").unwrap(),
            vec![seed, accepted]
        );
    }

    #[test]
    fn oversized_files_and_reads_beyond_the_remaining_project_budget_are_rejected() {
        let temp = TempDir::new();
        let path = write_revision(&temp.0, &draft("snapshot-1", "revision:first"));
        let len = fs::metadata(&path).unwrap().len() as usize;
        assert!(read_revision_bytes(&path, len - 1).is_err());
        assert_eq!(read_revision_bytes(&path, len).unwrap().len(), len);
        OpenOptions::new()
            .write(true)
            .open(&path)
            .unwrap()
            .set_len(MAX_REVISION_JSON_BYTES as u64 + 1)
            .unwrap();
        assert!(
            list_for_project(&temp.0, "project-test")
                .unwrap_err()
                .contains("storage size")
        );
        assert!(save_new_draft(&temp.0, draft("snapshot-2", "revision:next")).is_err());
    }

    #[test]
    fn project_revision_limit_prevents_creating_an_unlistable_store() {
        let temp = TempDir::new();
        let directory = target_directory(&temp.0, "project-test", "target:file:test");
        fs::create_dir_all(&directory).unwrap();
        for number in 1..=MAX_REVISION_COUNT {
            let mut revision = draft("snapshot-1", &format!("revision:{number}"));
            revision.revision_number = number as u32;
            fs::write(
                directory.join(format!("{}.json", revision.revision_id)),
                chapters::encode_chapter_revision(&revision).unwrap(),
            )
            .unwrap();
        }
        assert_eq!(
            list_for_project(&temp.0, "project-test").unwrap().len(),
            MAX_REVISION_COUNT
        );
        assert!(
            save_new_draft(&temp.0, draft("snapshot-2", "revision:overflow"))
                .unwrap_err()
                .contains("revision limit")
        );
        let mut overflow = draft("snapshot-2", "revision:overflow");
        overflow.revision_number = MAX_REVISION_COUNT as u32 + 1;
        write_revision(&temp.0, &overflow);
        assert!(
            list_for_project(&temp.0, "project-test")
                .unwrap_err()
                .contains("revision count")
        );
    }

    #[test]
    fn missing_storage_and_staging_files_are_handled_without_leaking_a_completed_save() {
        let temp = TempDir::new();
        assert!(
            list_for_project(&temp.0.join("missing"), "project-test")
                .unwrap()
                .is_empty()
        );
        let saved = save_new_draft(&temp.0, draft("snapshot-1", "revision:first")).unwrap();
        let directory = target_directory(&temp.0, "project-test", "target:file:test");
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
        fs::write(directory.join(".draft-orphan.tmp"), b"partial").unwrap();
        assert_eq!(
            list_for_project(&temp.0, "project-test").unwrap(),
            vec![saved]
        );
        let mut scanned = ProjectRevisions {
            revisions: Vec::new(),
            total_bytes: 0,
            entries_visited: MAX_STORAGE_ENTRY_COUNT - 1,
        };
        assert!(visit_entry(&mut scanned).is_ok());
        assert!(visit_entry(&mut scanned).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_link_storage_components_cannot_redirect_reads_or_saves() {
        use std::os::unix::fs::symlink;
        for component in ["root", "chapters", "project", "targets", "target"] {
            let temp = TempDir::new();
            let outside = TempDir::new();
            let directories = storage_directories(&temp.0, "project-test");
            let path = match component {
                "root" => temp.0.join("linked-root"),
                "chapters" => directories[1].clone(),
                "project" => directories[2].clone(),
                "targets" => directories[3].clone(),
                "target" => target_directory(&temp.0, "project-test", "target:file:test"),
                _ => unreachable!(),
            };
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            symlink(&outside.0, &path).unwrap();
            let storage = if component == "root" {
                path.as_path()
            } else {
                temp.0.as_path()
            };
            assert!(
                list_for_project(storage, "project-test").is_err(),
                "{component}"
            );
            assert!(
                save_new_draft(storage, draft("snapshot-1", "revision:first")).is_err(),
                "{component}"
            );
            assert_eq!(fs::read_dir(&outside.0).unwrap().count(), 0, "{component}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_link_and_directory_revision_destinations_cannot_be_replaced() {
        use std::os::unix::fs::symlink;
        for dangling in [false, true] {
            let temp = TempDir::new();
            let directory = target_directory(&temp.0, "project-test", "target:file:test");
            fs::create_dir_all(&directory).unwrap();
            let outside = TempDir::new();
            let outside_path = outside.0.join("original");
            if !dangling {
                fs::write(&outside_path, b"untouched").unwrap();
            }
            let link_path = directory.join("revision:first.json");
            symlink(&outside_path, &link_path).unwrap();
            assert!(list_for_project(&temp.0, "project-test").is_err());
            assert!(save_new_draft(&temp.0, draft("snapshot-1", "revision:first")).is_err());
            assert!(
                fs::symlink_metadata(&link_path)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
            if !dangling {
                assert_eq!(fs::read(&outside_path).unwrap(), b"untouched");
            } else {
                assert!(!outside_path.exists());
            }
        }
        let temp = TempDir::new();
        let directory = target_directory(&temp.0, "project-test", "target:file:test")
            .join("revision:first.json");
        fs::create_dir_all(&directory).unwrap();
        assert!(list_for_project(&temp.0, "project-test").is_err());
        assert!(save_new_draft(&temp.0, draft("snapshot-1", "revision:first")).is_err());
        assert!(directory.is_dir());
    }
}
