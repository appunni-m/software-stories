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
Three equally important workspaces share one project and immutable source snapshot:

- **Atlas:** repository tree and evidence-backed relationships.
- **Code Lens:** exact captured source, cited ranges, and related references.
- **Walkthrough:** accepted file chapters as manually paced, interactive sections.

Guide Plan and Chapter Studio are supporting workflows for coverage, generation, and review.

## Global interaction rules
- Every captured path appears in Guide Plan with one visible disposition and reason where applicable.
- Eligible files get an independent substantive chapter by default. Initial release has no automatic grouping or bulk generation; user-created groups never hide member files.
- Every code-behavior claim resolves to exact path, range, excerpt, hash, and selected snapshot. Inferences are labeled. Static relationships are not described as runtime facts.
- Every walkthrough section has Previous and deliberate Continue behavior. Hotspots have validated destinations. Back from source restores section and playback position.
- Each section/chapter revision is immutable after acceptance. Editing creates a new revision and only invalidates dependencies whose inputs changed.
- AI use is optional. The user explicitly chooses a supported direct model API or a separate coding-agent runtime; Google Gemini is one API option. Exact approved context, backend identity, applicable terms, and any multi-call behavior are reviewed before an outbound run. Opening a view or review screen sends nothing. TTS remains a separately disclosed capability.
- No local model requirement, hosted source proxy, telemetry, repository writes, script execution, paid fallback, silent regeneration, or automatic acceptance.

## Shared acceptance baseline
All implementation issues inherit the screen states, source/evidence rules, failure behavior, and exclusions in [`docs/PRODUCT_BLUEPRINT.md`](../PRODUCT_BLUEPRINT.md) and [`docs/TECHNICAL_PLAN.md`](../TECHNICAL_PLAN.md). When an issue conflicts with those contracts, update the product contract and affected issues before implementation continues.

## Done evidence
The product blueprint, technical boundaries, design concept images, complete milestone breakdown, and issue links are present and cross-referenced in this repository. Every implementation issue has an independent user outcome, interaction contract, acceptance criteria, exclusions, and evidence needed to close it. The repository files are versioned in Git and the live task state is tracked in Kata.

## Pending work — 2026-10-15

Pending: repo-level quality, local Pocket narration, section/revision/export acceptance, native reliability/release, local PR acceptance and verified agent adapters. See detailed child tasks; no feature milestone is closed by this static review.

## Core delivery gaps — October 15

The product review added generated visuals (`j53m`), repository narratives (`6qk9`) and qualified language support (`7xx7`). Chapter generation currently emits no visual payload, and existing file/overview generation is not evidence of a complete mixed-language narrative. Agent runtime delivery is split into Codex (`z3an`), Claude Agent (`8gqw`) and Antigravity (`m00g`); none is implemented in the app yet. Follow [the current pending work index](../PENDING_WORK.md) and [agent evaluation](../research/2026-10-15-agent-backends.md).
