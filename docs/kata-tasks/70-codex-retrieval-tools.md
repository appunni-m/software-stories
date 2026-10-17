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
