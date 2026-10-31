---
kind: task
key: intent-context-search
parent: design-parity
title: Unify chat, parallel code search and exact source context
priority: 1
labels: product, design, ux
---

## Outcome
One top chat bar captures intent everywhere. Readers may ask naturally, browse local matches, drag a file, or select code and Explain this code in one action.

## Behavior
Guide me / Show the flow above composer act on the entered question; without text they only select the current view. Enter submits current-level intent; Shift+Enter adds a line and IME composition never submits. Debounce local files/symbol/text search while typing, without AI calls per keystroke. Show useful local results and Explore with AI, including no-results state. Opening a result reveals exact code without submitting. Stale asynchronous results cannot replace newer queries.

Drag file to composer adds a removable chip; keyboard Attach is equivalent. Selection/function Explain sends a source-bound turn at the active level without a second Send, modal, reference checklist or mode reset. Add selection supports a custom question. Every context ref includes project/snapshot/path/hash and validated exact range or indexed symbol. Explicit context guides but does not replace AI-selected support. Validate stale/range/out-of-scope context before transmission. Never treat arbitrary OS file drops or foreign JSON as authorized repository source.

## Shortcuts and acceptance
Cmd+P quick search, Cmd+Shift+F repo text search, Cmd+L chat, Cmd+I selected-code explanation, Cmd+1/2/3 views, Alt+Left parent, Escape transient search. No shortcut hijacks text editing, IME or an active modal. Test result races/no results/large repos, file and range drops, keyboard alternatives, duplicate chips, changed project/snapshot and exact source citations. Show answers in Guide or Flow inline, with all questions in history at the same level. The sample preview does not establish live search or generation acceptance.

## October 16 implementation checkpoint
The production top composer and Search activity now merge captured paths, index symbols, open-source lines and a native bounded immutable text scan (24 MB / two seconds / 80 hits). Typing performs local work only; stale replies are ignored and limited search is labelled. File chips, drag from explorer, attach-open-file and selected-code Explain use snapshot context; native generation revalidates hashes/ranges. Selection end-line and invalid reveal-range handling were corrected. Cmd+P/K/Shift+F, Cmd+L/I, views and parent shortcuts are wired; responsive source/explanation switching avoids narrow columns. Build and pure state/search regression checks pass; real packaged keyboard/drag/race/large-repo acceptance remains open.

## Packaged Search result and no-match check — 31 October 2026

On the installed app built from `d91e3f2759a065bea8e44a5ccdaab8a550898591`
(executable SHA-256
`e8c761717da958e3ef9efd8b7d9e621ec6659672f95bad61790da18d2c0fd93b`), Search
for `memory_face` returned captured code matches, including `src/font.rs` and
`src/api.rs`, with `78 matches found across 22 files; more matches available.`
The app labeled the result set `Local search may be incomplete`. A unique
no-match query displayed `No local match; search coverage is incomplete.` and
the **Explore with AI** action. I did not activate it, so no provider request
or generation occurred. I restored the original `memory_face` query and
Explorer/Guide view; the selected source, explanation, and scene were
unchanged.

This verifies one packaged local-result and no-match path. Search-result races,
large repositories, keyboard shortcuts, drag/drop, source selection, snapshot
changes, and exact-citation revalidation remain open; keep Kata `nsfd` open.
