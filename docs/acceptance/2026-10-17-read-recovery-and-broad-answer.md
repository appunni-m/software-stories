# Read recovery and broad Fontdone answer — 17 October 2026

## Confirmed failure and correction

The previous [Fontdone partial answer](artifacts/2026-10-17-fontdone-complete-script-partial.json)
published one section and then recorded a rejected read without its native
cause. That receipt cannot identify the original failure retrospectively.

Source review found that an empty batch or more than eight read ranges failed
the investigation, while wrong bounds for an eligible path already returned
correctable metadata. The new regression reproduced the range-count abort:
`Only a range-read response may include one to eight source ranges.` It failed
before the batching fix, then passed with the fix. This establishes a real
recoverability defect, not the cause of the historical Fontdone rejection.

Read-count mistakes now return `invalidRangeCount` with no code, path, source
hash or size. The agent can correct/split the request inside the original
budget. The owner does not truncate a batch, choose replacement files or infer
an answer for the agent. Unsafe paths and unowned calls remain hard failures.

Rejected native deliveries also retain a fixed `rejectionCode`. Arbitrary error
text is mapped to `unknown`, never stored in this diagnostic. Legacy receipts
without a code remain readable, and codes on non-rejected calls are invalid.
The field records native delivery failure, not model semantics or provenance
for unseen source. See the [retrieval contract](../architecture/repository-retrieval.md).

## Verification scope

- Focused corrected-read suite: three tests passed, including the actual
  private MCP transport, source-free feedback and a subsequent exact read.
- Native suite: 349 passed, zero failed; 13 explicit account/runtime probes were
  ignored. Final validation after qualification is recorded below.
- The first compile exposed a Rust error-type inference ambiguity in the new
  classifier; the preparation result now has an explicit `String` error type.
  That compile failure is separate from the reproduced runtime regression.
- Test logs: `/private/tmp/code-atlas-read-count-before-fix.log`,
  `/private/tmp/code-atlas-read-count-fixed.log`,
  `/private/tmp/code-atlas-read-diagnostics-native-tests.log`.

## Broad account qualification

The ignored `fontdone_progressive_account_probe` asks the full font-bytes to
bitmap question, including entry points, parsing, outline loading, hinting and
rasterization. It uses the existing Codex account and GPT-6 Luna/max, with no
API key, new agent installation or supplied source-file selection. The complete
captured snapshot, syntax and text indexes were copied into isolated local app
storage. Its new grants, receipts and adopted draft sections do not change the
user's saved project. Snapshot metadata hashes are recorded separately.

The probe preserves the complete answer, rejected proposals, evidence ledger,
timings, exact native source/probe hashes and terminal failure before asserting
completion. A partial result fails the completion assertion. Mechanical
completion still requires independent source review and human narrative/
listening assessment; it does not pass the quality gate by itself.

### Result: partial, completion assertion failed

The [unchanged result artifact](artifacts/2026-10-17-fontdone-read-diagnostics-partial.json)
records operation `9fd1f79d-ecf7-4422-925b-ecac13ad6585`: **387.523 seconds**
through the measured native run and inspection; the complete test took 391.43
seconds and exited 101 because `Partial` is not `Complete`. The exact artifact
SHA-256 is `6c87df008ea6f4020eae90a05e9829d9c35b6b3dab778ce69945852191d19616`.
All 39 tool calls were delivered: two maps, 26 searches, three reads and eight
proposals. They admitted 123,317 distinct source bytes from `src/api.rs`,
`src/font.rs`, `src/grays.rs`, `src/render.rs` and `src/tt/glyf.rs`.

| Published draft | Context preparation | Durable publication | Probe adoption observed |
| --- | ---: | ---: | ---: |
| Character lookup and rendering entry | 98.981 s | 99.582 s | 99.936 s |
| Opening bytes and SFNT parsing | 190.589 s | 191.197 s | 192.016 s |
| Loading branches and hint selection | 339.876 s | 340.507 s | 341.735 s |

These are native/probe observations, **not UI-visible useful-answer timing**.
All three adoptions succeeded. Five proposals were rejected; the eighth and
last proposal, for outline-to-pixel behavior, still had an invalid citation.
The final manifest explicitly leaves TrueType decoding/scan conversion,
hint-interpreter/auto-hinter internals, the final bitmap handoff and CFF/CFF2
paths unfinished. No budget was reset and no replacement investigation was
started. This run did not exercise an invalid read count or native read
rejection, so it does not identify the cause of the historical rejected read.

All nine original snapshot metadata hashes still match their
[recorded inputs](artifacts/2026-10-17-fontdone-read-diagnostics-inputs.json).
Independent byte/digest comparison checked all 20 published source anchors
against the isolated captured objects. Exact source admission is not semantic
or narrative acceptance.

### Content review and remaining work

- Character lookup, the empty LCD return and the normal renderer call agree
  with their captured entry code. The opening sequence nevertheless starts
  with glyph loading and explains font bytes second, so it is a poor ordered
  story for this question.
- The SFNT claim describes `parse_maxp` and `parse_cmap` default recovery but
  calls it an “optional parse.” Their **table entries are required** before
  that recovery. The claim does not explicitly say a missing table succeeds,
  but omitting that distinction makes this contrast ambiguous. Review missing
  entries separately from malformed contents; do not treat this as a passed
  required/optional-table quality case.
- The flag normalization, early SVG/bitmap branches and hint selection match
  the cited `api.rs` branches. Naming the selected helper does not explain
  what hinting or rasterization means; those deeper requested steps remain
  unfinished.
- The first visual repeats the same node label on both sides of an arrow in
  its first two scenes. The presentation is admitted HTML, but it does not
  communicate an actual transformation clearly. Long function identifiers
  remain in the complete narration script. No Pocket clip or human listening
  pass was produced for this broad answer.

### Confirmed scene-feedback defect and correction

The second proposal used a claim-1 node in a claim-2 scene. Native scene
validation correctly rejected it, but the tool returned only
`/section: sectionValidation`. A synthetic two-claim regression reproduced
that generic result before the diagnostic change. After native rejection,
the owner now recognizes this specific mismatch and returns fixed
`sceneEvidence` at the content element's `nodeIndex` position. It emits no
label, excerpt, model value or suggested source. Other validation failures
still use their existing checks/findings. The canonicalizer and scene
validator remain authoritative; the diagnostic does not admit a visual.

The regression preserves the complete rejected proposal, verifies that no
section was published, and then publishes a corrected node reference in the
same answer. Another test covers source-free feedback, multi-claim nodes,
graph scenes and unknown shapes. The previously failed broad artifact is
unchanged. Its account run preceded this diagnostic and prompt clarification;
**no fresh broad account run has qualified those later changes**.

The eight-proposal bound is unchanged. This sample exhausted it after only
three published sections, despite remaining retrieval/time capacity. Kata
`3bz5` retains a requirement to evaluate correction allocation and the cost of
invalid citations using this preserved run, with bounded storage and original
ownership/call/time controls. Increasing a cap alone is not broad completion.

## Final validation and package

- Final native suite: **351 passed, zero failed, 13 explicit probes ignored**.
  This includes the strengthened delivered/rejected-state checks and bounded
  scene diagnostics. Account completion remains failed as recorded above.
- Frontend: **179 passed**, zero failed. The production TypeScript/Vite build
  passed as part of `npx tauri build --debug --no-sign --bundles app`.
- The unsigned debug bundle built successfully at
  `src-tauri/target/debug/bundle/macos/Code Atlas.app`. Its 119,320,144-byte
  executable passed embedded Mach-O integrity verification. SHA-256:
  `140f6183dd9c74a8a8fe6fe1112695e15f8aece378725337343219f366ff47c0`.
  This is not Developer ID, resource-seal, notarization or release acceptance.
- The bundle contains the read recovery and later scene diagnostic/prompt
  changes. **It has not replaced or restarted the installed app:** computer
  use reported a locked Mac. The existing app and saved reader state were left
  intact; fresh installed navigation/helper/playback acceptance remains open.
- The build still reports existing dead-code and large-frontend-chunk warnings.
  Tests/build do not establish responsiveness or accessibility.
- The Open Source documentation inventory reports zero structural errors and
  six review findings (including the missing root license and claim wording).
  It does not establish semantic quality, licensing or rendering acceptance.
- Live Kata was read back after synchronizing `3bz5`, `gk46`, `q0sm` and `j53m`:
  **73 records, 50 open, 23 closed**. No task was closed.

Final logs are `/private/tmp/code-atlas-read-scene-final-native-bounded.log`,
`/private/tmp/code-atlas-read-scene-final-frontend.log`,
`/private/tmp/code-atlas-read-scene-debug-build.log` and
`/private/tmp/code-atlas-read-scene-documentation-audit.json`.
