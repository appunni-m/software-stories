---
kind: milestone
key: m3
title: M3 — Interactive explainer
priority: 4
labels: container, milestone, phase-3, player
blocked_by: m2
---

## Goal
Turn accepted file chapters into an explorable learning experience with manual section controls, validated hotspots, visible source, transcript, and optional speech.

## Gate
A reader can stop at every boundary, follow a hotspot to the exact code, return to the same section and playback position, and choose when to continue. Narration never blocks reading or source navigation.

## Child issues
The basic player and linked static visuals are complete. Remaining generated visuals and narration work is tracked in `37-generated-visuals.md`, `34-pocket-runtime.md`, and `35-pocket-player-cache.md`. Generation-time prewarming is deferred to post-1.0 and is not an active acceptance gate.

## Current delivery scope — 8 November 2026

Formal accessibility audits, cloud TTS options, and generation-time prewarming
are out of scope. The local Pocket setup, playback, cache behavior, recovery,
and human listening acceptance remain owned by `q6g3` and `dbmd`. The duplicate
parent issue `70x5` was retired; prewarming is preserved in post-1.0 history.
