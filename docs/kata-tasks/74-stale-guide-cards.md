---
kind: task
key: stale-guide-cards
title: Hide stale unavailable answer cards from Guide
priority: 1
labels: recovery, ui, workspace
parent: 13yp
---

## User outcome

Guide stays focused on explanations the reader can open. Unavailable historical answers remain reachable from Chat history with an explicit recovery action.

## Requirements

- Do not treat saved-revision references as missing while the revision library is loading.
- Keep turns visible when they have an available section, active work, are explicitly selected, or own a visible child exploration.
- Hide other turns whose answer references all point to unavailable revisions.
- Opening such a question from Chat history focuses it in Guide and presents the unavailable state with an explicit re-explain action.
- Keep partial answers with at least one available section understandable; do not delete questions, receipts, answers, or generated content.

## Completion evidence recorded in Kata

Kata `5vw6` was closed on 20 October 2026. The recorded regression check passed **22/22** focused workspace outcome tests, the production build and macOS bundle passed, and the installed Fontdone workspace kept the selected PFR0 explanation in Guide while Chat history retained the unavailable question and recovery action. The broader parent design acceptance `13yp` remains open.
