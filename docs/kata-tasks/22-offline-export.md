---
kind: task
key: t22
parent: m4
title: Export an accepted interactive guide for offline reading
priority: 2
labels: phase-4, export, player
blocked_by: t16, t17, t21
---

## User outcome
The user can share or archive an accepted guide and open it without a Code Atlas account, app installation, or network connection.

## Contract
Export selected accepted chapters as a versioned, self-contained offline package with section navigation, transcript, typed visuals, and optional cached audio. Before writing, preview included chapter names, source paths, cited excerpts, and assets. Default export includes only selected exact cited excerpts, not the whole repository; user may omit excerpts. Package links go to bundled evidence or other package sections; external/local path links are clearly labeled and never silently open another file.

## Acceptance criteria
- Draft/rejected chapters are not exported unless the user explicitly chooses an export preview state that marks them as drafts.
- Package opens offline and reports missing/unsupported features instead of blank controls.
- A package manifest records schema version, chapter revisions, evidence snapshot IDs, and included asset hashes.
- The exporter warns about paths and excerpts that may expose sensitive code; no API key or hidden repository files are included.
- Before download, the preview shows each selected section's typed-diagram status and each matching cached-audio asset, alongside chapter names, source paths, and cited excerpts. Unsupported diagrams and missing audio are identified as omitted rather than silently disappearing.
- Reopening an export does not mutate the source project or guide history.

## Excludes
Uploading to a hosting service, bundling the full repository by default, MP4-only export, and executing package scripts.

## Done evidence
Export selected accepted chapters with/without excerpts and audio, review the per-section asset preview, disconnect network, open the package, navigate every section and hotspot, and verify manifest/hashes and excluded secrets.
