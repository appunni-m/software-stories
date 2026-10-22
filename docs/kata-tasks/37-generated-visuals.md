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

## Shared chapter compositions — 17 October 2026

File chapters, repository overviews and selected-section repairs now request
the same bounded HTML composition vocabulary as progressive chat. Fresh output
uses presentation v2 and complete grounded claim narration; carried sections
and stored legacy guides retain their bytes. The historical retrieval schema
and progressive tool catalog remain pinned. Native regressions pass (394), as
do 184 frontend tests and an unsigned debug app build.

A real ordinary Codex GPT-6 Luna/max Fontdone `loca.rs` chapter produced eleven
source-valid compositions in 240.902 s. Independent checks confirm all twenty
source anchors and the shown short-encoding arithmetic. **Quality remains
unaccepted:** signature-first exposition, fifteen code blocks among twenty-five
content elements, the two oversized-next alternatives sharing a misleading
Final-slot clip label, and code syntax in narration. [Original result, exact
scope and remaining gates](../acceptance/2026-10-17-shared-chapter-scenes.md).

Pending: make the agent open with the behavior/problem and a useful concrete
example; represent each branch outcome distinctly; favor meaning over copied
syntax; qualify fresh representation quality, real Pocket listening, native
source navigation, selective repair/cache preservation and language coverage.
Do not count this narrow source pass as a full repository or comprehension pass.

## Teaching-policy rejection and repair work — 17 October 2026

An isolated follow-up on the same Fontdone file/question returned in 182.906 s
but failed native composition validation. Five parent links nest content inside
leaf cards. The candidate uses seventeen cards rather than a useful visual
story, and still opens with a record definition. All original output and
thirteen exact source quotes are retained; no valid/accepted revision or audio
exists. [Evidence and exact scope](../acceptance/2026-10-17-shared-chapter-scenes.md#teaching-policy-follow-up-rejected-layout).

- [x] Extend the owned ordinary Codex chapter correction to bounded positional
  scene-structure findings alongside lexical findings. Apply it across supported
  file/overview/selected-repair paths, using the saved connection and no API
  prerequisite or new source request.
- [x] Use the existing single correction opportunity and original thread,
  question, model, settings, source, permission, Stop owner and deadline. Do not
  add independent retry loops, patch generated indices or remove claims locally.
- [ ] Independently validate the whole corrected candidate. Cover persistent
  invalid layout, unknown/oversized payloads, one-based/forward/leaf parents,
  Stop before correction, failures and unchanged saved history.
- [x] Share the meaning-first teaching policy with progressive chat while
  preserving stored receipt replay and original source/attempt bounds.
- [ ] Qualify actual example-led visuals and natural narrated meaning after
  structural admission. Fewer code elements, schema validity and the passing
  394-native regression suite cannot establish this quality. Keep open.

## Bounded native layout correction — 17 October 2026

The ordinary Codex chapter path now fingerprints the versioned one-correction
policy for file, overview, planned-file and selected-section requests, including
requests without a question. Shared native tree rules report source-free
positions/codes; the agent returns the complete candidate and full validation
still owns admission. The unchanged real Fontdone rejection yields all five
expected findings. Regression and mock-correction checks cover bounds, unknown
shapes, persistent failure, full source validation and unchanged history.

A deliberately invalid synthetic draft exercised the real installed GPT-6
Luna/max worker: one observed correction, unchanged claims and a source-valid
final candidate in 20.473 s. This is fault injection, not ordinary-generation
or visual-quality acceptance. All 401 native and 184 frontend tests pass and
the unsigned Mac bundle builds. [Exact evidence and limits](../acceptance/2026-10-17-codex-layout-correction.md).

The broader validation checkbox stays pending for live Stop/failure interaction
and native operation coverage. Progressive teaching-policy integration, an
ordinary fresh Fontdone outcome, meaningful visuals, Pocket listening, complete
repository narratives and language qualification remain open. No task closes
on this structural repair.

## Shared teaching and fresh ordinary Fontdone result — 17 October 2026

The ordinary GPT-6 Luna/max request from commit ecf891b returned in 363.349 s
and passed full native source validation: three narrated sections, nine current
compositions and 23 independently checked anchors. The worked introduction and
distinct clipping/empty-result labels improve the answer. **Quality still fails
acceptance:** nineteen metric elements mostly contain source syntax, and the
script still reads code expressions. Zero code leaves is not a teaching pass.
The probe does not observe correction turns, so do not infer that it repaired
an invalid first response. Original output and compiled source pins are retained.

Progressive chat now shares the same behavior-first/outcome policy paragraphs,
bound into the new request fingerprint as meaning-first-v2. Ordinary prompt
bytes remain identical. Historical scene/script/receipt replay still passes;
there is no new progressive account, native UI, Pocket or human quality pass.
The current suites pass 404 native and 184 frontend tests; the unsigned debug
Mac app builds. [Exact evidence and limits](../acceptance/2026-10-17-shared-teaching-policy.md).

- [ ] Qualify visuals by their rendered meaning, including expressions disguised
  as metric/token/card elements. An element-count rule cannot replace review.
- [ ] Qualify a fresh progressive answer under the shared teaching policy,
  preserve branch/error meanings and complete grounded narration, and exercise
  native source/return and Pocket listening. Keep this task open.


## Native replay and benchmark boundary — 17 October 2026

The quality checker now reuses the current native schema, canonicalizer, fresh
narration preparation and complete chapter validator. Its preserved Fontdone
loca.rs replay passes with 23 anchors; the original five leaf-parent errors still
fail. All 410 native and 190 frontend tests pass. This validates admission, not
meaningful visual teaching. [Exact evidence and remaining gates](../acceptance/2026-10-17-native-quality-boundary.md).

The rebuilt unsigned app is installed and open on the real existing Fontdone
workspace. The older saved PFR0 visual returns from lib.rs to font.rs lines
4053–4078 through its source control. Nested Back/return retains the parent's
section/scene and the child's Flow tab. Its saved narration player advances to
9.2 s of a displayed 17 s and Stop resets it. This is a bounded UI/player check
on an existing scene, not a fresh meaning-first generation, verified audible
quality, listening or comprehension acceptance. The broad narrative and human
teaching/listening gates remain open; no task closes on this replay.

## Prompt and semantic visual forms — 22 October 2026

The fresh-generation prompt now chooses a representation independently for each
claim and tells the model to make the data shape or state change the visual,
using cards only for concise labels. The shared scene contract adds an app-owned
input/operation/result transform, nested hierarchy, and FIFO queue alongside
the existing timelines, lanes, branches, before/after states, byte strips,
matrices, tables, stacks, and evidence-linked graphs. Transform proposals must
have exactly three direct stages. The live and offline readers render the same
forms; no model-authored HTML, JavaScript, or Three.js is executed. Three.js is
not included because the supported source examples do not yet show a spatial
behavior that benefits from 3-D.

This is prompt/schema/renderer implementation evidence only. A new account-backed
generation, rendered-source interaction, human visual/narration review, Pocket
listening, accessibility and language qualification remain unverified. Keep
`j53m` open.

The follow-up prompt revision (`ai-html-compositions-v8`, teaching policy 15)
adds a source-ordered selector ladder for first-match guard chains. It makes the
distinction between priority/fall-through and a single guard with alternatives
explicit, asks for visual grammar to vary by claim when evidence warrants it,
and rejects generic card piles as the main visual. Native and web validation
require two or more ordered branch checks, each with a guard and outcome; live
and offline HTML render visible check labels and miss handoffs. Three.js stays
an app-owned future option for genuinely spatial behavior; the model cannot
author executable 3-D code. Tests verify structure and markup only, not whether
a generated scene teaches the real source. The installed Fontdone guide remains
the open qualitative counterexample until it can be regenerated and reviewed.

### Ordered-decision fidelity — 22 October 2026

Live inspection found that the saved `Font::memory_face` first-match story was
drawn as a bottom-up LIFO stack, which put the fallback before the first probe.
Prompt `ai-html-compositions-v9` now says that first-match, priority,
ordered-probe and fall-through scenes require a source-ordered selector ladder
with each guard, outcome, miss and final fallback. The selector must be the
composition root, so a surrounding stack or other layout cannot reverse or
obscure precedence. Actual execution paths can still use a directed graph.

The native publisher, correction diagnostics, progressive authoring contract,
and browser-side validator share this requirement. An invalid draft receives a
fixed `selectorRequired` correction without exposing private scene text. The
prompt still chooses among behavior-specific forms per claim (for example
before/after, input/operation/result, iteration table, byte strip, matrix,
tree, queue, lanes or a graph where its edges are real). The model supplies
typed scene data; the app owns the safe responsive HTML renderer. 3-D remains
an app-owned future option for source-proved spatial behavior, not executable
model-authored JavaScript.

Verification on 22 October: 708 native tests passed (18 ignored), 611 frontend
tests passed, quality helper and corpus checks passed, and the production
frontend build passed. Prompt metadata remains under the existing 32 KB cap.
One approved fresh whole-repository Fontdone Guide is pending packaged-app
validation; the older saved Guide is preserved and does not qualify this
prompt. Human review of its scene meaning, narration, source navigation,
accessibility and comprehension remains open.
