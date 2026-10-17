# Current chapter quality boundary — 17 October 2026

## Result and scope

The benchmark checker now uses the app's compiled file-chapter request builder,
current response schema, canonicalizer, fresh narration preparation and complete
source/scene validation. It accepts the unchanged source-valid Fontdone answer
with **23 anchors** and continues to reject the original five leaf-parent errors.
No LLM/agent generation was run and no new revision was accepted.

This repairs the checker; it does not qualify the Fontdone answer's teaching
quality. That answer still has the previously recorded syntax-heavy visuals and
narration. [Original generation and quality limits](2026-10-17-shared-teaching-policy.md).

- [Source pins, results, failed runs and bounded native observations](artifacts/2026-10-17-native-quality-boundary.json).
- [Compiled contract metadata](artifacts/2026-10-17-native-quality-metadata.json).
- [Current workflow and remaining provenance limits](../CHAPTER_QUALITY_GATE.md).

The tests ran on the working tree based on `71a56d9`; the evidence file pins the
exact changed implementation inputs. Compare those pins with the resulting
commit rather than treating the base commit as the tested implementation.

## Contract changes

Report schema **5** / response schema **2** supports current visual compositions
and explicit backend/model/reasoning/approval fields. Metadata defaults to
Codex GPT-6 Luna/max, approval `never`. Direct API contract preparation uses the
app's current catalog without credentials or inference. Fixed public source and
synthetic runtime placeholders fingerprint the compiled contract; they do not
prove an installed account or completed agent run.

The separate `code-atlas-quality` executable accepts one bounded JSON request
on stdin. It stages only supplied, hash-checked source in private temporary
storage and removes that storage afterward. Source count, byte limits, paths,
duplicates, secret/exclusion rules and the current request builder still apply.
An infrastructure failure cannot be recorded as a candidate schema failure.
The existing `jsonschema` crate, pinned to **0.59.0** with default features off,
checks the native wire schema. File and HTTP reference resolvers are disabled.
Native code retains ownership of evidence, narration and scene admission.

Metadata includes the actual appended teaching/composition instructions and
compiled schema. The adapter matches `build.rs`'s native input hash
and fingerprints both checker files. Rust path component ordering is covered
by the real compiled-helper identity check. Stale helpers/reports fail closed.
The historical authored shallow/rubric fixture remains byte-identical and uses
its separate legacy validator only for that example.

## Mechanical verification

| Check | Result |
|---|---|
| Focused native boundary | 6 passed |
| Complete native library suite | 410 passed; 15 explicit account probes ignored |
| Complete frontend suite | 190 passed |
| TypeScript/Vite build | Passed; existing large-chunk warning |
| Full `npm run check` | Passed after final test placement cleanup |
| Clippy, all targets | Completed with warnings; none in the new helper |
| Unsigned debug Mac bundle | Built and installed; previous bundle preserved |

The evidence retains initial failures: a local compiler-cache startup failure;
an early test invocation that reached an older desktop executable before linking
finished; and a raw-path sorting error that wrongly rejected a current helper.
The separate executable and matching Rust path ordering fix the latter two.
Subsequent focused and complete suites pass. Builds used `RUSTC_WRAPPER=` to
bypass the failing local cache. No global cache configuration was changed.

## Native Fontdone observations

The installed rebuilt app opened the existing **partial** Fontdone snapshot,
head prefix `5249b72371`. These checks used the already saved **PFR0 prefix
dispatch** scene, separately from the replayed `loca.rs` artifact:

- Opened `src/lib.rs`, then clicked the PFR0 visual source control. The code pane
  returned to `src/font.rs`, selected lines **4053–4078**.
- Entered an existing nested exploration, selected Flow and returned. The parent
  retained Guide, section **2/3** and another scene **2/4**. Re-entering the child
  restored Flow. The child answer itself remains stopped.
- Played the saved narration. The player reported **PLAYING**, **00:09/00:17**,
  with position **9.2 s**. Stop reset the position. This verifies player progress,
  not audible output quality, pronunciation or comprehension. In the final build,
  playback also reached the end at 17.9 s before Stop.
- Left the app on the real PFR0 visual with the cited source alongside it.

Installed executable: **149,364,528 bytes**, SHA-256
`4de26acbc3122d703ec436346e90d90b1a3255de62571e372373d5c8e2d73586`.
The binary signature check with `--ignore-resources` passes. This is an unsigned
development bundle, without Developer ID signing or notarization. Its headless
metadata reports the same native input hash as the final checked source tree.
Restoration, source links and nested navigation were rechecked in this final build.

## Still open

Kata t14 remains open for captured real request-review/index identity binding,
the complete generated representative corpus with separately generated repairs,
and actual human semantic/visual/narration review. Backend/settings fields and
a copied fingerprint do not prove that an agent ran. Whole-repository narrative
completion/timing, Pocket listening/comprehension, full accessibility and release
acceptance remain separate unpassed requirements.
