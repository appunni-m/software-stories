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

### AI planning contract — source groundwork, still incomplete

A separate native [repository planning contract](../REPOSITORY_PLANNING.md) now prepares immutable selected source, a learning question, filtered local evidence, inventory/context/request digests and locally computed coverage gaps. It shares the chapter evidence loader without constructing an API request. The registered local preparation command and typed client start no agent and grant no consent.

Its distinct result schema and validator support ordered learning questions, earlier-step prerequisites, primary files, grounded rationales and labeled unknowns. They reuse the chapter quote/identifier/relationship validator, compute planned/cited/omitted selected paths locally and assign native destination IDs. Supporting citations cannot count as dedicated file steps. The validator reopens context before accepting unaccepted data; no chapter history or reading progress changes. This is a bounded planning unit, not a twelve-file overview presented as a full guide.

Codex and Claude now have separate registered native planning review commands and typed clients. They retain the exact learning question, capture status, source/index/coverage identity and explicit per-agent settings in the pending contract. Native rebuild helpers compare the entire reviewed contract without renewing its ID/expiry. A shared internal result bridge checks review/fingerprint/model identity and calls the independent plan validator. Old chapter/range/Git senders reject the distinct planning purpose, and direct API commands now preserve foreign reviews before credential loading. All of this remains source implementation.

Registered native Codex/Claude plan senders now reuse agent dispatch, active source validation and cancellation, retaining the exact interruption notice. Claude remains behind the empty qualification registry. There is no AI planning app dialog, durable AI plan storage, multi-request orchestration or native acceptance yet. Every agent planning cell remains Missing. These concrete integrations, exact question/scope recovery and human narrative review remain required; native reviews or a manual import cannot close them. No tests, builds, launches, runtime probes, logins or model requests ran.

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

## Planning interruption scope — source progress, 2026-10-16

The shared native journal can retain either agent's distinct repository plan, including the original learning question, selected paths and inventory/context/contract/capture/coverage/index identity. Local recovery rebuilds and compares the entire scope without restoring a runtime, settings or consent. Missing/mixed planning metadata is rejected; old non-planning records remain compatible. The app labels a planning notice and shows its question, then preserves it without opening a chapter, overview, Git or API request because the planning dialog is still absent. Native planning senders now record this exact scope before worker dispatch and retain the notice on success/failure; app recovery and explicit save/discard resolution are still pending. This groundwork cannot close automatic planning, durable plan storage or recovery; both planning cells stay Missing. See the [planning contract](../REPOSITORY_PLANNING.md) and deferred cases in `xymb`. No tests, builds, launches, probes or model calls ran.
