import type { PrChangeExplanationTarget, SelectedGitChangeIdentity } from './commands';

export type AutomaticAgentId = 'codex' | 'claude-agent';

/** Local journal-owned selection only. No connection, settings or consent
 * survives an interruption. Each agent starts a fresh reviewed operation. */
export interface AgentGitRecovery {
  operationId: string;
  backend: AutomaticAgentId;
  mode: ({
    kind: 'pr-guide'; projectId: string; baseRef: string; headRef: string;
    comparisonId: string; selectedChanges: SelectedGitChangeIdentity[];
  } | {
    kind: 'pr-change'; projectId: string; target: PrChangeExplanationTarget;
  }) & { recoveryOperationId: string };
}
