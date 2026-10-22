---
kind: task
key: codex-retrieval-tools
parent: large-source-discovery
title: Give the existing Codex harness bounded repository retrieval tools
priority: 1
labels: agents, codex, retrieval
---

## User outcome

Ask once. Codex finds implementation, follows necessary dependencies and returns a cited explanation through the saved connection. No API key, manual source checklist or repeated provider selection.

## Dependency and decision

Depends on [local range retrieval](69-local-range-retrieval.md). Prefer an app-owned private local MCP service over experimental dynamic tools; qualify support in the existing installed Codex runtime before changing runtime permissions. See the [industry review](../research/2026-10-17-repository-context-and-indexing.md). Existing unrelated MCP servers, shell access and repository code execution stay disabled. Claude and other agent adapters remain later work using the same context contract.

## Required contract

- Replace the fixed metadata-selector/whole-file handoff with bounded iterative retrieval for eligible workspace questions. Expose only owned search, range-read, definition/reference and module-map tools backed by the native immutable context service.
- Use the existing installed Codex account with GPT-6 Luna/max. Verify actual protocol/tool capability and model availability; no new CLI install, API prerequisite or model/provider fallback. Preserve one managed active runtime and existing cancellation ownership.
- Keep the snapshot/account/source-scope grant one-time and revocable. Define a versioned range/tool admission policy. Explain a materially widened scope once before use; do not reinterpret an old request fingerprint as permission for arbitrary reads or show a consent wizard for each tool.
- Enforce path/hash, exclusions, per-result bounds, total transmitted bytes, call count, deadline, runtime/account identity and revocation natively before every tool result. Continuation cannot read the live checkout, use arbitrary inherited MCP servers, invoke commands or expand its own grant.
- Admit only the explicitly enabled tool event types. Validate tool arguments and returned evidence before delivery. Repository text is data; it cannot configure tools or become system instructions. Reject unknown tools, forged ranges, cross-project paths and stale evidence.
- Preserve question/parent intent, retrieved evidence and unresolved boundaries across retrieval steps. Stop when enough evidence is available or the bounded budget is exhausted. Prefer local exact lookup for explicit symbols; avoid a separate mandatory LLM file-selection call for trivial cases.
- Persist an evidence ledger with result identities, selected ranges, reasons, omissions and request settings. Keep sources and model reasoning out of default diagnostics. The complete ledger and exact admitted code must support independent claim validation and recovery after restart.
- Keep strict source/claim/visual validation and unaccepted draft publication. A useful cited partial answer can describe missing evidence; unsupported behaviour cannot be filled in from docs. No silent inference retry or automatic acceptance.

## Acceptance

1. Native Fontdone broad font-to-bitmap question reaches relevant core `src/font.rs` ranges plus necessary callers/dependencies without manual attachments. Independently review the answer and flow; record omitted boundaries.
2. An attached `Font::memory_face` question succeeds through range retrieval. A focused Outline predicate answer begins with the requested predicate and its two branches, using the same saved connection.
3. Record actual tool calls, immutable result identities, bytes, model/reasoning and stage timings. Compare source recall and first validated answer against the old selector under the same fixture and settings. Merely observing successful MCP initialization is insufficient.
4. Exercise budget exhaustion, malformed tool output, changed hash/account, revoked grant, unknown tool, lost reply, Stop during search/read/inference/validation/publication, queued successor recovery and app restart. No late adoption or automatic resubmission.
5. Qualify packaged app invocation of the existing Codex binary, no added server port, no repeated connection picker and exact source-link navigation. Keep the parent's quality and whole-guide gates open until their complete matrices pass.

## Evidence

Workspace owned-tool retrieval is implemented in source, with deterministic tests, a successful small synthetic investigation, and the separately checked focused Fontdone predicate below. Broad Fontdone generation and packaged parent acceptance remain open. Earlier dated checkpoints describe the preceding fixed-evidence adapter; the current admission contract enables only the five owned tools.

## Retrieval foundation checkpoint — 17 October

Native search/read/cancel and structural definition/relationship/map commands now exist; see the [measured limitations](../acceptance/2026-10-17-range-retrieval.md). Broad single-query lexical retrieval found only 2/12 expected declarations. Named global retrieval missed FFI/C adapter declarations that file-scoped search recovered. The owned agent must inspect the module map, vary queries/scopes and follow validated relationships without asking the reader to select references. This is not implemented or enabled in Codex yet. `a699` remains the open prerequisite; retain the existing account/model constraints and qualify the complete evidence ledger before transmission.

## Expanded industry review — 17 October

Reuse Codex's existing agent loop and standard MCP integration. The app adapter currently supplies evidence and forbids tools; having an index on disk does not give that agent access to it. The implementation must demonstrate actual search/read/refinement calls and a grounded answer in the existing account-backed runtime. MCP initialization, a valid output schema or a filename-selector result does not satisfy that outcome.

Treat external context products as adapter candidates: Serena offers reusable LSP-backed navigation; Sourcebot offers a separately deployed multi-repo context service. Neither is a default dependency for this task. A candidate must fit the same snapshot, bounded read-only, cancellation and evidence-validation contract. Its advertised support for Codex is not proof that our integration works. Keep Claude deferred and avoid adding another agent framework or requiring a separate API connection.

## Structural retrieval checkpoint — 17 October

The [native structural probe](../acceptance/2026-10-17-structural-context.md) finds the eight syntax-backed expected declarations. Four FFI/C adapter expectations remain parser-unavailable text, with explicit `fileTooLarge` scoped status. The first page of 32 outgoing `memory_face` links contains 32 unresolved candidates and zero resolved static candidates. A module inventory and a parser graph cannot substitute for investigation of the implementation. The agent must use text fallback and follow explicit evidence; consider the reviewed LSP/SCIP reuse options before extending custom resolution. These native commands are not enabled as Codex tools. Aggregate evidence admission and its versioned account/snapshot grant remain required.

## Owned agent investigation checkpoint — 17 October

Implemented in source: the workspace composer invokes one `generate_workspace_answer` operation. The existing Codex app-server chooses relevance through five standard owned MCP tools; no mandatory AI file-selector or whole-file prompt precedes the question. `rmcp` supplies standard stdio transport over a private operation socket. A versioned snapshot/account/runtime grant, durable range/call ledger, overlap deduplication and final delivered-range validation bind the result. Legacy reviewed dialogs retain their original evidence contracts.

[Executed evidence](../acceptance/2026-10-17-codex-owned-retrieval.md): 260 native tests, 136 frontend tests and the frontend build pass. A real GPT-6 Luna/max synthetic investigation passed through the installed account: five calls, 197 distinct code bytes, 4,623 metadata bytes, 4,829 native payload bytes, one validated answer with a v3 receipt. It took 173.83 seconds including fixture/connection work; this does not meet a fast-answer claim. Its first attempt exposed a correctable invalid range; the new bounded, accounted error response sends no code and lets the agent correct its own request.

Remain **open**: Fontdone broad/focused/Outline answers and independent narrative/flow review; complete negative-budget, identity, mid-stage Stop/restart/successor matrix; packaged helper/source navigation/permission acceptance; ledger omissions/reasons and useful partial answers at limits. The successful synthetic is not a whole-repository or package gate. Earlier checkpoints above describe the state before this source implementation.

Fontdone runtime checks exposed two additional adapter issues: an excessive module-page request and ordering between the MCP stream and app-server started notifications. Page sizes now cap at 32; private requests wait up to three seconds for exact event ownership, with no data exposed before matching. Deterministic regressions pass. Both failed public-source attempts are preserved in the acceptance report; a further deliberate run remains pending. No Fontdone answer/package gate is closed.

## Runtime-check diagnosis — 17 October

The third broad Fontdone investigation passed five discovery calls and then stopped during private transport setup, before reading source. Native generation took 209.876 seconds; the complete test took 238.07 seconds. All three failed attempts remain preserved in the acceptance report.

A local probe identified expensive repeated hashing of the 246 MB installed runtime. The existing RustCrypto SHA implementation now uses its accelerated backend and an optimized dependency in development/test builds. Four actual native checks took 122–128 ms and produced identical hashes; known-vector, cancellation and no-follow contracts remain covered. The suite passes 262 native tests and 141 frontend tests. Transport errors now retain stage/kind/OS code without OS text, private paths or source. See the [measured scope](../acceptance/2026-10-17-codex-owned-retrieval.md#local-runtime-check-cost).

The timeout cause is not yet established by this isolated measurement. Release helper/package startup and another full Fontdone run remain required. No question was automatically retried and no model/reasoning settings were lowered.

## Packet transport checkpoint — 17 October

A macOS socket probe confirmed that accepted streams inherited nonblocking mode: setting a socket timeout still returned `WouldBlock` immediately. The native owner now explicitly uses blocking streams and one absolute deadline for each complete packet, with short cancellable waits. All owned-session tests fragment their header/body; further regressions cover slow input, Stop during blocked input/output, and output deadline expiry. The full native suite passes 265 tests with ten explicit probes ignored.

The unsigned release app/DMG passed bundle integrity checks before this last transport change. A new native Fontdone broad probe is being evaluated with the unchanged release helper protocol. Packaged parent acceptance, independent explanation/flow review and all remaining matrices above stay open.

## Broad investigation after transport fixes — 17 October

The fourth broad run delivered 52 calls and read 47,986 distinct source bytes in 21 ranges from `src/api.rs`, `src/font.rs` and `src/tables.rs`. It stopped on the 53rd call with a code-search matcher error, after 125.603 seconds of native generation (127.82 seconds for the complete test), without an answer. Its separate [failure artifact](../acceptance/artifacts/2026-10-17-fontdone-agent-broad-search-terms.json) remains preserved. A deterministic owned-session query containing an opening parenthesis reproduced the same error before fixing literal matcher configuration. The failing live query itself is not recoverable from the ledger's argument digest.

Before a further broad qualification, expose remaining call/source/metadata/wire budgets to the agent and qualify bounded query feedback and useful partial answers at limits. The observed discovery used 239,661 of 256,000 metadata bytes while reading only three implementation files. Do not raise whole-file or operation caps to hide this gap. Complete the focused predicate and attached-function cases separately; keep the broad narrative/flow and package gates open.

The separate focused `outline` request passed in 53.087 seconds of native generation (55.46 seconds for the whole test): four calls, 948 distinct source bytes, one section, two validated anchors and a source-linked OR diagram. Agent source inspection confirms direct-first predicate, disagreeing fields and false case. [Exact draft](../acceptance/artifacts/2026-10-17-fontdone-agent-outline.json). Its spoken script omits the actual predicate, so speech comprehension still fails. The attached `memory-face`, broad explanation, budget/identity/Stop matrices, human quality review and packaged UI remain open; this focused success closes none of those gates.

The current unsigned app/DMG subsequently rebuilt with the packet and literal-search fixes, passed bundle/schema/checksum verification, and passed a headless packaged-helper initialization/catalog probe. Exactly five read-only tools were listed; a missing owner returned an error without evidence. [Package probe](../acceptance/artifacts/2026-10-17-packaged-context-helper.json). The Mac remained locked, so the installed app was not replaced. Packaged parent generation and user-visible navigation/permission/playback acceptance remain open.

## Budget feedback and investigation recovery — 17 October

- [x] Expose conservative remaining call/file/source/metadata/payload/fragment/time capacity after each tool response, reserving room for limit feedback inside the original bounds.
- [x] Return a charged, source-free limit notice so Codex can finish from delivered evidence. Keep hard call/deadline boundaries, independent citation validation and uncertain-delivery rejection.
- [x] Return bounded correction metadata for invalid query text. Unsafe scopes and changed runtime/cancellation remain terminal; this does not catch arbitrary storage/index failures as query errors.
- [x] Cover actual source/file/fragment/per-result/call exhaustion, injected counter boundaries, prior-evidence validation after a limit, uncertain notice delivery, invalid query recovery and old-record compatibility in deterministic regressions. The final native suite passed 277 tests; 142 frontend tests and the production frontend build passed. Scope notes are retained on workspace turns; packaged visual acceptance and scope presentation outside that conversation remain open.
- [ ] Qualify an actual useful partial answer, its scope note across navigation/restart, and a complete broad Fontdone answer with independently reviewed code, flow and narration. The deliberate live run is recorded in the acceptance report when it terminates.
- [ ] Complete the remaining revocation, mid-stage Stop, successor/restart and packaged parent matrices; the tests above do not close these broader gates.
- [ ] Evaluate app-server thread reuse for an exploration so follow-ups can retain prior answers and investigation context. The current implementation creates ephemeral question threads with bounded earlier-question text. Bind any resumed context to the same snapshot/account/grant, invalidate it on changes, and qualify Stop/restart without inference replay. Resumable conversation support is documented by Codex; it is not implemented or qualified here merely by using app-server.

The agent still owns search/read decisions. [The desktop integration comparison](../research/2026-10-17-repository-context-and-indexing.md#how-codex-desktop-investigates-a-question) separates the reusable Codex loop from this application's stricter tool and lifetime policy. No embedding service, direct API provider, new CLI or extra application port was introduced.

The fifth broad Fontdone run delivered 58 calls and 94,809 source bytes across nine implementation paths, including the core parsing/loading/hinting/scaling/rendering code. It returned output, then failed identifier admission for `resolve_face_index`; no preview was saved. Native generation took 529.629 seconds and the whole test 531.81 seconds. [Preserved attempt](../acceptance/artifacts/2026-10-17-fontdone-agent-broad-budget-identifier.json). No limit notice was needed. Broad usefulness and speed still fail.

- [x] Preserve a bounded rejected structured candidate and positional finding with the operation/snapshot receipt, excluding model reasoning. Qualify one correction in the same owned investigation using only admitted evidence and the original limits; preserve the initial candidate and never replay the entire investigation automatically. The synthetic existing-account proof below establishes this narrow contract, without qualifying the broad Fontdone answer.
- [ ] Add recovery/retention controls for private rejected candidates. Show the failed question and a simple recovery action through Notifications, keep raw source-containing records out of default diagnostics, and permit explicit inspection/removal without navigating app storage. Exercise the eight-operation capacity boundary in the packaged UI; preserve existing failures and require an explicit user action before new inference. The native store currently refuses a ninth failed operation and does not expose a removal UI.

## Same-conversation correction checkpoint — 17 October

The workspace adapter now permits the shared one-correction policy and freezes its owned context service before that second turn. A new private candidate journal retains exact rejected/corrected JSON plus positional findings, response digests, request fingerprint, operation binding and delivered-evidence digest. It is separate from the source-free ledger and trusted preview store. Nine new offline regressions cover aligned-answer non-retention, both-candidate retention, immutable identities/order, request/evidence mismatch, corruption, symlinks, capacity, transport/event freeze, cancellation, grant removal and changed runtime.

The complete native suite passed **286 tests** with eleven explicit probes ignored; the frontend suite passed **142 tests**. The deliberate GPT-6 Luna/max owned-correction probe then passed in **22.64 seconds** for the whole test, with **19.714 seconds** in the measured run. It used one delivered 197-byte source read, two distinct turns in the same thread, exactly one correction, and full independent source/claim validation. Both candidates are preserved in this public synthetic [artifact](../acceptance/artifacts/2026-10-17-codex-owned-correction.json). The native freeze refuses all extra retrieval events and packets during correction.

The first synthetic probe reached correction but failed independent section-content validation; its test prompt omitted the production prompt's prose/narration requirements. Its [separate failure](../acceptance/artifacts/2026-10-17-codex-owned-correction-incomplete-test-prompt.json) is preserved. The test now saves failures and candidates before temporary-fixture cleanup. No production source or validator was weakened to make the corrected synthetic case pass. The successful second probe had no concurrent build; these small synthetic timings are not a Fontdone speed benchmark.

The broad Fontdone failure remains a failed historical run. It has not been replayed to obtain this correction evidence. Full broad narrative, attached-function, useful partial, semantic/speech, lifecycle and packaged-generation matrices stay open. The updated debug bundle was installed to qualify saved-reading navigation separately; it is not a signed release and does not qualify a complete project guide for general use.


## Exact-quote correction checkpoint — 17 October

The first attached `Font::memory_face` investigation delivered 29 calls and 69,653 distinct source bytes from three paths, including the 375,700-byte `font.rs`. It completed two turns in the same conversation in 391.557 seconds, then failed exact-source admission. The model moved `is` across a comment line break in one 937-byte quote; the identifier-only correction preserved that error. Both exact candidates and positional identifier findings are [preserved](../acceptance/artifacts/2026-10-17-fontdone-agent-memory-face.json); no preview was published.

- [x] Diagnose exact claim-quote length, uniqueness and delivered-range coverage before the same single owned correction. Share matching rules with final admission; send fixed problem codes/JSON positions without source; keep correction tools frozen and the original limits/owner. Deterministic native regressions cover the observed comment reflow, source-only correction, unread/ambiguous/UTF-8 passages and cancellation. The full native suite passes 294 tests (11 explicit probes ignored).
- [ ] Qualify the changed policy with the deliberate frozen Fontdone attached-function retest, then independently review its actual implementation facts, visuals and spoken script. The first failed sample is not a quality pass or a latency comparison.

[Failure, timing boundaries and package evidence](../acceptance/2026-10-17-fontdone-quotes-and-tool-timing.md). This advances preflight recovery only; broad explanation quality, packaged generation, retention controls, persistent exploration conversations and the complete ownership/Stop/restart matrices remain open.


### Attached-function retest outcome

The [exact-quote-policy retest](../acceptance/artifacts/2026-10-17-fontdone-agent-memory-face-exact-quotes.json) also failed: 380.156 seconds, 36 delivered calls, 48,698 source bytes and no preview. The initial output removed a comment from one quote; its sole correction restored a reflowed comment. Native preflight correctly rejected it, despite empty corrected identifier findings. Native calls totalled 12.568 seconds; the remaining time is not separately attributed. Both exact candidates are preserved. No third inference attempt was started.

- [ ] Implement and qualify [native evidence references, Kata yk6c](72-agent-evidence-references.md), replacing model-copied source quotes for this agent path while retaining byte/digest/coordinate, claim and visual admission. This is a reliability prerequisite for attached-function acceptance; another wording tweak is not a completed fix.

Failed second-candidate preflight now retains bounded fixed codes and positions in private recovery; final deterministic suite: 295 passed, 11 ignored. The live failure predates that diagnostic-only addition and is preserved unchanged. All broader parent gates remain open.

## Native citation implementation checkpoint — 17 October

The native evidence-reference protocol is implemented, with operation-bound delivered IDs resolved locally into exact captured spans. New schema versions preserve legacy chapters, receipts and rejected attempts. The full native suite passes **304 tests** (11 explicit probes ignored), frontend **145**, and production frontend build passes. The installed unsigned debug app passed executable integrity and reopened the saved nested Guide at section 2/3 with the same source selection and Codex connection. [Contract, limitations and package identity](../acceptance/2026-10-17-native-citation-references.md).

This remains open: one explicit frozen Fontdone GPT-6 Luna/max investigation, actual implementation-fact/diagram review, fresh citation navigation and Pocket narration must pass separately. Previous failed runs remain failed. Full broad explanation quality, latency, recovery controls and persistent exploration conversations are separate outstanding gates.

## Native-reference live investigation — 17 October

The correctly configured native-citation Fontdone probe delivered 40 calls and 53,291 source bytes across the large core font file and two dependencies, then failed after 358.714 seconds with no preview. Both candidates' references resolved; one corrected claim still failed identifier/evidence alignment. A separate review also found a semantic required-versus-optional table error. [Complete preserved result](../acceptance/2026-10-17-native-citation-references.md). This does not pass attached-function, broad narrative, spoken or latency acceptance.

[Progressive section publication, Kata 3bz5](73-progressive-answer-sections.md) now defines owned section proposals, independent admission/checkpoints, partial outcomes and installed acceptance. It remains unimplemented. Do not discard bad claims locally, reinterpret this failed answer as partial success or retry the whole investigation automatically. The source-free trace and public failed candidates retain separate identities; existing limits, account and GPT-6 Luna/max remain in force.

## Correctable read batches and fixed rejection diagnostics — 17 October

A regression reproduced an empty or nine-range read abort before the fix. Such requests now return accounted `invalidRangeCount` feedback without any partial source, path, size or hash. Codex can split/correct the batch inside the original budgets. Unsafe paths and changed owners remain terminal. New rejected native deliveries retain only a fixed `rejectionCode`; arbitrary native text becomes `unknown`. Legacy receipts remain readable. Ledger and private MCP regressions include an exact corrected read, no-source/error accounting and rejection-code state validation. Full native suite: 351 passed, 13 explicit probes ignored; frontend: 179 passed.

The fresh broad Fontdone probe delivered all 39 calls, reading 123,317 bytes across five paths, but returned only three of the requested pipeline sections and exhausted eight proposals. [Preserved partial answer and review](../acceptance/2026-10-17-read-recovery-and-broad-answer.md). It did not trigger invalid read-count feedback or any native read rejection; it cannot retrospectively explain the earlier rejected call whose cause was not recorded. Complete broad narrative, recovery/retention, persistent conversation and packaged lifecycle matrices remain open. No new CLI/provider or port was added.

## Native packet race and short-reference checkpoint — 17 October

The intermittent progressive packet failure is now causal: `set_read_timeout` returned `InvalidInput` / OS error 22 in about 154 microseconds after a successful full peer send. A local 100-open/100-closed-peer comparison reproduced the setter failure only after closure, while every queued message remained exactly readable. The deterministic complete queued-packet regression fails before the fix. Accepted native owners now use standard nonblocking I/O with waits of at most 25 ms on `WouldBlock`, preserving absolute 200 ms input/two-second output deadlines, cancellation and all packet limits. Incomplete EOF and cancelled queued packets remain rejected; arbitrary errors are not ignored. Temporary instrumentation is removed. The first complete fixed suite passes 359 native tests, with 13 explicit probes ignored.

New progressive read responses also supply operation-local short citation references, retaining native digest identities, exact delivered-source admission and historical answer compatibility. Legacy five-tool requests retain their original catalog. [Reproduction, transport contract and qualification limits](../acceptance/2026-10-17-short-native-references.md).

The exact fresh frozen Fontdone account run, installed replacement/native navigation, quality, speech and useful-answer timing remain pending. This checkpoint neither explains every historical generic transport error nor certifies complete large-repository answers. Keep this task open.

## Short-reference live failure and observability — 17 October

A fresh frozen Fontdone run used the rebuilt `502e2c2` helper and existing GPT-6 Luna/max account. It saved one adopted section, then returned Partial in 160.010 s. Thirty-two of 33 calls delivered; the last read started but has no prepared payload and is uncertain. Native input had already been admitted before this call began. The receipt does not prove a socket, argument, budget or model cause; the worker discarded its actual error when retaining partial content. [Exact artifact and scope](../acceptance/2026-10-17-short-native-references.md#result-partial-completion-assertion-failed).

The follow-up retains a fixed private terminal failure classification from the actual error; arbitrary messages become `unknown`, original terminal records are not backfilled, and no technical field is added to the reader summary. Full default suite: 363 native passed (13 explicit probes ignored), frontend 179. No new account run has qualified this later diagnostic. Reproduce/classify the actual uncertain-call failure and finish broad repository, native navigation, quality/voice and timing gates before closure. Keep this task open.

## Fresh complete-delivery, incomplete-answer evidence — 17 October

The new version-2 scene Fontdone run delivers all 61 calls and 80,833 distinct source bytes from nine paths. All 38 published anchors match exact immutable bytes and coordinates. No uncertain read or runtime failure occurs in this one run; this cannot retrospectively explain the prior uncertain read. The answer still finishes Partial at 491.831 seconds after all eight proposals: rasterization/final bitmap publication are missing. [Preserved complete result and scoped review](../acceptance/2026-10-17-scene-v2.md). Full broad completion, budgets/latency, cancellation/installed and quality gates remain open. No task is closed.
