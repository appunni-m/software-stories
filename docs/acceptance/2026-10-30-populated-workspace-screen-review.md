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
- **Search without matches:** a unique local query displayed “No local match; search coverage is incomplete” and the **Explore with AI** action. I did not activate it. The existing `memory_face` query and Explorer view were restored; no provider request was sent.
- **Changes:** the activity showed 80 recent commits and the saved comparison `v2.14.3 → main`. It labeled the README hunk unsupported while keeping the textual diff available, identified the retention inventory as oversized with zero hunks, and said that no cross-file relationship records were available. It did not imply runtime impact.
- **Explorations:** the root showed five child explorations and the active Level 2 question remained selected.
- **Notification navigation:** opened the existing “Explanation ready · fontdone · Glyph loading: normalize flags, then apply early gates” item and used **Open question**. The drawer closed and returned to the matching Fontdone Guide question with scene 2 of 3 and `src/api.rs:710–728` still selected; the header remained at 47 unread. This was a navigation-only check: no generation, answer edit, read-all, or dismissal was triggered. It qualifies one populated ready-item route, not failed/interrupted items, focus return, restart, or the full notification lifecycle.
- **Appearance:** Light appearance was inspected for Search, Flow, and Changes, then restored to System. The original Guide scene, selected source range, and Explorer activity were restored.
- CUA screenshots were inspected in-session but not retained as repository image artifacts. Window dimensions were not recorded. This is a focused interaction follow-up, not the durable screenshot set or full screen-by-screen acceptance required by `x21h` and `13yp`.

The observed Search, Flow, Changes, visible exploration hierarchy, and one ready-notification route agree with the current activity-bar workspace contract in these states. Empty repository/error states, failed/interrupted notification routes, PR Guide, nested child counts at deeper levels, and durable screenshots remain unqualified. No provider call or transmission occurred.

## Packaged notification focus correction — 1 November 2026

- Source change: `1125292` defers initial focus to the safe **Close notifications** control until the Radix popover has mounted. The earlier direct focus call still left **Mark all read** focused in the installed app.
- Build: `npm run bundle:macos` completed the TypeScript/Vite production build, unsigned app and DMG packaging, bundle metadata verification, and DMG checksum. The packaged executable SHA-256 is `bf4911531f1154e23fddba5d4aab572b752f472c8a6d1d4476e0fb5e9e012558`.
- Installed the bundle at `/Users/lazytrot/Applications/Code Atlas.app`, preserving the previous bundle under `/private/tmp/code-atlas-notification-focus-20261101-before/`.
- On Fontdone partial `main@5249b72371`, the header showed 47 unread. Opening Notifications focused **Close notifications**. Escape closed the panel and returned focus to the bell; the Guide, source selection, and unread count stayed unchanged. Reopening the same read **Explanation ready** item and choosing **Open question** closed the panel and returned to its Guide question, scene 2 of 3, with `src/api.rs:710–728` selected; unread remained 47. No notification was marked read/dismissed, no narration or generation started, and saved content was not edited.
- This verifies one packaged notification focus/return path. No screenshot was retained; action navigation after the focus fix, live completion, failed/interrupted notices, restart, 200% zoom, contrast, motion, and VoiceOver remain unqualified. Keep `hstr`, `x21h`, and `13yp` open.
