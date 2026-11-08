import { useEffect, useState } from 'react';
import { CheckCircle2, CircleAlert, LoaderCircle, PlugZap } from 'lucide-react';
import { AppButton } from './design/AppButton';
import { initializeConnection, useConnectionState } from './lib/connectionStore';
import { codexSelectionIsSupported } from './lib/codexConnection';
import { readGenerationBackend, type GenerationBackend } from './lib/generationPreferences';

export function ConnectionHeader({ onOpen }: { onOpen: () => void }) {
  const connection = useConnectionState();
  const [backend, setBackend] = useState<GenerationBackend | null>(() => readGenerationBackend());
  useEffect(() => {
    if (backend === 'codex') initializeConnection();
    const refreshBackend = () => setBackend(readGenerationBackend());
    window.addEventListener('code-atlas:connection-changed', refreshBackend);
    return () => {
      window.removeEventListener('code-atlas:connection-changed', refreshBackend);
    };
  }, [backend]);
  const codexSelected = backend === 'codex';
  const codexNeedsAttention = codexSelected && Boolean(connection.error);
  const codexRuntimeReady = codexSelected && connection.status?.authenticated && connection.status.runtimeRunning;
  const selectedModel = connection.models.find((model) => model.modelId === connection.selection.modelId);
  const codexModel = selectedModel?.displayName ?? connection.selection.modelId;
  const modelReady = codexSelectionIsSupported(connection.models, connection.selection);
  const ready = Boolean(codexRuntimeReady && modelReady);
  const label = backend === null ? 'Connect Codex'
    : connection.checking ? 'Connecting to Codex…'
      : codexNeedsAttention ? 'Retry Codex'
        : ready ? `Codex · ${codexModel}`
          : codexRuntimeReady ? 'Choose model' : 'Connect Codex';
  const Icon = codexSelected && connection.checking ? LoaderCircle : codexNeedsAttention ? CircleAlert : ready ? CheckCircle2
    : backend === null || codexSelected && (connection.error || codexRuntimeReady && !modelReady) ? CircleAlert : PlugZap;
  const details = backend === null ? 'Connect your Codex account once in Connections.'
    : backend === 'codex'
    ? codexNeedsAttention ? 'Could not refresh Codex right now. Open Connections to retry; your saved account and model choice are unchanged.'
      : ready ? `Connected to Codex. Model: ${codexModel}. Open Connections to change AI or voice settings.`
      : connection.checking ? 'Checking your saved Codex connection…'
        : codexRuntimeReady ? 'The saved Codex model is unavailable. Choose a model in Connections.' : 'Set up your Codex connection once'
    : 'Connect your Codex account once in Connections.';
  return <AppButton intent="custom" className={`connection-header ${ready ? 'ready' : ''}`} onClick={onOpen}
    aria-label={`${label}. ${details}`} title={details}>
    <Icon size={15} /><span className="connection-header-value">{label}</span>
  </AppButton>;
}
