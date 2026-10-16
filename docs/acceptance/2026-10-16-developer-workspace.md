# Developer workspace migration — October 16, 2026

## Scope
This record covers the production conversational workspace migration, not a production release. The earlier installed Guide / Files / Changes app and its screenshots predate this migration. The browser `workspace-preview.html` remains an illustrative design, never a source of production answers.

## Implemented source surfaces
- Production `WorkspaceFrame` / `DeveloperWorkspace`: left Explorer, Search, Changes and Explorations; one pinned composer; real immutable source and Guide / Flow / history. Shared Radix controls, CodeMirror and resizable panels; narrow windows switch readable panes.
- Flat exploration records: follow-ups stack at the active level; explicit child links/counts attach to their originating turn; parent/root navigation and per-exploration last view. Versioned WebView metadata references native revisions.
- Module-owned question operation: immutable indexing, automatic source discovery, exact-question generation, unaccepted native draft save, notification action and explicit stop. Branch topic and recent same-level questions inform follow-ups. Navigation does not own or cancel the operation. Restart interrupts without an automatic resend.
- Native revocable permission binds project/snapshot manifest/account/runtime; actual source-discovery and chapter dispatch recheck it. A changed question/focus changes the generation fingerprint. Advanced PR/editor flows retain explicit scoped reviews.
- Captured-source search uses local bounded reads (24 MB, two seconds, 80 matches); typing never calls AI. Git history reads at most 80 local commits with bounded subprocesses and opens a first-parent comparison. It is a commit list, not a complete DAG; root commits currently compare with themselves.
- Contextual revision editor, interactive player, typed evidence-linked visuals, exact citations and optional Pocket narration reuse existing native stores and validators. No sample checkout data enters production source.

## Checks executed

| Check | Result |
| --- | --- |
| `npm test` | 126 passed, zero failed |
| `RUSTC_WRAPPER= CARGO_BUILD_JOBS=4 CARGO_INCREMENTAL=0 cargo test --release --manifest-path src-tauri/Cargo.toml --lib` | 198 passed, zero failed, three ignored live checks |
| `npm run build` | Passed; large frontend chunk warning remains |
| `git diff --check` | Passed before source commit |
| Open Source documentation inventory | No broken-local-link findings; root license and three broad-claim review notes remain |

Focused tests cover native permission scope/revocation and dispatch fingerprinting, captured-text search, stacked turns, explicit nesting, last view, interrupted reload, corrupted graph/state fallback, implementation entry ranking and bounded follow-up intent. They do not establish visual/narrative quality.

## Browser observations
In the separate design preview, navigated root → child → grandchild. Back restored the parent's Guide view and reopening the grandchild restored Flow. Follow-up cards and originating-turn child counts are visible. A narrow browser panel initially squeezed code and explanation together; fixed responsive panel visibility and verified readable explanation/source switching. These observations do not qualify native generation or the packaged app.

## Native installation / qualification
The native unsigned app/DMG build is being prepared from the recorded source revision. The Mac is locked and computer use cannot unlock it. Therefore the new bundle has not been opened or visually checked, and no new native question has been submitted in this migration. Earlier installed-account Codex and Pocket baselines remain historical evidence only. Bundle identity and staging details are recorded below after packaging.

## Work still open
Kata `na54`, `3rcd`, `13yp`, `x21h` and the seven workspace tasks remain open. Required next evidence: replacement launch; real source and saved answers; synthetic-only GPT-6 Luna/max question through the new permission flow; nested live answers and restoration; Search/Changes/editor/player; keyboard/drag and packaged accessibility across themes/window sizes. Durable native job scheduling, complete section/playback/graph viewport persistence, root-commit diff and quality/language/release gates remain open. No production-readiness claim is made.
