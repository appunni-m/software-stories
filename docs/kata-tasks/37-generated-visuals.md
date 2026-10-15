---
kind: task
key: generated-visuals
parent: m3
title: Generate source-grounded visual explanations from real chapters
priority: 0
labels: product-quality
blocked_by: parser-binding-resolution
---

## User outcome
A freshly generated chapter contains a useful visual that explains actual behavior and lets the reader inspect the supporting code.

## Current evidence
src-tauri/src/provider.rs excludes visual/hotspot output from chapter prompts and schemas. chapter_generation.rs constructs new sections with visual: None and empty hotspots. The existing visual renderer/validator accepts dataFlow, sequence and hierarchy records; renderer support does not implement generation. This is a missing core product capability.

## Contract
Add a reviewed structured visual-planning step using the selected API or coding-agent backend. Choose an appropriate diagram from validated source claims: request/data flow, call sequence or component hierarchy. Resolve model-proposed references to app-owned exact anchors and typed destinations locally; validate every behavior node/edge. Use the existing app-owned renderer; never execute generated scripts/HTML. Plan bounded scenes and deliberate transitions so readers can follow one behavior at a time. Preserve text-only use when a diagram adds no explanatory value.

## Acceptance criteria
- Ordinary chapter generation produces useful diagrams on suitable fixtures without hand-inserting visual JSON.
- A route example shows input, relevant calls, state/data changes, success and failure branches supported by exact code; a structural-only graph is labeled accordingly.
- Each behavior node/edge opens its exact supporting snapshot range; Back preserves section, focus and playback position.
- Unknown references, invented control/data flow and unsupported runtime claims are rejected or visibly marked uncertain.
- Labels and layout remain readable in realistic multi-step chapters and at 200% zoom; keyboard and reduced-motion use retain meaning.
- Changing one section invalidates only dependent visuals; unchanged visual inputs retain their IDs.
- Offline export carries the validated diagram and destinations.
- Human review compares generated explanations against source and records whether the visual helps someone answer behavior questions; do not claim uniqueness from renderer availability.

## Excludes
Decorative diagrams counted as comprehension, claims of superiority without a comparison study, and arbitrary generated animation code.

## Done evidence
Real generated examples with source links, scene/click walkthrough and human comprehension findings when validation is authorized.
