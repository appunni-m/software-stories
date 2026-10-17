# Local repository retrieval

Implementation checkpoint: 17 October 2026. This service powers workspace text search and owned Codex investigation in source. Synthetic and focused Fontdone investigations have passed; broad repository quality and packaged parent acceptance remain open. Legacy reviewed dialogs still supply fixed evidence. See the [executed investigation evidence](../acceptance/2026-10-17-codex-owned-retrieval.md), [local measurements](../acceptance/2026-10-17-range-retrieval.md) and [remaining contract](../kata-tasks/69-local-range-retrieval.md).

## Native interface

| Command | Contract |
| --- | --- |
| `search_repository_code` | Input object: request UUID, active project, captured snapshot, query, optional exact file/directory scope, result limit. Returns ranked range metadata, bounded snippets, coverage, truncation and local timing. |
| `inspect_repository_structure` | Input object: request UUID, active project, captured snapshot, optional file/folder scope, page limit and offset, and one operation: `definitions` (query), `relationships` (captured endpoint ID and incoming/outgoing/both direction), or `map`. Returns verified declaration metadata, static links or immediate child modules, with syntax status and unavailable counts. |
| `read_repository_range` | Input object: active project, captured snapshot, original path, expected full-file SHA-256, start/end byte offsets. Reads only eligible captured text; returns verified coordinates, range digest and text. |
| `cancel_repository_context` | Cancels a request UUID, including a bounded tombstone when cancellation precedes dispatch. |
| `search_workspace_source` | Adapts the same search service to the existing workspace search UI. Every debounced query has an ID; replacing it or leaving the view cancels that request. |

These commands require an active native project session. Local search does not require an AI connection, permission to transmit source, an API key, a new server or a port. They do **not** authorize an agent to call them. The dependent [agent tool contract](../kata-tasks/70-codex-retrieval-tools.md) must bind its own transmission permission, account/runtime identity and cumulative limits.

## Source and range identity

All reads go through the existing immutable snapshot reader, which verifies actual blob size and SHA-256. The service additionally rejects invalid relative paths, excluded directories, non-file entries, unavailable text and native secret-like matches. A parser limit does not exclude captured text. Missing or corrupt syntax metadata permits text fallback; it does not create a parser-support claim.

Ranges carry snapshot, original path, full-file digest, range digest, absolute UTF-8 byte offsets and one-based inclusive line numbers. End byte offsets are exclusive. UTF-8 boundaries and coordinates are checked again before returning search evidence. Symbol/enclosing-parent context and the saved parser identity are optional. These are static syntax records, not compiler-resolved or observed runtime relationships.

Chunking prefers existing Tree-sitter declaration boundaries in the second half of an 8 KiB window. It otherwise prefers line boundaries, splitting very long lines at UTF-8 boundaries. Ranges partition a file without overlap. Oversized enclosing declarations retain symbol context in the windows they contain. The agent evidence ledger separately deduplicates overlapping requested ranges before transmission.

## Structural context

Definitions are ranked by qualified name, exact name, ASCII case-folded name, unqualified name and partial name, with path/offset/ID tie breaks. A declaration returns an at-most-8-KiB range and retains the full declaration extent; use a separately bounded range read for more source. Full-file hashes, UTF-8 boundaries and the original parser start/exclusive-end coordinates must match the captured blob. Invalid evidence is omitted and counted.

Relationship lookup verifies the requested endpoint, each evidence anchor and both available endpoints. High-confidence parser links are labelled `staticCandidate`, never compiler or runtime proof. Unresolved links retain their uncertainty. Excluded or corrupt requested endpoints disclose no links; unavailable recorded targets have their metadata redacted. A link to a target outside the selected result scope remains possible: scope filters the evidence location, while the active snapshot remains the authorization boundary. None of these commands independently grants permission to transmit that snapshot to a model.

The map groups eligible captured text into immediate child files/folders, with text/code counts, captured bytes, parser coverage and at most four verified exported declaration labels. Unsupported languages remain represented through text. Items sort by code-file count and path; this is not importance ranking. It is an orientation aid, not an inferred execution order or an architecture explanation. Maps currently verify in-scope blobs on every request; there is no persistent map cache.

Structural pages contain at most 32 items; continuation offsets cannot exceed 100,000. A request verifies at most 100 MiB of distinct source, shares the existing eight-owned-request registry and checks the cooperative 90-second deadline. Missing/corrupt syntax permits text fallback; the response reports syntax availability, saved status/version, exact-file scoped syntax status (including `fileTooLarge`), capture skips and unavailable items. The unavailable count is not yet a reason-by-reason coverage ledger. Agent transmission uses the separate evidence-ledger bounds below.

## Search and publication

- Tantivy indexes source tokens, original identifiers, snake/camel/acronym components, path terms and declarations where syntax metadata exists. Exact declarations rank before exact identifier mentions, then lexical candidates, with score/path/byte ordering. Candidate sets are capped; truncation is reported. Equal-score selection beyond the candidate cap still needs boundary qualification.
- The ripgrep project's Rust libraries locate matching snippet lines inside retrieved ranges, including parser-unavailable files. A full repository regex/substr fallback when lexical search is unsuitable is still pending.
- Native path-only secret filtering also runs before reads; the retrieval policy version is `atlas-ranges-v1-identifiers-v1-policy-v2`, invalidating older cache generations.
- A generation fingerprint binds the complete manifest, saved syntax index, chunking/tokenizer version and schema. A bounded content-addressed chunk-layout cache can reuse unchanged source. A new generation currently rehashes all eligible blobs and reinserts all search documents; this is **not** an incremental Tantivy update implementation.
- A staging directory receives the complete Tantivy commit and metadata. Publication renames it and atomically replaces a small current-generation pointer on the same filesystem. Failed/cancelled work does not replace that pointer. Metadata digests, schema, document count and Tantivy checksums are checked on disk load. The current and previous complete generations are retained during cleanup.
- The native owner serializes search/build operations and retains one prepared index in memory. The cache binds manifest identity, syntax-file and publication-pointer metadata, with a 60-second checksum refresh interval. Evidence blobs are still hash-checked on every search; each distinct file is read once per search. A parser finishing during a build invalidates reuse on the following search. Search errors invalidate the in-memory index.

## Current local bounds

| Resource | Bound |
| --- | --- |
| Captured paths admitted to a build | 25,000 |
| Verified source bytes per build | 100 MiB; existing capture limits also apply |
| Indexed ranges | 50,000, each at most 8,192 bytes |
| Index metadata / one chunk-cache record | 32 MiB / 4 MiB |
| Query | 2–240 characters, up to 32 derived terms; workspace UI currently caps 120 characters |
| Candidate sets | At most 256 each for lexical, exact identifier and exact declaration searches |
| Returned matches | 1–32; snippets at most 360 characters each |
| One explicit range read | At most 32,768 source bytes |
| Request registry / owned queued or running operations | 64 / 8; cancelled jobs retain ownership until return |
| Build/search operation deadline | 90 seconds, checked cooperatively between native operations |
| Tantivy indexing workers / memory arena | One / 30 MB |

The deadline does not preempt a synchronous Tantivy call. `resultBytes` measures serialized match records, including snippets/coordinates, not tokens or total wire framing. The cached build counters describe the originating index build, not new work performed on each warm search. Total request timing in the acceptance probe includes manifest/cache preparation; `elapsedMs` in a search result describes search/read work after preparation.

The current 250,000-byte model-operation source limit is unchanged. The source implementation now exposes five owned reading tools and a bounded section-proposal tool through the existing Codex runtime; its initial native tests and live-account/package qualification are separate. Global disk retention/eviction and complete cache-boundary corruption/cancellation qualification remain open; disposable cache files must not be confused with saved explanations.

## Rejected candidates and one correction

Workspace requests now use native citation references under `codex-retrieval-v2-native-citations` / `single-owned-correction-native-citations-v3`. Codex selects IDs from delivered `read_code` fragments; the app resolves exact captured bytes and coordinates locally. Claim evidence accepts `{referenceId}` only. This replaces the fragile model-copied quotes in the previous `single-owned-correction-exact-quotes-v2` workspace contract. Legacy quote-based proposals and saved records retain their original exact UTF-8/length/uniqueness checks. [Citation contract and qualification](../acceptance/2026-10-17-native-citation-references.md).

Newly delivered gaps are partitioned into at-most-1,000-byte fragments at UTF-8/CRLF-safe boundaries, preferring line ends. Short/blank fragments are explicitly not citeable. Overlap deduplication still sends each source byte once. Native IDs bind the full operation/grant/runtime/snapshot identity; unavailable or foreign IDs yield fixed position/code feedback without source text. Delivered coordinates disambiguate repeated code. Native lexical alignment still requires each declared identifier in its own claim prose and cited evidence. This uses the same 64-issue, 32,000-byte feedback limit.

The same conversation can correct the answer once using source already delivered. Event and socket admission refuse further tools during correction. The original deadline, account, GPT-6 Luna/max, schema and cancellation owner remain in force. Both rejected/corrected candidates are retained privately. A corrected candidate still needs full independent claim, evidence, visual and storage admission before publication. Preflight does not review semantic usefulness or repair arbitrary visual/schema failures. There is no automatic replay of the investigation. The [Fontdone failure and fix](../acceptance/2026-10-17-fontdone-quotes-and-tool-timing.md) separates that source contract from the pending large-repository quality and speed gates.

Private records live in `rejected-explanations/<operation UUID>.json`, separately from the source-free evidence ledger. They bind the request fingerprint, snapshot/runtime/grant receipt, delivered-range digest, thread/turn identities, exact structured response, response digest and positional alignment findings. The store admits at most eight operations, two candidates per operation and 2 MiB per record, reserving at most 16 MiB. Files use mode 0600 in a 0700 directory. Existing records are not silently evicted or overwritten with a different candidate. Reasoning and streamed analysis are not recorded. Default diagnostics do not export these source-containing records.

**Pending:** a user-facing recovery/retention control must let the reader inspect or remove saved rejections without editing app storage. A full store currently refuses another rejection/correction, so this quota-management gap is tracked in [Kata gk46](../kata-tasks/70-codex-retrieval-tools.md). Test-only inspection checks the record against its independently persisted evidence receipt. Persistent exploration threads, whole-guide resume and semantic-quality qualification remain separate work.

## Native retrieval timing

New evidence ledgers identify `contextClock: "capturedCodeSessionV1"` and store each call's monotonic `timing.startedMs`, `preparedMs` and `finishedMs`. The clock starts after initial journal creation and before syntax loading. It excludes earlier renderer queue/account preparation. Start is recorded before native retrieval; preparation ends before the durable Prepared write; finish records successful transport or rejection/uncertain interruption before the terminal journal write. These are boundary observations, so final fsync and outer MCP/event handling are not all included in a call's duration.

Missing preparation/finish boundaries remain null on in-flight calls; interrupted calls retain the last observed boundary. Legacy records omit the clock/timings and remain unknown, never zero. Inspection rejects unknown clocks, inconsistent states, backwards/overlapping call timelines and implausible offsets above one day. Restart reads existing observations without resuming inference. Timing fields contain no code, query, credentials or reasoning.

Gaps between native calls include unmeasured agent/provider/transport/event work; they cannot be labelled model reasoning time. This is the retrieval portion of [Kata ps02](../kata-tasks/71-generation-timing-traces.md), not the required complete queue/preparation/inference/validation/publication trace or its diagnostic export/retention qualification.

## Dependencies and qualification

Direct versions are locked in Cargo:

| Dependency | Declared licence / compatibility | Use |
| --- | --- | --- |
| Tantivy 0.26.2 | MIT; declared Rust minimum 1.86 | Embedded lexical index; default features off, `mmap` and `lz4-compression` enabled. |
| grep-regex 0.1.14 | Unlicense OR MIT; minimum Rust not declared in inspected metadata | Bounded literal snippet matching. |
| grep-searcher 0.1.17 | Unlicense OR MIT; minimum Rust not declared in inspected metadata | Search captured range bytes in-process. |

Versions/licences were checked against their downloaded Cargo package metadata. This is not a transitive licence audit or minimum-toolchain qualification. Current development checks use rustc 1.98.1 on this Mac. The implementation adds no external search executable or Node process. Native tests and `cargo check` compile these dependencies on macOS; a newly packaged application still needs qualification.

Run deterministic retrieval tests with:

```sh
env RUSTC_WRAPPER= cargo test --manifest-path src-tauri/Cargo.toml --lib repository_context

env RUSTC_WRAPPER= cargo test --manifest-path src-tauri/Cargo.toml --lib repository_structure
```

The [Fontdone report](../acceptance/2026-10-17-range-retrieval.md) gives explicit local measurement commands and the frozen source-range set. Passing these tests does not establish broad-question recall or a completed explanation.

## Owned Codex investigation

New progressive answers use `answerSectionV2` with `nativeEvidenceV2` short
operation-local references such as `code:1`. Read replies expose `referenceId`
and overlapping reads expose `reusedReferences`; cite only entries marked
`citeable:true`. The native owner retains the full digest identity and resolves
ordinals only against delivered captured evidence. New answer-record schema 2
replays the same checkpoint order; schema 1 keeps the original digest protocol.
Aliases never resolve globally or by a nearest match. [Contract and current
qualification](../acceptance/2026-10-17-short-native-references.md).


`generate_progressive_workspace_answer` starts the existing managed Codex app-server with `search_code`, `read_code`, `find_definitions`, `find_relationships`, `list_modules` and `propose_answer_section`. The five reading tools retain their source scope; section proposals do not add source access. Legacy `generate_workspace_answer` keeps its five-tool final-chapter contract. Codex chooses relevance. The app provides immutable tools and validates the result; it does not run another model to select whole files before the question.

`rmcp` 3.5.1 (Apache-2.0; declared Rust minimum 1.88) supplies the standard MCP stdio server. The application binary's `--atlas-answer-mcp` entry (legacy `--atlas-context-mcp` for five tools) runs a bridge to an operation-owned, private Unix socket. Tool execution stays in the owning native application and uses the existing cancellable retrieval service. The helper has no repository/storage path or model credentials; no HTTP server or port is added.

The native owner explicitly uses nonblocking accepted sockets. One absolute deadline bounds a complete private packet: 200 ms for input, two seconds for output. A `WouldBlock` wait lasts at most 25 ms and checks Stop/deadline again before the next attempt, including fragmented headers/bodies and blocked output. The owner does not change socket timeout options: macOS can reject that change after the peer closes despite a complete queued packet. Complete queued packets remain readable; incomplete EOF, cancellation and oversized packets still fail. These packet deadlines are separate from the overall investigation deadline and the helper's result wait. [Reproduction, fix and qualification limits](../acceptance/2026-10-17-short-native-references.md#native-packet-race).

The `atlas-captured-ranges-v1-files12-source250000` grant and ledger bind project, complete manifest, snapshot, operation UUID, account/runtime, GPT-6 Luna/max and policy. Limits: 12 files; 250,000 distinct source bytes; 256,000 metadata bytes; 1,000,000 serialized native tool-payload bytes; 128,000 bytes per result; 64 calls; 512 admitted fragments; ten minutes. UTF-8 range reads are at most 32,768 bytes each. Search snippets are omitted from agent responses. Local complete source validation is separate from transmission.

Each tool reply includes `budget.remaining` after its own encoded cost, with capacity reserved for subsequent limit feedback. It reports calls, files, distinct source bytes, metadata bytes, native payload bytes, fragments and remaining milliseconds; these byte counts are not token estimates. `budget.finishNow` asks the agent to finish from existing evidence near a boundary. Discovery instructions prefer small pages and early source reads.

The last allowed call and any result that would exceed a file, source, metadata, payload, fragment or per-result bound return a charged `budgetStop` notice with no new context or evidence. Subsequent admitted calls return the same source-free notice. The next call beyond 64 and the ten-minute deadline remain hard stops. Existing delivered evidence can support a validated partial draft; that draft includes a visible scope note. This does not guarantee that an agent will produce an answer before the deadline, and it does not widen the grant or automatically resubmit inference.

The ledger durably reserves delivery before writing a response, deduplicates overlaps and checks scope again before transport. A lost or uncertain write terminates the operation, including for a limit notice. Incorrect read bounds/hash for an eligible captured path and invalid query text return bounded, accounted correction metadata and no code. An empty read batch or more than eight ranges returns `readError.code:invalidRangeCount`, without a path, source hash, size or any source fragments; no partial batch is read. Codex can split/correct the request within the same call/time budget. Unsafe paths, native identity changes and malformed/unowned calls stop the run. Persisted source counters must match admitted fragment lengths; source plus metadata cannot exceed the recorded payload. Original records without a `budgetStop` field remain readable. Completed Codex MCP events must match native admitted call/argument/payload digests. Final citation coordinates, quotes and identities must match both the snapshot and delivered ranges. Progressive chapters store schema-5 checkpoint origins under `codex-retrieval-v3-answer-sections`; schema-2 ledger IDs and schema-4 origins retain the fixed `codex-retrieval-v2-native-citations` contract. Earlier records remain readable.

Rejected native deliveries now retain an optional `rejectionCode` from a fixed
enum (range, captured-source, encoding/accounting or budget failures). Unmapped
errors become `unknown`; raw messages, OS details and requested paths are not
copied into that diagnostic. A code is valid only on a rejected call. Older
receipts without the field remain readable and their cause remains unknown.
This receipt is local diagnostic evidence; it cannot resume a stopped run or
certify an earlier failure's cause.

Workspace reading and narration are built from validated claims; separate proposed presentation fields are discarded. New progressive checkpoints retain every distinct grounded sentence, as described below. Historical checkpoints keep their original behavior-prioritized sentence selection rather than being silently rewritten. Inference labels and citation checks remain required. Neither policy establishes natural speech or comprehension; those require review of the actual script and Pocket playback.

After native scene rejection, a known cross-claim content node returns fixed `sceneEvidence` at its `nodeIndex` position. The node must cite the scene’s own claim. Feedback contains no source or model labels and does not replace native scene validation.

Progressive sections are independently admitted and atomically retained before reader events. A strict final manifest names current section identities; it cannot supply a replacement chapter. Draft adoption and restart reconciliation are local/idempotent. The reader appends stable cards without selecting newly arriving sections or starting narration. Native ownership, terminal outcomes, exact proposal/final-response retention and explicit bounded receipt management are described in the [reader integration checkpoint](../acceptance/2026-10-17-progressive-reader-integration.md).

New section checkpoints use version 2 presentation: all distinct anchored claim
sentences become reading and saved speech in order, within a complete 4,000-byte
section limit. `presentationLimit` rejects oversized copy without publishing a
truncated section. Version-1 checkpoints retain their original sentence
selection policy during independent replay; changing a stored checkpoint version
does not recertify its content. The request fingerprint includes the presentation
policy and checkpoint version. Both versions still require the complete native
source, claim, diagram and origin validation. Preserving all sentences fixes
information loss; it does not prove that the generated claims are true or that
the narration sounds natural.

Workspace chat uses this route in source. Native tests do not establish a working packaged product or the Fontdone broad/focused quality gates. Track those in [the Codex retrieval task](../kata-tasks/70-codex-retrieval-tools.md). Legacy chapter, plan, section and Git dialogs retain their existing fixed-evidence contracts.
