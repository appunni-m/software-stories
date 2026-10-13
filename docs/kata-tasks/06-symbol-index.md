---
kind: task
key: t06
parent: m1
title: Build a supported-language symbol and relationship index
priority: 1
labels: phase-1, index, repository
blocked_by: t05
---

## User outcome
The user can follow code structure that local parsers actually resolved, with uncertainty visible instead of a fabricated complete call graph.

## Contract
Index captured snapshot blobs only. Select initial languages based on checked fixtures; TypeScript/JavaScript and Rust are candidates. Record parser/version, declaration identity, source range/hash, and edge type/confidence/evidence. Initial edge types may include imports, exports, definitions, statically resolved calls, test references, and configuration references. Dynamic dispatch, reflection, and unresolved references stay unresolved.

## Acceptance criteria
- Index output is deterministic for the same parser version and snapshot bytes.
- Every symbol and edge has an exact anchor into that snapshot.
- Unsupported language, parse error, partial parse, and unresolved relationship are separately represented.
- A line-only shift preserves stable symbol identity when the declaration itself is unchanged.
- Indexing is cancellable and bounded; it never executes repository code.

## Excludes
Runtime tracing, complete call-graph claims, AI-generated labels, and model interpretation.

## Done evidence
Representative TypeScript/JavaScript and Rust fixtures cover declarations, imports, calls, tests, syntax errors, dynamic dispatch, and line shifts; unsupported files remain browseable with an explicit state.
