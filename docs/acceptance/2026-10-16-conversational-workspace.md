# Conversational code workspace — 16 October 2026

## Scope and status

The latest user direction is recorded in [the workspace contract](../design/WORKSPACE_EXPLORATION.md). It supersedes the earlier Guide / Files / Changes design authority. Source commits `eb0e702` and `4e3e80d` contain a real-component interactive design preview, a production notification center and compact header controls. The new exploration shell is **not yet the native application**. Seven open Kata tasks define its migration and acceptance (`nnsf`, `eapn`, `nsfd`, `nw1h`, `mrpj`, `hstr`, `3rcd`). No broad quality or release task was closed.

## Production changes

- Runtime notices, general workspace feedback, interrupted operations and pending chapter previews are accessible from a standard non-modal notification bell. They no longer form a banner stack above source content.
- Notifications do not automatically open, send a request, save or accept a chapter. Existing native recovery and explicit preview actions remain in use. The badge reports available updates.
- Escape/outside close and ordinary actions return focus to the trigger. An action explicitly opening the preview dialog preserves that dialog's own focus. Background updates only announce a count politely.
- The header shows a compact Codex connection. Shared model/reasoning configuration remains in connection settings. Settings uses a compact icon; keyboard help is inside its menu. The footer no longer prints internal source identities and platform mechanics.
- Existing capture-specific screens and generation dialogs are still present. Moving those workflows into the background requires native job ownership and scoped sharing authorization; simply hiding or unmounting their dialogs would cancel active work. General notices are still a single transient item, not a durable event history.

## Interactive sample checks

The preview ran on the existing fixed port 4173. Only one Vite server was started. All content is illustrative local checkout code; no AI invocation, private-source transmission or audio generation occurred.

Observed with computer use in the browser:

| Interaction | Observation |
| --- | --- |
| Guide / Flow | Explorer, composer and code pane remained in place. Explanation views switched in the same exploration. |
| Citation | Revealed `cart.ts` lines 8–13 inline, with the selected-code action visible. |
| Explain this code | Added a selected-code answer card at the existing root level; no extra Send, dialog or new branch. |
| File drag | Dragging `cart.ts` from the explorer into the composer added a removable context chip without submitting. |
| Parallel search | Typing `subtotal` in the composer produced exact file/line matches and Explore with AI. |
| Keyboard search | `⌘P` opened quick search. A question with no literal match retained Explore with AI. Escape closed it. |
| View shortcut | `⌘1` returned to Guide. |
| Nested exploration | Explore subtotal opened a child Flow with a function-level validation branch. Back restored the root Flow and its prior follow-up. |
| Reload | Reopened the same child exploration on its last Flow view. This does not prove native persistence or restoration of every source/viewport/player field. |
| Notifications | Bell opened a compact panel; Escape closed it. Dismiss removed the sample update, closed the panel and visibly returned keyboard focus to the bell. |
| Light / dark | Source, selection, graph, cards and notification surfaces were inspected in both appearances. |

Images in `docs/design/workspace-*.jpg` are screenshots of this sample preview, not generated chapter quality or native-app evidence. The preview's visual player is not a TTS implementation. Sample responses and limited graphs cannot substitute for the real background generator, arbitrary-repository search, chapter revisions, sharing grants or native lifecycle qualification.

## Build and native boundary

- `npm test`: **120 passed**, zero failures. Existing source-structure assertions were updated to follow recovery actions into the bell; their native command/save/discard checks remain.
- `npm run build`: passed. Type checking includes preview source; the normal production bundle contains the application, not the separate preview HTML entry. The existing bundle-size warning remains.
- The first unsigned native build failed for disk space. A second build emitted Cargo's dependency artifact inventory; unused generated dependency artifacts outside that admitted inventory were removed from this checkout's release cache, reclaiming about 581 MiB. Project source, application data and saved explanations were preserved. That app/DMG build and metadata/checksum verification passed.
- The initial notification bundle was staged, checked and installed with its prior bundle retained. A final build incorporates compact header controls and the notification action focus fix; its install evidence is appended below.
- Computer use could not access the native app because the Mac was locked and automatic unlock failed. The lock was not bypassed and the existing app process was not terminated through an alternate UI-control mechanism. Native visual/focus, VoiceOver, 200% reflow, contrast and motion acceptance remain pending. Restart the app after unlocking to use an updated installed bundle.

Full production workspace integration, durable jobs, explicit workspace sharing scope, complete language narratives and accessibility remain open. No live Codex or Pocket validation was repeated for this frontend iteration.

## Final installed notification/header build

`CARGO_INCREMENTAL=0 npm run bundle:macos` completed from source commit `4e3e80d`, producing the unsigned app/DMG and passing metadata/checksum verification. Installed app: `/Users/lazytrot/Applications/Code Atlas.app`. Executable SHA-256: `a6c8ed9343f0a933ae5c7ed44105c6e2a8870ba267cc46d264962ebe377657cb`. Installation used a staged copy, binary comparison and replacement with the preceding bundle retained at `/Users/lazytrot/Applications/.Code Atlas.previous-4e3e80d.app`. No app data was removed and no source-sharing operation was triggered. The running old app was not force-quit; after unlocking, quit and reopen Code Atlas to load the update. Native visual acceptance is still pending. The interactive workspace preview remains a separate development entry, not the new native shell.
