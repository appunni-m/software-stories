---
kind: task
key: t07
parent: m1
title: Browse repository structure and evidence in Atlas
priority: 1
labels: phase-1, atlas, design
blocked_by: t06
---

## User outcome
The user can move from repository shape to a real file, symbol, relationship, or test without mistaking an inferred map for execution truth.

## Contract
Atlas combines a collapsible file tree, a filterable relationship canvas, search, and an evidence/details panel. Selecting a folder filters to descendants; selecting a node shows exact path, snapshot, type/counts, and supported edges. Each edge can be inspected for type, confidence, and source evidence. Clicking a file/symbol opens Code Lens at the same snapshot and anchor.

## Acceptance criteria
- The tree includes all inventory paths and visibly marks capture/index status.
- Relationship filters and search show their scope and preserve current snapshot selection.
- Dynamic or unresolved relationships are not drawn as proven edges.
- Selecting a node and returning from Code Lens restores the tree/map selection and viewport.
- Empty, no-match, unsupported-language, and stale-snapshot states explain the next action.

## Excludes
AI-assigned module responsibility, automatic architecture claims, and runtime visualization.

## Done evidence
Walk a fixture from root folder through a supported relationship to a symbol and test; inspect the evidence, open it in Code Lens, return, and verify selection and snapshot are unchanged.
