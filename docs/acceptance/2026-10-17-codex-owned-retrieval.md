# Codex chooses and reads captured code

Checkpoint: 17 October 2026, source changes after `5c0e248`. **Synthetic account-backed investigation passed. Fontdone and packaged-app acceptance are separate and still open.** This replaces the earlier workspace question's mandatory metadata-selector/whole-file handoff in source. It does not migrate all legacy contextual dialogs or establish production readiness.

## What changed

Workspace chat now sends one intent request to the existing installed Codex app-server. The agent chooses searches, definitions, static navigation hints, module inspection and source-range reads through five application-owned MCP tools. The app supplies immutable evidence and independently validates the final claim/visual anchors. No additional agent framework, model fallback, direct API key, Node server or TCP port is introduced. Claude remains deferred.

The native evidence ledger binds the snapshot/account/runtime grant and records delivery reservations, range identities, overlap deduplication, call/byte limits and final receipt. Full captured files are read locally for independent validation; those local reads are not additional model context. See [the interface and bounds](../architecture/repository-retrieval.md) and [sharing disclosure](../PROVIDER_DISCLOSURES.md#workspace-investigation).

## Executed checks

| Check | Observed outcome |
| --- | --- |
| Full native suite | 260 passed, 9 explicit live probes ignored; zero failures. |
| Frontend suite | 136 passed; zero failures. |
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
