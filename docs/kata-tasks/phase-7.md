---
kind: milestone
key: m7
title: M7 — Selectable AI backends
priority: 0
labels: phase-7, ai, providers, agents
blocked_by: m6, agent-first-generation, antigravity-agent-backend, claude-agent-backend
---

## Current sequencing — 2026-10-16

The immediate milestone is the Codex gate `2r6n`. Claude `8gqw` is deferred by the user and retained as a direct dependency of this full multi-backend milestone, alongside Antigravity. A working Codex route does not close all of M7.

## User outcome
The user can choose the AI integration that fits the work: a direct model API for bounded text-generation requests, a coding-agent runtime for multi-step investigation, or a modality-specific backend for speech. Google Gemini is one provider choice, not the only AI path. The catalog can grow through verified adapters to established providers and agent runtimes.

## Gate definition

Automatic Codex and Claude Agent coverage is a prerequisite, tracked by `2r6n` and its blocking adapter tasks `z3an` and `8gqw`. The [operation parity contract](../AGENT_OPERATION_PARITY.md) defines each required pair. Direct APIs and manual handoffs cannot satisfy it. The separately requested Antigravity integration `m00g` also blocks M7 until its permitted automatic route is delivered and accepted.

Ship a clear backend catalog with separate **Model APIs**, **Agent runtimes**, and modality-specific capabilities such as **Speech**. Direct API calls, agent sessions, and speech synthesis keep their own capabilities, authentication, permissions, data disclosures, cancellation behavior, and usage/billing information. Every AI-enabled feature maps to a supported capability and shows and uses the backend selected for that operation; unavailable features explain why they cannot run on a selected backend. Reuse established maintained clients and official programmatic surfaces where suitable: evaluate a maintained multi-provider client for model APIs, ACP for agent sessions, and MCP only for shared tools/context when needed. Do not create a new general-purpose agent framework or pretend one protocol works for every runtime; record adapter provenance, compatibility, and capability gaps.

## Exit criteria
- Direct text-generation options initially include Google Gemini, OpenAI, and Anthropic APIs, with a documented path to add providers through a maintained multi-provider integration library selected using dated maintenance, adoption, capability, and privacy evidence.
- Agent candidates include Codex, Claude Code, Gemini CLI, and Google Antigravity through supported programmatic interfaces; evaluate ACP registry adapters and vendor-documented SDK/app-server/headless surfaces, and expose a runtime only when its exact version, adapter provenance, account terms, authentication, lifecycle, and read-only boundary are verified. Evaluate Antigravity SDK and CLI separately, and expose CLI only for account plans and terms that permit third-party embedding.
- Users can configure each supported agent runtime's model, reasoning effort/depth, approval policy, and other documented runtime-specific options as per-runtime defaults, with deliberate per-run overrides. Show the effective settings before consent, distinguish agent action approvals from mandatory context consent, and never let an approval choice weaken the read-only boundary.
- Keep agent provider-control traffic separate from agent-tool egress: permit only the selected adapter's reviewed authentication/model endpoints through an app-managed broker or OS-enforced destination-restricted path; deny arbitrary network destinations, web/search/fetch tools, and unrelated MCP. Authentication/setup sends no repository context; inference requires fresh context consent. Prove the permitted provider path and denied destinations for each pinned runtime, or leave it unavailable.
- M7 completion requires a selectable, tested route for the named backends Codex, Claude Agent powered by Claude Code (using Anthropic's allowed product branding), and Google Antigravity through a permitted API-key or Enterprise path. If terms or safety prevent one route, keep M7 open and document that blocker; do not count an unavailable catalog entry or another vendor's API as fulfilling the request. Gemini CLI is an additional Google agent-runtime candidate.
- The capability matrix covers every AI-enabled product operation; speech remains a separate modality/provider path under t18, not an implied feature of a text-model adapter.
- The matrix maps every text-generation and coding-agent workflow—including chapter/orientation generation, selective regeneration, Code Lens explanations, and PR guide/change explanations—to its supported backends; unsupported combinations remain unavailable with an explanation.
- The user sees the files/context that can leave the Mac, the selected provider or runtime, applicable terms, and known or potentially multi-call usage before starting a run.
- Exact-context consent, local draft validation, explicit human acceptance, cancellation, and no automatic fallback apply across both integration types.
- Agent runs cannot modify the selected working copy or read unapproved repository content. An agent that cannot meet the verified permission boundary is shown as unavailable for repository explanation.

## Current research artifacts

- [AI backend capability matrix, separating implemented routes from accepted support and missing operations](../research/ai-backend-capability-matrix.md)
- [Dated Rust model-client comparison, `rig-core` adapter decision, and remaining acceptance evidence](../research/2026-10-13-rust-model-client-comparison.md)

## Excludes
Building a new model gateway, inventing an agent protocol or tool loop, silent provider routing/fallback, collecting vendor credentials through browser scraping, and treating API providers and agent runtimes as interchangeable.

## Child issues

- `c8nz` — [28-model-api-providers.md](28-model-api-providers.md) adds selectable direct model API providers.
- `tpta` — [29-agent-runtime-backends.md](29-agent-runtime-backends.md) adds selectable read-only coding-agent backends.

## Done evidence
Review the dated provider/runtime capability matrix and the selected existing libraries/protocols, with evidence for adoption, maintenance, and unsupported features. Complete one consented direct-API draft and one consented read-only agent-backed draft; verify context, disclosure, permission boundary, local evidence validation, cancellation, and manual acceptance for both. Verify provider failure never switches backend and prior guide history remains intact.

## Pending work — 2026-10-15

Pending: c8nz full direct-operation/provider acceptance and tpta verified supported agent adapters/configuration. Google is one selectable option; local Pocket TTS is owned separately by 70x5.

## Implementation order — October 15

Start Codex account-backed app-server `z3an` first. Claude Agent `8gqw` and Antigravity `m00g` have separate auth-eligibility and runtime tasks under `tpta`. Agent implementation no longer waits for all direct API operations; release acceptance remains explicit. See [the updated evaluation](../research/2026-10-15-agent-backends.md).
