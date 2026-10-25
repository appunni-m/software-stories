# AI-authored visual scenes

Guide gives the visual the main space, with narration and source details
collapsed. AI chooses what representation teaches the current question and
composes it as responsive HTML. A graph is one available representation.
The current graph layout and workspace navigation remain available.

## Choosing a representation

| What the reader needs to understand | Useful visual vocabulary |
| --- | --- |
| Which bytes identify a format | Indexed byte strip, highlighted signature, code value |
| How an input becomes a result | Three-part input → operation → result composition with distinct values |
| How a code-proven point or vector is mapped | Coordinate-free point-flow: symbolic point markers around the actual operation, with no invented positions |
| How a mode or enum selects a handler | Table mapping each value to its selected route |
| Why an outcome differs | Guarded branch with every outcome, or a comparison/table when branches do not matter |
| One guard decides success or failure | Branch showing the guard and each supported outcome |
| Independent validation guards run in a function | Separate scenes, one guard at a time; narration explains that passing lets the input continue |
| Competing ordered candidates or parser signatures | Selector ladder with one matched outcome per guard and the reader's visible miss-and-continue handoff |
| How state is organized | LIFO stack, FIFO queue, nested tree, or before/after state |
| How data changes over time | Ordered timeline, parallel actor lanes, or iteration table with state per step |
| A count or constraint | Large quantity with its meaning and source |
| Raster or grid concepts | Pixel matrix, explicitly illustrative unless captured from source |
| A direct call chain or ordered execution path | Timeline showing the actual handoffs, even when each call has a source link |
| A non-linear call network, fan-out or convergence | Source-linked graph with real labeled edges |

These are composable, app-rendered forms rather than complete canned diagrams.
The agent chooses a form independently for each claim and scene, then chooses
its nesting, labels, values and emphasis. Cards may label parts of a visualization
but should not replace its data shape, branch, or state change. One section can
mix HTML and graph scenes. A graph should be used only for source-proved paths or
relationships, not merely because a prior renderer expected edges.

The design pass starts with what the cited code does to its input or state:
change it, select it, move it, wait, branch, or fail. Each scene centers one
focal object or relationship and uses the narrowest semantic form that makes it
visible. Rows, columns, and groups arrange the visual; they are not a visual
idea by themselves. With narration collapsed,
the source-specific values and layout should still expose the claim's main
change or decision. Avoid dashboards, component inventories, collages, and
interchangeable card piles. Different behavior can justify a different form;
novelty alone does not.

## Generation and evidence

Progressive Codex publication and fresh chapter generation support
`visual.presentation` alongside the evidence-linked visual nodes and edges.
File chapters, repository overviews and selected-section repairs share the
same composition vocabulary and causal-story instructions. A chapter request
requires the presentation property and permits `null` for a plain graph or
absent visual. Each composed claim is one narrated scene. The agent authors:

1. The grounded claim and exact delivered evidence references.
2. Short node labels/details, each bound to that claim's evidence.
3. A scene composition and its source values or labeled illustrative examples.

Native canonicalization first validates claims and creates source anchors. It
then validates the composition against those exact anchors. Saving/reloading
and edited chapter validation apply the same composition checks. There is no
separate generated speech paraphrase that could lose an important condition.
The reader independently checks the stored presentation before rendering.

The current source contract uses story policy
`ai-html-compositions-v48` and teaching policy `meaning-first-v29`. It starts
from a real input or situation, follows only source-supported changes and
outcomes, and gives each scene one behavior to make visible. Each claim scene
selects composition or graph independently, so one visual section can mix those
modes when the behavior warrants it. Narration-only mode applies only when the
entire section has no useful visual. Layout follows the
behavior: one guard with alternatives uses a branch; independent validation
guards become separate scenes; first-match candidate dispatch uses a selector.
LIFO stacks are authored oldest/bottom first and rendered/read newest/top first,
with the next item to pop marked. A source-backed state change uses a transform with its operation visible, and
`beforeAfter` is reserved for comparing states when this claim does not cover
the operation. Verified order uses a timeline, and independent actors use lanes. A graph is for relationships those forms cannot
show clearly; a sequence of calls alone does not justify one. Before proposing
another section for the same answer, the generator compares its behavior and
visual focus with sections already published; a new title or repeated rendering
of the same decision is not new coverage. If one requested behavior cannot fit,
the answer names the uncovered behavior instead of repeating a neighboring
section.

Graph edges must name the operation that actually establishes the relationship.
A caller that looks up an item and then invokes a loader establishes call order;
it does not establish that the lookup produced the loader's success or failure
status. Outcome branches begin at the operation or status check that produces
them. The Full map is the complete set of source-backed edges, not an inferred
bridge between separate scenes. If the code only proves order, show order without
claiming the later operation's result.

The reader implementation offers **Full map** only when the authored section
includes a graph scene. Composition-only lessons keep their chosen visual
instead of exposing a separate AI-generated graph outline.

When a section has useful visuals, each claim gets a source-linked scene and
chooses `composition` or `graph` independently; one section may mix both. If no
claim benefits from a visual, the section uses narration with clickable
evidence instead. Narration follows the subject from its caller, input, or
decision through supported consequences, including guards, alternatives,
failures, and uncertainty. It points to a visual cue only as a brief landmark.
Exact code names stay visible beside the audio; narration uses their role in
the behavior. Fresh generated narration is rejected when it contains a compact
all-caps/code-format token so the model can replace it with plain speech. This
check is intentionally narrow: it does not prove that the remaining wording is
natural or that TTS pronounces it well. Layout must enact the behavior rather
than default to a uniform card grid; it changes at an evidenced handoff, not
for decoration.
For static code without a concrete runtime sample, a visual should use the
source variable or field as a symbolic input or result when that makes the
change clearer. Generic stand-ins such as “Loaded input” or “Processed result”
hide the implementation detail the visual is meant to teach. Label a symbolic
value if a reader could mistake it for an observed runtime value. Never invent
coordinates, payload contents or measured outputs. Exact source values are
used only when their literal bytes or spelling matter; illustrations are
labeled and never presented as observed execution. These are generation
instructions, not evidence of quality; fresh rendered examples still need
review.

The app renders only its typed, bounded HTML/CSS vocabulary. Model output cannot
execute HTML, JavaScript, CSS, URLs, remote assets or Three.js. A 3-D renderer
is not currently supported. Structural checks can reject invalid trees,
unsupported values and repeated content across a transformation arrow, but
they cannot prove that a visually changed value accurately teaches the code's
meaning. That still requires source review and human comprehension checks.
The 12-node and 20-edge limits apply across the whole section, not once per
scene; per-scene limits are ceilings, not targets. These prompt instructions
and structural checks do not prove that a fresh guide teaches well; rendered
quality still needs human review.

For ordered selector layouts, the selector root must contain its candidate
branches directly. A row or column between the selector and its checks is
invalid. Each candidate branch contains exactly one guard and its one matched
result. The reader already shows the miss-and-continue path between candidates;
duplicating the miss as an outcome makes a single check look like a selector.
A single if/else belongs in a `branch`, and independent validation guards are
split into separate scenes.

Fresh authoring also requires the claim, title or narration to say that a
candidate miss tries the next candidate before a `selector` is allowed. This
narrow wording check rejects an unsupported automatic handoff; it cannot prove
that the cited source actually implements first-match dispatch.

The reader renders execution paths as compact rails rather than bordered step
cards, and ordered selectors as numbered gates with visible match destinations
and a fall-through line. Their source-linked labels stay short; the complete
condition or call remains in the adjacent source pane. Narration starts from
the incoming value or situation and follows the visual through the consequence,
instead of opening with a documentation-style method summary. These render
changes target the first Fontdone qualification; they still require packaged
visual review before acceptance.

Repository-guide planning uses the same bounded section capacity. Long
ordered dispatch chains must be split into consecutive learning units before
their visuals exceed 12 source nodes or 20 edges. The planner targets eight
nodes and two to four claims/scenes per slide, with one behavior stage per
unit; each unit preserves its ordered checks, outcomes and fallbacks.

Spoken narration describes implementation tokens by role instead of reading
them aloud, including method/type names, abbreviations, format tags,
field/table names and error labels. Fresh composed scenes, authored narration-
only scenes and standalone generated narration all reject compact all-caps
tokens in speech; exact tokens remain available through source links. Speech
uses cues such as “the face's glyph-loading call,” “the first font-format
check,” or “the glyph's stored width.” Historical saved chapters remain
replayable under their original contract.

This is a compositional HTML contract, not permission to run model-produced
programs. React owns the tags, CSS, keyboard handlers and source actions. Model
strings become text. Unknown elements/fields, raw markup fields, scripts, styles,
URLs and handlers are rejected. No external image or resource loads are added.
Illustrative values carry a visible **Illustrative example** label. A value
declared `source` must be an exact substring of that node's cited excerpt.
This mechanical check does not prove that an illustration accurately teaches
the behavior; semantic and comprehension review remain necessary. Scenes use the
app's bounded 2-D renderers; model-authored JavaScript or Three.js is not run.
3-D remains a future app-owned renderer option only when depth explains a real
spatial behavior more clearly than a 2-D form. The model cannot return executable
JavaScript or Three.js; a useful 2-D cross-section is allowed only when it
preserves the behavior being explained.

### Stored shape

Stored visual version 2 has an optional `presentation` field. New chapter
compositions and progressive generations use presentation version 3;
progressive publication uses answer checkpoint version 8. Historical admitted
proposals and immutable answer checkpoints keep their original saved content.
The current renderer supports semantic `transform`, coordinate-free `pointFlow`, `tree`, `queue`, and
`selector` groups in addition to the existing layouts. Composition data
participates in visual fingerprints, revision comparisons and selective visual
reuse.

```json
{
  "schemaVersion": 3,
  "scenes": [{
    "claimIndex": 0,
    "title": "Put the name into the greeting",
    "narration": "The name enters as Ada, passes through the greeting template, and leaves as Hello, Ada.",
    "mode": "composition",
    "elements": [
      {"kind":"transform","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
      {"kind":"code","parentIndex":0,"nodeIndex":0,"tone":"accent","value":"Hello, ${name}","valueOrigin":"source"},
      {"kind":"token","parentIndex":0,"nodeIndex":1,"tone":"neutral","value":"Ada","valueOrigin":"illustration"},
      {"kind":"token","parentIndex":0,"nodeIndex":2,"tone":"positive","value":"Hello, Ada","valueOrigin":"illustration"}
    ]
  }]
}
```

The transform's direct children are always labeled **Input**, **Operation**,
and **Result**, in that order. The example assumes three nodes citing the same
claim; captured code contains the template substring. The output is an example,
not a recorded execution. Graph scenes use `mode: "graph", elements: []`.

### Prioritized guards

Use a `selector` only for first-match dispatch among competing ordered
candidates, such as format signatures, handlers, or priority routes. Each direct
child is a `branch` containing exactly two content leaves: its guard followed by
the one result when it matches. The reader labels candidates in order and shows
“No match · continue” between them. Do not add that miss as a second outcome.
A normal `branch` remains the right form for one guard with multiple outcomes;
independent validation guards in one function belong in separate scenes.

### Meaningful transformations

Native validation rejects an arrow with identical visible content on either
side. The check compares the nearest content-bearing sibling groups, including
their nested leaves, by visible node labels and exact values. Changing node IDs,
colors, wrappers or content primitive does not establish a transformation.
Presentation version 3 adds a dedicated `transform` group with exactly three
direct children; the renderer labels them input, operation, and result. These
structural checks do not prove that a changed label or illustrative value
accurately represents the code. Prefer `transform` when the cited code shows
the operation that changes state; its middle slot names that operation. Use
`beforeAfter` only to compare two source-backed states when the operation
between them is outside this claim. A semantic review must check the input,
actual operation, guards, and result against captured source.

The progressive agent receives fixed `sceneRepetition` feedback at the arrow
position and `transformShape` feedback for a transform with the wrong number of
children. Both repairs retain the original operation budget. `sceneVersion`
rejects an old, absent or malformed version on a newly supplied presentation.
Omitting the optional presentation remains valid. An exact repeat of an old
admitted proposal keeps its old checkpoint and IDs; replay never upgrades or
rewrites historical guides. New checkpoint replay independently enforces the
new version requirement. Rejected proposals remain in the private answer
record, while repair feedback contains positions and fixed codes only.

After native rejection, `sceneSourceValue` locates a source-valued element that
does not occur in its own node's same-claim cited excerpts. Repair the value or
its evidence, retaining exact citation authority; another node's or claim's
excerpt is insufficient. The feedback never includes source strings or raw
decoder errors. It does not authorize a value or bypass canonicalization.

### Primitives and bounds

- Groups: layout `row`, `column`, `grid`, `panel`, `compare`; semantic
  `timeline`, `lanes`, `branch`, `selector`, `beforeAfter`, `transform`,
  coordinate-free `pointFlow`, `tree`, `queue`, and LIFO `stack`.
- Content: `card`, `token`, `code`, `metric`, `bytes`, `matrix`, `table`, `text`.
- Decoration: `arrow`, `divider`, with no free content or source action.
- Tones: `neutral`, `accent`, `muted`, `positive`, `warning`, mapped to app theme
  tokens. A generated tone cannot supply custom CSS.
- Exactly one scene per claim in claim order, at most 20 in a stored
  presentation. Progressive proposals currently permit at most eight claims.
- One root, earlier container parents, at most 48 elements, six nesting levels,
  twelve children per group and twelve content leaves per scene.
- Each content leaf references a visual `nodeIndex` with evidence belonging to
  the scene's own claim. Values are at most 1,024 UTF-8 bytes.
- A `transform` has exactly three direct children: input, operation, result.
  The app renders them as labeled stages connected in reading order. The result
  must represent a meaningful change when the scene claims a transformation.
- `pointFlow` has the same three source-linked stages and a distinct point/function
  visual. Use it only when cited code proves a point or vector mapping. It labels
  itself symbolic and never implies coordinates or geometry absent from source.
- A `tree` groups nested source-backed data. A `queue` shows FIFO order from
  front to back; it is not an unordered collection.
- A `selector` has at least two direct `branch` groups in source order. Each
  branch has exactly two direct content leaves: a guard and its one matching
  result. The app renders the check sequence and miss handoffs. This shape
  validation does not prove that
  the chosen guards, outcomes or fallback match the cited code.
- Byte strips: 1–32 whitespace-separated cells. Matrices: 1–16 equal rows of
  1–24 cells using `0`, `1` or `.`. Tables: at most eight rows and six
  pipe-separated columns of equal width.

## Playback and interaction

The existing scene player displays the authored composition for the current
claim. Pocket narrates its complete grounded text. Actual clip completion
advances the scene; section boundaries remain manual. Choosing a scene or source
element stops the prior clip. Each HTML content element opens its exact captured
source through the existing app-owned route. Keyboard Enter/Space invokes that
same action. Scene and independent audio positions persist by immutable revision.
Restoring a paused clip at its end must not count as actual playback completion.

Older summarized or custom whole-section scripts remain intact. When exact claim
scene alignment is unavailable, the reader keeps a whole script rather than
estimating timestamps or changing the saved answer. Text, edits and details stay
available under disclosures, and the app honors Reduce Motion.

## Qualification and current limits

The proposal schema is exposed on progressive account-backed Codex and the
shared file/overview/selected-repair request builders, including direct-provider
wires. Schema and mock-wire checks do not qualify a remote API model's output.
The fresh chapter boundary requires presentation version 3 and assembles complete grounded
claim narration, with a 4,000-byte section bound and no silent selection of
shorter sentences. Selective repairs prepare only their selected section;
existing saved scripts and carried legacy scenes keep their content.
Broad representation quality across languages and operations remains task
`j53m` scope.

The schema-4 offline exporter renders HTML compositions and attaches verified
cached audio to individual scenes, with matching transcripts and independent
hashes. Live and offline matrix compositions expose their dimensions and exact
row values through an accessible name. Source-backed visual values are omitted when cited
excerpts are omitted.
Graph scenes retain a source-outline fallback. Navigation and playback remain
manual in the script-free export; it does not reproduce the app's automatic
scene advancement or graph canvas. See [offline use and limits](OFFLINE_EXPORT.md).
Native cache/export and actual browser playback qualification remain tracked.
Human visual quality,
pronunciation, pacing, comprehension and native accessibility still require
their acceptance evidence. A schema pass or a screenshot cannot close those
gates. See [Kata task](kata-tasks/37-generated-visuals.md) and
[narration run sheet](NARRATION_VALIDATION.md).

The scene contract does not accept model-generated JavaScript or raw HTML. A
3-D representation would need a tested, app-owned typed renderer for genuinely
spatial source behavior; Fontdone’s parser path is sequential, so a 2-D path and
ordered selector are the fitting forms.
