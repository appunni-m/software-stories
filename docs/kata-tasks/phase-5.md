---
kind: milestone
key: m5
title: M5 — Dependable first Mac release
priority: 4
labels: container, milestone, phase-5, release
blocked_by: m4
---

## Goal
Make first use understandable, failures recoverable, and the Mac app installable and supportable.

## Gate
A new user reaches offline Atlas without a key; cancellation and provider errors preserve work; ordinary keyboard navigation works; repository operations stay safe and responsive; release artifacts are signed/notarized and support limits are documented. Formal screen-reader and assistive-technology audits are out of scope.

## Child issues
`23-onboarding-recovery.md`, `24-repository-safety-performance.md`, and `25-macos-packaging.md`.

## Pending work — 2026-10-15

Pending: bcn2 repository-safety/responsiveness work and newly recorded parser/process/storage tasks, then e42a signed/notarized clean-account release. Formal accessibility audit is no longer a release gate. Static fixes are untested.
