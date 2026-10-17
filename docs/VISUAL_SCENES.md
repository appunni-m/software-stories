# AI-authored visual scenes

Guide gives the visual the main space, with narration and source details
collapsed. AI chooses what representation teaches the current question and
composes it as responsive HTML. A graph is one available representation.
The current graph layout and workspace navigation remain available.

## Choosing a representation

| What the reader needs to understand | Useful visual vocabulary |
| --- | --- |
| Which bytes identify a format | Indexed byte strip, highlighted signature, code value |
| How an input becomes a result | Input/output values, code fragment, before/after comparison |
| Why an outcome differs | Comparison, small table, decision graph when branches matter |
| How state is organized | Stack, nested framed groups, rows or grids |
| A count or constraint | Large quantity with its meaning and source |
| Raster or grid concepts | Pixel matrix, explicitly illustrative unless captured from source |
| Calls, paths, ownership or relationships | Source-linked graph with real labeled edges |

These are composable primitives rather than complete canned diagrams. The agent
chooses nesting, grouping, labels, values, emphasis and representation separately
for each scene. One section can mix HTML and graph scenes. A graph should not be
used merely because the old renderer expected edges.

## Generation and evidence

The progressive Codex tool accepts optional `visual.presentation` alongside the
existing evidence-linked visual nodes and edges. Each claim is one narrated
scene. The agent authors:

1. The grounded claim and exact delivered evidence references.
2. Short node labels/details, each bound to that claim's evidence.
3. A scene composition and its source values or labeled illustrative examples.

Native canonicalization first validates claims and creates source anchors. It
then validates the composition against those exact anchors. Saving/reloading
and edited chapter validation apply the same composition checks. There is no
separate generated speech paraphrase that could lose an important condition.
The reader independently checks the stored presentation before rendering.

This is a compositional HTML contract, not permission to run model-produced
programs. React owns the tags, CSS, keyboard handlers and source actions. Model
strings become text. Unknown elements/fields, raw markup fields, scripts, styles,
URLs and handlers are rejected. No external image or resource loads are added.
Illustrative values carry a visible **Illustrative example** label. A value
declared `source` must be an exact substring of that node's cited excerpt.
This mechanical check does not prove that an illustration accurately teaches
the behavior; semantic and comprehension review remain necessary.

### Stored shape

Visual version 2 gains an optional `presentation` field with its own version 1.
Absent fields remain absent on serialization. Existing diagrams and immutable
answer checkpoints are not regenerated or migrated on read. The renderer
contract is version 2; composition data participates in visual fingerprints,
revision comparisons and selective visual reuse.

```json
{
  "schemaVersion": 1,
  "scenes": [{
    "claimIndex": 0,
    "title": "Put the name into the greeting",
    "mode": "composition",
    "elements": [
      {"kind":"row","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
      {"kind":"code","parentIndex":0,"nodeIndex":0,"tone":"accent","value":"Hello, ${name}","valueOrigin":"source"},
      {"kind":"arrow","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
      {"kind":"token","parentIndex":0,"nodeIndex":1,"tone":"positive","value":"Hello, Ada","valueOrigin":"illustration"}
    ]
  }]
}
```

The example assumes two nodes citing the same claim, whose captured code
contains the literal template substring. Its output is an example, not a
recorded execution. Layout arrows express reading direction; runtime claims
still require evidence. Graph scenes use `mode: "graph", elements: []`.

### Primitives and bounds

- Groups: `row`, `column`, `grid`, `panel`, `compare`, `stack`.
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

The new proposal schema is exposed on the progressive account-backed Codex
path. Older direct-provider and non-progressive prompts still request diagrams;
their existing output remains readable. Broad representation selection across
languages and generation operations remains task `j53m` scope.

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
