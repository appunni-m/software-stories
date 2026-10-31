# From a question to code explanations

> **Large-repository status — October 31:** Codex discovery now keeps readable captured files above 250,000 bytes as range-only candidates, and its owned retrieval tools can read bounded ranges from the selected immutable snapshot. These files are not sent as whole-file sources. The old Fontdone eligibility failure is addressed in the source implementation; broad-repository recall, range selection, answer quality and packaged acceptance remain open. See the [retrieval contract](architecture/repository-retrieval.md), [large-source task](kata-tasks/66-large-source-discovery.md) and [Fontdone evidence](acceptance/2026-10-17-fontdone.md).

> **Current workspace — October 31:** the normal workflow is the pinned composer with Guide, Flow and chat history inside each exploration. AI selects relevant captured code in the background after snapshot/account/runtime permission; users can search, attach a file or select code for added context. The staged route wizard described below is a historical implementation contract, not the current navigation flow. Use [Getting started](GETTING_STARTED.md) for the current user journey. Broader multi-unit orchestration remains tracked in `a1sm`.

Code Atlas uses Codex to choose relevant implementation from an immutable snapshot. The connection belongs to the workspace header; a generation dialog does not ask you to choose it again. This is a bounded learning route, not a claim that every file or runtime path has been covered.

## Connect once

The header shows a compact Codex connection state. Click it to change connection settings, including the shared model and reasoning level. The app restores an existing installed Codex account at startup when Codex is selected, unless you explicitly disconnected. Account restoration and model discovery can contact Codex account services; they do not send repository source. Missing login opens setup when discovery needs it. Credentials remain managed by Codex.

Appearance is also in the header: **System**, **Light**, or **Dark**. The choice persists locally. Both palettes use neutral editor surfaces, blue actions, readable secondary text and distinct warning/error colors.

## Ask; let AI choose the code

1. Capture the repository locally. In **Guide → Ask the code**, enter the question you want answered and choose **Find code and build route**. A file’s Explain action and a repository tour can also discover their own supporting code.
2. Code Atlas loads the saved symbol/relationship index or builds it locally. Before discovery it shows the exact metadata: captured eligible code paths, sizes, bounded symbol names, static links, documentation labels and your question. Choose **Find relevant code** to share that metadata with Codex. This uses your account quota.
3. Codex proposes up to 12 implementation files totaling at most 250,000 captured bytes. Native validation rejects unknown paths, duplicate files, excluded files, oversized choices and omission of a fixed target. If the index is unavailable, paths remain usable and the limitation is visible. Metadata is bounded to a window; omitted paths are counted.
4. The next local review contains the exact chosen source, prompt and saved settings. Confirm it before sending code. Opening this review does not send file contents. The planner returns an unaccepted, cited reading sequence with prerequisites and known unknowns.

No manual reference checklist is required in this Codex path. The current discovery worker does not browse the live checkout or execute repository code. Its metadata window and 12-file source budget can miss relevant code in a large repository. Claude automatic discovery and multi-window repository orchestration remain pending.

## Code is the evidence

Documentation is guidance. The native scan labels up to 20 captured text documents, checking the first 40 lines for explicit deprecation/archive markers. The planner shows these labels and lets you open the original captured document. Other documents are **not verified against code**; a timestamp does not establish staleness.

Generation instructions require implementation evidence for code behavior and a specific cited contradiction before calling a document outdated. Native claim validation rejects a `codeBehavior` claim supported only by Markdown, MDX, reStructuredText or AsciiDoc. This check does not prove the model’s semantic interpretation is correct; review the code citations.

## Build the chapters together

After reviewing and accepting a route, choose **Build all chapters**. Code Atlas prepares the exact request for each remaining step locally. Review the batch’s files/prompts/settings once, then start it. Codex generates the chapters sequentially using the header connection, and each validated result is saved as an independent **unaccepted draft**. Choose **Review chapter** beside a saved result to open its editor directly.

- Existing draft/accepted chapters with the exact plan revision, record digest and step are skipped. Rejected and unrelated chapters are not skipped.
- Each request is rebuilt just before dispatch. Its fingerprint and body digest must still match the reviewed source, account, runtime, model and settings. A changed request stops the queue.
- Closing stops the queue and requests cancellation of an active turn. A save already in progress may complete. Completed drafts remain saved.
- Diagram nodes and edges refer to validated evidence in their own section; the app creates exact code anchors and source links locally. The model does not need to copy the same source quotes into the diagram again.
- Errors stop generation without automatic inference retries. Check Edit, pending previews and recovery notices before trying again. A lost save reply is reconciled against the exact revision ID.
- Reopening the action derives remaining work from saved chapter lineage. It does not persist or automatically resume an active queue across app restarts. Acceptance and playback remain deliberate actions.

The app still has separate quality and release gates for narrative depth, diagrams, language coverage, accessibility and reliability. Passing format/citation checks alone does not establish an excellent explanation.

## Reopening saved routes

Saved routes keep their original accepted revision, source snapshot and step
links. The reader independently checks captured code and claims locally. It
supports the three known planning request contracts across instruction/schema
changes and preserves both historical schema-1 record encodings; new records
use schema 2. Opening history starts no model request. Unsupported contracts or
changed/unavailable evidence produce distinct errors and preserve the record.
Older app binaries are not qualified to read new schema-2 records.

The [saved-route qualification](acceptance/2026-10-17-saved-route-replay.md)
contains the real saved fixture, simulated upgrade boundaries and pending
packaged-app acceptance. It does not qualify whole-repository orchestration.
