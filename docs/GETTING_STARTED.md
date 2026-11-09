# Getting started

Code Atlas is an in-progress macOS desktop app. This guide walks through a first local session from the source checkout. This repository does not provide a signed and notarized installer or a user download, so the native app must currently be run from source.

> **First-release scope:** Code Atlas is Codex-first, with local browsing and optional Pocket narration. Direct API selection, alternate agent integrations, PR explanations, and standalone HTML export are post-1.0 work. Some development-build controls may exist before they are ready for release; you do not need them for the first session.

## What you need

- macOS to use the native repository picker and local persistence. No API key is needed for the first-release Codex connection or local Pocket narration.
- Node.js 20.19 or newer in the 20.x line, or 22.12 or newer, plus npm.
- Rust 1.85 or newer with Cargo for the Rust 2024 edition; current stable is recommended.
- A local Git working copy to explore.

The project does not set Cargo's `rust-version`, so compatibility of the locked dependencies with Rust 1.85 is not verified. The macOS app bundle declares **macOS 14.0** as its minimum; the managed Pocket TTS lock targets Apple Silicon. This declaration comes from the selected runtime wheel and is not yet a clean-account support qualification.

## Launch the native app

From the repository root:

~~~sh
npm ci
npm run tauri dev
~~~

The first command installs the versions recorded in the npm lockfile. The second starts one Vite development server on `127.0.0.1:4173` and the Tauri desktop shell. The port is strict: if another process already owns it, startup reports the conflict instead of choosing a second port. Tauri starts Vite itself; do not also run `npm run dev` beside it.

~~~sh
npm run dev
~~~

This command starts the browser-only UI preview at http://127.0.0.1:4173. The browser preview and `npm run tauri dev` share the same strict port and cannot run at once. To switch modes, stop the current development process, then start the other command; keep one Vite server on port 4173. The browser preview is useful for layout, but cannot use the native folder picker, macOS Keychain, or local repository commands. Use the Tauri command for the native workflow.

Code Atlas stores captures and guide history in its local application data
folder. Read [Local data and permissions](LOCAL_DATA.md) before removing that
folder. Standalone HTML export is post-1.0 and is not a complete backup.

## Explore a repository

1. Choose **Open repository** and select a local Git working copy. Choose **Create snapshot** to capture its eligible source. Capture is local, bounded and cancellable; partial captures remain labelled partial.
2. The workspace opens with files on the left and a likely implementation entry point beside the explanation area. Browse files freely. Source comes from the selected immutable snapshot, rather than your changing working copy.
3. Connect Codex once from the workspace header if it is not already connected. Code Atlas reuses the installed Codex account, and the current connection is visible in the header. No API key is needed; generation uses your Codex account and may count against its quota.
4. Enter a question in the composer and choose **Guide me** or **Show the flow**. On the first question for this snapshot, review the sharing disclosure and choose **Allow and explain** only if you want relevant captured context sent to OpenAI through Codex. This permission is limited to the snapshot and connected account. To revoke it, open the current repository menu in the app header, choose **Repository sharing**, then choose **Revoke permission and stop questions**. A changed snapshot, account or runtime needs a new permission.
5. Codex finds relevant implementation in the background; no reference-file checklist or second generation stage is required. Validated sections appear in Guide as they are ready. Keep reading while it works, or choose **Stop** on the question. Completion and errors collect behind the notification bell; unfinished questions are not sent again automatically after a restart.
6. Read **Guide**, inspect **Flow**, or open **Chat history** at the current level. Follow a citation to its exact captured lines, use **Next section** when you are ready, or ask a follow-up in the composer. Choose **Listen** when you want narration; prepare the local Pocket voice once in **Connections**. Matching audio is reused when available, and playback stays under your control.
7. Ask another question to add an answer to this level. **Explore further** on an answer makes a child exploration. Its link and answer count stay on the originating card. Use the breadcrumb root or **Back** to return; the left exploration tree keeps branches reachable. Each exploration remembers its last Guide / Flow / history view.
8. Drag an explorer file into chat or choose **Attach open file**. Select code in the source pane and choose **Explain this code** to ask at the current level. AI still finds supporting implementation. Captured docs may guide discovery but do not replace source evidence.

### Find code and use the keyboard

Typing searches locally without sending a model request. The Search activity covers file paths, indexed symbols and bounded captured text. Results open code; **Explore with AI** asks the entered question even when there is no local match. Search limits are labelled; refine a query to inspect more.

| Shortcut | Action |
| --- | --- |
| Cmd+P | Focus quick file search in the Explorer |
| Cmd+K | Open the current exploration's Flow view |
| Cmd+Shift+F | Open repository text search |
| Cmd+L | Focus chat |
| Cmd+I | Explain the code selected in the source pane |
| Cmd+1 / 2 / 3 | Guide / Flow / Chat history |
| Alt+Left | Return to the parent exploration |
| Enter / Shift+Enter | Ask / add a new line |
| Escape | Dismiss chat search suggestions |

On narrower windows, switch between readable source and explanation using the code control above chat. On wider windows, resize the source and explanation panes directly.

## Compare local revisions

Choose **Changes** in the left activity bar. Its local commit list contains up to 80 commits. Select a commit to compare it with its first parent. Merge commits are marked; this is a bounded ancestry list, not a complete interactive Git DAG. The existing comparison view also accepts explicit local refs. Root commits currently compare with themselves rather than an empty tree.

Comparisons read existing local objects only: no fetch, checkout or repository writes. Expand a changed file for exact changed-line ranges and base/head hunks. Binary, oversized, unsupported, parser-uncertain and omitted evidence is labelled. Declaration overlap is static evidence, not inferred runtime impact.

PR Guide and **Explain hunk** are post-1.0 follow-ups. The first release includes local, read-only Changes browsing; PR explanations are not part of the first-release journey.

## Edit and narrate a guide

Use an answer's edit control to update one section or inspect its saved revisions. A section update keeps unaffected sections and assets intact.

**Listen** uses Pocket TTS on this Mac. On Apple Silicon, choose **Prepare voice** once; setup downloads the runtime and voice into app storage. You do not need to install Python or `uv` yourself. Narration text stays local, no provider API key is used, and the transcript and manual navigation remain available without audio. See [provider disclosures](PROVIDER_DISCLOSURES.md).

Standalone HTML export is planned after 1.0 and is not needed to use saved guides in the app. See the [export plan](OFFLINE_EXPORT.md) for its intended contents and privacy boundaries.

## Connect Codex once

Choose the Codex connection once in the workspace header. Local browsing does not require a connection; the first question that shares captured code asks for permission for that repository snapshot. Generation uses the connected Codex account and does not require a direct API key.

Direct API connections and Claude/Antigravity integrations remain future backend options; they are not choices needed for the first-release workflow. See [AI backend status](AGENT_OPERATION_PARITY.md) for implementation and qualification detail.

## Current boundaries

- The local structural parser covers bounded Rust, TypeScript, TSX, JavaScript, Python, and Go subsets. These are **partial structural** capabilities, not fully qualified language support. Other captured text remains citeable but is **text only**; no language has passed the full structural and narrative qualification gate. See the [language capability matrix](LANGUAGE_CAPABILITIES.md).
- Workspace questions automatically choose a bounded set of supporting files. Large-repository Guides are built incrementally and may show uncovered topics rather than claim complete coverage from one question.
- Generated prose and source identity validation are different checks: a valid citation does not prove that the explanation is semantically correct.
- Direct API routes, PR explanations, and non-Codex agent integrations are future work for after 1.0. Their source implementations do not make them qualified first-release options.
- Single-section stale-guide updates and user-facing revision compare/restore are implemented; updating several stale sections in one model request is not supported. Local Git comparisons show bounded, revision-specific static references to changed files and declarations; they do not infer runtime impact. Onboarding and release signing remain open; read the [Kata backlog](KATA_BACKLOG.md) for current task scope.
- Code Atlas-authored source uses the MIT License. Third-party notices, captured source, generated answers, and evaluation assets have separate terms; see [the license scope and notices](../THIRD_PARTY_NOTICES.md) before redistributing.

## Where to go next

- [Product blueprint](PRODUCT_BLUEPRINT.md) for the intended user experience and product rules.
- [Interaction contract](INTERACTION_CONTRACT.md) for control behavior, navigation, and side effects.
- [Technical plan](TECHNICAL_PLAN.md) for source-backed architecture and implementation boundaries.
- [Chapter schema](CHAPTER_SCHEMA.md) for saved chapter and evidence records.
- [Troubleshooting](TROUBLESHOOTING.md) for common setup and workflow problems.
- [Contributing](../CONTRIBUTING.md) and [security reporting](../SECURITY.md) for project limits and contact routes.

