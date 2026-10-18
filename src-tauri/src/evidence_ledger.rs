//! Native admission for source transmitted through owned context tools.
//! This records possible delivery before transport; it never resumes inference.

pub(crate) mod sections;

use crate::{
    bounded_file, chapters, codex_agent, codex_identity::CodexReviewRuntime,
    repository_context as context, repository_structure as structure, snapshot, workspace,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

pub(crate) const POLICY: &str = "atlas-captured-ranges-v1-files12-source250000";
pub(crate) const PROGRESSIVE_CONTRACT: &str = "codex-retrieval-v3-answer-sections";
pub(crate) const LEDGER_SCHEMA_VERSION: u32 = 3;
const MAX_CALLS: usize = 64;
const FINALIZATION_CALL_RESERVE: usize = 6;
const FINALIZATION_TIME_RESERVE: Duration = Duration::from_secs(60);
const MAX_FILES: usize = 12;
const MAX_SOURCE: usize = 250_000;
const MAX_METADATA: usize = 256_000;
const MAX_WIRE: usize = 1_000_000;
const MAX_RESPONSE: usize = 128_000;
const MAX_FRAGMENTS: usize = crate::native_citations::MAX_REFERENCES;
const MAX_JOURNAL: usize = 1_000_000;
const DEADLINE: Duration = Duration::from_secs(600);
// An operation has a ten-minute deadline. Permit slow teardown beyond that
// deadline, while refusing corrupt diagnostic offsets on inspection.
const MAX_CONTEXT_ELAPSED_MS: u64 = 86_400_000;
// Keep enough metadata/wire room to answer every remaining admitted call with
// a source-free limit notice, even if several calls were already in flight.
const MAX_LIMIT_NOTICE: usize = 1536;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum LimitReason {
    Calls,
    SourceBytes,
    MetadataBytes,
    WireBytes,
    Files,
    Fragments,
    ResultBytes,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemainingBudget {
    calls: usize,
    files: usize,
    source_bytes: usize,
    metadata_bytes: usize,
    wire_bytes: usize,
    fragments: usize,
    time_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResponseBudget {
    // Conservative capacity for more data after this result; excludes room
    // reserved for possible subsequent source-free limit notices.
    remaining: RemainingBudget,
    feedback_bytes_reserved: usize,
    finish_now: bool,
}

fn requires_finalization_now(budget_stop: bool, budget: &ResponseBudget) -> bool {
    budget_stop
        || budget.remaining.calls <= FINALIZATION_CALL_RESERVE
        || budget.remaining.metadata_bytes < 16_000
        || budget.remaining.source_bytes < 4_000
        || budget.remaining.time_ms < FINALIZATION_TIME_RESERVE.as_millis() as u64
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    operation_id: String,
    project_id: String,
    snapshot_id: String,
    manifest_sha256: String,
    grant_id: String,
    runtime: CodexReviewRuntime,
    model: String,
    effort: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Tool {
    Search,
    Read,
    Definitions,
    Relationships,
    Map,
    ProposeSection,
}

/// Owned, typed context only. Search snippets are deliberately excluded: code
/// enters the model through the range reader and its common byte accounting.
#[derive(Serialize)]
#[serde(tag = "kind", content = "result", rename_all = "camelCase")]
pub(crate) enum Context<'a> {
    Search(SearchMetadata<'a>),
    SearchError {
        code: &'static str,
        message: &'static str,
    },
    Read {},
    ReadArgumentsError {},
    SectionProposal {
        #[serde(rename = "generationId")]
        generation_id: Option<&'a str>,
        findings: &'a [serde_json::Value],
        #[serde(rename = "remainingAttempts")]
        remaining_attempts: usize,
        #[serde(rename = "remainingCorrections")]
        remaining_corrections: usize,
        #[serde(rename = "nextNewSectionOrder")]
        next_new_section_order: Option<usize>,
    },
    Definitions(&'a structure::Page<structure::Symbol>),
    Relationships(&'a structure::Page<structure::Relationship>),
    Map(&'a structure::Page<structure::Module>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchMetadata<'a> {
    matches: Vec<SearchCandidate<'a>>,
    limited: bool,
    coverage: &'a context::Coverage,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SearchCandidate<'a> {
    range: &'a context::CodeRange,
    line: usize,
    exact_identifier: bool,
    declaration_match: bool,
}

impl<'a> From<&'a context::SearchResult> for SearchMetadata<'a> {
    fn from(result: &'a context::SearchResult) -> Self {
        Self {
            matches: result
                .matches
                .iter()
                .map(|hit| SearchCandidate {
                    range: &hit.range,
                    line: hit.line,
                    exact_identifier: hit.exact_identifier,
                    declaration_match: hit.declaration_match,
                })
                .collect(),
            limited: result.limited,
            coverage: &result.coverage,
        }
    }
}

impl Context<'_> {
    fn tool(&self) -> Tool {
        match self {
            Self::Search(_) | Self::SearchError { .. } => Tool::Search,
            Self::Read {} | Self::ReadArgumentsError {} => Tool::Read,
            Self::Definitions(_) => Tool::Definitions,
            Self::Relationships(_) => Tool::Relationships,
            Self::Map(_) => Tool::Map,
            Self::SectionProposal { .. } => Tool::ProposeSection,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RangeRequest {
    pub(crate) path: String,
    pub(crate) source_sha256: String,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CallState {
    Started,
    Prepared,
    Delivered,
    Rejected,
    Uncertain,
}

/// Fixed native failure positions, never model text, paths or OS messages.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum RejectionCode {
    RangeCount,
    InvalidRange,
    PathNotCaptured,
    SourceVerificationLimit,
    SourceExcluded,
    SourceIntegrity,
    SourceUnavailable,
    SourceEncoding,
    RangeEncoding,
    ResponseEncoding,
    ResponseAccounting,
    ResponseBudget,
    Unknown,
}

fn rejection_code(error: &str) -> RejectionCode {
    match error {
        "Only a range-read response may include one to eight source ranges." => {
            RejectionCode::RangeCount
        }
        "The requested evidence range is invalid or too large." => RejectionCode::InvalidRange,
        "The evidence path was not captured." | "This path is not in the selected snapshot."
        | "This path was not captured in the selected snapshot." => RejectionCode::PathNotCaptured,
        "Source verification exceeded its local byte limit."
        | "Captured source exceeds the supported per-file size limit." => {
            RejectionCode::SourceVerificationLimit
        }
        "This captured path is excluded from code retrieval."
        | "This captured text is excluded by secret filtering." => RejectionCode::SourceExcluded,
        "Captured source failed its size or SHA-256 integrity check."
        | "The snapshot contains an invalid content hash for this path." => {
            RejectionCode::SourceIntegrity
        }
        "Captured evidence is unavailable." => RejectionCode::SourceUnavailable,
        "This captured file is binary and cannot be opened as source text."
        | "This captured file is not valid UTF-8 source text." => RejectionCode::SourceEncoding,
        "The citation range is not valid UTF-8."
        | "The code range must lie on UTF-8 boundaries inside this file."
        | "Read a nonempty code range of at most 32,768 bytes." => RejectionCode::RangeEncoding,
        "Source encoding failed." | "Context response encoding failed." => {
            RejectionCode::ResponseEncoding
        }
        "Context accounting is invalid." => RejectionCode::ResponseAccounting,
        "The context response exceeds this operation's evidence budget."
        | "The context limit notice exceeds its bound." => RejectionCode::ResponseBudget,
        _ => RejectionCode::Unknown,
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Evidence {
    id: String,
    range: context::CodeRange,
}

/// Native tool timing only. This clock starts after the ledger is created,
/// before syntax loading, excluding earlier renderer queue/account preparation. It cannot
/// identify time spent reasoning, awaiting the provider or rendering an answer.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum ContextClock {
    CapturedCodeSessionV1,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CallTiming {
    started_ms: u64,
    prepared_ms: Option<u64>,
    finished_ms: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Call {
    id: String,
    tool: Tool,
    arguments_sha256: String,
    state: CallState,
    evidence_ids: Vec<String>,
    source_bytes: usize,
    metadata_bytes: usize,
    wire_bytes: usize,
    payload_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    budget_stop: Option<LimitReason>,
    timing: CallTiming,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    rejection_code: Option<RejectionCode>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Status {
    Active,
    Complete,
    Interrupted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Record {
    schema_version: u32,
    policy: String,
    binding: Binding,
    started_at_unix_ms: u64,
    status: Status,
    calls: Vec<Call>,
    evidence: Vec<Evidence>,
    context_clock: ContextClock,
}

/// A token owns exactly one started attempt; callers cannot forge an ordinal.
pub(crate) struct CallToken {
    id: String,
}

pub(crate) struct Ledger {
    storage: PathBuf,
    path: PathBuf,
    record: Record,
    deadline: Instant,
    context_started: Instant,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Fragment {
    #[serde(skip)]
    evidence_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reference_id: Option<String>,
    range: context::CodeRange,
    contents: String,
    citeable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReusedReference {
    reference_id: String,
    citeable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Response<'a> {
    policy: &'static str,
    citation_version: &'static str,
    call_id: &'a str,
    context: Option<&'a Context<'a>>,
    added: Vec<Fragment>,
    #[serde(skip)]
    evidence_ids: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    reused_references: Vec<ReusedReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    read_error: Option<ReadError>,
    budget: ResponseBudget,
    #[serde(skip_serializing_if = "Option::is_none")]
    budget_stop: Option<LimitReason>,
}

/// A correctable request error is metadata, never source evidence. It travels
/// through the same durable reservation, byte budgets and final scope check.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadError {
    code: &'static str,
    message: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size_bytes: Option<usize>,
}

fn canonical_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
fn hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn manifest_hash(manifest: &snapshot::SnapshotManifest) -> Result<String, String> {
    serde_json::to_vec(manifest)
        .map(|b| context::digest(&b))
        .map_err(|_| "The captured evidence identity is unavailable.".to_owned())
}
fn path(storage: &Path, id: &str) -> Result<PathBuf, String> {
    if !canonical_uuid(id) {
        return Err("The evidence operation ID is invalid.".to_owned());
    }
    Ok(storage.join("evidence-ledgers").join(format!("{id}.json")))
}
fn encode(record: &Record) -> Result<Vec<u8>, String> {
    let bytes = serde_json::to_vec(record)
        .map_err(|_| "Evidence accounting could not be encoded.".to_owned())?;
    if bytes.len() > MAX_JOURNAL {
        return Err("Evidence accounting reached its local storage limit.".to_owned());
    }
    Ok(bytes)
}

fn evidence_id(record: &Record, range: &context::CodeRange) -> Result<String, String> {
    let identity = serde_json::to_vec(&(PROGRESSIVE_CONTRACT, &record.binding, range))
        .map_err(|_| "Evidence identity could not be encoded.")?;
    Ok(format!("evidence:{}", context::digest(&identity)))
}

fn valid_model_effort(model: &str, effort: &str) -> bool {
    !model.trim().is_empty() && model.len() <= 200 && !effort.trim().is_empty() && effort.len() <= 32
}

impl Ledger {
    pub(crate) fn new_progressive(
        storage: &Path,
        operation_id: &str,
        manifest: &snapshot::SnapshotManifest,
        grant_id: &str,
        runtime: &CodexReviewRuntime,
        model: &str,
        effort: &str,
    ) -> Result<Self, String> {
        let path = path(storage, operation_id)?;
        if !runtime.valid_shape() || !valid_model_effort(model, effort) {
            return Err("The selected model or reasoning effort is invalid.".to_owned());
        }
        workspace::check_retrieval_grant(
            storage,
            grant_id,
            &manifest.summary.project_id,
            &manifest.summary.snapshot_id,
            runtime,
        )?;
        let record = Record {
            schema_version: LEDGER_SCHEMA_VERSION,
            policy: POLICY.to_owned(),
            binding: Binding {
                operation_id: operation_id.to_owned(),
                project_id: manifest.summary.project_id.clone(),
                snapshot_id: manifest.summary.snapshot_id.clone(),
                manifest_sha256: manifest_hash(manifest)?,
                grant_id: grant_id.to_owned(),
                runtime: runtime.clone(),
                model: model.to_owned(),
                effort: effort.to_owned(),
            },
            started_at_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| "The local clock is unavailable.")?
                .as_millis()
                .try_into()
                .map_err(|_| "The local clock is out of range.")?,
            status: Status::Active,
            calls: vec![],
            evidence: vec![],
            context_clock: ContextClock::CapturedCodeSessionV1,
        };
        codex_agent::ensure_private_directory(
            path.parent().ok_or("Evidence storage is unavailable.")?,
        )?;
        codex_agent::create_private_file(&path, &encode(&record)?).map_err(|_| {
            "A fresh evidence record could not be created. No context was sent.".to_owned()
        })?;
        let context_started = Instant::now();
        Ok(Self {
            storage: storage.to_owned(),
            path,
            record,
            deadline: context_started + DEADLINE,
            context_started,
        })
    }

    pub(crate) fn check(
        &self,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<(), String> {
        if cancelled.load(Ordering::Acquire) || self.record.status != Status::Active {
            return Err("This evidence operation has stopped.".to_owned());
        }
        if Instant::now() >= self.deadline {
            return Err("This evidence operation reached its time limit.".to_owned());
        }
        if runtime != &self.record.binding.runtime
            || manifest_hash(manifest)? != self.record.binding.manifest_sha256
        {
            return Err("The evidence snapshot or Codex runtime changed.".to_owned());
        }
        workspace::check_retrieval_grant(
            &self.storage,
            &self.record.binding.grant_id,
            &self.record.binding.project_id,
            &self.record.binding.snapshot_id,
            runtime,
        )
    }

    fn save(&mut self, next: Record) -> Result<(), String> {
        codex_agent::atomic_private_write(&self.path, &encode(&next)?).map_err(|_| {
            "Evidence accounting could not be saved. The operation must stop.".to_owned()
        })?;
        self.record = next;
        Ok(())
    }

    pub(crate) fn begin_call(
        &mut self,
        tool: Tool,
        arguments_sha256: &str,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<CallToken, String> {
        self.check(manifest, runtime, cancelled)?;
        if !hash(arguments_sha256)
            || self.record.calls.len() >= MAX_CALLS
            || self
                .record
                .calls
                .last()
                .is_some_and(|c| matches!(c.state, CallState::Started | CallState::Prepared))
        {
            return Err(
                "The context call limit was reached or a prior delivery is incomplete.".to_owned(),
            );
        }
        let id = Uuid::new_v4().to_string();
        let mut next = self.record.clone();
        next.calls.push(Call {
            id: id.clone(),
            tool,
            arguments_sha256: arguments_sha256.to_owned(),
            state: CallState::Started,
            evidence_ids: vec![],
            source_bytes: 0,
            metadata_bytes: 0,
            wire_bytes: 0,
            payload_sha256: None,
            budget_stop: None,
            timing: CallTiming {
                started_ms: self.context_elapsed_ms(),
                prepared_ms: None,
                finished_ms: None,
            },
            rejection_code: None,
        });
        self.save(next)?;
        Ok(CallToken { id })
    }

    fn context_elapsed_ms(&self) -> u64 {
        // An in-memory Instant cannot be resumed after process exit. Persisted
        // values describe this operation only; restart never reissues its calls.
        u64::try_from(self.context_started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Export only fixed tool states, byte counters and monotonic offsets.
    /// A missing clock relation remains unknown; never infer it from wall time.
    pub(crate) fn timing_diagnostics(
        &self,
        run_started: Instant,
    ) -> crate::generation_timing::ContextTiming {
        crate::generation_timing::ContextTiming {
            origin_elapsed_ms: self.context_started
                .checked_duration_since(run_started)
                .and_then(|duration| u64::try_from(duration.as_millis()).ok()),
            calls: self.record.calls.iter()
                .map(|call| crate::generation_timing::ContextCall {
                    tool: call.tool,
                    state: call.state.clone(),
                    started_ms: Some(call.timing.started_ms),
                    prepared_ms: call.timing.prepared_ms,
                    finished_ms: call.timing.finished_ms,
                    source_bytes: call.source_bytes,
                    metadata_bytes: call.metadata_bytes,
                    wire_bytes: call.wire_bytes,
                })
                .collect(),
        }
    }

    fn pending(&self, token: &CallToken, tool: Tool) -> Result<(), String> {
        if !self
            .record
            .calls
            .last()
            .is_some_and(|c| c.id == token.id && c.tool == tool && c.state == CallState::Started)
        {
            return Err("This context response does not own its pending call.".to_owned());
        }
        Ok(())
    }

    fn response_budget(&self) -> ResponseBudget {
        let calls = MAX_CALLS.saturating_sub(self.record.calls.len());
        let reserved = calls * MAX_LIMIT_NOTICE;
        let source = self
            .record
            .calls
            .iter()
            .map(|c| c.source_bytes)
            .sum::<usize>();
        let metadata = self
            .record
            .calls
            .iter()
            .map(|c| c.metadata_bytes)
            .sum::<usize>();
        let wire = self
            .record
            .calls
            .iter()
            .map(|c| c.wire_bytes)
            .sum::<usize>();
        ResponseBudget {
            remaining: RemainingBudget {
                calls,
                files: MAX_FILES.saturating_sub(
                    self.record
                        .evidence
                        .iter()
                        .map(|e| &e.range.path)
                        .collect::<BTreeSet<_>>()
                        .len(),
                ),
                source_bytes: MAX_SOURCE.saturating_sub(source),
                metadata_bytes: MAX_METADATA
                    .saturating_sub(metadata)
                    .saturating_sub(reserved),
                wire_bytes: MAX_WIRE.saturating_sub(wire).saturating_sub(reserved),
                fragments: MAX_FRAGMENTS.saturating_sub(self.record.evidence.len()),
                time_ms: self
                    .deadline
                    .saturating_duration_since(Instant::now())
                    .as_millis()
                    .min(u64::MAX as u128) as u64,
            },
            feedback_bytes_reserved: reserved,
            finish_now: false,
        }
    }

    /// Include the result's own serialized cost. A second encoding can only
    /// shorten numeric fields, so the reported remaining capacity is a safe
    /// lower bound rather than an optimistic or recursively rounded estimate.
    fn encode_response(
        &self,
        response: &mut Response<'_>,
    ) -> Result<(Vec<u8>, usize, usize), String> {
        let source = response
            .added
            .iter()
            .map(|e| e.contents.len())
            .sum::<usize>();
        let encoded_source = response
            .added
            .iter()
            .map(|e| serde_json::to_vec(&e.contents).map(|b| b.len() - 2))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Source encoding failed.")?
            .into_iter()
            .sum::<usize>();
        let files = self
            .record
            .evidence
            .iter()
            .map(|e| &e.range.path)
            .chain(response.added.iter().map(|e| &e.range.path))
            .collect::<BTreeSet<_>>()
            .len();
        let base_metadata = response.budget.remaining.metadata_bytes;
        let base_wire = response.budget.remaining.wire_bytes;
        response.budget.remaining.source_bytes = response
            .budget
            .remaining
            .source_bytes
            .saturating_sub(source);
        response.budget.remaining.files = MAX_FILES.saturating_sub(files);
        response.budget.remaining.fragments = response
            .budget
            .remaining
            .fragments
            .saturating_sub(response.added.len());
        let first =
            serde_json::to_vec(response).map_err(|_| "Context response encoding failed.")?;
        response.budget.remaining.metadata_bytes = base_metadata.saturating_sub(
            first
                .len()
                .checked_sub(encoded_source)
                .ok_or("Context accounting is invalid.")?,
        );
        response.budget.remaining.wire_bytes = base_wire.saturating_sub(first.len());
        response.budget.finish_now =
            requires_finalization_now(response.budget_stop.is_some(), &response.budget);
        let bytes =
            serde_json::to_vec(response).map_err(|_| "Context response encoding failed.")?;
        let metadata = bytes
            .len()
            .checked_sub(encoded_source)
            .ok_or("Context accounting is invalid.")?;
        Ok((bytes, source, metadata))
    }

    pub(crate) fn budget_stopped(&self) -> bool {
        self.record
            .calls
            .iter()
            .any(|call| call.budget_stop.is_some())
    }

    fn response<'a>(
        &self,
        token: &'a CallToken,
        context: &'a Context<'a>,
        requests: &[RangeRequest],
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<Response<'a>, String> {
        self.pending(token, context.tool())?;
        if context.tool() != Tool::Read && !requests.is_empty() {
            return Err(
                "Only a range-read response may include one to eight source ranges.".to_owned(),
            );
        }
        let mut response = Response {
            policy: POLICY,
            citation_version: crate::native_citations::SHORT_VERSION,
            call_id: &token.id,
            context: Some(context),
            added: vec![],
            evidence_ids: vec![],
            reused_references: vec![],
            read_error: None,
            budget: self.response_budget(),
            budget_stop: None,
        };
        if matches!(context, Context::ReadArgumentsError {}) {
            if !requests.is_empty() {
                return Err("Malformed read arguments cannot supply source ranges.".into());
            }
            response.read_error = Some(ReadError {
                code: "invalidReadArguments",
                message: "No code was returned. Use exactly {ranges:[{path,sourceSha256,startByte,endByte}]} with one to eight entries, captured relative path/hash strings and nonnegative integer byte offsets. Do not quote numbers, add fields or invent/default ranges. Copy descriptors from search_code or find_definitions and correct within the remaining budget.",
                path: None,
                source_sha256: None,
                size_bytes: None,
            });
            return Ok(response);
        }
        if context.tool() == Tool::Read && !(1..=8).contains(&requests.len()) {
            // A batching mistake is correctable. Return no source at all,
            // rather than truncate to eight ranges or abort the investigation.
            response.read_error = Some(ReadError {
                code: "invalidRangeCount",
                message: "No code was returned. Supply one to eight ranges in each read_code call. Split larger batches into separate calls within the remaining budget.",
                path: None,
                source_sha256: None,
                size_bytes: None,
            });
            return Ok(response);
        }
        if requests.iter().any(|request| {
            !context::safe_path(&request.path)
                || !hash(&request.source_sha256)
                || !manifest.files.iter().any(|file| file.path == request.path)
        }) {
            // Validate the complete descriptor batch before loading any source.
            // A model typo grants no evidence and need not kill its connection.
            // Authorization, immutable-object integrity and delivery still fail
            // through their existing terminal paths, never through this reply.
            response.read_error = Some(ReadError {
                code: "invalidRead",
                message: "No code was returned. Copy the exact captured relative path and 64-character lowercase hexadecimal sourceSha256 from search_code or find_definitions. Do not invent paths or hashes. Correct the request within the remaining budget.",
                path: None,
                source_sha256: None,
                size_bytes: None,
            });
            return Ok(response);
        }
        let mut sources = HashMap::new();
        let mut verified_bytes = 0u64;
        for request in requests {
            self.check(manifest, runtime, cancelled)?;
            if !sources.contains_key(&request.path) {
                let file = manifest
                    .files
                    .iter()
                    .find(|f| f.path == request.path)
                    .ok_or("The evidence path was not captured.")?;
                verified_bytes = verified_bytes
                    .checked_add(file.size_bytes)
                    .filter(|b| *b <= 100 * 1024 * 1024)
                    .ok_or("Source verification exceeded its local byte limit.")?;
                sources.insert(
                    request.path.clone(),
                    context::source(&self.storage, manifest, &request.path)?,
                );
            }
            let source = sources
                .get(&request.path)
                .ok_or("Captured evidence is unavailable.")?;
            if source.sha256 != request.source_sha256
                || request.start_byte >= request.end_byte
                || request.end_byte.saturating_sub(request.start_byte) > context::MAX_RANGE_BYTES
                || source
                    .contents
                    .get(request.start_byte..request.end_byte)
                    .is_none()
            {
                // Do not clamp or substitute code for an invalid request. The
                // agent can correct its own request within the original budget.
                response.added.clear();
                response.evidence_ids.clear();
                response.reused_references.clear();
                response.read_error = Some(ReadError {
                    code: "invalidRead",
                    message: "No code was returned. Copy sourceSha256 and exact startByte/endByte from search_code or find_definitions. End is exclusive, inside sizeBytes, on UTF-8 boundaries, and at most 32768 bytes after start. Refine the search if needed.",
                    path: Some(request.path.clone()),
                    source_sha256: Some(source.sha256.clone()),
                    size_bytes: Some(source.contents.len()),
                });
                return Ok(response);
            }
            let previous = self
                .record
                .evidence
                .iter()
                .map(|e| (&e.id, &e.range))
                .chain(response.added.iter().map(|e| (&e.evidence_id, &e.range)))
                .filter(|(_, r)| r.path == request.path && r.source_sha256 == request.source_sha256)
                .collect::<Vec<_>>();
            let mut intervals = vec![];
            for (id, range) in previous {
                if range.start_byte < request.end_byte && range.end_byte > request.start_byte {
                    intervals.push((range.start_byte, range.end_byte));
                    if !response.evidence_ids.contains(id) {
                        response.evidence_ids.push(id.clone());
                        let reference_id = self
                            .record
                            .evidence
                            .iter()
                            .position(|e| e.id == *id)
                            .map(crate::native_citations::short_reference)
                            .or_else(|| {
                                response
                                    .added
                                    .iter()
                                    .find(|f| &f.evidence_id == id)
                                    .and_then(|f| f.reference_id.clone())
                            })
                            .ok_or("The reusable native reference is unavailable.")?;
                        response.reused_references.push(ReusedReference {
                            reference_id,
                            citeable: source
                                .contents
                                .get(range.start_byte..range.end_byte)
                                .is_some_and(crate::native_citations::citeable),
                        });
                    }
                }
            }
            for (start, end) in gaps(request.start_byte, request.end_byte, intervals) {
                for (from, to) in crate::native_citations::spans(&source.contents, start, end)? {
                    self.check(manifest, runtime, cancelled)?;
                    let read = context::range_from_source(source, from, to)?;
                    response.added.push(Fragment {
                        evidence_id: evidence_id(&self.record, &read.range)?,
                        reference_id: Some(crate::native_citations::short_reference(
                            self.record.evidence.len() + response.added.len(),
                        )),
                        citeable: crate::native_citations::citeable(&read.contents),
                        range: read.range,
                        contents: read.contents,
                    });
                }
            }
        }
        response.evidence_ids.sort();
        Ok(response)
    }

    /// The payload is exposed only inside the transport callback, after durable
    /// reservation and a final grant/identity/cancellation check. Callback errors
    /// mean delivery is uncertain and terminate this ledger permanently.
    pub(crate) fn deliver(
        &mut self,
        token: CallToken,
        context: Context<'_>,
        requests: &[RangeRequest],
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
        send: impl FnOnce(&[u8]) -> Result<(), String>,
    ) -> Result<(), String> {
        self.check(manifest, runtime, cancelled)?;
        let prepared: Result<(Record, Vec<u8>), String> = (|| {
            let mut response =
                self.response(&token, &context, requests, manifest, runtime, cancelled)?;
            let (mut bytes, mut source_bytes, mut metadata_bytes) =
                self.encode_response(&mut response)?;
            let prior_source = self
                .record
                .calls
                .iter()
                .map(|c| c.source_bytes)
                .sum::<usize>();
            let prior_metadata = self
                .record
                .calls
                .iter()
                .map(|c| c.metadata_bytes)
                .sum::<usize>();
            let prior_wire = self
                .record
                .calls
                .iter()
                .map(|c| c.wire_bytes)
                .sum::<usize>();
            let files = self
                .record
                .evidence
                .iter()
                .map(|e| &e.range.path)
                .chain(response.added.iter().map(|e| &e.range.path))
                .collect::<BTreeSet<_>>()
                .len();
            let reserved = response.budget.feedback_bytes_reserved;
            let limit = self
                .record
                .calls
                .iter()
                .find_map(|c| c.budget_stop)
                .or_else(|| {
                    if self.record.calls.len() == MAX_CALLS {
                        Some(LimitReason::Calls)
                    } else if prior_source + source_bytes > MAX_SOURCE {
                        Some(LimitReason::SourceBytes)
                    } else if files > MAX_FILES {
                        Some(LimitReason::Files)
                    } else if self.record.evidence.len() + response.added.len() > MAX_FRAGMENTS {
                        Some(LimitReason::Fragments)
                    } else if bytes.len() > MAX_RESPONSE {
                        Some(LimitReason::ResultBytes)
                    } else if prior_metadata + metadata_bytes + reserved > MAX_METADATA {
                        Some(LimitReason::MetadataBytes)
                    } else if prior_wire + bytes.len() + reserved > MAX_WIRE {
                        Some(LimitReason::WireBytes)
                    } else {
                        None
                    }
                });
            if let Some(reason) = limit {
                // Nothing from the oversized/new result crosses the boundary.
                // Keep the turn alive so Codex can answer from prior evidence.
                response.context = None;
                response.added.clear();
                response.evidence_ids.clear();
                response.reused_references.clear();
                response.read_error = None;
                response.budget = self.response_budget();
                response.budget_stop = Some(reason);
                (bytes, source_bytes, metadata_bytes) = self.encode_response(&mut response)?;
                if bytes.len() > MAX_LIMIT_NOTICE {
                    return Err("The context limit notice exceeds its bound.".into());
                }
            }
            if prior_metadata + metadata_bytes > MAX_METADATA || prior_wire + bytes.len() > MAX_WIRE
            {
                return Err(
                    "The context response exceeds this operation's evidence budget.".into(),
                );
            }
            let mut next = self.record.clone();
            let call = next
                .calls
                .last_mut()
                .ok_or("The context call is unavailable.")?;
            call.state = CallState::Prepared;
            call.timing.prepared_ms = Some(self.context_elapsed_ms());
            call.budget_stop = response.budget_stop;
            call.evidence_ids = response.evidence_ids.clone();
            for fragment in response.added {
                call.evidence_ids.push(fragment.evidence_id.clone());
                next.evidence.push(Evidence {
                    id: fragment.evidence_id,
                    range: fragment.range,
                });
            }
            call.evidence_ids.sort();
            call.evidence_ids.dedup();
            call.source_bytes = source_bytes;
            call.metadata_bytes = metadata_bytes;
            call.wire_bytes = bytes.len();
            call.payload_sha256 = Some(context::digest(&bytes));
            self.check(manifest, runtime, cancelled)?;
            Ok((next, bytes))
        })();
        let (next, bytes) = match prepared {
            Ok(value) => value,
            Err(error) => {
                if self.pending(&token, context.tool()).is_ok() {
                    let mut next = self.record.clone();
                    if let Some(call) = next.calls.last_mut() {
                        call.state = CallState::Rejected;
                        call.rejection_code = Some(rejection_code(&error));
                        call.timing.finished_ms = Some(self.context_elapsed_ms());
                    }
                    self.save(next)?;
                }
                return Err(error);
            }
        };
        self.save(next)?;
        if let Err(error) = self.check(manifest, runtime, cancelled) {
            self.interrupt();
            return Err(error);
        }
        if send(&bytes).is_err() {
            self.interrupt();
            return Err("Context delivery is uncertain. This operation stopped; no automatic retry is allowed.".to_owned());
        }
        let mut delivered = self.record.clone();
        if let Some(call) = delivered.calls.last_mut() {
            call.state = CallState::Delivered;
            call.timing.finished_ms = Some(self.context_elapsed_ms());
        }
        if let Err(error) = self.save(delivered) {
            self.interrupt();
            return Err(error);
        }
        Ok(())
    }

    fn interrupt(&mut self) {
        let finished_ms = self.context_elapsed_ms();
        self.record.status = Status::Interrupted;
        if let Some(call) = self.record.calls.last_mut() {
            if matches!(call.state, CallState::Started | CallState::Prepared) {
                call.state = CallState::Uncertain;
                call.timing.finished_ms = Some(finished_ms);
            }
        }
        // A durable Prepared record is already enough to prevent replay on restart.
        let _ = self.save(self.record.clone());
    }

    pub(crate) fn verify_revision(
        &self,
        revision: &chapters::ChapterRevision,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<(), String> {
        self.check(manifest, runtime, cancelled)?;
        verify_record_revision(&self.storage, &self.record, revision, manifest, || {
            self.check(manifest, runtime, cancelled)
        })?;
        self.check(manifest, runtime, cancelled)
    }

    /// Full sources are loaded only for local canonical validation, never sent
    /// to the model. Every resulting quote must also pass verify_revision.
    pub(crate) fn validation_sources(
        &self,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<Vec<snapshot::SnapshotSource>, String> {
        self.check(manifest, runtime, cancelled)?;
        self.record
            .evidence
            .iter()
            .filter(|e| {
                self.record
                    .calls
                    .iter()
                    .any(|c| c.state == CallState::Delivered && c.evidence_ids.contains(&e.id))
            })
            .map(|e| e.range.path.as_str())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|path| {
                self.check(manifest, runtime, cancelled)?;
                context::source(&self.storage, manifest, path)
            })
            .collect()
    }

    pub(crate) fn resolve_citations(
        &self,
        response: &str,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<crate::native_citations::Resolution, String> {
        self.check(manifest, runtime, cancelled)?;
        if self.record.schema_version != LEDGER_SCHEMA_VERSION {
            return Err("The evidence record uses an unsupported protocol version.".into());
        }
        let sources = self.validation_sources(manifest, runtime, cancelled)?;
        let sources: HashMap<_, _> = sources.iter().map(|s| (s.path.as_str(), s)).collect();
        // Verify captured bytes before building the lookup table. A corrupt or
        // unavailable blob stops validation; it cannot become correction data.
        let mut citations = HashMap::new();
        for (index, evidence) in self.record.evidence.iter().enumerate() {
            self.check(manifest, runtime, cancelled)?;
            if !self.record.calls.iter().any(|call| {
                call.state == CallState::Delivered && call.evidence_ids.contains(&evidence.id)
            }) {
                continue;
            }
            let source = sources
                .get(evidence.range.path.as_str())
                .ok_or("The cited captured source is unavailable.")?;
            let contents = source
                .contents
                .get(evidence.range.start_byte..evidence.range.end_byte)
                .ok_or("The cited captured span is unavailable.")?;
            if !crate::native_citations::citeable(contents) {
                continue;
            }
            if evidence.id != evidence_id(&self.record, &evidence.range)? {
                return Err("The native citation identity changed.".into());
            }
            let citation = crate::native_citations::Citation::checked(&evidence.range, source)?;
            citations.insert(crate::native_citations::short_reference(index), citation);
        }
        let result = crate::native_citations::resolve(response, |id| {
            citations.get(id).cloned().ok_or("referenceUnavailable")
        });
        self.check(manifest, runtime, cancelled)?;
        Ok(result)
    }

    pub(crate) fn admitted_ranges(&self) -> Vec<context::CodeRange> {
        self.record
            .evidence
            .iter()
            .filter(|e| {
                self.record
                    .calls
                    .iter()
                    .any(|c| c.state == CallState::Delivered && c.evidence_ids.contains(&e.id))
            })
            .map(|e| e.range.clone())
            .collect()
    }

    pub(crate) fn delivered_call(
        &self,
        tool: Tool,
        arguments_sha256: &str,
        payload_sha256: &str,
    ) -> Option<&str> {
        self.record
            .calls
            .iter()
            .find(|c| {
                c.state == CallState::Delivered
                    && c.tool == tool
                    && c.arguments_sha256 == arguments_sha256
                    && c.payload_sha256.as_deref() == Some(payload_sha256)
            })
            .map(|c| c.id.as_str())
    }

    pub(crate) fn complete(
        &mut self,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<(), String> {
        self.check(manifest, runtime, cancelled)?;
        if self.record.calls.last().is_some_and(|c| {
            matches!(
                c.state,
                CallState::Started | CallState::Prepared | CallState::Uncertain
            )
        }) {
            return Err("An incomplete context delivery cannot complete its operation.".to_owned());
        }
        let mut next = self.record.clone();
        next.status = Status::Complete;
        self.save(next)
    }
}

impl Drop for Ledger {
    fn drop(&mut self) {
        if self.record.status == Status::Active {
            self.interrupt();
        }
    }
}

/// Sorted interval subtraction merges overlap/adjacency without altering evidence IDs.
fn gaps(start: usize, end: usize, mut intervals: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
    intervals.sort_unstable();
    let mut cursor = start;
    let mut result = vec![];
    for (from, to) in intervals {
        if to <= cursor || from >= end {
            continue;
        }
        if from > cursor {
            result.push((cursor, from.min(end)));
        }
        cursor = cursor.max(to).min(end);
        if cursor == end {
            break;
        }
    }
    if cursor < end {
        result.push((cursor, end));
    }
    result
}

fn valid_context_timings(record: &Record) -> bool {
    if record.context_clock != ContextClock::CapturedCodeSessionV1 {
        return false;
    }
    let mut prior_finish = 0;
    for call in &record.calls {
        let timing = &call.timing;
        if timing.started_ms < prior_finish
            || [
                Some(timing.started_ms),
                timing.prepared_ms,
                timing.finished_ms,
            ]
            .into_iter()
            .flatten()
            .any(|elapsed| elapsed > MAX_CONTEXT_ELAPSED_MS)
            || timing
                .prepared_ms
                .is_some_and(|prepared| prepared < timing.started_ms)
            || timing
                .finished_ms
                .is_some_and(|finished| finished < timing.prepared_ms.unwrap_or(timing.started_ms))
            || match call.state {
                CallState::Started => timing.prepared_ms.is_some() || timing.finished_ms.is_some(),
                CallState::Prepared => timing.prepared_ms.is_none() || timing.finished_ms.is_some(),
                CallState::Delivered => {
                    timing.prepared_ms.is_none() || timing.finished_ms.is_none()
                }
                CallState::Rejected => timing.prepared_ms.is_some() || timing.finished_ms.is_none(),
                CallState::Uncertain => timing.finished_ms.is_none(),
            }
        {
            return false;
        }
        prior_finish = timing
            .finished_ms
            .unwrap_or(timing.prepared_ms.unwrap_or(timing.started_ms));
    }
    true
}

fn verify_record_revision(
    storage: &Path,
    record: &Record,
    revision: &chapters::ChapterRevision,
    manifest: &snapshot::SnapshotManifest,
    mut gate: impl FnMut() -> Result<(), String>,
) -> Result<(), String> {
    gate()?;
    if manifest_hash(manifest)? != record.binding.manifest_sha256 {
        return Err("The evidence checkpoint belongs to another captured manifest.".into());
    }
    if revision.sections.is_empty()
        || revision
            .sections
            .iter()
            .any(|s| s.claims.is_empty() || s.claims.iter().any(|c| c.anchors.is_empty()))
        || revision.project_id != record.binding.project_id
        || revision.snapshot_id != record.binding.snapshot_id
    {
        return Err(
            "The explanation does not belong to this active evidence operation.".to_owned(),
        );
    }
    let mut sources = HashMap::new();
    for anchor in revision
        .sections
        .iter()
        .flat_map(|s| &s.claims)
        .flat_map(|c| &c.anchors)
    {
        gate()?;
        let start: usize = anchor
            .start_byte
            .try_into()
            .map_err(|_| "The cited range is out of bounds.")?;
        let end: usize = anchor
            .end_byte_inclusive
            .checked_add(1)
            .and_then(|v| v.try_into().ok())
            .ok_or("The cited range is out of bounds.")?;
        let intervals = record
            .evidence
            .iter()
            .filter(|e| {
                e.range.path == anchor.path
                    && e.range.source_sha256 == anchor.source_sha256
                    && e.range.snapshot_id == anchor.snapshot_id
                    && record
                        .calls
                        .iter()
                        .any(|c| c.state == CallState::Delivered && c.evidence_ids.contains(&e.id))
            })
            .map(|e| (e.range.start_byte, e.range.end_byte))
            .collect();
        if start >= end
            || anchor.snapshot_id != record.binding.snapshot_id
            || !gaps(start, end, intervals).is_empty()
        {
            return Err(
                "An explanation cites captured code that was not admitted to this operation."
                    .to_owned(),
            );
        }
        if !sources.contains_key(&anchor.path) {
            sources.insert(
                anchor.path.clone(),
                context::source(storage, manifest, &anchor.path)?,
            );
        }
        let source = sources
            .get(&anchor.path)
            .ok_or("The cited source is unavailable.")?;
        let read = context::range_from_source(source, start, end)?;
        let column = |offset| {
            let before = &source.contents.as_bytes()[..offset];
            before
                .iter()
                .rposition(|b| *b == b'\n')
                .map_or(before.len(), |i| before.len() - i - 1)
                + 1
        };
        if source.sha256 != anchor.source_sha256
            || read.contents != anchor.excerpt
            || read.range.start_line != anchor.start_line as usize
            || read.range.end_line != anchor.end_line as usize
            || column(start) != anchor.start_column as usize
            || column(end - 1) != anchor.end_column as usize
        {
            return Err(
                "The cited excerpt or coordinates do not match admitted captured code.".to_owned(),
            );
        }
    }
    Ok(())
}

/// Reads metadata for inspection only. No constructor can resume this record.
pub(crate) fn inspect(storage: &Path, operation_id: &str) -> Result<Record, String> {
    let path = path(storage, operation_id)?;
    let metadata =
        fs::symlink_metadata(&path).map_err(|_| "Evidence accounting is unavailable.")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("Evidence accounting is not a regular file.".to_owned());
    }
    let bytes = bounded_file::read(&path, MAX_JOURNAL)
        .map_err(|_| "Evidence accounting is unreadable or exceeds its limit.".to_owned())?;
    let record: Record = serde_json::from_slice(&bytes)
        .map_err(|_| "Evidence accounting is damaged; the original record was preserved.")?;
    validate_record(&record, operation_id)?;
    Ok(record)
}

fn validate_record(record: &Record, operation_id: &str) -> Result<(), String> {
    if !canonical_uuid(operation_id)
        || record.schema_version != LEDGER_SCHEMA_VERSION
        || !valid_context_timings(&record)
        || record.policy != POLICY
        || record.binding.operation_id != operation_id
        || record.calls.len() > MAX_CALLS
        || record.evidence.len() > MAX_FRAGMENTS
        || !record.binding.runtime.valid_shape()
        || !valid_model_effort(&record.binding.model, &record.binding.effort)
        || !canonical_uuid(&record.binding.grant_id)
        || !hash(&record.binding.manifest_sha256)
        || record
            .calls
            .iter()
            .map(|c| c.source_bytes as u128)
            .sum::<u128>()
            > MAX_SOURCE as u128
        || record
            .calls
            .iter()
            .map(|c| c.metadata_bytes as u128)
            .sum::<u128>()
            > MAX_METADATA as u128
        || record
            .calls
            .iter()
            .map(|c| c.wire_bytes as u128)
            .sum::<u128>()
            > MAX_WIRE as u128
        || record
            .calls
            .iter()
            .map(|c| &c.id)
            .collect::<BTreeSet<_>>()
            .len()
            != record.calls.len()
        || record.calls.iter().any(|c| {
            !canonical_uuid(&c.id)
                || !hash(&c.arguments_sha256)
                || c.wire_bytes > MAX_RESPONSE
                || c.source_bytes as u128 + c.metadata_bytes as u128 > c.wire_bytes as u128
                || c.evidence_ids.len() > MAX_FRAGMENTS
                || c.payload_sha256.as_ref().is_some_and(|h| !hash(h))
                || (c.rejection_code.is_some() && c.state != CallState::Rejected)
                || (c.budget_stop.is_some()
                    && (c.source_bytes != 0
                        || !c.evidence_ids.is_empty()
                        || matches!(c.state, CallState::Started | CallState::Rejected)))
                || c.evidence_ids
                    .iter()
                    .any(|id| !record.evidence.iter().any(|e| &e.id == id))
        })
        || record
            .calls
            .iter()
            .scan(false, |stopped, c| {
                let invalid = *stopped && (c.source_bytes != 0 || !c.evidence_ids.is_empty());
                *stopped |= c.budget_stop.is_some();
                Some(invalid)
            })
            .any(|invalid| invalid)
        || record
            .calls
            .iter()
            .map(|c| c.source_bytes as u128)
            .sum::<u128>()
            != record
                .evidence
                .iter()
                .map(|e| e.range.end_byte.saturating_sub(e.range.start_byte) as u128)
                .sum::<u128>()
        || record
            .evidence
            .iter()
            .map(|e| &e.id)
            .collect::<BTreeSet<_>>()
            .len()
            != record.evidence.len()
        || record
            .evidence
            .iter()
            .map(|e| &e.range.path)
            .collect::<BTreeSet<_>>()
            .len()
            > MAX_FILES
        || record.evidence.iter().any(|e| {
            !context::safe_path(&e.range.path)
                || !hash(&e.range.source_sha256)
                || !hash(&e.range.chunk_sha256)
                || e.range.snapshot_id != record.binding.snapshot_id
                || e.range.start_byte >= e.range.end_byte
                || e.range.end_byte - e.range.start_byte > context::MAX_RANGE_BYTES
                || e.range.end_byte - e.range.start_byte > crate::native_citations::MAX_SPAN
                || !evidence_id(&record, &e.range).is_ok_and(|id| id == e.id)
        })
    {
        return Err(
            "Evidence accounting has an unsupported identity, policy or bounds; it was preserved."
                .to_owned(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests;
