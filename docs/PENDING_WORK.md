# Pending work after the October 15 static review

> **Latest workspace redesign — 16 October:** [contract and preview](design/WORKSPACE_EXPLORATION.md); Kata `nnsf`, `eapn`, `nsfd`, `nw1h`, `mrpj`, `hstr`, `3rcd`. The current-level chat/explorer/Guide/Flow/history design supersedes prior navigation proposals. The global notification bell is implemented in source; native shell migration and background generation remain pending.

> **Current flow update — October 16:** Codex chooses implementation from the captured index; connection/model/reasoning live in the header. Route chapters can be built sequentially after one exact batch review. Code is the evidence and docs are hints, with explicit deprecated labels. Light/dark/system palettes replace the lime styling. See [AI guide flow](AI_GUIDE_FLOW.md), discovery task `xvjk` and batch qualification `a1sm`. Broad native/language/large-repo acceptance remains open. Older entries below are historical checkpoints.

> **Connection reuse — October 16:** generation now restores the saved Codex account without entering sign-in on every request. Connected setup controls stay collapsed, model/reasoning remain shared, and explicit Disconnect pauses reuse across restarts. See [focused evidence](acceptance/2026-10-16-codex-connection.md); the wider Codex and product gates remain open.

> **Workspace update — October 16:** the native app now uses explorer / source / explanation panes, direct Codex entry with remembered settings, focused Flow and Player actions, and a pinned section-navigation row. See [native evidence and remaining design gaps](acceptance/2026-10-16-code-workspace.md). Full Changes composition, first-run polish, generated visual quality and accessibility remain open in `13yp`/`x21h`; question-to-source discovery is explicitly defined in `xvjk`.

> **October 16 update:** installed-account Codex file generation now has a native baseline: automatic discovery, source review/consent, validated chapter, draft/acceptance, verified citation and manual section advance. [Evidence and remaining gates](acceptance/2026-10-16-codex-validation.md). Full operation/reliability/quality acceptance remains open; Claude is deferred. Historical source-only and protocol-blocker entries below describe earlier checkpoints.

Kata project: `code-atlas-rebuild`. This list records **32 tracked issues: 18 open, 13 blocked, and 1 closed; 14 new detailed tasks and 18 existing tasks updated with remaining work**. Parser issue 129x closed after regression evidence on 2026-10-15. Each linked specification includes the intended behavior, acceptance criteria and closure evidence. Issue IDs are recorded in [issue-refs.json](kata-tasks/issue-refs.json). The bounded-metadata issue `k9yd` was closed after implementation and Rust-suite evidence on 2026-10-15.

The initial static-review pass intentionally did not run tests, builds, app launches, or runtime probes, as requested. That statement applies only to that pass; later verification is recorded below and in the relevant issue specifications. Source inspection alone cannot establish that every bug is fixed.

## Changes made by code inspection

- Snapshot capture now respects Git ignore negations and rechecks earlier captured file fingerprints before finalization.
- Index, recovery-journal, source-object and narration-cache reads apply bounds during reading. Recents, snapshot catalog/manifest and existing-object metadata bounds are implemented in commit `0a5e32e`; `k9yd` is closed with Rust-suite evidence.
- Direct-model and narration clients refuse redirects beyond the exact reviewed endpoint.
- Walkthrough and Git Changes ignore superseded async results; provider settings prevent Keychain results being applied to another provider.
- Chapter Studio retains history for files no longer captured, locks editing/selection during save or validation, filters unrelated save events, and avoids route updates after unmount.
- PR Guide locks context during decisions, scopes credential callbacks and discards late abandoned review builds.
- Narration cancels work on section unmount, discards late results/reviews and keeps the active request controls stable while synthesis runs.

These are implemented changes in commit `0494bd4`, awaiting acceptance in `xymb`.

The subsequent product review identified three core gaps: chapters currently contain no generated visuals; repository-wide narrative quality is unqualified; and structural language support is limited to Rust, TypeScript/TSX and JavaScript. See [the current agent evaluation](research/2026-10-15-agent-backends.md).

## Subsequent implementation update — 2026-10-15

The current source also contains the following in-progress implementation changes. They are not covered by the historical commit's acceptance evidence, and their related tasks remain open.

- **Codex (`z3an`):** native managed ChatGPT login, a Rust-managed app-server child over stdio, explicit runtime model/reasoning selection, reviewed file chapters or 1–12-file overviews, cancellation/progress, and shared validation into unaccepted drafts. This is a local-app integration, not direct Sign in with ChatGPT. Runtime `model/list` is a catalog, not entitlement proof. Runtime version pinning, account/distribution qualification, license, packaging, boundary verification, and real-run acceptance remain open; one-section stale repairs and exact Code Lens ranges are wired in source, and PR Guide/hunk dispatch are now wired in source; exact Git/range recovery remains pending. Threads are ephemeral with no persisted resume.
- **Pocket runtime (`q6g3`):** managed Python/venv/assets under app data, pinned model and Alba voice, setup/repair/status, bounded offline worker synthesis, and cancellation/exit cleanup are implemented in source. Apple Silicon only; `uv` is still an external prerequisite. Workers reload per segment. Installer packaging/signing, transitive artifact hashes, requirements, and clean-account/offline/process acceptance remain open.
- **Pocket player/cache (`dbmd`):** new preferences default local; prior Google preferences/cache remain distinct. Exact script review, local setup without a key/cloud consent, runtime-aware cache identity, guarded async lifecycle, and backend-aware cache lookup for export are wired. Listening, playback/cache/navigation, corruption and offline-export acceptance remain open.
- **Index v2 (`129x`, closed):** lexical bindings, shadow/reassignment uncertainty, direct module export checks, duplicate declaration occurrences, and a new semantics/cache version are implemented and covered by cross-language and base/head regressions. Restricted Rust visibility such as `pub(crate)` no longer counts as a crate-root export. Inline Rust module exports remain unsupported, intentionally avoiding claims without module visibility analysis. Broader language qualification remains open in `7xx7`.
- **Git lifecycle (`kghr`):** the earlier bounded runner was committed in `0a5e32e`. A later source review found unbounded worker joins after child exit. The current Unix source replaces those workers with one nonblocking pipe supervisor whose original deadline/cancellation checks remain active through output EOF; non-Unix hosts fail before launch. Historical unit evidence does not cover this replacement. Current IO/lifecycle, native cancellation/no-orphan acceptance and the remaining call-site audit are pending.
- **Design parity (`13yp`, gap ledger `x21h`):** the Guide now offers an exact-context-reviewed optional tour when no accepted tour exists for the selected snapshot, keeps per-file explanation access beside that action, surfaces current-snapshot drafts for review, and labels older-snapshot guides as historical. This is source-only progress; packaged screenshots and native build identity are still missing.

## Agent support takes priority — 2026-10-15

Claude source groundwork now includes documented CLI invocation preparation, environment-cleared command construction, bounded structured-output consumption, shared chapter/section review/revalidation and a native process supervisor. The supervisor adds private staging, executable digest reinspection, nonblocking pipes, progress, cancellation/deadlines and owned-child cleanup; uncertain cleanup disables later runs for the app session. An explicit native setup check and UI now inspect the selected executable/version and sanitized reported account metadata, with bounded output, cancellation and background updating disabled. They send no prompt and always leave generation unavailable. Model discovery, runtime/account/isolation qualification, generation workflow acceptance, persisted recovery, operation completion and supervisor acceptance remain in priority-0 `8gqw`. Deferred cases are recorded in `xymb`; no tests, builds, agent executions or logins ran for this change. The manual handoff's mode type was also restricted to its supported chapter/overview operations after the Codex range addition.

The [required operation parity contract](AGENT_OPERATION_PARITY.md) now names every Codex/Claude workflow, current automatic-adapter gap, common settings/review/lifecycle behavior and source reuse point. Kata `2r6n` is explicitly blocked by Codex `z3an` and Claude Agent `8gqw`; requested Antigravity `m00g` separately blocks M7. Implementing APIs, manual handoffs or catalog labels cannot close those gates. Automatic Claude implementation remains priority 0; no new automatic backend was delivered by this contract update.

Candidate Claude catalog delta: inspected the official TypeScript SDK `0.3.300` artifact and recorded its integrity/interface provenance. A discovery library uses the SDK's public initialization/model methods with zero user messages; native catalog admission independently bounds metadata, resolves canonical IDs, coalesces consistent aliases and rejects invalid effort capabilities. A native manager path now verifies/stages the SDK, rechecks executables/reported account metadata and supervises the SDK host under the existing operation lock/deadline. Its wrapper bounds raw vendor pipes and independently requires child completion/EOF. No SDK dependency was installed or run. A registered native catalog command and advanced setup UI now call this path explicitly, accepting installation path hints and deriving executable/SDK identities natively. They observe the Node engine version and display candidate model/effort metadata without persisting account/catalog observations. Qualified SDK/host packaging, accepted discovery and every automatic Claude generation operation remain open in `8gqw`; metadata wiring is not an accepted agent integration. See [the task's exact remaining order](kata-tasks/41-claude-agent-backend.md).

Claude generation source delta: an internal native file/overview/section pipeline now consumes an exact review, uses its effective settings, calls the managed CLI worker and reuses the existing independent validators for an unaccepted preview. The worker checks reported account/executable metadata before source delivery and after success, with locked source preflight and metadata deadlines constrained by the same run budget. Claude interruption metadata preserves the original backend/scope, and the current UI rejects unsupported agent recovery rather than opening an API request. Registered native send commands now require a release-owned admission type, but its profile registry is empty and app generation controls have source wiring but no runtime acceptance; every automatic Claude operation remains Missing in `8gqw`/`2r6n`.

Subsequent source delta: Codex now accepts one-section stale repair requests using shared API/agent context building and the existing preservation validator. The review binds the accepted base and section revision, source selection is fixed, old/current source bytes are bounded and old source is screened for secret-like content. PR Guide/hunk dispatch is now present in source; native runtime acceptance and exact Git interruption recovery remain pending. Code Lens now uses transport-independent immutable range context, native Codex review/revalidation and managed dispatch, then the same independent exact-quote validator as direct APIs. It stages only the excerpt and returns a temporary explanation without touching chapter drafts or history. Model/effort/approval, progress and cancellation share the Codex dialog; range recovery still requires deliberate reselection. Static review also fixed the manual handoff's response-envelope field in `0d1160b`; no tests or builds ran.

The user has reaffirmed that Codex and Claude Agent generation are core requirements and direct API keys must be optional. The [agent-first release gate](kata-tasks/45-agent-first-generation.md) is priority 0 and blocks the backend milestone. Complete those integrated paths before further visual/narrative polish. Existing per-backend tasks remain open.

Source progress in this change: every new generation task shows the backend chooser; Codex returns to it via Change backend. Manual task copy/import for Claude Code, Codex, Antigravity or another agent now shares bounded immutable-source review and native chapter/evidence validation. Imported agent/model provenance and runtime controls are unverified. This interim manual transport does **not** satisfy automatic multi-agent support. No test, build, agent process, login or provider request was run.

The previously started repository reading planner is also preserved in source: bounded static-call steps, saved local reading progress, exact source/call links and one-file generation context through the same chooser. It does not qualify complete repository narratives and remains secondary to the agent work.

## Recommended implementation order

1. Finish integrated Codex (`z3an`) and Claude Agent (`8gqw`) generation and the agent-first gate before further visual/narrative polish (`j53m`, `6qk9`). Resolve each runtime/version, account/distribution path, supported configuration and boundary qualification. Antigravity (`m00g`) needs its separately permitted adapter. Manual copy/import cannot close these gates; no agent-selected operation may silently use a direct API.
2. Index-v2 targeted correctness (`129x`) is verified and closed. Qualify broader language support (`7xx7`) before advertising structural capabilities. Complete the remaining native Git lifecycle acceptance for `kghr`; metadata bounds (`k9yd`) are complete.
3. Finish Pocket's packaged setup and whole-section scheduling (`q6g3`), setup/help/run-sheet documentation and player/cache/export qualification (`dbmd`). The app-managed adapter exists in source; earlier standalone CLI audio is not its acceptance evidence.
4. Complete PR generation/navigation recovery `qjm3`. Keep the exact immutable comparison attached to every operation.
5. When testing is authorized, execute `xymb`, then the existing quality, native accessibility, selective-update, revision and offline-export acceptance gates.
6. Finish signing/notarization and clean-account release acceptance. The initial Codex controls are selectable in the source app; do not advertise qualified backend support until its exact version, account route, configuration and permission boundaries have the required evidence.

## Detailed task index

All items below remain open. “Waiting on” lists open dependency edges; child acceptance is also required before a parent closes. Existing release sequencing is preserved; agent implementation no longer depends on completing the direct API backlog.

| Kata ID | Task specification | Waiting on |
| --- | --- | --- |
| `2r6n` | [Make Codex and Claude Agent complete generation choices without mandatory API keys](kata-tasks/45-agent-first-generation.md) | Parent `tpta`; blocks `mfz2`; acceptance pending |
| `z3an` | [Implement Codex app-server chapter generation with account sign-in](kata-tasks/40-codex-account-backend.md) | No open dependency edge |
| `j53m` | [Generate source-grounded visual explanations from real chapters](kata-tasks/37-generated-visuals.md) | No open dependency edge |
| `6qk9` | [Build coherent repository narratives with file-level drill-down](kata-tasks/38-repository-narratives.md) | No open dependency edge |
| `7xx7` | [Define and qualify language support for code explanations](kata-tasks/39-language-capability-tiers.md) | No open dependency edge |
| `8gqw` | [Implement Claude Agent chapter adapter and resolve account eligibility](kata-tasks/41-claude-agent-backend.md) | No open dependency edge |
| `m00g` | [Qualify Antigravity account integration and implement the supported adapter](kata-tasks/42-antigravity-agent-backend.md) | No open dependency edge |
| `13yp` | [Make the packaged app follow the approved product designs](kata-tasks/43-design-implementation-parity.md) | Parent `s0tj`; related to `48yv` |
| `x21h` | [Record concrete UI gaps against the approved designs](kata-tasks/44-design-gap-audit.md) | Parent `s0tj`; related to `13yp` |
| `kghr` | [Bound and cancel repository Git subprocesses](kata-tasks/32-bounded-git-lifecycle.md) | No open dependency edge |
| `q6g3` | [Integrate one managed local Pocket TTS runtime](kata-tasks/34-pocket-runtime.md) | No open dependency edge |
| `dbmd` | [Make Pocket TTS the default narration path with distinct cache identity](kata-tasks/35-pocket-player-cache.md) | `q6g3` |
| `qjm3` | [Define recoverable PR generation and decisions across navigation](kata-tasks/36-pr-navigation-lifecycle.md) | No open dependency edge |
| `xymb` | [Validate the October 15 static bug fixes when testing is authorized](kata-tasks/30-static-fix-acceptance.md) | No open dependency edge |
| `q0sm` | [Establish a repo-level chapter quality benchmark and review gate](kata-tasks/14-chapter-quality.md) | No open dependency edge |
| `70x5` | [Add local Pocket TTS narration and distinct cached audio](kata-tasks/18-optional-narration.md) | No open dependency edge |
| `tbsh` | [Edit or regenerate one selected stale section and reuse unaffected assets](kata-tasks/20-selective-section-update.md) | `70x5` |
| `gace` | [Compare immutable guide revisions and restore as a new draft](kata-tasks/21-revision-history.md) | `tbsh` |
| `8r4j` | [Export an accepted interactive guide for offline reading](kata-tasks/22-offline-export.md) | `gace` |
| `bcn2` | [Meet accessibility, responsiveness, and repository safety requirements](kata-tasks/24-accessibility-performance.md) | No open dependency edge |
| `e42a` | [Package, sign, notarize, and document the first Mac release](kata-tasks/25-macos-packaging.md) | `8r4j`, `bcn2` |
| `tbng` | [Build bounded read-only evidence for local Git base and head refs](kata-tasks/26-pr-diff-evidence.md) | `e42a` |
| `a9v9` | [Generate focused, reviewed explanations for a local change set](kata-tasks/27-pr-guide.md) | `tbng` |
| `c8nz` | [Add selectable direct model API providers](kata-tasks/28-model-api-providers.md) | `tbsh`, `a9v9` |
| `tpta` | [Add selectable read-only coding-agent backends](kata-tasks/29-agent-runtime-backends.md) | No open dependency edge |
| `wzce` | [M2 — Explain every eligible file](kata-tasks/phase-2.md) | No open dependency edge |
| `gtte` | [M3 — Interactive explainer](kata-tasks/phase-3.md) | `wzce` |
| `b4cp` | [M4 — Git-like guide revisions](kata-tasks/phase-4.md) | `gtte` |
| `48yv` | [M5 — Dependable first Mac release](kata-tasks/phase-5.md) | `b4cp` |
| `hsn3` | [M6 — Local PR Guide follow-up](kata-tasks/phase-6.md) | `48yv` |
| `mfz2` | [M7 — Selectable AI backends](kata-tasks/phase-7.md) | `hsn3` |
| `s0tj` | [Code Atlas — greenfield product contract](kata-tasks/00-contract.md) | No open dependency edge |

## Working with the list

From this checkout, use `kata show code-atlas-rebuild#<id>` to read a full task and `kata ready --project code-atlas-rebuild` to see currently unblocked work. Canonical Markdown specifications are kept alongside Kata. `k9yd` was closed after passing `cargo test --manifest-path src-tauri/Cargo.toml --lib` (177 passed, 0 failed, 1 ignored). No other issue was closed during this pass. A generated audio sample, passing schema check, or static code review does not satisfy the human listening, source-quality, native accessibility or release gates.

Claude connection/review source delta: `claude_connections.rs` now retains one native observed catalog connection for five minutes, with supersession/cancel/compare-and-forget handling. A registered offline chapter-review command uses its opaque ID, native capabilities and requested settings/source; it binds the ID into the review fingerprint and caps review expiry. The setup UI invalidates its record on edits/close. The app now calls local review through supported Claude settings, captured-source selection and exact request display. No qualified runtime owner or inference command/UI was delivered. Automatic Claude support, disconnect-aware active-run/result handling, recovery and acceptance remain open in `8gqw`/`2r6n`; no tests, builds or runtime requests ran.

Source-scope correction: Codex initialization/reset, manual handoff initialization and direct API file initialization now retain supplied selections above 12 paths instead of silently dropping the tail. The supported limit still requires explicit reduction before a valid review; fixed section scopes cannot be truncated to bypass preservation. Cases remain pending in `xymb`, and Codex acceptance remains open in `z3an`.

Codex Git-change delta: the PR screen now defaults to Codex and exposes direct APIs only as an optional backend. Shared immutable Git builders, four registered native review/send commands, native contract revalidation, neutral patch staging, progress/cancel controls and citation navigation connect the existing runtime to PR Guides and single hunks. Valid guides are saved as separate unaccepted PR drafts and trigger history reload even if a reply is lost; hunk output stays transient. A Codex worker panic now retains its recovery notice. Claude Agent remains a required pending automatic integration, not an API fallback or completed setup option. `z3an`, `8gqw`, `2r6n`, `tpta`, `xymb` and M7 remain open; no tests, builds or agent calls ran.

Claude range/Git source delta: candidate pure context/review/revalidation and internal worker/result paths now cover exact ranges, PR Guides and single hunks. Three registered native offline-review commands share native connection ownership and post-preparation expiry checks; new frontend helpers now have app-connected local preview callers for exact range, guide and hunk scopes. Internal senders retain explicit consent, exact operation guards, account pre/post inspection, local citation validation, transient range/hunk results and separate unaccepted PR drafts. Blocking result preparation panics retain recovery state. Source alone cannot qualify the runtime/account/isolation owner or enable inference; native send commands are registered behind an empty release qualification registry, app generation UI has source wiring but no runtime acceptance and every automatic Claude cell stays Missing. These gates remain priority 0, with no direct API fallback. No tests, builds, app/SDK/CLI probes or model calls ran.


## Claude automatic app controls — source progress, still unavailable

The six scoped local reviews now mount automatic generation controls linked to the four native send commands. Explicit review consent, progress/cancel, fixed per-run source/settings, chapter preview/save/discard, transient range/hunk citations and separate guide draft adoption are in source. Mounted-view/run checks dispose abandoned chapter replies, reject unwanted transient replies and refresh independently recoverable chapter/guide lists. A five-minute connection expiry cannot erase a consented bounded run or its saveable result. Lost save replies reconcile exact immutable revision identity.

The release qualification registry remains empty, so no current route can generate automatically with Claude. No tests, builds, app launch or model request ran. Account/runtime/installation qualification, exact Claude interruption recovery acceptance and real native user workflow evidence remain priority-zero work in `8gqw`/`2r6n`; automatic Claude cells stay Missing. This source checkpoint does not complete the user's multi-agent requirement.


## Both-agent exact recovery — source progress

Claude and Codex now share native journal-owned chapter/range/Git evidence recovery and a backend-tagged Git scope. Claude records reopen the original agent, exact chapter/section/range/comparison/path-pair/hunk and a fresh review. Section recovery uses project-wide accepted history. Chapter recovery lists possible pending/saved results before another request; guide recovery inspects separate PR history. No prior runtime/settings authorization or consent resumes. Preparation, failure, cancellation, closing and unavailable runtime preserve the original notice; validated wanted result adoption resolves it. API preview creation cannot resolve an agent notice.

This source has not been built or exercised. Claude's release registry is still empty, all automatic Claude cells remain Missing, and native runtime/account/boundary/recovery acceptance remains open. Deferred cases are in `xymb`.

- `na54`: complete packaged qualification of the activity-bar workspace, visible answer stacks and nested exploration; follow the detailed all-page audit in `docs/kata-tasks/64-workspace-ui-audit.md`.
