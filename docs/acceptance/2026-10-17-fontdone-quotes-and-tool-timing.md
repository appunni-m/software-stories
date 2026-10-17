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

Operation `d15f0817-79a6-4455-9f66-53e41768d336` used the same question/snapshot/model with the changed owner and updated packaged helper. It was a fresh explicit request, with no concurrent build, automatic replay or resumed inference. Its [separate exact result](artifacts/2026-10-17-fontdone-agent-memory-face-exact-quotes.json) is **failed**: 380.156 seconds managed execution; 382.40 seconds whole test; no preview.

| Observation | Result |
| --- | --- |
| Retrieval | 26 definition, seven read and three search calls |
| Delivered bytes | 48,698 distinct source; 68,777 metadata; 118,844 native payload |
| Native call durations | 12.568 seconds total; longest call 1.022 seconds |
| Last native tool finish | 113.301 seconds on the context clock |
| First agent-message delta | 286.377 seconds on the outer probe clock |
| First candidate validation | 316.027 seconds on the outer probe clock |
| Single correction | Started at 317.271 seconds; returned for validation at 379.455 seconds |
| Identifier findings | Seven initial findings; corrected candidate empty |
| Quote result | Both candidates fail exact source at `/sections/0/claims/3/evidence/0` |

The original draft omitted the intervening comment in the TrueType/WinFNT fallback quote. The correction restored a comment but moved `is` across the line break again. Every other claim quote had one exact captured-file match and delivered-range coverage in the local inspection. This is not a semantic/visual/narration pass; preflight refused the second candidate before complete chapter admission. The two-minute useful-answer gate fails again. Different retrieved evidence and changed integration mean the two samples do not establish a speedup or regression distribution.

The [timing companion](artifacts/2026-10-17-fontdone-memory-face-native-timing.json) copies source-free counters/boundaries from the persisted ledger and adds the post-run quote findings. It excludes account/runtime identity details, source, queries and reasoning. The original probe artifact omitted the clock label; future probe artifacts now include it. The native context and outer probe clocks have different origins; their values are not an aligned full-stage trace. The gap outside native calls cannot be assigned to model reasoning or provider latency.

The failed preflight candidates were preserved, but their per-candidate rejection strings were null. A follow-up change now retains bounded fixed problem codes and positions when the sole correction fails; it never adds source to that diagnostic or changes old observations. A new deterministic regression covers this path. The final native suite passes **295 tests**, with 11 explicit probes ignored, in 26.28 seconds (`/private/tmp/code-atlas-context-quote-native-final.log`). No additional inference was run for this diagnostic change.

## Packaging recovery and installed follow-up

The final diagnostic rebuild exhausted local disk space. A subsequent standalone bundle command reported success but contained a truncated Mach-O executable: 96,116,736 bytes, while its load commands expected 115,613,728. That bundle did not launch. The working app was immediately restored and reopened; saved repositories, explanations and connections were unchanged. The truncated artifact is a failed observation, not a successful package. Generated build caches were cleared to recover headroom; no captured repository or explanation was deleted.

The complete compiler output was retained and verified. After restoring it and recovering disk space, `tauri bundle --debug --no-sign --no-binary-patching --bundles app` succeeded (`/private/tmp/code-atlas-context-quote-bundle-space-recovered.log`). The source executable, restored root executable and final bundle executable independently matched **115,613,728 bytes**, SHA-256 **`08556fe5ba4c9aa4ce190c458b16c56360e0538465e4a6ad180a65bda48cb0ae`**. `codesign --verify --strict --ignore-resources` accepted their embedded executable integrity signatures. This check excludes resource sealing, Developer ID identity and notarization; using the no-patching flag alone is not evidence of integrity.

The Mac packaging verifier now checks executable integrity before accepting bundle metadata or the DMG. Three regressions cover an existing signed executable, a truncated executable retaining its header, and a missing executable. The final frontend suite passes **145 tests**, zero failures (`/private/tmp/code-atlas-context-quote-frontend-final.log`). The documentation audit reports zero structural errors and six review findings, including the existing root-license decision and broad-claim wording requiring review; it is not documentation-release acceptance.

The verified final debug app replaced `/Users/lazytrot/Applications/Code Atlas.app`. The working rollback is `/var/folders/ht/wsllq4_d55x89lcbwb5yvqpc0000gn/T/code-atlas-working-before-verified-update-zf9k7vpt/Code Atlas.app`. Computer use opened the replacement, observed the saved Codex connection without setup, and opened the saved Fontdone child exploration. Flow was restored on entry, then Guide restored section **2/3** and captured `outline.rs` lines **48–52**. No inference or narration was started by this check. The full navigation, accessibility, signed release and clean-account gates remain open.

## Next required change

Repeated model copying/reformatting is the underlying reliability gap. [Kata yk6c: native evidence references](../kata-tasks/72-agent-evidence-references.md) specifies that the agent chooses native reference IDs and the app resolves exact delivered bytes/coordinates locally, while retaining full source, identifier, visual, account and cancellation validation. It requires versioned migration, bounded payloads, fault regressions and fresh native quality/navigation/Pocket acceptance. The implementation and those gates remain pending. Another prompt-only retry was not started.
