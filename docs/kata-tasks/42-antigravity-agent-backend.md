---
kind: task
key: antigravity-agent-backend
parent: t29
title: Qualify Antigravity account integration and implement the supported adapter
priority: 1
labels: agents, provider
---

## User outcome
Antigravity is evaluated as its own agent backend, with an honest account/API distinction and working chapter-generation controls on the surface actually supported for Code Atlas.

## Research decision — 2026-10-15
Google provides an official Python SDK, headless agy CLI and official antigravity-acp binary. SDK supports custom apps with Gemini API key or Cloud/Enterprise credentials; no consumer OAuth SDK path was established. Official Zed/Xcode ACP integrations support personal Google accounts, but that does not establish eligibility for this independent app. The current FAQ explicitly disallows consumer-plan agy wrapping in third-party apps. Do not silently substitute Gemini CLI or present an API-backed SDK as meeting the user's subscription-login requirement.

## Contract
Evaluate the official Google-distributed ACP binary for a custom-client account path and obtain documented eligibility. Keep SDK/API and ACP/account routes as separate capability records. If account embedding is not supported, show the precise reason and keep this requested route pending; optional SDK credentials are configured once only by explicit user choice.

For an eligible surface, pin runtime/package version and Mac architecture. Discover actual model/reasoning/approval capabilities; do not infer them from generic ACP support. SDK documents model, tool selection, allow/deny/ask-user policies, streaming, structured output and cancellation; its reasoning-field contract needs confirmation. Headless CLI is not an interactive approval channel: its stream input rejects control responses and automatically permits workspace reads/writes. Use a suitable SDK/ACP surface and enforce the reviewed snapshot boundary.

## Acceptance criteria
- Record current official account/custom-client eligibility and exact supported auth path; no scraped/shared consumer token route.
- Supported adapter is selectable as Antigravity; Gemini CLI remains a different backend.
- Effective model, available reasoning options and approval behavior are passed through; unsupported options are marked explicitly.
- One chapter yields bounded structured output validated against snapshot sources before draft acceptance.
- Progress, cancellation, failure and process cleanup work under the packaged Mac app; closing stdin is not claimed as immediate CLI cancellation.
- Document runtime architecture/packaging and prevent arbitrary repository execution or inherited tool/config expansion.
- No automatic API/agent/provider fallback and no claim of subscription support from SDK availability alone.

## Excludes
Consumer-plan headless wrapping contrary to current FAQ, TUI automation and claiming protocol membership proves every runtime capability.

## Done evidence
Eligibility decision, selected adapter/version, config and chapter workflow plus later real-runtime acceptance. Research-only today.

## References
- https://antigravity.google/docs/sdk/overview/
- https://antigravity.google/docs/sdk/tools/
- https://antigravity.google/docs/sdk/policies/
- https://antigravity.google/docs/sdk/lifecycle/
- https://antigravity.google/docs/cli/headless/
- https://antigravity.google/docs/faq/
- https://antigravity.google/docs/ide/extensions/zed
- https://antigravity.google/docs/ide/extensions/xcode
- https://raw.githubusercontent.com/agentclientprotocol/registry/main/antigravity-acp/agent.json
