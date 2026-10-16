# Codex review account and runtime binding

**Status:** built and exercised in the [synthetic native file-chapter flow](acceptance/2026-10-16-codex-validation.md). Account/runtime replacement, refresh, race and adversarial cases below remain unaccepted; one baseline does not qualify production support. Claude remains deferred.

## User behavior

The app restores the existing Codex account and loads its models before preparing a request. This restoration cannot start OAuth or open a browser. Sign-in happens only after an explicit Connect action when no valid account is available; subsequent file, plan and Git requests reuse it. An explicit Disconnect pauses automatic reuse until Connect is selected again. The native review observes the already-running app-server and its ChatGPT account; it does not start a process, renew login, send repository source or request inference. Every Codex chapter, overview, section update, code selection, Git guide, hunk and repository/planned-step request includes that observation in its effective settings, encoded request and fingerprint.

Reconnecting, restarting the process, signing out, observing an account change or replacing the configured executable requires a fresh review. Sending a stale request never silently restarts Codex, changes account, switches backend or selects an API. Saved chapters remain available independently of the connection.

## Native identity and ownership

`CodexReviewRuntime` is a strict native record containing a per-process UUID, canonical executable path and SHA-256, bounded account-metadata digest and monotonic account-event revision. The renderer supplies none of those fields to review commands. Parsing the recorded identity for contract reconstruction does not authorize execution; only the native manager observes the current process/account.

The account digest contains the reported ChatGPT email and plan, without tokens or credentials. The [official stable account schema](https://github.com/openai/codex/blob/main/codex-rs/app-server-protocol/schema/json/v2/GetAccountResponse.json), inspected October 16, exposes a nullable email and plan metadata. It does not establish a guaranteed immutable account/workspace identity for this integration. Reviews currently require a bounded nonempty email; missing metadata returns an actionable error instead of inventing an identity. Plan metadata is not entitlement or quota evidence. See the [official app-server documentation](https://developers.openai.com/codex/app-server) for its account and authentication surfaces.

Native preflight rebuilds the complete reviewed contract with a fresh observation before consumption. The worker then retains its original process/run owner, refuses a missing/replaced connection, compares observations before model selection and source staging, and checks again after generation before local validation/publication. An observed `account/updated` event advances the native revision. If one arrives while preparing the thread, no model turn starts; if one arrives during generation, the run fails and its result is discarded. Result access compares the exact recorded runtime as well as the original review/fingerprint, cancellation and deadline. Existing interruption notices remain available after failures.

Ordinary dispatch uses `account/read` without requesting token refresh. Connection restoration and explicit model setup retain the existing account refresh path; the runtime owns credentials and normal token renewal. An account event seen during a run requires a new review even if its visible metadata later looks the same. This conservative behavior still needs real refresh/lifecycle acceptance.

## Executable observation and limits

Hashing opens only the configured regular executable, rejects a final symlink where supported, streams at most 1 GiB plus a sentinel, checks cancellation between reads, and rejects length/modification changes during the read. Startup compares observations before and after spawning. Explicit connection/model setup can replace an old process when its executable changed; generation cannot do so using old consent. No shell, executable or package manager runs merely to compute the digest.

These observations do not eliminate filesystem replacement races around process execution, authenticate an installation, bind every interpreter/library/child dependency, prove that account events cover all external credential changes, or distinguish every workspace sharing the same visible metadata. The process lock covers this app's own account/configuration commands, not outside credential writers. A pinned trusted package, exact supported account identity/route, actual runtime/OS boundary and native lifecycle evidence remain required. Hashing overhead and cancellation responsiveness have not been measured.

## Pending acceptance

Kata `z3an`, `tpta`, `2r6n` and `xymb` retain the actual account/runtime and all-operation gates. Exercise fresh review/send, account refresh/logout/switch, absent metadata, reconnect/restart, executable replacement, model/effort changes, queued notifications, cancellation/deadline, result rejection and interruption recovery on the real packaged app when validation is authorized. Builds, automated checks and one native file-chapter baseline now passed; they do not establish the remaining lifecycle/change cases above.
