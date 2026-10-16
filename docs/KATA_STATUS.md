# Kata delivery status snapshot

> **AI guide flow — October 16:** indexed Codex source discovery, header connection details, code-first evidence rules, light/dark themes and sequential route chapter drafts are implemented. Broader acceptance remains in `xvjk` and new task `a1sm`. See [the current flow](AI_GUIDE_FLOW.md).

> **Connection reuse — October 16:** generation now restores the saved Codex account without entering sign-in on every request. Connected setup controls stay collapsed, model/reasoning remain shared, and explicit Disconnect pauses reuse across restarts. See [focused evidence](acceptance/2026-10-16-codex-connection.md); the wider Codex and product gates remain open.

> **October 16 update:** installed-account Codex file generation now has a native baseline: automatic discovery, source review/consent, validated chapter, draft/acceptance, verified citation and manual section advance. [Evidence and remaining gates](acceptance/2026-10-16-codex-validation.md). Full operation/reliability/quality acceptance remains open; Claude is deferred. Historical source-only and protocol-blocker entries below describe earlier checkpoints.

Project: **code-atlas-rebuild**. Recorded October 16, 2026, during Codex validation and [workspace refinement](acceptance/2026-10-16-code-workspace.md).

**64 tasks: 43 open, 21 closed.** Every task title maps to exactly one checked-in definition. This is a dated Kata status snapshot; a closed historical task does not prove that the current app, new schema or agent runtime passed acceptance. No task was closed by this audit.

The full requested scope remains in [the backlog](KATA_BACKLOG.md). Codex is the first delivery gate; APIs and manual handoffs cannot substitute. The user deferred Claude, which remains unavailable behind an empty qualification registry and still gates full M7. Building/testing and a small synthetic Codex account validation were authorized on October 16; dated results are recorded in the validation task. Complete operation/runtime and distribution evidence remains pending.

## Latest workspace tasks

The latest user contract supersedes the earlier primary navigation. The [production workspace migration](design/WORKSPACE_EXPLORATION.md) now implements activity navigation, top chat, stacked answers, explicit branches, automatic Codex source discovery, native scoped permission, captured-source search and local commit comparison. A module-owned job continues across navigation; native durable scheduling, complete reading-position recovery and packaged visual/accessibility qualification remain open. Global outcomes stay behind the notification bell.

| Ref | Status | Definition |
| --- | --- | --- |
| `nnsf` | open | [Build the shared developer workspace with standard components](kata-tasks/57-persistent-code-workspace.md) |
| `eapn` | open | [Persist exploration branches, answers and last-view state](kata-tasks/58-exploration-history.md) |
| `nsfd` | open | [Unify chat, parallel code search and exact source context](kata-tasks/59-intent-context-search.md) |
| `nw1h` | open | [Run code discovery and explanation as owned background jobs](kata-tasks/60-background-generation.md) |
| `mrpj` | open | [Compose Guide and Flow from shared answers with inline code details](kata-tasks/61-guide-flow-detail.md) |
| `hstr` | open | [Collect background outcomes and recovery behind a notification bell](kata-tasks/62-notification-center.md) |
| `3rcd` | open | [Qualify the packaged conversation and code workspace against its design](kata-tasks/63-workspace-design-acceptance.md) |
| `na54` | open | [Replace inconsistent pages with the conversational developer workspace](kata-tasks/64-workspace-ui-audit.md) |

## All earlier tasks

Open blockers are recorded dependency edges, not a complete list of human/vendor/runtime requirements. A historical closed record may have a reopened prerequisite; do not infer current readiness transitively. Consult the task definition and current acceptance evidence before release.

| Ref | Kata status | Task definition | Open recorded prerequisites |
|---|---|---|---|
| `2r6n` | open | [Make Codex a complete generation choice without mandatory API keys](kata-tasks/45-agent-first-generation.md) | `z3an` |
| `6qk9` | open | [Build coherent repository narratives with file-level drill-down](kata-tasks/38-repository-narratives.md) | — |
| `8bw6` | open | [Define versioned chapter, section, claim, anchor, and hotspot records](kata-tasks/11-chapter-schema.md) | — |
| `8gqw` | open | [Implement automatic Claude Agent generation across required operations](kata-tasks/41-claude-agent-backend.md) | — |
| `j53m` | open | [Generate source-grounded visual explanations from real chapters](kata-tasks/37-generated-visuals.md) | — |
| `mfz2` | open | [M7 — Selectable AI backends](kata-tasks/phase-7.md) | `hsn3`, `m00g`, `2r6n`, `8gqw` |
| `tbsh` | open | [Edit or regenerate one selected stale section and reuse unaffected assets](kata-tasks/20-selective-section-update.md) | `70x5` |
| `tpta` | open | [Add selectable read-only coding-agent backends](kata-tasks/29-agent-runtime-backends.md) | — |
| `z3an` | open | [Implement Codex app-server chapter generation with account sign-in](kata-tasks/40-codex-account-backend.md) | — |
| `a1sm` | open | [Qualify automatic chapter building for an accepted learning route](kata-tasks/56-route-chapter-batch.md) | — |
| `xvjk` | open | [Find relevant code from a question without manual context hunting](kata-tasks/55-question-to-source-discovery.md) | — |
| `13yp` | open | [Make the packaged app follow the approved product designs](kata-tasks/43-design-implementation-parity.md) | — |
| `7xx7` | open | [Define and qualify language support for code explanations](kata-tasks/39-language-capability-tiers.md) | — |
| `bcn2` | open | [Meet accessibility, responsiveness, and repository safety requirements](kata-tasks/24-accessibility-performance.md) | — |
| `c8nz` | open | [Add selectable direct model API providers](kata-tasks/28-model-api-providers.md) | `tbsh`, `a9v9` |
| `dbmd` | open | [Make Pocket TTS the default narration path with distinct cache identity](kata-tasks/35-pocket-player-cache.md) | `q6g3` |
| `e42a` | open | [Package, sign, notarize, and document the first Mac release](kata-tasks/25-macos-packaging.md) | `8r4j`, `bcn2` |
| `gace` | open | [Compare immutable guide revisions and restore as a new draft](kata-tasks/21-revision-history.md) | `tbsh` |
| `kghr` | open | [Bound and cancel repository Git subprocesses](kata-tasks/32-bounded-git-lifecycle.md) | — |
| `m00g` | open | [Qualify Antigravity account integration and implement the supported adapter](kata-tasks/42-antigravity-agent-backend.md) | — |
| `q0sm` | open | [Establish a repo-level chapter quality benchmark and review gate](kata-tasks/14-chapter-quality.md) | — |
| `q6g3` | open | [Integrate one managed local Pocket TTS runtime](kata-tasks/34-pocket-runtime.md) | — |
| `tbng` | open | [Build bounded read-only evidence for local Git base and head refs](kata-tasks/26-pr-diff-evidence.md) | `e42a` |
| `x21h` | open | [Record concrete UI gaps against the approved designs](kata-tasks/44-design-gap-audit.md) | — |
| `xymb` | open | [Validate the October 15 static bug fixes when testing is authorized](kata-tasks/30-static-fix-acceptance.md) | — |
| `70x5` | open | [Add local Pocket TTS narration and distinct cached audio](kata-tasks/18-optional-narration.md) | — |
| `8r4j` | open | [Export an accepted interactive guide for offline reading](kata-tasks/22-offline-export.md) | `gace` |
| `a9v9` | open | [Generate focused, reviewed explanations for a local change set](kata-tasks/27-pr-guide.md) | `8bw6`, `tbng` |
| `qjm3` | open | [Define recoverable PR generation and decisions across navigation](kata-tasks/36-pr-navigation-lifecycle.md) | — |
| `48yv` | open | [M5 — Dependable first Mac release](kata-tasks/phase-5.md) | `b4cp` |
| `b4cp` | open | [M4 — Git-like guide revisions](kata-tasks/phase-4.md) | `gtte` |
| `gtte` | open | [M3 — Interactive explainer](kata-tasks/phase-3.md) | `wzce` |
| `hsn3` | open | [M6 — Local PR Guide follow-up](kata-tasks/phase-6.md) | `48yv` |
| `s0tj` | open | [Code Atlas — greenfield product contract](kata-tasks/00-contract.md) | — |
| `wzce` | open | [M2 — Explain every eligible file](kata-tasks/phase-2.md) | — |
| `23eb` | closed | [Open a local repository and create a Code Atlas project](kata-tasks/03-project-selection.md) | — |
| `8z8n` | closed | [Play accepted chapters as manually paced walkthrough sections](kata-tasks/16-walkthrough-player.md) | — |
| `a3xq` | closed | [Generate one substantive, source-grounded chapter per selected file](kata-tasks/12-file-chapter-generation.md) | `8bw6` |
| `h9sy` | closed | [Edit, validate, accept, or reject chapters in Chapter Studio](kata-tasks/13-chapter-studio.md) | — |
| `hjv4` | closed | [Capture immutable, bounded repository snapshots](kata-tasks/04-snapshot-capture.md) | — |
| `s4df` | closed | [Detect changed evidence and mark only dependent sections stale](kata-tasks/19-section-staleness.md) | `8bw6` |
| `t188` | closed | [Onboard users locally and recover from interrupted work](kata-tasks/23-onboarding-recovery.md) | — |
| `tsqk` | closed | [Track chapter coverage for every inventory path](kata-tasks/09-guide-coverage.md) | `8bw6` |
| `vpty` | closed | [Define the complete Code Atlas screen and interaction contract](kata-tasks/01-product-experience.md) | — |
| `wmaj` | closed | [Add optional Gemini with exact-context consent and billing disclosure](kata-tasks/10-gemini-request-boundary.md) | — |
| `129x` | closed | [Resolve calls and exports conservatively under lexical shadowing](kata-tasks/31-parser-binding-resolution.md) | — |
| `3g0a` | closed | [Inspect exact snapshot source and citations in Code Lens](kata-tasks/08-code-lens.md) | — |
| `4f44` | closed | [Render safe interactive diagrams with evidence-linked hotspots](kata-tasks/17-linked-visuals.md) | `8bw6` |
| `7te9` | closed | [Show every repository path and its capture disposition](kata-tasks/05-file-inventory.md) | — |
| `8kx6` | closed | [Browse repository structure and evidence in Atlas](kata-tasks/07-atlas.md) | — |
| `c11x` | closed | [Build a supported-language symbol and relationship index](kata-tasks/06-symbol-index.md) | — |
| `pn1n` | closed | [Build the shared Mac shell and typed application boundary](kata-tasks/02-desktop-shell.md) | — |
| `k9yd` | closed | [Bound remaining local metadata and existing-object reads](kata-tasks/33-bounded-metadata.md) | — |
| `bwdx` | closed | [Add an optional repository orientation chapter](kata-tasks/15-optional-overview.md) | — |
| `c92x` | closed | [M1 — Local repository truth](kata-tasks/phase-1.md) | — |
| `cnsm` | closed | [M0 — Coherent Mac product shell](kata-tasks/phase-0.md) | — |

## Remaining delivery distinctions

- **Agent implementation and qualification:** complete the required Codex/Claude workflows, trusted runtime installation, account route, effective settings and lifecycle evidence. Antigravity remains a separate unresolved integration; its SDK/API route cannot stand in for consumer-account support.
- **Missing product work:** whole-repository multi-unit narrative orchestration and dependency-based selective updates remain required. A bounded plan or single planned chapter does not complete that scope.
- **Native acceptance:** schema/history compatibility, all agent operations, grounded narratives/visuals, Pocket speech quality, accessibility, offline export, responsiveness and recovery require their actual acceptance evidence. Formatting, source inspection and old suite counts do not establish it.
- **Release:** supported macOS/architecture decisions, trusted runtime packaging, Developer ID/notarization credentials and clean-account installation remain required. Source code cannot manufacture those artifacts or credentials.

Refresh this snapshot from live Kata when status changes. Canonical task bodies and checked-in definitions must be reconciled without losing independently edited requirements or relabeling historical observations as current execution evidence.

## 16 October all-page UI migration

Task `na54` ([definition](kata-tasks/64-workspace-ui-audit.md)) records the concrete Files, Guide, navigation, conversation, search, changes and secondary-tool inconsistencies. It remains open until packaged evidence is complete. The activity bar and repository-backed conversation workspace replace the old normal reading path; the sample preview remains clearly separate.
