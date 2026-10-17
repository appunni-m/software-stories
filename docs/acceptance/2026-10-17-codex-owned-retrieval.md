# Codex chooses and reads captured code

Checkpoint: 17 October 2026, source changes after `5c0e248`. **Synthetic account-backed investigation passed. Fontdone and packaged-app acceptance are separate and still open.** This replaces the earlier workspace question's mandatory metadata-selector/whole-file handoff in source. It does not migrate all legacy contextual dialogs or establish production readiness.

## What changed

Workspace chat now sends one intent request to the existing installed Codex app-server. The agent chooses searches, definitions, static navigation hints, module inspection and source-range reads through five application-owned MCP tools. The app supplies immutable evidence and independently validates the final claim/visual anchors. No additional agent framework, model fallback, direct API key, Node server or TCP port is introduced. Claude remains deferred.

The native evidence ledger binds the snapshot/account/runtime grant and records delivery reservations, range identities, overlap deduplication, call/byte limits and final receipt. Full captured files are read locally for independent validation; those local reads are not additional model context. See [the interface and bounds](../architecture/repository-retrieval.md) and [sharing disclosure](../PROVIDER_DISCLOSURES.md#workspace-investigation).

## Executed checks

| Check | Observed outcome |
| --- | --- |
| Full native suite | Latest run: 266 passed in 18.48 seconds, 10 explicit probes ignored; zero failures. |
| Frontend suite | Latest reading-position checkpoint: 141 passed; zero failures. |
| Frontend production build | Passed; existing large-bundle warning remains. |
| Debug application build | Passed before the read-error feedback refinement; the standard stdio bridge was exercised by the live test below. A newly packaged UI has not been qualified. |
| First synthetic live attempt | Failed after 94.29 seconds when an agent read requested an invalid hash or UTF-8 range. It exposed no source for that read. |
| Corrected synthetic live attempt | Passed in 173.83 seconds including fixture creation, connection/account/catalog checks and generation. Existing installed account, GPT-6 Luna/max, fixed `never` approval. |

The successful attempt made five calls: definition lookup, an invalid read receiving correction metadata, a 72-byte read, a search, and an overlapping read adding 125 bytes. Totals: **197 distinct source bytes, 4,623 metadata bytes and 4,829 serialized native tool-payload bytes**. It produced a one-section answer whose anchors passed native snapshot/range/claim validation, with a v3 request/ledger receipt. [Recorded call and origin metadata](artifacts/2026-10-17-codex-tool-probe.json) contains no model reasoning or source contents. This programmatic validation does not independently assess narrative quality. The elapsed time is not a satisfactory speed result.

An invalid eligible-path read now returns a bounded `readError` with the actual captured file identity and size, with **no code or evidence**. The agent can correct its own request inside the original operation. That error response is durably reserved and charged as metadata/wire bytes. Native scope changes, foreign paths, unknown/unowned calls and uncertain delivery remain terminal. Tests cover mixed batches so a good range earlier in an invalid batch cannot leak through.

Additional deterministic checks cover overlap deduplication, exact UTF-8 byte positions, invalid source identities, cancellation/revocation/deadline/runtime changes, uncertain writes, forged tool events, strict packets, private socket cleanup, legacy permission rejection, large-source metadata-only attachments and v3 receipt round-trip/tampering. These checks do not cover every acceptance boundary below.

## Reproduce deliberately

Offline checks do not contact a model:

```sh
npm run test
npm run build
env RUSTC_WRAPPER= CARGO_INCREMENTAL=0 cargo test --manifest-path src-tauri/Cargo.toml --lib
```

The next command consumes the existing signed-in Codex account's quota. It requires the existing supported runtime/account and GPT-6 Luna/max; it never installs a CLI or substitutes another model. Build the debug application first so its executable can act as the standard MCP bridge for the Rust test harness:

```sh
env RUSTC_WRAPPER= CARGO_INCREMENTAL=0 cargo build --manifest-path src-tauri/Cargo.toml --bin code-atlas-desktop

env RUSTC_WRAPPER= CARGO_INCREMENTAL=0 \
  CODE_ATLAS_CONTEXT_HELPER_BIN="$PWD/src-tauri/target/debug/code-atlas-desktop" \
  cargo test --manifest-path src-tauri/Cargo.toml --lib \
  workspace_generation::tests::codex_account_retrieval_probe -- --ignored --exact --nocapture
```

`CODE_ATLAS_CONTEXT_HELPER_BIN` exists only in test builds of the parent; production always launches its own application executable. The helper receives an operation-owned socket/token through the bridge, not a model or repository selection.

The ignored `workspace_generation::tests::fontdone_codex_retrieval_probe` additionally requires the frozen Fontdone snapshot and `CODE_ATLAS_FONTDONE_DATA_DIR`; select `CODE_ATLAS_FONTDONE_CASE=broad`, `memory-face` or `outline`. It records exact public-source draft/evidence artifacts for independent review. Live runs are deliberate, separate requests; there is no inference resubmission loop.

## First Fontdone run

The first new broad run stopped before a source read because Codex requested 50 module items and the native page contract rejected anything above 32. The exact input was confirmed against its recorded argument digest: `{scope:null,limit:50,offset:0}`. Native execution took 40.654 seconds; the complete test took 68.18 seconds. [Preserved failure artifact](artifacts/2026-10-17-fontdone-agent-broad-invalid-page.json) records no published answer and no evidence ranges.

The bridge now caps requested page sizes at 32 while preserving native continuation offsets. A regression exercises module, definition and search calls with excessive/zero page sizes, plus dot as the root scope. This is bounded retrieval, not automatic inference resubmission. The second Fontdone operation passed two module calls, then stopped when a private tool request arrived before its owning app-server notification had been processed. It ran for 96.887 seconds in native generation (124.88 seconds for the whole test), with zero source reads. [Its separate artifact](artifacts/2026-10-17-fontdone-agent-broad-event-order.json) is preserved. The bridge now waits at most three seconds for the exact matching started event, without exposing any data while unowned. Tests exercise both legitimate early requests and a nonce with no matching event. The third operation passed five discovery calls (two module listings and three definition queries), then stopped at private transport setup before any source read. Native generation took 209.876 seconds; the complete test took 238.07 seconds. [The preserved transport failure](artifacts/2026-10-17-fontdone-agent-broad-transport.json) contains no published answer or admitted source ranges. Transport diagnosis remains open. Neither failed attempt is erased by the synthetic success.

## Remaining acceptance

- The broad Fontdone question must reach `src/font.rs` plus real parsing/loading/hinting/rendering dependencies, with an independently reviewed useful answer and flow. The attached `memory_face` and direct-first Outline cases need separate results.
- Complete file/source/wire/call-budget exhaustion, transport/account changes, mid-search/read/inference/validation/publication Stop, queued successor recovery and restart matrices. Partial-answer behavior at a budget boundary remains incomplete.
- Package the same implementation and qualify helper startup, source navigation, saved selection, no repeated connection chooser and one-time/revocable scope in the real app.
- Migrate or explicitly retain the legacy file/plan/section/Git operation contracts. Native tests and this small answer do not pass whole-repository narrative, diagram, language or voice-quality gates.

Kata `gk46`, `a699`, `0wdn` and the narrative/quality tasks remain open.

## Local runtime-check cost

The installed Codex executable is 246,046,384 bytes. A standalone unoptimized streaming SHA-256 diagnostic took 8,701 and 9,436 ms per read. The native owned-tool gate performs this check before accepting a connection, again before processing it, and before writing its result. Several parallel requests can therefore wait behind expensive local checks. This is a plausible contributor to the third transport failure, not a confirmed account/model or transport root cause.

The existing RustCrypto `sha2` dependency now enables its `asm` feature (including runtime-detected ARM SHA instructions with software fallback), and development/test builds optimize that dependency. The streaming check, source/account gates, cancellation checks and metadata validation remain in place. Four calls to the actual native `executable_digest` returned the same SHA-256 in **126, 128, 122 and 126 ms**. [Timing scope and limitations](artifacts/2026-10-17-runtime-digest-timing.json) separate these native samples from the earlier standalone diagnostic; they do not establish complete generation latency.

The native suite now passes **266 tests**, with ten explicit probes ignored, including new streaming known-vector/cancellation/path regressions, privacy-safe transport diagnostics, packet regressions and the literal-search regression below. The frontend has **141 passing tests** following the reading-position fix. The local timing result does not establish large-repository success.

## Native packet transport

A separate macOS probe reproduced an accepted Unix socket inheriting nonblocking mode from the listener: a read with a configured 50 ms socket timeout returned `WouldBlock` immediately. This is consistent with the [Apple `accept` contract](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/accept.2.html). The native owner now explicitly makes accepted sockets blocking before reading a packet.

Native packets also have one absolute deadline across all partial reads or writes: 200 ms for an input packet and two seconds for an output packet. Short socket waits check Stop between attempts; a slow peer cannot restart the deadline with each byte. The standard MCP stdio wire and helper protocol are unchanged. Existing owned-session regressions now split both the length header and body, exercising the real owner and evidence admission. Additional tests cover slow partial input, Stop during input or blocked output, and output deadline expiry without Stop. The complete native suite passed in 25.25 seconds.

An unsigned release `.app` and `.dmg` built and passed the bundle/DMG integrity checks with the SHA optimization and saved-reading-position fix. That build predates the packet changes above; rebuilding it and checking the replacement UI remain separate work. The next Fontdone probe uses the current native test driver with that release executable as its unchanged stdio helper. It does not establish acceptance of the packaged parent application.

## Fourth Fontdone broad run

The current native owner completed 52 tool deliveries and admitted **47,986 distinct code bytes** from `src/api.rs`, `src/font.rs` and `src/tables.rs`, represented by 21 evidence ranges. Metadata consumed 239,661 bytes; native serialized tool payloads consumed 288,861 bytes. The 53rd call stopped at `Code search terms are invalid.` Native generation took 125.603 seconds; the full test took 127.82 seconds. No answer was published. [The separate failure artifact](artifacts/2026-10-17-fontdone-agent-broad-search-terms.json) preserves the request, counters and admitted range identities.

This run progressed past the earlier transport failures and read the large implementation files. It still fails the broad-answer acceptance gate. The record stores argument digests, so it does not establish the exact text of the failing search. An independent owned-session reproducer with the valid code query `greeting(` produced the same error before the fix. In the locked `grep-regex` version, `build_literals` still parses regular expressions unless `fixed_strings(true)` is configured; the case-insensitive setting takes the parsing path. Both snippet matchers now explicitly use fixed strings. The regression also exercises brackets, a trailing backslash and regex-looking punctuation without exposing source as search evidence.

The literal-search regression failed before changing the matchers; the full suite then passed 266 tests with ten explicit probes ignored in 18.48 seconds. Discovery consumed most of the metadata budget before reaching loading/hinting/rasterization. Exposing remaining budgets, reducing redundant discovery and supporting a useful cited partial answer at a limit remain open; this run is not a two-minute useful-answer pass.

## Focused Fontdone predicate

A separate, deliberate `outline` request passed using GPT-6 Luna/max and the same saved connection: four tool calls, **948 distinct source bytes**, 13,043 metadata bytes and 14,014 native serialized payload bytes. Native generation took 53.087 seconds; the complete test took 55.46 seconds. It produced one validated section, two claims, two source anchors and a three-node/two-edge data-flow visual. [Exact public-source draft and receipt](artifacts/2026-10-17-fontdone-agent-outline.json).

Source inspection independently matched both anchors' full-file hash, exact UTF-8 bytes, uniqueness, line coordinates and admitted ranges against the pinned clean `src/outline.rs`. The answer leads with the actual OR predicate, explains that either true condition suffices when they disagree, and states the false case as nonempty points with a nonzero contour count. Both diagram edges cite that predicate; this is a logical dependency diagram, not an observed execution trace.

Narration is insufficient: it retained only “They are distinct stored fields, matching the two separate tests in the predicate.” The workspace canonicalizer discards the separate proposed narration and prefers claim sentences without code punctuation, so the principal predicate sentences were skipped. Align the agent instructions and supported narration selection before a listening-quality pass; do not bypass claim grounding or present this as Pocket playback evidence. The attached `memory-face` case, broad journey, human quality rubric and packaged player/navigation remain open.
