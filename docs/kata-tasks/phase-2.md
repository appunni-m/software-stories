---
kind: milestone
key: m2
title: M2 — On-demand file explanations and repository Guides
priority: 4
labels: container, milestone, phase-2, chapters
blocked_by: m1
---

## First-release scope

Code Atlas helps a developer understand relevant code quickly. A file can be explained when requested, and a repository Guide follows a useful, source-grounded behavior path. A useful Guide does not wait for every eligible file to have its own chapter. The connected Codex path and incremental Guide work are tracked in the child issues in live Kata.

## Goal

Explain selected files and relevant repository behavior with claims grounded in the captured source. Keep the inventory available for navigation and search, while creating explanations only when the user asks for them or a Guide needs them.

## Gate

- An eligible captured file can be explained on demand, with accepted claims validated against that snapshot's source.
- A repository Guide automatically selects relevant source and explains a supported behavior path; it does not require bulk file-by-file chapter generation or complete per-path chapter coverage.
- Published Guide sections remain useful while later sections are generated. Failure and cancellation preserve completed work and make remaining coverage clear.
- Invalid or unsupported claims cannot be accepted, and provider failures do not disable local browsing, search, or source inspection.

## Excludes

- Generating a chapter for every repository path by default.
- Treating inventory coverage as evidence that the repository's behavior has been explained.
- PR explanations, a direct-model API catalog, and non-Codex agent integrations in the first release; those are post-1.0 work.

## Related work

Use the current child links in Kata for repository Guides, source discovery and retrieval, Codex generation, visuals, chapter review, language qualification, and quality acceptance. The previous Gemini-based and bulk file-coverage milestone wording is superseded.

## Done evidence

A representative repository Guide explains a real behavior path through relevant files, with independently validated source citations, incremental publication, recoverable omissions, and on-demand file drill-down. Reviewers can answer predefined comprehension questions without relying on a chapter-per-file inventory.
