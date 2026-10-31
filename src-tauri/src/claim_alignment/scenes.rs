//! Positions and fixed tree codes only; no source, labels or model strings.

use crate::scene_presentation::{SceneMode, ScenePresentation, tree};
use serde_json::{Value, json};
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
        let claims = section.get("claims").and_then(Value::as_array);
        for (scene, element) in crate::scene_presentation::foreign_claim_positions(section) {
            if issues.len() >= super::MAX_ISSUES {
                return;
            }
            issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{scene}/elements/{element}/nodeIndex"),"sceneProblem":"sceneEvidence"}));
        }
        let branch_labels = section
            .get("visual")
            .and_then(|visual| visual.get("nodes"))
            .and_then(Value::as_array)
            .filter(|nodes| nodes.len() <= 12)
            .and_then(|nodes| {
                nodes
                    .iter()
                    .map(|node| node.get("label")?.as_str())
                    .collect::<Option<Vec<_>>>()
            });
        if let Some(labels) = branch_labels {
            for (scene, element) in crate::scene_presentation::repeated_selector_outcome_positions(
                &presentation,
                &labels,
            ) {
                if issues.len() >= super::MAX_ISSUES {
                    return;
                }
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{scene}/elements/{element}/nodeIndex"),"sceneProblem":"selectorOutcomeLabel"}));
            }
        }
        for (index, scene) in presentation.scenes.iter().enumerate() {
            if scene.narration.trim().is_empty() {
                if issues.len() >= super::MAX_ISSUES {
                    return;
                }
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{index}/narration"),"sceneProblem":"narrationRequired"}));
            } else if !crate::scene_presentation::scene_narration_is_concise(&scene.narration) {
                if issues.len() >= super::MAX_ISSUES {
                    return;
                }
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{index}/narration"),"sceneProblem":"narrationConciseness"}));
            }
            if scene.mode == SceneMode::Graph
                && graph_scene_is_overcrowded(section, scene.claim_index).unwrap_or(false)
            {
                if issues.len() >= super::MAX_ISSUES {
                    return;
                }
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{index}/mode"),"sceneProblem":"graphDensity"}));
            }
            let claim = claims
                .and_then(|items| items.get(scene.claim_index))
                .and_then(|claim| claim.get("prose"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if crate::scene_presentation::requires_ordered_selector(
                claim,
                &scene.title,
                &scene.narration,
            ) && (scene.mode != SceneMode::Composition
                || !crate::scene_presentation::has_ordered_selector_root(&scene.elements))
            {
                if issues.len() >= super::MAX_ISSUES {
                    return;
                }
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{index}/elements"),"sceneProblem":"selectorRequired"}));
            }
            if scene.mode == SceneMode::Composition
                && crate::scene_presentation::has_ordered_selector_root(&scene.elements)
                && !crate::scene_presentation::requires_ordered_selector(
                    claim,
                    &scene.title,
                    &scene.narration,
                )
            {
                if issues.len() >= super::MAX_ISSUES {
                    return;
                }
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{index}/elements"),"sceneProblem":"selectorChoice"}));
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
        for (scene, branch) in
            crate::scene_presentation::selector_multiple_outcome_positions(&presentation)
        {
            if issues.len() >= super::MAX_ISSUES {
                return;
            }
            issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{scene}/elements/{branch}"),"sceneProblem":"selectorShape"}));
        }
        if issues.len() < super::MAX_ISSUES
            && presentation
                .scenes
                .iter()
                .all(|s| crate::scene_presentation::scene_narration_is_concise(&s.narration))
        {
            let total = presentation
                .scenes
                .iter()
                .map(|s| s.narration.as_str())
                .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join(" ")
                .len();
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
    graph_scene_metrics(section, claim_index).map(|(count, _)| count)
}

pub(super) fn graph_scene_is_overcrowded(section: &Value, claim_index: usize) -> Option<bool> {
    graph_scene_metrics(section, claim_index)
        .map(|(count, focused)| focused && count > crate::scene_presentation::MAX_FOCUSED_GRAPH_NODES)
}

fn graph_scene_metrics(section: &Value, claim_index: usize) -> Option<(usize, bool)> {
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
        return Some((nodes.len(), false));
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
    Some((
        crate::scene_presentation::visible_graph_node_count(nodes.len(), &focused, valid_edges),
        true,
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
                    "evidence":[{"claimIndex":claim_index,"evidenceIndex":0}],"source":null})
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
            .map(|index| {
                json!({"label":format!("private-node-label-{index}"),
                "detail":"secret detail","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null})
            })
            .collect::<Vec<_>>();
        proposal["sections"][0]["visual"]["nodes"] = json!(nodes);
        proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["mode"] = json!("graph");
        proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["elements"] = json!([]);
        let findings = crate::claim_alignment::chapter_diagnostics(&proposal.to_string()).unwrap();
        let density = findings
            .iter()
            .find(|finding| finding["sceneProblem"] == "graphDensity")
            .unwrap();
        assert_eq!(
            density["location"],
            "/sections/0/visual/presentation/scenes/0/mode"
        );
        let serialized = serde_json::to_string(&findings).unwrap();
        assert!(!serialized.contains("private-node-label"));
        assert!(!serialized.contains("secret detail"));
        let prompt = crate::claim_alignment::prompt_for_issues(findings).unwrap();
        assert!(prompt.contains("eight visible nodes or fewer"));
        assert!(prompt.contains(
            "Preserve all guards, branches, failures, outcomes, evidence and real graph edges"
        ));
        assert!(!prompt.contains("private-node-label"));
    }

    #[test]
    fn long_scene_narration_gets_a_private_positional_repair() {
        let long_sentence = "a".repeat(crate::scene_presentation::MAX_SCENE_NARRATION_BYTES + 1);
        for narration in [
            "First outcome. Second outcome. Third outcome. Final result.",
            long_sentence.as_str(),
        ] {
            let mut proposal = composed_proposal(3);
            proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["narration"] =
                json!(narration);
            let findings =
                crate::claim_alignment::chapter_diagnostics(&proposal.to_string()).unwrap();
            let issue = findings
                .iter()
                .find(|finding| finding["sceneProblem"] == "narrationConciseness")
                .unwrap();
            assert_eq!(
                issue["location"],
                "/sections/0/visual/presentation/scenes/0/narration"
            );
            let serialized = serde_json::to_string(&findings).unwrap();
            assert!(!serialized.contains(narration));
            let prompt = crate::claim_alignment::prompt_for_issues(findings).unwrap();
            assert!(prompt.contains("narrationConciseness finding"));
            assert!(prompt.contains("three short sentences or 520 UTF-8 bytes"));
            assert!(!prompt.contains(narration));
        }
    }

    #[test]
    fn first_match_graphs_and_compositions_get_selector_diagnostics_without_leaking_text() {
        let mut proposal = composed_proposal(3);
        let scene = &mut proposal["sections"][0]["visual"]["presentation"]["scenes"][0];
        scene["title"] = json!("The first matching flag wins");
        scene["elements"][0]["kind"] = json!("stack");
        let findings = crate::claim_alignment::chapter_diagnostics(&proposal.to_string()).unwrap();
        let required = findings
            .iter()
            .find(|finding| finding["sceneProblem"] == "selectorRequired")
            .unwrap();
        assert_eq!(
            required["location"],
            "/sections/0/visual/presentation/scenes/0/elements"
        );
        let serialized = serde_json::to_string(&findings).unwrap();
        assert!(!serialized.contains("first matching flag"));
        let prompt = crate::claim_alignment::prompt_for_issues(findings).unwrap();
        assert!(prompt.contains("selectorRequired finding"));
        assert!(prompt.contains("selector must be the root layout"));
        assert!(prompt.contains("wrap it in stack"));
        assert!(!prompt.contains("first matching flag"));

        let mut graph_proposal = composed_proposal(3);
        let graph_scene = &mut graph_proposal["sections"][0]["visual"]["presentation"]["scenes"][0];
        graph_scene["title"] = json!("The first matching flag wins");
        graph_scene["mode"] = json!("graph");
        graph_scene["elements"] = json!([]);
        let graph_findings =
            crate::claim_alignment::chapter_diagnostics(&graph_proposal.to_string()).unwrap();
        assert!(
            graph_findings
                .iter()
                .any(|finding| finding["sceneProblem"] == "selectorRequired")
        );

        let mut independent = composed_proposal(3);
        independent["sections"][0]["visual"]["presentation"]["scenes"][0]["title"] =
            json!("Gate the memory-face open");
        independent["sections"][0]["visual"]["presentation"]["scenes"][0]["elements"][0]["kind"] =
            json!("selector");
        let independent_findings =
            crate::claim_alignment::chapter_diagnostics(&independent.to_string()).unwrap();
        let choice = independent_findings
            .iter()
            .find(|finding| finding["sceneProblem"] == "selectorChoice")
            .unwrap();
        assert_eq!(
            choice["location"],
            "/sections/0/visual/presentation/scenes/0/elements"
        );
        let choice_prompt =
            crate::claim_alignment::prompt_for_issues(independent_findings).unwrap();
        assert!(
            choice_prompt.contains("selectors render a miss as a handoff to the next candidate")
        );
        assert!(!choice_prompt.contains("Gate the memory-face open"));
    }

    #[test]
    fn selector_outcome_label_repetition_gets_a_bounded_positional_repair() {
        let mut proposal = composed_proposal(3);
        proposal["sections"][0]["visual"]["nodes"] = json!([
            {"label":"Invalid header","detail":"private detail","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null},
            {"label":"Public format dispatch","detail":"private detail","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null},
            {"label":"Valid header","detail":"private detail","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null},
            {"label":" public   FORMAT dispatch ","detail":"private detail","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null}
        ]);
        proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["elements"] = json!([
            {"kind":"selector","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"branch","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":1,"nodeIndex":0,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":1,"nodeIndex":1,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"branch","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":4,"nodeIndex":2,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":4,"nodeIndex":3,"tone":"neutral","value":"","valueOrigin":"none"}
        ]);

        let findings = crate::claim_alignment::chapter_diagnostics(&proposal.to_string()).unwrap();
        let duplicate = findings
            .iter()
            .find(|finding| finding["sceneProblem"] == "selectorOutcomeLabel")
            .unwrap();
        assert_eq!(
            duplicate["location"],
            "/sections/0/visual/presentation/scenes/0/elements/6/nodeIndex"
        );
        let serialized = serde_json::to_string(&findings).unwrap();
        assert!(!serialized.contains("Public format dispatch"));
        assert!(!serialized.contains("private detail"));
        let prompt = crate::claim_alignment::prompt_for_issues(findings).unwrap();
        assert!(prompt.contains("selectorOutcomeLabel finding"));
        assert!(
            prompt
                .contains("label each outcome with its specific source-grounded guard and result")
        );
        assert!(!prompt.contains("Public format dispatch"));
    }

    #[test]
    fn selector_miss_is_not_duplicated_as_another_branch_outcome() {
        let mut proposal = composed_proposal(3);
        proposal["sections"][0]["visual"]["nodes"] = json!([
            {"label":"First guard","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null},
            {"label":"First result","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null},
            {"label":"Second guard","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null},
            {"label":"Second result","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null},
            {"label":"Second result path","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null}
        ]);
        proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["elements"] = json!([
            {"kind":"selector","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"branch","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":1,"nodeIndex":0,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"token","parentIndex":1,"nodeIndex":1,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"branch","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":4,"nodeIndex":2,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"token","parentIndex":4,"nodeIndex":3,"tone":"neutral","value":"","valueOrigin":"none"},
            {"kind":"text","parentIndex":1,"nodeIndex":4,"tone":"neutral","value":"","valueOrigin":"none"}
        ]);

        let findings = crate::claim_alignment::chapter_diagnostics(&proposal.to_string()).unwrap();
        let shape = findings
            .iter()
            .find(|finding| finding["sceneProblem"] == "selectorShape")
            .unwrap();
        assert_eq!(
            shape["location"],
            "/sections/0/visual/presentation/scenes/0/elements/1"
        );
        let prompt = crate::claim_alignment::prompt_for_issues(findings).unwrap();
        assert!(prompt.contains("exactly one guard and its one matching result"));
        assert!(prompt.contains("miss-and-continue path"));
        assert!(!prompt.contains("First guard"));
    }
}
