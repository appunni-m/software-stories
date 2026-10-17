//! Shared structural rules for admission and source-free correction feedback.

use super::{SceneElement, ValueOrigin};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TreeFinding {
    pub element: Option<usize>,
    pub code: &'static str,
    pub field: &'static str,
    pub message: &'static str,
}

/// Input is already strictly decoded. Oversized trees are not traversed.
pub(crate) fn findings(elements: &[SceneElement]) -> Vec<TreeFinding> {
    let mut issues = Vec::new();
    let mut add = |element, code, field, message| {
        issues.push(TreeFinding {
            element,
            code,
            field,
            message,
        })
    };
    if elements.is_empty() || elements.len() > 48 {
        add(
            None,
            "elementCount",
            "",
            "A composition needs 1–48 elements.",
        );
        return issues;
    }
    let mut depths = vec![0; elements.len()];
    let mut children = vec![0; elements.len()];
    let mut leaves = 0;
    for (index, element) in elements.iter().enumerate() {
        match element.parent_index {
            None if index == 0 => {}
            Some(parent) if parent < index && elements[parent].kind.container() => {
                children[parent] += 1;
                if children[parent] == 13 {
                    add(
                        Some(parent),
                        "childCount",
                        "parentIndex",
                        "A composition group may contain at most twelve children.",
                    );
                }
                depths[index] = depths[parent] + 1;
                if depths[index] > 6 {
                    add(
                        Some(index),
                        "nesting",
                        "parentIndex",
                        "Composition nesting may be at most six levels.",
                    );
                }
            }
            _ => add(
                Some(index),
                "containerParent",
                "parentIndex",
                "Composition elements need one root and an earlier container parentIndex.",
            ),
        }
        if element.kind.container() || element.kind.decorative() {
            if element.node_index.is_some()
                || !element.value.is_empty()
                || element.value_origin != ValueOrigin::None
            {
                add(
                    Some(index),
                    "layoutContent",
                    "",
                    "Layout and decorative elements cannot add uncited content.",
                );
            }
        } else {
            leaves += 1;
        }
    }
    if leaves == 0 || leaves > 12 {
        add(
            None,
            "contentCount",
            "",
            "A composition needs 1–12 cited content elements and no empty groups.",
        );
    }
    for (index, element) in elements.iter().enumerate() {
        if element.kind.container() && children[index] == 0 {
            add(
                Some(index),
                "emptyGroup",
                "",
                "A composition needs 1–12 cited content elements and no empty groups.",
            );
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn element(kind: &str, parent: Option<usize>) -> SceneElement {
        serde_json::from_value(json!({"kind":kind,"parentIndex":parent,"nodeIndex":null,
            "tone":"neutral","value":"","valueOrigin":"none"}))
        .unwrap()
    }

    #[test]
    fn only_an_earlier_container_can_parent_content() {
        for kind in ["row", "column", "grid", "panel", "compare", "stack"] {
            assert!(findings(&[element(kind, None), element("token", Some(0))]).is_empty());
        }
        for kind in [
            "card", "token", "code", "metric", "bytes", "matrix", "table", "text", "arrow",
            "divider",
        ] {
            assert!(findings(&[element(kind, None), element("token", Some(0))])
                .iter()
                .any(|v| v.code == "containerParent" && v.element == Some(1)));
        }
        for parent in [None, Some(1), Some(2), Some(usize::MAX)] {
            assert!(findings(&[element("row", None), element("token", parent)])
                .iter()
                .any(|v| v.code == "containerParent"));
        }
        assert!(findings(&[element("token", None)]).is_empty());
    }

    #[test]
    fn tree_limits_reject_without_traversing_oversized_input() {
        assert_eq!(findings(&[])[0].code, "elementCount");
        assert_eq!(findings(&vec![element("token", None); 49]).len(), 1);
        let mut wide = vec![element("row", None)];
        wide.extend((0..12).map(|_| element("token", Some(0))));
        assert!(findings(&wide).is_empty());
        wide.push(element("token", Some(0)));
        assert!(findings(&wide).iter().any(|v| v.code == "childCount"));
        assert!(findings(&wide).iter().any(|v| v.code == "contentCount"));
        let mut deep: Vec<_> = (0usize..6)
            .map(|i| element("column", i.checked_sub(1)))
            .collect();
        deep.push(element("token", Some(5)));
        assert!(findings(&deep).is_empty());
        deep[6] = element("column", Some(5));
        deep.push(element("token", Some(6)));
        assert!(findings(&deep).iter().any(|v| v.code == "nesting"));
        assert!(findings(&[element("row", None)])
            .iter()
            .any(|v| v.code == "emptyGroup"));
        let mut layout = element("row", None);
        layout.node_index = Some(0);
        assert!(findings(&[layout, element("token", Some(0))])
            .iter()
            .any(|v| v.code == "layoutContent"));
    }
}
