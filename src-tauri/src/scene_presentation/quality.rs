//! A narrow versioned proposal-quality rule, not semantic truth or execution.

use super::{ElementKind, PresentationScene, SceneMode, ScenePresentation};

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
/// Historical version 1 scenes retain their original admission rules.
pub(crate) fn repetition_positions(
    presentation: &ScenePresentation,
    labels: &[&str],
) -> Vec<(usize, usize)> {
    if presentation.schema_version != 2 || presentation.scenes.len() > 20 || labels.len() > 12 {
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
