---
kind: task
key: repository-narratives
parent: m2
title: Build coherent repository narratives with file-level drill-down
priority: 0
labels: product-quality
---

## User outcome
A newcomer can follow a real feature end to end, understand each relevant file, and choose deeper explanations without hearing a list of document summaries.

## Current evidence
The implementation generates bounded file chapters and an optional orientation chapter from 1–12 selected files. The file quality gate does not qualify a complete repository narrative. There is no demonstrated whole-repository story planner or cross-language end-to-end quality report.

## Contract
Create an explicit repository guide plan over the immutable inventory. Identify evidenced entry points and user-relevant behavior journeys, prerequisite concepts, files/relationships, success/failure paths and known unknowns. Plan distinct sections with a concrete learning question, source scope, script, optional visual and deeper source/file links. Keep the overview optional and preserve per-file coverage. Generate in bounded reviewed units through the chosen API or agent runtime; show coverage, omissions, progress and recoverable failures. Do not squeeze a whole repository into one model request.

## Acceptance criteria
- A reader can choose orientation, a behavior journey or any eligible file; every completed section has explicit Next, Back and Explore code actions.
- At least one real feature follows its supported entry point through calls/data/state/error behavior, with each transition tied to source evidence.
- Plan records prerequisites and explains names before depending on them; no repeated generic introductions or unexplained jumps.
- Mixed-language boundaries are linked only from supported evidence; unknown dispatch/protocol details remain visible.
- Per-file explanations remain available as needed and coverage never counts the overview as explaining every file.
- One section can be edited/regenerated while untouched sections/assets retain identity; changed shared evidence marks affected sections separately.
- Human reviewers answer predefined behavior questions after the guide and record source errors, confusion and missing coverage. No whole-repo quality claim from individual file schema success.

## Excludes
Autoplay through an entire repository, hidden bulk provider spend, marketing-only intros and replacing per-file explanation with storytelling.

## Done evidence
A complete source-grounded guide on representative repositories, coverage map, incremental-edit demonstration and human comprehension review when validation is authorized.
