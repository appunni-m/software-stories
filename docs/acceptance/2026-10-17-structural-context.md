# Fontdone structural context checkpoint

Date: 17 October 2026. This qualifies native functions against captured source, not the installed application, Codex tools or explanation quality. The [industry review](../research/2026-10-17-repository-context-and-indexing.md) remains the architectural recommendation. Exact observations and implementation hashes are in the [metrics artifact](artifacts/2026-10-17-fontdone-structural-metrics.json).

## What changed

The native context service now offers bounded definition lookup, incoming/outgoing relationship lookup and immediate-child module maps through `inspect_repository_structure`. It reuses the captured snapshot reader, saved Tree-sitter index and existing request ownership/cancellation registry. No new dependency, agent runtime, server or port was added. The [interface reference](../architecture/repository-retrieval.md) defines bounds and uncertainty labels.

A declaration verifies its captured hash and original UTF-8 coordinates, returns a prefix of at most 8 KiB and retains the full enclosing extent. Relationship endpoints/evidence are verified separately. Missing targets are unresolved; excluded requested endpoints return no links. Exact-file lookup reports the saved file syntax status, including `fileTooLarge`, rather than presenting an empty result as successful semantic coverage. The compact map is file inventory with parser metadata, not an inferred architecture.

## Frozen Fontdone observations

The same [twelve independently selected declaration spans](fixtures/fontdone-retrieval-ranges.json) and snapshot from the [text retrieval checkpoint](2026-10-17-range-retrieval.md) were used. No source was sent to a model and no Fontdone code was executed.

| Native operation | One local debug observation |
| --- | --- |
| Scoped named definition lookup | Eight of twelve expected spans found; the eight syntax-backed cases took 2–28 ms each. |
| FFI/C adapter definition lookup | Four missing spans have saved `fileTooLarge` status. Their captured text remains available through the separately measured lexical range service. |
| `memory_face` outgoing links, first page | 32 unresolved, zero resolved static candidates, continuation offset 32; 508 ms and 27,392 serialized result bytes. |
| Root module map, first page | 1,238 ms; 4,732 serialized bytes; 32 entries, continuation offset 32. Saved syntax index status is partial. |

The first map page reports 1,147 unavailable captured entries and 25 capture skips. These counts are not a reason-by-reason evidence ledger. Items rank by code-file count and path, so Fontdone's large tests directory appears first; this is not an architecture-importance claim. Maps hash eligible in-scope text on every request; no persistent map cache is implemented.

The all-unresolved relationship page is evidence against relying on this parser graph for cross-module explanations. Investigate explicit text, imports and callers, and evaluate the reviewed LSP/SCIP adapters before extending custom semantic resolution. Scoped names were supplied by the frozen probe; no broad-question planner or model discovered them in this run. These results do not improve or replace the earlier 2/12 broad lexical measurement by themselves.

## Validation and limits

The current complete native suite passed **238 tests**, with seven intentionally ignored tests; the explicit Fontdone structural probe passed separately. The suite includes 14 default structural/input regressions for original coordinates/hashes, huge UTF-8/CRLF declarations, ranking/pagination, cancellation, source budgets, malformed command fields, excluded/corrupt sources, unresolved links and parser-status disclosure. The map's command uses an explicit empty payload so Serde rejects unexpected fields; the initial unit-variant shape did not enforce that restriction.

Local storage exhaustion interrupted an earlier linker attempt and one subsequent suite run. Removing only generated incremental/release dependency outputs recovered space; the reported suite and probe completed afterward. App bundles, source, snapshots and saved explanations were retained. This was an environment failure, not a passing product-storage qualification.

The full native suite exercises core functions and input deserialization; it does not establish packaged Tauri IPC, interruption at every filesystem boundary or visual quality. No frontend change or model inference is part of this checkpoint. The existing whole-file Codex generation selector remains unchanged. Aggregate transmitted-byte accounting, overlapping evidence admission, tool-call/deadline caps, versioned account/snapshot permission and output validation must pass before these operations can be delivered to an agent. Kata `a699` and `gk46` remain open.

## Reproduce the explicit probe

Use the existing immutable public snapshot; the probe does not rebuild or overwrite it.

```sh
env RUSTC_WRAPPER= CARGO_INCREMENTAL=0 \
  CODE_ATLAS_FONTDONE_DATA_DIR="$HOME/Library/Application Support/com.codeatlas.desktop" \
  cargo test --manifest-path src-tauri/Cargo.toml --lib \
  repository_structure::tests::fontdone_structural_profile -- --ignored --exact --nocapture
```

Logs contain counts, IDs, coordinates/status metadata and timing, with no source snippets, credentials or model reasoning. Timing is one debug observation without an OS cache flush or concurrent build; it is not a product performance guarantee.
