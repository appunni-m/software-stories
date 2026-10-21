---
kind: task
key: progressive-answer-sections
parent: background-generation
title: Publish validated answer sections while the owned agent continues
priority: 1
labels: agents, reliability, performance, ux
---

## User outcome

Ask once and start reading a useful, cited explanation while the agent investigates the remaining parts. A failure in a later part preserves earlier published draft sections and clearly identifies the unfinished topic. The reader does not manage section generation, source selection or retries.

## Confirmed failure

The frozen Fontdone native-citation investigation returned three sections after 40 calls and 53,291 source bytes. All source references resolved, but the sole correction left one claim with inconsistent identifier/evidence alignment. The whole answer was withheld after 358.714 seconds. A separate source review found an inaccurate distinction between missing required tables and tolerated malformed contents. [Preserved result and review](../acceptance/2026-10-17-native-citation-references.md). Do not relabel this failed run as a published partial answer.

## Required implementation

1. Extend the existing owned MCP service with `propose_answer_section`, a strictly typed tool for one complete draft section. Reuse the existing Codex loop and immutable range reader. The app supplies operation, question, snapshot, account, runtime and grant binding; the model cannot choose another owner. A proposal contains a stable bounded section key, ordinal, title, claims, evidence IDs and visual under a versioned schema. It cannot execute code, widen source access or accept a chapter. Use the current owned protocol only; do not add compatibility paths for superseded request contracts.
2. Lead with a small section that directly answers the question. For repository guides, persist a compact outline in its own bounded planning operation, then fill the opening and each outlined topic through independently owned bounded section jobs. No outline, opening, every topic, narration, and final validation may be required from one Codex operation. Keep each job within six sections/eight proposal attempts and the existing 64 owned calls/ten-minute operation cap; the cap applies to one job, never to the overall guide. Rejected proposals receive bounded fixed findings through that tool result; normal agent decisions may use the remaining attempts. Do not reset a job's budget, introduce another provider or quietly replay an operation. Do not preserve final-response correction behavior from a superseded request path.
3. Resolve only delivered native evidence IDs, then run the complete existing claim, exact byte/hash/coordinate, diagram and storage validators before publication. A proposal with one invalid claim is rejected as a whole section. Never delete or rewrite a claim, guess a reference, remove an offending visual edge or append identifier names locally to obtain a passing section. Earlier source admission does not certify semantic accuracy or narration quality; those retain the independent quality gate.
4. Atomically persist each admitted draft section with stable operation/section identity, proposal ordinal, exact response digest, immutable evidence-ledger checkpoint and content digest before notifying the renderer. Define a versioned checkpoint receipt that remains independently verifiable when later calls append to the ledger. Unknown delivery stays uncertain; a lost reply cannot duplicate a section. Reusing a section key with different content is an explicit new revision, never silent replacement. Bound durable count/bytes and fail before destroying prior data when capacity is reached.
5. Show the first published section in the current exploration's Guide/Flow cards without taking focus, advancing playback or changing the selected section. Use one quiet working indicator for remaining work. Keep the file explorer, source pane, pinned composer, nested path and chat history available. Mark incomplete answers plainly with their missing topics; raw JSON, settings and internal tool stages stay outside the reader's main surface.
6. Finalize the question as complete, partial, stopped or failed independently from each published section's source-validation state and draft/accepted lifecycle. A final-response mismatch cannot overwrite an already persisted section. Stop, revoked grants or changed runtime/account prevent every subsequent transmission/publication. A per-job deadline ends that job; it does not cancel the guide, and one bounded fresh run may recover a slide that published nothing. Previously published drafts survive close, navigation, restart, quota failure and late callbacks. Restart reconciles stored state without inference; continuing after the automatic recovery budget requires an explicit new user action.
7. Record first complete published section, each later section, terminal outcome and narration readiness separately from first token/partial JSON. Use the existing saved Codex account and GPT-6 Luna/max. Pocket remains the voice connection; section publication must not trigger unreviewed narration generation or playback. Preserve original rejected outputs and separate new qualification artifacts.

## Acceptance

- Regressions cover first-section success followed by invalid citation/identifier/visual, malformed or duplicate proposals, key collisions, exhausted proposal and storage limits, lost replies, ledger append/checkpoint tampering, Stop before/after persistence, changed permission/account/runtime, deadline, restart and late delivery. A later failure preserves exactly the already published draft identities and bytes.
- The current guide schema, saved section receipts and source validators replay successfully. Unsupported superseded records fail closed and remain untouched; no compatibility path silently migrates them or launches inference. Native evidence references prove only exact delivered source, not claim meaning.
- In the installed app, ask once using the saved connection. Read section one while investigation continues; open a diagram node and source citation, navigate a nested exploration and return to the same section. Exercise a later failure and restart without losing the first section or opening setup again. Verify keyboard/VoiceOver announcements and focus behavior separately.
- Run a new explicitly authorized frozen Fontdone attached-function and broad-repository qualification. Independently review actual inputs, ordered dispatch, face/index validation, missing-versus-malformed required-table behavior, outputs, important error branches, meaningful diagrams and the actual Pocket script/audio. The failed historical candidate is a regression fixture, not an accepted answer or quality oracle.
- The timing gate requires a useful independently admitted first section within 120 seconds under the recorded fixture/settings. First token, an unvalidated proposal, a schema pass, a cached unrelated answer or an omitted main answer cannot satisfy it. Keep the task open if this target or the content/navigation gates fail.
- The outline is shown as soon as its plan-only job passes native citation and scope validation. The first validated slide is shown as soon as it is ready; later slides have explicit pending/working/ready/partial/failed states. Keep a maximum five-slide ready/active look-ahead buffer, fill the next eligible topic while audio plays, and do not wait to accumulate five slides before showing the outline or first ready slide.
- A single slide's timeout, quota error, invalid proposal or provider failure preserves the outline and every earlier/later saved slide. If no section was published, allow one fresh bounded automatic attempt; then record any remaining failure only on that slide and continue with unrelated eligible topics. Expose unresolved prerequisite context without treating a prerequisite as permission to block independent source research. A later explicit continuation cannot rerun the outline or ready slides.

## Delivery order and boundaries

First define and test durable section/checkpoint admission; then integrate the owned proposal tool, background job events and existing Guide/Flow reader; finally qualify real generation and playback. No new app server/port, embedding service, agent framework, local model or API key prerequisite. Persistent cross-question agent conversations, full language/framework coverage, section editing and signed release remain separate tasks. This task is open until all of its gates pass.

## Incremental outline and slide runway — 21 October 2026

Before the incremental implementation checkpoint below, the guide planner returned `{outline, openingAnswer}` from one Codex operation. `repository_guide::plan::run` split the response only after operation and result validation; a timeout before that left no saved outline. The same worker loop then stopped on its first topic error.

The implementation now saves a cited plan first, then runs the opening and each planned topic as separate, one-section-bounded Codex jobs. A failed outline gets one automatic fresh planning run and one later user-triggered retry. A slide that fails before publishing gets one automatic fresh attempt; the guide then continues through unrelated units. Each slide retains a separate manual continuation after automatic recovery, and the manual action is withheld while its automatic retry is pending. Stop, permission errors or changed Codex identity still halt transmission. Per-run time and source limits remain bounded; there is no single ten-minute deadline around the whole guide. This is still a partial implementation checkpoint, not full acceptance.

## Separate planning, first slide and topic jobs — 21 October 2026

The planner now returns only a cited outline and persists it before any narration/visual generation. The opening and topic jobs use separate operation IDs; the native proposal tool limits each guide job to one published section, and section events let the reader adopt it while the run continues. A timed-out or failed slide with no publication gets one automatic fresh operation before a manual continuation appears, and unrelated slides continue afterward. The outline also gets one automatic fresh planning operation and retains one explicit retry. Stop, permission errors and changed Codex runtime halt further transmission. The full journey keeps finite per-operation time limits; progress is checkpointed across runs instead of being lost to a whole-guide timeout.

- [x] Make every planned slide a one-section job with a native-enforced per-slide publication bound.
- [x] Save the outline before opening generation, retry a failed outline once automatically, and preserve a later explicit outline retry.
- [x] Retry a failed/empty slide once automatically with a fresh owner, then continue unrelated eligible topics while preserving a user-triggered continuation.
- [ ] Expose slide-level pending, working, ready and failed states, with a maximum five-slide ready/active audio runway; verify that the outline and first validated slide appear without waiting for later jobs.
- [ ] Add an explicit slide-only retry action. It must use a fresh run ID and leave the outline, successful slides and unrelated pending slides unchanged.
- [ ] Verify the five-slide audio runway, slide-only retry, timeout recovery, Stop, permission revocation, changed Codex runtime, close/reopen and late callbacks against a real packaged app and a large captured repository.

## Native storage checkpoint — 17 October

Implemented the private bounded section/proposal store and immutable checkpoint replay using the existing citation, claim, diagram and chapter validators. Exact repeats retain generation identity; changed content requires an explicit immutable revision. Preparation and compare-and-swap commit are separate. The full native suite passes 318 tests (11 live probes ignored), including 14 new section-storage regressions. These cover preservation after a later rejection, limits, stale writes, Stop/permission/runtime/snapshot/deadline changes, source delivery uncertainty and stored-content tampering. [Implementation evidence and exact integration boundaries](../acceptance/2026-10-17-progressive-section-storage.md).

The agent tool/catalog, shared native cancellation lock, background events, Guide/Flow rendering, draft adoption, restart/retention UI and real Fontdone/Pocket/accessibility/timing qualification remain pending. No progressive answer has been published in the installed app by this checkpoint. Keep this task open.

## Native protocol checkpoint — 17 October

Added the explicit six-tool session/helper, bounded `answerSectionV1` proposal dispatch, schema-3 evidence ledgers and schema-5 checkpoint origins while preserving legacy IDs, receipts and five-tool catalogs. The native manager binds publication to its actual run, request, account/runtime and cancellation owner; Stop and permission revocation share the commit lock. Successful notices/replies follow durable storage and final scope/deadline checks. A post-commit gate failure preserves the draft without sending a successful notice/reply. Synthetic socket tests exercise this path and verify the ledger's delivered response.

The complete native suite passes 326 tests (11 live probes ignored); frontend regressions pass 145 and the production frontend/native checks pass. [Protocol evidence, warnings and limits](../acceptance/2026-10-17-progressive-section-protocol.md).

The workspace command still uses the legacy final-chapter path. Progressive request/final-manifest handling, terminal outcome reconciliation, Tauri section events, normal draft adoption, Guide/Flow rendering during work, restart/retention UI and fresh installed Fontdone/Pocket/accessibility/timing qualification remain pending. Build and install the new helper before a real progressive probe. Keep this task open.

## Reader integration checkpoint — 17 October

Connected a separate progressive workspace command, exact final-manifest validation/retention, source-free section events, independently verified native chapter adoption and terminal/restart reconciliation. The production reader now selects this command, appends stable Guide cards during work, preserves earlier cards on later failure/Stop/retry, and offers explicit Flow section selection. Recovery does not reconnect or replay inference. A native worker lease protects preparation from false restart interruption. At the bounded storage error, an explicit retention action saves all section drafts before removing a terminal investigation receipt; native guards reject active or unsaved work.

The full native suite passes 334 tests (12 explicit live probes ignored); frontend 148 and the production frontend build pass. [Implementation evidence and qualification boundaries](../acceptance/2026-10-17-progressive-reader-integration.md).

Package/install/helper qualification, fresh Fontdone attached/broad semantic and timing acceptance, actual Pocket playback and installed keyboard/VoiceOver/focus/navigation/Stop/restart checks remain pending. Keep this task open.

## First installed synthetic checkpoint — 17 October

Built and integrity-checked the unsigned `2c7dda3` replacement, installed it and observed restoration of Fontdone's saved nested section and existing connection without setup. The first real public synthetic GPT-6 Luna/max run independently adopted sections at 26,211 ms and 70,403 ms, then returned partial at 76,252 ms. Keep its complete-answer assertion failed. The retained manifest exposed a 36-character schema bound for a native 47-character prefixed generation UUID; completion was correctly rejected while both drafts survived. The schema bound is fixed, with a dedicated regression; the full native suite passes 335 tests (12 live probes ignored). [Original failed artifact, diagnosis and limits](../acceptance/2026-10-17-progressive-reader-integration.md#installed-helper-and-first-live-synthetic-result).

A fresh fixed-build account qualification, installed progressive card/navigation/Stop/restart checks, Fontdone attached/broad semantic and timing acceptance, Pocket playback and keyboard/VoiceOver gates remain pending. No failed artifact has been overwritten or converted into a pass. Keep this task open.

## Fixed-build synthetic checkpoint — 17 October

The separate `f1dca7f` real GPT-6 Luna/max public synthetic run completed with both full generation identities. Independently saved sections were ready at 18,913 ms and 48,343 ms; the complete outcome arrived at 51,671 ms. Both proposals had zero findings and no adoption errors. Review confirms the greeting behavior and example against the fixture, with a three-node/two-edge transformation diagram. [Exact fresh result and qualification limits](../acceptance/2026-10-17-progressive-reader-integration.md#fresh-fixed-build-synthetic-qualification). This qualifies that small fixture's first-section timing and installed helper path; it does not qualify Fontdone, UI-visible timing, narration or accessibility. The earlier failed run remains failed. Keep this task open for the remaining gates.

## Native identity and proposal-feedback checkpoint — 17 October

A fresh installed Fontdone attempt exposed two further contract gaps. Four underscore section keys received generic shape findings; one narrower hyphen-key section saved at 92,556 ms but the UI rejected its native `generation:`-prefixed identity as a non-UUID. Computer use stopped the run at 103,143 ms; the stopped record retained its draft, while the UI incorrectly said nothing had published. [Exact stopped run, diagnosis and quality limits](../acceptance/2026-10-17-progressive-reader-integration.md#installed-fontdone-attempt-exposed-reader-and-feedback-defects).

The reader now accepts and preserves the actual native generation and supersession format. A regression round-trips real native section notices through stopped/restarted Guide state, and the synthetic fixtures use real native ID shapes. Invalid keys receive fixed field-specific `sectionKeyFormat` findings and the existing grammar is explicit in the agent instructions/tool description. No key/content is rewritten, and renamed rejected keys still leave their original proposals unresolved. Full native suite: 336 passed, 12 explicit probes ignored; frontend: 149 passed. Installed fixed-reader recovery and fresh complete Fontdone/Pocket/accessibility/timing qualification remain pending. Keep this task open.

## Installed recovery observation — 17 October

Built, integrity-checked and installed the reader-ID/feedback fix. Computer use observed the original stopped Fontdone section restored with its actual claim and controls, and its citation opened font.rs lines 4053–4078. Native answer records still contained only that stopped operation, so local recovery did not replay inference. The saved connection reconnected without setup. [Installed evidence and exact remaining limits](../acceptance/2026-10-17-progressive-reader-integration.md#installed-fixed-reader-recovery). Complete Fontdone content, meaningful diagrams, fresh UI-visible timing, Pocket and accessibility acceptance remain open.

## Live progressive Fontdone and precise feedback checkpoint — 17 October

A manual UI retry using installed source `2f9f138` published new sections at 71,337 ms and 166,389 ms, then ended partial with explicit SFNT-table and final-face-construction omissions. Computer use observed the first new card at 81,626 ms, followed a visual node to font.rs:4079–4096, navigated into an existing exploration and back while work continued, and observed the explicit Flow selection preserved when section two arrived and at partial completion. The earlier stopped section remained readable. The run used 22 calls and 20,739 source bytes, rather than a source dump. [Exact result and independent review](../acceptance/2026-10-17-progressive-reader-integration.md#fresh-fontdone-retry-useful-delivery-mechanics-incomplete-content).

This is not qualified answer quality or a first-useful-answer timing pass: the dispatcher claim omits the BDF constructor-error exception, the diagram has only two nodes/one edge, and narration drops validation details or retains raw underscored names. Complete required-table behavior and returned-face construction remain unfinished. Eight proposal attempts were consumed; several count/alignment failures had only generic feedback. These outputs remain unaccepted.

The checkout now provides bounded positional identifier-count/prose/evidence/citation findings without copying source or modifying claims; Flow starts with the question's evidenced diagram and honors an explicit choice; an existing answer without a diagram no longer falls back to unrelated file relations. The misleading accepted-revision visual footnote is removed. The full native suite passes 337 tests (12 explicit probes ignored), frontend 150. The first diagnostic suite had two assertions for the old generic feedback; they were updated to assert the new exact positions while retaining all no-publication/preservation checks, then the full suite passed. Fresh installed fix checks and real-account feedback/content/Pocket/accessibility qualification remain open.

## Final replacement observation — 17 October

The final diagnostic/Flow replacement was built, executable-integrity checked, installed and opened. Computer use confirmed restoration of the latest explicit Input validation and face-index failures Flow selection and graph, all three retained cards, the exact font.rs:4079–4096 source selection and the saved Codex connection. No new answer operation was created on restart. Full native 337, frontend 150 and production frontend/app builds pass. [Verification, executable hash and pending limits](../acceptance/2026-10-17-progressive-reader-integration.md#final-source-verification-and-replacement). This remains an unsigned debug build; fresh real diagnostic/content, broad Fontdone, complete narration and accessibility gates are still open.

## October 17 complete narration and visual-story follow-up

Checkpoint version 2 now retains all distinct grounded claim sentences in prose and speech, with whole-section rejection above 4,000 UTF-8 bytes; version 1 replay remains unchanged. The installed `7d6e335` Fontdone retry published one section in 59.040 s (UI first observed by 68.945 s), retained all four claims and a 12-node/11-edge branch visual, and ended partial after 12 calls/16,844 source bytes. The last read was rejected; its native cause was not retained, so no root cause or broad-answer pass is claimed. A BDF early-error exception is now explicit, but “all control bytes” is less precise than the actual below-ASCII-space guard. Pocket loaded a 72-second track; listening quality was not reviewed. Preserve the [original partial evidence](../acceptance/artifacts/2026-10-17-fontdone-complete-script-partial.json).

The current Guide implementation follows [narrated visual scenes](61-guide-flow-detail.md): collapsed script, a visual stage and audio-driven scene boundaries, without changing immutable saved answers. Current rendering, fresh story-generation, complete Fontdone, semantic depth, audio and accessibility evidence remain required; this follow-up does not close the task.

## October 17 HTML scene qualification

The complete synthetic HTML operation published in 19.950 s and finished in 22.351 s. A separate installed Fontdone PFR0-only operation published a complete scene at run elapsed 75.442 s, after three missing-title proposal rejections. A follow-up now provides fixed visualField positions and explicit required-field instructions; this diagnostic refinement has unit evidence only. Keep broader Fontdone completion, semantic accuracy and voice-quality gates open. See [results, preserved failures and remaining gates](../acceptance/2026-10-17-visual-story-guide.md). This task remains open.

## Broad read-diagnostics qualification — 17 October

The frozen Fontdone GPT-6 Luna/max native probe returned **partial** after 387.523 measured seconds (391.43 s for the test), publishing three independently adopted drafts at 99.936 s, 192.016 s and 341.735 s. All 39 calls delivered 123,317 distinct source bytes from five implementation paths. Five of eight proposals were rejected; the last outline-to-pixels proposal still had an invalid citation. The completion assertion failed. [Exact unchanged result, source review and remaining scope](../acceptance/2026-10-17-read-recovery-and-broad-answer.md).

- [x] Preserve the whole partial answer, rejected proposals, final manifest and evidence ledger in a fresh isolated qualification; compare captured metadata and all 20 published anchor byte ranges without accepting chapters or modifying the user's live project.
- [x] Reproduce a cross-claim HTML-scene node rejection and replace its generic tool feedback with fixed `sceneEvidence` at the element's `nodeIndex`. Preserve the failed proposal, native validation and correction within the same answer. Full native suite: 351 passed, 13 explicit probes ignored; frontend: 179 passed. The account artifact predates this diagnostic/prompt change and is not its fresh broad qualification.
- [ ] Evaluate section-attempt allocation and citation correction using this eight-proposal exhaustion. Define bounded retained storage, per-section correction opportunity and finalization reserves inside the original ownership/call/deadline policy; do not simply increase a cap or reset the investigation. Qualify a new broad run only after the implementation decision, preserving this failed completion check.
- [ ] Qualify the actual font-bytes-to-pixels narrative, meaningful transformations, original missing-entry/malformed-content distinction, natural complete narration/Pocket playback and installed reader/Stop/restart/accessibility. Repeated node labels, long helper names and a partial manifest do not pass these gates.

Keep this task open. Native/probe arrival times are not UI-visible useful-answer timing; no human quality/listening pass is claimed.

## Short-reference and packet-race checkpoint — 17 October

The eight-proposal failure is preserved unchanged. Before increasing proposal limits, the new progressive contract uses operation-local `code:N` references from `read_code`, while native digest identities, delivered-source checks, checkpoints and all budgets stay unchanged. Answer-record schema 2 / `answerSectionV2` replays short references; historical schema 1 / `answerSectionV1` remains readable without migration. Strict malformed/unavailable/tiny/unread/uncertain-source and overlap/reuse regressions cover admission. No local code selects substitute evidence or repairs an agent's claim.

A deterministic macOS queued-packet regression also exposed the intermittent native failure: setting a socket timeout after a peer closes returns OS error 22 while its complete packet remains readable. The owner now uses nonblocking I/O, preserving the 200 ms input/two-second output deadlines, Stop checks and size limits. The regression fails before the fix; the first fixed full suite passes 359 native tests (13 explicit probes ignored). [Contract, reproduction and remaining qualification](../acceptance/2026-10-17-short-native-references.md).

- [x] Choose the bounded citation-transport correction before changing section-attempt allocation; keep eight proposals/six unique sections/64 calls/ten minutes and retain the failed broad artifact.
- [ ] Qualify this exact implementation in a fresh isolated full Fontdone investigation. Preserve all rejected proposals and partial/terminal results, inspect actual source claims and scenes, and distinguish native arrival from UI-visible useful-answer timing.
- [ ] Complete the existing installed reader, complete narrated story/Pocket and accessibility gates. Short-reference reliability and local packet tests do not satisfy these.

Keep this task open. The fresh account run and current-bundle installation are pending.

## Fresh short-reference Fontdone result — 17 October

Source `502e2c2` returned Partial after 160.010 s (test 162.35 s, exit 101), with one draft published at 74.578 s and adopted at 74.951 s. Thirty-two of 33 calls delivered 46,172 source bytes across five paths; the last read is uncertain before payload preparation. Two proposals used the short-reference protocol: one identifier/evidence/prose rejection and one publication. All eight anchors independently match captured bytes and all nine input metadata hashes remain unchanged. There is no final manifest or broad completion. [Unchanged result, source review and limitations](../acceptance/2026-10-17-short-native-references.md#result-partial-completion-assertion-failed).

The worker discarded its actual terminal error when preserving this partial answer. The follow-up now retains only a fixed private `failureCode`, with unknown text mapped to `unknown`, no implementation diagnostic added to the UI summary and no replay. Four regressions cover section preservation, exact classification, terminal idempotence, historical absence and damaged/nonterminal metadata. The full native suite passes 363 tests (13 explicit probes ignored), frontend 179. This account artifact predates the diagnostic follow-up; the historical cause remains unknown.

- [ ] Reproduce the actual uncertain-call failure with retained native classification before attributing it to arguments, sockets, context size or a model. Preserve the failed completion; do not reset budgets or rerun automatically.
- [ ] Correct the missing dispatcher exceptions and repeated empty-card scene through the existing quality/visual tasks; the source-valid draft is not an accepted narrative.
- [ ] Complete the original broad answer, installed reader/Stop/restart, Pocket and accessibility gates. Keep this task open.

## Scene policy checkpoint — 17 October

New progressive sections now bind checkpoint version 3 to presentation version 2. Old checkpoints 1/2 remain immutable and readable; a fresh proposal cannot downgrade its scene policy. Fixed positional feedback preserves full rejected arguments and permits a corrected complete section within the same attempt budget. Private diagnostic retention from the prior checkpoint is unchanged. Native suite: 368 pass, 13 explicit probes ignored; frontend: 183 pass. [Evidence and remaining broad/native gates](../acceptance/2026-10-17-scene-v2.md). Fresh broad account completion and installed acceptance remain unproven; keep this task open.

## Fresh broad run and exact source-value recovery — 17 October

A fresh isolated `0650790` Fontdone run is Partial at 491.831 seconds: three drafts at 76.328/336.939/475.887 seconds, 61 delivered calls, eight proposals (five rejected). The final manifest explicitly retains render-mode/rasterization/bitmap publication as incomplete because the proposal limit was reached; no runtime terminal-failure code is present. Do not relabel the failed Complete assertion as a pass. All 38 anchors and nine input metadata hashes independently match. [Full evidence and limits](../acceptance/2026-10-17-scene-v2.md).

Two generic rejections were diagnosed as source-valued content outside its own node's same-claim excerpts. The later local fix provides fixed positional sceneSourceValue feedback without returning source text or changing authority/budgets. Real-artifact and full correction regressions pass; final suites 371 native/184 frontend. No account retry was run after that fix. Broad completion, native/Pocket/quality and useful-answer latency remain open.

## Fresh precise-feedback Fontdone result — 17 October

The new isolated `97e420d` GPT-6 Luna/max run is **Partial at 599.786 seconds**;
its Complete assertion remains failed. It delivers three drafts covering byte
entry, TrueType outlines/hinting and normal-gray rasterization/bitmap output.
CFF/CFF2/Type 1 loader internals and alternate render-mode/overlap internals
remain explicitly incomplete. Fifty-five calls deliver 127,466 distinct source
bytes from seven paths; six proposals include three full rejections/repairs.
The native source-value diagnostic is used and no generic sectionValidation
finding appears. All 29 anchors and nine input metadata hashes match.
[Unchanged result, timing, review and limits](../acceptance/2026-10-17-fontdone-source-value.md).

- [ ] Complete the original broad narrative within the original policy without
  relabeling this Partial result, omitting declared main scope or resetting caps.
- [ ] Correct semantic representation: `loca_data → glyf_data` falsely suggests
  a transformation between copied tables, and the overlap rasterizer scene lacks
  its visual condition. Most scenes are code cards, with dense spoken identifiers.
- [ ] Retain installed navigation/Stop/restart, meaningful causal narration,
  Pocket listening and accessibility gates. First native adoption at 85.391 s
  is not a UI-visible useful-answer or human comprehension pass.

No automatic retry occurred; previous failures remain preserved. Keep open.
