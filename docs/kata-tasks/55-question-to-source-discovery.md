---
kind: task
key: question-to-source-discovery
parent: design-parity
title: Find relevant code from a question without manual context hunting
priority: 1
labels: product, design, agents, source
---

## User outcome
A reader asks a question such as “How does a font file become pixels?” and gets a short, source-grounded reading path without manually guessing which files to select.

## Required behavior
- Search the captured inventory and local symbols/relationships to propose relevant entry points and supporting files. Operate against the selected immutable snapshot.
- Show a compact proposed scope, why each file is relevant, and capture/language gaps. Never present a static link as a proven execution sequence.
- Default to the remembered supported agent, model and reasoning settings. Opening the planner must not repeatedly ask the reader to choose a backend.
- Keep the question and selected scope when changing between Files, Flow and Player, or restarting the app. Clearly separate unsent local drafts from sent requests and accepted plans.
- Prepare the exact bounded source review locally. Require fresh consent before transmission. Respect secret, byte, file-count and source validation limits; explain when a question cannot fit one request.
- Present accepted steps beside real code. Clicking a step, graph node or citation opens the exact snapshot source and preserves the reading position.
- An empty or unsupported index must offer a clear fallback; never invent a path or silently broaden what is sent.

## Acceptance
Use a Rust multi-crate project, a TypeScript project, a missing-index case, and a question whose relevant source exceeds one request. Record the proposed files, reasons, omissions, consent scope and return-state behavior. Verify backend/model settings survive repeat file requests. User-visible controls must describe the question and code rather than internal request-schema concepts.

## Exclusions
No paid service requirement, automatic private-source upload, runtime-trace claims, or automatic acceptance of generated narratives. This task remains open for the broader acceptance matrix; the source implementation below does not establish exhaustive discovery.

## Existing baseline to preserve
The 16 October workspace iteration already remembers the backend/model, restores a saved source version and last file, and retains an unsent planning question and selected paths locally. These reduce repeated setup; they do not select the relevant source automatically. See the native workspace acceptance record before reimplementing those pieces.

## October 16 implementation delta
The Codex path now prepares a local index automatically, reviews metadata, asks Codex to select implementation, validates up to 12 code paths/250,000 bytes, then automatically prepares the exact source review. File and overview requests share this discovery path. The workspace header restores and displays the saved connection/model/reasoning. Code-behavior claims backed only by documentation are rejected; explicit deprecated/archive labels and unverified docs are visible in planning. Routes support one reviewed sequential chapter batch with saved-step skipping and independent drafts.

Remaining: Rust multi-crate and large TypeScript native runs, missing/unreadable index and metadata/source overflow acceptance, persisted discovery scope/reasons across restart, multi-window coverage for large repositories, and deferred non-Codex agent discovery. See [current flow](../AI_GUIDE_FLOW.md) and [batch qualification](56-route-chapter-batch.md).

## Fontdone qualification — October 17

The real Rust multi-crate repository exposes a gap between capture/index eligibility and question retrieval: the central 375,700-byte `src/font.rs` is indexed but excluded from discovery, and larger adapter implementations are above both request and parser limits. The broad font-to-bitmap request staged nine smaller files without the core font implementation or public Rust API. Keep large-repository acceptance open. Bounded source ranges and staged retrieval have a separate contract in [task 66](66-large-source-discovery.md), Kata `0wdn`; [native observations](../acceptance/2026-10-17-fontdone.md) preserve the limitations and failed attempts.
