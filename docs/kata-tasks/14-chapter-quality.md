---
kind: task
key: t14
parent: m2
title: Establish a repo-level chapter quality benchmark and review gate
priority: 1
labels: phase-2, quality, chapters
blocked_by: t12, t13
---

## User outcome
The user can trust that a chapter teaches the behavior in its file instead of sounding polished while staying shallow.

## Contract
Maintain a small representative benchmark corpus covering at least a route/entrypoint, service or core module, data model, test, configuration file, and unsupported language. Review generated chapter claims against source with a rubric: file responsibility, key declarations, actual execution path, inputs/outputs/state/error paths where present, related tests/callers, citation precision, and honest uncertainty. Record model ID, prompt/schema version, and reviewer findings locally in benchmark artifacts, not telemetry.

## Acceptance criteria
- The rubric has anchored, binary criteria and examples of generic/unsupported claims that fail.
- A chapter with no file-specific behavior or missing required evidence fails the quality gate even if it passes schema validation.
- Simple files may be shorter but still answer their applicable questions with file-specific evidence.
- Benchmark report tracks the corpus manifest hash, per-attempt exact request fingerprint, model ID/version, prompt and request-assembly hashes, generation-config hash, chapter/response schema versions, model-allowlist hash, reviewer/time, pass/fail, and failure category.
- Every initial and repaired attempt records binary rubric results plus a mapping from each `mustCover` fact to positional section claims in that attempt; the report gate rejects evidence-limit IDs and requires each mapped claim to cite a unique exact quote on the fact's expected path whose UTF-8 byte span contains the manifest fact span. A failed attempt may leave a fact's list empty to record that no candidate claim covers it; passing attempts require non-empty mappings for every fact.
- Every repair preserves the initial artifact and review, uses a new artifact path and digest, records its own summary, and leaves the final case fields matching the last reviewed attempt.
- Quality review runs before release and after changes to prompt, schema, or model allowlist.

## Excludes
Claiming semantic truth from a citation hash, using provider self-critique as sole quality proof, and sending benchmark code without consent.

## Done evidence
Review a benchmark batch, identify and repair shallow failures, and show that schema-valid but generic output is rejected by the human quality gate.
