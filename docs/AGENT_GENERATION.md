# Generate explanations with a coding agent

Coding agents are primary generation options. Direct API keys are optional. The following paths are implemented in source and still need native runtime acceptance; this page does not certify a production release.

**Automatic multi-agent generation is incomplete:** Codex has an automatic source adapter; Claude Agent and Antigravity do not. The [required operation matrix](AGENT_OPERATION_PARITY.md) records the exact gaps and implementation order. Manual handoffs below do not meet that requirement.

| Choice | How the task runs | Configure model, reasoning and approvals | Current scope |
| --- | --- | --- | --- |
| Codex · ChatGPT account | Native managed official app-server | Select advertised model/reasoning in Code Atlas; approval currently fixed to `never` | File chapters, selected-source overviews, one-section repairs, exact code selections, PR Guides and single hunks; runtime/account/package qualification pending |
| Claude Agent · setup check | Explicit native version/account-metadata inspection; no prompt | No generation settings yet | Setup only; automatic generation remains missing |
| Claude Code · manual handoff | Copy task into your own agent, then import JSON | In Claude Code; Code Atlas cannot apply or verify them | File chapters and selected-source overviews; automatic Claude Agent adapter remains required |
| Antigravity or another agent · manual handoff | Same user-operated text exchange | In the external agent; unverified here | Same bounded chapters; automatic vendor integrations remain pending |
| Direct API | An explicitly selected API provider | In its Code Atlas request dialog | Existing API operation matrix; separate API key and billing |

## Codex

1. Capture source, then choose **Explain** on a file or **Create repository tour** from Guide.
2. Choose **Use Codex**. Configure the supported executable and connect your account through the native flow. No direct API key is required for this path.
3. Load its model catalog and select model/reasoning. Catalog membership is not proof of account entitlement or remaining quota.
4. Select and review the immutable source and effective request. Confirm sharing that context, then generate.
5. Review the locally validated preview. **Save unaccepted draft**, then review and accept from the app's editing workflow.

**Change backend** returns to all choices; it discards an unsaved preview. Requests never automatically switch to a paid API. The source adapter covers exact Code Lens selections and Git comparisons/hunks. Its full account, permission, cancellation and packaged-runtime acceptance remains open in `z3an`.

### Explain a selected code range with Codex

In Files, select the captured code range, choose its explanation action and **Use Codex**. The range stays fixed while you choose model/reasoning and review the excerpt. Confirm the reviewed source, then generate. Locally validated claims appear temporarily with actions that open their exact citations. Closing or discarding the explanation saves no chapter draft or history. After interruption, reselect the original range and review again. Native runtime acceptance remains pending.

### Repair one stale section with Codex

In Edit, assess an accepted older chapter against the selected current snapshot. When exactly one section is proven stale and every other section can carry forward, choose that section's regeneration action and **Use Codex**. The declared current source selection is fixed. Review the exact prompt: it includes the previous section and old source as context, plus current source as the only citable evidence. The 250,000-byte source limit counts old and current source together. Select model/reasoning, review and confirm, then **Regenerate section with Codex**. Local validation reconstructs a new unaccepted chapter revision and preserves eligible untouched section/asset identities. Saving and accepting remain separate. Multiple stale sections, a superseded base, changed source/section identity or invalid evidence stops the operation. This route is source-only and has not been run in the native app.

### Explain Git changes with Codex

In **Compare local revisions**, resolve the base/head refs. The PR Guide panel defaults to **Codex · ChatGPT account**. Select 1–12 changed files and **Review with Codex**, or use **Explain hunk** for a single hunk. Configure the installed Codex runtime/account, load models, choose model/reasoning and build the exact request review. Inspect the patches, resolved comparison evidence and effective settings, then confirm sharing and generate. No API key is required.

A validated guide is saved as an unaccepted PR Guide draft. Select **Done** to review its claims before accepting or rejecting. A single-hunk explanation appears temporarily; its citations open the selected base/head hunk. Closing it writes no chapter or guide revision. A late cancellation or lost reply can follow a saved guide, so check PR Guide history before regenerating. The source workflow has not been run or accepted in the packaged app.

Claude Agent is a separate required automatic backend; its PR operations remain missing. Selecting its pending option preserves that choice and offers an installation check. It does not send through a direct API.

### Interrupted requests

**Review interrupted work** restores a recorded Codex file, selected-source overview or exact section repair into Codex. It reloads recorded source paths and the accepted section/base identity, then requires fresh review and consent. It does not resume a runtime thread or automatically send anything. Old records without request-scope metadata require you to reselect the original evidence. Range and Git-change records also require their original range/comparison to be selected again; they never become whole-file generation requests. Navigating away abandons a pending recovery lookup. Native recovery acceptance remains pending.

## Check Claude installation

From the backend chooser, select **Check Claude installation** in the macOS app. Enter the absolute path to your installed CLI and explicitly start the check. It inspects version and, for the candidate adapter version, reported account metadata. The app remembers only the canonical executable path. Cancel or close to abandon the check; browser preview cannot run it.

The result is an installation observation. It does not log in, discover models, prove plan eligibility or start generation. A reported API-key/helper route stays distinct from a reported Claude account. Automatic Claude generation remains required in `8gqw`; this setup screen does not complete it.

## Manual Claude Code or other agent handoff

1. Choose **Use Claude Code handoff**, or the other manual-agent option, from the same generation chooser.
2. Choose the external agent. Set its model, reasoning, account and permissions in that agent. Code Atlas does not start, manage or cancel it and cannot enforce its tools or verify those settings.
3. Select the captured source. A file chapter keeps its target selected. Each task contains 1–12 captured text files and at most 250,000 source bytes; encoded/exported tasks are limited to 512 KB. Native preparation blocks secret-like source.
4. Choose **Prepare local task** and inspect source, instructions, schema and fingerprint. Confirm copying that code, then choose **Copy reviewed task**. Clipboard managers may retain the code. If clipboard access fails, select and copy the task text shown in the dialog.
5. Keep the dialog open and use a fresh agent conversation. Paste the complete task. It requests no commands, file changes or additional reads, but those instructions do not sandbox the external agent. Supervise its permissions and usage in its own interface.
6. Paste the complete JSON response back into Code Atlas, including `handoffId`, `fingerprint` and `chapter`, without Markdown fences. Choose **Validate imported explanation**. The app checks task identity, exact snapshot/source evidence, chapter structure and citations before creating a preview. An invalid import does not change accepted chapters.
7. Inspect the explanation and citations, then **Save unaccepted draft**. Source validation does not establish explanation quality, agent identity, effective model, account use, token usage or runtime permissions. Review and accept separately.

The task expires after one hour or on app restart. Changed selection requires a new task; the old response cannot be imported into it. Source-validation failures can be corrected and reimported against the original unexpired task. Closing or changing backend discards an unsaved preview and leaves external work running. Stop that work in the agent. A lost save response checks for an existing immutable revision; recovery previews remain available for interrupted imports/saves.

This manual path needs no API credential in Code Atlas. Your external agent's normal account rules, access, costs and usage limits still apply. Code Atlas neither provides subscription access nor implements Claude/Antigravity login through a handoff. Do not treat this path as evidence that an automatically embedded account route is approved or implemented.

## Required automatic agent support

Automatic Codex and Claude Agent flows are release requirements, including supported model/reasoning/approval controls, progress, cancellation, recovery and operation coverage. A manual handoff does not complete that requirement. See [the precise agent-first gate](kata-tasks/45-agent-first-generation.md), [the runtime tasks](kata-tasks/29-agent-runtime-backends.md) and [dated integration research](research/2026-10-15-agent-backends.md).
