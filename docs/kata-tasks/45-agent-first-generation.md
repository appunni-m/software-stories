---
kind: task
key: agent-first-generation
title: Make Codex the complete first-release generation path
priority: 0
labels: agents, product-quality, release-gate
blocked_by: codex-account-backend
---

> **0.1.0 scope — 8 November 2026:** Code Atlas is Codex-first. This task qualifies the installed Codex account path for code explanations and repository Guides. Claude Code, Antigravity, direct API catalogs, PR explanations, standalone export, and formal accessibility audit matrices are post-1.0 or out of scope. No elapsed-time SLA or route-wide batch builder is required. Keep ordinary keyboard navigation and readable light/dark themes.

## User outcome

Connect Codex once, open a repository, ask what the code does, and get a source-grounded explanation without choosing reference files or configuring an API key for each request. Code Atlas finds relevant captured code, explains it in useful sections, and lets the user inspect and navigate the evidence.

## In scope

- Reuse the connected Codex account and saved model/reasoning choice across requests and restarts. Offer only settings the installed runtime supports; never silently switch to a direct API or another backend.
- Let Codex search the repository index and request bounded source ranges as needed. Show ordinary search results alongside chat; manual file selection stays optional.
- Support file explanations, repository Guides, section repair, selected-code explanations, and nested follow-up questions from the current Guide or Flow.
- Build a Guide progressively: show its outline, then publish each independently validated section as it becomes ready. Keep work running until it finishes, fails, or the user stops it; elapsed time is diagnostic only.
- Review the exact question, captured source, and effective settings before sending. Validate claims and citations locally, save results as drafts, and require an explicit user action to accept them.
- Preserve saved answers and source history on cancellation, provider failure, invalid output, or restart. Give the user one clear recovery action when work is interrupted.

## Acceptance

- Packaged-app evidence shows one-time Codex connection reuse, supported model/reasoning settings, exact source review, consent, cancellation, and recovery without an API key.
- Native examples cover a file explanation, a repository Guide with sections published incrementally, a section repair, selected-code explanation, and a nested follow-up from Guide or Flow.
- Each published claim links to captured source and passes local validation before appearing in the Guide. A partial Guide remains readable when later work fails or is stopped.
- A representative real repository run demonstrates that the user can ask a question without preselecting reference files, follow the code evidence, and understand the result. Synthetic fixtures alone do not qualify repository usefulness.
- Build identity, Codex runtime/model, exact source snapshot, outcome, failures, and recovery are recorded. No fixed elapsed-time threshold is used.

## Exclusions

Claude Code, Antigravity, direct-model API catalogs, PR or Git-change explanations, standalone export, accessibility audit matrices, a fixed first-answer deadline, and automatic generation of every route chapter. These are not first-release blockers. Follow-up questions are asked from the current Guide or Flow; a separate accepted-plan-step generation flow is post-1.0.

## Related active work

- `z3an` owns the native Codex account/runtime implementation.
- `xvjk`, `a699`, and `gk46` own question-to-source discovery and bounded retrieval.
- `3bz5` owns progressive answer publication and reader behavior.
- `6qk9`, `j53m`, and `q0sm` own repository narrative, visuals, and quality acceptance.
- `13yp` owns packaged workspace design acceptance; `xymb` owns the previously requested static-fix acceptance.

Keep this task open until the packaged Codex path and representative repository outcome are accepted. A successful build or synthetic request alone is insufficient.
