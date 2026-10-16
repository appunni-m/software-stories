# Codex validation — October 16, 2026

## Scope and authorization

The user authorized app builds, tests and a small synthetic Codex account generation. The user deferred Claude; no Claude runtime qualification was performed. No direct API key was used. The model request used **`gpt-6-luna` with `max` reasoning**, preserving the user's model constraint.

## Runtime discovery correction

Shell lookup selected `/Users/lazytrot/.local/bin/codex` (0.149.1). Its catalog advertised GPT-5.6 Luna, but not GPT-6 Luna. The user correctly pointed out another installed Codex. The desktop-bundled executable is:

```text
/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex
```

It reports `0.162.0-alpha.17.2`, and its native `model/list` includes `gpt-6-luna` and `max`. Account observation reports the ChatGPT route. No credentials were copied or recorded. One official 0.163.0 npm candidate was downloaded into a temporary tool directory before the installed desktop binary was identified; it was not selected in Code Atlas and did not replace either existing installation. The temporary candidate was then removed. Its model/runtime route was not qualified.

The native Code Atlas setup was explicitly saved with the desktop-bundled executable above. Do not infer the intended executable from the first shell PATH match or substitute an older model.

## Real synthetic generation: passed baseline

A temporary Git repository contained only [cart.ts](../../benchmarks/codex-account-smoke/cart.ts). The official app-server ran over stdio with ChatGPT authentication, the exact requested model/effort, an ephemeral thread, approval `never`, ordinary read-only sandboxing, disabled command/web/plugin/agent features and a JSON output schema. The prompt supplied the complete synthetic file directly. One turn completed successfully, without a tool event. Local checks confirmed exactly two named functions, exact unique source quotes and unchanged source bytes.

The [redacted result record](../../benchmarks/codex-account-smoke/2026-10-16-result.json) contains only synthetic source/output and runtime/settings metadata. It omits credentials, account email, login URLs and account/thread identifiers. This baseline proves the selected account completed that request; model catalog membership alone would not establish access.

This **does not qualify the Code Atlas adapter**. The baseline used the supported ordinary read-only policy and a minimal response contract. It did not bypass the app's stricter gate, create a Code Atlas chapter, exercise its consent/draft validation/publication, or prove all required operation/cancellation/recovery boundaries. The account request uses plan quota.

## Packaged app reproduction: integration still blocked

1. Build and open the local unsigned Code Atlas package.
2. Choose the temporary synthetic Git repository, create a local snapshot and open `cart.ts` in Files. File opening and the explanation action worked without crashing.
3. Choose **Explain this file → Use Codex**, enter the desktop executable above and select **Save executable**.
4. Select **Connect ChatGPT account**.
5. The adapter rejects the runtime during its local schema check, before starting login or sending source/model input.

The selected runtime's generated stable **and experimental** schemas contain ordinary `readOnly` sandbox policies, structured turn output and ephemeral threads, but do not expose `ReadOnlyAccess`/restricted roots (`access`, `readableRoots`, `includePlatformDefaults`). The app requires those fields. Official documentation describes restricted reads, but that is not evidence that this installed runtime implements them. The existing adapter remains gated; removing the check or allowing ignored parameters would not prove the declared boundary.

The independent baseline additionally reproduced `thread/start` rejecting `sandbox: "readOnly"` with JSON-RPC `-32600`: this field expects **`"read-only"`**. `turn/start.sandboxPolicy.type` separately uses `"readOnly"`. The source now uses the proper ThreadStartParams spelling and has a regression test. The old generic setup catch hid native failures; the app now displays the native controlled compatibility error and lists the missing capabilities.

## Build and test evidence

The initial run found and repaired:

- An overbroad manual-handoff union admitting planning and PR scopes without snapshot fields.
- Literal JSON braces left unescaped in Rust formatted chapter/visual prompts.
- Tauri async commands with borrowed State returning a custom envelope without the required outer Result. IPC payload shapes remain the same.
- A native test constructor missing its visual argument; old no-hotspot/error-text/schema-type assertions; a checker referring to a renamed parser-language set; a stale recovery label assertion.
- A missing accessible group role on source citations and an accessibility checker missing the valid progressbar role.
- The Codex thread sandbox spelling and hidden native connection error described above.

Final `npm run check` passed: 98 frontend tests; 181 Rust tests passed, one ignored; the offline corpus check, production frontend build and Cargo check passed. This includes the two focused Codex protocol regressions. Eight existing native dead-code warnings and a frontend chunk-size warning remain; they did not fail the build. Final `npm run bundle:macos` also passed. After restarting the final package, the saved desktop executable persisted; opening the saved synthetic snapshot/file and choosing Codex worked. Connect displayed the precise missing restricted-read capabilities above. Package and UI results are also recorded in `xymb`.

`npm run bundle:macos` produced the local unsigned app and aarch64 DMG; the packaging script verified metadata and the DMG checksum. Unsigned local packaging does not qualify signing, notarization, accessibility or release readiness.

## Required next work — `z3an`, priority 0

- Match the adapter's actual protocol and filesystem boundary to the **installed desktop runtime**. Use a runtime that verifiably supports restricted reads, or design and qualify an equivalent app-owned boundary with supported official protocol fields. Do not silently weaken the declared boundary or retry using a different provider/model.
- Prefer/discover the intended installed desktop executable rather than requiring users to guess from PATH; retain explicit selection and change invalidation.
- Validate effective disabled tools/configuration and the app-owned account connection after compatibility is resolved.
- Complete the synthetic **in-app** review → consent → generate → validated preview → saved unaccepted chapter flow, then cancellation/recovery and each remaining operation in the matrix.

Keep `z3an`, `2r6n` and the relevant validation criteria open. Claude stays deferred in `8gqw`; the broader product and multi-backend milestone remain incomplete.

## Primary protocol references

- [Official app-server protocol](https://learn.chatgpt.com/docs/app-server): initialization, native account/model discovery, thread/turn lifecycle and read-access contract. Installed schemas must be checked independently.
- [Official Codex authentication](https://learn.chatgpt.com/docs/auth): account-managed authentication and storage.
