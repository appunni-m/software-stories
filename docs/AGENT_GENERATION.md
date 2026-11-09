# How Code Atlas generates explanations

Code Atlas 0.1 uses the connected Codex account for explanations. It uses local Pocket TTS for optional narration. The current connection is visible in the workspace header and is reused across repositories.

## Ask about code

1. Open a repository and create a snapshot. The snapshot gives each explanation a stable source version.
2. Enter a question in the pinned chat bar and choose **Guide me** or **Show the flow**. For a specific implementation, select code or open a file and ask from there.
3. The first question for a snapshot asks permission to share relevant captured code with Codex. You do not need to select reference files; Code Atlas finds bounded source context for the question.
4. Read each validated section in Guide, follow its citations into the captured source, or inspect its structure in Flow. Ask a follow-up or explore a detail from the same workspace.

Generation runs in the background. Completed sections remain available if a later part is interrupted. Code Atlas does not resend an unfinished question automatically after a restart; check its notification before choosing what to do next.

## Listen

Prepare Pocket voice once in **Connections**. Narration text and synthesis stay on this Mac. Playback is manual; matching audio is reused from the local cache when available. Voice preparation is optional, and explanations, citations, and navigation remain usable without it. See [provider and data disclosures](PROVIDER_DISCLOSURES.md).

## What is available in 0.1

- One Codex connection for repository and file explanations. No direct model API key is needed.
- Local repository browsing, search, Git history and source comparisons.
- Guide, Flow, nested follow-up questions, citations and local Pocket narration.
- A question about a repository does not generate a separate chapter for every file; open or select a file when you want a focused explanation.

Direct API providers, Claude Code, Antigravity, manual agent handoffs, AI explanations of Git changes and standalone HTML export are not first-release choices. Their preserved future scope is in [post-1.0 history](history/post-1.0/README.md). Current generation quality and release acceptance remain tracked in [repository narratives](kata-tasks/38-repository-narratives.md), [Codex first-release generation](kata-tasks/40-codex-account-backend.md), and [progressive answer sections](kata-tasks/73-progressive-answer-sections.md).

The details in this document describe the user workflow, not a guarantee that every operation or repository has passed quality acceptance. See the [dated Codex evidence](acceptance/2026-10-16-codex-validation.md) and the [release-readiness draft](releases/0.1.0-draft.md) for the exact evidence limits.
