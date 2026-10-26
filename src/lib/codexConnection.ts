import type { CodexAgentStatus, CodexModel } from './commands';

interface ConnectionCommands {
  status: () => Promise<CodexAgentStatus>;
  resume: () => Promise<CodexAgentStatus>;
  models: () => Promise<CodexModel[]>;
  onStatus?: (status: CodexAgentStatus) => void;
}

export function codexSelectionIsSupported(
  models: readonly CodexModel[],
  selection: { modelId: string; effort: string },
): boolean {
  return models.some((model) => model.modelId === selection.modelId
    && model.supportedReasoningEfforts.includes(selection.effort));
}

/** Reuse Codex's saved account. Sign-in is deliberately not part of this path. */
export async function restoreCodexConnection(commands: ConnectionCommands) {
  let status = await commands.status();
  if (!status.activeRunId && (!status.runtimeRunning || !status.authenticated)) {
    status = await commands.resume();
  }
  commands.onStatus?.(status);
  const models = status.authenticated && !status.activeRunId ? await commands.models() : [];
  return { status, models };
}
