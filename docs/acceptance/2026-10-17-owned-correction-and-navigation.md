# Owned Codex correction and saved-reading navigation

Date: 17 October 2026. Source base: `84289c2`, plus the correction changes committed with this report. This is a narrow acceptance record; broad repository narrative, semantic review, speech, accessibility and release acceptance remain open.

## Same-conversation correction

The native workspace adapter uses the existing installed Codex account and **GPT-6 Luna/max**. Its request carries intent and starting-point metadata, with an empty initial source bundle. The synthetic probe deliberately asks for a lexical mismatch in the first answer after reading the captured fixture. The native adapter detects the mismatch, retains that candidate privately, and requests one correction in the same thread. The second turn cannot acquire additional repository context.

| Observation | Result |
| --- | --- |
| Whole synthetic test | Passed, 22.64 seconds |
| Measured managed run | 19.714 seconds; not a performance benchmark |
| Delivered code | One `read_code` call, 197 source bytes |
| Corrections | Exactly one |
| Conversation | Same thread, two distinct turn identities |
| Original candidate | Preserved unchanged, with `/sections/0/claims/0` identifying missing `greeting` in prose |
| Corrected candidate | Preserved; lexical findings empty |
| Independent validation | Exact source, claim and visual admission passed |
| Additional context in correction | Denied by native event and socket admission; no second source call occurred |

[Exact public synthetic candidates, receipt and resulting revision](artifacts/2026-10-17-codex-owned-correction.json).

The first deliberate synthetic probe reached correction, then failed independent section-content validation. The test prompt had omitted section-prose/narration requirements that exist in the production prompt. [Failure record](artifacts/2026-10-17-codex-owned-correction-incomplete-test-prompt.json). Its exact candidates are unavailable because the first probe cleaned its temporary fixture before exporting them. The probe now saves candidates and terminal outcomes before cleanup. The successful second run followed that test-prompt repair, used unchanged production validators, and had no concurrent build. There is no production inference retry or new broad Fontdone run in this acceptance.

Private production records are separate from chapters and source-free diagnostics. They retain at most two candidates per failed operation, eight operations and 2 MiB per operation, without silently evicting failures. They bind request/snapshot/runtime/grant identities and the delivered-evidence digest. Raw candidates contain source and are not default diagnostic exports. A user-facing inspection/removal control and packaged quota handling remain pending under [gk46](../kata-tasks/70-codex-retrieval-tools.md).

## Packaged navigation

Computer use accessed the unlocked Mac. Before replacement, the installed app reproduced the reported bug: a saved Fontdone section reset from 3/3 to 1/3 after Flow → Guide.

The updated **unsigned debug bundle** was built with `npm run tauri -- build --debug --no-sign --bundles app` and installed at `/Users/lazytrot/Applications/Code Atlas.app`. The previous bundle is preserved at `/private/tmp/code-atlas-before-correction-p9jb1kb0/Code Atlas.app`. This is not a signed/notarized release or a DMG qualification. The saved Fontdone snapshot is `snap-1792140879637485000-11155-0`, commit `5249b723719445813b8771a602ea5b96891d4450`.

| Manual path | Observed result |
| --- | --- |
| Saved child answer section 2 → Flow → Chat history → Guide | Returned to 2/3, “When `is_empty` returns true” |
| Parent section 2 → saved stopped child branch → Back | Returned to parent 2/3 |
| Sibling answer section 3 → root breadcrumb | Parent retained 2/3 |
| Sidebar returns to sibling | Sibling retained 3/3 independently |
| Quit on sibling Flow, reopen app | Same exploration and Flow selected; section 3 content shown |
| Guide after restart | 3/3, “What the source does not prove” |
| Saved citation after restart | Opened `src/outline.rs`, selection “Rust Lines 48–52” |
| Saved account after restart | Header resumed Codex without a connection-selection dialog |
| Playback | Listen remained idle; navigation did not autoplay narration |

These navigation actions did not send a question, create an explanation or accept a revision. Stopped/failed historical questions remained visible. One root/child nesting level and independent sibling state were exercised; a third level, independently selected available Flow diagrams, precise scroll restoration, replaced revisions and damaged persisted state still need packaged coverage. Deterministic state tests cover several of these cases, but are not substituted for that remaining manual matrix. [Reading-position task](../kata-tasks/68-exploration-reading-position.md) stays open.

The accessibility tree still exposes legacy hidden heading copy and historical failure details in chat history. These observations do not establish that every workspace page, graph or accessibility behavior is polished. The earlier broad Fontdone failure still has no accepted answer.

## Checks and reproduction

- Full native suite: **286 passed, 11 explicit probes ignored**, zero failures; final rerun 26.04 seconds. Log: `/private/tmp/code-atlas-owned-correction-native-complete.log` (the earlier passing run took 25.79 seconds).
- Frontend suite: **142 passed**, zero failures. Log: `/private/tmp/code-atlas-owned-correction-frontend.log`.
- Production frontend build and unsigned debug app bundle: passed. Log: `/private/tmp/code-atlas-owned-correction-package.log`. Existing frontend chunk-size/native dead-code warnings remain.
- Live probe: `workspace_generation::tests::codex_owned_correction_probe`, explicitly ignored by default; uses account quota. Logs: `/private/tmp/code-atlas-owned-correction-live.log` (first failure) and `/private/tmp/code-atlas-owned-correction-live-final.log` (pass).
- Documentation inventory: zero structural errors, five review findings. These include the pre-existing missing root licence and wording requiring evidence; the flagged “production-ready” wording in this checkpoint explicitly denies that claim. No licence was invented. Log: `/private/tmp/code-atlas-owned-correction-documentation-audit.json`.

The live probe used the previously preserved packaged MCP helper, whose packet protocol is unchanged, and the newly compiled native owner. It does not qualify a full parent generation inside the installed bundle. No additional application server or port was started. The remaining work is tracked in [gk46](../kata-tasks/70-codex-retrieval-tools.md), [erzc](../kata-tasks/68-exploration-reading-position.md) and the broader chapter-quality and lifecycle tasks.
