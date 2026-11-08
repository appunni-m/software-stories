import { useEffect, useId, useMemo, useRef, useState } from 'react';
import { InlineCodeText } from './InlineCodeText';
import { AppTextArea } from './design/AppTextArea';
import { AppTextField } from './design/AppTextField';
import { AppCheckbox } from './design/AppCheckbox';
import { plainInlineCodeText } from './lib/inlineCode';
import {
  AlertCircle,
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
  getLocalNarrationPreparationStatus,
  getPocketTtsStatus,
  type PocketTtsStatus,
  type NarrationBackend,
  isNativeRuntime,
  loadReviewedNarrationAudio,
  synthesizeReviewedNarrationAudio,
  type NarrationAudio,
  type NarrationPronunciationEntry,
  type NarrationRequestReview,
  type NarrationPrewarmState,
} from './lib/commands';
import {
  parseNarrationPreferences,
  defaultNarrationText,
  narrationAvailabilityLabel,
  narrationDurationLabel,
  narrationPreferencesMatch,
  narrationTrackMatchesPreferences,
  serializeNarrationPreferences,
  type NarrationPreferencesSnapshot,
} from './lib/narration';
import { localNarrationPrewarmState, localNarrationPrewarmer } from './workspace/localNarrationQueue';
import { listenForPocketTtsRuntimeReady } from './workspace/pocketRuntimeEvents';
import { AppButton } from './design/AppButton';
import { AppSlider } from './design/AppSlider';
import { narrationStorageKey, scopedNarrationPreferences } from './lib/narrationScope';
import { applyVoiceConnection } from './lib/voiceConnection';
import { openVoiceConnections, readVoiceConnection, readVoiceConnectionIssue, useVoiceConnection } from './lib/voiceConnectionStore';
import type { ChapterSection } from './lib/commands';
import {appCloseCoordinator} from './lib/closeCoordinator';
import { playbackPositionToPersist } from './lib/narrationPlayback';

interface NarrationPlayerProps {
  projectId: string;
  chapterId: string;
  revisionId: string;
  section: ChapterSection;
  restorePosition: number;
  transcriptPosition: number;
  onPlaybackPositionChange: (position: number) => void;
  onTranscriptPositionChange: (position: number) => void;
  onPlaybackStarted?: () => void;
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

export function NarrationPlayer({ projectId, chapterId, revisionId, section, restorePosition, transcriptPosition, onPlaybackPositionChange, onTranscriptPositionChange, onPlaybackStarted, autoPlay = false, clip, onEnded, compact = false }: NarrationPlayerProps) {
  const titleId = useId();
  const baseStorageKey = narrationStorageKey(projectId, chapterId, revisionId, section.sectionId);
  const storageKey = clip ? `${baseStorageKey}:scene:${clip.id}` : baseStorageKey;
  const fallbackScript = useMemo(() => defaultNarrationText(clip?.text ?? section.spokenText), [clip?.text, section.spokenText]);
  const preferenceLoad = useMemo(() => readPreferences(storageKey, fallbackScript, clip ? baseStorageKey : undefined), [storageKey, fallbackScript, baseStorageKey, !!clip]);
  const preferences = preferenceLoad.preferences;
  const [draft, setDraft] = useState<NarrationPreferences>(preferences);
  const [preferenceWarning, setPreferenceWarning] = useState<string | null>(preferenceLoad.warning);
  const voiceConnection = useVoiceConnection();
  const voiceConnectionIssue = readVoiceConnectionIssue();
  const [pocketStatus, setPocketStatus] = useState<PocketTtsStatus | null>(null);
  const [prewarmState, setPrewarmState] = useState<NarrationPrewarmState | 'checking'>(
    isNativeRuntime() && !voiceConnectionIssue && preferences.backend === 'pocket-tts' && preferences.spokenText.trim()
      ? 'checking'
      : 'idle',
  );
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
  const [informationError, setInformationError] = useState<string | null>(null);
  const [review, setReview] = useState<NarrationRequestReview | null>(null);
  const [reviewBusy, setReviewBusy] = useState(false);
  const [requestBusy, setRequestBusy] = useState(false);
  const [providerRequestBusy, setProviderRequestBusy] = useState(false);
  const [cancelConfirmation, setCancelConfirmation] = useState(false);
  const [cancelBusy, setCancelBusy] = useState(false);
  const [cancelStatus, setCancelStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [track, setTrack] = useState<LoadedTrack | null>(null);
  const [isPlaying, setIsPlaying] = useState(false);
  const [currentTime, setCurrentTime] = useState(0);
  const [duration, setDuration] = useState(0);
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
    if (!voiceConnectionIssue) return;
    audioRef.current?.pause();
    setIsPlaying(false);
  }, [voiceConnectionIssue]);

  useEffect(() => {
    if (!isNativeRuntime()) return;
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
  }, [providerRequestBusy, connectionVersion]);

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
      // This effect cleanup runs before the mounted ref cleanup on unmount,
      // and on track changes it still has the old route's callback captured.
      const position = playbackPositionToPersist(player, restoreTimeRef.current !== null);
      if (position !== undefined) persistPlaybackPosition(position);
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
      const player = audioRef.current;
      const position = playbackPositionToPersist(player, restoreTimeRef.current !== null);
      if (position !== undefined) persistPlaybackPosition(position);
      mountedRef.current = false;
      completionArmed.current = false;
      reviewGenerationRef.current += 1;
      autoPlayRef.current = false;
      player?.pause();
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
    setReview(null); setReviewBusy(false); setError(null);
  }, [voiceConnection.backend]);

  const savedAudioMatchesDraft = narrationTrackMatchesPreferences(track, draft);
  useEffect(() => {
    if (!track || savedAudioMatchesDraft) return;
    // A retained clip belongs to its original transcript. Stop it as soon as
    // the reader changes the script/settings, and disarm deferred canplay so
    // an old clip cannot start after the edit.
    autoPlayRef.current = false;
    completionArmed.current = false;
    audioRef.current?.pause();
    setIsPlaying(false);
  }, [track, savedAudioMatchesDraft]);
  const localAudioNeedsPreparation = pocketStatus?.ready === true
    && !savedAudioMatchesDraft
    && prewarmState === 'idle';

  const updateDraft = (update: (value: NarrationPreferences) => NarrationPreferences) => {
    const next = update(draftRef.current);
    draftRef.current = next;
    if (track && !narrationTrackMatchesPreferences(track, next)) {
      // Do this synchronously with the edit; waiting for a passive effect can
      // let a queued `canplay` event start the old recording once more.
      autoPlayRef.current = false;
      completionArmed.current = false;
      audioRef.current?.pause();
      setIsPlaying(false);
    }
    setDraft(next);
    const serialized = serializeNarrationPreferences(next);
    if (serialized === null) {
      setPreferenceWarning('This narration edit exceeds the saved format limits. The previous saved script remains unchanged.');
    } else {
      try {
        window.localStorage.setItem(storageKey, serialized);
        setPreferenceWarning(null);
      } catch {
        setPreferenceWarning('This narration edit is available for this session, but could not be saved on this Mac. The previously saved script remains unchanged.');
      }
    }
    if (review) void discardNarrationRequestReview(review.reviewId).catch(() => {});
    setReview(null);
    setError(null);
  };

  const makeReview = async (restoreCache = false): Promise<NarrationRequestReview | null> => {
    if (voiceConnectionIssue) {
      setError(voiceConnectionIssue);
      return null;
    }
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
      // The request review stays internal; the reader controls playback.
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
    const shouldCheckCache = isNativeRuntime()
      && !voiceConnectionIssue
      && requestedDraft.backend === 'pocket-tts'
      && requestedDraft.spokenText.trim().length > 0;
    const input = {
      spokenText: requestedDraft.spokenText,
      voice: requestedDraft.voice,
      style: requestedDraft.style,
      lexicon: requestedDraft.lexicon,
    };
    const refresh = async () => {
      try {
        const result = await getLocalNarrationPreparationStatus(input);
        if (!active || !narrationPreferencesMatch(draftRef.current, requestedDraft)) return;
        if (result.status === 'error') {
          setPrewarmState('unavailable');
          return;
        }
        if (result.data.state === 'idle' && localNarrationPrewarmState(input) !== 'idle') {
          // The shared queue may still be waiting for a native worker slot.
          // Keep polling until that request is visible to native status.
          setPrewarmState('preparing');
          timer = setTimeout(() => { void refresh(); }, 750);
          return;
        }
        setPrewarmState(result.data.state);
        if (result.data.state === 'preparing') timer = setTimeout(() => { void refresh(); }, 750);
      } catch {
        if (active) setPrewarmState('unavailable');
      }
    };
    setPrewarmState(shouldCheckCache ? 'checking' : 'idle');
    const stopRuntimeReadyListener = shouldCheckCache && typeof window !== 'undefined'
      ? listenForPocketTtsRuntimeReady(window, () => {
        if (!active) return;
        // An earlier attempt can fail before Pocket is installed. When setup
        // finishes, retry only the clip the reader currently has open.
        localNarrationPrewarmer.retry([input]);
        if (timer !== undefined) clearTimeout(timer);
        void refresh();
      })
      : undefined;
    if (shouldCheckCache) {
      timer = setTimeout(() => { void refresh(); }, 100);
    }
    return () => {
      active = false;
      stopRuntimeReadyListener?.();
      if (timer !== undefined) clearTimeout(timer);
    };
  }, [storageKey, draft.backend, draft.spokenText, draft.voice, draft.style, draft.lexicon, voiceConnectionIssue]);

  useEffect(() => {
    if (restoreTimeRef.current === null || !isNativeRuntime() || voiceConnectionIssue) return;
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
  }, [voiceConnectionIssue]);

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
    if (voiceConnectionIssue) { setError(voiceConnectionIssue); return; }
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
      || voiceConnectionIssue
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
  }, [cacheHydrationKey, prewarmState, savedAudioMatchesDraft, reviewBusy, requestBusy, providerRequestBusy, track?.audioUrl, voiceConnectionIssue]);

  const synthesize = async (currentReview: NarrationRequestReview | null = review) => {
    if (!currentReview || synthesisReviewIdRef.current) return;
    if (voiceConnectionIssue) { setError(voiceConnectionIssue); return; }
    if (!pocketStatus?.ready) {
      setError('Set up Pocket TTS on this Mac before generating audio.');
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
      });
      if (!mountedRef.current) return;
      if (result.status === 'error') {
        setError(result.error.message);
        // The native command consumes its reviewed request before checking the
        // provider response. Require a fresh local review before any retry.
        setReview(null);
        return;
      }
      if (cancelRequested.current) {
        setError('Local synthesis was cancelled. The returned audio was discarded; the transcript and previous audio remain available.');
        setReview(null);
        return;
      }
      if (!narrationPreferencesMatch(draftRef.current, draft)) {
        setError('The narration settings changed while audio was being generated. The returned audio was discarded; review the current text before generating again.');
        setReview(null);
        return;
      }
      setLoadedTrack(result.data, currentReview, true, draft);
      setReview(null);
    } catch {
      if (!mountedRef.current) return;
      setError('Pocket TTS could not finish. The transcript and previous audio remain available; try again when ready.');
      setReview(null);
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
      const stopped = await cancelProviderRequest(review.reviewId, {
        cancelLocalPreparation: true,
      });
      if (!mountedRef.current) return;
      setCancelStatus(stopped
        ? 'Cancellation requested. Waiting for local synthesis to stop; no returned audio will be accepted.'
        : 'The request had already finished locally. Its audio will be discarded because you chose Cancel.');
    } catch {
      if (mountedRef.current) setCancelStatus('The local abort could not be confirmed; any returned audio will be discarded.');
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
    if (voiceConnectionIssue) { setError(voiceConnectionIssue); return; }
    if (track && savedAudioMatchesDraft && player?.src) {
      try { await player.play(); } catch { if (mountedRef.current) setError('Audio playback could not start on this device. The transcript remains available.'); }
      return;
    }
    if (!pocketStatus?.ready) { pendingListen.current = true; openVoiceConnections(); return; }
    const currentReview = review ?? await makeReview();
    if (!currentReview) return;
    if (currentReview.cachedAudioAvailable) await loadCache(currentReview, true);
    else if (mountedRef.current) await synthesize(currentReview);
  };

  const listenStarted = useRef(false);
  useEffect(() => {
    if (voiceConnectionIssue || !autoPlay || listenStarted.current || !pocketStatus) return;
    listenStarted.current = true;
    void playOrPause();
  }, [autoPlay, pocketStatus, voiceConnectionIssue]);

  useEffect(() => {
    if (voiceConnectionIssue || !pendingListen.current || !pocketStatus?.ready) return;
    pendingListen.current = false;
    void playOrPause();
  }, [connectionVersion, pocketStatus, voiceConnectionIssue]);

  const stop = () => {
    // Stop is an explicit position choice. It supersedes a saved position
    // waiting for loadedmetadata, which would otherwise jump the player back.
    restoreTimeRef.current = null;
    const player = audioRef.current;
    if (player) {
      player.pause();
      player.currentTime = 0;
    }
    setCurrentTime(0);
    persistPlaybackPosition(0);
  };

  const replay = async () => {
    if (voiceConnectionIssue) { setError(voiceConnectionIssue); return; }
    // Replay always starts at zero, including while a restored position is
    // still waiting for the audio metadata event.
    restoreTimeRef.current = null;
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
    // A deliberate seek takes precedence over any not-yet-applied restore.
    restoreTimeRef.current = null;
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
      // An unloaded element or a pending restore must not erase a saved time.
      const position=playbackPositionToPersist(player,restoreTimeRef.current!==null);
      if(mountedRef.current&&player&&position!==undefined){
        persistPlaybackPosition(position);
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
        onPlay={() => { completionArmed.current = true; setIsPlaying(true); onPlaybackStarted?.(); window.dispatchEvent(new CustomEvent('code-atlas:narration-playing', { detail: titleId })); }}
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
        <span className={`walkthrough-transcript-state${isPlaying ? ' is-playing' : track && savedAudioMatchesDraft && currentTime <= 0 ? ' is-ready' : track ? '' : prewarmState === 'checking' || prewarmState === 'preparing' || providerRequestBusy ? ' is-preparing' : prewarmState === 'ready' ? ' is-ready' : prewarmState === 'unavailable' ? ' is-unavailable' : ''}`} role="status"><Volume2 size={13} /> {isPlaying ? 'Playing' : providerRequestBusy ? 'Preparing audio' : track && savedAudioMatchesDraft && currentTime <= 0 ? narrationAvailabilityLabel('ready') : track ? 'Paused' : localAudioNeedsPreparation ? 'First listen creates audio' : narrationAvailabilityLabel(prewarmState)}</span>
      </div>

      <details className="walkthrough-narration-script" onToggle={(event) => { if (event.currentTarget.open) { const editor = event.currentTarget.parentElement?.querySelector<HTMLDetailsElement>(':scope > .walkthrough-narration-editor'); if (editor?.open) editor.open = false; } }}>
      <summary>Read narration</summary>
      <div className="walkthrough-transcript-scroll" ref={transcriptRef} onScroll={(event) => onTranscriptPositionChange(event.currentTarget.scrollTop)} role="group" tabIndex={0} aria-label={`Narration for ${plainInlineCodeText(section.title)}`}>
        <div className="walkthrough-caption">
          <p>{captionText.trim() ? <InlineCodeText text={captionText}/> : <em>No narration text is saved for this section. The written explanation and source evidence remain available.</em>}</p>
        </div>
      </div>
      </details>

      <details className="walkthrough-narration-editor" onToggle={(event) => { if (event.currentTarget.open) { const script = event.currentTarget.parentElement?.querySelector<HTMLDetailsElement>(':scope > .walkthrough-narration-script'); if (script?.open) script.open = false; } }}>
        <summary><span>Edit spoken text</span><ChevronDown size={13} /></summary>
        <div className="walkthrough-narration-editor-content">
          <label className="walkthrough-narration-field">Spoken text
            <AppTextArea value={draft.spokenText} disabled={reviewBusy || requestBusy} maxLength={16000} rows={4} onValueChange={(spokenText) => updateDraft((value) => ({ ...value, spokenText }))} placeholder="Write what you want the reader to say in natural language." />
            {spokenTextUtf8Bytes > 16000 && <small role="alert">Shorten this script before listening.</small>}
          </label>
          <details className="walkthrough-pronunciation">
            <summary>Pronunciation lexicon <span>Optional · off by default</span></summary>
            <p>Add a code name and how it should sound. Include it only when you want matching text in the script replaced for speech.</p>
            {draft.lexicon.map((entry, index) => (
              <div className="walkthrough-pronunciation-row" key={`lexicon-${index}`}>
                <div className="walkthrough-pronunciation-field"><label htmlFor={`${titleId}-lexicon-name-${index}`}>Exact name</label><AppTextField id={`${titleId}-lexicon-name-${index}`} className="walkthrough-pronunciation-text" value={entry.name} disabled={reviewBusy || requestBusy} maxLength={120} onValueChange={(name) => updateDraft((value) => ({ ...value, lexicon: replaceLexiconEntry(value.lexicon, index, { ...entry, name }) }))} placeholder="parseRoutes" /></div>
                <div className="walkthrough-pronunciation-field"><label htmlFor={`${titleId}-lexicon-say-${index}`}>Say it as</label><AppTextField id={`${titleId}-lexicon-say-${index}`} className="walkthrough-pronunciation-text" value={entry.sayAs} disabled={reviewBusy || requestBusy} maxLength={160} onValueChange={(sayAs) => updateDraft((value) => ({ ...value, lexicon: replaceLexiconEntry(value.lexicon, index, { ...entry, sayAs }) }))} placeholder="parse routes" /></div>
                <label className="walkthrough-pronunciation-include"><AppCheckbox disabled={reviewBusy || requestBusy} checked={entry.include} onChange={(event) => updateDraft((value) => ({ ...value, lexicon: replaceLexiconEntry(value.lexicon, index, { ...entry, include: event.target.checked }) }))} /> Include</label>
                <AppButton intent="custom" type="button" className="walkthrough-pronunciation-remove" disabled={reviewBusy || requestBusy} onClick={() => updateDraft((value) => ({ ...value, lexicon: value.lexicon.filter((_, itemIndex) => itemIndex !== index) }))} aria-label="Remove pronunciation entry"><X size={13} /></AppButton>
              </div>
            ))}
            {draft.lexicon.length < 12 && <AppButton intent="custom" type="button" className="walkthrough-add-pronunciation" disabled={reviewBusy || requestBusy} onClick={() => updateDraft((value) => ({ ...value, lexicon: [...value.lexicon, { name: '', sayAs: '', include: false }] }))}>Add a name</AppButton>}
          </details>
        </div>
      </details>

      <div className="walkthrough-playback">
        <div className="walkthrough-playback-buttons">
          <AppButton intent="custom" type="button" disabled={reviewBusy || requestBusy || !draft.spokenText.trim() || Boolean(voiceConnectionIssue)} onClick={() => { void replay(); }} aria-label="Replay narration"><RotateCcw size={14} /></AppButton>
          <AppButton intent="custom" type="button" className="walkthrough-play-control" disabled={reviewBusy || requestBusy || !draft.spokenText.trim() || (!isPlaying && Boolean(voiceConnectionIssue))} onClick={() => { void playOrPause(); }} aria-label={isPlaying ? 'Pause narration' : localAudioNeedsPreparation ? 'Create narration and play' : 'Play narration'}>
            {isPlaying ? <Pause size={14} /> : <Play size={14} />} {requestBusy || reviewBusy ? 'Preparing audio…' : isPlaying ? 'Pause' : localAudioNeedsPreparation ? 'Create & play' : 'Play'}
          </AppButton>
          <AppButton intent="custom" type="button" disabled={reviewBusy || requestBusy || (!track && !review)} onClick={stop} aria-label="Stop narration"><Square size={12} /></AppButton>
          <span className="walkthrough-duration" role="group" aria-label={`Elapsed ${formatTime(currentTime)}, duration ${duration > 0 ? narrationDurationLabel(duration) : 'loading'}`}>
            {formatTime(currentTime)} / {narrationDurationLabel(duration)}
          </span>
        </div>
        <div className="walkthrough-seek-label">
          <span>Audio position</span>
          <AppSlider ariaLabel="Audio position" ariaValueText={formatTime(Math.min(currentTime, duration || 0))} min={0} max={duration || 1} step={0.05} value={[Math.min(currentTime, duration || 0)]} disabled={!track || !duration} onValueChange={([value]) => { if (value !== undefined) seek(value); }} />
        </div>
      </div>
      {providerRequestBusy && <div className="walkthrough-narration-cancel">
        <AppButton intent="secondary" type="button" disabled={cancelBusy} onClick={() => setCancelConfirmation(true)}>Cancel audio</AppButton>
        {cancelStatus && <p role="status">{cancelStatus}</p>}
        {cancelConfirmation && <div className="provider-cancel-confirm" role="group" aria-labelledby="narration-cancel-title" aria-describedby="narration-cancel-description">
          <strong id="narration-cancel-title">Cancel local narration?</strong>
          <p id="narration-cancel-description">Pocket TTS will stop preparing this clip. Any incomplete audio will be discarded; the transcript and previous audio remain available.</p>
          <div>
            <AppButton intent="secondary" type="button" onClick={() => setCancelConfirmation(false)}>Keep waiting</AppButton>
            <AppButton intent="danger" type="button" disabled={cancelBusy} onClick={() => { void confirmCancelSynthesis(); }}>{cancelBusy ? 'Stopping…' : 'Cancel audio'}</AppButton>
          </div>
        </div>}
      </div>}
      {track && !savedAudioMatchesDraft && <p className="walkthrough-narration-prior-audio" role="status">Previous audio is kept for its reviewed transcript. New edits do not change it unless you generate another recording.</p>}
      {preferenceWarning && <p className="walkthrough-narration-note" role="status">{preferenceWarning}</p>}
      {voiceConnectionIssue && <p className="walkthrough-narration-note" role="status">{voiceConnectionIssue} <AppButton intent="quiet" type="button" onClick={openVoiceConnections}>Open Connections</AppButton></p>}
      {informationError && <p className="walkthrough-narration-note" role="status">{informationError}</p>}
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
