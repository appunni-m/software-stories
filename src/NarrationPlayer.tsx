import { useEffect, useId, useMemo, useRef, useState } from 'react';
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
  cancelProviderRequest,
  discardNarrationRequestReview,
  getNarrationDisclosure,
  getLocalNarrationPreparationStatus,
  getPocketTtsStatus,
  type PocketTtsStatus,
  type NarrationBackend,
  getProviderCredentialStatus,
  isNativeRuntime,
  loadReviewedNarrationAudio,
  synthesizeReviewedNarrationAudio,
  type NarrationAudio,
  type NarrationDisclosure,
  type NarrationPronunciationEntry,
  type NarrationRequestReview,
  type NarrationPrewarmState,
  type ProviderCredentialStatus,
} from './lib/commands';
import {
  parseNarrationPreferences,
  defaultNarrationText,
  localNarrationAction,
  narrationAvailabilityLabel,
  narrationPreferencesMatch,
  type NarrationPreferencesSnapshot,
} from './lib/narration';
import { narrationStorageKey, scopedNarrationPreferences } from './lib/narrationScope';
import { applyVoiceConnection } from './lib/voiceConnection';
import { openVoiceConnections, readVoiceConnection, readVoiceConnectionIssue, useVoiceConnection } from './lib/voiceConnectionStore';
import type { ChapterSection } from './lib/commands';
import {appCloseCoordinator} from './lib/closeCoordinator';

interface NarrationPlayerProps {
  projectId: string;
  chapterId: string;
  revisionId: string;
  section: ChapterSection;
  restorePosition: number;
  transcriptPosition: number;
  onPlaybackPositionChange: (position: number) => void;
  onTranscriptPositionChange: (position: number) => void;
  onOpenProviderSettings: () => void;
  autoPlay?: boolean;
  clip?: { id: string; text: string };
  onEnded?: () => void;
  compact?: boolean;
}

type NarrationPreferences = NarrationPreferencesSnapshot;

interface LoadedTrack {
  backend: NarrationBackend;
  audioUrl: string;
  exactText: string;
  sourceText: string;
  voice: string;
  style: string;
  lexicon: NarrationPronunciationEntry[];
  fingerprint: string;
  cacheHit: boolean;
}

export function NarrationPlayer({ projectId, chapterId, revisionId, section, restorePosition, transcriptPosition, onPlaybackPositionChange, onTranscriptPositionChange, autoPlay = false, clip, onEnded, compact = false }: NarrationPlayerProps) {
  const titleId = useId();
  const baseStorageKey = narrationStorageKey(projectId, chapterId, revisionId, section.sectionId);
  const storageKey = clip ? `${baseStorageKey}:scene:${clip.id}` : baseStorageKey;
  const fallbackScript = useMemo(() => defaultNarrationText(clip?.text ?? section.spokenText), [clip?.text, section.spokenText]);
  const preferenceLoad = useMemo(() => readPreferences(storageKey, fallbackScript, clip ? baseStorageKey : undefined), [storageKey, fallbackScript, baseStorageKey, !!clip]);
  const preferences = preferenceLoad.preferences;
  const [draft, setDraft] = useState<NarrationPreferences>(preferences);
  const [preferenceWarning, setPreferenceWarning] = useState<string | null>(preferenceLoad.warning);
  const voiceConnection = useVoiceConnection();
  const localNarration = draft.backend === 'pocket-tts';
  const [pocketStatus, setPocketStatus] = useState<PocketTtsStatus | null>(null);
  const [prewarmState, setPrewarmState] = useState<NarrationPrewarmState>('idle');
  const pocketStatusGenerationRef = useRef(0);
  const spokenTextUtf8Bytes = useMemo(() => new TextEncoder().encode(draft.spokenText).length, [draft.spokenText]);
  const draftRef = useRef(draft);
  draftRef.current = draft;
  const [connectionVersion, setConnectionVersion] = useState(0);
  const pendingListen = useRef(false);
  useEffect(() => {
    const refresh = () => setConnectionVersion((value) => value + 1);
    window.addEventListener('code-atlas:connections-changed', refresh);
    return () => window.removeEventListener('code-atlas:connections-changed', refresh);
  }, []);
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
  const completionArmed = useRef(false);
  const cancelRequested = useRef(false);
  const mountedRef = useRef(false);
  const reviewGenerationRef = useRef(0);
  const makeReviewRef = useRef<(restoreCache?: boolean) => Promise<NarrationRequestReview | null>>(async () => null);
  const cacheHydrationKeyRef = useRef<string | null>(null);
  const synthesisReviewIdRef = useRef<string | null>(null);
  const restoreTimeRef = useRef<number | null>(restorePosition > 0 ? restorePosition : null);
  const lastRoutePositionRef = useRef(restorePosition);
  const lastRouteWriteRef = useRef(0);

  useEffect(() => {
    const pauseOtherNarration = (event: Event) => {
      if ((event as CustomEvent<string>).detail !== titleId) audioRef.current?.pause();
    };
    window.addEventListener('code-atlas:narration-playing', pauseOtherNarration);
    return () => window.removeEventListener('code-atlas:narration-playing', pauseOtherNarration);
  }, [titleId]);

  useEffect(() => {
    const loaded = readPreferences(storageKey, fallbackScript, clip ? baseStorageKey : undefined);
    setDraft(loaded.preferences);
    setPreferenceWarning(loaded.warning);
    setReview(null);
    setTrack(null);
    setCurrentTime(0);
    setDuration(0);
  }, [storageKey, fallbackScript]);

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
    setDisclosure(null);
    setCredential(null);
    setInformationError(null);
    void (async () => {
      const nextDisclosure = await getNarrationDisclosure(draft.backend);
      if (!active) return;
      setDisclosure(nextDisclosure);
      if (!localNarration) {
        const status = await getProviderCredentialStatus('gemini');
        if (!active) return;
        if (status.status === 'ok') setCredential(status.data);
        else setInformationError(status.error.message);
      }
    })().catch(() => {
      if (active) setInformationError('Narration setup could not be loaded. The transcript remains available.');
    });
    return () => { active = false; };
  }, [draft.backend, localNarration, connectionVersion]);

  useEffect(() => {
    if (!isNativeRuntime() || !localNarration) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const poll = async () => {
      const generation = ++pocketStatusGenerationRef.current;
      try {
        const status = await getPocketTtsStatus();
        if (!active) return;
        if (generation === pocketStatusGenerationRef.current) setPocketStatus(status);
        if (status.busy || synthesisReviewIdRef.current) {
          timer = setTimeout(() => { void poll(); }, 1_000);
        }
      } catch {
        if (active && generation === pocketStatusGenerationRef.current) {
          setInformationError('Local runtime status could not be read. Refresh the setup status to retry.');
        }
      }
    };
    void poll();
    return () => {
      active = false;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [localNarration, providerRequestBusy, connectionVersion]);

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
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      completionArmed.current = false;
      reviewGenerationRef.current += 1;
      autoPlayRef.current = false;
      audioRef.current?.pause();
      // A section change also ends its in-flight request. Discarding its review
      // covers a command which has not consumed it yet; cancellation covers one
      // which has already started generating the reviewed narration.
      const synthesisReviewId = synthesisReviewIdRef.current;
      if (synthesisReviewId) {
        cancelRequested.current = true;
        void cancelProviderRequest(synthesisReviewId).catch(() => {});
        void discardNarrationRequestReview(synthesisReviewId).catch(() => {});
      }
      if (reviewRef.current?.reviewId) void discardNarrationRequestReview(reviewRef.current.reviewId).catch(() => {});
    };
  }, []);

  useEffect(() => {
    reviewGenerationRef.current += 1;
    const currentReview = reviewRef.current;
    if (currentReview) void discardNarrationRequestReview(currentReview.reviewId).catch(() => {});
    const running = synthesisReviewIdRef.current;
    if (running) { cancelRequested.current = true; void cancelProviderRequest(running).catch(() => {}); }
    audioRef.current?.pause();
    setDraft((value) => applyVoiceConnection(value, voiceConnection));
    setTrack(null); setCurrentTime(0); setDuration(0); setIsPlaying(false);
    setReview(null); setReviewBusy(false); setConsent(false); setShowReview(false); setError(null);
  }, [voiceConnection.backend, voiceConnection.googleVoice, voiceConnection.googleStyle]);

  const savedAudioMatchesDraft = Boolean(track
    && narrationPreferencesMatch({
      backend: track.backend,
      spokenText: track.sourceText,
      voice: track.voice,
      style: track.style,
      lexicon: track.lexicon,
    }, draft));

  const updateDraft = (update: (value: NarrationPreferences) => NarrationPreferences) => {
    setDraft((value) => {
      const next = update(value);
      try {
        window.localStorage.setItem(storageKey, JSON.stringify(next));
      } catch { /* Speech editing still works without storage. */ }
      setPreferenceWarning(null);
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
    const generation = ++reviewGenerationRef.current;
    const isCurrent = () => mountedRef.current && reviewGenerationRef.current === generation;
    const requestedDraft = draft;
    try {
      const result = await buildNarrationRequestReview({
        backend: requestedDraft.backend,
        spokenText: requestedDraft.spokenText,
        voice: requestedDraft.voice,
        style: requestedDraft.style,
        lexicon: requestedDraft.lexicon,
      });
      if (!isCurrent()) {
        if (result.status === 'ok') void discardNarrationRequestReview(result.data.reviewId).catch(() => {});
        return null;
      }
      if (result.status === 'error') {
        setError(result.error.message);
        return null;
      }
      if (!narrationPreferencesMatch(draftRef.current, requestedDraft)) {
        void discardNarrationRequestReview(result.data.reviewId).catch(() => {});
        setError('The script or narration settings changed during review. Review the current text before generating audio.');
        return null;
      }
      if (review && review.reviewId !== result.data.reviewId) void discardNarrationRequestReview(review.reviewId).catch(() => {});
      setReview(result.data);
      setConsent(false);
      // Keep provider/script review for cloud requests; local Pocket narration
      // is user-controlled and stays in the background until the reader presses Play.
      setShowReview(!restoreCache && !localNarration);
      if (restoreCache && result.data.cachedAudioAvailable) {
        const cached = await loadReviewedNarrationAudio({ reviewId: result.data.reviewId, fingerprint: result.data.fingerprint });
        if (!isCurrent()) {
          void discardNarrationRequestReview(result.data.reviewId).catch(() => {});
          return null;
        }
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
      if (isCurrent()) setError('The local narration review could not be built. No model request was sent.');
      return null;
    } finally {
      if (isCurrent()) setReviewBusy(false);
    }
  };
  makeReviewRef.current = makeReview;

  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const requestedDraft = draft;
    const refresh = async () => {
      try {
        const result = await getLocalNarrationPreparationStatus({
          spokenText: requestedDraft.spokenText,
          voice: requestedDraft.voice,
          style: requestedDraft.style,
          lexicon: requestedDraft.lexicon,
        });
        if (!active || !narrationPreferencesMatch(draftRef.current, requestedDraft)) return;
        if (result.status === 'error') {
          setPrewarmState('unavailable');
          return;
        }
        setPrewarmState(result.data.state);
        if (result.data.state === 'preparing') timer = setTimeout(() => { void refresh(); }, 750);
      } catch {
        if (active) setPrewarmState('unavailable');
      }
    };
    setPrewarmState('idle');
    if (isNativeRuntime() && requestedDraft.backend === 'pocket-tts' && requestedDraft.spokenText.trim()) {
      timer = setTimeout(() => { void refresh(); }, 100);
    }
    return () => {
      active = false;
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [storageKey, draft.backend, draft.spokenText, draft.voice, draft.style, draft.lexicon]);

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
    return () => {
      active = false;
      // React may restart this mount effect before its cache read completes.
      if (restoreTimeRef.current === null) restoreTimeRef.current = restoredPosition;
    };
  }, []);

  const setLoadedTrack = (audio: NarrationAudio, currentReview: NarrationRequestReview, playAfterLoad: boolean, sourcePreferences: NarrationPreferences = draft) => {
    const dataUrl = `data:${audio.mimeType};base64,${audio.audioBase64}`;
    setTrack({
      backend: sourcePreferences.backend,
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
      if (!mountedRef.current) return;
      if (result.status === 'error') {
        setError(result.error.message);
        return;
      }
      if (!narrationPreferencesMatch(draftRef.current, requestedDraft)) return;
      setLoadedTrack(result.data, currentReview, playAfterLoad, requestedDraft);
    } catch {
      if (mountedRef.current) setError('The local audio cache could not be read. The transcript remains available.');
    } finally {
      if (mountedRef.current) setRequestBusy(false);
    }
  };

  const cacheHydrationKey = JSON.stringify([
    storageKey,
    draft.backend,
    draft.spokenText,
    draft.voice,
    draft.style,
    draft.lexicon,
  ]);
  useEffect(() => {
    if (!isNativeRuntime()
      || !localNarration
      || prewarmState !== 'ready'
      || savedAudioMatchesDraft
      || reviewBusy
      || requestBusy
      || providerRequestBusy
      || synthesisReviewIdRef.current
      || cacheHydrationKeyRef.current === cacheHydrationKey) return;

    // Load the already-prepared local WAV so duration and seek controls are
    // ready before Play. This reads the canonical cache only; it never starts
    // synthesis and never changes manual playback into autoplay.
    cacheHydrationKeyRef.current = cacheHydrationKey;
    void makeReviewRef.current(true);
  }, [cacheHydrationKey, localNarration, prewarmState, savedAudioMatchesDraft, reviewBusy, requestBusy, providerRequestBusy, track?.audioUrl]);

  const synthesize = async (currentReview: NarrationRequestReview | null = review) => {
    if (!currentReview || synthesisReviewIdRef.current) return;
    if (localNarration ? !pocketStatus?.ready : !credential?.apiKeyConfigured) {
      setError(localNarration ? 'Set up Pocket TTS on this Mac before generating audio.' : 'Add a Gemini API key before requesting cloud audio.');
      return;
    }
    setRequestBusy(true);
    setProviderRequestBusy(true);
    synthesisReviewIdRef.current = currentReview.reviewId;
    cancelRequested.current = false;
    setCancelConfirmation(false);
    setCancelBusy(false);
    setCancelStatus(null);
    setError(null);
    audioRef.current?.pause();
    try {
      const result = await synthesizeReviewedNarrationAudio({
        reviewId: currentReview.reviewId,
        fingerprint: currentReview.fingerprint,
        consent: { termsAndRequestAcknowledged: !localNarration && consent },
      });
      if (!mountedRef.current) return;
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
        setError(localNarration
          ? 'Local synthesis was cancelled. The returned audio was discarded; the transcript and previous audio remain available.'
          : 'Cancellation was requested after Google may have received the reviewed narration text. The returned audio was discarded; the transcript and previous audio remain available.');
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
      setLoadedTrack(result.data, currentReview, true, draft);
      setConsent(false);
      setReview(null);
      setShowReview(false);
    } catch {
      if (!mountedRef.current) return;
      setError(localNarration ? 'Pocket TTS could not finish. The transcript and previous audio remain available; review again before retrying.' : 'Google TTS could not be reached. The transcript and previous audio remain available; review again before retrying.');
      setReview(null);
      setConsent(false);
      setShowReview(false);
    } finally {
      synthesisReviewIdRef.current = null;
      if (mountedRef.current) {
        setRequestBusy(false);
        setProviderRequestBusy(false);
        setCancelConfirmation(false);
        setCancelBusy(false);
      }
    }
  };

  const confirmCancelSynthesis = async () => {
    if (!review || !providerRequestBusy || cancelBusy) return;
    cancelRequested.current = true;
    setCancelBusy(true);
    setCancelConfirmation(false);
    setCancelStatus(null);
    try {
      const stopped = await cancelProviderRequest(review.reviewId);
      if (!mountedRef.current) return;
      setCancelStatus(stopped
        ? (localNarration ? 'Cancellation requested. Waiting for local synthesis to stop; no returned audio will be accepted.' : 'Cancellation was requested locally. Google may already have received the reviewed narration text; no returned audio will be accepted.')
        : 'The request had already finished locally. Its audio will be discarded because you chose Cancel.');
    } catch {
      if (mountedRef.current) setCancelStatus(localNarration ? 'The local abort could not be confirmed; any returned audio will be discarded.' : 'The app could not confirm a local abort. Google may already have received the reviewed narration text; any returned audio will be discarded.');
    } finally {
      if (mountedRef.current) setCancelBusy(false);
    }
  };

  const playOrPause = async () => {
    const player = audioRef.current;
    if (isPlaying) {
      player?.pause();
      return;
    }
    if (track && savedAudioMatchesDraft && player?.src) {
      try { await player.play(); } catch { if (mountedRef.current) setError('Audio playback could not start on this device. The transcript remains available.'); }
      return;
    }
    if (localNarration && !pocketStatus?.ready) { pendingListen.current = true; openVoiceConnections(); return; }
    const currentReview = review ?? await makeReview();
    if (!currentReview) return;
    if (localNarration) {
      const action = localNarrationAction(currentReview.cachedAudioAvailable);
      if (action === 'play-cache') await loadCache(currentReview, true);
      else if (mountedRef.current) await synthesize(currentReview);
    } else if (currentReview.cachedAudioAvailable) await loadCache(currentReview, true);
  };

  const listenStarted = useRef(false);
  useEffect(() => {
    if (!autoPlay || listenStarted.current || !disclosure || (localNarration && !pocketStatus)) return;
    listenStarted.current = true;
    void playOrPause();
  }, [autoPlay, disclosure, localNarration, pocketStatus]);

  useEffect(() => {
    if (!pendingListen.current || !disclosure || (localNarration && !pocketStatus?.ready)) return;
    pendingListen.current = false;
    void playOrPause();
  }, [connectionVersion, disclosure, localNarration, pocketStatus]);

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
    try { await player.play(); } catch { if (mountedRef.current) setError('Audio playback could not start on this device. The transcript remains available.'); }
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

  useEffect(()=>appCloseCoordinator.register({
    capture:()=>{
      const player=audioRef.current;
      // An unloaded audio element reports zero; it must not erase a saved time.
      if(mountedRef.current&&player&&player.readyState>=1&&restoreTimeRef.current===null){
        persistPlaybackPosition(player.currentTime);
        completionArmed.current=false;player.pause();
      }
    },
    flush:()=>Promise.resolve(),
  }),[storageKey,onPlaybackPositionChange]);

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

  const captionText = track?.exactText ?? review?.exactText ?? draft.spokenText;

  return (
    <section className={`walkthrough-transcript walkthrough-narration${compact ? ' is-story-audio' : ''}`} aria-labelledby={titleId}>
      <audio
        ref={audioRef}
        className="walkthrough-narration-audio"
        preload="metadata"
        onPlay={() => { completionArmed.current = true; setIsPlaying(true); window.dispatchEvent(new CustomEvent('code-atlas:narration-playing', { detail: titleId })); }}
        onPause={(event) => {
          setIsPlaying(false);
          if (!event.currentTarget.ended) {
            completionArmed.current = false;
            if(mountedRef.current)persistPlaybackPosition(event.currentTarget.currentTime);
          }
        }}
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
          void event.currentTarget.play().catch(() => {
            if (mountedRef.current) setError('Audio playback could not start on this device. The transcript remains available.');
          });
        }}
        onTimeUpdate={handleTimeUpdate}
        onEnded={() => {
          setIsPlaying(false);
          const completedPlayback = completionArmed.current && mountedRef.current && audioRef.current?.ended;
          completionArmed.current = false;
          persistPlaybackPosition(audioRef.current?.currentTime ?? 0);
          // Seeking a paused restored clip to its saved end can also emit ended.
          // Only an actual playback completion may advance the visual story.
          if (completedPlayback) onEnded?.();
        }}
      />
      <div className="walkthrough-panel-heading">
        <div><h3 id={titleId}>{clip ? 'Scene narration' : 'Narration'}</h3></div>
        <span className={`walkthrough-transcript-state${isPlaying ? ' is-playing' : track && savedAudioMatchesDraft && currentTime <= 0 ? ' is-ready' : track ? '' : prewarmState === 'preparing' || (localNarration && providerRequestBusy) ? ' is-preparing' : prewarmState === 'ready' ? ' is-ready' : prewarmState === 'unavailable' ? ' is-unavailable' : ''}`} role="status"><Volume2 size={13} /> {isPlaying ? 'Playing' : localNarration && providerRequestBusy ? 'Preparing audio' : track && savedAudioMatchesDraft && currentTime <= 0 ? narrationAvailabilityLabel('ready') : track ? 'Paused' : narrationAvailabilityLabel(prewarmState)}</span>
      </div>

      <details className="walkthrough-narration-script" onToggle={(event) => { if (event.currentTarget.open) { const editor = event.currentTarget.parentElement?.querySelector<HTMLDetailsElement>(':scope > .walkthrough-narration-editor'); if (editor?.open) editor.open = false; } }}>
      <summary>Read narration</summary>
      <div className="walkthrough-transcript-scroll" ref={transcriptRef} onScroll={(event) => onTranscriptPositionChange(event.currentTarget.scrollTop)} role="group" tabIndex={0} aria-label={`Narration for ${section.title}`}>
        <div className="walkthrough-caption">
          <p>{captionText.trim() ? captionText : <em>No narration text is saved for this section. The written explanation and source evidence remain available.</em>}</p>
        </div>
      </div>
      </details>

      <details className="walkthrough-narration-editor" onToggle={(event) => { if (event.currentTarget.open) { const script = event.currentTarget.parentElement?.querySelector<HTMLDetailsElement>(':scope > .walkthrough-narration-script'); if (script?.open) script.open = false; } }}>
        <summary><span>Edit spoken text</span><ChevronDown size={13} /></summary>
        <div className="walkthrough-narration-editor-content">
          <label className="walkthrough-narration-field">Spoken text
            <textarea value={draft.spokenText} disabled={reviewBusy || requestBusy} maxLength={16000} rows={4} onChange={(event) => updateDraft((value) => ({ ...value, spokenText: event.target.value }))} placeholder="Write what you want the reader to say in natural language." />
            {spokenTextUtf8Bytes > 16000 && <small role="alert">Shorten this script before listening.</small>}
          </label>
          <details className="walkthrough-pronunciation">
            <summary>Pronunciation lexicon <span>Optional · off by default</span></summary>
            <p>Add a code name and how it should sound. Include it only when you want matching text in the script replaced for speech.</p>
            {draft.lexicon.map((entry, index) => (
              <div className="walkthrough-pronunciation-row" key={`lexicon-${index}`}>
                <label>Exact name<input value={entry.name} disabled={reviewBusy || requestBusy} maxLength={120} onChange={(event) => updateDraft((value) => ({ ...value, lexicon: replaceLexiconEntry(value.lexicon, index, { ...entry, name: event.target.value }) }))} placeholder="parseRoutes" /></label>
                <label>Say it as<input value={entry.sayAs} disabled={reviewBusy || requestBusy} maxLength={160} onChange={(event) => updateDraft((value) => ({ ...value, lexicon: replaceLexiconEntry(value.lexicon, index, { ...entry, sayAs: event.target.value }) }))} placeholder="parse routes" /></label>
                <label className="walkthrough-pronunciation-include"><input type="checkbox" disabled={reviewBusy || requestBusy} checked={entry.include} onChange={(event) => updateDraft((value) => ({ ...value, lexicon: replaceLexiconEntry(value.lexicon, index, { ...entry, include: event.target.checked }) }))} /> Include</label>
                <button type="button" className="walkthrough-pronunciation-remove" disabled={reviewBusy || requestBusy} onClick={() => updateDraft((value) => ({ ...value, lexicon: value.lexicon.filter((_, itemIndex) => itemIndex !== index) }))} aria-label="Remove pronunciation entry"><X size={13} /></button>
              </div>
            ))}
            {draft.lexicon.length < 12 && <button type="button" className="walkthrough-add-pronunciation" disabled={reviewBusy || requestBusy} onClick={() => updateDraft((value) => ({ ...value, lexicon: [...value.lexicon, { name: '', sayAs: '', include: false }] }))}>Add a name</button>}
          </details>
        </div>
      </details>

      <div className="walkthrough-playback">
        <div className="walkthrough-playback-buttons">
          <button type="button" disabled={reviewBusy || requestBusy || !draft.spokenText.trim()} onClick={() => { void replay(); }} aria-label="Replay narration"><RotateCcw size={14} /></button>
          <button type="button" className="walkthrough-play-control" disabled={reviewBusy || requestBusy || !draft.spokenText.trim()} onClick={() => { void playOrPause(); }} aria-label={isPlaying ? 'Pause narration' : 'Play narration'}>
            {isPlaying ? <Pause size={14} /> : <Play size={14} />} {requestBusy || reviewBusy ? 'Preparing audio…' : isPlaying ? 'Pause' : 'Play'}
          </button>
          <button type="button" disabled={reviewBusy || requestBusy || (!track && !review)} onClick={stop} aria-label="Stop narration"><Square size={12} /></button>
          <span className="walkthrough-duration">{formatTime(currentTime)} / {formatTime(duration)}</span>
        </div>
        <label className="walkthrough-seek-label">
          <span>Audio position</span>
          <input type="range" min={0} max={duration || 0} step={0.05} value={Math.min(currentTime, duration || 0)} disabled={!track || !duration} onChange={(event) => seek(Number(event.target.value))} />
        </label>
      </div>
      {localNarration && providerRequestBusy && <button type="button" className="text-button" disabled={cancelBusy} onClick={() => { void confirmCancelSynthesis(); }}>{cancelBusy ? 'Stopping…' : 'Cancel audio'}</button>}
      {track && !savedAudioMatchesDraft && <p className="walkthrough-narration-prior-audio" role="status">Previous audio is kept for its reviewed transcript. New edits do not change it unless you generate another recording.</p>}

      {!localNarration && <div className="walkthrough-narration-review-actions">
        <button type="button" className="secondary-button" onClick={() => { void makeReview(); }} disabled={reviewBusy || requestBusy || !disclosure || !draft.spokenText.trim() || spokenTextUtf8Bytes > 16000}>
          {reviewBusy ? 'Reviewing locally…' : 'Review exact speech request'}
        </button>
        {review && <span className="walkthrough-narration-fingerprint"><Check size={12} /> Request reviewed · {review.fingerprint.slice(0, 12)}</span>}
      </div>}

      {showReview && review && disclosure && !localNarration && (
        <div className="walkthrough-narration-review" role="group" aria-label="Narration request review">
          <div className="walkthrough-narration-review-title"><div><span>{providerRequestBusy ? (localNarration ? 'GENERATING LOCALLY' : 'CLOUD REQUEST IN PROGRESS') : localNarration ? 'LOCAL SCRIPT REVIEW' : 'LOCAL REVIEW · NO MODEL REQUEST SENT'}</span><strong>{disclosure.modelName}</strong></div><span>{review.cachedAudioAvailable ? 'AUDIO CACHED' : 'NEW AUDIO'}</span></div>
          <details>
            <summary>Audio details</summary>
            <dl>
              <div><dt>Provider/model</dt><dd>{disclosure.provider} · <code>{review.modelId}</code></dd></div>
              <div><dt>Voice and style</dt><dd>{review.voice}{review.style ? ` · ${review.style}` : ' · default delivery'}</dd></div>
              <div><dt>Format</dt><dd>{review.format} · {review.requestSegments.length} {localNarration ? 'local segment' : 'sequential request'}{review.requestSegments.length === 1 ? '' : 's'} for this section</dd></div>
              <div><dt>Terms version</dt><dd><code>{review.disclosureVersion}</code></dd></div>
              <div><dt>Text hash</dt><dd><code>{review.spokenTextSha256}</code></dd></div>
            </dl>
          </details>
          <div className="walkthrough-narration-exact-text"><span>{localNarration ? 'WHAT YOU WILL HEAR' : 'FULL EXACT NARRATION TEXT FOR THESE REQUESTS'}</span><p>{review.exactText}</p></div>
          {review.requestSegments.length > 1 && <details className="walkthrough-narration-segments">
            <summary>Inspect the {review.requestSegments.length} exact request segments</summary>
            <ol>{review.requestSegments.map((segment, index) => <li key={`${review.reviewId}-${index}`}><span>Request {index + 1}</span><p>{segment}</p></li>)}</ol>
          </details>}
          {localNarration ? <div className="walkthrough-narration-disclosure"><p>{disclosure.dataUseTerms}</p><p>{disclosure.eligibilityTerms}</p><p>{disclosure.requestScope}</p></div> : <div className="walkthrough-narration-disclosure">
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
          </div>}
          {review.cachedAudioAvailable ? (
            <button type="button" className="primary-button" disabled={requestBusy} onClick={() => { void loadCache(review, true); }}><Play size={14} /> {requestBusy ? 'Loading cached audio…' : 'Play local cached audio'}</button>
          ) : (
            <>
              {!localNarration && <><label className="walkthrough-narration-consent"><input type="checkbox" checked={consent} onChange={(event) => setConsent(event.target.checked)} /> I checked this key's project tier and billing status in Google AI Studio, reviewed Google's data-use and eligibility terms, meet those requirements, and authorize sending this exact text to Gemini.</label>
              <div className="walkthrough-narration-key-status" role="status">
                <span>{informationError ?? (credential?.apiKeyConfigured ? 'Gemini key is stored in macOS Keychain.' : 'No Gemini API key is saved.')} Code Atlas cannot verify the key's project tier or quota.</span>
                <button type="button" className="text-button" onClick={openVoiceConnections}>{credential?.apiKeyConfigured ? 'Manage Gemini key' : 'Set up Gemini for narration'}</button>
              </div>
              </>}
              <button type="button" className="primary-button" disabled={(localNarration ? !pocketStatus?.ready : !consent || !credential?.apiKeyConfigured) || requestBusy} onClick={() => { void synthesize(); }}>
                <Play size={14} /> {requestBusy ? 'Generating audio…' : localNarration ? prewarmState === 'preparing' ? 'Play when ready' : 'Generate locally and play' : 'Synthesize and play this section'}
              </button>
              {providerRequestBusy && (
                <div className="walkthrough-narration-cancel">
                  <button type="button" className="secondary-button" disabled={cancelBusy} onClick={() => setCancelConfirmation(true)}>Cancel request</button>
                  {cancelStatus && <p role="status">{cancelStatus}</p>}
                  {cancelConfirmation && (
                    <div className="provider-cancel-confirm" role="group" aria-labelledby="narration-cancel-title" aria-describedby="narration-cancel-description">
                      <strong id="narration-cancel-title">Cancel this narration request?</strong>
                      <p id="narration-cancel-description">{localNarration ? 'Code Atlas will stop the local playback request. Background audio preparation may continue and cache this clip.' : 'Code Atlas will try to stop its HTTPS request. Google may already have received the reviewed speech text, and charges may still apply.'} Any returned audio will be discarded; your transcript and previous audio remain available.</p>
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
      {preferenceWarning && <p className="walkthrough-narration-note" role="status">{preferenceWarning}</p>}
      {readVoiceConnectionIssue() && <p className="walkthrough-narration-note" role="status">{readVoiceConnectionIssue()}</p>}
      {!disclosure && informationError && <p className="walkthrough-narration-note" role="status">{informationError}</p>}
      {error && <p className="walkthrough-narration-error" role="alert"><AlertCircle size={14} /> {error}</p>}
    </section>
  );
}

function readPreferences(storageKey: string, fallbackText: string, parentKey?: string): { preferences: NarrationPreferences; warning: string | null } {
  try {
    return { preferences: applyVoiceConnection(scopedNarrationPreferences(key => window.localStorage.getItem(key), storageKey, fallbackText, parentKey), readVoiceConnection()), warning: null };
  } catch (error) {
    const warning = error instanceof Error ? error.message : 'Saved narration preferences could not be read. The accepted transcript remains available.';
    return { preferences: applyVoiceConnection(parseNarrationPreferences(null, fallbackText), readVoiceConnection()), warning };
  }
}

function replaceLexiconEntry(entries: NarrationPronunciationEntry[], index: number, entry: NarrationPronunciationEntry) {
  return entries.map((current, currentIndex) => currentIndex === index ? entry : current);
}

function formatTime(seconds: number) {
  if (!Number.isFinite(seconds) || seconds < 0) return '--:--';
  const total = Math.floor(seconds);
  return `${Math.floor(total / 60).toString().padStart(2, '0')}:${(total % 60).toString().padStart(2, '0')}`;
}
