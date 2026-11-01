# Kata backlog — Code Atlas rebuild

**Snapshot: 1 November 2026.** The live `code-atlas-rebuild` project has 82
records: 51 open, 31 closed, and 30 open issues ready under the current
dependencies. This count is a dated snapshot; Kata is authoritative for live
status, priority, ownership, comments, and dependency state.

## Find current work

- Live issue state is in the maintainer's configured Kata project. From that workspace, `kata list --project code-atlas-rebuild` shows records and `kata ready --project code-atlas-rebuild --agent` shows dependency-ready work.
- [Kata status snapshot](KATA_STATUS.md) — reconciled issue IDs, statuses, priorities, and dated evidence as of 1 November.
- [Task definitions](kata-tasks/) — user outcomes, behavior contracts, acceptance criteria, exclusions, and closure evidence. A definition describes required work; it is not proof that the work passed.
- [Acceptance records](acceptance/) — build-, repository-, and date-scoped observations. Each record proves only the path it describes.

Use the issue comments and linked acceptance records before closing an issue. Do not infer completion from a task-file update, source presence, a successful build, a synthetic fixture, or a different issue's result.

## Milestone map

| Milestone | Kata reference | Definition |
| --- | --- | --- |
| M0 — Coherent Mac product shell | `cnsm` | [phase 0](kata-tasks/phase-0.md) |
| M1 — Local repository truth | `c92x` | [phase 1](kata-tasks/phase-1.md) |
| M2 — Explain every eligible file | `wzce` | [phase 2](kata-tasks/phase-2.md) |
| M3 — Interactive explainer | `gtte` | [phase 3](kata-tasks/phase-3.md) |
| M4 — Git-like guide revisions | `b4cp` | [phase 4](kata-tasks/phase-4.md) |
| M5 — Dependable first Mac release | `48yv` | [phase 5](kata-tasks/phase-5.md) |
| M6 — Local PR Guide follow-up | `hsn3` | [phase 6](kata-tasks/phase-6.md) |
| M7 — Selectable AI backends | `mfz2` | [phase 7](kata-tasks/phase-7.md) |

The milestone contracts define intended outcomes. Current implementation and
release evidence are tracked in their linked work items and acceptance records.

## Historical checkpoints

Earlier backlog snapshots, generation outcomes, and implementation notes were
removed from this index because their record counts and statuses became stale.
Their dated evidence remains under [acceptance records](acceptance/); consult
the individual record for its exact build, repository, outcome, and limits.

Code Atlas is still unreleased. There is no signed, notarized public download;
see the [0.1.0 release-readiness draft](releases/0.1.0-draft.md).
