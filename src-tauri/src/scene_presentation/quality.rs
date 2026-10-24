//! A narrow versioned proposal-quality rule, not semantic truth or execution.

use super::{ElementKind, PresentationScene, SceneMode, ScenePresentation};
use std::collections::HashMap;

fn has_layout_tree(scene: &PresentationScene) -> bool {
    !scene.elements.is_empty()
        && scene.elements.len() <= 48
        && scene
            .elements
            .iter()
            .enumerate()
            .all(|(i, e)| match e.parent_index {
                None => i == 0,
                Some(parent) => parent < i && scene.elements[parent].kind.container(),
            })
}

fn descends_from(scene: &PresentationScene, mut index: usize, root: usize) -> bool {
    while let Some(parent) = scene.elements[index].parent_index {
        if parent == root {
            return true;
        }
        index = parent;
    }
    false
}

/// Compare visible content, not node identity, tone, primitive or wrappers.
/// Values remain byte exact: case/whitespace changes can be the actual lesson.
fn signature<'a>(
    scene: &'a PresentationScene,
    root: usize,
    labels: &[&'a str],
) -> Option<Vec<(&'a str, &'a str)>> {
    scene
        .elements
        .iter()
        .enumerate()
        .filter(|(i, e)| {
            (*i == root || descends_from(scene, *i, root))
                && !e.kind.container()
                && !e.kind.decorative()
        })
        .map(|(_, e)| Some((labels.get(e.node_index?).copied()?.trim(), e.value.as_str())))
        .collect()
}

/// Return only bounded fixed positions; labels and values never enter findings.
pub(crate) fn repetition_positions(
    presentation: &ScenePresentation,
    labels: &[&str],
) -> Vec<(usize, usize)> {
    if presentation.schema_version != 3 || presentation.scenes.len() > 20 || labels.len() > 12 {
        return vec![];
    }
    let mut positions = Vec::new();
    for (scene_index, scene) in presentation.scenes.iter().enumerate() {
        if scene.mode != SceneMode::Composition || !has_layout_tree(scene) {
            continue;
        }
        for (index, element) in scene.elements.iter().enumerate() {
            if element.kind != ElementKind::Arrow {
                continue;
            }
            let Some(parent) = element.parent_index else {
                continue;
            };
            let siblings: Vec<_> = scene
                .elements
                .iter()
                .enumerate()
                .filter(|(_, e)| e.parent_index == Some(parent))
                .map(|(i, _)| i)
                .collect();
            let Some(at) = siblings.iter().position(|i| *i == index) else {
                continue;
            };
            let content = |root| signature(scene, root, labels).filter(|s| !s.is_empty());
            let before = siblings[..at].iter().rev().find_map(|i| content(*i));
            let after = siblings[at + 1..].iter().find_map(|i| content(*i));
            if before.is_some() && before == after {
                positions.push((scene_index, index));
                if positions.len() == 64 {
                    return positions;
                }
            }
        }
    }
    positions
}

/// Find repeated visible labels among outcomes in separate selector branches.
/// The first direct item in each branch is its guard; only outcomes are
/// compared. Findings contain positions only, never model-authored labels.
pub(crate) fn repeated_selector_outcome_positions(
    presentation: &ScenePresentation,
    labels: &[&str],
) -> Vec<(usize, usize)> {
    if presentation.schema_version != 3 || presentation.scenes.len() > 20 || labels.len() > 12 {
        return vec![];
    }
    let mut positions = Vec::new();
    for (scene_index, scene) in presentation.scenes.iter().enumerate() {
        if scene.mode != SceneMode::Composition
            || !has_layout_tree(scene)
            || !scene
                .elements
                .first()
                .is_some_and(|element| element.kind == ElementKind::Selector)
        {
            continue;
        }
        let branches: Vec<_> = scene
            .elements
            .iter()
            .enumerate()
            .filter(|(_, element)| {
                element.parent_index == Some(0) && element.kind == ElementKind::Branch
            })
            .map(|(index, _)| index)
            .collect();
        let mut first_branch_by_label = HashMap::<String, usize>::new();
        for (branch_order, branch_index) in branches.into_iter().enumerate() {
            let outcomes = scene
                .elements
                .iter()
                .enumerate()
                .filter(|(_, element)| element.parent_index == Some(branch_index))
                .skip(1);
            for (element_index, element) in outcomes {
                if element.kind.container() || element.kind.decorative() {
                    continue;
                }
                let Some(label) = element
                    .node_index
                    .and_then(|node_index| labels.get(node_index))
                    .map(|label| label.trim())
                    .filter(|label| !label.is_empty())
                else {
                    continue;
                };
                let normalized = label
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase();
                if normalized.is_empty() {
                    continue;
                }
                match first_branch_by_label.get(&normalized) {
                    Some(first_branch) if *first_branch != branch_order => {
                        positions.push((scene_index, element_index));
                        if positions.len() == 64 {
                            return positions;
                        }
                    }
                    Some(_) => {}
                    None => {
                        first_branch_by_label.insert(normalized, branch_order);
                    }
                }
            }
        }
    }
    positions
}

/// Return a selector branch when it lists more than the one result associated
/// with its guard. The reader renders a miss-and-continue path automatically;
/// a second outcome under the same guard would blur that path with the matched
/// result. Structural replay may still accept historical scenes with this
/// shape; fresh authoring applies this stricter rule.
pub(crate) fn selector_multiple_outcome_positions(
    presentation: &ScenePresentation,
) -> Vec<(usize, usize)> {
    if presentation.schema_version != 3 || presentation.scenes.len() > 20 {
        return vec![];
    }
    let mut positions = Vec::new();
    for (scene_index, scene) in presentation.scenes.iter().enumerate() {
        if scene.mode != SceneMode::Composition
            || !has_layout_tree(scene)
            || !scene
                .elements
                .first()
                .is_some_and(|element| element.kind == ElementKind::Selector)
        {
            continue;
        }
        for (branch_index, _) in scene.elements.iter().enumerate().filter(|(_, element)| {
            element.parent_index == Some(0) && element.kind == ElementKind::Branch
        }) {
            let direct_leaves = scene
                .elements
                .iter()
                .filter(|element| element.parent_index == Some(branch_index))
                .count();
            if direct_leaves > 2 {
                positions.push((scene_index, branch_index));
                if positions.len() == 64 {
                    return positions;
                }
            }
        }
    }
    positions
}
