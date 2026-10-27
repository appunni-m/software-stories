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
frontend build and unsigned macOS app/DMG builds passed. Prompt metadata remains
under the existing 32 KB cap. One approved fresh whole-repository Fontdone
Guide is running in the packaged app. Its opening, “Opening a face from memory,”
uses two scenes; the selected first-format-probe scene shows input bytes,
PCF-signature match/miss paths, and the PFR0 handoff. Its 11-second narration
explains that a miss continues probing instead of becoming an error. Clicking
the PCF check opened captured `src/font.rs` at `Font::memory_face` (line 4058).
The next slide is still building, so this early result does not qualify the
whole guide, broader visual variety, audible quality, accessibility or
comprehension. The prior partial Guide remains saved but was marked Interrupted
when the app was restarted for the new build. Keep `j53m` open.

## Section-wide scene budgets and intent selection — 22 October 2026

A second approved whole-repository Fontdone Guide completed with **partial**
status on the packaged `ai-html-compositions-v9` prompt. Its four-topic outline
produced no opening and no sections for the first three topics; the fourth
topic produced one accepted section, “Outline modes, empty slots, and safe
normal bounds.” That section has two claims and two graph-mode scenes, with 12
nodes and 10 edges total. The graph shows real dispatch and guard relationships,
but both scenes use graphs and the rendered labels become dense at the current
zoom. No HTML composition was generated in this Guide. The outline also says it
does not cover the C ABI/WASM adapters or individual format parsers. This is a
useful source-grounded partial result, not whole-repository or visual-quality
acceptance. Audio and source-click behavior were not reviewed in this run.

The failures expose two prompt issues: the whole-section node/edge cap was not
clear enough when several claim scenes were authored, and valid relationships
still led to a graph where a more direct form would teach better. The current
source prompt is now `ai-html-compositions-v10` / `meaning-first-v16`. It states
the 12-node/20-edge cap once per entire section, calls per-scene element/leaf
limits ceilings rather than goals, and directs mode/enum dispatch to a table,
sequential early-return guards to a source-ordered selector, and graphs to real
relationships that benefit from tracing. Safe correction feedback now includes
the exact native authoring bounds. Rust and browser validators recognize
sequential early-return guard language as ordered-selector behavior. Model-
authored HTML, JavaScript and Three.js remain disallowed; the app owns the
renderer.

Focused Rust prompt/fingerprint/selector tests and the 14-test browser scene
contract suite pass. A full `cargo fmt --check` still reports a formatting
difference in the untouched `src-tauri/src/workspace_conversation/tests.rs`;
all individually modified Rust files pass the scoped rustfmt check. The app
was rebuilt and installed with the v10 prompt. The fresh Guide review and the
follow-up prompt change are recorded below.

## Fresh visual-grammar evaluation — 22 October 2026

One user-approved fresh Guide started on Fontdone `main@5249b72371` with the
installed v10 build. Its outline, “Tracing face requests to glyph outcomes,”
has five topics and explicitly omits C ABI/WASM entry points, the complete
rasterizer result/error handling and direct runtime linkage outside the
captured ranges. The saved guide is at `waitingForPlayback`: four of its five
topics have attempts, with one published section each in the opening and first
three topics; the interpreter-failures topic published one partial section,
and render-dispatch has not started. The first topic required one automatic
retry. Audio quality and source-click behavior have not been reviewed.

The visible representation evidence is mixed. The opening contains three
scenes: a direct three-node call chain authored as a graph, an early-return
guard with visible error/continue outcomes authored as a graph, and a
`NO_RECURSE` decision authored as a composition. The SVG/bitmap routing section
uses graph mode in all three scenes, despite those claims being an ordered
preflight, a first image probe and a bitmap eligibility/fallback decision. The
outline-load-strategy section is stronger: two composition scenes use selector
groups for the ordered load-flag branches. This confirms that v10 can produce a
useful semantic composition, but the graph fallback still dominates linear
calls and simple decisions. When first opened, the first graph scene was blank;
selecting that scene again made the three-node call path render. Track this as a
first-visit graph-rendering defect separately from prompt selection.

The source prompt is now `ai-html-compositions-v11` / `meaning-first-v17`. It
defaults to built-in semantic compositions, maps direct call chains to
timelines and single guards to branches, and reserves graph mode for genuinely
non-linear networks such as fan-out, convergence, cross-links or cycles. It
explicitly says that source links or real caller/callee edges alone do not
justify a graph, and prefers `kind:none` over an empty graph when no visual
helps. The saved v10 Guide remains available for playback. The v11 build has
been packaged and installed, but has not been evaluated in a fresh Guide.
Keep `j53m` open until its remaining topics, source navigation, narration and
human comprehension are reviewed. A future v11 Guide should check the
graph-to-timeline distinction and the first-visit rendering defect.

## Per-scene representation and ordered-guard validation — 22 October 2026

The fresh visual prompt is now `ai-html-compositions-v14` with teaching policy
`meaning-first-v22`. It asks the model to identify one focal subject per scene,
then choose a form that makes the source-backed route, choice, change, state,
data shape, parallel work or relationship visible. Concrete guidance covers
timelines, first-match selectors, mode tables, before/after comparisons,
transforms, lanes, byte layouts and graphs. It discourages card grids and
identifier-only networks, and allows narration with clickable source where a
visual adds no value. Graphs are reserved for relationships that genuinely
need a network view. Source narration must explain what the visual means and
what follows from its decisions rather than reading declarations aloud.

Fresh native validation and safe correction diagnostics now reject a graph for
an ordered guard chain; the frontend reader recognizes multiple early returns
as ordered decisions. Historical saved graphs remain readable. Model output
remains data interpreted by app-owned renderers: generated HTML, scripts and
Three.js are not run. A future spatial renderer can be added if source behavior
actually calls for a spatial model; adding 3-D to a parser dispatch story would
make it harder to understand.

Verification for this implementation: 714 Rust library tests passed (18
ignored), 612 frontend tests passed, the prompt/quality helper checks passed,
and the production macOS app and DMG builds completed. The metadata was checked
in the built app for v13/v21. One user-approved fresh whole-repository Guide was
started in the packaged app using this prompt. It has produced the opening
section, “From memory bytes to a face,” with two scenes. Scene one uses a
decision and outcomes layout to show the borrowed input, the memory-face call,
wrapping a successful result in `Face`, and early return on an error. Scene two
shows the first PCF signature probe: a match returns the PCF parser result,
while a miss continues to the PFR0 probe. Both use non-graph layouts suited to
their local behavior. The narration is two short, consequence-focused passages
(10 and 11 seconds); it explains success/error propagation and
match/fall-through in plain words. Clicking the PCF check opened captured
`src/font.rs` at line 4058.

The live Guide has emitted a second draft slide, “From character code to glyph
slot,” with four scenes. These choose different layouts for a direct request
timeline, a Type 1 first-match selector, cmap-format cases and a glyph-index
bound decision. The selector shows its guard, result and explicit fall-through;
the final decision distinguishes index zero at/above the glyph-count bound from
keeping an in-range index. This is stronger per-scene variety than the earlier
graph-heavy outputs. However, visible values still include large source
expressions inside the cards, so reduced text prominence is not established.

The Guide remains at “Your guide is coming together…” with another slide
building. Narration text was reviewed for both draft slides and explains the
input, decision and consequence in concise language. Audio state was mixed:
some scene players reported “Ready to play” with a 10–11 second duration, while
another showed “Audio not prepared” on first visit. Pocket audio was not
listened to. The PCF scene source link was verified at `src/font.rs` line 4058.
This partial result does not establish whole-repository coverage, audible
quality or human comprehension. Keep `j53m` open; do not count older Fontdone
attempts as v13 evidence.

## Fresh packaged v14 Guide review — 22 October 2026

The approved run `9555ca72-9b65-4c22-9359-a3a85db06348` uses Fontdone
`main@5249b72371` and the packaged v14/v22 build. Its outline has four follow-on
units. The partial snapshot and outline explicitly leave the C ABI/WASM entry
points and later parsing/rendering behavior outside the opening answer.

The opening publishes two different source-linked forms: a timeline for the
Rust byte-input boundary, and an ordered selector for the first parser probes.
The selector's source action opened captured `src/font.rs` at line 4058. This
confirms correct per-scene form selection and source navigation, but its current
renderer still displays large pill/card-like source expressions. The two
narrations begin with declarative implementation summaries rather than guiding
attention through a story. Audio was Ready to play (13–15 seconds) but was not
listened to.

Timing is not a pass: outline generation took 61.368 seconds and the opening
operation took 144.172 seconds, or about 205.5 seconds from guide start to its
first persisted section. The first follow-on (`parse-font`) returned a final
manifest without a proposal twice. The next follow-on returned without a
section after 308.925 seconds and entered its automatic retry; the guide still
showed “Building next slide…” at review time. No full-guide coverage or
comprehension pass is claimed.

The packaged source iteration `ai-html-compositions-v15` / `meaning-first-v23`
uses compact rails, numbered selector gates and input-first narration. A
separate whole-repository Guide now exercises that version. Its opening still
uses source-code values in its cards and omits two meaningful constructor
arguments from the visual, even though narration names them. The first accepted
follow-on section uses row/branch layouts for the loader path, early exits,
missing-bitmap substitution and an empty slot, but every visual leaf displays a
raw code expression and spoken narration names methods and error variants. Two
planned constructor sections have failed with `namedIdentifierEvidence`,
`selectorShape`, `sceneSourceValue`, `sectionValidation` and
`proposalCorrectionLimit`; the guide is continuing through independent units.
Keep these as failure evidence, not as a full-guide pass.

The packaged source iteration is now `ai-html-compositions-v17` /
`meaning-first-v25`. It retains the v16 readable-label and value-coverage rules,
and spells out the native selector tree shape after repeated proposals wrapped
direct selector checks in rows. The selector must own its guarded branch groups
as direct children; guard and outcomes go inside each branch. Source links stay
clickable from evidence anchors without displaying code expressions.

The earlier v15/v23 run was stopped from the app after the new run was approved;
its published sections and receipts remain saved. The packaged v17 app was
installed with the preceding app kept as a rollback. Its fresh Fontdone Guide is
active. The first published section, “The direct-host face-opening entry,” has
four scenes: the public host route, the error-slot guard, byte-pointer handling,
and the loader result/handle. It uses an ordered path and decision layouts with
plain-language labels. Narration follows the visible decisions, explains the
status/handle consequences, and avoids reading code identifiers. Audio was
available to play but was not listened to. This is a promising first section,
not a full-repository or human-comprehension pass; review later slides, source
links, rendering and audio before closing the task.

## Fresh v17 visual-prompt Guide review — 23 October 2026

The user-authorized packaged Guide `79461b1a-e33d-40d2-be00-0ad86abec96a`
used Fontdone `main@5249b72371`, snapshot
`snap-1792140879637485000-11155-0`, GPT-6 Luna/max. The final native Guide
record reached `ready`; the reader showed seven slides and all six planned
topics complete. The outline title is “A representative WASM glyph path.” It
uses six independent WASM units plus an opening, rather than explaining the
repository broadly. The plan explicitly says this is not the repository's full
behavior, does not trace the native Rust API or C ABI, and cannot establish the
outermost face-opening boundary because its evidence for that helper is only a
contract test. This is a visual-prompt review, **not whole-repository
acceptance**.

The authored scene forms vary with the code: input/operation/result for the
host-facing byte handoff; decision/outcome branches for face lookup and loader
results; an ordered first-match selector for the advance lookup; a four-gate
selector followed by a temporary-status check and explicit caller-output map
for subglyph retrieval; decision scenes for render success/error; and a
pointer/length decision plus a compact width/rows/pitch reading for bitmap
accessors. Scene narration explains the behavior's consequence. For example,
loader failures return unchanged without assigning the slot, subglyph output
copies happen only after success, and missing bitmap lookups yield a null
pointer/zero length or zero dimensions. The narration panels reported Pocket
audio “Ready to play”; no human listened to it, and no human comprehension,
keyboard, reflow, contrast, or motion review was performed.

Source actions were opened for the real scenes, including
`fontdone-wasm/src/implementation.rs:9732–9768` (character/glyph load),
`9769–9799` (advance lookup), `9823–9895` (subglyph guards and copy),
`9861–9923` (render result), and `9924–9981` (bitmap accessors). The face
handoff scene also cited `fontdone-wasm/src/lib.rs:397–426` and the captured
implementation ranges. These source links worked in the packaged reader.

The bitmap-accessor slide required three Guide-level attempts. The first
returned an incomplete manifest after native section-validation rejection
(165.469 seconds); the automatic retry returned no section after
`sceneAuthoringShape`, generic `sectionValidation`, and
`proposalCorrectionLimit` findings (116.027 seconds). A final bounded attempt
published the slide (78.298 seconds). Earlier slides stayed available while
this topic retried. This is successful recovery with a concrete prompt/schema
gap, not zero-retry reliability. Keep `j53m` open; whole-repository coverage,
audio listening, comprehension, and accessibility remain unqualified.

## v18 visual fallback follow-up — 23 October 2026

Reviewing the final bitmap-accessor scene exposed a specific rendering gap:
three clickable metrics named pixel width, row count and signed byte pitch, but
the visual showed neither their shared zero fallback nor the guard/result
relationship. The narration did explain that missing bitmap lookups produce
zero. Labels alone did not make that behavior visible.

The current prompt is `ai-html-compositions-v18` / `meaning-first-v26`. It now
requires defaults and fallbacks beside each output, connects shared guards to
their results, and rejects label-only metrics as incomplete. Existing prompt
guidance still chooses a scene form per behavior (for example, a selector,
before/after change, transform, byte layout, lanes or a graph when a network is
actually present), rather than defaulting to graph or card layouts. Proposals
remain typed scene data rendered by the app; the runtime does not execute
model-written HTML or JavaScript, and there is no tested spatial renderer for
Three.js scenes.

The full Rust library suite passes **715 tests** (18 ignored); the frontend
TypeScript check and Vite build also pass, and the unsigned debug app bundle
was rebuilt. This prompt revision has **not** been evaluated in another Guide:
the reviewed seven-slide Guide used v17. Treat the new rule as implemented but
not yet confirmed in generated output. Whole-repository scope, repeated
section-generation reliability, Pocket listening, comprehension and
accessibility remain open.

## Fresh v18 Guide review and v19 visual follow-up — 23 October 2026

One fresh Fontdone Guide produced seven slides and completed all six planned
topics. It generated incrementally and did not time out. Its outline chose four
starting files and explicitly narrowed the story to a representative Rust CLI
path: C ABI and WASM entry points, parser/rasterizer internals and broader
failure behavior were not traced. This is useful chapter-level evidence, not
whole-repository coverage.

The Guide varied scene forms across guarded decisions, input/operation/result
transforms and output/report views. The bitmap/report scene still reduced
multiple fields to repeated generic “Report fields” cards and did not show an
actual pixel matrix. The source action on the opening story opened
`examples/load_glyph.rs` at its cited source. Pocket narration was ready to
play and its text was reviewed; no human listening or comprehension check was
done.

That concrete output gap led to `ai-html-compositions-v19` in the source
prompt. It asks for a behavior-appropriate shape before labels, a single table
for related output fields, an actual pixel matrix when source data supports
one (otherwise a clearly illustrative grid), and one source-backed output
template for formats without runtime values. It also permits narration and
source links when no truthful visual fits. The version is wired into the
progressive generation contract. **v19 has not been evaluated in another
Guide**, so these are prompt requirements, not confirmed generated behavior.
The complete Rust library suite now passes **720 tests** (18 ignored), including
the updated prompt-contract assertions.
Keep `j53m` open; whole-repository scope, visual quality, human audio,
comprehension and accessibility remain open.

## v20 visual specificity follow-up — 23 October 2026

The next prompt revision is `ai-html-compositions-v20` / `meaning-first-v26`.
It adds a visual-specificity review: identify the exact source-backed behavior
first, center the most specific available representation, reject generic
label-swappable layouts, and prefer one concrete guard, transition or result
over a component catalog. It preserves typed app-rendered HTML, complete
outcomes, clickable evidence and the rule that 3-D is only for spatial
behavior with a tested app-owned renderer. No arbitrary model JavaScript is
enabled.

The source contract and prompt assertions are updated. A separate account-backed
Fontdone Guide evaluating the preceding v19 prompt has a five-topic outline
that narrows to the Rust memory-face path and explicitly omits the C ABI, WASM
and complete glyph path. Its opening uses a vertical stack of generic input,
entry and result boxes; the distinct error and success destinations continue
below the current viewport. A second scene uses the numbered selector ladder.
Treat this as partial story-scope and visual evidence, not whole-repository
coverage or v20 visual acceptance. Keep `j53m` open.

## v21 outcome-fork follow-up — 23 October 2026

The v21 prompt follow-up made mutually exclusive success and failure results a
visible fork at the operation result, so they cannot be presented as
consecutive events. This follows the observed v19 opening: its code-backed
outcomes were present in the scene data, but the visible viewport showed the
preceding vertical sequence before the two destinations. The app continues to
render typed scene data with accessible, source-linked primitives; it does not
execute model-authored JavaScript or Three.js. Prompt assertions pass, but no
Guide has yet qualified v21.

## v22 selector readability follow-up — 23 October 2026

The current source prompt is `ai-html-compositions-v22` / `meaning-first-v26`.
The v19 Fontdone run's second scene places six source-ordered probes in one
selector and repeats the same generic scene label on each guard and
destination. v22 caps a selector at three checks per scene, continues longer
chains in ordered scenes with an explicit miss handoff, and requires each guard
and destination to have a short distinct label. It preserves the final
fallback and does not execute model-authored JavaScript or Three.js.

Prompt assertions pass; no Guide has yet qualified v22. The approved v19
Fontdone run has now published all five planned topics, but its outline still
selects only the Rust memory-face route. It explicitly excludes C ABI and WASM
surfaces, the public glyph-loading path and final rendered output, so it cannot
establish full-repository coverage. Keep `j53m` open for source-grounded visual,
whole-repository coverage, audio and comprehension gates.

## v23 visual direction follow-up — 23 October 2026

The user asked for the Guide to generate richer, case-specific visual
representations rather than defaulting to blocks or graphs, including 3-D only
where it helps. The current policy is now `ai-html-compositions-v23` /
`meaning-first-v26`. It asks the model to begin each scene from a plain-language
visual idea, center one focal object/relationship/consequence, use different
semantic forms when the behavior changes, and compose the available responsive
HTML primitives around the real source behavior. It retains the typed,
app-owned renderer and disallows executable model markup or 3-D code; 3-D stays
limited to source-grounded spatial behavior with a tested native renderer.

Prompt-contract assertions were updated. The completed Fontdone Guide used the
old prompt and cannot qualify v23. A fresh Guide was run under v23 in the
rebuilt app; its result, rendered scene variety, and scope limits are recorded
below. Do not claim Guide or human visual acceptance from the prompt change or
build alone.

## v23 Fontdone evaluation and responsive layout check — 23 October 2026

One user-approved fresh Guide ran with `gpt-6-luna` at max reasoning against
Fontdone `main@5249b72371`. It ended `partial`: the outline chose a Rust
in-memory font route and omitted C-ABI/WASM entry points, full format dispatch,
glyph loading, hinting, rasterization, and detailed failure paths. Three of
five topics were marked explored; saved material exists for four, with one
topic partial and one unavailable after `proposalCorrectionLimit`. This run
does not establish whole-repository coverage or reliability.

The generated scenes did vary by behavior, including a timeline, panels, and
branch decisions. The PCF type-byte scene is a useful fit for a decision fork,
but it repeats the generic “String-type byte” label for distinct invalid and
integer outcomes. The open-options path also used branches for a sequential
open-then-setters path, where a timeline would communicate the order better.
So the prompt is influencing composition, while visual specificity and
behavior-to-form matching still need work. These scenes are typed data rendered
by the app-owned HTML scene system; the model does not emit executable HTML,
JavaScript, or Three.js. No 3-D scene was warranted by this font-processing
example.

Reviewing the PCF scene in the rebuilt packaged app exposed a separate reader
issue: branch outcomes stacked at the normal split-pane width because the
responsive breakpoint was wider than the available scene canvas. Live and
offline scene styles now keep branches side by side until 480 px, and move row,
compare, timeline, transform, and queue layouts to narrow forms at widths that
match the actual content area. The packaged reader now shows the two PCF
outcomes side by side at normal width. The visual-story test file has a
regression assertion for live and offline thresholds. Focused tests passed
(47/47), and `npm run bundle:macos` produced a verified app bundle and DMG.

Keep `j53m` open. Next gates are better distinct labels and sequence-form
selection in model output, broader repository coverage, and human
audio/comprehension/accessibility review. This single Guide is evaluation
evidence, not acceptance.

## v24 visual form-fit repair — 23 October 2026

The v23 Guide exposed two cases where the existing directions were not
specific enough: it repeated “String-type byte” for different PCF destinations,
and represented an ordered open-then-setters path as branches. The source prompt
is now `ai-html-compositions-v24`. It states that sequential calls remain a
timeline/linked path even when calls can fail, with an error exit only at the
source-backed early return; a branch requires mutually exclusive destinations
from one evidenced guard. Branch conditions are labeled once and each
destination names its behavior/result. The prompt version is included in the
progressive operation contract, so the next fresh run will not replay v23.

Prompt-contract assertions were added. The user-approved v24 Guide completed
partially; its final evidence and the next prompt repair are recorded below.
Keep `j53m` open for complete repository entry-point/path coverage, real
source-click and offline review, accessibility, and human comprehension/audio
evidence.

## v24 packaged Guide result and v25 prompt follow-up — 23 October 2026

The user-approved v24 Guide operation `c2e33321-45b5-423d-b624-b57af220fbc2`
uses `gpt-6-luna` / max on Fontdone `main@5249b72371`. Its six-unit outline
selects the Rust memory-face to glyph-loading path and explicitly omits C-ABI
and WASM entry points, the name-options variant, full `Font::memory_face`
dispatch, an exact scaler handoff, and complete renderer outcomes. It completed
partially: the opening and four of six follow-on topics were saved; two topics
failed validation after three attempts each (scaler hint outcomes and slot-render
handoff). This is a partial Rust-path guide, not whole-repository coverage.

The first slide shows useful form-fit examples: an input-to-parser timeline, a
PCF match/miss fork with one guard and distinct destinations, and the API's
error/success result fork. Clicking the PCF guard opens the captured
`src/font.rs:4058` implementation. Narration describes the behavior in natural
language; Pocket audio was generated and played locally but not human-reviewed
for pronunciation or comprehension. The sequential handoff timeline repeats
generic node names as visible values and puts the actual argument changes only
in help text. Its flag-normalization scene stacks six oversized rules; the list
is clipped at the normal reader viewport. These examples show that v24 improves
branch and sequence selection, but visible-value specificity and compact
composition still fail in places. Playback buffering refilled the next Guide
topic after local narration began, confirming that this run exercised
progressive generation. It does not prove latency, resilience, or broad
reliability.

The source prompt is now `ai-html-compositions-v25`. It requires visible values
to carry the source-backed input, condition, change, output or destination;
help/meaning text stays supplementary. It rejects repeated generic label/value
pairs and asks for a compact table or short scenes when four or more sequential
state updates would become a tall card stack. The v25 unsigned debug bundle is
installed and open in Code Atlas; the v24 app remains available as a rollback
copy. The saved Fontdone Guide data was preserved. A fresh v25 Guide evaluation
is recorded below; it remains incomplete and does not qualify whole-repository
coverage, audio, or human comprehension.

## v25 Fontdone visual-prompt evaluation — 23 October 2026

The user-authorized Guide `2000ce69-a16b-417a-8d0b-6bbe7728d77d` used
`gpt-6-luna` / max on Fontdone `main@5249b72371`. Its five-topic outline follows
the safe Rust `Library::new_memory_face` route, and explicitly leaves out the
separate `Font::truetype/getmask` path, C-ABI/WASM facades, parser internals and
the full renderer/rasterizer implementation. At the latest observation, two of
five topics were complete. The glyph-request topic failed after two attempts,
the glyph-dispatch topic completed after two attempts, and render-result was
still building; caller-outcome had not started. The run is not complete and
cannot establish whole-repository coverage.

On the next check, render-result had reached three rejected proposals with
`sceneAuthoringShape`, `sectionValidation` and `proposalCorrectionLimit`, but no
section was published. The Guide coordinator remained active while these
details were recorded, so there is still no terminal outcome. This is concrete
validation/recovery evidence, not a reason to start a duplicate generation.
The coordinator then retried render-result automatically in the same Guide. At
the latest check it had delivered nine bounded tool calls totaling 16,622
source bytes, but after 169 seconds it still had no proposal or published
section. The two completed topics remain saved; overall progress remains 2/5.

The completed face-opening section contains three source-grounded visual
scenes: an ordered timeline with an error exit and successful face result, a
PCF match/miss fork, and a parse-error/bitmap-face fork. The next saved scene
uses a short timeline followed by the parameter-specific `sbix` decision and
distinct rejected/success results. This is positive evidence that v25 can
choose different semantic forms for different behaviors instead of emitting
only a graph or a collection of cards. The Rust `?` error-propagation token is
shown as a source value, paired with visible “error exits” labels and narration
that explains the propagation; it is not an empty renderer fallback. This token
is still less immediately legible to a novice than a plain-language outcome.

The app displayed the section narration and a ready-to-play local Pocket track,
but no human listened to the audio in this review. No 3-D representation was
warranted for this font-processing path. The Guide remained active at the last
check, with the next topic building, so its terminal reliability and total
runtime are still unknown. Keep `j53m` open for completion, broader language and
entry-point coverage, audio/listening review, source navigation, accessibility,
and comprehension acceptance.

## v34 narration-only authoring contract and live Guide failures — 23 October 2026

The current source prompt is now `ai-html-compositions-v34`. The prior wording
said narration-only was valid, while the progressive tool schema and native
compiler required a nonempty source-linked node for every visual claim. The
shared story prompt and progressive prompt now specify the valid alternative:
use `kind:none` for the whole section and one `mode:narration` scene with empty
nodes, edges and elements for every claim. When a section chooses a visual kind,
every claim must instead have a nonempty scene with a source-linked node. Mixed
modes and empty visual scenes are explicitly invalid. The fixed correction
feedback now gives the same choice, so a rejected scene can be corrected without
discarding the claim or its evidence.

The provider visual-prompt suite passes 10/10 and the progressive-generation
suite passes 14 tests with two account probes explicitly ignored;
`cargo fmt --check` and `git diff --check` pass. This is source and prompt
evidence; the v34 packaged app has not been built or evaluated yet.

The user-authorized v30 packaged Fontdone Guide is still running against
`main@5249b72371`. At the latest read, it has 0 slides ready: the Rust face API,
C ABI stream and WASM input topics failed after their automatic second attempts;
the Rust example-input topic is building, and four topics remain pending. The
first Rust topic spent 397.709 s and 435.460 s across its two runs, making 11 and
10 retrieval calls respectively; neither published a section. The first run
ended with scene-limit findings; its retry found scene-shape, selector and
identifier-evidence findings. The C ABI topic spent 1,749.383 s and 277.071 s;
its two runs made 8 and 5 retrieval calls and likewise published no section.
Those attempts recorded selector/section-validation findings and exhausted
their proposal corrections. The WASM topic also failed both runs. The first
lasted 202.611 s with 12 retrieval calls; the automatic retry ended with
identifier-evidence and section-validation findings. The fourth topic is a
separate active run with five retrieval calls and 11,978 source bytes at the
latest measurement. Two proposals returned section-validation findings and no
section is published yet.

A later read-only app snapshot showed four topics failed, the parser topic
building, and the last three topics pending, with 0 slides ready. The Rust
example-input topic failed after its earlier active measurement. No retry or
stop was initiated; the Guide remains in progress.

This run shows that the v33 ordered-selector clarification did not prevent
validation failures in the first three repository topics. The v34 fallback is
not yet in a packaged build and cannot be credited with those outcomes. Do not
start a duplicate Guide or reset attempts. Preserve this run and its receipts;
qualify v34 only after this run reaches a terminal state. Whole-repository
coverage, rendered explanation quality, source clicks, audio listening,
accessibility and human comprehension remain open.

## v30 fresh Guide stopped without a section — 23 October 2026

The separate seven-topic Fontdone Guide
`c9028af6-e6e1-4a3e-bd9f-464d6d4ec5f5` was active in the installed v30 app for
`main@5249b72371`. Its active C-ABI operation
`396a0fdd-7114-4cde-a90f-533887013763` completed four evidence-tool calls and
delivered 3,000 source bytes, then produced no proposal or section for more than
ten minutes. The app still showed it as building, so Stop was used; the guide
and job record now both show `stopped`, with 0 slides and 0/7 topics explored.
Earlier saved explanations and receipts were preserved. No new request was
started.

This is a stalled-operation observation, not evidence that the model rejected
the visual schema: no proposal reached native validation. The runtime trace
contains no terminal stage explaining the idle interval. At this 23 October
checkpoint, source policy was `ai-html-compositions-v37` / teaching policy
`meaning-first-v27` at `a8a9891`; the installed v30 executable predates those
prompts and this run cannot qualify them. Preserve `ps02`'s timing gap and
`3bz5`'s incremental publication gap. Fresh rendered visuals, source-click behavior, narration,
accessibility and human comprehension still require qualification.

## Current source policy — 24 October 2026

The current policy constants are `ai-html-compositions-v42` and
`meaning-first-v28`, defined in `src-tauri/src/provider/visual_scenes.rs` and
asserted by the progressive contract tests. The revisions clarify behavior-to-
visual fit: a single guard uses a branch; only ordered, first-match candidates
use a selector; an actual source-backed state change uses a transform whose
operation is visible; and graph outcome edges attach to the operation or status
check that establishes them. Exact code names remain visible in claims and
source links while narration describes their role instead of spelling compact
format tags or identifiers aloud. The app still renders only its bounded,
typed composition vocabulary; generated executable markup and 3-D/WebGL are
not supported. See the [reader-facing scene contract](../VISUAL_SCENES.md).

These source/test-contract changes do not qualify the installed app. The v30
Fontdone Guide remains stopped and predates v42/v28; no fresh Guide or extra
provider request was run for this update. Keep `j53m` open for generated visual
quality, coverage, source navigation, narration/listening, accessibility,
offline export, and human comprehension.

The two focused Rust prompt-contract tests passed (**2/2**). The Open Source
documentation inventory ran with strict local-link checking and returned zero
errors, review items, or info items. These checks verify source/version and
documentation structure only; they do not qualify a rendered Guide.

## Packaged v42 deployment — 24 October 2026

The v42/v28 source was packaged with `npm run bundle:macos`. License and bundle
metadata checks, the production frontend build, optimized Rust build, app/DMG
bundling, and the final metadata verifier completed successfully. The verified
app was installed at `~/Applications/Code Atlas.app`; its executable SHA-256 is
`631493e52cda17bff3a7c8dc8c214d2103436bd837570011567eddc71acde7fb`. The
previous app bundle is retained at
`/private/tmp/code-atlas-before-v42-build-20261024-2303/Code Atlas.pre-update.app`.

After restart, Code Atlas reopened Fontdone `main@5249b72371`, restored the
saved Code Lens explanation and its six sections, and reconnected the existing
Codex connection without setup. No Guide or provider request was started. This
verifies package installation and saved-workspace restoration only; the v42
visual policy still needs a fresh generated Guide and human review. Keep `j53m`
open for that quality work and the remaining source-navigation, narration,
accessibility, offline-export, coverage and comprehension gates.

## Symbolic source values in static-code visuals — 25 October 2026

At this checkpoint, the story policy was `ai-html-compositions-v43`. When no concrete
runtime sample is available, it asks a scene to show the actual source variable
or field as a symbolic input/result instead of generic stand-ins such as
“Loaded input” or “Processed result”. It tells the model to mark symbolic values
when they might be mistaken for observed runtime data and forbids invented
coordinates, payload contents or measurements. The typed renderer and its
existing layout vocabulary are unchanged. `docs/VISUAL_SCENES.md` now documents
the same rule.

Verification: `cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
visual_scenes` passed 10/10; the progressive prompt-binding test passed 1/1;
`cargo fmt --check` passed. This verifies the current source prompt and contract
tests only. No Guide was generated and no provider request was made, so improved
rendered quality, source grounding, narration, accessibility and comprehension
remain unqualified. Keep `j53m` open.

## Coordinate-free point-flow visual — 25 October 2026

The story policy is now `ai-html-compositions-v44`. A new `pointFlow` scene is
available for source-proved point/vector mappings. It renders the cited input,
operation and result as symbolic point/function cues and does not invent
coordinates. The admission validator requires exactly three direct content
children, and the app owns the visual in both the live scene and offline export.
The progressive, correction and claim-alignment prompts describe when it is
appropriate.

The fresh Code Lens result for `src/tt/glyf.rs:564–583` under v43 still used a
generic three-stage transform for a composite-point mapping, motivating this
addition. It does not qualify v44: no new provider request has been made after
the policy change. Keep `j53m` open for generated-scene review, source
navigation, narration/audio, accessibility/reflow, export and comprehension
acceptance.

## Visual composition pass — 25 October 2026

The authoring prompt is now `ai-html-compositions-v45`. It gives the model an
explicit design pass before it emits the bounded scene tree: identify the code
behavior and one focal object or relationship, choose the narrowest semantic
visual, and check that the source-specific change remains visible with
narration hidden. It separates layout containers from visual meaning and
calls out generic dashboards, component inventories, collages, and
interchangeable card piles as failures. Evidence, schema limits, and the
app-owned renderer rules are unchanged. Three.js remains unsupported as
model-authored code; an app-owned 3-D form would require a demonstrated
spatial behavior and a renderer.

This is prompt-contract work only. No fresh Guide/provider request or TTS
generation was made, and the saved v30 Guide predates v45. Fresh rendered
quality, source navigation, narration/listening, accessibility, offline export,
and comprehension remain open. Keep `j53m` open.

## Packaged point-transform source round-trip — 25 October 2026

In the installed Code Atlas workspace on Fontdone
`main@5249b72371`, opened the saved one-scene visual
“How composite component points are transformed.” Its operation cue opened
captured `src/tt/glyf.rs:554–574`; the source pane moved to those lines, and
“Show explanation” returned to the same visual and scene. This confirms a
saved composition can open and return from its cited source without a new
provider request. The visual predates the v44 `pointFlow` form and still uses a
three-card transform, so it does not qualify the newer form or show that the
scene is sufficiently visual. Audio was not played. No fresh generation,
human comprehension, 200% reflow, keyboard, VoiceOver, reduced-motion,
offline-export or whole-guide quality check was performed. Keep `j53m` open.

## Cross-section coverage prompt — 25 October 2026

The current shared authoring prompt is `ai-html-compositions-v46` with teaching
policy `meaning-first-v29`. Before proposing another section for one answer, it
asks the model to compare the behavior, guard, consequence and visual focus
against already published sections. A new title or another rendering of the
same decision is not new coverage. Related outcomes should stay together when
one semantic scene can show the complete guard and result; an uncovered topic
must be named rather than replaced by a neighboring repeat. The app still uses
its bounded, inert composition renderer; arbitrary generated JavaScript and
Three.js remain unsupported.

One user-authorized packaged Code Lens request completed before this prompt
change. It published three one-scene sections with ready narration, then ended
partial and named point-attachment success/invalid-index outcomes as uncovered.
The source links opened captured code. This is a useful baseline for the
coverage problem, not qualification of v46: no new provider request or audio
playback was made after the prompt edit. Fresh visual-quality, listening,
accessibility, export and comprehension acceptance remain open.

## Per-claim visual modes — 25 October 2026

The shared story policy is now `ai-html-compositions-v47`. The v46 prompt
incorrectly required one visual mode for an entire section even though each
claim scene already carries its own mode and the reader supports both graph and
HTML-composition scenes. Ordinary and progressive generation now choose mode
per claim, allowing the forms to mix when the source behavior calls for it.
Narration-only remains an all-claim choice when no claim benefits from a
visual. A focused scene-contract test covers a composition followed by a graph
scene and passes (20/20 in `tests/scenePresentation.test.mjs`). Rust prompt
assertions and documentation were updated. Native Rust tests were not run
because free disk space is below 1 GB; no fresh provider generation or human
visual-quality acceptance was performed. Keep `j53m` open.

## Branch clarity and nested decisions — 25 October 2026

The shared prompt is now `ai-html-compositions-v49`. A read-only review of the
installed Fontdone Code Lens visual “Choose the component displacement” found
that its branch used “Argument interpretation” for both the guard and one
outcome, then compressed multiple nested cases into broad outcome cards. The
spoken narration preserved those cases, but the visual did not expose them
clearly at a glance. The prompt now requires one visible guard, distinct
source-grounded labels and results for each outcome, and explicit nested
guards or a follow-up scene naming the path condition. The shared prompt test
passes and `git diff --check` is clean.

This is a prompt-quality correction, not proof the model follows it. No new
provider request or audio playback was made. Qualify it on a future authorized
fresh generation and continue the source, Pocket listening, accessibility,
export, and human-comprehension gates; keep `j53m` open.

## Current narration and visual policy — 27 October 2026

The shared story contract is now `ai-html-compositions-v61`, with teaching
policy `meaning-first-v31` and reader-language policy `reader-language-v1`.
Scene speech may use up to three short sentences and 520 UTF-8 bytes; the
complete grounded section script is assembled from claims and remains bounded
to 20–4,000 bytes. The prompt keeps source behavior and cause-and-effect
narration central, chooses an app-supported composition or graph per claim
scene, and limits graphs to genuine non-linear relationships. Rendering still
uses the app's structured, inert HTML/CSS primitives; arbitrary JavaScript and
Three.js are unsupported.

The scene and schema docs now match these source constants and bounds. This is
documentation/source-contract alignment only: no tests, build, fresh generation,
audio playback, or human comprehension review ran. `j53m` remains open for
generated-content quality, source navigation, narration, accessibility, export,
and representative-repository acceptance.

## Current packaged visual availability failure — 28 October 2026

In the open packaged app on Fontdone `main@5249b72371`, Guide shows multiple
saved explanation cards with “This visual cannot be previewed here”; at least
two cards say no visual was saved. The active one-scene Code Lens story,
“How a composite glyph prepares component points,” instead reports “Saved
visual unavailable” while keeping narration and the source link available.
The app bundle is version `0.1.0`, with `visualSchema: 3` and
`visualRendererContract: 2` in its release metadata. The UI does not expose the
validator failure reason, and the source commit corresponding to this installed
bundle was not established. No saved artifact was edited, no audio played and
no provider request was sent.

This is direct packaged evidence that the currently selected workspace does
not render these saved visuals; it does not yet distinguish malformed saved
presentation data from a build/schema mismatch. The next diagnostic must trace
the local saved visual through `inspectInteractiveVisual` and the current scene
renderer, then preserve the rejected record while repairing the producer or
compatibility boundary. Do not hide this with a generic replacement diagram.
Keep `j53m` and packaged visual acceptance open.

## Saved visual schema mismatch — 28 October 2026

A metadata-only read of the named persisted section found chapter schema 5,
`visual.schemaVersion: 2`, and `visual.presentation.schemaVersion: 3`. The
current frontend `inspectInteractiveVisual` requires visual schema 3, so it
rejects the saved payload before validating or rendering the presentation.
This identifies the observed unavailability as a saved-version mismatch; it
does not prove fresh schema-3 output is rendered or useful. No chapter data was
rewritten. The current product has no legacy visual renderer: do not relabel
this payload as schema 3 or silently mutate it. Fresh current-schema generation
and an appropriate recovery path for this saved scene remain open.

The reader now distinguishes a positive but unsupported visual schema from a
malformed current-schema visual. For the former it says that the saved visual
needs refreshing and explicitly preserves access to narration and source links;
the saved chapter remains untouched. The Guide and answer preview share this
diagnosis. The two focused visual test files pass (57 tests). This improves the
failure state only; it does not generate a replacement visual or qualify fresh
schema-3 content, accessibility, export, or comprehension. Keep `j53m` open.

Rejected saved visuals now have a **Create a fresh visual** action in both
Guide and Flow, including malformed current-version payloads as well as older
versions. Sections with no saved visual by design do not show the action. It
starts a nested, section-scoped question using that section's cited source
context and asks for a new source-grounded visual; the existing explanation
remains in history. This is an explicit new request, not an automatic
migration or replacement. Focused visual tests pass (58), and `npm run build`
passes. A packaged-provider generation, actual-source review, audio,
accessibility, export and comprehension qualification are still open.
