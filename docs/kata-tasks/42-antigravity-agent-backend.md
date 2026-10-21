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
Google provides an official Python SDK, headless agy CLI and official antigravity-acp binary. SDK supports custom apps with Gemini API key or Cloud/Enterprise credentials; no consumer OAuth SDK path was established. Official Zed/Xcode ACP integrations support personal Google accounts, but that does not establish eligibility for this independent app. The FAQ disallows consumer-plan agy wrapping in third-party apps. Do not silently substitute Gemini CLI or present an API-backed SDK as meeting the user's subscription-login requirement.

## Official recheck — 2026-10-22
Google's current [Antigravity FAQ](https://antigravity.google/docs/faq) says third-party software, tools, or services accessing Antigravity violate its Terms and says wrapping `agy -p` in third-party apps is unsupported on consumer plans. The [Additional Terms](https://antigravity.google/terms) likewise prohibit third-party software/tools/services from accessing the service. This rules out Code Atlas embedding a personal Antigravity login through the ACP binary or CLI unless Google gives an explicit product-specific exception. Editor integrations documented by Google do not grant that exception to this app.

The distinct, documented custom-app route is the official [Antigravity Python SDK](https://antigravity.google/docs/sdk/overview), using a Gemini API key or Gemini Enterprise / Google Cloud credentials. This is an explicit SDK/API or enterprise route, not Antigravity consumer subscription usage; it must be configured separately and never selected as fallback. Do not collect credentials or enable this route until the user explicitly chooses it. Consumer account login remains unavailable pending written eligibility evidence.

## Contract
Keep the consumer account route unavailable unless Google provides written, product-specific eligibility. Do not embed or wrap `agy` or the official ACP binary to access a consumer Antigravity account. Keep this route distinct from the official SDK route. If the user explicitly selects the SDK route, use only its documented Gemini API-key or Gemini Enterprise / Google Cloud credentials, configure them once, and label its billing/account model accurately. Never fall back between these routes or to Gemini CLI/API.

For an eligible surface, pin runtime/package version and Mac architecture. Discover actual model/reasoning/approval capabilities; do not infer them from generic ACP support. SDK documents model, tool selection, allow/deny/ask-user policies, streaming, structured output and cancellation; its reasoning-field contract needs confirmation. Headless CLI is not an interactive approval channel: its stream input rejects control responses and automatically permits workspace reads/writes. Use a suitable SDK/ACP surface and enforce the reviewed snapshot boundary.

## Acceptance criteria
- Record current official account/custom-client eligibility and exact supported auth path; no scraped/shared consumer token route.
- Keep consumer Antigravity account access disabled unless written product-specific eligibility is obtained; explain the official restriction in app-facing connection copy.
- Do not add the SDK-backed Antigravity option until the user explicitly chooses Gemini API-key or Enterprise credentials for that distinct route.
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
