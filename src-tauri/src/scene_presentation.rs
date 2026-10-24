//! Bounded AI-authored HTML compositions. Data is interpreted by app-owned
//! components; no generated markup, CSS, scripts, URLs or event handlers run.

use crate::chapters::{ChapterClaim, VisualNode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub(crate) const MAX_FOCUSED_GRAPH_NODES: usize = 8;

pub(crate) fn visible_graph_node_count(
    node_count: usize,
    focused: &HashSet<usize>,
    edges: impl IntoIterator<Item = (usize, usize)>,
) -> usize {
    if focused.is_empty() {
        return node_count;
    }
    let mut visible = focused.clone();
    for (from, to) in edges {
        if focused.contains(&from) || focused.contains(&to) {
            visible.insert(from);
            visible.insert(to);
        }
    }
    visible.len()
}

mod quality;
pub(crate) mod tree;
pub(crate) use quality::repetition_positions;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScenePresentation {
    pub schema_version: u32,
    pub scenes: Vec<PresentationScene>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PresentationScene {
    pub claim_index: usize,
    pub title: String,
    pub narration: String,
    pub mode: SceneMode,
    pub elements: Vec<SceneElement>,
}

/// A first-match decision must keep source order visible. In particular, the
/// LIFO stack renderer reverses its children and must never stand in for an
/// ordered selector. This is a narrow wording check, not semantic proof.
pub(crate) fn requires_ordered_selector(claim: &str, title: &str, narration: &str) -> bool {
    let text = format!("{claim} {title} {narration}")
        .to_lowercase()
        .replace(['-', '_'], " ");
    [
        "first match",
        "first matching",
        "first applicable",
        "first successful",
        "ordered probe",
        "probe order",
        "ordered checks",
        "checked in source order",
        "checks in source order",
        "priority order",
        "priority chain",
        "precedence chain",
        "fall through",
        "fallthrough",
        "sequential early return",
        "early return guard chain",
        "multiple early returns",
        "several early returns",
        "guard chain",
        "ordered guards",
    ]
    .iter()
    .any(|signal| text.contains(signal))
}

pub(crate) fn has_ordered_selector_root(elements: &[SceneElement]) -> bool {
    elements
        .first()
        .is_some_and(|root| root.kind == ElementKind::Selector)
}

/// Fresh model output must not use a graph for source-ordered guard chains.
/// Stored scenes retain the version-3 replay contract; this stricter choice
/// check is called only while admitting newly generated chapter content.
pub(crate) fn validate_fresh_semantic_layouts(
    presentation: &ScenePresentation,
    claims: &[ChapterClaim],
) -> Result<(), String> {
    for (scene, claim) in presentation.scenes.iter().zip(claims) {
        if requires_ordered_selector(&claim.prose, &scene.title, &scene.narration)
            && (scene.mode != SceneMode::Composition || !has_ordered_selector_root(&scene.elements))
        {
            return Err(
                "Source-ordered guards need a selector composition at the root, not a graph or another layout."
                    .into(),
            );
        }
    }
    Ok(())
}

/// Authored speech stays separate from exact implementation claims. This checks
/// coverage/identity and bounds, not whether a paraphrase preserves semantics.
pub(crate) fn narration(
    presentation: &ScenePresentation,
    claims: &[ChapterClaim],
) -> Result<Vec<String>, String> {
    if presentation.schema_version != 3 {
        return Err("A scene presentation must use schema version 3.".into());
    }
    if presentation.scenes.len() != claims.len() || claims.is_empty() || claims.len() > 20 {
        return Err("Scene narration needs one ordered scene for every claim.".into());
    }
    let mut scripts = Vec::with_capacity(claims.len());
    for (index, (scene, claim)) in presentation.scenes.iter().zip(claims).enumerate() {
        let text = scene.narration.trim();
        if scene.claim_index != index
            || claim.anchors.is_empty()
            || text.is_empty()
            || text.len() > 4_000
        {
            return Err(
                "Each scene needs bounded narration and its own consecutive anchored claim.".into(),
            );
        }
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        scripts.push(match &claim.category {
            crate::chapters::ClaimCategory::Inference { visible_label, .. } => {
                if visible_label.trim().is_empty() {
                    return Err("An inferred scene needs its visible inference label.".into());
                }
                format!("{}: {text}", visible_label.trim())
            }
            _ => text,
        });
    }
    if !(20..=4_000).contains(&scripts.join(" ").len()) {
        return Err("The complete scene narration must be 20–4,000 UTF-8 bytes; split the section without omitting behavior.".into());
    }
    Ok(scripts)
}

/// Known proposal-reference mismatch, shared by chapter and progressive repair.
/// Positions only: no code, labels or quotes leave this diagnostic. It resolves
/// no evidence and cannot admit a draft or renew the single correction budget.
pub(crate) fn foreign_claim_positions(section: &Value) -> Vec<(usize, usize)> {
    let visual = &section["visual"];
    let Some(nodes) = visual["nodes"].as_array().filter(|n| n.len() <= 12) else {
        return vec![];
    };
    let Ok(presentation) =
        serde_json::from_value::<ScenePresentation>(visual["presentation"].clone())
    else {
        return vec![];
    };
    if presentation.schema_version != 3 || presentation.scenes.len() > 20 {
        return vec![];
    }
    let mut positions = Vec::new();
    for (scene_index, scene) in presentation.scenes.iter().enumerate() {
        if scene.mode != SceneMode::Composition
            || scene.claim_index != scene_index
            || scene.elements.len() > 48
        {
            continue;
        }
        for (element_index, element) in scene.elements.iter().enumerate() {
            if element.kind.container() || element.kind.decorative() {
                continue;
            }
            let Some(references) = element
                .node_index
                .and_then(|n| nodes.get(n))
                .and_then(|node| node["evidence"].as_array())
            else {
                continue;
            };
            if !references
                .iter()
                .any(|r| r["claimIndex"].as_u64() == Some(scene_index as u64))
            {
                positions.push((scene_index, element_index));
                if positions.len() == 64 {
                    return positions;
                }
            }
        }
    }
    positions
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SceneMode {
    Composition,
    Graph,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SceneElement {
    pub kind: ElementKind,
    pub parent_index: Option<usize>,
    pub node_index: Option<usize>,
    pub tone: ElementTone,
    pub value: String,
    pub value_origin: ValueOrigin,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementKind {
    Row,
    Column,
    Grid,
    Panel,
    Compare,
    Stack,
    Timeline,
    Lanes,
    Branch,
    BeforeAfter,
    Transform,
    Tree,
    Queue,
    Selector,
    Card,
    Token,
    Code,
    Metric,
    Bytes,
    Matrix,
    Table,
    Text,
    Arrow,
    Divider,
}
impl ElementKind {
    fn container(&self) -> bool {
        matches!(
            self,
            Self::Row
                | Self::Column
                | Self::Grid
                | Self::Panel
                | Self::Compare
                | Self::Stack
                | Self::Timeline
                | Self::Lanes
                | Self::Branch
                | Self::BeforeAfter
                | Self::Transform
                | Self::Tree
                | Self::Queue
                | Self::Selector
        )
    }
    fn decorative(&self) -> bool {
        matches!(self, Self::Arrow | Self::Divider)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ElementTone {
    Neutral,
    Accent,
    Muted,
    Positive,
    Warning,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ValueOrigin {
    None,
    Source,
    Explanation,
    Illustration,
}

/// Same checks apply at generation and when validating an edited saved chapter.
pub fn validate(
    presentation: &ScenePresentation,
    nodes: &[VisualNode],
    claims: &[ChapterClaim],
) -> Result<(), String> {
    if presentation.schema_version != 3
        || presentation.scenes.len() != claims.len()
        || presentation.scenes.is_empty()
        || presentation.scenes.len() > 20
    {
        return Err(
            "A scene presentation needs version 3 and one scene for each claim, in claim order."
                .into(),
        );
    }
    narration(presentation, claims)?;
    for (index, scene) in presentation.scenes.iter().enumerate() {
        if scene.claim_index != index || scene.title.trim().is_empty() || scene.title.len() > 120 {
            return Err(
                "A scene needs its consecutive claimIndex and a short visible title.".into(),
            );
        }
        if scene.mode == SceneMode::Graph {
            if !scene.elements.is_empty() {
                return Err("A graph scene must have no HTML composition elements.".into());
            }
            continue;
        }
        if requires_ordered_selector(&claims[index].prose, &scene.title, &scene.narration)
            && !has_ordered_selector_root(&scene.elements)
        {
            return Err("A first-match composition needs a root selector layout so each guard, outcome and fall-through stays visible; a LIFO stack reverses the priority.".into());
        }
        if let Some(finding) = tree::findings(&scene.elements).first() {
            return Err(finding.message.into());
        }
        for element in &scene.elements {
            if element.kind.container() || element.kind.decorative() {
                continue;
            }
            let node = element.node_index.and_then(|n| nodes.get(n)).ok_or(
                "A content element needs an existing nodeIndex for its label and code evidence.",
            )?;
            let anchors: Vec<_> = claims[index]
                .anchors
                .iter()
                .filter(|a| node.evidence_anchor_ids.contains(&a.anchor_id))
                .collect();
            if anchors.is_empty() {
                return Err("A scene content element must cite its own claim's evidence.".into());
            }
            if element.value.len() > 1024 {
                return Err("A scene value may contain at most 1,024 UTF-8 bytes.".into());
            }
            match element.value_origin {
                ValueOrigin::None if element.value.is_empty() => {},
                ValueOrigin::Source if !element.value.trim().is_empty()
                    && anchors.iter().any(|a| a.excerpt.contains(&element.value)) => {},
                ValueOrigin::Explanation if !element.value.trim().is_empty() => {},
                ValueOrigin::Illustration if !element.value.trim().is_empty() => {},
                _ => return Err("A value needs an exact source substring, grounded explanation, or explicitly labeled illustration; empty values use origin none.".into()),
            }
            if element.kind == ElementKind::Matrix {
                let rows: Vec<_> = element.value.lines().collect();
                if rows.is_empty()
                    || rows.len() > 16
                    || rows.iter().any(|r| {
                        r.is_empty()
                            || r.len() > 24
                            || r.len() != rows[0].len()
                            || !r.bytes().all(|b| b == b'0' || b == b'1' || b == b'.')
                    })
                {
                    return Err(
                        "A matrix needs 1–16 equal rows of 1–24 cells using only 0, 1 or dot."
                            .into(),
                    );
                }
            }
            if element.kind == ElementKind::Table {
                let rows: Vec<_> = element.value.lines().collect();
                if rows.is_empty()
                    || rows.len() > 8
                    || rows.iter().any(|r| {
                        r.split('|').count() > 6
                            || r.split('|').count() != rows[0].split('|').count()
                    })
                {
                    return Err("A table needs at most eight rows and six pipe-separated columns of equal width.".into());
                }
            }
            if element.kind == ElementKind::Bytes
                && (element.value.split_whitespace().count() > 32
                    || element.value.trim().is_empty())
            {
                return Err("A byte strip needs 1–32 whitespace-separated cells.".into());
            }
        }
    }
    let labels: Vec<_> = nodes.iter().map(|n| n.label.as_str()).collect();
    if !repetition_positions(presentation, &labels).is_empty() {
        return Err("A transformation must show distinct input/result labels or values.".into());
    }
    Ok(())
}

/// A narrated graph scene must remain readable in the focused reader view.
/// It shows nodes cited by the scene's claim plus one edge of context; when no
/// node cites that claim, the reader falls back to the full graph.
pub(crate) fn validate_graph_density(
    presentation: &ScenePresentation,
    nodes: &[VisualNode],
    edges: &[crate::chapters::VisualEdge],
    claims: &[ChapterClaim],
) -> Result<(), String> {
    let indices: HashMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| (node.node_id.as_str(), index))
        .collect();
    for scene in &presentation.scenes {
        if scene.mode != SceneMode::Graph {
            continue;
        }
        let Some(claim) = claims.get(scene.claim_index) else {
            continue;
        };
        let anchor_ids: HashSet<_> = claim
            .anchors
            .iter()
            .map(|anchor| anchor.anchor_id.as_str())
            .collect();
        let focused: HashSet<_> = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                node.evidence_anchor_ids
                    .iter()
                    .any(|id| anchor_ids.contains(id.as_str()))
            })
            .map(|(index, _)| index)
            .collect();
        let graph_edges = edges.iter().filter_map(|edge| {
            Some((
                *indices.get(edge.from_node_id.as_str())?,
                *indices.get(edge.to_node_id.as_str())?,
            ))
        });
        let count = visible_graph_node_count(nodes.len(), &focused, graph_edges);
        if count > MAX_FOCUSED_GRAPH_NODES {
            return Err("A focused graph scene may show at most eight nodes, including one edge of context. Split the explanation into narrower claims and scenes or use a meaningful composition; preserve all outcomes and the full map.".into());
        }
    }
    Ok(())
}

/// Flat parent indices keep the model schema small and avoid recursive schemas.
pub(crate) fn proposal_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["schemaVersion","scenes"],"properties":{
        "schemaVersion":{"type":"integer","enum":[3]},
        "scenes":{"type":"array","minItems":1,"maxItems":20,"items":{
            "type":"object","additionalProperties":false,"required":["claimIndex","title","narration","mode","elements"],"properties":{
                "claimIndex":{"type":"integer","minimum":0,"maximum":19},
                "title":{"type":"string","minLength":1,"maxLength":120},
                "narration":{"type":"string","minLength":1,"maxLength":4000},
                "mode":{"type":"string","enum":["composition","graph"]},
                "elements":{"type":"array","maxItems":48,"items":{
                    "type":"object","additionalProperties":false,
                    "required":["kind","parentIndex","nodeIndex","tone","value","valueOrigin"],"properties":{
                        "kind":{"type":"string","enum":["row","column","grid","panel","compare","stack","timeline","lanes","branch","beforeAfter","transform","tree","queue","selector","card","token","code","metric","bytes","matrix","table","text","arrow","divider"]},
                        "parentIndex":{"type":["integer","null"],"minimum":0,"maximum":47},
                        "nodeIndex":{"type":["integer","null"],"minimum":0,"maximum":11},
                        "tone":{"type":"string","enum":["neutral","accent","muted","positive","warning"]},
                        "value":{"type":"string","maxLength":1024},
                        "valueOrigin":{"type":"string","enum":["none","source","explanation","illustration"]}
                    }}}
            }}}
    }})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_density_matches_the_reader_focus_and_one_edge_context() {
        let make = |context_count: usize| {
            let (template_nodes, claims, mut presentation) = fixture();
            let active_anchor = claims[0].anchors[0].anchor_id.clone();
            let context_anchor = claims[1].anchors[0].anchor_id.clone();
            let nodes = (0..=context_count)
                .map(|index| {
                    let mut node = template_nodes[0].clone();
                    node.node_id = format!("density-{index}");
                    node.evidence_anchor_ids = vec![if index == 0 {
                        active_anchor.clone()
                    } else {
                        context_anchor.clone()
                    }];
                    node
                })
                .collect::<Vec<_>>();
            let edges = (1..=context_count)
                .map(|index| crate::chapters::VisualEdge {
                    edge_id: format!("edge-{index}"),
                    from_node_id: nodes[0].node_id.clone(),
                    to_node_id: nodes[index].node_id.clone(),
                    label: "related step".into(),
                    evidence_anchor_ids: vec![active_anchor.clone()],
                })
                .collect::<Vec<_>>();
            presentation.scenes[0].mode = SceneMode::Graph;
            presentation.scenes[0].elements.clear();
            (nodes, edges, claims, presentation)
        };

        let (nodes, edges, claims, presentation) = make(7);
        assert!(validate_graph_density(&presentation, &nodes, &edges, &claims).is_ok());
        let (nodes, edges, claims, presentation) = make(8);
        assert!(
            validate_graph_density(&presentation, &nodes, &edges, &claims)
                .unwrap_err()
                .contains("at most eight nodes")
        );
    }

    #[test]
    fn old_scene_versions_and_missing_current_narration_are_rejected() {
        let (nodes, claims, mut presentation) = fixture();
        for version in [1, 2] {
            presentation.schema_version = version;
            assert!(validate(&presentation, &nodes, &claims).is_err());
            assert!(narration(&presentation, &claims).is_err());
        }
        presentation.schema_version = 3;
        let mut value = serde_json::to_value(&presentation).unwrap();
        value["scenes"][0]
            .as_object_mut()
            .unwrap()
            .remove("narration");
        assert!(serde_json::from_value::<ScenePresentation>(value).is_err());
    }

    #[test]
    fn first_match_story_rejects_a_reversed_stack_and_accepts_selector_layout() {
        let (nodes, mut claims, mut presentation) = fixture();
        claims[0].prose = "The normal renderer uses a sequential early-return guard chain: each match returns, otherwise the next guard is tried.".into();
        let scene = &mut presentation.scenes[0];
        scene.title = "The first matching flag wins".into();
        scene.elements = vec![
            SceneElement {
                kind: ElementKind::Stack,
                parent_index: None,
                node_index: None,
                tone: ElementTone::Neutral,
                value: String::new(),
                value_origin: ValueOrigin::None,
            },
            SceneElement {
                kind: ElementKind::Text,
                parent_index: Some(0),
                node_index: Some(3),
                tone: ElementTone::Accent,
                value: "Default fallback".into(),
                value_origin: ValueOrigin::Illustration,
            },
        ];
        let error = validate(&presentation, &nodes, &claims).unwrap_err();
        assert!(error.contains("LIFO stack reverses the priority"));

        let leaf = |kind, parent, value: &str| SceneElement {
            kind,
            parent_index: Some(parent),
            node_index: Some(3),
            tone: ElementTone::Accent,
            value: value.into(),
            value_origin: ValueOrigin::Illustration,
        };
        presentation.scenes[0].elements = vec![
            SceneElement {
                kind: ElementKind::Selector,
                parent_index: None,
                node_index: None,
                tone: ElementTone::Neutral,
                value: String::new(),
                value_origin: ValueOrigin::None,
            },
            SceneElement {
                kind: ElementKind::Branch,
                parent_index: Some(0),
                node_index: None,
                tone: ElementTone::Neutral,
                value: String::new(),
                value_origin: ValueOrigin::None,
            },
            leaf(ElementKind::Text, 1, "Flag A is set?"),
            leaf(ElementKind::Token, 1, "Load by route A"),
            SceneElement {
                kind: ElementKind::Branch,
                parent_index: Some(0),
                node_index: None,
                tone: ElementTone::Neutral,
                value: String::new(),
                value_origin: ValueOrigin::None,
            },
            leaf(ElementKind::Text, 4, "No earlier flag matched"),
            leaf(ElementKind::Token, 4, "Use the default loader"),
        ];
        validate(&presentation, &nodes, &claims).unwrap();

        let mut wrapped = presentation.scenes[0].elements.clone();
        for element in &mut wrapped {
            element.parent_index = Some(element.parent_index.map_or(0, |parent| parent + 1));
        }
        wrapped.insert(
            0,
            SceneElement {
                kind: ElementKind::Stack,
                parent_index: None,
                node_index: None,
                tone: ElementTone::Neutral,
                value: String::new(),
                value_origin: ValueOrigin::None,
            },
        );
        presentation.scenes[0].elements = wrapped;
        let error = validate(&presentation, &nodes, &claims).unwrap_err();
        assert!(error.contains("root selector layout"));
    }

    #[test]
    fn fresh_ordered_early_returns_cannot_choose_graph_mode() {
        let (nodes, claims, mut presentation) = fixture();
        let mut claims = vec![claims[0].clone()];
        presentation.scenes.truncate(1);
        claims[0].prose = "Several early returns are checked in source order; a miss continues to the next guard.".into();
        presentation.scenes[0].mode = SceneMode::Graph;
        presentation.scenes[0].elements.clear();

        // Historical version-3 graphs remain replayable, but a fresh proposal
        // must use the source-ordered selector grammar for these guards.
        validate(&presentation, &nodes, &claims).unwrap();
        assert!(
            validate_fresh_semantic_layouts(&presentation, &claims)
                .unwrap_err()
                .contains("selector composition at the root")
        );

        presentation.scenes[0].mode = SceneMode::Composition;
        presentation.scenes[0].elements = vec![SceneElement {
            kind: ElementKind::Selector,
            parent_index: None,
            node_index: None,
            tone: ElementTone::Neutral,
            value: String::new(),
            value_origin: ValueOrigin::None,
        }];
        validate_fresh_semantic_layouts(&presentation, &claims).unwrap();
    }

    fn fixture() -> (Vec<VisualNode>, Vec<ChapterClaim>, ScenePresentation) {
        let artifact: Value = serde_json::from_str(include_str!(
            "../../docs/acceptance/artifacts/2026-10-17-fontdone-complete-script-partial.json"
        ))
        .unwrap();
        let revision: crate::chapters::ChapterRevision = serde_json::from_value(
            artifact["answer"]["sections"][0]["preview"]["revision"].clone(),
        )
        .unwrap();
        let section = &revision.sections[0];
        let nodes = section
            .visual
            .as_ref()
            .unwrap()
            .typed_visual()
            .unwrap()
            .nodes
            .clone();
        let mut scenes: Vec<_> = section
            .claims
            .iter()
            .enumerate()
            .map(|(i, _)| PresentationScene {
                claim_index: i,
                title: format!("Scene {}", i + 1),
                narration: "The operation handles the input and returns its result.".into(),
                mode: SceneMode::Graph,
                elements: vec![],
            })
            .collect();
        scenes[0].mode = SceneMode::Composition;
        scenes[0].elements = vec![SceneElement {
            kind: ElementKind::Code,
            parent_index: None,
            node_index: Some(3),
            tone: ElementTone::Accent,
            value: "PFR0".into(),
            value_origin: ValueOrigin::Source,
        }];
        (
            nodes,
            section.claims.clone(),
            ScenePresentation {
                schema_version: 3,
                scenes,
            },
        )
    }

    #[test]
    fn exact_source_values_and_mixed_graph_html_scenes_roundtrip() {
        let (nodes, claims, presentation) = fixture();
        validate(&presentation, &nodes, &claims).unwrap();
        let encoded = serde_json::to_value(&presentation).unwrap();
        let decoded: ScenePresentation = serde_json::from_value(encoded).unwrap();
        assert_eq!(presentation, decoded);
        assert!(validate(&decoded, &nodes, &claims).is_ok());
    }

    #[test]
    fn grounded_explanatory_values_have_a_distinct_serialized_origin() {
        let (nodes, claims, mut presentation) = fixture();
        let element = &mut presentation.scenes[0].elements[0];
        element.value = "The first matching format selects this decoder".into();
        element.value_origin = ValueOrigin::Explanation;
        validate(&presentation, &nodes, &claims).unwrap();

        let encoded = serde_json::to_value(&presentation).unwrap();
        assert_eq!(
            encoded["scenes"][0]["elements"][0]["valueOrigin"],
            "explanation"
        );
        assert!(proposal_schema()["properties"]["scenes"]["items"]["properties"]["elements"]["items"]["properties"]["valueOrigin"]["enum"]
            .as_array().is_some_and(|origins| origins.iter().any(|origin| origin == "explanation")));
    }

    #[test]
    fn authored_speech_covers_every_claim_without_requiring_code_names_in_audio() {
        let (nodes, mut claims, mut presentation) = fixture();
        for scene in &mut presentation.scenes {
            scene.narration = "For example, the input passes this check; an invalid input keeps its original error.".into();
        }
        claims[0].category = crate::chapters::ClaimCategory::Inference {
            visible_label: "Inference".into(),
            basis: "Based on the captured branch.".into(),
        };
        validate(&presentation, &nodes, &claims).unwrap();
        let scripts = narration(&presentation, &claims).unwrap();
        assert_eq!(scripts.len(), claims.len());
        assert!(scripts[0].starts_with("Inference: For example"));
        assert!(scripts.iter().skip(1).all(|s| !s.starts_with("Inference:")));
        let encoded = serde_json::to_value(&presentation).unwrap();
        assert_eq!(
            serde_json::from_value::<ScenePresentation>(encoded).unwrap(),
            presentation
        );
        for invalid in [String::new(), "é".repeat(2_001)] {
            let mut broken = presentation.clone();
            broken.scenes[0].narration = invalid;
            assert!(validate(&broken, &nodes, &claims).is_err());
        }
        let mut missing = presentation.clone();
        missing.scenes.pop();
        assert!(narration(&missing, &claims).is_err());
        let mut wrong = presentation.clone();
        wrong.scenes[0].claim_index = 1;
        assert!(narration(&wrong, &claims).is_err());
        for scene in &mut presentation.scenes {
            scene.narration = "é".repeat(600);
        }
        assert!(narration(&presentation, &claims).is_err());
    }

    #[test]
    fn current_scene_requires_a_visible_transition_but_allows_real_state_comparisons() {
        let (mut nodes, claims, mut presentation) = fixture();
        let elements = json!([
            {"kind":"row","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"card","parentIndex":0,"nodeIndex":3,"tone":"accent","value":"","valueOrigin":"none"},
            {"kind":"arrow","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"code","parentIndex":0,"nodeIndex":3,"tone":"positive","value":"","valueOrigin":"none"}
        ]);
        presentation.scenes[0].elements = serde_json::from_value(elements).unwrap();
        assert!(validate(&presentation, &nodes, &claims).is_err());
        for (before, after) in [("PFR0", "font bytes"), ("  Mira  ", "Mira"), ("a", "A")] {
            for (element, value) in [(1, before), (3, after)] {
                let item = &mut presentation.scenes[0].elements[element];
                item.value = value.into();
                item.value_origin = ValueOrigin::Illustration;
            }
            validate(&presentation, &nodes, &claims).unwrap();
        }
        // Distinct roles can legitimately share the same source and value.
        nodes[0] = nodes[3].clone();
        nodes[0].label = "Result role".into();
        presentation.scenes[0].elements[3].node_index = Some(0);
        presentation.scenes[0].elements[3].value = "a".into();
        validate(&presentation, &nodes, &claims).unwrap();
        // A comparison without a transformation arrow is outside this rule.
        presentation.scenes[0].elements[3].node_index = Some(3);
        presentation.scenes[0].elements[2].kind = ElementKind::Divider;
        validate(&presentation, &nodes, &claims).unwrap();
        for scene in &mut presentation.scenes {
            scene.mode = SceneMode::Graph;
            scene.elements.clear();
        }
        validate(&presentation, &nodes, &claims).unwrap();
        assert_eq!(
            proposal_schema()["properties"]["schemaVersion"]["enum"],
            json!([3])
        );
    }

    #[test]
    fn repetition_rule_compares_nested_visible_content_not_layout_metadata() {
        let (mut nodes, _, mut presentation) = fixture();
        nodes[0] = nodes[3].clone();
        nodes[0].node_id.push_str("-different-id");
        presentation.scenes[0].elements = serde_json::from_value(json!([
            {"kind":"row","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"column","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"code","parentIndex":1,"nodeIndex":3,"tone":"accent","value":"same value","valueOrigin":"illustration"},
            {"kind":"arrow","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"panel","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":4,"nodeIndex":0,"tone":"warning","value":"same value","valueOrigin":"illustration"}
        ]))
        .unwrap();

        let labels: Vec<_> = nodes.iter().map(|node| node.label.as_str()).collect();
        assert_eq!(repetition_positions(&presentation, &labels), vec![(0, 3)]);

        presentation.scenes[0].elements[5].value = "changed value".into();
        assert!(repetition_positions(&presentation, &labels).is_empty());

        presentation.scenes[0].elements[5].value = "same value".into();
        nodes[0].label = "A different visible role".into();
        let labels: Vec<_> = nodes.iter().map(|node| node.label.as_str()).collect();
        assert!(repetition_positions(&presentation, &labels).is_empty());
    }

    #[test]
    fn repetition_rule_ignores_graph_scenes_and_comparisons_without_arrows() {
        let (nodes, _, mut presentation) = fixture();
        let elements = json!([
            {"kind":"row","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"code","parentIndex":0,"nodeIndex":3,"tone":"accent","value":"same","valueOrigin":"illustration"},
            {"kind":"arrow","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":0,"nodeIndex":3,"tone":"positive","value":"same","valueOrigin":"illustration"}
        ]);
        presentation.scenes[0].elements = serde_json::from_value(elements).unwrap();
        let labels: Vec<_> = nodes.iter().map(|node| node.label.as_str()).collect();
        assert_eq!(repetition_positions(&presentation, &labels), vec![(0, 2)]);

        presentation.scenes[0].elements[2].kind = ElementKind::Divider;
        assert!(repetition_positions(&presentation, &labels).is_empty());

        presentation.scenes[0].mode = SceneMode::Graph;
        assert!(repetition_positions(&presentation, &labels).is_empty());
    }

    #[test]
    fn content_cannot_borrow_a_different_claim_or_fabricate_a_source_value() {
        let (nodes, claims, mut presentation) = fixture();
        presentation.scenes[0].elements[0].node_index = Some(8);
        assert!(
            validate(&presentation, &nodes, &claims)
                .unwrap_err()
                .contains("own claim")
        );
        presentation.scenes[0].elements[0].node_index = Some(3);
        presentation.scenes[0].elements[0].value = "not an actual input".into();
        assert!(validate(&presentation, &nodes, &claims).is_err());
        presentation.scenes[0].elements[0].value_origin = ValueOrigin::Illustration;
        assert!(validate(&presentation, &nodes, &claims).is_ok());
    }

    #[test]
    fn malformed_trees_and_unbounded_graphics_fail_as_a_whole() {
        let (nodes, claims, presentation) = fixture();
        for (kind, value) in [
            (ElementKind::Matrix, "01\n1"),
            (ElementKind::Matrix, "aa"),
            (ElementKind::Table, "a|b\nc"),
        ] {
            let mut invalid = presentation.clone();
            let item = &mut invalid.scenes[0].elements[0];
            item.kind = kind;
            item.value = value.into();
            item.value_origin = ValueOrigin::Illustration;
            assert!(validate(&invalid, &nodes, &claims).is_err());
        }
        let mut invalid = presentation.clone();
        invalid.scenes[0].elements[0].parent_index = Some(0);
        assert!(validate(&invalid, &nodes, &claims).is_err());
        invalid = presentation.clone();
        invalid.scenes[0].elements[0].node_index = Some(999);
        assert!(validate(&invalid, &nodes, &claims).is_err());
        invalid = presentation.clone();
        invalid.scenes[0].claim_index = 1;
        assert!(validate(&invalid, &nodes, &claims).is_err());
        invalid = presentation;
        invalid.scenes[0].elements[0].value = "é".repeat(513);
        assert!(validate(&invalid, &nodes, &claims).is_err());
    }

    #[test]
    fn raw_markup_and_executable_fields_are_not_in_the_contract() {
        let (_, _, presentation) = fixture();
        for field in ["html", "css", "script", "style", "url", "onClick"] {
            let mut value = serde_json::to_value(&presentation).unwrap();
            value["scenes"][0]["elements"][0][field] = json!("untrusted");
            assert!(
                serde_json::from_value::<ScenePresentation>(value).is_err(),
                "{field}"
            );
        }
    }
}
