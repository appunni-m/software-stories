---
kind: milestone
key: m4
title: M4 — Git-like guide revisions
priority: 4
labels: container, milestone, phase-4, revisions
blocked_by: m3
---

## Goal
Track guide history independently of source Git, identify affected sections/assets precisely, update selected content, compare/restore, and share a standalone guide.

## Gate
Changing a cited source range stales only its dependent sections and assets. One selected section can be regenerated without rebuilding unrelated guide parts. Prior accepted revisions remain immutable and an offline package is readable without Code Atlas installed.

## Child issues
`19-section-staleness.md`, `20-selective-section-update.md`, `21-revision-history.md`, and `22-offline-export.md`.

## Pending work — 2026-10-15

Pending leaves: tbsh one-section/asset reuse acceptance, gace revision compare/restore acceptance, 8r4j offline export with local narration. Dependencies remain open.
