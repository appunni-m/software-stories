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
Before the source implementation recorded below, `src-tauri/src/provider.rs` excluded diagram proposals from chapter prompts and schemas, while `src-tauri/src/chapter_generation.rs` created new sections with no visual or hotspots. The existing visual renderer and validator could read `dataFlow`, `sequence`, and `hierarchy` records, but generation did not produce them.

## Source implementation progress (2026-10-15)
File chapters, repository overviews, and selective section updates now accept a bounded visual proposal alongside claims. A proposal may explicitly select `none`, preserving text-only explanations, or describe a small data-flow, sequence, or hierarchy graph. Each node and edge cites exact `{path, quote}` entries copied from its section's claims; local canonicalization resolves those quotes to the app's source anchors and creates Code Lens hotspots. The canonicalizer rejects malformed plans, dangling/self edges, out-of-range indexes, oversized labels, and citations that do not resolve to claim evidence. Visual schema version is independent of chapter revision schema version.

Selective updates now preserve the prior graph's node, edge, and hotspot IDs only when the regenerated graph content, destinations, and remapped evidence are identical. A changed graph stays attached to the edited section; unchanged sections continue through the existing carry-forward path.

The existing player renders clickable source citations for node and edge evidence and routes node hotspots through revision-aware navigation (`src/Walkthrough.tsx`). The offline exporter serializes typed diagrams and links node/edge evidence and Code Lens hotspots to bundled targets (`src/lib/offlineExport.ts`). This is static implementation evidence only.

This is partial source progress, not acceptance evidence. No tests or builds were run. There is not yet a generated example with a source-click walkthrough, visual accessibility/reflow review, exercised offline export, or a human comprehension review. Generation uses the shared structured chapter contract and canonicalizer in the direct-provider and native agent source paths. That shared source wiring does not establish accepted Codex/Claude runtime support, visual quality or production readiness.

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

## October 16 evidence reference update

A packaged synthetic chapter run rejected a visual quote that was not attached to a validated claim. Current generation schemas now use zero-based `{claimIndex, evidenceIndex}` references into the same section's claims, after those claims have passed exact quote/identifier validation. Native code creates all source anchor and hotspot IDs. Bounds, missing references, duplicate references, malformed indexes and foreign fields remain errors. Legacy quoted visual proposals keep their original exact-attachment checks; the stored visual schema is unchanged. Three Rust regressions cover native source-link construction, invalid references and legacy quote rejection. See [current contract](../CHAPTER_SCHEMA.md) and [dated flow evidence](../acceptance/2026-10-16-ai-guide-flow.md). Full visual-quality acceptance remains open.
