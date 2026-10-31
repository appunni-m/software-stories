//! Durable staging for repairing multiple stale chapter sections one at a time.
//!
//! A repair session is deliberately not a `ChapterRevision`: unresolved sections
//! retain citations to their original snapshot and must never reach the normal
//! chapter validator as if those citations belonged to the target snapshot.

use crate::chapter_freshness::{ChapterFreshnessReport, SectionFreshnessStatus};
use crate::chapters::{ChapterRevision, ChapterSection, RevisionLifecycle};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

const SCHEMA_VERSION: u32 = 1;
const MAX_SESSION_BYTES: usize = 8 * 1024 * 1024;
const MAX_PROJECT_SESSION_BYTES: usize = 64 * 1024 * 1024;
const MAX_SESSION_COUNT: usize = 32;
const MAX_SECTIONS: usize = 500;
const MAX_DIR_ENTRIES: usize = MAX_SESSION_COUNT * 4;
static SESSION_WRITE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum RepairSectionState {
    Current,
    SafeToCarry,
    Stale,
    NeedsReview,
    Replaced,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepairSection {
    pub section_id: String,
    pub original_section_revision_id: String,
    pub state: RepairSectionState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replacement: Option<ChapterSection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SectionRepairSession {
    pub schema_version: u32,
    pub session_id: String,
    pub project_id: String,
    pub base_revision_id: String,
    pub source_snapshot_id: String,
    pub target_snapshot_id: String,
    pub freshness_sha256: String,
    pub created_at_unix_ms: u64,
    pub updated_at_unix_ms: u64,
    pub sections: Vec<RepairSection>,
}

impl SectionRepairSession {
    /// Creates a new session from the exact accepted revision and its freshness report.
    pub fn from_freshness(
        base: &ChapterRevision,
        report: &ChapterFreshnessReport,
    ) -> Result<Self, String> {
        if base.lifecycle != RevisionLifecycle::Accepted
            || report.chapter_revision_id != base.revision_id
            || report.from_snapshot_id != base.snapshot_id
            || report.to_snapshot_id == base.snapshot_id
            || base.project_id.trim().is_empty()
        {
            return Err("A section repair session needs an accepted chapter and its exact cross-snapshot freshness report.".to_owned());
        }
        if base.sections.is_empty() || base.sections.len() > MAX_SECTIONS {
            return Err(
                "This chapter has an unsupported number of sections for repair.".to_owned(),
            );
        }
        if report.sections.len() != base.sections.len() {
            return Err(
                "The freshness report does not describe every section in the accepted chapter."
                    .to_owned(),
            );
        }
        let assessed_ids = report
            .sections
            .iter()
            .map(|item| item.section_id.as_str())
            .collect::<std::collections::HashSet<_>>();
        if assessed_ids.len() != report.sections.len() {
            return Err("The freshness report contains duplicate chapter sections.".to_owned());
        }

        let mut sections = Vec::with_capacity(base.sections.len());
        for original in &base.sections {
            let assessed = report
                .sections
                .iter()
                .find(|item| item.section_id == original.section_id)
                .ok_or_else(|| "The freshness report is missing a chapter section.".to_owned())?;
            if assessed.section_revision_id != original.section_revision_id {
                return Err(
                    "The freshness report refers to a different section revision.".to_owned(),
                );
            }
            let state = match assessed.status {
                SectionFreshnessStatus::Current => RepairSectionState::Current,
                SectionFreshnessStatus::SafeToCarry => RepairSectionState::SafeToCarry,
                SectionFreshnessStatus::Stale => RepairSectionState::Stale,
                SectionFreshnessStatus::NeedsReview => RepairSectionState::NeedsReview,
            };
            sections.push(RepairSection {
                section_id: original.section_id.clone(),
                original_section_revision_id: original.section_revision_id.clone(),
                state,
                replacement: None,
            });
        }
        if report.sections.iter().any(|assessed| {
            !base
                .sections
                .iter()
                .any(|section| section.section_id == assessed.section_id)
        }) {
            return Err(
                "The freshness report contains a section outside the accepted chapter.".to_owned(),
            );
        }

        let now = now_unix_ms();
        let session = Self {
            schema_version: SCHEMA_VERSION,
            session_id: format!("repair:{}", Uuid::new_v4()),
            project_id: base.project_id.clone(),
            base_revision_id: base.revision_id.clone(),
            source_snapshot_id: base.snapshot_id.clone(),
            target_snapshot_id: report.to_snapshot_id.clone(),
            freshness_sha256: freshness_fingerprint(report)?,
            created_at_unix_ms: now,
            updated_at_unix_ms: now,
            sections,
        };
        validate_session(&session)?;
        Ok(session)
    }

    pub fn is_ready_to_assemble(&self) -> bool {
        self.sections.iter().all(|section| {
            matches!(
                section.state,
                RepairSectionState::Current
                    | RepairSectionState::SafeToCarry
                    | RepairSectionState::Replaced
            )
        })
    }

    pub fn pending_section_ids(&self) -> Vec<&str> {
        self.sections
            .iter()
            .filter(|section| {
                matches!(
                    section.state,
                    RepairSectionState::Stale | RepairSectionState::NeedsReview
                )
            })
            .map(|section| section.section_id.as_str())
            .collect()
    }
}

pub fn freshness_fingerprint(report: &ChapterFreshnessReport) -> Result<String, String> {
    let freshness_bytes = serde_json::to_vec(report)
        .map_err(|_| "The chapter freshness report could not be fingerprinted.".to_owned())?;
    Ok(digest(&freshness_bytes))
}

pub fn verify_binding(
    session: &SectionRepairSession,
    base: &ChapterRevision,
    report: &ChapterFreshnessReport,
) -> Result<(), String> {
    validate_session(session)?;
    if base.lifecycle != RevisionLifecycle::Accepted
        || session.project_id != base.project_id
        || session.base_revision_id != base.revision_id
        || session.source_snapshot_id != base.snapshot_id
        || session.target_snapshot_id != report.to_snapshot_id
        || report.chapter_revision_id != base.revision_id
        || report.from_snapshot_id != base.snapshot_id
        || session.freshness_sha256 != freshness_fingerprint(report)?
        || report.sections.len() != session.sections.len()
    {
        return Err("The section repair session no longer matches the accepted base, target snapshot, and freshness assessment.".to_owned());
    }
    for assessment in &report.sections {
        let stored = session
            .sections
            .iter()
            .find(|section| section.section_id == assessment.section_id)
            .ok_or_else(|| "The section repair session is missing a freshness entry.".to_owned())?;
        let original_state = match assessment.status {
            SectionFreshnessStatus::Current => RepairSectionState::Current,
            SectionFreshnessStatus::SafeToCarry => RepairSectionState::SafeToCarry,
            SectionFreshnessStatus::Stale => RepairSectionState::Stale,
            SectionFreshnessStatus::NeedsReview => RepairSectionState::NeedsReview,
        };
        if assessment.section_revision_id != stored.original_section_revision_id
            || (stored.state != original_state
                && !(original_state == RepairSectionState::Stale
                    && stored.state == RepairSectionState::Replaced))
        {
            return Err("The section repair session no longer matches the freshness status of every original section.".to_owned());
        }
    }
    Ok(())
}

/// Loads an existing session or durably creates one for this exact freshness assessment.
pub fn get_or_create(
    storage_dir: &Path,
    base: &ChapterRevision,
    report: &ChapterFreshnessReport,
) -> Result<SectionRepairSession, String> {
    let _guard = SESSION_WRITE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "The local section repair store is temporarily unavailable.".to_owned())?;
    let proposed = SectionRepairSession::from_freshness(base, report)?;
    let sessions = list_for_project(storage_dir, &base.project_id)?;
    if let Some(existing) = sessions.into_iter().find(|session| {
        session.base_revision_id == proposed.base_revision_id
            && session.source_snapshot_id == proposed.source_snapshot_id
            && session.target_snapshot_id == proposed.target_snapshot_id
            && session.freshness_sha256 == proposed.freshness_sha256
    }) {
        return Ok(existing);
    }
    save_new(storage_dir, proposed)
}

pub fn load(
    storage_dir: &Path,
    project_id: &str,
    session_id: &str,
) -> Result<SectionRepairSession, String> {
    let repairs_directory = storage_dir.join("section-repairs-v1");
    if !inspect_directory(&repairs_directory)?
        || !inspect_directory(&project_directory(storage_dir, project_id))?
    {
        return Err("The saved section repair session is unavailable.".to_owned());
    }
    let path = session_path(storage_dir, project_id, session_id)?;
    let bytes = read_bounded(&path, MAX_SESSION_BYTES)?;
    let session: SectionRepairSession = serde_json::from_slice(&bytes)
        .map_err(|_| "The saved section repair session is corrupt or unsupported.".to_owned())?;
    validate_session(&session)?;
    if session.project_id != project_id || session.session_id != session_id {
        return Err("The saved section repair session does not match this project.".to_owned());
    }
    Ok(session)
}

pub fn list_for_project(
    storage_dir: &Path,
    project_id: &str,
) -> Result<Vec<SectionRepairSession>, String> {
    if project_id.trim().is_empty() || project_id.len() > 4_096 {
        return Err("The section repair project identity is invalid.".to_owned());
    }
    let repairs_directory = storage_dir.join("section-repairs-v1");
    if !inspect_directory(&repairs_directory)? {
        return Ok(Vec::new());
    }
    let directory = project_directory(storage_dir, project_id);
    if !inspect_directory(&directory)? {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(&directory)
        .map_err(|_| "Saved section repair sessions could not be listed.".to_owned())?;
    let mut sessions = Vec::new();
    let mut visited = 0usize;
    let mut total_bytes = 0usize;
    for entry in entries {
        visited = visited.saturating_add(1);
        if visited > MAX_DIR_ENTRIES {
            return Err(
                "The section repair folder contains too many entries to inspect safely.".to_owned(),
            );
        }
        let entry =
            entry.map_err(|_| "A section repair entry could not be inspected.".to_owned())?;
        let metadata = fs::symlink_metadata(entry.path())
            .map_err(|_| "A section repair entry could not be inspected.".to_owned())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(
                "The section repair folder contains an unexpected non-file entry.".to_owned(),
            );
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        if sessions.len() >= MAX_SESSION_COUNT {
            return Err("This project reached the local section repair session limit.".to_owned());
        }
        let remaining = MAX_PROJECT_SESSION_BYTES.saturating_sub(total_bytes);
        let bytes = read_bounded(&entry.path(), MAX_SESSION_BYTES.min(remaining))?;
        total_bytes = total_bytes.saturating_add(bytes.len());
        if total_bytes > MAX_PROJECT_SESSION_BYTES {
            return Err(
                "Saved section repair sessions exceed the supported project storage limit."
                    .to_owned(),
            );
        }
        let session: SectionRepairSession = serde_json::from_slice(&bytes)
            .map_err(|_| "A saved section repair session is corrupt or unsupported.".to_owned())?;
        validate_session(&session)?;
        if session.project_id != project_id {
            return Err(
                "A saved section repair session belongs to a different project.".to_owned(),
            );
        }
        if entry.file_name() != std::ffi::OsStr::new(&session_filename(&session.session_id)) {
            return Err("A section repair session has an invalid storage identity.".to_owned());
        }
        sessions.push(session);
    }
    sessions.sort_by(|left, right| {
        right
            .updated_at_unix_ms
            .cmp(&left.updated_at_unix_ms)
            .then_with(|| left.session_id.cmp(&right.session_id))
    });
    Ok(sessions)
}

/// Records a replacement only after the caller has canonicalized and validated it
/// against this session's target snapshot.
pub fn record_canonicalized_replacement(
    storage_dir: &Path,
    project_id: &str,
    session_id: &str,
    base_revision_id: &str,
    target_snapshot_id: &str,
    freshness_sha256: &str,
    section_id: &str,
    original_section_revision_id: &str,
    replacement: ChapterSection,
) -> Result<SectionRepairSession, String> {
    let _guard = SESSION_WRITE_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "The local section repair store is temporarily unavailable.".to_owned())?;
    let mut session = load(storage_dir, project_id, session_id)?;
    if session.base_revision_id != base_revision_id
        || session.target_snapshot_id != target_snapshot_id
        || session.freshness_sha256 != freshness_sha256
    {
        return Err("The selected repair session is out of date. Reassess source freshness before sending or saving a replacement.".to_owned());
    }
    let target = session
        .sections
        .iter_mut()
        .find(|item| item.section_id == section_id)
        .ok_or_else(|| "The selected section is not part of this repair session.".to_owned())?;
    if !matches!(
        target.state,
        RepairSectionState::Stale | RepairSectionState::Replaced
    ) || target.original_section_revision_id != original_section_revision_id
        || replacement.section_id != section_id
        || replacement.section_revision_id.trim().is_empty()
        || replacement.section_revision_id == original_section_revision_id
    {
        return Err(
            "The replacement does not match a pending stale section in this repair session."
                .to_owned(),
        );
    }
    target.state = RepairSectionState::Replaced;
    target.replacement = Some(replacement);
    session.updated_at_unix_ms = now_unix_ms();
    save_existing(storage_dir, &session)?;
    Ok(session)
}

fn save_new(
    storage_dir: &Path,
    session: SectionRepairSession,
) -> Result<SectionRepairSession, String> {
    validate_session(&session)?;
    let existing = list_for_project(storage_dir, &session.project_id)?;
    if existing.len() >= MAX_SESSION_COUNT {
        return Err("This project reached the local section repair session limit.".to_owned());
    }
    let bytes = serde_json::to_vec(&session)
        .map_err(|_| "The section repair session could not be encoded.".to_owned())?;
    if bytes.len() > MAX_SESSION_BYTES {
        return Err("This section repair session exceeds the 8 MB safety limit.".to_owned());
    }
    let existing_bytes = encoded_total_bytes(&existing)?;
    if existing_bytes.saturating_add(bytes.len()) > MAX_PROJECT_SESSION_BYTES {
        return Err(
            "Saved section repair sessions exceed the supported project storage limit.".to_owned(),
        );
    }
    let directory = ensure_project_directory(storage_dir, &session.project_id)?;
    let path = directory.join(session_filename(&session.session_id));
    match fs::symlink_metadata(&path) {
        Ok(_) => return Err("This section repair session already exists.".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(
                "The section repair session destination could not be inspected.".to_owned(),
            );
        }
    }
    atomic_write(&path, &bytes)?;
    Ok(session)
}

fn save_existing(storage_dir: &Path, session: &SectionRepairSession) -> Result<(), String> {
    validate_session(session)?;
    let bytes = serde_json::to_vec(session)
        .map_err(|_| "The section repair session could not be encoded.".to_owned())?;
    if bytes.len() > MAX_SESSION_BYTES {
        return Err("This section repair session exceeds the 8 MB safety limit.".to_owned());
    }
    let existing = list_for_project(storage_dir, &session.project_id)?;
    if !existing
        .iter()
        .any(|current| current.session_id == session.session_id)
    {
        return Err(
            "The section repair session disappeared before its replacement was saved.".to_owned(),
        );
    }
    let other_bytes = encoded_total_bytes(
        &existing
            .into_iter()
            .filter(|current| current.session_id != session.session_id)
            .collect::<Vec<_>>(),
    )?;
    if other_bytes.saturating_add(bytes.len()) > MAX_PROJECT_SESSION_BYTES {
        return Err(
            "Saved section repair sessions exceed the supported project storage limit.".to_owned(),
        );
    }
    let path = session_path(storage_dir, &session.project_id, &session.session_id)?;
    atomic_write(&path, &bytes)
}

fn encoded_total_bytes(sessions: &[SectionRepairSession]) -> Result<usize, String> {
    sessions.iter().try_fold(0usize, |total, session| {
        let length = serde_json::to_vec(session)
            .map_err(|_| "A saved section repair session could not be measured.".to_owned())?
            .len();
        Ok(total.saturating_add(length))
    })
}

fn validate_session(session: &SectionRepairSession) -> Result<(), String> {
    if session.schema_version != SCHEMA_VERSION
        || !valid_session_id(&session.session_id)
        || session.project_id.trim().is_empty()
        || session.project_id.len() > 4_096
        || !valid_identifier(&session.base_revision_id)
        || !valid_identifier(&session.source_snapshot_id)
        || !valid_identifier(&session.target_snapshot_id)
        || session.source_snapshot_id == session.target_snapshot_id
        || !valid_digest(&session.freshness_sha256)
        || session.created_at_unix_ms == 0
        || session.updated_at_unix_ms < session.created_at_unix_ms
        || session.sections.is_empty()
        || session.sections.len() > MAX_SECTIONS
    {
        return Err(
            "The saved section repair session failed its identity or size checks.".to_owned(),
        );
    }
    let mut section_ids = std::collections::HashSet::new();
    for section in &session.sections {
        if !valid_identifier(&section.section_id)
            || !valid_identifier(&section.original_section_revision_id)
            || !section_ids.insert(section.section_id.as_str())
        {
            return Err(
                "The saved section repair session contains an invalid or duplicate section."
                    .to_owned(),
            );
        }
        match (section.state, section.replacement.as_ref()) {
            (RepairSectionState::Replaced, Some(replacement))
                if replacement.section_id == section.section_id
                    && valid_identifier(&replacement.section_revision_id)
                    && replacement.section_revision_id != section.original_section_revision_id => {}
            (RepairSectionState::Replaced, _) => {
                return Err("A repaired section is missing its validated replacement.".to_owned());
            }
            (_, None) => {}
            (_, Some(_)) => {
                return Err("An unresolved section unexpectedly contains a replacement.".to_owned());
            }
        }
    }
    Ok(())
}

fn project_directory(storage_dir: &Path, project_id: &str) -> PathBuf {
    storage_dir
        .join("section-repairs-v1")
        .join(digest(project_id.as_bytes()))
}

fn ensure_project_directory(storage_dir: &Path, project_id: &str) -> Result<PathBuf, String> {
    if project_id.trim().is_empty() || project_id.len() > 4_096 {
        return Err("The section repair project identity is invalid.".to_owned());
    }
    let root = storage_dir.join("section-repairs-v1");
    ensure_private_directory(storage_dir)?;
    ensure_private_directory(&root)?;
    let project = project_directory(storage_dir, project_id);
    ensure_private_directory(&project)?;
    Ok(project)
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err("The section repair storage path is not a regular directory.".to_owned())
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path)
                .map_err(|_| "The local section repair folder could not be created.".to_owned())?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| {
                    "Private section repair folder permissions could not be applied.".to_owned()
                })?;
            }
            Ok(())
        }
        Err(_) => Err("The local section repair folder could not be inspected.".to_owned()),
    }
}

fn inspect_directory(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            Err("The section repair folder is not a regular directory.".to_owned())
        }
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("The section repair folder could not be inspected.".to_owned()),
    }
}

fn session_path(storage_dir: &Path, project_id: &str, session_id: &str) -> Result<PathBuf, String> {
    if !valid_session_id(session_id) || project_id.trim().is_empty() || project_id.len() > 4_096 {
        return Err("The section repair session identity is invalid.".to_owned());
    }
    Ok(project_directory(storage_dir, project_id).join(session_filename(session_id)))
}

fn session_filename(session_id: &str) -> String {
    format!("{}.json", session_id.trim_start_matches("repair:"))
}

fn valid_session_id(value: &str) -> bool {
    let Some(uuid) = value.strip_prefix("repair:") else {
        return false;
    };
    Uuid::parse_str(uuid).is_ok_and(|parsed| parsed.to_string() == uuid)
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'_' | b'-' | b'.'))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    if limit == 0 {
        return Err(
            "Saved section repair data exceeds the supported project storage limit.".to_owned(),
        );
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| "A saved section repair file could not be inspected.".to_owned())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > limit as u64 {
        return Err(
            "Saved section repair data exceeds its safety limit or is not a regular file."
                .to_owned(),
        );
    }
    let file = File::open(path)
        .map_err(|_| "A saved section repair file could not be opened.".to_owned())?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "A saved section repair file could not be read.".to_owned())?;
    if bytes.len() > limit {
        return Err("Saved section repair data exceeds its safety limit.".to_owned());
    }
    Ok(bytes)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "The section repair storage folder is unavailable.".to_owned())?;
    let temporary = parent.join(format!(".repair-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|_| "A private section repair file could not be staged.".to_owned())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| "A section repair session could not be written and synced.".to_owned())?;
        drop(file);
        fs::rename(&temporary, path).map_err(|_| {
            "A section repair session could not be published atomically.".to_owned()
        })?;
        #[cfg(unix)]
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| "The section repair session could not be durably published.".to_owned())?;
        Ok(())
    })();
    let _ = fs::remove_file(temporary);
    result
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_freshness::{EvidenceState, SectionFreshness};
    use crate::chapters::{
        ChapterClaim, ChapterSourceAnchor, ChapterTarget, ChapterTargetScope, ClaimCategory,
        ValidationStatus,
    };
    use uuid::Uuid;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "code-atlas-section-repair-{}-{}",
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

    fn base() -> ChapterRevision {
        ChapterRevision {
            schema_version: crate::chapters::CHAPTER_SCHEMA_VERSION,
            chapter_id: "chapter:one".to_owned(),
            revision_id: "revision:accepted".to_owned(),
            revision_number: 4,
            project_id: "project:one".to_owned(),
            snapshot_id: "snapshot:source".to_owned(),
            target: ChapterTarget {
                target_id: "target:one".to_owned(),
                scope: ChapterTargetScope::File {
                    path: "src/lib.rs".to_owned(),
                },
            },
            title: "Test chapter".to_owned(),
            lifecycle: RevisionLifecycle::Accepted,
            validation_status: ValidationStatus::Valid,
            created_at_unix_ms: 1,
            based_on_revision_id: None,
            decision_note: None,
            generation_origin: None,
            sections: vec![
                section("section:first", "sectionrev:first"),
                section("section:second", "sectionrev:second"),
            ],
        }
    }

    fn section(id: &str, revision_id: &str) -> ChapterSection {
        ChapterSection {
            section_id: id.to_owned(),
            section_revision_id: revision_id.to_owned(),
            order: 0,
            title: id.to_owned(),
            prose: "The section explains the implementation.".to_owned(),
            spoken_text: "The implementation follows this path.".to_owned(),
            claims: vec![ChapterClaim {
                claim_id: format!("claim:{id}"),
                category: ClaimCategory::CodeBehavior,
                prose: "The function validates the input.".to_owned(),
                dependencies: Vec::new(),
                anchors: vec![ChapterSourceAnchor {
                    anchor_id: format!("anchor:{id}"),
                    path: "src/lib.rs".to_owned(),
                    symbol_id: None,
                    snapshot_id: "snapshot:source".to_owned(),
                    source_sha256: "a".repeat(64),
                    start_byte: 0,
                    end_byte_inclusive: 11,
                    start_line: 1,
                    end_line: 2,
                    start_column: 1,
                    end_column: 12,
                    excerpt: "fn main() {}".to_owned(),
                }],
            }],
            dependencies: Vec::new(),
            visual: None,
            hotspots: Vec::new(),
        }
    }

    fn report(base: &ChapterRevision) -> ChapterFreshnessReport {
        ChapterFreshnessReport {
            chapter_revision_id: base.revision_id.clone(),
            from_snapshot_id: base.snapshot_id.clone(),
            to_snapshot_id: "snapshot:target".to_owned(),
            indexes: crate::chapter_freshness::FreshnessIndexPair {
                from: crate::chapter_freshness::FreshnessIndexSummary {
                    status: crate::chapter_freshness::FreshnessIndexStatus::Unavailable,
                    indexed_files: None,
                    total_files: None,
                },
                to: crate::chapter_freshness::FreshnessIndexSummary {
                    status: crate::chapter_freshness::FreshnessIndexStatus::Unavailable,
                    indexed_files: None,
                    total_files: None,
                },
            },
            sections: vec![
                freshness(
                    "section:first",
                    "sectionrev:first",
                    SectionFreshnessStatus::Stale,
                ),
                freshness(
                    "section:second",
                    "sectionrev:second",
                    SectionFreshnessStatus::Stale,
                ),
            ],
            reverse_dependencies: Vec::new(),
        }
    }

    fn freshness(id: &str, revision_id: &str, status: SectionFreshnessStatus) -> SectionFreshness {
        SectionFreshness {
            section_id: id.to_owned(),
            section_revision_id: revision_id.to_owned(),
            status,
            evidence_state: EvidenceState::Complete,
            reasons: Vec::new(),
            source_changes: Vec::new(),
            narration_script_sha256: None,
        }
    }

    #[test]
    fn session_persists_one_stale_replacement_and_leaves_the_other_pending() {
        let temp = TempDir::new();
        let base = base();
        let report = report(&base);
        let session = get_or_create(&temp.0, &base, &report).unwrap();
        assert_eq!(
            session.pending_section_ids(),
            vec!["section:first", "section:second"]
        );

        let replacement = ChapterSection {
            section_revision_id: "sectionrev:first-new".to_owned(),
            ..section("section:first", "sectionrev:first-new")
        };
        let progressed = record_canonicalized_replacement(
            &temp.0,
            &base.project_id,
            &session.session_id,
            &base.revision_id,
            &report.to_snapshot_id,
            &session.freshness_sha256,
            "section:first",
            "sectionrev:first",
            replacement,
        )
        .unwrap();
        assert_eq!(progressed.pending_section_ids(), vec!["section:second"]);
        assert!(!progressed.is_ready_to_assemble());

        let restored = load(&temp.0, &base.project_id, &session.session_id).unwrap();
        assert_eq!(restored, progressed);
        assert_eq!(
            list_for_project(&temp.0, &base.project_id).unwrap(),
            vec![progressed]
        );
    }

    #[test]
    fn a_changed_freshness_binding_cannot_record_a_replacement() {
        let temp = TempDir::new();
        let base = base();
        let report = report(&base);
        let session = get_or_create(&temp.0, &base, &report).unwrap();
        let error = record_canonicalized_replacement(
            &temp.0,
            &base.project_id,
            &session.session_id,
            &base.revision_id,
            &report.to_snapshot_id,
            &"0".repeat(64),
            "section:first",
            "sectionrev:first",
            section("section:first", "sectionrev:first-new"),
        )
        .unwrap_err();
        assert!(error.contains("out of date"));
        assert_eq!(
            load(&temp.0, &base.project_id, &session.session_id).unwrap(),
            session
        );
    }

    #[test]
    fn damaged_or_oversized_session_files_are_rejected() {
        let temp = TempDir::new();
        let base = base();
        let session = get_or_create(&temp.0, &base, &report(&base)).unwrap();
        let path = session_path(&temp.0, &base.project_id, &session.session_id).unwrap();
        fs::write(&path, b"{").unwrap();
        assert!(
            load(&temp.0, &base.project_id, &session.session_id)
                .unwrap_err()
                .contains("corrupt")
        );

        fs::File::create(&path)
            .unwrap()
            .set_len((MAX_SESSION_BYTES + 1) as u64)
            .unwrap();
        assert!(
            load(&temp.0, &base.project_id, &session.session_id)
                .unwrap_err()
                .contains("safety limit")
        );
    }

    #[test]
    fn freshness_report_must_cover_exactly_the_accepted_sections() {
        let base = base();
        let mut report = report(&base);
        report.sections.pop();
        assert!(
            SectionRepairSession::from_freshness(&base, &report)
                .unwrap_err()
                .contains("every section")
        );
    }

    #[test]
    fn a_needs_review_section_prevents_assembly_without_repairing_it() {
        let base = base();
        let mut report = report(&base);
        report.sections[1].status = SectionFreshnessStatus::NeedsReview;
        let session = SectionRepairSession::from_freshness(&base, &report).unwrap();
        assert_eq!(
            session.pending_section_ids(),
            vec!["section:first", "section:second"]
        );
        assert!(!session.is_ready_to_assemble());
    }
}
