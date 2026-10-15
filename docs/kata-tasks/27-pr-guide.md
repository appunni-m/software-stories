---
kind: task
key: t27
parent: m6
title: Generate focused, reviewed explanations for a local change set
priority: 2
labels: phase-6, pull-request, provider
blocked_by: t10, t11, t12, t19, t26
---

## User outcome
The user can quickly understand what a local change set does and inspect every explanation against its diff or source evidence.

## Contract
Create a separate PR Guide draft for selected base/head refs and selected changed files. Explain changed behavior and locally supported impact, tests, and dependencies. Every code-change claim links to a diff hunk or source anchor and labels inference. Use the same exact-context Gemini review, consent, local validation, and Chapter Studio acceptance used by file chapters.

## Acceptance criteria
- PR Guide history is distinct from repository file chapters and the working Git history.
- Every changed-file claim cites the correct base or head evidence; unsupported impact is labeled or omitted.
- User can select a subset of files and review exact outbound context before each request.
- Accept/reject does not mutate repo refs or existing guide chapters.
- Changing either ref creates a new comparison identity and invalidates prior draft consent.

## Excludes
Remote PR fetch, GitHub comments, merge/checkouts, auto-approval, and broad repository re-generation.

## Done evidence
Explain a fixture change, trace each claim to a base/head hunk or source anchor, reject one inference, accept the remainder, and verify repository guide histories and Git refs are untouched.

## Pending work — 2026-10-15

- [ ] Complete the PR navigation/recovery child and the static-fix acceptance cases for review/decision races.
- [ ] Generate a consented local base/head guide, inspect concrete claims and both-side anchors, reject invalid/shallow claims and accept only selected valid ones.
- [ ] Reopen the same immutable comparison and confirm drafts/decisions/history remain discoverable and separate from repository chapters.
- [ ] Record cancellation/error/retry behavior; no remote PR write or automatic provider fallback.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.
