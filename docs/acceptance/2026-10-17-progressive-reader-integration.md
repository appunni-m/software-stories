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
