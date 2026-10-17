---
kind: task
key: agent-evidence-references
parent: codex-retrieval-tools
title: Let agents cite native evidence references without copying source quotes
priority: 1
labels: agents, citations, reliability
---

## User outcome

A correct implementation explanation does not fail because the agent reformats a source comment. Readers still open the exact captured lines behind every claim and visual. Source access, permissions and generation limits stay under native ownership.

## Confirmed failure

Two frozen Fontdone `memory-face` investigations failed after one correction. The first reflowed `is` across a comment line break. In the second, the original draft omitted the comment; native feedback identified the bad quote, but the correction restored a reflowed version. Identifier findings were empty on both corrected candidates. The app correctly withheld publication, but the user received no answer after more than six minutes. [Exact candidates and measurements](../acceptance/2026-10-17-fontdone-quotes-and-tool-timing.md).

## Required contract

1. Extend the existing owned range-reader protocol with native citation references tied to the operation's delivered snapshot/path/full-source digest/UTF-8 byte span and excerpt digest. Let Codex choose the relevant references per claim. Do not require the model to reconstruct quotes, whitespace, byte offsets or hashes in its answer.
2. Reuse the existing immutable range reader, source admission ledger, structured schema and claim/visual validation. Resolve references locally from exact delivered code before canonical chapter construction. Never resolve against the live checkout, a guessed substring, a first matching occurrence, documentation or a range outside the grant.
3. Define bounded citeable spans and coverage for large functions, repeated code, UTF-8, CRLF, adjacent/overlapping reads and partially delivered packets. A span may be disambiguated by its verified coordinates; do not disable exact source/digest/coordinate checks or normalize source to make it match. Mark genuinely unavailable references explicitly.
4. Include the minimal readable code and citation metadata in owned tool results without duplicating full source in metadata/structured-content wrappers. Account all source, metadata and payload bytes inside the original call/file/source/fragment/deadline bounds. A new source range still requires an admitted agent tool call.
5. Version the retrieval/schema/request/receipt contract. Keep existing saved quote-based chapters, origins, evidence receipts and rejected candidates readable and independently verifiable. Do not rewrite historical source, hashes or failed outcomes, and never replay inference during migration/restart.
6. Retain concrete identifier-to-claim/evidence alignment and existing inference/relationship rules. Visual nodes/edges continue referring to validated claim evidence; evidence references do not certify the semantic accuracy of an explanation or arrow.
7. Diagnose unknown, stale, foreign, unread or malformed references with bounded positions and fixed codes. Preserve the exact initial/corrected candidates and failure finding. At most one correction remains in the same owned conversation with no extra tools and the original cancellation/deadline/account/model; no automatic investigation retry or fallback provider.
8. Keep this change in the background. The reader asks a question, sees a grounded explanation, and opens source links. Do not add reference selectors, model metadata forms or another setup step.

## Acceptance

- Deterministic regressions exercise the two observed comment errors, repeated/overlapping source occurrences, Unicode/CRLF boundaries, large functions, adjacent ranges, invalid IDs, another operation/snapshot/runtime/grant, revoked permission, Stop, missing/corrupt blobs and uncertain transport. Every admitted citation must equal verified captured bytes at the stored coordinates.
- Verify exact claim/visual source navigation and legacy chapter/receipt compatibility. Inject a forged reference that names real but unread code; it must fail without fallback or source disclosure.
- Test metadata/source/payload limits and reference retention independently from inference; no new server/port or whole-file model preload.
- Use the existing installed Codex account and GPT-6 Luna/max for one deliberate frozen Fontdone attached-function acceptance after deterministic gates pass. Preserve failures as separate artifacts. Review the actual code facts, meaningful branches, clickable diagram and spoken script independently from successful source admission.
- Qualify the same path inside the installed app with the one-time connection and source grant, nested Guide/Flow/history navigation, restart, citations and Pocket narration. Record time to first independently admitted answer separately; the two-minute goal is a measured gate, not an exemption from evidence or quality.

## Scope boundaries

This task replaces fragile model-copied citation transport. Full persistent exploration conversations, progressive section publication, complete stage tracing, broad-repository narrative/language coverage and recovery retention UI remain required in their existing tasks. Closing this child does not close parent `gk46`, chapter-quality `q0sm` or timing `ps02`.

## Native citation implementation checkpoint — 17 October

The native evidence-reference protocol is implemented, with operation-bound delivered IDs resolved locally into exact captured spans. New schema versions preserve legacy chapters, receipts and rejected attempts. The full native suite passes **304 tests** (11 explicit probes ignored), frontend **145**, and production frontend build passes. The installed unsigned debug app passed executable integrity and reopened the saved nested Guide at section 2/3 with the same source selection and Codex connection. [Contract, limitations and package identity](../acceptance/2026-10-17-native-citation-references.md).

This remains open: one explicit frozen Fontdone GPT-6 Luna/max investigation, actual implementation-fact/diagram review, fresh citation navigation and Pocket narration must pass separately. Previous failed runs remain failed. Full broad explanation quality, latency, recovery controls and persistent exploration conversations are separate outstanding gates.
