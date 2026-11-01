# Documentation map

Use this page to find the current user journey, implementation contracts, and dated evidence without treating old plans or screenshots as proof of shipped behavior.

## Use Code Atlas

- [Getting started](GETTING_STARTED.md) — source-checkout setup and a first native session.
- [Local data and permissions](LOCAL_DATA.md) — what the app stores, where it lives, and how sharing is scoped.
- [Troubleshooting](TROUBLESHOOTING.md) — recovery steps for common failures.
- [Language capabilities](LANGUAGE_CAPABILITIES.md) — parser and index support, with limitations.
- [Offline export](OFFLINE_EXPORT.md) — export contents, navigation, and omissions.
- [Provider disclosures](PROVIDER_DISCLOSURES.md) — account-backed agents, direct APIs, and speech providers.

## Understand the implementation

- [Current workspace design](design/WORKSPACE_EXPLORATION.md) — the activity bar, persistent source, chat, explorations, and Guide / Flow / history contract. Its preview is illustrative sample data, not packaged-app evidence.
- [Chapter schema](CHAPTER_SCHEMA.md) and [generation records](CHAPTER_GENERATION_RECORDS.md) — persisted answer, evidence, visual, revision, and recovery formats.
- [Visual scenes](VISUAL_SCENES.md) — the safe, app-rendered scene vocabulary and source-link rules.
- [Agent generation](AGENT_GENERATION.md), [operation parity](AGENT_OPERATION_PARITY.md), and [result lifecycle](AGENT_RESULT_LIFECYCLE.md) — distinguish shipped paths from deferred or unqualified agent operations.
- [Repository retrieval architecture](architecture/repository-retrieval.md) — current bounded retrieval components. Dated indexing research is in [Research](research/).
- [Interaction contract](INTERACTION_CONTRACT.md) — detailed behavior and navigation requirements.

Source code, lockfiles, native configuration, and CI are authoritative for implementation behavior. A design contract states intended behavior; it does not prove that the current app implements it.

## Release, quality, and operations

- [0.1.0 release-readiness draft](releases/0.1.0-draft.md) — release gates and evidence; this is not a published release.
- [Chapter quality gate](CHAPTER_QUALITY_GATE.md) and [benchmark corpus](../benchmarks/chapter-quality/corpus/README.md) — synthetic measurements and the required human review.
- [Accessibility validation](ACCESSIBILITY_VALIDATION.md), [performance validation](PERFORMANCE_VALIDATION.md), and [security validation](SECURITY_VALIDATION.md) — scoped checks, limitations, and open work.
- [Local-data lifecycle](LOCAL_DATA.md) and [provider disclosures](PROVIDER_DISCLOSURES.md) — storage and outbound-data boundaries.
- [Kata backlog map](KATA_BACKLOG.md) and [task contracts](kata-tasks/) — planned and open work. The Kata service is authoritative for current issue status; task files are not completion evidence.

## Historical, planned, and dated material

- [AI Guide Flow](AI_GUIDE_FLOW.md) describes earlier route-planning contracts. The staged Guide wizard is historical; the current workspace starts from chat and discovers code in the background.
- [Product blueprint](PRODUCT_BLUEPRINT.md) and [technical plan](TECHNICAL_PLAN.md) mix target design and implementation notes. Check their labels and verify current behavior in source before relying on a statement.
- [Proposed simplification](design/PROPOSED_SIMPLIFICATION.md), [image prompt](design/IMAGE_PROMPT.md), and older design images are historical design references. The current workspace contract above supersedes them where they disagree.
- [Pending work](PENDING_WORK.md) is an October 2026 engineering checkpoint, not a live task list. Use Kata for current status.
- [Acceptance records](acceptance/) are dated observations for a named build, repository, and path. They prove only the exact scope recorded; they do not establish general quality, accessibility, or release readiness.
- [Research notes](research/) are dated investigations. Recheck time-sensitive claims against primary sources and current implementation.

## Project trust routes

- [Contributing](../CONTRIBUTING.md)
- [Security reporting](../SECURITY.md)
- [Support](../SUPPORT.md)
- [MIT license](../LICENSE), [third-party notices](../THIRD_PARTY_NOTICES.md), and [full bundled license texts](../THIRD_PARTY_LICENSE_TEXTS.txt)

The project does not currently publish a monitored private security-reporting route, a support response-time guarantee, or a signed/notarized app download. See the linked files and the release-readiness draft for those limits.
