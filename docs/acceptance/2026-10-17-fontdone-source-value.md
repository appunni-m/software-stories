# Fontdone after precise scene feedback — 17 October 2026

## Fresh account qualification: still Partial

This is a new, isolated GPT-6 Luna/max account run using source `97e420d`,
including the precise source-value feedback added after the [earlier partial
run](2026-10-17-scene-v2.md). It uses the same frozen Fontdone snapshot, original
bytes-to-bitmap question, six-section/eight-proposal/64-call/250,000-source-byte
bounds and ten-minute deadline. No prior answers, accepted decisions or source
grants were copied. No API, model change, manual source selection, automatic
retry or increased cap was involved.

- Operation: `9eda207b-ab6e-48af-89f1-866c68bd8163`.
- **Partial at 599.786 seconds**. The retained Complete assertion fails; the
  explicit ignored test exits 101 after 602.10 seconds. There is a final
  manifest, with two incomplete topics and no terminal runtime failure code.
- Three drafts published at run elapsed **84.661 / 495.159 / 588.242 seconds**;
  independent native adoption was observed at **85.391 / 496.576 / 590.295
  seconds**. These are native probe timings, not UI-visible useful-answer timing.
- **55 calls, all delivered; 127,466 distinct captured source bytes, seven
  implementation paths; six proposals, three rejected and three published**.
- The sections now reach byte entry, TrueType tables/simple and composite
  outlines/hinting, render dispatch, normal-gray rasterization and bitmap
  publication. CFF/CFF2/Type 1 outline-loader internals and mono/LCD/SDF/overlap
  rendering internals are explicitly incomplete. This does not qualify an
  entire repository guide or comprehensive branch coverage.

[Original full result](artifacts/2026-10-17-fontdone-source-value-partial.json)
remains byte-identical, SHA-256
`d132905ba77f6d99173f75d3159b6d177c88587b03d64b9f23b85a4f5811036c`.
The older failed artifacts remain unchanged. Local log:
`/private/tmp/code-atlas-fontdone-source-value-v2.log`.

## Independent mechanical review

The seven implementation hashes, ledger and probe identities match the
committed source. All nine [frozen input metadata hashes](artifacts/2026-10-17-fontdone-scene-v2-inputs.json)
remain unchanged. All **29 published anchors** independently match full captured
source hashes, exact byte excerpts and byte-based line/column coordinates.
Every section retains identical full prose/spoken text and presentation-v2 /
checkpoint-v3 identity. [Mechanical review and exact anchor checks](artifacts/2026-10-17-fontdone-source-value-review.json).
These checks certify source identity and admission, not semantic accuracy.

Proposal one omitted a declared identifier from its narration. Proposal three
had eight scene/node claim-alignment findings plus one exact `sceneSourceValue`
finding. Proposal five had one identifier/evidence mismatch. The agent repaired
each as a complete section in the same operation, without dropping local
validation or resetting limits. No generic `sectionValidation` finding appears.
This observation does not prove that the diagnostic change caused a timing or
quality improvement; there is only one non-completing run under these settings.

## Representation and speech remain inadequate

A targeted agent review finds that most visuals still present source snippets
rather than illuminating transformations. In particular, the “Geometry and
programs” scene draws `loca_data → glyf_data` even though the narration describes
two copied tables, not one becoming the other. The final coverage scene connects
buffer allocation to the overlap-specific rasterizer without visualizing its
condition. These are concrete semantic presentation counterexamples; exact
source tokens and a valid scene schema do not make those arrows correct.

Narration remains dense with helper names and load flags. No human comprehension
or Pocket listening pass is recorded. The drafts remain unaccepted. Tasks
`3bz5`, `j53m` and `q0sm` retain broad completion, meaningful representations,
causal narration and installed navigation/accessibility requirements. The Mac
is still locked; this probe did not install or visually validate the app.
