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
  const label = backend === null ? 'Connect AI'
    : backend === 'api'
    ? apiSelectionIssue ? 'Review AI connection' : `${providerName || 'Choose provider'}${apiSelection.modelId ? ` · ${apiSelection.modelId}` : ''}`
    : backend === 'claude-setup' ? 'Claude Agent'
      : backend === 'handoff' ? `${handoffAgentName} handoff`
        : connection.checking ? 'Connecting to Codex…'
          : ready ? `Codex · ${codexModel}`
            : codexRuntimeReady ? 'Choose model' : 'Connect Codex';
  const Icon = codexSelected && connection.checking ? LoaderCircle : ready ? CheckCircle2
    : backend === null || backend === 'api' && Boolean(apiSelectionIssue) || codexSelected && (connection.error || codexRuntimeReady && !modelReady) ? CircleAlert : PlugZap;
  const details = backend === null ? 'Choose an AI connection in Connections.'
    : backend === 'codex'
    ? ready ? `Connected to Codex. Model: ${codexModel}. Open Connections to change AI or voice settings.`
      : connection.checking ? 'Checking your saved Codex connection…'
        : codexRuntimeReady ? 'The saved Codex model is unavailable. Choose a model in Connections.' : 'Set up your Codex connection once'
    : backend === 'api' ? apiSelectionIssue ? 'The saved AI connection needs attention. Choose a provider in Connections.'
      : `${providerName || 'AI provider selected'}${apiSelection.modelId ? ` · ${apiSelection.modelId}` : ''}. Open Connections to change the model or voice.`
      : backend === 'claude-setup' ? 'Claude Agent selected. Open Connections to manage setup.'
        : `${handoffAgentName} handoff selected. Agent setup and model settings stay in that app.`;
  return <AppButton intent="custom" className={`connection-header ${ready ? 'ready' : ''}`} onClick={onOpen}
    aria-label={`${label}. ${details}`} title={details}>
    <Icon size={15} /><span className="connection-header-value">{label}</span>
  </AppButton>;
}
