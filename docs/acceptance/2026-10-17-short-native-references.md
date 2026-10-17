# Short native evidence references — 17 October 2026

## Observed failure

The [previous broad Fontdone result](artifacts/2026-10-17-fontdone-read-diagnostics-partial.json)
ended partial after eight proposals. In proposal five the agent supplied
`evidence:729`, `evidence:2569` and `evidence:90709`, none of which has the required
73-character digest shape. Proposal eight included a well-shaped hash that did
not exist in the delivered evidence. These are observed invalid references,
not proof that all generation failures are transcription errors.

The new transport removes that copying burden before reconsidering the
proposal budget. Eight proposals, six unique sections, 64 total calls,
250,000 distinct source bytes and the original ten-minute deadline remain
unchanged. The agent still investigates the repository and authors the answer;
no local code chooses replacement evidence or removes failed claims.

## Versioned contract

New progressive answers use answer-record schema 2 and `answerSectionV2`.
Their citation transport is `nativeEvidenceV2`: a `read_code` fragment supplies
`referenceId` such as `code:1`. The ordinal is one-based in that operation's
append-only native evidence sequence, including non-citeable fragments.
`reusedReferences` provides the same reference and citeability on overlapping
reads, including overlap within one batch. Cite only `citeable:true` fragments.
The longer `id`/`alreadyAvailable` values remain integrity metadata; the new
proposal schema requests the short `referenceId`.

This is an alias, not a replacement integrity identity. Native evidence IDs,
operation/account/runtime/grant binding, captured hashes, byte ranges, delivery
receipts, source admission, budgets and checkpoint digests are unchanged. The
owner resolves a short reference only from a citeable fragment delivered by
that exact operation. A missing, tiny, unread or uncertain span does not become
evidence. There is no nearest-match lookup, source-quote search, ID shortening
or fallback to another operation. The alias is not a global source identifier.

New references accept canonical ordinals 1–512 with no leading zero. Malformed
references fail shape checks before lookup; well-shaped unavailable references
still fail native admission. Saved section replay reconstructs the alias from
its original immutable checkpoint, then runs the full existing citation,
claim, scene and chapter validation.

Legacy answer-record schema 1 retains `answerSectionV1` and the original
`nativeEvidenceV1` digest references. Inspection does not migrate or replay
inference, and changing a record's version cannot reinterpret a saved proposal.
The original five-tool catalog remains unchanged for legacy requests. The
explicit progressive catalog advertises the new read response and proposal
schema. Request fingerprints include the citation policy/version and the
updated instructions. New and old records still share the existing 4 MiB
answer-file and eight-retained-answer bounds.

## Story instructions

The agent is now explicitly asked to follow the question's requested causal
order, starting at its input before moving downstream. Transformations should
have distinct input/result roles, useful source-backed expressions or labeled
illustrative values. Identical empty labels on both sides of an arrow are not
an adequate transformation. These are generation instructions, not evidence
of resulting semantic quality or a native visual-quality pass.

## Verification

Two new transport/ledger regressions failed before the change. Five focused
checks now cover strict reference shapes, exact source, unavailable IDs,
non-citeable/undelivered spans, same-batch overlap, reuse and unchanged legacy
answer replay. The native progressive publication test uses the new short
reference through the actual owned private MCP path.

## Native packet race

Earlier full suites intermittently failed the 34,000-byte envelope check.
Retaining elapsed time and the peer result exposed a failure in 59 microseconds
with a successful writer, ruling out the suspected 200 ms deadline for that
observation. Stage instrumentation then identified `set_read_timeout` returning
`InvalidInput` / OS error 22, before reading queued source-free fixture bytes.

A separate local reproduction on macOS 15.7.7 reported no timeout-setting
failures with an open peer (100/100), and 100/100 failures after that peer
closed; all 200 pending messages remained exactly readable. Apple's
[matching XNU source family](https://github.com/apple-oss-distributions/xnu/blob/xnu-11417.140.69/bsd/kern/uipc_socket.c)
rejects socket-option changes once both send and receive are shut down. This
supports the observed stage and race; it does not establish the cause of every
historical generic transport error.

The owner now uses standard nonblocking socket reads/writes with waits of at
most 25 ms only on `WouldBlock`. Each complete packet retains its original
absolute 200 ms input/two-second output deadline and Stop checks. Accepted
owner sockets no longer set read/write timeout options. The bridge's separate
result-wait policy is unchanged. No arbitrary socket error is ignored, packet
size increased or failed result reinterpreted.

The deterministic complete-packet/closed-peer regression failed before this
fix and passes after it. A truncated closed-peer packet still fails with a
fixed incomplete-packet message; a cancelled queued packet remains rejected.
Fixed read errors distinguish timeout, incomplete EOF and other transport
failure without retaining arbitrary error text. Temporary stage/error logging
is removed. The first complete fixed suite passes **359 native tests** with
13 explicit live probes ignored; slow fragmented input, Stop and blocked
output checks remain in the suite. Final rerun/build and fresh repository
qualification are recorded below when complete.

## Final mechanical qualification

The final full native rerun passes **359 tests**, with 13 explicit account
probes ignored, in 31.74 s. The frontend suite passes **179 tests**. Ten repeated envelope matrices also
pass **640 cases** in 46.436 s. The
production frontend build and unsigned debug app bundle succeed. The app's
embedded Mach-O signature passes the local executable-integrity check; this is
not Developer ID signing, notarization or resource sealing.

- Executable: `119341616` bytes.
- SHA-256: `12aed600e5e9c8f99bf37fdd880b7dd3a4932fe72ea8836b1ab435c06985d106`.
- The Mac is locked. This bundle has **not** replaced the installed app; native
  reader/playback/accessibility acceptance remains pending.

## Fresh qualification

A fresh isolated copy retains all nine original captured-metadata hashes.
Only the immutable snapshot/object/index/range stores and existing runtime
configuration are copied; prior answers, grants and chapter decisions are not.
The user’s live project is untouched. The probe will use the existing Codex
account and GPT-6 Luna/max, with no API key, new agent installation or manual source
selection. Its output preserves all failed proposals and terminal outcomes
before asserting broad completion. Exact source hashes for the citation,
section, catalog, prompt and owner implementations are recorded with the run.

Qualification is pending. Do not infer broad completion, useful-answer timing,
visual/narrative quality, Pocket speech or native accessibility from the
transport checks.
