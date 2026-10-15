# Generate a chapter from an accepted learning step

Source checkpoint: October 16, 2026. This flow has not been built, launched or exercised. Codex runtime acceptance remains pending. Automatic Claude generation remains unavailable behind the empty native qualification registry.

## User flow

1. In Guide → Learning plans, open an accepted plan revision and choose a reading step. An unaccepted draft offers a separately labeled general file explanation; it does not carry the step question.
2. Choose **Explain this step with an agent**. Native code replays the accepted proposal against its original captured evidence, checks the reviewed record digest and locates that exact step. A missing/changed record or unavailable source stops preparation.
3. Choose Codex or Claude Agent. This chooser has no API prerequisite and no API/manual fallback. Each agent keeps its existing installation/account, supported model and reasoning controls. Codex approval remains fixed to `never`; Claude approval remains fixed to `dontAsk` with supported turn bounds. These adapters request no command, write or external tools; configurable interactive tool approvals are not implemented by this change.
4. Review the step question, overall plan question, learning prerequisites, fixed source and effective settings. Native agent review preparation derives all plan prose and source paths again from accepted history. The renderer supplies the accepted revision/step/digest selector and requested agent settings, not a prompt or replacement question.
5. Explicitly consent and generate one chapter. Existing agent progress, cancellation, operation ownership, independent claim/visual validation and recoverable preview publication apply. Saving an unaccepted chapter and accepting it remain separate actions.

No other step runs automatically. Back/Next remain reading controls and do not imply that a chapter was generated, read or understood.

## Exact request identity

`planned_chapter.rs` derives `PlannedChapterScope` from one accepted immutable record:

| Field | Meaning |
| --- | --- |
| `target.revisionId`, `recordSha256`, `stepId` | Exact accepted revision, canonical typed-record digest and stable step identity |
| `snapshotId`, `primaryPath`, `selectedPaths` | Original snapshot, primary file and sorted distinct paths cited by that step's validated claim, including the primary file |
| `planQuestion`, `question` | Original plan question and this step's learning question |
| `prerequisites` | Original prerequisite IDs/questions, describing learning order rather than runtime calls |
| `planContext` | Original plan coverage, capture/index status and evidence/contract digests |

Chapter source remains bounded to 1–12 captured files and 250,000 source bytes; the existing encoded request limit also applies. Prerequisite questions add no uncaptured source or authority. The full plan's coverage limits remain visible in its reader. A step can be prepared from an earlier immutable revision without silently replacing it with the latest head. The app requires the original snapshot to be selected.

Both native review builders add the scope as `generationConfig.plannedChapter` and descriptive prompt data before the shared encoder computes the request digest and fingerprint. The contracts are `codex-planned-chapter-v1` and `claude-planned-chapter-v1`; their operation remains `fileChapter`. They reuse each vendor's existing integration, rather than implement another agent loop.

The prompt asks for a concrete answer grounded in the selected code, a brief prerequisite recap and explicit unknowns. Plan questions and metadata are untrusted data, not instructions or source evidence. Structural/source validation cannot establish that the chapter is a useful answer: narrative quality and comprehension review remain acceptance requirements.

Preflight and active result rebuilding replay accepted history, reconstruct the complete scope and compare the reviewed contract, source and settings. A successful rebuild cannot renew source consent, create another run or reset its deadline. The [result lifecycle](AGENT_RESULT_LIFECYCLE.md) still owns cancellation through validation and publication.

## Interrupted work

New planned chapter notices retain the complete original scope under `providerRequest.plannedChapter`, alongside the original agent/file identity. Existing journal byte/count limits apply; scope is never silently truncated. Older ordinary chapter notices remain ordinary requests and cannot establish a plan association.

**Review interrupted work** rereads the native journal, replays the accepted plan and compares the entire original scope, then validates the chapter source. Changed/missing plan records, changed index/evidence, mismatched step/question/source or mixed operation targets stop recovery. The original notice stays available.

Successful recovery returns to Guide and opens the original agent-only step request. Check recoverable previews and chapter history first: a lost response may already have published a result. Choose current supported settings, create a new exact review and consent again. No runtime thread, previous settings authorization or source consent resumes. Preparing or closing a review cannot resolve the old notice. The existing successful wanted-result callback resolves it after adoption.

## Remaining work

- Execute both agents' complete native workflows, account/runtime qualification, lifecycle and failure cases. An unavailable Claude option does not satisfy multi-agent support.
- Qualify semantic answer quality across representative languages and real repositories. Prompt instructions are not proof of teaching quality.
- Qualify the new [version-4 generation records and step links](CHAPTER_GENERATION_RECORDS.md). Both native agent result paths now retain original request/settings/plan scope in previews and immutable chapters, and the plan reader exposes recorded links. Legacy same-file chapters remain unassociated; metadata does not establish account attestation, teaching quality or step completion.
- Add deliberate multi-unit orchestration, omission/failure tracking and dependency-based selective updates. One generated step does not explain the entire repository.
- Complete the deferred cases in Kata `xymb`; keep `6qk9`, `z3an`, `8gqw`, `tpta` and `2r6n` open until their actual acceptance requirements are met.

No tests, builds, app launches, agent/SDK probes, login or model requests were run for this source checkpoint. Changed Rust regions were formatted; this does not establish compilation or runtime behavior.
