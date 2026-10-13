import { useEffect, useMemo, useRef, useState } from 'react';
import {
  AlertCircle,
  Check,
  ChevronDown,
  Pause,
  Play,
  RotateCcw,
  Square,
  Volume2,
  X,
} from 'lucide-react';
import {
  buildNarrationRequestReview,
  cancelGeminiRequest,
  discardNarrationRequestReview,
  getNarrationDisclosure,
  getProviderCredentialStatus,
  isNativeRuntime,
  loadReviewedNarrationAudio,
  synthesizeReviewedNarrationAudio,
  type NarrationAudio,
  type NarrationDisclosure,
  type NarrationPronunciationEntry,
  type NarrationRequestReview,
  type ProviderCredentialStatus,
} from './lib/commands';
import { narrationPreferencesMatch } from './lib/narration';
import type { ChapterSection } from './lib/commands';

interface NarrationPlayerProps {
  projectId: string;
  chapterId: string;
  revisionId: string;
  section: ChapterSection;
  restorePosition: number;
  transcriptPosition: number;
  onPlaybackPositionChange: (position: number) => void;
  onTranscriptPositionChange: (position: number) => void;
}

interface NarrationPreferences {
  spokenText: string;
  voice: string;
  style: string;
  lexicon: NarrationPronunciationEntry[];
}

interface LoadedTrack {
  audioUrl: string;
  exactText: string;
  sourceText: string;
  voice: string;
  style: string;
  lexicon: NarrationPronunciationEntry[];
  fingerprint: string;
  cacheHit: boolean;
}

const DEFAULT_STYLE = 'Clear, calm, conversational.';

export function NarrationPlayer({ projectId, chapterId, revisionId, section, restorePosition, transcriptPosition, onPlaybackPositionChange, onTranscriptPositionChange }: NarrationPlayerProps) {
  const storageKey = `code-atlas:narration:${projectId}:${chapterId}:${revisionId}:${section.sectionId}`;
  const preferences = useMemo(() => readPreferences(storageKey, section.spokenText), [storageKey, section.spokenText]);
  const [draft, setDraft] = useState<NarrationPreferences>(preferences);
  const draftRef = useRef(draft);
  draftRef.current = draft;
  const [disclosure, setDisclosure] = useState<NarrationDisclosure | null>(null);
  const [credential, setCredential] = useState<ProviderCredentialStatus | null>(null);
  const [informationError, setInformationError] = useState<string | null>(null);
  const [review, setReview] = useState<NarrationRequestReview | null>(null);
  const [reviewBusy, setReviewBusy] = useState(false);
  const [requestBusy, setRequestBusy] = useState(false);
  const [providerRequestBusy, setProviderRequestBusy] = useState(false);
  const [cancelConfirmation, setCancelConfirmation] = useState(false);
  const [cancelBusy, setCancelBusy] = useState(false);
  const [cancelStatus, setCancelStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [consent, setConsent] = useState(false);
  const [track, setTrack] = useState<LoadedTrack | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
  const [showReview, setShowReview] = useState(false);
  const audioRef = useRef<HTMLAudioElement>(null);
  const transcriptRef = useRef<HTMLDivElement>(null);
  const autoPlayRef = useRef(false);
  const cancelRequested = useRef(false);
  const restoreTimeRef = useRef<number | null>(restorePosition > 0 ? restorePosition : null);
  const lastRoutePositionRef = useRef(restorePosition);
  const lastRouteWriteRef = useRef(0);

  useEffect(() => {
    setDraft(readPreferences(storageKey, section.spokenText));
    setReview(null);
    setTrack(null);
    setCurrentTime(0);
    setDuration(0);
  }, [storageKey, section.spokenText]);

  useEffect(() => {
    if (!transcriptRef.current) return;
    const requestedPosition = Math.max(0, transcriptPosition);
    if (Math.abs(transcriptRef.current.scrollTop - requestedPosition) > 1) {
      transcriptRef.current.scrollTop = requestedPosition;
    }
  }, [storageKey, transcriptPosition]);

  useEffect(() => {
    if (!isNativeRuntime()) return;
    let active = true;
    void Promise.all([getNarrationDisclosure(), getProviderCredentialStatus()])
      .then(([nextDisclosure, nextCredential]) => {
        if (!active) return;
        setDisclosure(nextDisclosure);
        if (nextCredential.status === 'ok') setCredential(nextCredential.data);
        else setInformationError(nextCredential.error.message);
      })
      .catch(() => {
        if (active) setInformationError('Narration setup could not be loaded. The transcript remains available and no provider request was sent.');
      });
    return () => { active = false; };
  }, []);

  useEffect(() => {
    const player = audioRef.current;
    if (!player) return;
    if (track?.audioUrl) {
      player.pause();
      player.src = track.audioUrl;
      player.load();
    } else {
      player.pause();
      player.removeAttribute('src');
      player.load();
    }
    return () => {
      player.pause();
      player.removeAttribute('src');
      player.load();
    };
  }, [track?.audioUrl]);

  const reviewRef = useRef<NarrationRequestReview | null>(review);
  reviewRef.current = review;
  useEffect(() => () => {
    audioRef.current?.pause();
    if (reviewRef.current?.reviewId) void discardNarrationRequestReview(reviewRef.current.reviewId).catch(() => {});
  }, []);

  const savedAudioMatchesDraft = Boolean(track
    && narrationPreferencesMatch({
      spokenText: track.sourceText,
      voice: track.voice,
      style: track.style,
      lexicon: track.lexicon,
    }, draft));

  const updateDraft = (update: (value: NarrationPreferences) => NarrationPreferences) => {
    setDraft((value) => {
      const next = update(value);
      try { window.localStorage.setItem(storageKey, JSON.stringify(next)); } catch { /* Speech editing still works without storage. */ }
      return next;
    });
    if (review) void discardNarrationRequestReview(review.reviewId).catch(() => {});
    setReview(null);
    setConsent(false);
    setShowReview(false);
    setError(null);
  };

  const makeReview = async (restoreCache = false): Promise<NarrationRequestReview | null> => {
    if (!isNativeRuntime()) {
      setError('Narration generation needs the Code Atlas desktop app. The transcript is still available here.');
      return null;
    }
    setReviewBusy(true);
    setError(null);
    const requestedDraft = draft;
    try {
      const result = await buildNarrationRequestReview({
        spokenText: requestedDraft.spokenText,
        voice: requestedDraft.voice,
        style: requestedDraft.style,
        lexicon: requestedDraft.lexicon,
      });
      if (result.status === 'error') {
        setError(result.error.message);
        return null;
      }
      if (!narrationPreferencesMatch(draftRef.current, requestedDraft)) {
        void discardNarrationRequestReview(result.data.reviewId).catch(() => {});
        setError('The script or narration settings changed during review. Review the current text before sending.');
        return null;
      }
      if (review && review.reviewId !== result.data.reviewId) void discardNarrationRequestReview(review.reviewId).catch(() => {});
      setReview(result.data);
      setConsent(false);
      setShowReview(true);
      if (restoreCache && result.data.cachedAudioAvailable) {
        const cached = await loadReviewedNarrationAudio({ reviewId: result.data.reviewId, fingerprint: result.data.fingerprint });
        if (cached.status === 'ok') {
          if (!narrationPreferencesMatch(draftRef.current, requestedDraft)) {
            void discardNarrationRequestReview(result.data.reviewId).catch(() => {});
            return null;
          }
          setLoadedTrack(cached.data, result.data, false, requestedDraft);
        } else setError(cached.error.message);
      }
      return result.data;
    } catch {
      setError('The local narration review could not be built. No model request was sent.');
      return null;
    } finally {
      setReviewBusy(false);
    }
  };

  useEffect(() => {
    if (restoreTimeRef.current === null || !isNativeRuntime()) return;
    const restoredPosition = restoreTimeRef.current;
    restoreTimeRef.current = null;
    let active = true;
    void makeReview(true).then((restoredReview) => {
      if (!active) return;
      if (!restoredReview?.cachedAudioAvailable) restoreTimeRef.current = null;
      // loadedmetadata restores a paused position; playback remains manual.
      else restoreTimeRef.current = restoredPosition;
    });
    return () => { active = false; };
  }, []);

  const setLoadedTrack = (audio: NarrationAudio, currentReview: NarrationRequestReview, playAfterLoad: boolean, sourcePreferences: NarrationPreferences = draft) => {
    const dataUrl = `data:${audio.mimeType};base64,${audio.audioBase64}`;
    setTrack({
      audioUrl: dataUrl,
      exactText: currentReview.exactText,
      sourceText: sourcePreferences.spokenText,
      voice: sourcePreferences.voice,
      style: sourcePreferences.style,
      lexicon: sourcePreferences.lexicon.map((entry) => ({ ...entry })),
      fingerprint: audio.fingerprint,
      cacheHit: audio.cacheHit,
    });
    autoPlayRef.current = playAfterLoad;
    setCurrentTime(0);
    setDuration(0);
    setError(null);
  };

  const loadCache = async (currentReview: NarrationRequestReview, playAfterLoad: boolean) => {
    if (!isNativeRuntime()) return;
    const requestedDraft = draftRef.current;
    setRequestBusy(true);
    setError(null);
    try {
      const result = await loadReviewedNarrationAudio({ reviewId: currentReview.reviewId, fingerprint: currentReview.fingerprint });
      if (result.status === 'error') {
        setError(result.error.message);
        return;
      }
      if (!narrationPreferencesMatch(draftRef.current, requestedDraft)) return;
      setLoadedTrack(result.data, currentReview, playAfterLoad, requestedDraft);
    } catch {
      setError('The local audio cache could not be read. The transcript remains available.');
    } finally {
      setRequestBusy(false);
    }
  };

  const synthesize = async () => {
    if (!review) return;
    if (!credential?.apiKeyConfigured) {
      setError('Save a Gemini API key in Tools → AI and narration setup before requesting audio.');
      return;
    }
    setRequestBusy(true);
    setProviderRequestBusy(true);
    cancelRequested.current = false;
    setCancelConfirmation(false);
    setCancelBusy(false);
    setCancelStatus(null);
    setError(null);
    audioRef.current?.pause();
    try {
      const result = await synthesizeReviewedNarrationAudio({
        reviewId: review.reviewId,
        fingerprint: review.fingerprint,
        consent: { termsAndRequestAcknowledged: consent },
      });
      if (result.status === 'error') {
        setError(result.error.message);
        // The native command consumes its reviewed request before checking the
        // provider response. Require a fresh local review before any retry.
        setReview(null);
        setConsent(false);
        setShowReview(false);
        return;
      }
      if (cancelRequested.current) {
        setError('Cancellation was requested after Google may have received the reviewed narration text. The returned audio was discarded; the transcript and previous audio remain available.');
        setReview(null);
        setConsent(false);
        setShowReview(false);
        return;
      }
      if (!narrationPreferencesMatch(draftRef.current, draft)) {
        setError('The narration settings changed while audio was being generated. The returned audio was discarded; review the current text before generating again.');
        setReview(null);
        setConsent(false);
        setShowReview(false);
        return;
      }
    setLoadedTrack(result.data, review, true, draft);
      setConsent(false);
      setReview(null);
      setShowReview(false);
    } catch {
      setError('Google TTS could not be reached. The transcript and previous audio remain available; review again before retrying.');
      setReview(null);
      setConsent(false);
      setShowReview(false);
    } finally {
      setRequestBusy(false);
      setProviderRequestBusy(false);
      setCancelConfirmation(false);
      setCancelBusy(false);
    }
  };

  const confirmCancelSynthesis = async () => {
    if (!review || !providerRequestBusy || cancelBusy) return;
    cancelRequested.current = true;
    setCancelBusy(true);
    setCancelConfirmation(false);
    setCancelStatus(null);
    try {
      const stopped = await cancelGeminiRequest(review.reviewId);
      setCancelStatus(stopped
        ? 'The local request was stopped. Google may already have received the reviewed narration text; no returned audio will be accepted.'
        : 'The request had already finished locally. Its audio will be discarded because you chose Cancel.');
    } catch {
      setCancelStatus('The app could not confirm a local abort. Google may already have received the reviewed narration text; any returned audio will be discarded.');
    } finally {
      setCancelBusy(false);
    }
  };

  const playOrPause = async () => {
    const player = audioRef.current;
    if (isPlaying) {
      player?.pause();
      return;
    }
    if (track && player?.src) {
      try { await player.play(); } catch { setError('Audio playback could not start on this device. The transcript remains available.'); }
      return;
    }
    const currentReview = review ?? await makeReview();
    if (currentReview?.cachedAudioAvailable) await loadCache(currentReview, true);
    else if (currentReview) setError('Review the exact text and provider terms below, then confirm before generating audio.');
  };

  const stop = () => {
    const player = audioRef.current;
    if (player) {
      player.pause();
      player.currentTime = 0;
    }
    setCurrentTime(0);
    persistPlaybackPosition(0);
  };

  const replay = async () => {
    const player = audioRef.current;
    if (!track) {
      await playOrPause();
      return;
    }
    if (!player) return;
    player.currentTime = 0;
    setCurrentTime(0);
    persistPlaybackPosition(0);
    try { await player.play(); } catch { setError('Audio playback could not start on this device. The transcript remains available.'); }
  };

  const seek = (value: number) => {
    const player = audioRef.current;
    if (!player || !Number.isFinite(value)) return;
    const next = Math.max(0, Math.min(value, Number.isFinite(player.duration) ? player.duration : value));
    player.currentTime = next;
    setCurrentTime(next);
    persistPlaybackPosition(next);
  };

  const persistPlaybackPosition = (position: number) => {
    lastRoutePositionRef.current = position;
    onPlaybackPositionChange(position);
  };

  const handleTimeUpdate = () => {
    const player = audioRef.current;
    if (!player) return;
    setCurrentTime(player.currentTime);
    const now = performance.now();
    if (now - lastRouteWriteRef.current > 700) {
      lastRouteWriteRef.current = now;
      persistPlaybackPosition(player.currentTime);
    }
  };

  useEffect(() => {
    const position = Math.max(0, restorePosition);
    if (Math.abs(position - lastRoutePositionRef.current) <= 0.05) return;
    lastRoutePositionRef.current = position;
    const player = audioRef.current;
    player?.pause();
    if (track && player && Number.isFinite(player.duration) && player.duration > 0) {
      const bounded = Math.max(0, Math.min(position, player.duration));
      player.currentTime = bounded;
      setCurrentTime(bounded);
      restoreTimeRef.current = null;
    } else {
      restoreTimeRef.current = position > 0 ? position : null;
      setCurrentTime(position);
    }
  }, [restorePosition, track?.audioUrl]);

  const captionText = track?.exactText ?? draft.spokenText;

  return (
    <section className="walkthrough-transcript walkthrough-narration" aria-labelledby="walkthrough-transcript-title">
      <audio
        ref={audioRef}
        className="walkthrough-narration-audio"
        preload="metadata"
        onPlay={() => setIsPlaying(true)}
        onPause={() => setIsPlaying(false)}
        onDurationChange={(event) => setDuration(Number.isFinite(event.currentTarget.duration) ? event.currentTarget.duration : 0)}
        onLoadedMetadata={(event) => {
          setDuration(Number.isFinite(event.currentTarget.duration) ? event.currentTarget.duration : 0);
          if (restoreTimeRef.current !== null) {
            const position = Math.max(0, Math.min(restoreTimeRef.current, event.currentTarget.duration || 0));
            event.currentTarget.currentTime = position;
            setCurrentTime(position);
            restoreTimeRef.current = null;
          }
        }}
        onCanPlay={(event) => {
          if (!autoPlayRef.current) return;
          autoPlayRef.current = false;
          void event.currentTarget.play().catch(() => setError('Audio playback could not start on this device. The transcript remains available.'));
        }}
        onTimeUpdate={handleTimeUpdate}
        onEnded={() => {
          setIsPlaying(false);
          persistPlaybackPosition(audioRef.current?.currentTime ?? 0);
        }}
      />
      <div className="walkthrough-panel-heading">
        <div><span className="walkthrough-kicker">CAPTIONS & TRANSCRIPT</span><h3 id="walkthrough-transcript-title">Listen to this section</h3></div>
        <span className={`walkthrough-transcript-state${isPlaying ? ' is-playing' : ''}`}><Volume2 size={13} /> {isPlaying ? 'PLAYING' : track ? 'PAUSED' : 'AUDIO OFF'}</span>
      </div>

      <div className="walkthrough-transcript-scroll" ref={transcriptRef} onScroll={(event) => onTranscriptPositionChange(event.currentTarget.scrollTop)} tabIndex={0} aria-label={`Transcript for ${section.title}`}>
        <div className="walkthrough-caption" aria-live="polite" aria-atomic="true">
          <span>{isPlaying ? 'CAPTION · CURRENT SECTION' : 'TRANSCRIPT · CURRENT SECTION'}</span>
          <p>{captionText.trim() ? captionText : <em>No narration text is saved for this section. The written explanation and source evidence remain available.</em>}</p>
        </div>
      </div>

      <details className="walkthrough-narration-editor" open>
        <summary><span>Edit spoken text</span><ChevronDown size={13} /></summary>
        <p>Only this section's script is sent for narration. Source code, file paths, and citations are left out.</p>
        <label className="walkthrough-narration-field">Spoken text
          <textarea value={draft.spokenText} maxLength={4000} rows={4} onChange={(event) => updateDraft((value) => ({ ...value, spokenText: event.target.value }))} placeholder="Write what you want the reader to say in natural language." />
        </label>
        <div className="walkthrough-narration-settings">
          <label className="walkthrough-narration-field">Voice
            <select value={draft.voice} disabled={!disclosure} onChange={(event) => updateDraft((value) => ({ ...value, voice: event.target.value }))}>
              {(disclosure?.voices ?? ['Kore', 'Charon', 'Puck', 'Aoede']).map((voice) => <option value={voice} key={voice}>{voice}</option>)}
            </select>
          </label>
          <label className="walkthrough-narration-field">Delivery style
            <input value={draft.style} maxLength={160} onChange={(event) => updateDraft((value) => ({ ...value, style: event.target.value }))} placeholder="Optional, e.g. clear and conversational" />
          </label>
        </div>

        <details className="walkthrough-pronunciation">
          <summary>Pronunciation lexicon <span>Optional · off by default</span></summary>
          <p>Add a code name and how it should sound. Include it only when you want matching text in the script replaced for speech.</p>
          {draft.lexicon.map((entry, index) => (
            <div className="walkthrough-pronunciation-row" key={`lexicon-${index}`}>
              <label>Exact name<input value={entry.name} maxLength={120} onChange={(event) => updateDraft((value) => ({ ...value, lexicon: replaceLexiconEntry(value.lexicon, index, { ...entry, name: event.target.value }) }))} placeholder="parseRoutes" /></label>
              <label>Say it as<input value={entry.sayAs} maxLength={160} onChange={(event) => updateDraft((value) => ({ ...value, lexicon: replaceLexiconEntry(value.lexicon, index, { ...entry, sayAs: event.target.value }) }))} placeholder="parse routes" /></label>
              <label className="walkthrough-pronunciation-include"><input type="checkbox" checked={entry.include} onChange={(event) => updateDraft((value) => ({ ...value, lexicon: replaceLexiconEntry(value.lexicon, index, { ...entry, include: event.target.checked }) }))} /> Include</label>
              <button type="button" className="walkthrough-pronunciation-remove" onClick={() => updateDraft((value) => ({ ...value, lexicon: value.lexicon.filter((_, itemIndex) => itemIndex !== index) }))} aria-label="Remove pronunciation entry"><X size={13} /></button>
            </div>
          ))}
          {draft.lexicon.length < 12 && <button type="button" className="walkthrough-add-pronunciation" onClick={() => updateDraft((value) => ({ ...value, lexicon: [...value.lexicon, { name: '', sayAs: '', include: false }] }))}>Add a name</button>}
        </details>
      </details>

      <div className="walkthrough-playback">
        <div className="walkthrough-playback-buttons">
          <button type="button" disabled={requestBusy || !draft.spokenText.trim()} onClick={() => { void replay(); }} aria-label="Replay narration"><RotateCcw size={14} /></button>
          <button type="button" className="walkthrough-play-control" disabled={requestBusy || !draft.spokenText.trim()} onClick={() => { void playOrPause(); }} aria-label={isPlaying ? 'Pause narration' : track ? 'Resume narration' : 'Review and play narration'}>
            {isPlaying ? <Pause size={14} /> : <Play size={14} />} {isPlaying ? 'Pause' : track ? 'Play' : 'Play'}
          </button>
          <button type="button" disabled={!track && !review} onClick={stop} aria-label="Stop narration"><Square size={12} /></button>
          <span className="walkthrough-duration">{formatTime(currentTime)} / {formatTime(duration)}</span>
        </div>
        <label className="walkthrough-seek-label" aria-label="Seek narration">
          <span>Audio position</span>
          <input type="range" min={0} max={duration || 0} step={0.05} value={Math.min(currentTime, duration || 0)} disabled={!track || !duration} onChange={(event) => seek(Number(event.target.value))} />
        </label>
      </div>
      {track && !savedAudioMatchesDraft && <p className="walkthrough-narration-prior-audio" role="status">Previous audio is kept for its reviewed transcript. New edits do not change it unless you generate another recording.</p>}

      <div className="walkthrough-narration-review-actions">
        <button type="button" className="secondary-button" onClick={() => { void makeReview(); }} disabled={reviewBusy || requestBusy || !disclosure || !draft.spokenText.trim()}>
          {reviewBusy ? 'Reviewing locally…' : 'Review exact speech request'}
        </button>
        {review && <span className="walkthrough-narration-fingerprint"><Check size={12} /> Request reviewed · {review.fingerprint.slice(0, 12)}</span>}
      </div>

      {showReview && review && disclosure && (
        <div className="walkthrough-narration-review" aria-label="Narration request review">
          <div className="walkthrough-narration-review-title"><div><span>LOCAL REVIEW · NO MODEL REQUEST SENT</span><strong>{disclosure.modelName}</strong></div><span>{review.cachedAudioAvailable ? 'AUDIO CACHED' : 'NEW AUDIO'}</span></div>
          <dl>
            <div><dt>Provider/model</dt><dd>{disclosure.provider} · <code>{review.modelId}</code></dd></div>
            <div><dt>Voice and style</dt><dd>{review.voice}{review.style ? ` · ${review.style}` : ' · default delivery'}</dd></div>
            <div><dt>Format</dt><dd>{review.format} · one current section</dd></div>
            <div><dt>Text hash</dt><dd><code>{review.spokenTextSha256}</code></dd></div>
          </dl>
          <div className="walkthrough-narration-exact-text"><span>EXACT NARRATION TEXT THAT WOULD BE SENT</span><p>{review.exactText}</p></div>
          <div className="walkthrough-narration-disclosure">
            <p><strong>Free Tier.</strong> {disclosure.freeTierTerms}</p>
            <p><strong>Possible paid-tier charges.</strong> {disclosure.paidTierTerms}</p>
            <p><strong>Unpaid-service data use.</strong> {disclosure.dataUseTerms}</p>
            <p><strong>Paid-service data use.</strong> {disclosure.paidDataUseTerms}</p>
            <p><strong>Regional data terms.</strong> {disclosure.regionalDataUseTerms}</p>
            <p><strong>Service eligibility.</strong> {disclosure.eligibilityTerms}</p>
            <p><strong>Billing and quota check.</strong> {disclosure.billingUncertainty}</p>
            <p><strong>Request contents.</strong> {disclosure.requestScope} Checked {disclosure.checkedDate}; availability and pricing may change. Code Atlas does not enable billing and will not switch models.</p>
            <nav aria-label="Google provider terms">
              <a href={disclosure.pricingUrl} target="_blank" rel="noreferrer">Pricing</a>
              <a href="https://ai.google.dev/gemini-api/docs/billing/" target="_blank" rel="noreferrer">Billing and project tier</a>
              <a href={disclosure.termsUrl} target="_blank" rel="noreferrer">Terms</a>
              <a href={disclosure.regionsUrl} target="_blank" rel="noreferrer">Supported regions</a>
            </nav>
          </div>
          {review.cachedAudioAvailable ? (
            <button type="button" className="primary-button" disabled={requestBusy} onClick={() => { void loadCache(review, true); }}><Play size={14} /> {requestBusy ? 'Loading cached audio…' : 'Play local cached audio'}</button>
          ) : (
            <>
              <label className="walkthrough-narration-consent"><input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} /> I checked this key's project tier and billing status in Google AI Studio, reviewed Google's data-use and eligibility terms, meet those requirements, and authorize sending this exact text to Gemini.</label>
              {!credential?.apiKeyConfigured && <p className="walkthrough-narration-key-status" role="status">{informationError ?? 'No Gemini API key is saved.'} Open <strong>Tools → AI and narration setup</strong> to add a key. Code Atlas cannot verify the key's project tier or quota.</p>}
              <button type="button" className="primary-button" disabled={!consent || !credential?.apiKeyConfigured || requestBusy} onClick={() => { void synthesize(); }}>
                <Play size={14} /> {requestBusy ? 'Requesting audio…' : 'Synthesize and play this section'}
              </button>
              {providerRequestBusy && (
                <div className="walkthrough-narration-cancel">
                  <button type="button" className="secondary-button" disabled={cancelBusy} onClick={() => setCancelConfirmation(true)}>Cancel request</button>
                  {cancelStatus && <p role="status">{cancelStatus}</p>}
                  {cancelConfirmation && (
                    <div className="provider-cancel-confirm" role="group" aria-labelledby="narration-cancel-title" aria-describedby="narration-cancel-description">
                      <strong id="narration-cancel-title">Cancel this narration request?</strong>
                      <p id="narration-cancel-description">Code Atlas will try to stop its local HTTPS request. Google may already have received the reviewed speech text, and charges may still apply. Any returned audio will be discarded; your transcript and previous audio remain available.</p>
                      <div>
                        <button type="button" className="secondary-button" onClick={() => setCancelConfirmation(false)}>Keep waiting</button>
                        <button type="button" className="danger-button" disabled={cancelBusy} onClick={() => { void confirmCancelSynthesis(); }}>{cancelBusy ? 'Stopping…' : 'Confirm cancel'}</button>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </>
          )}
        </div>
      )}
      {!disclosure && informationError && <p className="walkthrough-narration-note" role="status">{informationError}</p>}
      {error && <p className="walkthrough-narration-error" role="alert"><AlertCircle size={14} /> {error}</p>}
      <p className="walkthrough-narration-offline"><Volume2 size={13} /> Audio is optional. The visible explanation, editable transcript, and source references remain available offline.</p>
    </section>
  );
}

function readPreferences(storageKey: string, fallbackText: string): NarrationPreferences {
  try {
    const value = window.localStorage.getItem(storageKey);
    if (value) {
      const parsed = JSON.parse(value) as Partial<NarrationPreferences>;
      return {
        spokenText: typeof parsed.spokenText === 'string' ? parsed.spokenText : fallbackText,
        voice: typeof parsed.voice === 'string' ? parsed.voice : 'Kore',
        style: typeof parsed.style === 'string' ? parsed.style : DEFAULT_STYLE,
        lexicon: Array.isArray(parsed.lexicon) ? parsed.lexicon.filter(isLexiconEntry).slice(0, 12) : [],
      };
    }
  } catch { /* Default to the accepted chapter script when storage is unavailable. */ }
  return { spokenText: fallbackText, voice: 'Kore', style: DEFAULT_STYLE, lexicon: [] };
}

function isLexiconEntry(value: unknown): value is NarrationPronunciationEntry {
  return Boolean(value && typeof value === 'object'
    && typeof (value as NarrationPronunciationEntry).name === 'string'
    && typeof (value as NarrationPronunciationEntry).sayAs === 'string'
    && typeof (value as NarrationPronunciationEntry).include === 'boolean');
}

function replaceLexiconEntry(entries: NarrationPronunciationEntry[], index: number, entry: NarrationPronunciationEntry) {
  return entries.map((current, currentIndex) => currentIndex === index ? entry : current);
}

function formatTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds < 0) return '--:--';
  const total = Math.floor(seconds);
  return `${Math.floor(total / 60).toString().padStart(2, '0')}:${(total % 60).toString().padStart(2, '0')}`;
}
