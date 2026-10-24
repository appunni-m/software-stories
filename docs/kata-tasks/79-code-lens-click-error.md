---
kind: task
key: code-lens-click-error
phase: M1
title: Reproduce and repair the Code Lens click error
priority: 1
labels: acceptance, bug, code-lens
related: design-parity, exploration-history, t08
---

## User outcome
Clicking a captured source file or Code Lens control either opens the exact source/range or presents a recoverable, actionable error. It must not crash the workspace or silently send source to an agent.

## Report to reproduce
The user reported that clicking Code Lens on a file threw an error. The running v30 Fontdone workspace is on `main@5249b72371`. Current UI inspection found no Code Lens-specific notification. The source-linked Guide hotspot opened `src/render.rs:425–446`, so that path works; it is not the reported click. Exact file path, clicked control, error text, and crash state remain unknown. Do not count source navigation as reproduction.

## Reproduction matrix
On the packaged app, distinguish these actions and record the exact result for each: open a captured file from Explorer; open a cited range from Guide; select code and choose **Explain this code**; open the legacy **Explain cited range** review if that screen is reachable. For each, record project/snapshot, relative path, inclusive line range, visible error, whether the app stays open, and whether a provider review was created. Start with Fontdone `main@5249b72371` and a captured text file.

## Contract
- Validate snapshot identity, captured-file membership, source hash, and range before opening or creating a provider review.
- A local review failure must stay in the app and explain what the user can do next; no uncaught error or unexplained dead click.
- Opening source/citations is local and read-only. Do not send code to Codex or another provider until the user explicitly accepts the reviewed request.
- Preserve the current exploration, Guide section/scene, source selection, and focus when opening and returning.
- Cover both supported Code Lens entry points, if both remain in the current product, without adding a legacy path solely for this test.

## Completion evidence
Reproduce the reported path or obtain the exact message from app diagnostics; fix the evidenced cause; add a focused regression for the cause and related invalid/stale/missing-source boundaries; then verify the packaged UI remains usable and returns to the same source/Guide location. If the original failure cannot be reproduced, record the tested controls and keep the user report unconfirmed rather than claiming it fixed. No model invocation is required for source navigation; any provider generation needs its own authorization.
