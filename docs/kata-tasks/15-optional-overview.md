---
kind: task
key: t15
parent: m2
title: Add an optional repository orientation chapter
priority: 3
labels: phase-2, orientation, chapters
blocked_by: t12
---

## User outcome
The user can start with a concise repository map before choosing any file chapter, or skip the overview entirely.

## Contract
Generate the overview as a separate schema-version-3 chapter using one to twelve explicitly selected files from the chosen immutable snapshot and locally resolved relationships whose endpoints are in that selection. Each claim cites exact source anchors; relationship claims cite only the reviewed local index evidence. Unsupported repository-purpose or architecture statements are visibly labeled as inference or omitted. The overview uses stable IDs `chapter:overview` and `target:overview`; it has its own append-only revision history, can be omitted, and never marks a file as covered.

## Acceptance criteria
- Users can start a file chapter without opening the overview.
- Omitting, editing, or regenerating the overview does not change per-file coverage or chapter history.
- Unsupported repository purpose or architecture statements are labeled as inference or omitted.
- Overview citations use the same selected snapshot and Code Lens navigation as file chapters.
- Request preview, consent, generation, validation, unsaved preview, explicit save, and review use the same lifecycle as file chapters without requiring a target file.

## Excludes
Replacing file chapters, claiming that directory names prove responsibility, and making overview completion a first-release gate.

## Done evidence
Create and omit an overview on a fixture; confirm every file's coverage and accepted revision remain identical.
