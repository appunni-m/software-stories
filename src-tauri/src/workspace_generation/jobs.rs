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

const STORE_SCHEMA_VERSION: u32 = 3;
const MAX_STORE_BYTES: usize = 1024 * 1024;
const MAX_JOB_COUNT: usize = 512;
const MAX_TURN_IDS: usize = 64;
static STORE: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum JobState {
    Queued,
    Preparing,
    Running,
    StopRequested,
    Completed,
    PartiallyCompleted,
    Stopped,
    Failed,
    Interrupted,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum JobKind {
    WorkspaceQuestion,
    RepositoryGuideSlide,
}

impl JobState {
    fn is_active(&self) -> bool {
        matches!(
            self,
            Self::Queued | Self::Preparing | Self::Running | Self::StopRequested
        )
    }

    fn is_terminal(&self) -> bool {
        !self.is_active()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct JobRecord {
    pub(crate) job_id: String,
    pub(crate) kind: Option<JobKind>,
    pub(crate) conversation_turn_id: Option<String>,
    /// Ephemeral WebView owner for a queued renderer operation. A new renderer
    /// session can recover it as Interrupted without replaying its request.
    pub(crate) renderer_session_id: Option<String>,
    pub(crate) project_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) model_id: Option<String>,
    pub(crate) reasoning_effort: Option<String>,
    /// Hashes bind the exact captured source, reviewed request and observed
    /// Codex account/runtime without retaining source, prompts, paths or email.
    pub(crate) source_fingerprint_sha256: Option<String>,
    pub(crate) request_fingerprint_sha256: Option<String>,
    pub(crate) runtime_identity_sha256: Option<String>,
    pub(crate) state: JobState,
    pub(crate) worker_registered: bool,
    pub(crate) provider_thread_id: Option<String>,
    pub(crate) provider_turn_ids: Vec<String>,
    /// Fixed, source-free stage measurements for failures before provider
    /// dispatch. Successful runs keep these checkpoints in the timing trace.
    pub(crate) preparation_stages: Vec<crate::generation_timing::PreparationStage>,
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
    kind: JobKind,
    conversation_turn_id: Option<&str>,
    project_id: &str,
    snapshot_id: &str,
    model_id: &str,
    reasoning_effort: &str,
) -> Result<JobRecord, String> {
    validate_identity(job_id, project_id, snapshot_id)?;
    validate_requested_identity(kind, conversation_turn_id, model_id, reasoning_effort)?;
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    if let Some(existing_index) = store.jobs.iter().position(|job| job.job_id == job_id) {
        let existing = &store.jobs[existing_index];
        if existing.project_id == project_id
            && existing.snapshot_id == snapshot_id
            && matches!(existing.state, JobState::Queued | JobState::StopRequested)
            && !existing.worker_registered
            && existing.provider_thread_id.is_none()
            && existing.provider_turn_ids.is_empty()
        {
            // Stop may arrive before the worker has completed local preparation.
            // Preserve that durable request when the worker later registers.
            let job = &mut store.jobs[existing_index];
            bind_requested_identity(job, kind, conversation_turn_id, model_id, reasoning_effort)?;
            job.worker_registered = true;
            job.renderer_session_id = None;
            if job.state == JobState::Queued {
                job.state = JobState::Preparing;
            }
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
        kind: Some(kind),
        conversation_turn_id: conversation_turn_id.map(str::to_owned),
        renderer_session_id: None,
        project_id: project_id.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        model_id: Some(model_id.to_owned()),
        reasoning_effort: Some(reasoning_effort.to_owned()),
        source_fingerprint_sha256: None,
        request_fingerprint_sha256: None,
        runtime_identity_sha256: None,
        state: JobState::Preparing,
        worker_registered: true,
        provider_thread_id: None,
        provider_turn_ids: Vec::new(),
        preparation_stages: Vec::new(),
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
    };
    store.jobs.push(job.clone());
    write_store(storage, &store)?;
    Ok(job)
}

/// Persist a question before it waits for the renderer-owned work queue. This
/// records intent and configuration only; it contains no question or source.
pub(crate) fn queue_workspace_question(
    storage: &Path,
    job_id: &str,
    conversation_turn_id: &str,
    renderer_session_id: &str,
    project_id: &str,
    snapshot_id: &str,
    model_id: &str,
    reasoning_effort: &str,
) -> Result<JobRecord, String> {
    validate_identity(job_id, project_id, snapshot_id)?;
    validate_requested_identity(
        JobKind::WorkspaceQuestion,
        Some(conversation_turn_id),
        model_id,
        reasoning_effort,
    )?;
    if Uuid::parse_str(renderer_session_id).is_err() {
        return Err("The workspace renderer session identity is invalid.".into());
    }
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    if let Some(index) = store.jobs.iter().position(|job| job.job_id == job_id) {
        let existing = &store.jobs[index];
        if existing.project_id != project_id || existing.snapshot_id != snapshot_id {
            return Err("This workspace generation job belongs to another snapshot.".into());
        }
        if existing.state == JobState::Queued
            && !existing.worker_registered
            && existing.renderer_session_id.as_deref() == Some(renderer_session_id)
        {
            let job = &store.jobs[index];
            if job.kind == Some(JobKind::WorkspaceQuestion)
                && job.conversation_turn_id.as_deref() == Some(conversation_turn_id)
                && job.model_id.as_deref() == Some(model_id)
                && job.reasoning_effort.as_deref() == Some(reasoning_effort)
            {
                return Ok(job.clone());
            }
            return Err("This queued workspace job is already bound to another question.".into());
        }
        if existing.state == JobState::StopRequested
            && !existing.worker_registered
            && existing.provider_thread_id.is_none()
            && existing.provider_turn_ids.is_empty()
        {
            let job = &mut store.jobs[index];
            bind_requested_identity(
                job,
                JobKind::WorkspaceQuestion,
                Some(conversation_turn_id),
                model_id,
                reasoning_effort,
            )?;
            job.renderer_session_id = Some(renderer_session_id.to_owned());
            job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
            let stopped = job.clone();
            write_store(storage, &store)?;
            return Ok(stopped);
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
        kind: Some(JobKind::WorkspaceQuestion),
        conversation_turn_id: Some(conversation_turn_id.to_owned()),
        renderer_session_id: Some(renderer_session_id.to_owned()),
        project_id: project_id.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        model_id: Some(model_id.to_owned()),
        reasoning_effort: Some(reasoning_effort.to_owned()),
        source_fingerprint_sha256: None,
        request_fingerprint_sha256: None,
        runtime_identity_sha256: None,
        state: JobState::Queued,
        worker_registered: false,
        provider_thread_id: None,
        provider_turn_ids: Vec::new(),
        preparation_stages: Vec::new(),
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
    };
    store.jobs.push(job.clone());
    write_store(storage, &store)?;
    Ok(job)
}

/// Bind the immutable source, request and observed account/runtime after local
/// request preparation but before the first provider dispatch.
pub(crate) fn bind_reviewed_request(
    storage: &Path,
    job_id: &str,
    source_fingerprint_sha256: &str,
    request_fingerprint_sha256: &str,
    runtime_identity_sha256: &str,
) -> Result<JobRecord, String> {
    validate_job_id(job_id)?;
    for digest in [
        source_fingerprint_sha256,
        request_fingerprint_sha256,
        runtime_identity_sha256,
    ] {
        if !valid_digest(digest) {
            return Err("The workspace job request identity is invalid.".into());
        }
    }
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let job = find_job_mut(&mut store, job_id)?;
    if job.model_id.is_none() || job.reasoning_effort.is_none() {
        return Err("The workspace job is missing its original question settings.".into());
    }
    let bound = [
        &job.source_fingerprint_sha256,
        &job.request_fingerprint_sha256,
        &job.runtime_identity_sha256,
    ];
    let proposed = [
        source_fingerprint_sha256,
        request_fingerprint_sha256,
        runtime_identity_sha256,
    ];
    if bound.iter().any(|saved| saved.is_some()) {
        if bound
            .iter()
            .zip(proposed)
            .all(|(saved, proposed)| saved.as_deref() == Some(proposed))
        {
            return Ok(job.clone());
        }
        return Err("This workspace job is already bound to another reviewed request.".into());
    }
    if job.state.is_terminal() {
        return Err("A finished workspace job cannot be bound to a new request.".into());
    }
    job.source_fingerprint_sha256 = Some(source_fingerprint_sha256.to_owned());
    job.request_fingerprint_sha256 = Some(request_fingerprint_sha256.to_owned());
    job.runtime_identity_sha256 = Some(runtime_identity_sha256.to_owned());
    job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
    let updated = job.clone();
    write_store(storage, &store)?;
    Ok(updated)
}

fn bind_requested_identity(
    job: &mut JobRecord,
    kind: JobKind,
    conversation_turn_id: Option<&str>,
    model_id: &str,
    reasoning_effort: &str,
) -> Result<(), String> {
    let matches = job.kind.is_none_or(|saved| saved == kind)
        && job
            .conversation_turn_id
            .as_deref()
            .is_none_or(|saved| conversation_turn_id == Some(saved))
        && job
            .model_id
            .as_deref()
            .is_none_or(|saved| saved == model_id)
        && job
            .reasoning_effort
            .as_deref()
            .is_none_or(|saved| saved == reasoning_effort);
    if !matches {
        return Err("This workspace job is already bound to another question.".into());
    }
    job.kind = Some(kind);
    if let Some(conversation_turn_id) = conversation_turn_id {
        job.conversation_turn_id = Some(conversation_turn_id.to_owned());
    }
    job.model_id = Some(model_id.to_owned());
    job.reasoning_effort = Some(reasoning_effort.to_owned());
    Ok(())
}

fn validate_requested_identity(
    kind: JobKind,
    conversation_turn_id: Option<&str>,
    model_id: &str,
    reasoning_effort: &str,
) -> Result<(), String> {
    if (kind == JobKind::WorkspaceQuestion && conversation_turn_id.is_none())
        || conversation_turn_id.is_some_and(|id| Uuid::parse_str(id).is_err())
        || !valid_text(model_id, 200)
        || !valid_text(reasoning_effort, 32)
    {
        return Err("The workspace question identity or Codex settings are invalid.".into());
    }
    Ok(())
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

pub(crate) fn list_for_scope(
    storage: &Path,
    project_id: &str,
    snapshot_id: &str,
) -> Result<Vec<JobRecord>, String> {
    if !valid_text(project_id, 4096) || !valid_text(snapshot_id, 256) {
        return Err("The workspace job scope is invalid.".into());
    }
    let _guard = lock()?;
    Ok(read_store(storage)?
        .jobs
        .into_iter()
        .filter(|job| job.project_id == project_id && job.snapshot_id == snapshot_id)
        .collect())
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
        job.state = if job.state == JobState::Queued && !job.worker_registered {
            JobState::Stopped
        } else {
            JobState::StopRequested
        };
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
            job.state = if job.state == JobState::Queued && !job.worker_registered {
                JobState::Stopped
            } else {
                JobState::StopRequested
            };
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
        kind: None,
        conversation_turn_id: None,
        renderer_session_id: None,
        project_id: project_id.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        model_id: None,
        reasoning_effort: None,
        source_fingerprint_sha256: None,
        request_fingerprint_sha256: None,
        runtime_identity_sha256: None,
        state: JobState::StopRequested,
        worker_registered: false,
        provider_thread_id: None,
        provider_turn_ids: Vec::new(),
        preparation_stages: Vec::new(),
        created_at_unix_ms: now,
        updated_at_unix_ms: now,
    };
    store.jobs.push(job.clone());
    write_store(storage, &store)?;
    Ok(job)
}

/// Finish a question that never entered the native worker. This is used when
/// connection or conversation preparation fails while the job is still queued.
pub(crate) fn finish_unstarted(
    storage: &Path,
    job_id: &str,
    project_id: &str,
    snapshot_id: &str,
    state: JobState,
) -> Result<JobRecord, String> {
    validate_identity(job_id, project_id, snapshot_id)?;
    if !matches!(
        state,
        JobState::Stopped | JobState::Failed | JobState::Interrupted
    ) {
        return Err(
            "An unstarted workspace job must finish as stopped, failed or interrupted.".into(),
        );
    }
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let job = find_job_mut(&mut store, job_id)?;
    if job.project_id != project_id || job.snapshot_id != snapshot_id {
        return Err("This workspace job belongs to another snapshot.".into());
    }
    if job.state.is_terminal() {
        return Ok(job.clone());
    }
    if job.worker_registered || !matches!(job.state, JobState::Queued | JobState::StopRequested) {
        return Err("This workspace job already entered its native worker.".into());
    }
    job.state = if job.state == JobState::StopRequested {
        JobState::Stopped
    } else {
        state
    };
    job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
    let updated = job.clone();
    write_store(storage, &store)?;
    Ok(updated)
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

/// Preserve bounded preparation stages when a registered job fails before a
/// provider timing trace can be created. The existing job ID remains the only
/// correlation key, and stage names never include raw error or source text.
pub(crate) fn finish_preparation_failure(
    storage: &Path,
    job_id: &str,
    state: JobState,
    stages: Vec<crate::generation_timing::PreparationStage>,
) -> Result<JobRecord, String> {
    validate_job_id(job_id)?;
    if !matches!(state, JobState::Failed | JobState::Stopped) {
        return Err("A preparation failure must end in a failed or stopped state.".into());
    }
    crate::generation_timing::validate_preparation_stages(&stages)?;
    let expected = match state {
        JobState::Failed => crate::generation_timing::PreparationOutcome::Failed,
        JobState::Stopped => crate::generation_timing::PreparationOutcome::Stopped,
        _ => unreachable!("validated terminal preparation state"),
    };
    if stages.last().is_none_or(|stage| stage.outcome != expected) {
        return Err("The final preparation stage does not match its job outcome.".into());
    }
    if stages
        .iter()
        .take(stages.len().saturating_sub(1))
        .any(|stage| stage.outcome != crate::generation_timing::PreparationOutcome::Complete)
    {
        return Err("A preparation failure contains an earlier failed stage.".into());
    }
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let job = find_job_mut(&mut store, job_id)?;
    if !job.state.is_active() {
        return Ok(job.clone());
    }
    job.state = state;
    job.preparation_stages = stages;
    job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
    let updated = job.clone();
    write_store(storage, &store)?;
    Ok(updated)
}

/// Keep successful local preparation checkpoints on the durable job too. This
/// is a best-effort fallback if trace creation fails before dispatch records
/// its own copy.
pub(crate) fn record_preparation_stages(
    storage: &Path,
    job_id: &str,
    stages: Vec<crate::generation_timing::PreparationStage>,
) -> Result<JobRecord, String> {
    validate_job_id(job_id)?;
    crate::generation_timing::validate_preparation_stages(&stages)?;
    if stages.is_empty()
        || stages
            .iter()
            .any(|stage| stage.outcome != crate::generation_timing::PreparationOutcome::Complete)
    {
        return Err("Only successful preparation checkpoints can be recorded here.".into());
    }
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let job = find_job_mut(&mut store, job_id)?;
    if !job.state.is_active() {
        return Ok(job.clone());
    }
    job.preparation_stages = stages;
    job.updated_at_unix_ms = now_unix_ms().max(job.created_at_unix_ms);
    let updated = job.clone();
    write_store(storage, &store)?;
    Ok(updated)
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
            Some(Outcome::Working | Outcome::Interrupted) | None => match job.state {
                JobState::StopRequested if !job.worker_registered => JobState::Stopped,
                _ => JobState::Interrupted,
            },
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
    reconcile_one_for_renderer(storage, job_id, None, worker_active, answer_outcome)
}

/// Reconcile a scoped job while preserving queued work owned by this live
/// renderer. Queued work from any earlier renderer is interruption-only.
pub(crate) fn reconcile_one_for_renderer(
    storage: &Path,
    job_id: &str,
    renderer_session_id: Option<&str>,
    worker_active: bool,
    answer_outcome: Option<Outcome>,
) -> Result<JobRecord, String> {
    validate_job_id(job_id)?;
    let _guard = lock()?;
    let mut store = read_store(storage)?;
    let job = find_job_mut(&mut store, job_id)?;
    if job.state == JobState::Queued
        && renderer_session_id
            .is_some_and(|session| job.renderer_session_id.as_deref() == Some(session))
    {
        return Ok(job.clone());
    }
    if !job.state.is_active() || worker_active {
        return Ok(job.clone());
    }
    job.state = match answer_outcome {
        Some(Outcome::Complete) => JobState::Completed,
        Some(Outcome::Partial) => JobState::PartiallyCompleted,
        Some(Outcome::Stopped) => JobState::Stopped,
        Some(Outcome::Failed) => JobState::Failed,
        Some(Outcome::Working | Outcome::Interrupted) | None => match job.state {
            JobState::StopRequested if !job.worker_registered => JobState::Stopped,
            _ => JobState::Interrupted,
        },
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
    if (job.kind == Some(JobKind::WorkspaceQuestion) && job.conversation_turn_id.is_none())
        || job
            .conversation_turn_id
            .as_deref()
            .is_some_and(|id| Uuid::parse_str(id).is_err())
        || job
            .renderer_session_id
            .as_deref()
            .is_some_and(|id| Uuid::parse_str(id).is_err())
        || job
            .model_id
            .as_deref()
            .is_some_and(|value| !valid_text(value, 200))
        || job
            .reasoning_effort
            .as_deref()
            .is_some_and(|value| !valid_text(value, 32))
        || [
            &job.source_fingerprint_sha256,
            &job.request_fingerprint_sha256,
            &job.runtime_identity_sha256,
        ]
        .iter()
        .any(|value| value.as_deref().is_some_and(|digest| !valid_digest(digest)))
    {
        return Err("The saved workspace job identity is invalid.".into());
    }
    let identity_hashes = [
        &job.source_fingerprint_sha256,
        &job.request_fingerprint_sha256,
        &job.runtime_identity_sha256,
    ];
    let saved_hash_count = identity_hashes
        .iter()
        .filter(|value| value.is_some())
        .count();
    if (saved_hash_count != 0 && saved_hash_count != identity_hashes.len())
        || (saved_hash_count == identity_hashes.len()
            && (job.conversation_turn_id.is_none()
                || job.model_id.is_none()
                || job.reasoning_effort.is_none()))
    {
        return Err("The saved workspace job identity is incomplete.".into());
    }
    if job.state == JobState::Queued
        && (job.worker_registered
            || job.kind != Some(JobKind::WorkspaceQuestion)
            || job.conversation_turn_id.is_none()
            || job.renderer_session_id.is_none()
            || saved_hash_count != 0
            || job.provider_thread_id.is_some()
            || !job.provider_turn_ids.is_empty())
    {
        return Err("The saved queued workspace job is invalid.".into());
    }
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
    crate::generation_timing::validate_preparation_stages(&job.preparation_stages)?;
    if job
        .preparation_stages
        .iter()
        .any(|stage| stage.outcome != crate::generation_timing::PreparationOutcome::Complete)
    {
        let state_matches = match (job.state, job.preparation_stages.last()) {
            (JobState::Failed, Some(stage)) => {
                stage.outcome == crate::generation_timing::PreparationOutcome::Failed
            }
            (JobState::Stopped, Some(stage)) => {
                stage.outcome == crate::generation_timing::PreparationOutcome::Stopped
            }
            _ => false,
        };
        let earlier_failure = job.preparation_stages[..job.preparation_stages.len() - 1]
            .iter()
            .any(|stage| stage.outcome != crate::generation_timing::PreparationOutcome::Complete);
        if !state_matches || earlier_failure {
            return Err("The saved workspace preparation failure is invalid.".into());
        }
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

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
    let value = crate::strict_json::from_slice(&bytes)
        .map_err(|error| format!("Workspace generation job data is invalid: {error}"))?;
    if value.get("schemaVersion").and_then(serde_json::Value::as_u64)
        != Some(u64::from(STORE_SCHEMA_VERSION))
    {
        return Err("Workspace generation job data uses an unsupported format.".into());
    }
    const REQUIRED_JOB_FIELDS: &[&str] = &[
        "jobId",
        "kind",
        "conversationTurnId",
        "rendererSessionId",
        "projectId",
        "snapshotId",
        "modelId",
        "reasoningEffort",
        "sourceFingerprintSha256",
        "requestFingerprintSha256",
        "runtimeIdentitySha256",
        "state",
        "workerRegistered",
        "providerThreadId",
        "providerTurnIds",
        "preparationStages",
        "createdAtUnixMs",
        "updatedAtUnixMs",
    ];
    let Some(jobs) = value.get("jobs").and_then(serde_json::Value::as_array) else {
        return Err("Workspace generation job data is invalid.".into());
    };
    if jobs.iter().any(|job| {
        !REQUIRED_JOB_FIELDS
            .iter()
            .all(|field| job.get(*field).is_some())
    }) {
        return Err("Workspace generation job data is incomplete.".into());
    }
    let store: StoreFile = serde_json::from_value(value)
        .map_err(|error| format!("Workspace generation job data is invalid: {error}"))?;
    if store.jobs.len() > MAX_JOB_COUNT {
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
        let job_id = Uuid::new_v4().to_string();
        let conversation_turn_id = Uuid::new_v4().to_string();
        start(
            &directory.0,
            &job_id,
            JobKind::WorkspaceQuestion,
            Some(&conversation_turn_id),
            "project-a",
            "snapshot-a",
            "gpt-6-luna",
            "max",
        )
        .unwrap()
    }

    #[test]
    fn reviewed_job_identity_survives_reopen_without_saving_source_or_account_text() {
        let directory = TestDirectory::new();
        let job = start_job(&directory);
        let source_fingerprint = "a".repeat(64);
        let request_fingerprint = "b".repeat(64);
        let runtime_fingerprint = "c".repeat(64);
        let saved = bind_reviewed_request(
            &directory.0,
            &job.job_id,
            &source_fingerprint,
            &request_fingerprint,
            &runtime_fingerprint,
        )
        .unwrap();
        assert_eq!(
            bind_reviewed_request(
                &directory.0,
                &job.job_id,
                &source_fingerprint,
                &request_fingerprint,
                &runtime_fingerprint,
            )
            .unwrap(),
            saved
        );
        assert!(
            bind_reviewed_request(
                &directory.0,
                &job.job_id,
                &"d".repeat(64),
                &"b".repeat(64),
                &"c".repeat(64),
            )
            .is_err()
        );

        let reopened = get(&directory.0, &job.job_id).unwrap();
        assert_eq!(reopened.conversation_turn_id, job.conversation_turn_id);
        assert_eq!(reopened.model_id.as_deref(), Some("gpt-6-luna"));
        assert_eq!(reopened.reasoning_effort.as_deref(), Some("max"));
        assert_eq!(
            reopened.source_fingerprint_sha256.as_deref(),
            Some(source_fingerprint.as_str())
        );
        assert_eq!(
            reopened.request_fingerprint_sha256.as_deref(),
            Some(request_fingerprint.as_str())
        );
        assert_eq!(
            reopened.runtime_identity_sha256.as_deref(),
            Some(runtime_fingerprint.as_str())
        );
        let contents = fs::read_to_string(store_path(&directory.0)).unwrap();
        assert!(!contents.contains("private/source.rs"));
        assert!(!contents.contains("person@example.com"));
        assert!(!contents.contains("secret source contents"));
    }

    #[test]
    fn prestart_stop_tombstone_adopts_only_the_original_question_identity() {
        let directory = TestDirectory::new();
        let job_id = Uuid::new_v4().to_string();
        let conversation_turn_id = Uuid::new_v4().to_string();
        request_stop_or_record(&directory.0, &job_id, "project-a", "snapshot-a").unwrap();
        let adopted = start(
            &directory.0,
            &job_id,
            JobKind::WorkspaceQuestion,
            Some(&conversation_turn_id),
            "project-a",
            "snapshot-a",
            "gpt-6-luna",
            "max",
        )
        .unwrap();
        assert_eq!(adopted.state, JobState::StopRequested);
        assert!(adopted.worker_registered);
        assert!(
            start(
                &directory.0,
                &job_id,
                JobKind::WorkspaceQuestion,
                Some(&conversation_turn_id),
                "project-a",
                "snapshot-a",
                "gpt-6-luna",
                "max",
            )
            .is_err()
        );
    }

    #[test]
    fn terminal_job_cannot_be_bound_to_a_late_reviewed_request() {
        let directory = TestDirectory::new();
        let job = start_job(&directory);
        finish(&directory.0, &job.job_id, JobState::Interrupted).unwrap();
        assert!(
            bind_reviewed_request(
                &directory.0,
                &job.job_id,
                &"a".repeat(64),
                &"b".repeat(64),
                &"c".repeat(64),
            )
            .is_err()
        );
    }

    #[test]
    fn version_one_records_are_rejected_without_rewriting() {
        let directory = TestDirectory::new();
        let job_id = Uuid::new_v4().to_string();
        let legacy = serde_json::json!({"schemaVersion":1,"jobs":[{"jobId":job_id,
            "projectId":"project-a","snapshotId":"snapshot-a","state":"running",
            "workerRegistered":true,"providerThreadId":"thread-a","providerTurnIds":["turn-a"],
            "preparationStages":[],"createdAtUnixMs":1,"updatedAtUnixMs":2}]});
        fs::write(
            store_path(&directory.0),
            serde_json::to_vec(&legacy).unwrap(),
        )
        .unwrap();
        let original = fs::read(store_path(&directory.0)).unwrap();

        assert!(get(&directory.0, legacy["jobs"][0]["jobId"].as_str().unwrap()).is_err());
        assert_eq!(fs::read(store_path(&directory.0)).unwrap(), original);
    }

    #[test]
    fn version_two_records_are_rejected_without_rewriting() {
        let directory = TestDirectory::new();
        let job_id = Uuid::new_v4().to_string();
        let turn_id = Uuid::new_v4().to_string();
        let legacy = serde_json::json!({"schemaVersion":2,"jobs":[{"jobId":job_id,
            "kind":"workspaceQuestion","conversationTurnId":turn_id,
            "projectId":"project-a","snapshotId":"snapshot-a","modelId":"gpt-6-luna",
            "reasoningEffort":"max","sourceFingerprintSha256":null,
            "requestFingerprintSha256":null,"runtimeIdentitySha256":null,"state":"running",
            "workerRegistered":true,"providerThreadId":"thread-a","providerTurnIds":["turn-a"],
            "preparationStages":[],"createdAtUnixMs":1,"updatedAtUnixMs":2}]});
        fs::write(
            store_path(&directory.0),
            serde_json::to_vec(&legacy).unwrap(),
        )
        .unwrap();
        let original = fs::read(store_path(&directory.0)).unwrap();

        assert!(get(&directory.0, &job_id).is_err());
        assert_eq!(fs::read(store_path(&directory.0)).unwrap(), original);
    }

    #[test]
    fn scoped_job_reads_do_not_return_another_snapshot() {
        let directory = TestDirectory::new();
        let first = start_job(&directory);
        start(
            &directory.0,
            &Uuid::new_v4().to_string(),
            JobKind::WorkspaceQuestion,
            Some(&Uuid::new_v4().to_string()),
            "project-b",
            "snapshot-b",
            "gpt-6-luna",
            "max",
        )
        .unwrap();
        assert_eq!(
            list_for_scope(&directory.0, "project-a", "snapshot-a").unwrap(),
            vec![first]
        );
    }

    #[test]
    fn queued_question_is_durable_idempotent_and_contains_no_prompt_or_source() {
        let directory = TestDirectory::new();
        let job_id = Uuid::new_v4().to_string();
        let turn_id = Uuid::new_v4().to_string();
        let renderer_id = Uuid::new_v4().to_string();
        let queued = queue_workspace_question(
            &directory.0,
            &job_id,
            &turn_id,
            &renderer_id,
            "project-a",
            "snapshot-a",
            "gpt-6-luna",
            "max",
        )
        .unwrap();
        assert_eq!(queued.state, JobState::Queued);
        assert!(!queued.worker_registered);
        assert_eq!(
            queued.renderer_session_id.as_deref(),
            Some(renderer_id.as_str())
        );
        assert_eq!(
            queue_workspace_question(
                &directory.0,
                &job_id,
                &turn_id,
                &renderer_id,
                "project-a",
                "snapshot-a",
                "gpt-6-luna",
                "max",
            )
            .unwrap(),
            queued
        );
        assert!(
            queue_workspace_question(
                &directory.0,
                &job_id,
                &turn_id,
                &Uuid::new_v4().to_string(),
                "project-a",
                "snapshot-a",
                "gpt-6-luna",
                "max",
            )
            .is_err()
        );
        let saved = fs::read_to_string(store_path(&directory.0)).unwrap();
        assert!(!saved.contains("question"));
        assert!(!saved.contains("source.rs"));
        assert!(!saved.contains("secret source"));
    }

    #[test]
    fn queued_question_is_adopted_by_its_native_worker_without_changing_identity() {
        let directory = TestDirectory::new();
        let job_id = Uuid::new_v4().to_string();
        let turn_id = Uuid::new_v4().to_string();
        queue_workspace_question(
            &directory.0,
            &job_id,
            &turn_id,
            &Uuid::new_v4().to_string(),
            "project-a",
            "snapshot-a",
            "gpt-6-luna",
            "max",
        )
        .unwrap();
        let started = start(
            &directory.0,
            &job_id,
            JobKind::WorkspaceQuestion,
            Some(&turn_id),
            "project-a",
            "snapshot-a",
            "gpt-6-luna",
            "max",
        )
        .unwrap();
        assert_eq!(started.state, JobState::Preparing);
        assert!(started.worker_registered);
        assert_eq!(started.renderer_session_id, None);
        assert_eq!(
            started.conversation_turn_id.as_deref(),
            Some(turn_id.as_str())
        );
    }

    #[test]
    fn queued_question_stop_and_renderer_reconciliation_never_replay_inference() {
        let directory = TestDirectory::new();
        let current_id = Uuid::new_v4().to_string();
        let turn_id = Uuid::new_v4().to_string();
        let renderer_id = Uuid::new_v4().to_string();
        queue_workspace_question(
            &directory.0,
            &current_id,
            &turn_id,
            &renderer_id,
            "project-a",
            "snapshot-a",
            "gpt-6-luna",
            "max",
        )
        .unwrap();
        assert_eq!(
            reconcile_one_for_renderer(&directory.0, &current_id, Some(&renderer_id), false, None,)
                .unwrap()
                .state,
            JobState::Queued
        );
        assert_eq!(
            reconcile_one_for_renderer(
                &directory.0,
                &current_id,
                Some(&Uuid::new_v4().to_string()),
                false,
                None,
            )
            .unwrap()
            .state,
            JobState::Interrupted
        );

        let stop_id = Uuid::new_v4().to_string();
        queue_workspace_question(
            &directory.0,
            &stop_id,
            &Uuid::new_v4().to_string(),
            &renderer_id,
            "project-a",
            "snapshot-a",
            "gpt-6-luna",
            "max",
        )
        .unwrap();
        assert_eq!(
            request_stop_or_record(&directory.0, &stop_id, "project-a", "snapshot-a")
                .unwrap()
                .state,
            JobState::Stopped
        );
        assert_eq!(
            finish_unstarted(
                &directory.0,
                &stop_id,
                "project-a",
                "snapshot-a",
                JobState::Failed,
            )
            .unwrap()
            .state,
            JobState::Stopped
        );
        assert!(
            finish_unstarted(
                &directory.0,
                &stop_id,
                "project-b",
                "snapshot-a",
                JobState::Failed,
            )
            .is_err()
        );
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
    fn preparation_failure_survives_reopen_without_raw_error_text() {
        let directory = TestDirectory::new();
        let job = start_job(&directory);
        let stages = vec![
            crate::generation_timing::PreparationStage {
                kind: crate::generation_timing::PreparationKind::SyntaxIndexLoad,
                started_ms: 0,
                finished_ms: 5,
                outcome: crate::generation_timing::PreparationOutcome::Complete,
            },
            crate::generation_timing::PreparationStage {
                kind: crate::generation_timing::PreparationKind::RuntimeReview,
                started_ms: 5,
                finished_ms: 9,
                outcome: crate::generation_timing::PreparationOutcome::Failed,
            },
        ];
        let saved =
            finish_preparation_failure(&directory.0, &job.job_id, JobState::Failed, stages.clone())
                .unwrap();
        assert_eq!(saved.state, JobState::Failed);
        assert_eq!(saved.preparation_stages, stages);
        let text = fs::read_to_string(store_path(&directory.0)).unwrap();
        assert!(!text.contains("error"));
        assert_eq!(get(&directory.0, &job.job_id).unwrap(), saved);
    }

    #[test]
    fn preparation_failure_state_must_match_final_stage_outcome() {
        let directory = TestDirectory::new();
        let job = start_job(&directory);
        let stages = vec![crate::generation_timing::PreparationStage {
            kind: crate::generation_timing::PreparationKind::SnapshotLoad,
            started_ms: 0,
            finished_ms: 2,
            outcome: crate::generation_timing::PreparationOutcome::Failed,
        }];
        assert!(
            finish_preparation_failure(&directory.0, &job.job_id, JobState::Stopped, stages)
                .is_err()
        );
        assert_eq!(get(&directory.0, &job.job_id).unwrap(), job);
    }

    #[test]
    fn successful_preparation_checkpoints_remain_available_while_job_runs() {
        let directory = TestDirectory::new();
        let job = start_job(&directory);
        let stages = vec![crate::generation_timing::PreparationStage {
            kind: crate::generation_timing::PreparationKind::SyntaxIndexLoad,
            started_ms: 0,
            finished_ms: 4,
            outcome: crate::generation_timing::PreparationOutcome::Complete,
        }];
        let updated = record_preparation_stages(&directory.0, &job.job_id, stages.clone()).unwrap();
        assert_eq!(updated.state, JobState::Preparing);
        assert_eq!(updated.preparation_stages, stages);
        assert_eq!(get(&directory.0, &job.job_id).unwrap(), updated);
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
        let conversation_turn_id = Uuid::new_v4().to_string();
        let stopped =
            request_stop_or_record(&directory.0, &job_id, "project-a", "snapshot-a").unwrap();
        assert_eq!(stopped.state, JobState::StopRequested);
        let adopted = start(
            &directory.0,
            &job_id,
            JobKind::WorkspaceQuestion,
            Some(&conversation_turn_id),
            "project-a",
            "snapshot-a",
            "gpt-6-luna",
            "max",
        )
        .unwrap();
        assert_eq!(adopted.state, stopped.state);
        assert!(adopted.worker_registered);
        assert!(
            start(
                &directory.0,
                &job_id,
                JobKind::WorkspaceQuestion,
                Some(&conversation_turn_id),
                "project-a",
                "snapshot-a",
                "gpt-6-luna",
                "max",
            )
            .is_err()
        );
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
