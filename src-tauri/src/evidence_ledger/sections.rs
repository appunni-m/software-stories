//! Durable, independently checked draft sections. Inspection never recreates
//! an active investigation or acquires source-sharing permission.

use super::*;
use crate::{chapter_generation, chapter_origin, native_citations};
use serde_json::{Value, json};
use std::sync::Mutex;

pub(crate) const MAX_PROPOSAL_BYTES: usize = 32_768;
pub(crate) const PROPOSAL_VERSION: &str = "answerSectionV2";
const LEGACY_PROPOSAL_VERSION: &str = "answerSectionV1";
const CURRENT_CHECKPOINT_VERSION: u32 = 4;
const MAX_SECTIONS: usize = 6;
const MAX_ATTEMPTS: usize = 8;
const MAX_ANSWERS: usize = 8;
const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 32;
static STORE: Mutex<()> = Mutex::new(());

pub(crate) mod reading;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Outcome {
    Working,
    Complete,
    Partial,
    Stopped,
    Failed,
    Interrupted,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PublishedSection {
    checkpoint_version: u32,
    pub(crate) key: String,
    pub(crate) order: usize,
    pub(crate) supersedes: Option<String>,
    pub(crate) preview: chapters::ChapterDraftPreview,
    pub(crate) published_elapsed_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) published_run_elapsed_ms: Option<u64>,
    proposal_sha256: String,
    checkpoint_sha256: String,
    checkpoint: Record,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Attempt {
    arguments: String,
    arguments_sha256: String,
    proposal_sha256: String,
    findings: Vec<Value>,
    generation_id: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FinalResponse {
    text: String,
    sha256: String,
}

/// Private terminal diagnostics. Never retain arbitrary agent/OS error text.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum FailureCode {
    Stopped,
    InvalidToolArguments,
    ToolEvent,
    ContextTimeout,
    ContextPeerClosed,
    ContextTransport,
    ContextDelivery,
    RunTimeout,
    RuntimeIdentity,
    Unknown,
}

impl FailureCode {
    pub(crate) fn from_error(error: &str) -> Self {
        match error {
            "The agent supplied invalid captured-code arguments." => Self::InvalidToolArguments,
            "The captured-code tool did not complete safely."
            | "Code tool reply was not admitted by this operation."
            | "An unverified code tool appeared in the final turn."
            | "Codex did not finish its captured-code investigation." => Self::ToolEvent,
            "The code context packet timed out." => Self::ContextTimeout,
            "The context peer closed an incomplete packet." => Self::ContextPeerClosed,
            "The context transport disconnected."
            | "The context transport could not use nonblocking I/O." => Self::ContextTransport,
            "Context delivery is uncertain. This operation stopped; no automatic retry is allowed." => {
                Self::ContextDelivery
            }
            "The Codex run exceeded ten minutes and was stopped. Review again before retrying." => {
                Self::RunTimeout
            }
            error if error == crate::codex_identity::changed() => Self::RuntimeIdentity,
            _ => Self::Unknown,
        }
    }
}

fn failure_code<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<FailureCode>, D::Error> {
    FailureCode::deserialize(deserializer).map(Some)
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Answer {
    schema_version: u32,
    binding: Binding,
    request_sha256: String,
    pub(crate) question: String,
    pub(crate) outcome: Outcome,
    pub(crate) sections: Vec<PublishedSection>,
    attempts: Vec<Attempt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    final_responses: Vec<FinalResponse>,
    pub(crate) incomplete_topics: Vec<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "failure_code"
    )]
    failure_code: Option<FailureCode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Proposal {
    proposal_version: String,
    section_key: String,
    order: usize,
    previous_generation_id: Option<String>,
    section: Value,
}

/// Constructed only by an active Ledger; saved records cannot resume it.
pub(crate) struct ActiveAnswer {
    path: PathBuf,
    value: Answer,
}

/// A prepared value does not authorize a write. Its owner must commit under
/// the same native publication lock as Stop, after rechecking the live ledger.
pub(crate) struct PreparedSection {
    before_sha256: String,
    next: Answer,
    pub(crate) generation_id: Option<String>,
    pub(crate) findings: Vec<Value>,
}

/// Source-free event identity. Draft contents stay behind native inspection.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SectionNotice {
    pub(crate) run_id: String,
    pub(crate) project_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) key: String,
    pub(crate) order: usize,
    pub(crate) generation_id: String,
    pub(crate) revision_id: String,
    pub(crate) supersedes: Option<String>,
    /// Preparation checkpoint on the captured-code clock; excludes earlier setup.
    pub(crate) context_elapsed_ms: u64,
    /// Observed after durable commit on the owning run's original clock.
    pub(crate) run_elapsed_ms: Option<u64>,
}

fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 80
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn encode_answer(answer: &Answer) -> Result<Vec<u8>, String> {
    let bytes = serde_json::to_vec(answer).map_err(|_| "The answer could not be encoded.")?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(
            "This answer reached its local storage limit. Existing sections are preserved.".into(),
        );
    }
    Ok(bytes)
}

fn directory(storage: &Path) -> Result<PathBuf, String> {
    let directory = storage.join("answer-sections");
    codex_agent::ensure_private_directory(&directory)?;
    Ok(directory)
}

fn record_path(storage: &Path, operation: &str) -> Result<PathBuf, String> {
    if !canonical_uuid(operation) {
        return Err("The answer operation identity is invalid.".into());
    }
    Ok(directory(storage)?.join(format!("{operation}.json")))
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "The saved answer is unavailable.")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("The saved answer is not a regular file.".into());
    }
    bounded_file::read(path, MAX_RECORD_BYTES)
        .map_err(|_| "The saved answer is unreadable or too large.".into())
}

fn check_capacity(directory: &Path) -> Result<(), String> {
    let mut count = 0;
    let mut entries = 0;
    for entry in fs::read_dir(directory).map_err(|_| "Saved answers could not be inspected.")? {
        entries += 1;
        if entries > MAX_DIRECTORY_ENTRIES {
            return Err(
                "Saved answers reached their directory limit. Existing work was preserved.".into(),
            );
        }
        let path = entry
            .map_err(|_| "Saved answers could not be inspected.")?
            .path();
        if path.extension().is_some_and(|x| x == "json") {
            // Reject damaged/unexpected entries rather than evicting or silently
            // resetting them. Each record reserves its full 4 MiB capacity.
            let bytes = read_bytes(&path)?;
            let _: Answer = serde_json::from_slice(&bytes)
                .map_err(|_| "A saved answer is damaged; it was preserved.")?;
            count += 1;
        }
    }
    if count >= MAX_ANSWERS {
        return Err("Eight answers are retained locally. Manage saved answers before starting another investigation.".into());
    }
    Ok(())
}

fn checkpoint(ledger: &Ledger) -> Result<Record, String> {
    let mut record = ledger.record.clone();
    // A proposal is currently executing. Its preceding immutable prefix proves
    // delivery of the cited code, without claiming delivery of this new reply.
    if record
        .calls
        .last()
        .is_some_and(|c| c.state == CallState::Started && c.tool == Tool::ProposeSection)
    {
        record.calls.pop();
    }
    if record.status != Status::Active
        || record
            .calls
            .iter()
            .any(|c| !matches!(c.state, CallState::Delivered | CallState::Rejected))
    {
        return Err("Uncertain context cannot publish an answer section.".into());
    }
    validate_record(&record, &record.binding.operation_id)?;
    Ok(record)
}

fn record_ranges(record: &Record) -> Vec<context::CodeRange> {
    record
        .evidence
        .iter()
        .filter(|e| {
            record
                .calls
                .iter()
                .any(|c| c.state == CallState::Delivered && c.evidence_ids.contains(&e.id))
        })
        .map(|e| e.range.clone())
        .collect()
}

/// Presentation is derived exclusively from the canonicalized anchored claims.
/// The legacy canonicalizer requires these wire fields before replacing them;
/// the fixed placeholders never survive successful canonicalization.
fn wrapped(proposal: &Proposal) -> Result<String, String> {
    let mut section = proposal.section.clone();
    let object = section.as_object_mut().ok_or("Invalid section shape.")?;
    if object.len() != 3
        || !["title", "claims", "visual"]
            .iter()
            .all(|k| object.contains_key(*k))
        || !object["claims"]
            .as_array()
            .is_some_and(|c| (1..=8).contains(&c.len()))
    {
        return Err("Invalid section shape.".into());
    }
    let title = object["title"]
        .as_str()
        .filter(|t| (8..=120).contains(&t.len()))
        .ok_or("Invalid section title.")?
        .to_owned();
    object.insert(
        "prose".into(),
        json!("Presentation is derived from the anchored claims."),
    );
    object.insert(
        "spokenText".into(),
        json!("Presentation is derived from the anchored claims."),
    );
    let citation_version = if proposal.proposal_version == PROPOSAL_VERSION {
        native_citations::SHORT_VERSION
    } else {
        native_citations::VERSION
    };
    serde_json::to_string(
        &json!({"citationVersion":citation_version,"title":title,"sections":[section]}),
    )
    .map_err(|_| "The section could not be encoded.".into())
}

fn finding(code: &str) -> Vec<Value> {
    vec![json!({"location":"/section","code":code})]
}

fn section_location(issue: &Value) -> String {
    issue["location"]
        .as_str()
        .and_then(|location| location.strip_prefix("/sections/0"))
        .map(|suffix| format!("/section{suffix}"))
        .filter(|location| {
            location.len() <= 160
                && location
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'/')
        })
        .unwrap_or_else(|| "/section".into())
}

/// Retain native diagnostic positions, never names, quotes or guessed repairs.
fn alignment_findings(section: &Value, resolved: &str) -> Vec<Value> {
    let Some(claims) = section["claims"].as_array() else {
        return finding("identifierAlignment");
    };
    let counts: Vec<_> = claims.iter().enumerate().filter(|(_, claim)| {
        claim["identifiers"].as_array().is_some_and(|ids| !(1..=8).contains(&ids.len()))
    }).map(|(index, _)| json!({"location":format!("/section/claims/{index}/identifiers"),"code":"identifierCount"})).collect();
    if !counts.is_empty() {
        return counts;
    }
    let Some(issues) = crate::claim_alignment::diagnostics(resolved) else {
        return finding("identifierAlignment");
    };
    let mut findings = Vec::new();
    for issue in issues {
        let before = findings.len();
        let location = section_location(&issue);
        let claim = issue["location"]
            .as_str()
            .and_then(|s| s.strip_prefix("/sections/0/claims/"))
            .and_then(|s| s.parse::<usize>().ok())
            .and_then(|index| claims.get(index));
        let Some(identifiers) = claim.and_then(|c| c["identifiers"].as_array()) else {
            return finding("identifierAlignment");
        };
        for (field, code) in [
            ("missingFromProse", "identifierProse"),
            ("missingFromQuotes", "identifierEvidence"),
        ] {
            for missing in issue[field].as_array().into_iter().flatten() {
                if let Some(index) = identifiers
                    .iter()
                    .position(|id| id.as_str().map(str::trim) == missing.as_str())
                {
                    findings.push(
                        json!({"location":format!("{location}/identifiers/{index}"),"code":code}),
                    );
                }
            }
        }
        for index in issue["quotesWithoutDeclaredIdentifier"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_u64)
            .filter(|index| *index < 8)
        {
            findings.push(json!({"location":format!("{location}/evidence/{index}"),"code":"evidenceIdentifier"}));
        }
        if findings.len() == before {
            findings.extend(finding("identifierAlignment"));
        }
    }
    findings.truncate(64);
    findings
}

/// Field positions are sufficient to repair a malformed visual; never echo
/// model strings, repository contents or raw decoder errors into tool feedback.
fn visual_shape_findings(section: &Value) -> Vec<Value> {
    let Some(visual) = section.get("visual").and_then(Value::as_object) else {
        return vec![json!({"location":"/section/visual","code":"visualField"})];
    };
    ["kind", "title", "direction", "nodes", "edges"]
        .into_iter()
        .filter_map(|field| {
            let valid = visual.get(field).is_some_and(|value| match field {
                "nodes" | "edges" => value.is_array(),
                _ => value.is_string(),
            });
            (!valid).then(
                || json!({"location":format!("/section/visual/{field}"),"code":"visualField"}),
            )
        })
        .collect()
}

/// A repeated reference within one item is rejected by the canonicalizer.
/// Report its exact position so repairing it need not replace the visual.
fn visual_evidence_findings(section: &Value) -> Vec<Value> {
    let mut findings = Vec::new();
    for field in ["nodes", "edges"] {
        for (item_index, item) in section["visual"][field]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let mut seen = BTreeSet::new();
            for (reference_index, reference) in item["evidence"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                let pair = reference["claimIndex"]
                    .as_u64()
                    .zip(reference["evidenceIndex"].as_u64());
                if pair.is_some_and(|pair| !seen.insert(pair)) {
                    findings.push(json!({"location":format!("/section/visual/{field}/{item_index}/evidence/{reference_index}"),"code":"visualEvidenceDuplicate"}));
                    if findings.len() == 64 {
                        return findings;
                    }
                }
            }
        }
    }
    findings
}

/// New checkpoints bind the new scene policy; old immutable checkpoints keep
/// the policy under which they were published. Omitted presentations are valid.
fn scene_version_findings(section: &Value, checkpoint_version: u32) -> Vec<Value> {
    let Some(presentation) = section["visual"].get("presentation") else { return vec![]; };
    let supported = match checkpoint_version {
        1 | 2 => presentation.is_null() || matches!(presentation["schemaVersion"].as_u64(), Some(1 | 2)),
        3 => presentation["schemaVersion"].as_u64() == Some(2),
        4 => presentation["schemaVersion"].as_u64() == Some(3),
        _ => false,
    };
    if !supported {
        vec![json!({"location":"/section/visual/presentation/schemaVersion","code":"sceneVersion"})]
    } else {
        vec![]
    }
}

/// Fixed positions only. Full canonical validation still owns admission; this
/// never invents speech, removes claims or grants another attempt.
fn scene_narration_findings(section: &Value) -> Vec<Value> {
    let presentation = &section["visual"]["presentation"];
    if presentation["schemaVersion"].as_u64() != Some(3) { return vec![]; }
    let Some(scenes) = presentation["scenes"].as_array().filter(|s| s.len() <= 20) else { return vec![]; };
    let mut findings = Vec::new();
    let mut total = 0;
    for (index, scene) in scenes.iter().enumerate() {
        match scene["narration"].as_str() {
            Some(text) if !text.trim().is_empty() && text.len() <= 4_000 => {
                total += text.split_whitespace().collect::<Vec<_>>().join(" ").len() + usize::from(index > 0);
            }
            _ => findings.push(json!({"location":format!("/section/visual/presentation/scenes/{index}/narration"),"code":"sceneNarration"})),
        }
    }
    if findings.is_empty() && !(20..=4_000).contains(&total) {
        findings.push(json!({"location":"/section/visual/presentation/scenes","code":"sceneNarration"}));
    }
    findings
}

fn scene_repetition_findings(section: &Value) -> Vec<Value> {
    let visual = &section["visual"];
    let Some(nodes) = visual["nodes"].as_array().filter(|nodes| nodes.len() <= 12) else {
        return vec![];
    };
    let Some(labels) = nodes
        .iter()
        .map(|node| node["label"].as_str())
        .collect::<Option<Vec<_>>>()
    else {
        return vec![];
    };
    let Ok(presentation) = serde_json::from_value(visual["presentation"].clone()) else {
        return vec![];
    };
    crate::scene_presentation::repetition_positions(&presentation, &labels).into_iter()
        .map(|(scene, element)| json!({"location":format!("/section/visual/presentation/scenes/{scene}/elements/{element}"),"code":"sceneRepetition"}))
        .collect()
}

/// Only called after native citation resolution and failed canonicalization.
/// Exact same-claim quotes explain a known failure; this never admits a value
/// or returns source strings as repair feedback.
fn scene_source_value_findings(section: &Value, resolved: &str) -> Vec<Value> {
    use crate::scene_presentation::{SceneMode, ScenePresentation, ValueOrigin};
    let visual = &section["visual"];
    let Some(nodes) = visual["nodes"].as_array().filter(|nodes| nodes.len() <= 12) else {
        return vec![];
    };
    let Ok(presentation) =
        serde_json::from_value::<ScenePresentation>(visual["presentation"].clone())
    else {
        return vec![];
    };
    let Ok(resolved) = serde_json::from_str::<Value>(resolved) else {
        return vec![];
    };
    let Some(claims) = resolved["sections"][0]["claims"].as_array() else {
        return vec![];
    };
    let mut findings = Vec::new();
    for (scene_index, scene) in presentation.scenes.iter().take(20).enumerate() {
        if scene.mode != SceneMode::Composition || scene.claim_index != scene_index {
            continue;
        }
        for (element_index, element) in scene.elements.iter().take(48).enumerate() {
            if element.value_origin != ValueOrigin::Source
                || element.value.trim().is_empty()
                || element.value.len() > 1024
            {
                continue;
            }
            let Some(references) = element
                .node_index
                .and_then(|i| nodes.get(i))
                .and_then(|node| node["evidence"].as_array())
                .filter(|refs| refs.len() <= 8)
            else {
                continue;
            };
            let quotes: Vec<_> = references
                .iter()
                .filter_map(|reference| {
                    if reference["claimIndex"].as_u64()? != scene_index as u64 {
                        return None;
                    }
                    let evidence_index =
                        usize::try_from(reference["evidenceIndex"].as_u64()?).ok()?;
                    claims.get(scene_index)?["evidence"]
                        .as_array()?
                        .get(evidence_index)?["quote"]
                        .as_str()
                })
                .collect();
            if !quotes.is_empty() && !quotes.iter().any(|quote| quote.contains(&element.value)) {
                findings.push(json!({"location":format!("/section/visual/presentation/scenes/{scene_index}/elements/{element_index}/value"),"code":"sceneSourceValue"}));
                if findings.len() == 64 {
                    return findings;
                }
            }
        }
    }
    findings
}

/// Diagnose a known scene failure only after native admission has rejected it.
/// This does not resolve evidence or authorize a visual; the canonicalizer
/// remains authoritative for node references and the complete presentation.
fn scene_evidence_findings(section: &Value) -> Vec<Value> {
    crate::scene_presentation::foreign_claim_positions(section).into_iter()
        .map(|(scene, element)| json!({"location":format!("/section/visual/presentation/scenes/{scene}/elements/{element}/nodeIndex"),"code":"sceneEvidence"}))
        .collect()
}

fn valid_findings(findings: &[Value]) -> bool {
    findings.len() <= 64
        && findings.iter().all(|v| {
            v.as_object().is_some_and(|o| o.len() == 2)
                && v["location"].as_str().is_some_and(|s| {
                    s.len() <= 160 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'/')
                })
                && v["code"].as_str().is_some_and(|s| {
                    matches!(
                        s,
                        "proposalShape"
                            | "sectionKeyFormat"
                            | "sectionOrder"
                            | "sectionConflict"
                            | "sectionLimit"
                            | "citation"
                            | "identifierAlignment"
                            | "identifierCount"
                            | "identifierProse"
                            | "identifierEvidence"
                            | "evidenceIdentifier"
                            | "presentationLimit"
                            | "visualField"
                            | "visualEvidenceDuplicate"
                            | "sceneEvidence"
                            | "sceneVersion"
                            | "sceneNarration"
                            | "sceneRepetition"
                            | "sceneSourceValue"
                            | "sectionValidation"
                    )
                })
        })
}

impl Ledger {
    pub(crate) fn start_answer_sections(
        &self,
        question: &str,
        request_sha256: &str,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<ActiveAnswer, String> {
        self.check(manifest, runtime, cancelled)?;
        if self.record.schema_version != 3
            || !hash(request_sha256)
            || question.trim().is_empty()
            || question.len() > 12_000
        {
            return Err("The answer intent or request identity is invalid.".into());
        }
        let path = record_path(&self.storage, &self.record.binding.operation_id)?;
        let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
        check_capacity(path.parent().ok_or("Answer storage is unavailable.")?)?;
        let value = Answer {
            schema_version: 2,
            binding: self.record.binding.clone(),
            request_sha256: request_sha256.into(),
            question: question.into(),
            outcome: Outcome::Working,
            sections: vec![],
            attempts: vec![],
            final_responses: vec![],
            incomplete_topics: vec![],
            failure_code: None,
        };
        self.check(manifest, runtime, cancelled)?;
        codex_agent::create_private_file(&path, &encode_answer(&value)?)?;
        Ok(ActiveAnswer { path, value })
    }
}

impl Answer {
    fn proposal_version(&self) -> &'static str {
        if self.schema_version == 2 {
            PROPOSAL_VERSION
        } else {
            LEGACY_PROPOSAL_VERSION
        }
    }

    fn unresolved_proposals(&self) -> bool {
        self.attempts.iter().enumerate().any(|(i, attempt)| {
            if attempt.findings.is_empty() {
                return false;
            }
            let key = crate::strict_json::from_slice(attempt.arguments.as_bytes())
                .ok()
                .and_then(|v| {
                    v.get("sectionKey")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                });
            !key.is_some_and(|key| {
                self.attempts[i + 1..].iter().any(|later| {
                    later.generation_id.as_ref().is_some_and(|id| {
                        self.sections
                            .iter()
                            .any(|s| &s.preview.generation_id == id && s.key == key)
                    })
                })
            })
        })
    }
}

impl ActiveAnswer {
    pub(crate) fn finish_manifest(
        &mut self,
        text: &str,
        budget_stopped: bool,
    ) -> Result<(), String> {
        // Preserve the exact native-owned final response, including malformed
        // manifests, before evaluating it. No provider retry is started here.
        if self.value.outcome != Outcome::Working || text.len() > 300 * 1024 {
            return Err("The answer's final response is unavailable or exceeds its limit.".into());
        }
        if !self.value.final_responses.iter().any(|r| r.text == text) {
            if self.value.final_responses.len() >= 4 {
                return Err("The answer's final response record reached its limit.".into());
            }
            let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
            if read_bytes(&self.path)? != encode_answer(&self.value)? {
                return Err("The saved answer changed; no final response was appended.".into());
            }
            let mut next = self.value.clone();
            next.final_responses.push(FinalResponse {
                text: text.into(),
                sha256: context::digest(text.as_bytes()),
            });
            codex_agent::atomic_private_write(&self.path, &encode_answer(&next)?)?;
            self.value = next;
        }
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Manifest {
            manifest_version: String,
            sections: Vec<reading::SectionIdentity>,
            incomplete_topics: Vec<String>,
        }
        if text.len() > 4096 {
            return Err("The answer's final manifest exceeds its limit.".into());
        }
        let manifest: Manifest = crate::strict_json::from_slice(text.as_bytes())
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .ok_or("The answer's final manifest is invalid. Published sections are preserved.")?;
        if manifest.manifest_version != "answerManifestV1"
            || manifest.sections != self.value.latest_identities()
            || manifest.sections.is_empty()
        {
            return Err(
                "The answer's final manifest does not match its published sections.".into(),
            );
        }
        let mut topics = manifest.incomplete_topics;
        if topics.len() > 6
            || topics
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 240 || s.chars().any(char::is_control))
        {
            return Err("The answer's unfinished topics are invalid.".into());
        }
        // A rejected proposal is not proof that its intended subject was
        // completed. Only a later accepted proposal for that same key repairs
        // it; malformed attempts without a key remain unresolved.
        let unresolved = self.value.unresolved_proposals();
        if (budget_stopped || unresolved) && topics.is_empty() {
            topics.push("Some planned details could not be completed within this investigation. Published sections remain available.".into());
        }
        self.finish(
            if topics.is_empty() {
                Outcome::Complete
            } else {
                Outcome::Partial
            },
            topics,
        )
    }

    pub(crate) fn identity(&self) -> (&str, &str) {
        (&self.value.binding.operation_id, &self.value.request_sha256)
    }

    pub(crate) fn remaining_attempts(&self) -> usize {
        MAX_ATTEMPTS.saturating_sub(self.value.attempts.len())
    }

    pub(crate) fn notice(&self, generation: &str) -> Option<SectionNotice> {
        self.value
            .sections
            .iter()
            .find(|s| s.preview.generation_id == generation)
            .map(|s| SectionNotice {
                run_id: self.value.binding.operation_id.clone(),
                project_id: self.value.binding.project_id.clone(),
                snapshot_id: self.value.binding.snapshot_id.clone(),
                key: s.key.clone(),
                order: s.order,
                generation_id: s.preview.generation_id.clone(),
                revision_id: s.preview.revision.revision_id.clone(),
                supersedes: s.supersedes.clone(),
                context_elapsed_ms: s.published_elapsed_ms,
                run_elapsed_ms: s.published_run_elapsed_ms,
            })
    }

    pub(crate) fn prepare(
        &self,
        ledger: &Ledger,
        arguments: &str,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<PreparedSection, String> {
        self.prepare_with_checkpoint_version(
            ledger,
            arguments,
            manifest,
            runtime,
            cancelled,
            CURRENT_CHECKPOINT_VERSION,
        )
    }

    fn prepare_with_checkpoint_version(
        &self,
        ledger: &Ledger,
        arguments: &str,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
        checkpoint_version: u32,
    ) -> Result<PreparedSection, String> {
        if !matches!(checkpoint_version, 1 | 2 | 3 | 4) {
            return Err("Unsupported answer presentation version.".into());
        }
        ledger.check(manifest, runtime, cancelled)?;
        if self.value.outcome != Outcome::Working
            || !same_binding(&self.value.binding, &ledger.record.binding)?
            || self.value.attempts.len() >= MAX_ATTEMPTS
        {
            return Err("This answer stopped or reached its eight-proposal limit.".into());
        }
        if arguments.len() > MAX_PROPOSAL_BYTES {
            return Err("The section proposal exceeds its 32 KiB limit.".into());
        }
        let parsed = crate::strict_json::from_slice(arguments.as_bytes()).ok();
        let arguments_sha256 = parsed
            .as_ref()
            .map(crate::agent_context::arguments_digest)
            .transpose()?
            .unwrap_or_else(|| context::digest(arguments.as_bytes()));
        let proposal_sha256 = context::digest(arguments.as_bytes());
        let mut next = self.value.clone();
        let mut generation_id = None;
        let mut findings = finding("proposalShape");
        if let Some(proposal) = parsed.and_then(|v| serde_json::from_value::<Proposal>(v).ok()) {
            if !valid_key(&proposal.section_key) {
                findings = vec![json!({"location":"/sectionKey","code":"sectionKeyFormat"})];
            } else if proposal.proposal_version == self.value.proposal_version()
                && proposal.order < MAX_SECTIONS
            {
                let prior = self
                    .value
                    .sections
                    .iter()
                    .rev()
                    .find(|s| s.key == proposal.section_key);
                let unique = self
                    .value
                    .sections
                    .iter()
                    .map(|s| &s.key)
                    .collect::<BTreeSet<_>>()
                    .len();
                if let Some(prior) = prior.filter(|p| p.proposal_sha256 == proposal_sha256) {
                    // An exact repeated proposal identifies its existing durable
                    // section; it never regenerates IDs or duplicates the section.
                    generation_id = Some(prior.preview.generation_id.clone());
                    findings.clear();
                } else if prior.is_some_and(|p| {
                    proposal.previous_generation_id.as_deref()
                        != Some(p.preview.generation_id.as_str())
                        || p.order != proposal.order
                }) || (prior.is_none() && proposal.previous_generation_id.is_some())
                {
                    findings = finding("sectionConflict");
                } else if prior.is_none() && unique >= MAX_SECTIONS {
                    findings = finding("sectionLimit");
                } else if prior.is_none() && proposal.order != unique {
                    findings = finding("sectionOrder");
                } else if let Ok(response) = wrapped(&proposal) {
                    match ledger.resolve_citations(&response, manifest, runtime, cancelled)? {
                        native_citations::Resolution::Invalid(issues) => findings = issues.iter().take(64).map(|issue| json!({"location":section_location(issue),"code":"citation"})).collect(),
                        native_citations::Resolution::Ready(resolved) => {
                            let mut alignment = alignment_findings(&proposal.section, resolved.text());
                            alignment.extend(visual_shape_findings(&proposal.section));
                            alignment.extend(scene_version_findings(&proposal.section, checkpoint_version));
                            if checkpoint_version >= 4 { alignment.extend(scene_narration_findings(&proposal.section)); }
                            alignment.truncate(64);
                            if !alignment.is_empty() {
                                findings = alignment;
                            } else {
                                let sources =
                                    ledger.validation_sources(manifest, runtime, cancelled)?;
                                let syntax = crate::index::load_snapshot_index(
                                    &ledger.storage,
                                    &manifest.summary.project_id,
                                    &manifest.summary.snapshot_id,
                                )
                                .unwrap_or(None);
                                let canonicalize = if checkpoint_version == 1 {
                                    chapter_generation::canonicalize_cited_workspace_overview
                                } else if checkpoint_version >= 4 {
                                    chapter_generation::canonicalize_narrated_answer_section
                                } else {
                                    chapter_generation::canonicalize_complete_answer_section
                                };
                                match canonicalize(
                                    &manifest.summary.project_id,
                                    manifest,
                                    syntax.as_ref(),
                                    &sources,
                                    &resolved,
                                ) {
                                    Err(error) if error == chapter_generation::COMPLETE_CLAIMS_LIMIT_ERROR => findings = finding("presentationLimit"),
                                    Err(_) => {
                                        findings = visual_evidence_findings(&proposal.section);
                                        findings.extend(scene_evidence_findings(&proposal.section));
                                        findings.extend(scene_repetition_findings(&proposal.section));
                                        findings.extend(scene_source_value_findings(&proposal.section, resolved.text()));
                                        findings.truncate(64);
                                        if findings.is_empty() {
                                            findings = finding("sectionValidation");
                                        }
                                    },
                                    Ok(mut revision) => {
                                        ledger.verify_revision(
                                            &revision, manifest, runtime, cancelled,
                                        )?;
                                        let validation = chapters::validate_chapter_revision(
                                            &ledger.storage,
                                            manifest,
                                            syntax.as_ref(),
                                            &revision,
                                        );
                                        if validation.status
                                            != chapters::ValidationReportStatus::Valid
                                        {
                                            findings = finding("sectionValidation");
                                        } else {
                                            let checkpoint = checkpoint(ledger)?;
                                            let checkpoint_sha256 =
                                                context::digest(&encode(&checkpoint)?);
                                            revision.generation_origin = Some(chapter_origin::ChapterGenerationOrigin::from_answer_checkpoint(
                                                &self.value.binding.operation_id, &self.value.request_sha256,
                                                &checkpoint_sha256, record_ranges(&checkpoint), &sources, &revision,
                                            )?);
                                            revision.validation_status =
                                                chapters::ValidationStatus::Valid;
                                            let id = format!("generation:{}", Uuid::new_v4());
                                            next.sections.push(PublishedSection {
                                                checkpoint_version,
                                                key: proposal.section_key,
                                                order: proposal.order,
                                                supersedes: proposal.previous_generation_id,
                                                published_elapsed_ms: ledger.context_elapsed_ms(),
                                                published_run_elapsed_ms: None,
                                                proposal_sha256: proposal_sha256.clone(),
                                                checkpoint_sha256,
                                                checkpoint,
                                                preview: chapters::ChapterDraftPreview {
                                                    generation_id: id.clone(),
                                                    revision,
                                                    validation,
                                                    model_id: ledger.record.binding.model.clone(),
                                                    model_version: None,
                                                    input_tokens: None,
                                                    output_tokens: None,
                                                    update_notes: vec![],
                                                },
                                            });
                                            generation_id = Some(id);
                                            findings.clear();
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        next.attempts.push(Attempt {
            arguments: arguments.into(),
            arguments_sha256,
            proposal_sha256,
            findings: findings.clone(),
            generation_id: generation_id.clone(),
        });
        encode_answer(&next)?;
        ledger.check(manifest, runtime, cancelled)?;
        Ok(PreparedSection {
            before_sha256: context::digest(&encode_answer(&self.value)?),
            next,
            generation_id,
            findings,
        })
    }

    /// The caller's native operation publication lock must cover this complete
    /// method. It must not send a model reply or renderer event before success.
    pub(crate) fn commit(
        &mut self,
        ledger: &Ledger,
        prepared: PreparedSection,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<(), String> {
        let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
        ledger.check(manifest, runtime, cancelled)?;
        if prepared.before_sha256 != context::digest(&encode_answer(&self.value)?)
            || prepared.before_sha256 != context::digest(&read_bytes(&self.path)?)
            || !same_binding(&prepared.next.binding, &ledger.record.binding)?
            || self.value.outcome != Outcome::Working
        {
            return Err(
                "The answer publication owner changed. Existing sections were preserved.".into(),
            );
        }
        codex_agent::atomic_private_write(&self.path, &encode_answer(&prepared.next)?)?;
        self.value = prepared.next;
        Ok(())
    }

    /// Caller still holds the native publication lock after section commit.
    /// An absent observation means the commit was not successfully announced.
    pub(crate) fn record_publication_time(
        &mut self,
        generation: &str,
        elapsed: u64,
    ) -> Result<(), String> {
        let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
        if self.value.outcome != Outcome::Working
            || elapsed > 600_000
            || read_bytes(&self.path)? != encode_answer(&self.value)?
        {
            return Err("The section publication observation could not be saved.".into());
        }
        let mut next = self.value.clone();
        let section = next
            .sections
            .iter_mut()
            .find(|s| s.preview.generation_id == generation)
            .ok_or("The publication observation names an unknown section.")?;
        if section.published_run_elapsed_ms.is_some() {
            return Ok(());
        }
        section.published_run_elapsed_ms = Some(elapsed);
        codex_agent::atomic_private_write(&self.path, &encode_answer(&next)?)?;
        self.value = next;
        Ok(())
    }

    pub(crate) fn finish(
        &mut self,
        outcome: Outcome,
        incomplete_topics: Vec<String>,
    ) -> Result<(), String> {
        if self.value.outcome != Outcome::Working
            || outcome == Outcome::Working
            || incomplete_topics.len() > 6
            || incomplete_topics
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 240 || s.chars().any(char::is_control))
            || (outcome == Outcome::Complete
                && (self.value.sections.is_empty()
                    || !incomplete_topics.is_empty()
                    || self.value.unresolved_proposals()))
            || (outcome == Outcome::Partial && self.value.sections.is_empty())
            || (outcome == Outcome::Failed && !self.value.sections.is_empty())
        {
            return Err("The answer terminal outcome is invalid.".into());
        }
        let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
        if read_bytes(&self.path)? != encode_answer(&self.value)? {
            return Err("The answer terminal owner changed; existing work was preserved.".into());
        }
        let mut next = self.value.clone();
        next.outcome = outcome;
        next.incomplete_topics = incomplete_topics;
        codex_agent::atomic_private_write(&self.path, &encode_answer(&next)?)?;
        self.value = next;
        Ok(())
    }
}

fn same_binding(left: &Binding, right: &Binding) -> Result<bool, String> {
    Ok(
        serde_json::to_vec(left).map_err(|_| "Answer identity is unavailable.")?
            == serde_json::to_vec(right).map_err(|_| "Answer identity is unavailable.")?,
    )
}

fn normalized_revision(revision: &chapters::ChapterRevision) -> Result<Value, String> {
    let mut revision = revision.clone();
    revision.created_at_unix_ms = 0;
    revision.generation_origin = None;
    revision.validation_status = chapters::ValidationStatus::Unvalidated;
    let mut value =
        serde_json::to_value(revision).map_err(|_| "The section could not be inspected.")?;
    fn visit(value: &mut Value, ids: &mut HashMap<String, String>) {
        match value {
            Value::Object(object) => {
                for (key, value) in object.iter_mut() {
                    if matches!(
                        key.as_str(),
                        "revisionId"
                            | "sectionId"
                            | "sectionRevisionId"
                            | "claimId"
                            | "anchorId"
                            | "hotspotId"
                            | "nodeId"
                            | "edgeId"
                            | "fromNodeId"
                            | "toNodeId"
                    ) {
                        if let Some(id) = value.as_str() {
                            let next = format!("native-id:{}", ids.len());
                            let normalized = ids.entry(id.to_owned()).or_insert(next).clone();
                            *value = json!(normalized);
                        }
                    } else if key == "evidenceAnchorIds" {
                        if let Value::Array(items) = value {
                            for item in items {
                                if let Some(id) = item.as_str() {
                                    let next = format!("native-id:{}", ids.len());
                                    *item = json!(ids.entry(id.to_owned()).or_insert(next).clone());
                                }
                            }
                        }
                    } else {
                        visit(value, ids);
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    visit(item, ids);
                }
            }
            _ => {}
        }
    }
    visit(&mut value, &mut HashMap::new());
    Ok(value)
}

/// Full local replay of native admission, without an active ledger, model call
/// or current sharing grant. Exact immutable checkpoints are self-contained;
/// later ledger appends, revocation or cleanup do not alter their delivered code.
pub(crate) fn inspect(storage: &Path, operation: &str) -> Result<Answer, String> {
    let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
    let value: Answer =
        crate::strict_json::from_slice(&read_bytes(&record_path(storage, operation)?)?)
            .map_err(|_| "Invalid saved answer".to_owned())
            .and_then(|v| serde_json::from_value(v).map_err(|_| "Invalid saved answer".to_owned()))
            .map_err(|_| "The saved answer is damaged; it was preserved.")?;
    if !matches!(value.schema_version, 1 | 2)
        || value.binding.operation_id != operation
        || !value.binding.runtime.valid_shape()
        || value.binding.model != "gpt-6-luna"
        || value.binding.effort != "max"
        || !canonical_uuid(&value.binding.grant_id)
        || !hash(&value.binding.manifest_sha256)
        || !hash(&value.request_sha256)
        || value.question.trim().is_empty()
        || value.question.len() > 12_000
        || value.attempts.len() > MAX_ATTEMPTS
        || value.final_responses.len() > 4
        || value
            .final_responses
            .iter()
            .any(|r| r.text.len() > 300 * 1024 || r.sha256 != context::digest(r.text.as_bytes()))
        || value.sections.len() > MAX_ATTEMPTS
        || value
            .sections
            .iter()
            .map(|s| &s.key)
            .collect::<BTreeSet<_>>()
            .len()
            > MAX_SECTIONS
        || value
            .sections
            .iter()
            .map(|s| &s.preview.generation_id)
            .collect::<BTreeSet<_>>()
            .len()
            != value.sections.len()
        || (value.outcome == Outcome::Working && !value.incomplete_topics.is_empty())
        || (matches!(value.outcome, Outcome::Working | Outcome::Complete)
            && value.failure_code.is_some())
        || (value.failure_code == Some(FailureCode::Stopped) && value.outcome != Outcome::Stopped)
        || value.incomplete_topics.len() > 6
        || value
            .incomplete_topics
            .iter()
            .any(|s| s.trim().is_empty() || s.len() > 240 || s.chars().any(char::is_control))
        || (value.outcome == Outcome::Complete
            && (value.sections.is_empty()
                || !value.incomplete_topics.is_empty()
                || value.unresolved_proposals()))
        || (value.outcome == Outcome::Partial && value.sections.is_empty())
        || (value.outcome == Outcome::Failed && !value.sections.is_empty())
    {
        return Err("The saved answer has inconsistent identities or bounds.".into());
    }
    for attempt in &value.attempts {
        let parsed = crate::strict_json::from_slice(attempt.arguments.as_bytes()).ok();
        let expected = parsed
            .as_ref()
            .map(crate::agent_context::arguments_digest)
            .transpose()?
            .unwrap_or_else(|| context::digest(attempt.arguments.as_bytes()));
        if attempt.arguments.len() > MAX_PROPOSAL_BYTES
            || attempt.arguments_sha256 != expected
            || attempt.proposal_sha256 != context::digest(attempt.arguments.as_bytes())
            || !valid_findings(&attempt.findings)
            || attempt.generation_id.as_ref().is_some_and(|id| {
                !attempt.findings.is_empty()
                    || !value.sections.iter().any(|s| {
                        &s.preview.generation_id == id
                            && s.proposal_sha256 == attempt.proposal_sha256
                    })
            })
            || (attempt.generation_id.is_none() && attempt.findings.is_empty())
        {
            return Err("A saved section proposal is inconsistent.".into());
        }
    }
    let manifest = snapshot::load_snapshot_manifest(
        storage,
        &value.binding.project_id,
        &value.binding.snapshot_id,
    )?;
    if manifest_hash(&manifest)? != value.binding.manifest_sha256 {
        return Err("The saved answer belongs to another captured manifest.".into());
    }
    let syntax = crate::index::load_snapshot_index(
        storage,
        &value.binding.project_id,
        &value.binding.snapshot_id,
    )
    .unwrap_or(None);
    for (index, section) in value.sections.iter().enumerate() {
        let checkpoint = &section.checkpoint;
        validate_record(checkpoint, operation)?;
        let prior = value.sections[..index]
            .iter()
            .rev()
            .find(|s| s.key == section.key);
        let expected_order = value.sections[..index]
            .iter()
            .map(|s| &s.key)
            .collect::<BTreeSet<_>>()
            .len();
        if !matches!(section.checkpoint_version, 1 | 2 | 3 | 4)
            || checkpoint.schema_version != 3
            || !same_binding(&value.binding, &checkpoint.binding)?
            || checkpoint.status != Status::Active
            || checkpoint
                .calls
                .iter()
                .any(|c| !matches!(c.state, CallState::Delivered | CallState::Rejected))
            || manifest_hash(&manifest)? != checkpoint.binding.manifest_sha256
            || section.checkpoint_sha256 != context::digest(&encode(checkpoint)?)
            || !valid_key(&section.key)
            || section.order >= MAX_SECTIONS
            || section.published_elapsed_ms > MAX_CONTEXT_ELAPSED_MS
            || section
                .published_run_elapsed_ms
                .is_some_and(|ms| ms > 600_000)
            || prior.map(|s| s.preview.generation_id.as_str()) != section.supersedes.as_deref()
            || prior.map_or(section.order != expected_order, |p| {
                p.order != section.order
            })
            || !section
                .preview
                .generation_id
                .strip_prefix("generation:")
                .is_some_and(canonical_uuid)
            || section.preview.model_id != value.binding.model
            || section.preview.model_version.is_some()
            || section.preview.input_tokens.is_some()
            || section.preview.output_tokens.is_some()
            || !section.preview.update_notes.is_empty()
            || section.preview.revision.lifecycle != chapters::RevisionLifecycle::Draft
            || section.preview.revision.validation_status != chapters::ValidationStatus::Valid
        {
            return Err(
                "The section checkpoint has an inconsistent identity or delivery prefix.".into(),
            );
        }
        let attempt = value
            .attempts
            .iter()
            .find(|a| {
                a.proposal_sha256 == section.proposal_sha256
                    && a.generation_id.as_deref() == Some(section.preview.generation_id.as_str())
            })
            .ok_or("The section has no owned proposal.")?;
        let proposal: Proposal = serde_json::from_str(&attempt.arguments)
            .map_err(|_| "The published proposal is damaged.")?;
        if proposal.proposal_version != value.proposal_version()
            || proposal.section_key != section.key
            || proposal.order != section.order
            || proposal.previous_generation_id != section.supersedes
        {
            return Err("The section differs from its owned proposal.".into());
        }
        let sources = record_ranges(checkpoint)
            .iter()
            .map(|r| r.path.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|p| context::source(storage, &manifest, &p))
            .collect::<Result<Vec<_>, _>>()?;
        let sources_by_path: HashMap<_, _> = sources.iter().map(|s| (s.path.as_str(), s)).collect();
        let mut citations = HashMap::new();
        for (index, evidence) in checkpoint.evidence.iter().enumerate() {
            if !checkpoint
                .calls
                .iter()
                .any(|c| c.state == CallState::Delivered && c.evidence_ids.contains(&evidence.id))
            {
                continue;
            }
            let source = sources_by_path
                .get(evidence.range.path.as_str())
                .ok_or("The section source is unavailable.")?;
            if source
                .contents
                .get(evidence.range.start_byte..evidence.range.end_byte)
                .is_some_and(native_citations::citeable)
            {
                let citation = native_citations::Citation::checked(&evidence.range, source)?;
                citations.insert(evidence.id.clone(), citation.clone());
                if value.schema_version == 2 {
                    citations.insert(native_citations::short_reference(index), citation);
                }
            }
        }
        let resolved = match native_citations::resolve(&wrapped(&proposal)?, |id| {
            citations.get(id).cloned().ok_or("referenceUnavailable")
        }) {
            native_citations::Resolution::Ready(value) => value,
            native_citations::Resolution::Invalid(_) => {
                return Err("The section cites code outside its delivered checkpoint.".into());
            }
        };
        if !scene_version_findings(&proposal.section, section.checkpoint_version).is_empty() {
            return Err("The saved scene does not match its checkpoint policy.".into());
        }
        let canonicalize = if section.checkpoint_version == 1 {
            chapter_generation::canonicalize_cited_workspace_overview
        } else if section.checkpoint_version >= 4 {
            chapter_generation::canonicalize_narrated_answer_section
        } else {
            chapter_generation::canonicalize_complete_answer_section
        };
        let rebuilt = canonicalize(
            &value.binding.project_id,
            &manifest,
            syntax.as_ref(),
            &sources,
            &resolved,
        )?;
        if normalized_revision(&rebuilt)? != normalized_revision(&section.preview.revision)? {
            return Err("The saved section differs from its canonical proposal.".into());
        }
        verify_record_revision(
            storage,
            checkpoint,
            &section.preview.revision,
            &manifest,
            || Ok(()),
        )?;
        let validation = chapters::validate_chapter_revision(
            storage,
            &manifest,
            syntax.as_ref(),
            &section.preview.revision,
        );
        if validation.status != chapters::ValidationReportStatus::Valid
            || validation != section.preview.validation
        {
            return Err("The saved section no longer passes independent validation.".into());
        }
        let origin = section
            .preview
            .revision
            .generation_origin
            .as_ref()
            .ok_or("The section lacks its native origin.")?;
        let origin_json =
            serde_json::to_value(origin).map_err(|_| "The section origin is unavailable.")?;
        if origin.schema_version != 5
            || origin.contract_version != PROGRESSIVE_CONTRACT
            || !origin.matches_content(&section.preview.revision)
            || origin.request_id != operation
            || origin.request_body_sha256 != value.request_sha256
            || origin_json["retrieval"]["ledgerSha256"] != section.checkpoint_sha256
            || origin_json["retrieval"]["ranges"]
                != serde_json::to_value(record_ranges(checkpoint))
                    .map_err(|_| "The section ranges are unavailable.")?
        {
            return Err("The section origin does not match its evidence checkpoint.".into());
        }
    }
    Ok(value)
}

#[cfg(test)]
pub(crate) mod tests;

#[cfg(test)]
mod rejected_inspection_tests;
