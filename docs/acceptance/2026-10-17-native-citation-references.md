# Native citation references

Date: 17 October 2026. Source base: `cc83987` plus this checkpoint. This follows the two [failed frozen Fontdone investigations](2026-10-17-fontdone-quotes-and-tool-timing.md). It changes citation transport, without certifying explanation quality or completing the broad-repository acceptance gate.

## Contract

Workspace investigation requests fingerprint `codex-retrieval-v2-native-citations`, output `citationVersion: nativeEvidenceV1`, and use `single-owned-correction-native-citations-v3`. Claim evidence contains only `{referenceId}`. Codex chooses an ID from a delivered `read_code` fragment; the app supplies the exact original excerpt and coordinates locally. Model-written paths, quotes, offsets, hashes and extra evidence properties are refused.

The range reader partitions newly delivered gaps into fragments of at most 1,000 bytes, preferring line boundaries and preserving UTF-8 and CRLF. Fragments with fewer than eight bytes or only whitespace explicitly have `citeable:false`. These cannot justify a claim. Adjacent or overlapping reads reuse the existing delivered IDs and send only previously unread bytes; fragmentation does not widen the requested range. A claim may cite up to eight suitable fragments. This version does not synthesize a larger citation across previously separate fragments; short/blank fragments remain unavailable for citation.

New ledger schema 2 binds each ID to the complete operation/project/snapshot/manifest/grant/account/runtime/model/effort binding and captured range identity. A valid-looking ID for actual but unread code does not resolve. Only Delivered calls contribute references; Prepared, Uncertain, rejected or absent delivery cannot justify a claim. Stop, runtime/account changes, revoked grants and missing/corrupt captured blobs still stop admission. The same source/file/call/fragment/metadata/payload/deadline limits apply; added ID/flag metadata counts against the original budgets. No extra server, provider or source preload was added.

Resolution reads the immutable captured blobs, verifies exact range/digest/coordinates, and supplies an internal type that cannot be deserialized from provider JSON. The chapter builder rechecks that type against its full captured sources. Explicit verified coordinates disambiguate repeated occurrences; the legacy quote route retains its original uniqueness rule. Final independent claim, source, diagram and storage checks still run. Valid references do not prove semantic accuracy, meaningful arrows or good narration.

One owned correction can fix reference/identifier findings using the existing conversation and already delivered IDs. It cannot call tools or restart research; account/model/max reasoning, cancellation and the original deadline remain owned by the same run. Recovery schema 2 retains the raw candidates and bounded fixed preflight codes/positions. Historical recovery schema 1 remains readable. No raw failed response is rewritten into an accepted chapter.

New generation origins use schema 4 and the new retrieval contract. Old schema-3 retrieval origins, schema-1 ledgers and quote-based chapters remain readable under their original contracts; historical records cannot resume generation or acquire new references. Sharing policy/limits are unchanged, so this transport update does not request another connection setup or extend a source grant.

## Qualification

The full native regression suite passed **304 tests**, with 11 account/native probes explicitly ignored. The frontend suite passed **145 tests** and the production frontend build passed. Regressions cover the two failed comment passages, repeated implementation coordinates through full chapter validation, source-linked diagram construction, bounded delivery, foreign/unread/uncertain references, revocation/corruption and legacy identity compatibility. These checks establish transport and deterministic admission; they do not establish explanation quality.

The unsigned debug Mac app packaged successfully and was installed after its source and bundle executable matched: **115,741,104 bytes**, SHA-256 `29868c4bb55c66a1f953d5dcad5a602c01a998b31a73d9d54540b85260934450`. Embedded executable integrity passed, and computer use confirmed the installed app opened with the saved Codex connection and nested Outline Guide still on section **2/3**, with source lines **48–52** selected. Switching that legacy answer from Guide to Flow and back also preserves section 2/3. The Flow displays an explicit absence of verified static links for this file; this check does not establish generated visual quality. This verifies preservation of that existing reading state; it does not qualify generation through the new citation contract or a signed release.

The frozen Fontdone investigation below failed; complete code-fact/visual qualification, fresh generated citation navigation and Pocket narration remain pending. Record their outcomes separately before closing [Kata yk6c](../kata-tasks/72-agent-evidence-references.md). The previous Fontdone failures and their two-minute target failures remain unchanged.

## Harness preflight failure

The first explicit probe stopped at `thread/start` in **0.947 seconds** of measured native work (3.48 seconds for the whole test). It delivered **zero calls and zero source bytes**, returned no answer, and is preserved in its [separate artifact](artifacts/2026-10-17-fontdone-agent-memory-face-native-citations.json). The test command omitted `CODE_ATLAS_CONTEXT_HELPER_BIN`, so the test-only helper override defaulted to the Rust test executable. Invoking that executable with the app's `--atlas-context-mcp` argument reproduces `Unrecognized option: 'atlas-context-mcp'`. This was a harness setup error; it does not qualify the new citation protocol.

The three explicit account probes now require an absolute existing executable helper distinct from the test runner before account access or artifact reservation. The focused offline workspace checks pass (3 tests; the account correction probe remains ignored), and explicitly launching the Fontdone probe without the variable now refuses before account setup or artifact creation. Production continues to use its own application executable. The corrected explicit check used `/Users/lazytrot/Applications/Code Atlas.app/Contents/MacOS/code-atlas-desktop`; no runtime installation, account reconnection or model fallback is involved.

## Frozen Fontdone result: still failed

The correctly configured GPT-6 Luna/max investigation used installed production helper bytes from source checkpoint `9fa1ca5`; its parent test differs only by the explicit helper preflight guard in `41bf90a`. It investigated frozen snapshot `snap-1792140879637485000-11155-0`, commit `5249b723719445813b8771a602ea5b96891d4450`, through the existing account and range-sharing grant. No heavy build ran concurrently.

The [complete public-source artifact](artifacts/2026-10-17-fontdone-agent-memory-face-native-citations-helper.json) preserves both exact candidates, findings, evidence and call timings. The [source-free measurement summary](artifacts/2026-10-17-fontdone-memory-face-native-citation-timing.json) records its SHA-256 `6fafd715ba0604607b68f8186b05c02ac64576b36dd3be4c6eb30445d215f147` and candidate digests. Operation: `da7f3bb9-b3a8-4ce1-973e-e5ccd9916340`.

| Observation | Result |
|---|---|
| Whole native run | **358.714 s**, failed; whole test 361.36 s |
| Delivered calls | **40**: 22 definition, 8 read, 10 search |
| Source / metadata / payload | **53,291 / 110,896 / 165,699 bytes** |
| Captured source paths | `src/font.rs`, `src/tables.rs`, `src/tt/mod.rs` |
| Aggregate / longest native tool work | **18.542 s / 1.742 s** |
| Last native call completed | **92.239 s**, separate captured-context clock |
| First answer delta / first preflight | **229.125 s / 258.780 s**, outer run clock |
| Correction start / first delta / preflight | **270.063 / 323.996 / 351.580 s**, outer run clock |
| Initial / corrected candidate | 3 sections, 15 claims, 40 citation entries each |
| Native reference findings | **None** for both candidates; all IDs resolved against delivered captured spans |
| Identifier findings | **10 → 1** after the sole same-conversation correction |
| Published answer / two-minute target | **No answer; target failed** |

The two clocks have different origins. Tool durations do not measure provider inference or explain the entire remaining interval. The complete stage trace remains pending in Kata `ps02`.

### Remaining source-alignment failure

The corrected `/sections/2/claims/3` declares `parse_type1_metadata`, but its prose says “parses type1 metadata” instead of naming that identifier. It also declares and describes `non_sfnt_outline_font_data` without citing a fragment containing it. Its fourth reference resolves exactly to `src/font.rs:3031–3053`, intermediate Type 1 design-map code, which contains none of the claim's eight declared identifiers. The reference transport worked; the agent chose unsuitable evidence. The strict claim gate correctly withheld publication. No local claim rewriting, reference substitution, validator relaxation, extra correction or automatic investigation replay was used.

### Separate content and visual review

The corrected first section describes the actual ordered `memory_face` dispatch and the final TrueType/WinFNT fallback, including the 17-byte control-stream and 118-byte retry guards. This is a limited source comparison, not full constructor/narrative qualification.

A semantic error remains in `/sections/1/claims/3`: its plain-language sentence calls tolerated malformed table data **optional**, although the same cited implementation requires `maxp`, `cmap` and `hmtx` entries and defaults certain parse failures only after those entries exist (`src/font.rs:4698–4727`). Missing required tables and malformed contents are different behaviors. That claim passed lexical alignment; this demonstrates why citation/identifier validity cannot replace the separate content gate.

Only section zero proposes a visual: four nodes and three labeled edges for request, ordered probes, a format constructor and final fallback. It does not diagram the face-index/named-instance decisions or constructor/error branches. The other two sections use `visual:none`. No draft was published, so fresh installed diagram clicks, generated narration and Pocket audio were **not qualified**. The separately emitted `spokenText` fields are discarded by this workspace path; they must not be reported as the spoken script the app would actually play.

### Next implementation gate

[Kata `3bz5`: publish validated answer sections while the owned agent continues](../kata-tasks/73-progressive-answer-sections.md) defines bounded native section proposals, durable evidence checkpoints, partial outcomes, background job ownership, reader behavior and real timing/quality acceptance. It remains unimplemented. The failed candidates above remain failed historical observations; they are not silently salvaged into an accepted guide. Native citations `yk6c`, retrieval `gk46`, timing `ps02`, chapter quality `q0sm` and packaged generation/narration remain open.
