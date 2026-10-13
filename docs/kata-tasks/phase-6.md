---
kind: milestone
key: m6
title: M6 — Local PR Guide follow-up
priority: 4
labels: container, milestone, phase-6, pull-request
blocked_by: m5
---

## Goal
Extend the trusted repository guide to a selected local Git base/head change set after the first release is stable.

## Gate
Every PR explanation is a separate reviewed draft with claims pinned to a real diff or snapshot source anchor. The feature uses local refs only and never rewrites the repo or existing guide.

## Child issues
`26-pr-diff-evidence.md` and `27-pr-guide.md`. This follow-up does not block the first Mac release.
