---
kind: task
key: distribution-license
title: Choose and publish the project's distribution license
priority: 1
labels: docs, release
---

## User outcome

Contributors can tell whether and under what terms they may use, modify, and redistribute Code Atlas.

## Current state

The owner selected MIT on 21 October 2026. The root `LICENSE`, package metadata, README and Tauri metadata now state MIT, and the unsigned macOS app embeds the license and current notice document. The complete target-specific dependency notice inventory and file-level asset provenance still need verification before redistribution or task closure.

## Required work

- Add the owner-selected canonical license text and required notices.
- Make README, Cargo/npm/Tauri metadata and release materials agree.
- Check source, bundled asset and generated-file provenance; document exclusions or attribution requirements.
- Keep user-generated explanations, narration and local repository data outside the licensed source distribution unless ownership and terms are explicitly addressed.

## Acceptance

The owner's license choice is recorded; the canonical license text is present, metadata agrees, and the documentation audit no longer reports a missing license. Keep this task open until all dependencies in the shipped npm and Cargo graphs have a verified license identifier and required text or attribution in the app bundle, asset provenance/exclusions are reviewed, and the packaged output is checked against those inventories.
