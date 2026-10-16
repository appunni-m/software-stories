# Getting started

Code Atlas is an in-progress macOS desktop app. This guide walks through a first local session from the source checkout. This repository does not provide a signed and notarized installer or a user download, so the native app must currently be run from source.

## What you need

- macOS to use the native repository picker, local persistence, and provider keys in Keychain; Gemini narration uses a separate key.
- Node.js 20.19 or newer in the 20.x line, or 22.12 or newer, plus npm.
- Rust 1.85 or newer with Cargo for the Rust 2024 edition; current stable is recommended.
- A local Git working copy to explore.

The project does not set Cargo's `rust-version`, so compatibility of the locked dependencies with Rust 1.85 is not verified. It also has not declared a minimum supported macOS version. The source-only development workflow has not been verified here from a clean macOS account.

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

## Explore a repository locally

1. In the app, choose **Open repository** and select a folder inside a local Git working copy. Code Atlas resolves the Git working-copy root. Choosing a folder checks directory and Git metadata; it does not read repository source.
2. Review the selected project and choose **Create snapshot**. This starts a bounded local read into an immutable snapshot. The capture view reports progress and skipped paths, and a running capture can be cancelled. A partial capture remains labeled partial.
3. Open **Files**. Browse the captured repository tree and select a text file to see its snapshot source and saved explanation together. Click a citation to open the exact cited range. Relationship exploration is optional; its links also return to exact snapshot source. Parser relationships are static evidence, not a complete runtime call graph.
4. Open **Guide** for the repository overview and section-by-section reader. Use **File-by-file coverage** when you want to see which captured files have accepted explanations, drafts, or source limitations.
5. Check the **Codex** connection in the header. Setup is needed once; the existing installed account, model and reasoning are reused. Choose **Explain this file**: Codex selects relevant implementation from the captured index after metadata disclosure, then prepares the exact source review. Confirm and generate; inspect the validated preview, save an unaccepted draft and accept it when ready. In **Guide → Ask the code**, ask a question to build a route without selecting references, then accept it and choose **Build all chapters**. See [AI guide flow](AI_GUIDE_FLOW.md) for scope and recovery details.
6. In the Guide reader, select a chapter and section, inspect its citations or visual hotspots, and use **Previous** or **Continue** when you are ready to move. Audio is optional; the transcript and manual controls remain available without it.
7. To take a guide offline, choose **Offline export** in the Guide reader footer. Select accepted revisions, review each section's audio coverage and the transcript paired with any included cached track, source paths, and excerpts, acknowledge the privacy review, and download the single HTML file. Export never generates or sends narration. The package includes written sections, transcripts, navigation, and supported typed diagrams; live repository browsing is unavailable offline.

## Compare local revisions

After opening a local Git repository, choose **Compare local revisions** from **Project tools**. A snapshot is not required for this operation. Enter a local branch, tag, unambiguous short ref, or full commit ID for each side, then choose **Compare revisions**. The comparison resolves both refs to immutable commit IDs and reads existing local Git objects only; it does not fetch, check out, or modify the working copy.

Expand a changed file to inspect its bounded diff hunks and exact changed-line ranges. Each hunk identifies its base and head path, commit, and blob separately. When all available source revisions parse without errors, Code Atlas lists declarations whose syntax ranges overlap changed lines; it does not infer runtime impact or attach cross-file relationships. Binary, oversized, unsupported, parser-uncertain, and omitted evidence is labeled instead of being presented as complete text. The browser preview cannot run local Git commands; use the native Tauri Mac app for comparisons.

When a complete comparison is ready, the **Build a PR Guide** panel lets you select up to twelve text-diff files. Choose **Review exact provider request** to select an API provider and model and see the selected diff, prompt, endpoint, pricing, and terms disclosures before any request. No request is sent until all consent controls are checked and you choose **Send reviewed request**. Each returned claim must cite an exact line from the correct base or head hunk. Citation buttons expand and scroll to the cited hunk. Keep or reject claims, then accept or reject the draft; decisions append to a separate local PR Guide history and do not change Git refs or file chapters. This flow runs in the native Mac app; the browser preview cannot read Git or call a model API.

For an explanation of one change without creating a saved guide, select a direct API provider and model in the PR Guide panel, then choose **Explain hunk N** beside an eligible text hunk. The local review shows the exact hunk, prompt, encoded request, provider disclosures, and fingerprint. Sending requires the same three explicit acknowledgements and sends only that hunk. Returned claims are checked against exact base/head citations and remain transient; closing the panel returns to the comparison without adding PR Guide history or changing Git refs. This flow also requires the native Mac app.

## Coding agents without a mandatory API key

Codex is the default. The header shows the connection, model and reasoning and opens their settings. It reuses the installed account across generation requests and app restarts; missing login requires explicit sign-in. Ordinary file, tour and planning flows do not show a backend chooser each time. Use **Change backend** in header connection settings to choose direct APIs, deferred Claude setup or manual handoff. Each request still has a fresh source review; choosing a connection does not authorize source transmission.

The [AI guide flow](AI_GUIDE_FLOW.md) describes automatic code selection and route chapter batches. [Agent generation](AGENT_GENERATION.md) describes the other transports and their limits. Automatic Claude generation remains unavailable pending runtime qualification.

Codex is also wired in source for one-section stale repairs from Edit. That route fixes the selected section's declared evidence, reviews old/current source and uses the existing preservation validator. Manual handoffs remain limited to file chapters and selected-source overviews. See [the section-repair instructions](AGENT_GENERATION.md#repair-one-stale-section-with-codex); native acceptance remains pending.

## Optional direct API generation

Local capture, Files, existing saved explanations, and chapter editing do not require an API key. To explicitly use a direct model API:

1. Click the connection in the header, then choose **Change backend → Direct API**.
2. Choose Gemini, OpenAI, or Anthropic, review its model and provider disclosures, then save that provider's API key to macOS Keychain.
3. In Chapter Studio, select one file and choose **Generate current snapshot**.
4. Inspect the exact selected source files, hashes, prompt, model, and estimate in the request review. The target file is included; up to eleven supporting files from the same snapshot can be selected.
5. Read the privacy and billing notices, check each required acknowledgement, and choose **Send** only when you accept that exact request.
6. Inspect the response. Saving creates an unaccepted local draft. Review and accept it separately in Chapter Studio. The same provider selection is available for optional repository orientation, one-section updates, and PR Guides.

Code Atlas cannot determine account billing, region, eligibility, quota, or retention settings for a provider key. A displayed model price or free-tier label does not guarantee that a particular request is free or available. Provider-specific disclosures and first-party links are in [Provider disclosures](PROVIDER_DISCLOSURES.md).

## Current boundaries

- The local structural parser covers a bounded Rust, TypeScript, TSX, and JavaScript subset. These are **partial structural** capabilities, not fully qualified language support. Other captured text remains citeable but is **text only**; no language has passed the full structural and narrative qualification gate. See the [language capability matrix](LANGUAGE_CAPABILITIES.md).
- Each selected file is generated and reviewed individually. There is no generate-all action.
- Generated prose and source identity validation are different checks: a valid citation does not prove that the explanation is semantically correct.
- Gemini, OpenAI, and Anthropic are selectable for file chapters, optional orientation, selective section updates, PR Guides, transient one-hunk PR change explanations, and transient source-range explanations grounded in a verified file range. Offline mock fixtures cover every wired provider/operation pair; full capability and acceptance evidence remains open for live provider behavior, eligibility, production transport, response parsing, and manual disclosure review. Coding-agent routes for file chapters and selected-source overviews are implemented in source: native Codex and manual task handoffs. Automatic Claude/Antigravity, other agent operation pairs and native acceptance remain pending; see [Agent generation](AGENT_GENERATION.md).
- Single-section stale-guide updates and user-facing revision compare/restore are implemented; updating several stale sections in one model request is not supported. Local Git comparisons now show bounded, revision-specific static references to changed files and declarations, with explicit partial results when analysis limits apply; they do not infer runtime impact. Polished onboarding, release signing, and the remaining PR Guide and offline-export acceptance evidence remain tracked; read the [Kata backlog](KATA_BACKLOG.md) for task contracts.
- This checkout has no root license file. Reuse and redistribution terms are unspecified.

## Where to go next

- [Product blueprint](PRODUCT_BLUEPRINT.md) for the intended user experience and product rules.
- [Interaction contract](INTERACTION_CONTRACT.md) for control behavior, navigation, and side effects.
- [Technical plan](TECHNICAL_PLAN.md) for source-backed architecture and implementation boundaries.
- [Chapter schema](CHAPTER_SCHEMA.md) for saved chapter and evidence records.
- [Troubleshooting](TROUBLESHOOTING.md) for common setup and workflow problems.
- [Contributing](../CONTRIBUTING.md) and [security reporting](../SECURITY.md) for project limits and contact routes.

