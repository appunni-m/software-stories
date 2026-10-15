---
kind: task
key: parser-binding-resolution
parent: t24
title: Resolve calls and exports conservatively under lexical shadowing
priority: 1
labels: parser, correctness
---

## User outcome
Atlas, Code Lens, freshness analysis and PR guides do not present a false high-confidence relationship as source evidence.

## Current evidence
Static finding in src-tauri/src/index.rs: call resolution considers callable declarations but omits parameter/local noncallable shadow bindings. For example, import { run } from './worker'; function execute(run) { run(); } can resolve to the imported function incorrectly. collect_file_index propagates inside_export into nested bodies; nested helpers/methods may be marked as module exports. Duplicate declaration signatures share a symbol ID and are collapsed by or_insert, losing ambiguity. Implementation remains pending.

## Contract
Resolve a call only when its exact lexical scope has one statically supported binding. Shadowed, dynamic or ambiguous bindings stay unresolved with a reason. Module exports must reflect actual exported bindings. Preserve duplicate occurrences or explicitly record ambiguity. Bump the indexer semantics/cache version so old relationships are not silently reused.

## Acceptance criteria
- Parameters, local variables, destructuring and block-scoped bindings shadow imports/outer functions correctly.
- True named imports and supported unambiguous same-scope calls still resolve with exact source and target anchors.
- Nested functions and class methods are not promoted to module named exports by an exported enclosing declaration.
- Duplicate declarations remain ambiguous instead of becoming a unique high-confidence edge.
- Snapshot and separate base/head Git indexes apply the same rules; UI and provider evidence preserve unresolved reasons and partial analysis status.
- Record later regression evidence on representative Rust/TypeScript/TSX/JavaScript cases; no runtime verification was performed in this pass.

## Excludes
Runtime call graph inference and pretending to resolve dynamic dispatch.

## Done evidence
Implementation diff, indexer version change, focused ambiguity cases and source-grounded UI/PR evidence inspection when testing is authorized.
