use crate::index::{
    IndexFileStatus, IndexedSymbol, SnapshotIndex, SourceAnchor as IndexedSourceAnchor,
};
use crate::snapshot::{self, SnapshotManifest, SnapshotSource};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub const CHAPTER_SCHEMA_VERSION: u32 = 5;
pub(crate) const VISUAL_SCHEMA_VERSION: u32 = 2;
pub const VISUAL_RENDERER_CONTRACT_VERSION: u32 = 2;
const MAX_CHAPTER_JSON_BYTES: usize = 4 * 1024 * 1024;
const MAX_ANCHOR_JSON_BYTES: usize = 128 * 1024;
const MAX_SECTIONS: usize = 500;
const MAX_CLAIMS_PER_SECTION: usize = 100;
const MAX_ANCHORS_PER_CLAIM: usize = 32;
const MAX_HOTSPOTS_PER_SECTION: usize = 100;
const MAX_VISUAL_NODES: usize = 500;
const MAX_VISUAL_EDGES: usize = 1_000;
const MAX_VISUAL_TITLE_BYTES: usize = 160;
const MAX_VISUAL_LABEL_BYTES: usize = 240;
const MAX_VISUAL_DETAIL_BYTES: usize = 512;
const MAX_VISUAL_SOURCE_VALUE_BYTES: usize = 240;
const MAX_VISUAL_EVIDENCE_ANCHORS_PER_ITEM: usize = 8;
const MAX_DEPENDENCIES_PER_ITEM: usize = 100;
const MAX_EXCERPT_BYTES: usize = 64 * 1024;
const MAX_VALIDATION_ISSUES: usize = 2_000;
const MAX_VALIDATION_SOURCE_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_GENERATED_RESPONSE_BYTES: usize = 300 * 1024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterRevision {
    pub schema_version: u32,
    pub chapter_id: String,
    pub revision_id: String,
    pub revision_number: u32,
    pub project_id: String,
    pub snapshot_id: String,
    pub target: ChapterTarget,
    pub title: String,
    pub lifecycle: RevisionLifecycle,
    pub validation_status: ValidationStatus,
    pub created_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub based_on_revision_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_note: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::chapter_origin::deserialize_origin",
        skip_serializing_if = "Option::is_none"
    )]
    pub generation_origin: Option<crate::chapter_origin::ChapterGenerationOrigin>,
    pub sections: Vec<ChapterSection>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterDraftPreview {
    pub generation_id: String,
    pub revision: ChapterRevision,
    pub validation: ChapterValidationReport,
    pub model_id: String,
    pub model_version: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub update_notes: Vec<String>,
}

#[derive(Clone)]
struct PendingChapterPreview {
    revision: ChapterRevision,
    expires_at_unix_ms: u64,
}

#[derive(Clone, Default)]
pub struct PendingChapterDrafts(Arc<Mutex<HashMap<String, PendingChapterPreview>>>);

impl PendingChapterDrafts {
    pub fn insert(&self, revision: ChapterRevision) -> Result<String, String> {
        let mut pending = self.0.lock().map_err(|_| {
            "The generated draft preview store is temporarily unavailable.".to_owned()
        })?;
        Ok(Self::insert_pending(
            &mut pending,
            revision,
            format!("generation:{}", Uuid::new_v4()),
        ))
    }

    /// Persist under the memory-store guard before registering the new preview
    /// or evicting any prior one. The callback cannot call this store; it must
    /// finish durable publication before the preview becomes visible.
    pub(crate) fn try_publish<T>(
        &self,
        revision: ChapterRevision,
        publish: impl FnOnce(String) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut pending = self.0.lock().map_err(|_| {
            "The generated draft preview store is temporarily unavailable.".to_owned()
        })?;
        let generation_id = format!("generation:{}", Uuid::new_v4());
        let published = publish(generation_id.clone())?;
        Self::insert_pending(&mut pending, revision, generation_id);
        Ok(published)
    }

    fn insert_pending(
        pending: &mut HashMap<String, PendingChapterPreview>,
        revision: ChapterRevision,
        generation_id: String,
    ) -> String {
        let now = unix_time_ms();
        pending.retain(|_, entry| entry.expires_at_unix_ms > now);
        while pending.len() >= 8 {
            let oldest = pending
                .iter()
                .min_by_key(|(_, entry)| entry.expires_at_unix_ms)
                .map(|(id, _)| id.clone());
            if let Some(id) = oldest {
                pending.remove(&id);
            } else {
                break;
            }
        }
        pending.insert(
            generation_id.clone(),
            PendingChapterPreview {
                revision,
                expires_at_unix_ms: now.saturating_add(15 * 60 * 1_000),
            },
        );
        generation_id
    }

    pub fn restore(&self, generation_id: &str, revision: ChapterRevision) -> Result<(), String> {
        let value = generation_id
            .strip_prefix("generation:")
            .ok_or_else(|| "The generated draft preview ID is invalid.".to_owned())?;
        let uuid = Uuid::parse_str(value)
            .map_err(|_| "The generated draft preview ID is invalid.".to_owned())?;
        if uuid.to_string() != value {
            return Err("The generated draft preview ID is not canonical.".to_owned());
        }
        let mut pending = self.0.lock().map_err(|_| {
            "The generated draft preview store is temporarily unavailable.".to_owned()
        })?;
        let now = unix_time_ms();
        pending.retain(|_, entry| entry.expires_at_unix_ms > now);
        if pending.len() >= 8 && !pending.contains_key(generation_id) {
            let oldest = pending
                .iter()
                .min_by_key(|(_, entry)| entry.expires_at_unix_ms)
                .map(|(id, _)| id.clone());
            if let Some(id) = oldest {
                pending.remove(&id);
            }
        }
        pending.insert(
            generation_id.to_owned(),
            PendingChapterPreview {
                revision,
                expires_at_unix_ms: now.saturating_add(15 * 60 * 1_000),
            },
        );
        Ok(())
    }

    pub fn get(&self, generation_id: &str) -> Result<ChapterRevision, String> {
        let mut pending = self.0.lock().map_err(|_| {
            "The generated draft preview store is temporarily unavailable.".to_owned()
        })?;
        let now = unix_time_ms();
        pending.retain(|_, entry| entry.expires_at_unix_ms > now);
        pending.get(generation_id).map(|entry| entry.revision.clone()).ok_or_else(|| {
            "This generated draft preview expired or was discarded. Generate and review it again.".to_owned()
        })
    }

    pub fn remove(&self, generation_id: &str) {
        if let Ok(mut pending) = self.0.lock() {
            pending.remove(generation_id);
        }
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(1)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedChapterClaim {
    pub category: String,
    pub aspect: String,
    pub prose: String,
    pub inference_basis: String,
    pub identifiers: Vec<String>,
    pub evidence: Vec<GeneratedChapterEvidence>,
    pub relationship_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedChapterEvidence {
    pub path: String,
    pub quote: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedChapterSection {
    pub title: String,
    pub prose: String,
    pub spoken_text: String,
    pub claims: Vec<GeneratedChapterClaim>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedChapterEvidenceLimit {
    pub prose: String,
    pub basis: String,
    pub identifiers: Vec<String>,
    pub evidence: Vec<GeneratedChapterEvidence>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedChapterProposal {
    pub title: String,
    pub sections: Vec<GeneratedChapterSection>,
    pub evidence_limits: Vec<GeneratedChapterEvidenceLimit>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterTarget {
    pub target_id: String,
    pub scope: ChapterTargetScope,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ChapterTargetScope {
    Overview,
    File { path: String },
    Group { paths: Vec<String> },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RevisionLifecycle {
    Draft,
    Accepted,
    Rejected,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationStatus {
    Unvalidated,
    Valid,
    Invalid,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterSection {
    pub section_id: String,
    pub section_revision_id: String,
    pub order: u32,
    pub title: String,
    pub prose: String,
    /// Empty text is valid when this section has no narration asset.
    pub spoken_text: String,
    pub claims: Vec<ChapterClaim>,
    pub dependencies: Vec<SectionDependency>,
    pub visual: Option<ChapterVisualPayload>,
    pub hotspots: Vec<ChapterHotspot>,
}

/// Keeps malformed diagrams attached to an otherwise readable chapter. The
/// raw JSON is serialized back unchanged in value so validation and player
/// fallback never rewrite an accepted revision as a side effect of reading it.
#[derive(Clone, Debug)]
pub enum ChapterVisualPayload {
    Typed(VisualSpec),
    Decoded {
        visual: VisualSpec,
        original_payload: (VisualSpec, serde_json::Value),
    },
    Unavailable(serde_json::Value),
}

impl PartialEq for ChapterVisualPayload {
    fn eq(&self, other: &Self) -> bool {
        match (self.typed_visual(), other.typed_visual()) {
            (Some(left), Some(right)) => left == right,
            (None, None) => match (self, other) {
                (Self::Unavailable(left), Self::Unavailable(right)) => left == right,
                _ => false,
            },
            _ => false,
        }
    }
}

impl Eq for ChapterVisualPayload {}

impl From<VisualSpec> for ChapterVisualPayload {
    fn from(visual: VisualSpec) -> Self {
        Self::Typed(visual)
    }
}

impl ChapterVisualPayload {
    pub fn typed_visual(&self) -> Option<&VisualSpec> {
        match self {
            Self::Typed(visual) | Self::Decoded { visual, .. } => Some(visual),
            Self::Unavailable(_) => None,
        }
    }

    pub fn typed_visual_mut(&mut self) -> Option<&mut VisualSpec> {
        match self {
            Self::Typed(visual) | Self::Decoded { visual, .. } => Some(visual),
            Self::Unavailable(_) => None,
        }
    }
}

impl Serialize for ChapterVisualPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Typed(visual) => visual.serialize(serializer),
            Self::Decoded {
                visual,
                original_payload: (original_visual, raw),
            } if visual == original_visual => raw.serialize(serializer),
            Self::Decoded { visual, .. } => visual.serialize(serializer),
            Self::Unavailable(raw) => raw.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for ChapterVisualPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = serde_json::Value::deserialize(deserializer)?;
        match serde_json::from_value::<VisualSpec>(raw.clone()) {
            Ok(visual) => Ok(Self::Decoded {
                original_payload: (visual.clone(), raw),
                visual,
            }),
            Err(_) => Ok(Self::Unavailable(raw)),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterClaim {
    pub claim_id: String,
    pub category: ClaimCategory,
    pub prose: String,
    pub anchors: Vec<ChapterSourceAnchor>,
    pub dependencies: Vec<SectionDependency>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum ClaimCategory {
    CodeBehavior,
    RepositoryContext,
    Inference {
        #[serde(rename = "visibleLabel")]
        visible_label: String,
        basis: String,
    },
}

/// Exact implementation prose stays grounded in claims. Authored narration may
/// be supplied separately when no scene presentation is present. This does not
/// certify paraphrase semantics.
pub fn overview_section_copy_supported(section: &ChapterSection) -> bool {
    if section.claims.is_empty() {
        return false;
    }
    let supported_sentences = overview_claim_sentences(&section.claims)
        .into_iter()
        .collect::<HashSet<_>>();
    let copied = |text: &str| {
        sentence_fragments(text)
            .iter()
            .all(|s| supported_sentences.contains(s))
    };
    copied(&section.prose)
        && !sentence_fragments(&section.prose).is_empty()
        && (20..=4_000).contains(&section.spoken_text.trim().len())
        && (section_has_standalone_narration(section)
            || copied(&section.spoken_text)
            || authored_section_narration(section)
                .ok()
                .flatten()
                .as_deref()
                == Some(section.spoken_text.as_str()))
}

/// Schema 5 permits complete authored speech without decorative visuals.
/// This checks bounds and visible inference labels, not semantic equivalence.
fn section_has_standalone_narration(section: &ChapterSection) -> bool {
    let no_presentation = section
        .visual
        .as_ref()
        .and_then(|v| v.typed_visual())
        .is_none_or(|visual| visual.presentation.is_none());
    no_presentation
        && (20..=4_000).contains(&section.spoken_text.trim().len())
        && section.claims.iter().all(|claim| match &claim.category {
            ClaimCategory::Inference { visible_label, .. } => section
                .spoken_text
                .contains(&format!("{}:", visible_label.trim())),
            _ => true,
        })
}

pub(crate) fn authored_section_narration(
    section: &ChapterSection,
) -> Result<Option<String>, String> {
    authored_section_narration_with_policy(section, false)
}

/// Validate newly generated scene scripts against the current authoring
/// contract. The read path deliberately keeps using the prior bound so saved
/// revisions remain readable after prompt-policy changes.
pub(crate) fn authored_section_narration_for_generation(
    section: &ChapterSection,
) -> Result<Option<String>, String> {
    authored_section_narration_with_policy(section, true)
}

fn authored_section_narration_with_policy(
    section: &ChapterSection,
    require_concise: bool,
) -> Result<Option<String>, String> {
    let Some(presentation) = section
        .visual
        .as_ref()
        .and_then(|v| v.typed_visual())
        .and_then(|v| v.presentation.as_ref())
    else {
        return Ok(None);
    };
    let scripts = if require_concise {
        crate::scene_presentation::narration_for_generation(presentation, &section.claims)?
    } else {
        crate::scene_presentation::narration(presentation, &section.claims)?
    };
    Ok(Some(scripts.join(" ")))
}

/// Ordered, complete reader-visible sentences from anchored claims only.
/// Used by both deterministic presentation and the independent storage validator.
pub(crate) fn overview_claim_sentences(claims: &[ChapterClaim]) -> Vec<String> {
    let mut sentences = Vec::new();
    for claim in claims {
        if claim.anchors.is_empty() {
            continue;
        }
        for sentence in sentence_fragments(&claim.prose) {
            let visible = match &claim.category {
                ClaimCategory::Inference { visible_label, .. } => {
                    if visible_label.trim().is_empty() {
                        continue;
                    }
                    format!("{}: {sentence}", visible_label.trim())
                }
                ClaimCategory::CodeBehavior | ClaimCategory::RepositoryContext => sentence,
            };
            if !sentences.contains(&visible) {
                sentences.push(visible);
            }
        }
    }
    sentences
}

/// Validate the content-only invariants that must hold at every storage
/// boundary, even when a caller has not yet loaded the immutable snapshot to
/// perform full anchor verification.
pub fn validate_overview_content(revision: &ChapterRevision) -> Result<(), String> {
    if !matches!(&revision.target.scope, ChapterTargetScope::Overview) {
        return Ok(());
    }
    if revision.schema_version != CHAPTER_SCHEMA_VERSION {
        return Err("A repository overview must use the current chapter schema.".to_owned());
    }
    if revision.sections.is_empty() {
        return Err(
            "A repository overview must contain at least one evidence-backed section.".to_owned(),
        );
    }
    for section in &revision.sections {
        if section.claims.is_empty() {
            return Err(
                "Every repository overview section must contain an evidence-backed claim."
                    .to_owned(),
            );
        }
        if section.claims.iter().any(|claim| claim.anchors.is_empty()) {
            return Err(
                "Every repository overview claim must carry exact source anchors.".to_owned(),
            );
        }
        if section.claims.iter().any(|claim| {
            claim.prose.trim().is_empty()
                || matches!(
                    &claim.category,
                    ClaimCategory::Inference { visible_label, basis }
                        if visible_label.trim().is_empty() || basis.trim().is_empty()
                )
        }) {
            return Err("Every repository overview claim needs readable prose, and inferences need a visible label and basis.".to_owned());
        }
        if !overview_section_copy_supported(section) {
            return Err("Overview prose and narration must use complete sentences from anchored claims, with inference labels preserved.".to_owned());
        }
    }
    Ok(())
}

fn sentence_fragments(text: &str) -> Vec<String> {
    let mut fragments = Vec::new();
    let mut start = 0;
    for (index, character) in text.char_indices() {
        let end = index + character.len_utf8();
        let newline = character == '\n';
        let sentence_end = matches!(character, '.' | '!' | '?')
            && text[end..].chars().next().is_none_or(char::is_whitespace);
        if newline || sentence_end {
            let boundary = if newline { index } else { end };
            let fragment = text[start..boundary].trim();
            if !fragment.is_empty() {
                fragments.push(fragment.to_owned());
            }
            start = end;
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        fragments.push(tail.to_owned());
    }
    fragments
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SectionDependency {
    SourceFile {
        path: String,
        #[serde(rename = "sourceSha256")]
        source_sha256: String,
    },
    Symbol {
        path: String,
        #[serde(rename = "sourceSha256")]
        source_sha256: String,
        #[serde(rename = "symbolId")]
        symbol_id: String,
    },
    Relationship {
        path: String,
        #[serde(rename = "sourceSha256")]
        source_sha256: String,
        #[serde(rename = "relationshipId")]
        relationship_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterSourceAnchor {
    pub anchor_id: String,
    pub path: String,
    pub symbol_id: Option<String>,
    pub snapshot_id: String,
    pub source_sha256: String,
    pub start_byte: u64,
    /// Inclusive UTF-8 byte offset. Line and byte-column ranges are inclusive too.
    pub end_byte_inclusive: u64,
    pub start_line: u32,
    pub end_line: u32,
    pub start_column: u32,
    pub end_column: u32,
    pub excerpt: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterHotspot {
    pub hotspot_id: String,
    pub label: String,
    pub destination: HotspotDestination,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum HotspotDestination {
    Section {
        #[serde(rename = "sectionId")]
        section_id: String,
    },
    CodeLens {
        #[serde(rename = "anchorId")]
        anchor_id: String,
    },
    AtlasFile {
        path: String,
    },
    AtlasSymbol {
        path: String,
        #[serde(rename = "symbolId")]
        symbol_id: String,
    },
    Relationship {
        #[serde(rename = "relationshipId")]
        relationship_id: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualSpec {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<crate::scene_presentation::ScenePresentation>,
    pub schema_version: u32,
    pub kind: VisualKind,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout_hint: Option<VisualLayoutHint>,
    pub nodes: Vec<VisualNode>,
    pub edges: Vec<VisualEdge>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualLayoutHint {
    pub direction: VisualLayoutDirection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VisualLayoutDirection {
    Auto,
    LeftToRight,
    TopToBottom,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VisualKind {
    DataFlow,
    Sequence,
    Hierarchy,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualNode {
    pub node_id: String,
    pub label: String,
    pub detail: Option<String>,
    pub hotspot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_anchor_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_anchor_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VisualEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_anchor_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationReportStatus {
    Valid,
    Invalid,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterValidationReport {
    pub schema_version: u32,
    pub chapter_id: String,
    pub revision_id: String,
    pub snapshot_id: String,
    pub status: ValidationReportStatus,
    pub issues: Vec<ChapterValidationIssue>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterValidationIssue {
    pub code: ChapterIssueCode,
    pub message: String,
    pub path: Option<String>,
    pub section_id: Option<String>,
    pub claim_id: Option<String>,
    pub anchor_id: Option<String>,
    pub hotspot_id: Option<String>,
    pub visual_item_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChapterIssueCode {
    UnsupportedSchemaVersion,
    InvalidId,
    InvalidRevision,
    InvalidTarget,
    InvalidSectionOrder,
    DuplicateId,
    InvalidClaim,
    WrongSnapshot,
    MissingPath,
    RangeMismatch,
    HashMismatch,
    ExcerptMismatch,
    AmbiguousRemap,
    MissingSymbol,
    InvalidDependency,
    InvalidDestination,
    InvalidVisual,
    IndexUnavailable,
    SourceUnavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AnchorRemapStatus {
    Remapped,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorRemapResult {
    pub status: AnchorRemapStatus,
    pub anchor: Option<ChapterSourceAnchor>,
    pub issues: Vec<ChapterValidationIssue>,
}

pub fn decode_chapter_revision(bytes: &[u8]) -> Result<ChapterRevision, String> {
    decode_chapter_payload(bytes, true)
}

/// Decode editable content at the review boundary. Only the trusted saved base
/// supplies lineage; this proposal must never be imported or persisted directly.
pub(crate) fn decode_chapter_edit_proposal(bytes: &[u8]) -> Result<ChapterRevision, String> {
    decode_chapter_payload(bytes, false)
}

fn decode_chapter_payload(bytes: &[u8], verify_origin: bool) -> Result<ChapterRevision, String> {
    if bytes.len() > MAX_CHAPTER_JSON_BYTES {
        return Err("The chapter revision exceeds its 4 MB size limit.".to_owned());
    }
    let value = crate::strict_json::from_slice(bytes).map_err(|error| {
        if error.to_string().contains("duplicate JSON object key") {
            "The chapter revision contains a duplicate JSON object key.".to_owned()
        } else {
            "The chapter revision is not valid JSON.".to_owned()
        }
    })?;
    let schema_version = value
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        .and_then(|version| u32::try_from(version).ok())
        .ok_or_else(|| "The chapter revision has no supported schema version.".to_owned())?;
    if schema_version != CHAPTER_SCHEMA_VERSION {
        return Err(format!(
            "Chapter schema version {schema_version} is not supported by this app."
        ));
    }
    if chapter_contains_executable_visual_fields(&value) {
        return Err("Chapter visuals cannot contain HTML or script fields.".to_owned());
    }
    let revision: ChapterRevision = serde_json::from_value(value)
        .map_err(|_| "The chapter revision does not match its declared schema.".to_owned())?;
    if verify_origin {
        crate::chapter_origin::validate_revision_origin(&revision)?;
    }
    validate_overview_revision_identity(&revision)?;
    Ok(revision)
}

fn chapter_contains_executable_visual_fields(value: &serde_json::Value) -> bool {
    value
        .get("sections")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|sections| {
            sections.iter().any(|section| {
                section
                    .get("visual")
                    .is_some_and(visual_contains_executable_fields)
            })
        })
}

fn visual_contains_executable_fields(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(fields) => fields.iter().any(|(key, nested)| {
            ["html", "script", "javascript"]
                .iter()
                .any(|field| key.eq_ignore_ascii_case(field))
                || visual_contains_executable_fields(nested)
        }),
        serde_json::Value::Array(values) => values.iter().any(visual_contains_executable_fields),
        _ => false,
    }
}

pub fn encode_chapter_revision(revision: &ChapterRevision) -> Result<Vec<u8>, String> {
    if revision.schema_version != CHAPTER_SCHEMA_VERSION {
        return Err("The chapter revision uses an unsupported schema version.".to_owned());
    }
    crate::chapter_origin::validate_revision_origin(revision)?;
    validate_overview_revision_identity(revision)?;
    let bytes = serde_json::to_vec(revision)
        .map_err(|_| "The chapter revision could not be serialized.".to_owned())?;
    if bytes.len() > MAX_CHAPTER_JSON_BYTES {
        return Err("The chapter revision exceeds its 4 MB size limit.".to_owned());
    }
    Ok(bytes)
}

fn validate_overview_revision_identity(revision: &ChapterRevision) -> Result<(), String> {
    if matches!(&revision.target.scope, ChapterTargetScope::Overview)
        && (revision.schema_version != CHAPTER_SCHEMA_VERSION
            || revision.target.target_id != "target:overview"
            || revision.chapter_id != "chapter:overview")
    {
        return Err(
            "An overview chapter must use the current schema and its stable overview identities."
                .to_owned(),
        );
    }
    Ok(())
}

pub fn decode_chapter_anchor(bytes: &[u8]) -> Result<ChapterSourceAnchor, String> {
    if bytes.len() > MAX_ANCHOR_JSON_BYTES {
        return Err("The source anchor exceeds its 128 KiB input limit.".to_owned());
    }
    serde_json::from_slice(bytes)
        .map_err(|_| "The source anchor does not match its version 1 schema.".to_owned())
}

/// Convert the parser's half-open source range into the chapter schema's inclusive range.
/// The captured source hash and exact UTF-8 excerpt are verified before an anchor is returned.
pub fn anchor_from_index(
    indexed: &IndexedSourceAnchor,
    snapshot_id: &str,
    symbol: Option<&IndexedSymbol>,
    anchor_id: &str,
    source: &SnapshotSource,
) -> Result<ChapterSourceAnchor, ChapterValidationIssue> {
    let failure = |code, message: &str| ChapterValidationIssue {
        code,
        message: message.to_owned(),
        path: Some(indexed.path.clone()),
        section_id: None,
        claim_id: None,
        anchor_id: Some(anchor_id.to_owned()),
        hotspot_id: None,
        visual_item_id: None,
    };
    if source.snapshot_id != snapshot_id {
        return Err(failure(
            ChapterIssueCode::WrongSnapshot,
            "The source bytes do not belong to the requested snapshot.",
        ));
    }
    if source.path != indexed.path {
        return Err(failure(
            ChapterIssueCode::MissingPath,
            "The parser evidence path does not match the captured source path.",
        ));
    }
    if source.sha256 != indexed.source_sha256 {
        return Err(failure(
            ChapterIssueCode::HashMismatch,
            "The parser anchor hash does not match the captured source.",
        ));
    }
    if sha256_hex(source.contents.as_bytes()) != source.sha256 {
        return Err(failure(
            ChapterIssueCode::HashMismatch,
            "The captured source does not match its SHA-256 hash.",
        ));
    }
    let (Some(start), Some(end_exclusive)) = (
        usize::try_from(indexed.start_byte).ok(),
        usize::try_from(indexed.end_byte).ok(),
    ) else {
        return Err(failure(
            ChapterIssueCode::RangeMismatch,
            "The parser byte range cannot be represented on this platform.",
        ));
    };
    let bytes = source.contents.as_bytes();
    if start >= end_exclusive || end_exclusive > bytes.len() {
        return Err(failure(
            ChapterIssueCode::RangeMismatch,
            "The parser byte range is empty or outside the captured source.",
        ));
    }
    if let Some(symbol) = symbol {
        if symbol.path != indexed.path || symbol.anchor.source_sha256 != indexed.source_sha256 {
            return Err(failure(
                ChapterIssueCode::MissingSymbol,
                "The supplied stable symbol does not match the parser evidence path and hash.",
            ));
        }
        if symbol.anchor.start_byte > start || symbol.anchor.end_byte < end_exclusive {
            return Err(failure(
                ChapterIssueCode::RangeMismatch,
                "The parser evidence does not fall inside the supplied stable symbol range.",
            ));
        }
    }
    let Ok(excerpt) = std::str::from_utf8(&bytes[start..end_exclusive]) else {
        return Err(failure(
            ChapterIssueCode::RangeMismatch,
            "The parser byte range does not align to UTF-8 boundaries.",
        ));
    };
    let (start_line, start_column) = byte_position(bytes, start);
    let (end_line, end_column) = byte_position(bytes, end_exclusive - 1);
    Ok(ChapterSourceAnchor {
        anchor_id: anchor_id.to_owned(),
        path: indexed.path.clone(),
        symbol_id: symbol.map(|symbol| symbol.symbol_id.clone()),
        snapshot_id: snapshot_id.to_owned(),
        source_sha256: source.sha256.clone(),
        start_byte: start as u64,
        end_byte_inclusive: (end_exclusive - 1) as u64,
        start_line,
        end_line,
        start_column,
        end_column,
        excerpt: excerpt.to_owned(),
    })
}

/// Build an exact whole-line citation from already verified captured bytes.
/// Neither a live file nor caller-supplied hash/excerpt participates.
pub(crate) fn anchor_from_lines(
    source: &SnapshotSource,
    snapshot_index: Option<&SnapshotIndex>,
    anchor_id: &str,
    start_line: u32,
    end_line: u32,
) -> Result<ChapterSourceAnchor, String> {
    if !is_valid_id(anchor_id) || start_line == 0 || end_line < start_line {
        return Err("Choose a valid citation and an ordered range of source lines.".to_owned());
    }
    let mut offset = 0usize;
    let mut start = None;
    let mut end = None;
    for (i, line) in source.contents.split_inclusive('\n').enumerate() {
        let number = i + 1;
        if number == start_line as usize {
            start = Some(offset);
        }
        offset += line.len();
        if number == end_line as usize {
            end = Some(offset);
            break;
        }
    }
    let (Some(start), Some(end)) = (start, end) else {
        return Err("The selected line range is outside the captured file.".to_owned());
    };
    if end <= start || end - start > MAX_EXCERPT_BYTES {
        return Err("Choose a nonempty source range of at most 64 KiB.".to_owned());
    }
    let symbol = snapshot_index
        .filter(|i| i.snapshot_id == source.snapshot_id)
        .and_then(|i| {
            i.symbols
                .iter()
                .filter(|s| {
                    s.path == source.path
                        && s.anchor.source_sha256 == source.sha256
                        && s.anchor.start_byte <= start
                        && s.anchor.end_byte >= end
                })
                .min_by_key(|s| s.anchor.end_byte.saturating_sub(s.anchor.start_byte))
        });
    let indexed = IndexedSourceAnchor {
        path: source.path.clone(),
        source_sha256: source.sha256.clone(),
        start_byte: start,
        end_byte: end,
        start_line: start_line as usize,
        end_line: end_line as usize,
        start_column: 1,
        end_column: 1,
    };
    anchor_from_index(&indexed, &source.snapshot_id, symbol, anchor_id, source)
        .map_err(|issue| issue.message)
}

pub fn validate_chapter_revision(
    storage_dir: &Path,
    manifest: &SnapshotManifest,
    index: Option<&SnapshotIndex>,
    revision: &ChapterRevision,
) -> ChapterValidationReport {
    let mut validator = Validator {
        storage_dir,
        manifest,
        index,
        source_cache: HashMap::new(),
        source_bytes_loaded: 0,
        issues: Vec::new(),
    };
    if revision.schema_version != CHAPTER_SCHEMA_VERSION {
        validator.issue(
            ChapterIssueCode::UnsupportedSchemaVersion,
            "This chapter schema version is not supported.",
            None,
            None,
            None,
            None,
        );
    }
    if let Err(message) = crate::chapter_origin::validate_revision_origin(revision) {
        validator.issue(
            ChapterIssueCode::InvalidRevision,
            &message,
            None,
            None,
            None,
            None,
        );
    }
    validate_id(&mut validator, &revision.chapter_id, None, None, None);
    validate_id(&mut validator, &revision.revision_id, None, None, None);
    if revision.revision_number == 0
        || revision.created_at_unix_ms == 0
        || revision.title.trim().is_empty()
    {
        validator.issue(
            ChapterIssueCode::InvalidRevision,
            "A revision needs a positive revision number, creation time, and chapter title.",
            None,
            None,
            None,
            None,
        );
    }
    if revision.project_id != manifest.summary.project_id
        || revision.snapshot_id != manifest.summary.snapshot_id
    {
        validator.issue(
            ChapterIssueCode::WrongSnapshot,
            "This chapter revision is pinned to a different project or snapshot.",
            None,
            None,
            None,
            None,
        );
    }
    if index.is_some_and(|index| {
        index.project_id != manifest.summary.project_id
            || index.snapshot_id != manifest.summary.snapshot_id
    }) {
        validator.issue(
            ChapterIssueCode::WrongSnapshot,
            "The supplied symbol index does not belong to this project snapshot.",
            None,
            None,
            None,
            None,
        );
    }
    if revision.lifecycle == RevisionLifecycle::Accepted
        && revision.validation_status != ValidationStatus::Valid
    {
        validator.issue(
            ChapterIssueCode::InvalidRevision,
            "An accepted revision must record a successful validation status.",
            None,
            None,
            None,
            None,
        );
    }
    match revision.based_on_revision_id.as_deref() {
        Some(parent_id) => {
            validate_id(&mut validator, parent_id, None, None, None);
            if parent_id == revision.revision_id {
                validator.issue(
                    ChapterIssueCode::InvalidRevision,
                    "A revision cannot be based on itself.",
                    None,
                    None,
                    None,
                    None,
                );
            }
        }
        None if matches!(
            revision.lifecycle,
            RevisionLifecycle::Accepted | RevisionLifecycle::Rejected
        ) =>
        {
            validator.issue(
                ChapterIssueCode::InvalidRevision,
                "An accepted or rejected revision must identify its parent revision.",
                None,
                None,
                None,
                None,
            );
        }
        None => {}
    }
    if revision.lifecycle == RevisionLifecycle::Rejected
        && revision
            .decision_note
            .as_deref()
            .is_none_or(|note| note.trim().is_empty() || note.len() > 4_000)
    {
        validator.issue(
            ChapterIssueCode::InvalidRevision,
            "A rejected revision needs a reason of at most 4,000 bytes.",
            None,
            None,
            None,
            None,
        );
    }
    if revision.lifecycle != RevisionLifecycle::Rejected && revision.decision_note.is_some() {
        validator.issue(
            ChapterIssueCode::InvalidRevision,
            "Only a rejected revision can carry a decision reason.",
            None,
            None,
            None,
            None,
        );
    }
    validate_target(&mut validator, &revision.target);
    if matches!(&revision.target.scope, ChapterTargetScope::Overview)
        && (revision.schema_version != CHAPTER_SCHEMA_VERSION
            || revision.target.target_id != "target:overview"
            || revision.chapter_id != "chapter:overview")
    {
        validator.issue(
            ChapterIssueCode::InvalidTarget,
            "An overview chapter must use the current schema and its stable overview identities.",
            None,
            None,
            None,
            None,
        );
    }
    let is_overview = matches!(&revision.target.scope, ChapterTargetScope::Overview);
    if revision.sections.is_empty() || revision.sections.len() > MAX_SECTIONS {
        validator.issue(
            ChapterIssueCode::InvalidRevision,
            "A chapter must contain between 1 and 500 ordered sections.",
            None,
            None,
            None,
            None,
        );
    }

    let section_ids = revision
        .sections
        .iter()
        .map(|section| section.section_id.as_str())
        .collect::<HashSet<_>>();
    let anchor_ids = revision
        .sections
        .iter()
        .flat_map(|section| section.claims.iter())
        .flat_map(|claim| claim.anchors.iter())
        .map(|anchor| anchor.anchor_id.clone())
        .collect::<HashSet<_>>();
    let mut ids = HashSet::<String>::new();
    register_unique_id(
        &mut validator,
        &mut ids,
        &revision.chapter_id,
        None,
        None,
        None,
    );
    register_unique_id(
        &mut validator,
        &mut ids,
        &revision.revision_id,
        None,
        None,
        None,
    );
    validate_id(&mut validator, &revision.target.target_id, None, None, None);
    register_unique_id(
        &mut validator,
        &mut ids,
        &revision.target.target_id,
        None,
        None,
        None,
    );

    for (position, section) in revision.sections.iter().take(MAX_SECTIONS).enumerate() {
        let section_id = Some(section.section_id.as_str());
        let section_anchor_ids = section
            .claims
            .iter()
            .flat_map(|claim| claim.anchors.iter())
            .map(|anchor| anchor.anchor_id.clone())
            .collect::<HashSet<_>>();
        validate_id(&mut validator, &section.section_id, section_id, None, None);
        validate_id(
            &mut validator,
            &section.section_revision_id,
            section_id,
            None,
            None,
        );
        register_unique_id(
            &mut validator,
            &mut ids,
            &section.section_id,
            section_id,
            None,
            None,
        );
        register_unique_id(
            &mut validator,
            &mut ids,
            &section.section_revision_id,
            section_id,
            None,
            None,
        );
        if section.order as usize != position {
            validator.issue(
                ChapterIssueCode::InvalidSectionOrder,
                "Section order must be contiguous and start at 0.",
                None,
                section_id,
                None,
                None,
            );
        }
        if section.title.trim().is_empty() || section.prose.trim().is_empty() {
            validator.issue(
                ChapterIssueCode::InvalidRevision,
                "Every section needs a title and explanation prose.",
                None,
                section_id,
                None,
                None,
            );
        }
        if section.claims.len() > MAX_CLAIMS_PER_SECTION {
            validator.issue(
                ChapterIssueCode::InvalidRevision,
                "A section may contain at most 100 claims.",
                None,
                section_id,
                None,
                None,
            );
        }
        if is_overview && section.claims.is_empty() {
            validator.issue(
                ChapterIssueCode::InvalidClaim,
                "Every repository overview section needs at least one evidence-backed claim.",
                None,
                section_id,
                None,
                None,
            );
        }
        if section.dependencies.len() > MAX_DEPENDENCIES_PER_ITEM {
            validator.issue(
                ChapterIssueCode::InvalidDependency,
                "A section may contain at most 100 dependencies.",
                None,
                section_id,
                None,
                None,
            );
        }
        for dependency in section.dependencies.iter().take(MAX_DEPENDENCIES_PER_ITEM) {
            validator.validate_dependency(dependency, section_id, None);
        }
        for claim in section.claims.iter().take(MAX_CLAIMS_PER_SECTION) {
            validate_id(
                &mut validator,
                &claim.claim_id,
                section_id,
                Some(&claim.claim_id),
                None,
            );
            register_unique_id(
                &mut validator,
                &mut ids,
                &claim.claim_id,
                section_id,
                Some(&claim.claim_id),
                None,
            );
            let claim_id = Some(claim.claim_id.as_str());
            if claim.prose.trim().is_empty() {
                validator.issue(
                    ChapterIssueCode::InvalidClaim,
                    "A claim needs readable prose.",
                    None,
                    section_id,
                    claim_id,
                    None,
                );
            }
            match &claim.category {
                ClaimCategory::CodeBehavior if claim.anchors.is_empty() => validator.issue(
                    ChapterIssueCode::InvalidClaim,
                    "A code-behavior claim requires at least one exact source anchor.",
                    None,
                    section_id,
                    claim_id,
                    None,
                ),
                ClaimCategory::Inference {
                    visible_label,
                    basis,
                } if visible_label.trim().is_empty() || basis.trim().is_empty() => validator.issue(
                    ChapterIssueCode::InvalidClaim,
                    "An inference needs a visible label and a recorded basis.",
                    None,
                    section_id,
                    claim_id,
                    None,
                ),
                _ => {}
            }
            if is_overview && claim.anchors.is_empty() {
                validator.issue(
                    ChapterIssueCode::InvalidClaim,
                    "Every repository overview claim requires exact source anchors.",
                    None,
                    section_id,
                    claim_id,
                    None,
                );
            }
            if claim.anchors.len() > MAX_ANCHORS_PER_CLAIM {
                validator.issue(
                    ChapterIssueCode::InvalidClaim,
                    "A claim may contain at most 32 source anchors.",
                    None,
                    section_id,
                    claim_id,
                    None,
                );
            }
            for anchor in claim.anchors.iter().take(MAX_ANCHORS_PER_CLAIM) {
                validate_id(
                    &mut validator,
                    &anchor.anchor_id,
                    section_id,
                    claim_id,
                    Some(&anchor.anchor_id),
                );
                register_unique_id(
                    &mut validator,
                    &mut ids,
                    &anchor.anchor_id,
                    section_id,
                    claim_id,
                    Some(&anchor.anchor_id),
                );
                validator.validate_anchor(anchor, section_id, claim_id, None);
            }
            if claim.dependencies.len() > MAX_DEPENDENCIES_PER_ITEM {
                validator.issue(
                    ChapterIssueCode::InvalidDependency,
                    "A claim may contain at most 100 dependencies.",
                    None,
                    section_id,
                    claim_id,
                    None,
                );
            }
            for dependency in claim.dependencies.iter().take(MAX_DEPENDENCIES_PER_ITEM) {
                validator.validate_dependency(dependency, section_id, claim_id);
            }
        }

        if is_overview && !overview_section_copy_supported(section) {
            validator.issue(
                ChapterIssueCode::InvalidClaim,
                "Overview prose and narration must use complete sentences from anchored claims, with inference labels preserved.",
                None,
                section_id,
                None,
                None,
            );
        }

        let mut hotspot_ids = HashSet::<String>::new();
        if section.hotspots.len() > MAX_HOTSPOTS_PER_SECTION {
            validator.issue(
                ChapterIssueCode::InvalidDestination,
                "A section may contain at most 100 hotspots.",
                None,
                section_id,
                None,
                None,
            );
        }
        for hotspot in section.hotspots.iter().take(MAX_HOTSPOTS_PER_SECTION) {
            validate_id(
                &mut validator,
                &hotspot.hotspot_id,
                section_id,
                None,
                Some(&hotspot.hotspot_id),
            );
            register_unique_id(
                &mut validator,
                &mut ids,
                &hotspot.hotspot_id,
                section_id,
                None,
                Some(&hotspot.hotspot_id),
            );
            hotspot_ids.insert(hotspot.hotspot_id.clone());
            if hotspot.label.trim().is_empty() {
                validator.issue(
                    ChapterIssueCode::InvalidDestination,
                    "A hotspot needs an accessible label.",
                    None,
                    section_id,
                    None,
                    Some(&hotspot.hotspot_id),
                );
            } else if hotspot.label.len() > MAX_VISUAL_LABEL_BYTES {
                validator.issue(
                    ChapterIssueCode::InvalidDestination,
                    "A hotspot label must be at most 240 UTF-8 bytes.",
                    None,
                    section_id,
                    None,
                    Some(&hotspot.hotspot_id),
                );
            }
            validator.validate_destination(
                &hotspot.destination,
                &section_ids,
                &anchor_ids,
                section_id,
                Some(&hotspot.hotspot_id),
            );
        }
        if let Some(visual) = &section.visual {
            if let Some(visual_spec) = visual.typed_visual() {
                validator.validate_visual(
                    visual_spec,
                    &section.claims,
                    &hotspot_ids,
                    &section_anchor_ids,
                    &mut ids,
                    section_id,
                );
            } else {
                validator.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    "This diagram has an unsupported or malformed shape. The chapter explanation and source evidence remain readable.",
                    section_id,
                    None,
                    None,
                );
            }
        }
    }

    if validator.issues.len() >= MAX_VALIDATION_ISSUES {
        validator.issues.truncate(MAX_VALIDATION_ISSUES - 1);
        validator.issue(
            ChapterIssueCode::InvalidRevision,
            "Validation stopped after the 2,000-issue safety limit.",
            None,
            None,
            None,
            None,
        );
    }
    let status = if validator.issues.is_empty() {
        ValidationReportStatus::Valid
    } else {
        ValidationReportStatus::Invalid
    };
    ChapterValidationReport {
        schema_version: revision.schema_version,
        chapter_id: revision.chapter_id.clone(),
        revision_id: revision.revision_id.clone(),
        snapshot_id: manifest.summary.snapshot_id.clone(),
        status,
        issues: validator.issues,
    }
}

struct Validator<'a> {
    storage_dir: &'a Path,
    manifest: &'a SnapshotManifest,
    index: Option<&'a SnapshotIndex>,
    source_cache: HashMap<String, Result<SnapshotSource, String>>,
    source_bytes_loaded: u64,
    issues: Vec<ChapterValidationIssue>,
}

impl Validator<'_> {
    fn issue(
        &mut self,
        code: ChapterIssueCode,
        message: &str,
        path: Option<&str>,
        section_id: Option<&str>,
        claim_id: Option<&str>,
        hotspot_id: Option<&str>,
    ) {
        if self.issues.len() < MAX_VALIDATION_ISSUES {
            self.issues.push(ChapterValidationIssue {
                code,
                message: message.to_owned(),
                path: path.map(str::to_owned),
                section_id: section_id.map(str::to_owned),
                claim_id: claim_id.map(str::to_owned),
                anchor_id: None,
                hotspot_id: hotspot_id.map(str::to_owned),
                visual_item_id: None,
            });
        }
    }

    fn issue_for_anchor(
        &mut self,
        code: ChapterIssueCode,
        message: &str,
        anchor: &ChapterSourceAnchor,
        section_id: Option<&str>,
        claim_id: Option<&str>,
        hotspot_id: Option<&str>,
    ) {
        if self.issues.len() < MAX_VALIDATION_ISSUES {
            self.issues.push(ChapterValidationIssue {
                code,
                message: message.to_owned(),
                path: Some(anchor.path.clone()),
                section_id: section_id.map(str::to_owned),
                claim_id: claim_id.map(str::to_owned),
                anchor_id: Some(anchor.anchor_id.clone()),
                hotspot_id: hotspot_id.map(str::to_owned),
                visual_item_id: None,
            });
        }
    }

    fn source(&mut self, path: &str) -> Option<&SnapshotSource> {
        if !self.source_cache.contains_key(path) {
            let loaded =
                snapshot::read_snapshot_source_from_manifest(self.storage_dir, self.manifest, path)
                    .and_then(|source| {
                        if self.source_bytes_loaded.saturating_add(source.size_bytes)
                            > MAX_VALIDATION_SOURCE_BYTES
                        {
                            Err("The chapter validation source-byte budget was reached.".to_owned())
                        } else {
                            self.source_bytes_loaded += source.size_bytes;
                            Ok(source)
                        }
                    });
            self.source_cache.insert(path.to_owned(), loaded);
        }
        match self.source_cache.get(path) {
            Some(Ok(source)) => Some(source),
            _ => None,
        }
    }

    fn validate_anchor(
        &mut self,
        anchor: &ChapterSourceAnchor,
        section_id: Option<&str>,
        claim_id: Option<&str>,
        hotspot_id: Option<&str>,
    ) {
        if anchor.snapshot_id != self.manifest.summary.snapshot_id {
            self.issue_for_anchor(
                ChapterIssueCode::WrongSnapshot,
                "This anchor belongs to a different immutable snapshot.",
                anchor,
                section_id,
                claim_id,
                hotspot_id,
            );
            return;
        }
        if !self
            .manifest
            .files
            .iter()
            .any(|file| file.path == anchor.path)
        {
            self.issue_for_anchor(
                ChapterIssueCode::MissingPath,
                "The anchor path was not captured in this snapshot.",
                anchor,
                section_id,
                claim_id,
                hotspot_id,
            );
            return;
        }
        let Some(source) = self.source(&anchor.path) else {
            self.issue_for_anchor(
                ChapterIssueCode::SourceUnavailable,
                "Captured source bytes are unavailable or failed their integrity check.",
                anchor,
                section_id,
                claim_id,
                hotspot_id,
            );
            return;
        };
        if let Err((code, message)) = verify_anchor_against_source(anchor, source) {
            self.issue_for_anchor(code, message, anchor, section_id, claim_id, hotspot_id);
            return;
        }
        if let Some(symbol_id) = &anchor.symbol_id {
            let Some(index) = self.index else {
                self.issue_for_anchor(
                    ChapterIssueCode::IndexUnavailable,
                    "This anchor names a symbol, but this snapshot has no saved symbol index.",
                    anchor,
                    section_id,
                    claim_id,
                    hotspot_id,
                );
                return;
            };
            let symbol = index
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == *symbol_id);
            if !symbol.is_some_and(|symbol| symbol_contains_anchor(symbol, anchor)) {
                self.issue_for_anchor(
                    ChapterIssueCode::MissingSymbol,
                    "The stable symbol ID does not identify this source range in the saved index.",
                    anchor,
                    section_id,
                    claim_id,
                    hotspot_id,
                );
            }
        }
    }

    fn validate_dependency(
        &mut self,
        dependency: &SectionDependency,
        section_id: Option<&str>,
        claim_id: Option<&str>,
    ) {
        let (path, source_sha256) = match dependency {
            SectionDependency::SourceFile {
                path,
                source_sha256,
            }
            | SectionDependency::Symbol {
                path,
                source_sha256,
                ..
            }
            | SectionDependency::Relationship {
                path,
                source_sha256,
                ..
            } => (path, source_sha256),
        };
        let Some(file) = self.manifest.files.iter().find(|file| file.path == *path) else {
            self.issue(
                ChapterIssueCode::MissingPath,
                "A dependency path was not captured in this snapshot.",
                Some(path),
                section_id,
                claim_id,
                None,
            );
            return;
        };
        if file.sha256 != *source_sha256 {
            self.issue(
                ChapterIssueCode::HashMismatch,
                "A dependency hash does not match the captured file.",
                Some(path),
                section_id,
                claim_id,
                None,
            );
            return;
        }
        match dependency {
            SectionDependency::SourceFile { .. } => {}
            SectionDependency::Symbol { symbol_id, .. } => {
                let Some(index) = self.index else {
                    self.issue(
                        ChapterIssueCode::IndexUnavailable,
                        "A symbol dependency cannot be checked without the saved snapshot index.",
                        Some(path),
                        section_id,
                        claim_id,
                        None,
                    );
                    return;
                };
                if !index.symbols.iter().any(|symbol| {
                    symbol.symbol_id == *symbol_id
                        && symbol.path == *path
                        && symbol.anchor.source_sha256 == *source_sha256
                }) {
                    self.issue(
                        ChapterIssueCode::InvalidDependency,
                        "The symbol dependency does not exist at this path and source hash.",
                        Some(path),
                        section_id,
                        claim_id,
                        None,
                    );
                }
            }
            SectionDependency::Relationship {
                relationship_id, ..
            } => {
                let Some(index) = self.index else {
                    self.issue(
                        ChapterIssueCode::IndexUnavailable,
                        "A relationship dependency cannot be checked without the saved snapshot index.",
                        Some(path),
                        section_id,
                        claim_id,
                        None,
                    );
                    return;
                };
                if !index.relationships.iter().any(|relationship| {
                    relationship.relationship_id == *relationship_id
                        && relationship.evidence.path == *path
                        && relationship.evidence.source_sha256 == *source_sha256
                }) {
                    self.issue(
                        ChapterIssueCode::InvalidDependency,
                        "The relationship dependency does not exist at this path and source hash.",
                        Some(path),
                        section_id,
                        claim_id,
                        None,
                    );
                }
            }
        }
    }

    fn validate_destination(
        &mut self,
        destination: &HotspotDestination,
        section_ids: &HashSet<&str>,
        anchor_ids: &HashSet<String>,
        section_id: Option<&str>,
        hotspot_id: Option<&str>,
    ) {
        let invalid = |validator: &mut Self, message: &str, path: Option<&str>| {
            validator.issue(
                ChapterIssueCode::InvalidDestination,
                message,
                path,
                section_id,
                None,
                hotspot_id,
            );
        };
        match destination {
            HotspotDestination::Section { section_id: target } => {
                if !section_ids.contains(target.as_str()) {
                    invalid(
                        self,
                        "The section hotspot points to no section in this chapter.",
                        None,
                    );
                }
            }
            HotspotDestination::CodeLens { anchor_id } => {
                if !anchor_ids.contains(anchor_id) {
                    invalid(
                        self,
                        "The Code Lens hotspot destination refers to no source anchor in this chapter.",
                        None,
                    );
                }
            }
            HotspotDestination::AtlasFile { path } => {
                if !self.manifest.files.iter().any(|file| file.path == *path) {
                    invalid(
                        self,
                        "The Atlas file destination is not captured in this snapshot.",
                        Some(path),
                    );
                }
            }
            HotspotDestination::AtlasSymbol { path, symbol_id } => {
                let Some(index) = self.index else {
                    invalid(
                        self,
                        "The Atlas symbol destination cannot be checked without a saved index.",
                        Some(path),
                    );
                    return;
                };
                if !index
                    .symbols
                    .iter()
                    .any(|symbol| symbol.symbol_id == *symbol_id && symbol.path == *path)
                {
                    invalid(
                        self,
                        "The Atlas symbol destination does not match a symbol in this snapshot.",
                        Some(path),
                    );
                }
            }
            HotspotDestination::Relationship { relationship_id } => {
                let Some(index) = self.index else {
                    invalid(
                        self,
                        "The relationship destination cannot be checked without a saved index.",
                        None,
                    );
                    return;
                };
                if !index
                    .relationships
                    .iter()
                    .any(|edge| edge.relationship_id == *relationship_id)
                {
                    invalid(
                        self,
                        "The relationship destination does not exist in this snapshot index.",
                        None,
                    );
                }
            }
        }
    }

    fn validate_visual(
        &mut self,
        visual: &VisualSpec,
        claims: &[ChapterClaim],
        hotspot_ids: &HashSet<String>,
        section_anchor_ids: &HashSet<String>,
        revision_ids: &mut HashSet<String>,
        section_id: Option<&str>,
    ) {
        if let Some(presentation) = &visual.presentation {
            if visual.schema_version != VISUAL_SCHEMA_VERSION {
                self.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    "HTML compositions require the current evidence-linked visual schema.",
                    section_id,
                    None,
                    None,
                );
            }
            if let Err(error) =
                crate::scene_presentation::validate(presentation, &visual.nodes, claims)
            {
                self.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    &error,
                    section_id,
                    None,
                    None,
                );
            }
            if let Err(error) = crate::scene_presentation::validate_graph_density(
                presentation,
                &visual.nodes,
                &visual.edges,
                claims,
            ) {
                self.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    &error,
                    section_id,
                    None,
                    None,
                );
            }
        }
        if visual.schema_version != VISUAL_SCHEMA_VERSION {
            self.visual_issue(
                ChapterIssueCode::InvalidVisual,
                "A diagram must use the current visual schema version 2.",
                section_id,
                None,
                None,
            );
        }
        let is_current = visual.schema_version == VISUAL_SCHEMA_VERSION;
        if visual.title.trim().is_empty() || visual.title.len() > MAX_VISUAL_TITLE_BYTES {
            self.visual_issue(
                ChapterIssueCode::InvalidVisual,
                "A visual title must contain 1–160 UTF-8 bytes.",
                section_id,
                None,
                None,
            );
        }
        if visual.nodes.is_empty() || visual.nodes.len() > MAX_VISUAL_NODES {
            self.visual_issue(
                ChapterIssueCode::InvalidVisual,
                "A visual must contain 1–500 nodes.",
                section_id,
                None,
                None,
            );
        }
        if visual.edges.len() > MAX_VISUAL_EDGES {
            self.visual_issue(
                ChapterIssueCode::InvalidVisual,
                "A visual may contain at most 1,000 edges.",
                section_id,
                None,
                None,
            );
        }
        if is_current && visual.layout_hint.is_none() {
            self.visual_issue(
                ChapterIssueCode::InvalidVisual,
                "A version 2 visual must declare an app-owned layout direction.",
                section_id,
                None,
                None,
            );
        }
        let mut node_ids = HashSet::new();
        for node in visual.nodes.iter().take(MAX_VISUAL_NODES) {
            if !is_valid_id(&node.node_id) {
                self.visual_issue(
                    ChapterIssueCode::InvalidId,
                    "A visual node ID must use 1–128 ASCII letters, digits, colons, underscores, or hyphens and start with a letter or digit.",
                    section_id,
                    Some(&node.node_id),
                    None,
                );
            }
            node_ids.insert(node.node_id.as_str());
            if !revision_ids.insert(node.node_id.clone()) {
                self.visual_issue(
                    ChapterIssueCode::DuplicateId,
                    "Visual node IDs must be unique within this chapter revision.",
                    section_id,
                    Some(&node.node_id),
                    None,
                );
            }
            if node.label.trim().is_empty() || node.label.len() > MAX_VISUAL_LABEL_BYTES {
                self.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    "A node label must contain 1–240 UTF-8 bytes.",
                    section_id,
                    Some(&node.node_id),
                    None,
                );
            }
            if node
                .detail
                .as_ref()
                .is_some_and(|detail| detail.len() > MAX_VISUAL_DETAIL_BYTES)
            {
                self.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    "A node detail may contain at most 512 UTF-8 bytes.",
                    section_id,
                    Some(&node.node_id),
                    None,
                );
            }
            if is_current {
                self.validate_visual_evidence(
                    &node.evidence_anchor_ids,
                    section_anchor_ids,
                    section_id,
                    &node.node_id,
                );
                self.validate_visual_source_binding(node, claims, section_anchor_ids, section_id);
            }
            if let Some(hotspot_id) = &node.hotspot_id {
                if !hotspot_ids.contains(hotspot_id) {
                    self.visual_hotspot_issue(
                        ChapterIssueCode::InvalidDestination,
                        "A visual node links to a hotspot that does not exist in this section.",
                        section_id,
                        &node.node_id,
                        hotspot_id,
                    );
                }
            }
        }
        for edge in visual.edges.iter().take(MAX_VISUAL_EDGES) {
            if !is_valid_id(&edge.edge_id) {
                self.visual_issue(
                    ChapterIssueCode::InvalidId,
                    "A visual edge ID must use 1–128 ASCII letters, digits, colons, underscores, or hyphens and start with a letter or digit.",
                    section_id,
                    Some(&edge.edge_id),
                    None,
                );
            }
            if !revision_ids.insert(edge.edge_id.clone()) {
                self.visual_issue(
                    ChapterIssueCode::DuplicateId,
                    "Visual edge IDs must be unique within this chapter revision.",
                    section_id,
                    Some(&edge.edge_id),
                    None,
                );
            }
            if edge.label.trim().is_empty() || edge.label.len() > MAX_VISUAL_LABEL_BYTES {
                self.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    "An edge label must contain 1–240 UTF-8 bytes; an unlabeled edge cannot be presented as code behavior.",
                    section_id,
                    Some(&edge.edge_id),
                    None,
                );
            }
            if is_current {
                self.validate_visual_evidence(
                    &edge.evidence_anchor_ids,
                    section_anchor_ids,
                    section_id,
                    &edge.edge_id,
                );
            }
            if !node_ids.contains(edge.from_node_id.as_str())
                || !node_ids.contains(edge.to_node_id.as_str())
            {
                self.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    "Visual edges need endpoints that exist in this visual.",
                    section_id,
                    Some(&edge.edge_id),
                    None,
                );
            }
        }
    }

    fn validate_visual_evidence(
        &mut self,
        anchor_ids: &[String],
        section_anchor_ids: &HashSet<String>,
        section_id: Option<&str>,
        visual_item_id: &str,
    ) {
        if anchor_ids.is_empty() || anchor_ids.len() > MAX_VISUAL_EVIDENCE_ANCHORS_PER_ITEM {
            self.visual_issue(
                ChapterIssueCode::InvalidVisual,
                "Every node and edge needs 1–8 exact source evidence links.",
                section_id,
                Some(visual_item_id),
                None,
            );
        }
        let mut unique = HashSet::new();
        for anchor_id in anchor_ids.iter().take(MAX_VISUAL_EVIDENCE_ANCHORS_PER_ITEM) {
            if !unique.insert(anchor_id.as_str()) {
                self.visual_issue(
                    ChapterIssueCode::DuplicateId,
                    "A visual item cannot list the same evidence anchor more than once.",
                    section_id,
                    Some(visual_item_id),
                    Some(anchor_id),
                );
            }
            if !section_anchor_ids.contains(anchor_id) {
                self.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    "A visual evidence link must refer to an exact anchor attached to a claim in this section.",
                    section_id,
                    Some(visual_item_id),
                    Some(anchor_id),
                );
            }
        }
    }

    fn validate_visual_source_binding(
        &mut self,
        node: &VisualNode,
        claims: &[ChapterClaim],
        section_anchor_ids: &HashSet<String>,
        section_id: Option<&str>,
    ) {
        let (Some(source_anchor_id), Some(source_value)) =
            (&node.source_anchor_id, &node.source_value)
        else {
            if node.source_anchor_id.is_some() || node.source_value.is_some() {
                self.visual_issue(
                    ChapterIssueCode::InvalidVisual,
                    "A visual source binding must include both its cited anchor and exact source value.",
                    section_id,
                    Some(&node.node_id),
                    node.source_anchor_id.as_deref(),
                );
            }
            return;
        };

        if !node.evidence_anchor_ids.contains(source_anchor_id)
            || !section_anchor_ids.contains(source_anchor_id)
        {
            self.visual_issue(
                ChapterIssueCode::InvalidVisual,
                "A visual source binding must use one of the node's cited section anchors.",
                section_id,
                Some(&node.node_id),
                Some(source_anchor_id),
            );
            return;
        }

        let anchor = claims
            .iter()
            .flat_map(|claim| &claim.anchors)
            .find(|anchor| anchor.anchor_id == *source_anchor_id);
        if !(1..=MAX_VISUAL_SOURCE_VALUE_BYTES).contains(&source_value.len())
            || anchor.is_none_or(|anchor| anchor.excerpt.match_indices(source_value).count() != 1)
        {
            self.visual_issue(
                ChapterIssueCode::InvalidVisual,
                "A visual source value must be a unique exact substring of its cited anchor.",
                section_id,
                Some(&node.node_id),
                Some(source_anchor_id),
            );
        }
    }

    fn visual_issue(
        &mut self,
        code: ChapterIssueCode,
        message: &str,
        section_id: Option<&str>,
        visual_item_id: Option<&str>,
        anchor_id: Option<&str>,
    ) {
        if self.issues.len() < MAX_VALIDATION_ISSUES {
            self.issues.push(ChapterValidationIssue {
                code,
                message: message.to_owned(),
                path: None,
                section_id: section_id.map(str::to_owned),
                claim_id: None,
                anchor_id: anchor_id.map(str::to_owned),
                hotspot_id: None,
                visual_item_id: visual_item_id.map(str::to_owned),
            });
        }
    }

    fn visual_hotspot_issue(
        &mut self,
        code: ChapterIssueCode,
        message: &str,
        section_id: Option<&str>,
        visual_item_id: &str,
        hotspot_id: &str,
    ) {
        if self.issues.len() < MAX_VALIDATION_ISSUES {
            self.issues.push(ChapterValidationIssue {
                code,
                message: message.to_owned(),
                path: None,
                section_id: section_id.map(str::to_owned),
                claim_id: None,
                anchor_id: None,
                hotspot_id: Some(hotspot_id.to_owned()),
                visual_item_id: Some(visual_item_id.to_owned()),
            });
        }
    }
}

fn validate_target(validator: &mut Validator<'_>, target: &ChapterTarget) {
    match &target.scope {
        ChapterTargetScope::Overview => {}
        ChapterTargetScope::File { path } => validate_target_path(validator, path),
        ChapterTargetScope::Group { paths } => {
            if paths.is_empty() || paths.len() > MAX_SECTIONS {
                validator.issue(
                    ChapterIssueCode::InvalidTarget,
                    "A target group needs between 1 and 500 captured paths.",
                    None,
                    None,
                    None,
                    None,
                );
            }
            let mut unique = HashSet::new();
            for path in paths {
                if !unique.insert(path) {
                    validator.issue(
                        ChapterIssueCode::InvalidTarget,
                        "A target group cannot repeat a path.",
                        Some(path),
                        None,
                        None,
                        None,
                    );
                }
                validate_target_path(validator, path);
            }
        }
    }
}

fn validate_target_path(validator: &mut Validator<'_>, path: &str) {
    if !validator
        .manifest
        .files
        .iter()
        .any(|file| file.path == path)
    {
        validator.issue(
            ChapterIssueCode::MissingPath,
            "The chapter target path was not captured in this snapshot.",
            Some(path),
            None,
            None,
            None,
        );
    }
}

fn validate_id(
    validator: &mut Validator<'_>,
    id: &str,
    section_id: Option<&str>,
    claim_id: Option<&str>,
    hotspot_id: Option<&str>,
) {
    if !is_valid_id(id) {
        validator.issue(
            ChapterIssueCode::InvalidId,
            "IDs must be 1–128 ASCII letters, digits, colons, underscores, or hyphens and start with a letter or digit.",
            None,
            section_id,
            claim_id,
            hotspot_id,
        );
    }
}

fn is_valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    let valid_start = chars
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric());
    let valid_tail = chars
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, ':' | '_' | '-'));
    !id.is_empty() && id.len() <= 128 && valid_start && valid_tail
}

fn register_unique_id(
    validator: &mut Validator<'_>,
    ids: &mut HashSet<String>,
    id: &str,
    section_id: Option<&str>,
    claim_id: Option<&str>,
    hotspot_id: Option<&str>,
) {
    if !ids.insert(id.to_owned()) {
        validator.issue(
            ChapterIssueCode::DuplicateId,
            "IDs must be unique within a chapter revision.",
            None,
            section_id,
            claim_id,
            hotspot_id,
        );
    }
}

fn inclusive_range(anchor: &ChapterSourceAnchor, source_len: usize) -> Option<(usize, usize)> {
    let start = usize::try_from(anchor.start_byte).ok()?;
    let end_inclusive = usize::try_from(anchor.end_byte_inclusive).ok()?;
    if start > end_inclusive || end_inclusive >= source_len {
        return None;
    }
    Some((start, end_inclusive.checked_add(1)?))
}

fn byte_position(bytes: &[u8], offset: usize) -> (u32, u32) {
    let prefix = &bytes[..offset];
    let line = prefix.iter().filter(|byte| **byte == b'\n').count() as u32 + 1;
    let column = prefix
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(prefix.len(), |last_newline| prefix.len() - last_newline - 1)
        as u32
        + 1;
    (line, column)
}

fn symbol_contains_anchor(symbol: &IndexedSymbol, anchor: &ChapterSourceAnchor) -> bool {
    let Ok(end_inclusive) = usize::try_from(anchor.end_byte_inclusive) else {
        return false;
    };
    symbol.path == anchor.path
        && symbol.anchor.source_sha256 == anchor.source_sha256
        && symbol.anchor.start_byte as u64 <= anchor.start_byte
        && symbol.anchor.end_byte > end_inclusive
}

fn file_indexed_cleanly(index: &SnapshotIndex, path: &str) -> bool {
    index
        .files
        .iter()
        .any(|file| file.path == path && file.status == IndexFileStatus::Indexed)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub fn remap_anchor_between_snapshots(
    storage_dir: &Path,
    old_manifest: &SnapshotManifest,
    old_index: Option<&SnapshotIndex>,
    new_manifest: &SnapshotManifest,
    new_index: Option<&SnapshotIndex>,
    anchor: &ChapterSourceAnchor,
) -> AnchorRemapResult {
    if old_manifest.summary.project_id != new_manifest.summary.project_id
        || anchor.snapshot_id != old_manifest.summary.snapshot_id
    {
        return remap_unavailable(
            ChapterIssueCode::WrongSnapshot,
            "The old anchor and the two snapshots must belong to the same project and source snapshot.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    let Some(symbol_id) = anchor.symbol_id.as_deref() else {
        return remap_unavailable(
            ChapterIssueCode::MissingSymbol,
            "This source range has no stable symbol ID, so it cannot be remapped automatically.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    };
    let Some(old_index) = old_index else {
        return remap_unavailable(
            ChapterIssueCode::IndexUnavailable,
            "The old snapshot symbol index is unavailable; automatic remapping is disabled.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    };
    if old_index.project_id != old_manifest.summary.project_id
        || old_index.snapshot_id != old_manifest.summary.snapshot_id
    {
        return remap_unavailable(
            ChapterIssueCode::WrongSnapshot,
            "The old symbol index does not belong to the source snapshot.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    let Some(old_symbol) = old_index
        .symbols
        .iter()
        .find(|symbol| symbol.symbol_id == symbol_id)
    else {
        return remap_unavailable(
            ChapterIssueCode::MissingSymbol,
            "The old stable symbol ID is not present in its saved snapshot index.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    };
    if !file_indexed_cleanly(old_index, &anchor.path) {
        return remap_unavailable(
            ChapterIssueCode::IndexUnavailable,
            "The old file does not have a complete parser result, so automatic remapping is disabled.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    if !symbol_contains_anchor(old_symbol, anchor) {
        return remap_unavailable(
            ChapterIssueCode::RangeMismatch,
            "The old source range is not inside the indexed declaration for its stable symbol ID.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    let old_source =
        match snapshot::read_snapshot_source_from_manifest(storage_dir, old_manifest, &anchor.path)
        {
            Ok(source) => source,
            Err(_) => {
                return remap_unavailable(
                    ChapterIssueCode::SourceUnavailable,
                    "The old captured source is unavailable or failed its integrity check.",
                    Some(&anchor.path),
                    Some(&anchor.anchor_id),
                );
            }
        };
    if let Err((code, message)) = verify_anchor_against_source(anchor, &old_source) {
        return remap_unavailable(code, message, Some(&anchor.path), Some(&anchor.anchor_id));
    }

    let Some(new_index) = new_index else {
        return remap_unavailable(
            ChapterIssueCode::IndexUnavailable,
            "The target snapshot has no saved symbol index; automatic remapping is disabled.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    };
    if new_index.project_id != new_manifest.summary.project_id
        || new_index.snapshot_id != new_manifest.summary.snapshot_id
    {
        return remap_unavailable(
            ChapterIssueCode::WrongSnapshot,
            "The new symbol index does not belong to the requested target snapshot.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    let Some(new_symbol) = new_index
        .symbols
        .iter()
        .find(|symbol| symbol.symbol_id == symbol_id)
    else {
        return remap_unavailable(
            ChapterIssueCode::MissingSymbol,
            "The target snapshot no longer contains the same stable symbol ID.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    };
    if !file_indexed_cleanly(new_index, &anchor.path) {
        return remap_unavailable(
            ChapterIssueCode::IndexUnavailable,
            "The target file does not have a complete parser result, so automatic remapping is disabled.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    if new_symbol.path != anchor.path {
        return remap_unavailable(
            ChapterIssueCode::MissingSymbol,
            "The target symbol ID resolves to a different path.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    let new_source =
        match snapshot::read_snapshot_source_from_manifest(storage_dir, new_manifest, &anchor.path)
        {
            Ok(source) => source,
            Err(_) => {
                return remap_unavailable(
                    ChapterIssueCode::MissingPath,
                    "The target snapshot does not contain readable captured source at this path.",
                    Some(&anchor.path),
                    Some(&anchor.anchor_id),
                );
            }
        };
    if new_symbol.anchor.source_sha256 != new_source.sha256
        || sha256_hex(new_source.contents.as_bytes()) != new_source.sha256
    {
        return remap_unavailable(
            ChapterIssueCode::HashMismatch,
            "The target symbol index and captured source do not have the same SHA-256 hash.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    let (Some(symbol_start), Some(symbol_end)) = (
        usize::try_from(new_symbol.anchor.start_byte).ok(),
        usize::try_from(new_symbol.anchor.end_byte).ok(),
    ) else {
        return remap_unavailable(
            ChapterIssueCode::RangeMismatch,
            "The target symbol anchor cannot be represented on this platform.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    };
    let new_bytes = new_source.contents.as_bytes();
    if symbol_start >= symbol_end || symbol_end > new_bytes.len() {
        return remap_unavailable(
            ChapterIssueCode::RangeMismatch,
            "The target symbol range is outside its captured source bytes.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    let Ok(symbol_text) = std::str::from_utf8(&new_bytes[symbol_start..symbol_end]) else {
        return remap_unavailable(
            ChapterIssueCode::RangeMismatch,
            "The target symbol range does not align to UTF-8 boundaries.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    };
    let needle = anchor.excerpt.as_bytes();
    let mut match_offset = None;
    let mut has_multiple_matches = false;
    if needle.len() <= symbol_text.len() {
        for (offset, candidate) in symbol_text.as_bytes().windows(needle.len()).enumerate() {
            if candidate == needle {
                if match_offset.is_some() {
                    has_multiple_matches = true;
                    break;
                }
                match_offset = Some(offset);
            }
        }
    }
    if has_multiple_matches {
        return remap_unavailable(
            ChapterIssueCode::AmbiguousRemap,
            "The old excerpt appears more than once inside the target symbol, so its destination is ambiguous.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    if match_offset.is_none() {
        return remap_unavailable(
            ChapterIssueCode::ExcerptMismatch,
            "The exact old excerpt no longer appears inside the same stable symbol.",
            Some(&anchor.path),
            Some(&anchor.anchor_id),
        );
    }
    let new_start = symbol_start + match_offset.expect("one excerpt match was found");
    let new_end_inclusive = new_start + anchor.excerpt.len() - 1;
    let mut remapped = anchor.clone();
    remapped.snapshot_id = new_manifest.summary.snapshot_id.clone();
    remapped.source_sha256 = new_source.sha256.clone();
    remapped.start_byte = new_start as u64;
    remapped.end_byte_inclusive = new_end_inclusive as u64;
    (remapped.start_line, remapped.start_column) = byte_position(new_bytes, new_start);
    (remapped.end_line, remapped.end_column) = byte_position(new_bytes, new_end_inclusive);
    if let Err((code, message)) = verify_anchor_against_source(&remapped, &new_source) {
        return remap_unavailable(code, message, Some(&anchor.path), Some(&anchor.anchor_id));
    }
    AnchorRemapResult {
        status: AnchorRemapStatus::Remapped,
        anchor: Some(remapped),
        issues: Vec::new(),
    }
}

fn verify_anchor_against_source(
    anchor: &ChapterSourceAnchor,
    source: &SnapshotSource,
) -> Result<(), (ChapterIssueCode, &'static str)> {
    if source.snapshot_id != anchor.snapshot_id {
        return Err((
            ChapterIssueCode::WrongSnapshot,
            "The source bytes do not belong to the anchor snapshot.",
        ));
    }
    if source.path != anchor.path || source.sha256 != anchor.source_sha256 {
        return Err((
            ChapterIssueCode::HashMismatch,
            "The anchor path or hash does not match the captured source.",
        ));
    }
    if sha256_hex(source.contents.as_bytes()) != source.sha256 {
        return Err((
            ChapterIssueCode::HashMismatch,
            "The captured source does not match its recorded SHA-256 hash.",
        ));
    }
    if anchor.excerpt.is_empty() || anchor.excerpt.len() > MAX_EXCERPT_BYTES {
        return Err((
            ChapterIssueCode::RangeMismatch,
            "The anchor excerpt is empty or exceeds the 64 KiB evidence limit.",
        ));
    }
    let Some((start, end_exclusive)) = inclusive_range(anchor, source.contents.len()) else {
        return Err((
            ChapterIssueCode::RangeMismatch,
            "The inclusive byte range is outside the captured file or is reversed.",
        ));
    };
    let bytes = source.contents.as_bytes();
    let Ok(excerpt) = std::str::from_utf8(&bytes[start..end_exclusive]) else {
        return Err((
            ChapterIssueCode::RangeMismatch,
            "The anchor byte range does not fall on UTF-8 character boundaries.",
        ));
    };
    if excerpt != anchor.excerpt {
        return Err((
            ChapterIssueCode::ExcerptMismatch,
            "The stored excerpt does not equal the selected captured source bytes.",
        ));
    }
    if byte_position(bytes, start) != (anchor.start_line, anchor.start_column)
        || byte_position(bytes, end_exclusive - 1) != (anchor.end_line, anchor.end_column)
    {
        return Err((
            ChapterIssueCode::RangeMismatch,
            "The stored inclusive line or byte-column range does not match its byte offsets.",
        ));
    }
    Ok(())
}

fn remap_unavailable(
    code: ChapterIssueCode,
    message: &str,
    path: Option<&str>,
    anchor_id: Option<&str>,
) -> AnchorRemapResult {
    AnchorRemapResult {
        status: AnchorRemapStatus::Unavailable,
        anchor: None,
        issues: vec![ChapterValidationIssue {
            code,
            message: message.to_owned(),
            path: path.map(str::to_owned),
            section_id: None,
            claim_id: None,
            anchor_id: anchor_id.map(str::to_owned),
            hotspot_id: None,
            visual_item_id: None,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{self, RelationshipKind};
    use crate::project::{AccessStatus, RecentProject};
    use crate::snapshot::CaptureLimits;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::AtomicBool;
    use uuid::Uuid;

    #[test]
    fn line_citations_use_exact_utf8_crlf_and_final_line_bytes() {
        let text = "let café = 1;\r\nnext(\"🧭\");\nlast";
        let source = SnapshotSource {
            snapshot_id: "snapshot:lines".into(),
            path: "src/lines.rs".into(),
            sha256: sha256_hex(text.as_bytes()),
            size_bytes: text.len() as u64,
            contents: text.into(),
        };
        let anchor = anchor_from_lines(&source, None, "anchor:lines", 2, 2).unwrap();
        let first = text.split_inclusive('\n').next().unwrap();
        let selected = text.split_inclusive('\n').nth(1).unwrap();
        assert_eq!(anchor.start_byte, first.len() as u64);
        assert_eq!(
            anchor.end_byte_inclusive,
            (first.len() + selected.len() - 1) as u64
        );
        assert_eq!(anchor.excerpt, selected);
        assert_eq!(anchor.start_line, 2);
        assert_eq!(anchor.end_line, 2);
        assert_eq!(anchor.end_column, selected.len() as u32);
        verify_anchor_against_source(&anchor, &source).unwrap();
        let last = anchor_from_lines(&source, None, "anchor:last", 3, 3).unwrap();
        assert_eq!(last.excerpt, "last");
        verify_anchor_against_source(&last, &source).unwrap();
        assert!(anchor_from_lines(&source, None, "anchor:last", 4, 4).is_err());
        let mut damaged = source.clone();
        damaged.sha256 = "f".repeat(64);
        assert!(anchor_from_lines(&damaged, None, "anchor:bad", 1, 1).is_err());
    }

    #[test]
    fn line_citations_reject_invalid_identity_ranges_and_oversized_excerpts() {
        let source = SnapshotSource {
            snapshot_id: "snapshot:lines".into(),
            path: "src/lines.rs".into(),
            sha256: sha256_hex(b"\n"),
            size_bytes: 1,
            contents: "\n".into(),
        };
        assert_eq!(
            anchor_from_lines(&source, None, "anchor:blank", 1, 1)
                .unwrap()
                .excerpt,
            "\n"
        );
        for (id, from, to) in [
            ("bad/id", 1, 1),
            ("anchor:zero", 0, 1),
            ("anchor:reverse", 2, 1),
            ("anchor:missing", 2, 2),
            ("anchor:huge", 1, u32::MAX),
        ] {
            assert!(anchor_from_lines(&source, None, id, from, to).is_err());
        }
        for len in [0, MAX_EXCERPT_BYTES + 1] {
            let text = "x".repeat(len);
            let too_large = SnapshotSource {
                contents: text.clone(),
                sha256: sha256_hex(text.as_bytes()),
                size_bytes: len as u64,
                ..source.clone()
            };
            assert!(anchor_from_lines(&too_large, None, "anchor:limit", 1, 1).is_err());
        }
    }

    const FIXTURE_SOURCE: &str =
        "pub fn answer() -> i32 { compute() }\nfn compute() -> i32 { 42 }\n";

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "code-atlas-chapter-{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git should be available for snapshot fixtures");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn commit(root: &Path, message: &str) {
        git(root, &["add", "--all"]);
        git(root, &["commit", "-qm", message]);
    }

    fn fixture(
        temporary: &TestDirectory,
    ) -> (
        PathBuf,
        PathBuf,
        RecentProject,
        SnapshotManifest,
        SnapshotIndex,
    ) {
        let root = temporary.path().join("repository");
        fs::create_dir_all(root.join("src")).unwrap();
        git(&root, &["init", "-q", "--initial-branch=main"]);
        git(&root, &["config", "user.name", "Chapter Schema Tests"]);
        git(&root, &["config", "user.email", "chapters@example.invalid"]);
        fs::write(root.join("src/lib.rs"), FIXTURE_SOURCE).unwrap();
        commit(&root, "chapter schema fixture");

        let project_path = root.canonicalize().unwrap().display().to_string();
        let project = RecentProject {
            project_id: project_path.clone(),
            display_name: "chapter fixture".to_owned(),
            selected_path: project_path.clone(),
            repository_root: project_path,
            branch: Some("main".to_owned()),
            head: None,
            access_status: AccessStatus::Available,
            last_opened_unix_ms: 0,
        };
        let storage = temporary.path().join("app-data");
        let progress =
            snapshot::new_capture_progress(&storage, &project, CaptureLimits::default()).unwrap();
        let manifest = snapshot::capture_snapshot_with_progress(
            &storage,
            &project,
            progress,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        let index = index::build_snapshot_index(
            &storage,
            &manifest,
            "chapter-schema-index",
            &AtomicBool::new(false),
            |_| {},
        );
        index::save_snapshot_index(&storage, &index).unwrap();
        (root, storage, project, manifest, index)
    }

    fn anchor_for(
        source: &SnapshotSource,
        symbol_id: Option<String>,
        excerpt: &str,
        anchor_id: &str,
    ) -> ChapterSourceAnchor {
        let start = source
            .contents
            .find(excerpt)
            .expect("fixture should contain the exact anchor excerpt");
        let end_inclusive = start + excerpt.len() - 1;
        let (start_line, start_column) = byte_position(source.contents.as_bytes(), start);
        let (end_line, end_column) = byte_position(source.contents.as_bytes(), end_inclusive);
        ChapterSourceAnchor {
            anchor_id: anchor_id.to_owned(),
            path: source.path.clone(),
            symbol_id,
            snapshot_id: source.snapshot_id.clone(),
            source_sha256: source.sha256.clone(),
            start_byte: start as u64,
            end_byte_inclusive: end_inclusive as u64,
            start_line,
            end_line,
            start_column,
            end_column,
            excerpt: excerpt.to_owned(),
        }
    }

    fn valid_revision(
        storage: &Path,
        project: &RecentProject,
        manifest: &SnapshotManifest,
        index: &SnapshotIndex,
    ) -> ChapterRevision {
        let source =
            snapshot::read_snapshot_source_from_manifest(storage, manifest, "src/lib.rs").unwrap();
        let answer = index
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/lib.rs" && symbol.name == "answer")
            .unwrap();
        let compute = index
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/lib.rs" && symbol.name == "compute")
            .unwrap();
        let call = index
            .relationships
            .iter()
            .find(|edge| {
                edge.kind == RelationshipKind::Calls
                    && edge.target_id.as_deref() == Some(compute.symbol_id.as_str())
            })
            .unwrap();
        let call_anchor = anchor_from_index(
            &call.evidence,
            &manifest.summary.snapshot_id,
            Some(answer),
            "anchor:compute-call",
            &source,
        )
        .expect("an indexed relationship should produce an exact chapter anchor");
        let result_anchor = anchor_for(
            &source,
            Some(compute.symbol_id.clone()),
            "42",
            "anchor:return-value",
        );
        let context_anchor = anchor_from_index(
            &answer.anchor,
            &manifest.summary.snapshot_id,
            None,
            "anchor:public-entry",
            &source,
        )
        .expect("an indexed symbol should produce an exact chapter anchor");
        let code_claim = ChapterClaim {
            claim_id: "claim:call-flow".to_owned(),
            category: ClaimCategory::CodeBehavior,
            prose: "The public answer function calls compute and returns its result.".to_owned(),
            anchors: vec![call_anchor.clone(), result_anchor.clone()],
            dependencies: vec![
                SectionDependency::Symbol {
                    path: "src/lib.rs".to_owned(),
                    source_sha256: source.sha256.clone(),
                    symbol_id: answer.symbol_id.clone(),
                },
                SectionDependency::Relationship {
                    path: "src/lib.rs".to_owned(),
                    source_sha256: source.sha256.clone(),
                    relationship_id: call.relationship_id.clone(),
                },
            ],
        };
        let context_claim = ChapterClaim {
            claim_id: "claim:repository-context".to_owned(),
            category: ClaimCategory::RepositoryContext,
            prose: "This file contains the public entry point and its helper.".to_owned(),
            anchors: vec![context_anchor.clone()],
            dependencies: Vec::new(),
        };
        let inference_claim = ChapterClaim {
            claim_id: "claim:intent-inference".to_owned(),
            category: ClaimCategory::Inference {
                visible_label: "Inference".to_owned(),
                basis:
                    "The small helper is only called by the public entry point in this snapshot."
                        .to_owned(),
            },
            prose: "The helper appears to keep the public function's implementation small."
                .to_owned(),
            anchors: vec![anchor_for(
                &source,
                Some(answer.symbol_id.clone()),
                "compute()",
                "anchor:compute-call-inference",
            )],
            dependencies: Vec::new(),
        };
        ChapterRevision {
            schema_version: CHAPTER_SCHEMA_VERSION,
            chapter_id: "chapter:answer".to_owned(),
            revision_id: "revision:answer:1".to_owned(),
            revision_number: 1,
            project_id: project.project_id.clone(),
            snapshot_id: manifest.summary.snapshot_id.clone(),
            target: ChapterTarget {
                target_id: "target:answer-file-group".to_owned(),
                scope: ChapterTargetScope::Group {
                    paths: vec!["src/lib.rs".to_owned()],
                },
            },
            title: "How the answer is computed".to_owned(),
            lifecycle: RevisionLifecycle::Draft,
            validation_status: ValidationStatus::Unvalidated,
            created_at_unix_ms: 1_800_000_000_000,
            based_on_revision_id: None,
            decision_note: None,
            generation_origin: None,
            sections: vec![
                ChapterSection {
                    section_id: "section:entry".to_owned(),
                    section_revision_id: "section-revision:entry:1".to_owned(),
                    order: 0,
                    title: "Follow the call".to_owned(),
                    prose: "Start at answer, then follow the exact call into compute.".to_owned(),
                    spoken_text: "The answer function delegates the calculation to compute."
                        .to_owned(),
                    claims: vec![code_claim, context_claim, inference_claim],
                    dependencies: vec![
                        SectionDependency::SourceFile {
                            path: "src/lib.rs".to_owned(),
                            source_sha256: source.sha256.clone(),
                        },
                        SectionDependency::Symbol {
                            path: "src/lib.rs".to_owned(),
                            source_sha256: source.sha256.clone(),
                            symbol_id: answer.symbol_id.clone(),
                        },
                    ],
                    visual: Some(ChapterVisualPayload::Typed(VisualSpec {
                        presentation: None,
                        schema_version: VISUAL_SCHEMA_VERSION,
                        kind: VisualKind::DataFlow,
                        title: "Call path".to_owned(),
                        layout_hint: Some(VisualLayoutHint {
                            direction: VisualLayoutDirection::LeftToRight,
                        }),
                        nodes: vec![
                            VisualNode {
                                node_id: "node:answer".to_owned(),
                                label: "answer".to_owned(),
                                detail: Some("public entry point".to_owned()),
                                hotspot_id: Some("hotspot:source".to_owned()),
                                evidence_anchor_ids: vec![call_anchor.anchor_id.clone()],
                                source_anchor_id: None,
                                source_value: None,
                            },
                            VisualNode {
                                node_id: "node:compute".to_owned(),
                                label: "compute".to_owned(),
                                detail: None,
                                hotspot_id: Some("hotspot:symbol".to_owned()),
                                evidence_anchor_ids: vec![result_anchor.anchor_id.clone()],
                                source_anchor_id: None,
                                source_value: None,
                            },
                        ],
                        edges: vec![VisualEdge {
                            edge_id: "edge:answer-compute".to_owned(),
                            from_node_id: "node:answer".to_owned(),
                            to_node_id: "node:compute".to_owned(),
                            label: "calls".to_owned(),
                            evidence_anchor_ids: vec![call_anchor.anchor_id.clone()],
                        }],
                    })),
                    hotspots: vec![
                        ChapterHotspot {
                            hotspot_id: "hotspot:next-section".to_owned(),
                            label: "Open the result section".to_owned(),
                            destination: HotspotDestination::Section {
                                section_id: "section:result".to_owned(),
                            },
                        },
                        ChapterHotspot {
                            hotspot_id: "hotspot:source".to_owned(),
                            label: "Open the exact call in Code Lens".to_owned(),
                            destination: HotspotDestination::CodeLens {
                                anchor_id: call_anchor.anchor_id.clone(),
                            },
                        },
                        ChapterHotspot {
                            hotspot_id: "hotspot:file".to_owned(),
                            label: "Open the source file in Atlas".to_owned(),
                            destination: HotspotDestination::AtlasFile {
                                path: "src/lib.rs".to_owned(),
                            },
                        },
                        ChapterHotspot {
                            hotspot_id: "hotspot:symbol".to_owned(),
                            label: "Open the helper symbol in Atlas".to_owned(),
                            destination: HotspotDestination::AtlasSymbol {
                                path: "src/lib.rs".to_owned(),
                                symbol_id: compute.symbol_id.clone(),
                            },
                        },
                        ChapterHotspot {
                            hotspot_id: "hotspot:relationship".to_owned(),
                            label: "Follow the resolved call".to_owned(),
                            destination: HotspotDestination::Relationship {
                                relationship_id: call.relationship_id.clone(),
                            },
                        },
                    ],
                },
                ChapterSection {
                    section_id: "section:result".to_owned(),
                    section_revision_id: "section-revision:result:1".to_owned(),
                    order: 1,
                    title: "Read the result".to_owned(),
                    prose: "The helper returns the integer used by the caller.".to_owned(),
                    spoken_text: String::new(),
                    claims: Vec::new(),
                    dependencies: vec![SectionDependency::SourceFile {
                        path: "src/lib.rs".to_owned(),
                        source_sha256: source.sha256,
                    }],
                    visual: None,
                    hotspots: Vec::new(),
                },
            ],
        }
    }

    fn issue_codes(report: &ChapterValidationReport) -> HashSet<ChapterIssueCode> {
        report
            .issues
            .iter()
            .map(|issue| issue.code.clone())
            .collect()
    }

    #[test]
    fn chapter_enum_fields_match_frontend_and_reject_snake_case_aliases() {
        let destinations = [
            (
                HotspotDestination::Section {
                    section_id: "section:one".to_owned(),
                },
                "sectionId",
                "section_id",
            ),
            (
                HotspotDestination::CodeLens {
                    anchor_id: "anchor:one".to_owned(),
                },
                "anchorId",
                "anchor_id",
            ),
            (
                HotspotDestination::AtlasSymbol {
                    path: "a.ts".to_owned(),
                    symbol_id: "symbol:one".to_owned(),
                },
                "symbolId",
                "symbol_id",
            ),
            (
                HotspotDestination::Relationship {
                    relationship_id: "relationship:one".to_owned(),
                },
                "relationshipId",
                "relationship_id",
            ),
        ];
        for (destination, camel, legacy) in destinations {
            let mut value = serde_json::to_value(&destination).unwrap();
            assert!(value.get(camel).is_some());
            assert!(value.get(legacy).is_none());
            assert_eq!(
                serde_json::from_value::<HotspotDestination>(value.clone()).unwrap(),
                destination
            );
            let id = value.as_object_mut().unwrap().remove(camel).unwrap();
            let mut old_shape = value;
            old_shape[legacy] = id;
            assert!(serde_json::from_value::<HotspotDestination>(old_shape).is_err());
        }
        let category = ClaimCategory::Inference {
            visible_label: "Inference".to_owned(),
            basis: "Selected source interpretation".to_owned(),
        };
        let value = serde_json::to_value(&category).unwrap();
        assert_eq!(value["visibleLabel"], "Inference");
        let old_shape = serde_json::json!({"kind":"inference","visible_label":"Inference","basis":"Selected source interpretation"});
        assert!(serde_json::from_value::<ClaimCategory>(old_shape).is_err());
        let dependency = SectionDependency::Symbol {
            path: "a.ts".to_owned(),
            source_sha256: "hash".to_owned(),
            symbol_id: "symbol:one".to_owned(),
        };
        let value = serde_json::to_value(&dependency).unwrap();
        assert_eq!(value["sourceSha256"], "hash");
        assert_eq!(value["symbolId"], "symbol:one");
        let old_shape = serde_json::json!({"kind":"symbol","path":"a.ts","source_sha256":"hash","symbol_id":"symbol:one"});
        assert!(serde_json::from_value::<SectionDependency>(old_shape).is_err());
    }

    #[test]
    fn valid_revision_exercises_the_versioned_typed_payload_and_round_trips_immutably() {
        let temporary = TestDirectory::new();
        let (_root, storage, project, manifest, index) = fixture(&temporary);
        let mut revision = valid_revision(&storage, &project, &manifest, &index);
        let report = validate_chapter_revision(&storage, &manifest, Some(&index), &revision);
        assert_eq!(report.status, ValidationReportStatus::Valid, "{report:#?}");
        assert!(report.issues.is_empty());
        let mut file_target_revision = revision.clone();
        file_target_revision.target.scope = ChapterTargetScope::File {
            path: "src/lib.rs".to_owned(),
        };
        assert_eq!(
            validate_chapter_revision(&storage, &manifest, Some(&index), &file_target_revision)
                .status,
            ValidationReportStatus::Valid
        );

        // Accepted revisions are immutable values: reading and serializing one does not
        // update its validation state, anchors, visual spec, or any other payload field.
        revision.lifecycle = RevisionLifecycle::Accepted;
        revision.validation_status = ValidationStatus::Valid;
        revision.based_on_revision_id = Some("revision:prior-draft".to_owned());
        let before = serde_json::to_value(&revision).unwrap();
        let encoded = encode_chapter_revision(&revision).unwrap();
        let decoded = decode_chapter_revision(&encoded).unwrap();
        assert_eq!(decoded, revision);
        let after: serde_json::Value =
            serde_json::from_slice(&encode_chapter_revision(&decoded).unwrap()).unwrap();
        assert_eq!(before, after);
        assert_eq!(
            validate_chapter_revision(&storage, &manifest, Some(&index), &decoded).status,
            ValidationReportStatus::Valid
        );

        let mut unsupported_shape = before.clone();
        unsupported_shape["sections"][0]["visual"]["futureRendererField"] =
            serde_json::json!("not executable data");
        let unsupported_bytes = serde_json::to_vec(&unsupported_shape).unwrap();
        let safe_read = decode_chapter_revision(&unsupported_bytes).unwrap();
        assert!(matches!(
            safe_read.sections[0].visual.as_ref(),
            Some(ChapterVisualPayload::Unavailable(_))
        ));
        assert_eq!(
            serde_json::to_value(
                decode_chapter_revision(&encode_chapter_revision(&safe_read).unwrap()).unwrap()
            )
            .unwrap(),
            unsupported_shape,
            "an unsupported non-executable visual remains value-equivalent when read and encoded"
        );

        let mut semantically_invalid_visual = serde_json::to_value(&revision).unwrap();
        semantically_invalid_visual["sections"][0]["visual"]["layoutHint"] =
            serde_json::Value::Null;
        semantically_invalid_visual["sections"][0]["visual"]["nodes"][0]["evidenceAnchorIds"] =
            serde_json::json!([]);
        let invalid_bytes = serde_json::to_vec(&semantically_invalid_visual).unwrap();
        let invalid_visual_revision = decode_chapter_revision(&invalid_bytes).unwrap();
        assert!(matches!(
            invalid_visual_revision.sections[0].visual.as_ref(),
            Some(ChapterVisualPayload::Decoded { .. })
        ));
        let invalid_round_trip = encode_chapter_revision(&invalid_visual_revision).unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&invalid_round_trip).unwrap(),
            semantically_invalid_visual,
            "serde-decodable but semantically invalid visual fields retain their original JSON shape"
        );
        let invalid_report =
            validate_chapter_revision(&storage, &manifest, Some(&index), &invalid_visual_revision);
        assert_eq!(invalid_report.status, ValidationReportStatus::Invalid);

        let mut edited_invalid_visual = invalid_visual_revision.clone();
        let visual = edited_invalid_visual.sections[0]
            .visual
            .as_mut()
            .and_then(ChapterVisualPayload::typed_visual_mut)
            .expect("structurally typed visual remains inspectable");
        visual.title = "Edited call path".to_owned();
        let edited_value: serde_json::Value =
            serde_json::from_slice(&encode_chapter_revision(&edited_invalid_visual).unwrap())
                .unwrap();
        assert_eq!(
            edited_value["sections"][0]["visual"]["title"], "Edited call path",
            "editing the typed view serializes the edit instead of replaying stale raw JSON"
        );

        let unsafe_visual_report =
            validate_chapter_revision(&storage, &manifest, Some(&index), &safe_read);
        assert!(
            unsafe_visual_report
                .issues
                .iter()
                .any(|issue| issue.code == ChapterIssueCode::InvalidVisual)
        );
        let mut future_version = before;
        future_version["schemaVersion"] = serde_json::json!(CHAPTER_SCHEMA_VERSION + 1);
        assert!(decode_chapter_revision(&serde_json::to_vec(&future_version).unwrap()).is_err());
    }

    #[test]
    fn chapter_decoder_rejects_html_and_script_fields_in_visual_payloads() {
        let temporary = TestDirectory::new();
        let (_root, storage, project, manifest, index) = fixture(&temporary);
        let revision = valid_revision(&storage, &project, &manifest, &index);
        let baseline = serde_json::to_value(&revision).unwrap();

        for field in ["html", "HTML", "script", "Script", "javascript"] {
            let mut executable_visual = baseline.clone();
            executable_visual["sections"][0]["visual"][field] =
                serde_json::json!("<script>run()</script>");
            assert_eq!(
                decode_chapter_revision(&serde_json::to_vec(&executable_visual).unwrap())
                    .unwrap_err(),
                "Chapter visuals cannot contain HTML or script fields.",
                "root visual field {field:?} must be rejected"
            );
        }

        let mut nested_script = baseline;
        nested_script["sections"][0]["visual"]["nodes"][0]["script"] = serde_json::json!("run()");
        assert_eq!(
            decode_chapter_revision(&serde_json::to_vec(&nested_script).unwrap()).unwrap_err(),
            "Chapter visuals cannot contain HTML or script fields.",
            "executable-shaped fields are rejected at any depth"
        );
    }

    #[test]
    fn visual_validation_reports_item_and_anchor_for_malformed_typed_inputs() {
        let temporary = TestDirectory::new();
        let (_root, storage, project, manifest, index) = fixture(&temporary);
        let base = valid_revision(&storage, &project, &manifest, &index);

        let mut missing_evidence = base.clone();
        let Some(visual) = missing_evidence.sections[0]
            .visual
            .as_mut()
            .and_then(ChapterVisualPayload::typed_visual_mut)
        else {
            panic!("fixture should contain a typed visual");
        };
        visual.edges[0].evidence_anchor_ids.clear();
        let report =
            validate_chapter_revision(&storage, &manifest, Some(&index), &missing_evidence);
        let issue = report.issues.iter().find(|issue| {
            issue.code == ChapterIssueCode::InvalidVisual
                && issue.visual_item_id.as_deref() == Some("edge:answer-compute")
        });
        assert!(issue.is_some(), "{report:#?}");

        let mut non_unique_source_binding = base.clone();
        let visual = non_unique_source_binding.sections[0]
            .visual
            .as_mut()
            .and_then(ChapterVisualPayload::typed_visual_mut)
            .expect("fixture should contain a typed visual");
        visual.nodes[0].source_anchor_id =
            Some(base.sections[0].claims[0].anchors[0].anchor_id.clone());
        visual.nodes[0].source_value = Some("not present in the cited excerpt".to_owned());
        let report = validate_chapter_revision(
            &storage,
            &manifest,
            Some(&index),
            &non_unique_source_binding,
        );
        assert!(
            report.issues.iter().any(|issue| {
                issue.code == ChapterIssueCode::InvalidVisual
                    && issue.message.contains("unique exact substring")
            }),
            "{report:#?}"
        );

        let mut foreign_source_binding = base.clone();
        let visual = foreign_source_binding.sections[0]
            .visual
            .as_mut()
            .and_then(ChapterVisualPayload::typed_visual_mut)
            .expect("fixture should contain a typed visual");
        visual.nodes[0].source_anchor_id = Some("anchor:foreign".to_owned());
        visual.nodes[0].source_value = Some("compute()".to_owned());
        let report =
            validate_chapter_revision(&storage, &manifest, Some(&index), &foreign_source_binding);
        assert!(
            report.issues.iter().any(|issue| {
                issue.code == ChapterIssueCode::InvalidVisual
                    && issue.message.contains("node's cited section anchors")
            }),
            "{report:#?}"
        );

        for unsupported_version in [1, VISUAL_SCHEMA_VERSION + 1] {
            let mut unsupported_visual_version = base.clone();
            let Some(visual) = unsupported_visual_version.sections[0]
                .visual
                .as_mut()
                .and_then(ChapterVisualPayload::typed_visual_mut)
            else {
                panic!("fixture should contain a typed visual");
            };
            visual.schema_version = unsupported_version;
            let report = validate_chapter_revision(
                &storage,
                &manifest,
                Some(&index),
                &unsupported_visual_version,
            );
            assert!(
                report.issues.iter().any(|issue| {
                    issue.code == ChapterIssueCode::InvalidVisual
                        && issue.message.contains("schema version 2")
                }),
                "{report:#?}"
            );
        }

        let mut oversized_visual = base.clone();
        let Some(visual) = oversized_visual.sections[0]
            .visual
            .as_mut()
            .and_then(ChapterVisualPayload::typed_visual_mut)
        else {
            panic!("fixture should contain a typed visual");
        };
        let template = visual.nodes[0].clone();
        visual.nodes = (0..=MAX_VISUAL_NODES)
            .map(|index| VisualNode {
                node_id: format!("node:oversized-{index}"),
                hotspot_id: None,
                ..template.clone()
            })
            .collect();
        visual.edges[0].from_node_id = "node:oversized-0".to_owned();
        visual.edges[0].to_node_id = "node:oversized-1".to_owned();
        let report =
            validate_chapter_revision(&storage, &manifest, Some(&index), &oversized_visual);
        assert!(
            report.issues.iter().any(|issue| {
                issue.code == ChapterIssueCode::InvalidVisual
                    && issue.message.contains("1–500 nodes")
            }),
            "{report:#?}"
        );

        let mut missing_anchor = base.clone();
        let Some(visual) = missing_anchor.sections[0]
            .visual
            .as_mut()
            .and_then(ChapterVisualPayload::typed_visual_mut)
        else {
            panic!("fixture should contain a typed visual");
        };
        visual.nodes[0].evidence_anchor_ids = vec!["anchor:missing".to_owned()];
        let report = validate_chapter_revision(&storage, &manifest, Some(&index), &missing_anchor);
        assert!(
            report.issues.iter().any(|issue| {
                issue.code == ChapterIssueCode::InvalidVisual
                    && issue.visual_item_id.as_deref() == Some("node:answer")
                    && issue.anchor_id.as_deref() == Some("anchor:missing")
            }),
            "{report:#?}"
        );

        let mut duplicate_namespace_id = base.clone();
        let Some(visual) = duplicate_namespace_id.sections[0]
            .visual
            .as_mut()
            .and_then(ChapterVisualPayload::typed_visual_mut)
        else {
            panic!("fixture should contain a typed visual");
        };
        visual.edges[0].edge_id = visual.nodes[0].node_id.clone();
        let report =
            validate_chapter_revision(&storage, &manifest, Some(&index), &duplicate_namespace_id);
        assert!(
            report.issues.iter().any(|issue| {
                issue.code == ChapterIssueCode::DuplicateId
                    && issue.visual_item_id.as_deref() == Some("node:answer")
            }),
            "{report:#?}"
        );

        let mut bad_destination = base;
        let mut dangling_node_hotspot = bad_destination.clone();
        let visual = dangling_node_hotspot.sections[0]
            .visual
            .as_mut()
            .and_then(ChapterVisualPayload::typed_visual_mut)
            .expect("fixture should contain a typed visual");
        visual.nodes[0].hotspot_id = Some("hotspot:missing".to_owned());
        let report =
            validate_chapter_revision(&storage, &manifest, Some(&index), &dangling_node_hotspot);
        assert!(
            report.issues.iter().any(|issue| {
                issue.code == ChapterIssueCode::InvalidDestination
                    && issue.visual_item_id.as_deref() == Some("node:answer")
                    && issue.hotspot_id.as_deref() == Some("hotspot:missing")
            }),
            "{report:#?}"
        );

        bad_destination.sections[0].hotspots[0].destination = HotspotDestination::Section {
            section_id: "section:missing".to_owned(),
        };
        let report = validate_chapter_revision(&storage, &manifest, Some(&index), &bad_destination);
        assert!(
            report.issues.iter().any(|issue| {
                issue.code == ChapterIssueCode::InvalidDestination
                    && issue.hotspot_id.as_deref() == Some("hotspot:next-section")
            }),
            "{report:#?}"
        );
    }

    #[test]
    fn chapter_revision_json_enforces_its_four_megabyte_boundary() {
        let temporary = TestDirectory::new();
        let (_root, storage, project, manifest, index) = fixture(&temporary);
        let revision = valid_revision(&storage, &project, &manifest, &index);

        let mut at_limit = encode_chapter_revision(&revision).unwrap();
        assert!(at_limit.len() < MAX_CHAPTER_JSON_BYTES);
        at_limit.resize(MAX_CHAPTER_JSON_BYTES, b' ');
        assert_eq!(decode_chapter_revision(&at_limit).unwrap(), revision);

        let mut over_limit = at_limit;
        over_limit.push(b' ');
        assert!(
            decode_chapter_revision(&over_limit)
                .unwrap_err()
                .contains("4 MB size limit")
        );

        let mut oversized_revision = revision;
        oversized_revision.title = "x".repeat(MAX_CHAPTER_JSON_BYTES);
        assert!(
            encode_chapter_revision(&oversized_revision)
                .unwrap_err()
                .contains("4 MB size limit")
        );
    }

    #[test]
    fn chapter_decoder_rejects_duplicate_keys_and_unsupported_schema_versions() {
        let temporary = TestDirectory::new();
        let (_root, storage, project, manifest, index) = fixture(&temporary);
        let revision = valid_revision(&storage, &project, &manifest, &index);
        let encoded = String::from_utf8(encode_chapter_revision(&revision).unwrap()).unwrap();

        for duplicate in [
            encoded.replacen(
                "\"schemaVersion\":",
                "\"schemaVersion\":5,\"schemaVersion\":",
                1,
            ),
            encoded.replacen(
                "\"targetId\":",
                "\"targetId\":\"target:shadow\",\"targetId\":",
                1,
            ),
            encoded.replacen(
                "\"sectionId\":",
                "\"sectionId\":\"section:shadow\",\"sectionId\":",
                1,
            ),
        ] {
            let error = decode_chapter_revision(duplicate.as_bytes()).unwrap_err();
            assert!(error.contains("duplicate JSON object key"), "{error}");
        }

        let current = serde_json::from_slice::<serde_json::Value>(
            &encode_chapter_revision(&revision).unwrap(),
        )
        .unwrap();
        for unsupported in [1, 2, 3, 4, 6] {
            let mut value = current.clone();
            value["schemaVersion"] = serde_json::json!(unsupported);
            let bytes = serde_json::to_vec(&value).unwrap();
            assert!(
                decode_chapter_revision(&bytes).is_err(),
                "schema {unsupported}"
            );
            let mut revision = valid_revision(&storage, &project, &manifest, &index);
            revision.schema_version = unsupported;
            assert!(
                encode_chapter_revision(&revision).is_err(),
                "schema {unsupported}"
            );
        }
    }

    #[test]
    fn current_accepted_and_rejected_validation_requires_a_valid_parent_id() {
        let temporary = TestDirectory::new();
        let (_root, storage, project, manifest, index) = fixture(&temporary);
        let base = valid_revision(&storage, &project, &manifest, &index);

        for lifecycle in [RevisionLifecycle::Accepted, RevisionLifecycle::Rejected] {
            let mut revision = base.clone();
            revision.lifecycle = lifecycle.clone();
            revision.validation_status = if lifecycle == RevisionLifecycle::Accepted {
                ValidationStatus::Valid
            } else {
                ValidationStatus::Unvalidated
            };
            if lifecycle == RevisionLifecycle::Rejected {
                revision.decision_note = Some("Needs a clearer explanation.".to_owned());
            }

            revision.based_on_revision_id = None;
            let report = validate_chapter_revision(&storage, &manifest, Some(&index), &revision);
            assert_eq!(report.status, ValidationReportStatus::Invalid);
            assert!(issue_codes(&report).contains(&ChapterIssueCode::InvalidRevision));

            revision.based_on_revision_id = Some("not a valid id".to_owned());
            let report = validate_chapter_revision(&storage, &manifest, Some(&index), &revision);
            assert_eq!(report.status, ValidationReportStatus::Invalid);
            assert!(issue_codes(&report).contains(&ChapterIssueCode::InvalidId));

            revision.based_on_revision_id = Some("revision:prior-draft".to_owned());
            let report = validate_chapter_revision(&storage, &manifest, Some(&index), &revision);
            assert_eq!(report.status, ValidationReportStatus::Valid, "{report:#?}");
        }
    }

    #[test]
    fn invalid_revision_reports_snapshot_path_range_hash_excerpt_claim_and_destination_codes() {
        let temporary = TestDirectory::new();
        let (_root, storage, project, manifest, index) = fixture(&temporary);
        let mut revision = valid_revision(&storage, &project, &manifest, &index);
        revision.snapshot_id = "different-snapshot".to_owned();
        revision.target.scope = ChapterTargetScope::Group {
            paths: vec!["src/lib.rs".to_owned(), "missing.rs".to_owned()],
        };
        let section = &mut revision.sections[0];
        let valid_anchor = section.claims[0].anchors[0].clone();

        let mut missing_path = valid_anchor.clone();
        missing_path.anchor_id = "anchor:missing-path".to_owned();
        missing_path.path = "missing.rs".to_owned();
        let mut wrong_range = valid_anchor.clone();
        wrong_range.anchor_id = "anchor:wrong-range".to_owned();
        wrong_range.start_byte = u64::MAX - 2;
        wrong_range.end_byte_inclusive = u64::MAX;
        let mut wrong_hash = valid_anchor.clone();
        wrong_hash.anchor_id = "anchor:wrong-hash".to_owned();
        wrong_hash.source_sha256 = "0".repeat(64);
        let mut wrong_excerpt = valid_anchor.clone();
        wrong_excerpt.anchor_id = "anchor:wrong-excerpt".to_owned();
        wrong_excerpt.excerpt = "different captured bytes".to_owned();
        let mut wrong_anchor_snapshot = valid_anchor.clone();
        wrong_anchor_snapshot.anchor_id = "anchor:wrong-snapshot".to_owned();
        wrong_anchor_snapshot.snapshot_id = "old-snapshot".to_owned();
        section.claims[0].anchors.extend([
            missing_path,
            wrong_range,
            wrong_hash,
            wrong_excerpt,
            wrong_anchor_snapshot,
        ]);

        section.claims.push(ChapterClaim {
            claim_id: "claim:missing-code-anchor".to_owned(),
            category: ClaimCategory::CodeBehavior,
            prose: "An unanchored behavior statement.".to_owned(),
            anchors: Vec::new(),
            dependencies: Vec::new(),
        });
        section.claims.push(ChapterClaim {
            claim_id: "claim:unlabeled-inference".to_owned(),
            category: ClaimCategory::Inference {
                visible_label: " ".to_owned(),
                basis: String::new(),
            },
            prose: "An inference without a label or basis.".to_owned(),
            anchors: Vec::new(),
            dependencies: Vec::new(),
        });
        section.hotspots[2].destination = HotspotDestination::AtlasFile {
            path: "not-captured.rs".to_owned(),
        };

        let report = validate_chapter_revision(&storage, &manifest, Some(&index), &revision);
        assert_eq!(report.status, ValidationReportStatus::Invalid);
        let codes = issue_codes(&report);
        for expected in [
            ChapterIssueCode::WrongSnapshot,
            ChapterIssueCode::MissingPath,
            ChapterIssueCode::RangeMismatch,
            ChapterIssueCode::HashMismatch,
            ChapterIssueCode::ExcerptMismatch,
            ChapterIssueCode::InvalidClaim,
            ChapterIssueCode::InvalidDestination,
        ] {
            assert!(
                codes.contains(&expected),
                "missing {expected:?} in {report:#?}"
            );
        }
    }

    #[test]
    fn parser_line_shift_remaps_only_the_same_symbol_and_one_exact_old_excerpt() {
        let temporary = TestDirectory::new();
        let (root, storage, project, old_manifest, old_index) = fixture(&temporary);
        let old_source =
            snapshot::read_snapshot_source_from_manifest(&storage, &old_manifest, "src/lib.rs")
                .unwrap();
        let old_symbol = old_index
            .symbols
            .iter()
            .find(|symbol| symbol.name == "answer")
            .unwrap();
        let old_anchor = anchor_for(
            &old_source,
            Some(old_symbol.symbol_id.clone()),
            "compute()",
            "anchor:answer-call",
        );

        fs::write(
            root.join("src/lib.rs"),
            format!("// line inserted before the declaration\n\n{FIXTURE_SOURCE}"),
        )
        .unwrap();
        commit(&root, "shift chapter source line");
        let progress =
            snapshot::new_capture_progress(&storage, &project, CaptureLimits::default()).unwrap();
        let new_manifest = snapshot::capture_snapshot_with_progress(
            &storage,
            &project,
            progress,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        let new_index = index::build_snapshot_index(
            &storage,
            &new_manifest,
            "chapter-schema-new-index",
            &AtomicBool::new(false),
            |_| {},
        );
        index::save_snapshot_index(&storage, &new_index).unwrap();

        let result = remap_anchor_between_snapshots(
            &storage,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &old_anchor,
        );
        assert_eq!(result.status, AnchorRemapStatus::Remapped, "{result:#?}");
        let remapped = result.anchor.unwrap();
        let new_symbol = new_index
            .symbols
            .iter()
            .find(|symbol| symbol.name == "answer")
            .unwrap();
        assert_eq!(old_symbol.symbol_id, new_symbol.symbol_id);
        assert_eq!(remapped.anchor_id, old_anchor.anchor_id);
        assert_eq!(remapped.symbol_id, old_anchor.symbol_id);
        assert_eq!(remapped.snapshot_id, new_manifest.summary.snapshot_id);
        assert_eq!(remapped.source_sha256, new_symbol.anchor.source_sha256);
        assert_eq!(remapped.start_line, old_anchor.start_line + 2);
        assert_eq!(remapped.excerpt, old_anchor.excerpt);

        let mut changed_excerpt = old_anchor.clone();
        changed_excerpt.excerpt = "compute".to_owned();
        let changed_excerpt_result = remap_anchor_between_snapshots(
            &storage,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &changed_excerpt,
        );
        assert_eq!(
            changed_excerpt_result.status,
            AnchorRemapStatus::Unavailable
        );
        assert_eq!(
            changed_excerpt_result.issues[0].code,
            ChapterIssueCode::ExcerptMismatch
        );

        fs::write(
            root.join("src/lib.rs"),
            "pub fn answer() -> i32 { let _ = compute(); compute() }\nfn compute() -> i32 { 42 }\n",
        )
        .unwrap();
        commit(&root, "make the anchor excerpt ambiguous");
        let ambiguous_progress =
            snapshot::new_capture_progress(&storage, &project, CaptureLimits::default()).unwrap();
        let ambiguous_manifest = snapshot::capture_snapshot_with_progress(
            &storage,
            &project,
            ambiguous_progress,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .unwrap();
        let ambiguous_index = index::build_snapshot_index(
            &storage,
            &ambiguous_manifest,
            "chapter-schema-ambiguous-index",
            &AtomicBool::new(false),
            |_| {},
        );
        let ambiguous = remap_anchor_between_snapshots(
            &storage,
            &old_manifest,
            Some(&old_index),
            &ambiguous_manifest,
            Some(&ambiguous_index),
            &old_anchor,
        );
        assert_eq!(ambiguous.status, AnchorRemapStatus::Unavailable);
        assert_eq!(ambiguous.issues[0].code, ChapterIssueCode::AmbiguousRemap);

        let mut no_symbol = old_anchor;
        no_symbol.symbol_id = None;
        let unavailable = remap_anchor_between_snapshots(
            &storage,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &no_symbol,
        );
        assert_eq!(unavailable.status, AnchorRemapStatus::Unavailable);
        assert_eq!(unavailable.issues[0].code, ChapterIssueCode::MissingSymbol);
    }
}
