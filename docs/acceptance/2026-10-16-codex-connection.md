# One-time Codex connection — 16 October 2026

## Report and cause

The user reported being asked to set up the connection on every request. `CodexChapterDialog` reset its account/model state on every mount and unconditionally entered `startCodexLogin` whenever an executable was found. That command can start OAuth and open the browser. Until the asynchronous status check completed, the dialog treated an unknown account as disconnected and expanded the setup form. The connected account itself was held by the native app manager, so request-dialog setup and connection ownership were inconsistent. Native inspection also confirmed that the Connect button remained visible inside connected settings: its `hidden` attribute was overridden by button styling. The connected state now omits that button entirely.

## Fix

- Opening an explanation checks the existing native connection and restores the Codex-managed account silently when the app has restarted. The restoration command has no OAuth or browser-opening path and sends no repository source or model prompt.
- Only an explicit Connect action can begin sign-in when needed. A connected request shows one compact Codex/model/reasoning row, with configuration available on demand. During the initial check it shows a reuse status, without a disconnected setup form.
- The native manager owns the runtime across file, plan, range and Git request dialogs. Supported model/reasoning preferences remain shared and are not erased by an empty catalog or an active operation.
- Disconnect closes this app's runtime and saves `autoConnect: false` with the installed executable path. Future dialogs and a new app manager respect that pause. A deliberate Connect enables reuse again. Older executable-only configuration defaults to automatic reuse.
- Source review and consent remain specific to each generation. The fix does not retain consent, send source automatically, replace the account for a reviewed request or weaken native review identity checks. No credential is stored in browser preferences.

## Executed checks

| Check | Result |
| --- | --- |
| `npm test` | 111 passed; seven focused connection tests cover repeated opens, restart restoration, missing/expired account, disconnect, active generation, catalog failure and absence of initialization-time sign-in. |
| `npm run build` | Passed. Existing bundle-size warning remains. |
| `CARGO_INCREMENTAL=0 cargo test --manifest-path src-tauri/Cargo.toml --lib` | 187 passed, two explicit live checks ignored. New cases cover legacy settings and a persisted disconnect across new manager instances without spawning an executable. |
| Explicit installed-account metadata test | Passed separately using the existing desktop-bundled Codex. Silent restoration found the ChatGPT account and GPT-6 Luna/max catalog. Repeated restoration retained the same connection UUID. A new manager restored the account after the prior manager shut down. No OAuth, source transmission or model generation was requested. |

The live metadata check used `/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex`. It did not install a runtime or use a direct API key.

## Native app observations

The final unsigned Code Atlas 0.1.0 app and DMG were built and the DMG checksum verified. The installed executable SHA-256 is `5992ddc196557f608d2cb29210aab58b093ff58ece2bfdf595aec586b6adda9f`.

- A request for synthetic `cart.ts:1–6` reused the installed account and GPT-6 Luna/max. During restoration the app displayed a reuse status, without an expanded setup form or browser sign-in. The ready connection row was collapsed.
- Closing and reopening the request reused the same effective settings without selecting a backend, installation, account or model again.
- Explicit Disconnect paused reuse on the next request. That pause remained after quitting the app and launching the final installed build: Settings displayed “Codex is disconnected” and an explicit Connect action.
- In the final build, connected Settings exposed Refresh and Disconnect, with no Connect button in the accessibility tree.
- Opening the real fontdone file explanation also reused the same account/model without backend selection or connection setup. After a further full quit/relaunch with reuse enabled, the same file again reached a ready GPT-6 Luna/max local review automatically. Consent remained unchecked. Only local source reviews were prepared.
- Explicit Connect reused the existing ChatGPT account and preserved GPT-6 Luna/max. This inspected account already had valid Codex credentials; a new-account OAuth flow was not exercised.

No source was sent and no generation was started during these UI checks. The saved synthetic source range was used for the connection screen. [Connected request screenshot](../design/native-codex-reuse-2026-10-16.jpg).

## Remaining limits

These checks cover connection reuse, not every backend operation or a production release. Expired-login UI recovery, network/account changes during an active generation, clean-account distribution and the broader `z3an` / `2r6n` acceptance matrix remain open. No new generated-content quality claim is made.
