---
kind: task
key: t14
parent: m2
title: Establish a repo-level chapter quality benchmark and review gate
priority: 1
labels: phase-2, quality, chapters
blocked_by: t12, t13
---

## User outcome
The user can trust that a chapter teaches the behavior in its file instead of sounding polished while staying shallow.

## Contract
Maintain a small representative benchmark corpus covering at least a route/entrypoint, service or core module, data model, test, configuration file, and unsupported language. Review generated chapter claims against source with a rubric: file responsibility, key declarations, actual execution path, inputs/outputs/state/error paths where present, related tests/callers, citation precision, and honest uncertainty. Record model ID, prompt/schema version, and reviewer findings locally in benchmark artifacts, not telemetry.

## Acceptance criteria
- The rubric has anchored, binary criteria and examples of generic/unsupported claims that fail.
- A chapter with no file-specific behavior or missing required evidence fails the quality gate even if it passes schema validation.
- Simple files may be shorter but still answer their applicable questions with file-specific evidence.
- Benchmark report tracks the corpus manifest hash, per-attempt exact request fingerprint, model ID/version, prompt and request-assembly hashes, generation-config hash, chapter/response schema versions, model-allowlist hash, reviewer/time, pass/fail, and failure category.
- Every initial and repaired attempt records binary rubric results plus a mapping from each `mustCover` fact to positional section claims in that attempt; the report gate rejects evidence-limit IDs and requires each mapped claim to cite a unique exact quote on the fact's expected path whose UTF-8 byte span contains the manifest fact span. A failed attempt may leave a fact's list empty to record that no candidate claim covers it; passing attempts require non-empty mappings for every fact.
- Every repair preserves the initial artifact and review, uses a new artifact path and digest, records its own summary, and leaves the final case fields matching the last reviewed attempt.
- Quality review runs before release and after changes to prompt, schema, or model allowlist.

## Excludes
Claiming semantic truth from a citation hash, using provider self-critique as sole quality proof, and sending benchmark code without consent.

## Done evidence
Review a benchmark batch, identify and repair shallow failures, and show that schema-valid but generic output is rejected by the human quality gate.

## Pending work — 2026-10-15

- [ ] Run a representative consented benchmark batch only when generation/validation is authorized; include route, core module, model, test, config and unsupported language.
- [ ] Have a human score source-specific behavior, execution/data/error paths, exact claims/anchors and uncertainty using the binary rubric.
- [ ] Preserve a shallow failed attempt and a separately generated/reviewed repair with exact artifact fingerprints and mustCover mappings.
- [ ] Do not close from schema validation, source inspection, polished prose or provider self-review.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.

## October 16 native workspace failures to qualify
- [ ] Include same-level contextual follow-ups and two explicit nested levels in the representative account-backed benchmark; measure failure categories and completion time as well as semantic quality.
- [ ] Cover a claim whose declared `Number.isFinite` or `prices` identifier is absent from prose. Keep strict source admission; preserve the failed attempt and a separately reviewed repair. Do not count a successful sibling answer as resolving this case.
- [ ] Qualify the workspace overview's `anchored-claims-v1` presentation with at least two selected files. Check that discarded independent prose/narration never reaches the reader and inference labels remain visible. Single-file routing cannot substitute for this case.
- [ ] Review code-aware speech for natural sentences, names that must be spoken, pacing and section comprehension. Local Pocket playback alone does not pass this rubric.

## Identifier alignment correction — October 16

The reported `record_pipeline_operation_path` failure is the same class as the earlier identifier/prose mismatches. Workspace Codex questions now fingerprint a single correction policy and send bounded positional diagnostics in the same ephemeral thread. No local prose patch, fabricated quote, model fallback or new file selection is permitted. The correction still requires full source and storage validation. The narrow repair regression and packaged smoke evidence belong to [identifier correction acceptance](../acceptance/2026-10-16-claim-alignment.md). Keep this quality gate open: automated lexical correction does not establish semantic accuracy, narrative depth, benchmark coverage or natural speech.

## Fontdone sample — October 17

The [native acceptance record](../acceptance/2026-10-17-fontdone.md) preserves a broad attempt manually stopped after 606 seconds, two ensuing stale-runtime failures, an independently reproduced oversized-file rejection, and a successful small Outline draft. Native cancellation recovery was separately corrected. All seven anchors in the saved draft match independently checked bytes; the OR predicate explanation matches its implementation. This is agent inspection of one small file inside a large repository, not the representative human-reviewed benchmark or successful broad repository explanation.

- [ ] Prioritize the current question's direct answer in the first section. The Outline predicate answer appeared after a data-model section, adding an unnecessary Next action.
- [ ] Distinguish a field's documented contract from verified consuming behavior. The successful sample describes tag/dropout fallbacks from inline documentation without selecting their implementation.
- [ ] Benchmark discovery omissions and time to a useful validated answer on large core files. The broad Fontdone request excluded `src/font.rs` and `src/api.rs`; its smaller successful sibling does not resolve that gap.
- [ ] Qualify generated visuals on a nontrivial implementation path. All sections in the Outline sample had no generated visual; static file Flow cannot substitute.

Keep this gate open, including human narration review and language/PR coverage. The exact successful draft and source-scope metadata are linked from the acceptance record.
