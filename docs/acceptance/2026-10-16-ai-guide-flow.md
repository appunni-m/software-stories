# AI-selected code and route chapters — 16 October 2026

## Requested behavior

The reader asks about code without choosing reference files. Codex uses the captured inventory, symbols and resolved links to propose implementation scope. The header owns the saved connection, model and reasoning. Implementation supports behavior claims; documentation provides hints and explicitly declared deprecation labels. An accepted route can generate its remaining chapters as independent drafts with one reviewed batch.

See [the current interaction flow](../AI_GUIDE_FLOW.md), [source discovery acceptance](../kata-tasks/55-question-to-source-discovery.md), and [batch acceptance](../kata-tasks/56-route-chapter-batch.md).

## Evidence and fixes

The native checks use only `benchmarks/codex-account-smoke/cart.ts` in a synthetic repository. No private repository source was transmitted. Model generation used the existing desktop-bundled Codex, GPT-6 Luna, max reasoning and its existing account. No runtime or model was installed; no direct API key was used.

- The updated app restored the account after restart and displayed **Codex · GPT-6-Luna · max** in the header. Discovery and source review did not reopen backend/model selection.
- Discovery chose `cart.ts` from its captured functions and static relationship metadata, and returned a reason. The next source review opened without a reference checklist.
- An early native run exposed an unstable modal lifetime: a parent refresh recreated the planning scope object and cancelled inference. Semantic scope identity now controls initialization; two frontend regressions cover equivalent scope objects and genuine scope changes.
- A later local storage failure coincided with a full disk. Removing this checkout's regenerable debug build cache restored space; project source and saved explanation history were preserved.
- Another generated route failed evidence admission. An isolated account check reproduced a concrete failure: the response declared function identifiers absent from its quotes. A preserved rejected response and regression retain that strict rejection. The planning prompt now spells out the whole-identifier/quote contract, inference basis rules and coherent step sizing. Evidence validation was not relaxed.
- A second isolated live response passed native route canonicalization **without editing the response**. It contained two steps, exact function evidence, a prerequisite, and an inference about the absent post-arithmetic finite check. The response is preserved as [validated synthetic output](../../benchmarks/codex-account-smoke/2026-10-16-validated-plan.json). This isolated check does not save or accept a route.
- Native light/dark appearance and scrolling were inspected. The header remains readable in both palettes; the Guide footer no longer overlaps expanded route content. Full accessibility acceptance remains separate.

## Remaining qualification

The native publication and batch observations are recorded below when exercised. This is a synthetic baseline, not a production-readiness claim. Multi-crate and large repository discovery, missing/partial indexes, overflow windows, persisted discovery reasons, account changes, queue interruption/save boundaries, language narrative quality, visual quality and the complete accessibility matrix remain open in Kata. Claude and other agent discovery remain deferred.
