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
