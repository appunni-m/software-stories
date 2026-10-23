# Chapter schema

This document describes the current version 4 chapter payload used by Chapter Studio, validation, linked visuals, and the walkthrough player. The reader also supports strict legacy chapter payloads v1, v2 and v3; future versions are rejected. Visuals have their own version number: v2 is current, while v1 visuals are preserved only on legacy accepted revisions. The Rust structs in `src-tauri/src/chapters.rs` are the serialization authority; the matching TypeScript interfaces are in `src/lib/commands.ts`.

## Revision shape

A chapter revision is one immutable, snapshot-pinned value. Its JSON uses camelCase keys and rejects unknown fields. The payload contains a stable `chapterId`, a unique `revisionId`, a one-based `revisionNumber`, the repository and snapshot identities, a stable target ID and typed overview/file/group scope, lifecycle and validation status, and an ordered section array.

```json
{
  "schemaVersion": 4,
  "chapterId": "chapter:request-flow",
  "revisionId": "revision:request-flow:1",
  "revisionNumber": 1,
  "projectId": "<canonical repository identity>",
  "snapshotId": "<captured snapshot identity>",
  "target": {
    "targetId": "target:provider-flow",
    "scope": { "kind": "group", "paths": ["src-tauri/src/provider.rs"] }
  },
  "title": "How a reviewed request is sent",
  "lifecycle": "draft",
  "validationStatus": "unvalidated",
  "createdAtUnixMs": 1800000000000,
  "sections": []
}
```

`basedOnRevisionId` is set on every Chapter Studio edit, acceptance, or rejection revision. `decisionNote` is present only on a rejected revision. When either optional value is absent, its JSON key is omitted. Version 1 JSON omits both keys, and the version 1 reader rejects either key even when its value is `null`. A generated first draft has no parent; later changes append a new immutable revision rather than replacing the saved value.

Chapter Studio transitions are append-only: edit creates a draft from the currently latest revision, accept creates a validated accepted revision from a saved draft, and reject creates a rejected revision with a reason while preserving the draft. A rejected revision cannot be accepted directly; repair creates another draft first. A stale snapshot remains readable but cannot be accepted as current code.

`target.scope` is a tagged union: `overview` has no path, `file` has one captured `path`, and `group` has a non-empty, duplicate-free list of captured paths. Section `order` is zero-based and contiguous. `sectionId` stays stable when that conceptual section is carried forward; `sectionRevisionId` identifies that section's exact content revision. `claimId`, `anchorId`, `hotspotId`, visual `nodeId`, and `edgeId` are stable IDs within their documented scopes. IDs are 1–128 ASCII letters, digits, colons, underscores, or hyphens and start with a letter or digit.

## Generated file-chapter proposal

File generation accepts a separate, strict proposal, defined by the `GeneratedChapter*` structs in `src-tauri/src/chapters.rs`. It describes one selected target file and can cite up to eleven explicitly selected supporting files. Every property below is required at its respective level; unknown properties are rejected. The local request requires JSON output, one candidate, temperature 0.2, and at most 8,192 output tokens. The selected Rig provider wire maps the schema and generation settings to Gemini `responseSchema`, OpenAI Responses structured output, or Anthropic Messages structured output; the returned proposal is still validated locally. Repository-overview generation is a separate request purpose and canonicalization path; this subsection documents the file-target proposal.

```json
{
  "title": "How greeting formats a caller's name",
  "sections": [
    {
      "title": "Formatting the greeting",
      "prose": "greeting formats the supplied name into the returned greeting string.",
      "spokenText": "greeting returns a greeting string containing the supplied name.",
      "claims": [
        {
          "category": "codeBehavior",
          "aspect": "responsibility",
          "prose": "greeting formats the supplied name into the returned greeting string.",
          "inferenceBasis": "",
          "identifiers": ["greeting"],
          "evidence": [
            {
              "path": "src/greeting.rs",
              "quote": "pub fn greeting(name: &str) -> String { format!(\"Hello {name}\") }"
            }
          ],
          "relationshipIds": []
        }
      ],
      "visual": {
        "kind": "none",
        "title": "",
        "direction": "auto",
        "nodes": [],
        "edges": [],
        "presentation": null
      }
    }
  ],
  "evidenceLimits": [
    {
      "prose": "greeting's implementation does not establish which production callers invoke it.",
      "basis": "The selected source contains the function body but provides no production-caller evidence.",
      "identifiers": ["greeting"],
      "evidence": [
        {
          "path": "src/greeting.rs",
          "quote": "pub fn greeting(name: &str) -> String { format!(\"Hello {name}\") }"
        }
      ]
    }
  ]
}
```

`category` is `codeBehavior`, `repositoryContext`, or `inference`; an inference requires a nonempty `inferenceBasis`. `aspect` is `responsibility`, `execution`, `inputOutput`, `stateEffect`, `failurePath`, `dependency`, or `relationship`. A chapter needs a responsibility claim anchored to the target. Every section and every evidence limit must cite the target file. Each claim declares distinct, nonempty identifiers or configuration keys that occur as complete identifiers in its prose and in its cited quotes; every quote must contain at least one declared identifier.

Ordinary Codex file chapters, overviews, planned file chapters and selected-section repairs fingerprint `candidateCorrection: {"policy":"chapter-layout-and-identifiers-v2","maxAttempts":1}` into the native review, including requests without a chat question. Before source admission, bounded checks can request one correction in the same ephemeral Codex thread. The follow-up reports identifier problems and fixed positional composition-tree codes: invalid parents, element/content/child counts, nesting, empty groups and content in layout elements. Tree checks share the native validator's rules; their feedback does not copy source paths, excerpts, scene labels or values.

The correction uses the original source context, model, reasoning, schema, permissions, cancellation owner and ten-minute deadline, and cannot fetch new files. It does not patch or admit output locally. The complete corrected JSON still passes normal quote, dependency, visual and independent storage checks. Persistent known findings, cancellation or transport failure stop the operation; malformed or unknown shapes remain subject to full admission. There is no automatic new generation. This structural check does not prove visual meaning or teaching quality. [Scope and retained evidence](acceptance/2026-10-17-codex-layout-correction.md).

Historical retrieval requests retain `claimAlignmentCorrection: {"policy":"identifier-alignment-v1","maxAttempts":1}` and their original diagnostics. Progressive chat keeps its bounded section-proposal mechanism rather than using this final-chapter correction. Direct API, Claude, discovery, planning and PR requests do not acquire the ordinary Codex chapter policy.

An evidence item contains only a selected captured `path` and an exact `quote`. The quote must occur exactly once in that file, including overlapping occurrences, and its whitespace and UTF-8 bytes are preserved. Repeated, missing, empty, unselected, or malformed evidence rejects the proposal. The app locates each verified quote in the captured bytes and constructs its snapshot identity, source hash, inclusive byte offsets, line positions, byte columns, optional enclosing index symbol, and source dependencies locally. The model does not supply a revision ID, anchor ID, source hash, offset, lifecycle, validation status, saved visual ID, or hotspot ID.

A section can propose a bounded `visual` (`none`, `dataFlow`, `sequence`, or `hierarchy`). Current generation schemas require each node/edge's evidence to contain references such as `{"claimIndex":0,"evidenceIndex":0}`. Both indexes are zero-based within that same section's claims and their evidence arrays. The app validates the claims first, resolves existing references to exact anchors, then constructs diagram and source-link IDs. Missing or duplicate references, foreign fields, malformed indexes and invalid graph edges reject the proposal. A reference does not establish that a diagram's semantic interpretation is correct; content review remains necessary. Legacy visual proposals using exact `{path, quote}` items remain readable with their original strict claim-attachment checks. Stored visual schema and anchor destinations are unchanged.

Fresh file chapters, repository overviews and single-section repairs request
`presentation`, either `null` for no composition or a version-3 narrated
[visual scene composition](VISUAL_SCENES.md). The prompt identities are
`structured-file-chapter-v5`, `structured-repository-overview-v5` and
`structured-single-section-update-v3`. The shared context builder supplies this
contract to account-agent and direct-provider chapter requests. Existing
five-tool retrieval schemas and progressive tool catalogs retain their pinned
historical identities.

Ordinary and progressive requests share the current scene and teaching
instructions. Progressive contracts fingerprint the current source policies
`storyPolicy: "ai-html-compositions-v37"`,
`teachingPolicy: "meaning-first-v27"`, and the exact composed instructions
without changing stored checkpoint identities or replaying generation. A
fingerprint change applies to future requests; it does not rewrite saved guides.
These are generation instructions, not semantic acceptance.
[Implementation evidence and remaining quality gaps](acceptance/2026-10-17-shared-teaching-policy.md).

After source canonicalization, the fresh-preview boundary assembles displayed
prose and speech from **all** anchored claim sentences, preserving inference
labels and sentence order. The complete script must be 20–4,000 UTF-8 bytes;
an oversized answer fails rather than losing later behavior. The proposal's
shorter `spokenText` remains a required compatibility transport field under
its existing 1,000-byte bound. It does not select which claims are narrated.
Evidence-limit sections remain silent. A selective repair prepares only its
selected section; carried sections, custom scripts and stored revisions are
not rewritten by this policy. Legacy presentations remain readable under
their original validation, but a fresh result cannot downgrade to version 1.

Relationship claims cite IDs copied from the reviewed `selectedRelationships` context. Each relationship carries its indexed type, confidence, source and target, exact source excerpt, and unresolved reason when present. The request filter includes only relationships whose source and any resolved target refer to selected, verified indexed files or symbols. The app rechecks the cited IDs, hashes, ranges, and selected endpoints, then appends the actual indexed type and confidence to the saved claim. Unresolved targets retain their unresolved status; static index evidence does not establish runtime calls or outcomes.

Proposal limits are stricter than general revision limits:

| Item | Generation limit |
| --- | --- |
| Chapter title | 8–120 characters and a specific subject |
| Sections | 1–12, plus one app-created evidence-limits section |
| Section title / prose / spoken text | At most 120 / 4,000 / 1,000 UTF-8 bytes; prose is at least 20 bytes |
| Claims per section / claim prose | 1–20 / 24–2,000 UTF-8 bytes |
| Identifiers / evidence quotes per claim or limit | 1–8 each |
| Exact quote | 8–1,000 UTF-8 bytes |
| Relationship IDs per claim | At most 8 |
| Evidence limits / limit prose / limit basis | 1–5 / 24–800 / 12–500 UTF-8 bytes |
| Generated proposal text | At most 300 KiB |

The Gemini response reader bounds the HTTP body at 1,000,000 bytes, requires a completed `STOP` candidate, excludes thought parts, and enforces the 300 KiB chapter-text limit. Truncated, blocked, malformed, oversized, or locally invalid responses create no chapter draft.

## Request review, preview, and explicit save

The request review displays the exact system instruction, user prompt, generation configuration and schema, selected source bytes, and selected index evidence before sending. It also identifies the provider and endpoint, and shows the encoded JSON body and its SHA-256 digest; the credential header is omitted. Source text comes from the immutable snapshot, never a later worktree read. Selection is limited to twelve distinct text files and 250,000 source bytes; secret-like paths, binary files, invalid UTF-8, and blobs that fail integrity checks are refused. The exact encoded request body is limited to 512,000 bytes. Selected index context is bounded at 200 symbols and 300 relationships, with target-file evidence first; a partial index or omitted evidence is marked incomplete.

The fingerprint covers provider identity and endpoint, the encoded request-body digest, project, snapshot, target, selected model, disclosure and prompt versions, system instruction, exact user prompt including source and index JSON, generation settings and response schema, and each selected file's path, recorded hash, size, and locally recomputed content digest. Reviews expire after fifteen minutes. The app rebuilds the review from the selected snapshot before sending, verifies its exact body and fingerprint, requires complete provider consent, and consumes the review once. A retry requires a fresh review.

A successful response is decoded, canonicalized, and validated against the captured source and saved index before it enters a pending preview. The preview is `draft` with `validationStatus: "valid"`; it is not appended to chapter revision history and is not accepted. A recovery copy is persisted as a separate JSON record under Tauri app data so it can be reviewed after restart. The recovery store retains at most 16 previews, with a 2 MiB per-preview limit and a 16 MiB total limit. Its in-memory review handle expires after fifteen minutes and is restored only when the preview is explicitly loaded; loading revalidates its evidence against the pinned snapshot. Restoring a preview does not restore provider consent, the request body, or credentials. A user can save it as an unaccepted local draft or discard it.

Only the explicit **Save local draft** action adds that pending revision to chapter history. Saving revalidates its snapshot evidence and appends a new immutable JSON file with the next revision number for the chapter target. A saved draft reloads from its pinned project and snapshot. Regeneration creates another review and pending preview, then a separate saved revision when explicitly requested; it never replaces a prior payload. This generation flow has no acceptance action and never sets `lifecycle: "accepted"`.

## Claims and evidence

Each section stores `title`, explanatory `prose`, independently editable `spokenText`, claims, explicit dependencies, an optional typed visual, and typed hotspots. Empty `spokenText` remains allowed for legacy or manually authored sections where narration is absent. Newly generated behavior sections receive the complete grounded claim narration described above; the app-generated evidence-limit section remains silent. TTS audio is still optional and is never requested until the reader reviews and consents to that request.

Claim categories are tagged values:

- `{"kind":"codeBehavior"}` describes behavior directly visible in source and requires one or more exact anchors.
- `{"kind":"repositoryContext"}` describes repository context and may include anchors.
- `{"kind":"inference","visibleLabel":"Inference","basis":"..."}` records both the visible inference label and the reason for the inference.

An anchor has `path`, optional `symbolId`, `snapshotId`, lowercase SHA-256 `sourceSha256`, an exact `excerpt`, and inclusive byte, line, and UTF-8 byte-column ranges. Lines and columns start at 1. `endByteInclusive` points to the last included byte. Validation loads the immutable captured blob, verifies its size/hash, then checks that the range, line/column values, and excerpt all select the same captured bytes. It does not accept a current worktree path as a substitute.

Anchor offsets are built locally from verified evidence. For parser evidence, the index's half-open `[startByte, endByte)` range becomes the chapter's inclusive `[startByte, endByteInclusive]` range after the app verifies the snapshot hash and exact UTF-8 slice. For a generated proposal, the app locates the unique exact quote in the captured bytes and computes the same inclusive range and positions. A model or editor does not invent offsets.

For the source line `pub fn answer() -> i32 { compute() }`, the call anchor shape is:

```json
{
  "anchorId": "anchor:answer-compute-call",
  "path": "src/lib.rs",
  "symbolId": "sym:<stable symbol id>",
  "snapshotId": "<snapshot id>",
  "sourceSha256": "<64 lowercase hex characters>",
  "startByte": 25,
  "endByteInclusive": 33,
  "startLine": 1,
  "endLine": 1,
  "startColumn": 26,
  "endColumn": 34,
  "excerpt": "compute()"
}
```

Dependencies are typed as `sourceFile`, `symbol`, or `relationship`. Each records the captured path and file hash; symbol and relationship dependencies also record the stable index ID. The validator checks each dependency against the pinned snapshot manifest and, when needed, its saved symbol index.

## Typed visuals and destinations

A visual is data for the built-in renderer, not a document or program. Interactive visual version 2 permits `dataFlow`, `sequence`, or `hierarchy` diagrams made from labeled nodes and directed edges. It requires an app-owned `layoutHint.direction` (`auto`, `leftToRight`, or `topToBottom`). Every node and edge has one to eight `evidenceAnchorIds`; each ID must resolve to an exact source anchor attached to a claim in the same section. Nodes may also reference a `hotspotId`.

Version 1 diagrams remain preserved only on an already accepted, validated chapter revision. The current player marks them as legacy and leaves the section explanation and evidence available. A v1 visual cannot contain v2 layout or evidence-link fields. New drafts and edited diagrams must use version 2. The bounded payload permits 1–500 nodes and 0–1,000 edges, titles up to 160 UTF-8 bytes, labels and hotspot labels up to 240 bytes, and node details up to 512 bytes. IDs are unique across node and edge IDs in the chapter revision. Every edge has a non-empty label and endpoints in the same diagram.

The closed v2 schema has no HTML, script, URL, or CSS field; unknown fields are rejected. A malformed or unsupported visual is reported as invalid and falls back to a visual-unavailable state while the chapter prose and source evidence remain readable. The reader renders labels, details, prose, and excerpts as text, never as markup.

Version 2 now also permits optional `presentation` (presentation schema version
1): AI-authored compositions of bounded app-owned HTML components, or graph
scenes, one per claim. Content elements refer to same-claim visual node indices;
source values must exactly occur in their cited excerpts, and invented examples
must be visibly labeled illustrations. No raw HTML/CSS/script or model-defined
handlers execute. Older visuals omit this field unchanged. Renderer contract
version 2 includes the composition in asset identity and selective reuse. See
[scene grammar, bounds and limitations](VISUAL_SCENES.md).

Each app-rendered diagram has a deterministic identity based on its typed visual data, the renderer contract version, the exact payloads of linked anchors, and destinations of hotspots used by its nodes. Anchor payloads include their snapshot IDs because an interactive source link is scoped to one immutable snapshot; moving an otherwise identical link to another snapshot changes the diagram identity. Section prose, narration, section order, unused anchors, unused hotspots, and unrelated sections do not change this identity. App-owned diagrams have no generated image/audio cache; the identity and reverse links define which diagram would need updating if a typed input changes.

Every hotspot has an accessible label and exactly one tagged destination:

- `section` with a section ID in the same chapter revision;
- `codeLens` with an `anchorId` present in the chapter;
- `atlasFile` with a captured path;
- `atlasSymbol` with a path and symbol ID from this snapshot's index; or
- `relationship` with an ID from this snapshot's index.

There are no arbitrary URLs, scripts, iframe destinations, or model-defined actions. A hotspot always resolves through an app-owned route while preserving the selected project and snapshot.

## Validation and issue codes

Validation is a read-only operation. It returns `valid` only when no issues are present; otherwise it returns `invalid` plus structured issue records with a code and any relevant path, section, claim, anchor, hotspot, or visual item ID. Chapter validation issue codes are:

| Code | Meaning |
| --- | --- |
| `unsupportedSchemaVersion` | The payload version is not supported. |
| `invalidId`, `duplicateId` | An ID is malformed or reused within the chapter revision. |
| `invalidRevision`, `invalidTarget`, `invalidSectionOrder` | Revision, target, or ordered section structure is invalid. |
| `invalidClaim` | Claim prose/category/evidence requirements are not met. |
| `wrongSnapshot` | Revision, anchor, or supplied index is pinned to a different snapshot. |
| `missingPath`, `sourceUnavailable` | A path is not captured or its captured bytes cannot be read and verified. |
| `rangeMismatch` | Inclusive byte offsets or line/byte-column values are invalid. |
| `hashMismatch` | The source hash differs from the captured file hash. |
| `excerptMismatch` | The excerpt is not exactly the bytes selected by its anchor. |
| `missingSymbol`, `indexUnavailable` | A stable symbol is absent or the snapshot index is unavailable. |
| `invalidDependency` | A typed source/symbol/relationship dependency does not match the snapshot. |
| `invalidDestination` | A hotspot target does not exist in the pinned chapter, inventory, or index. |
| `invalidVisual` | The typed visual version, shape, node, edge, evidence link, or hotspot reference is invalid. `visualItemId` and `anchorId` identify the affected item and evidence where available. |

The validator caps a payload at 4 MiB; 500 sections; 100 claims per section; 32 anchors per claim; 100 dependencies per section or claim; 100 hotspots per section; 500 visual nodes; 1,000 visual edges; 64 KiB per excerpt; and 50 MiB of captured source loaded during one validation. Reaching a structural limit is an invalid result with a structured issue; source browsing remains available.

## Remapping after a line shift

Remapping is a separate explicit read operation. It succeeds only when the old anchor's snapshot, source hash, inclusive range, and excerpt verify against the old captured blob; the anchor has a stable symbol ID present at the old and new indexes; both indexed file results are complete; that same ID still identifies the same path; the new symbol index hash matches the new captured source; and the old exact excerpt appears exactly once inside the new symbol range. The new anchor receives the new snapshot/hash/positions while preserving its stable `anchorId` and `symbolId`.

No line-distance search, fuzzy match, nearest-symbol fallback, or automatic save is performed. Missing symbols, changed excerpts, repeated excerpts, unavailable indexes, or hash mismatches return `unavailable` with a structured issue. The caller can then show the stale evidence for an explicit human decision.

## Versioning and existing accepted revisions

Readers dispatch on `schemaVersion`; versions 1, 2, 3, and 4 are decoded strictly and future versions are rejected. Version 1 predates review metadata; v2 introduced `basedOnRevisionId` and `decisionNote`; v3 introduced grounded overview content; v4 is the current write version and adds optional native agent generation lineage. Reading, validating, or serializing an accepted revision never changes its lifecycle, status, sections, anchors, visual data, or payload values. Version 1 accepted JSON keeps its original shape because absent review metadata is omitted during serialization. Any future migration preserves the old accepted payload and appends a new draft/revision; accepted revisions are never rewritten in place.


## Version 4 agent generation record

`generationOrigin` is optional and omitted when absent. Native Codex and Claude chapter senders create it after independent source/claim validation; API and manual handoff generation do not create agent provenance. It retains the exact original request, supported agent settings, source digests, optional accepted-plan step scope and a bounded original-content digest. Saved previews and immutable chapter revisions keep this record together. The app exposes original generation in previews/history and exact recorded plan-step chapter links. Manual edits and review decisions inherit native lineage; new generation gets its own record rather than claiming the old one. Later content can differ, and the record does not attest account identity or teaching quality.

Versions 1–3 reject origin metadata, including a present null field. Version-3 overview revisions and pending previews remain readable/recoverable without fabricated lineage. New review/edit decisions append version 4, preserving the older accepted bytes. See [the complete record, digest, evolution and acceptance contract](CHAPTER_GENERATION_RECORDS.md). This source extension has not been built or exercised; old evidence must be rerun against version 4.

### Agent lineage digest compatibility
New agent lineage uses origin schema 2 and the current camelCase content serialization. Origin schema 1 remains readable using its original snake_case enum-field content digest (or the transitional camelCase digest). Verification reconstructs the exact original bounded serialization for comparison only. JSON object order is preserved when decoding retained visual payloads, so a read does not change their original digest serialization, including mixed carried-forward and newly generated visuals. It never updates a saved hash, rewrites stored chapter bytes, skips evidence validation or accepts changed prose/narration. This is local request lineage, not an account attestation or semantic-quality guarantee.
