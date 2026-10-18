//! Positions and fixed tree codes only; no source, labels or model strings.

use crate::scene_presentation::{tree, SceneMode, ScenePresentation};
use serde_json::{json, Value};
use std::collections::HashSet;

pub(super) fn append(response: &str, issues: &mut Vec<Value>) {
    let Ok(root) = crate::strict_json::from_slice(response.as_bytes()) else {
        return;
    };
    let sections: Vec<(&Value, String)> = match root.get("sections").and_then(Value::as_array) {
        Some(sections) => sections
            .iter()
            .enumerate()
            .map(|(i, section)| (section, format!("/sections/{i}")))
            .collect(),
        None => vec![(&root, String::new())],
    };
    for (section, prefix) in sections {
        if issues.len() >= super::MAX_ISSUES {
            return;
        }
        let Some(value) = section.get("visual").and_then(|v| v.get("presentation")) else {
            continue;
        };
        // Unknown/future shapes are left to full admission, never interpreted
        // under a known grammar merely to obtain more repair opportunities.
        let Ok(presentation) = serde_json::from_value::<ScenePresentation>(value.clone()) else {
            continue;
        };
        if presentation.schema_version != 3 || presentation.scenes.len() > 20 {
            continue;
        }
        for (scene, element) in crate::scene_presentation::foreign_claim_positions(section) {
            if issues.len() >= super::MAX_ISSUES { return; }
            issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{scene}/elements/{element}/nodeIndex"),"sceneProblem":"sceneEvidence"}));
        }
        for (index, scene) in presentation.scenes.iter().enumerate() {
            if scene.narration.trim().is_empty() || scene.narration.len() > 4_000 {
                if issues.len() >= super::MAX_ISSUES { return; }
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{index}/narration"),"sceneProblem":"narrationRequired"}));
            }
            if scene.mode == SceneMode::Graph
                && graph_scene_node_count(section, scene.claim_index).is_some_and(|count| {
                    count > crate::scene_presentation::MAX_FOCUSED_GRAPH_NODES
                })
            {
                if issues.len() >= super::MAX_ISSUES {
                    return;
                }
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{index}/mode"),"sceneProblem":"graphDensity"}));
            }
            if scene.mode != SceneMode::Composition {
                continue;
            }
            let base = format!("{prefix}/visual/presentation/scenes/{index}/elements");
            for finding in tree::findings(&scene.elements) {
                if issues.len() >= super::MAX_ISSUES {
                    return;
                }
                let mut location = base.clone();
                if let Some(element) = finding.element {
                    location.push_str(&format!("/{element}"));
                }
                if !finding.field.is_empty() {
                    location.push_str(&format!("/{}", finding.field));
                }
                issues.push(json!({"location":location,"sceneProblem":finding.code}));
            }
        }
        if issues.len() < super::MAX_ISSUES
            && presentation.scenes.iter().all(|s| !s.narration.trim().is_empty() && s.narration.len() <= 4_000) {
            let total = presentation.scenes.iter().map(|s| s.narration.as_str())
                .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>().join(" ").len();
            if !(20..=4_000).contains(&total) {
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes"),"sceneProblem":"narrationLength"}));
            }
        }
    }
}

/// Match the reader's focused graph: claim-linked nodes plus one edge of
/// context. A scene without a linked node shows the entire graph. Return None
/// for an unknown proposal shape and leave it to full admission.
pub(super) fn graph_scene_node_count(section: &Value, claim_index: usize) -> Option<usize> {
    let visual = section.get("visual")?;
    let nodes = visual.get("nodes")?.as_array()?;
    let edges = visual.get("edges")?.as_array()?;
    if nodes.len() > 12 || edges.len() > 20 {
        return None;
    }

    let mut focused = HashSet::new();
    let claim_index = u64::try_from(claim_index).ok()?;
    for (index, node) in nodes.iter().enumerate() {
        let references = node.get("evidence")?.as_array()?;
        if references.iter().any(|reference| {
            reference.get("claimIndex").and_then(Value::as_u64) == Some(claim_index)
        }) {
            focused.insert(index);
        }
    }
    if focused.is_empty() {
        return Some(nodes.len());
    }

    let mut valid_edges = Vec::with_capacity(edges.len());
    for edge in edges {
        let from = usize::try_from(edge.get("fromNodeIndex")?.as_u64()?).ok()?;
        let to = usize::try_from(edge.get("toNodeIndex")?.as_u64()?).ok()?;
        if from >= nodes.len() || to >= nodes.len() {
            return None;
        }
        valid_edges.push((from, to));
    }
    Some(crate::scene_presentation::visible_graph_node_count(
        nodes.len(),
        &focused,
        valid_edges,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::visual_scenes::tests::composed_proposal;

    fn graph_section(active: usize, adjacent: usize, total: usize) -> Value {
        let mut section = composed_proposal(3)["sections"][0].clone();
        let nodes = (0..total)
            .map(|index| {
                let claim_index = if index < active { 0 } else { 1 };
                json!({"label":format!("private-node-label-{index}"),"detail":"",
                    "evidence":[{"claimIndex":claim_index,"evidenceIndex":0}]})
            })
            .collect::<Vec<_>>();
        let edges = (active..active + adjacent)
            .map(|index| {
                json!({"fromNodeIndex":0,"toNodeIndex":index,"label":"context edge",
                    "evidence":[{"claimIndex":0,"evidenceIndex":0}]})
            })
            .collect::<Vec<_>>();
        section["visual"]["nodes"] = json!(nodes);
        section["visual"]["edges"] = json!(edges);
        section["visual"]["presentation"]["scenes"][0]["mode"] = json!("graph");
        section["visual"]["presentation"]["scenes"][0]["elements"] = json!([]);
        section
    }

    #[test]
    fn focused_graph_density_counts_claim_nodes_and_one_hop_context() {
        let eight = graph_section(1, 7, 9);
        let nine = graph_section(1, 8, 9);
        assert_eq!(graph_scene_node_count(&eight, 0), Some(8));
        assert_eq!(graph_scene_node_count(&nine, 0), Some(9));
    }

    #[test]
    fn graph_without_focus_uses_the_full_map_count_and_skips_unknown_shapes() {
        let crowded = graph_section(0, 0, 9);
        let readable = graph_section(0, 0, 8);
        assert_eq!(graph_scene_node_count(&crowded, 0), Some(9));
        assert_eq!(graph_scene_node_count(&readable, 0), Some(8));

        let mut unknown = graph_section(1, 1, 9);
        unknown["visual"]["edges"][0] = json!({"fromNodeIndex":"unknown"});
        assert_eq!(graph_scene_node_count(&unknown, 0), None);
    }

    #[test]
    fn crowded_graph_scene_gets_a_positional_diagnostic_without_node_text() {
        let mut proposal = composed_proposal(3);
        let nodes = (0..9)
            .map(|index| json!({"label":format!("private-node-label-{index}"),
                "detail":"secret detail","evidence":[{"claimIndex":0,"evidenceIndex":0}]}))
            .collect::<Vec<_>>();
        proposal["sections"][0]["visual"]["nodes"] = json!(nodes);
        proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["mode"] = json!("graph");
        proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["elements"] = json!([]);
        let findings = crate::claim_alignment::chapter_diagnostics(&proposal.to_string()).unwrap();
        let density = findings.iter().find(|finding| finding["sceneProblem"] == "graphDensity").unwrap();
        assert_eq!(density["location"], "/sections/0/visual/presentation/scenes/0/mode");
        let serialized = serde_json::to_string(&findings).unwrap();
        assert!(!serialized.contains("private-node-label"));
        assert!(!serialized.contains("secret detail"));
        let prompt = crate::claim_alignment::prompt_for_issues(findings).unwrap();
        assert!(prompt.contains("eight visible nodes or fewer"));
        assert!(prompt.contains("Preserve all guards, branches, failures, outcomes, evidence and real graph edges"));
        assert!(!prompt.contains("private-node-label"));
    }
}
