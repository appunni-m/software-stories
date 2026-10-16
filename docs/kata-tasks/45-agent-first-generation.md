---
kind: task
key: agent-first-generation
parent: t29
title: Make Codex a complete generation choice without mandatory API keys
priority: 0
labels: agents, product-quality, release-gate
blocked_by: codex-account-backend
---

## Current native evidence — 2026-10-16

The packaged app automatically discovered the existing desktop Codex with no saved executable selection, reused the ChatGPT account and generated a synthetic file chapter using **GPT-6 Luna / max**, with no direct API key. Exact source review and consent preceded generation. A locally validated preview was saved as r1 Draft and explicitly accepted as r2; a citation opened verified source lines and the manually paced reader advanced sections. Named permission profiles replaced unsupported protocol fields without removing source/citation validation. See [dated evidence](../acceptance/2026-10-16-codex-validation.md).

`npm run check` passed: corpus, 98 frontend tests, 185 Rust tests passed/two ignored, frontend build and Cargo check. The opt-in installed-account metadata test passed separately. Local unsigned packaging and native file flow were exercised. The synthetic permission probe allowed staged reads and denied outside reads/writes; no network probe ran.

**Still required:** native evidence for every remaining operation; account/runtime/change/cancel/quota/crash/recovery boundaries; bounded repair and actionable diagnostics after invalid chapter output; usable real-repository narratives/visuals; narration, accessibility, clean-account packaging and distribution qualification. One prior native response failed validation; the later success did not prove automatic recovery. Keep this task open. Claude is deferred in `8gqw`.

## Current delivery decision — 2026-10-16

The user explicitly deferred Claude and chose Codex for the current milestone. This task now gates a complete automatic Codex flow without a mandatory direct API key. Claude remains open in `8gqw` at priority 2 and is retained as a dependency of the full multi-backend milestone `mfz2`; it no longer blocks this Codex gate. Antigravity remains later work in `m00g`.

The user authorized building, testing and one small synthetic validation through the installed Codex account on October 16. This supersedes the earlier no-testing instruction for this work. Use GPT-6 Luna with max reasoning only if advertised and available; do not silently substitute another model or direct API route. Account login can still require the human in the system browser.

## User outcome

Choose Codex, configure its supported model/reasoning settings, review the exact captured source, generate, inspect grounded claims/visuals and save an unaccepted draft inside Code Atlas. No API key, manual prompt exchange or fallback is required. Existing accepted chapters remain intact.

## Required product flow

1. Open a captured file, repository selection, accepted learning-plan step, section repair, code selection or Git-change scope; select Codex without an API setup gate.
2. Discover the installed official Codex and reuse its supported ChatGPT account automatically. Put installation overrides in advanced settings; show one concrete recovery action for setup failures.
3. List supported models/reasoning from that runtime. Display effective settings in the request review. Approval stays fixed to `never` for the read-only explanation boundary; unsupported settings are unavailable.
4. Review immutable source, learning question and settings, then explicitly start. Show progress and cancellation. Backend/source/settings/account changes invalidate the review and consent.
5. Independently validate claims, citations and visuals; save a separate unaccepted draft, then allow deliberate acceptance. Failure/cancellation never switches backend or overwrites accepted history.
6. Recover exact interrupted scope and pending previews without widening the request or automatically generating again.

## Acceptance checklist

- [x] App production build and relevant frontend/native tests pass with dated command/results (October 16 baseline above).
- [x] A small synthetic file chapter completes through the installed Codex account without a direct API key; runtime/model/effort and redacted source/result evidence are recorded (October 16).
- [ ] Selectable model/reasoning settings reach the runtime and original-generation record; unsupported choices fail explicitly.
- [ ] File, overview, section, range, planned chapter, bounded repository plan, PR Guide and single-hunk routes have dated native evidence before their matrix cells are promoted to supported.
- [ ] Cancellation, expired login, unavailable model, quota/runtime/validation failures and interrupted-save recovery preserve source scope and accepted history.
- [ ] Runtime permission/account/packaging qualification and native usability evidence are recorded. A successful small synthetic run alone cannot close the entire gate.
- [ ] A real user can complete the Codex flow without developer intervention.

## Owning tasks and evidence

Codex: `z3an`; current UX gate: `2r6n`; validation: `xymb`; parent: `tpta`. Full backend milestone: `mfz2`, with deferred Claude `8gqw` and Antigravity `m00g` retained. The [operation matrix](../AGENT_OPERATION_PARITY.md) distinguishes source implementation from actual acceptance. Keep tasks open until their own acceptance is met.

## Historical implementation notes

The earlier joint Codex/Claude sequencing below is superseded by the current decision above. Historical no-test/source-only statements describe those checkpoints; they do not prohibit the newly authorized validation. Deferred Claude work remains unqualified behind its empty runtime registry.

### Native review ownership delta

Claude now has a native five-minute observed catalog connection and a registered offline file/overview/section review command. Requested settings/source enter from the renderer; account/runtime/capability observations come from native state only. Refresh/cancel/path change/close invalidate the connection, late catalog completion cannot republish it, and review fingerprints/expiry bind to the current record. The app now calls offline review through the Claude settings/source/request panel for file/overview/section/range/guide/hunk scopes. Native inference commands are registered behind an empty qualification registry; app generation UI has source wiring but no eligible runtime or workflow acceptance was delivered. The account/runtime/isolation qualification and every automatic Claude operation remain open, along with disconnect-aware active-run/result handling. Metadata and offline reviews still cannot satisfy this release gate. No tests, builds, agent probes or model calls ran.

### Claude operation pipeline delta

The internal candidate Claude pipeline now includes exact range, PR Guide and single-hunk operations through shared immutable context, strict per-operation contracts, one worker lifecycle and independent result validators. New registered commands are offline review only; their typed helpers now have explicit app callers for local request preview. App generation UI and disconnect-aware inference results now have source wiring. An eligible release profile, actual runtime/account/isolation/workflow qualification and scope-preserving recovery acceptance remain absent. Source-range and Git preview targets stay fixed and are checked independently against the returned review; preview does not grant consent or start generation. This does not complete any automatic Claude cell or weaken this release gate. No tests, builds or runtime/model calls ran.


### Native lifecycle delta; automatic app delivery remains open

The internal Claude pipeline now binds every operation to a revocable native owner shared with worker cancellation. Refresh/disconnect/cancel prevent subsequent publication; ordinary five-minute review expiry cannot invalidate the same already-consented ten-minute run. An async owner drop revokes worker and blocking validator access. A preview or guide whose publication already won remains unaccepted and may need recovery/history lookup after a late disconnect. Native send commands now connect this pipeline through release-owned admission, but the registry is empty and generation UI now has source wiring; this does not qualify its account/runtime boundary or establish accepted renderer cancellation/late-reply disposal. Keep the priority-0 gate and all Claude automatic cells open until the actual integrated flows, exact settings and scope-preserving recovery are delivered and accepted. No tests, builds or agent/model calls ran.


### Exact range/Git recovery source delta

Newer Codex interruption notices now restore exact immutable ranges, selected Git changes and original hunks after journal-owned native local validation. Guide recovery checks saved history first. Backend choice, fresh model controls, exact review and consent remain explicit; there is no automatic request or API fallback. Original Codex notices persist through review-only abandonment and failed/cancelled results, then clear on validated result adoption. The metadata/native validator are reusable by Claude, whose automatic sender remains gated and exact recovery UI now has source wiring but no native acceptance. This improves Codex source coverage and does not complete either agent's packaged acceptance or this multi-agent gate. Deferred evidence cases are in `xymb`.


## Both-agent exact recovery — source progress

Claude and Codex now share native journal-owned chapter/range/Git evidence recovery and a backend-tagged Git scope. Claude records reopen the original agent, exact chapter/section/range/comparison/path-pair/hunk and a fresh review. Section recovery uses project-wide accepted history. Chapter recovery lists possible pending/saved results before another request; guide recovery inspects separate PR history. No prior runtime/settings authorization or consent resumes. Preparation, failure, cancellation, closing and unavailable runtime preserve the original notice; validated wanted result adoption resolves it. API preview creation cannot resolve an agent notice.

This source has not been built or exercised. Claude's release registry is still empty, all automatic Claude cells remain Missing, and native runtime/account/boundary/recovery acceptance remains open. Deferred cases are in `xymb`.

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

The default generation chooser must keep coding agents visible independently of any saved API preference. Accept real automatic Codex and Claude workflows with supported settings, progress, cancel and recoverable result review before this product gate closes.

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

### Agent JSON and reported-turn admission — source checkpoint, 2026-10-16

Both native agent frame readers now reuse the existing chapter duplicate-key decoder, extracted into a shared module. Codex rejects non-object RPC frames and checks final JSON text strictly. Claude retains the reviewed turn limit and rejects successful results without an integer `num_turns` from 1 through that limit. Existing byte/depth limits, stream failure latching, independent source validation, clean exit, cancellation and publication ownership remain in place. Chapter storage/schema versions are unchanged.

These unbuilt/unexecuted checks neither prevent nor refund reported usage and cannot qualify runtime/account behavior. Complete the new parser/count/EOF/lifecycle cases in `xymb`, including actual CLI formatter/retry count semantics for every supported turn setting. No tests, builds, app launches, runtime probes, logins or model requests ran. Both-agent native acceptance remains open; Claude stays behind the empty qualification registry. See [the lifecycle contract](../AGENT_RESULT_LIFECYCLE.md#protocol-parsing-and-reported-limits).
