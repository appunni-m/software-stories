# Repository planning contract

## Current state

Repository planning is incomplete. Source now contains a transport-independent planning context, JSON result contract, local validator, registered local preparation command and separate Codex/Claude planning review commands. There is no app planning dialog, registered planning sender, saved AI plan, multi-request orchestration or accepted native evidence. The existing call-based reading planner remains a separate feature. All agent planning cells in the [operation matrix](AGENT_OPERATION_PARITY.md) remain **Missing**.

The implementation is in [repository_plan.rs](../src-tauri/src/repository_plan.rs), [its local command](../src-tauri/src/repository_plan_commands.rs) and [the typed client](../src/lib/repositoryPlans.ts). No new agent loop, model catalog, API credential flow or service was added. No tests, builds, app launches, CLI/SDK probes, logins or model requests ran for this change.

## One bounded planning unit

`prepare_repository_plan_context` accepts one `input` object with `projectId`, `snapshotId`, `question` and `selectedPaths`. It requires an open project and an immutable snapshot belonging to it. The learning question contains 8–300 characters. Source selection uses the shared chapter evidence loader: 1–12 distinct captured files, at most 250,000 source bytes, integrity checks and bounded local index evidence. Planning additionally rejects ineligible captured paths and secret-like selected contents. The encoded prompt/schema contract is limited to 512,000 bytes.

The result shows the exact selected source, question, filtered symbols/relationships, index status/truncation, prompt, schema, byte count and approximate input token count. `contextSha256` binds the planning data and prompt version; `requestBodySha256` binds the complete transport-independent prompt/schema envelope. Neither is a source consent, stored agent review or inference authorization. The command starts no agent and sends nothing to a provider.

The inventory digest covers sorted captured-file, skipped-path and directory records plus snapshot/capture-policy identity. Absolute repository roots are not included in model context. Coverage counts selected files, other eligible captured files, excluded captured records and unique skipped paths. Eligibility describes captured metadata; it does not establish behavior or inspect unselected contents. A partially captured repository stays visibly partial. Preparing all eligible files still does not establish that they have explanations.

## Native agent request reviews

The registered `build_codex_repository_plan_review` and `build_claude_repository_plan_review` commands prepare distinct `repositoryPlan` operations. Their strict nested inputs contain project/snapshot, learning question and selected paths, plus the chosen agent's settings. They reuse the existing native pending-review store and agent contract builder. Neither starts inference, reads an API key, grants consent or authorizes a planning sender. The typed client has no app planning caller yet.

| Agent | Native identity and settings |
| --- | --- |
| Codex | Configured installation path comes from native status. Review records selected model/effort and fixed `never` approval; runtime/model/account qualification remains separate. |
| Claude Agent | An opaque current native connection supplies executable/account/catalog identity. Supported model/effort, fixed `dontAsk` and turn bounds are checked against that observation. Connection replacement/refresh invalidates publication and insertion caps expiry to that connection. This is still an observed connection, without qualified inference. |

`generationConfig.repositoryPlan` retains the exact question, capture status, inventory/context/planning-contract digests, coverage and index status. The review fingerprint also binds the complete agent contract, selected source and settings. Source-only rebuild helpers require the correct agent/endpoint/purpose/contract and an unexpired review, rebuild from native source/runtime observations, and compare every review field after retaining its original ID and expiry. Rebuilding cannot renew admission. These helpers still need the actual planning dispatch/lifecycle integration.

The shared result bridge checks review/fingerprint/model identity, bounds the response, reopens the same native context and calls the independent plan validator. Its provider/model/usage labels depend on a future qualified native dispatcher; this helper cannot prove which agent produced arbitrary JSON. It saves nothing. Full native preflight, active-run revalidation, owned progress/cancellation and durable recovery must precede a planning sender. Existing chapter/range/Git senders reject the distinct planning purpose. The direct API command boundary now checks backend and operation before fingerprint removal, source revalidation, credential loading or consumption, preserving a foreign agent review after a wrong-command attempt. These are unexecuted source changes.

## Generated plan and local validation

The separate plan result contains `contextSha256`, a specific title, 1–24 steps and 1–8 anchored unknowns. Each step has a short local ID, learning question, primary source path, prerequisite IDs and one grounded claim. Questions end in a question mark. Prerequisites may name only earlier steps, preventing forward references, self-dependencies and cycles. They represent a learning order; they cannot prove an execution or data-flow edge.

Each claim uses the existing chapter evidence schema. The validator reuses the chapter quote/identifier/relationship checks rather than accepting invented anchors. Exact unique quotes must exist in the selected captured bytes and name the identifiers stated in the rationale. Relationship IDs must come from the reviewed resolved evidence and pass the existing independent source/index checks. Every primary path must occur in that step's verified anchors. Unknowns must be labeled inference with a bounded basis and their own evidence. Unsupported module, protocol, language and runtime boundaries cannot become proven relationships because a model suggests an order.

Validation reopens the native preparation input and rejects changed inventory, context, request contract or selected index evidence. It bounds response bytes, rejects malformed JSON, unknown/duplicate typed properties, invalid IDs and excessive collections. Native code assigns draft, step, claim and anchor identities; model local IDs cannot become global destinations. Planned paths come from the steps' validated primary files. Merely citing a supporting file or an unknown does not give it a dedicated step: cited paths are listed separately, and selected files without a primary step stay omitted. These sets are computed locally, independently of any model coverage claim.

The validator returns unaccepted data. It does not persist anything, accept a chapter, mark a file explained or validate human comprehension. Agent/account/model provenance must come from the eventual agent dispatcher; the local validator cannot identify which tool produced arbitrary JSON. Questions and pedagogical quality still require review.

## Required next integration

1. Connect the native Codex and Claude plan reviews to a simple app flow with actual supported settings, connection/account identity, expiry and fresh source consent. Reuse their official runtime adapters. No mandatory API key, manual copy/import or silent provider fallback can satisfy this step.
2. Add automatic plan dispatch, progress/cancel, model/result validation and exact interruption metadata including the original learning question and selected source. Retain recoverable failures and late-result handling without widening scope.
3. Add durable native plan draft/history storage and a simple app review flow. Separate plan acceptance, reading progress and actual per-file chapter coverage. Give each step explicit Back, Next and Explore source actions.
4. Generate chapters in reviewed bounded units, tracking omissions and recoverable failures across a larger repository. Add selective updates using actual source dependencies. One planning response or a twelve-file overview cannot qualify a complete repository guide.
5. Complete the deferred acceptance cases in `xymb`, all required agent operation cells and representative narrative/comprehension reviews before closing `6qk9` or the agent release gate.
