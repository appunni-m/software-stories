# Saved-route replay fixture

This is a frozen copy of an existing synthetic Codex route, accepted route
revision and independently saved chapter, captured from local app storage on
17 October 2026. The source is the same 501-byte shopping-cart example as
[the account smoke input](../codex-account-smoke/cart.ts). It contains no account
configuration, credentials, inference permission or private repository code.

The route has two steps about subtotal and tax validation. Its request already
uses the current explicit-evidence planning instructions. Its native record
predates the shared enum field rename to camelCase. Loading the draft succeeded,
but loading accepted history failed because reserialization changed its digest.

The native regressions install these unchanged files in unique temporary app
storage. They check draft/accepted replay, both exact step scopes, full chapter
source validation and unchanged stored bytes. Separate **synthetic variants**
use the initial and code-first planning contracts from Git history to simulate
an instruction/schema upgrade. Those variants are not additional account runs.

Run from the repository root:

```sh
cargo test --manifest-path src-tauri/Cargo.toml --lib repository_plan::replay_tests
```

The suite needs no running app, agent, account, model or network service. It is
mechanical compatibility evidence, not a whole-repository narrative, speech,
native UI or comprehension assessment. File hashes and reproducer results are
recorded in [the acceptance report](../../docs/acceptance/2026-10-17-saved-route-replay.md).
