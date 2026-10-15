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
The original static findings in `src-tauri/src/index.rs` were missing parameter/local shadow bindings, inherited export flags on nested declarations, and duplicate declaration identities collapsing into one target. For example, `import { run } from './worker'; function execute(run) { run(); }` could resolve the parameter call to the import. These findings now have source changes; regression and language qualification are still pending.

## Implementation update — 2026-10-15

The indexer now uses `atlas-symbol-index-v2`, gathers lexical binding/scope information, marks shadowed/reassigned/ambiguous or unsupported bindings unresolved, checks direct module exports, and retains repeated declaration occurrences. Snapshot and separate base/head comparison indexes share the changed implementation. The new version separates derived index caches from v1 evidence. Source presence does not qualify complete Rust, TypeScript, TSX or JavaScript semantics.

Remaining work, with every acceptance criterion still open:

- [ ] Review and later exercise parameter/default/destructuring, block/catch/loop, hoisting/initialization, reassignment and duplicate-declaration cases across the existing language subset.
- [ ] Confirm genuine supported calls/imports keep exact anchors while uncertain bindings retain an explicit unresolved reason; inspect opaque/unsupported scope behavior for false high-confidence edges.
- [ ] Qualify nested export and duplicate occurrence identity behavior, including source movement and freshness/remapping.
- [ ] Verify v1 indexes are not reused, both Git revision indexes use v2, and Atlas/Code Lens/provider evidence reports partial/unresolved results consistently.
- [ ] Record pinned grammar/runtime versions and actual regression results in the language capability task before advertising support.

No tests, builds or runtime probes ran for this source-only change.

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
