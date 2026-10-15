# Code Atlas simplification

These five SVGs are the design reference for a simpler Code Atlas. The first
implementation pass now carries that direction into the app while preserving
the existing source, guide, and revision workflows.

## The proposed shape

Keep three destinations in the main bar:

1. **Guide** — understand the repository, continue a short tour, and see which
   files still need explanations.
2. **Files** — browse the real source and read its explanation in the same
   place. The relationship map becomes an optional view here; Code Lens becomes
   the selected-file/source detail rather than a separate destination.
3. **Changes** — compare two local revisions, inspect exact before/after source,
   and understand which behavior changed. Pull request sharing stays a later,
   explicit extension of this local workflow.

`Guide Plan` and `Chapter Studio` should no longer appear as separate project
workspaces. File coverage belongs on Guide and editing/review belongs beside the
selected file explanation. An optional overview remains visibly separate from
the individual file explanations. The existing guide must keep its promise to
explain every eligible file; a repo tour introduces the system and helps a
reader choose what to inspect next.

Model, voice, exact request review, advanced index filters, snapshot hashes,
unresolved relationships, and recovery tools remain available behind the
relevant action or Settings. This changes when details appear, not whether
source evidence, request consent, safe capture, or revision history exist.

## Main images

### First run

![Proposed first-run screen with one next action and a plain three-step explanation](proposed-00-first-run.svg)

### Guide home

![Proposed Guide home with a short tour, file coverage, and three plain-language actions](proposed-01-guide-home.svg)

### Files and explanation

![Proposed Files screen combining a repository tree, exact source, concise explanation, and clickable evidence](proposed-02-files-explainer.svg)

### Section-by-section walkthrough

![Proposed Walkthrough with visible progress, clickable source-linked nodes, optional audio, and deliberate Continue](proposed-03-walkthrough.svg)

### Change explanation

![Proposed Changes screen with two revisions, a focused diff, and evidence-backed behavior explanation](proposed-04-changes.svg)

## What the current app showed

The open Mac app has three primary tabs, then Guide Plan, Chapter Studio, and
revision comparison hidden in a project-tools menu. Atlas combines an inventory
tree, a relationship canvas, multiple filters, unresolved-edge counts, and
evidence details at once. In the inspected fontdone snapshot, Code Lens showed
hundreds of unresolved relationship rows below the source. Guide Plan presents
per-path statuses, filters, grouping, pagination, and chapter actions together.
That forces a new reader to learn internal product concepts before they can
answer “what does this file do?”

The redesign focuses each screen on one question:

| Screen | First question answered | Main action |
| --- | --- | --- |
| Guide | Where should I start? | Start or continue a short tour |
| Files | What does this file do? | Open exact evidence or explain selected lines |
| Changes | What is different between these versions? | Compare, inspect, then explain the change |

## Interaction rules for the implementation

- Keep one project and one immutable source version visible across all three
  screens. Put full hashes, capture counts, and policy details on demand.
- Explain one idea at a time. Every explanation links to exact captured lines;
  unresolved relationships stay collapsed and are never presented as facts.
- Every tour section has **Previous** and **Continue**. Audio is optional,
  never starts automatically, and never advances the reader on its own.
- Clicking a diagram node, source range, or related-file link opens the exact
  local evidence. Returning restores the selected section and its scroll/audio
  position.
- Show partial-capture gaps as a plain warning with one next action, such as
  **Review skipped files**. Do not bury the warning among metadata.
- Keep advanced setup, provider/model choice, filters, and troubleshooting out
  of the first-use path. Explain the current action before asking for exact
  source-sharing consent.
- Offer one-file explanation as the default. Multi-file overviews and grouping
  require an explicit selection and show what the selection means.
- Put recovery, stale citations, and failed generation next to the item they
  affect, with a clear retry, inspect, or return action.

## Image and implementation notes

The images are exact-label SVG screen proposals, drawn directly as vectors to
honor the standing instruction to use only GPT-6 Luna Max and not call another
model. They use the existing dark graphite, warm-neutral, blue, and restrained
lime design language. The SVGs remain design references; they are not screenshots
of the current build.

## First implementation pass

- The main bar now presents **Guide**, **Files**, and **Changes**. File source
  and the guide reader keep their route state and return to their parent
  destination.
- Guide opens on file-by-file coverage. Its first view offers three plain
  filters, path search, direct actions to read accepted explanations or open a
  file that needs one, and optional advanced filters and grouping.
- Files opens a file directly from its row. Snapshot hashes, capture limits,
  and the relationship map stay available behind deliberate details controls.
- Code Lens presents a short list of resolved links first; unresolved references
  are separate and collapsed.
- The repository search shortcut now focuses the file search instead of showing
  a “coming soon” message.

This is an initial usability pass, not a claim that every screen is finished.
The AI request/review flow, change comparison, native accessibility, and visual
acceptance still need their own review. No automated test or build was run for
this pass, in keeping with the request to work from source without testing.
