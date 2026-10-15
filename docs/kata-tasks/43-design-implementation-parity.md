---
kind: task
key: design-parity
parent: contract
title: Make the packaged app follow the approved product designs
priority: 1
labels: product, design, ux, acceptance
---

## User outcome

A new developer can open Code Atlas and immediately understand where to start, what a file does, and what changed between two revisions. The shipped Mac app follows the approved screens and interactions, rather than only containing matching workspace labels in source code.

## Evidence observed on 2026-10-15

There are two conflicting product directions in the repository:

- The current simplification proposal, docs/design/PROPOSED_SIMPLIFICATION.md and proposed-00 through proposed-04 SVGs, defines Guide, Files, and Changes. Guide is a start/continue home with file coverage; Files combines the tree, selected source, explanation, and evidence; Walkthrough is a manually paced Guide flow.
- The greenfield contract, docs/kata-tasks/00-contract.md, and the older design image prompt still define Atlas, Code Lens, and Walkthrough as three equally important destinations, with Guide Plan and Chapter Studio as supporting workflows.

The code and packaged app also disagree:

- Source src/lib/shell.ts maps the primary bar to Guide, Files, and Changes, but Guide currently opens GuidePlanScreen headed “What still needs an explanation?”; it does not open the proposed overview/continue-tour home.
- Files currently opens the Atlas workspace. Code Lens remains a separate route. Opening a file therefore moves from the file inventory/map into a separate source screen instead of showing the proposed tree, exact source, concise explanation, and clickable citations together.
- Guide Plan and Chapter Studio remain distinct route screens in src/App.tsx. The More menu exposes separate “Browse accepted guides” and “Review a draft” actions; the source-first editing/review surface is not beside the selected file explanation as specified.
- The running packaged Mac app showed Atlas, Code Lens, and Walkthrough in its primary bar, with Guide Plan and Chapter Studio outside that navigation. On fontdone it showed 2,768 captured files and 25 skipped paths in the persistent project panel. The inspected Code Lens view for tests/support/generated_constant_lookup.rs had long repeated TestReference cards, including adjacent lines all resolving generated_rust_constant to constant_to_i64. This makes parser/index evidence dominate the file explanation.
- The app reported Mac app 0.1.0. This visible bundle does not match the checked-out shell source, which already labels the primary destinations Guide, Files, and Changes. Therefore source-only inspection or a web preview cannot prove the user-facing app is current.

These are information-architecture and workflow gaps, not merely spacing or color differences. The existing design proposal’s “first implementation pass” notes are not packaged-app parity evidence.

## Required work

1. Resolve the competing design contracts before implementation. Record which screens the user-approved design references define as authoritative. Update the product blueprint, Kata contract, and design notes so they no longer specify conflicting primary navigation. Label older concept boards as superseded or historical if the current approved direction replaces them. Preserve useful Atlas/map, Code Lens, Guide Plan, and Chapter Studio capabilities as contextual or advanced actions where appropriate.
2. Implement the selected Guide home as a genuine starting point: repository orientation, a short optional tour with visible progress and deliberate Previous/Continue, file explanation coverage, and one clear next action. Coverage must remain available without being the only Guide landing screen.
3. Make Files a unified file-understanding surface: searchable repository tree, selected captured source, concise file explanation, exact clickable citations, and related-file/source links in the same destination. Keep the relationship map optional or secondary. Opening a citation or related file must preserve the prior selection and return context.
4. Make Changes match its reference: explicit base/head selection, changed-file list, focused before/after source, plain-language explanation, and evidence links to the changed lines. Keep the local-only Git contract intact.
5. Apply the design’s content hierarchy to real repositories, including large or generated files. Repeated low-value static references must be collapsed, grouped, or clearly secondary; they must not crowd out the explanation of what the file actually does. Partial capture must present an actionable next step. Put hashes, counts, and advanced controls behind the relevant details affordance where the reference calls for it.
6. Align the packaged Mac app with the source version being reviewed. Keep the native bundle’s navigation, routes, labels, and behavior in sync with the current checkout. Do not count a successful web preview, component screenshot, or source-level label check as native acceptance.
7. Keep the visual language coherent with the selected reference set. Use the actual project, snapshot, and source content; do not substitute static mock data for core screen content.

## Acceptance criteria

- One committed design source of truth describes the same primary destinations and screen hierarchy across the product blueprint, Kata contract, design notes, source, and packaged app.
- In the packaged Mac app, the primary bar has exactly the selected three destinations and each opens the intended landing screen. No old top-level destination or detached workflow is needed to complete the main Guide → Files → Changes journey.
- Guide first opens on the intended home. A reader can enter a tour, move one section at a time, open a source hotspot, return to the same section, and find file coverage and the next useful action.
- Files opens one file and shows its source, explanation, and evidence together. Relationship-map exploration is optional. Citation and related-file links open exact local snapshot evidence and return without losing context.
- Changes displays a real local comparison with the selected base and head, focused diff, explanation, and exact source evidence; empty, loading, and error states explain the next action.
- On the fontdone fixture and a second non-trivial repository, generated/repetitive relationship rows do not swamp the primary explanation. Partial-snapshot status leads directly to skipped-path review.
- Capture screenshots of the packaged app for Guide home, one walkthrough section, Files with a selected file, and Changes with a selected diff. Review each against the authoritative reference at the same window size and record pass/fail with deviations. Include the app version/build identity in the evidence.
- Verify primary navigation, tour Next/Previous, source/citation links, and return-state preservation in the packaged app. Record any accessibility issues under the dedicated accessibility task rather than silently treating this task as an accessibility sign-off.

## Exclusions

Do not add new model providers, generation behavior, PR hosting, or unrelated parser/index features here. Do not remove evidence, safe local Git boundaries, revision history, or accessibility work. This task does not close the separate accessibility, packaging, or chapter-quality acceptance gates.

## Done evidence

A single reconciled reference set, updated product/design contracts, the implementation commit, and a native-app acceptance record with the required screenshots, build identity, interaction results, and remaining deviations.
