//! Explicit local plan decisions. Revisions retain the complete validated
//! proposal and stable source/step identities. They do not accept chapters,
//! change coverage/read progress, grant consent or qualify an agent.

use crate::{
    recovery::{InterruptedOperationKind, InterruptionJournal},
    repository_plan::canonical_uuid_id,
    repository_plan_store::{self as drafts, RepositoryPlanDraftDetail, SavedPlanRecord},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const MAX_REVISION_BYTES: usize = 2 * 1024 * 1024 + 4096;
const MAX_TOTAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_REVISIONS: usize = 128;
const MAX_ENTRIES: usize = 512;

#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepositoryPlanRevision {
    pub revision_id: String,
    pub history_id: String,
    pub revision_number: u32,
    pub based_on_revision_id: Option<String>,
    pub accepted_at_unix_ms: u64,
    pub draft_id: String,
    pub record_sha256: String,
    pub title: String,
    pub question: String,
    pub snapshot_id: String,
    pub provider_id: String,
    pub model_id: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RevisionFile {
    schema_version: u32,
    revision: RepositoryPlanRevision,
    record: SavedPlanRecord,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryPlanRevisionSummary {
    revision: RepositoryPlanRevision,
    is_head: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryPlanRevisionDetail {
    revision: RepositoryPlanRevision,
    plan: RepositoryPlanDraftDetail,
    is_head: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AcceptRepositoryPlanInput {
    pub project_id: String,
    pub draft_id: String,
    pub expected_record_sha256: String,
    pub based_on_revision_id: Option<String>,
}

/// A result identity and its last reviewed canonical digest, never a journal
/// entry, storage root or reusable generation authorization from the renderer.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum ReviewedPlanSource {
    #[serde(rename_all = "camelCase")]
    Draft {
        draft_id: String,
        record_sha256: String,
    },
    #[serde(rename_all = "camelCase")]
    Revision {
        revision_id: String,
        record_sha256: String,
    },
}

pub(crate) fn list(
    storage: &Path,
    project_id: &str,
) -> Result<Vec<RepositoryPlanRevisionSummary>, String> {
    let _guard = drafts::lock_store()?;
    let contents = read_history(storage, project_id)?;
    let mut summaries = contents
        .records
        .iter()
        .map(|record| RepositoryPlanRevisionSummary {
            revision: record.revision.clone(),
            is_head: is_head(&contents.records, &record.revision),
        })
        .collect::<Vec<_>>();
    summaries.sort_by(|a, b| {
        b.revision
            .accepted_at_unix_ms
            .cmp(&a.revision.accepted_at_unix_ms)
            .then_with(|| b.revision.revision_number.cmp(&a.revision.revision_number))
            .then_with(|| a.revision.revision_id.cmp(&b.revision.revision_id))
    });
    Ok(summaries)
}

pub(crate) fn load(
    storage: &Path,
    project_id: &str,
    revision_id: &str,
) -> Result<RepositoryPlanRevisionDetail, String> {
    let _guard = drafts::lock_store()?;
    let contents = read_history(storage, project_id)?;
    let record = find_revision(&contents.records, revision_id)?;
    drafts::revalidate(storage, &record.record)?;
    revision_detail(record, is_head(&contents.records, &record.revision))
}

/// Append one explicit decision. A deterministic family/number destination
/// prevents two writers from publishing different children for one parent.
/// The input digest binds the exact canonical record shown at review.
pub(crate) fn accept(
    storage: &Path,
    input: AcceptRepositoryPlanInput,
) -> Result<RepositoryPlanRevisionDetail, String> {
    let _guard = drafts::lock_store()?;
    if !canonical_uuid_id(&input.draft_id, "plan") || !valid_digest(&input.expected_record_sha256) {
        return Err("Open and review the exact saved draft before accepting it.".to_owned());
    }
    let directory = history_directory(storage, &input.project_id, true)?
        .ok_or("Accepted plan storage is unavailable.")?;
    // Every cooperating native acceptor locks this persistent inode before
    // reading history or choosing a family. Never delete/recreate the lock.
    let _history_guard = lock_history(&directory)?;
    let contents = read_history(storage, &input.project_id)?;
    // A lost reply can be reconciled after the original draft was discarded.
    if let Some(existing) = contents
        .records
        .iter()
        .find(|item| item.revision.draft_id == input.draft_id)
    {
        if existing.revision.record_sha256 != input.expected_record_sha256
            || existing.revision.based_on_revision_id != input.based_on_revision_id
        {
            return Err("This draft already has a different immutable acceptance decision. Refresh accepted plan history before proceeding.".to_owned());
        }
        drafts::revalidate(storage, &existing.record)?;
        return revision_detail(existing, is_head(&contents.records, &existing.revision));
    }
    let record = drafts::read_draft_record(storage, &input.project_id, &input.draft_id)?;
    if drafts::record_digest(&record)? != input.expected_record_sha256 {
        return Err("The saved draft changed after review. Reopen it before accepting; existing history was preserved.".to_owned());
    }
    drafts::revalidate(storage, &record)?;
    let (history_id, revision_number) = if let Some(parent_id) = &input.based_on_revision_id {
        let parent = find_revision(&contents.records, parent_id)?;
        if !is_head(&contents.records, &parent.revision) {
            return Err("This plan has a newer accepted revision. Refresh history and review the intended parent before accepting an update.".to_owned());
        }
        if parent.record.scope.question != record.scope.question {
            return Err("An updated plan must keep its original learning question. Accept this different question as a new independent plan.".to_owned());
        }
        // UI comparison is not authority: replay the parent's original
        // evidence again before appending an update to its history.
        drafts::revalidate(storage, &parent.record)?;
        (
            parent.revision.history_id.clone(),
            parent
                .revision
                .revision_number
                .checked_add(1)
                .ok_or("The plan revision number is exhausted.")?,
        )
    } else {
        (format!("plan-history:{}", Uuid::new_v4()), 1)
    };
    let revision = RepositoryPlanRevision {
        revision_id: format!("plan-revision:{}", Uuid::new_v4()),
        history_id,
        revision_number,
        based_on_revision_id: input.based_on_revision_id,
        accepted_at_unix_ms: now()?,
        draft_id: record.preview.plan.draft_id.clone(),
        record_sha256: input.expected_record_sha256,
        title: record.preview.plan.title.clone(),
        question: record.scope.question.clone(),
        snapshot_id: record.preview.plan.snapshot_id.clone(),
        provider_id: record.preview.provider_id.clone(),
        model_id: record.preview.model_id.clone(),
    };
    let saved = RevisionFile {
        schema_version: 1,
        revision,
        record,
    };
    let bytes = serde_json::to_vec(&saved)
        .map_err(|_| "The accepted plan could not be encoded.".to_owned())?;
    if bytes.len() > MAX_REVISION_BYTES
        || contents.records.len() >= MAX_REVISIONS
        || contents.entries >= MAX_ENTRIES
        || contents.bytes.saturating_add(bytes.len() as u64) > MAX_TOTAL_BYTES
    {
        return Err("Accepted plan history is full or this record exceeds its size limit. Existing revisions and the original draft were preserved; no history was pruned.".to_owned());
    }
    let target = directory.join(revision_filename(&saved.revision)?);
    publish(&directory, &target, &bytes)?;
    revision_detail(&saved, true)
}

/// Resolve only an exact native journal scope after full saved-result replay.
/// Store -> journal is the sole lock order; no agent runtime is called here.
pub(crate) fn resolve_notice(
    storage: &Path,
    journal: &InterruptionJournal,
    project_id: &str,
    source: ReviewedPlanSource,
) -> Result<bool, String> {
    let _guard = drafts::lock_store()?;
    let (record, digest) = match source {
        ReviewedPlanSource::Draft {
            draft_id,
            record_sha256,
        } => (
            drafts::read_draft_record(storage, project_id, &draft_id)?,
            record_sha256,
        ),
        ReviewedPlanSource::Revision {
            revision_id,
            record_sha256,
        } => {
            let contents = read_history(storage, project_id)?;
            (
                find_revision(&contents.records, &revision_id)?
                    .record
                    .clone(),
                record_sha256,
            )
        }
    };
    if !valid_digest(&digest) || drafts::record_digest(&record)? != digest {
        return Err(
            "The reviewed plan changed. Reopen it before resolving a recovery notice.".to_owned(),
        );
    }
    drafts::revalidate(storage, &record)?;
    let operations = journal.list(storage)?;
    let Some(operation) = operations
        .iter()
        .find(|operation| operation.operation_id == record.preview.recovery_operation_id)
    else {
        return Ok(false);
    };
    let matches = operation.kind == InterruptedOperationKind::ProviderGeneration
        && operation.project_id == project_id
        && operation.snapshot_id == record.preview.plan.snapshot_id
        && operation.target_path.is_none()
        && operation.provider_request.as_ref().is_some_and(|request| {
            let selected = request
                .selected_paths
                .iter()
                .map(String::as_str)
                .collect::<HashSet<_>>();
            request.provider_id == record.preview.provider_id
                && request.purpose == "repositoryPlan"
                && request.repository_plan.as_ref() == Some(&record.scope)
                && request.section_update.is_none()
                && request.code_lens.is_none()
                && request.git.is_none()
                && selected.len() == request.selected_paths.len()
                && selected.len() == record.selected_paths.len()
                && record
                    .selected_paths
                    .iter()
                    .all(|path| selected.contains(path.as_str()))
        });
    if !matches {
        return Err("The saved plan does not match this recovery notice's original question, source or backend. The notice was preserved.".to_owned());
    }
    journal.discard_if_unchanged(storage, operation)
}

struct HistoryContents {
    records: Vec<RevisionFile>,
    entries: usize,
    bytes: u64,
}

fn read_history(storage: &Path, project_id: &str) -> Result<HistoryContents, String> {
    let mut result = HistoryContents {
        records: Vec::new(),
        entries: 0,
        bytes: 0,
    };
    let Some(directory) = history_directory(storage, project_id, false)? else {
        return Ok(result);
    };
    for entry in fs::read_dir(directory).map_err(|_| "Accepted plan history could not be read.")? {
        result.entries += 1;
        if result.entries > MAX_ENTRIES {
            return Err(
                "Accepted plan history contains too many entries. Existing data was preserved."
                    .to_owned(),
            );
        }
        let entry = entry.map_err(|_| "An accepted plan entry could not be inspected.")?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| "An accepted plan entry could not be inspected.")?;
        if !metadata.is_file() {
            return Err(
                "Accepted plan history contains a nonregular entry. Existing data was preserved."
                    .to_owned(),
            );
        }
        result.bytes = result
            .bytes
            .checked_add(metadata.len())
            .ok_or("Accepted plan history has an unsupported size.")?;
        if result.bytes > MAX_TOTAL_BYTES {
            return Err(
                "Accepted plan history exceeds its local byte limit. Existing data was preserved."
                    .to_owned(),
            );
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.ends_with(".json") {
            continue;
        }
        let bytes = drafts::read_bounded_file(&path, MAX_REVISION_BYTES)?;
        let saved: RevisionFile = serde_json::from_slice(&bytes).map_err(
            |_| "An accepted plan revision is damaged or incompatible. No history was changed.",
        )?;
        validate_revision(&saved, project_id)?;
        if revision_filename(&saved.revision)? != name {
            return Err("An accepted plan file does not match its immutable history identity. Existing data was preserved.".to_owned());
        }
        result.records.push(saved);
        if result.records.len() > MAX_REVISIONS {
            return Err(
                "Accepted plan history contains too many revisions. Existing data was preserved."
                    .to_owned(),
            );
        }
    }
    validate_chains(&result.records)?;
    Ok(result)
}

fn validate_revision(saved: &RevisionFile, project_id: &str) -> Result<(), String> {
    let r = &saved.revision;
    drafts::validate_record_shape(&saved.record, project_id, &r.draft_id)?;
    if saved.schema_version != 1
        || !canonical_uuid_id(&r.revision_id, "plan-revision")
        || !canonical_uuid_id(&r.history_id, "plan-history")
        || r.revision_number == 0
        || r.based_on_revision_id
            .as_ref()
            .is_some_and(|id| !canonical_uuid_id(id, "plan-revision"))
        || r.accepted_at_unix_ms == 0
        || r.accepted_at_unix_ms > (1_u64 << 53) - 1
        || r.record_sha256 != drafts::record_digest(&saved.record)?
        || r.title != saved.record.preview.plan.title
        || r.question != saved.record.scope.question
        || r.snapshot_id != saved.record.preview.plan.snapshot_id
        || r.provider_id != saved.record.preview.provider_id
        || r.model_id != saved.record.preview.model_id
    {
        return Err("An accepted plan revision has inconsistent identity or content. Existing history was preserved.".to_owned());
    }
    Ok(())
}

fn validate_chains(records: &[RevisionFile]) -> Result<(), String> {
    let mut revision_ids = HashSet::new();
    let mut draft_ids = HashSet::new();
    let mut histories = BTreeMap::<&str, Vec<&RepositoryPlanRevision>>::new();
    for record in records {
        if !revision_ids.insert(&record.revision.revision_id)
            || !draft_ids.insert(&record.revision.draft_id)
        {
            return Err("Accepted plan history contains repeated revision or draft identities. No history was changed.".to_owned());
        }
        histories
            .entry(&record.revision.history_id)
            .or_default()
            .push(&record.revision);
    }
    for revisions in histories.values_mut() {
        revisions.sort_by_key(|revision| revision.revision_number);
        for (index, revision) in revisions.iter().enumerate() {
            let parent = index
                .checked_sub(1)
                .map(|previous| revisions[previous].revision_id.as_str());
            if revision.revision_number as usize != index + 1
                || revision.based_on_revision_id.as_deref() != parent
                || revision.question != revisions[0].question
            {
                return Err("Accepted plan history has a gap, fork or mismatched parent/question. Existing revisions were preserved.".to_owned());
            }
        }
    }
    Ok(())
}

fn is_head(records: &[RevisionFile], revision: &RepositoryPlanRevision) -> bool {
    !records.iter().any(|item| {
        item.revision.history_id == revision.history_id
            && item.revision.revision_number > revision.revision_number
    })
}

fn find_revision<'a>(
    records: &'a [RevisionFile],
    revision_id: &str,
) -> Result<&'a RevisionFile, String> {
    if !canonical_uuid_id(revision_id, "plan-revision") {
        return Err("Select an accepted native plan revision.".to_owned());
    }
    records
        .iter()
        .find(|record| record.revision.revision_id == revision_id)
        .ok_or_else(|| {
            "The accepted plan revision is unavailable. Existing history was preserved.".to_owned()
        })
}

fn revision_detail(
    record: &RevisionFile,
    is_head: bool,
) -> Result<RepositoryPlanRevisionDetail, String> {
    Ok(RepositoryPlanRevisionDetail {
        revision: record.revision.clone(),
        plan: drafts::record_detail(&record.record)?,
        is_head,
    })
}

fn history_directory(
    storage: &Path,
    project_id: &str,
    create: bool,
) -> Result<Option<PathBuf>, String> {
    drafts::project_directory(storage, project_id, "repository-plan-history", create)
}

fn revision_filename(revision: &RepositoryPlanRevision) -> Result<String, String> {
    if !canonical_uuid_id(&revision.history_id, "plan-history") || revision.revision_number == 0 {
        return Err("The accepted plan destination is invalid.".to_owned());
    }
    Ok(format!(
        "{}.{:010}.json",
        revision
            .history_id
            .strip_prefix("plan-history:")
            .ok_or("The plan history identity is invalid.")?,
        revision.revision_number
    ))
}

fn publish(directory: &Path, target: &Path, bytes: &[u8]) -> Result<(), String> {
    let stage = directory.join(format!(".plan-revision-{}.tmp", Uuid::new_v4()));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&stage).map_err(
        |_| "The accepted plan could not be staged. The original draft and history were preserved.",
    )?;
    let cleanup = StagedFile(stage);
    file.write_all(bytes).and_then(|_| file.sync_all()).map_err(|_| "The accepted plan could not be written completely. The original draft and history were preserved.")?;
    fs::hard_link(&cleanup.0, target).map_err(|_| "The accepted revision could not be published without overwriting history. Another review may have advanced this plan; refresh accepted history before retrying.")?;
    drop(file);
    drop(cleanup);
    File::open(directory).and_then(|directory| directory.sync_all()).map_err(|_| "The accepted revision may already exist, but directory durability could not be confirmed. Refresh accepted history before retrying; the original draft was preserved.".to_owned())
}

#[cfg(unix)]
fn lock_history(directory: &Path) -> Result<File, String> {
    use std::os::{fd::AsRawFd, unix::fs::OpenOptionsExt};
    let file = OpenOptions::new().read(true).write(true).create(true).truncate(false)
        .mode(0o600).custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(directory.join(".acceptance.lock"))
        .map_err(|_| "The accepted plan writer lock could not be opened. Existing history was preserved.")?;
    if !file
        .metadata()
        .map_err(|_| "The accepted plan writer lock could not be inspected.")?
        .is_file()
    {
        return Err("The accepted plan writer lock is not a regular local file. Existing data was preserved.".to_owned());
    }
    // SAFETY: file owns a live descriptor for the duration of this call and
    // the returned File guard. flock(int, int) borrows that descriptor, takes
    // no pointers, transfers no ownership and cannot unwind through Rust.
    // LOCK_NB prevents blocking; closing the guard releases this advisory lock.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
        return Err(if std::io::Error::last_os_error().kind() == std::io::ErrorKind::WouldBlock {
            "Another native review owns accepted plan history. Refresh and retry after it finishes; no acceptance was written."
        } else {
            "The local filesystem writer lock could not be acquired. Existing history was preserved; use native storage that supports advisory locks."
        }.to_owned());
    }
    Ok(file)
}

#[cfg(not(unix))]
fn lock_history(_directory: &Path) -> Result<File, String> {
    Err("Accepted plan writing requires the macOS/Unix native writer lock. Existing history was preserved.".to_owned())
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn now() -> Result<u64, String> {
    let time: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "The local clock is unavailable.")?
        .as_millis()
        .try_into()
        .map_err(|_| "The local clock is out of range.")?;
    if time == 0 || time > (1_u64 << 53) - 1 {
        return Err("The local clock is out of range.".to_owned());
    }
    Ok(time)
}
struct StagedFile(PathBuf);
impl Drop for StagedFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
