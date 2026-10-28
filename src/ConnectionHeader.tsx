import { useEffect, useState } from 'react';
import { CheckCircle2, CircleAlert, LoaderCircle, PlugZap } from 'lucide-react';
import { AppButton } from './design/AppButton';
import { initializeConnection, useConnectionState } from './lib/connectionStore';
import { codexSelectionIsSupported } from './lib/codexConnection';
import { readDirectApiSelection, readDirectApiSelectionIssue, readGenerationBackend, readHandoffAgent, type GenerationBackend, type DirectApiSelection, type DirectApiSelectionIssue } from './lib/generationPreferences';

export function ConnectionHeader({ onOpen }: { onOpen: () => void }) {
  const connection = useConnectionState();
  const [backend, setBackend] = useState<GenerationBackend | null>(() => readGenerationBackend());
  const [apiSelection, setApiSelection] = useState<DirectApiSelection>(() => readDirectApiSelection());
  const [apiSelectionIssue, setApiSelectionIssue] = useState<DirectApiSelectionIssue | null>(() => readDirectApiSelectionIssue());
  const [handoffAgent, setHandoffAgent] = useState(readHandoffAgent);
  useEffect(() => {
    if (backend === 'codex') initializeConnection();
    const refreshBackend = () => setBackend(readGenerationBackend());
    const refreshApiSelection = () => {
      setApiSelection(readDirectApiSelection());
      setApiSelectionIssue(readDirectApiSelectionIssue());
    };
    const refreshHandoffAgent = () => setHandoffAgent(readHandoffAgent());
    window.addEventListener('code-atlas:connection-changed', refreshBackend);
    window.addEventListener('code-atlas:direct-api-selection-changed', refreshApiSelection);
    window.addEventListener('code-atlas:handoff-agent-changed', refreshHandoffAgent);
    return () => {
      window.removeEventListener('code-atlas:connection-changed', refreshBackend);
      window.removeEventListener('code-atlas:direct-api-selection-changed', refreshApiSelection);
      window.removeEventListener('code-atlas:handoff-agent-changed', refreshHandoffAgent);
    };
  }, [backend]);
  const codexSelected = backend === 'codex';
  const codexRuntimeReady = codexSelected && connection.status?.authenticated && connection.status.runtimeRunning;
  const providerName = apiSelection.providerId === 'gemini' ? 'Gemini'
    : apiSelection.providerId === 'openai' ? 'OpenAI'
      : apiSelection.providerId === 'anthropic' ? 'Anthropic' : apiSelection.providerId;
  const selectedModel = connection.models.find((model) => model.modelId === connection.selection.modelId);
  const codexModel = selectedModel?.displayName ?? connection.selection.modelId;
  const handoffAgentName = handoffAgent === 'claude-code' ? 'Claude Code'
    : handoffAgent === 'codex' ? 'Codex' : handoffAgent === 'antigravity' ? 'Antigravity' : 'Another agent';
  const modelReady = codexSelectionIsSupported(connection.models, connection.selection);
  const ready = Boolean(codexRuntimeReady && modelReady);
  const label = backend === null ? 'Choose connection'
    : backend === 'api'
    ? apiSelectionIssue ? 'API · Saved selection unavailable' : `API · ${providerName || 'Choose provider'}${apiSelection.modelId ? ` · ${apiSelection.modelId}` : ''}`
    : backend === 'claude-setup' ? 'Claude Agent'
      : backend === 'handoff' ? `${handoffAgentName} handoff`
        : connection.checking ? 'Connecting to Codex…'
          : ready ? `Codex · ${codexModel} · ${connection.selection.effort}`
            : codexRuntimeReady ? 'Choose Codex model' : 'Connect Codex';
  const Icon = codexSelected && connection.checking ? LoaderCircle : ready ? CheckCircle2
    : backend === null || backend === 'api' && Boolean(apiSelectionIssue) || codexSelected && (connection.error || codexRuntimeReady && !modelReady) ? CircleAlert : PlugZap;
  const details = backend === null ? 'The saved AI connection could not be read. Choose a connection to replace it.'
    : backend === 'codex'
    ? ready ? `Connected${connection.status?.accountEmail ? ` as ${connection.status.accountEmail}` : ''}. Model ${codexModel}, reasoning ${connection.selection.effort}.`
      : codexRuntimeReady ? 'The saved Codex model or reasoning level is unavailable. Choose a listed model and supported reasoning level in Connections.' : 'Set up your Codex connection once'
    : backend === 'api' ? apiSelectionIssue ? `The saved direct API selection could not be read (${apiSelectionIssue}). Choose a provider in Connections to replace it.`
      : `${providerName ? `${providerName}${apiSelection.modelId ? ` · ${apiSelection.modelId}` : ''}` : 'Direct API selected without a provider'}. Open Connections to change provider or model.`
      : backend === 'claude-setup' ? 'Claude Agent selected. Open Connections to manage setup.'
        : `${handoffAgentName} handoff selected. Agent setup and model settings stay in that app.`;
  return <AppButton intent="custom" className={`connection-header ${ready ? 'ready' : ''}`} onClick={onOpen}
    aria-label={`${label}. ${details} Open Connections.`} title={`${details} Voice settings are also in Connections.`}>
    <Icon size={15} /><span className="connection-header-value">{label}</span>
  </AppButton>;
}
