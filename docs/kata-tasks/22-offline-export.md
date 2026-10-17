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

## Pending work — 2026-10-15

- [ ] After revision and Pocket integration, export selected accepted sections with/without excerpts and matching audio.
- [ ] Inspect the exact per-section preview and manifest, including omitted unsupported diagrams/missing audio.
- [ ] Open the package offline and exercise every section/hotspot; verify hashes and no unselected source/keys/hidden files.
- [ ] Record unchanged source history and clear unsupported/malformed-package errors.

No tests or builds were run during the October 15 static bug-fix pass. Keep acceptance open until its required evidence is recorded.

## October 17 HTML scenes and scene audio implementation

Export now renders validated HTML compositions with manual scene/source links,
collapsed notes and independently scoped cached scene audio. Graphs retain a
disclosed source outline. Shared player/export preferences preserve custom
whole-section scripts; missing scene clips show partial coverage. Schema 4
hashes every included transcript/audio asset. Omitting excerpts also removes
declared source-backed visual values, while common credential/path filtering
now covers presentation titles and values. Audio is not rewritten by filtering.

The synthetic two-scene Pocket fixture passed all four excerpt/audio package
combinations, local-link/asset-hash checks and input-immutability assertions.
Frontend 179 tests and native 346 tests passed (12 explicit live probes ignored),
and the unsigned app built. These are not actual accepted-chapter/cache or
browser playback acceptance. The native Fontdone export dialog has no accepted
revision; saved drafts were not changed to create a pass. Computer-use policy
blocked opening the local HTML; manual review remains unreported.
See [scoped evidence](../acceptance/2026-10-17-offline-visual-scenes.md).

- [ ] Export genuine accepted chapters through the native preview, covering
  scene-scoped clips, custom whole scripts and partial/missing cached audio.
- [ ] Open all four source/audio combinations with network disconnected;
  exercise every section, scene/source link, native audio control, keyboard
  focus and narrow layout, recording exact browser/build versions.
- [ ] Match preview text, native fingerprints and package hashes; inspect source
  omissions and confirm source history is unchanged.
- [ ] Record human listening/comprehension independently of worker success.

Keep this task open until the required installed/browser evidence is recorded.
