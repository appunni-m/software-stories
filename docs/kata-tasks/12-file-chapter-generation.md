---
kind: task
key: t12
parent: m2
title: Generate one substantive, source-grounded chapter per selected file
priority: 0
labels: phase-2, chapters, provider
blocked_by: t09, t10, t11
---

## User outcome
The user selects one file and gets an explanation of what its actual code does, not a generic paragraph derived from its path or a repository overview.

## Contract
Generate only for the selected file/group; never start bulk generation. The reviewed manifest includes the target bytes and only explicitly selected, bounded supporting context. The output explains the file's responsibility with evidence; important declarations and execution path; applicable inputs, outputs, state changes, side effects, and error paths; resolved callers/imports/dependents/tests; and what evidence does not prove. Chapter depth follows file complexity. Simple files stay concise but specific.

## Acceptance criteria
- Each eligible file has an independent chapter target by default and can be generated without an overview.
- Claims are structured and anchored to the selected snapshot; relationship facts include type/confidence.
- Output cites actual functions/types/configuration and describes their behavior; generic filler and unsupported runtime/business-flow claims fail quality validation.
- The provider response is bounded, schema-validated, citation-validated locally, and saved as an unaccepted draft.
- Provider cancellation or malformed output leaves existing drafts and accepted chapters intact.

## Excludes
Auto-acceptance, generated cinematic video, a required introduction, unselected supporting context, and automatic file grouping.

## Done evidence
Generate chapters for a real multi-file fixture with simple, complex, test, and configuration files; trace specific claims to source/tests and show that only the selected target becomes a draft.
