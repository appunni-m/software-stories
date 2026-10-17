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

1. Extend the existing owned MCP service with `propose_answer_section`, a strictly typed tool for one complete draft section. Reuse the existing Codex loop and immutable range reader. The app supplies operation, question, snapshot, account, runtime and grant binding; the model cannot choose another owner. A proposal contains a stable bounded section key, ordinal, title, claims, evidence IDs and visual under a versioned schema. It cannot execute code, widen source access or accept a chapter. Keep legacy five-tool request contracts independently verifiable.
2. Lead with a small section that directly answers the question. Investigate deeper dependencies afterward in the same owned thread. Limit this contract to six sections and eight proposal attempts in total, counting proposals within the existing 64 owned calls and the original ten-minute deadline. Rejected proposals receive bounded fixed findings through that tool result; normal agent decisions may use the remaining attempts. Do not start another investigation, reset a budget, introduce another provider or quietly replay a question. The existing final-response one-correction contract stays unchanged for legacy requests.
3. Resolve only delivered native evidence IDs, then run the complete existing claim, exact byte/hash/coordinate, diagram and storage validators before publication. A proposal with one invalid claim is rejected as a whole section. Never delete or rewrite a claim, guess a reference, remove an offending visual edge or append identifier names locally to obtain a passing section. Earlier source admission does not certify semantic accuracy or narration quality; those retain the independent quality gate.
4. Atomically persist each admitted draft section with stable operation/section identity, proposal ordinal, exact response digest, immutable evidence-ledger checkpoint and content digest before notifying the renderer. Define a versioned checkpoint receipt that remains independently verifiable when later calls append to the ledger. Unknown delivery stays uncertain; a lost reply cannot duplicate a section. Reusing a section key with different content is an explicit new revision, never silent replacement. Bound durable count/bytes and fail before destroying prior data when capacity is reached.
5. Show the first published section in the current exploration's Guide/Flow cards without taking focus, advancing playback or changing the selected section. Use one quiet working indicator for remaining work. Keep the file explorer, source pane, pinned composer, nested path and chat history available. Mark incomplete answers plainly with their missing topics; raw JSON, settings and internal tool stages stay outside the reader's main surface.
6. Finalize the question as complete, partial, stopped or failed independently from each published section's source-validation state and draft/accepted lifecycle. A final-response mismatch cannot overwrite an already persisted section. Stop, revoked grants, changed runtime/account or a missed deadline prevent every subsequent transmission/publication. Previously published drafts survive close, navigation, restart, quota failure and late callbacks. Restart reconciles stored state without inference; continuing requires an explicit new user action.
7. Record first complete published section, each later section, terminal outcome and narration readiness separately from first token/partial JSON. Use the existing saved Codex account and GPT-6 Luna/max. Pocket remains the voice connection; section publication must not trigger unreviewed narration generation or playback. Preserve original rejected outputs and separate new qualification artifacts.

## Acceptance

- Regressions cover first-section success followed by invalid citation/identifier/visual, malformed or duplicate proposals, key collisions, exhausted proposal and storage limits, lost replies, ledger append/checkpoint tampering, Stop before/after persistence, changed permission/account/runtime, deadline, restart and late delivery. A later failure preserves exactly the already published draft identities and bytes.
- Legacy saved chapters, pending previews, quote-based origins and five-tool receipts remain readable; no automatic migration launches inference. Native evidence references still prove only exact delivered source, not claim meaning.
- In the installed app, ask once using the saved connection. Read section one while investigation continues; open a diagram node and source citation, navigate a nested exploration and return to the same section. Exercise a later failure and restart without losing the first section or opening setup again. Verify keyboard/VoiceOver announcements and focus behavior separately.
- Run a new explicitly authorized frozen Fontdone attached-function and broad-repository qualification. Independently review actual inputs, ordered dispatch, face/index validation, missing-versus-malformed required-table behavior, outputs, important error branches, meaningful diagrams and the actual Pocket script/audio. The failed historical candidate is a regression fixture, not an accepted answer or quality oracle.
- The timing gate requires a useful independently admitted first section within 120 seconds under the recorded fixture/settings. First token, an unvalidated proposal, a schema pass, a cached unrelated answer or an omitted main answer cannot satisfy it. Keep the task open if this target or the content/navigation gates fail.

## Delivery order and boundaries

First define and test durable section/checkpoint admission; then integrate the owned proposal tool, background job events and existing Guide/Flow reader; finally qualify real generation and playback. No new app server/port, embedding service, agent framework, local model or API key prerequisite. Persistent cross-question agent conversations, full language/framework coverage, section editing and signed release remain separate tasks. This task is open until all of its gates pass.

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
