//! Bounded AI-authored HTML compositions. Data is interpreted by app-owned
//! components; no generated markup, CSS, scripts, URLs or event handlers run.

use crate::chapters::{ChapterClaim, VisualNode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

pub(crate) const MAX_FOCUSED_GRAPH_NODES: usize = 8;
pub(crate) const MAX_SCENE_NARRATION_BYTES: usize = 520;

/// One audio scene must stay listenable even when a generator ignores its
/// prompt. The limit leaves room to explain a real decision and its outcome;
/// it does not prove that the narration is useful or accurately paced.
pub(crate) fn scene_narration_is_concise(text: &str) -> bool {
    !text.trim().is_empty()
        && text.len() <= MAX_SCENE_NARRATION_BYTES
        && sentence_ending_count(text) <= 3
}

fn sentence_ending_count(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut count = 0;
    let mut index = 0;
    while index < chars.len() {
        if !is_sentence_ending(chars[index]) {
            index += 1;
            continue;
        }

        let mut has_cjk_ending = matches!(chars[index], '。' | '！' | '？');
        while index + 1 < chars.len() && is_sentence_ending(chars[index + 1]) {
            index += 1;
            has_cjk_ending |= matches!(chars[index], '。' | '！' | '？');
        }
        let previous = index.checked_sub(1).and_then(|at| chars.get(at)).copied();
        let mut after = index + 1;
        while chars.get(after).is_some_and(|character| {
            matches!(
                *character,
                '\'' | '"' | '”' | '’' | ')' | ']' | '}' | '»' | '」' | '』'
            )
        }) {
            after += 1;
        }
        let next = chars.get(after).copied();
        let decimal_point = chars[index] == '.'
            && previous.is_some_and(|character| character.is_ascii_digit())
            && next.is_some_and(|character| character.is_ascii_digit());
        if !decimal_point && (has_cjk_ending || next.is_none_or(char::is_whitespace)) {
            count += 1;
        }
        index = after;
    }
    count
}

fn is_sentence_ending(character: char) -> bool {
    matches!(character, '.' | '!' | '?' | '。' | '！' | '？')
}

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
pub(crate) use quality::{
    repeated_selector_outcome_positions, repetition_positions, selector_multiple_outcome_positions,
};

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

/// A first-match dispatch must keep candidate order visible. Independent
/// validation guards do not qualify: they are separate claims/scenes. This is
/// a narrow wording check, not semantic proof.
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
        "priority order",
        "priority chain",
        "precedence chain",
        "candidate order",
        "candidate dispatch",
        "tries the next candidate",
        "miss tries the next candidate",
        "next candidate",
        "first candidate",
        "first handler",
        "signature probe",
        "first matching signature",
    ]
    .iter()
    .any(|signal| text.contains(signal))
}

pub(crate) fn has_ordered_selector_root(elements: &[SceneElement]) -> bool {
    elements
        .first()
        .is_some_and(|root| root.kind == ElementKind::Selector)
}

/// Apply the current authored-layout rules to new content and every replayed
/// revision so a saved scene cannot retain an obsolete presentation policy.
pub(crate) fn validate_semantic_layouts(
    presentation: &ScenePresentation,
    claims: &[ChapterClaim],
    nodes: &[VisualNode],
) -> Result<(), String> {
    for (scene, claim) in presentation.scenes.iter().zip(claims) {
        if requires_ordered_selector(&claim.prose, &scene.title, &scene.narration)
            && (scene.mode != SceneMode::Composition || !has_ordered_selector_root(&scene.elements))
        {
            return Err(
                "Source-ordered guards need a selector composition at the root so each guard, outcome and fall-through stays visible; a LIFO stack reverses the priority."
                    .into(),
            );
        }
        if scene.mode == SceneMode::Composition
            && has_ordered_selector_root(&scene.elements)
            && !requires_ordered_selector(&claim.prose, &scene.title, &scene.narration)
        {
            return Err(
                "A selector implies first-match dispatch among competing candidates, where a miss tries the next candidate. Use a branch for one guard or split independent validation checks into separate scenes."
                    .into(),
            );
        }
    }
    let labels: Vec<_> = nodes.iter().map(|node| node.label.as_str()).collect();
    if !repeated_selector_outcome_positions(presentation, &labels).is_empty() {
        return Err(
            "A selector must label each branch outcome by its specific guard and result, rather than repeating a shared mechanism label."
                .into(),
        );
    }
    if !selector_multiple_outcome_positions(presentation).is_empty() {
        return Err(
            "Each selector branch needs one guard and its one matching outcome; the rendered miss path already continues to the next candidate. Use a branch for one guard with multiple outcomes, or split independent checks into separate scenes."
                .into(),
        );
    }
    Ok(())
}

/// Detect compact all-caps/code-format tokens in newly authored speech. Exact
/// identifiers remain available in the adjacent claim and source link. This is
/// a narrow TTS-readability check, not a semantic or pronunciation guarantee.
pub(crate) fn has_compact_spoken_code_token(text: &str) -> bool {
    text.split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|token| !token.is_empty())
        .any(|token| {
            let letters = token
                .chars()
                .filter(|character| character.is_ascii_alphabetic())
                .collect::<Vec<_>>();
            letters.len() >= 3
                && letters
                    .iter()
                    .all(|character| character.is_ascii_uppercase())
        })
}

/// Authored speech stays separate from exact implementation claims and follows
/// one concise, code-aware policy both when saved and when generated.
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
            || !scene_narration_is_concise(text)
        {
            return Err("Each scene needs at most three short natural sentences, no longer than 520 UTF-8 bytes, and its own consecutive anchored claim.".into());
        }
        if has_compact_spoken_code_token(text) {
            return Err(
                "Narration contains a compact code abbreviation or format token. Keep exact names in the visible claim and source link; speak what the token does instead.".into(),
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
    PointFlow,
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
                | Self::PointFlow
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
    validate_semantic_layouts(presentation, claims, nodes)?;
    let labels: Vec<_> = nodes.iter().map(|n| n.label.as_str()).collect();
    if !repetition_positions(presentation, &labels).is_empty() {
        return Err("A transformation must show distinct input/result labels or values.".into());
    }
    Ok(())
}

/// Apply additional visible-content requirements to fresh AI proposals.
/// Saved chapters use `validate` above so tightened generation policy does not
/// invalidate already-persisted reader content.
pub(crate) fn validate_generated(
    presentation: &ScenePresentation,
    nodes: &[VisualNode],
    claims: &[ChapterClaim],
) -> Result<(), String> {
    validate(presentation, nodes, claims)?;
    for scene in &presentation.scenes {
        if scene.mode == SceneMode::Composition
            && !scene
                .elements
                .iter()
                .any(|element| !element.kind.container() && !element.kind.decorative())
        {
            return Err(
                "A composition scene needs at least one visible content node linked to its claim. No visual was saved.".into(),
            );
        }
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
                "narration":{"type":"string","minLength":1,"maxLength":520},
                "mode":{"type":"string","enum":["composition","graph"]},
                "elements":{"type":"array","maxItems":48,"items":{
                    "type":"object","additionalProperties":false,
                    "required":["kind","parentIndex","nodeIndex","tone","value","valueOrigin"],"properties":{
                        "kind":{"type":"string","enum":["row","column","grid","panel","compare","stack","timeline","lanes","branch","beforeAfter","transform","pointFlow","tree","queue","selector","card","token","code","metric","bytes","matrix","table","text","arrow","divider"]},
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
    fn ordered_candidate_story_rejects_a_reversed_stack_and_accepts_selector_layout() {
        let (mut nodes, mut claims, mut presentation) = fixture();
        claims[0].prose = "Parser signatures are competing candidates checked in source order; the first matching parser handles the input.".into();
        let claim_anchors: Vec<_> = claims[0]
            .anchors
            .iter()
            .map(|anchor| anchor.anchor_id.clone())
            .collect();
        for node in &mut nodes {
            node.evidence_anchor_ids = claim_anchors.clone();
        }
        nodes[1].label = "First candidate decoder".into();
        nodes[3].label = "Default decoder".into();
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
        presentation.scenes[0].elements[2].node_index = Some(0);
        presentation.scenes[0].elements[3].node_index = Some(1);
        presentation.scenes[0].elements[5].node_index = Some(2);
        presentation.scenes[0].elements[6].node_index = Some(3);
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
        assert!(error.contains("selector composition at the root"));
    }

    #[test]
    fn current_independent_guards_do_not_require_an_ordered_selector() {
        let (nodes, claims, mut presentation) = fixture();
        let mut claims = vec![claims[0].clone()];
        presentation.scenes.truncate(1);
        claims[0].prose = "An invalid face index returns an error; a valid index continues into the table-bounds check.".into();
        presentation.scenes[0].mode = SceneMode::Graph;
        presentation.scenes[0].elements.clear();

        // Independent validation gates are not first-match candidate dispatch.
        assert!(!requires_ordered_selector(
            &claims[0].prose,
            "Validate the face index",
            "If conversion fails, return; otherwise continue to the next independent validation."
        ));
        assert!(!requires_ordered_selector(
            "Several early returns validate independent inputs.",
            "Check each input",
            "A miss falls through to the next validation check."
        ));
        validate(&presentation, &nodes, &claims).unwrap();
        validate_semantic_layouts(&presentation, &claims, &nodes).unwrap();

        claims[0].prose = "Parser signatures are tried in source order; the first matching candidate wins and a miss tries the next candidate.".into();
        assert!(requires_ordered_selector(
            &claims[0].prose,
            "Choose a parser",
            ""
        ));
        assert!(
            validate_semantic_layouts(&presentation, &claims, &nodes)
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
        claims[0].prose = "An invalid face index returns an error; a valid index continues into the table-bounds check.".into();
        assert!(
            validate_semantic_layouts(&presentation, &claims, &nodes)
                .unwrap_err()
                .contains("A selector implies first-match dispatch")
        );
        claims[0].prose = "Parser signatures are tried in source order; the first matching candidate wins and a miss tries the next candidate.".into();
        validate_semantic_layouts(&presentation, &claims, &nodes).unwrap();
    }

    #[test]
    fn current_selector_branches_have_one_matching_outcome() {
        let (mut nodes, mut claims, mut presentation) = fixture();
        claims.truncate(1);
        presentation.scenes.truncate(1);
        presentation.scenes[0].mode = SceneMode::Composition;
        claims[0].prose =
            "The first matching candidate wins; a miss tries the next candidate.".into();
        for (node, label) in
            nodes
                .iter_mut()
                .zip(["Guard one", "Outcome one", "Guard two", "Outcome two"])
        {
            node.label = label.into();
        }
        let mut extra = nodes[0].clone();
        extra.node_id = "extra-outcome".into();
        extra.label = "Second outcome".into();
        nodes.push(extra);
        let group = |kind, parent| SceneElement {
            kind,
            parent_index: parent,
            node_index: None,
            tone: ElementTone::Neutral,
            value: String::new(),
            value_origin: ValueOrigin::None,
        };
        let leaf = |kind, parent| SceneElement {
            kind,
            parent_index: Some(parent),
            node_index: Some(0),
            tone: ElementTone::Neutral,
            value: String::new(),
            value_origin: ValueOrigin::None,
        };
        presentation.scenes[0].elements = vec![group(ElementKind::Selector, None)];
        presentation.scenes[0].elements.extend([
            group(ElementKind::Branch, Some(0)),
            leaf(ElementKind::Text, 1),
            leaf(ElementKind::Token, 1),
            group(ElementKind::Branch, Some(0)),
            leaf(ElementKind::Text, 4),
            leaf(ElementKind::Token, 4),
        ]);
        presentation.scenes[0].elements[2].node_index = Some(0);
        presentation.scenes[0].elements[3].node_index = Some(1);
        presentation.scenes[0].elements[5].node_index = Some(2);
        presentation.scenes[0].elements[6].node_index = Some(3);
        assert!(selector_multiple_outcome_positions(&presentation).is_empty());

        let mut extra_outcome = leaf(ElementKind::Text, 1);
        extra_outcome.node_index = Some(4);
        presentation.scenes[0].elements.push(extra_outcome);
        assert_eq!(
            selector_multiple_outcome_positions(&presentation),
            vec![(0, 1)]
        );
        assert!(
            validate_semantic_layouts(&presentation, &claims, &nodes)
                .unwrap_err()
                .contains("one matching outcome")
        );
    }

    #[test]
    fn current_selector_scenes_use_distinct_behavior_specific_outcome_labels() {
        let (mut nodes, mut claims, mut presentation) = fixture();
        presentation.scenes.truncate(1);
        claims.truncate(1);
        claims[0].prose =
            "The first matching candidate wins and a miss tries the next candidate.".into();
        nodes[0].label = "Public format dispatch".into();
        nodes[1].label = " public   FORMAT dispatch ".into();
        for node in nodes.iter_mut().take(4) {
            node.evidence_anchor_ids = claims[0]
                .anchors
                .iter()
                .map(|anchor| anchor.anchor_id.clone())
                .collect();
        }
        presentation.scenes[0].mode = SceneMode::Composition;
        presentation.scenes[0].elements = serde_json::from_value(json!([
            {"kind":"selector","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"branch","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":1,"nodeIndex":2,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":1,"nodeIndex":0,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"branch","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":4,"nodeIndex":3,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":4,"nodeIndex":1,"tone":"neutral","value":"","valueOrigin":"none"}
        ]))
        .unwrap();
        let node_labels: Vec<_> = nodes.iter().map(|node| node.label.as_str()).collect();
        let labels = repeated_selector_outcome_positions(&presentation, &node_labels);
        assert_eq!(labels, vec![(0, 6)]);
        assert!(validate(&presentation, &nodes, &claims).is_err());
        assert!(
            validate_semantic_layouts(&presentation, &claims, &nodes)
                .unwrap_err()
                .contains("specific guard and result")
        );

        nodes[1].label = "PFR face parser".into();
        let node_labels: Vec<_> = nodes.iter().map(|node| node.label.as_str()).collect();
        assert!(repeated_selector_outcome_positions(&presentation, &node_labels).is_empty());
        validate_semantic_layouts(&presentation, &claims, &nodes).unwrap();
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
        let mut claims = section.claims.clone();
        for claim in &mut claims {
            claim.prose =
                "The implementation transforms the selected input and returns its result."
                    .into();
        }
        let mut scenes: Vec<_> = claims
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
            claims,
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
    fn scene_narration_allows_a_story_beat_without_splitting_decimals_or_cjk_sentences() {
        assert!(scene_narration_is_concise(
            "A point enters the transform. Its outline position is preserved."
        ));
        assert!(scene_narration_is_concise(
            "Scale 1.5 keeps the curve. Then points are placed."
        ));
        assert!(scene_narration_is_concise("点进入变换。轮廓标记保持不变！"));
        assert!(!scene_narration_is_concise(
            "First outcome. Second outcome. Third outcome. Final result."
        ));
        assert!(!scene_narration_is_concise(
            &"a".repeat(MAX_SCENE_NARRATION_BYTES + 1)
        ));
        assert!(!scene_narration_is_concise("  \n  "));
        assert_eq!(
            proposal_schema()["properties"]["scenes"]["items"]["properties"]["narration"]["maxLength"],
            MAX_SCENE_NARRATION_BYTES
        );
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
        let mut long_legacy_speech = presentation.clone();
        long_legacy_speech.scenes[0].narration =
            "First outcome. Second outcome. Third outcome.".into();
        assert!(validate(&long_legacy_speech, &nodes, &claims).is_err());
        assert!(narration(&long_legacy_speech, &claims).is_err());
        presentation.scenes[0].narration = "é".repeat(600);
        assert!(validate(&presentation, &nodes, &claims).is_err());
        assert!(narration(&presentation, &claims).is_err());
    }

    #[test]
    fn current_spoken_script_flags_compact_code_tokens_but_allows_role_language() {
        for script in [
            "The SFNT marker selects the parser.",
            "PFR0 goes to its dedicated loader.",
            "The C ABI returns an error when the handle is absent.",
            "UTF-8 text follows the byte path.",
        ] {
            assert!(
                has_compact_spoken_code_token(script),
                "expected a compact code token in {script:?}"
            );
        }
        for script in [
            "The font-table signature selects the parser.",
            "The dedicated loader handles that format.",
            "The exported interface returns an error when the handle is absent.",
            "The input text follows the byte path.",
            "Rust reads the file and returns a bitmap.",
        ] {
            assert!(
                !has_compact_spoken_code_token(script),
                "unexpected code token in {script:?}"
            );
        }
    }

    #[test]
    fn current_scene_admission_rejects_compact_spoken_tokens() {
        let (nodes, claims, mut presentation) = fixture();
        presentation.scenes[0].narration =
            "The SFNT marker selects the parser; its role stays visible in the source.".into();
        let error = validate(&presentation, &nodes, &claims).unwrap_err();
        assert!(error.contains("compact code abbreviation or format token"));
        assert!(!error.contains("SFNT"));
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
