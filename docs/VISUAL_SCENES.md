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
| How a mode or enum selects a handler | Table mapping each value to its selected route |
| Why an outcome differs | Guarded branch with every outcome, or a comparison/table when branches do not matter |
| Which ordered guard returns first | Selector ladder with each guard, outcome and visible handoff after a miss, including sequential early returns |
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

The current prompt is story policy `ai-html-compositions-v24` and teaching
policy `meaning-first-v26`. It asks the model to identify the source-backed
relationship first, then make the most specific available visual form the
focal point. It rejects scenes that would tell the same story after swapping
project names, generic panels/cards/rows used as the diagram, and component
catalogs without a concrete guard, transition or result. Supported forms
include timelines, branches, selectors, transforms, before/after states,
tables, lanes, trees, stacks, queues, byte strips, matrices, metrics, and
graphs. Mutually exclusive success and failure outcomes fork visibly from the
operation result instead of appearing as consecutive steps. The prompt keeps
plain-language labels, all meaningful inputs and
outcomes, and clickable evidence. Ordered selectors use at most three checks
per scene and split longer chains into consecutive scenes; every guard and
destination gets its own short label. Exact source values appear only when their
literal spelling or bytes matter. If the code evidence cannot support a useful
visual, narration and source links remain valid. Adjacent scenes may keep the
same visual identity when that helps the reader track one object; form changes
when behavior changes. A graph is reserved for genuinely non-linear
relationships; source links or a sequence of real calls do not by themselves
justify one. Model output remains typed data rendered by the app; it cannot
execute HTML, JavaScript, CSS or Three.js. A 3-D renderer is only appropriate
for source-grounded spatial behavior when a tested app-owned renderer exists.
Each scene now starts from a short visual idea and a focal object, relationship
and consequence. Across a story, the model should vary forms when the code's
behavior changes (for example, bytes, a decision fork, changed state, a mapping
table, or an ordered path), while reusing a form only when it helps track the
same subject. It should compose the available semantic primitives into a
behavior-specific responsive HTML scene rather than defaulting to a generic
block diagram. Version 24 also distinguishes source-ordered calls from
alternatives: calls that run in sequence use a timeline or linked path, while a
branch requires mutually exclusive destinations selected by one evidenced
guard. Each branch condition appears once and each destination label names its
behavior or resulting state. These are generation instructions, not evidence
of quality; fresh rendered examples still need review.
The 12-node and 20-edge limits apply across the whole section, not once per
scene; per-scene limits are ceilings, not targets. These prompt instructions
and structural checks do not prove that a fresh guide teaches well; rendered
quality still needs human review.

For ordered selector layouts, the selector root must contain its guarded branch
groups directly. A row or column between the selector and its checks is invalid;
the guard and each supported destination belong inside the branch group.

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
field/table names and error labels. Exact tokens remain available through
source links; speech uses cues such as “the face's glyph-loading call,” “the
first font-format check,” or “the glyph's stored width.”

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
The current renderer supports semantic `transform`, `tree`, `queue`, and
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

Use a `selector` only when source order and short-circuiting determine which
guard wins. Each direct child is a `branch` containing its guard followed by
one or more outcomes; include the final fallback. The reader labels checks in
order and shows “No match · continue” between them. A normal `branch` remains
the right form for one guard with alternatives that have no priority between
them.

### Meaningful transformations

Native validation rejects an arrow with identical visible content on either
side. The check compares the nearest content-bearing sibling groups, including
their nested leaves, by visible node labels and exact values. Changing node IDs,
colors, wrappers or content primitive does not establish a transformation.
Presentation version 3 adds a dedicated `transform` group with exactly three
direct children; the renderer labels them input, operation, and result. These
structural checks do not prove that a changed label or illustrative value
accurately represents the code. A semantic review must check the input, actual
operation, guards, and result against captured source.

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
  `tree`, `queue`, and LIFO `stack`.
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
- A `tree` groups nested source-backed data. A `queue` shows FIFO order from
  front to back; it is not an unordered collection.
- A `selector` has at least two direct `branch` groups in source order. Each
  branch begins with a guard and has at least one outcome; the app renders the
  check sequence and miss handoffs. This shape validation does not prove that
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
hashes. Source-backed visual values are omitted when cited excerpts are omitted.
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
