---
kind: task
key: codex-account-backend
parent: t29
title: Implement Codex app-server chapter generation with account sign-in
priority: 0
labels: agents, provider
---

## User outcome
The user connects an eligible ChatGPT account once and generates chapters through Codex without configuring a direct API key for each workflow.

## Research decision — 2026-10-15
Use official Codex app-server over Rust-managed stdio. The initial local-app implementation uses the runtime's native managed ChatGPT login. OpenAI's separate direct Sign in with ChatGPT documentation remains relevant to a possible qualified distribution; Code Atlas has not implemented direct client registration, PKCE or token sharing. Establish this project's license, exact account/distribution eligibility and pinned runtime contract. An older installed runtime is not evidence that current app-server lacks required controls.

## Implementation update — 2026-10-15

Source now includes the managed runtime in `codex_agent.rs`, native commands in `codex_commands.rs`, shared chapter context/schema/evidence validation, `CodexChapterDialog`, and the app backend picker. The route covers one file chapter with selected supporting files or an overview of 1–12 selected captured files. It includes executable setup, native browser login/cancel/logout, explicit runtime `model/list`, model/effort preferences, fixed `never` approval policy, exact review/consent, staged context, progress/cancel, unaccepted draft save/review and child shutdown.

Schema/configuration checks require restricted reads, structured output and ephemeral threads. Shell/web/plugin-related controls are explicitly configured and unexpected tool events are rejected; this is not proof of a universal `tools: none` boundary. Runtime `model/list` provides capability choices, not account-entitlement or remaining-quota proof. Threads are ephemeral; persistent thread resume is absent. Exact Code Lens ranges, PR Guides and single-hunk explanations are now connected in source; full runtime and native workflow acceptance remain open.

The source now also connects **one-section stale repairs** to Codex. The common chapter context builder owns freshness, accepted-base/latest-revision checks, exact declared current source, bounded old/current source, section-only prompt/schema and dependency filtering; API and agent transports use that same context. The Codex v2 operation review binds the exact chapter/section revision metadata, prompt, schema, model, effort and executable. Revalidation rebuilds the repair before dispatch. The UI freezes the section's declared source selection and discloses that old source/section text appear in the prompt as context only. Successful output uses the existing section canonicalizer and unaccepted-draft flow, preserving unaffected identities/assets according to that canonicalizer. Additional secret-like scanning and early size bounds now cover old section source. These are static source changes, without generation or acceptance evidence.

Interruption records now retain provider, purpose, selected paths and exact section revision identity without source bytes or consent. A recovered Codex file/overview/section operation returns to Codex with the recorded source scope and a fresh review; section recovery reloads the exact accepted base. A different route or newer recovery selection abandons pending lookups. Legacy records without scope, and range/PR records without their full original evidence targets, do not reopen as a whole-file request. They require deliberate reselection of the original evidence. This is scope-safe source recovery, not persistent runtime thread resume or qualified interruption acceptance.

The source also connects **exact Code Lens selections**. The backend chooser keeps Codex available for range requests without an API prerequisite. The existing Codex dialog shows a fixed path/line/optional UTF-8-byte selection, native model/effort controls and a fresh exact review/consent. A shared API/agent range builder performs snapshot, full-file hash, UTF-8 bounds, 32 KiB excerpt and secret-like checks before transport encoding. The Codex review binds the exact target, prompt, schema and effective controls, and revalidation rebuilds that same range. One native execution helper owns the chapter/range lifecycle and refuses cross-operation review IDs. Only the excerpt is staged; its staged-content hash is distinct from the captured-file hash in the review. Returned range claims pass the existing independent exact/unique-quote and identifier validator, open their exact citations, and remain transient. No chapter draft or accepted history is written. Early cancel, stale dialogs and backend switching discard late range responses. Runtime qualification and packaged range acceptance remain pending; interruption recovery still requires deliberate reselection of the original range.

Remaining work, with every acceptance criterion still open:

- [ ] Pin and package a supported runtime/version, record its license/distribution inventory and confirm the exact native-login route is eligible for this app; separately decide whether a direct SIWC client is needed for distribution.
- [ ] Qualify account availability and model selection independently of the runtime catalog; qualify the native account/runtime review binding now present in source, including expiry, refresh, logout, same-metadata workspaces, executable changes and restart behavior.
- [ ] Exercise actual filesystem, command, write, `.git`, instruction/plugin/MCP and tool-egress restrictions; account/provider control traffic must remain possible without granting arbitrary agent egress. Add an OS boundary or keep the route unqualified if runtime controls cannot satisfy the contract.
- [ ] Record real file/overview generation, structured-output rejection, local citation validation, draft save/accept, quota/error recovery, cancellation/crash/restart and process cleanup on the packaged app.
- [ ] Implement or explicitly scope persistent resume, qualify exact range/Git-scope recovery and every implemented chapter/section/range/PR route. Current ephemeral runs and source wiring do not satisfy native lifecycle/workflow acceptance.

No tests, builds, login, app-server launch, model request or runtime probes ran for this implementation. The selectable source UI does not establish supported-backend acceptance.

## Repository planning reviews — source progress, 2026-10-16

Codex now has a registered native `build_codex_repository_plan_review` command and typed client using the shared [planning contract](../REPOSITORY_PLANNING.md). Native installation status supplies its executable path; requested model/effort and fixed `never` approval stay explicit. The distinct `repositoryPlan` review binds the exact question, captured source, inventory/index/coverage identity, prompt/schema and settings. A native rebuild helper compares all contract fields while retaining the original ID/expiry; a shared result bridge checks review/fingerprint/model identity and validates plan evidence.

A registered native `send_codex_repository_plan_request` now reuses shared consent/scope/source preflight and journal admission, then keeps native operation/process ownership through pure result validation. Its original review/fingerprint/executable and admission clock bind the result access; cancellation, timeout and async owner drop can suppress late output. It returns transient unaccepted plan data and keeps the exact recovery notice. The app planning flow is now connected in source; accepted plan/history workflow and complete planning recovery remain pending; runtime acceptance remains missing and the planning matrix cell stays Missing. Direct API commands now check backend/operation before removing a foreign review or loading credentials. This does not add an API prerequisite or qualify Codex runtime/model/account behavior. No tests, builds, launches, runtime probes, logins or model requests ran.

## Contract
Add an Agent runtime category alongside Direct API. Connect/disconnect with the documented browser OAuth flow, app attribution and native token storage/refresh. Read account-entitled models; combine with pinned runtime capability metadata for supported reasoning values. Show effective model, effort, approval policy and immutable source manifest before a run. Launch one managed app-server child, with app-owned configuration and no additional listener/Node daemon. Request the existing chapter output schema and route successful output through local source/citation validation to an unaccepted draft.

Use restricted read-only staging, explicitly disable command execution and web/unrelated tools, and do not load repository instructions/plugins. Read-only access alone is not a no-command policy. Begin with bounded reviewed evidence passed directly; expand investigation with app-owned read/search tools. Implement progress, turn/interrupt, exit/reap, refresh/restart/resume and late-result guards. No automatic direct API fallback.

## Acceptance criteria
- Eligible account login and one real chapter run require no API key; plan limits/eligibility and account route are clear.
- Account-entitled model discovery controls choices; unsupported effort/approval values cannot be silently ignored.
- OAuth secrets stay outside WebView, repository files, exports and logs; expiry/logout are recoverable.
- Exact selected context is staged; supported runtime tool controls and sandbox enforce the declared boundary.
- Structured output is independently validated; invalid claims cannot become accepted chapters.
- Progress/cancel/quota/refresh/crash states preserve prior history and no orphan process remains.
- Update matrix from candidate to supported only with pinned-version packaged evidence when testing is authorized.

## Excludes
ACP wrappers without need, token scraping, paid fallback, speech synthesis through this account route and an extra Node server.

## Done evidence
Documented auth integration, versioned adapter/configuration, complete source-grounded chapter workflow and later account/cancel/permission acceptance.

## References
- https://developers.openai.com/siwc/quickstart
- https://developers.openai.com/siwc/token-sharing-open-source/sign-in
- https://developers.openai.com/siwc/token-sharing-open-source/codex-app-server
- https://developers.openai.com/siwc/token-sharing-open-source/models-and-inference
- https://developers.openai.com/siwc/token-sharing-open-source/preview-limitations
- https://developers.openai.com/codex/app-server

## Source scope correction — no implicit truncation

Codex chapter initialization and context reset now retain the complete supplied source selection instead of silently taking its first 12 paths. A selection above the supported limit remains visible and cannot prepare a review until supporting files are deliberately removed. A fixed section-repair scope cannot be truncated or edited to bypass preservation checks; an oversized declared scope requires correction at its owner. Matching silent truncation was removed from manual handoff initialization and direct API file-chapter initialization. Native source/byte/path limits and independent validation remain authoritative. This is a source correction, not accepted workflow evidence; no tests, builds or runtime requests ran.

## Git comparison and hunk source dispatch

The PR Guide screen now defaults to Codex, keeps Claude Agent visibly pending and exposes direct APIs as an optional choice. Agent selection never requires API settings or credentials. Selecting changed files then **Review with Codex**, or **Explain hunk**, opens the existing Codex setup/model/reasoning/review flow. Approval stays fixed to `never`. These are source changes, not an exercised native workflow.

- `build_codex_pr_guide_review` and `build_codex_pr_change_review` use transport-independent immutable Git context. API admission/encoding remain in the separate API builders; an agent does not need an API request to obtain a prompt.
- Reviews bind the comparison ID, refs/resolved commits, exact path pairs or hunk, patch bytes/hashes, schema, model, effort, executable and staged patch mapping. Native send rebuilds and compares the reviewed contract before consumption. Source remains bounded by 12 changes/250,000 bytes and encoded requests by 512 KB.
- `send_codex_pr_guide_request` and `send_codex_pr_change_request` use the common managed Codex lifecycle and operation guards. Temporary patch names are neutral aliases; original Git path/side/hunk identities remain authoritative for citations. Rename labels cannot become staged filesystem paths.
- Valid guides enter independent unaccepted PR Guide history through the existing citation validator. Done returns to claim review/accept/reject. Hunk claims use the existing exact base/head quote validator and open their selected hunk; no chapter/guide history is written.
- Progress, cancellation and late-result handling reuse the Codex dialog. A late cancellation or lost reply may follow a saved guide; the UI reloads PR Guide history and tells the user to inspect it before regenerating. A worker panic preserves recovery state and shuts down the manager rather than clearing the notice.

Remaining: automatic repository narrative planning, permitted pinned runtime/account/distribution qualification, effective boundary checks, persistent resume or its explicit scope, exact Git/range interruption recovery, packaged acceptance and all deferred cases in `xymb`. The journal still requires deliberate comparison/hunk reselection. No tests, builds, launches, logins, runtime probes or model calls ran.


## Source progress — exact range and Git interruption recovery

New interruption records retain the exact Code Lens target (snapshot path, full-file hash, inclusive lines and optional UTF-8 byte bounds), or original Git refs/comparison ID, ordered selected base/head path pairs, optional original hunk index and ordered patch SHA-256 values. They contain no patches/source, credentials, effective settings or reusable consent. Optional fields keep older records readable; records without exact targets require reselection.

A registered local-only `load_interrupted_generation_scope` command loads the record natively by ID, binds repository ownership through the current native project session, and revalidates evidence on the blocking pool. It reuses the transport-independent range/PR builders and their size, UTF-8, secret-like and scope rules without API/model admission. Requested refs must still resolve to the original comparison, and source/patch hashes must match. A second journal lookup rejects discard/replacement during blocking reads. The command creates no pending provider request and starts no model/agent; Git recovery may run the existing local read-only Git comparison.

The app restores newer Codex ranges directly into the fixed range dialog. Guide/hunk recovery opens Changes on the original comparison and presents an explicit original-scope recovery action. Guide recovery first requires the user to check saved PR Guide history; an already persisted draft must be considered before another generation. Recovery keeps Codex selected, restores the original change set or hunk, and requires current supported settings, fresh exact review and fresh consent. Navigation/new lookup guards reject abandoned completion; Git comparison IDs are no longer used as snapshot IDs.

Building a Codex review no longer clears its old interruption notice. Failed/cancelled generation and abandoned review preserve it. Successful result adoption clears it; a saved PR Guide received after late cancellation is still shown as an unaccepted saved draft and can clear the resolved original notice. Accepted history remains untouched. Existing notices/worker cleanup do not become thread resume. All runtime/package/account/isolation/native acceptance criteria remain open; no test, build, app launch, CLI/SDK probe, login or model call ran.

PR Guide history now shares a latest-request sequence between initial/event loads and manual refresh, keeps loading state during refresh, and drops abandoned replies. This supports the recovery history check; native race acceptance remains deferred. A currently open transient API hunk panel blocks opening the recovery dialog rather than stacking active explanation flows.


## Shared chapter and Git recovery source checkpoint

Git recovery now has a shared backend-tagged type so Claude notices do not route through Codex. Codex chapter/overview/section recovery also uses native pure context revalidation and a local pending/saved-result inspection panel before another review. The accepted section base is looked up with project-wide history, fixing an omitted snapshot argument in the prior source call. Candidate preview matches are explicitly not proof of originating agent/request. No tests, builds, launches, login or model requests ran; all native acceptance remains pending in `xymb`.

## Planning interruption scope — source progress, 2026-10-16

The shared native journal can retain either agent's distinct repository plan, including the original learning question, selected paths and inventory/context/contract/capture/coverage/index identity. Local recovery rebuilds and compares the entire scope without restoring a runtime, settings or consent. Missing/mixed planning metadata is rejected; old non-planning records remain compatible. The app labels a planning notice and shows its question, then preserves it without opening a chapter, overview, Git or API request because the planning dialog is still absent. Native planning senders now record this exact scope before worker dispatch and retain the notice on success/failure; app recovery and explicit save/discard resolution are still pending. This groundwork cannot close automatic planning, durable plan storage or recovery; both planning cells stay Missing. See the [planning contract](../REPOSITORY_PLANNING.md) and deferred cases in `xymb`. No tests, builds, launches, probes or model calls ran.

## Local agent plan drafts — earlier source checkpoint, 2026-10-16

Both native Codex and qualification-gated Claude planning senders now publish an unaccepted local draft inside their existing operation publication guard. The native store retains the strict proposal and canonical plan, exact question/scope/selection, IDs, review/model metadata and bounded vendor-specific settings; it stores no executable/account/connection metadata, assembled prompt, full source files, API key or reusable consent. Local load independently rebuilds the original context and replays the proposal, compares every semantic field and preserves saved step/claim/citation IDs. Metadata listing is not source validation. Registered list/load/discard commands and typed helpers are present; the newer app checkpoint below adds a planning caller and saved-draft reader; adoption/accepted history remain pending.

Publication uses bounded private staging and a no-overwrite link, retains the interruption notice on all outcomes and reports uncertain post-publication durability without claiming the plan is absent. There are no chapter/accepted-history/reading-progress/coverage mutations. Damaged regular records remain discoverable/discardable; unexpected nonregular entries fail closed. Explicit notice resolution, accepted plan revisions, larger-repository orchestration and native workflow/filesystem/cancel/crash acceptance remain pending. Claude's registry remains empty, and both agent planning cells remain Missing. See [the exact storage contract](../REPOSITORY_PLANNING.md). Source inspection and Rust formatting only; no tests, builds, launches, agent/SDK probes, logins or model requests ran. No acceptance criterion is closed.

## Agent planning app flow — source progress, 2026-10-16

Guide now has a Learning plans form: original question, 1–12 selected captured files within 250,000 source bytes, explicit agent choice and fixed source/question inside that agent's review. Codex reuses its existing account/catalog, model/effort/approval, exact request/consent, progress/cancel and owner lifecycle. Claude reuses its existing connection/catalog, supported settings and native-gated generation panel with a distinct planning sender. There is no direct API prerequisite, manual exchange or silent backend fallback. The Claude registry remains empty, so its generation is still unavailable; displaying the flow does not satisfy automatic Claude acceptance.

Saved drafts are discoverable in the app; opening one calls native replay before showing rationales/citations. The reader has Back/Next, step selection, prerequisite numbers, exact original-snapshot source links, inference bases, partial source/index/coverage limits and omitted/unknown files. Explain this file opens the existing agent-first chapter chooser for the current snapshot using the primary file and cited support. Earlier-snapshot plans require opening their original source first. No autoplay, automatic chapter generation, chapter acceptance, plan acceptance or read/coverage progress is inferred. Damaged-record discard is explicit, and every already published plan survives dialog close/backend change/late cancellation/lost reply.

Native journal recovery now restores the original planning question, files, snapshot and backend into a fresh-review chooser after unchanged notice/scope validation. It requires an explicit earlier-result check, restores no settings/connection/consent, retains notices and never opens an overview/chapter/API route. Saved-plan and recovery lists refresh after completed/failed sends, including abandoned replies. Linked notice resolution and immutable accepted plan revisions remain pending. Static call reading order remains separately accessible and preserves its journey return-focus route.

Codex bounded planning is now **Source** under the operation matrix definition (app dispatch plus local validation, unexecuted); Claude planning remains **Missing** behind runtime qualification. Complete account/runtime/boundary/packaging and native app acceptance, accepted plan revisions, exact notice resolution, repository-scale orchestration/selective updates and narrative/comprehension quality are still required. No tests, builds, app launches, runtime/SDK probes, login or model requests ran. Source review and Git whitespace checks only; no acceptance criterion is closed. Deferred app cases are in `xymb`, and the [planning contract](../REPOSITORY_PLANNING.md) describes exact behavior.

## Explicit local agent plan decisions — source progress, 2026-10-16

This checkpoint supersedes earlier statements that accepted plan history and linked notice-resolution commands/app callers were absent. Both agent planning paths now share the same explicit local decision flow: open a source-replayed draft, review its exact steps/evidence/unknowns/coverage, choose a new plan or a same-question current accepted parent, review that parent, acknowledge the decision and append an immutable revision. Native acceptance checks the reviewed canonical record digest, replays the draft and chosen parent's evidence, and rejects a stale head. Original drafts and all earlier revisions remain intact. Accepted records embed the complete original validated proposal and identities, so deleting a draft does not delete its accepted copy. Source replay remains required when reopening history; metadata is not evidence or account proof.

Native history uses separate bounded app-data storage, strict family/parent/identity/digest checks, a nonblocking persistent Unix advisory writer lock and a deterministic no-overwrite revision destination. A repeated identical decision returns its original revision after replay, including after draft deletion; a different decision fails. Lost or uncertain replies require explicit history refresh, without automatic regeneration/retry. A separate action resolves only the reviewed record's own native journal entry after full question/source/backend scope matching and unchanged-entry comparison. It cannot accept a plan, clear an unrelated/older retry operation, restore consent or mark files explained/read. Journal comparison uses the existing in-process mutex and is not a cross-process transaction guarantee.

These additions are source-only local decisions. They do not implement selective regeneration, semantic plan diffs, identity retention across separate generations, whole-repository orchestration or chapter/read/coverage completion. Codex bounded planning remains Source with actual native acceptance pending. Claude's qualification registry is still empty, generation remains unavailable and every automatic Claude cell remains Missing. The user requires automatic Codex and Claude Code-powered Claude generation without mandatory API keys; setup, manual exchange, disabled choices and API results cannot satisfy that requirement. Antigravity and remaining operation/runtime/account/packaging/narrative/visual gates stay in scope. No test, build, app launch, CLI/SDK/agent probe, login or model request ran; Rust formatting/source inspection and Git whitespace checks do not close acceptance. See [the exact decision/storage contract](../REPOSITORY_PLANNING.md) and the pending cases in `xymb`.

Exercise the actual Codex account/app-server route for all seven required operation cells, including plan acceptance/recovery, supported settings, isolation and owner cancellation. Source wiring is insufficient for release.

## Both-agent result ownership — source correction, 2026-10-16

Static review found that ordinary Codex senders discarded the original admission clock and returned raw output after releasing manager ownership, before local validation/publication. They now keep an async cancellation owner and the original queue/setup/generation/validation clock through the managed result callback. Full active review rebuilding compares every source/Git/range/section/settings/contract field after retaining the original review ID/expiry. Only that same admitted run may finish after ordinary review expiry; hunk validation now uses active result access rather than a fresh admission check. Pure validation creates only a private in-memory candidate; final publication is serialized with cancellation. No raw response escapes the owned worker for later preparation.

Claude chapter and PR Guide validation now also prepare candidates outside its revocation/publication lock. Both agents publish through nonblocking local stores. Agent chapter publication persists its recovery copy before registering/evicting any in-memory preview; failed/busy publication leaves old previews intact. PR Guides retain their separate immutable unaccepted store. Provider/validation/publication failures, cancellation, timeout and rejected late results retain exact recovery notices. Claude final wanted-result admission now precedes successful notice cleanup. Both app panels refresh recovery for non-planning operations, including range/hunk errors and abandoned replies; the shell fences superseded reads/startup and refreshes after local notice removal.

These are uncompiled, unexecuted source corrections, not proof of working automatic agents. An admitted publication can already exist after a late cancellation, lost reply or uncertain filesystem synchronization; inspect pending previews/guide history before another model request. IPC delivery/adoption, actual cancellation races, storage/lock behavior, native account/runtime/boundary/packaging and the complete operation matrix remain pending. Codex remains Source; every automatic Claude cell remains Missing and its release registry remains empty. No tests, builds, app launches, agent/SDK probes, logins or model requests ran; source inspection, changed-region Rust formatting and Git whitespace checks close no gate. See [the exact lifecycle and remaining delivery limits](../AGENT_RESULT_LIFECYCLE.md) and deferred `xymb` cases.


### Accepted reading step → both-agent chapter — source checkpoint, 2026-10-16

The app now offers **Explain this step with an agent** on a replayed accepted plan revision. Native preparation derives the exact accepted revision/digest/step, original plan and step questions, learning prerequisite IDs/questions, original snapshot and fixed distinct cited source. Both Codex and Claude have separate strict native review inputs/registered commands and typed app callers, then reuse their existing file-chapter send/validation/publication lifecycle. The agent-only chooser requires no direct API credential and has no API/manual fallback. Supported model/reasoning settings remain agent-specific; Codex approval stays fixed to `never`, Claude to `dontAsk` with bounded turns. No new agent loop or interactive tool-approval mode is claimed.

The complete planned-step scope enters the encoded review/fingerprint before consent. Native preflight and active result rebuilding replay accepted history rather than trusting renderer prose. New interruption notices retain the exact scope; native recovery requires the same accepted record/question/source, restores the original agent and requires a fresh review/consent. Missing/changed plan or evidence never widens into a general file request. Unaccepted plans retain a separately labeled general-file action. This creates one unaccepted chapter, not all steps, reading progress or accepted coverage. See [planned agent chapters](../PLANNED_AGENT_CHAPTERS.md).

**Remaining acceptance/work:**
- [ ] Exercise accepted-step generation, settings, progress, cancellation, late/abandoned replies and recovery through **each** permitted native Codex and Claude account/runtime route. Complete the matching deferred cases in `xymb`; source code or an unavailable option cannot satisfy this.
- [ ] Qualify the version-4 durable chapter-to-plan-revision/step/request records now present in source. Legacy records remain project/snapshot/file candidates. Recorded lineage is not account attestation, answer quality or step completion; do not automatically attach older same-file chapters.
- [ ] Review actual answer quality across representative languages: grounded concrete code behavior, prerequisite recaps, explicit unknowns and useful visuals. Schema/citation validity does not establish that the learning question was answered.
- [ ] Complete deliberate multi-unit orchestration, omission/failure tracking and dependency-based selective updates before claiming an entire repository narrative.

No tests, builds, app launches, native agent/SDK probes, login or model requests ran. Changed Rust regions were formatted; this does not establish compilation. Codex remains unaccepted source. The Claude qualification registry remains empty; all automatic Claude cells remain Missing. Official account guidance was rechecked October 16 and still does not resolve this product's subscription integration/distribution route. This task and the multi-agent release gate remain open.


### Original agent generation and exact step links — source checkpoint, 2026-10-16

Both native Codex/Claude chapter result closures now attach schema-4 `generationOrigin` metadata after independent source/claim validation and before guarded preview publication. It retains the original request/fingerprint/contract, agent-specific model/reasoning/fixed approval/turn settings, distinct source digests, optional section target and complete optional accepted-plan step scope. API/manual import paths do not call this creator. Previews and immutable chapter history persist the record together; no sidecar transaction or agent process was added. Original-content hashing uses bounded streaming serialization and excludes root decision metadata.

The app exposes Original generation in native agent previews, recovered previews and chapter review. The accepted plan reader lists saved chapters for the exact accepted revision/digest/step, with explicit lifecycle/snapshot labels and separate read/review actions. Recovery distinguishes recorded request/step matches from older filename candidates. These are local recorded labels, not account attestation, complete-repository coverage, proof that a question was answered or step/read completion.

Strict legacy v1–v3 chapter reads and version-3 overview/pending-preview recovery remain in source. New writes/review decisions use version 4 and preserve older immutable bytes. Missing origin is omitted; present null/invalid origin and future versions fail. Manual edits ignore renderer-supplied origin replacement and preserve native lineage. Restore retains the historical source origin; edited/remapped content can differ. New section generation clears inherited origin and receives its own record only from a native agent result, preserving earlier revisions/links. See [the record, compatibility and evolution contract](../CHAPTER_GENERATION_RECORDS.md).

**Pending:**
- [ ] Qualify schema-4/legacy decoding, preview restart/save and immutable decision/restore behavior; prior version-3 evidence is insufficient. The schema task `8bw6` requires reopening/revalidation for this change.
- [ ] Complete actual native Codex and Claude planned/ordinary chapter and section flows with matching settings/records, cancellation ownership, interrupted-save recovery and correct step links. No API prerequisite or fallback is allowed.
- [ ] Verify wrong request/backend/step, malformed/duplicate/oversized metadata, digest mismatch, forged renderer replacement, missing history, two same-file learning questions and cross-snapshot derived lineage. Use the detailed deferred cases in `xymb`.
- [ ] Review pacing, source-grounded answer quality, navigability and comprehension across representative languages. Recorded metadata/citation validity cannot satisfy this. Multi-unit orchestration and selective updates remain separate pending work.

No tests, builds, app launches, agent/SDK probes, login or model requests ran. Existing fixture constructors only received the required absent-metadata field; no tests were added or executed. Changed Rust regions were formatted and Git whitespace checks are separate from compilation/runtime acceptance. Codex remains Source, every automatic Claude cell remains Missing behind the empty registry, and the multi-agent release requirement remains open.

### Codex account/runtime review binding — source checkpoint, 2026-10-16

Every registered Codex review now derives a strict connection UUID, executable digest, account-metadata digest and event revision from an already-running native owner. These fields enter the exact request/settings/fingerprint. Preflight reconstructs with a fresh observation; dispatch refuses a lost/replaced connection and compares observations before staging and after generation. Account events stop an in-flight turn/result, and active result rebuilding requires the manager-owned identity. The app explains that changes require a fresh review. No automatic API/backend/account switch or consent-preserving runtime restart was added.

This is source change detection, not immutable account/workspace identity, entitlement, trusted distribution or execution attestation. Missing account email is unsupported for review; same-metadata workspaces, outside credential writers, interpreter/dependency changes and path-to-exec races remain qualification limits. See [Codex review identity](../CODEX_REVIEW_IDENTITY.md). All native both-agent account/runtime, operation, lifecycle and package acceptance remains open; Claude's release registry remains empty. No test, build, app launch, runtime probe, login or model request ran. Formatting and Git whitespace checks close no gate.
