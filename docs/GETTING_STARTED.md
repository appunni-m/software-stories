# Getting started

Code Atlas is an in-progress macOS desktop app. This guide walks through a first local session from the source checkout. This repository does not provide a signed and notarized installer or a user download, so the native app must currently be run from source.

## What you need

- macOS to use the native repository picker, local persistence, and provider keys in Keychain; Gemini narration uses a separate key.
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

The first command installs the versions recorded in the npm lockfile. The second starts one Vite development server on `127.0.0.1:4173` and the Tauri desktop shell. The port is strict: if another process already owns it, startup reports the conflict instead of choosing a second port.

~~~sh
npm run dev
~~~

This command starts the same browser-only UI preview at http://127.0.0.1:4173. Do not run it alongside an already running preview; reuse or restart that one process. The browser preview is useful for layout, but cannot use the native folder picker, macOS Keychain, local repository commands, or direct API requests. Use the Tauri command for the native workflow.

Code Atlas stores captures and guide history in its local application data
folder. Read [Local data and permissions](LOCAL_DATA.md) before removing that
folder or relying on an offline export as a backup.

## Explore a repository

1. Choose **Open repository** and select a local Git working copy. Choose **Create snapshot** to capture its eligible source. Capture is local, bounded and cancellable; partial captures remain labelled partial.
2. The workspace opens with files on the left and a likely implementation entry point beside the explanation area. Browse files freely. Source comes from the selected immutable snapshot, rather than your changing working copy.
3. Open **Connections** in the header. Choose Codex under **Explanations**; Code Atlas discovers the installed runtime and reuses its existing account. Saved model and reasoning choices are reused across projects. GPT-6 Luna with max reasoning is the initial preference when the installed catalog offers it; choose another supported catalog option if needed. No API key is required; account quota applies.
4. Enter a question in the composer and choose **Guide me** or **Show the flow**. On the first question for this snapshot, review the sharing disclosure and choose **Allow and explain** only if you want relevant captured context sent to OpenAI through Codex. This permission is limited to the snapshot and connected account. To revoke it, open the current repository menu in the app header, choose **Repository sharing**, then choose **Revoke permission and stop questions**. A changed snapshot, account or runtime needs a new permission.
5. Codex finds relevant implementation in the background and saves a cited draft. No reference-file checklist or second generation stage is required. Keep reading files while it works, or choose **Stop** on the question. Completion and errors collect behind the notification bell. A restart marks unfinished questions interrupted and never resends them automatically.
6. Read **Guide**, inspect **Flow**, or review **Chat history** at the current level. Click a citation to reveal its exact captured lines. **Next section** advances only when you choose it. Each answer has its own edit and narration controls. **Play narration** uses a matching cached clip when available; otherwise Pocket prepares it locally. Choose the voice once in **Connections → Voice**. Generated explanations are drafts until reviewed and accepted.
7. Ask another question to add an answer to this level. **Explore further** on an answer makes a child exploration. Its link and answer count stay on the originating card. Use the breadcrumb root or **Back** to return; the left exploration tree keeps branches reachable. Each exploration remembers its last Guide / Flow / history view.
8. Drag an explorer file into chat or choose **Attach open file**. Select code in the source pane and choose **Explain this code** to ask at the current level. AI still finds supporting implementation. Captured docs may guide discovery but do not replace source evidence.

### Find code and use the keyboard

Typing searches locally without sending a model request. The Search activity covers file paths, indexed symbols and bounded captured text. Results open code; **Explore with AI** asks the entered question even when there is no local match. Search limits are labelled; refine a query to inspect more.

| Shortcut | Action |
| --- | --- |
| Cmd+P, Cmd+K, Cmd+Shift+F | Open repository search |
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

The comparison's **Build a PR Guide** and **Explain hunk** tools use the existing reviewed request flows. Codex is the default connection; direct APIs are optional. Their context review is separate from the workspace question permission. Generated claims must cite the correct base/head hunk; PR decisions are saved separately and never alter Git state. Full operation acceptance remains tracked in [agent generation](AGENT_GENERATION.md).

## Edit, narrate and take a guide offline

Open an answer's edit control to review or accept a draft, edit sections, compare immutable revisions or restore an earlier revision as a new draft. One-section regeneration preserves unaffected sections and assets; it still uses its explicit scoped review flow.

**Listen** uses Pocket TTS by default for new narration preferences. On Apple Silicon, choose **Prepare voice** once; setup downloads the pinned runtime, model and voice into app storage and verifies its setup tool. You do not need to install Python or `uv` yourself. Local synthesis uses finite workers, no API key and no listening server. The transcript and manual navigation remain available without audio. Existing Google preferences and cached tracks remain distinct. See [provider disclosures](PROVIDER_DISCLOSURES.md).

The interactive player retains **Offline export** for selected accepted revisions. Review audio coverage, source paths and excerpts before exporting a self-contained HTML reader. Export does not generate speech or send source; it contains supported visuals, text and any included cached tracks. Live code browsing and a restorable project backup are not included.

## Coding agents without a mandatory API key

The shared header connection manages Codex runtime, sign-in, model and reasoning. Disconnect stops automatic account reuse. Opening connection settings is not permission to send code. Workspace questions use the native scoped snapshot grant described above; advanced editor, PR and legacy generation operations retain their exact request reviews.

Automatic Claude and Antigravity generation are deferred. Manual handoffs and direct APIs remain available in advanced backend settings, with their existing constraints. They do not substitute for the current Codex workspace. See [agent generation](AGENT_GENERATION.md) for transport and operation details, and [workspace exploration](design/WORKSPACE_EXPLORATION.md) for persistence and recovery limits.

## Optional direct API generation

Local capture, Files, existing saved explanations, and chapter editing do not require an API key. To explicitly use a direct model API:

1. For an operation that offers direct APIs, open the generation chooser and choose **Direct API**. The shared **Connections** header shows the current connection and opens its settings.
2. Choose Gemini, OpenAI, or Anthropic, review its model and provider disclosures, then save that provider's API key to macOS Keychain.
3. In Chapter Studio, select one file and choose **Generate current snapshot**.
4. Inspect the exact selected source files, hashes, prompt, model, and estimate in the request review. The target file is included; up to eleven supporting files from the same snapshot can be selected.
5. Read the privacy and billing notices, check each required acknowledgement, and choose **Send** only when you accept that exact request.
6. Inspect the response. Saving creates an unaccepted local draft. Review and accept it separately in Chapter Studio. The same provider selection is available for one-section updates and PR Guides. Previously reviewed repository-overview requests can be recovered, but a new manually selected overview is not offered in the current UI; use the repository Guide for current codebase exploration.

Code Atlas cannot determine account billing, region, eligibility, quota, or retention settings for a provider key. A displayed model price or free-tier label does not guarantee that a particular request is free or available. Provider-specific disclosures and first-party links are in [Provider disclosures](PROVIDER_DISCLOSURES.md).

## Current boundaries

- The local structural parser covers bounded Rust, TypeScript, TSX, JavaScript, Python, and Go subsets. These are **partial structural** capabilities, not fully qualified language support. Other captured text remains citeable but is **text only**; no language has passed the full structural and narrative qualification gate. See the [language capability matrix](LANGUAGE_CAPABILITIES.md).
- Workspace questions automatically choose a bounded set of supporting files. The source retains a legacy sequential route-chapter builder, whose staged wizard is superseded by the workspace. Neither path guarantees complete repo-wide coverage from one question.
- Generated prose and source identity validation are different checks: a valid citation does not prove that the explanation is semantically correct.
- Gemini, OpenAI, and Anthropic are selectable for file chapters, selective section updates, PR Guides, transient one-hunk PR change explanations, and transient source-range explanations grounded in a verified file range. New repository-overview requests are not exposed in the current UI; saved overview revisions remain in history, and an already-reviewed operation can be recovered. Offline mock fixtures cover every wired provider/operation pair; full capability and acceptance evidence remains open for live provider behavior, eligibility, production transport, response parsing, and manual disclosure review. Automatic Claude/Antigravity, other agent operation pairs and native acceptance remain pending; see [Agent generation](AGENT_GENERATION.md).
- Single-section stale-guide updates and user-facing revision compare/restore are implemented; updating several stale sections in one model request is not supported. Local Git comparisons now show bounded, revision-specific static references to changed files and declarations, with explicit partial results when analysis limits apply; they do not infer runtime impact. Polished onboarding, release signing, and the remaining PR Guide and offline-export acceptance evidence remain tracked; read the [Kata backlog](KATA_BACKLOG.md) for task contracts.
- Code Atlas-authored source uses the MIT License. Third-party notices, captured source, generated answers, and evaluation assets have separate terms; see [the license scope and notices](../THIRD_PARTY_NOTICES.md) before redistributing.

## Where to go next

- [Product blueprint](PRODUCT_BLUEPRINT.md) for the intended user experience and product rules.
- [Interaction contract](INTERACTION_CONTRACT.md) for control behavior, navigation, and side effects.
- [Technical plan](TECHNICAL_PLAN.md) for source-backed architecture and implementation boundaries.
- [Chapter schema](CHAPTER_SCHEMA.md) for saved chapter and evidence records.
- [Troubleshooting](TROUBLESHOOTING.md) for common setup and workflow problems.
- [Contributing](../CONTRIBUTING.md) and [security reporting](../SECURITY.md) for project limits and contact routes.

