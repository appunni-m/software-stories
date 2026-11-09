---
kind: milestone
key: m4
title: M4 — Git-like guide revisions
priority: 4
labels: container, milestone, phase-4, revisions
blocked_by: m3
---

## Goal
Keep accepted guide sections immutable, identify affected sections/assets precisely, and regenerate only selected stale content. Full revision compare/restore and standalone offline export are post-1.0 follow-ups.

## Gate
Changing a cited source range stales only its dependent sections and assets. One selected section can be regenerated without rebuilding unrelated guide parts. Prior accepted revisions remain immutable. Compare/restore and offline export are not required for release 0.1.0.

## Child issues
`19-section-staleness.md` and `20-selective-section-update.md`. Full revision compare/restore and standalone export are preserved under [post-1.0 history](../history/post-1.0/README.md).

## Historical pending-work checkpoint — 2026-10-15

At that checkpoint, the list still included revision compare/restore. Current
first-release work is `tbsh` one-section repair and unaffected-asset reuse;
revision compare/restore and export are now post-1.0.
