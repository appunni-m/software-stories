# Troubleshooting

These steps apply to the current source checkout. This repository does not provide a signed and notarized installer or a user download, and it does not publish a public support inbox.

## The repository picker is unavailable or a folder is rejected

The browser preview does not expose the native folder picker. Start the macOS app with:

~~~sh
npm run tauri dev
~~~

Select a directory inside a local Git working copy. Code Atlas requires Git metadata to identify a project and its root. It checks the directory and Git metadata before capture; source bytes are not read until you choose **Create snapshot**.

If macOS denies folder access, choose an accessible local working copy and try again. The app does not execute repository scripts while capturing.

## Atlas has no source, or a path cannot be opened

Confirm that a snapshot was created for the selected project. The app does not read the working tree until capture starts. In Atlas, inspect the path's capture disposition and parser/index state:

- **Skipped, excluded, binary, oversized, or invalid encoding** means source text was not captured for that path. The stated reason is the evidence available; Code Atlas does not substitute the current working-tree file.
- **Unsupported parser** means the path remains in inventory but is outside the supported indexing subset. If readable source was captured, it can still be opened in Code Lens.
- **Partial index or parse error** means some structural relationships may be absent. A missing edge is not proof that two files are unrelated.
- A **partial snapshot** remains partial. Capture again to make a new snapshot; prior snapshots remain separate.

## A citation is stale or unavailable

Code Lens opens captured bytes from the selected snapshot. If the citation's source hash or line range does not match, it reports that evidence as stale or unavailable instead of jumping to a nearby line or reading the live file. Select the snapshot the chapter names, inspect the captured source, and use Chapter Studio's freshness information to decide what to review. Do not treat a nearby line as a verified replacement.

## Chapter Studio has no current chapter for a file

Open Guide Plan to see that file's status, then select the file in Chapter Studio. A chapter from an older snapshot is not current for the selected snapshot. The current workflow creates a new chapter draft through **Generate current snapshot** and an exact-context Gemini request review. There is no bulk generation action.

A generated preview must be saved as a draft before it appears in chapter history. A saved draft still requires a separate review and acceptance. Schema and citation validation do not establish that the prose is accurate; read the explanation against the cited code.

If an unaccepted generated preview remains after restarting the app, use **Review saved preview** or **Discard** in its recovery notice. That recovery applies to generated previews. Do not assume an unsaved Chapter Studio text edit survives closing or restarting the app.

## Gemini setup or request fails

Gemini generation runs only in the native macOS app. Open **AI and narration setup** from the project-tools menu, check that Keychain is available, and save a key created for the Gemini API. Code Atlas does not read the saved key back into the UI.

Before sending, inspect the request review and all provider notices. A request can fail because of network access, model availability, quota, billing, eligibility, region, or provider limits. Code Atlas does not silently retry another model or switch to paid service. After failure, create and review a fresh request before retrying; prior consent is not reused.

Do not send confidential, personal, or secret source to a provider unless you have reviewed the applicable terms and have authorization to do so. See [Provider disclosures](PROVIDER_DISCLOSURES.md) for request contents, data use, billing, and first-party links.

## PR Guide cannot build or a citation does not open its hunk

PR Guide generation requires a complete local comparison with at least one
selected, bounded text diff. The browser preview cannot read local Git or use
Keychain; run the native Mac app. Rebuild **Review exact Gemini request** after
changing the selected files or refs. If either local ref moved, compare again
so the old request fingerprint and consent are discarded. A citation button
should expand the cited file and scroll to its hunk; use the shown side, commit,
path, blob, and line range to inspect the evidence if it is not visible. A
claim can cite real code and still be semantically wrong, so exclude it or
reject the draft when the explanation overstates what the diff proves.

## Narration is missing or synthesis is rejected

Audio is optional. The transcript and section navigation remain usable without narration. A narration request has its own review and confirmation; it sends the edited spoken script and speech settings, not source excerpts by default. A script can contain up to 16,000 UTF-8 bytes; longer scripts should be divided into separate sections. The review shows the exact sequential requests of at most 4,000 bytes each, split at sentence boundaries when possible and whole-word boundaries otherwise.

## Build or documentation checks fail

- Confirm that Node.js satisfies the version in the root [README](../README.md), then run npm ci from the repository root.
- The npm run build command checks TypeScript and builds the browser UI.
- The npm run check command runs the local corpus check, Node test suite, UI build, Rust library tests, and Rust crate check.
- The npm run quality:gate command needs a completed, current human-reviewed report. The checked-in template is pending and is not a passing report.

These commands were not run as part of this troubleshooting guide's documentation review. Follow the root README and [Chapter quality gate](CHAPTER_QUALITY_GATE.md) for their exact scope.

## Reporting a security issue

No verified private security-reporting address is published in this checkout. Do not post credentials, private repository content, or an unpublished vulnerability publicly. See [SECURITY.md](../SECURITY.md) for the current reporting limitation.

