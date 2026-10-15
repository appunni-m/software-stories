---
kind: task
key: current-design-gap-audit
parent: contract
title: Record concrete UI gaps against the approved designs
priority: 1
labels: product, design, audit, ux
---

## User outcome
A newcomer can open Code Atlas and find the intended starting point, inspect what real files do, follow a section at their own pace, and understand a change without learning the app’s internal workflow names first.

## Relationship to existing work
This is a separate, evidence-first design audit. It records exactly where the current app diverges from the approved screens and what those differences cost the user. Issue `13yp` owns implementation and packaged-app parity; this task does not replace or close it.

## Design authority
Use `docs/design/proposed-00-first-run.svg` through `proposed-04-changes.svg` and `docs/design/PROPOSED_SIMPLIFICATION.md`. The old Atlas / Code Lens / Walkthrough PNG concepts are historical. The primary labels Guide, Files, and Changes in `src/lib/shell.ts` already match the approved direction; do not treat relabeling as a design fix.

## Static source gap baseline — 2026-10-15
This baseline comes from reading the current checkout only. The app was not built, launched, or screenshot-reviewed for this task. Packaged-app behavior and build identity are therefore unverified. Treat the entries below as source-confirmed layout or flow differences, then verify what reaches the user in the packaged build.

| Reference | Current source behavior | Concrete gap and user impact | Source evidence / status |
| --- | --- | --- | --- |
| First run (`proposed-00`) | The initial route is `atlas` (the Files destination). With no repository, `AtlasEmptyState` provides an Open repository action, a three-step explainer, and local-browsing reassurance inside the normal app shell. | The approved reference is a dedicated welcome surface with one clear action. The current route also renders primary workspace navigation, repository/snapshot controls, search, More, help, and the persistent Project context sidebar. The user first lands in Files and must visually parse the full workspace shell. This is a partial match, not a missing open-repository action. | `src/lib/navigation.ts` (`initialNavigationState`); `src/App.tsx` (`AtlasEmptyState`, persistent shell). Packaged result unknown. |
| Guide (`proposed-01`) | When no accepted repository overview exists, `GuidePlanScreen` makes the hero file-first: “What does an important file do?”, opens the first file awaiting explanation, and says repository tours are optional. Accepted overviews get a Read the guide action; accepted guides are also reachable through More. | The approved Guide makes the short repository tour the primary start-here action, with file coverage and next files alongside it. A first-time user with no accepted overview has no obvious tour action and is steered away from the repo-level orientation shown in the design. | `src/App.tsx` (`GuidePlanScreen`, `acceptedOverview` fallback, guide actions). Source-confirmed; native presentation unknown. |
| Files (`proposed-02`) | The selected-file route composes the inventory tree, inline captured source, and `FileExplanationPanel`; relationship exploration is collapsed under an optional disclosure. | The core three-pane structure and optional map are present. Two visible hierarchy gaps remain: the persistent outer Project context sidebar makes the Files work area effectively a fourth column, and the explanation panel renders accepted chapter sections/citations without the reference’s guaranteed one-sentence summary, short numbered behavior steps, and explicit connected-file list. This makes the first-glance answer harder to find. | `src/App.tsx` (`SnapshotReadyState`, `AtlasWorkbench`, `FileExplanationPanel`); `docs/design/proposed-02-files-explainer.svg`. Source-confirmed; native presentation unknown. |
| Walkthrough (`proposed-03`) | `WalkthroughScreen` has a chapter/section rail, a long center column containing prose, evidence cards, file links, optional visual, narration, and transcript citations, plus a right rail with generic progress/help cards. Previous and Continue are rendered after all section content. | The design keeps a behavior diagram and source excerpt central, gives the right panel a section-specific key idea and term definitions, and keeps Previous/Continue visible at the bottom. Current controls can fall below long evidence/narration content; the right rail does not explain the selected section’s key idea or vocabulary. This undermines deliberate stop, inspect, and continue pacing. | `src/Walkthrough.tsx` (`WalkthroughScreen`, `EvidencePanel`, `VisualPanel`); `src/App.css` (`.walkthrough-section-actions`, `.walkthrough-side-notes`). Source-confirmed; common-window behavior unknown. |
| Changes (`proposed-04`) | `GitChangesScreen` stacks a full-width base/head form above `ComparisonResults`. Results stack the changed-file disclosures and `PrGuideStudio`; plain-language generation is a separate provider-backed workflow. | The design keeps changed files, focused before/after code, a behavior explanation, and evidence links visible together. Current source does not provide that persistent comparison layout or an always-visible explanation of the selected change; the reader must work through stacked disclosures and the separate PR Guide path. | `src/GitChanges.tsx` (`GitChangesScreen`, `ComparisonResults`); `src/PrGuideStudio.tsx`; `src/App.css` (`.git-changes-workflow`). Source-confirmed; native presentation unknown. |

## Required work
1. Open the packaged Mac app and record its app version/build identity and window size. Capture the real first-run, Guide home, selected-file Files view, one Walkthrough section, and populated Changes comparison. Use the same viewport for the SVG reference and app capture.
2. For each of the five references, classify the shipped screen as match, partial, deviation, or not inspectable. Attach the screenshot evidence and note the repository/snapshot/state used; never substitute the web preview or source labels for native evidence.
3. Review and correct the source baseline table above. Record the exact screen element that differs, the user consequence, the responsible component/route, severity, and the smallest specific fix. Mark unknowns instead of guessing.
4. Separate true mismatches from intended complexity. Preserve exact source citations, snapshot identity, explicit provider consent, local-only Git safety, and optional relationship exploration while simplifying the visible path.
5. Turn implementation changes into focused updates to `13yp` or a narrowly scoped follow-up Kata issue. Keep this issue as the complete, reviewable gap ledger.

## Acceptance criteria
- One screen-by-screen gap ledger covers all five approved SVG references and includes the app build identity, viewport, evidence screenshot, source component, observed deviation, impact, priority, and proposed fix for each non-matching screen.
- The ledger confirms both what already matches and what does not. It explicitly records that the Guide / Files / Changes labels and the selected-file tree/source/explanation composition exist in source, while distinguishing that from verified packaged-app parity.
- The first-run state, no-overview Guide state, Files explanation hierarchy, Walkthrough controls/context panel, and Changes result layout are each directly compared against their specified design content and action hierarchy.
- Findings are specific enough to become implementation criteria without phrases such as “make it cleaner” or “match the mockup”; every fix names the control, content, placement, and expected navigation or return behavior.
- Packaged-app unknowns remain labeled unknown until observed. This task does not claim visual acceptance, accessibility sign-off, or completion of `13yp`.

## Exclusions
Do not implement UI changes in this audit task, regenerate the design images, add product features, or treat source inspection as a native-app review. Do not close `13yp`.

## Done evidence
A dated gap ledger with five packaged-app screenshots, build/window metadata, corrections to the source baseline, prioritized specific fixes, and links from each fix to its implementation issue.
