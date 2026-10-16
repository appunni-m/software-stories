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
No paid service requirement, automatic private-source upload, runtime-trace claims, or automatic acceptance of generated narratives. This is pending product work, not a claim that the current file checklist already provides intelligent discovery.

## Existing baseline to preserve
The 16 October workspace iteration already remembers the backend/model, restores a saved source version and last file, and retains an unsent planning question and selected paths locally. These reduce repeated setup; they do not select the relevant source automatically. See the native workspace acceptance record before reimplementing those pieces.
