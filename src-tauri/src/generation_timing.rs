//! Bounded local observations of a native Codex operation. Diagnostics cannot
//! authorize a request, resume inference, or affect chapter publication.

use crate::{bounded_file, codex_agent, repository_context as context};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    future::Future,
    path::{Path, PathBuf},
    sync::Mutex,
    time::{Duration, Instant},
};

mod runtime;
use runtime::{ReportedUsage, RuntimeTurn};
pub(crate) use runtime::{RuntimeEventKind, TokenUsageSnapshot};

const MAX_RECORDS: usize = 32;
const MAX_EVENTS: usize = 128;
const MAX_PREPARATION_STAGES: usize = 16;
const MAX_CONTEXT_STAGES_PER_CALL: usize = 8;
const MAX_BYTES: usize = 64 * 1024;
pub(crate) const MAX_ELAPSED: u64 = 86_400_000;
static STORE: Mutex<()> = Mutex::new(());

const SCHEMA_VERSION: u32 = 5;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum EventKind {
    ManagerEntered,
    RuntimeLockAcquired,
    Preparing,
    Starting,
    Generating,
    Validating,
    Cancelling,
    ResultCallbackStarted,
    ResultCallbackReturned,
    PublicationStarted,
    PublicationCallbackReturned,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PreparationKind {
    JobRegistration,
    WorkerRegistration,
    SnapshotLoad,
    RequestPreparation,
    RuntimeReview,
    JobIdentityBinding,
    RetrievalGrant,
    SyntaxIndexLoad,
    RetrievalSession,
    OperationJournal,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum PreparationOutcome {
    Complete,
    Fallback,
    Failed,
    Stopped,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreparationStage {
    pub(crate) kind: PreparationKind,
    pub(crate) started_ms: u64,
    pub(crate) finished_ms: u64,
    pub(crate) outcome: PreparationOutcome,
}

/// Native pre-manager preparation, measured on the same monotonic clock as
/// dispatch. Only fixed stage names and elapsed offsets enter diagnostics.
pub(crate) struct Preparation {
    origin: Instant,
    stage_started: Instant,
    stages: Vec<PreparationStage>,
}

impl Preparation {
    pub(crate) fn new(origin: Instant) -> Self {
        Self {
            origin,
            stage_started: origin,
            stages: Vec::with_capacity(MAX_PREPARATION_STAGES),
        }
    }

    pub(crate) fn run_stage<T>(
        &mut self,
        kind: PreparationKind,
        operation: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        self.run_stage_with_error(kind, operation)
    }

    pub(crate) fn run_stage_with_error<T, E>(
        &mut self,
        kind: PreparationKind,
        operation: impl FnOnce() -> Result<T, E>,
    ) -> Result<T, E> {
        let result = operation();
        self.record_stage(
            kind,
            if result.is_ok() {
                PreparationOutcome::Complete
            } else {
                PreparationOutcome::Failed
            },
        );
        result
    }

    pub(crate) async fn run_async_stage<T, E, F, Fut>(
        &mut self,
        kind: PreparationKind,
        operation: F,
    ) -> Result<T, E>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let result = operation().await;
        self.record_stage(
            kind,
            if result.is_ok() {
                PreparationOutcome::Complete
            } else {
                PreparationOutcome::Failed
            },
        );
        result
    }

    /// Measure an optional local capability read whose failure is intentionally
    /// handled by a fallback. A loaded capability records `Complete`; a
    /// missing or unreadable one records `Fallback` while returning `None`.
    /// Diagnostics never retain the error text or source data.
    pub(crate) fn run_optional_stage<T>(
        &mut self,
        kind: PreparationKind,
        operation: impl FnOnce() -> Result<Option<T>, String>,
    ) -> Option<T> {
        let result = operation();
        let outcome = match &result {
            Ok(Some(_)) => PreparationOutcome::Complete,
            Ok(None) | Err(_) => PreparationOutcome::Fallback,
        };
        self.record_stage(kind, outcome);
        result.ok().flatten()
    }

    fn record_stage(&mut self, kind: PreparationKind, outcome: PreparationOutcome) {
        let now = Instant::now();
        self.stages.push(PreparationStage {
            kind,
            started_ms: offset_ms(self.origin, self.stage_started),
            finished_ms: offset_ms(self.origin, now),
            outcome,
        });
        self.stage_started = now;
    }

    pub(crate) fn mark_last_stopped(&mut self) {
        if let Some(stage) = self.stages.last_mut() {
            if stage.outcome == PreparationOutcome::Failed {
                stage.outcome = PreparationOutcome::Stopped;
            }
        }
    }

    pub(crate) fn into_stages(self) -> Vec<PreparationStage> {
        self.stages
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Event {
    kind: EventKind,
    elapsed_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Terminal {
    Returned,
    Cancelled,
    TimedOut,
    RuntimeChanged,
    Failed,
    Abandoned,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ApplicationOutcome {
    Complete,
    Partial,
    Stopped,
    Failed,
    Interrupted,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ApplicationObservation {
    Live,
    Reconciled,
}

/// The answer-store result correlated with a provider run. Reconciled results
/// deliberately have no elapsed time because they may be observed after a
/// process restart on a different monotonic clock.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ApplicationResult {
    pub(crate) outcome: ApplicationOutcome,
    pub(crate) observation: ApplicationObservation,
    pub(crate) observed_elapsed_ms: Option<u64>,
    pub(crate) published_section_count: usize,
    pub(crate) first_useful_elapsed_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct End {
    state: Terminal,
    elapsed_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ContextCall {
    pub(crate) tool: crate::evidence_ledger::Tool,
    pub(crate) state: crate::evidence_ledger::CallState,
    pub(crate) started_ms: Option<u64>,
    pub(crate) prepared_ms: Option<u64>,
    pub(crate) finished_ms: Option<u64>,
    pub(crate) source_bytes: usize,
    pub(crate) metadata_bytes: usize,
    pub(crate) wire_bytes: usize,
    pub(crate) stages: Vec<ContextStage>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ContextStageKind {
    SearchValidation,
    SearchCacheLock,
    SnapshotManifestLoad,
    SearchCacheCheck,
    SyntaxIndexLoad,
    SearchIndexPreparation,
    SearchCachePublishCheck,
    SearchQuery,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ContextStageOutcome {
    Complete,
    Fallback,
    Failed,
    Stopped,
}

/// Bounded local search stage offsets on the owning native context clock.
/// No query, path, source, or diagnostic string is retained.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ContextStage {
    pub(crate) kind: ContextStageKind,
    pub(crate) started_ms: u64,
    pub(crate) finished_ms: u64,
    pub(crate) outcome: ContextStageOutcome,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ContextTiming {
    /// Exact monotonic offset, never reconstructed from wall-clock timestamps.
    pub(crate) origin_elapsed_ms: Option<u64>,
    pub(crate) calls: Vec<ContextCall>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Record {
    schema_version: u32,
    clock: String,
    run_id: String,
    request_fingerprint: Option<String>,
    prompt_contract_version: Option<String>,
    model_id: String,
    reasoning_effort: String,
    runtime_identity_sha256: String,
    events: Vec<Event>,
    /// Renderer queue duration from WebView performance.now(); independent of
    /// this record's native monotonic clock and never combined with offsets.
    renderer_queue_wait_ms: Option<u64>,
    preparation_stages: Vec<PreparationStage>,
    context: Option<ContextTiming>,
    output_bytes: Option<usize>,
    /// Latest cumulative thread usage reported by app-server, never an estimate
    /// or a sum of updates. Null means that no valid usage update was observed.
    token_usage: Option<ReportedUsage>,
    runtime_turns: Vec<RuntimeTurn>,
    terminal: Option<End>,
    application_result: Option<ApplicationResult>,
    write_failures: u32,
    truncated: bool,
}

impl Record {
    /// First observed assistant-message delta on this trace's monotonic clock.
    /// This is transport-visible text progress, not a model-token timestamp.
    pub(crate) fn first_agent_message_delta_ms(&self) -> Option<u64> {
        runtime::first_agent_message_delta_ms(self)
    }

    pub(crate) fn take_preparation_stages(&mut self) -> Vec<PreparationStage> {
        std::mem::take(&mut self.preparation_stages)
    }
}

struct State {
    path: PathBuf,
    record: Record,
    last_write: Instant,
}
pub(crate) struct Trace {
    started: Instant,
    state: Mutex<Option<State>>,
}

fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}
fn canonical_run(run: &str) -> bool {
    uuid::Uuid::parse_str(run).is_ok_and(|id| id.to_string() == run)
}

fn safe_model_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/+".contains(&byte))
}

pub(crate) fn valid_prompt_contract_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/+".contains(&byte))
}

fn path(storage: &Path, run: &str) -> Result<PathBuf, String> {
    if !canonical_run(run) {
        return Err("The timing identity is invalid.".into());
    }
    Ok(storage
        .join("generation-timings")
        .join(format!("{run}.json")))
}

fn read(path: &Path) -> Result<Record, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "Timing diagnostics are unavailable.")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Timing diagnostics are not a regular file.".into());
    }
    let bytes = bounded_file::read(path, MAX_BYTES)
        .map_err(|_| "Timing diagnostics exceed their limit.")?;
    let value =
        crate::strict_json::from_slice(&bytes).map_err(|_| "Timing diagnostics are damaged.")?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64);
    if schema_version != Some(u64::from(SCHEMA_VERSION)) {
        return Err("Timing diagnostics use an unsupported format.".into());
    }
    // `Option<T>` fields deserialize as `None` when absent, even without a
    // serde default. Require the complete current envelope so an incomplete
    // record cannot be mistaken for a current diagnostic.
    const REQUIRED_FIELDS: &[&str] = &[
        "schemaVersion",
        "clock",
        "runId",
        "requestFingerprint",
        "modelId",
        "reasoningEffort",
        "runtimeIdentitySha256",
        "events",
        "rendererQueueWaitMs",
        "preparationStages",
        "context",
        "outputBytes",
        "tokenUsage",
        "runtimeTurns",
        "terminal",
        "applicationResult",
        "promptContractVersion",
        "writeFailures",
        "truncated",
    ];
    if !REQUIRED_FIELDS
        .iter()
        .all(|field| value.get(*field).is_some())
    {
        return Err("Timing diagnostics are damaged.".into());
    }
    let record: Record =
        serde_json::from_value(value).map_err(|_| "Timing diagnostics are damaged.")?;
    validate(&record)?;
    Ok(record)
}

fn validate(record: &Record) -> Result<(), String> {
    let invalid = || "Timing diagnostics are invalid.".to_owned();
    if record.schema_version != SCHEMA_VERSION
        || record.clock != "nativeCodexAdmissionV1"
        || !canonical_run(&record.run_id)
        || !safe_model_id(&record.model_id)
        || record.reasoning_effort.trim().is_empty()
        || record.reasoning_effort.len() > 32
        || !hash(&record.runtime_identity_sha256)
        || record
            .request_fingerprint
            .as_ref()
            .is_some_and(|v| !hash(v))
        || record
            .prompt_contract_version
            .as_ref()
            .is_some_and(|value| !valid_prompt_contract_version(value))
        || record.events.len() > MAX_EVENTS
        || record
            .renderer_queue_wait_ms
            .is_some_and(|v| v > MAX_ELAPSED)
        || record.output_bytes.is_some_and(|v| v > 300 * 1024)
        || record
            .events
            .first()
            .is_none_or(|e| e.kind != EventKind::ManagerEntered)
    {
        return Err(invalid());
    }
    runtime::validate(record)?;
    validate_preparation_stages(&record.preparation_stages)?;
    let manager_entered_ms = record.events.first().map_or(0, |event| event.elapsed_ms);
    let mut preparation_end = 0;
    for stage in &record.preparation_stages {
        if stage.started_ms < preparation_end
            || stage.finished_ms < stage.started_ms
            || stage.finished_ms > manager_entered_ms
            || !matches!(
                stage.outcome,
                PreparationOutcome::Complete | PreparationOutcome::Fallback
            )
        {
            return Err(invalid());
        }
        preparation_end = stage.finished_ms;
    }
    let mut previous = 0;
    for event in &record.events {
        if event.elapsed_ms < previous || event.elapsed_ms > MAX_ELAPSED {
            return Err(invalid());
        }
        previous = event.elapsed_ms;
    }
    if record
        .terminal
        .as_ref()
        .is_some_and(|end| end.elapsed_ms < previous || end.elapsed_ms > MAX_ELAPSED)
    {
        return Err(invalid());
    }
    if let Some(result) = &record.application_result {
        if record.schema_version != SCHEMA_VERSION
            || result.published_section_count > 6
            || result
                .observed_elapsed_ms
                .is_some_and(|elapsed| elapsed > MAX_ELAPSED)
            || result
                .first_useful_elapsed_ms
                .is_some_and(|elapsed| elapsed > MAX_ELAPSED)
            || (result.published_section_count == 0 && result.first_useful_elapsed_ms.is_some())
            || record.terminal.as_ref().is_some_and(|terminal| {
                result
                    .first_useful_elapsed_ms
                    .is_some_and(|elapsed| elapsed > terminal.elapsed_ms)
            })
            || match result.observation {
                ApplicationObservation::Live => result.observed_elapsed_ms.is_none_or(|elapsed| {
                    record
                        .terminal
                        .as_ref()
                        .is_none_or(|terminal| elapsed < terminal.elapsed_ms)
                        || result
                            .first_useful_elapsed_ms
                            .is_some_and(|first| first > elapsed)
                }),
                ApplicationObservation::Reconciled => result.observed_elapsed_ms.is_some(),
            }
        {
            return Err(invalid());
        }
    }
    if let Some(context) = &record.context {
        if context.origin_elapsed_ms.is_some_and(|ms| ms > MAX_ELAPSED) || context.calls.len() > 64
        {
            return Err(invalid());
        }
        let mut prior = 0;
        let mut bytes = (0_u64, 0_u64, 0_u64);
        for call in &context.calls {
            if call.source_bytes as u128 + call.metadata_bytes as u128 > call.wire_bytes as u128 {
                return Err(invalid());
            }
            validate_context_stages(
                call.tool,
                call.started_ms,
                call.prepared_ms,
                call.finished_ms,
                &call.stages,
                context.origin_elapsed_ms,
            )?;
            let values = [call.started_ms, call.prepared_ms, call.finished_ms];
            let mut last = prior;
            for offset in values.into_iter().flatten() {
                if offset < last
                    || offset > MAX_ELAPSED
                    || context
                        .origin_elapsed_ms
                        .is_some_and(|origin| origin.saturating_add(offset) > MAX_ELAPSED)
                {
                    return Err(invalid());
                }
                last = offset;
            }
            if call.started_ms.is_none()
                && (call.prepared_ms.is_some() || call.finished_ms.is_some())
            {
                return Err(invalid());
            }
            if call.started_ms.is_some()
                && match call.state {
                    crate::evidence_ledger::CallState::Started => {
                        call.prepared_ms.is_some() || call.finished_ms.is_some()
                    }
                    crate::evidence_ledger::CallState::Prepared => {
                        call.prepared_ms.is_none() || call.finished_ms.is_some()
                    }
                    crate::evidence_ledger::CallState::Delivered => {
                        call.prepared_ms.is_none() || call.finished_ms.is_none()
                    }
                    crate::evidence_ledger::CallState::Rejected => {
                        call.prepared_ms.is_some() || call.finished_ms.is_none()
                    }
                    crate::evidence_ledger::CallState::Uncertain => call.finished_ms.is_none(),
                }
            {
                return Err(invalid());
            }
            prior = last;
            bytes.0 = bytes.0.saturating_add(call.source_bytes as u64);
            bytes.1 = bytes.1.saturating_add(call.metadata_bytes as u64);
            bytes.2 = bytes.2.saturating_add(call.wire_bytes as u64);
        }
        if let (Some(origin), Some(end)) = (context.origin_elapsed_ms, &record.terminal) {
            if origin.saturating_add(prior) > end.elapsed_ms {
                return Err(invalid());
            }
        }
        if bytes.0 > 250_000 || bytes.1 > 256_000 || bytes.2 > 1_000_000 {
            return Err(invalid());
        }
    }
    Ok(())
}

pub(crate) fn validate_preparation_stages(stages: &[PreparationStage]) -> Result<(), String> {
    if stages.len() > MAX_PREPARATION_STAGES {
        return Err("Timing diagnostics are invalid.".into());
    }
    let mut previous_end = 0;
    for stage in stages {
        if stage.started_ms < previous_end
            || stage.finished_ms < stage.started_ms
            || stage.finished_ms > MAX_ELAPSED
            || (stage.outcome == PreparationOutcome::Fallback
                && stage.kind != PreparationKind::SyntaxIndexLoad)
        {
            return Err("Timing diagnostics are invalid.".into());
        }
        previous_end = stage.finished_ms;
    }
    Ok(())
}

pub(crate) fn validate_context_stages(
    tool: crate::evidence_ledger::Tool,
    started_ms: Option<u64>,
    prepared_ms: Option<u64>,
    finished_ms: Option<u64>,
    stages: &[ContextStage],
    origin_elapsed_ms: Option<u64>,
) -> Result<(), String> {
    let invalid = || "Timing diagnostics are invalid.".to_owned();
    if stages.len() > MAX_CONTEXT_STAGES_PER_CALL
        || (!stages.is_empty() && tool != crate::evidence_ledger::Tool::Search)
    {
        return Err(invalid());
    }
    let mut previous_end = started_ms.unwrap_or(0);
    for stage in stages {
        if started_ms.is_none_or(|started| stage.started_ms < started)
            || stage.started_ms < previous_end
            || stage.finished_ms < stage.started_ms
            || stage.finished_ms > MAX_ELAPSED
            || prepared_ms.is_some_and(|prepared| stage.finished_ms > prepared)
            || finished_ms.is_some_and(|finished| stage.finished_ms > finished)
            || origin_elapsed_ms
                .is_some_and(|origin| origin.saturating_add(stage.finished_ms) > MAX_ELAPSED)
            || (stage.outcome == ContextStageOutcome::Fallback
                && stage.kind != ContextStageKind::SyntaxIndexLoad)
        {
            return Err(invalid());
        }
        previous_end = stage.finished_ms;
    }
    Ok(())
}

/// Make room only by removing a validated terminal diagnostic. Working,
/// corrupt and unrelated files are never silently deleted to enable tracing.
fn reserve(storage: &Path, run: &str) -> Result<PathBuf, String> {
    let path = path(storage, run)?;
    if fs::symlink_metadata(&path).is_ok() {
        return Err("The timing record already exists.".into());
    }
    let directory = path.parent().ok_or("Timing storage is unavailable.")?;
    codex_agent::ensure_private_directory(directory)?;
    let mut records = vec![];
    for (index, entry) in fs::read_dir(directory)
        .map_err(|_| "Timing storage is unavailable.")?
        .enumerate()
    {
        if index >= MAX_RECORDS + 8 {
            return Err("Timing storage reached its entry limit.".into());
        }
        let entry = entry.map_err(|_| "Timing storage is unavailable.")?;
        if entry.path().extension().is_some_and(|e| e == "json") {
            let record = read(&entry.path())?;
            if entry.path().file_stem().and_then(|s| s.to_str()) != Some(record.run_id.as_str()) {
                return Err("Timing filename changed.".into());
            }
            records.push((
                entry.path(),
                record.terminal.is_some(),
                entry.metadata().and_then(|m| m.modified()).ok(),
            ));
        }
    }
    if records.len() > MAX_RECORDS {
        return Err("Timing storage already exceeds its record limit.".into());
    }
    if records.len() == MAX_RECORDS {
        records.sort_by_key(|(_, _, modified)| *modified);
        let oldest = records
            .into_iter()
            .find(|(_, terminal, _)| *terminal)
            .ok_or("Timing storage contains only unfinished records.")?;
        fs::remove_file(oldest.0).map_err(|_| "Timing storage could not be bounded.")?;
    }
    Ok(path)
}

impl Trace {
    /// A failure disables diagnostics for this run; it cannot lose a chapter or
    /// turn a completed publication into another model request.
    pub(crate) fn new(
        storage: &Path,
        run: &str,
        fingerprint: &str,
        runtime: &crate::codex_identity::CodexReviewRuntime,
        model: &str,
        effort: &str,
        started: Instant,
    ) -> Self {
        Self::new_with_preparation(
            storage,
            run,
            fingerprint,
            runtime,
            model,
            effort,
            started,
            None,
            vec![],
        )
    }

    pub(crate) fn new_with_preparation(
        storage: &Path,
        run: &str,
        fingerprint: &str,
        runtime: &crate::codex_identity::CodexReviewRuntime,
        model: &str,
        effort: &str,
        started: Instant,
        renderer_queue_wait_ms: Option<u64>,
        preparation_stages: Vec<PreparationStage>,
    ) -> Self {
        Self::new_with_preparation_and_prompt(
            storage,
            run,
            fingerprint,
            runtime,
            model,
            effort,
            started,
            None,
            renderer_queue_wait_ms,
            preparation_stages,
        )
    }

    pub(crate) fn new_with_preparation_and_prompt(
        storage: &Path,
        run: &str,
        fingerprint: &str,
        runtime: &crate::codex_identity::CodexReviewRuntime,
        model: &str,
        effort: &str,
        started: Instant,
        prompt_contract_version: Option<&str>,
        renderer_queue_wait_ms: Option<u64>,
        preparation_stages: Vec<PreparationStage>,
    ) -> Self {
        let state = (|| -> Result<State, String> {
            let _guard = STORE.lock().map_err(|_| "Timing storage is unavailable.")?;
            let record = Record {
                schema_version: SCHEMA_VERSION,
                clock: "nativeCodexAdmissionV1".into(),
                run_id: run.into(),
                request_fingerprint: (!fingerprint.is_empty()).then(|| fingerprint.into()),
                prompt_contract_version: prompt_contract_version.map(str::to_owned),
                model_id: model.into(),
                reasoning_effort: effort.into(),
                runtime_identity_sha256: context::digest(
                    &serde_json::to_vec(runtime)
                        .map_err(|_| "Timing runtime could not be encoded.")?,
                ),
                events: vec![Event {
                    kind: EventKind::ManagerEntered,
                    elapsed_ms: elapsed(started),
                }],
                renderer_queue_wait_ms,
                preparation_stages,
                context: None,
                output_bytes: None,
                token_usage: None,
                runtime_turns: vec![],
                terminal: None,
                application_result: None,
                write_failures: 0,
                truncated: false,
            };
            validate(&record)?;
            let bytes = serde_json::to_vec(&record)
                .map_err(|_| "Timing diagnostics could not be encoded.")?;
            let path = reserve(storage, run)?;
            codex_agent::create_private_file(&path, &bytes)
                .map_err(|_| "A fresh timing record could not be created.")?;
            Ok(State {
                path,
                record,
                last_write: Instant::now(),
            })
        })()
        .ok();
        Self {
            started,
            state: Mutex::new(state),
        }
    }

    fn update(&self, mutate: impl FnOnce(&mut Record) -> bool) {
        self.update_with_flush(false, mutate);
    }

    fn update_with_flush(&self, throttle: bool, mutate: impl FnOnce(&mut Record) -> bool) {
        let Ok(mut guard) = self.state.lock() else {
            return;
        };
        let Some(state) = guard.as_mut() else {
            return;
        };
        if state.record.terminal.is_some() {
            return;
        }
        if !mutate(&mut state.record) {
            return;
        }
        // Streamed deltas can arrive many times a second. Keep bounded counters
        // in memory and flush at most once a second; phase/terminal writes also
        // flush them. Diagnostic IO must not slow every model token.
        if throttle && state.last_write.elapsed() < Duration::from_secs(1) {
            return;
        }
        let result = serde_json::to_vec(&state.record)
            .ok()
            .filter(|b| b.len() <= MAX_BYTES)
            .ok_or_else(|| "Timing diagnostics exceeded their limit.".to_owned())
            .and_then(|bytes| codex_agent::atomic_private_write(&state.path, &bytes));
        if result.is_err() {
            state.record.write_failures = state.record.write_failures.saturating_add(1);
        }
        state.last_write = Instant::now();
    }

    pub(crate) fn event(&self, kind: EventKind) {
        self.update(|record| {
            if record.events.last().is_some_and(|e| e.kind == kind) {
                return false;
            }
            if record.events.len() < MAX_EVENTS {
                record.events.push(Event {
                    kind,
                    elapsed_ms: elapsed(self.started),
                });
            } else {
                record.truncated = true;
            }
            true
        });
    }

    pub(crate) fn progress(&self, phase: &str) {
        let kind = match phase {
            "preparing" => EventKind::Preparing,
            "starting" => EventKind::Starting,
            "generating" => EventKind::Generating,
            "validating" => EventKind::Validating,
            "cancelling" => EventKind::Cancelling,
            _ => return,
        };
        self.event(kind);
    }

    pub(crate) fn context(&self, context: ContextTiming) {
        self.update(|record| {
            record.context = Some(context);
            true
        });
    }
    pub(crate) fn output(&self, bytes: usize) {
        self.update(|record| {
            record.output_bytes = Some(bytes);
            true
        });
    }
    pub(crate) fn finish(&self, result: Result<(), &str>) {
        let state = match result {
            Ok(()) => Terminal::Returned,
            Err(error) if error == crate::codex_identity::changed() => Terminal::RuntimeChanged,
            Err(
                "The Codex run was cancelled locally. No response was accepted; the provider may already have processed some reviewed context."
                | "Codex stopped because Code Atlas is closing.",
            ) => Terminal::Cancelled,
            Err(
                "The Codex run exceeded ten minutes and was stopped. Review again before retrying."
                | "The Codex operation exceeded ten minutes, including result validation. No late response was published."
                | "The Codex operation exceeded ten minutes before dispatch. Prepare a fresh review."
                | "The Codex operation exceeded ten minutes before creating a thread. No generation was started."
                | "The Codex operation exceeded ten minutes before starting the model turn. No generation was started.",
            ) => Terminal::TimedOut,
            Err(error)
                if error.starts_with("The Codex account probe exceeded its ten-minute limit") =>
            {
                Terminal::TimedOut
            }
            Err(_) => Terminal::Failed,
        };
        self.update(|record| {
            record.terminal = Some(End {
                state,
                elapsed_ms: elapsed(self.started),
            });
            true
        });
    }
}

/// Append the answer-store outcome after the provider trace has closed. This
/// is intentionally independent of the trace object's terminal update; a
/// diagnostic write failure never changes generation or saved answer state.
pub(crate) fn record_application_result(
    storage: &Path,
    run: &str,
    result: ApplicationResult,
) -> Result<(), String> {
    let path = path(storage, run)?;
    let _guard = STORE.lock().map_err(|_| "Timing storage is unavailable.")?;
    let mut record = read(&path)?;
    if record.run_id != run {
        return Err("Timing identity changed.".into());
    }
    if let Some(existing) = &record.application_result {
        if existing.observation != ApplicationObservation::Reconciled
            || result.observation != ApplicationObservation::Live
        {
            return Ok(());
        }
    }
    record.schema_version = SCHEMA_VERSION;
    record.application_result = Some(result);
    validate(&record)?;
    let bytes =
        serde_json::to_vec(&record).map_err(|_| "Timing diagnostics could not be encoded.")?;
    if bytes.len() > MAX_BYTES {
        return Err("Timing diagnostics exceeded their limit.".into());
    }
    codex_agent::atomic_private_write(&path, &bytes)
}

impl Drop for Trace {
    fn drop(&mut self) {
        self.update(|record| {
            record.terminal = Some(End {
                state: Terminal::Abandoned,
                elapsed_ms: elapsed(self.started),
            });
            true
        });
    }
}
fn elapsed(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
fn offset_ms(origin: Instant, at: Instant) -> u64 {
    u64::try_from(at.saturating_duration_since(origin).as_millis()).unwrap_or(u64::MAX)
}

pub(crate) fn inspect(storage: &Path, run: &str) -> Result<Option<Record>, String> {
    let path = path(storage, run)?;
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        _ => {
            let record = read(&path)?;
            if record.run_id != run {
                return Err("Timing identity changed.".into());
            }
            Ok(Some(record))
        }
    }
}

#[cfg(test)]
mod tests;
