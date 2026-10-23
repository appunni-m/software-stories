//! Durable lifecycle receipts for progressive workspace answer jobs.
//!
//! These records contain only local job identity, provider thread/turn IDs and
//! lifecycle state. They never contain prompts, source, credentials or a
//! resumable provider session.

use crate::{codex_agent, evidence_ledger::sections::Outcome};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::HashMap;
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const STORE_SCHEMA_VERSION: u32 = 1;
const MAX_STORE_BYTES: usize = 1024 * 1024;
const MAX_JOB_COUNT: usize = 512;
const MAX_TURN_IDS: usize = 64;
static STORE: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum JobState {
    Preparing,
    Running,
    StopRequested,
    Completed,
    PartiallyCompleted,
    Stopped,
    Failed,
    Interrupted,
}

impl JobState {
    fn is_active(&self) -> bool {
        matches!(self, Self::Preparing | Self::Running | Self::StopRequested)
    }

    fn is_terminal(&self) -> bool {
        !self.is_active()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct JobRecord {
    pub(crate) job_id: String,
    pub(crate) project_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) state: JobState,
    pub(crate) worker_registered: bool,
    pub(crate) provider_thread_id: Option<String>,
    pub(crate) provider_turn_ids: Vec<String>,
    pub(crate) created_at_unix_ms: u64,
    pub(crate) updated_at_unix_ms: u64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StoreFile {
    schema_version: u32,
    jobs: Vec<JobRecord>,
}

pub(crate) fn start(
    storage: &Path,
    job_id: &str,
    project_id: &str,
    snapshot_id: &str,
) -> Result<JobRecord, String> {
    validate_identity(job_id, project_id, snapshot_id)?;
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    if let Some(existing_index) = store.jobs.iter().position(|job| job.job_id == job_id) {
        let existing = &store.jobs[existing_index];
        if existing.project_id == project_id
            && existing.snapshot_id == snapshot_id
            && existing.state == JobState::StopRequested
            && !existing.worker_registered
            && existing.provider_thread_id.is_none()
            && existing.provider_turn_ids.is_empty()
        {
            // Stop may arrive before the worker has completed local preparation.
            // Preserve that durable request when the worker later registers.
            let job = &mut store.jobs[existing_index];
            job.worker_registered = true;
            job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
            let registered = job.clone();
            write_store(storage, &store)?;
            return Ok(registered);
        }
        return Err("This workspace generation job ID has already been used.".into());
    }
    if store.jobs.len() >= MAX_JOB_COUNT {
        return Err(
            "The local workspace job history is full. Existing job records were preserved.".into(),
        );
    }
    let now = now_unix_ms();
    let job = JobRecord {
        job_id: job_id.to_owned(),
        project_id: project_id.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        state: JobState::Preparing,
        worker_registered: true,
        provider_thread_id: None,
        provider_turn_ids: Vec::new(),
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
    };
    store.jobs.push(job.clone());
    write_store(storage, &store)?;
    Ok(job)
}

pub(crate) fn get(storage: &Path, job_id: &str) -> Result<JobRecord, String> {
    validate_job_id(job_id)?;
    let _guard = lock()?;
    read_store(storage)?
        .jobs
        .into_iter()
        .find(|job| job.job_id == job_id)
        .ok_or_else(|| "This workspace generation job is no longer available.".into())
}

pub(crate) fn list(storage: &Path) -> Result<Vec<JobRecord>, String> {
    let _guard = lock()?;
    Ok(read_store(storage)?.jobs)
}

/// Persist provider identifiers from native progress before that progress is
/// forwarded to the renderer. A late callback cannot revive a terminal job.
pub(crate) fn record_progress(
    storage: &Path,
    job_id: &str,
    thread_id: Option<&str>,
    turn_id: Option<&str>,
) -> Result<JobRecord, String> {
    validate_job_id(job_id)?;
    if thread_id.is_some_and(|value| !valid_text(value, 256))
        || turn_id.is_some_and(|value| !valid_text(value, 256))
    {
        return Err("The workspace job provider identifiers are invalid.".into());
    }
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let job = find_job_mut(&mut store, job_id)?;
    if job.state.is_terminal() {
        return Ok(job.clone());
    }
    if let Some(thread_id) = thread_id {
        if job
            .provider_thread_id
            .as_deref()
            .is_some_and(|saved| saved != thread_id)
        {
            return Err(
                "The workspace job changed provider threads. Its existing record was preserved."
                    .into(),
            );
        }
        job.provider_thread_id = Some(thread_id.to_owned());
    }
    if let Some(turn_id) = turn_id {
        if !job.provider_turn_ids.iter().any(|saved| saved == turn_id) {
            if job.provider_turn_ids.len() >= MAX_TURN_IDS {
                return Err("The workspace job exceeded its local provider turn limit.".into());
            }
            job.provider_turn_ids.push(turn_id.to_owned());
        }
    }
    if job.state == JobState::Preparing {
        job.state = JobState::Running;
    }
    job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
    let updated = job.clone();
    write_store(storage, &store)?;
    Ok(updated)
}

/// Record Stop before delivering cancellation to the active run owner.
pub(crate) fn request_stop(storage: &Path, job_id: &str) -> Result<JobRecord, String> {
    validate_job_id(job_id)?;
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let job = find_job_mut(&mut store, job_id)?;
    if job.state.is_terminal() {
        return Err("This workspace generation job has already finished.".into());
    }
    if job.state != JobState::StopRequested {
        job.state = JobState::StopRequested;
        job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
    }
    let updated = job.clone();
    write_store(storage, &store)?;
    Ok(updated)
}

/// Persist a Stop request even when local request preparation has not yet
/// registered the job. The worker later adopts this exact scoped tombstone.
pub(crate) fn request_stop_or_record(
    storage: &Path,
    job_id: &str,
    project_id: &str,
    snapshot_id: &str,
) -> Result<JobRecord, String> {
    validate_identity(job_id, project_id, snapshot_id)?;
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    if let Some(job) = store.jobs.iter_mut().find(|job| job.job_id == job_id) {
        if job.project_id != project_id || job.snapshot_id != snapshot_id {
            return Err(
                "This workspace job belongs to another snapshot. Its record was preserved.".into(),
            );
        }
        if job.state.is_terminal() {
            return Err("This workspace generation job has already finished.".into());
        }
        if job.state != JobState::StopRequested {
            job.state = JobState::StopRequested;
            job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
        }
        let updated = job.clone();
        write_store(storage, &store)?;
        return Ok(updated);
    }
    if store.jobs.len() >= MAX_JOB_COUNT {
        return Err(
            "The local workspace job history is full. Existing job records were preserved.".into(),
        );
    }
    let now = now_unix_ms();
    let job = JobRecord {
        job_id: job_id.to_owned(),
        project_id: project_id.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        state: JobState::StopRequested,
        worker_registered: false,
        provider_thread_id: None,
        provider_turn_ids: Vec::new(),
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
    };
    store.jobs.push(job.clone());
    write_store(storage, &store)?;
    Ok(job)
}

pub(crate) fn finish(storage: &Path, job_id: &str, state: JobState) -> Result<JobRecord, String> {
    validate_job_id(job_id)?;
    if !state.is_terminal() {
        return Err("A workspace generation job can only finish in a terminal state.".into());
    }
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let job = find_job_mut(&mut store, job_id)?;
    if job.state.is_active() {
        job.state = state;
        job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
        let updated = job.clone();
        write_store(storage, &store)?;
        return Ok(updated);
    }
    Ok(job.clone())
}

/// Reconcile records that lost their in-process worker. A validated saved
/// answer outcome takes precedence over a crash between answer publication and
/// the corresponding job-state write.
#[cfg(test)]
pub(crate) fn reconcile(
    storage: &Path,
    active_job_ids: &HashSet<String>,
    answer_outcomes: &HashMap<String, Outcome>,
) -> Result<Vec<JobRecord>, String> {
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let mut changed = false;
    for job in &mut store.jobs {
        if !job.state.is_active() || active_job_ids.contains(&job.job_id) {
            continue;
        }
        job.state = match answer_outcomes.get(&job.job_id) {
            Some(Outcome::Complete) => JobState::Completed,
            Some(Outcome::Partial) => JobState::PartiallyCompleted,
            Some(Outcome::Stopped) => JobState::Stopped,
            Some(Outcome::Failed) => JobState::Failed,
            Some(Outcome::Working | Outcome::Interrupted) | None => JobState::Interrupted,
        };
        job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
        changed = true;
    }
    if changed {
        write_store(storage, &store)?;
    }
    Ok(store.jobs)
}

/// Reconcile just one job after checking only its own active worker and answer
/// receipt. Listing jobs must not replay every saved answer to discover stale
/// records, since each replay independently validates a full snapshot index.
pub(crate) fn reconcile_one(
    storage: &Path,
    job_id: &str,
    worker_active: bool,
    answer_outcome: Option<Outcome>,
) -> Result<JobRecord, String> {
    validate_job_id(job_id)?;
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let job = find_job_mut(&mut store, job_id)?;
    if !job.state.is_active() || worker_active {
        return Ok(job.clone());
    }
    job.state = match answer_outcome {
        Some(Outcome::Complete) => JobState::Completed,
        Some(Outcome::Partial) => JobState::PartiallyCompleted,
        Some(Outcome::Stopped) => JobState::Stopped,
        Some(Outcome::Failed) => JobState::Failed,
        Some(Outcome::Working | Outcome::Interrupted) | None => JobState::Interrupted,
    };
    job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
    let reconciled = job.clone();
    write_store(storage, &store)?;
    Ok(reconciled)
}

fn find_job_mut<'a>(store: &'a mut StoreFile, job_id: &str) -> Result<&'a mut JobRecord, String> {
    store
        .jobs
        .iter_mut()
        .find(|job| job.job_id == job_id)
        .ok_or_else(|| "This workspace generation job is no longer available.".into())
}

fn validate_job(job: &JobRecord) -> Result<(), String> {
    validate_identity(&job.job_id, &job.project_id, &job.snapshot_id)?;
    if job
        .provider_thread_id
        .as_deref()
        .is_some_and(|value| !valid_text(value, 256))
        || job.provider_turn_ids.len() > MAX_TURN_IDS
        || job
            .provider_turn_ids
            .iter()
            .any(|value| !valid_text(value, 256))
        || job.provider_turn_ids.iter().collect::<HashSet<_>>().len() != job.provider_turn_ids.len()
        || job.updated_at_unix_ms < job.created_at_unix_ms
    {
        return Err("The saved workspace job record is invalid.".into());
    }
    Ok(())
}

fn validate_identity(job_id: &str, project_id: &str, snapshot_id: &str) -> Result<(), String> {
    validate_job_id(job_id)?;
    if !valid_text(project_id, 4096) || !valid_text(snapshot_id, 256) {
        return Err("The workspace generation job scope is invalid.".into());
    }
    Ok(())
}

fn validate_job_id(job_id: &str) -> Result<(), String> {
    if Uuid::parse_str(job_id).is_err() {
        return Err("The workspace generation job ID is invalid.".into());
    }
    Ok(())
}

fn valid_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
}

fn store_path(storage: &Path) -> PathBuf {
    storage.join("workspace-generation-jobs.json")
}

fn read_store(storage: &Path) -> Result<StoreFile, String> {
    let path = store_path(storage);
    let bytes = match crate::bounded_file::read(&path, MAX_STORE_BYTES) {
        Ok(bytes) => bytes,
        Err(crate::bounded_file::ReadError::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(empty_store());
        }
        Err(crate::bounded_file::ReadError::Io(error)) => {
            return Err(format!(
                "Workspace generation jobs could not be read: {error}"
            ));
        }
        Err(crate::bounded_file::ReadError::TooLarge) => {
            return Err("Workspace generation job data exceeds its local storage limit.".into());
        }
    };
    let store: StoreFile = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Workspace generation job data is invalid: {error}"))?;
    if store.schema_version != STORE_SCHEMA_VERSION || store.jobs.len() > MAX_JOB_COUNT {
        return Err(
            "Workspace generation job data uses an unsupported or oversized format.".into(),
        );
    }
    let mut ids = HashSet::new();
    for job in &store.jobs {
        validate_job(job)?;
        if !ids.insert(&job.job_id) {
            return Err("Workspace generation job data contains duplicate job IDs.".into());
        }
    }
    Ok(store)
}

fn empty_store() -> StoreFile {
    StoreFile {
        schema_version: STORE_SCHEMA_VERSION,
        jobs: Vec::new(),
    }
}

fn write_store(storage: &Path, store: &StoreFile) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(store)
        .map_err(|_| "Workspace generation jobs could not be encoded.")?;
    if bytes.len() > MAX_STORE_BYTES {
        return Err("Workspace generation jobs would exceed the local storage limit.".into());
    }
    fs::create_dir_all(storage)
        .map_err(|_| "The local workspace generation folder could not be created.")?;
    codex_agent::atomic_private_write(&store_path(storage), &bytes)
        .map_err(|_| "Workspace generation jobs could not be saved.".into())
}

fn lock() -> Result<std::sync::MutexGuard<'static, ()>, String> {
    STORE
        .lock()
        .map_err(|_| "Workspace generation job storage is temporarily unavailable.".into())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("workspace-generation-jobs-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn start_job(directory: &TestDirectory) -> JobRecord {
        start(
            &directory.0,
            &Uuid::new_v4().to_string(),
            "project-a",
            "snapshot-a",
        )
        .unwrap()
    }

    #[test]
    fn provider_ids_and_terminal_state_survive_reopen() {
        let directory = TestDirectory::new();
        let job = start_job(&directory);
        record_progress(&directory.0, &job.job_id, Some("thread-a"), Some("turn-a")).unwrap();
        record_progress(&directory.0, &job.job_id, Some("thread-a"), Some("turn-b")).unwrap();
        let complete = finish(&directory.0, &job.job_id, JobState::Completed).unwrap();
        let reopened = get(&directory.0, &job.job_id).unwrap();
        assert_eq!(complete, reopened);
        assert_eq!(reopened.provider_thread_id.as_deref(), Some("thread-a"));
        assert_eq!(
            reopened.provider_turn_ids,
            vec!["turn-a".to_owned(), "turn-b".to_owned()]
        );
        assert_eq!(reopened.state, JobState::Completed);
    }

    #[test]
    fn reconciliation_marks_only_inactive_jobs_and_uses_saved_answer_outcome() {
        let directory = TestDirectory::new();
        let interrupted = start_job(&directory);
        let completed = start_job(&directory);
        let stopped = start_job(&directory);
        let active_ids = HashSet::from([stopped.job_id.clone()]);
        let answer_outcomes = HashMap::from([
            (completed.job_id.clone(), Outcome::Complete),
            (stopped.job_id.clone(), Outcome::Stopped),
        ]);

        let records = reconcile(&directory.0, &active_ids, &answer_outcomes).unwrap();
        assert_eq!(
            records
                .iter()
                .find(|job| job.job_id == interrupted.job_id)
                .unwrap()
                .state,
            JobState::Interrupted
        );
        assert_eq!(
            records
                .iter()
                .find(|job| job.job_id == completed.job_id)
                .unwrap()
                .state,
            JobState::Completed
        );
        assert_eq!(
            records
                .iter()
                .find(|job| job.job_id == stopped.job_id)
                .unwrap()
                .state,
            JobState::Preparing
        );
    }

    #[test]
    fn one_job_reconciliation_never_changes_unrelated_records() {
        let directory = TestDirectory::new();
        let partial = start_job(&directory);
        let unrelated = start_job(&directory);

        assert_eq!(
            reconcile_one(&directory.0, &partial.job_id, false, Some(Outcome::Partial))
                .unwrap()
                .state,
            JobState::PartiallyCompleted
        );
        assert_eq!(
            get(&directory.0, &unrelated.job_id).unwrap().state,
            JobState::Preparing
        );
    }

    #[test]
    fn stop_request_is_durable_and_terminal_jobs_cannot_be_stopped() {
        let directory = TestDirectory::new();
        let job = start_job(&directory);
        assert_eq!(
            request_stop(&directory.0, &job.job_id).unwrap().state,
            JobState::StopRequested
        );
        assert_eq!(
            get(&directory.0, &job.job_id).unwrap().state,
            JobState::StopRequested
        );
        finish(&directory.0, &job.job_id, JobState::Stopped).unwrap();
        assert!(request_stop(&directory.0, &job.job_id).is_err());
    }

    #[test]
    fn stop_before_worker_registration_is_adopted_without_replaying_the_job() {
        let directory = TestDirectory::new();
        let job_id = Uuid::new_v4().to_string();
        let stopped =
            request_stop_or_record(&directory.0, &job_id, "project-a", "snapshot-a").unwrap();
        assert_eq!(stopped.state, JobState::StopRequested);
        let adopted = start(&directory.0, &job_id, "project-a", "snapshot-a").unwrap();
        assert_eq!(adopted.state, stopped.state);
        assert!(adopted.worker_registered);
        assert!(start(&directory.0, &job_id, "project-a", "snapshot-a").is_err());
    }

    #[test]
    fn malformed_job_store_is_preserved_on_read() {
        let directory = TestDirectory::new();
        let original = br#"{"schemaVersion":99,"jobs":[]}"#;
        fs::write(store_path(&directory.0), original).unwrap();
        assert!(list(&directory.0).is_err());
        assert_eq!(fs::read(store_path(&directory.0)).unwrap(), original);
    }
}
