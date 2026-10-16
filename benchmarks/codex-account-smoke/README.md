# Synthetic Codex account checks

`cart.ts` is a small synthetic TypeScript implementation. It contains no project or account secrets.

- `2026-10-16-native-proposal.json` is the previously recorded chapter output, exercised by a local canonicalization regression.
- `2026-10-16-rejected-plan.json` records a diagnostic GPT-6 Luna/max route response. It was rejected: claims named functions absent from their evidence quotes. The regression preserves that rejection and checks that exact complete evidence is required. It does not represent a saved or accepted route.
- `2026-10-16-validated-plan.json` records a second live response after clarifying the prompt. Native canonicalization passed without modifying its response. It contains two code-backed steps and a source-grounded uncertainty about arithmetic overflow. Validation alone does not mean the route was saved or accepted.

## Optional live planning check

This ignored test uses the selected installed Codex account and its quota. It creates an isolated synthetic repository, requests GPT-6 Luna with max reasoning, validates the result, then removes the temporary repository and managed runtime. It does not save or accept a plan. The output path receives only the synthetic response, including when source validation rejects it.

```sh
CODE_ATLAS_LIVE_CODEX='/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex' \
CODE_ATLAS_LIVE_PLAN_OUTPUT=/tmp/code-atlas-synthetic-plan.json \
CARGO_INCREMENTAL=0 cargo test --release --manifest-path src-tauri/Cargo.toml --lib \
  repository_plan::live_tests::installed_codex_synthetic_plan -- --ignored --exact
```

Change the executable path to an existing compatible Codex installation. The test does not install a CLI, open sign-in, choose another model, or use an API key. It exercises generation and local validation; packaged UI, durable publication and chapter batching need their own acceptance checks.
