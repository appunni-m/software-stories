---
kind: task
key: guide-flow-detail
parent: design-parity
title: Compose Guide and Flow from shared answers with inline code details
priority: 1
labels: product, design, ux
---

## Outcome
Guide teaches the current question; Flow lets the reader explore its structure. Both can reveal and explain implementation beside actual code. Detail is a reusable component, not another mode tab.

## Guide
Render independently revisioned answer sections as headed stacked cards. Use text, safe typed HTML/layout templates and source-linked visuals as appropriate to each concept. Optional Pocket narration follows deliberate visual cues; Pause/Next and clickable code remain available. No document-reading script, auto section advance or generic diagram substituted for the actual implementation. Save current-level follow-ups as cards and show real nested exploration counts. Keep edit/revision tools contextual; selective changes reuse unaffected sections and assets.

## Flow and code
Start at the question's useful abstraction, allow parent/child traversal from repository to module/function, and preserve viewport/expansion per exploration. Click a node or citation to open verified immutable source; inline detail explains what the code does, then offers meaningful follow-ups. Keep context and chat level stable. Show uncertainty or unsupported links plainly; static relationships are not live execution. Reuse validated claim anchors and generated visual evidence references. Never execute arbitrary model HTML/JS; no broadened filesystem/network capabilities in rich views.

## Acceptance
One complete source-backed narrative and graph for a TypeScript and Rust project, click-to-code/return without losing position, follow-up from both views, five-level drill-down, missing evidence and graph cycles, locally narrated playback and cache reuse, per-section edit without unrelated regeneration. Review actual depth, pace and language; schema validity alone does not pass. Preserve the existing independent chapter revisions, evidence validation, staleness and Pocket caches. The sample preview player is visual-only and is not narration acceptance.

## October 16 implementation checkpoint
Production Guide renders real native revision sections as stacked question/answer cards with manual Next, exact source citations, contextual edit/player controls, optional typed visual and Pocket narration. Explicit branches attach to their originating turn with actual child counts. Flow consumes the same latest answer's validated evidence-linked visual; before an answer it uses the verified index relationships for the opened file. Unsupported diagrams have an honest empty state. Source uses shared read-only CodeMirror rather than sample data. Rust/TypeScript narrative depth, five-level live exploration, per-exploration graph viewport and complete section/playback position persistence still need qualification or implementation; the detached sample preview is not evidence for these gates.
