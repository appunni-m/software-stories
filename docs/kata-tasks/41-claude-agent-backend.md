---
kind: task
key: claude-agent-backend
parent: t29
title: Implement Claude Agent chapter adapter and resolve account eligibility
priority: 0
labels: agents, provider
---

## User outcome
The user can choose the Claude Code-powered agent loop, configure its supported model/effort/approvals, and generate a source-grounded chapter through an explicitly chosen account route.

## Research decision — 2026-10-15
Prefer the official Claude Agent SDK behind one managed worker; a documented claude -p stream-json subprocess is an alternative. The October 7 Help Center explicitly permits subscription usage by SDK, -p and third-party apps, while the SDK overview still requires prior approval for third-party claude.ai login/subscription limits. Preserve both sources and resolve custom-product distribution eligibility before advertising approved subscription support. Do not state that subscription usage is technically impossible.

### Eligibility recheck — 2026-10-15

The official pages remain distinct in their wording: the Help Center says third-party apps may use subscription limits, while the SDK overview's product-integration guidance says a third-party developer needs prior approval to offer claude.ai login or subscription limits. Because Code Atlas would expose that login/plan route from its own UI, keep **Claude plan login unavailable until Anthropic confirms this product is approved**. The Help Center confirms that Agent SDK calls signed in with a Claude plan consume plan limits; it does not state that a third-party product is exempt from the SDK overview's approval condition. API-key/Console billing is a separate explicit backend choice and must never be selected as a fallback. [Help Center plan usage](https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan), [Agent SDK integration requirements](https://code.claude.com/docs/en/agent-sdk/overview).

## Contract

Implement automatic dispatch first for file chapters and selected-source overviews, then the section/range/PR operations in the [required parity matrix](../AGENT_OPERATION_PARITY.md). Reuse transport-independent captured context and the existing local validators. The same task owns agent installation/connection state, supported model/effort/approval defaults and overrides, exact review, streaming progress, bounded cancellation/process cleanup and scope-preserving interruption recovery. Claude must be selectable before any direct API setup. An API-key-only Agent SDK route cannot close the user's no-mandatory-API requirement.

This is a required automatic agent integration, not an optional API adapter. The separate manual Claude Code handoff now present in source provides reviewed task copy/import and local evidence validation without an API key in Code Atlas. It does not manage a Claude runtime or account, verify model/reasoning settings, enforce permissions, or provide cancellation. It does not satisfy this task. See the [agent-first release gate](45-agent-first-generation.md).
Pin SDK/binary versions and use the documented auth store/flow with isolated app configuration. Separate account-backed usage from optional API-key usage. Strip inherited credential/provider/endpoint environment that could select API billing; ANTHROPIC_API_KEY overrides account auth in noninteractive mode. Do not assume --bare supports OAuth token behavior. Reuse chapter JSON schema, independent citation validation and explicit draft acceptance.

Expose model-dependent effort, approval behavior and max turns; unsupported values are unavailable. Explicitly configure available tools, empty settingSources and strict app-owned MCP settings. allowedTools is an auto-approval list, not a tool-removal boundary. Remove write/command/web/subagent capabilities and enforce staged-file scope with per-call checks and appropriate sandboxing. Stream progress; interrupt then terminate/reap after a bounded grace period. Never retry through a direct API automatically.

## Acceptance criteria

- All required Claude operation rows in the parity matrix complete through an automatic supported agent adapter; file/overview milestones alone do not close the full task. The API path, manual handoff and a disabled chooser entry are not acceptance evidence.
- Dated resolution records which exact account/distribution path is permitted; login UI follows it.
- Account mode cannot silently pick inherited API credentials or incur API billing; quota/eligibility failures stay on selected backend.
- A real chapter returns structured_output and passes existing local schema/evidence checks; success without structured output is a failure.
- Effective model/effort/permission settings reach the pinned runtime unchanged or by documented mapping.
- Progress, cancellation, invalid output, crash and shutdown preserve history and release process resources.
- Runtime cannot read outside reviewed staging, execute repository instructions, or widen tools via inherited settings/MCP.
- Record packaged acceptance later; no tests/logins/agent execution occurred during this evaluation.

## Excludes
Reimplementing the agent loop, token extraction, invented universal effort options and treating a cost budget as guaranteed zero spending.

## Done evidence
Auth eligibility decision, pinned worker adapter, configuration UI and later account/permission/cancel/chapter acceptance.

## References
- https://code.claude.com/docs/en/agent-sdk/overview
- https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan
- https://support.claude.com/en/articles/17154008-monthly-api-credits-for-max-and-team-plans
- https://code.claude.com/docs/en/authentication
- https://code.claude.com/docs/en/agent-sdk/permissions
- https://code.claude.com/docs/en/agent-sdk/python
- https://code.claude.com/docs/en/agent-sdk/structured-outputs
