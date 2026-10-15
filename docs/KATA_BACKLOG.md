# Kata backlog — Code Atlas rebuild

Workspace project: **code-atlas-rebuild**. Product plan: [PRODUCT_BLUEPRINT.md](PRODUCT_BLUEPRINT.md).
Technical plan: [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md). Generated on 2026-10-14.
Kata issue bodies are synchronized from `docs/kata-tasks/*.md`; each issue states its user outcome, interaction contract, acceptance criteria, exclusions, and closure evidence.

**Priority update — 2026-10-15:** [Agent-first generation](kata-tasks/45-agent-first-generation.md), Kata `2r6n`, is priority 0 under `tpta` and blocks `mfz2`. Complete integrated Codex and Claude Agent support without mandatory direct API keys before further visual/narrative polish. Manual agent handoffs are interim and do not satisfy that release gate. See [Agent generation](AGENT_GENERATION.md).

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
| M7 | 2r6n | Make Codex and Claude Agent complete generation choices without mandatory API keys | Parent tpta; blocks mfz2 | [45-agent-first-generation.md](kata-tasks/45-agent-first-generation.md) |

## Additional design and acceptance tasks

| Ref | Task | Relationship | Definition |
|---|---|---|---|
| 13yp | Make the packaged app follow the approved product designs | Parent `s0tj`; related to `48yv` | [43-design-implementation-parity.md](kata-tasks/43-design-implementation-parity.md) |
| x21h | Record concrete UI gaps against the approved designs | Parent `s0tj`; related to `13yp` | [44-design-gap-audit.md](kata-tasks/44-design-gap-audit.md) |

## Build order

1. Design the screen/state/navigation contract, then build the empty app shell.
2. Capture local snapshots, account for every path, index supported relationships, and connect Atlas to exact Code Lens source.
3. Make file-by-file coverage visible; add the exact-context Gemini boundary, structured chapters, generation, human review, and quality gate.
4. Add manual section playback, typed linked visuals, and optional natural-speech narration.
5. Add section-level freshness, selective revision, history, and offline export.
6. Release the dependable Mac app. Build local PR explanation later; it does not block the first release.
7. Add selectable direct model APIs and coding-agent runtimes through maintained libraries, ACP where supported, and each agent’s documented integration surface.
