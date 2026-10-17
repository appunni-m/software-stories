//! Positions and fixed tree codes only; no source, labels or model strings.

use crate::scene_presentation::{tree, SceneMode, ScenePresentation};
use serde_json::{json, Value};

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
        if !matches!(presentation.schema_version, 2 | 3) || presentation.scenes.len() > 20 {
            continue;
        }
        for (scene, element) in crate::scene_presentation::foreign_claim_positions(section) {
            if issues.len() >= super::MAX_ISSUES { return; }
            issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{scene}/elements/{element}/nodeIndex"),"sceneProblem":"sceneEvidence"}));
        }
        for (index, scene) in presentation.scenes.iter().enumerate() {
            if presentation.schema_version == 3 && !scene.narration.as_ref().is_some_and(|s| !s.trim().is_empty() && s.len() <= 4_000) {
                if issues.len() >= super::MAX_ISSUES { return; }
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes/{index}/narration"),"sceneProblem":"narrationRequired"}));
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
        if presentation.schema_version == 3 && issues.len() < super::MAX_ISSUES
            && presentation.scenes.iter().all(|s| s.narration.as_ref().is_some_and(|s| !s.trim().is_empty() && s.len() <= 4_000)) {
            let total = presentation.scenes.iter().filter_map(|s| s.narration.as_deref())
                .map(|s| s.split_whitespace().collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>().join(" ").len();
            if !(20..=4_000).contains(&total) {
                issues.push(json!({"location":format!("{prefix}/visual/presentation/scenes"),"sceneProblem":"narrationLength"}));
            }
        }
    }
}
