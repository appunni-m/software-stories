---
kind: task
key: language-capability-tiers
parent: m2
title: Define and qualify language support for code explanations
priority: 1
labels: product-quality
blocked_by: parser-binding-resolution
---

## User outcome
Users know which languages have reliable structural evidence and receive honest useful explanations for mixed-language repositories.

## Current evidence
index.rs ships Rust, TypeScript, TSX and JavaScript parser variants. Other text can be cited, but text explanation is not equivalent to symbol/import/call analysis. Even the current supported set has unresolved/shadowing correctness work. No evidence demonstrates full narrative quality for arbitrary languages.

## Contract
Publish capability tiers by language and operation: text capture/citations, declarations, imports, calls/tests, framework entry points, cross-language links and generated explanation quality. Separate grammar availability from semantic resolution and narrative quality. Repair the existing parser set first. Add maintained Tree-sitter/LSP/compiler integrations where they fit; do not build a custom parser framework. Choose expansion languages from representative target repos, with Python and Go as initial candidates rather than promises of finished support.

## Acceptance criteria
- UI and documentation distinguish text-only, partial structural and qualified structural support; unresolved relationships retain reasons.
- Each advertised language has source fixtures for responsibility, input/output, state/error behavior, imports/calls/shadowing and relevant tests/config.
- Real framework conventions and mixed-language boundaries are qualified separately; filenames alone cannot prove an execution path.
- Unsupported files remain browsable/explainable with exact citations and visible evidence limits.
- Human review qualifies generated file explanations and at least one behavior narrative per advertised language; a grammar parsing successfully is insufficient.
- Every added adapter/version updates cache identity and capability metadata; source anchors remain exact.

## Excludes
Advertising all languages from an LLM's general ability to read text, runtime guarantees from static analysis and guessing cross-language calls.

## Done evidence
Dated capability matrix, source-linked fixture outputs and human-reviewed narratives; acceptance remains pending until authorized validation.
