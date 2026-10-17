//! Bounded AI-authored HTML compositions. Data is interpreted by app-owned
//! components; no generated markup, CSS, scripts, URLs or event handlers run.

use crate::chapters::{ChapterClaim, VisualNode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

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
    #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "present_narration")]
    pub narration: Option<String>,
    pub mode: SceneMode,
    pub elements: Vec<SceneElement>,
}

// Absence preserves historical scene bytes; an explicit null is never a script.
fn present_narration<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    String::deserialize(d).map(Some)
}

/// Authored speech stays separate from exact implementation claims. This checks
/// coverage/identity and bounds, not whether a paraphrase preserves semantics.
pub(crate) fn narration(
    presentation: &ScenePresentation,
    claims: &[ChapterClaim],
) -> Result<Option<Vec<String>>, String> {
    if presentation.schema_version != 3 {
        if presentation.scenes.iter().any(|s| s.narration.is_some()) {
            return Err("Historical scenes cannot contain separate narration.".into());
        }
        return Ok(None);
    }
    if presentation.scenes.len() != claims.len() || claims.is_empty() || claims.len() > 20 {
        return Err("Scene narration needs one ordered scene for every claim.".into());
    }
    let mut scripts = Vec::with_capacity(claims.len());
    for (index, (scene, claim)) in presentation.scenes.iter().zip(claims).enumerate() {
        let text = scene.narration.as_deref().unwrap_or("");
        if scene.claim_index != index || claim.anchors.is_empty() || text.trim().is_empty() || text.len() > 4_000 {
            return Err("Each scene needs bounded narration and its own consecutive anchored claim.".into());
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
    Ok(Some(scripts))
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
    if !matches!(presentation.schema_version, 1 | 2 | 3)
        || presentation.scenes.len() != claims.len()
        || presentation.scenes.is_empty()
        || presentation.scenes.len() > 20
    {
        return Err(
            "A scene presentation needs version 1, 2 or 3 and one scene for each claim, in claim order."
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
                narration: None,
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
    fn authored_speech_covers_every_claim_without_requiring_code_names_in_audio() {
        let (nodes, mut claims, mut presentation) = fixture();
        presentation.schema_version = 3;
        for scene in &mut presentation.scenes {
            scene.narration = Some("For example, the input passes this check; an invalid input keeps its original error.".into());
        }
        claims[0].category = crate::chapters::ClaimCategory::Inference {
            visible_label: "Inference".into(), basis: "Based on the captured branch.".into(),
        };
        validate(&presentation, &nodes, &claims).unwrap();
        let scripts = narration(&presentation, &claims).unwrap().unwrap();
        assert_eq!(scripts.len(), claims.len());
        assert!(scripts[0].starts_with("Inference: For example"));
        assert!(scripts.iter().skip(1).all(|s| !s.starts_with("Inference:")));
        let encoded = serde_json::to_value(&presentation).unwrap();
        assert_eq!(serde_json::from_value::<ScenePresentation>(encoded).unwrap(), presentation);
        for invalid in [None, Some(String::new()), Some("é".repeat(2_001))] {
            let mut broken = presentation.clone();
            broken.scenes[0].narration = invalid;
            assert!(validate(&broken, &nodes, &claims).is_err());
        }
        let mut missing = presentation.clone(); missing.scenes.pop();
        assert!(narration(&missing, &claims).is_err());
        let mut wrong = presentation.clone(); wrong.scenes[0].claim_index = 1;
        assert!(narration(&wrong, &claims).is_err());
        for scene in &mut presentation.scenes { scene.narration = Some("é".repeat(600)); }
        assert!(narration(&presentation, &claims).is_err());
    }

    #[test]
    fn legacy_scene_bytes_stay_unchanged_and_cannot_gain_a_new_script() {
        let (nodes, claims, mut presentation) = fixture();
        for version in [1, 2] {
            presentation.schema_version = version;
            let encoded = serde_json::to_string(&presentation).unwrap();
            assert!(!encoded.contains("narration"));
            let decoded: ScenePresentation = serde_json::from_str(&encoded).unwrap();
            assert_eq!(serde_json::to_string(&decoded).unwrap(), encoded);
            assert_eq!(narration(&presentation, &claims).unwrap(), None);
            let mut changed = presentation.clone();
            changed.scenes[0].narration = Some("This is not a historical script.".into());
            assert!(validate(&changed, &nodes, &claims).is_err());
            let mut explicit_null = serde_json::to_value(&presentation).unwrap();
            explicit_null["scenes"][0]["narration"] = Value::Null;
            assert!(serde_json::from_value::<ScenePresentation>(explicit_null).is_err());
        }
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
            json!([3])
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
