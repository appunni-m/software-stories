# Progressive answer reader integration — 17 October 2026

Source base: `313f647` plus this checkpoint. Status: the native workspace command and production reader now select progressive section generation. Installed-app and real-account qualification are separate gates below. Kata `3bz5` remains open.

## What changed

A workspace question now uses `generate_progressive_workspace_answer`, the owned six-tool session and the existing saved Codex account with GPT-6 Luna/max. Its bounded initial request contains intent and starting references, not a repository/source dump. Its fingerprint binds the progressive instructions, actual tool schemas, policy, final schema, question, snapshot, grant and settings. Legacy `generate_workspace_answer` and its five-tool contract remain available for historical callers and probes.

Codex publishes sections through `propose_answer_section`. A source-free Tauri event follows native durable validation/publication. The original-run ready-time observation is persisted separately from preparation timing; duplicate proposals keep the first observation. The reader subscribes before dispatch, independently adopts each native section into the existing immutable chapter library, and appends a stable card. Guide, Flow, citations, explicit narration/player actions and nested exploration reuse real chapter revisions. A later card does not select a different section or remount earlier cards. Publication does not start Pocket synthesis or playback.

The final `answerManifestV1` response names the latest accepted key/generation identities in order. It cannot omit, replace or add section content. Exact bounded final responses, including rejected manifests, are retained with digests. An unresolved rejected proposal or exhausted research budget prevents a complete result. Later failure returns a partial/stopped result when sections exist, preserving earlier drafts; malformed final JSON cannot clear those drafts.

Restart recovery lists and independently inspects saved answers, adopts any missing chapter revisions and marks an abandoned working run interrupted. It never reconnects Codex or submits inference. A native worker lease prevents navigation/recovery from mistaking a worker preparing its run for an abandoned job. Section and terminal observers support a reloaded webview, and Stop uses the persisted native run identity. Terminal answer records replace the legacy final-chapter recovery notice. Explicit retries use a new native run identity and retain cards from previous attempts. Lost events and save replies reconcile existing native identities rather than requesting another answer.

Investigation records remain bounded to eight answers, each reserving at most 4 MiB. At the capacity error, **Manage saved answers** offers an explicit action to save every published section and remove an older terminal investigation receipt. Native removal rejects active work or unsaved/changed revisions. It preserves questions and chapter drafts, but deliberately removes the detailed generation receipt. Records are never evicted automatically. Legacy pending-preview recovery is unchanged.

## Verification

- Complete native suite: **334 passed, 12 explicit live probes ignored**, 32.09 s. Log: `/private/tmp/code-atlas-progressive-integration-native-final.log`. Environment: `RUSTC_WRAPPER=`, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2`.
- Frontend suite: **148 passed**, 674.08 ms. Log: `/private/tmp/code-atlas-progressive-integration-frontend-test.log`.
- Production TypeScript/Vite build passed; existing large-chunk warning remains. Log: `/private/tmp/code-atlas-progressive-integration-frontend-build.log`.

New regressions cover manifest identities, immutable versions, unresolved/repaired rejection, partial outcomes, independent saved-section adoption after Stop/revocation, idempotent adoption, explicit retention preserving drafts, abandoned-worker recovery and reader state across restart. They supplement the earlier owner/socket/source/visual/tampering tests; they do not prove live answer meaning, two-minute usefulness, VoiceOver behavior or audio quality.

A separate opt-in synthetic account probe requires the rebuilt installed helper and a fresh absolute artifact path. It records durable section notices, local chapter adoption, first ready section timing, terminal result and original native records. It neither runs in the ordinary suite nor overwrites earlier artifacts.

## Remaining acceptance

The replacement native app must be packaged, integrity checked and opened. The six-tool helper must be qualified with the actual installed Codex account. Then run fresh frozen Fontdone attached-function and broad-repository checks, review claim meaning/diagrams and actual Pocket narration, and inspect installed navigation, Stop/restart, keyboard/focus and VoiceOver behavior. The first useful independently published section must meet the recorded 120-second target. Historical Fontdone failures remain failed; no old candidate has been salvaged or silently replayed. No task is closed by this source checkpoint.

## Installed helper and first live synthetic result

The `2c7dda3` unsigned debug app was built, checked with the executable-integrity helper and installed at `/Users/lazytrot/Applications/Code Atlas.app`. Its executable SHA-256 was `4e004e04908c4ba4d1a9be9bada400eb528ddbea1bfe7522ad776ca8fd606b3c`. A recoverable previous app copy was retained. Computer use confirmed that the installed app opened the saved Fontdone nested exploration at section 2 of 3 and reconnected the existing Codex account without a setup prompt. This verifies the observed restoration only; it does not qualify progressive cards, VoiceOver or narration.

The first explicit public synthetic GPT-6 Luna/max run, operation `a9b5d9ce-094f-48a4-83f6-4d7990a6589c`, **failed its complete-answer assertion**. It independently adopted two validated section drafts at **26,211 ms** and **70,403 ms**, and terminated partial at **76,252 ms**. Both proposal attempts had no native findings. Their explanation correctly described the synthetic function's direct string interpolation and empty-string behavior; the first included a three-node/two-edge data-flow diagram. No audio was generated or assessed.

The first confirmed divergence was the final output contract: `generationId.maxLength` was 36, while native publication returned a 47-character `generation:`-prefixed UUID. The retained final response truncated the first ID and omitted the second accepted section, so exact native manifest validation rejected completion. We cannot infer that the length constraint caused the omission, but it necessarily prevented a valid complete ID. Earlier drafts and their exact native identities survived. [Original failed artifact](artifacts/2026-10-17-progressive-synthetic-truncated-manifest.json), SHA-256 `a1b37fb64d7fa3eddb218d1bc5c5a5b25a1953d2cdab61ea963f16ae773e76c2`.

The output schema now permits the full 47-character native ID. A focused regression checks that bound against a native-format identity. This fix does not rewrite the failed output, weaken manifest equality, invent missing IDs or automatically replay the investigation. A fresh, separately recorded qualification is required.

The corrected checkout passes the complete native suite: **335 passed, 12 explicit probes ignored**, 36.29 s (`/private/tmp/code-atlas-progressive-manifest-native-full.log`). The debug app build passed with the existing 13 unused-code warnings and frontend chunk warning (`/private/tmp/code-atlas-progressive-manifest-app-build.log`). Its installed executable passes the same integrity check; SHA-256 is `282d58972c54ea8b58c650ea9e16e82633d58968d7d24c473feb7c38a5b23184`. This remains an unsigned debug installation, not release/signing acceptance.
