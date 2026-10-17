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

## October 17 visual story contract (supersedes text-first Guide)

- Guide opens on a large, source-linked visual stage. Narration, node details and supporting evidence begin collapsed. Flow remains the exploratory full graph; Chat history remains conversation. Persistent source, chat and nested exploration stay in place.
- Each authored, grounded claim is one spoken scene when the complete script exactly matches the ordered claim sentences. Its declared evidence anchors select the highlighted visual nodes. Scenes explain an input or goal, what changes, decisions and consequences; the narrator refers to the short visual labels. Names and definitions alone do not qualify as teaching.
- Author visuals and narration together, usually 2–5 scenes with 2–4 focal nodes each. Keep actual rejection/fallback/uncertainty, avoid decorative arrows and invented runtime traces, and preserve every grounded sentence. Older summaries and custom scripts stay whole unless exact scene coverage is possible; do not silently rewrite saved answers.
- Play advances scenes only after the current audio actually ends. Pause, replay, seek, Previous/Next scene, node selection and Source remain available. A section boundary always waits for the reader. No guessed word timings or automatic advance into another section. At most one narration plays at once; changing scenes stops/cancels the old clip.
- Scene selection and per-scene audio positions persist locally by immutable section revision. Keyboard controls, light/dark appearance, 200% zoom/reflow and Reduce Motion must work. When no validated diagram exists, show exact captured code or an honest unavailable state, with narration still accessible.
- Editing a scene script or pronunciation invalidates only that clip's request/cache. A pre-existing whole-section edited script remains authoritative. Engine/voice setup stays in Connections; Google still requires the exact request review/consent. No implicit cloud fallback.

Implementation is under validation. Closure requires actual installed Guide and standalone-player checks, a fresh Rust and TypeScript generated story whose narration explains its visual cues, exact source opening/return, scene/audio restart and offline-cache evidence, comprehension/listening review, and export compatibility. A partial Fontdone answer, a static screenshot, prompt wording or passing schemas cannot close this task. Multi-scene audio export must include its matching clips or explicitly remain transcript-only; never claim unavailable audio was included.

## October 17 clarification: choose the visual representation

Keep the improved shared workspace and current graph capability. Graphs are one representation, not the required Guide format. AI may compose responsive HTML scenes with byte strips, transformations, comparisons, state stacks, tables, metrics, code and pixel matrices, or choose a graph when the relationships/branches warrant it. Different claims can use different representations. See [scene contract](../VISUAL_SCENES.md). The visual has the main space, script/details remain collapsed, and the narration explains what the visual means. Exact source values and illustrative examples must be distinguished visibly. No arbitrary generated scripts, styles, external resources or handlers execute. Current graph-only saved answers are preserved rather than silently regenerated. Native completion, source-navigation, human visual/story quality and accessibility gates remain open.

## October 17 HTML scene qualification

Guide and the standalone player share the HTML/graph visual stage, with collapsed narration and details, manual section boundaries and independent scene/audio positions. Real Fontdone source-click and Pocket playback observations are recorded. Restoring paused audio at its saved end no longer advances scenes. Full navigation/focus/accessibility and human storytelling acceptance remain open. See [results, preserved failures and remaining gates](../acceptance/2026-10-17-visual-story-guide.md). This task remains open.
