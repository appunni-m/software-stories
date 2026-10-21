# Native workspace observation — 21 October 2026

Status: partial evidence for Kata `x21h`; this is not the required screen-by-screen audit.

## Build and workspace

- Installed app: `/Users/lazytrot/Applications/Code Atlas.app`, version `0.1.0`.
- Installed executable SHA-256: `c908d35c1f53eea20e3484305a2e187071ef79495c7a3629051eb4310ea8e73c`.
- Window: 2880 × 1688 pixels; System appearance (dark on this Mac).
- Repository: Fontdone; captured snapshot `main@5249b72371`, partial capture.
- Visible route: populated Explorer, `src/tt/mod.rs` selected, Guide tab selected.
- Guide item: `TableDirectory::find`, one visual scene, manual Pocket narration Ready to play at 00:00 / 00:12.
- The source pane displayed the captured implementation around `src/tt/mod.rs:62–71`; the visual's source-linked controls and the chat composer were visible.

## Observations

- **Match:** the installed workspace has the left activity bar, persistent Explorer, source pane, composer above the workspace, and Guide / Flow / Chat history views inside the exploration.
- **Partial:** the Guide presents the lookup as a graph, which fits the scan-and-return branches. At the installed build's default view, node descriptions are clipped to two lines and several end in an ellipsis. This makes it hard to understand a node without opening another detail surface.
- **Source change pending native review:** the working tree now exposes full node text on hover and expands narrated nodes to four lines and focused nodes to six. The installed app still has the SHA above, so it does not contain or verify that change.
- **Observed notification state:** before inspection the notification drawer was open with 24 items under “Needs attention” and 69 unread updates overall. It covered a large part of the reading pane. Closing it restored the same Guide/source view; no notification was dismissed, read, or otherwise changed.
- **Observed exploration state:** the Explorer listed 33 questions for Fontdone, including partial explanations and unavailable saved answers. These are existing workspace records and were not modified.
- **Duplicate setup route:** the top bar has a persistent Connections control, but the adjacent Workspace tools menu also listed Connections. That creates a second setup path after the user asked to keep connection selection in the header. The source now removes the duplicate menu item and a shell regression checks that Connections appears in the header only. This source change has not yet been installed in the app whose hash is recorded above.

The CUA screenshots of the open drawer and restored workspace are visible in the 21 October task transcript. They are not archived as PNG files in this repository yet, so the screenshot artifact requirement remains incomplete.

## Not inspected

First-run and empty states, Search, Changes, Flow, Chat history, nested Back/root restoration, notification actions, light appearance, resizing, 200% zoom, keyboard-only navigation, VoiceOver, and other repositories remain uninspected. Capture the rebuilt installed app and archive screenshots before claiming native acceptance.

Related implementation/evidence tasks: `13yp` (approved workspace), `j53m` (visual explanations), and `hstr` (notifications). No task is closed by this observation.
