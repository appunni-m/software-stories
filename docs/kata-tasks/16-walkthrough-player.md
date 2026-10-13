---
kind: task
key: t16
parent: m3
title: Play accepted chapters as manually paced walkthrough sections
priority: 0
labels: phase-3, player, design
blocked_by: t13
---

## User outcome
The reader can understand one code-backed idea at a time and decide when to stop, inspect source, revisit, or continue.

## Contract
Show chapter list, section list, current section question/title, explanation, evidence panel, visual slot, transcript, and Previous/Continue controls. Continue is a deliberate action; there is no timer-based advance. Support section selection, pause/resume, replay, captions/transcript, and keyboard controls. Entering Code Lens preserves section ID and playback position; returning restores both.

## Acceptance criteria
- Every section has an explicit start and completion boundary and accessible Previous/Continue labels.
- Playback never auto-advances or hides source while narration runs.
- Selecting a section updates code highlights, visual, transcript, and narration to that section's revision.
- Back from source, Atlas, or related chapter restores the same reader context.
- Missing audio leaves the transcript and manual controls fully usable.

## Excludes
Autoplay between chapters, a linear MP4 as canonical content, and auto-generated cinematic footage.

## Done evidence
Navigate forward/back, jump sections, pause, inspect a citation, return, and verify exact section, source highlight, transcript, and playback position.
