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

The repository has no root `LICENSE` file. The project owner must choose the license after confirming ownership and intended distribution. Do not infer a license, copy one from another project, or claim the repository is open source before that decision.

## Required work

- Add the owner-selected canonical license text and required notices.
- Make README, Cargo/npm/Tauri metadata and release materials agree.
- Check source, bundled asset and generated-file provenance; document exclusions or attribution requirements.
- Keep user-generated explanations, narration and local repository data outside the licensed source distribution unless ownership and terms are explicitly addressed.

## Acceptance

The chosen license is present with unmodified canonical terms, metadata agrees, third-party notices and package exclusions are checked, and the documentation audit no longer reports a missing license. This task remains open pending the project owner's decision.
