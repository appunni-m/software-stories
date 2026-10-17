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

Research and contracts only; no capability/runtime acceptance claim. The current adapter rejects tools intentionally, so this requires a narrow new admission contract rather than simply enabling all tool events.

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
