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
The original static findings in `src-tauri/src/index.rs` were missing parameter/local shadow bindings, inherited export flags on nested declarations, and duplicate declaration identities collapsing into one target. For example, `import { run } from './worker'; function execute(run) { run(); }` could resolve the parameter call to the import. These targeted findings are corrected and covered by the regression evidence below. Broader language and narrative qualification remains in task 7xx7.

## Implementation update — 2026-10-15

The indexer now uses `atlas-symbol-index-v2`, gathers lexical binding/scope information, marks shadowed/reassigned/ambiguous or unsupported bindings unresolved, checks direct module exports, and retains repeated declaration occurrences. Snapshot and separate base/head comparison indexes share the changed implementation. The new version separates derived index caches from v1 evidence. Source presence does not qualify complete Rust, TypeScript, TSX or JavaScript semantics.

Static follow-up on 2026-10-15 found that any Rust `visibility_modifier` was treated as a public export, including restricted forms such as `pub(crate)` and `pub(super)`. The check now accepts only bare `pub`, which is the only unambiguous crate-root visibility. Rust items inside inline modules are still not marked exported; module-chain visibility is not resolved. The correction is covered by the new parser regression matrix; inline module visibility remains intentionally unresolved.

## Validation update — 2026-10-15

The lexical-resolution regression matrix now exercises TypeScript, TSX, JavaScript, and Rust fixtures. It covers parameter/default/destructured/catch/loop/block bindings, callable and non-callable local shadows, reassignment, initialization order, duplicate declarations/exports, nested declarations, exact source anchors, restricted Rust visibility, and the v1-cache boundary.

The separate base/head Git regression verifies that each revision independently applies lexical shadowing and that the resolved head call retains exact revision, path, blob, and evidence anchors.

Validation completed after the restricted-Rust-visibility correction:
- `cargo test --manifest-path src-tauri/Cargo.toml --lib lexical_resolution_is_conservative_across_the_shipped_grammars -- --nocapture`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib base_and_head_revision_indexes_apply_lexical_shadowing_independently -- --nocapture`: passed.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`: 179 passed, 0 failed, 1 ignored.
- cargo fmt --check and git diff --check: passed.

Source review confirms unresolved reasons remain present in Code Lens, Atlas detail, provider review data, and generated-chapter evidence. Unresolved/dynamic links remain outside the proven Atlas canvas; incomplete index status is described as incomplete provider evidence. This is source and unit-test evidence, not a native packaged-app acceptance run.

The pinned parser versions and ABI metadata were already recorded in the language-capability task. These tests resolve the targeted lexical binding/export correctness gap; they do not qualify complete semantics or explanation quality for the four languages. Those broader capability and narrative criteria remain in task 7xx7.

## Contract
Resolve a call only when its exact lexical scope has one statically supported binding. Shadowed, dynamic or ambiguous bindings stay unresolved with a reason. Module exports must reflect actual exported bindings. Preserve duplicate occurrences or explicitly record ambiguity. Bump the indexer semantics/cache version so old relationships are not silently reused.

## Acceptance criteria
- Parameters, local variables, destructuring and block-scoped bindings shadow imports/outer functions correctly.
- True named imports and supported unambiguous same-scope calls still resolve with exact source and target anchors.
- Nested functions and class methods are not promoted to module named exports by an exported enclosing declaration.
- Duplicate declarations remain ambiguous instead of becoming a unique high-confidence edge.
- Snapshot and separate base/head Git indexes apply the same rules; UI and provider evidence preserve unresolved reasons and partial analysis status.
- Record regression evidence on representative Rust/TypeScript/TSX/JavaScript cases; completed in the Validation update. This does not qualify complete language semantics.

## Excludes
Runtime call graph inference and pretending to resolve dynamic dispatch.

## Done evidence
Implementation diff, indexer version change, focused ambiguity matrix, base/head Git-index regression, full Rust library suite, and source-grounded UI/provider evidence review. Broader language qualification remains tracked in 7xx7.

## Scope of recorded validation — 2026-10-16 audit

The dated validation counts above are historical records, not a new run against the current checkout. No tests or builds ran during this audit. They do not qualify the later chapter-version-4, agent-runtime, narration, packaged-app or complete language/narrative changes. Current release acceptance remains governed by the open parent/capability tasks and the deferred cases in `xymb`.
