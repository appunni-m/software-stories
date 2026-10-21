# Required agent generation coverage

> **Current sequencing — October 22:** Codex is the immediate delivery and validation target. The user deferred Claude to `8gqw` (priority 2); it no longer blocks the current Codex gate `2r6n`. The complete multi-backend milestone retains Claude and Antigravity requirements. Building/tests and a small synthetic Codex account generation are authorized. Historical source-only statements below describe earlier checkpoints.

This is the implementation contract for multi-agent generation, reviewed October 22, 2026. It describes required behavior and current source gaps. Codex file-chapter generation has completed one synthetic native baseline, including draft/acceptance, citation navigation and manual section advance. Other operations and full backend qualification remain pending. See [executed validation](acceptance/2026-10-16-codex-validation.md).

## Product requirement

Codex and Claude Agent, powered by Claude Code, must each generate explanations inside Code Atlas through a supported automatic integration. The user selects the agent, reviews source and settings, starts the run, follows progress and reviews the result in the app. They do not have to configure a direct API provider or copy prompts and results between applications.

Direct APIs remain separately selectable. Manual handoff remains separately labeled. Neither completes an automatic agent requirement. A backend that is unavailable for the requested operation must explain the gap and preserve the user's choice; it cannot silently select an API.

## Required operation matrix

**Source** means registered app dispatch and local validation are present in source, with runtime acceptance still pending. **Missing** means the automatic operation is unavailable; gated source paths do not establish an executable, accepted account/runtime route. None of these entries means production support.

Claude now has invocation/stream-consumer, shared chapter/section review helpers, a managed process supervisor and explicit installation/account-metadata inspection in source. An advanced setup action now calls a registered native SDK catalog check, with bounded SDK/host supervision and independent catalog admission. It accepts installation path hints, derives identities natively, verifies the inspected SDK bytes and displays candidate model/effort metadata. It supplies no repository source or user prompt and never enables generation. Packaged runtime installation, qualified discovery, generation workflow acceptance, account/runtime qualification and lifecycle acceptance remain missing, so its automatic operation cells stay **Missing**. Constructing the manager or mounting setup starts no process. An internal native chapter pipeline now calls the worker and shared validators. A native owner now retains a short-lived observed catalog connection and supplies it to a registered offline chapter-review command; the renderer supplies only its opaque ID and requested settings/source. The app now calls offline review for file/overview/section/range/PR Guide/hunk operations after native catalog inspection, with supported model/effort/turn settings, fixed approval and exact request display. Range and Git targets stay fixed to the original selection; chapter context retains its bounded captured-file picker. Five native send commands, including planning, now resolve strict stored-review inputs through a release-owned admission factory; its qualification registry is empty, so it rejects every inference attempt before consumption/execution. Automatic app controls now call those send helpers for all six explanation scopes plus repository planning, with review consent, progress/cancel, chapter save/discard, transient range/hunk citations and separate guide results. Renderer expiry preserves already-started work; close/backend changes revoke it, dispose abandoned chapter previews and refresh recoverable history. The registry remains empty and these flows are unaccepted. Catalog observations/offline reviews cannot authorize inference. These pieces do not qualify an automatic operation.

Codex reviews now bind a native process connection, executable digest and observed account metadata across every operation; worker/result checks reject changed observations. This binding was built and exercised in the synthetic native file flow; adversarial identity/change cases remain unaccepted. It is source change detection, with identity, execution-race and package limits described in [Codex review identity](CODEX_REVIEW_IDENTITY.md). It closes no automatic-operation acceptance cell.

| Operation and result | Codex | Claude Agent | Antigravity |
| --- | --- | --- | --- |
| Plan a repository explanation with file coverage, ordering and evidence | Source · bounded planning and local revisions; orchestration/acceptance pending | Missing | Missing |
| Generate a file chapter from selected captured files | Native synthetic baseline · full acceptance pending | Missing | Missing |
| Generate an optional overview of selected captured files | Source | Missing | Missing |
| Regenerate one stale section while preserving eligible unchanged sections | Source | Missing | Missing |
| Explain the exact selected source range in Files | Source | Missing | Missing |
| Generate a guide for a selected immutable Git comparison | Source | Missing | Missing |
| Explain one selected base/head diff hunk | Source | Missing | Missing |

File chapters, overviews, section repair, source-range explanations and both Git-change operations are required for Codex and Claude Agent. Repository planning must also support these agents when the AI narrative planner is implemented; a static call list does not satisfy generated narrative planning. Antigravity remains a separately required M7 integration subject to its exact permitted surface and account route. A direct API result in any row cannot qualify its agent counterpart.

Generation through either agent must retain the same language evidence limits and visual/chapter schema as other transports. Switching agent cannot turn a partial parser into full language support, establish runtime behavior from static evidence, or bypass source citations. Narrative and visual quality remain separate acceptance gates.

The [repository planning contract](REPOSITORY_PLANNING.md) now has shared native context preparation, distinct registered Codex/Claude plan review commands, full-contract rebuild helpers and a separate source-grounded result validator/response bridge. Preparation and reviews start no model or API request. Native planning send commands now use those contracts and each agent's existing worker lifecycle, with active source/result validation and exact interruption metadata. Claude remains qualification-gated. Native senders now publish unaccepted local plan records with registered list/revalidate/discard commands. The app now has both native planning callers, exact fresh-review recovery routing and a saved-draft reader with explicit step/source actions. The saved-plan reader now explicitly accepts immutable revisions and resolves only its exact native generation notice after source replay. Codex bounded planning is Source; repository-scale orchestration, selective updates and native acceptance remain pending. Claude stays Missing behind empty runtime qualification. Native dispatch itself is unexecuted source. Wrong direct API commands now check backend/purpose before removing a review or loading credentials; that source correction still needs native acceptance.

## Result lifecycle — unexecuted source

All registered Codex operation senders now retain their original admitted clock and native owner through full active review rebuilding, independent validation and final guarded publication. Claude chapter/guide validation now prepares in-memory candidates outside its revocation lock and only holds it for publication. Both use nonblocking local result-store entry points, retain failure/rejection recovery notices and refresh app recovery after every operation, including ranges/hunks and abandoned replies. See [the exact source contract and delivery limits](AGENT_RESULT_LIFECYCLE.md). These changes do not qualify any runtime, account or operation; native acceptance and the empty Claude registry remain unchanged.

## User flow and configuration

1. Choose a repository and capture its source. Browsing and setup require no API key.
2. From any generation action choose a coding agent or direct API. Agent choices remain visible even when an API preference was previously saved.
3. For the chosen agent, show installation, connection and operation availability, with a specific recovery action. Connecting sends no source. Use vendor-supported authentication; keep account usage separate from API billing.
4. Remember non-secret defaults independently for each agent. Offer only its supported model, reasoning/effort, approval policy and run limits. Allow deliberate overrides. Unsupported settings are unavailable with an explanation; a fixed policy is displayed as fixed.
5. Review the exact captured context and effective settings. Changing context, backend, account or effective settings invalidates the review and consent. Action approvals and consent to send source remain separate.
6. Start once, show progress, permit cancellation and retain the selected backend through failures. Do not silently retry using another backend or account route.
7. Validate output locally. Chapter and PR Guide results remain unaccepted drafts; range/hunk explanations remain transient. Preserve the existing operation-specific history rules.
8. Recover the original operation after interruption with fresh review and consent. Missing range/comparison identity requires reselection; recovery cannot widen it into a whole-file request.

## Reuse the existing implementation

Use vendor-maintained agent loops: the official Codex app-server, the official Claude Agent SDK or its documented programmatic CLI, and a qualified official Antigravity surface. Evaluate maintained ACP adapters where they fit. Code Atlas owns context review, dispatch, lifecycle and validation; it does not implement a new agent loop, credential system or universal model/effort catalog.

The dated [ACP adapter review](research/2026-10-22-agent-protocol-evaluation.md) records exact registry versions, provenance, current probe observations, and why ACP inclusion does not qualify a runtime. Keep the existing Codex app-server path as the immediate acceptance target; do not enable another ACP candidate from registry metadata alone.

Keep operation context and validation independent of transport. Extend the existing shared chapter context rather than building an API request to obtain an agent prompt. Range and PR context builders now have that separation for the Codex source routes; reuse those builders when wiring Claude operations.

| Existing source | Responsibility to reuse or extend |
| --- | --- |
| [provider.rs](../src-tauri/src/provider.rs) | Immutable chapter/section context, review fingerprints and operation-specific range/PR validators |
| [codex_commands.rs](../src-tauri/src/codex_commands.rs), [codex_agent.rs](../src-tauri/src/codex_agent.rs) | Existing native Codex dispatch and managed runtime lifecycle |
| [claude_agent.rs](../src-tauri/src/claude_agent.rs), [claude_worker.rs](../src-tauri/src/claude_worker.rs), [claude_connection.rs](../src-tauri/src/claude_connection.rs), [claude_commands.rs](../src-tauri/src/claude_commands.rs) | Candidate Claude protocol/process supervision, sanitized inspection, offline review and qualification-gated registered send commands; app generation workflow acceptance and runtime qualification remain open |
| [claude_model_discovery.mjs](../src-tauri/resources/claude_model_discovery.mjs), [claude_discovery_worker.mjs](../src-tauri/resources/claude_discovery_worker.mjs), [claude_discovery.rs](../src-tauri/src/claude_discovery.rs), [claude_models.rs](../src-tauri/src/claude_models.rs) | Candidate SDK initialization/projection, native supervision and catalog admission, called by the explicit setup check; qualified installation, generation dispatch and acceptance remain open |
| [claude_connections.rs](../src-tauri/src/claude_connections.rs), [claude_generation.rs](../src-tauri/src/claude_generation.rs) | Native short-lived observed connection, registered offline operation reviews and internal chapter/range/Git worker/validation pipelines; release-owned admission now gates registered send commands, with no eligible profile; app generation flow and actual qualification remain missing |
| [chapter_generation.rs](../src-tauri/src/chapter_generation.rs), [lib.rs](../src-tauri/src/lib.rs) | Chapter/section preservation and draft preparation; separate PR Guide preparation |
| [recovery.rs](../src-tauri/src/recovery.rs), [recovery_commands.rs](../src-tauri/src/recovery_commands.rs) | Bounded journal-owned interruption metadata and local exact-evidence restoration without source, credentials or reusable consent |
| [App.tsx](../src/App.tsx), [CodexChapterDialog.tsx](../src/CodexChapterDialog.tsx), [ClaudeOperationReview.tsx](../src/ClaudeOperationReview.tsx), [ClaudeGenerationPanel.tsx](../src/ClaudeGenerationPanel.tsx), [claudeReview.ts](../src/lib/claudeReview.ts) | Backend chooser, Codex operation/settings flow, and Claude supported-settings/exact offline review; Claude inference UI has source wiring but remains unaccepted |

## Implementation order and owners

1. **Codex completion (`z3an`, priority 0):** qualify the implemented chapter, overview, section, range, planning and Git-change routes and exact-scope recovery. Preserve section identity and untouched content.
2. **Current Codex coverage and UX gate (`2r6n`, priority 0):** require effective supported controls, cancellation and recovery for the Codex operations above. Record exact runtime/adapter versions and dated acceptance before promoting a matrix cell.
3. **Deferred Claude Agent (`8gqw`, priority 2):** implement and qualify the maintained SDK/CLI adapter, settings, lifecycle, account route and all required operations in later work. A manual handoff or another vendor's API cannot close that task. See [the deferred scope](kata-tasks/41-claude-agent-backend.md).
4. **Antigravity (`m00g`):** qualify its supported integration and account route independently. An unresolved requested backend keeps M7 open.

Codex range dispatch now shares immutable range context with direct APIs and uses the existing managed runtime and independent citation validator. It stages only the excerpt, returns a transient explanation and never prepares a chapter draft. The full captured-file identity remains in the review; staged-content integrity uses the excerpt digest. Native acceptance, runtime qualification and scope-preserving interruption recovery remain pending.

Runtime qualification, login, real generation and native acceptance remain pending. This contract update ran no tests, builds, agent processes or provider requests.

Codex Git-change source dispatch now calls the shared transport-independent comparison/hunk builders. Four native review/send commands and the PR Guide backend chooser connect model/effort controls, exact review/consent, progress and cancellation to the managed runtime without an API prerequisite. Revalidation binds refs, resolved commits, selected path pairs/hunk, patch hashes, schema and settings. Neutral patch aliases keep arbitrary Git rename labels out of staging paths; the reviewed mapping retains original citation identities. Guides pass independent base/head citation validation into separate unaccepted PR Guide history; hunk results remain transient with citation navigation. A worker panic preserves its interruption notice. Full Git-scope recovery, exact runtime/account/boundary qualification and native acceptance remain pending. Claude automatic dispatch remains Missing; showing its pending option or metadata check cannot satisfy that row. No tests, builds or agent processes ran for this source delta.

Claude candidate range/Git source delta: all six required explanation operations now have internal context/revalidation, worker-dispatch and result-validation paths. Three additional registered native commands prepare **offline** range/guide/hunk reviews from an observed connection; their frontend helpers now have explicit local-preview callers through the shared operation review panel. The shared operation guard rejects cross-purpose/mixed contexts. Claude context travels over stdin, while Codex keeps its separate reviewed physical patch mapping. Range/hunk output remains transient and guide results use separate unaccepted PR history. Registered native Claude send commands are now gated by an empty release qualification registry, have app generation callers but no runtime acceptance, and establish no approval/runtime qualification, so every automatic Claude cell stays **Missing**. Source implementation cannot close the priority-0 multi-agent requirement. No tests, builds or agent/model calls ran.


Claude lifecycle source delta: the internal pipeline now holds one revocable native operation owner, shares cancellation with the worker and guards consumption/publication against connection replacement. Admission expiry remains enforced; the same consumed bounded run may finish after ordinary review expiry without consent renewal. Async owner drop revokes blocking access, while already-published unaccepted previews/guides can remain recoverable. Applicable account/runtime/isolation qualification, trusted installation, generation/renderer/recovery workflow acceptance and packaged acceptance remain open. This does not change any Missing cell. No tests, builds or runtime/model calls ran.


Codex recovery source delta: newer interruption notices now restore exact ranges or the original Git comparison/path pairs/hunk after journal-owned native local validation of hashes and refs. The app keeps Codex selected, requires fresh configuration/review/consent, and asks guide recovery users to inspect saved history first. Review creation alone no longer removes the old notice; successful result adoption resolves it. Legacy/incomplete targets require reselection. This is source wiring without packaged acceptance; no cell becomes Supported. Claude recovery now uses the same native evidence validator and restores its original backend/scope, without resuming consent or enabling its unqualified runtime. No tests, builds or runtime/model calls ran.


## Both-agent exact recovery — source progress

Claude and Codex now share native journal-owned chapter/range/Git evidence recovery and a backend-tagged Git scope. Claude records reopen the original agent, exact chapter/section/range/comparison/path-pair/hunk and a fresh review. Section recovery uses project-wide accepted history. Chapter recovery lists possible pending/saved results before another request; guide recovery inspects separate PR history. No prior runtime/settings authorization or consent resumes. Preparation, failure, cancellation, closing and unavailable runtime preserve the original notice; validated wanted result adoption resolves it. API preview creation cannot resolve an agent notice.

This source has not been built or exercised. Claude's release registry is still empty, all automatic Claude cells remain Missing, and native runtime/account/boundary/recovery acceptance remains open. Deferred cases are in `xymb`.


## Accepted-step chapter dispatch — unexecuted source

Accepted learning steps now have separate native Codex/Claude review builders and an agent-only app chooser. Both derive the exact question, prerequisites, original snapshot and fixed cited source from the accepted plan record. Their existing file-chapter senders and validators are reused; no direct API prerequisite or fallback is introduced. Preflight/result rebuilding and interruption recovery replay the same immutable plan scope. This adds no background step loop or claim of complete-repository coverage. Durable chapter-to-step association and native/quality acceptance remain pending. See [the detailed contract](PLANNED_AGENT_CHAPTERS.md). Codex remains Source and every automatic Claude cell remains Missing behind its empty qualification registry. No tests, builds, app launches, agent probes, login or model requests ran.


## Chapter request and step lineage — unexecuted source

Both native agent chapter result closures now attach version-4 original-generation metadata before guarded preview publication. Previews and immutable history retain request/settings/source-digest and optional exact accepted-plan scope together. The app exposes generation records, recorded recovery matches and saved step links without inferring association from filenames. Direct API/manual handoff routes cannot create agent lineage through these closures. Legacy v1–v3 chapters/v3 previews stay readable; compatibility, edit/restore, lifecycle and real agent acceptance remain pending in `xymb`. See [the detailed record contract](CHAPTER_GENERATION_RECORDS.md). This does not change Codex's Source or Claude's Missing status and enables no new runtime route.
