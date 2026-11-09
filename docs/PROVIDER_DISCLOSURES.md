# Provider and data disclosures

Code Atlas 0.1 uses your connected Codex account for explanations and local Pocket TTS for optional narration. Direct API providers, Claude, Antigravity, and cloud speech are not available in the first-release app.

## Explanations with Codex

Repository browsing and search stay on this Mac. When you first ask about a captured snapshot, Code Atlas asks whether to let Codex use relevant code from that snapshot. If you allow it, the connected Codex runtime receives the question and the bounded captured context it selects. Later questions for that same snapshot can reuse the permission; a different snapshot or account needs its own permission.

Code Atlas uses the installed Codex account and does not ask for or store an API key. Connecting, signing in, and loading available model settings may contact Codex account services without sending repository source. Explanation requests can use the connected account's quota and are subject to its current terms and usage limits. Code Atlas cannot see your remaining quota or promise that a request is free.

## Narration with Pocket TTS

Pocket prepares and synthesizes narration on this Mac. The selected narration text is not sent to a cloud speech provider. Initial setup downloads the local runtime, model and voice files; the prepared voice works offline. Pocket TTS has no provider usage fee, but setup uses local disk space and network access to download its assets. The app currently offers the Alba English voice; see [voice attribution and licenses](../THIRD_PARTY_NOTICES.md).

Playback is manual. The app reuses matching audio from its local cache and keeps text and source navigation available if voice setup or synthesis is unavailable.

## Local data and permission

Repository snapshots, generated explanations and narration audio are stored in the app's local data directory. A snapshot permission authorizes Codex to receive relevant source from that snapshot; it does not grant access to the live working tree or other repositories. Review [local data and permissions](LOCAL_DATA.md) for storage and revocation details.

## Deferred provider research

Detailed disclosures for former direct API and Gemini speech prototypes are preserved in [historical provider notes](history/provider-disclosures-pre-0.1.md). They do not describe selectable first-release routes. Future provider work must recheck current terms, pricing, retention and eligibility against primary sources before release.
