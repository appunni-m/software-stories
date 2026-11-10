# Troubleshooting

These steps apply to the current source checkout. This repository does not provide a signed and notarized installer or a user download, and it does not publish a public support inbox.

## The browser preview refuses to connect

The Vite preview uses the fixed address `http://127.0.0.1:4173`; it does not move to a second port when that port is occupied. A refused connection means no preview is listening at that address. From the repository root, start `npm run dev` only if the preview is not already running. If the terminal reports that port 4173 is occupied, reuse or restart the existing preview process instead of starting another one on a different port. The browser preview is separate from the native Mac app.

## The repository picker is unavailable or a folder is rejected

The browser preview does not expose the native folder picker. Start the macOS app with:

~~~sh
npm run tauri dev
~~~

Select a directory inside a local Git working copy. Code Atlas requires Git metadata to identify a project and its root. It checks the directory and Git metadata before capture; source bytes are not read until you choose **Create snapshot**.

If macOS denies folder access, choose an accessible local working copy and try again. The app does not execute repository scripts while capturing.

## The Codex account is disconnected

Open **Connections** in the workspace header, choose **Explanations**, and
connect or reconnect Codex with the account already installed on this Mac.
Code Atlas does not ask you for an API key or use Keychain for this connection.
Local browsing and saved explanations remain available while Codex is
disconnected.

## Atlas has no source, or a path cannot be opened

Confirm that a snapshot was created for the selected project. The app does not read the working tree until capture starts. If the path is missing from the workspace, inspect the capture result and its skip reason:

The browser preview cannot read local snapshots. Open Code Atlas in the Mac app to view captured source; this browser limitation does not indicate that the file or snapshot is corrupt.

- **Skipped, excluded, binary, oversized, or invalid encoding** means source text was not captured for that path. The stated reason is the evidence available; Code Atlas does not substitute the current working-tree file.
- If readable source was captured, it can still be opened and cited even when Code Atlas cannot establish a relationship to another file.
- A missing relationship is not proof that two files are unrelated. Ask about the file or behavior directly, or open the relevant source and follow its citations.
- A **partial snapshot** remains partial. Capture again to make a new snapshot; prior snapshots remain separate.

### A file opens, but AI cannot explain it

Capture, indexing and AI context have different limits. A source file above 250,000 bytes is not sent as a whole-file input, but readable captured files can be offered to Codex as range-only candidates; the owned reader retrieves bounded ranges from the selected immutable snapshot. The earlier Fontdone `src/font.rs` eligibility failure is addressed in the current source. Retrieval can still miss relevant ranges, and broad-repository recall and answer quality remain unqualified. Try a focused question about one behavior or symbol; if needed, use Search or Explorer to open the relevant code and ask about that selection. See the [large-source task](kata-tasks/66-large-source-discovery.md) and [retrieval contract](architecture/repository-retrieval.md) for current limits.

### Questions queued after Stop fail to connect

Update to the build containing `96a40a3`. Each dequeued question now checks the actual Codex runtime and restores the saved connection before preparing a new request. This does not resend the stopped question. Earlier failed cards remain failed until deliberately retried; explicit disconnect or a changed account still requires attention in Connections. See [Fontdone acceptance](acceptance/2026-10-17-fontdone.md) for the observed failure and qualification scope.

## Opening a file in Code Lens shows an error

First note how you opened it: **Open in Code Lens** from an Atlas file row, a
source citation, or a relationship. Check that the selected snapshot is the
one shown by the file or chapter. In Atlas, inspect the path's capture
disposition. A path that was skipped, excluded, binary, oversized, or not
valid UTF-8 has no readable captured source for Code Lens to show.

If Atlas says the file was captured but Code Lens still shows an error, report
the exact error text (or a screenshot with source contents obscured), which
control you used, the repository-relative path if it is safe to share, the
snapshot ID, and whether Code Atlas stayed open or closed. If the path itself
is sensitive, share only its extension and a redacted path shape. These details
help distinguish a failed snapshot-source read from a citation or navigation
problem. Do not include source text, API keys, or secrets. Code Lens reads the
immutable snapshot; switching to the live working tree would hide the failure.

## A citation is stale or unavailable

Code Lens opens captured bytes from the selected snapshot. If the citation's source hash or line range does not match, it reports that evidence as stale or unavailable instead of jumping to a nearby line or reading the live file. Select the snapshot the chapter names, inspect the captured source, and use Chapter Studio's freshness information to decide what to review. Do not treat a nearby line as a verified replacement.

## Chapter Studio has no current chapter for a file

Open Guide Plan to see that file's status, then select the file in Chapter Studio. A chapter from an older snapshot is not current for the selected snapshot. Choose **Generate current snapshot** to prepare a draft from the selected snapshot. Code Atlas reuses the saved connection when it supports the operation and asks you to review the exact request before source is sent. There is no bulk generation action.

A generated preview must be saved as a draft before it appears in chapter history. A saved draft still requires a separate review and acceptance. Schema and citation validation do not establish that the prose is accurate; read the explanation against the cited code.

If an unaccepted generated preview remains after restarting the app, use **Review saved preview** or **Discard** in its recovery notice. That recovery applies to generated previews. Do not assume an unsaved Chapter Studio text edit survives closing or restarting the app.

## An AI draft does not connect its explanation to the cited code

This means the generated draft named a function or variable in its evidence metadata without connecting it to the explanation or exact source quote. It does not mean the repository is broken. Current workspace Codex questions attempt one correction within the original conversation, then apply the full source checks. If correction fails, no draft is saved and the question remains available with **Try this question again**. Existing failed questions are not sent again automatically. Older internal section IDs are hidden in the current workspace error display. A retry uses the existing snapshot sharing permission where still valid and may consume Codex account quota.

## A Codex request fails

Workspace explanations use the connected Codex account. Open **Connections**
to confirm that the account is connected and the selected model and reasoning
level are available. A request can fail because of sign-in, network, model
availability, or account usage limits. Code Atlas does not switch to another
model automatically. Resolve the issue and retry the question; an interrupted
question is not resent automatically after restart.

The first question about a snapshot asks permission to send relevant captured
source through Codex. Review that snapshot-specific request before allowing
it. Code Atlas does not require an API key. See [Provider disclosures](PROVIDER_DISCLOSURES.md)
for what is sent and where it is stored.

## AI PR explanations are unavailable

The first release includes read-only local **Changes** browsing. AI-generated
PR Guides and hunk explanations are not part of this release. Use the Changes
view to inspect local commits and source hunks without sending them to an AI
provider.

## Narration is missing or synthesis is rejected

Audio is optional. The transcript and section navigation remain usable without narration.

For local Pocket audio, use the native Mac app on Apple Silicon with macOS 14 or
later. Open **Connections → Voice** and choose **Prepare voice**. Setup downloads
the app-managed runtime, English model, and Alba voice; it needs internet access,
about 1.1 GB of disk space, and available memory. You do not need to install
Python or `uv` yourself. Once prepared, Pocket synthesizes locally and has no
provider usage fee. If setup fails, follow the visible connection or storage
guidance and choose **Prepare voice** again.
If an already prepared installation needs repair, open **Voice details and
troubleshooting → Repair voice setup**. Repair rebuilds the app-managed Python
environment. Setup-only download caches remain available if setup fails so an
explicit retry can reuse them; after a successful setup, Code Atlas removes
those temporary caches and keeps the verified runtime and voice assets. Do not
delete the app data folder to repair narration; that folder also contains
snapshots, indexes, revisions, and saved audio.

Pocket currently offers the English Alba voice only. Alba MacKenna's source recording is credited
under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) in Voice details;
see the [voice catalog](https://huggingface.co/kyutai/tts-voices). If the Mac
does not meet Pocket's platform requirements, keep using the transcript.
The complete runtime, platform limits, and asset identity are documented in
[Pocket runtime](research/pocket-tts-runtime.md).

Choose **Listen** on an answer to open its narration controls. The first
**Create & play** starts local synthesis for that section; synthesis does not
start while a Guide is being generated, saved, or opened. Later plays reuse a
matching complete clip from the local cache. While the first clip is being
created, the transcript and source remain available. If playback fails, keep
using the transcript, check the Pocket status under **Connections → Voice**, and
retry **Create & play** when the voice is ready. Cloud speech is not available
as a fallback.

## A capture or save fails while the Mac is low on disk space

Free space outside Code Atlas' application data folder, reopen the app, and
retry the operation. Do not delete that folder unless you intend to erase all
local snapshots, indexes, revisions, recovery records, and narration audio.
See [Local data and permissions](LOCAL_DATA.md) for the storage path and the
limits of Offline export.

## Build or documentation checks fail

- Confirm that Node.js satisfies the version in the root [README](../README.md), then run npm ci from the repository root.
- The npm run build command checks TypeScript and builds the browser UI.
- The npm run check command runs the local corpus check, Node test suite, UI build, Rust library tests, and Rust crate check.
- The npm run quality:gate command needs a completed, current human-reviewed report. The checked-in template is pending and is not a passing report.

These commands were not run as part of this troubleshooting guide's documentation review. Follow the root README and [Chapter quality gate](CHAPTER_QUALITY_GATE.md) for their exact scope.

## Reporting a security issue

No verified private security-reporting address is published in this checkout. Do not post credentials, private repository content, or an unpublished vulnerability publicly. See [SECURITY.md](../SECURITY.md) for the current reporting limitation.

