---
kind: contract
key: contract
title: Code Atlas — greenfield product contract
priority: 4
labels: container, product, contract
blocked_by:
---

## Product outcome
Code Atlas is a Mac-first desktop app that helps a developer understand an unfamiliar local repository quickly and accurately. Its default guide explains every eligible file individually, links every code-behavior claim to exact captured source, and lets readers inspect and navigate at their own pace.

## Product shape
Three primary destinations share one project and immutable source snapshot:

- **Guide:** repository orientation, an optional accepted repository tour, file-level progress, and direct next actions.
- **Files:** a repository tree with exact captured source, the selected file's explanation and clickable evidence; the relationship map is optional within this destination.
- **Changes:** local base/head comparisons and evidence-backed change explanations.

The section-by-section Walkthrough is the Guide's reading mode, with deliberate Previous/Continue controls and clickable source evidence. File-by-file coverage remains available inside Guide. Chapter Studio remains a contextual editing/review surface, not a primary destination. This approved direction is detailed in [`docs/design/PROPOSED_SIMPLIFICATION.md`](../design/PROPOSED_SIMPLIFICATION.md); task `13yp` tracks implementation and packaged-app parity.

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
