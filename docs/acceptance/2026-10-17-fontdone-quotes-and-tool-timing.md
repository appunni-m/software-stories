# Fontdone exact quotes and native tool timing

Date: 17 October 2026. Source base: `e87bfe2` plus this checkpoint. Existing installed Codex account, GPT-6 Luna/max, frozen Fontdone snapshot `snap-1792140879637485000-11155-0` at commit `5249b723719445813b8771a602ea5b96891d4450`. These are attached-function observations; the broad explanation and release gates remain open.

## Preserved failed investigation

Question: “How does Font::memory_face work? Explain its inputs, selection and validation of the face, and the data it creates. Follow necessary implementation dependencies.” The attached `src/font.rs` path was a starting point, without sending the complete file.

Operation `864c8de6-481b-4c1c-baee-a56f68ac1246` took 391.557 seconds in the managed run; the entire test took 394.30 seconds. It delivered 15 definition, nine search and five read calls: 69,653 distinct source bytes, 63,143 metadata bytes and 134,898 native payload bytes from `src/font.rs`, `src/pfr.rs` and `src/tt/mod.rs`. No budget limit notice occurred. Original call timings were unavailable and were not inferred from the coarse phase events.

The first completed response reached validation at 329.459 seconds and needed an identifier correction. One same-conversation correction returned a second candidate at 390.848 seconds. Its identifier findings were empty, but full source admission rejected a quote. The proposed comment had “unknown-format result” at the end of one line and “is the final…” at the start of the next; captured source had “unknown-format result is” and “the final…”. The word moved across a line break. Both candidates contain the same mismatch at `/sections/0/claims/4/evidence/1`.

[Exact public-source candidates, phases and evidence ranges](artifacts/2026-10-17-fontdone-agent-memory-face.json) are preserved. Full captured blob SHA-256 values were verified locally before comparison; all other claim quotes had one exact match in their full named captured file. This local match inspection is not semantic, visual, narration or publication acceptance. No preview was saved; the two-minute first useful answer target fails.

## Change and deterministic checks

The owned correction now includes exact-quote diagnostics before asking for the one correction. Shared matching enforces original bytes, length and unique occurrence, including overlapping matches. Native delivered-range coverage rejects unread quotes. Feedback contains fixed problem codes and JSON positions, without a source passage or path. No whitespace normalization, source replacement or extra context was added. Final independent source, visual and claim validation still owns publication.

The request fingerprints `single-owned-correction-exact-quotes-v2`. The same thread, model, max reasoning, original deadline and cancellation owner are used; event/socket admission continues to refuse tools during correction. Arbitrary schema/visual/semantic failures are not repaired by this preflight.

New source-free ledgers persist native call boundaries under `capturedCodeSessionV1`. The origin is after journal creation, before syntax loading; it excludes earlier queue/account preparation. Start precedes native retrieval, preparation precedes the Prepared journal write, and finish precedes the terminal journal write. Final fsync and outer MCP/event work are not fully included. Legacy timings remain absent/unknown. Native call gaps cannot be attributed to model reasoning or provider latency from these fields alone. The complete stage trace/export/retention/restart contract remains [open](../kata-tasks/71-generation-timing-traces.md).

- Native suite: **294 passed, 11 explicit probes ignored**, zero failures; 30.35 seconds. Log: `/private/tmp/code-atlas-context-quote-native.log`.
- Frontend suite: **142 passed**, zero failures. Log: `/private/tmp/code-atlas-context-quote-frontend.log`.
- Production frontend and unsigned debug app bundle: passed with existing chunk-size/dead-code warnings. Log: `/private/tmp/code-atlas-context-quote-package-final.log`. A first build was deliberately interrupted to restore the documented low-disk Cargo settings; it is not a successful package observation.
- Installed update: `/Users/lazytrot/Applications/Code Atlas.app`; previous app preserved at `/private/tmp/code-atlas-before-exact-quotes-uvy8svzh/Code Atlas.app`. Executable SHA-256: `0cbe33674180ebb95ffe3dcd69d4a07c3aaf3b9f3a181d1c4afe80d5d37cc808`.
- Computer use accessed the unlocked Mac, reopened the updated native app and observed the saved Fontdone child Guide at 3/3, with captured `outline.rs` lines 48–52 still selected. Before replacement, another Flow → Guide cycle retained 3/3. No new question or narration was started through these navigation actions.

This is an unsigned debug bundle, not a signed/notarized release or qualified DMG. The remaining navigation matrix is in [erzc](../kata-tasks/68-exploration-reading-position.md).

## Deliberate retest

A separate attached-function acceptance using the changed native owner and updated packaged helper is running under operation `d15f0817-79a6-4455-9f66-53e41768d336`. It uses the same question/snapshot/model and a fresh explicit request. It is not an automatic replay or resumed inference. Its distinct artifact is reserved as `2026-10-17-fontdone-agent-memory-face-exact-quotes.json`; the probe now refuses an existing artifact before consuming account quota. Results and independent quality review remain pending. No build runs concurrently with this investigation.
