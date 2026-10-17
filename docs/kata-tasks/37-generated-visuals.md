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

## October 16 native rendering correction
A real Codex-generated synthetic diagram passed native source validation but the viewer rejected snake_case hotspot destination fields. Native serialization now emits frontend camelCase, with legacy aliases on read for hotspots, inference labels and source dependencies. Require a stored legacy diagram and a freshly generated diagram to render, click a node and edge to their captured lines, and return without losing exploration. This corrects a wire contract; it is not visual-quality or accessibility acceptance.

## October 17 narrated visual scenes

Apply the [visual story contract](61-guide-flow-detail.md): scenes are authored claim units, linked to their own validated node evidence; narration describes the visual's decisions and meaning. Guide reveals a focused graph with real branching and readable action labels, while full-map inspection remains available. No generated executable HTML, guessed word alignment, invented runtime paths or automatic whole-section replacement. Require fresh generated Rust and TypeScript stories, preserved branches/errors, readable focal nodes, source/return, keyboard/reflow/motion evidence and human comprehension before closure. The older Fontdone dispatch graph can exercise rendering but is not a newly authored story or a complete repository answer.

## October 17 representation choice and AI-authored HTML

The user's clarified requirement supersedes the graph-only interpretation: preserve the improved workspace, but let AI choose and compose the visual representation. A graph remains valid when paths or relationships make it the clearest teaching device. Guide must also support byte/header strips, code/input/output transformations, comparisons, stacks, tables, quantities and pixel matrices, with responsive groups and a narrated story across scenes. A collection of diagram nodes restyled as cards does not establish this outcome.

The implementation contract is [AI-authored scene compositions](../VISUAL_SCENES.md). The selected account-backed Codex agent authors a bounded HTML component tree via `visual.presentation`; the app renders the HTML. Native and reader validation enforce one scene per claim, same-claim evidence links, exact source values, explicitly labeled illustrative data, a single bounded tree and no executable content. AI chooses layouts and representation per scene, including graph scenes; app-owned rendering controls appearance, source clicks, playback, keyboard use and motion. Legacy graphs remain unchanged. Presentation changes participate in revision comparison and selective visual reuse.

Closure still requires a fresh ordinary Codex-generated HTML composition and a justified graph on real Rust/TypeScript behavior, native display/source clicks, scene/audio lifecycle, readable narrow/light/dark views, preserved failure paths, comprehension/listening review, and equivalent offline export. The new structured contract is generation infrastructure, not proof of visual or semantic quality. Offline export must preserve the actual composition or explicitly disclose its current graph/evidence fallback; it cannot claim full scene fidelity prematurely.

## October 17 HTML scene qualification

The progressive Codex path generated a complete synthetic HTML scene and a complete narrow Fontdone PFR0 scene. Native source clicks and Pocket playback were observed; graphs remain available where appropriate. The final mechanical suites pass 346 native tests (12 live probes ignored) and 171 frontend tests. Other generation operations, representation quality, human comprehension/accessibility and faithful offline scene export remain open. See [results, preserved failures and remaining gates](../acceptance/2026-10-17-visual-story-guide.md). This task remains open.

## October 17 offline composition checkpoint

The exporter now reproduces validated HTML composition primitives as inert
markup with manual scene/source links and matching scene audio when cached.
Graphs keep their explicitly disclosed source-outline fallback. Presentation
titles/values are filtered and source-valued content follows the excerpts
choice. All primitive/group combinations and four synthetic excerpt/audio
packages pass mechanical tests. Actual accepted-chapter/browser/cache evidence,
keyboard/reflow and human comprehension remain pending; the fixture is not an
agent-generation or visual-quality pass. See [scoped qualification and remaining
checks](../acceptance/2026-10-17-offline-visual-scenes.md). Keep this task open.

## Real broad-scene review and diagnostic recovery — 17 October

The [broad Fontdone artifact](../acceptance/2026-10-17-read-recovery-and-broad-answer.md) publishes three HTML-scene sections but remains partial. Its first scenes repeat the same node label on both sides of an arrow; admitted composition alone is not an effective transformation. Retain this as a real visual-quality counterexample. The second proposal also used a node cited only by a different claim, correctly rejected by native validation but reported generically. A reproducing regression now verifies fixed `sceneEvidence` feedback at the content `nodeIndex`, complete rejected-proposal retention, no publication before repair and successful correction. Native validation is unchanged. Source-free/unknown-shape checks also pass; full native suite: 351 passed, 13 explicit probes ignored, frontend: 179 passed.

The account artifact precedes the feedback and prompt clarification; no fresh broad run qualifies those later changes. Meaningful examples/transformations, actual Pocket narration, installed source interaction/accessibility and complete broad repository visuals remain open. The dummy offline greeting export demonstrates packaging mechanics only; user clarification is not a playback or quality acceptance result.

## Fresh visual counterexample — 17 October

The fresh frozen broad Fontdone operation published one HTML-backed draft, but scene two uses the identical `nodeIndex:2` (`format probes`) card with empty values on both sides of an arrow. It is valid structure but communicates no transition or fallback decision. The source claim also omits relevant early-return exceptions and the complete narration remains too technical. [Preserved scene, source review and incomplete run](../acceptance/2026-10-17-short-native-references.md#independent-source-and-representation-review-failed).

- [x] Add a versioned, concrete proposal-quality check/feedback for identical empty content across a transformation arrow. Require distinct input/result roles or meaningful distinct values where a transformation is claimed; do not reject legitimate same-source comparisons with different states/values or graph loops.
- [x] Preserve old saved scene decoding/behavior; do not retroactively invalidate historical admitted compositions with a new proposal-quality policy.
- [ ] Qualify AI-authored representations that teach actual changes/branches, then check narration and captured-source hotspots in the installed app. The existing generic “avoid identical labels” instruction did not prevent this sample.

Keep this task open. The broad completion assertion remains failed; local mechanical tests or an HTML mode label do not establish visual quality.

## Versioned scene-quality implementation — 17 October

Presentation version 2 and progressive checkpoint version 3 now reject identical visible labels/values across a composition arrow, including nested groups. Changing IDs, tone or wrappers does not evade the rule. Distinct states/roles, exact case/whitespace changes, comparisons without arrows and graph scenes remain available. New proposals receive positional `sceneRepetition`/`sceneVersion` feedback and retain rejected arguments; no original budget is reset. Historical presentation version 1/checkpoints 1/2 replay unchanged and exact repeats remain idempotent. The real Fontdone counterexample fails before the guard and is rejected afterward.

Mechanical evidence: 368 native tests, 183 frontend tests, production build and unsigned debug bundle pass, including legacy replay, correction and offline version 2 source/audio export. [Contract and limits](../acceptance/2026-10-17-scene-v2.md). A fresh broad account qualification, actual scene meaning/source interaction, Pocket listening and native accessibility remain required. This task stays open.

## Fresh version-2 Fontdone result — 17 October

The committed-helper GPT-6 Luna/max run publishes three scene-backed drafts but finishes Partial at 491.831 seconds. All 61 calls deliver; eight proposals are exhausted before render dispatch, rasterization and final bitmap publication. All 38 anchors independently match captured bytes/hashes/coordinates. The repeated empty-card defect is absent, but the Optional parse defaults label remains ambiguous and narration remains too technical. This is not human comprehension, speech or complete-repository acceptance. [Unchanged result and targeted review](../acceptance/2026-10-17-scene-v2.md).

Two generic source-value rejections are now exact `sceneSourceValue` findings at the offending element's `/value`, after native rejection and using only native-resolved same-claim excerpts. Both real rejections and a failing-before/passing-after correction regression are preserved. Final mechanical suites: 371 native pass, 13 explicit probes ignored, 184 frontend pass. This follow-up postdates the account artifact; complete account, installed source/scene/Pocket and accessibility evidence remain required. Keep this task open.

## Exact tokens still permit misleading scenes — 17 October

The new source-valid Fontdone result reaches three drafts through rasterization,
but remains Partial. Its “Geometry and programs” scene places `loca_data` and
`glyf_data` across an arrow although the narration describes distinct copied
tables; the final scene singles out the overlap rasterizer without showing its
branch condition. Most compositions are code-card arrangements, not helpful
visual explanations. [Exact preserved proposals/scenes and review](../acceptance/2026-10-17-fontdone-source-value.md).

- [ ] Teach relations and changing state through AI-authored representations,
  including accurate arrows, branch conditions and meaningful examples. Different
  exact source tokens do not by themselves imply a transformation.
- [ ] Review the actual narrated scene in context with clickable captured code;
  keep semantic/human comprehension and Pocket listening gates separate from
  schema, source grounding or export-link checks.

Keep this task open. The dummy greeting export remains only a packaging demo.
