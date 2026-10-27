---
kind: contract
key: contract
title: Code Atlas — greenfield product contract
priority: 4
labels: container, product, contract
blocked_by:
---

> **Latest workspace direction — 16 October:** the [persistent code/chat/exploration workspace](../design/WORKSPACE_EXPLORATION.md) supersedes earlier primary-navigation proposals. It keeps the explorer left, chat at the top, Guide / Flow / history within each exploration, inline code details, and background notifications. The new shell is a design preview; existing native behavior and prior observations below remain historical/implementation evidence, not proof of the new design.

## Product outcome
Code Atlas is a Mac-first desktop app that helps a developer understand an unfamiliar local repository quickly and accurately. Its default guide explains every eligible file individually, links every code-behavior claim to exact captured source, and lets readers inspect and navigate at their own pace.

## Product shape
One persistent VS Code-style workspace shares the selected project and immutable source snapshot. Its left activity bar contains **Explorer**, **Search**, **Changes**, and **Explorations**. A pinned chat composer stays above the content. Each exploration keeps **Guide**, **Flow**, and **Chat history** as views of the same conversation; source details open inline beside the active Guide or Flow view.

- **Explorer:** searchable repository tree and captured source. Selecting code or opening a citation keeps the current exploration in view; file explanation is a chat action at that level. The relationship map is optional context in Flow.
- **Search:** local search of captured paths, symbols, and source, with an **Explore with AI** action when the user wants an answer.
- **Changes:** local commit history and read-only base/head comparison with changed files, diff, and evidence.
- **Explorations:** the nested question history, with Back and root navigation restoring each level's saved view and source position.

Guide is an answer style: repository orientation can be a multi-section exploration, and file-level explanations appear as cards alongside code. The reader exposes deliberate Previous/Continue controls, source evidence, and optional narration. File coverage, Chapter Studio, settings, and map details remain contextual workflows, not additional primary destinations. The current authority is [`docs/design/WORKSPACE_EXPLORATION.md`](../design/WORKSPACE_EXPLORATION.md); [`docs/design/PROPOSED_SIMPLIFICATION.md`](../design/PROPOSED_SIMPLIFICATION.md) and its Guide / Files / Changes navigation are historical. Task `13yp` tracks implementation and packaged-app parity.

## Global interaction rules
- Every captured path appears in Guide's file-by-file coverage with one visible disposition and reason where applicable.
- Eligible files get an independent substantive chapter by default. Initial release has no automatic grouping or bulk generation; user-created groups never hide member files.
- Every code-behavior claim resolves to exact path, range, excerpt, hash, and selected snapshot. Inferences are labeled. Static relationships are not described as runtime facts.
- Every walkthrough section has Previous and deliberate Continue behavior. Hotspots have validated destinations. Back from source restores section and playback position.
- Each section/chapter revision is immutable after acceptance. Editing creates a new revision and only invalidates dependencies whose inputs changed.
- AI use is optional. The user explicitly chooses a supported direct model API or a separate coding-agent runtime; Google Gemini is one API option. Exact approved context, backend identity, applicable terms, and any multi-call behavior are reviewed before an outbound run. Opening a view or review screen sends nothing. TTS remains a separately disclosed capability.
- No local model requirement, hosted source proxy, telemetry, repository writes, script execution, paid fallback, silent regeneration, or automatic acceptance.

## Shared acceptance baseline
All implementation issues inherit the screen states, source/evidence rules, failure behavior, and exclusions in [`docs/PRODUCT_BLUEPRINT.md`](../PRODUCT_BLUEPRINT.md) and [`docs/TECHNICAL_PLAN.md`](../TECHNICAL_PLAN.md). When an issue conflicts with those contracts, update the product contract and affected issues before implementation continues.

## Done evidence
The product blueprint, technical boundaries, approved design proposals, milestone breakdown, and issue links are versioned here and tracked in Kata. The former Atlas / Code Lens / Walkthrough concept images are historical and are not the current primary-navigation contract. Task `13yp` records the remaining source-to-package parity work; it is not closed by these documents or by source-only edits.

## Pending work — 2026-10-15

Pending: repo-level quality, local Pocket narration, section/revision/export acceptance, native reliability/release, local PR acceptance and verified agent adapters. See detailed child tasks; no feature milestone is closed by this static review.

## Core delivery gaps — October 15

The product review added generated visuals (`j53m`), repository narratives (`6qk9`) and qualified language support (`7xx7`). Chapter generation currently emits no visual payload, and existing file/overview generation is not evidence of a complete mixed-language narrative. Agent runtime delivery is split into Codex (`z3an`), Claude Agent (`8gqw`) and Antigravity (`m00g`); none is implemented in the app yet. Follow [the current pending work index](../PENDING_WORK.md) and [agent evaluation](../research/2026-10-15-agent-backends.md).
