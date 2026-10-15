---
kind: task
key: repository-narratives
parent: m2
title: Build coherent repository narratives with file-level drill-down
priority: 0
labels: product-quality
---

## User outcome
A newcomer can follow a real feature end to end, understand each relevant file, and choose deeper explanations without hearing a list of document summaries.

## Current evidence
The implementation generates bounded file chapters and an optional orientation chapter from 1–12 selected files. The file quality gate does not qualify a complete repository narrative. There is no demonstrated whole-repository story planner or cross-language end-to-end quality report.

## Source implementation progress (2026-10-15)
The repository-overview request now frames the optional overview as a short learning path through the selected repository slice: explain prerequisites before using them, prioritize a supported behavior journey from an evidenced entry point, and stop at unknown module/protocol/language boundaries. It asks for question-form section titles and avoids one generic responsibility summary per file. The prompt version was advanced so reviewed requests bind to the new instructions.

Overview canonicalization now creates app-owned `AtlasFile` hotspots from the exact paths attached to each section's validated claim anchors. This lets the reader move from a story section to one of the cited files without trusting model-invented destinations. The existing walkthrough already has one-step section navigation and clickable evidence citations; the generated file hotspots add the missing file-level drill-down. The selected-source bound and separate per-file coverage remain in place.

The Guide now also contains a bounded local reading planner: choose a learning question, captured file and optional indexed callable declaration. It follows only verified static call edges, records caller prerequisites and exact source anchors, and shows unsupported boundaries, omitted files and truncation. Plans, selected steps and manual reading progress are saved in bounded local browser storage for the project/snapshot; damaged data or a changed stored token prevents silent replacement. Previous, Mark read and continue, Explore source and Inspect call actions preserve the reader's place. Generating a file uses a reviewed bounded selection of that file plus eligible adjacent files, through the chosen Codex, manual-agent or direct API route. Reading progress is separate from accepted chapter coverage.

This remains source progress, not acceptance evidence. The call-based reading planner does not synthesize an AI narrative or prove runtime ordering, branches, state or cross-language dispatch. Overview generation still selects at most twelve files. Native durable journey storage, multi-request narrative orchestration, selective shared-evidence updates, a representative complete guide and human comprehension review remain pending. Integrated Codex and Claude support has priority under the [agent-first gate](45-agent-first-generation.md). No tests or builds were run.

## Contract
Create an explicit repository guide plan over the immutable inventory. Identify evidenced entry points and user-relevant behavior journeys, prerequisite concepts, files/relationships, success/failure paths and known unknowns. Plan distinct sections with a concrete learning question, source scope, script, optional visual and deeper source/file links. Keep the overview optional and preserve per-file coverage. Generate in bounded reviewed units through the chosen API or agent runtime; show coverage, omissions, progress and recoverable failures. Do not squeeze a whole repository into one model request.

## Acceptance criteria
- A reader can choose orientation, a behavior journey or any eligible file; every completed section has explicit Next, Back and Explore code actions.
- At least one real feature follows its supported entry point through calls/data/state/error behavior, with each transition tied to source evidence.
- Plan records prerequisites and explains names before depending on them; no repeated generic introductions or unexplained jumps.
- Mixed-language boundaries are linked only from supported evidence; unknown dispatch/protocol details remain visible.
- Per-file explanations remain available as needed and coverage never counts the overview as explaining every file.
- One section can be edited/regenerated while untouched sections/assets retain identity; changed shared evidence marks affected sections separately.
- Human reviewers answer predefined behavior questions after the guide and record source errors, confusion and missing coverage. No whole-repo quality claim from individual file schema success.

## Excludes
Autoplay through an entire repository, hidden bulk provider spend, marketing-only intros and replacing per-file explanation with storytelling.

## Done evidence
A complete source-grounded guide on representative repositories, coverage map, incremental-edit demonstration and human comprehension review when validation is authorized.
