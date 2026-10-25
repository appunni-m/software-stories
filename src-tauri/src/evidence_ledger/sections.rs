//! Durable, independently checked draft sections. Inspection never recreates
//! an active investigation or acquires source-sharing permission.

use super::*;
use crate::{chapter_generation, chapter_origin, native_citations};
use serde_json::{Value, json};
use std::sync::Mutex;

pub(crate) const MAX_PROPOSAL_BYTES: usize = 32_768;
pub(crate) const NARRATION_PROPOSAL_VERSION: &str = "answerSectionV6";
pub(crate) const NARRATION_ANSWER_VERSION: u32 = 7;
pub(crate) const NARRATION_CHECKPOINT_VERSION: u32 = 8;
pub(crate) const NATIVE_MANIFEST_VERSION: &str = "answerManifestV2";
pub(crate) const NATIVE_MANIFEST_POLICY: &str = "all-latest-published-sections-v1";
const MAX_SECTIONS: usize = 6;
const MAX_ATTEMPTS: usize = 8;
// Six first proposals plus two shared correction attempts. Exact successful
// replays still count as proposals, but do not consume a correction attempt.
const MAX_CORRECTIONS: usize = MAX_ATTEMPTS - MAX_SECTIONS;
const MAX_CORRECTIONS_PER_KEY: usize = 1;
// A guide reserves its full bounded lifetime allowance and standalone
// receipts against one total, so adding slides never multiplies storage
// without a global cap.
const MAX_ANSWER_RECORDS: usize = 1_000;
const MAX_RECORD_BYTES: usize = 4 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = MAX_ANSWER_RECORDS + 32;
static STORE: Mutex<()> = Mutex::new(());

pub(crate) mod authoring;
pub(crate) mod capacity;
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
    FinalManifestTooLarge,
    InvalidFinalManifest,
    UnsupportedManifestVersion,
    NoPublishedSections,
    InvalidIncompleteTopics,
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
            "The Codex run exceeded ten minutes and was stopped. Review again before retrying."
            | "The Codex operation exceeded ten minutes, including result validation. No late response was published."
            | "The Codex operation exceeded ten minutes before dispatch. Prepare a fresh review."
            | "The Codex operation exceeded ten minutes before creating a thread. No generation was started."
            | "The Codex operation exceeded ten minutes before starting the model turn. No generation was started." => {
                Self::RunTimeout
            }
            error if error == crate::codex_identity::changed() => Self::RuntimeIdentity,
            "The answer's final manifest exceeds its limit." => Self::FinalManifestTooLarge,
            "The answer's final manifest is invalid. Published sections are preserved." => {
                Self::InvalidFinalManifest
            }
            "The answer's final manifest version is unsupported." => {
                Self::UnsupportedManifestVersion
            }
            "The answer did not publish any sections." => Self::NoPublishedSections,
            "The answer's unfinished topics are invalid." => Self::InvalidIncompleteTopics,
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

/// Chapter prose and speech are derived from the canonicalized anchored claims.
/// The placeholders satisfy the native wrapper shape and are never presented.
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
    let has_inference = object["claims"]
        .as_array()
        .is_some_and(|claims| claims.iter().any(|claim| claim["category"] == "inference"));
    object.insert(
        "spokenText".into(),
        json!(if has_inference {
            "Inference: Presentation is derived from the anchored claims."
        } else {
            "Presentation is derived from the anchored claims."
        }),
    );
    if let Some(visual) = object.get_mut("visual").and_then(Value::as_object_mut) {
        if visual.get("kind").and_then(Value::as_str) == Some("none") {
            visual.insert("nodes".into(), json!([]));
            visual.insert("edges".into(), json!([]));
        }
    }
    serde_json::to_string(
        &json!({"citationVersion":native_citations::SHORT_VERSION,"title":title,"sections":[section]}),
    )
    .map_err(|_| "The section could not be encoded.".into())
}

fn finding(code: &str) -> Vec<Value> {
    vec![json!({"location":"/section","code":code})]
}

const MAX_CHAPTER_VALIDATION_FEEDBACK: usize = 16;

/// Return only fixed validator categories. Issue messages and source paths can
/// contain repository data, so they never cross back to the generation agent.
fn chapter_validation_findings(report: &chapters::ChapterValidationReport) -> Vec<Value> {
    let mut seen = std::collections::HashSet::new();
    let findings = report
        .issues
        .iter()
        .filter_map(|issue| {
            let code = match &issue.code {
                chapters::ChapterIssueCode::UnsupportedSchemaVersion => {
                    "chapterUnsupportedSchemaVersion"
                }
                chapters::ChapterIssueCode::InvalidId => "chapterInvalidId",
                chapters::ChapterIssueCode::InvalidRevision => "chapterInvalidRevision",
                chapters::ChapterIssueCode::InvalidTarget => "chapterInvalidTarget",
                chapters::ChapterIssueCode::InvalidSectionOrder => "chapterInvalidSectionOrder",
                chapters::ChapterIssueCode::DuplicateId => "chapterDuplicateId",
                chapters::ChapterIssueCode::InvalidClaim => "chapterInvalidClaim",
                chapters::ChapterIssueCode::WrongSnapshot => "chapterWrongSnapshot",
                chapters::ChapterIssueCode::MissingPath => "chapterMissingPath",
                chapters::ChapterIssueCode::RangeMismatch => "chapterRangeMismatch",
                chapters::ChapterIssueCode::HashMismatch => "chapterHashMismatch",
                chapters::ChapterIssueCode::ExcerptMismatch => "chapterExcerptMismatch",
                chapters::ChapterIssueCode::AmbiguousRemap => "chapterAmbiguousRemap",
                chapters::ChapterIssueCode::MissingSymbol => "chapterMissingSymbol",
                chapters::ChapterIssueCode::InvalidDependency => "chapterInvalidDependency",
                chapters::ChapterIssueCode::InvalidDestination => "chapterInvalidDestination",
                chapters::ChapterIssueCode::InvalidVisual => "chapterInvalidVisual",
                chapters::ChapterIssueCode::IndexUnavailable => "chapterIndexUnavailable",
                chapters::ChapterIssueCode::SourceUnavailable => "chapterSourceUnavailable",
            };
            seen.insert(code)
                .then(|| json!({"location":"/section","code":code}))
        })
        .take(MAX_CHAPTER_VALIDATION_FEEDBACK)
        .collect::<Vec<_>>();

    if findings.is_empty() {
        finding("sectionValidation")
    } else {
        findings
    }
}

/// Resolve authored local identities before native source validation. Raw
/// proposal bytes remain the durable ownership and idempotence identity.
fn wrapped_authored(proposal: &mut Proposal) -> Result<(String, authoring::Compiled), Vec<Value>> {
    if proposal.proposal_version != NARRATION_PROPOSAL_VERSION {
        return Err(finding("proposalShape"));
    }
    let mut value_findings = authored_scene_value_findings(&proposal.section);
    let compiled = match authoring::narration::compile(&proposal.section) {
        Ok(compiled) if value_findings.is_empty() => compiled,
        Ok(_) => return Err(value_findings),
        Err(mut shape_findings) => {
            value_findings.append(&mut shape_findings);
            value_findings.truncate(64);
            return Err(value_findings);
        }
    };
    proposal.section = compiled.section.clone();
    Ok((
        wrapped(proposal).map_err(|_| finding("proposalShape"))?,
        compiled,
    ))
}

/// Report authored scene values before scene compilation can stop at an
/// unrelated shape finding. This lets one repair address both defects.
fn authored_scene_value_findings(section: &Value) -> Vec<Value> {
    let Some(claims) = section["claims"]
        .as_array()
        .filter(|claims| claims.len() <= 8)
    else {
        return vec![];
    };
    let mut findings = Vec::new();
    for (claim_index, claim) in claims.iter().enumerate() {
        let Some(elements) = claim["scene"]["elements"]
            .as_array()
            .filter(|elements| elements.len() <= 48)
        else {
            continue;
        };
        for (element_index, element) in elements.iter().enumerate() {
            let Some(value) = element["value"].as_str() else {
                continue;
            };
            let location =
                format!("/section/claims/{claim_index}/scene/elements/{element_index}/value");
            let code = if value.len() > 1024 {
                Some("scenePresentationLimit")
            } else {
                let invalid_origin = match element["valueOrigin"].as_str() {
                    Some("none") | None => !value.is_empty(),
                    Some("source" | "explanation" | "illustration") => value.trim().is_empty(),
                    Some(_) => false,
                };
                invalid_origin.then_some("sceneSourceValue")
            };
            if let Some(code) = code {
                findings.push(json!({"location":location,"code":code}));
                if findings.len() == 64 {
                    return findings;
                }
            }
        }
    }
    findings
}

/// Resolve every proposal through the current grounded-name policy.
fn resolved_authored(
    resolution: native_citations::Resolution,
) -> Result<native_citations::Resolved, Vec<Value>> {
    match resolution {
        native_citations::Resolution::Invalid(issues) => Err(issues
            .iter()
            .take(64)
            .map(|issue| json!({"location":section_location(issue),"code":"citation"}))
            .collect()),
        native_citations::Resolution::Ready(resolved) => resolved
            .with_grounded_named_identifiers()
            .map_err(|issues| {
                issues
                    .iter()
                    .map(|issue| json!({"location":section_location(issue),"code":issue["code"]}))
                    .collect()
            }),
    }
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

/// Field positions are sufficient to repair a malformed visual; never echo
/// model strings, repository contents or raw decoder errors into tool feedback.
fn visual_shape_findings(section: &Value) -> Vec<Value> {
    let Some(visual) = section.get("visual").and_then(Value::as_object) else {
        return vec![json!({"location":"/section/visual","code":"visualField"})];
    };
    let mut findings = ["kind", "title", "direction", "nodes", "edges"]
        .into_iter()
        .filter_map(|field| {
            let valid = visual.get(field).is_some_and(|value| match field {
                "nodes" | "edges" => value.is_array(),
                "kind" => matches!(
                    value.as_str(),
                    Some("none" | "dataFlow" | "sequence" | "hierarchy")
                ),
                "direction" => {
                    matches!(value.as_str(), Some("auto" | "leftToRight" | "topToBottom"))
                }
                _ => value.is_string(),
            });
            (!valid).then(
                || json!({"location":format!("/section/visual/{field}"),"code":"visualField"}),
            )
        })
        .collect::<Vec<_>>();

    // The no-visual representation has one canonical shape. In particular,
    // an unused title is still rejected by the chapter canonicalizer; report
    // that exact field here so a progressive repair does not fall back to the
    // opaque sectionValidation category.
    if visual.get("kind").and_then(Value::as_str) == Some("none") {
        if visual
            .get("title")
            .and_then(Value::as_str)
            .is_some_and(|title| !title.is_empty())
        {
            findings.push(json!({"location":"/section/visual/title","code":"visualField"}));
        }
        if visual
            .get("direction")
            .and_then(Value::as_str)
            .is_some_and(|direction| matches!(direction, "leftToRight" | "topToBottom"))
        {
            findings.push(json!({"location":"/section/visual/direction","code":"visualField"}));
        }
        for field in ["nodes", "edges"] {
            if visual
                .get(field)
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty())
            {
                findings.push(
                    json!({"location":format!("/section/visual/{field}"),"code":"visualField"}),
                );
            }
        }
        if visual
            .get("presentation")
            .is_some_and(|presentation| !presentation.is_null())
        {
            findings.push(json!({"location":"/section/visual/presentation","code":"visualField"}));
        }
    }
    findings.truncate(64);
    findings
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

/// The current authoring format accepts only scene presentation version 3.
fn scene_version_findings(section: &Value) -> Vec<Value> {
    let Some(presentation) = section["visual"].get("presentation") else {
        return vec![];
    };
    if !presentation.is_null() && presentation["schemaVersion"].as_u64() != Some(3) {
        vec![json!({"location":"/section/visual/presentation/schemaVersion","code":"sceneVersion"})]
    } else {
        vec![]
    }
}

/// Reuse the source-free scene checks on progressive proposals. Guide sections
/// are published through this ledger and need the same actionable feedback as
/// ordinary Codex chapters.
fn guide_scene_findings(response: &str) -> Vec<Value> {
    let Some(diagnostics) = crate::claim_alignment::chapter_diagnostics(response) else {
        return Vec::new();
    };
    diagnostics
        .into_iter()
        .filter_map(|diagnostic| {
            let problem = diagnostic.get("sceneProblem")?.as_str()?;
            let code = match problem {
                "selectorRequired" => "selectorRequired",
                "selectorShape" => "selectorShape",
                "selectorOutcomeLabel" => "sceneBranchLabels",
                "transformShape" => "transformShape",
                "sceneEvidence" => "sceneEvidence",
                "narrationRequired" | "narrationLength" | "narrationConciseness" => {
                    "sceneNarration"
                }
                "graphDensity" | "elementCount" | "childCount" | "nesting" | "contentCount" => {
                    "sceneAuthoringLimit"
                }
                "containerParent" | "layoutContent" | "emptyGroup" => "sceneAuthoringShape",
                _ => return None,
            };
            Some(json!({
                "location": section_location(&diagnostic),
                "code": code,
            }))
        })
        .take(64)
        .collect()
}

/// Fixed positions only. Full canonical validation still owns admission; this
/// never invents speech, removes claims or grants another attempt.
fn scene_narration_findings(section: &Value) -> Vec<Value> {
    let presentation = &section["visual"]["presentation"];
    if presentation["schemaVersion"].as_u64() != Some(3) {
        return vec![];
    }
    let Some(scenes) = presentation["scenes"].as_array().filter(|s| s.len() <= 20) else {
        return vec![];
    };
    let mut findings = Vec::new();
    let mut total = 0;
    for (index, scene) in scenes.iter().enumerate() {
        match scene["narration"].as_str() {
            Some(text) if crate::scene_presentation::scene_narration_is_concise(text) => {
                if crate::scene_presentation::has_compact_spoken_code_token(text) {
                    findings.push(json!({"location":format!("/section/visual/presentation/scenes/{index}/narration"),"code":"sceneSpeechRole"}));
                }
                total += text.split_whitespace().collect::<Vec<_>>().join(" ").len() + usize::from(index > 0);
            }
            _ => findings.push(json!({"location":format!("/section/visual/presentation/scenes/{index}/narration"),"code":"sceneNarration"})),
        }
    }
    if findings.is_empty() && !(20..=4_000).contains(&total) {
        findings.push(
            json!({"location":"/section/visual/presentation/scenes","code":"sceneNarration"}),
        );
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

fn scene_branch_label_findings(section: &Value) -> Vec<Value> {
    let visual = &section["visual"];
    let Some(labels) = visual["nodes"]
        .as_array()
        .filter(|nodes| nodes.len() <= 12)
        .and_then(|nodes| {
            nodes
                .iter()
                .map(|node| node["label"].as_str())
                .collect::<Option<Vec<_>>>()
        })
    else {
        return vec![];
    };
    let Ok(presentation) = serde_json::from_value(visual["presentation"].clone()) else {
        return vec![];
    };
    crate::scene_presentation::repeated_selector_outcome_positions(&presentation, &labels)
        .into_iter()
        .map(|(scene, element)| {
            json!({"location":format!("/section/visual/presentation/scenes/{scene}/elements/{element}/nodeIndex"),"code":"sceneBranchLabels"})
        })
        .collect()
}

/// Only called after native citation resolution and failed canonicalization.
/// Exact same-claim quotes and explicit value origins explain known node and
/// scene value failures; this never admits a value or returns source strings
/// as repair feedback.
fn scene_source_value_findings(section: &Value, resolved: &str) -> Vec<Value> {
    use crate::scene_presentation::{SceneMode, ScenePresentation, ValueOrigin};
    let visual = &section["visual"];
    let Some(nodes) = visual["nodes"].as_array().filter(|nodes| nodes.len() <= 12) else {
        return vec![];
    };
    let Ok(resolved) = serde_json::from_str::<Value>(resolved) else {
        return vec![];
    };
    let Some(claims) = resolved["sections"][0]["claims"].as_array() else {
        return vec![];
    };
    let mut findings = Vec::new();

    // Node bindings are validated before scene elements. Keep the same exact,
    // unique-substring rule here so the agent gets a repairable diagnostic
    // instead of a generic sectionValidation when a line repeats in an excerpt.
    for (node_index, node) in nodes.iter().enumerate() {
        let Some(source) = node.get("source").filter(|source| source.is_object()) else {
            continue;
        };
        let Some(value) = source["value"].as_str() else {
            continue;
        };
        let evidence = &source["evidence"];
        let (Some(claim_index), Some(evidence_index)) = (
            evidence["claimIndex"]
                .as_u64()
                .and_then(|index| usize::try_from(index).ok()),
            evidence["evidenceIndex"]
                .as_u64()
                .and_then(|index| usize::try_from(index).ok()),
        ) else {
            continue;
        };
        let cites_source = node["evidence"].as_array().is_some_and(|references| {
            references.iter().any(|reference| {
                reference["claimIndex"]
                    .as_u64()
                    .and_then(|index| usize::try_from(index).ok())
                    == Some(claim_index)
                    && reference["evidenceIndex"]
                        .as_u64()
                        .and_then(|index| usize::try_from(index).ok())
                        == Some(evidence_index)
            })
        });
        if !cites_source {
            continue;
        }
        let quote = claims
            .get(claim_index)
            .and_then(|claim| claim["evidence"].as_array())
            .and_then(|evidence| evidence.get(evidence_index))
            .and_then(|evidence| evidence["quote"].as_str());
        let unique_exact_value = quote.is_some_and(|quote| {
            (1..=240).contains(&value.len()) && quote.match_indices(value).count() == 1
        });
        if !unique_exact_value {
            findings.push(json!({
                "location":format!("/section/visual/nodes/{node_index}/source/value"),
                "code":"sceneSourceValue"
            }));
            if findings.len() == 64 {
                return findings;
            }
        }
    }

    let Ok(presentation) =
        serde_json::from_value::<ScenePresentation>(visual["presentation"].clone())
    else {
        return findings;
    };
    for (scene_index, scene) in presentation.scenes.iter().take(20).enumerate() {
        if scene.mode != SceneMode::Composition || scene.claim_index != scene_index {
            continue;
        }
        for (element_index, element) in scene.elements.iter().take(48).enumerate() {
            let location = format!(
                "/section/visual/presentation/scenes/{scene_index}/elements/{element_index}/value"
            );
            if element.value.len() > 1024 {
                findings.push(json!({"location":location,"code":"scenePresentationLimit"}));
                if findings.len() == 64 {
                    return findings;
                }
                continue;
            }
            let invalid_origin = match &element.value_origin {
                ValueOrigin::None => !element.value.is_empty(),
                ValueOrigin::Source | ValueOrigin::Explanation | ValueOrigin::Illustration => {
                    element.value.trim().is_empty()
                }
            };
            if invalid_origin {
                findings.push(json!({"location":location,"code":"sceneSourceValue"}));
                if findings.len() == 64 {
                    return findings;
                }
                continue;
            }
            if !matches!(&element.value_origin, ValueOrigin::Source) {
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
                findings.push(json!({"location":location,"code":"sceneSourceValue"}));
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
                        "namedIdentifier"
                            | "namedIdentifierEvidence"
                            | "sceneAuthoringShape"
                            | "sceneAuthoringKey"
                            | "sceneAuthoringReference"
                            | "sceneAuthoringLimit"
                            | "unsupportedProposalVersion"
                            | "proposalShape"
                            | "sectionKeyFormat"
                            | "guideSlideKeyConflict"
                            | "sectionOrder"
                            | "sectionConflict"
                            | "sectionLimit"
                            | "guideSlideComplete"
                            | "proposalCorrectionLimit"
                            | "citation"
                            | "presentationLimit"
                            | "visualField"
                            | "visualEvidenceDuplicate"
                            | "sceneEvidence"
                            | "sceneVersion"
                            | "sceneNarration"
                            | "sceneSpeechRole"
                            | "sceneRepetition"
                            | "sceneBranchLabels"
                            | "sceneSourceValue"
                            | "scenePresentationLimit"
                            | "selectorRequired"
                            | "selectorShape"
                            | "transformShape"
                            | "sectionValidation"
                            | "chapterUnsupportedSchemaVersion"
                            | "chapterInvalidId"
                            | "chapterInvalidRevision"
                            | "chapterInvalidTarget"
                            | "chapterInvalidSectionOrder"
                            | "chapterDuplicateId"
                            | "chapterInvalidClaim"
                            | "chapterWrongSnapshot"
                            | "chapterMissingPath"
                            | "chapterRangeMismatch"
                            | "chapterHashMismatch"
                            | "chapterExcerptMismatch"
                            | "chapterAmbiguousRemap"
                            | "chapterMissingSymbol"
                            | "chapterInvalidDependency"
                            | "chapterInvalidDestination"
                            | "chapterInvalidVisual"
                            | "chapterIndexUnavailable"
                            | "chapterSourceUnavailable"
                    )
                })
        })
}

impl Ledger {
    pub(crate) fn start_narration_answer_sections(
        &self,
        question: &str,
        request_sha256: &str,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
    ) -> Result<ActiveAnswer, String> {
        self.check(manifest, runtime, cancelled)?;
        if self.record.schema_version != crate::evidence_ledger::LEDGER_SCHEMA_VERSION
            || !hash(request_sha256)
            || question.trim().is_empty()
            || question.len() > 12_000
        {
            return Err("The answer intent or request identity is invalid.".into());
        }
        let path = record_path(&self.storage, &self.record.binding.operation_id)?;
        let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
        let _capacity = capacity::lease(&self.storage)?;
        capacity::check(&self.storage, None, Some(&self.record.binding))?;
        let value = Answer {
            schema_version: NARRATION_ANSWER_VERSION,
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
    fn proposal_key(arguments: &str) -> Option<String> {
        crate::strict_json::from_slice(arguments.as_bytes())
            .ok()?
            .get("sectionKey")?
            .as_str()
            .map(str::to_owned)
    }

    fn has_proposal_key(&self, key: &Option<String>) -> bool {
        self.attempts
            .iter()
            .any(|attempt| Self::proposal_key(&attempt.arguments) == *key)
    }

    fn first_valid_proposal_key(&self) -> Option<String> {
        self.attempts.iter().find_map(|attempt| {
            let key = Self::proposal_key(&attempt.arguments)?;
            valid_key(&key).then_some(key)
        })
    }

    fn exact_successful_replay(&self, key: &Option<String>, proposal_sha256: &str) -> bool {
        self.attempts.iter().any(|attempt| {
            attempt.generation_id.is_some()
                && attempt.proposal_sha256 == proposal_sha256
                && Self::proposal_key(&attempt.arguments) == *key
        })
    }

    fn has_substantive_attempt_for_key(&self, key: &Option<String>) -> bool {
        self.attempts.iter().any(|attempt| {
            Self::proposal_key(&attempt.arguments) == *key
                && (attempt.generation_id.is_some()
                    || Self::correction_required(
                        &attempt.findings,
                        attempt.generation_id.as_deref(),
                    ))
        })
    }

    fn correction_required(findings: &[Value], generation_id: Option<&str>) -> bool {
        if findings.is_empty() {
            return generation_id.is_some();
        }
        findings.iter().any(|finding| {
            finding["code"].as_str().is_some_and(|code| {
                !matches!(
                    code,
                    "proposalShape"
                        | "unsupportedProposalVersion"
                        | "sectionKeyFormat"
                        | "guideSlideKeyConflict"
                        | "sectionOrder"
                        | "sectionConflict"
                        | "sectionLimit"
                        | "guideSlideComplete"
                        | "proposalCorrectionLimit"
                )
            })
        })
    }

    fn corrections_for_key(&self, key: &Option<String>) -> usize {
        let mut seen = false;
        let mut substantive_seen = false;
        let mut corrections = 0;
        for (index, attempt) in self.attempts.iter().enumerate() {
            if Self::proposal_key(&attempt.arguments) != *key {
                continue;
            }
            if seen {
                let successful_replay = attempt.generation_id.as_ref().is_some_and(|generation| {
                    self.attempts[..index].iter().any(|prior| {
                        prior.generation_id.as_ref() == Some(generation)
                            && prior.proposal_sha256 == attempt.proposal_sha256
                    })
                });
                if substantive_seen
                    && !successful_replay
                    && Self::correction_required(
                        &attempt.findings,
                        attempt.generation_id.as_deref(),
                    )
                {
                    corrections += 1;
                }
            }
            substantive_seen |= attempt.generation_id.is_some()
                || Self::correction_required(&attempt.findings, attempt.generation_id.as_deref());
            seen = true;
        }
        corrections
    }

    fn corrections_used(&self) -> usize {
        let keys = self
            .attempts
            .iter()
            .map(|attempt| Self::proposal_key(&attempt.arguments))
            .collect::<BTreeSet<_>>();
        keys.iter().map(|key| self.corrections_for_key(key)).sum()
    }

    fn remaining_corrections(&self) -> usize {
        MAX_CORRECTIONS.saturating_sub(self.corrections_used())
    }

    fn supports_checkpoint(&self, checkpoint: u32) -> bool {
        self.schema_version == NARRATION_ANSWER_VERSION
            && checkpoint == NARRATION_CHECKPOINT_VERSION
    }

    /// Final responses cannot select, omit or replace native publications.
    fn manifest_topics(&self, text: &str) -> Result<Vec<String>, String> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct AnswerManifest {
            #[serde(rename = "manifestVersion")]
            _manifest_version: String,
            #[serde(rename = "incompleteTopics")]
            incomplete_topics: Vec<String>,
        }
        if text.len() > 4096 {
            return Err("The answer's final manifest exceeds its limit.".into());
        }
        let parsed = crate::strict_json::from_slice(text.as_bytes()).map_err(
            |_| "The answer's final manifest is invalid. Published sections are preserved.",
        )?;
        if parsed
            .get("manifestVersion")
            .and_then(Value::as_str)
            .is_some_and(|version| version != NATIVE_MANIFEST_VERSION)
        {
            return Err("The answer's final manifest version is unsupported.".into());
        }
        let manifest: AnswerManifest = serde_json::from_value(parsed).map_err(
            |_| "The answer's final manifest is invalid. Published sections are preserved.",
        )?;
        if self.sections.is_empty() {
            return Err("The answer did not publish any sections.".into());
        }
        let topics = manifest.incomplete_topics;
        if topics.len() > 6
            || topics
                .iter()
                .any(|s| s.trim().is_empty() || s.len() > 240 || s.chars().any(char::is_control))
        {
            return Err("The answer's unfinished topics are invalid.".into());
        }
        Ok(topics)
    }

    fn has_required_completion_manifest(&self) -> bool {
        self.final_responses.last().is_some_and(|response| {
            self.manifest_topics(&response.text)
                .is_ok_and(|topics| topics.is_empty())
        })
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
        let mut topics = self.value.manifest_topics(text)?;
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

    pub(crate) fn remaining_corrections(&self) -> usize {
        self.value.remaining_corrections()
    }

    pub(crate) fn next_new_section_order(&self) -> Option<usize> {
        let next = self
            .value
            .sections
            .iter()
            .map(|section| section.key.as_str())
            .collect::<BTreeSet<_>>()
            .len();
        (next < MAX_SECTIONS).then_some(next)
    }

    pub(crate) fn next_new_section_order_with_limit(&self, limit: usize) -> Option<usize> {
        self.next_new_section_order().filter(|next| *next < limit)
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
        self.prepare_with_section_limit(
            ledger,
            arguments,
            manifest,
            runtime,
            cancelled,
            MAX_SECTIONS,
        )
    }

    pub(crate) fn prepare_with_section_limit(
        &self,
        ledger: &Ledger,
        arguments: &str,
        manifest: &snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        cancelled: &AtomicBool,
        section_limit: usize,
    ) -> Result<PreparedSection, String> {
        if !(1..=MAX_SECTIONS).contains(&section_limit) {
            return Err("The slide publication limit is invalid.".into());
        }
        let checkpoint_version = NARRATION_CHECKPOINT_VERSION;
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
        let proposal_key = Answer::proposal_key(arguments);
        let repeated_key = self.value.has_proposal_key(&proposal_key);
        let successful_replay = self
            .value
            .exact_successful_replay(&proposal_key, &proposal_sha256);
        let guide_slide_key_conflict = section_limit == 1
            && self.value.sections.is_empty()
            && self
                .value
                .first_valid_proposal_key()
                .is_some_and(|first| proposal_key.as_deref() != Some(first.as_str()));
        let mut next = self.value.clone();
        let mut generation_id = None;
        let mut findings = finding("proposalShape");
        if guide_slide_key_conflict {
            findings = vec![json!({"location":"/sectionKey","code":"guideSlideKeyConflict"})];
        } else if let Some(mut proposal) =
            parsed.and_then(|v| serde_json::from_value::<Proposal>(v).ok())
        {
            if proposal.proposal_version != NARRATION_PROPOSAL_VERSION {
                findings = vec![
                    json!({"location":"/proposalVersion","code":"unsupportedProposalVersion"}),
                ];
            } else if !valid_key(&proposal.section_key) {
                findings = vec![json!({"location":"/sectionKey","code":"sectionKeyFormat"})];
            } else if proposal.order < MAX_SECTIONS {
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
                } else if prior.is_none() && unique >= section_limit {
                    findings = finding("guideSlideComplete");
                } else if prior.is_none() && unique >= MAX_SECTIONS {
                    findings = finding("sectionLimit");
                } else if prior.is_none() && proposal.order != unique {
                    findings = finding("sectionOrder");
                } else {
                    match wrapped_authored(&mut proposal) {
                        Err(issues) => findings = issues,
                        Ok((response, compiled)) => {
                            match resolved_authored(
                                ledger
                                    .resolve_citations(&response, manifest, runtime, cancelled)?,
                            ) {
                                Err(issues) => findings = issues,
                                Ok(resolved) => {
                                    let mut shape_findings =
                                        visual_shape_findings(&proposal.section);
                                    shape_findings
                                        .extend(scene_version_findings(&proposal.section));
                                    shape_findings
                                        .extend(scene_narration_findings(&proposal.section));
                                    shape_findings.extend(guide_scene_findings(resolved.text()));
                                    shape_findings.truncate(64);
                                    if !shape_findings.is_empty() {
                                        findings = shape_findings;
                                    } else {
                                        let sources = ledger
                                            .validation_sources(manifest, runtime, cancelled)?;
                                        let syntax = crate::index::load_snapshot_index(
                                            &ledger.storage,
                                            &manifest.summary.project_id,
                                            &manifest.summary.snapshot_id,
                                        )
                                        .unwrap_or(None);
                                        match chapter_generation::canonicalize_narrated_answer_section(
                                            &manifest.summary.project_id,
                                            manifest,
                                            syntax.as_ref(),
                                            &sources,
                                            &resolved,
                                        ).and_then(|mut revision| {
                                            compiled.apply_authored_narration(&mut revision)?;
                                            Ok(revision)
                                        }) {
                                            Err(error) if error == chapter_generation::COMPLETE_CLAIMS_LIMIT_ERROR => findings = finding("presentationLimit"),
                                            Err(_) => {
                                                findings = visual_evidence_findings(&proposal.section);
                                                findings.extend(scene_evidence_findings(&proposal.section));
                                                findings.extend(scene_repetition_findings(&proposal.section));
                                                findings.extend(scene_branch_label_findings(&proposal.section));
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
                                                    findings =
                                                        chapter_validation_findings(&validation);
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
                            compiled.remap(&mut findings);
                        }
                    }
                }
            }
        }
        let requires_correction = repeated_key
            && !successful_replay
            && self.value.has_substantive_attempt_for_key(&proposal_key)
            && Answer::correction_required(&findings, generation_id.as_deref());
        if requires_correction
            && (self.value.corrections_for_key(&proposal_key) >= MAX_CORRECTIONS_PER_KEY
                || self.value.corrections_used() >= MAX_CORRECTIONS)
        {
            // Validation may have prepared a new immutable revision. Enforce
            // the correction reserve before admitting or recording its ID.
            next.sections.truncate(self.value.sections.len());
            generation_id = None;
            findings = finding("proposalCorrectionLimit");
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
                    || !self.value.has_required_completion_manifest()
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
                            | "sourceAnchorId"
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
    replay_answer(storage, operation, &read_record_bytes(storage, operation)?)
}

fn read_record_bytes(storage: &Path, operation: &str) -> Result<Vec<u8>, String> {
    // Take one durable point-in-time record. Source/checkpoint replay is
    // read-only and must not stall another answer's native publication.
    let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
    read_bytes(&record_path(storage, operation)?)
}

pub(super) struct ReplayContext {
    project_id: String,
    snapshot_id: String,
    manifest: snapshot::SnapshotManifest,
    manifest_sha256: String,
    syntax: Option<crate::index::SnapshotIndex>,
}

impl ReplayContext {
    fn load(storage: &Path, project: &str, snapshot: &str) -> Result<Self, String> {
        let manifest = snapshot::load_snapshot_manifest(storage, project, snapshot)?;
        let manifest_sha256 = manifest_hash(&manifest)?;
        // An absent or stale relationship index has historically been treated
        // as unavailable during answer replay. Keep that behavior consistent.
        let syntax = crate::index::load_snapshot_index(storage, project, snapshot).unwrap_or(None);
        Ok(Self {
            project_id: project.to_owned(),
            snapshot_id: snapshot.to_owned(),
            manifest,
            manifest_sha256,
            syntax,
        })
    }
}

fn replay_answer(storage: &Path, operation: &str, bytes: &[u8]) -> Result<Answer, String> {
    let answer = decode_answer(bytes)?;
    let context = ReplayContext::load(
        storage,
        &answer.binding.project_id,
        &answer.binding.snapshot_id,
    )?;
    replay_answer_value(storage, operation, answer, &context)
}

pub(super) fn replay_answer_with_context(
    storage: &Path,
    operation: &str,
    bytes: &[u8],
    shared_context: &ReplayContext,
) -> Result<Answer, String> {
    replay_answer_value(storage, operation, decode_answer(bytes)?, shared_context)
}

fn decode_answer(bytes: &[u8]) -> Result<Answer, String> {
    crate::strict_json::from_slice(bytes)
        .map_err(|_| "Invalid saved answer".to_owned())
        .and_then(|value| {
            serde_json::from_value(value).map_err(|_| "Invalid saved answer".to_owned())
        })
        .map_err(|_| "The saved answer is damaged; it was preserved.".to_owned())
}

fn replay_answer_value(
    storage: &Path,
    operation: &str,
    value: Answer,
    shared_context: &ReplayContext,
) -> Result<Answer, String> {
    if value.schema_version != NARRATION_ANSWER_VERSION
        || value.binding.operation_id != operation
        || !value.binding.runtime.valid_shape()
        || !super::valid_model_effort(&value.binding.model, &value.binding.effort)
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
                || !value.has_required_completion_manifest()
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
    if shared_context.project_id != value.binding.project_id
        || shared_context.snapshot_id != value.binding.snapshot_id
    {
        return Err("The saved answer belongs to another captured manifest.".into());
    }
    let manifest = &shared_context.manifest;
    if shared_context.manifest_sha256 != value.binding.manifest_sha256 {
        return Err("The saved answer belongs to another captured manifest.".into());
    }
    let syntax = shared_context.syntax.as_ref();
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
        if !value.supports_checkpoint(section.checkpoint_version)
            || checkpoint.schema_version != crate::evidence_ledger::LEDGER_SCHEMA_VERSION
            || !same_binding(&value.binding, &checkpoint.binding)?
            || checkpoint.status != Status::Active
            || checkpoint
                .calls
                .iter()
                .any(|c| !matches!(c.state, CallState::Delivered | CallState::Rejected))
            || shared_context.manifest_sha256 != checkpoint.binding.manifest_sha256
            || section.checkpoint_sha256 != context::digest(&encode(checkpoint)?)
            || !valid_key(&section.key)
            || section.order >= MAX_SECTIONS
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
        let mut proposal: Proposal = serde_json::from_str(&attempt.arguments)
            .map_err(|_| "The published proposal is damaged.")?;
        if proposal.proposal_version != NARRATION_PROPOSAL_VERSION
            || proposal.section_key != section.key
            || proposal.order != section.order
            || proposal.previous_generation_id != section.supersedes
        {
            return Err("The section differs from its owned proposal.".into());
        }
        let (response, compiled) = wrapped_authored(&mut proposal)
            .map_err(|_| "The published scene identities are inconsistent.")?;
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
                citations.insert(native_citations::short_reference(index), citation);
            }
        }
        let resolved = match resolved_authored(native_citations::resolve(&response, |id| {
            citations.get(id).cloned().ok_or("referenceUnavailable")
        })) {
            Ok(value) => value,
            Err(_) => {
                return Err("The section cites code outside its delivered checkpoint.".into());
            }
        };
        if !scene_version_findings(&proposal.section).is_empty() {
            return Err("The saved scene does not match its checkpoint policy.".into());
        }
        let mut rebuilt = chapter_generation::canonicalize_narrated_answer_section(
            &value.binding.project_id,
            &manifest,
            syntax,
            &sources,
            &resolved,
        )?;
        compiled.apply_authored_narration(&mut rebuilt)?;
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
            syntax,
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
