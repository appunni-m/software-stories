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
2. Lead with a small section that directly answers the question. For repository guides, persist a compact outline first, then fill the opening and each outlined topic through independently owned section jobs. Show the outline as soon as it is saved; publish and prewarm each validated slide as it is ready, and keep generating the next while the reader follows the current one. Never wait for the whole guide or an arbitrary five-slide buffer before showing the first useful slide. Keep the existing structural budgets (six unique sections/eight proposal attempts per answer job and 64 owned calls); these are not elapsed-time limits. Repository outline and slide jobs have no wall-clock deadline, including result validation and publication. Only an explicit Stop, app shutdown, changed account/runtime/grant, provider exit, or fixed source/call/storage limit can end a job. Rejected proposals receive bounded fixed findings through that tool result; normal agent decisions may use the remaining attempts. Do not reset a job's budget, introduce another provider or quietly replay an operation. Do not preserve final-response correction behavior from a superseded request path.
3. Resolve only delivered native evidence IDs, then run the complete existing claim, exact byte/hash/coordinate, diagram and storage validators before publication. A proposal with one invalid claim is rejected as a whole section. Never delete or rewrite a claim, guess a reference, remove an offending visual edge or append identifier names locally to obtain a passing section. Earlier source admission does not certify semantic accuracy or narration quality; those retain the independent quality gate.
4. Atomically persist each admitted draft section with stable operation/section identity, proposal ordinal, exact response digest, immutable evidence-ledger checkpoint and content digest before notifying the renderer. Define a versioned checkpoint receipt that remains independently verifiable when later calls append to the ledger. Unknown delivery stays uncertain; a lost reply cannot duplicate a section. Reusing a section key with different content is an explicit new revision, never silent replacement. Bound durable count/bytes and fail before destroying prior data when capacity is reached.
5. Show the first published section in the current exploration's Guide/Flow cards without taking focus, advancing playback or changing the selected section. Use one quiet working indicator for remaining work. Keep the file explorer, source pane, pinned composer, nested path and chat history available. Mark incomplete answers plainly with their missing topics; raw JSON, settings and internal tool stages stay outside the reader's main surface.
6. Finalize the question as complete, partial, stopped or failed independently from each published section's source-validation state and draft/accepted lifecycle. A final-response mismatch cannot overwrite an already persisted section. Stop, revoked grants or changed runtime/account prevent every subsequent transmission/publication. No wall-clock deadline may end an outline or slide job, including during result validation. Provider/transport failure may end its current attempt, but preserves every published draft and exposes a clear continue/restart action; time elapsed alone never triggers automatic retries or finalizes a partial guide. Previously published drafts survive close, navigation, restart, quota failure and late callbacks. Restart reconciles stored state without inference; continuing after the automatic recovery budget requires an explicit new user action.
7. Record first complete published section, each later section, terminal outcome and narration readiness separately from first token/partial JSON. Use the existing saved Codex account and GPT-6 Luna/max. Pocket remains the voice connection; section publication must not trigger unreviewed narration generation or playback. Preserve original rejected outputs and separate new qualification artifacts.

## Acceptance

- Regressions cover first-section success followed by invalid citation/identifier/visual, malformed or duplicate proposals, key collisions, exhausted proposal and storage limits, lost replies, ledger append/checkpoint tampering, Stop before/after persistence, changed permission/account/runtime, a guide operation continuing past ten minutes through result validation, restart and late delivery. A later failure preserves exactly the already published draft identities and bytes; elapsed time alone never fails or retries a guide job.
- The current guide schema, saved section receipts and source validators replay successfully. Unsupported superseded records fail closed and remain untouched; no compatibility path silently migrates them or launches inference. Native evidence references prove only exact delivered source, not claim meaning.
- In the installed app, ask once using the saved connection. Read section one while investigation continues; open a diagram node and source citation, navigate a nested exploration and return to the same section. Exercise a later failure and restart without losing the first section or opening setup again. Verify keyboard/VoiceOver announcements and focus behavior separately.
- Run a new explicitly authorized frozen Fontdone attached-function and broad-repository qualification. Independently review actual inputs, ordered dispatch, face/index validation, missing-versus-malformed required-table behavior, outputs, important error branches, meaningful diagrams and the actual Pocket script/audio. The failed historical candidate is a regression fixture, not an accepted answer or quality oracle.
- The timing gate requires a useful independently admitted first section within 120 seconds under the recorded fixture/settings. First token, an unvalidated proposal, a schema pass, a cached unrelated answer or an omitted main answer cannot satisfy it. Keep the task open if this target or the content/navigation gates fail.
- The outline is shown as soon as its plan-only job passes native citation and scope validation. The first validated slide is shown as soon as it is ready; later slides have explicit pending/working/ready/partial/failed states. Keep a maximum five-slide ready/active look-ahead buffer, fill the next eligible topic while audio plays, and do not wait to accumulate five slides before showing the outline or first ready slide.
- A single slide's quota error, invalid proposal or provider failure preserves the outline and every earlier/later saved slide. If no section was published, allow one fresh bounded automatic attempt; then record any remaining failure only on that slide and continue with unrelated eligible topics. Elapsed time is never a slide failure. Expose unresolved prerequisite context without treating a prerequisite as permission to block independent source research. A later explicit continuation cannot rerun the outline or ready slides.

## Delivery order and boundaries

First define and test durable section/checkpoint admission; then integrate the owned proposal tool, background job events and existing Guide/Flow reader; finally qualify real generation and playback. No new app server/port, embedding service, agent framework, local model or API key prerequisite. Persistent cross-question agent conversations, full language/framework coverage, section editing and signed release remain separate tasks. This task is open until all of its gates pass.

## Cross-window playback receipt handoff — source progress, 27 October 2026

Static review found that a playback-driven refill from a second app window could
reach the native guide while another window held the worker lease. The native
command correctly returned without claiming that consumed receipt, but the
active renderer had no way to learn about it. Guide summaries now include the
bounded consumed-revision list; a window that sees its receipt is still missing
sends a local Tauri event, and the active renderer queues that revision for its
existing post-run refill path. This preserves native idempotence and sends no
source or provider request for the handoff. No tests, build, or packaged
multi-window check ran. A closed or reloaded owner can still miss this transient
event, so durable receipt recovery and the packaged multi-window case remain
open; no completion criterion is closed.

## Incremental outline and slide runway — 21 October 2026

Before the incremental implementation checkpoint below, the guide planner returned `{outline, openingAnswer}` from one Codex operation. `repository_guide::plan::run` split the response only after operation and result validation; a timeout before that left no saved outline. The same worker loop then stopped on its first topic error.

The implementation now saves a cited plan first, then runs the opening and each planned topic as separate, one-section-bounded Codex jobs. A failed outline gets one automatic fresh planning run and one later user-triggered retry. A slide that fails before publishing gets one automatic fresh attempt; the guide then continues through unrelated units. Each slide retains a separate manual continuation after automatic recovery, and the manual action is withheld while its automatic retry is pending. Stop, permission errors or changed Codex identity still halt transmission. This was an intermediate checkpoint: the later user-controlled lifetime section below removes the per-run wall-clock limit too.

## Separate planning, first slide and topic jobs — 21 October 2026

The planner now returns only a cited outline and persists it before any narration/visual generation. The opening and topic jobs use separate operation IDs; the native proposal tool limits each guide job to one published section, and section events let the reader adopt it while the run continues. A failed slide with no publication gets one automatic fresh operation before a manual continuation appears, and unrelated slides continue afterward. The outline also gets one automatic fresh planning operation and retains one explicit retry. Stop, permission errors and changed Codex runtime halt further transmission. This intermediate checkpoint still had a finite per-operation deadline; the user-controlled guide lifetime below supersedes it.

- [x] Make every planned slide a one-section job with a native-enforced per-slide publication bound.
- [x] Save the outline before opening generation, retry a failed outline once automatically, and preserve a later explicit outline retry.
- [x] Retry a failed/empty slide once automatically with a fresh owner, then continue unrelated eligible topics while preserving a user-triggered continuation.
- [x] Expose per-slide pending/working/ready/incomplete states and bound the ready/active runway to five slides in the saved native guide contract and reader.
- [x] Add an explicit slide-only retry action with a fresh run identity; it leaves the outline, successful slides and unrelated pending units intact.
- [ ] In the packaged app, verify the outline and first validated slide appear without waiting for later jobs, and qualify slide-only retry at a full runway boundary.
- [ ] Verify the five-slide audio runway, slide-only retry, no-timeout continuation, Stop, permission revocation, changed Codex runtime, close/reopen and late callbacks against a real packaged app and a large captured repository.

## Slide retry and runway regression — 28 October 2026

The source already exposes persisted unit states in the Guide outline and limits
retry admission to one terminally incomplete unit with an available slot in the
five-slide runway. The reader's **Retry slide** action sends that exact unit key
to `retry_repository_guide_unit`; the native command takes a fresh run UUID,
leaves the outline and other units intact, and retains the original attempt.

Added focused native regressions for retry admission (terminal failure, Stop,
active owner, attempt limit, full runway and one consumed slide) and for a retry
appending a fresh attempt without replaying the plan, opening or another unit.
The same Guide suite exposed a stale test expecting an error when opening an
unsupported guide in its own snapshot. The reader contract reports that saved
record as unavailable instead; the assertion now checks the count and confirms
that the original bytes stay unchanged. Focused result: **27 passed, 0 failed,
1 live-account probe ignored**. The changed Rust file passes standalone
rustfmt; workspace-wide `cargo fmt --check` still reports unrelated existing
formatting differences in other files.

This is source and deterministic state-machine evidence only. Packaged retry,
initial/first-slide visibility, five-slide playback, large-repository content,
audio, accessibility and restart/failure acceptance remain open.

## User-controlled guide lifetime and visible recovery — 21 October 2026

The user clarified that a guide is an agentic process and must not fail just because it has run for ten minutes. Outline creation, individual slide research, native result validation and publication therefore have no wall-clock deadline; explicit Stop and fixed source/call/storage/identity protections remain. The outline appears first, then the opening and one planned slide are filled and published independently. The first validated slide is available immediately, with local Pocket prewarming while the next slide is researched; a five-slide look-ahead is optional and must never delay the first slide.

Packaged UI inspection found an existing pillow-rs card displaying the old exact ten-minute timeout message. A later binary check corrected the initial conclusion that this was only historical: the installed app at 16:15 had SHA-256 `854db36dba0c36f04abbf1e136e9b366ce4527ccb9c4ac5e4c5cfccab0936a58`, while the source bundle built at 17:50 had SHA-256 `b995bf3046ca8f7023d1ea505261f0294d2af3a397b2585159857d357166ab5f`. The installed app predated the project-guide routing fixes, so it was stale. The old bundle has now been kept as a rollback copy and replaced by a fresh bundle from the current source; the installed executable hash matches the rebuilt bundle, and the app opened with the Codex connection restored. No repository was selected after launch. No live repository request was sent. The workspace labels planning, the opening slide and the next slide with their ready count, and offers "Start a fresh guide" when an old owner cannot resume. Starting fresh creates a separate guide record; it does not rewrite, delete or implicitly replay the old record.

- [x] Use user-controlled lifetime for outline, opening and slide Codex runs through result validation and native publication.
- [x] Show phase-specific guide progress and let a terminal guide card start a separate fresh guide when its saved owner cannot resume.
- [x] Route explicit project-wide guide requests from Guide or Flow into the durable repository-guide planner instead of the ordinary ten-minute answer command. Ordinary nested follow-ups remain focused.
- [x] Bound each planned Guide operation to one published slide. Publish the outline and first slide as soon as each is ready, then fill a five-slide runway; when the reader finishes narration or advances past a slide, save its revision receipt and generate one next slide. Duplicate playback/Next events cannot spend a second slot.
- [x] Persist a waiting-for-playback guide state between buffers and keep playback-driven generation user-controlled with no wall-clock cutoff. Fixed account, grant, source, proposal, call and storage guards remain active. No real repository request was sent for this code change.
- [ ] In the packaged app, verify the saved outline and each slide remain visible/playable while the next is generated, local narration prewarming stays nonblocking, and Stop ends the active job.
- [ ] Qualify the five-slide audio runway, slide-only retry, provider/transport failure recovery, account/grant changes, close/reopen and late callbacks on a large captured repository. The runway may hold at most five ready/active slides but must never gate the outline or first validated slide.

## Project-wide chat routing and playback-paced refills — 21 October 2026

An unscoped repo-wide prompt sent from Guide chat could previously enter the generic answer worker and hit its ten-minute deadline, despite the dedicated repository-guide command already using the agent-owned, user-stoppable deadline policy. Matching project-wide Guide prompts now use the dedicated planner; file-scoped and nested questions still use the focused answer path. Each guide run is constrained to one unique published section, so the five-slide ready runway has an actual section bound. The outline and any first useful slide still publish immediately; the reader is never held back to fill the runway.

When the last narration scene finishes, or the reader advances beyond the current slide, the UI submits the stable revision identity. The native record validates and deduplicates that receipt, saves a `waitingForPlayback` state at the buffer boundary, and permits one new slide operation for each consumed slide. The waiting card no longer offers an unbounded “Continue guide” action that could bypass pacing; its last-slide control says “Build next slide.” Native state, not a browser-only counter, owns the progression receipts.

Source regressions cover project-wide chat routing, exact one-section guide contracts, stable slide counting, receipt validation, waiting-state persistence, and completed narration progression. Frontend and native checks are recorded with this implementation. The packaged app must still be observed through one large-repository run before accepting the five-slide buffer, local Pocket readiness, Stop/failure/restart, account/grant changes, or user-visible latency.

## Wording-safe project-guide routing — 21 October 2026

A user reported the exact native “Codex operation exceeded ten minutes, including result validation” failure while requesting a project guide. That error is emitted only by a run using `RunDeadline::TenMinutes`; guide planner and slide runs use `RunDeadline::UserControlled`. The Guide composer missed common phrasings such as “Explain how this repository works,” and a selected file could also disqualify an otherwise project-wide ask, sending it through the ten-minute one-answer route. The router now recognizes these project-level phrasings and no longer lets an incidental file selection override them. File-specific questions and nested follow-ups remain focused. The consent copy now explains that the outline is saved first, the opening slide appears as soon as it is ready, and up to five slides may be prepared ahead while listening proceeds.

This closes the routing regression in source; it does not prove that an already-open older app bundle contains the fix. The packaged app still needs a fresh large-repository run to confirm the outline and slide events appear during generation and the guide stays alive until completion or explicit Stop.

A focused native regression now advances the guide's start clock by eleven minutes and verifies the Codex result-validation gate still accepts it under `UserControlled`; the ordinary `TenMinutes` policy remains separately guarded.

## Project-wide routing from Flow and nested conversations — 21 October 2026

The reported ten-minute error can still occur if a repository-wide request is submitted from Flow or while the reader is inside a nested exploration: the composer previously invoked the durable planner only from the Guide tab, and the classifier rejected every nested request before checking whether it explicitly asked for a project guide. Those requests fell through to the ordinary single-answer worker. Explicit project-wide guide requests now enter the same durable planner from either tab and any exploration depth. Narrow nested questions such as “How does this function work?” still use the focused answer path. Regression coverage includes explicit nested project-guide phrasing and confirms ordinary nested project questions stay focused.

The incremental contract remains: save and show the cited slide outline first, publish the first validated opening as soon as it is ready, then fill independently generated slides while playback proceeds, with at most five slides ahead. The five-slide buffer does not gate the outline or first slide, and elapsed time does not fail a guide. Packaged-app acceptance on a large repository remains open.

## Project-purpose wording regression — 21 October 2026

The reported ten-minute failure can also be triggered by **“Explain what this project does”**. The explicit guide and “how it works” patterns did not recognize project-purpose questions, so the composer sent them to the ordinary one-shot answer worker, whose result-validation policy still has a ten-minute deadline. Added project-purpose routing for forms such as “What does this repository do?” and “Tell me what the codebase provides,” with regressions; narrowly scoped implementation questions remain on the focused-answer path. This closes the identified wording gap in source only. A new installed large-repository run is still needed to verify the user's phrasing reaches the saved outline and progressive slides.

## Refill while a slide is playing — 22 October 2026

The five-slide runway previously requested its next slide only after narration ended or the reader advanced manually. The Guide now submits the slide's stable revision receipt as narration starts, so the next eligible slide can build during playback. Completion and Next still submit the same receipt; native receipt validation deduplicates those events. The first outline and first ready slide remain visible immediately, and no five-slide accumulation gates playback.

- [x] Notify the native guide when narration starts so playback overlaps with the next slide operation.
- [ ] In the packaged app, confirm playback-start refill works at the five-slide boundary, duplicate start/completion/Next events admit at most one slide, and no more than five ready/active slides are maintained.
- [ ] Re-qualify Guide generation, Pocket readiness, Stop, provider failure, account/grant changes and close/reopen on a large captured repository. Do not accept based on the source change or an existing partial guide.

## Negated broad-scope prompt routing — 22 October 2026

A focused one-section request that said it was not a repository overview was misclassified by the repository-guide keyword heuristic. The current app created a guide attempt but published no section. The frontend classifier now removes explicitly negated broad-scope phrases and keeps explicit single-section/file/function/branch requests on the focused path, while affirmative whole-repository guide requests still use the planner. Regression cases cover both classes. No follow-up account request was sent; large-repository routing and incremental delivery remain unqualified.

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

## Fresh incremental Guide evidence — 22 October 2026

The packaged v14 Fontdone guide has an outline, but its first useful section
arrived about 205.5 seconds after generation began (61.368 seconds for the
outline, followed by a 144.172-second opening operation). The opening is
independently saved and cited; it shows the borrowed-byte boundary and first
parser probes. The first follow-on topic (`parse-font`) returned a final
manifest without publishing a section twice. The next topic (`load-glyph`)
did the same after 308.925 seconds and entered the built-in retry. At this
checkpoint that retry is still building, while the UI retains the opening
section. This demonstrates preservation and continued work, but fails the
120-second first-section gate and does not establish useful incremental
progress for follow-on topics.

Source prompts now explicitly say that a guide slide's normal success requires
one accepted section before its final manifest; an incomplete-topic manifest
cannot substitute for the slide when delivered code supports a narrower
answer. This prompt-only change needs packaged qualification. Keep the fresh
run's later retry intact and retain all failed operation receipts.

## Fontdone whole-guide qualification and smaller slide budgets — 23 October 2026

The user-authorized v15/v23 packaged run is `852ab40c-3fac-42c8-a191-05a63b67fb67` on Fontdone `main@5249b72371`, GPT-6 Luna/max. Its six-unit outline and published sections remain saved. The first planned unit failed twice after three proposals each. The `glyph-entry` unit published one four-scene section. The `svg-bitmap` unit reached `sceneAuthoringLimit`. The user later approved a fresh run for the updated prompt; the v15/v23 run was stopped in the app, with saved sections and receipts retained.

The accepted glyph scene uses row/branch/branch/row layouts, but every leaf
still displays a raw source expression and the narration speaks method and
error identifiers. The rejected constructor and SVG proposals also show that
one unit can contain too many claims or evidence nodes for the 12-node section
budget; validator repairs then consume the per-key proposal budget.

The packaged v17/v25 app is now under whole-repository evaluation. It retains
readable labels, literal-only values, complete input/outcome coverage and
role-based speech from v16, and requires selector roots to own their guarded
branch groups as direct children. Its first opening section follows the
WebAssembly face-opening route in four scenes, including the error-slot and byte
pointer guards and the loader result/handle. The scenes use ordered path and
decision layouts, with plain-language narration tied to visible outcomes. The
Guide is still building later slides. This is initial evidence only; keep visual
quality, native rendering, source-link accuracy, Pocket listening and human
comprehension open until the full run has been reviewed.

## Fresh packaged v17 incremental Guide result — 23 October 2026

The approved Guide `79461b1a-e33d-40d2-be00-0ad86abec96a` reached native
`ready` with seven persisted slides: an opening and six separately generated
WASM topics. The packaged reader advanced through them while displaying the
next-slide progress indicator; the earlier slides remained available while
later work continued. The native record preserved all six topic outcomes and
their accepted sections. This confirms incremental publication and recovery
for this run, not a general reliability or latency pass.

The final bitmap-accessor topic failed twice after successful agent responses
but native visual/section validation rejected the proposals. The second
Guide-level attempt exhausted its correction limit; a third attempt published
one complete section. Per-run elapsed times were 165.469 s, 116.027 s, and
78.298 s. The opening and first planned unit each took over three minutes at
the provider-operation level (223.537 s and 292.095 s); these measurements are
not UI-visible first-useful-section latency. The completed plan still covers
only one representative WASM glyph path, explicitly excluding native Rust and
C ABI paths, so it does not satisfy broad repository coverage. Audio showed
Ready to play but was not listened to. See task `j53m` for the scene, narration,
source-link and scope review. Keep `3bz5` open.

## Guide lifetime regression check — 24 October 2026

The current full-Guide outline and per-slide route use `RunDeadline::UserControlled`; native source, tool-call, proposal and storage budgets remain bounded, and the user can stop the run. Focused regressions passed at HEAD `2fe8b16`: `cargo test --lib ten_minutes` (3/3), including continued evidence-ledger work, validation and publication after eleven minutes; and `cargo test --lib no_wall_clock_deadline` (3/3), covering the Guide run guard, outline planner and progressive slide contract. The current Repository Guide route is therefore not subject to the ten-minute timeout seen in the historical operation. This does not prove a fresh packaged whole-repository Guide, timely progress, answer quality or stop/restart recovery; no provider request was made. Keep `3bz5` open.

## Current Codex operation-lifetime regressions — 25 October 2026

At HEAD `6673846`, `cargo test --locked --manifest-path src-tauri/Cargo.toml
--lib codex_agent::` passed 20 tests; two installed-runtime probes were
explicitly ignored. The current suite confirms that a Guide run has no
wall-clock deadline, late validation and section publication remain eligible,
Stop is serialized with the complete native publication, and owner/deadline/
shutdown checks still guard publication. This is deterministic native test
evidence only; it does not prove a fresh app-server operation remains active
past ten minutes, packaged progress, latency, restart recovery, or answer
quality. No Codex account or provider request was used. Keep `3bz5` open.

## Partial manifest job-state alignment — 28 October 2026

A valid final manifest may finish the persisted answer as `Partial` while the
Codex operation itself returns successfully. The workspace job success path
previously marked every such operation `Completed`, disagreeing with the
answer record and restart reconciliation. It now maps the inspected terminal
answer outcome to the matching job state, including `Partial` to
`PartiallyCompleted`; a focused native regression passed at the current source
revision (`cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
final_answer_outcome_maps_to_the_matching_terminal_job_state`, 1/1). This is
source-level lifecycle evidence only. No provider request or packaged app run
was made, and the latency, reader/navigation, restart, accessibility, content
quality and audio acceptance gates remain open. Keep `3bz5` open.

## Refresh stale source-contract assertions — 29 October 2026

The progressive-generation tests still named an earlier visual policy phrase
and tested guide record schema 8 as unsupported after schema 8 became current.
Updated the assertions to follow the current prompt wording and reject prior
and future schema versions instead. The focused progressive prompt and guide
store regressions both pass. The route-planner test also now checks the current
app-authored opening-range wording, exact discovery guidance and `nextOffset`
gap rule; `plan_instructions_keep_the_full_requested_scope` passes. No Guide
was generated and no provider request was made. Packaged latency, multi-slide
playback, recovery, accessibility and content-quality acceptance remain open.
