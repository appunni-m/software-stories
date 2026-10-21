# Packaged parent-section return check — 2026-10-21

## Environment

- Running Code Atlas Mac app.
- Fontdone snapshot `main@5249b72371`.
- No executable checksum or app restart was part of this check.

## Observation

The root exploration was showing a one-section Guide answer, with the source
pane at `src/tt/mod.rs:149` and narration paused at its existing end position.
I opened an already saved Level 2 child from the exploration list. It retained
its stopped/saved state and selected source at `src/outline.rs:48–52`. I used
Back. The root reopened with the parent answer still at `1 / 1`, and the source
pane returned to `src/tt/mod.rs:149`. Narration stayed paused and did not
autoplay.

I did not retry the saved child or submit a question. These navigation actions
did not initiate model inference or narration synthesis.

## Limits

This is one packaged parent/child/Back observation. It does not cover a
multi-section parent, two nested levels, independent sibling positions, Flow
selection or viewport, restart, replaced/deleted revisions, malformed saved
state, or accessibility. The full `erzc` acceptance matrix remains open.
