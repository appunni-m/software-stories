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
