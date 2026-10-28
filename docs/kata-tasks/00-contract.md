---
kind: contract
key: contract
title: Code Atlas — greenfield product contract
priority: 4
labels: container, product, contract
blocked_by:
---

> **Latest workspace direction — 16 October:** the [persistent code/chat/exploration workspace](../design/WORKSPACE_EXPLORATION.md) supersedes earlier primary-navigation proposals. It keeps the explorer left, chat at the top, Guide / Flow / history within each exploration, inline code details, and background notifications. The workspace is implemented in source and has been inspected in a packaged app; that observation covers only its displayed build and state. Task `13yp` tracks remaining design parity and native acceptance.

## Product outcome

Code Atlas is a Mac-first desktop app for quickly understanding a local codebase. A developer asks a question or opens a file, then follows a concise, visual answer into exact source and deeper implementation detail. Explanations use captured code as truth, show evidence, and let the reader control narration and pace.

## Product shape

One persistent, VS Code style workspace serves the repository and every exploration:

- **Header:** current repository and snapshot, one-time connection status, appearance, search, and notifications. Connection details live in connection settings and should not be requested for each question.
- **Activity bar and explorer:** persistent Explorer, Search, Changes, and Explorations entry points; the file tree stays available while the reader follows an answer.
- **Pinned chat:** Guide me and Show the flow actions sit above the chat input. Search runs locally alongside typing; matching source appears directly, with Explore with AI available when a generated answer is useful.
- **Exploration content:** Guide, Flow, and Chat history are views of the same question and its answer. Guide presents visual and narrated cards, with prose collapsed when the visual carries the explanation. Flow shows verified relationships at the current level. Inline code details can open from either view.
- **Nested questions:** each follow-up is a child answer card. Back and the root breadcrumb return to an ancestor and restore its view and position. The parent shows real child links and counts.
- **Contextual surfaces:** Changes, Chapter Studio, and the interactive player are available when relevant to the current repository or answer; they are not extra setup steps in the main reading flow.

The authoritative interaction details are in [the workspace contract](../design/WORKSPACE_EXPLORATION.md). The browser preview illustrates the design; packaged behavior must be qualified separately. Task `13yp` tracks native implementation and packaged-app parity.

## Global interaction rules

- The Explorer lists captured paths. File and range explanations are available on demand; repository-level discovery selects relevant source automatically. Do not require a user-maintained reference-file checklist or create an explanation chapter for every path by default.
- Code is the source of truth. Documentation can suggest locations; flag it when evidence shows it is stale or deprecated. Claims about behavior link to exact captured source and snapshot. Inferences are labeled; static relationships are not described as runtime facts.
- The reader controls progression through Guide cards and narration. Source links open the cited range while preserving the selected answer and playback position. Back restores the selected ancestor without regeneration.
- Each accepted answer or chapter revision is immutable. Editing creates a new revision and invalidates only content whose declared inputs changed.
- Configure a connection once. Show its current state in the header; expose model, reasoning, and approval preferences in connection settings. Direct model APIs and coding-agent runtimes are distinct choices. A request sends only after explicit user intent and applicable source authorization.
- Keep indexing, code discovery, route planning, evidence validation, and asset reuse in the background. Normal reading views show useful progress and actionable outcomes, not internal orchestration details.
- No hosted source proxy, telemetry, repository writes, script execution, paid fallback, silent regeneration, or automatic acceptance.

## Shared acceptance baseline

All implementation issues inherit the screen states, source/evidence rules, failure behavior, and exclusions in [`docs/PRODUCT_BLUEPRINT.md`](../PRODUCT_BLUEPRINT.md) and [`docs/TECHNICAL_PLAN.md`](../TECHNICAL_PLAN.md). When an issue conflicts with those contracts, update the product contract and affected issues before implementation continues.

## Done evidence

The product blueprint, technical boundaries, approved design proposals, milestone breakdown, and issue links are versioned here and tracked in Kata. The former Atlas / Code Lens / Walkthrough concept images are historical and are not the current primary-navigation contract. Task `13yp` records remaining source-to-package parity and is not closed by documents or source-only edits.

## Historical pending work recorded 15 October 2026

That review listed repo-level quality, local Pocket narration, section/revision/export acceptance, native reliability/release, local PR acceptance and verified agent adapters. Its issue list and recommendations are historical; use live Kata and the [current task status snapshot](../KATA_STATUS.md) for current status.

## Current delivery status — 29 October 2026

- The persistent code/chat/exploration workspace and its file, search, source, Guide, Flow, history, Changes and notification surfaces exist in source. Packaged review covers specific builds and states only; remaining design and native acceptance are open in `13yp`, `3rcd`, and `x21h`.
- Repository Guides plan a bounded route and publish independently validated slides, with up to five ready slides buffered. Complete-repository quality, first useful answer timing, failure recovery and packaged navigation remain open in `3bz5`, `gk46`, `q0sm`, `6qk9`, and `a1sm`.
- Local Pocket TTS, distinct cache identity and generation-time narration prewarming exist in source. Voice quality, cache reuse and packaged listening acceptance remain open in `dbmd`, `tp6r`, `70x5`, and `q6g3`.
- Chapter-generation source supports visual payloads (chapter schema v5, visual schema v3). Source grounding, representation quality, comprehension and packaged acceptance remain open in `j53m`; language and narrative qualification remain open in `7xx7` and `6qk9`.
- Codex and Claude Agent adapters exist in source, but no coding-agent runtime is release-qualified. Codex runtime/operation acceptance remains open in `z3an` and `tpta`; Claude automatic generation remains unavailable behind its empty release-qualification registry (`8gqw`); the Antigravity automatic adapter remains unimplemented (`m00g`). Manual handoff does not qualify as an integrated adapter.
- The first Mac release is not ready to ship. Packaging, signing, notarization and clean-install acceptance remain open in `e42a` and `48yv`.

These are dated implementation-status notes, not acceptance claims. Each linked task retains its own closure evidence.
