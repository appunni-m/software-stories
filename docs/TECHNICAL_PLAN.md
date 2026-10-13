# Code Atlas — technical plan

This document records source-backed implementation facts alongside the planned architecture. The [Product blueprint](PRODUCT_BLUEPRINT.md) defines intended product behavior; neither document proves every acceptance or release gate has passed. The current repository source is the authority for what is implemented. Planned boundaries and remaining work are called out below.

## Implementation status in this checkout

- The app has a React/TypeScript WebView and a Rust/Tauri command layer. Local repository selection, bounded snapshot capture, content-addressed source storage, static indexing, chapter generation/review commands, chapter revision files, and freshness/remapping code are present in source.
- Walkthrough builds a single self-contained HTML file from selected accepted chapter revisions. It embeds written sections, navigation, supported typed visuals, and optional locally cached narration audio. When audio is included, its exact reviewed speech script is sanitized and bundled as the matching transcript; schema 3 records SHA-256 hashes and byte lengths for that transcript and the WAV alongside included excerpts and visual data. Internal section/evidence links stay inside the file; Atlas/Code Lens destinations are labeled unavailable offline. The full offline acceptance workflow remains open.
- Chapter Studio lists immutable guide revisions, compares a selected revision with its immediate predecessor, and restores an earlier revision as a new draft parented to the latest current revision. Citation anchors move to a newer snapshot only when the same stable symbol and exact excerpt are proven; the new draft is validated before it can be accepted. Selective updates can prepare one stale section at a time; multiple stale sections are not combined into one request. A bounded local Git base/head comparison is exposed from Project Tools. It resolves local refs to immutable IDs and renders classified files, exact changed-line ranges, bounded hunks, and separate base/head line, commit, and blob identities. When both blobs parse without errors, it attaches declaration names whose syntax spans overlap changed lines. A separate PR Guide panel can send a selected subset of exact diff hunks through an explicitly selected Gemini, OpenAI, or Anthropic API after request review and consent; returned claims are checked against the correct base/head hunk and saved in a separate immutable per-comparison history. Each eligible hunk also has a transient, reviewed single-hunk explanation route that returns locally validated side-aware claims without writing PR Guide history. These flows do not fetch, check out, or write Git state. They do not yet attach cross-file relationship impact.
- Snapshot catalogs and manifests are JSON under Tauri app data. Captured source bytes are stored by SHA-256 under `objects/sha256/`. Indexes, chapter revisions, and pending generated previews are also local files. **There is no SQLite dependency or database in the current implementation.**
- Direct text API selection currently routes Gemini, OpenAI Responses, and Anthropic Messages for file chapters, optional repository orientation, selective section updates, PR Guide drafts, transient PR change explanations, and transient Code Lens range explanations. The pinned `rig-core` provider wires encode and decode unary requests/responses; a narrow Reqwest transport wrapper enforces the reviewed endpoint and body, replaces the credential placeholder with the selected provider's sensitive auth header, and bounds network time and response size. The request fingerprint includes provider, model, endpoint, body hash, and disclosure version. Requests require exact-context review and explicit consent; responses are locally validated. Chapter results are unaccepted previews with Chapter Studio decisions. PR Guide results have separate immutable per-comparison revisions and their own claim-by-claim accept/reject review; they never modify chapter history or Git refs. PR change and Code Lens explanations cite only their reviewed hunk/range and are not persisted as chapters. Streaming is disabled in this transport. Offline fixtures cover reviewed provider/model/body dispatch, complete-consent gating, invalidated-review rejection, and single-call synthetic quota failures across every currently wired operation/provider pair; a separate classifier fixture checks vendor-specific status categories using constructed Rig error responses. Live endpoint behavior, eligibility, credential non-disclosure through the real transport, and cancellation/transport-close acceptance remain incomplete.
- Optional speech generation calls Google's Gemini API separately with a reviewed section transcript and speech settings. It has its own disclosure, consent check, and local WAV cache; it does not send the source excerpt by default.
- Direct API provider selection is implemented for the operations listed above. Coding-agent runtimes (Codex, Claude Agent, Gemini CLI, and Google Antigravity) remain planned. The full direct API operation matrix and acceptance evidence remain incomplete. See [M7](kata-tasks/phase-7.md) and the [capability matrix](research/ai-backend-capability-matrix.md).

Source presence is not release evidence: the exact native app acceptance, quality review, supported macOS range, signing/notarization, and distribution gates remain separate.

## System shape

The diagram is the target boundary. The local chapter flow, initial static offline export, bounded local base/head comparison command, and separate PR Guide implementation exist in current source. The direct API layer routes three providers for its currently wired operations; agent-runtime integration remains planned, and direct API/PR Guide acceptance evidence is still open.

```mermaid
flowchart LR
  Repo[Local Git working copy] -->|explicit capture| Core[Rust repository core]
  Core --> Snapshot[Immutable snapshot objects]
  Snapshot --> Index[Parser and relationship index]
  Index --> Atlas[Atlas]
  Snapshot --> Lens[Code Lens]
  Index --> Guide[Guide Plan and Chapter Studio]
  Guide -->|exact manifest + separate consent| APIs[Selected model API]
  APIs --> Draft[Untrusted structured draft]
  Guide -->|approved immutable input + explicit run consent| Agents[Selected agent runtime]
  Agents --> Draft
  Draft --> Validate[Local schema and evidence validator]
  Validate --> History[Immutable chapter/section revisions]
  History --> Player[Interactive Walkthrough]
  History --> Media[Linked visuals and optional cached TTS]
  Snapshot --> Diff[Local Git base/head evidence]
  Diff --> PR[Separate PR Guide draft]
```

All local browsing, accepted guide reading, chapter editing, source navigation, and existing audio work without a provider connection.

### Planned AI backend boundary

The direct API path routes Gemini, OpenAI, and Anthropic for file chapters, optional orientation, selective section updates, PR Guides, transient single-hunk PR change explanations, and transient range-level Code Lens explanations. The full per-operation capability/test matrix remains open. The [M7 AI-backend tasks](kata-tasks/phase-7.md) also cover coding-agent runtimes as a separate integration type. The agent path should use Agent Client Protocol (ACP) when a runtime supports the required lifecycle and boundary, otherwise that product's documented SDK, app-server, or headless CLI. It must not recreate an agent loop or pretend these modes have identical permissions, costs, credentials, or capabilities.

## Runtime boundary

### React/TypeScript UI

- Owns visible layout, selection, navigation history, keyboard controls, playback time, and user-facing loading/error/empty states.
- Calls typed Tauri commands for repository, snapshot, source, indexing, persistence, validation, and provider operations.
- Does not get general filesystem, shell, Git, keychain, or network access.
- Uses one shared selection identity: `(project_id, snapshot_id, path, symbol_id?, section_id?)`.

### Rust core

- Owns folder selection validation, Git queries, bounded file reads, content hashes, local parser runs, source anchors, filesystem persistence under app data, request manifest construction, chapter validation, and TTS cache.
- Does not execute repository scripts or build tools during scan.
- Exposes narrow typed commands; validates every repository-relative path and snapshot ID before access.
- Owns direct API request assembly, reviewed transport, response parsing, and the separate Gemini TTS path so API keys remain outside the WebView. Provider-specific keys use provider-specific macOS Keychain entries; Gemini narration uses a separate Gemini key. The current Rig/Reqwest adapter accepts only explicit Gemini, OpenAI, or Anthropic routes and unary JSON requests. The separate agent-runtime bridge remains planned; all routes must preserve local consent and validation boundaries.

### Persistence

- Before capture, Rust owns the native folder picker and returns only a canonical selected path plus Git root/branch/HEAD metadata. The WebView can request the picker but cannot submit an arbitrary filesystem path. A recent project's stable ID is its canonical Git root path; recent paths are stored atomically under Tauri's app-data directory, and removing one edits only that metadata.
- Snapshot catalogs and manifests are JSON records under the Tauri app-data directory; source blobs are content-addressed by SHA-256. Index data is stored as versioned JSON.
- Chapter revisions are immutable JSON files under per-project/per-target app-data directories. A save stages and publishes a complete new record; prior revisions are not overwritten.
- Generated but unsaved chapter previews have a separate bounded JSON recovery store. They can be reviewed after restart and are revalidated against their pinned snapshot before saving.
- macOS Keychain stores one item per direct API provider, plus a separate Gemini narration key. Keys are not copied into local chapter/snapshot files, logs, exports, or application-authored diagnostics.
- There is no SQLite transaction layer in the current source. Future persistence changes must describe the migration and recovery semantics explicitly rather than relying on the earlier proposed database design.

## Planned data identities and revision graph

The intended editing unit is an independently versioned section. The current `chapter_store` appends complete chapter-revision JSON files; it does not yet persist a deduplicated section-revision graph or demonstrate that an edit reuses unaffected sections and assets. The TypeScript revision summary helper compares stable fields, but it does not provide a user-facing compare or restore flow. The design below is the target for the Git-like editing milestone, not a claim about current persistence.

Use stable IDs for a chapter target, section, claim, anchor, hotspot, and asset. A symbol ID is based on parser module/path and qualified declaration identity, not current line number. Line numbers are presentation/remapping data.

Each immutable snapshot records the repository root identity, full Git HEAD object ID when available, index/worktree state, capture timestamp, inventory, content hashes, and capture-limit outcomes. Snapshot ancestry connects local source captures but never mutates the source repository.

Each accepted chapter revision is a manifest over immutable section revisions. A section revision stores:

- ordered structured claims and exact source anchors;
- explanation prose and independently editable speech text;
- optional typed visual specification and hotspot destinations;
- source and relationship dependency IDs/fingerprints;
- derived asset fingerprints and local references;
- validation result and snapshot ID.

A chapter edit creates a new manifest that can point to unchanged prior section revisions. A changed section creates a new section revision. Accepted rows and their asset hashes are immutable.

The current version 3 chapter payload, legacy readers, inclusive anchor coordinates, exact validation rules, typed hotspot destinations, visual safety boundary, and line-shift remapping contract are specified in [Chapter schema](CHAPTER_SCHEMA.md).

## Local repository capture

The scanner is read-only. Limits are named and user-visible, versioned, and covered by fixtures. A starting safety profile is:

- do not enter `.git` internals, symlink targets, nested repositories, ignored contents, or vendored/generated directories by default;
- cap file size, total captured bytes, path count, and per-directory entries; show each skipped path and its reason;
- classify binary, oversized, invalid UTF-8, secret-like, and parser-unsupported inputs before they can become model context;
- never run code from the selected repository;
- persist exact captured bytes before source views or citations are offered.

The initial limits are constants of the product specification and must be validated against sample repositories before release; users see them in the capture summary and can inspect skipped content locally.

## Index and relationship semantics

The current Rust manifest pins Tree-sitter 0.27.1 with the Rust 0.24.2, TypeScript/TSX 0.23.2, and JavaScript 0.25.0 grammars. The indexer records runtime and grammar versions/ABIs in each snapshot index. The Rust fixtures cover representative declarations, imports, calls, test files, line movement, dynamic dispatch, recovered syntax errors, and unsupported files; this is a supported subset, not a claim of complete language semantics or a released support promise.

An index is derived only from captured snapshot blobs after their SHA-256 and UTF-8 checks. It never reads the live worktree or executes repository code. Output rows and edges sort by stable keys. Symbol identity hashes language, path, qualified declaration name, kind, and a declaration signature that excludes the body and line number; source anchors retain the exact captured source hash, half-open byte range, one-based line, and one-based UTF-8 byte column. A moved unchanged declaration therefore keeps its ID while its source anchor changes.

The initial index recognizes Rust `.rs`, TypeScript `.ts`/`.mts`/`.cts`, TSX `.tsx`, and JavaScript `.js`/`.jsx`/`.mjs`/`.cjs`. Per-file results distinguish indexed, partial parse, parse error, unsupported language, binary, invalid encoding, missing or hash-invalid source, oversized input, global index limits, parser unavailability, analysis limit, and cancellation. Snapshot index status is complete only when every captured file is indexed; any omission or parse gap yields partial. Unsupported files remain in the captured inventory and source browser.

Every edge has a type, confidence, stable ID, and exact evidence anchor. M0 records definitions and explicit exports, import statements, direct identifier calls with one same-file target, and direct calls through a unique named ESM import to one explicit exported callable in a fully parsed captured module. Test-path calls are labelled test references. Relative ESM module resolution is accepted only when the captured candidate path is unique. Rust `use` paths, default and namespace import targets, re-exports, CommonJS/dynamic imports, external packages, ambiguous module/symbol matches, member dispatch, reflection, and dynamic callees remain unresolved with a reason. Partial parse files do not prove exported targets unique. No complete call-graph claim is made.

Index work caps input at 5,000 files, 512 KiB per file, 50 MiB total source, 250,000 syntax nodes per file, 50,000 symbols, 100,000 candidate/retained relationships, 50,000 total named-import bindings (10,000 per file), 128 nesting levels, 1.5 seconds per parse, and 32 MiB serialized output. Work runs off the UI thread, coalesces progress events, accepts cancellation during parsing and AST traversal, and does not save cancelled output. Saved indexes are versioned JSON keyed by project and snapshot. These limits bound work while leaving every captured path browsable.

## Direct API request boundary

The current shared text-generation route uses explicit provider/model selection behind one reviewed request boundary:

1. For file chapters and one-section updates, Rust loads the target plus selected supporting files from one immutable snapshot, verifies stored hashes, rejects secret-like, binary, invalid-encoding, missing, and over-limit context, and never falls back to the working tree. A file request is capped at 250 KB of selected source and 512 KB of encoded request body. Optional orientation uses its selected snapshot context. PR Guide requests use their separate bounded base/head diff evidence.
2. The WebView loads local snapshot metadata for the context picker. The user chooses a provider and model explicitly. Rust builds the prompt and provider JSON locally and presents the selected context, prompt, settings, endpoint, body digest, terms, and cost uncertainty. Building or rebuilding the preview makes no network request.
3. Pending reviews are ephemeral in-memory Rust state, capped at eight and expiring after fifteen minutes. Three independent consent acknowledgements cover the selected provider's data-use/privacy notice, billing uncertainty, and age/region/use requirements.
4. The fingerprint binds purpose and versions, project/snapshot/target, selected provider and model, endpoint, disclosure version, prompt/schema/settings, selected paths and source hashes/sizes, and encoded body hash. Before send, Rust revalidates the review against the active session and current snapshot/disclosure. A changed input requires a new preview and consent.
5. After consent and revalidation, Rust loads the provider-specific key from macOS Keychain and makes one HTTPS POST through the Rig provider wire and guarded Reqwest transport. The transport verifies method, endpoint, content type, and body against the reviewed request, then replaces the Rig placeholder with the selected provider's sensitive auth header. It uses a 90-second timeout and bounded response bytes; streaming and multipart are disabled. There is no provider/model fallback, retry, tool/grounding access, or model autodiscovery. Cancel drops the local request future; the provider may already have received or started the request.
6. Successful responses become locally validated, unaccepted previews. Chapter previews are recoverable after restart; saving creates an unaccepted chapter revision, and the user accepts or rejects it later in Chapter Studio. PR Guide results have separate per-comparison history and claim review.

The app routes Gemini, OpenAI Responses, and Anthropic Messages for file chapters, optional repository orientation, selective section updates, PR Guides, a transient single-hunk PR change explanation, and a distinct transient Code Lens range explanation. These operations share provider selection, exact request review, and consent-gated Rig dispatch. Mocked dispatch, consent, invalidation, and quota-failure fixtures now cover each wired operation/provider pair; classifier fixtures check documented status categories from constructed Rig error responses. Live endpoint behavior, eligibility evidence, real-transport credential non-disclosure, and cancellation/transport-close evidence remain open for t28. The full operation-to-capability matrix is recorded in [the provider matrix](research/ai-backend-capability-matrix.md).

Provider model IDs, prices, and terms are static versioned release metadata. The app links to each provider's first-party sources and displays its disclosure date; it cannot verify live price/terms changes or the account, region, billing, quota, or eligibility attached to a key. See [Provider disclosures](PROVIDER_DISCLOSURES.md).

A pending generated preview is limited to 2 MiB each, 16 records, and 16 MiB total. Loading a recovered preview revalidates evidence and restores only an in-memory review handle; it does not restore consent or a provider request.

## Interactive visuals and audio

Visuals are versioned data structures (for example typed nodes, edges, labels, and anchor destinations), rendered by product code. Their hotspot IDs and destinations are validated locally. The model cannot emit executable scripts, raw HTML, or arbitrary CSS.

Speech is a separate optional Gemini TTS path. Each section has editable `spokenText` separate from code, path labels, and display text. The request review includes the exact section script after enabled pronunciation substitutions, voice, style, and WAV format; it excludes source excerpts by default. A confirmed request can be cancelled locally, though Google may already have received it. Cached audio is keyed by script hash, model, voice, style, and format; matching audio can be loaded locally. The date-stamped usage and billing notice is documented in [Provider disclosures](PROVIDER_DISCLOSURES.md#optional-section-narration).

## Offline export

The current Walkthrough can build one static, self-contained HTML file from selected accepted chapter revisions. The preview lists paths and cited excerpts and permits excerpts to be omitted. The exporter excludes non-accepted revisions, omits hidden or secret-like paths, redacts recognized credential patterns and local absolute paths, and records chapter revision/snapshot identities plus SHA-256 hashes for included excerpts and typed visual data.

The package supports written sections, manual section navigation, typed visuals that the current renderer recognizes, and optional cached narration WAV tracks selected in the export preview. Export only reads matching local cache entries; it never generates speech. The manifest records hashes and identities for included assets. Live Atlas/Code Lens navigation is not available offline; unsupported legacy or malformed visuals produce an explanatory message. Full offline acceptance—including opening and navigating an exported package while disconnected and verifying its manifest and privacy behavior—remains a release task.

## Source change propagation

The current source contains an explicit local, read-only freshness assessment and anchor-remapping path for comparing an older accepted chapter with a newer snapshot. Freshness is computed per section from captured source and per-file index evidence:

- **Current:** all source and relationship dependencies match.
- **Safe to carry forward:** stable symbol identity and one exact excerpt match allow an unambiguous range remap.
- **Stale:** cited bytes, symbol identity, relationship dependency, or destination are proven to have changed or disappeared.
- **Needs review:** captured source or relevant index evidence is missing, partial, unsupported, or ambiguous, so change cannot be confirmed.

Changed whole-file hashes trigger comparison but do not stale every section in that file. Relationship identity is compared semantically because serialized relationship IDs include byte positions. A relationship is carried only when its exact evidence span maps uniquely in verified captured bytes; this also handles file-level imports with no enclosing symbol. Source anchors require stable symbol identity and an exact, unique excerpt match; no nearest-line guess is offered. The report retains old/new line ranges and reasons. Assessment never writes the accepted revision or snapshot. Carry-forward, section-selective regeneration, and asset invalidation are separate planned work; a freshness result alone does not create a new draft or alter accepted content.

## Local Git comparison and PR Guide extension

The Rust backend exposes a bounded local base/head comparison command, and **Compare local revisions** in Project Tools provides its evidence-browsing UI. It resolves local refs to immutable commit IDs and returns deterministic changed-file classifications, exact changed-line ranges, bounded diff hunks, and separate base/head line anchors with commit and blob identity. For cleanly parsed Rust, TypeScript, TSX, JavaScript, and JSX blobs, each hunk also includes declaration names whose source spans overlap changed lines. A declaration anchor retains its revision, path, blob, and full syntax range. Parser errors, unsupported extensions, binary data, and bounded-parse failures do not receive declaration evidence. Fixture tests cover invalid, missing, and ambiguous refs; changed-line/declaration identity; deterministic output and unchanged refs/worktree; and add, delete, rename, binary, oversized, unsupported, and parser-uncertain files. Cross-file relationship impact remains unavailable.

The PR Guide builds on a ready comparison. The user selects up to twelve eligible text-diff files, chooses Gemini, OpenAI, or Anthropic, reviews the exact prompt, request settings, selected diff evidence, endpoint, and provider disclosure, then explicitly consents before the selected provider receives it. Each generated claim must cite a unique exact diff line from the correct base or head side and carries an immutable commit/blob/source anchor. The review UI can expand and scroll to the cited hunk, exclude individual claims, and append an accepted or rejected PR Guide revision to local per-project/per-comparison JSON history. This history is independent of repository chapters and Git refs. The workflow does not fetch a GitHub PR, push, comment, check out refs, or modify the checkout. Its Kata acceptance evidence remains open.

## Nonfunctional requirements

- Local-only browsing succeeds with networking disabled.
- A cancellation or provider failure cannot leave a half-accepted chapter or corrupt prior revisions.
- Large repository scans yield progress and cancellation; a measured 2,000-file fixture is recorded in [Performance validation](PERFORMANCE_VALIDATION.md). It measures backend phases, not rendered UI responsiveness.
- Keyboard focus, VoiceOver names/announcements, text zoom, reduced motion, and high-contrast legibility are part of each feature's acceptance.
- Logs contain operation IDs and error categories, not API keys, raw prompt context, or source excerpts.
- Release checks verify signed/notarized macOS artifacts and document supported macOS versions and storage locations.

Current accessibility source evidence and unverified native checks are listed in [Accessibility validation](ACCESSIBILITY_VALIDATION.md). Repository and provider input boundaries are indexed in [Security validation](SECURITY_VALIDATION.md).
