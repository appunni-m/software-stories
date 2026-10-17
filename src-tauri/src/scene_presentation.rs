//! Bounded AI-authored HTML compositions. Data is interpreted by app-owned
//! components; no generated markup, CSS, scripts, URLs or event handlers run.

use crate::chapters::{ChapterClaim, VisualNode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

mod quality;
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
    pub mode: SceneMode,
    pub elements: Vec<SceneElement>,
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
            Self::Row | Self::Column | Self::Grid | Self::Panel | Self::Compare | Self::Stack
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
    Illustration,
}

/// Same checks apply at generation and when validating an edited saved chapter.
pub fn validate(
    presentation: &ScenePresentation,
    nodes: &[VisualNode],
    claims: &[ChapterClaim],
) -> Result<(), String> {
    if !matches!(presentation.schema_version, 1 | 2)
        || presentation.scenes.len() != claims.len()
        || presentation.scenes.is_empty()
        || presentation.scenes.len() > 20
    {
        return Err(
            "A scene presentation needs version 1 or 2 and one scene for each claim, in claim order."
                .into(),
        );
    }
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
        if scene.elements.is_empty() || scene.elements.len() > 48 {
            return Err("A composition needs 1–48 elements.".into());
        }
        let mut depths = Vec::new();
        let mut child_counts = vec![0; scene.elements.len()];
        let mut leaves = 0;
        for (i, element) in scene.elements.iter().enumerate() {
            let depth =
                match element.parent_index {
                    None if i == 0 => 0,
                    Some(parent) if parent < i && scene.elements[parent].kind.container() => {
                        child_counts[parent] += 1;
                        if child_counts[parent] > 12 {
                            return Err(
                                "A composition group may contain at most twelve children.".into()
                            );
                        }
                        depths[parent] + 1
                    }
                    _ => return Err(
                        "Composition elements need one root and an earlier container parentIndex."
                            .into(),
                    ),
                };
            if depth > 6 {
                return Err("Composition nesting may be at most six levels.".into());
            }
            depths.push(depth);
            if element.kind.container() || element.kind.decorative() {
                if element.node_index.is_some()
                    || !element.value.is_empty()
                    || element.value_origin != ValueOrigin::None
                {
                    return Err("Layout and decorative elements cannot add uncited content.".into());
                }
                continue;
            }
            leaves += 1;
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
                ValueOrigin::Illustration if !element.value.trim().is_empty() => {},
                _ => return Err("A value needs an exact source substring or an explicitly labeled illustration; empty values use origin none.".into()),
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
        if leaves == 0
            || leaves > 12
            || scene
                .elements
                .iter()
                .enumerate()
                .any(|(i, e)| e.kind.container() && child_counts[i] == 0)
        {
            return Err(
                "A composition needs 1–12 cited content elements and no empty groups.".into(),
            );
        }
    }
    let labels: Vec<_> = nodes.iter().map(|n| n.label.as_str()).collect();
    if !repetition_positions(presentation, &labels).is_empty() {
        return Err("A transformation must show distinct input/result labels or values.".into());
    }
    Ok(())
}

/// Flat parent indices keep the model schema small and avoid recursive schemas.
pub(crate) fn proposal_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"required":["schemaVersion","scenes"],"properties":{
        "schemaVersion":{"type":"integer","enum":[2]},
        "scenes":{"type":"array","minItems":1,"maxItems":20,"items":{
            "type":"object","additionalProperties":false,"required":["claimIndex","title","mode","elements"],"properties":{
                "claimIndex":{"type":"integer","minimum":0,"maximum":19},
                "title":{"type":"string","minLength":1,"maxLength":120},
                "mode":{"type":"string","enum":["composition","graph"]},
                "elements":{"type":"array","maxItems":48,"items":{
                    "type":"object","additionalProperties":false,
                    "required":["kind","parentIndex","nodeIndex","tone","value","valueOrigin"],"properties":{
                        "kind":{"type":"string","enum":["row","column","grid","panel","compare","stack","card","token","code","metric","bytes","matrix","table","text","arrow","divider"]},
                        "parentIndex":{"type":["integer","null"],"minimum":0,"maximum":47},
                        "nodeIndex":{"type":["integer","null"],"minimum":0,"maximum":11},
                        "tone":{"type":"string","enum":["neutral","accent","muted","positive","warning"]},
                        "value":{"type":"string","maxLength":1024},
                        "valueOrigin":{"type":"string","enum":["none","source","illustration"]}
                    }}}
            }}}
    }})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_scene_policy_rejects_the_recorded_empty_self_transformation_but_keeps_legacy() {
        let artifact: Value = serde_json::from_str(include_str!(
            "../../docs/acceptance/artifacts/2026-10-17-fontdone-short-references-partial.json"
        ))
        .unwrap();
        let revision: crate::chapters::ChapterRevision = serde_json::from_value(
            artifact["answer"]["sections"][0]["preview"]["revision"].clone(),
        )
        .unwrap();
        let section = &revision.sections[0];
        let visual = section.visual.as_ref().unwrap().typed_visual().unwrap();
        let mut presentation = visual.presentation.clone().unwrap();
        // The original admitted guide is immutable and stays readable.
        assert_eq!(presentation.schema_version, 1);
        validate(&presentation, &visual.nodes, &section.claims).unwrap();
        presentation.schema_version = 2;
        assert!(validate(&presentation, &visual.nodes, &section.claims)
            .unwrap_err()
            .contains("distinct input/result labels or values"));
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
                schema_version: 1,
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
    fn scene_v2_requires_a_visible_transition_but_allows_real_state_comparisons() {
        let (mut nodes, claims, mut presentation) = fixture();
        presentation.schema_version = 2;
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
        presentation.schema_version = 3;
        assert!(validate(&presentation, &nodes, &claims).is_err());
        assert_eq!(
            proposal_schema()["properties"]["schemaVersion"]["enum"],
            json!([2])
        );
    }

    #[test]
    fn content_cannot_borrow_a_different_claim_or_fabricate_a_source_value() {
        let (nodes, claims, mut presentation) = fixture();
        presentation.scenes[0].elements[0].node_index = Some(8);
        assert!(validate(&presentation, &nodes, &claims)
            .unwrap_err()
            .contains("own claim"));
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
