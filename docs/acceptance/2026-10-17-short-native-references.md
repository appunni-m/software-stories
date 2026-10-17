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

### Result: partial; completion assertion failed

The [fresh unchanged result](artifacts/2026-10-17-fontdone-short-references-partial.json)
uses source `502e2c2acd71329b3f14441ba401d5759388be60` and operation
`5ddb3d61-1e01-42e4-8250-0c266e5bf9fc`. It returned **Partial** in 160.010 s
through the measured native run/inspection; the complete test took 162.35 s and
exited 101. One independently adopted draft was durably published at 74.578 s
and observed by the probe at 74.951 s. Neither is UI-visible useful-answer timing.

The 33 calls comprise one map, nine searches, 13 definitions, eight reads and
two proposals. Thirty-two delivered; the last read is **uncertain**, with no
prepared payload and no source admitted. The run admitted 46,172 distinct source
bytes from five paths (`api.rs`, `font.rs`, `render.rs`, `tables.rs`, `tt/glyf.rs`).
The two proposals included one identifier/evidence/prose rejection and one
published section. There is no final manifest; parsing, detailed outline/hinting
and rasterization remain unfinished. No deadline, call or proposal limit was
reset, and this failure was not followed by another account attempt.

The last uncertain receipt does not identify its cause. Packet input was
already admitted before that receipt began; the record does not prove a socket
failure, malformed arguments, context exhaustion or model failure. A prior
passing transport matrix cannot turn this into a pass. The original result
has SHA-256 `d2fa901ddd2e363a81bef1c3856980d5c0b9ad047d89054050492574c76752fa`.
All [nine captured metadata hashes](artifacts/2026-10-17-fontdone-short-references-inputs.json)
remain unchanged, all recorded implementation/probe hashes match the tested
source, and independent comparison verifies all eight published anchor byte
ranges/hashes/line coordinates against the captured objects.

### Independent source and representation review: failed

The first transition now begins with font bytes and distinguishes opening a
face from producing pixels. The remaining [agent source/structure review](artifacts/2026-10-17-fontdone-short-references-review.json)
finds material gaps:

- The dispatcher claim generalizes recognized BDF inputs to their constructor
  and other inputs to TrueType. Captured code includes a BDF constructor-error
  return and a separate early rejection for at least 17 bytes all below ASCII space. Those exceptions
  are missing; exact citation admission does not make this generalization true.
- Scene two uses `nodeIndex:2` on both sides of the arrow, with identical empty
  values. “format probes → format probes” does not explain a transformation or
  decision, despite valid HTML structure and the new instructions.
- The complete reading/speech script still enumerates helper names, raw
  underscored identifiers and technical format branches. It does not yet meet
  the requested accessible, meaning-led visual story. No Pocket audio or human
  listening assessment was produced for this run.

These are agent findings, not the required human binary-rubric, visual,
comprehension or accessibility review. The result remains unaccepted. Native
short-reference mechanics worked for the published draft; broad completion,
quality, timing and narration gates remain open.

## Preserve the actual terminal failure in future records

Source review confirms a separate observability defect: the progressive worker
receives its terminal error but, once a draft exists, recovers a partial answer
with a generic topic and discards the error. This explains why the saved
answer cannot recover the original cause; it does not identify that cause.

The follow-up retains an optional fixed `failureCode` in the **private answer
record** when the actual worker failure is observed. It distinguishes known
invalid tool arguments, tool-event mismatch, input timeout/EOF/transport,
uncertain delivery, run timeout and changed runtime; unmapped text becomes
`unknown`. Stop uses its own fixed code. Arbitrary OS/provider/model messages,
paths and source are never stored in this field, and it is not added to the UI
summary. It does not infer a cause from call counts, source size or a receipt.

Original records without the field remain readable. Historical terminal
records are not backfilled or overwritten. Null/unknown diagnostic values and
diagnostics on working/complete records are rejected. Saved section bytes and
all existing ownership, no-retry, scope and size checks remain unchanged.
Four added regressions cover fixed classification, section preservation,
terminal idempotence, absent legacy metadata, untrusted error text and damaged
record metadata. The focused section suite passes 34 tests. Full final
verification and the replacement-bundle identity are recorded below. The fresh
account artifact predates this follow-up, so it is **not** live qualification
of retaining the actual failure code.

### Final follow-up verification

- Full native suite: **363 passed**, zero failed, 13 explicit probes ignored,
  33.36 s. The Stop/diagnostic consistency guard rejects an inconsistent write
  before changing a saved answer.
- Frontend: **179 passed**; production TypeScript/Vite build succeeds.
- Unsigned debug app build succeeds; embedded executable integrity passes.
  Executable: `119393744` bytes; SHA-256:
  `e3460f3e4398c4f8914bc0514c748e41ba0754e7f0dede6ba2d41b985b91f52a`.
- The Mac remains locked. The installed app is unchanged; native navigation,
  accepted-guide export, playback/listening and accessibility remain pending.
- All **73** checked-in task bodies/statuses/priorities match live Kata
  (**50 open, 23 closed**). Three duplicate frontmatter blocks were removed
  from live bodies without changing requirements/status. No task was closed.

No new account request followed the failed broad run. The terminal-diagnostic
follow-up has local regression/build evidence only. The original failed
artifact and all earlier failed results are unchanged.
