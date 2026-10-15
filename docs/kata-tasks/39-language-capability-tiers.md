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
index.rs ships Rust, TypeScript, TSX and JavaScript parser variants. `src-tauri/Cargo.toml` pins tree-sitter runtime 0.27.1, tree-sitter-rust 0.24.2, tree-sitter-typescript 0.23.2 and tree-sitter-javascript 0.25.0; indexed parser metadata also records grammar version and ABI. These pins identify the parser build only; they do not establish semantic correctness. Other text can be cited, but text explanation is not equivalent to symbol/import/call analysis. The targeted lexical-shadowing/export regressions in the current parser set passed a Rust/TypeScript/TSX/JavaScript matrix and full Rust library suite on 2026-10-15. This is a correctness prerequisite, not complete language qualification. No evidence demonstrates full narrative quality for arbitrary languages.

## Parser prerequisite evidence — 2026-10-15

Task 129x added and passed a lexical-resolution matrix across Rust, TypeScript, TSX, and JavaScript, plus an independent base/head Git-index regression. The full Rust library suite passed (179 passed, 0 failed, 1 ignored). Cargo.lock pins tree-sitter 0.27.1, tree-sitter-rust 0.24.2, tree-sitter-typescript 0.23.2, and tree-sitter-javascript 0.25.0; indexed parser metadata records grammar version and ABI.

This evidence closes the targeted shadowing/export defect only. It does not establish complete semantics, framework entry-point coverage, mixed-language behavior, or human-reviewed explanation quality. All language-tier acceptance criteria remain open.

## Source implementation progress — 2026-10-15

Added `docs/LANGUAGE_CAPABILITIES.md` with a dated operation matrix that separates text capture/citations, partial structural parsing, framework and mixed-language coverage, and generated explanation quality. It lists no language as fully qualified. The Guide inventory and Files detail UI now call parser-backed support **Partial structural**, uncatalogued captured text **Text only**, and non-text/unavailable entries **Not analyzed**; they explain that parser availability does not qualify framework behavior or narrative quality. README, blueprint, interaction contract, technical plan, chapter-quality gate, troubleshooting, and getting-started docs link to or clarify the matrix. The tier mapping now treats parser `notApplicable` as **Not analyzed**, rather than inferring text-only capability.

This is documentation and UI progress, not language acceptance. Rust, TypeScript, TSX, and JavaScript still need representative source fixtures and human-reviewed file explanations and behavior journeys; framework and cross-language boundaries remain unqualified. No tests or builds were run.

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
