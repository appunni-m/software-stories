import { useEffect, useState } from 'react';
import { CheckCircle2, CircleAlert, LoaderCircle, PlugZap } from 'lucide-react';
import { initializeConnection, useConnectionState } from './lib/connectionStore';
import { readDirectApiSelection, readGenerationBackend, type GenerationBackend, type DirectApiSelection } from './lib/generationPreferences';

export function ConnectionHeader({ onOpen }: { onOpen: () => void }) {
  const connection = useConnectionState();
  const [backend, setBackend] = useState<GenerationBackend>(() => readGenerationBackend());
  const [apiSelection, setApiSelection] = useState<DirectApiSelection>(() => readDirectApiSelection());
  useEffect(() => {
    initializeConnection();
    const refreshBackend = () => setBackend(readGenerationBackend());
    const refreshApiSelection = () => setApiSelection(readDirectApiSelection());
    window.addEventListener('code-atlas:connection-changed', refreshBackend);
    window.addEventListener('code-atlas:direct-api-selection-changed', refreshApiSelection);
    return () => {
      window.removeEventListener('code-atlas:connection-changed', refreshBackend);
      window.removeEventListener('code-atlas:direct-api-selection-changed', refreshApiSelection);
    };
  }, []);
  const codexSelected = backend === 'codex';
  const ready = codexSelected && connection.status?.authenticated && connection.status.runtimeRunning;
  const providerName = apiSelection.providerId === 'gemini' ? 'Gemini'
    : apiSelection.providerId === 'openai' ? 'OpenAI'
      : apiSelection.providerId === 'anthropic' ? 'Anthropic' : apiSelection.providerId;
  const selectedModel = connection.models.find((model) => model.modelId === connection.selection.modelId);
  const codexModel = selectedModel?.displayName ?? connection.selection.modelId;
  const label = backend === 'api'
    ? `API · ${providerName || 'Choose provider'}${apiSelection.modelId ? ` · ${apiSelection.modelId}` : ''}`
    : backend === 'claude-setup' ? 'Claude Agent'
      : backend === 'handoff' ? 'Manual agent'
        : connection.checking ? 'Connecting to Codex…'
          : ready ? `Codex · ${codexModel} · ${connection.selection.effort}` : 'Connect Codex';
  const Icon = codexSelected && connection.checking ? LoaderCircle : ready ? CheckCircle2
    : codexSelected && connection.error ? CircleAlert : PlugZap;
  const details = backend === 'codex'
    ? ready ? `Connected${connection.status?.accountEmail ? ` as ${connection.status.accountEmail}` : ''}. Model ${codexModel}, reasoning ${connection.selection.effort}.` : 'Set up your Codex connection once'
    : backend === 'api' ? `${providerName ? `${providerName}${apiSelection.modelId ? ` · ${apiSelection.modelId}` : ''}` : 'Direct API selected without a provider'}. Open Connections to change provider or model.`
      : backend === 'claude-setup' ? 'Claude Agent selected. Open Connections to manage setup.'
        : 'Manual agent handoff selected. Open Connections to manage it.';
  return <button className={`connection-header ${ready ? 'ready' : ''}`} onClick={onOpen}
    aria-label={`${label}. ${details} Open Connections.`} title={`${details} Voice settings are also in Connections.`}>
    <Icon size={15} /><span className="connection-header-value">{label}</span>
  </button>;
}
