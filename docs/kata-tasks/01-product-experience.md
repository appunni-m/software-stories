---
kind: task
key: t01
parent: m0
title: Define the complete Code Atlas screen and interaction contract
priority: 0
labels: phase-0, design, product
blocked_by:
---

## User outcome
The user sees one understandable product, not three unrelated demos. Every main control, destination, empty state, and error state has an agreed meaning before feature implementation.

## Contract
Use the overview and three screen concepts in `docs/design/` as design references. Specify the global shell; Atlas; Code Lens; Walkthrough; Guide Plan; and Chapter Studio. Atlas, Code Lens, and Walkthrough remain the three equal primary workspaces. Guide Plan and Chapter Studio are supporting workflows.

For each screen define: purpose, primary selection, visible data, main actions and destinations, back/return behavior, empty/loading/error/stale/draft/accepted states, keyboard focus, and what action can trigger filesystem or network work. Every clickable element must have a destination or be visibly disabled with an explanation. Maintain the detailed contract in `docs/INTERACTION_CONTRACT.md`.

## Acceptance criteria
- A screen/state matrix and cross-screen navigation map covers the first-use journey in `docs/PRODUCT_BLUEPRINT.md` and `docs/INTERACTION_CONTRACT.md`.
- Every source link resolves through `(project, snapshot, path, anchor)`; every chapter link resolves through `(chapter, revision, section)`.
- Source navigation preserves originating chapter section and playback position on return.
- The exact-context review and explicit-send action are distinct; opening or previewing it has no provider side effect.
- Concept images are labeled as references, not implementation screenshots.

## Excludes
Implementing screens, choosing provider models permanently, or adding a fourth primary workspace.

## Done evidence
The detailed interaction contract and product blueprint cover success, cancel, unsupported content, permission failure, network failure, stale source, recovery, keyboard focus, and return-state restoration. A reviewer can follow every primary control to a defined destination and identify the exact action that may read source or send a request.
