import { useEffect, useRef, useState } from 'react';
import { AlertDialog, Tabs } from '@radix-ui/themes';
import { CheckCircle2, PlugZap, Volume2, X } from 'lucide-react';
import { CodexChapterDialog } from './CodexChapterDialog';
import { AppButton } from './design/AppButton';
import { cancelProviderRequest, getPocketTtsStatus, removePocketTtsRuntime, setupPocketTts, type PocketTtsStatus } from './lib/commands';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import { readVoiceConnectionIssue, useVoiceConnection } from './lib/voiceConnectionStore';
import { invalidateLocalNarrationPrewarmIdentity } from './workspace/localNarrationQueue';
import { notifyPocketTtsRuntimeReady } from './workspace/pocketRuntimeEvents';

export function ConnectionsDialog({ native, initialSection = 'explanations', onClose }: { native: boolean; initialSection?: 'explanations' | 'voice'; onClose: () => void }) {
  const [section, setSection] = useState(initialSection);
  const [busy, setBusy] = useState(false);
  const dialogRef = useRef<HTMLElement>(null);
  const closeRef = useRef(onClose); closeRef.current = onClose;
  useModalFocusTrap(dialogRef);
  useEffect(() => { dialogRef.current?.focus(); return () => { window.dispatchEvent(new Event('code-atlas:connections-changed')); }; }, []);
  useEffect(() => {
    const key = (event: KeyboardEvent) => {
      if (event.key === 'Escape') { event.preventDefault(); event.stopPropagation(); if (!busy) closeRef.current(); }
    };
    document.addEventListener('keydown', key, true);
    return () => document.removeEventListener('keydown', key, true);
  }, [busy]);
  return <div className="provider-overlay"><section className="provider-dialog connections-dialog" ref={dialogRef} tabIndex={-1} role="dialog" aria-modal="true" aria-labelledby="connections-title">
    <header className="provider-dialog-header"><div><h2 id="connections-title">Connections</h2><p>Set up once. Your choices apply across repositories and answers.</p></div><AppButton intent="icon" aria-label="Close connections" disabled={busy} onClick={onClose}><X size={17}/></AppButton></header>
    <Tabs.Root value={section} onValueChange={(value) => setSection(value as 'explanations' | 'voice')}>
      <Tabs.List className="connections-nav" aria-label="Connection categories">
        <Tabs.Trigger value="explanations" disabled={busy}><PlugZap size={16}/>Explanations</Tabs.Trigger>
        <Tabs.Trigger value="voice" disabled={busy}><Volume2 size={16}/>Voice</Tabs.Trigger>
      </Tabs.List>
      <Tabs.Content value="explanations"><CodexChapterDialog mode="settings" native={native} embeddedSettings onSettingsBusyChange={setBusy} onClose={onClose}/></Tabs.Content>
      <Tabs.Content value="voice"><VoiceSettings native={native} onBusyChange={setBusy}/></Tabs.Content>
    </Tabs.Root>
    <footer className="provider-dialog-footer"><span className="provider-small-note">Saved on this Mac</span><div className="provider-draft-actions"><AppButton intent="primary" disabled={busy} onClick={onClose}>Done</AppButton></div></footer>
  </section></div>;
}

function VoiceSettings({ native, onBusyChange }: { native: boolean; onBusyChange: (busy: boolean) => void }) {
  const settings = useVoiceConnection();
  const [status, setStatus] = useState<PocketTtsStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [removeOpen, setRemoveOpen] = useState(false);
  const [error, setError] = useState<string | null>(() => readVoiceConnectionIssue());
  const [message, setMessage] = useState<string | null>(null);
  const setupId = useRef<string | null>(null);
  const alive = useRef(false);
  useEffect(() => { setError(readVoiceConnectionIssue()); }, [settings.backend, settings.googleVoice, settings.googleStyle]);
  useEffect(() => { alive.current = true; return () => { alive.current = false; if (setupId.current) void cancelProviderRequest(setupId.current).catch(() => {}); }; }, []);
  useEffect(() => { onBusyChange(busy); }, [busy, onBusyChange]);
  useEffect(() => {
    if (!native) return;
    let active = true;
    void getPocketTtsStatus().then(value => { if (active) setStatus(value); })
      .catch(() => { if (active) setError('Could not read local voice status. Close and reopen Connections to retry.'); });
    return () => { active = false; };
  }, [native]);
  const prepare = async () => {
    if (busy || status?.busy) return;
    const id = crypto.randomUUID(); setupId.current = id; setBusy(true); setError(null); setMessage(null);
    let runtimeReady = false;
    try {
      const result = await setupPocketTts(id);
      if (!alive.current) return;
      if (result.status === 'ok') {
        setStatus(result.data);
        runtimeReady = result.data.ready;
        setMessage(runtimeReady ? 'Voice is ready. Open any answer and choose Listen.' : 'Local voice setup is not ready yet. Check the status below.');
      }
      else setError(result.error.message);
    } catch { if (alive.current) setError('Local voice setup could not finish. Try again.'); }
    finally {
      // Setup and repair can change the runtime fingerprint used by native
      // cached audio. Do not let an older in-flight client key suppress new
      // Pocket work after this attempt.
      invalidateLocalNarrationPrewarmIdentity();
      setupId.current = null;
      if (runtimeReady && typeof window !== 'undefined') notifyPocketTtsRuntimeReady(window);
      if (alive.current) setBusy(false);
    }
  };
  const removeRuntime = async () => {
    if (busy || status?.busy) return;
    setBusy(true); setError(null); setMessage(null);
    try {
      const result = await removePocketTtsRuntime();
      if (!alive.current) return;
      if (result.status === 'ok') {
        setStatus(result.data);
        invalidateLocalNarrationPrewarmIdentity();
        setMessage('Local voice removed. Saved narration audio is still available; set up the voice again to create new audio.');
      } else setError(result.error.message);
    } catch { if (alive.current) setError('Local voice files could not be removed. Try again.'); }
    finally { if (alive.current) setBusy(false); }
  };
  return <div className="provider-dialog-content connections-voice">
    <div><h3>Voice for your explanations</h3><p>Local Pocket TTS is ready when you choose Listen on an answer.</p></div>
    {!native && <p role="status">Voice setup is available in the Code Atlas Mac app.</p>}
    <div className="connections-status"><CheckCircle2 size={17}/><div><strong>{busy ? 'Preparing voice…' : status?.ready ? 'Ready to listen' : 'Prepare local voice'}</strong><p>Alba · English · narration text stays on this Mac</p></div></div>
    <p>The prepared local voice uses about 1.1 GB on this Mac. It works offline once ready, with no provider usage fee.</p>
    {(!status?.ready || busy) && <AppButton intent="primary" disabled={!native || busy || status?.busy || !status?.setupAvailable} onClick={() => { void prepare(); }}>{busy ? 'Preparing…' : 'Prepare voice'}</AppButton>}
    {busy && <AppButton intent="secondary" onClick={() => { if (setupId.current) void cancelProviderRequest(setupId.current).catch(() => setError('Could not stop setup.')); }}>Cancel setup</AppButton>}
    <details><summary>Voice details and troubleshooting</summary><p>{status?.message ?? 'Checking availability…'}</p><p>Alba MacKenna voice · CC BY 4.0.</p><a href="https://huggingface.co/kyutai/tts-voices" target="_blank" rel="noreferrer">Voice source and license</a>{status?.ready && <>
      <AppButton intent="secondary" disabled={busy || status.busy || !status.setupAvailable} onClick={() => { void prepare(); }}>Repair voice setup</AppButton>
      <AlertDialog.Root open={removeOpen} onOpenChange={setRemoveOpen}>
        <AlertDialog.Trigger><AppButton intent="quiet" disabled={!native || busy || status.busy}>Remove local voice…</AppButton></AlertDialog.Trigger>
        <AlertDialog.Content maxWidth="420px">
          <AlertDialog.Title>Remove local voice?</AlertDialog.Title>
          <AlertDialog.Description>
            This removes the Pocket TTS setup files (about 1.1 GB) from this Mac. Saved narration audio stays available. Setting up the voice again will download these files again.
          </AlertDialog.Description>
          <div className="provider-draft-actions">
            <AlertDialog.Cancel><AppButton intent="secondary">Keep voice</AppButton></AlertDialog.Cancel>
            <AlertDialog.Action><AppButton intent="danger" disabled={busy} onClick={() => { void removeRuntime(); }}>Remove voice</AppButton></AlertDialog.Action>
          </div>
        </AlertDialog.Content>
      </AlertDialog.Root>
    </>}</details>
    {message && <p role="status">{message}</p>}{error && <p className="provider-error-text" role="alert">{error}</p>}
  </div>;
}
