# Coding-agent protocol evaluation — 2026-10-22

This update records the current Agent Client Protocol (ACP) candidates and the
limits of what its registry and compatibility probe establish. It supersedes
only the ACP-version and adapter-provenance observations in the [October 15
backend review](2026-10-15-agent-backends.md). Account eligibility, sandbox
qualification, app behavior, and the Codex-first delivery sequence remain
separate gates.

## Evidence captured

The [ACP v1 registry](https://cdn.agentclientprotocol.com/registry/v1/latest/registry.json)
was fetched at `2026-10-21T23:19:03.156Z`. Its version is `1.0.0`; the SHA-256
of the complete response is
`4995dd1654669b384dd8187004065928a7e25f92187ae8869c6176ceaeec90d7`. The
four exact registry entries are preserved in
[`2026-10-22-acp-registry-v1.0.0.json`](snapshots/2026-10-22-acp-registry-v1.0.0.json).

The ACP maintainers' [protocol adaptation matrix](https://github.com/agentclientprotocol/registry/blob/main/.protocol-matrix/latest.md)
was generated at `2026-10-21T10:13:24Z`. It probed 36 registered agents;
35 completed `initialize`, and 24 returned `auth_required` from `session/new`.
For the four candidates it reported:

| Candidate and pinned observation | ACP implementation and reported capabilities | Decision for Code Atlas |
| --- | --- | --- |
| **Codex** — `@agentclientprotocol/codex-acp@2.2.2`, Apache-2.0 | An ACP stdio adapter that starts Codex App Server and translates ACP requests/events. The probe reported `session/list`, `session/fork`, and `session/resume`. | Keep the existing direct Codex app-server integration for the first release path. ACP would add a wrapper and another runtime to qualify without removing the need to verify app-server permissions, account, and lifecycle. Reconsider only if it materially reduces supported-operation work. |
| **Claude Agent** — `@agentclientprotocol/claude-agent-acp@0.94.0` | An ACP wrapper around the Claude Agent SDK; the probe reported `session/list`, `session/fork`, and `session/resume`. The frozen ACP registry snapshot labels it proprietary, but the exact npm release and included `LICENSE` declare Apache-2.0. | Deferred as requested. ACP does not resolve Anthropic's product/account approval question or qualify the agent's read-only boundary. |
| **Gemini CLI** — `@google/gemini-cli@0.65.0`, launched with `--acp`, Apache-2.0 | The official CLI implements ACP. The probe reported `loadSession`; it did not report `session/list` or `session/resume` for this candidate. | Keep as an optional candidate. The CLI's ACP support is not evidence of repository read-only behavior, account entitlement, or product acceptance. |
| **Google Antigravity** — official registry binary `antigravity-acp@1.4.1`, proprietary | The registry distributes a Google-authored binary for macOS, Linux, and Windows. The probe reported `session/list` and `session/resume`. | Do not use this to wrap a consumer Antigravity account inside Code Atlas. Google's [current FAQ](https://antigravity.google/docs/faq) says third-party software accessing Antigravity violates its terms, and recommends Gemini Enterprise or Google AI Studio API-key credentials for third-party coding agents. That is a separate [SDK/API integration](https://antigravity.google/docs/sdk/overview) and still needs its own permission, packaging, tool-boundary, and workflow qualification. |

The [ACP prompt-turn specification](https://github.com/agentclientprotocol/agent-client-protocol/blob/main/docs/protocol/v1/prompt-turn.mdx)
defines streamed `session/update` events, `session/request_permission`, and
cooperative `session/cancel`; session resume is capability-dependent. Those
are protocol contracts, not proof that a specific adapter implements each
operation correctly. The registry's
`initialize` success, `auth_required` response, and advertised session
capabilities likewise do not prove effective model selection, usage limits,
permission enforcement, cancellation completion, or account terms. The
candidate-specific observations above come from the dated probe. For Claude
Agent ACP, the exact [0.94.0 npm metadata](https://registry.npmjs.org/@agentclientprotocol%2fclaude-agent-acp/0.94.0)
and [0.94.0 package tarball](https://registry.npmjs.org/@agentclientprotocol/claude-agent-acp/-/claude-agent-acp-0.94.0.tgz)
declare Apache-2.0 and the tarball contains the Apache license text. Its
`dist.integrity` is
`sha512-3quBG9QaMKhETZPF0QaD/Ue2C0UCQDzDztDbzs8AVYM8RQKX93TPL0YzJl7sx06qYPLQvxrCDdiAp5a6cgFfZg==`.
The discrepancy is in the ACP registry's license field; release qualification
should pin and verify the exact tarball rather than trust that field.

## Product and safety implications

ACP is a session transport, not a security boundary. An adapter or agent can
have its own file, shell, network, and tool access even when the Code Atlas
client exposes no such ACP tools. For each pinned runtime, acceptance must
prove that it can read only the reviewed immutable staging area, cannot write
or execute commands, and cannot access `.git`, unrelated tools, or arbitrary
network destinations. Permission prompts and a protocol-level `session/cancel`
are insufficient without verifying the runtime's actual behavior and
post-cancel process cleanup. Provider/account traffic must remain on its
documented path and must not grant the agent unrestricted egress.

The ACP registry is useful for discovering maintained candidate integrations;
its 36-agent probe is evidence of registry breadth and current adapter
maintenance, not user adoption or a production compatibility guarantee. ACP
does not remove the product's per-runtime model, reasoning, approval, account,
and lifecycle differences. The upstream [Codex ACP adapter](https://github.com/agentclientprotocol/codex-acp),
[Claude Agent ACP adapter](https://github.com/agentclientprotocol/claude-agent-acp),
[ACP registry](https://github.com/agentclientprotocol/registry), and
[ACP protocol schema](https://github.com/agentclientprotocol/agent-client-protocol)
show ongoing maintenance. GitHub stars/forks and registry size are only rough
ecosystem interest signals, not evidence of deployments or compatibility.
Code Atlas should keep each runtime's settings visible only when that exact
runtime supports and accepts them.

## Remaining work

- Continue Codex qualification through the existing app-server adapter and
  current Codex gates `z3an` and `2r6n`; this protocol review closes neither.
- Keep Claude deferred to `8gqw`. Before selecting its ACP wrapper, retain the
  verified package integrity and account-policy evidence; the registry's
  proprietary label does not match the current Apache-2.0 package.
- Do not count Gemini CLI as an Antigravity substitute or imply that its
  account route is accepted.
- Keep Antigravity consumer-account access unavailable unless Google grants
  written, product-specific permission. Any permitted API-key or Enterprise
  route must be reviewed as its own integration.
- Run pinned-runtime fixtures for progress, permission denial, cancellation,
  process cleanup, resume, model/reasoning forwarding, provider-only egress,
  and read-only source scope before marking any workflow supported.

No ACP runtime was installed or launched, no account was connected, no source
was sent to an agent, and no tests or builds ran for this research update.
