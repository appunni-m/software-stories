# Kata backlog — Code Atlas rebuild

**Snapshot: 9 November 2026.** The live `code-atlas-rebuild` project has 82 records: 32 open, 32 closed, and 18 soft-deleted. Twenty-one open issues are ready and 11 are blocked; Kata is authoritative for current status, priority, ownership, comments, and dependency state.

> **Scope update — 9 November 2026:** first release is Codex-first. Formal
> accessibility audit matrices are out of scope; ordinary keyboard use,
> readable labels, and light/dark layouts remain baseline usability. PR
> explanations, standalone offline export, the direct-API catalog, non-Codex
> agent integrations, full revision compare/restore, and generation-time audio
> prewarming are post-1.0. Their task definitions are preserved in [post-1.0
> history](history/post-1.0/README.md); restore a Kata issue before scheduling
> that work. Selective section repair and cached Pocket replay remain in 0.1.0.
> The duplicate all-pages workspace audit `na54` is soft-deleted; its unique
> **Load latest changes** requirement is tracked in `13yp`. Repository Guides
> stream an outline and useful sections; there is no separate accepted-route
> batch-building wizard or elapsed-time SLA. The duplicate narration wrapper
> `70x5` is retired; runtime setup and playback/cache remain in `q6g3` and
> `dbmd`. Prewarming has no active issue.

## Find current work

- Live issue state is in the maintainer's configured Kata project. From that workspace, `kata list --project code-atlas-rebuild` shows records and `kata ready --project code-atlas-rebuild --agent` shows dependency-ready work.
- [Kata status snapshot](KATA_STATUS.md) — reconciled issue IDs, statuses, priorities, and dated evidence as of 9 November.
- [Task definitions](kata-tasks/) — user outcomes, behavior contracts, acceptance criteria, exclusions, and closure evidence. A definition describes required work; it is not proof that the work passed.
- [Acceptance records](acceptance/) — build-, repository-, and date-scoped observations. Each record proves only the path it describes.

Use the issue comments and linked acceptance records before closing an issue. Do not infer completion from a task-file update, source presence, a successful build, a synthetic fixture, or a different issue's result.

## Milestone map

| Milestone | Kata reference | Definition |
| --- | --- | --- |
| M0 — Coherent Mac product shell | `cnsm` | [phase 0](kata-tasks/phase-0.md) |
| M1 — Local repository truth | `c92x` | [phase 1](kata-tasks/phase-1.md) |
| M2 — On-demand file explanations and repository Guides | `wzce` | [phase 2](kata-tasks/phase-2.md) |
| M3 — Interactive explainer | `gtte` | [phase 3](kata-tasks/phase-3.md) |
| M4 — Git-like guide revisions | `b4cp` | [phase 4](kata-tasks/phase-4.md) |
| M5 — Dependable first Mac release | `48yv` | [phase 5](kata-tasks/phase-5.md) |

The milestone contracts define intended outcomes. Current implementation and
release evidence are tracked in their linked work items and acceptance records.
The former M6 PR Guide and M7 multi-backend contracts are archived outside the
active first-release milestone map.

## Historical checkpoints

Earlier backlog snapshots, generation outcomes, and implementation notes were
removed from this index because their record counts and statuses became stale.
Their dated evidence remains under [acceptance records](acceptance/); consult
the individual record for its exact build, repository, outcome, and limits.

Code Atlas is still unreleased. There is no signed, notarized public download;
see the [0.1.0 release-readiness draft](releases/0.1.0-draft.md).
