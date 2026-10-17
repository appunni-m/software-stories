# Saved route and chapter replay — 17 October 2026

## Observed failures

The installed app's existing synthetic shopping-cart route uses the **current**
planning request hash, `c19d282d051624a77f2afd78ed45ee2550a280806e28a3fbcf4aae0f8723c88c`.
Its local draft loads; its accepted route fails with “inconsistent identity or
content.” A focused typed-round-trip regression fails before the fix: enum
fields `source_sha256`, `relationship_id` and `visible_label` are serialized as
their newer camelCase names. This changes the recorded plan digest even though
the independently validated claims/source are unchanged. The accepted record
expects `665b5886ff8e246abd6a499735bdd18559b70c2e5aa7f2b398a7d968973ae11a`.

A separate regression constructs older-contract variants using exact templates
from Git history. Before the fix they fail with the misleading “original
question, inventory, source or index changed” message. These variants are
synthetic upgrade simulations, not historical account output. The actual saved
record does not reproduce a request-prompt mismatch.

Before-fix logs: `/private/tmp/code-atlas-route-replay-before.log` and
`/private/tmp/code-atlas-route-digest-before.log`. The unchanged native files are
in [the frozen fixture](../../benchmarks/saved-route-replay/README.md).

## Versioned correction

- Saved record schema 1 supports its two known native enum encodings. Reading
  and reserializing retain the original encoding and property order, preserving
  exact route digests, accepted revision IDs and chapter/step links. Hashes are
  recomputed from all current typed fields, never cached as reusable authority.
- New records use schema 2 with current camelCase enum fields. Future versions,
  mixed enum encodings, unknown fields and duplicate fields remain errors.
  Strict typed preview decoding occurs before alias inspection; buffering into
  a generic JSON value cannot erase duplicate-field errors. Record byte/count
  limits, no-follow reads and immutable publication are unchanged.
- Request contracts are frozen separately from the v1 immutable evidence
  context. They cover initial instructions (`a1636b2`), code-first instructions
  (`5fa83fd`) and the current explicit-evidence prompt/schema (`bd1b106`). The
  current request bytes/hash remain unchanged.
- Historical replay requires **all** recorded scope fields and the exact full
  request digest to match one known contract. It independently reopens source,
  inventory/index and all claims/citations, preserves original IDs and compares
  the full rebuilt plan with its saved version. Unknown policies produce a
  contract-specific error; changed evidence remains a source error.
- Historical templates cannot create new live plans. New generation keeps the
  current contract and existing runtime/account/source review boundaries. No
  inference, consent renewal or background retry occurs when opening history.

Future instruction/schema changes must append a new immutable request contract.
Changes to evidence/context serialization need a separate supported version;
this registry does not guess unknown formats. Older app binaries may reject
new schema-2 records; backward reading by those binaries is not qualified.

## Mechanical qualification

The regressions cover the real saved draft, accepted route and independent
chapter, including native source validation; simulated upgrades through both
older contracts; new schema-2 explicit acceptance and step preparation; both
schema-1 encodings; wrong context/inventory/question/coverage/index; corrupted
captured source/index; invalid quotes, identifiers, prerequisite order and
destinations; modified saved prose; unknown contracts; and future, mixed,
unknown or duplicate record fields. Successful and failed reads preserve
stored bytes. Adding a new explicit decision leaves older records unchanged.

- **382 native tests passed**, zero failed, 13 explicit account probes ignored,
  39.94 seconds. Eleven regressions specifically cover this replay correction.
- **184 frontend tests passed**, zero failed. Production TypeScript/Vite and
  the unsigned debug Mac app bundle build pass. Existing unused-code and large
  frontend chunk warnings remain.
- All three frozen contracts match exact request JSON bytes from separately
  extracted original Git implementations, using a fixed empty-data input.
  The actual saved fixture also pins the unchanged current request digest.
- The 138,694,144-byte debug executable has SHA-256
  `9776a7cfd4c86f754b465e9733409c6880f3dc5514973634ddaa41bfd026b107`.
  Embedded Mach-O integrity passes; this is not Developer ID, bundle resource
  sealing or notarization evidence. The bundle is not installed.
- Documentation inventory: zero errors, six existing review items. All 73 live
  Kata bodies match their checked-in definitions; statuses/priorities remain
  unchanged at 50 open and 23 closed.

[Exact source, frozen input, bundle and reference identities](artifacts/2026-10-17-saved-route-replay.json).
No fresh model request is involved. Logs:
`/private/tmp/code-atlas-route-replay-native-final.log`,
`/private/tmp/code-atlas-route-replay-frontend.log`,
`/private/tmp/code-atlas-route-replay-app-build.log` and
`/private/tmp/code-atlas-route-replay-doc-audit.txt`.

## Remaining product acceptance

The Mac remains locked; the replacement bundle has not been installed or
visually checked. Packaged multi-step batching, cancellation/recovery cases,
semantic quality, Pocket narration, accessibility and the full Fontdone guide
remain open. The last broad Fontdone run is still
[Partial](2026-10-17-scene-v2.md). This compatibility correction closes no full
Kata task and does not establish production readiness.
