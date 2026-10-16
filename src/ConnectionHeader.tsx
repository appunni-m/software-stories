import { useEffect } from 'react';
import { CheckCircle2, CircleAlert, LoaderCircle, PlugZap } from 'lucide-react';
import { initializeConnection, useConnectionState } from './lib/connectionStore';

export function ConnectionHeader({ onOpen }: { onOpen: () => void }) {
  const connection = useConnectionState();
  useEffect(() => { initializeConnection(); }, []);
  const ready = connection.status?.authenticated && connection.status.runtimeRunning;
  const label = connection.checking ? 'Connecting…' : ready ? 'Codex' : 'Connect Codex';
  const Icon = connection.checking ? LoaderCircle : ready ? CheckCircle2 : connection.error ? CircleAlert : PlugZap;
  return <button className={`connection-header ${ready ? 'ready' : ''}`} onClick={onOpen}
    aria-label={`${label}. Open connection settings`} title={ready ? `Connected${connection.status?.accountEmail ? ` as ${connection.status.accountEmail}` : ''}. Change connection settings.` : 'Set up your generation connection once'}>
    <Icon size={15} /><span>{label}</span>
  </button>;
}
