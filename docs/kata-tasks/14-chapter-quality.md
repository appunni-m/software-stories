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

## Owned retrieval predicate and narration gap — October 17

The [new owned-tool Outline draft](../acceptance/artifacts/2026-10-17-fontdone-agent-outline.json) answers the predicate directly in its first and only section and includes a three-node/two-edge source-linked OR diagram. Two anchors independently match the pinned source bytes, hash, line coordinates, unique quotes and admitted evidence. This is agent source inspection, not the required human binary-rubric review or broad narrative pass.

- [x] Align the owned retrieval instructions with `canonicalize_workspace_overview`: separate `prose`/`spokenText` fields are discarded, and final narration is selected from anchored claim sentences. Main behavior claims must include a plain-language restatement of that same behavior.
- [x] Prioritize main behavior in deterministic narration selection. Keep behavior claims in order and preserve their exact code-heavy sentence if no plain restatement exists. A simpler peripheral sentence cannot displace it. Source grounding and visible inference labels remain enforced.
- [x] Add deterministic coverage for a code-heavy main claim plus a peripheral plain-language sentence and a later plain behavior claim. Both the plain-restatement and code-only fallback cases pass independent content/anchor validation; unsupported separate narration is still discarded.
- [ ] Review a fresh exact prepared script, then listen to Pocket output for the two conditions, disagreement and false case. The earlier sample's only spoken sentence omitted the predicate. A selector regression, valid citation or diagram does not establish that a newly generated script teaches all the behavior or sounds natural.
- [ ] Keep broad Fontdone parsing/loading/hinting/rasterization, multiple-language and PR narratives in the representative human-reviewed corpus. This small predicate does not resolve them.

The [next broad owned investigation](../acceptance/artifacts/2026-10-17-fontdone-agent-broad-budget-identifier.json) retrieved nine implementation paths but failed admission for declared `resolve_face_index` missing from prose after 529.629 seconds of native execution. It published no preview, so it provides no narrative, flow or speech quality pass. Preserve the rejected candidate with positional diagnostics before a separately recorded repair/review; the current artifact contains the error and evidence identities without the original candidate text. The owned route currently disables the legacy alignment correction. This gate remains open.

## Native-reference semantic counterexample — 17 October

The latest frozen Fontdone draft failed source alignment after one correction and was not published. Separately, corrected `/sections/1/claims/3` passed lexical alignment but describes tolerated malformed required SFNT tables as optional data. Captured `src/font.rs:4698–4727` requires the `maxp`, `cmap` and `hmtx` entries, then defaults selected parse failures. Missing tables and malformed table contents are distinct cases. [Exact candidate, source comparison and timing](../acceptance/2026-10-17-native-citation-references.md).

Preserve this as an explicit semantic regression fixture: a reference ID resolving correctly and an identifier appearing in prose do not establish truth. Review the actual presentation-selected narration, rather than the separately emitted spokenText that this path discards. The proposed visual covers routing only; constructor/index/error diagrams and Pocket playback were not qualified. [Progressive publication, Kata 3bz5](73-progressive-answer-sections.md) must retain this separate content gate. This source review is not a human-scored full benchmark batch and does not close the task.

## Progressive Fontdone counterexample — 17 October

The new installed progressive run published two cited drafts but finished partial, so exact source validation still cannot stand in for answer quality. Its dispatch claim says the first matching format branch returns its constructor result, while the cited BDF code may return parse_bdf_constructor_error before bdf_face. Its two-node/one-edge visual omits the ordered branches. The actual scripts either retain raw underscored identifiers or omit most validation/fallback sentences; Pocket audio was not assessed. SFNT required-table missing-versus-malformed behavior and final face construction remain explicitly unfinished. [Preserved result and reviewed limits](../acceptance/2026-10-17-progressive-reader-integration.md#fresh-fontdone-retry-useful-delivery-mechanics-incomplete-content).

Required follow-up: review complete generated claims against conditions and error branches, verify visual structure teaches those branches, preserve behavior when repairing identifier/count findings, and compare the actual spoken script with the displayed explanation before playback qualification. Do not accept this answer or count its early card as a qualified useful-answer timing pass. This task remains open.

## October 17 narration and visual-story quality boundary

The next installed Fontdone retry (`7d6e335`) fixed the lost-script presentation defect and included the BDF early-error exception with a genuine branching graph. It still ended partial after one section; its phrase “all control bytes” does not exactly express the below-ASCII-space guard. The original artifact is retained [here](../acceptance/artifacts/2026-10-17-fontdone-complete-script-partial.json). Neither complete copy, a larger graph, native source validation nor loaded Pocket audio qualifies semantic accuracy or comprehension.

Apply the [visual story acceptance contract](61-guide-flow-detail.md): the narration must follow an input/goal, decisions and consequences, referring to readable visual cues. A definition list or documentation paragraph read aloud fails, even with correct citations. The text remains accessible under collapse. Require fresh Rust and TypeScript examples, source-independent semantic review, human listening/comprehension, complete large-repository answers and the existing performance/error gates before closure. Keep open.

## October 17 representation quality gate

Evaluate whether the agent chose a representation that helps answer the question: a byte strip for probing, a transformation for changed data, a comparison/table for alternatives, a stack for state, a pixel grid for raster concepts, or a graph for paths and relationships. Judge a fresh AI-authored composition rather than a hand-inserted showcase. Illustrative input/output/pixels must be labeled and defensible from code; never present them as observed runtime data. Narration must refer to visual cues and explain cause and consequence. Require exact branch/error review, human comprehension and listening evidence, not only acceptance of a schema or a rendered screenshot. Keep the failed partial synthetic and Fontdone runs as failures of completion even if some individual scenes are useful.

## October 17 HTML scene qualification

A complete narrow PFR0 Fontdone answer proves HTML composition/source-click mechanics, not whole-repository or narration quality. Its accepted script still reads call syntax; new-generation instructions move expressions to visuals and explain their meaning aloud, retaining bare identifiers and supported conditions. Fresh human quality qualification remains required. Earlier partial and failed outcomes are preserved separately. See [results, preserved failures and remaining gates](../acceptance/2026-10-17-visual-story-guide.md). This task remains open.
