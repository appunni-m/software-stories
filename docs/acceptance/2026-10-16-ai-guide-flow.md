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

## Executed checks

| Check | Result |
| --- | --- |
| `npm test` | 120 passed. |
| `npm run build` | Passed; existing bundle-size warning remains. |
| `CARGO_INCREMENTAL=0 cargo test --release --manifest-path src-tauri/Cargo.toml --lib` | 195 passed, three opt-in live checks ignored. |
| Live synthetic Codex planning check | Corrected response passed native canonicalization unchanged with GPT-6 Luna/max. |
| `npm run quality:corpus` | Six synthetic cases across three repositories passed earlier in this iteration; this does not measure real generated narrative quality. |
| `CARGO_INCREMENTAL=0 npm run bundle:macos` | Unsigned app/DMG build and metadata/checksum verification passed. |

Installed app: `/Users/lazytrot/Applications/Code Atlas.app`. Executable SHA-256: `b7ef3b6b43f92f486cfb842b4861933a4cd1c8d71f926f8376d4f574cd2c52e5`.

## Final packaged route observation

The installed final build silently restored the account. From **Guide → Ask the code**, discovery selected `cart.ts` from the index and automatically opened the source review. Generation saved **Subtotal and taxed totals**, a two-step unaccepted route. Opening the draft revalidated it; both steps showed exact function quotes, and the second step showed the resolved `totalWithTax → subtotal` call as static index evidence. Back/Next stayed under reader control. The known unknown concerned the unstated tax-rate unit convention.

After reviewing both steps, the route was explicitly accepted as revision `plan-revision:3be55a8a-1d3e-446f-958d-19766a2784f6`. **Build all chapters** prepared two exact requests locally. Both included only the same 501-byte synthetic `cart.ts`; the batch retained GPT-6 Luna/max and the header connection. A single start action began sequential generation.

The first packaged batch stopped at chapter one: a diagram quote was not attached to a validated claim, and no chapter draft was saved. The generation contract was then changed to section-local claim/evidence references. Validated claim anchors remain the only source destinations; legacy quoted diagrams retain strict matching. New regressions cover correct source hotspots, malformed/out-of-range/duplicate references and rejection of invented legacy quotes. Batch results also gained a direct **Review chapter** action. The source changes passed 195 Rust tests and 120 frontend tests before rebuilding.

The next unsigned app/DMG build passed metadata and checksum verification from source commit `d2600b6`. After installing it and restarting, the header silently restored GPT-6 Luna/max, the accepted route reopened, and its batch prepared the same two source scopes without running discovery again. The original failed batch had saved no draft; the ordinary same-file accepted chapter was correctly not treated as a generated route step.


The next native batch saved its first chapter as draft `revision:145ce15d-5bac-4a10-a949-6238cac05025`, with the exact accepted route/step lineage, three sections and two validated diagrams. The second chapter stopped on an identifier/prose mismatch: a claim declared `subtotal` without naming it in its explanation. The first draft remained saved. Direct chapter review and diagram hotspot navigation were not exercised before the user redirected the work to workspace design. The saved first-step narrative also covered both functions despite its narrower subtotal question; scope discipline remains a narrative-quality gap. This run is partial evidence, not a successful whole-route batch or visual-quality acceptance.
