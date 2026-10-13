---
kind: task
key: t17
parent: m3
title: Render safe interactive diagrams with evidence-linked hotspots
priority: 1
labels: phase-3, visuals, player
blocked_by: t11, t16
---

## User outcome
The reader can click a real component or step in a visual and inspect the code or chapter that supports it.

## Contract
Define a typed, versioned diagram format for bounded nodes, edges, labels, layout hints, section IDs, evidence links, and hotspot targets. Render with app-owned components. Each node/edge label that describes code behavior carries one or more exact source links. Hotspots navigate only to a validated source anchor, section, or Atlas selection and preserve return context. A malformed visual falls back without hiding the section's explanation or evidence.

## Acceptance criteria
- Unknown schema versions, oversized diagrams, duplicate IDs, missing evidence, and invalid destinations are rejected with useful issue details.
- No model output is executed as script, raw HTML, or CSS.
- Node and edge evidence buttons open their cited source ranges, and Back restores focus to the exact visual control.
- Selecting a hotspot visibly highlights the initiating action after Back and preserves the reader's section, transcript position, and playback state.
- Each app-rendered diagram has a deterministic identity derived from its typed visual data, renderer contract, referenced anchor payloads, and used hotspot destinations. Prose, narration, unlinked anchors, unused hotspots, and unrelated sections do not change that identity.
- Diagram changes invalidate only the app-rendered diagram whose typed inputs changed; no generated media cache is implied.
- Reader can use the chapter without a visual when the diagram is unavailable.

## Excludes
Free-form generated animation, decorative unlabeled edges presented as facts, and external image uploads.

## Done evidence
Use valid and malformed fixtures to validate a multi-step flow, click node/edge evidence links and section/source hotspots, preserve exact return focus and reader context, confirm invalid hotspots cannot open unrelated code, and prove the fingerprint ignores unrelated prose while changing for diagram/evidence/destination changes.
