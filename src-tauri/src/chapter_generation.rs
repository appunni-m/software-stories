use crate::chapter_freshness::{self, SectionFreshnessStatus};
use crate::chapters::{
    self, ChapterClaim, ChapterRevision, ChapterSection, ChapterSourceAnchor, ChapterVisualPayload,
    ClaimCategory, GeneratedChapterClaim, GeneratedChapterEvidence, GeneratedChapterEvidenceLimit,
    GeneratedChapterProposal, GeneratedChapterSection, HotspotDestination, RevisionLifecycle,
    SectionDependency, ValidationStatus,
};
use crate::index::{IndexedRelationship, RelationshipConfidence, SnapshotIndex};
use crate::snapshot::{SnapshotManifest, SnapshotSource};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
enum GeneratedVisualKind {
    None,
    DataFlow,
    Sequence,
    Hierarchy,
}

fn take_section_visual_proposals(
    response: &mut serde_json::Value,
) -> Result<Vec<GeneratedVisualProposal>, String> {
    let sections = response
        .get_mut("sections")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or_else(|| "The generated response has no section list.".to_owned())?;
    sections.iter_mut().map(take_visual_proposal).collect()
}

fn take_visual_proposal(section: &mut serde_json::Value) -> Result<GeneratedVisualProposal, String> {
    let object = section
        .as_object_mut()
        .ok_or_else(|| "A generated section must be a JSON object.".to_owned())?;
    let Some(value) = object.remove("visual") else {
        return Ok(no_visual_proposal());
    };
    if value.is_null() {
        return Ok(no_visual_proposal());
    }
    serde_json::from_value(value)
        .map_err(|_| "The generated visual plan did not match its strict schema.".to_owned())
}

fn no_visual_proposal() -> GeneratedVisualProposal {
    GeneratedVisualProposal {
        kind: GeneratedVisualKind::None,
        title: String::new(),
        direction: chapters::VisualLayoutDirection::Auto,
        nodes: Vec::new(),
        edges: Vec::new(),
    }
}

fn canonicalize_visual_proposal(
    proposed: &GeneratedVisualProposal,
    claims: &[ChapterClaim],
    hotspots: &mut Vec<chapters::ChapterHotspot>,
) -> Result<Option<chapters::VisualSpec>, String> {
    if proposed.kind == GeneratedVisualKind::None {
        if !proposed.title.is_empty()
            || proposed.direction != chapters::VisualLayoutDirection::Auto
            || !proposed.nodes.is_empty()
            || !proposed.edges.is_empty()
        {
            return Err(
                "A visual marked none must have no title, custom layout, nodes, or edges."
                    .to_owned(),
            );
        }
        return Ok(None);
    }
    if proposed.title.trim().is_empty()
        || proposed.title.len() > 160
        || !(2..=12).contains(&proposed.nodes.len())
        || !(1..=20).contains(&proposed.edges.len())
    {
        return Err(
            "A generated visual needs a specific title, two to twelve nodes, and one to twenty edges."
                .to_owned(),
        );
    }

    let mut nodes = Vec::with_capacity(proposed.nodes.len());
    for node in &proposed.nodes {
        if node.label.trim().is_empty()
            || node.label.len() > 240
            || node.detail.len() > 512
        {
            return Err("A generated visual node has an empty label or oversized text.".to_owned());
        }
        let evidence_anchor_ids = resolve_visual_evidence(&node.evidence, claims)?;
        let node_id = generated_id("visualnode");
        let hotspot_id = generated_id("visualhotspot");
        hotspots.push(chapters::ChapterHotspot {
            hotspot_id: hotspot_id.clone(),
            label: "Open supporting code".to_owned(),
            destination: HotspotDestination::CodeLens {
                anchor_id: evidence_anchor_ids[0].clone(),
            },
        });
        nodes.push(chapters::VisualNode {
            node_id,
            label: node.label.trim().to_owned(),
            detail: (!node.detail.trim().is_empty()).then(|| node.detail.trim().to_owned()),
            hotspot_id: Some(hotspot_id),
            evidence_anchor_ids,
        });
    }

    let mut edges = Vec::with_capacity(proposed.edges.len());
    for edge in &proposed.edges {
        let from = usize::try_from(edge.from_node_index)
            .map_err(|_| "A visual edge source index is invalid.".to_owned())?;
        let to = usize::try_from(edge.to_node_index)
            .map_err(|_| "A visual edge destination index is invalid.".to_owned())?;
        if from >= nodes.len() || to >= nodes.len() || from == to {
            return Err("A visual edge must connect two distinct nodes in this visual.".to_owned());
        }
        if edge.label.trim().is_empty() || edge.label.len() > 240 {
            return Err("A generated visual edge has an empty label or oversized text.".to_owned());
        }
        let evidence_anchor_ids = resolve_visual_evidence(&edge.evidence, claims)?;
        edges.push(chapters::VisualEdge {
            edge_id: generated_id("visualedge"),
            from_node_id: nodes[from].node_id.clone(),
            to_node_id: nodes[to].node_id.clone(),
            label: edge.label.trim().to_owned(),
            evidence_anchor_ids,
        });
    }

    let kind = match proposed.kind {
        GeneratedVisualKind::None => unreachable!("none visuals returned before conversion"),
        GeneratedVisualKind::DataFlow => chapters::VisualKind::DataFlow,
        GeneratedVisualKind::Sequence => chapters::VisualKind::Sequence,
        GeneratedVisualKind::Hierarchy => chapters::VisualKind::Hierarchy,
    };
    Ok(Some(chapters::VisualSpec {
        schema_version: chapters::VISUAL_SCHEMA_VERSION,
        kind,
        title: proposed.title.trim().to_owned(),
        layout_hint: Some(chapters::VisualLayoutHint {
            direction: proposed.direction.clone(),
        }),
        nodes,
        edges,
    }))
}

fn resolve_visual_evidence(
    evidence: &[GeneratedChapterEvidence],
    claims: &[ChapterClaim],
) -> Result<Vec<String>, String> {
    if evidence.is_empty() || evidence.len() > 8 {
        return Err("Every visual node and edge needs one to eight cited claim quotes.".to_owned());
    }
    let mut seen_quotes = HashSet::new();
    let mut anchor_ids = Vec::with_capacity(evidence.len());
    for item in evidence {
        if !seen_quotes.insert((item.path.as_str(), item.quote.as_str())) {
            return Err("A visual item repeats the same source quote.".to_owned());
        }
        let matches = claims
            .iter()
            .flat_map(|claim| claim.anchors.iter())
            .filter(|anchor| anchor.path == item.path && anchor.excerpt == item.quote)
            .collect::<Vec<_>>();
        let Some(first) = matches.first() else {
            return Err(
                "A visual node or edge cites a quote that is not attached to a validated claim."
                    .to_owned(),
            );
        };
        if matches.iter().skip(1).any(|anchor| {
            anchor.source_sha256 != first.source_sha256
                || anchor.start_byte != first.start_byte
                || anchor.end_byte_inclusive != first.end_byte_inclusive
        }) {
            return Err("A visual quote maps to more than one source range.".to_owned());
        }
        if !anchor_ids.contains(&first.anchor_id) {
            anchor_ids.push(first.anchor_id.clone());
        }
    }
    if anchor_ids.is_empty() {
        return Err("A visual item has no distinct source evidence.".to_owned());
    }
    Ok(anchor_ids)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneratedVisualProposal {
    kind: GeneratedVisualKind,
    title: String,
    direction: chapters::VisualLayoutDirection,
    nodes: Vec<GeneratedVisualNode>,
    edges: Vec<GeneratedVisualEdge>,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneratedVisualNode {
    label: String,
    detail: String,
    evidence: Vec<GeneratedChapterEvidence>,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneratedVisualEdge {
    from_node_index: u32,
    to_node_index: u32,
    label: String,
    evidence: Vec<GeneratedChapterEvidence>,
}

pub fn canonicalize_section_update(
    storage_dir: &std::path::Path,
    base: &ChapterRevision,
    current_manifest: &SnapshotManifest,
    current_index: Option<&SnapshotIndex>,
    current_sources: &[SnapshotSource],
    selected_section_id: &str,
    selected_section_revision_id: &str,
    allowed_relationship_ids: &HashSet<String>,
    response_text: &str,
) -> Result<(ChapterRevision, Vec<String>), String> {
    if response_text.len() > chapters::MAX_GENERATED_RESPONSE_BYTES {
        return Err(
            "The generated section exceeds the 300 KB structured-draft limit.".to_owned(),
        );
    }
    if base.lifecycle != RevisionLifecycle::Accepted
        || base.project_id != current_manifest.summary.project_id
        || base.snapshot_id == current_manifest.summary.snapshot_id
    {
        return Err("Section updates require an accepted source chapter and a distinct current snapshot from the same project.".to_owned());
    }
    let (old_manifest, old_index) = (
        crate::snapshot::load_snapshot_manifest(storage_dir, &base.project_id, &base.snapshot_id)?,
        crate::index::load_snapshot_index(storage_dir, &base.project_id, &base.snapshot_id)
            .ok()
            .flatten(),
    );
    let freshness = chapter_freshness::classify_chapter_freshness(
        storage_dir,
        &old_manifest,
        old_index.as_ref(),
        current_manifest,
        current_index,
        base,
    )?;
    let selected_freshness = freshness
        .sections
        .iter()
        .find(|item| item.section_id == selected_section_id)
        .ok_or_else(|| "The selected section is absent from the freshness report.".to_owned())?;
    if selected_freshness.section_revision_id != selected_section_revision_id
        || selected_freshness.status != SectionFreshnessStatus::Stale
    {
        return Err("The selected section changed or is no longer proven stale. Reassess freshness and review a new request.".to_owned());
    }
    let blocked_sections = freshness
        .sections
        .iter()
        .filter(|item| {
            item.section_id != selected_section_id
                && !matches!(
                    item.status,
                    SectionFreshnessStatus::Current | SectionFreshnessStatus::SafeToCarry
                )
        })
        .map(|item| item.section_id.as_str())
        .collect::<Vec<_>>();
    if !blocked_sections.is_empty() {
        return Err(format!(
            "Other sections also need repair or review: {}. They were not changed. Update each explicitly before creating a current-snapshot chapter draft.",
            blocked_sections.join(", ")
        ));
    }
    let target_path = match &base.target.scope {
        chapters::ChapterTargetScope::File { path } => path.as_str(),
        _ => {
            return Err(
                "Selective regeneration currently supports a single-file chapter.".to_owned(),
            );
        }
    };
    let old_section = base
        .sections
        .iter()
        .find(|section| section.section_id == selected_section_id)
        .ok_or_else(|| "The selected section no longer exists.".to_owned())?;
    if old_section.section_revision_id != selected_section_revision_id {
        return Err(
            "The selected section revision changed before its response was prepared.".to_owned(),
        );
    }
    let sources = checked_sources(current_manifest, current_sources)?;
    if !sources.contains_key(target_path) {
        return Err(
            "The file chapter's target is missing from the selected section's current evidence."
                .to_owned(),
        );
    }
    let selected_paths = sources.keys().cloned().collect::<HashSet<_>>();
    let mut generated_value = serde_json::from_str::<serde_json::Value>(response_text)
        .map_err(|_| "The provider response was not valid JSON. No draft was created.".to_owned())?;
    let visual_proposal = take_visual_proposal(&mut generated_value)?;
    let proposed_section = serde_json::from_value::<GeneratedChapterSection>(generated_value)
        .map_err(|_| "The provider response did not match the strict single-section schema. No draft was created.".to_owned())?;
    let mut section = canonicalize_generated_section(
        &proposed_section,
        &visual_proposal,
        selected_section_id,
        target_path,
        &selected_paths,
        &sources,
        current_index,
        allowed_relationship_ids,
    )?;
    section.section_revision_id = generated_id("sectionrev");
    let mut update_notes = Vec::new();
    reuse_matching_anchor_ids(old_section, &mut section, &mut update_notes);

    let mut sections = Vec::with_capacity(base.sections.len());
    for (order, previous) in base.sections.iter().enumerate() {
        if previous.section_id == selected_section_id {
            let mut updated = section.clone();
            updated.order = order as u32;
            sections.push(updated);
        } else {
            let mut remapped = remap_carried_section(
                storage_dir,
                &old_manifest,
                old_index.as_ref(),
                current_manifest,
                current_index,
                previous,
            )?;
            remapped.order = order as u32;
            sections.push(remapped);
        }
    }
    let mut revision = base.clone();
    revision.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
    // A new generation has its own native dispatch record. It must not claim
    // the prior agent/request/plan origin inherited from the accepted base.
    revision.generation_origin = None;
    revision.revision_id = generated_id("revision");
    revision.revision_number = base
        .revision_number
        .checked_add(1)
        .ok_or_else(|| "The chapter revision number is exhausted.".to_owned())?;
    revision.snapshot_id = current_manifest.summary.snapshot_id.clone();
    revision.lifecycle = RevisionLifecycle::Draft;
    revision.validation_status = ValidationStatus::Unvalidated;
    revision.created_at_unix_ms = unix_time_ms();
    revision.based_on_revision_id = Some(base.revision_id.clone());
    revision.decision_note = None;
    revision.sections = sections;
    Ok((revision, update_notes))
}

fn canonicalize_generated_section(
    proposed: &GeneratedChapterSection,
    visual_proposal: &GeneratedVisualProposal,
    section_id: &str,
    target_path: &str,
    selected_paths: &HashSet<String>,
    sources: &HashMap<String, &SnapshotSource>,
    index: Option<&SnapshotIndex>,
    allowed_relationship_ids: &HashSet<String>,
) -> Result<ChapterSection, String> {
    if proposed.title.trim().is_empty()
        || proposed.title.len() > 120
        || !(20..=4_000).contains(&proposed.prose.trim().len())
        || proposed.spoken_text.trim().is_empty()
        || proposed.spoken_text.len() > 1_000
        || proposed.spoken_text.contains("```")
        || proposed.claims.is_empty()
        || proposed.claims.len() > 20
    {
        return Err("The regenerated section needs a specific title, substantive prose, non-empty bounded narration, and one to twenty claims.".to_owned());
    }
    let mut claims = Vec::with_capacity(proposed.claims.len());
    let mut dependencies = Vec::new();
    let mut target_anchored = false;
    for proposed_claim in &proposed.claims {
        let (claim, claim_dependencies, anchored_target) = build_claim(
            proposed_claim,
            section_id,
            Some(target_path),
            selected_paths,
            sources,
            index,
            allowed_relationship_ids,
        )?;
        target_anchored |= anchored_target;
        for dependency in claim_dependencies {
            push_unique(&mut dependencies, dependency);
        }
        claims.push(claim);
    }
    if !target_anchored {
        return Err(
            "The regenerated section must cite exact evidence from its target file.".to_owned(),
        );
    }
    let mut hotspots = Vec::new();
    let visual = canonicalize_visual_proposal(visual_proposal, &claims, &mut hotspots)?;
    Ok(ChapterSection {
        section_id: section_id.to_owned(),
        section_revision_id: generated_id("sectionrev"),
        order: 0,
        title: proposed.title.trim().to_owned(),
        prose: proposed.prose.trim().to_owned(),
        spoken_text: proposed.spoken_text.trim().to_owned(),
        claims,
        dependencies,
        visual: visual.map(Into::into),
        hotspots,
    })
}

fn remap_carried_section(
    storage_dir: &std::path::Path,
    old_manifest: &SnapshotManifest,
    old_index: Option<&SnapshotIndex>,
    new_manifest: &SnapshotManifest,
    new_index: Option<&SnapshotIndex>,
    previous: &ChapterSection,
) -> Result<ChapterSection, String> {
    let mut section = previous.clone();
    for anchor in section
        .claims
        .iter_mut()
        .flat_map(|claim| &mut claim.anchors)
    {
        let new_file = new_manifest.files.iter().find(|file| file.path == anchor.path)
            .ok_or_else(|| format!("Unaffected section '{}' cites a file that is missing from the selected snapshot.", previous.title))?;
        let remap = if new_file.sha256 == anchor.source_sha256 {
            let source = crate::snapshot::read_snapshot_source_from_manifest(
                storage_dir,
                new_manifest,
                &anchor.path,
            )?;
            let start = usize::try_from(anchor.start_byte)
                .map_err(|_| "A carried citation offset is unavailable.".to_owned())?;
            let end = usize::try_from(anchor.end_byte_inclusive.saturating_add(1))
                .map_err(|_| "A carried citation offset is unavailable.".to_owned())?;
            if source.contents.as_bytes().get(start..end) != Some(anchor.excerpt.as_bytes()) {
                return Err(format!(
                    "An unchanged citation in '{}' no longer matches its captured bytes.",
                    previous.title
                ));
            }
            let mut updated = anchor.clone();
            updated.snapshot_id = new_manifest.summary.snapshot_id.clone();
            Some(updated)
        } else {
            let result = chapters::remap_anchor_between_snapshots(
                storage_dir,
                old_manifest,
                old_index,
                new_manifest,
                new_index,
                anchor,
            );
            if matches!(result.status, chapters::AnchorRemapStatus::Remapped) {
                result.anchor
            } else {
                None
            }
        };
        *anchor = remap.ok_or_else(|| format!("A citation in unaffected section '{}' can no longer be verified in the selected snapshot.", previous.title))?;
    }
    for dependency in section.dependencies.iter_mut().chain(
        section
            .claims
            .iter_mut()
            .flat_map(|claim| &mut claim.dependencies),
    ) {
        let (path, symbol_id, relationship_id) = match dependency {
            SectionDependency::SourceFile { path, .. } => (path.clone(), None, None),
            SectionDependency::Symbol {
                path, symbol_id, ..
            } => (path.clone(), Some(symbol_id.clone()), None),
            SectionDependency::Relationship {
                path,
                relationship_id,
                ..
            } => (path.clone(), None, Some(relationship_id.clone())),
        };
        let new_file = new_manifest.files.iter().find(|file| file.path == path)
            .ok_or_else(|| format!("A declared dependency in unaffected section '{}' is missing from the selected snapshot.", previous.title))?;
        if let Some(symbol_id) = symbol_id {
            let present = new_index.is_some_and(|index| {
                index.symbols.iter().any(|symbol| {
                    symbol.symbol_id == symbol_id
                        && symbol.path == path
                        && symbol.anchor.source_sha256 == new_file.sha256
                })
            });
            if !present {
                return Err(format!(
                    "A symbol dependency in unaffected section '{}' cannot be verified in the selected index.",
                    previous.title
                ));
            }
        }
        if let Some(relationship_id) = relationship_id {
            let present = new_index.is_some_and(|index| {
                index.relationships.iter().any(|relationship| {
                    relationship.relationship_id == relationship_id
                        && relationship.evidence.path == path
                        && relationship.evidence.source_sha256 == new_file.sha256
                })
            });
            if !present {
                return Err(format!(
                    "A relationship dependency in unaffected section '{}' cannot be verified in the selected index.",
                    previous.title
                ));
            }
        }
        match dependency {
            SectionDependency::SourceFile { source_sha256, .. }
            | SectionDependency::Symbol { source_sha256, .. }
            | SectionDependency::Relationship { source_sha256, .. } => {
                *source_sha256 = new_file.sha256.clone()
            }
        }
    }
    Ok(section)
}

fn reuse_matching_anchor_ids(
    previous: &ChapterSection,
    next: &mut ChapterSection,
    notes: &mut Vec<String>,
) {
    let old_anchors = previous
        .claims
        .iter()
        .flat_map(|claim| &claim.anchors)
        .collect::<Vec<_>>();
    let next_anchors = next
        .claims
        .iter_mut()
        .flat_map(|claim| &mut claim.anchors)
        .collect::<Vec<_>>();
    let mut used = HashSet::<String>::new();
    let mut mapped = HashSet::<String>::new();
    let mut anchor_id_mapping = HashMap::<String, String>::new();
    for anchor in next_anchors {
        let matches = old_anchors
            .iter()
            .filter(|old| {
                old.path == anchor.path
                    && old.excerpt == anchor.excerpt
                    && old.symbol_id == anchor.symbol_id
                    && !used.contains(&old.anchor_id)
            })
            .collect::<Vec<_>>();
        if matches.len() == 1 {
            let old = matches[0];
            used.insert(old.anchor_id.clone());
            mapped.insert(old.anchor_id.clone());
            anchor_id_mapping.insert(anchor.anchor_id.clone(), old.anchor_id.clone());
            anchor.anchor_id = old.anchor_id.clone();
        }
    }

    if let Some(visual) = next.visual.as_mut().and_then(ChapterVisualPayload::typed_visual_mut) {
        for anchor_id in visual
            .nodes
            .iter_mut()
            .flat_map(|node| &mut node.evidence_anchor_ids)
            .chain(
                visual
                    .edges
                    .iter_mut()
                    .flat_map(|edge| &mut edge.evidence_anchor_ids),
            )
        {
            if let Some(previous_id) = anchor_id_mapping.get(anchor_id) {
                *anchor_id = previous_id.clone();
            }
        }
    }
    for hotspot in &mut next.hotspots {
        if let HotspotDestination::CodeLens { anchor_id } = &mut hotspot.destination {
            if let Some(previous_id) = anchor_id_mapping.get(anchor_id) {
                *anchor_id = previous_id.clone();
            }
        }
    }

    let previous_visual_hotspot_ids = previous
        .visual
        .as_ref()
        .and_then(ChapterVisualPayload::typed_visual)
        .map(|visual| {
            visual
                .nodes
                .iter()
                .filter_map(|node| node.hotspot_id.clone())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();

    let same_visual_inputs = same_visual_inputs(previous, next);
    let mut carried_hotspots = previous
        .hotspots
        .iter()
        .filter(|hotspot| {
            if previous_visual_hotspot_ids.contains(&hotspot.hotspot_id) {
                return same_visual_inputs;
            }
            match &hotspot.destination {
                HotspotDestination::CodeLens { anchor_id } => mapped.contains(anchor_id),
                _ => true,
            }
        })
        .cloned()
        .collect::<Vec<_>>();

    if same_visual_inputs {
        next.visual = previous.visual.clone();
        next.hotspots = carried_hotspots;
    } else {
        if previous.visual.is_some() && next.visual.is_none() {
            notes.push(format!(
                "The visual for '{}' was removed because the regenerated section no longer supports it.",
                previous.title
            ));
        }
        carried_hotspots.append(&mut next.hotspots);
        next.hotspots = carried_hotspots;
    }

    if next.hotspots.len() != previous.hotspots.len()
        && previous_visual_hotspot_ids.is_empty()
    {
        notes.push(format!(
            "Code Lens hotspots for '{}' were removed because their cited ranges changed.",
            previous.title
        ));
    }
}

fn same_visual_inputs(previous: &ChapterSection, next: &ChapterSection) -> bool {
    let (Some(previous_visual), Some(next_visual)) = (
        previous
            .visual
            .as_ref()
            .and_then(ChapterVisualPayload::typed_visual),
        next.visual
            .as_ref()
            .and_then(ChapterVisualPayload::typed_visual),
    ) else {
        return false;
    };
    if previous_visual.schema_version != next_visual.schema_version
        || previous_visual.kind != next_visual.kind
        || previous_visual.title != next_visual.title
        || previous_visual.layout_hint != next_visual.layout_hint
        || previous_visual.nodes.len() != next_visual.nodes.len()
        || previous_visual.edges.len() != next_visual.edges.len()
    {
        return false;
    }

    for (previous_node, next_node) in previous_visual.nodes.iter().zip(&next_visual.nodes) {
        if previous_node.label != next_node.label
            || previous_node.detail != next_node.detail
            || previous_node.evidence_anchor_ids != next_node.evidence_anchor_ids
            || hotspot_destination(previous, previous_node.hotspot_id.as_deref())
                != hotspot_destination(next, next_node.hotspot_id.as_deref())
        {
            return false;
        }
    }
    let node_index = |visual: &chapters::VisualSpec, node_id: &str| {
        visual.nodes.iter().position(|node| node.node_id == node_id)
    };
    previous_visual
        .edges
        .iter()
        .zip(&next_visual.edges)
        .all(|(previous_edge, next_edge)| {
            previous_edge.label == next_edge.label
                && previous_edge.evidence_anchor_ids == next_edge.evidence_anchor_ids
                && node_index(previous_visual, &previous_edge.from_node_id)
                    == node_index(next_visual, &next_edge.from_node_id)
                && node_index(previous_visual, &previous_edge.to_node_id)
                    == node_index(next_visual, &next_edge.to_node_id)
        })
}

fn hotspot_destination<'a>(
    section: &'a ChapterSection,
    hotspot_id: Option<&str>,
) -> Option<&'a HotspotDestination> {
    hotspot_id.and_then(|hotspot_id| {
        section
            .hotspots
            .iter()
            .find(|hotspot| hotspot.hotspot_id == hotspot_id)
            .map(|hotspot| &hotspot.destination)
    })
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneratedOverviewProposal {
    title: String,
    sections: Vec<chapters::GeneratedChapterSection>,
}

pub fn canonicalize_file_chapter(
    project_id: &str,
    manifest: &SnapshotManifest,
    index: Option<&SnapshotIndex>,
    target_path: &str,
    selected_sources: &[SnapshotSource],
    allowed_relationship_ids: &HashSet<String>,
    response_text: &str,
) -> Result<ChapterRevision, String> {
    if response_text.len() > chapters::MAX_GENERATED_RESPONSE_BYTES {
        return Err(
            "The generated chapter exceeds the 300 KB structured-draft limit.".to_owned(),
        );
    }
    if manifest.summary.project_id != project_id || target_path.trim().is_empty() {
        return Err(
            "The generation target does not match the selected repository snapshot.".to_owned(),
        );
    }
    if let Some(index) = index {
        if index.project_id != project_id || index.snapshot_id != manifest.summary.snapshot_id {
            return Err(
                "The local symbol index belongs to a different project or snapshot.".to_owned(),
            );
        }
    }
    let mut proposal_value = serde_json::from_str::<serde_json::Value>(response_text)
        .map_err(|_| "The provider response was not valid JSON. No draft was created.".to_owned())?;
    let visual_proposals = take_section_visual_proposals(&mut proposal_value)?;
    let proposal: GeneratedChapterProposal = serde_json::from_value(proposal_value)
        .map_err(|_| "The provider response did not match the strict chapter proposal schema. No draft was created.".to_owned())?;
    if visual_proposals.len() != proposal.sections.len() {
        return Err("The visual plan count did not match the generated section count.".to_owned());
    }
    let sources = checked_sources(manifest, selected_sources)?;
    let target_source = sources.get(target_path).ok_or_else(|| {
        "The target source is not among the exact files selected for this request.".to_owned()
    })?;
    validate_proposal_shape(&proposal)?;

    let selected_paths = sources.keys().cloned().collect::<HashSet<_>>();
    let mut has_responsibility = false;
    let mut sections = Vec::with_capacity(proposal.sections.len() + 1);
    for (order, (proposed_section, visual_proposal)) in proposal
        .sections
        .iter()
        .zip(&visual_proposals)
        .enumerate()
    {
        let section_id = generated_id("section");
        let mut claims = Vec::with_capacity(proposed_section.claims.len());
        let mut section_dependencies = Vec::new();
        let mut anchors_target = false;
        for proposed_claim in &proposed_section.claims {
            let (claim, dependencies, target_anchored) = build_claim(
                proposed_claim,
                &section_id,
                Some(target_path),
                &selected_paths,
                &sources,
                index,
                allowed_relationship_ids,
            )?;
            if proposed_claim.aspect == "responsibility" {
                if !target_anchored {
                    return Err("The selected file's responsibility must cite exact evidence from the target file.".to_owned());
                }
                has_responsibility = true;
            }
            anchors_target |= target_anchored;
            for dependency in dependencies {
                push_unique(&mut section_dependencies, dependency);
            }
            claims.push(claim);
        }
        if !anchors_target {
            return Err(format!(
                "Section '{}' has no exact evidence from the selected target file.",
                proposed_section.title.trim()
            ));
        }
        let mut hotspots = Vec::new();
        let visual = canonicalize_visual_proposal(visual_proposal, &claims, &mut hotspots)?;
        let section = ChapterSection {
            section_id,
            section_revision_id: generated_id("sectionrev"),
            order: order as u32,
            title: proposed_section.title.trim().to_owned(),
            prose: proposed_section.prose.trim().to_owned(),
            spoken_text: proposed_section.spoken_text.trim().to_owned(),
            claims,
            dependencies: section_dependencies,
            visual: visual.map(Into::into),
            hotspots,
        };
        sections.push(section);
    }
    if !has_responsibility {
        return Err(
            "The chapter does not identify the selected file's actual responsibility.".to_owned(),
        );
    }

    let limits_section_id = generated_id("section");
    let mut limit_claims = Vec::with_capacity(proposal.evidence_limits.len());
    let mut limit_dependencies = Vec::new();
    let mut limits_prose = Vec::new();
    let mut limits_target_anchored = false;
    for limit in &proposal.evidence_limits {
        let (claim, dependencies, target_anchored) = build_evidence_limit(
            limit,
            &limits_section_id,
            target_path,
            &selected_paths,
            &sources,
            index,
        )?;
        if !target_anchored {
            return Err(
                "Each evidence limit must cite exact evidence from the selected target file."
                    .to_owned(),
            );
        }
        limits_prose.push(limit.prose.trim().to_owned());
        limits_target_anchored |= target_anchored;
        for dependency in dependencies {
            push_unique(&mut limit_dependencies, dependency);
        }
        limit_claims.push(claim);
    }
    if !limits_target_anchored {
        return Err(
            "The chapter's evidence limits are not anchored to the selected target file."
                .to_owned(),
        );
    }
    sections.push(ChapterSection {
        section_id: limits_section_id,
        section_revision_id: generated_id("sectionrev"),
        order: sections.len() as u32,
        title: "What the source does not prove".to_owned(),
        prose: limits_prose.join("\n\n"),
        spoken_text: String::new(),
        claims: limit_claims,
        dependencies: limit_dependencies,
        visual: None,
        hotspots: Vec::new(),
    });

    let path_digest = digest_hex(target_path.as_bytes());
    let revision = ChapterRevision {
        schema_version: chapters::CHAPTER_SCHEMA_VERSION,
        chapter_id: format!("chapter:file:{path_digest}"),
        revision_id: generated_id("revision"),
        revision_number: 1,
        project_id: project_id.to_owned(),
        snapshot_id: manifest.summary.snapshot_id.clone(),
        target: chapters::ChapterTarget {
            target_id: format!("target:file:{path_digest}"),
            scope: chapters::ChapterTargetScope::File {
                path: target_path.to_owned(),
            },
        },
        title: proposal.title.trim().to_owned(),
        lifecycle: chapters::RevisionLifecycle::Draft,
        validation_status: chapters::ValidationStatus::Unvalidated,
        created_at_unix_ms: unix_time_ms(),
        based_on_revision_id: None,
        decision_note: None,
        generation_origin: None,
        sections,
    };
    if target_source.snapshot_id != revision.snapshot_id {
        return Err("The selected target bytes belong to a different snapshot.".to_owned());
    }
    Ok(revision)
}

pub fn canonicalize_repository_overview(
    project_id: &str,
    manifest: &SnapshotManifest,
    index: Option<&SnapshotIndex>,
    selected_sources: &[SnapshotSource],
    allowed_relationship_ids: &HashSet<String>,
    response_text: &str,
) -> Result<ChapterRevision, String> {
    if response_text.len() > chapters::MAX_GENERATED_RESPONSE_BYTES {
        return Err(
            "The generated overview exceeds the 300 KB structured-draft limit.".to_owned(),
        );
    }
    if manifest.summary.project_id != project_id {
        return Err(
            "The overview request does not match the selected repository snapshot.".to_owned(),
        );
    }
    if let Some(index) = index {
        if index.project_id != project_id || index.snapshot_id != manifest.summary.snapshot_id {
            return Err(
                "The local symbol index belongs to a different project or snapshot.".to_owned(),
            );
        }
    }
    let mut proposal_value = serde_json::from_str::<serde_json::Value>(response_text)
        .map_err(|_| "The provider response was not valid JSON. No draft was created.".to_owned())?;
    let visual_proposals = take_section_visual_proposals(&mut proposal_value)?;
    let proposal: GeneratedOverviewProposal = serde_json::from_value(proposal_value)
        .map_err(|_| "The provider response did not match the strict repository overview schema. No draft was created.".to_owned())?;
    if visual_proposals.len() != proposal.sections.len() {
        return Err("The visual plan count did not match the generated section count.".to_owned());
    }
    let sources = checked_sources(manifest, selected_sources)?;
    validate_overview_proposal_shape(&proposal)?;
    let selected_paths = sources.keys().cloned().collect::<HashSet<_>>();
    let mut sections = Vec::with_capacity(proposal.sections.len());
    for (order, (proposed_section, visual_proposal)) in proposal
        .sections
        .iter()
        .zip(&visual_proposals)
        .enumerate()
    {
        let section_id = generated_id("section");
        let mut claims = Vec::with_capacity(proposed_section.claims.len());
        let mut dependencies = Vec::new();
        for proposed_claim in &proposed_section.claims {
            validate_resolved_overview_relationships(
                proposed_claim,
                index,
                &selected_paths,
                &sources,
                allowed_relationship_ids,
            )?;
            let (claim, claim_dependencies, _) = build_claim(
                proposed_claim,
                &section_id,
                None,
                &selected_paths,
                &sources,
                index,
                allowed_relationship_ids,
            )?;
            if claim.anchors.is_empty() {
                return Err(
                    "Every repository overview claim must cite exact selected source evidence."
                        .to_owned(),
                );
            }
            for dependency in claim_dependencies {
                push_unique(&mut dependencies, dependency);
            }
            claims.push(claim);
        }
        let mut hotspots = Vec::new();
        add_overview_file_hotspots(&claims, &sources, &mut hotspots)?;
        let visual = canonicalize_visual_proposal(visual_proposal, &claims, &mut hotspots)?;
        let section = ChapterSection {
            section_id,
            section_revision_id: generated_id("sectionrev"),
            order: order as u32,
            title: proposed_section.title.trim().to_owned(),
            prose: proposed_section.prose.trim().to_owned(),
            spoken_text: proposed_section.spoken_text.trim().to_owned(),
            claims,
            dependencies,
            visual: visual.map(Into::into),
            hotspots,
        };
        if !chapters::overview_section_copy_supported(&section) {
            return Err("Every repository overview prose and narration sentence must be copied from an anchored claim; inference sentences must keep their visible label.".to_owned());
        }
        sections.push(section);
    }
    Ok(ChapterRevision {
        schema_version: chapters::CHAPTER_SCHEMA_VERSION,
        chapter_id: "chapter:overview".to_owned(),
        revision_id: generated_id("revision"),
        revision_number: 1,
        project_id: project_id.to_owned(),
        snapshot_id: manifest.summary.snapshot_id.clone(),
        target: chapters::ChapterTarget {
            target_id: "target:overview".to_owned(),
            scope: chapters::ChapterTargetScope::Overview,
        },
        title: proposal.title.trim().to_owned(),
        lifecycle: chapters::RevisionLifecycle::Draft,
        validation_status: chapters::ValidationStatus::Unvalidated,
        created_at_unix_ms: unix_time_ms(),
        based_on_revision_id: None,
        decision_note: None,
        generation_origin: None,
        sections,
    })
}

fn add_overview_file_hotspots(
    claims: &[ChapterClaim],
    sources: &HashMap<String, &SnapshotSource>,
    hotspots: &mut Vec<chapters::ChapterHotspot>,
) -> Result<(), String> {
    let mut linked_paths = HashSet::new();
    for path in claims
        .iter()
        .flat_map(|claim| &claim.anchors)
        .map(|anchor| anchor.path.as_str())
    {
        if !linked_paths.insert(path) {
            continue;
        }
        if !sources.contains_key(path) {
            return Err("An overview file link points outside its selected source set.".to_owned());
        }
        hotspots.push(chapters::ChapterHotspot {
            hotspot_id: generated_id("overviewfile"),
            label: overview_file_hotspot_label(path),
            destination: HotspotDestination::AtlasFile {
                path: path.to_owned(),
            },
        });
    }
    Ok(())
}

fn overview_file_hotspot_label(path: &str) -> String {
    let full_label = format!("Open file: {path}");
    if full_label.len() <= 240 {
        return full_label;
    }

    let prefix = "Open file: …/";
    let suffix = format!(" · {}", &digest_hex(path.as_bytes())[..8]);
    let basename = path.rsplit('/').next().unwrap_or(path);
    let max_basename_bytes = 240usize.saturating_sub(prefix.len() + suffix.len());
    let mut reversed_basename = String::new();
    for character in basename.chars().rev() {
        if reversed_basename.len() + character.len_utf8() > max_basename_bytes {
            break;
        }
        reversed_basename.push(character);
    }
    let basename_tail = reversed_basename.chars().rev().collect::<String>();
    format!("{prefix}{basename_tail}{suffix}")
}

fn validate_overview_proposal_shape(proposal: &GeneratedOverviewProposal) -> Result<(), String> {
    let title = proposal.title.trim();
    if !(8..=120).contains(&title.chars().count())
        || ["overview", "repository overview", "codebase overview"]
            .iter()
            .any(|generic| title.eq_ignore_ascii_case(generic))
    {
        return Err(
            "The repository overview title must identify a specific architectural subject."
                .to_owned(),
        );
    }
    if proposal.sections.is_empty() || proposal.sections.len() > 12 {
        return Err(
            "A repository overview must contain between one and twelve focused sections."
                .to_owned(),
        );
    }
    for section in &proposal.sections {
        if section.title.trim().is_empty()
            || section.title.len() > 120
            || !(20..=4_000).contains(&section.prose.trim().len())
            || section.spoken_text.trim().is_empty()
            || section.spoken_text.len() > 1_000
            || section.spoken_text.contains("```")
            || section.claims.is_empty()
            || section.claims.len() > 20
        {
            return Err("Each overview section needs a specific title, substantive prose, non-empty bounded narration, and one to twenty claims.".to_owned());
        }
    }
    Ok(())
}

fn validate_resolved_overview_relationships(
    claim: &GeneratedChapterClaim,
    index: Option<&SnapshotIndex>,
    selected_paths: &HashSet<String>,
    sources: &HashMap<String, &SnapshotSource>,
    allowed_relationship_ids: &HashSet<String>,
) -> Result<(), String> {
    if claim.relationship_ids.is_empty() {
        return Ok(());
    }
    let index = index.ok_or_else(|| {
        "This overview relationship claim has no saved local index to verify against.".to_owned()
    })?;
    for relationship_id in &claim.relationship_ids {
        if !allowed_relationship_ids.contains(relationship_id) {
            return Err(
                "The overview cites a relationship that was not included in the reviewed request."
                    .to_owned(),
            );
        }
        let relationship = index
            .relationships
            .iter()
            .find(|relationship| relationship.relationship_id == *relationship_id)
            .ok_or_else(|| {
                "A cited overview relationship is absent from the local index.".to_owned()
            })?;
        if relationship.confidence != RelationshipConfidence::High
            || relationship.target_id.is_none()
            || relationship.target_path.is_none()
            || relationship.unresolved_reason.is_some()
        {
            return Err(
                "A repository overview may cite only locally resolved relationships.".to_owned(),
            );
        }
        validate_selected_relationship(relationship, selected_paths, sources, index)?;
    }
    Ok(())
}

fn checked_sources<'a>(
    manifest: &SnapshotManifest,
    sources: &'a [SnapshotSource],
) -> Result<HashMap<String, &'a SnapshotSource>, String> {
    if sources.is_empty() || sources.len() > 12 {
        return Err("A chapter requires one to twelve explicitly selected text files from the same immutable snapshot.".to_owned());
    }
    let mut by_path = HashMap::new();
    for source in sources {
        if source.snapshot_id != manifest.summary.snapshot_id {
            return Err("A selected source file belongs to a different snapshot.".to_owned());
        }
        let Some(file) = manifest.files.iter().find(|file| file.path == source.path) else {
            return Err("A selected source is not a captured file in this snapshot.".to_owned());
        };
        if file.sha256 != source.sha256
            || file.size_bytes != source.size_bytes
            || source.contents.len() as u64 != source.size_bytes
            || digest_hex(source.contents.as_bytes()) != source.sha256
        {
            return Err(
                "A selected source's bytes or hash do not match the immutable manifest.".to_owned(),
            );
        }
        if by_path.insert(source.path.clone(), source).is_some() {
            return Err("A selected source path is duplicated.".to_owned());
        }
    }
    Ok(by_path)
}

fn validate_proposal_shape(proposal: &GeneratedChapterProposal) -> Result<(), String> {
    let title = proposal.title.trim();
    if !(8..=120).contains(&title.chars().count())
        || [
            "overview",
            "file explanation",
            "code overview",
            "understanding this file",
        ]
        .iter()
        .any(|generic| title.eq_ignore_ascii_case(generic))
    {
        return Err(
            "The chapter title is missing or too generic to identify this file's subject."
                .to_owned(),
        );
    }
    if proposal.sections.is_empty() || proposal.sections.len() > 12 {
        return Err(
            "A generated chapter must contain between one and twelve focused sections.".to_owned(),
        );
    }
    if proposal.evidence_limits.is_empty() || proposal.evidence_limits.len() > 5 {
        return Err("A generated chapter must identify one to five limits of what the selected source proves.".to_owned());
    }
    for section in &proposal.sections {
        if section.title.trim().is_empty()
            || section.title.len() > 120
            || !(20..=4_000).contains(&section.prose.trim().len())
            || section.spoken_text.trim().is_empty()
            || section.spoken_text.len() > 1_000
            || section.spoken_text.contains("```")
        {
            return Err("A chapter section needs a specific title, substantive prose, and non-empty bounded narration text.".to_owned());
        }
        if section.claims.is_empty() || section.claims.len() > 20 {
            return Err(
                "Each chapter section needs between one and twenty source-backed claims."
                    .to_owned(),
            );
        }
    }
    for limit in &proposal.evidence_limits {
        if !(24..=800).contains(&limit.prose.trim().len())
            || !(12..=500).contains(&limit.basis.trim().len())
        {
            return Err(
                "Each evidence limit needs a specific statement and a recorded basis.".to_owned(),
            );
        }
    }
    Ok(())
}

/// Reuse exact quote/identifier/relationship checks for a planning rationale.
/// A valid rationale is source evidence, not proof of narrative comprehension.
pub(crate) fn canonicalize_repository_plan_claim(
    proposed: &GeneratedChapterClaim,
    step_id: &str,
    manifest: &SnapshotManifest,
    sources: &[SnapshotSource],
    index: Option<&SnapshotIndex>,
    allowed_relationship_ids: &HashSet<String>,
) -> Result<ChapterClaim, String> {
    let checked = checked_sources(manifest, sources)?;
    let paths = checked.keys().cloned().collect();
    build_claim(
        proposed,
        step_id,
        None,
        &paths,
        &checked,
        index,
        allowed_relationship_ids,
    )
    .map(|(claim, _, _)| claim)
}

fn build_claim(
    proposed: &GeneratedChapterClaim,
    section_id: &str,
    target_path: Option<&str>,
    selected_paths: &HashSet<String>,
    sources: &HashMap<String, &SnapshotSource>,
    index: Option<&SnapshotIndex>,
    allowed_relationship_ids: &HashSet<String>,
) -> Result<(ChapterClaim, Vec<SectionDependency>, bool), String> {
    let category = match proposed.category.as_str() {
        "codeBehavior" => ClaimCategory::CodeBehavior,
        "repositoryContext" => ClaimCategory::RepositoryContext,
        "inference" if !proposed.inference_basis.trim().is_empty() => ClaimCategory::Inference {
            visible_label: "Inference".to_owned(),
            basis: proposed.inference_basis.trim().to_owned(),
        },
        "inference" => return Err("An inference claim needs its basis so the reader can distinguish it from direct evidence.".to_owned()),
        _ => return Err("A generated claim used an unknown evidence category.".to_owned()),
    };
    validate_aspect(&proposed.aspect, !proposed.relationship_ids.is_empty())?;
    let (mut anchors, mut dependencies, target_anchored) = build_anchors(
        &proposed.prose,
        &proposed.identifiers,
        &proposed.evidence,
        section_id,
        target_path,
        selected_paths,
        sources,
        index,
    )?;
    if category == ClaimCategory::CodeBehavior && anchors.is_empty() {
        return Err("A code-behavior claim requires an exact source quote.".to_owned());
    }
    let mut prose = proposed.prose.trim().to_owned();
    add_relationship_dependencies(
        &mut prose,
        &proposed.relationship_ids,
        selected_paths,
        sources,
        index,
        &mut dependencies,
        allowed_relationship_ids,
    )?;
    let claim = ChapterClaim {
        claim_id: generated_id("claim"),
        category,
        prose,
        anchors: std::mem::take(&mut anchors),
        dependencies: dependencies.clone(),
    };
    Ok((claim, dependencies, target_anchored))
}

fn build_evidence_limit(
    proposed: &GeneratedChapterEvidenceLimit,
    _section_id: &str,
    target_path: &str,
    selected_paths: &HashSet<String>,
    sources: &HashMap<String, &SnapshotSource>,
    index: Option<&SnapshotIndex>,
) -> Result<(ChapterClaim, Vec<SectionDependency>, bool), String> {
    let (anchors, dependencies, target_anchored) = build_anchors(
        &proposed.prose,
        &proposed.identifiers,
        &proposed.evidence,
        "evidence-limits",
        Some(target_path),
        selected_paths,
        sources,
        index,
    )?;
    Ok((
        ChapterClaim {
            claim_id: generated_id("claim"),
            category: ClaimCategory::Inference {
                visible_label: "Evidence limit".to_owned(),
                basis: proposed.basis.trim().to_owned(),
            },
            prose: proposed.prose.trim().to_owned(),
            anchors,
            dependencies: dependencies.clone(),
        },
        dependencies,
        target_anchored,
    ))
}

fn validate_aspect(aspect: &str, has_relationships: bool) -> Result<(), String> {
    if !matches!(
        aspect,
        "responsibility"
            | "execution"
            | "inputOutput"
            | "stateEffect"
            | "failurePath"
            | "dependency"
            | "relationship"
    ) {
        return Err("A generated claim used an unknown explanation aspect.".to_owned());
    }
    if aspect == "relationship" && !has_relationships {
        return Err(
            "A relationship claim must cite a relationship from the saved local index.".to_owned(),
        );
    }
    Ok(())
}

fn build_anchors(
    prose: &str,
    identifiers: &[String],
    evidence: &[GeneratedChapterEvidence],
    section_id: &str,
    target_path: Option<&str>,
    selected_paths: &HashSet<String>,
    sources: &HashMap<String, &SnapshotSource>,
    index: Option<&SnapshotIndex>,
) -> Result<(Vec<ChapterSourceAnchor>, Vec<SectionDependency>, bool), String> {
    let prose = prose.trim();
    if !(24..=2_000).contains(&prose.len())
        || [
            "this file plays an important role",
            "it is important to note",
            "this file contains code",
        ]
        .iter()
        .any(|filler| prose.to_lowercase().contains(filler))
    {
        return Err(
            "A generated claim is too vague or generic to review as a code explanation.".to_owned(),
        );
    }
    if identifiers.is_empty() || identifiers.len() > 8 {
        return Err("Each generated claim must name one to eight concrete code identifiers or configuration keys.".to_owned());
    }
    if evidence.is_empty() || evidence.len() > 8 {
        return Err(
            "Each generated claim must provide one to eight exact source quotes.".to_owned(),
        );
    }
    let mut anchors = Vec::with_capacity(evidence.len());
    let mut dependencies = Vec::new();
    let mut target_anchored = false;
    let mut unique_identifiers = HashSet::new();
    for identifier in identifiers {
        let identifier = identifier.trim();
        if identifier.is_empty() || !unique_identifiers.insert(identifier) {
            return Err(
                "A generated claim must list distinct, nonempty source identifiers.".to_owned(),
            );
        }
        if !contains_identifier(prose, identifier) {
            return Err(format!(
                "Claim in section '{section_id}' does not name its declared identifier '{identifier}' in the explanation."
            ));
        }
    }
    let mut verified_identifiers = HashSet::new();
    for item in evidence {
        if !selected_paths.contains(&item.path) {
            return Err(format!(
                "Evidence path '{}' was not explicitly selected for this request.",
                item.path
            ));
        }
        let source = sources.get(&item.path).ok_or_else(|| {
            "A cited source was not loaded from the selected snapshot.".to_owned()
        })?;
        let quote = item.quote.as_str();
        if !(8..=1_000).contains(&quote.len()) || quote.trim().is_empty() {
            return Err("An evidence quote must contain 8 to 1,000 exact source bytes.".to_owned());
        }
        let Some(start) = source.contents.find(quote) else {
            return Err(format!(
                "The cited quote for '{}' is not present in the captured source.",
                item.path
            ));
        };
        // Advance by one character so overlapping occurrences are ambiguous too.
        let after_first_start = start + quote.chars().next().map_or(0, char::len_utf8);
        if source.contents[after_first_start..].contains(quote) {
            return Err(format!(
                "The cited quote for '{}' appears more than once and is ambiguous.",
                item.path
            ));
        }
        if Some(item.path.as_str()) == target_path {
            target_anchored = true;
        }
        let mut quote_has_identifier = false;
        for identifier in &unique_identifiers {
            if contains_identifier(quote, identifier) {
                quote_has_identifier = true;
                verified_identifiers.insert(*identifier);
            }
        }
        if !quote_has_identifier {
            return Err(
                "Each evidence quote must contain an identifier named in the claim's explanation."
                    .to_owned(),
            );
        }
        let end_exclusive = start + quote.len();
        let symbol = index.and_then(|index| {
            index
                .symbols
                .iter()
                .filter(|symbol| {
                    symbol.path == item.path
                        && symbol.anchor.source_sha256 == source.sha256
                        && symbol.anchor.start_byte as usize <= start
                        && symbol.anchor.end_byte >= end_exclusive
                })
                .min_by_key(|symbol| {
                    symbol
                        .anchor
                        .end_byte
                        .saturating_sub(symbol.anchor.start_byte)
                })
        });
        let (start_line, start_column) = byte_position(source.contents.as_bytes(), start);
        let (end_line, end_column) = byte_position(source.contents.as_bytes(), end_exclusive - 1);
        let anchor = ChapterSourceAnchor {
            anchor_id: generated_id("anchor"),
            path: item.path.clone(),
            symbol_id: symbol.map(|item| item.symbol_id.clone()),
            snapshot_id: source.snapshot_id.clone(),
            source_sha256: source.sha256.clone(),
            start_byte: start as u64,
            end_byte_inclusive: (end_exclusive - 1) as u64,
            start_line,
            end_line,
            start_column,
            end_column,
            excerpt: quote.to_owned(),
        };
        push_unique(
            &mut dependencies,
            SectionDependency::SourceFile {
                path: source.path.clone(),
                source_sha256: source.sha256.clone(),
            },
        );
        if let Some(symbol) = symbol {
            push_unique(
                &mut dependencies,
                SectionDependency::Symbol {
                    path: source.path.clone(),
                    source_sha256: source.sha256.clone(),
                    symbol_id: symbol.symbol_id.clone(),
                },
            );
        }
        anchors.push(anchor);
    }
    for identifier in unique_identifiers {
        if !verified_identifiers.contains(identifier) {
            return Err(format!(
                "Claim in section '{section_id}' does not connect identifier '{identifier}' to the cited code."
            ));
        }
    }
    Ok((anchors, dependencies, target_anchored))
}

fn add_relationship_dependencies(
    prose: &mut String,
    relationship_ids: &[String],
    selected_paths: &HashSet<String>,
    sources: &HashMap<String, &SnapshotSource>,
    index: Option<&SnapshotIndex>,
    dependencies: &mut Vec<SectionDependency>,
    allowed_relationship_ids: &HashSet<String>,
) -> Result<(), String> {
    if relationship_ids.len() > 8 {
        return Err("A claim may cite no more than eight indexed relationships.".to_owned());
    }
    if relationship_ids.is_empty() {
        return Ok(());
    }
    let index = index.ok_or_else(|| "This relationship claim cannot be checked because the snapshot has no saved symbol index.".to_owned())?;
    let mut unique = HashSet::new();
    for relationship_id in relationship_ids {
        if !allowed_relationship_ids.contains(relationship_id) {
            return Err("The generated chapter cites a relationship that was not included in the reviewed request.".to_owned());
        }
        if !unique.insert(relationship_id) {
            return Err("A relationship is cited twice in the same claim.".to_owned());
        }
        let relationship = index
            .relationships
            .iter()
            .find(|relationship| relationship.relationship_id == *relationship_id)
            .ok_or_else(|| {
                "A generated relationship ID is absent from the saved snapshot index.".to_owned()
            })?;
        validate_selected_relationship(relationship, selected_paths, sources, index)?;
        let kind = serde_json::to_value(&relationship.kind)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".to_owned());
        let confidence = serde_json::to_value(&relationship.confidence)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| "unknown".to_owned());
        let source_symbol = index
            .symbols
            .iter()
            .find(|symbol| symbol.symbol_id == relationship.source_id);
        let target_symbol = relationship.target_id.as_ref().and_then(|target| {
            index
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == *target)
        });
        let source_label = source_symbol
            .map(|symbol| symbol.qualified_name.as_str())
            .unwrap_or(relationship.source_id.as_str());
        let target_label = target_symbol
            .map(|symbol| symbol.qualified_name.as_str())
            .or(relationship.target_label.as_deref())
            .or(relationship.target_path.as_deref())
            .unwrap_or("unresolved target");
        prose.push_str(&format!("\n\nLocal index evidence: {kind} relationship, {confidence} confidence ({source_label} → {target_label})."));
        push_unique(
            dependencies,
            SectionDependency::Relationship {
                path: relationship.evidence.path.clone(),
                source_sha256: relationship.evidence.source_sha256.clone(),
                relationship_id: relationship.relationship_id.clone(),
            },
        );
    }
    Ok(())
}

fn validate_selected_relationship(
    relationship: &IndexedRelationship,
    selected_paths: &HashSet<String>,
    sources: &HashMap<String, &SnapshotSource>,
    index: &SnapshotIndex,
) -> Result<(), String> {
    if relationship.confidence == RelationshipConfidence::High
        && (relationship.target_id.is_none()
            || relationship.target_path.is_none()
            || relationship.unresolved_reason.is_some())
    {
        return Err("A high-confidence resolved relationship must include a target ID and path and must not carry an unresolved reason.".to_owned());
    }
    if !selected_paths.contains(&relationship.evidence.path)
        || relationship
            .target_path
            .as_ref()
            .is_some_and(|path| !selected_paths.contains(path))
    {
        return Err("The generated chapter cites a relationship whose source or target file was not explicitly selected.".to_owned());
    }
    let source_path = index
        .symbols
        .iter()
        .find(|symbol| symbol.symbol_id == relationship.source_id)
        .map(|symbol| symbol.path.as_str())
        .or_else(|| {
            index
                .files
                .iter()
                .find(|file| file.file_id == relationship.source_id)
                .map(|file| file.path.as_str())
        });
    if source_path != Some(relationship.evidence.path.as_str()) {
        return Err(
            "The indexed relationship source ID does not identify its selected evidence file."
                .to_owned(),
        );
    }
    if let Some(target_id) = &relationship.target_id {
        let target_path = index
            .symbols
            .iter()
            .find(|symbol| symbol.symbol_id == *target_id)
            .map(|symbol| symbol.path.as_str())
            .or_else(|| {
                index
                    .files
                    .iter()
                    .find(|file| file.file_id == *target_id)
                    .map(|file| file.path.as_str())
            });
        if !target_path.is_some_and(|path| selected_paths.contains(path)) {
            return Err("The generated chapter cites a resolved relationship target that was not explicitly selected.".to_owned());
        }
        if relationship.target_path.as_deref() != target_path {
            return Err(
                "The indexed relationship target ID does not match its recorded target file."
                    .to_owned(),
            );
        }
    }
    let source = sources.get(&relationship.evidence.path).ok_or_else(|| {
        "An indexed relationship's source was not loaded from the selected snapshot.".to_owned()
    })?;
    let evidence = &relationship.evidence;
    if evidence.source_sha256 != source.sha256
        || evidence.start_byte >= evidence.end_byte
        || source
            .contents
            .get(evidence.start_byte..evidence.end_byte)
            .is_none()
    {
        return Err(
            "The indexed relationship evidence does not match the selected immutable source."
                .to_owned(),
        );
    }
    Ok(())
}

fn contains_identifier(text: &str, identifier: &str) -> bool {
    let is_identifier_character =
        |character: char| character.is_alphanumeric() || matches!(character, '_' | '$');
    let starts_as_identifier = identifier
        .chars()
        .next()
        .is_some_and(is_identifier_character);
    let ends_as_identifier = identifier
        .chars()
        .last()
        .is_some_and(is_identifier_character);
    text.match_indices(identifier).any(|(start, _)| {
        let end = start + identifier.len();
        (!starts_as_identifier
            || !text[..start]
                .chars()
                .next_back()
                .is_some_and(is_identifier_character))
            && (!ends_as_identifier
                || !text[end..]
                    .chars()
                    .next()
                    .is_some_and(is_identifier_character))
    })
}

fn push_unique<T: PartialEq>(items: &mut Vec<T>, item: T) {
    if !items.contains(&item) {
        items.push(item);
    }
}

fn generated_id(prefix: &str) -> String {
    format!("{prefix}:{}", Uuid::new_v4())
}

fn digest_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |duration| duration.as_millis() as u64)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::chapters::{GeneratedChapterSection, ValidationReportStatus};
    use crate::index::{self, IndexFileStatus, RelationshipConfidence, RelationshipKind};
    use crate::project::{AccessStatus, RecentProject};
    use crate::snapshot::{self, CaptureLimits};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::AtomicBool;

    pub(crate) const SIMPLE_PATH: &str = "src/greeting.ts";
    const COMPLEX_PATH: &str = "src/user.ts";
    const SUPPORT_PATH: &str = "src/names.ts";
    const TEST_PATH: &str = "tests/user.test.ts";
    const CONFIG_PATH: &str = "config/release.toml";
    const UNSELECTED_PATH: &str = "src/unselected.ts";
    const SIMPLE_FUNCTION: &str =
        "function greeting(name: string): string {\n    return `Hello, ${name}`;\n}";
    pub(crate) const SIMPLE_SOURCE: &str = "// greeting formats UTF-8 text such as café.\nexport function greeting(name: string): string {\n    return `Hello, ${name}`;\n}\n// marker repeats.\n// marker repeats.\nconst overlapping = \"aaaaaaaaa\";\n";
    const COMPLEX_SOURCE: &str = "import { normalizeName } from './names';\nexport function parseUser(input: string): string {\n    if (!input.trim()) {\n        throw new Error('user name is empty');\n    }\n    const normalized = normalizeName(input);\n    return normalized.toUpperCase();\n}\n";
    const SUPPORT_SOURCE: &str =
        "export function normalizeName(value: string): string { return value.trim(); }\n";
    const TEST_SOURCE: &str = "import assert from 'node:assert/strict';\nimport { test } from 'node:test';\nimport { parseUser } from '../src/user';\ntest('rejects empty input', () => {\n    assert.throws(() => parseUser(''), /user name is empty/);\n});\n";
    const CONFIG_SOURCE: &str =
        "[release]\nchannel = \"stable\"\nretry_limit = 3\nartifact_name = \"atlas-desktop\"\n";

    #[test]
    fn recorded_luna_chapter_validates_without_an_index_and_rejects_invented_quotes() {
        let mut fixture = Fixture::new();
        fixture.recapture_with_source(
            "cart.ts",
            include_str!("../../benchmarks/codex-account-smoke/cart.ts"),
        );
        let response =
            include_str!("../../benchmarks/codex-account-smoke/2026-10-16-native-proposal.json");
        let sources = fixture.sources(&["cart.ts"]);
        let revision = canonicalize_file_chapter(
            &fixture.manifest.summary.project_id,
            &fixture.manifest,
            None,
            "cart.ts",
            &sources,
            &HashSet::new(),
            response,
        )
        .unwrap();
        let report = chapters::validate_chapter_revision(
            &fixture.storage,
            &fixture.manifest,
            None,
            &revision,
        );
        assert_eq!(report.status, chapters::ValidationReportStatus::Valid);
        assert_eq!(revision.sections.len(), 3);
        let mut changed: serde_json::Value = serde_json::from_str(response).unwrap();
        changed["sections"][0]["claims"][0]["evidence"][0]["quote"] =
            serde_json::json!("invented source that does not occur");
        assert!(
            canonicalize_file_chapter(
                &fixture.manifest.summary.project_id,
                &fixture.manifest,
                None,
                "cart.ts",
                &sources,
                &HashSet::new(),
                &changed.to_string()
            )
            .is_err()
        );
    }
    pub(crate) struct Fixture {
        temporary: PathBuf,
        root: PathBuf,
        pub(crate) storage: PathBuf,
        pub(crate) manifest: SnapshotManifest,
        pub(crate) index: SnapshotIndex,
    }

    impl Fixture {
        pub(crate) fn new() -> Self {
            let temporary = std::env::temp_dir().join(format!(
                "code-atlas-generated-chapter-{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
            let root = temporary.join("repository");
            fs::create_dir_all(&root).unwrap();
            git(&root, &["init", "-q", "--initial-branch=main"]);
            git(&root, &["config", "user.name", "Chapter Generation Tests"]);
            git(
                &root,
                &["config", "user.email", "chapter-generation@example.invalid"],
            );
            for (path, contents) in [
                (SIMPLE_PATH, SIMPLE_SOURCE),
                (COMPLEX_PATH, COMPLEX_SOURCE),
                (SUPPORT_PATH, SUPPORT_SOURCE),
                (TEST_PATH, TEST_SOURCE),
                (CONFIG_PATH, CONFIG_SOURCE),
                (UNSELECTED_PATH, "export const secretWorker = () => 99;\n"),
            ] {
                let path = root.join(path);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, contents).unwrap();
            }
            git(&root, &["add", "--all"]);
            git(&root, &["commit", "-qm", "independent chapter fixtures"]);
            let project_path = root.canonicalize().unwrap().display().to_string();
            let project = RecentProject {
                project_id: project_path.clone(),
                display_name: "independent chapters".to_owned(),
                selected_path: project_path.clone(),
                repository_root: project_path,
                branch: Some("main".to_owned()),
                head: None,
                access_status: AccessStatus::Available,
                last_opened_unix_ms: 0,
            };
            let storage = temporary.join("app-data");
            let progress =
                snapshot::new_capture_progress(&storage, &project, CaptureLimits::default())
                    .unwrap();
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
                "generated-chapter-fixtures",
                &AtomicBool::new(false),
                |_| {},
            );
            index::save_snapshot_index(&storage, &index).unwrap();
            Self {
                temporary,
                root,
                storage,
                manifest,
                index,
            }
        }

        pub(crate) fn sources(&self, paths: &[&str]) -> Vec<SnapshotSource> {
            paths
                .iter()
                .map(|path| {
                    snapshot::read_snapshot_source_from_manifest(
                        &self.storage,
                        &self.manifest,
                        path,
                    )
                    .unwrap()
                })
                .collect()
        }

        pub(crate) fn recapture_with_source(&mut self, path: &str, contents: &str) {
            let file_path = self.root.join(path);
            fs::write(&file_path, contents).unwrap();
            let project = RecentProject {
                project_id: self.manifest.summary.project_id.clone(),
                display_name: "independent chapters".to_owned(),
                selected_path: self.manifest.selected_path.clone(),
                repository_root: self.root.canonicalize().unwrap().display().to_string(),
                branch: Some("main".to_owned()),
                head: self.manifest.summary.head.clone(),
                access_status: AccessStatus::Available,
                last_opened_unix_ms: 0,
            };
            let progress =
                snapshot::new_capture_progress(&self.storage, &project, CaptureLimits::default())
                    .unwrap();
            let manifest = snapshot::capture_snapshot_with_progress(
                &self.storage,
                &project,
                progress,
                &AtomicBool::new(false),
                &mut |_| {},
            )
            .unwrap();
            let index = index::build_snapshot_index(
                &self.storage,
                &manifest,
                "generated-chapter-fixtures",
                &AtomicBool::new(false),
                |_| {},
            );
            index::save_snapshot_index(&self.storage, &index).unwrap();
            self.manifest = manifest;
            self.index = index;
        }

        fn relationship(
            &self,
            kind: RelationshipKind,
            path: &str,
            label: &str,
        ) -> &IndexedRelationship {
            self.index
                .relationships
                .iter()
                .find(|relationship| {
                    relationship.kind == kind
                        && relationship.evidence.path == path
                        && relationship.target_label.as_deref() == Some(label)
                })
                .unwrap_or_else(|| {
                    panic!("missing indexed {kind:?} relationship for {path}:{label}")
                })
        }

        fn generate(
            &self,
            target: &str,
            paths: &[&str],
            proposal: &GeneratedChapterProposal,
        ) -> Result<ChapterRevision, String> {
            let allowed = proposal
                .sections
                .iter()
                .flat_map(|section| &section.claims)
                .flat_map(|claim| &claim.relationship_ids)
                .cloned()
                .collect();
            canonicalize_file_chapter(
                &self.manifest.summary.project_id,
                &self.manifest,
                Some(&self.index),
                target,
                &self.sources(paths),
                &allowed,
                &serde_json::to_string(proposal).unwrap(),
            )
        }

        fn assert_traceable(&self, revision: &ChapterRevision, selected_paths: &[&str]) {
            let report = chapters::validate_chapter_revision(
                &self.storage,
                &self.manifest,
                Some(&self.index),
                revision,
            );
            assert_eq!(report.status, ValidationReportStatus::Valid, "{report:#?}");
            assert!(report.issues.is_empty());
            for section in &revision.sections {
                assert!(section.visual.is_none());
                if revision.target.scope == chapters::ChapterTargetScope::Overview {
                    let cited_paths = section.claims.iter().flat_map(|claim| &claim.anchors)
                        .map(|anchor| anchor.path.as_str()).collect::<HashSet<_>>();
                    assert_eq!(section.hotspots.len(), cited_paths.len());
                    let mut linked_paths = HashSet::new();
                    for hotspot in &section.hotspots {
                        let HotspotDestination::AtlasFile { path } = &hotspot.destination else {
                            panic!("overview source link must open its captured file");
                        };
                        assert!(cited_paths.contains(path.as_str()));
                        assert!(linked_paths.insert(path));
                    }
                } else {
                    assert!(section.hotspots.is_empty());
                }
                for claim in &section.claims {
                    assert!(!claim.anchors.is_empty());
                    for anchor in &claim.anchors {
                        assert!(selected_paths.contains(&anchor.path.as_str()));
                        assert_eq!(anchor.snapshot_id, self.manifest.summary.snapshot_id);
                        let source = self.sources(&[&anchor.path]).remove(0);
                        assert_eq!(anchor.source_sha256, digest_hex(source.contents.as_bytes()));
                        let start = anchor.start_byte as usize;
                        let end = anchor.end_byte_inclusive as usize;
                        assert_eq!(&source.contents[start..=end], anchor.excerpt);
                        assert_eq!(
                            byte_position(source.contents.as_bytes(), start),
                            (anchor.start_line, anchor.start_column)
                        );
                        assert_eq!(
                            byte_position(source.contents.as_bytes(), end),
                            (anchor.end_line, anchor.end_column)
                        );
                        let dependency = SectionDependency::SourceFile {
                            path: source.path,
                            source_sha256: source.sha256,
                        };
                        assert!(claim.dependencies.contains(&dependency));
                        assert!(section.dependencies.contains(&dependency));
                        if let Some(symbol_id) = &anchor.symbol_id {
                            assert!(self.index.symbols.iter().any(|symbol| {
                                symbol.symbol_id == *symbol_id
                                    && symbol.path == anchor.path
                                    && symbol.anchor.start_byte <= start
                                    && symbol.anchor.end_byte > end
                            }));
                        }
                    }
                    for dependency in &claim.dependencies {
                        assert!(section.dependencies.contains(dependency));
                        if let SectionDependency::Relationship {
                            relationship_id,
                            path,
                            source_sha256,
                        } = dependency
                        {
                            assert!(self.index.relationships.iter().any(|relationship| {
                                relationship.relationship_id == *relationship_id
                                    && relationship.evidence.path == *path
                                    && relationship.evidence.source_sha256 == *source_sha256
                            }));
                        }
                    }
                }
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.temporary);
        }
    }

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn claim(
        aspect: &str,
        prose: &str,
        identifiers: &[&str],
        evidence: &[(&str, &str)],
    ) -> GeneratedChapterClaim {
        GeneratedChapterClaim {
            category: "codeBehavior".to_owned(),
            aspect: aspect.to_owned(),
            prose: prose.to_owned(),
            inference_basis: String::new(),
            identifiers: identifiers
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            evidence: evidence
                .iter()
                .map(|(path, quote)| GeneratedChapterEvidence {
                    path: (*path).to_owned(),
                    quote: (*quote).to_owned(),
                })
                .collect(),
            relationship_ids: Vec::new(),
        }
    }

    fn section(
        title: &str,
        prose: &str,
        claims: Vec<GeneratedChapterClaim>,
    ) -> GeneratedChapterSection {
        GeneratedChapterSection {
            title: title.to_owned(),
            prose: prose.to_owned(),
            spoken_text: prose.to_owned(),
            claims,
        }
    }

    fn limit(
        path: &str,
        quote: &str,
        identifier: &str,
        prose: &str,
    ) -> GeneratedChapterEvidenceLimit {
        GeneratedChapterEvidenceLimit {
            prose: prose.to_owned(),
            basis: "Selected static source establishes the shown declaration, without executing callers or deployment tooling.".to_owned(),
            identifiers: vec![identifier.to_owned()],
            evidence: vec![GeneratedChapterEvidence { path: path.to_owned(), quote: quote.to_owned() }],
        }
    }

    pub(crate) fn simple_proposal() -> GeneratedChapterProposal {
        let prose = "greeting accepts name and returns the name inside a Hello template string without changing other state.";
        GeneratedChapterProposal {
            title: "Formatting the greeting template".to_owned(),
            sections: vec![section(
                "Name to greeting string",
                prose,
                vec![claim(
                    "responsibility",
                    prose,
                    &["greeting", "name"],
                    &[(SIMPLE_PATH, SIMPLE_FUNCTION)],
                )],
            )],
            evidence_limits: vec![limit(
                SIMPLE_PATH,
                SIMPLE_FUNCTION,
                "greeting",
                "greeting establishes string formatting here; this source does not establish which production callers supply a name.",
            )],
        }
    }

    fn complex_proposal(fixture: &Fixture) -> GeneratedChapterProposal {
        let role = "parseUser accepts input, rejects a blank name, normalizes the text, and returns an uppercase string.";
        let execution = "normalized receives normalizeName(input), then normalized.toUpperCase() produces the returned string.";
        let failure = "input.trim() is checked before normalization; an empty result causes Error with the user name is empty message.";
        let mut call = claim(
            "relationship",
            execution,
            &["normalized", "normalizeName", "input"],
            &[
                (COMPLEX_PATH, "const normalized = normalizeName(input);"),
                (SUPPORT_PATH, SUPPORT_SOURCE),
            ],
        );
        call.relationship_ids.push(
            fixture
                .relationship(RelationshipKind::Calls, COMPLEX_PATH, "normalizeName")
                .relationship_id
                .clone(),
        );
        let mut support = claim(
            "dependency",
            "normalizeName trims value before handing the result back to its selected caller.",
            &["normalizeName", "value"],
            &[(SUPPORT_PATH, SUPPORT_SOURCE)],
        );
        support.category = "repositoryContext".to_owned();
        GeneratedChapterProposal {
            title: "Rejecting and normalizing user names".to_owned(),
            sections: vec![
                section(
                    "The user name boundary",
                    role,
                    vec![claim(
                        "responsibility",
                        role,
                        &["parseUser", "input"],
                        &[(
                            COMPLEX_PATH,
                            "export function parseUser(input: string): string {",
                        )],
                    )],
                ),
                section(
                    "Normalize before uppercase conversion",
                    execution,
                    vec![
                        call,
                        support,
                        claim(
                            "execution",
                            "normalized.toUpperCase() returns an uppercase copy of the normalization result.",
                            &["normalized", "toUpperCase"],
                            &[(COMPLEX_PATH, "return normalized.toUpperCase();")],
                        ),
                    ],
                ),
                section(
                    "Reject empty input before calling the helper",
                    failure,
                    vec![claim(
                        "failurePath",
                        failure,
                        &["input", "Error"],
                        &[(
                            COMPLEX_PATH,
                            "if (!input.trim()) {\n        throw new Error('user name is empty');\n    }",
                        )],
                    )],
                ),
            ],
            evidence_limits: vec![limit(
                COMPLEX_PATH,
                "export function parseUser(input: string): string {",
                "parseUser",
                "parseUser shows local validation and conversion; selected static evidence does not prove runtime callers catch its exception.",
            )],
        }
    }

    fn test_proposal(fixture: &Fixture) -> GeneratedChapterProposal {
        let role = "test registers the empty input case, where assert.throws expects parseUser to throw the user name is empty error.";
        let mut reference = claim(
            "relationship",
            "parseUser is referenced by the selected test assertion; input is the parameter in its selected declaration.",
            &["parseUser", "input"],
            &[
                (
                    TEST_PATH,
                    "assert.throws(() => parseUser(''), /user name is empty/);",
                ),
                (
                    COMPLEX_PATH,
                    "export function parseUser(input: string): string {",
                ),
            ],
        );
        reference.relationship_ids.push(
            fixture
                .relationship(RelationshipKind::TestReference, TEST_PATH, "parseUser")
                .relationship_id
                .clone(),
        );
        GeneratedChapterProposal {
            title: "The empty user name rejection test".to_owned(),
            sections: vec![section(
                "The exception assertion",
                role,
                vec![
                    claim(
                        "responsibility",
                        role,
                        &["test", "assert", "parseUser"],
                        &[(
                            TEST_PATH,
                            "test('rejects empty input', () => {\n    assert.throws(() => parseUser(''), /user name is empty/);\n});",
                        )],
                    ),
                    reference,
                ],
            )],
            evidence_limits: vec![limit(
                TEST_PATH,
                "assert.throws(() => parseUser(''), /user name is empty/);",
                "assert",
                "assert.throws records the expected exception, but selected static evidence does not establish that the test has run or passed.",
            )],
        }
    }

    fn config_proposal() -> GeneratedChapterProposal {
        let role = "release declares channel as stable, retry_limit as three, and artifact_name as atlas-desktop for the configuration consumer.";
        GeneratedChapterProposal {
            title: "The stable release configuration values".to_owned(),
            sections: vec![section(
                "Declared release settings",
                role,
                vec![claim(
                    "responsibility",
                    role,
                    &["release", "channel", "retry_limit", "artifact_name"],
                    &[(CONFIG_PATH, CONFIG_SOURCE)],
                )],
            )],
            evidence_limits: vec![limit(
                CONFIG_PATH,
                "channel = \"stable\"",
                "channel",
                "channel records the stable value, but this configuration alone does not prove a release consumer reads or applies the setting.",
            )],
        }
    }

    #[test]
    fn captured_simple_complex_test_and_config_files_generate_independent_traceable_chapters() {
        let fixture = Fixture::new();
        // A live checkout edit must have no effect on the captured bytes used below.
        fs::write(
            fixture.root.join(SIMPLE_PATH),
            "throw new Error('never execute repository source');\n",
        )
        .unwrap();
        let cases = [
            (SIMPLE_PATH, vec![SIMPLE_PATH], simple_proposal(), 2),
            (
                COMPLEX_PATH,
                vec![COMPLEX_PATH, SUPPORT_PATH],
                complex_proposal(&fixture),
                4,
            ),
            (
                TEST_PATH,
                vec![TEST_PATH, COMPLEX_PATH],
                test_proposal(&fixture),
                2,
            ),
            (CONFIG_PATH, vec![CONFIG_PATH], config_proposal(), 2),
        ];
        let mut chapter_ids = HashSet::new();
        for (target, paths, proposal, section_count) in cases {
            let revision = fixture.generate(target, &paths, &proposal).unwrap();
            assert_eq!(
                revision.target.scope,
                chapters::ChapterTargetScope::File {
                    path: target.to_owned()
                }
            );
            assert_eq!(
                revision.chapter_id,
                format!("chapter:file:{}", digest_hex(target.as_bytes()))
            );
            assert!(chapter_ids.insert(revision.chapter_id.clone()));
            assert_eq!(revision.sections.len(), section_count);
            assert_eq!(revision.lifecycle, chapters::RevisionLifecycle::Draft);
            assert_eq!(
                revision.validation_status,
                chapters::ValidationStatus::Unvalidated
            );
            assert_eq!(revision.sections[0].prose, proposal.sections[0].prose);
            fixture.assert_traceable(&revision, &paths);
            if target == CONFIG_PATH {
                assert!(fixture.index.files.iter().any(|file| file.path == target
                    && file.status == IndexFileStatus::UnsupportedLanguage));
                assert!(
                    revision
                        .sections
                        .iter()
                        .flat_map(|section| &section.claims)
                        .flat_map(|claim| &claim.anchors)
                        .all(|anchor| anchor.symbol_id.is_none())
                );
            }
            if target == COMPLEX_PATH {
                let prose = &revision.sections[1].claims[0].prose;
                assert!(prose.contains("calls relationship, high confidence"));
                assert!(prose.contains("normalizeName"));
            }
            if target == TEST_PATH {
                assert!(
                    revision.sections[0].claims[1]
                        .prose
                        .contains("testReference relationship, high confidence")
                );
            }
        }
        assert_eq!(chapter_ids.len(), 4);
        assert_eq!(fixture.manifest.files.len(), 6);
    }

    #[test]
    fn generated_behavior_sections_require_narration_but_synthetic_limits_may_be_silent() {
        let fixture = Fixture::new();

        let mut file_proposal = simple_proposal();
        file_proposal.sections[0].spoken_text = "  \n".to_owned();
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &file_proposal)
                .unwrap_err()
                .contains("non-empty bounded narration")
        );

        let generated = fixture
            .generate(SIMPLE_PATH, &[SIMPLE_PATH], &simple_proposal())
            .unwrap();
        assert!(generated.sections.last().unwrap().spoken_text.is_empty());

        let selected_sources = fixture.sources(&[SIMPLE_PATH]);
        let source_map = checked_sources(&fixture.manifest, &selected_sources).unwrap();
        let selected_paths = source_map.keys().cloned().collect::<HashSet<_>>();
        let mut regenerated_proposal = simple_proposal();
        let mut regenerated = regenerated_proposal.sections.remove(0);
        regenerated.spoken_text.clear();
        assert!(
            canonicalize_generated_section(
                &regenerated,
                &no_visual_proposal(),
                "section:test",
                SIMPLE_PATH,
                &selected_paths,
                &source_map,
                Some(&fixture.index),
                &HashSet::new(),
            )
            .unwrap_err()
            .contains("non-empty bounded narration")
        );

        let mut overview = simple_proposal();
        overview.sections[0].spoken_text.clear();
        let overview = GeneratedOverviewProposal {
            title: "Greeting behavior across the repository".to_owned(),
            sections: overview.sections,
        };
        assert!(
            canonicalize_repository_overview(
                &fixture.manifest.summary.project_id,
                &fixture.manifest,
                Some(&fixture.index),
                &fixture.sources(&[SIMPLE_PATH, SUPPORT_PATH]),
                &HashSet::new(),
                &serde_json::to_string(&overview).unwrap(),
            )
            .unwrap_err()
            .contains("non-empty bounded narration")
        );
    }

    #[test]
    fn repository_overview_uses_stable_identity_exact_selected_evidence_and_labeled_inference() {
        let fixture = Fixture::new();
        let mut file_proposal = simple_proposal();
        file_proposal.title = "Greeting flow across the selected repository".to_owned();
        file_proposal.sections[0].claims[0].category = "inference".to_owned();
        file_proposal.sections[0].claims[0].inference_basis =
            "This is a structural interpretation of the captured greeting implementation."
                .to_owned();
        file_proposal.sections[0].prose =
            format!("Inference: {}", file_proposal.sections[0].claims[0].prose);
        file_proposal.sections[0].spoken_text = file_proposal.sections[0].prose.clone();
        let proposal = GeneratedOverviewProposal {
            title: file_proposal.title.clone(),
            sections: file_proposal.sections,
        };
        let selected = fixture.sources(&[SIMPLE_PATH, SUPPORT_PATH]);
        let revision = canonicalize_repository_overview(
            &fixture.manifest.summary.project_id,
            &fixture.manifest,
            Some(&fixture.index),
            &selected,
            &HashSet::new(),
            &serde_json::to_string(&proposal).unwrap(),
        )
        .unwrap();
        assert_eq!(revision.chapter_id, "chapter:overview");
        assert_eq!(revision.target.target_id, "target:overview");
        assert_eq!(
            revision.target.scope,
            chapters::ChapterTargetScope::Overview
        );
        assert!(matches!(
            revision.sections[0].claims[0].category,
            ClaimCategory::Inference { ref visible_label, .. } if visible_label == "Inference"
        ));
        fixture.assert_traceable(&revision, &[SIMPLE_PATH, SUPPORT_PATH]);

        let mut unknown = serde_json::to_value(&proposal).unwrap();
        unknown["evidenceLimits"] = serde_json::json!([]);
        assert!(
            canonicalize_repository_overview(
                &fixture.manifest.summary.project_id,
                &fixture.manifest,
                Some(&fixture.index),
                &selected,
                &HashSet::new(),
                &serde_json::to_string(&unknown).unwrap(),
            )
            .unwrap_err()
            .contains("strict repository overview schema")
        );
    }

    #[test]
    fn every_declared_identifier_and_evidence_quote_must_ground_the_explanation() {
        let fixture = Fixture::new();
        for identifiers in [
            vec!["greeting", "inventedIdentifier"],
            vec!["greeting", ""],
            vec!["greeting", "greeting"],
            vec!["greet"],
        ] {
            let mut proposal = simple_proposal();
            proposal.sections[0].claims[0].identifiers = identifiers
                .iter()
                .map(|value| (*value).to_owned())
                .collect();
            proposal.sections[0].claims[0]
                .prose
                .push_str(" inventedIdentifier and greet are described here.");
            assert!(
                fixture
                    .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
                    .is_err(),
                "accepted {identifiers:?}"
            );
        }
        let mut proposal = simple_proposal();
        proposal.sections[0].claims[0]
            .identifiers
            .push("toUpperCase".to_owned());
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
                .unwrap_err()
                .contains("does not name")
        );
        let mut proposal = simple_proposal();
        proposal.sections[0].claims[0]
            .evidence
            .push(GeneratedChapterEvidence {
                path: SIMPLE_PATH.to_owned(),
                quote: "const overlapping = \"aaaaaaaaa\";".to_owned(),
            });
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
                .unwrap_err()
                .contains("Each evidence quote")
        );
        let mut proposal = simple_proposal();
        proposal.evidence_limits[0]
            .identifiers
            .push("inventedIdentifier".to_owned());
        proposal.evidence_limits[0]
            .prose
            .push_str(" inventedIdentifier remains unproven.");
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
                .unwrap_err()
                .contains("inventedIdentifier")
        );
    }

    #[test]
    fn quotes_preserve_exact_whitespace_and_use_utf8_byte_positions() {
        let fixture = Fixture::new();
        let mut proposal = simple_proposal();
        let quote = "    return `Hello, ${name}`;\n";
        proposal.sections[0].claims.push(claim(
            "inputOutput",
            "name is interpolated into the returned Hello template string in greeting.",
            &["name"],
            &[(SIMPLE_PATH, quote)],
        ));
        proposal.evidence_limits[0].evidence[0].quote =
            "// greeting formats UTF-8 text such as café".to_owned();
        let revision = fixture
            .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
            .unwrap();
        let anchor = &revision.sections[0].claims[1].anchors[0];
        assert_eq!(anchor.excerpt, quote);
        assert_eq!((anchor.start_line, anchor.start_column), (3, 1));
        let unicode_anchor = &revision.sections[1].claims[0].anchors[0];
        assert_eq!(
            unicode_anchor.end_column as usize,
            unicode_anchor.excerpt.len()
        );
        fixture.assert_traceable(&revision, &[SIMPLE_PATH]);
    }

    #[test]
    fn absent_ambiguous_overlapping_and_unselected_quotes_are_rejected() {
        let fixture = Fixture::new();
        for (quote, identifier, expected) in [
            (
                "greeting returns an invented value",
                "greeting",
                "not present",
            ),
            ("// marker repeats.", "marker", "more than once"),
            ("aaaaaaaa", "aaaaaaaa", "more than once"),
        ] {
            let mut proposal = simple_proposal();
            let claim = &mut proposal.sections[0].claims[0];
            claim.prose = format!(
                "The {identifier} declaration is the concrete source behavior being described."
            );
            claim.identifiers = vec![identifier.to_owned()];
            claim.evidence[0].quote = quote.to_owned();
            assert!(
                fixture
                    .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
                    .unwrap_err()
                    .contains(expected)
            );
        }
        let mut proposal = simple_proposal();
        proposal.sections[0].claims[0].evidence[0].path = SUPPORT_PATH.to_owned();
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
                .unwrap_err()
                .contains("not explicitly selected")
        );
    }

    #[test]
    fn target_responsibility_and_each_evidence_limit_require_selected_target_evidence() {
        let fixture = Fixture::new();
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SUPPORT_PATH], &simple_proposal())
                .unwrap_err()
                .contains("target source")
        );
        let support_claim = claim(
            "responsibility",
            "normalizeName returns a trimmed copy of value for a caller to consume.",
            &["normalizeName", "value"],
            &[(SUPPORT_PATH, SUPPORT_SOURCE)],
        );
        let mut proposal = simple_proposal();
        proposal.sections[0].claims[0] = support_claim;
        let mut target_claim = simple_proposal().sections[0].claims.remove(0);
        target_claim.aspect = "execution".to_owned();
        proposal.sections[0].claims.push(target_claim);
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH, SUPPORT_PATH], &proposal)
                .unwrap_err()
                .contains("responsibility must cite")
        );
        let mut proposal = simple_proposal();
        let mut support_only = proposal.sections[0].clone();
        support_only.claims = vec![claim(
            "dependency",
            "normalizeName returns a trimmed copy of value for a caller to consume.",
            &["normalizeName", "value"],
            &[(SUPPORT_PATH, SUPPORT_SOURCE)],
        )];
        proposal.sections.push(support_only);
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH, SUPPORT_PATH], &proposal)
                .unwrap_err()
                .contains("no exact evidence")
        );
        let mut proposal = simple_proposal();
        proposal.evidence_limits.push(limit(
            SUPPORT_PATH, SUPPORT_SOURCE, "normalizeName",
            "normalizeName does not establish the production caller or observed behavior of the selected greeting file.",
        ));
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH, SUPPORT_PATH], &proposal)
                .unwrap_err()
                .contains("Each evidence limit")
        );
    }

    #[test]
    fn strict_schema_generic_claims_and_missing_responsibility_do_not_create_drafts() {
        let fixture = Fixture::new();
        let sources = fixture.sources(&[SIMPLE_PATH]);
        let proposal = simple_proposal();
        let mut unknown_field = serde_json::to_value(&proposal).unwrap();
        unknown_field["sections"][0]["claims"][0]["anchors"] = serde_json::json!([]);
        let mut missing_field = serde_json::to_value(&proposal).unwrap();
        missing_field
            .as_object_mut()
            .unwrap()
            .remove("evidenceLimits");
        for (text, expected_error) in [
            ("not JSON".to_owned(), "not valid JSON"),
            (format!(
                "```json\n{}\n```",
                serde_json::to_string(&proposal).unwrap()
            ), "not valid JSON"),
            (unknown_field.to_string(), "strict chapter proposal schema"),
            (missing_field.to_string(), "strict chapter proposal schema"),
        ] {
            let result = canonicalize_file_chapter(
                &fixture.manifest.summary.project_id,
                &fixture.manifest,
                Some(&fixture.index),
                SIMPLE_PATH,
                &sources,
                &HashSet::new(),
                &text,
            );
            assert!(
                result
                    .unwrap_err()
                    .contains(expected_error)
            );
        }
        let mut generic_title = simple_proposal();
        generic_title.title = "Code overview".to_owned();
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &generic_title)
                .unwrap_err()
                .contains("too generic")
        );
        let mut generic_claim = simple_proposal();
        generic_claim.sections[0].claims[0].prose =
            "This file plays an important role by using greeting to handle a name.".to_owned();
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &generic_claim)
                .unwrap_err()
                .contains("too vague or generic")
        );
        let mut no_role = simple_proposal();
        no_role.sections[0].claims[0].aspect = "execution".to_owned();
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &no_role)
                .unwrap_err()
                .contains("actual responsibility")
        );
        let mut unlabeled_inference = simple_proposal();
        unlabeled_inference.sections[0].claims[0].category = "inference".to_owned();
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &unlabeled_inference)
                .unwrap_err()
                .contains("inference claim needs its basis")
        );
        let oversized = " ".repeat(chapters::MAX_GENERATED_RESPONSE_BYTES + 1);
        assert!(
            canonicalize_file_chapter(
                &fixture.manifest.summary.project_id,
                &fixture.manifest,
                Some(&fixture.index),
                SIMPLE_PATH,
                &sources,
                &HashSet::new(),
                &oversized,
            )
            .unwrap_err()
            .contains("structured-draft limit")
        );
    }

    #[test]
    fn mechanically_traceable_but_semantically_shallow_claim_still_needs_human_review() {
        let fixture = Fixture::new();
        let mut proposal = simple_proposal();
        let shallow = "greeting is a useful function that works with name and helps its caller.";
        proposal.sections[0].prose = shallow.to_owned();
        proposal.sections[0].claims[0].prose = shallow.to_owned();

        // Both listed identifiers occur in the exact cited function, so the current
        // canonicalizer can establish lexical/source traceability, but not claim depth.
        let revision = fixture
            .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
            .expect("the mechanical gate accepts this schema-valid, cited claim");
        assert_eq!(revision.sections[0].claims[0].prose, shallow);
        fixture.assert_traceable(&revision, &[SIMPLE_PATH]);
    }

    #[test]
    fn committed_quality_gate_example_passes_mechanical_canonicalization() {
        let fixture = Fixture::new();
        let example: serde_json::Value = serde_json::from_str(include_str!(
            "../../benchmarks/chapter-quality/review/QUALITY_GATE_EXAMPLE.json"
        ))
        .expect("the checked-in rubric example is valid JSON");
        let proposal: GeneratedChapterProposal = serde_json::from_value(
            example
                .get("shallowCandidate")
                .expect("the example includes a shallow proposal")
                .clone(),
        )
        .expect("the shallow example has the generated proposal shape");

        let revision = fixture
            .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
            .expect("schema and citation validation do not judge semantic depth");
        assert_eq!(
            revision.sections[0].claims[0].prose,
            "greeting uses name to build a result string."
        );
        fixture.assert_traceable(&revision, &[SIMPLE_PATH]);
    }

    #[test]
    fn supplied_bytes_size_snapshot_and_index_must_match_the_immutable_manifest() {
        let fixture = Fixture::new();
        let text = serde_json::to_string(&simple_proposal()).unwrap();
        let canonicalize =
            |manifest: &SnapshotManifest, index: &SnapshotIndex, sources: &[SnapshotSource]| {
                canonicalize_file_chapter(
                    &fixture.manifest.summary.project_id,
                    manifest,
                    Some(index),
                    SIMPLE_PATH,
                    sources,
                    &HashSet::new(),
                    &text,
                )
            };
        let mut forged = fixture.sources(&[SIMPLE_PATH]);
        forged[0].contents = forged[0].contents.replace("Hello", "Jello");
        assert!(
            canonicalize(&fixture.manifest, &fixture.index, &forged)
                .unwrap_err()
                .contains("bytes or hash")
        );
        let mut wrong_size = fixture.manifest.clone();
        wrong_size
            .files
            .iter_mut()
            .find(|file| file.path == SIMPLE_PATH)
            .unwrap()
            .size_bytes += 1;
        assert!(
            canonicalize(
                &wrong_size,
                &fixture.index,
                &fixture.sources(&[SIMPLE_PATH])
            )
            .unwrap_err()
            .contains("bytes or hash")
        );
        let mut old_source = fixture.sources(&[SIMPLE_PATH]);
        old_source[0].snapshot_id = "old-snapshot".to_owned();
        assert!(
            canonicalize(&fixture.manifest, &fixture.index, &old_source)
                .unwrap_err()
                .contains("different snapshot")
        );
        let sources = fixture.sources(&[SIMPLE_PATH, SIMPLE_PATH]);
        assert!(
            canonicalize(&fixture.manifest, &fixture.index, &sources)
                .unwrap_err()
                .contains("duplicated")
        );
        let mut unknown_source = fixture.sources(&[SIMPLE_PATH]);
        unknown_source[0].path = "never-captured.ts".to_owned();
        assert!(
            canonicalize(&fixture.manifest, &fixture.index, &unknown_source)
                .unwrap_err()
                .contains("not a captured file")
        );
        let mut old_index = fixture.index.clone();
        old_index.snapshot_id = "old-snapshot".to_owned();
        assert!(
            canonicalize(
                &fixture.manifest,
                &old_index,
                &fixture.sources(&[SIMPLE_PATH])
            )
            .unwrap_err()
            .contains("different project or snapshot")
        );
    }

    #[test]
    fn relationship_citations_require_reviewed_ids_selected_endpoints_and_matching_source() {
        let fixture = Fixture::new();
        let proposal = complex_proposal(&fixture);
        let relationship_id = proposal.sections[1].claims[0].relationship_ids[0].clone();
        let sources = fixture.sources(&[COMPLEX_PATH, SUPPORT_PATH]);
        let text = serde_json::to_string(&proposal).unwrap();
        let reviewed = HashSet::from([relationship_id.clone()]);
        assert!(
            canonicalize_file_chapter(
                &fixture.manifest.summary.project_id,
                &fixture.manifest,
                Some(&fixture.index),
                COMPLEX_PATH,
                &sources,
                &HashSet::new(),
                &text,
            )
            .unwrap_err()
            .contains("not included in the reviewed request")
        );
        assert!(
            canonicalize_file_chapter(
                &fixture.manifest.summary.project_id,
                &fixture.manifest,
                None,
                COMPLEX_PATH,
                &sources,
                &reviewed,
                &text,
            )
            .unwrap_err()
            .contains("no saved symbol index")
        );
        let mut unknown = proposal.clone();
        unknown.sections[1].claims[0].relationship_ids = vec!["rel:invented".to_owned()];
        assert!(
            fixture
                .generate(COMPLEX_PATH, &[COMPLEX_PATH, SUPPORT_PATH], &unknown)
                .unwrap_err()
                .contains("absent from the saved snapshot index")
        );
        let mut duplicate = proposal.clone();
        duplicate.sections[1].claims[0]
            .relationship_ids
            .push(relationship_id.clone());
        assert!(
            fixture
                .generate(COMPLEX_PATH, &[COMPLEX_PATH, SUPPORT_PATH], &duplicate)
                .unwrap_err()
                .contains("cited twice")
        );
        let hidden_symbol = fixture
            .index
            .symbols
            .iter()
            .find(|symbol| symbol.path == UNSELECTED_PATH)
            .unwrap();
        for (field, expected) in [
            ("target_path", "not explicitly selected"),
            ("source_id", "source ID"),
            ("target_id", "target that was not explicitly selected"),
            ("hash", "evidence does not match"),
            ("range", "evidence does not match"),
        ] {
            let mut index = fixture.index.clone();
            let relationship = index
                .relationships
                .iter_mut()
                .find(|item| item.relationship_id == relationship_id)
                .unwrap();
            match field {
                "target_path" => relationship.target_path = Some(UNSELECTED_PATH.to_owned()),
                "source_id" => relationship.source_id = hidden_symbol.symbol_id.clone(),
                "target_id" => relationship.target_id = Some(hidden_symbol.symbol_id.clone()),
                "hash" => relationship.evidence.source_sha256 = "0".repeat(64),
                "range" => relationship.evidence.end_byte = usize::MAX,
                _ => unreachable!(),
            }
            assert!(
                canonicalize_file_chapter(
                    &fixture.manifest.summary.project_id,
                    &fixture.manifest,
                    Some(&index),
                    COMPLEX_PATH,
                    &sources,
                    &reviewed,
                    &text,
                )
                .unwrap_err()
                .contains(expected),
                "accepted unsupported {field}"
            );
        }
        let mut unselected = proposal;
        unselected.sections[1].claims[0]
            .evidence
            .retain(|evidence| evidence.path == COMPLEX_PATH);
        unselected.sections[1].claims.truncate(1);
        assert!(
            fixture
                .generate(COMPLEX_PATH, &[COMPLEX_PATH], &unselected)
                .unwrap_err()
                .contains("not explicitly selected")
        );
    }

    #[test]
    fn unresolved_index_relationships_remain_visibly_unresolved() {
        let fixture = Fixture::new();
        let relationship = fixture.relationship(
            RelationshipKind::Calls,
            COMPLEX_PATH,
            "normalized.toUpperCase",
        );
        assert_eq!(relationship.confidence, RelationshipConfidence::Unresolved);
        assert!(relationship.target_id.is_none());
        let mut proposal = complex_proposal(&fixture);
        proposal.sections[1].claims[2].aspect = "relationship".to_owned();
        proposal.sections[1].claims[2]
            .relationship_ids
            .push(relationship.relationship_id.clone());
        assert!(
            proposal.sections[1].claims[2]
                .relationship_ids
                .contains(&relationship.relationship_id)
        );
        let revision = fixture
            .generate(COMPLEX_PATH, &[COMPLEX_PATH, SUPPORT_PATH], &proposal)
            .unwrap();
        assert!(
            revision.sections[1].claims[2]
                .prose
                .contains("calls relationship, unresolved confidence")
        );
        fixture.assert_traceable(&revision, &[COMPLEX_PATH, SUPPORT_PATH]);
    }

    #[test]
    fn unsupported_runtime_effects_and_business_relationships_require_actual_source_evidence() {
        let fixture = Fixture::new();
        // These gates establish mechanical traceability. They cannot certify the
        // meaning of arbitrary natural-language paraphrases of a valid quote.
        for (prose, identifiers) in [
            (
                "greeting writes every returned name to stdout at runtime.",
                vec!["greeting", "stdout"],
            ),
            (
                "greeting submits checkoutRequest and runs chargeCreditCard before formatting name.",
                vec!["greeting", "checkoutRequest", "chargeCreditCard"],
            ),
        ] {
            let mut proposal = simple_proposal();
            proposal.sections[0].claims[0].prose = prose.to_owned();
            proposal.sections[0].claims[0].identifiers = identifiers
                .iter()
                .map(|identifier| (*identifier).to_owned())
                .collect();
            assert!(
                fixture
                    .generate(SIMPLE_PATH, &[SIMPLE_PATH], &proposal)
                    .unwrap_err()
                    .contains("does not connect identifier")
            );
        }
        let mut unsupported_flow = simple_proposal();
        unsupported_flow.sections[0].claims.push(claim(
            "relationship", "greeting forwards name through the billing business flow before returning its template.",
            &["greeting", "name"], &[(SIMPLE_PATH, SIMPLE_FUNCTION)],
        ));
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &unsupported_flow)
                .unwrap_err()
                .contains("must cite a relationship")
        );
        unsupported_flow.sections[0].claims[1].relationship_ids =
            vec!["rel:invented-billing-flow".to_owned()];
        assert!(
            fixture
                .generate(SIMPLE_PATH, &[SIMPLE_PATH], &unsupported_flow)
                .unwrap_err()
                .contains("absent from the saved snapshot index")
        );
        assert!(
            crate::chapter_store::list_for_project(
                &fixture.storage,
                &fixture.manifest.summary.project_id
            )
            .unwrap()
            .is_empty()
        );

        let mut labeled_inference = simple_proposal();
        let mut inference = claim(
            "dependency",
            "greeting may be used as presentation text because its return value includes a Hello prefix.",
            &["greeting"],
            &[(SIMPLE_PATH, SIMPLE_FUNCTION)],
        );
        inference.category = "inference".to_owned();
        inference.inference_basis = "The return expression formats a Hello prefix; selected evidence does not include its production callers.".to_owned();
        labeled_inference.sections[0].claims.push(inference);
        let revision = fixture
            .generate(SIMPLE_PATH, &[SIMPLE_PATH], &labeled_inference)
            .unwrap();
        assert!(
            matches!(&revision.sections[0].claims[1].category, ClaimCategory::Inference { visible_label, basis }
            if visible_label == "Inference" && !basis.is_empty())
        );
        fixture.assert_traceable(&revision, &[SIMPLE_PATH]);
    }

    #[test]
    fn saving_a_generated_chapter_adds_revisions_only_to_the_requested_file_target() {
        let fixture = Fixture::new();
        let project_id = &fixture.manifest.summary.project_id;
        let proposal = complex_proposal(&fixture);
        let paths = &[COMPLEX_PATH, SUPPORT_PATH];
        let mut preview = fixture.generate(COMPLEX_PATH, paths, &proposal).unwrap();
        fixture.assert_traceable(&preview, paths);
        assert!(
            crate::chapter_store::list_for_project(&fixture.storage, project_id)
                .unwrap()
                .is_empty()
        );
        preview.validation_status = chapters::ValidationStatus::Valid;
        let first = crate::chapter_store::save_new_draft(&fixture.storage, preview).unwrap();
        let saved = crate::chapter_store::list_for_snapshot(
            &fixture.storage,
            project_id,
            &fixture.manifest.summary.snapshot_id,
        )
        .unwrap();
        assert_eq!(saved, vec![first.clone()]);
        for file in &fixture.manifest.files {
            let count = saved
                .iter()
                .filter(|revision| {
                    revision.target.scope
                        == chapters::ChapterTargetScope::File {
                            path: file.path.clone(),
                        }
                })
                .count();
            assert_eq!(
                count,
                usize::from(file.path == COMPLEX_PATH),
                "unexpected draft for {}",
                file.path
            );
        }
        let mut next = fixture.generate(COMPLEX_PATH, paths, &proposal).unwrap();
        fixture.assert_traceable(&next, paths);
        next.validation_status = chapters::ValidationStatus::Valid;
        let second = crate::chapter_store::save_new_draft(&fixture.storage, next).unwrap();
        assert_eq!(second.revision_number, first.revision_number + 1);
        let saved = crate::chapter_store::list_for_project(&fixture.storage, project_id).unwrap();
        assert_eq!(saved, vec![first.clone(), second]);
        assert_eq!(saved[0], first);
        assert!(saved.iter().all(|revision| {
            revision.target.scope
                == chapters::ChapterTargetScope::File {
                    path: COMPLEX_PATH.to_owned(),
                }
        }));
    }
}
