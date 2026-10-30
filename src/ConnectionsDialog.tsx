import { useEffect, useRef, useState } from 'react';
import { AlertDialog, Tabs } from '@radix-ui/themes';
import { CheckCircle2, PlugZap, Volume2, X } from 'lucide-react';
import { CodexChapterDialog } from './CodexChapterDialog';
import { AppSelect } from './design/AppSelect';
import { AppButton } from './design/AppButton';
import { providerFailureMessage } from './lib/errorMessage';
import { cancelProviderRequest, getNarrationDisclosure, getPocketTtsStatus, getProviderCredentialStatus, removePocketTtsRuntime, removeProviderApiKey, setupPocketTts, storeProviderApiKey, type NarrationDisclosure, type PocketTtsStatus, type ProviderCredentialStatus } from './lib/commands';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import { readVoiceConnectionIssue, saveVoiceConnection, useVoiceConnection } from './lib/voiceConnectionStore';
import { invalidateLocalNarrationPrewarmIdentity } from './workspace/localNarrationQueue';

export function ConnectionsDialog({ native, initialSection = 'explanations', onClose, onChooseBackend }: { native: boolean; initialSection?: 'explanations' | 'voice'; onClose: () => void; onChooseBackend: () => void }) {
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
    <footer className="provider-dialog-footer"><span className="provider-small-note">Saved on this Mac</span><div className="provider-draft-actions"><AppButton intent="secondary" disabled={busy} onClick={onChooseBackend}>Change AI connection</AppButton><AppButton intent="primary" disabled={busy} onClick={onClose}>Done</AppButton></div></footer>
  </section></div>;
}

function VoiceSettings({ native, onBusyChange }: { native: boolean; onBusyChange: (busy: boolean) => void }) {
  const settings = useVoiceConnection();
  const [status, setStatus] = useState<PocketTtsStatus | null>(null);
  const [disclosure, setDisclosure] = useState<NarrationDisclosure | null>(null);
  const [credential, setCredential] = useState<ProviderCredentialStatus | null>(null);
  const [key, setKey] = useState('');
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
    void (async () => {
      if (settings.backend === 'pocket-tts') { const value = await getPocketTtsStatus(); if (active) setStatus(value); }
      else {
        const [info, stored] = await Promise.all([getNarrationDisclosure('gemini'), getProviderCredentialStatus('gemini')]);
        if (!active) return;
        setDisclosure(info);
        if (stored.status === 'ok') setCredential(stored.data); else setError(providerFailureMessage(stored.error));
      }
    })().catch(() => { if (active) setError('Could not read voice connection status. Close and reopen Connections to retry.'); });
    return () => { active = false; };
  }, [native, settings.backend]);
  const save = (update: Partial<typeof settings>) => {
    try { saveVoiceConnection({ ...settings, ...update }); setError(null); setMessage('Voice preference saved for all answers.'); }
    catch { setError('Could not save your voice preference on this Mac.'); }
  };
  const prepare = async () => {
    if (busy || status?.busy) return;
    const id = crypto.randomUUID(); setupId.current = id; setBusy(true); setError(null); setMessage(null);
    try {
      const result = await setupPocketTts(id);
      if (!alive.current) return;
      if (result.status === 'ok') { setStatus(result.data); setMessage('Voice is ready. Open any answer and choose Listen.'); }
      else setError(result.error.message);
    } catch { if (alive.current) setError('Local voice setup could not finish. Try again.'); }
    finally {
      // Setup and repair can change the runtime fingerprint used by native
      // cached audio. Do not let an older in-flight client key suppress new
      // Pocket work after this attempt.
      invalidateLocalNarrationPrewarmIdentity();
      setupId.current = null;
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
  const changeKey = async (remove = false) => {
    if (busy) return;
    setBusy(true); setError(null); setMessage(null);
    try {
      const result = remove ? await removeProviderApiKey('gemini') : await storeProviderApiKey('gemini', key.trim());
      if (!alive.current) return;
      if (result.status === 'ok') { setCredential(result.data); setKey(''); setMessage(remove ? 'Google key removed.' : 'Google key saved in macOS Keychain.'); }
      else setError(providerFailureMessage(result.error));
    } catch { if (alive.current) setError('Could not update the key in macOS Keychain.'); }
    finally { if (alive.current) setBusy(false); }
  };
  return <div className="provider-dialog-content connections-voice">
    <div><h3>Voice for your explanations</h3><p>Choose once, then press Listen on any answer.</p></div>
    <label className="walkthrough-narration-field">Voice engine<AppSelect ariaLabel="Voice engine" size="1" value={settings.backend} disabled={busy} options={[{value:'pocket-tts',label:'Pocket TTS — on this Mac, free'},{value:'gemini',label:'Google Gemini — cloud'}]} onValueChange={(backend)=>save({backend:backend==='gemini'?'gemini':'pocket-tts'})}/></label>
    {!native && <p role="status">Voice setup is available in the Code Atlas Mac app.</p>}
    {settings.backend === 'pocket-tts' ? <>
      <div className="connections-status"><CheckCircle2 size={17}/><div><strong>{busy ? 'Preparing voice…' : status?.ready ? 'Ready to listen' : 'Prepare local voice'}</strong><p>Alba · English · text stays on this Mac</p></div></div>
      <p>{status?.ready ? 'The local voice uses about 1.1 GB on this Mac.' : 'Setting up the local voice downloads about 1.1 GB to this Mac.'} It works offline once ready, with no provider usage fee.</p>
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
    </> : <>
      <p>Only narration text is sent to Google after you review and approve it. Free-tier limits, data use and possible charges depend on your Google project.</p>
      <label className="walkthrough-narration-field">Voice<AppSelect ariaLabel="Google narration voice" size="1" value={settings.googleVoice} disabled={!native || busy} options={(disclosure?.voices ?? ['Kore', 'Charon', 'Puck', 'Aoede']).map((voice)=>({value:voice,label:voice}))} onValueChange={(googleVoice)=>save({googleVoice})}/></label>
      <details><summary>Delivery style</summary><label className="walkthrough-narration-field">Style<input value={settings.googleStyle} disabled={busy} maxLength={160} onChange={(event) => save({ googleStyle: event.target.value })} placeholder="Optional, e.g. calm and conversational"/></label></details>
      <form onSubmit={(event) => { event.preventDefault(); void changeKey(); }}><label className="walkthrough-narration-field">{credential?.apiKeyConfigured ? 'Replace Google API key' : 'Google API key'}<input type="password" value={key} onChange={(event) => setKey(event.target.value)} autoComplete="off" spellCheck={false} disabled={!native || busy}/></label><div className="provider-key-actions"><AppButton intent="secondary" type="submit" disabled={!native || busy || !key.trim()}>Save key</AppButton>{credential?.apiKeyConfigured && <AppButton intent="quiet" type="button" disabled={busy} onClick={() => { void changeKey(true); }}>Remove key</AppButton>}</div></form>
      <p>{credential?.apiKeyConfigured ? 'Key saved in macOS Keychain.' : 'No Google key saved.'} {disclosure && <a href={disclosure.apiKeyUrl} target="_blank" rel="noreferrer">Get a Google key</a>}</p>
    </>}
    {message && <p role="status">{message}</p>}{error && <p className="provider-error-text" role="alert">{error}</p>}
  </div>;
}
