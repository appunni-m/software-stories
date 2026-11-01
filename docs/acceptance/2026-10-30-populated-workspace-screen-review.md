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

## Remaining audit states at the 30 October checkpoint

First run and repository selection; empty repository; no-result Search and AI fallback; multi-scene navigation; PR explanation; notifications and matching-question return; connection/model/reasoning/approval/voice settings; light appearance; compact and narrow layouts; 200% zoom; keyboard-only navigation; VoiceOver; Increase Contrast; and Reduce Motion remain uninspected. No durable screenshot artifact is attached. Keep `x21h`, `13yp`, accessibility, and packaged acceptance open.

## Packaged workspace follow-up — 1 November 2026

- App: Code Atlas 0.1.0, installed macOS app; executable SHA-256 `a69eb4809384e41858486506fdb437d5fe64bce4cc79589463bd8a392d150fe2`.
- Repository: Fontdone, partial snapshot `main@5249b72371`.
- Starting state was the saved Level 2 Guide, scene 2 of 3, with `src/api.rs:710–728` selected. No question, generation, narration playback, or saved answer edit was started.
- **Guide and source:** the source pane remained alongside the current Guide. Scene 2 of 3 and its decision visual were visible. The reader remained on the same scene while opening other activity views and returning.
- **Flow:** the existing section presented three source-backed points. Selecting `Flag normalization` showed its detail and citation `src/api.rs:685–709`; the unlinked points were presented without invented connecting edges.
- **Search:** the existing `memory_face` query displayed 78 matches across 22 files and explicitly warned that local search may be incomplete. Results included exact captured source ranges.
- **Changes:** the activity showed 80 recent commits and the saved comparison `v2.14.3 → main`. It labeled the README hunk unsupported while keeping the textual diff available, identified the retention inventory as oversized with zero hunks, and said that no cross-file relationship records were available. It did not imply runtime impact.
- **Explorations:** the root showed five child explorations and the active Level 2 question remained selected.
- **Appearance:** Light appearance was inspected for Search, Flow, and Changes, then restored to System. The original Guide scene, selected source range, and Explorer activity were restored.
- CUA screenshots were inspected in-session but not retained as repository image artifacts. Window dimensions were not recorded. This is a focused interaction follow-up, not the durable screenshot set or full screen-by-screen acceptance required by `x21h` and `13yp`.

The observed Search, Flow, Changes, and visible exploration hierarchy agree with the current activity-bar workspace contract in these populated states. Empty/error states, notifications, PR Guide, nested child counts at deeper levels, and screenshots remain unqualified. No provider call or transmission occurred.
