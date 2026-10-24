# Kata backlog — Code Atlas rebuild

> **Live snapshot — 24 October 2026:** Kata project `code-atlas-rebuild` has **79 records: 51 open, 28 closed**. The [current status table](KATA_STATUS.md) is reconciled with live Kata, including the new Code Lens reproduction issue (`mhqx`). Dated implementation notes below are historical checkpoints and do not replace current issue status or acceptance evidence.

> **October 16 update:** installed-account Codex file generation now has a native baseline: automatic discovery, source review/consent, validated chapter, draft/acceptance, verified citation and manual section advance. [Evidence and remaining gates](acceptance/2026-10-16-codex-validation.md). Full operation/reliability/quality acceptance remains open; Claude is deferred. Historical source-only and protocol-blocker entries below describe earlier checkpoints.

Workspace project: **code-atlas-rebuild**. Product plan: [PRODUCT_BLUEPRINT.md](PRODUCT_BLUEPRINT.md).
Technical plan: [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md). Original inventory generated on 2026-10-14; delivery order reconciled on 2026-10-16.
Live-state snapshot: [all task records and their recorded status](KATA_STATUS.md).
Kata issue bodies are synchronized from `docs/kata-tasks/*.md`; each issue states its user outcome, interaction contract, acceptance criteria, exclusions, and closure evidence.

**October 17 offline-story checkpoint:** `8r4j` now implements HTML compositions,
manual scene/source navigation and separately scoped cached audio. All four
synthetic source/audio packages pass link/hash checks; native accepted-revision
cache export, actual browser playback and human listening remain pending.
[Evidence and remaining work](acceptance/2026-10-17-offline-visual-scenes.md).
No task is closed; live Kata remains 73 records, 50 open and 23 closed.

**October 17 read-recovery checkpoint:** the broad Fontdone run is still
partial: three drafts, 387.523 seconds, eight proposals exhausted. Read batching
and scene feedback fixes pass 351 native and 179 frontend tests; the new unsigned
bundle awaits an unlocked Mac for replacement/native checks. The failed broad
completion and visual/narration weaknesses remain recorded in `3bz5`, `gk46`,
`q0sm` and `j53m`. [Evidence and remaining work](acceptance/2026-10-17-read-recovery-and-broad-answer.md).
No task is closed; live Kata remains 73 records, 50 open and 23 closed.

**Priority update — 2026-10-16:** Deliver Codex first through [agent-first generation](kata-tasks/45-agent-first-generation.md), Kata `2r6n` (priority 0). The user deferred Claude (`8gqw`, priority 2); it no longer blocks the current Codex gate, but remains a direct dependency of full M7. Building, testing and a small synthetic installed-account Codex validation are authorized. Manual handoffs and direct API results cannot qualify Codex. See [Agent generation](AGENT_GENERATION.md).

## Earlier short-reference qualification — 17 October

The fresh short-reference Fontdone run still fails broad completion: one draft,
160.010 s and an uncertain final read. Independent review finds missing branch
exceptions, an empty repeated-card visual and dense narration. The follow-up
preserves fixed private terminal error classifications, with 363 native/179
frontend tests; it does not infer the historical cause or qualify content.
[Exact failed result, source checks and remaining work](acceptance/2026-10-17-short-native-references.md).
`3bz5`, `yk6c`, `gk46`, `q0sm` and `j53m` retain the concrete pending work.
All 73 task bodies/statuses/priorities match live Kata; 50 open, 23 closed.

## Focused generation repair

Kata `tmck` tracks [identifier correction](kata-tasks/65-claim-alignment-correction.md): one fingerprinted correction in the same Codex conversation, unchanged source validation and readable persisted failure messages. Parent quality `q0sm` remains open.

## Large-repository generation checkpoint — 17 October

The [native-citation Fontdone qualification](acceptance/2026-10-17-native-citation-references.md) preserves a failed six-minute investigation, despite correct reference resolution. The complete current tracker has **73 records: 50 open, 23 closed**. No task was closed by this checkpoint.

- `yk6c` — [native evidence references](kata-tasks/72-agent-evidence-references.md): implemented and regression-tested; complete live answer/navigation/narration gates remain open.
- `3bz5` — [progressive answer sections](kata-tasks/73-progressive-answer-sections.md): native storage/ownership, planner-first incremental generation, one bounded slide per user-stoppable job, playback-paced refill, events, draft adoption and reader/restart/retention handling are implemented with regressions. The installed helper published two synthetic drafts, but the first live run failed completion at a mismatched generation-ID schema bound. That bound is fixed with a regression; a fresh fixed-build synthetic run completed with the first section ready in 18.913 s. The subsequent installed Fontdone attempt exposed a reader/native ID mismatch and imprecise key findings; these are fixed with real-notice restart regressions (336 native, 149 frontend tests). A fresh installed retry delivered two sections but finished partial; independent review found a BDF exception omission, inadequate diagram and narration gaps. Positional diagnostic and Flow selection fixes pass 337 native and 150 frontend regressions. The 16:15 app was stale and predated guide routing fixes; a fresh bundle was rebuilt and installed on 21 October. Project-purpose wording routes to the outline-and-slide planner. A later manual check exposed a negated-scope false positive; it is fixed with focused-versus-broad routing regressions. The current app was rebuilt and installed, and all 606 frontend tests pass. No retry was sent after the misrouted narrow request. Complete Fontdone/Pocket/native accessibility and useful-answer timing qualification remain pending. [Reader checkpoint](acceptance/2026-10-17-progressive-reader-integration.md).
- `ps02` — [generation timing](kata-tasks/71-generation-timing-traces.md): native call and coarse phase observations exist; the two-minute target and complete stage tracing remain open.
- `q0sm` — [content quality](kata-tasks/14-chapter-quality.md): retain the required-versus-optional table counterexample; exact citations are not semantic proof.

## Product contract

| Ref | Issue | Definition |
|---|---|---|
| s0tj | Code Atlas — greenfield product contract | [00-contract.md](kata-tasks/00-contract.md) |

## Milestones

| Key | Ref | Milestone | Gate definition |
|---|---|---|---|
| M0 | cnsm | M0 — Coherent Mac product shell | [phase-0.md](kata-tasks/phase-0.md) |
| M1 | c92x | M1 — Local repository truth | [phase-1.md](kata-tasks/phase-1.md) |
| M2 | wzce | M2 — Explain every eligible file | [phase-2.md](kata-tasks/phase-2.md) |
| M3 | gtte | M3 — Interactive explainer | [phase-3.md](kata-tasks/phase-3.md) |
| M4 | b4cp | M4 — Git-like guide revisions | [phase-4.md](kata-tasks/phase-4.md) |
| M5 | 48yv | M5 — Dependable first Mac release | [phase-5.md](kata-tasks/phase-5.md) |
| M6 | hsn3 | M6 — Local PR Guide follow-up | [phase-6.md](kata-tasks/phase-6.md) |
| M7 | mfz2 | M7 — Selectable AI backends | [phase-7.md](kata-tasks/phase-7.md) |

## Implementation tasks

| Phase | Ref | Task | Blocked by | Definition |
|---|---|---|---|---|
| M0 | vpty | Define the complete Code Atlas screen and interaction contract | — | [01-product-experience.md](kata-tasks/01-product-experience.md) |
| M0 | pn1n | Build the shared Mac shell and typed application boundary | vpty Define the complete Code Atlas screen and interaction contract | [02-desktop-shell.md](kata-tasks/02-desktop-shell.md) |
| M1 | 23eb | Open a local repository and create a Code Atlas project | pn1n Build the shared Mac shell and typed application boundary | [03-project-selection.md](kata-tasks/03-project-selection.md) |
| M1 | hjv4 | Capture immutable, bounded repository snapshots | 23eb Open a local repository and create a Code Atlas project | [04-snapshot-capture.md](kata-tasks/04-snapshot-capture.md) |
| M1 | 7te9 | Show every repository path and its capture disposition | hjv4 Capture immutable, bounded repository snapshots | [05-file-inventory.md](kata-tasks/05-file-inventory.md) |
| M1 | c11x | Build a supported-language symbol and relationship index | 7te9 Show every repository path and its capture disposition | [06-symbol-index.md](kata-tasks/06-symbol-index.md) |
| M1 | 8kx6 | Browse repository structure and evidence in Atlas | c11x Build a supported-language symbol and relationship index | [07-atlas.md](kata-tasks/07-atlas.md) |
| M1 | 3g0a | Inspect exact snapshot source and citations in Code Lens | 7te9 Show every repository path and its capture disposition<br>c11x Build a supported-language symbol and relationship index | [08-code-lens.md](kata-tasks/08-code-lens.md) |
| M1 | mhqx | Reproduce and repair the Code Lens click error | — | [79-code-lens-click-error.md](kata-tasks/79-code-lens-click-error.md) |
| M2 | tsqk | Track chapter coverage for every inventory path | 7te9 Show every repository path and its capture disposition<br>8bw6 Define versioned chapter, section, claim, anchor, and hotspot records | [09-guide-coverage.md](kata-tasks/09-guide-coverage.md) |
| M2 | wmaj | Add optional Gemini with exact-context consent and billing disclosure | pn1n Build the shared Mac shell and typed application boundary<br>7te9 Show every repository path and its capture disposition | [10-gemini-request-boundary.md](kata-tasks/10-gemini-request-boundary.md) |
| M2 | 8bw6 | Define versioned chapter, section, claim, anchor, and hotspot records | c11x Build a supported-language symbol and relationship index | [11-chapter-schema.md](kata-tasks/11-chapter-schema.md) |
| M2 | a3xq | Generate one substantive, source-grounded chapter per selected file | tsqk Track chapter coverage for every inventory path<br>wmaj Add optional Gemini with exact-context consent and billing disclosure<br>8bw6 Define versioned chapter, section, claim, anchor, and hotspot records | [12-file-chapter-generation.md](kata-tasks/12-file-chapter-generation.md) |
| M2 | h9sy | Edit, validate, accept, or reject chapters in Chapter Studio | a3xq Generate one substantive, source-grounded chapter per selected file | [13-chapter-studio.md](kata-tasks/13-chapter-studio.md) |
| M2 | q0sm | Establish a repo-level chapter quality benchmark and review gate | a3xq Generate one substantive, source-grounded chapter per selected file<br>h9sy Edit, validate, accept, or reject chapters in Chapter Studio | [14-chapter-quality.md](kata-tasks/14-chapter-quality.md) |
| M2 | bwdx | Add an optional repository orientation chapter | a3xq Generate one substantive, source-grounded chapter per selected file | [15-optional-overview.md](kata-tasks/15-optional-overview.md) |
| M3 | 8z8n | Play accepted chapters as manually paced walkthrough sections | h9sy Edit, validate, accept, or reject chapters in Chapter Studio | [16-walkthrough-player.md](kata-tasks/16-walkthrough-player.md) |
| M3 | 4f44 | Render safe interactive diagrams with evidence-linked hotspots | 8bw6 Define versioned chapter, section, claim, anchor, and hotspot records<br>8z8n Play accepted chapters as manually paced walkthrough sections | [17-linked-visuals.md](kata-tasks/17-linked-visuals.md) |
| M3 | 70x5 | Add optional natural-speech narration and cached TTS | wmaj Add optional Gemini with exact-context consent and billing disclosure<br>a3xq Generate one substantive, source-grounded chapter per selected file<br>8z8n Play accepted chapters as manually paced walkthrough sections | [18-optional-narration.md](kata-tasks/18-optional-narration.md) |
| M4 | s4df | Detect changed evidence and mark only dependent sections stale | hjv4 Capture immutable, bounded repository snapshots<br>c11x Build a supported-language symbol and relationship index<br>8bw6 Define versioned chapter, section, claim, anchor, and hotspot records<br>h9sy Edit, validate, accept, or reject chapters in Chapter Studio | [19-section-staleness.md](kata-tasks/19-section-staleness.md) |
| M4 | tbsh | Edit or regenerate one selected stale section and reuse unaffected assets | wmaj Add optional Gemini with exact-context consent and billing disclosure<br>4f44 Render safe interactive diagrams with evidence-linked hotspots<br>70x5 Add optional natural-speech narration and cached TTS<br>s4df Detect changed evidence and mark only dependent sections stale | [20-selective-section-update.md](kata-tasks/20-selective-section-update.md) |
| M4 | gace | Compare immutable guide revisions and restore as a new draft | tbsh Edit or regenerate one selected stale section and reuse unaffected assets | [21-revision-history.md](kata-tasks/21-revision-history.md) |
| M4 | 8r4j | Export an accepted interactive guide for offline reading | 8z8n Play accepted chapters as manually paced walkthrough sections<br>4f44 Render safe interactive diagrams with evidence-linked hotspots<br>gace Compare immutable guide revisions and restore as a new draft | [22-offline-export.md](kata-tasks/22-offline-export.md) |
| M5 | t188 | Onboard users locally and recover from interrupted work | pn1n Build the shared Mac shell and typed application boundary<br>23eb Open a local repository and create a Code Atlas project<br>8kx6 Browse repository structure and evidence in Atlas<br>3g0a Inspect exact snapshot source and citations in Code Lens<br>wmaj Add optional Gemini with exact-context consent and billing disclosure<br>h9sy Edit, validate, accept, or reject chapters in Chapter Studio<br>8z8n Play accepted chapters as manually paced walkthrough sections | [23-onboarding-recovery.md](kata-tasks/23-onboarding-recovery.md) |
| M5 | bcn2 | Meet accessibility, responsiveness, and repository safety requirements | pn1n Build the shared Mac shell and typed application boundary<br>8kx6 Browse repository structure and evidence in Atlas<br>3g0a Inspect exact snapshot source and citations in Code Lens<br>h9sy Edit, validate, accept, or reject chapters in Chapter Studio<br>8z8n Play accepted chapters as manually paced walkthrough sections<br>4f44 Render safe interactive diagrams with evidence-linked hotspots | [24-accessibility-performance.md](kata-tasks/24-accessibility-performance.md) |
| M5 | e42a | Package, sign, notarize, and document the first Mac release | 8r4j Export an accepted interactive guide for offline reading<br>t188 Onboard users locally and recover from interrupted work<br>bcn2 Meet accessibility, responsiveness, and repository safety requirements | [25-macos-packaging.md](kata-tasks/25-macos-packaging.md) |
| M6 | tbng | Build bounded read-only evidence for local Git base and head refs | hjv4 Capture immutable, bounded repository snapshots<br>c11x Build a supported-language symbol and relationship index<br>e42a Package, sign, notarize, and document the first Mac release | [26-pr-diff-evidence.md](kata-tasks/26-pr-diff-evidence.md) |
| M6 | a9v9 | Generate focused, reviewed explanations for a local change set | wmaj Add optional Gemini with exact-context consent and billing disclosure<br>8bw6 Define versioned chapter, section, claim, anchor, and hotspot records<br>a3xq Generate one substantive, source-grounded chapter per selected file<br>s4df Detect changed evidence and mark only dependent sections stale<br>tbng Build bounded read-only evidence for local Git base and head refs | [27-pr-guide.md](kata-tasks/27-pr-guide.md) |
| M7 | c8nz | Add selectable direct model API providers | wmaj Add optional Gemini with exact-context consent and billing disclosure<br>a3xq Generate one substantive, source-grounded chapter per selected file<br>bwdx Add an optional repository orientation chapter<br>tbsh Edit or regenerate one selected stale section and reuse unaffected assets<br>a9v9 Generate focused, reviewed explanations for a local change set | [28-model-api-providers.md](kata-tasks/28-model-api-providers.md) |
| M7 | tpta | Add selectable read-only coding-agent backends | No open dependency edge | [29-agent-runtime-backends.md](kata-tasks/29-agent-runtime-backends.md) |
| M7 | 2r6n | Make Codex a complete generation choice without mandatory API keys | Blocked by z3an; parent tpta; blocks mfz2 | [45-agent-first-generation.md](kata-tasks/45-agent-first-generation.md) |

## Additional design and acceptance tasks

| Ref | Task | Relationship | Definition |
|---|---|---|---|
| 13yp | Make the packaged app follow the approved product designs | Parent `s0tj`; related to `48yv` | [43-design-implementation-parity.md](kata-tasks/43-design-implementation-parity.md) |
| x21h | Record concrete UI gaps against the approved designs | Parent `s0tj`; related to `13yp` | [44-design-gap-audit.md](kata-tasks/44-design-gap-audit.md) |

## Current delivery order

1. Complete automatic Codex generation without mandatory direct API keys, including runtime setup, supported settings, exact review, progress, cancellation and recovery. Build/test and validate a small synthetic case now; qualify each scope separately. Claude is deferred in `8gqw`; full multi-backend completion remains open.
2. Qualify the shared chapter-version-4, source/citation, history, visual and exact plan-step contracts used by those workflows. Reuse the existing local evidence and validators; older source or suite evidence cannot certify later changes.
3. Complete substantive file explanations and whole-repository narrative orchestration, with explicit omissions, file drill-down and selective updates. Bounded plans and one generated chapter do not satisfy an entire repository guide.
4. Qualify manual playback, generated clickable visuals, Pocket narration, selective revision/history and offline export using the required human/native quality evidence.
5. Complete native accessibility, repository safety, responsiveness and recovery, then the signed/notarized Mac release with supported platforms, runtime provenance and clean-account installation evidence.
6. Complete the full local PR Guide and remaining named backend requirements, including Antigravity's independently eligible runtime/account route. Direct APIs remain optional choices; none may silently replace an agent.

The canonical dependency edges and every task's acceptance criteria still apply. This order prioritizes the requested agent-first work; it does not close earlier foundations, erase missing product work or waive release/PR/backend scope. The [status snapshot](KATA_STATUS.md) includes implementation and acceptance children omitted from the original table above.

**Schema revalidation — 2026-10-16:** chapter version 4 adds original native agent request/settings and exact plan-step lineage to previews/history. The schema task `8bw6` is reopened because its version-3 completion evidence does not qualify this extension. Both agents' runtime/account acceptance and the nine additional deferred origin/legacy/history cases in `xymb` remain pending. See [generation records](CHAPTER_GENERATION_RECORDS.md).

## Question-first source navigation — October 16

Kata `xvjk`: [find relevant code from a question](kata-tasks/55-question-to-source-discovery.md). This follows the native workspace refinement and addresses manual source hunting. The source now includes Codex discovery over immutable indexed metadata, a global header connection, code-first claim rules and route chapter batching. Broad native/language/large-repository acceptance remains open. See [AI guide flow](AI_GUIDE_FLOW.md) and [route batch qualification](kata-tasks/56-route-chapter-batch.md).


## 17 October visual-story checkpoint

The progressive Codex path now supports AI-authored HTML compositions alongside graphs, synchronized with complete scene narration and clickable captured source. A real synthetic scene and a narrow Fontdone PFR0 scene completed; earlier partial/failed runs remain preserved. Mechanical checks pass **346 native tests** (12 explicit live probes ignored) and **171 frontend tests**. [Evidence and remaining quality/export/accessibility gates](acceptance/2026-10-17-visual-story-guide.md). Live Kata still contains **73 tasks: 50 open, 23 closed**; no task is closed by this checkpoint.

## 17 October short-reference and packet-race checkpoint

New progressive answers use short operation-local `code:N` references, retaining
exact native evidence identities, all original budgets and independent legacy
answer replay. A deterministic macOS closed-peer timeout-setting failure is
fixed through bounded nonblocking I/O; no error is ignored or deadline relaxed.
The final rerun passes **359 native tests** (13 explicit account probes ignored),
**179 frontend tests**, the production frontend and unsigned debug app build.
The Mac is locked, so replacement/native acceptance remains pending. A fresh
isolated broad Fontdone account qualification is still required; the earlier
partial result remains failed. [Contract, causal evidence and limits](acceptance/2026-10-17-short-native-references.md).
Live Kata remains **73 records: 50 open, 23 closed**. `3bz5`, `yk6c` and `gk46`
remain open; no task is closed by this checkpoint.

## Versioned scene-quality checkpoint — 17 October

`j53m`, `3bz5` and `q0sm` record the new presentation-v2/checkpoint-v3 guard,
legacy preservation and bounded repair feedback. Mechanical suites/builds
pass; the fresh broad account and installed scene/narration qualification
remain required. [Evidence and scope](acceptance/2026-10-17-scene-v2.md).
All 73 records remain present; 50 are open and 23 closed.

## Fresh version-2 broad run — still partial

`j53m`, `3bz5`, `q0sm`, `gk46` and `ps02` retain the failed complete-answer
assertion: three drafts, 61 delivered calls, eight proposals exhausted, missing
rasterization/bitmap output. Exact source-value diagnostics follow the actual
rejections; 371 native and 184 frontend checks pass. [Current result and scope](acceptance/2026-10-17-scene-v2.md).
No task is closed or requirement removed.

## Saved-route replay correction — 17 October

`a1sm` and `8bw6` retain their full acceptance contracts while recording the
native correction for historical enum digests and known planning request
versions. Real saved route/chapter replay and explicit synthetic upgrades are
covered separately. [Evidence and pending packaged qualification](acceptance/2026-10-17-saved-route-replay.md).
All **73 records remain: 50 open, 23 closed**; no task is closed here.

## Latest broad result and narration ownership — 17 October

The fresh `97e420d` Fontdone run remains Partial near ten minutes, preserving
three drafts through normal-gray rasterization/bitmap publication. Exact source
checks pass for all 29 anchors, but misleading arrows/code-card scenes and dense
narration remain tracked in `3bz5`, `j53m` and `q0sm`. [Result and pending work](acceptance/2026-10-17-fontdone-source-value.md).

`q6g3` now implements one finite worker per complete reviewed script, model/voice
reuse within it, four waiting slots, cancellable bounded stdin and a 49-package
hash-locked macOS/Python runtime. `dbmd` retains installed cache/player/export
and listening acceptance. [Exact whole-script evidence and limits](acceptance/2026-10-17-pocket-whole-script.md)
and [current runtime qualification](kata-tasks/34-pocket-runtime.md). No task is
closed based on source or unit evidence alone.
