import { useEffect, useRef, useState } from 'react';
import { CheckCircle2, PlugZap, Volume2, X } from 'lucide-react';
import { CodexChapterDialog } from './CodexChapterDialog';
import { cancelProviderRequest, getNarrationDisclosure, getPocketTtsStatus, getProviderCredentialStatus, removeProviderApiKey, setupPocketTts, storeProviderApiKey, type NarrationDisclosure, type PocketTtsStatus, type ProviderCredentialStatus } from './lib/commands';
import { useModalFocusTrap } from './lib/useModalFocusTrap';
import { saveVoiceConnection, useVoiceConnection } from './lib/voiceConnectionStore';

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
    <header className="provider-dialog-header"><div><h2 id="connections-title">Connections</h2><p>Set up once. Your choices apply across repositories and answers.</p></div><button className="icon-button" aria-label="Close connections" disabled={busy} onClick={onClose}><X size={17}/></button></header>
    <nav className="connections-nav" aria-label="Connection categories">
      <button aria-pressed={section === 'explanations'} disabled={busy} onClick={() => setSection('explanations')}><PlugZap size={16}/>Explanations</button>
      <button aria-pressed={section === 'voice'} disabled={busy} onClick={() => setSection('voice')}><Volume2 size={16}/>Voice</button>
    </nav>
    {section === 'explanations' ? <CodexChapterDialog mode="settings" native={native} embeddedSettings onSettingsBusyChange={setBusy} onClose={onClose}/> : <VoiceSettings native={native} onBusyChange={setBusy}/>}
    <footer className="provider-dialog-footer"><span className="provider-small-note">Saved on this Mac</span><button className="primary-button" disabled={busy} onClick={onClose}>Done</button></footer>
  </section></div>;
}

function VoiceSettings({ native, onBusyChange }: { native: boolean; onBusyChange: (busy: boolean) => void }) {
  const settings = useVoiceConnection();
  const [status, setStatus] = useState<PocketTtsStatus | null>(null);
  const [disclosure, setDisclosure] = useState<NarrationDisclosure | null>(null);
  const [credential, setCredential] = useState<ProviderCredentialStatus | null>(null);
  const [key, setKey] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const setupId = useRef<string | null>(null);
  const alive = useRef(false);
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
        if (stored.status === 'ok') setCredential(stored.data); else setError(stored.error.message);
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
    finally { setupId.current = null; if (alive.current) setBusy(false); }
  };
  const changeKey = async (remove = false) => {
    if (busy) return;
    setBusy(true); setError(null); setMessage(null);
    try {
      const result = remove ? await removeProviderApiKey('gemini') : await storeProviderApiKey('gemini', key.trim());
      if (!alive.current) return;
      if (result.status === 'ok') { setCredential(result.data); setKey(''); setMessage(remove ? 'Google key removed.' : 'Google key saved in macOS Keychain.'); }
      else setError(result.error.message);
    } catch { if (alive.current) setError('Could not update the key in macOS Keychain.'); }
    finally { if (alive.current) setBusy(false); }
  };
  return <div className="provider-dialog-content connections-voice">
    <div><h3>Voice for your explanations</h3><p>Choose once, then press Listen on any answer.</p></div>
    <label className="walkthrough-narration-field">Voice engine<select value={settings.backend} disabled={busy} onChange={(event) => save({ backend: event.target.value === 'gemini' ? 'gemini' : 'pocket-tts' })}><option value="pocket-tts">Pocket TTS — on this Mac, free</option><option value="gemini">Google Gemini — cloud</option></select></label>
    {!native && <p role="status">Voice setup is available in the Code Atlas Mac app.</p>}
    {settings.backend === 'pocket-tts' ? <>
      <div className="connections-status"><CheckCircle2 size={17}/><div><strong>{busy ? 'Preparing voice…' : status?.ready ? 'Ready to listen' : 'Prepare local voice'}</strong><p>Alba · English · text stays on this Mac</p></div></div>
      {!status?.ready && <p>One download prepares the voice. Listening has no provider usage fee.</p>}
      {(!status?.ready || busy) && <button className="primary-button" disabled={!native || busy || status?.busy || !status?.setupAvailable} onClick={() => { void prepare(); }}>{busy ? 'Preparing…' : 'Prepare voice'}</button>}
      {busy && <button className="secondary-button" onClick={() => { if (setupId.current) void cancelProviderRequest(setupId.current).catch(() => setError('Could not stop setup.')); }}>Cancel setup</button>}
      <details><summary>Voice details and troubleshooting</summary><p>{status?.message ?? 'Checking availability…'}</p><p>Alba MacKenna voice · CC BY 4.0.</p><a href="https://huggingface.co/kyutai/tts-voices" target="_blank" rel="noreferrer">Voice source and license</a>{status?.ready && <button className="secondary-button" disabled={busy || status.busy || !status.setupAvailable} onClick={() => { void prepare(); }}>Repair voice setup</button>}</details>
    </> : <>
      <p>Only narration text is sent to Google after you review and approve it. Free-tier limits, data use and possible charges depend on your Google project.</p>
      <label className="walkthrough-narration-field">Voice<select value={settings.googleVoice} disabled={!native || busy} onChange={(event) => save({ googleVoice: event.target.value })}>{(disclosure?.voices ?? ['Kore', 'Charon', 'Puck', 'Aoede']).map((voice) => <option key={voice}>{voice}</option>)}</select></label>
      <details><summary>Delivery style</summary><label className="walkthrough-narration-field">Style<input value={settings.googleStyle} disabled={busy} maxLength={160} onChange={(event) => save({ googleStyle: event.target.value })} placeholder="Optional, e.g. calm and conversational"/></label></details>
      <form onSubmit={(event) => { event.preventDefault(); void changeKey(); }}><label className="walkthrough-narration-field">{credential?.apiKeyConfigured ? 'Replace Google API key' : 'Google API key'}<input type="password" value={key} onChange={(event) => setKey(event.target.value)} autoComplete="off" spellCheck={false} disabled={!native || busy}/></label><div className="provider-key-actions"><button className="secondary-button" type="submit" disabled={!native || busy || !key.trim()}>Save key</button>{credential?.apiKeyConfigured && <button className="text-button" type="button" disabled={busy} onClick={() => { void changeKey(true); }}>Remove key</button>}</div></form>
      <p>{credential?.apiKeyConfigured ? 'Key saved in macOS Keychain.' : 'No Google key saved.'} {disclosure && <a href={disclosure.apiKeyUrl} target="_blank" rel="noreferrer">Get a Google key</a>}</p>
    </>}
    {message && <p role="status">{message}</p>}{error && <p className="provider-error-text" role="alert">{error}</p>}
  </div>;
}
