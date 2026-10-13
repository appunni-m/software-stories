# Code Atlas — screen and interaction contract

This document turns the visual concepts into the intended observable product behavior. It is the interaction contract for M0 and a shared acceptance reference for later Kata issues, not an implementation-status report. Check the [technical plan](TECHNICAL_PLAN.md) and [Kata backlog](KATA_BACKLOG.md) for current source status and outstanding acceptance. The images in `docs/design/` show visual direction; they do not override this contract.

## Shared identity and navigation

Every workspace operates on one shared selection:

```text
project_id
snapshot_id
relative_path? / symbol_id?
chapter_id? / chapter_revision_id? / section_id?
anchor_id? / playback_position?
```

Optional fields are absent when they do not apply. Changing project or snapshot is explicit and visible in the global header. Navigation never changes the selected snapshot as a side effect of opening a file, chapter, or link.

When leaving a screen, save a local navigation-return entry with the origin screen, project/snapshot, selected file/symbol/section, tree or diagram viewport, source scroll/line, playback position, transcript position, and focused control. A Back action restores that entry. It does not create or select a newer source snapshot.

### Global controls

| Control | Activation | Result | Focus and side effects |
|---|---|---|---|
| Project picker | Click project name or `⌘O` | Show recent local projects and **Open repository**. Selecting a different project updates the shared project selection after confirmation when there is unsaved work. | Focus stays in picker until selection/cancel. No source bytes are read; selecting a project does not create a snapshot. |
| Open repository | Choose from picker or `⌘O` | Open the native macOS folder picker, then show the canonical selected folder, detected Git root, branch/HEAD when available, folder access, and **Create snapshot**. A nested folder resolves to its containing working copy. | Cancel returns to the previous screen with focus on the invoking control. Non-Git folders, missing folders, permission denial, missing Git, and metadata-storage failures have distinct recovery actions. No source bytes are read. |
| Recent project | Choose a row in the project picker | Recheck the saved path and Git metadata, then select the project in Atlas. | Reopening never starts a snapshot. If the path moved or access changed, preserve the recent entry and offer **Choose a folder** or **Remove from Recents**. |
| Remove from Recents | Click the row's remove control | Remove that path from Code Atlas' local recent-project metadata. | Never delete or modify repository files. If it is the active project, the current session stays selected until the user switches projects or quits. |
| Snapshot picker | Click snapshot/revision label | List immutable snapshots with capture time, branch/HEAD, dirty-state summary, and partial/capture status. | Choosing one updates every view's shared snapshot. If a draft is open, ask Save draft / Discard draft / Cancel before switching. No new capture is made. |
| Create snapshot | Click from project preflight or snapshot list | Start a bounded local capture and show progress, phase, count, limits, and Cancel. | This is the only action that starts repository byte capture. It performs no network request and never writes into the selected repository. |
| Primary workspace nav | Click Atlas, Code Lens, or Walkthrough; shortcuts `⌘1`, `⌘2`, `⌘3` | Switch to the chosen workspace using the current project/snapshot and selection when applicable. | Focus moves to the destination heading/primary selection. Switching away from playback pauses it and stores position. |
| Guide Plan entry | Click **Guide Plan** in project tools | Show file coverage for the current snapshot. | Returns to prior origin with selection/focus restored. Does not call Gemini. |
| Chapter Studio entry | Click **Chapter Studio** on a chapter row or draft notification | Open the selected chapter revision/draft. | Opening a draft is local. Opening an accepted revision is read-only until the user chooses Edit. |
| Global search | `⌘K`, then type | Search captured paths, parsed symbols, and chapter titles within the displayed scope. Show scope and result type in each row. | Escape closes search and restores prior focus. Enter opens the selected result's defined destination; it never changes snapshot silently. |
| Back / Forward | `⌘[` / `⌘]` or toolbar buttons | Traverse the app's local navigation-return entries. | Back restores previous selection, scroll, focus, and playback context. It never reverses a source edit or accepted revision. |
| Help / About | App menu | Show product version, parser/provider metadata date, storage location help, keyboard shortcuts, and documentation links. | No provider request. |

`Escape` closes the topmost picker, menu, or non-sending dialog. If a provider request is in flight, use the explicit **Cancel request** control and confirm cancellation; Escape alone does not imply a network abort. `Space` toggles audio only when focus is in the Walkthrough player and not in a text input. `⌘S` saves the current local draft in Chapter Studio. Enter activates the focused row/button; arrow keys move within a tree/list/menu. Focus is always visible.

## Startup and capture state machine

```mermaid
stateDiagram-v2
  [*] --> NoProject
  NoProject --> ChoosingFolder: Open repository
  ChoosingFolder --> NoProject: cancel
  ChoosingFolder --> ProjectPreflight: valid folder
  ProjectPreflight --> Capturing: Create snapshot
  Capturing --> SnapshotReady: complete
  Capturing --> SnapshotPartial: limits or recoverable read errors
  Capturing --> CaptureCancelled: Cancel
  CaptureCancelled --> ProjectPreflight: inspect or create a new snapshot
  SnapshotPartial --> SnapshotReady: create a later complete snapshot
  SnapshotReady --> Indexing: open Atlas / build index
  Indexing --> AtlasReady: supported evidence available
  Indexing --> AtlasPartial: parser gaps or errors
```

`SnapshotPartial` is not presented as a full repository capture. Atlas, Code Lens, and Guide Plan identify that snapshot as partial until the user captures again. Existing accepted chapters remain pinned to their original snapshot.

### Snapshot path inventory

The completed capture view includes one searchable inventory for the selected immutable snapshot. Its total is the union of captured files, recorded skipped/excluded paths, and traversed folders; rows are not deduplicated by filter or page. Directories are listed as containers and do not count as captured source files. The summary always distinguishes total inventory paths, captured files, captured source bytes, and skipped/excluded rows.

- Search matches the path and the stable reason/details for a row. Language, capture outcome, parser readiness, content kind, and config/test/generated/secret-like category filters combine with AND. Clear resets filters and returns to the first page; no filter changes snapshot contents or guide coverage.
- Each row identifies path, filesystem kind, inferred language, known size, captured/skipped/excluded disposition, reason, parser readiness, and index eligibility. Binary and invalid-UTF-8 content are separate. A parser marked not ready is not presented as an indexed file.
- Pages show both the visible range and the filtered/whole-inventory totals. Changing filters or page size returns to page one; an out-of-range page is clamped after the result set changes.
- **Open captured text** loads the path by project ID, snapshot ID, and relative path from the immutable snapshot blob store, then verifies its byte count and SHA-256 before display. A missing or corrupt blob produces an unavailable state; there is no working-tree fallback.
- Capture disposition is not a chapter state and does not select source for a future Gemini request. Guide Plan owns separate chapter coverage, and Request Review owns explicit outbound file selection.

## Surface contracts

### App shell and project preflight

**Purpose:** make the local, no-AI path the first thing a developer can use.

**Primary selection:** current project; no snapshot until one is explicitly chosen or captured.

**Visible content:** project name and canonical path; recent-project list; captured snapshot selector; capture status; three primary workspaces; supporting Guide Plan/Chapter Studio actions.

**Controls and destinations:**

- **Open repository** → native folder picker → project preflight.
- **Create snapshot** → capture state machine above.
- **Atlas / Code Lens / Walkthrough** → selected workspace; if no snapshot exists, show its designed no-snapshot screen with an Open/Create action.
- **Guide Plan** without a snapshot → explain that a snapshot is required and route to preflight.
- **Remove from recents** → remove only Code Atlas project metadata after confirmation; never delete repository files or snapshots unless a separate destructive action names exactly what will be deleted.
- **Settings** → local provider setup and app preferences. Provider setup itself does not send a request.

**States:** first launch/no projects; project selected/no snapshot; capture in progress; capture cancelled; partial capture; complete snapshot; no repository permission; project unavailable/moved; persistence error. Each state has a next action and preserves prior projects/snapshots.

**Focus:** first launch focuses Open repository. Picker cancel returns focus to its invoking control. Capture completion announces counts and focuses the snapshot summary, not an unrelated tab.

**Side effects:** project selection may read Git metadata needed for preflight. Only Create snapshot reads eligible source bytes. No screen entry checks a Gemini key or contacts Google.

### Atlas

**Purpose:** answer “what is in this repository, and what evidence-backed relationships can I follow?”

**Primary selection:** folder, file, symbol, or relationship in the current snapshot.

**Layout:** searchable repository tree; relationship canvas; explicit relationship/type filters; selected-item evidence panel; snapshot/freshness status.

**Controls and destinations:**

- Typing in **Search** narrows paths/symbols in the current scope and updates the visible result count. A scope selector chooses repository, current folder, or indexed symbols. **Clear** removes only the text query.
- Clicking a folder selects it and filters canvas results to descendants. It does not open a chapter.
- Clicking a file/symbol selects it and shows details/evidence. **Open in Code Lens** or Enter opens the exact source anchor. Double-clicking a file row performs the same action.
- Clicking a relationship selects the edge and shows its type, confidence, and evidence. **Open evidence** navigates to the exact source range that proves the edge.
- Relationship-type filters use AND with the current path/language filters. Selected filters appear as removable chips; **Clear filters** removes all active filter chips but preserves the selected snapshot.
- Map **Fit**, **Zoom +**, **Zoom −**, and **Reset** affect only the viewport. Dragging empty canvas pans; selecting a node does not recenter the map unexpectedly.
- **Freshness status** opens the source-change report. It does not update or regenerate chapters.
- An unsupported relationship badge opens the parser status and reason; it is not a link to guessed callers.

**States:** no snapshot; indexing; indexed; partial index; empty repository; no search result; selected folder with no indexed descendants; selected unsupported file; scan/index error; stale accepted guide indicator. Tree rows remain available when the graph has no supported nodes.

**Focus/return:** tree uses arrow navigation; Enter opens Code Lens for a selected file/symbol; relationship canvas nodes are keyboard focusable with name/type/evidence summary. Returning from Code Lens restores query, filter chips, selected node, zoom, and pan.

**Side effects:** searching/filtering/selecting/panning are local reads only. No source is sent to Google from Atlas.

### Code Lens

**Purpose:** verify what a claim or relationship says against exact immutable source.

**Primary selection:** path, source range, and snapshot; optional related symbol.

**Layout:** breadcrumb and snapshot identity; read-only source; highlighted anchor; claim/evidence panel; tabs or list for resolved callers/imports/dependents/tests.

**Controls and destinations:**

- Clicking a citation selects its exact range and scrolls it into view. The evidence panel shows path, snapshot, hash, line range, excerpt, and claim category.
- Clicking **Back to chapter/map** restores the stored origin entry. The app also offers normal Back/Forward history.
- Clicking a resolved caller/import/test opens that file and exact relationship anchor while stacking a return entry.
- Clicking **Copy path** copies only the repository-relative path. **Copy citation** copies path plus range and snapshot ID. Neither reads the live repository.
- A stale/missing citation has **View source change** when both snapshots are available, or **Return to chapter**. No “closest line” action is offered.

**States:** exact source available; unsupported highlighting/plain-text source; uncaptured path; binary/too large; invalid encoding; missing blob; range/hash mismatch; parser unavailable; relationship unresolved; permission revoked. Every unavailable state explains why and offers a valid return/refresh route.

**Focus/return:** source editor is read-only and keyboard navigable. Citation activation places focus on the source heading/range; Back returns focus to the initiating citation/hotspot. Source scrolling never changes the selected snapshot.

**Side effects:** opens captured local bytes only. No repository write, model request, or implicit recapture.

### Guide Plan

**Purpose:** answer “which files have an accepted explanation, and what remains?”

**Primary selection:** one inventory path or an explicit user-created coverage group.

**Layout:** coverage totals and selected snapshot; path table with status/reason; search, language/status filters, pagination; selected-file detail and actions.

**Controls and destinations:**

- Clicking a path row selects it and shows its disposition, current chapter revision, and exact reason. **Open source** opens Code Lens; **Open chapter** opens Chapter Studio or Walkthrough based on revision state.
- **Create chapter** opens a local empty draft for that one file; no request is sent.
- **Generate draft** builds a local manifest and opens Request Review. It does not send until the separate final confirmation there.
- **Exclude from guide** requires a reason and affects guide coverage only; it does not remove inventory/source data. **Restore coverage** removes that exclusion.
- **Create group** requires selecting members and entering a rationale. **Edit group** shows every member; **Remove group** returns each file to its own individual missing/draft/accepted state. Groups are never implicit.
- Search, status/language filters, page size, and sort order affect visible rows only. Counts always state whether they describe all paths or current results.
- There is no **Generate all** action in the initial release.

**States:** not indexed; inventory loading; all chapters missing; partial coverage; complete eligible coverage; partial snapshot; filtered no-results; unsupported/excluded paths; selected file with no chapter; selected group with missing members; stale chapters awaiting review; persistence error.

**Focus/return:** table uses row/column navigation and announces status. When returning from Code Lens or Chapter Studio, restore the selected row, query, filters, page, and focus.

**Side effects:** all views and edits are local until the user completes Request Review confirmation. Excluding/restoring a path does not delete captured bytes.

### Chapter Studio

**Purpose:** make source-grounded drafts reviewable and editable before acceptance.

**Primary selection:** chapter revision and one section in the current snapshot.

**Layout:** chapter/file identity and revision status; section outline; editable prose and spoken text for a draft; claims/evidence panel; validation results; revision actions.

**Controls and destinations:**

- Selecting a section changes the editor/evidence panel only. Section reorder controls move the selected section in the draft and show its new order before save.
- **Edit accepted revision** creates a new draft based on it; original accepted content remains read-only.
- Editing prose changes draft text; editing spoken text changes only the speech input. Removing a claim removes that claim from the draft and marks validation dirty; source anchors cannot be changed by changing visible prose.
- Clicking an anchor opens Code Lens at that range; Back restores section, editor scroll, and focus.
- On an older accepted revision, **Assess freshness** compares it with the selected snapshot without changing the revision. Each section reports Current, Safe to carry forward, Stale, or Needs review with evidence completeness and an explanation. Exact old/new source ranges open in Code Lens at their respective snapshots; no line is guessed when mapping is ambiguous.
- **Validate** runs local schema/source/destination checks. **Accept** is disabled until validation passes. **Reject** asks for a reason and preserves the rejected payload in history.
- **Save draft** writes local content only. **Accept** creates an immutable accepted revision and routes to Walkthrough with this chapter selected. **Cancel** returns to origin without deleting a saved draft.

**States:** new blank draft; generated draft; unsaved edits; validation running; validation errors; valid draft; accepted read-only; rejected; stale against newer snapshot; freshness assessing; current; safe to carry forward; stale evidence; incomplete/unsupported evidence; storage error. A stale state identifies source and affected sections; it does not silently shift anchors. An incomplete index is shown as Needs review, not as proof that the source changed.

**Focus/return:** section outline and claims list are keyboard navigable. After validation, focus moves to the first error or success summary. After acceptance, focus moves to the selected chapter heading in Walkthrough.

**Side effects:** editing, validation, save, accept, and reject are local database operations. Only an explicitly chosen Generate action leaves this surface for Request Review.

### Walkthrough

**Purpose:** help a developer learn a real file one small, inspectable section at a time.

**Primary selection:** accepted chapter, selected section, and optional playback position.

**Layout:** chapter/section rail; current section question/title; explanation; diagram; exact source excerpt; transcript/captions; transport controls.

**Diagram behavior:** only a validated visual version 2 is interactive. App-owned layout uses the saved direction; each node/edge label has visible source links to its own exact claim anchors. Edges name their destination node and keep their evidence links beside the edge. A missing, legacy, malformed, or unsupported diagram shows a short unavailable/legacy message while prose and evidence remain usable. Diagram values are text; no generated markup, script, CSS, or external URL is run.

**Controls and destinations:**

- Selecting a chapter opens its first section, or the last locally stored section when resuming that same chapter. It never auto-plays audio.
- Selecting an item in the section rail opens that section immediately and updates code/diagram/transcript to its accepted revision.
- **Previous** selects the preceding section; disabled on the first section. **Continue** selects the next section only after the reader clicks it; no timed auto-advance. On the last section the control becomes **Finish chapter**, which returns to the chapter list and remembers the last section.
- **Play/Pause**, **Replay section**, and the section timeline control narration. Playback completion does not advance the section. If there is no audio, controls are disabled with “Audio unavailable”; text remains usable.
- Clicking a diagram hotspot opens only its declared source/section/Atlas destination. The focused hotspot action remains visibly highlighted when Back restores the origin section, playback time, and transcript position; audio resumes only if the user explicitly resumes it.
- Clicking a node or edge source link opens the exact cited range. Back returns focus to that specific visual evidence link, including when multiple visual items cite the same anchor.
- Clicking a code citation opens Code Lens. **Return to walkthrough** restores reader context.
- **Transcript** remains visible with audio off; if timestamps exist, selecting a timestamp seeks within the current section. The current chapter format stores one unsegmented `spokenText` string, so show section-wide source references separately and do not imply that they support a particular sentence. Each reference opens its exact anchor. Passage-level citations require transcript segments with explicit anchor relationships.

**States:** no accepted chapter; chapter with one section; selected section; first/middle/last section; audio generating/available/unavailable; stale chapter; missing diagram; source anchor unavailable; offline. Stale content is clearly labeled; the reader can still inspect the accepted revision's captured snapshot.

**Focus/return:** the focused section heading is announced after navigation. Space toggles play/pause only when focus is in player controls. Reduced-motion mode disables decorative transitions without changing section or hotspot behavior.

**Side effects:** viewing, section navigation, and playback are local. No chapter generation occurs from Continue or a hotspot.

### Request Review and provider consent

**Purpose:** let the user understand and approve one exact outbound request, not a broad repository upload.

**Primary selection:** request purpose, one captured target file, zero to eleven supporting captured files, model ID, and disclosure version. The target remains selected; supporting context can be filtered by path and selected from the same immutable snapshot.

**Review layout:** target and request purpose; model ID and published pricing/free-tier disclosure date; selectable eligible files; each selected file's complete captured UTF-8 contents, byte size, and hash; combined size/token estimate; exact system and user prompts and generation settings; request fingerprint; unpaid-service privacy notice; external Google terms/pricing/region links; separate unchecked privacy, billing, and age/region/use acknowledgements.

**Controls and destinations:**

- **Choose source files** shows only captured regular text files with known or text content, each no larger than 250 KB, excluding paths marked secret-like. The target is required; up to eleven supporting files can be checked. The total selected source limit is 250 KB, and the encoded request limit is 512 KB. Binary, invalid-encoding, skipped, uncaptured, and secret-like files cannot be selected. Every selection change rebuilds the local request preview and clears all three acknowledgements.
- **Change model** selects only an explicitly listed versioned model and refreshes disclosure/fingerprint. It never silently substitutes another model.
- **View Google terms/pricing** opens the current first-party page in the system browser; it sends no repository source.
- **Confirm privacy and data use**, **Confirm billing uncertainty**, and **Confirm age, region, and use** are separate, unchecked controls. They must all be checked with a current reviewed fingerprint before send.
- **Send this request** makes exactly one request matching the reviewed fingerprint. It is the only control on this surface that sends source context to Google. The result is shown as an unaccepted response preview and is not saved as a chapter.
- **Close** discards the temporary review and acknowledgement and sends nothing. During an active request it is disabled; there is no promise that an in-flight HTTPS request can be cancelled.
- Provider failure shows a sanitized named error. The consumed request is not retried; Code Atlas builds a fresh review and clears consent. The previous error remains visible while the new review is prepared.

**States:** no key; Keychain unavailable; snapshot file list loading; local manifest preview; stale/expired manifest; awaiting three acknowledgements; sending; unaccepted response preview; quota/billing/region/network/model error; malformed response. A request review and exact source exist only in Rust/WebView memory for up to fifteen minutes. Key setup and model metadata display are local and do not send prompt content; the key is stored in macOS Keychain.

**Focus/side effects:** opening settings reads provider disclosure metadata and Keychain status. In Request Review, loading the captured-file inventory reads local snapshot metadata only. Building a review reads selected immutable snapshot blobs but makes no network request. No request is sent by opening, filtering, changing context/model, scrolling, key setup, or reading disclosures. Consent is scoped to the exact fingerprint and disclosure version and one send. There is no retry, paid fallback, model discovery, or automatic model switching.

### Future PR Guide

The PR Guide is a separate follow-up workflow. It starts with explicit local base/head refs and a changed-file list. Selecting a changed file opens Code Lens at base or head evidence; **Create PR draft** opens Request Review for the selected subset. Its Back path returns to the same ref pair and file selection. No remote PR, GitHub login, checkout, or write operation is part of this flow.

## Cross-screen state and return table

| Origin | Action | Destination | State restored on Back |
|---|---|---|---|
| Atlas selected file/symbol | Open in Code Lens | Same snapshot, exact anchor | Tree/map selection, viewport, query, filter chips, focused row. |
| Atlas selected relationship | Open evidence | Source range proving that edge | Selected edge and evidence panel. |
| Guide Plan file | Create chapter | Blank local draft | Selected row/status/filter/page; no request exists. |
| Guide Plan file | Generate draft | Request Review | Request cancel returns to selected row and chapter status. |
| Request Review | Send this request | Provider, then Chapter Studio draft | On success, draft is a new unaccepted revision; on failure, manifest can be reviewed again. |
| Chapter Studio draft | Open citation | Code Lens | Draft values, active section, editor scroll, focused claim. |
| Chapter Studio valid draft | Accept | Walkthrough at first section | Guide Plan shows accepted revision; prior history remains. |
| Walkthrough diagram/citation | Open target | Atlas or Code Lens | Chapter/revision/section, audio state/position, transcript scroll, hotspot/citation focus. |
| New snapshot | Assess freshness | Per-section status, evidence reasons, and exact old/new source ranges | Prior accepted guide, snapshot, and assets are unchanged. |
| Stale section | Carry forward | New local draft with old/new anchors | Old accepted section remains available. |
| Stale section | Regenerate | Request Review for one selected section | Cancel/reject keeps accepted revision; accept creates a new chapter manifest. |

## Product-level acceptance walkthrough

1. Launch with no project; verify no source/provider request and that Open repository is focused.
2. Choose a temporary repo, cancel once, then choose it again; verify no bytes are captured until Create snapshot.
3. Capture; inspect counts/reasons; navigate Atlas → Code Lens; Back must restore selection, map viewport, and focus.
4. Open Guide Plan and verify each inventory path has a visible status. Create one file draft; no network call occurs.
5. Generate that file; inspect exact request manifest; verify the send button is disabled until all three separate acknowledgements; Close; verify no request and origin state is restored.
6. Repeat with explicit consent in a mocked provider flow; land in Chapter Studio as an unaccepted draft; click a citation and return to the same section/editor position.
7. Accept valid content; in Walkthrough use Previous/Continue, select a section directly, click a hotspot, return, and verify section/playback/transcript state.
8. Trigger permission, parser, network, quota, malformed response, stale anchor, and persistence failures; verify the recovery actions from the product blueprint.

This walkthrough defines the behavior M0 must implement. Later feature issues add their own domain-specific acceptance; they must preserve these shared transitions and side-effect boundaries.
