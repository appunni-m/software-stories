# Getting started

Code Atlas is an in-progress macOS desktop app. This guide walks through a first local session from the source checkout. This repository does not provide a signed and notarized installer or a user download, so the native app must currently be run from source.

## What you need

- macOS to use the native repository picker, local persistence, and provider keys in Keychain; Gemini narration uses a separate key.
- Node.js 20.19 or newer in the 20.x line, or 22.12 or newer, plus npm.
- A current stable Rust toolchain with Cargo.
- A local Git working copy to explore.

The project does not declare a Rust minimum version or a minimum supported macOS version. The source-only development workflow has not been verified here from a clean macOS account.

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
3. Open **Atlas**. Use the repository tree and its capture/parser details to see what was captured and what was skipped. Select a captured text file and choose **Open in Code Lens** to inspect the source bytes from that snapshot. Atlas relationships are static parser evidence; they are not a complete runtime call graph.
4. Open **Guide Plan** from the project-tools menu. It keeps every recorded path visible and shows whether a file still needs a chapter, has a draft or accepted revision, is excluded, or is unavailable or unsupported.
5. Open **Chapter Studio** from the same menu. Select a file with no chapter for the current snapshot and choose **Generate current snapshot** if you want to use a direct model API. Select the provider and model explicitly, then review the exact request before sending. The generated result starts as a preview or draft and is not automatically accepted.
6. After reviewing and accepting a chapter, open **Walkthrough**. Select a chapter and section, inspect its citations or visual hotspots, and use **Previous** or **Continue** when you are ready to move. Audio is optional; the transcript and manual controls remain available without it.
7. To take a guide offline, choose **Offline export** in the Walkthrough footer. Select accepted revisions, review each section's audio coverage and the transcript paired with any included cached track, source paths, and excerpts, acknowledge the privacy review, and download the single HTML file. Export never generates or sends narration. The package includes written sections, transcripts, navigation, and supported typed diagrams; live Atlas and Code Lens destinations remain unavailable offline.

## Compare local revisions

After opening a local Git repository, choose **Compare local revisions** from **Project tools**. A snapshot is not required for this operation. Enter a local branch, tag, unambiguous short ref, or full commit ID for each side, then choose **Compare revisions**. The comparison resolves both refs to immutable commit IDs and reads existing local Git objects only; it does not fetch, check out, or modify the working copy.

Expand a changed file to inspect its bounded diff hunks and exact changed-line ranges. Each hunk identifies its base and head path, commit, and blob separately. When all available source revisions parse without errors, Code Atlas lists declarations whose syntax ranges overlap changed lines; it does not infer runtime impact or attach cross-file relationships. Binary, oversized, unsupported, parser-uncertain, and omitted evidence is labeled instead of being presented as complete text. The browser preview cannot run local Git commands; use the native Tauri Mac app for comparisons.

When a complete comparison is ready, the **Build a PR Guide** panel lets you select up to twelve text-diff files. Choose **Review exact provider request** to select an API provider and model and see the selected diff, prompt, endpoint, pricing, and terms disclosures before any request. No request is sent until all consent controls are checked and you choose **Send reviewed request**. Each returned claim must cite an exact line from the correct base or head hunk. Citation buttons expand and scroll to the cited hunk. Keep or reject claims, then accept or reject the draft; decisions append to a separate local PR Guide history and do not change Git refs or file chapters. This flow runs in the native Mac app; the browser preview cannot read Git or call a model API.

## Optional direct API generation

Local capture, Atlas, Code Lens, existing saved chapters, and chapter editing do not require an API key. To send a generation request:

1. Open the project-tools menu and choose **AI and narration setup**.
2. Choose Gemini, OpenAI, or Anthropic, review its model and provider disclosures, then save that provider's API key to macOS Keychain.
3. In Chapter Studio, select one file and choose **Generate current snapshot**.
4. Inspect the exact selected source files, hashes, prompt, model, and estimate in the request review. The target file is included; up to eleven supporting files from the same snapshot can be selected.
5. Read the privacy and billing notices, check each required acknowledgement, and choose **Send** only when you accept that exact request.
6. Inspect the response. Saving creates an unaccepted local draft. Review and accept it separately in Chapter Studio. The same provider selection is available for optional repository orientation, one-section updates, and PR Guides.

Code Atlas cannot determine account billing, region, eligibility, quota, or retention settings for a provider key. A displayed model price or free-tier label does not guarantee that a particular request is free or available. Provider-specific disclosures and first-party links are in [Provider disclosures](PROVIDER_DISCLOSURES.md).

## Current boundaries

- The indexer recognizes a defined Rust, TypeScript, TSX, and JavaScript subset. Other files remain visible in inventory, with parser or capture limitations shown.
- Each selected file is generated and reviewed individually. There is no generate-all action.
- Generated prose and source identity validation are different checks: a valid citation does not prove that the explanation is semantically correct.
- Gemini, OpenAI, and Anthropic are selectable for file chapters, optional orientation, selective section updates, PR Guides, and transient Code Lens explanations grounded in a verified source range. The cross-provider fixture/acceptance gate remains open; PR change explanations and coding-agent backends are not implemented as direct API operations.
- Single-section stale-guide updates and user-facing revision compare/restore are implemented; updating several stale sections in one model request is not supported. Cross-file relationship impact in Git comparisons, polished onboarding, release signing, and the remaining PR Guide and offline-export acceptance evidence remain tracked; read the [Kata backlog](KATA_BACKLOG.md) for task contracts.
- This checkout has no root license file. Reuse and redistribution terms are unspecified.

## Where to go next

- [Product blueprint](PRODUCT_BLUEPRINT.md) for the intended user experience and product rules.
- [Interaction contract](INTERACTION_CONTRACT.md) for control behavior, navigation, and side effects.
- [Technical plan](TECHNICAL_PLAN.md) for source-backed architecture and implementation boundaries.
- [Chapter schema](CHAPTER_SCHEMA.md) for saved chapter and evidence records.
- [Troubleshooting](TROUBLESHOOTING.md) for common setup and workflow problems.
- [Contributing](../CONTRIBUTING.md) and [security reporting](../SECURITY.md) for project limits and contact routes.

