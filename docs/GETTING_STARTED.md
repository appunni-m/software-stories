# Getting started

Code Atlas is an in-progress macOS desktop app. This guide walks through a first local session from the source checkout. This repository does not provide a signed and notarized installer or a user download, so the native app must currently be run from source.

## What you need

- macOS to use the native repository picker, local persistence, and Keychain-backed Gemini setup.
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

The first command installs the versions recorded in the npm lockfile. The second starts the Vite development server and the Tauri desktop shell.

~~~sh
npm run dev
~~~

This last command starts a browser-only UI preview at http://127.0.0.1:1420. It is useful for checking layout, but it cannot use the native folder picker, macOS Keychain, local repository commands, or Gemini requests. Use the Tauri command for the native workflow.

## Explore a repository locally

1. In the app, choose **Open repository** and select a folder inside a local Git working copy. Code Atlas resolves the Git working-copy root. Choosing a folder checks directory and Git metadata; it does not read repository source.
2. Review the selected project and choose **Create snapshot**. This starts a bounded local read into an immutable snapshot. The capture view reports progress and skipped paths, and a running capture can be cancelled. A partial capture remains labeled partial.
3. Open **Atlas**. Use the repository tree and its capture/parser details to see what was captured and what was skipped. Select a captured text file and choose **Open in Code Lens** to inspect the source bytes from that snapshot. Atlas relationships are static parser evidence; they are not a complete runtime call graph.
4. Open **Guide Plan** from the project-tools menu. It keeps every recorded path visible and shows whether a file still needs a chapter, has a draft or accepted revision, is excluded, or is unavailable or unsupported.
5. Open **Chapter Studio** from the same menu. Select a file with no chapter for the current snapshot and choose **Generate current snapshot** only if you want to try Gemini. The generated result starts as a preview or draft and is not automatically accepted.
6. After reviewing and accepting a chapter, open **Walkthrough**. Select a chapter and section, inspect its citations or visual hotspots, and use **Previous** or **Continue** when you are ready to move. Audio is optional; the transcript and manual controls remain available without it.
7. To take a guide offline, choose **Offline export** in the Walkthrough footer. Select accepted revisions, review the source paths and excerpts included, acknowledge the privacy review, and download the single HTML file. The current export includes written sections, navigation, and supported typed diagrams; it does not include narration audio or live Atlas/Code Lens destinations.

## Optional Gemini generation

Local capture, Atlas, Code Lens, existing saved chapters, and chapter editing do not require an API key. To send a generation request:

1. Open the project-tools menu and choose **AI and narration setup**.
2. Review the current model and provider disclosures, then save your Gemini API key to macOS Keychain.
3. In Chapter Studio, select one file and choose **Generate current snapshot**.
4. Inspect the exact selected source files, hashes, prompt, model, and estimate in the request review. The target file is included; up to eleven supporting files from the same snapshot can be selected.
5. Read the privacy and billing notices, check each required acknowledgement, and choose **Send** only when you accept that exact request.
6. Inspect the response. Saving creates an unaccepted local draft. Review and accept it separately in Chapter Studio.

Code Atlas cannot determine the billing state, region, eligibility, or quota of the Google Cloud project attached to your key. A displayed free-tier model does not guarantee that a particular request is free. The current Gemini disclosures and first-party links are in [Provider disclosures](PROVIDER_DISCLOSURES.md).

## Current boundaries

- The indexer recognizes a defined Rust, TypeScript, TSX, and JavaScript subset. Other files remain visible in inventory, with parser or capture limitations shown.
- Each selected file is generated and reviewed individually. There is no generate-all action.
- Generated prose and source identity validation are different checks: a valid citation does not prove that the explanation is semantically correct.
- Gemini is the only direct text-generation provider in the current source. Other direct APIs and agent backends are planned, not selectable.
- Fine-grained section regeneration and a user-facing revision compare/restore workflow are still tracked product work. A first offline export path is available, but it has not passed the full offline-package acceptance workflow. Release signing and PR explanations also remain outstanding; read the [Kata backlog](KATA_BACKLOG.md) for task contracts.
- This checkout has no root license file. Reuse and redistribution terms are unspecified.

## Where to go next

- [Product blueprint](PRODUCT_BLUEPRINT.md) for the intended user experience and product rules.
- [Interaction contract](INTERACTION_CONTRACT.md) for control behavior, navigation, and side effects.
- [Technical plan](TECHNICAL_PLAN.md) for source-backed architecture and implementation boundaries.
- [Chapter schema](CHAPTER_SCHEMA.md) for saved chapter and evidence records.
- [Troubleshooting](TROUBLESHOOTING.md) for common setup and workflow problems.
- [Contributing](../CONTRIBUTING.md) and [security reporting](../SECURITY.md) for project limits and contact routes.

