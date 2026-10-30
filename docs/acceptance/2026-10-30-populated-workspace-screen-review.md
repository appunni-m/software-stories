# Populated workspace screen review — 30 October 2026

## Capture identity

- App: Code Atlas 0.1.0, installed macOS app.
- Installed executable SHA-256: `38e1cb55c92ba3686357190d2eb92316e7f3ec70c8afc6669a5ab1a9ce4e447c`.
- App source revision: `f225c73` (the later `152f9e4` source-discovery prompt change was not bundled).
- Repository: Fontdone; selected snapshot `main@5249b72371`, partial capture from 16 October 2026.
- Screen capture: native CUA output at 2932 × 1680 pixels. The screenshot was displayed in the Codex session but could not be saved as a repository artifact; this note therefore does not meet the durable-screenshot acceptance requirement.
- Actions were read-only view navigation. No question was sent, source was not transmitted, no answer was edited, and narration was not played.

## Populated states inspected

### Explorer and Guide — partial match

The app presents the expected VS Code-style Explorer/Search/Changes/Explorations rail, a persistent file tree and selected `src/outline.rs` source, and Guide/Flow/Chat history within the exploration. `Guide me`, `Show the flow`, and the composer sit above the source and answer panes. The header keeps repository, snapshot, latest-changes, connection, appearance, notifications, and help controls in one row.

The current answer is a visual decision with a selected cue, exact `src/outline.rs:49–51` source selection, collapsed scene details, collapsed notes, and narration controls. It has one scene, so no previous/next arrows or scene markers are shown. This build does not establish multi-scene positioning. The saved root also retains recoverable unavailable historical answers and a six-slide guide; these were not opened or changed.

### Search — partial match

With the pre-existing `memory_face` query, local search reported 47 matches across 22 files, more available, and disclosed incomplete local coverage for this partial snapshot. Results included implementation source and test fixtures. This inspected only the populated-results state. Empty results, search errors, keyboard entry, and the `Explore with AI` fallback were not exercised.

### Changes — partial match

The page displayed an 80-commit local history and a two-file comparison. The README comparison remained readable as text despite an unsupported parser; the retention inventory was identified as oversized with no rendered hunks. Empty history, errors, another selected base/head, and PR-guide behavior were not inspected.

### Explorations, Flow, and Chat history — partial match

The exploration tree exposed a filter, 110 root questions, and five child explorations. Flow showed three source-linked ideas with code-detail controls. Chat history had search and grouped prior attempts. The root guide was shown as having 37 attempts and the Code Lens question 27; this is visible only in history and was not modified. Nested restoration, independent sibling positions, restart, and selected Flow viewport restoration were not tested in this review.

## Remaining audit states

First run and repository selection; empty repository; no-result Search and AI fallback; multi-scene navigation; PR explanation; notifications and matching-question return; connection/model/reasoning/approval/voice settings; light appearance; compact and narrow layouts; 200% zoom; keyboard-only navigation; VoiceOver; Increase Contrast; and Reduce Motion remain uninspected. No durable screenshot artifact is attached. Keep `x21h`, `13yp`, accessibility, and packaged acceptance open.
