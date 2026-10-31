//! Reviewed per-section narration using local Pocket TTS or explicitly selected Gemini.
//!
//! Pocket synthesis keeps narration text on this machine. A confirmed Gemini
//! request sends only the edited transcript and pronunciation substitutions;
//! source files, evidence excerpts, and paths are not attached to either request.

use crate::provider::{
    ProviderCommandResult, ProviderFailure, ProviderFailureKind, is_region_restriction_error,
    load_api_key,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::future::Future;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{oneshot, watch};
use uuid::Uuid;

const MODEL_ID: &str = "gemini-3.8-flash-lite-tts";
const ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/interactions";
const CACHE_VERSION: &str = "gemini-narration-cache-v1";
const POCKET_CACHE_VERSION: &str = "pocket-narration-cache-v1";
const DISCLOSURE_VERSION: &str = "gemini-tts-disclosure-2026-10-13-v2";
const LOCAL_DISCLOSURE_VERSION: &str = "pocket-narration-2026-10-15-v1";
const REVIEW_LIFETIME_MS: u64 = 15 * 60 * 1_000;
const MAX_PENDING_REVIEWS: usize = 16;
const MAX_SPOKEN_TEXT_BYTES: usize = 16_000;
const MAX_TTS_REQUEST_TEXT_BYTES: usize = 4_000;
const MAX_STYLE_BYTES: usize = 160;
const MAX_LEXICON_ENTRIES: usize = 12;
const MAX_RESPONSE_BYTES: usize = 12 * 1024 * 1024;
// Local speech is free to recreate, but distinct scripts must not grow app
// storage indefinitely. This keeps a recent runway for several Guides.
const MAX_POCKET_CACHE_FILES: usize = 256;
const MAX_POCKET_CACHE_BYTES: u64 = 256 * 1024 * 1024;
const ABANDONED_CACHE_TEMP_LIFETIME: Duration = Duration::from_secs(24 * 60 * 60);
// A five-slide Guide buffer can expose up to eight visual narration scenes per
// slide. Keep room for that buffer plus one refill slide so later scenes are
// not marked unavailable before the serial local worker reaches them.
const MAX_PREWARM_JOBS: usize = 48;
const MAX_PREWARM_STATUS_ENTRIES: usize = 128;
const POCKET_SEGMENT_POLICY: &str = "utf8-4000-sentence-word-v1-pause200ms";

const PREBUILT_VOICES: &[&str] = &[
    "Zephyr",
    "Puck",
    "Charon",
    "Kore",
    "Fenrir",
    "Leda",
    "Orus",
    "Aoede",
    "Callirrhoe",
    "Autonoe",
    "Enceladus",
    "Iapetus",
    "Umbriel",
    "Algieba",
    "Despina",
    "Erinome",
    "Algenib",
    "Rasalgethi",
    "Laomedeia",
    "Achernar",
    "Alnilam",
    "Schedar",
    "Gacrux",
    "Pulcherrima",
    "Achird",
    "Zubenelgenubi",
    "Vindemiatrix",
    "Sadachbia",
    "Sadaltager",
    "Sulafat",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrationDisclosure {
    pub version: String,
    pub checked_date: String,
    pub provider: String,
    pub model_id: String,
    pub model_name: String,
    pub free_tier_terms: String,
    pub paid_tier_terms: String,
    pub data_use_terms: String,
    pub paid_data_use_terms: String,
    pub regional_data_use_terms: String,
    pub eligibility_terms: String,
    pub billing_uncertainty: String,
    pub terms_url: String,
    pub pricing_url: String,
    pub regions_url: String,
    pub api_key_url: String,
    pub request_scope: String,
    pub voices: Vec<String>,
}

pub fn disclosure() -> NarrationDisclosure {
    let provider_disclosure = crate::provider::disclosure();
    NarrationDisclosure {
        version: DISCLOSURE_VERSION.to_owned(),
        checked_date: "2026-10-13".to_owned(),
        provider: "Google Gemini Developer API".to_owned(),
        model_id: MODEL_ID.to_owned(),
        model_name: "Gemini 3.8 Flash-Lite TTS".to_owned(),
        free_tier_terms: "Google lists standard text and audio output for this model as free of charge on the Free Tier, subject to the API key project's current quotas and availability. Code Atlas cannot inspect the key's project tier or remaining quota.".to_owned(),
        paid_tier_terms: "If this key belongs to a paid project, Google lists $0.50 per million input text tokens and $6.00 per million output audio tokens through 31 December 2026 ($0.0015 per 10 seconds of generated audio), then $1.00 and $12.00 per million tokens from 1 January 2027. Code Atlas never changes the project tier or falls back to another model.".to_owned(),
        data_use_terms: provider_disclosure.data_use_terms,
        paid_data_use_terms: "For Paid Services, Google says it does not use prompts or responses to improve its products and processes them under its Data Processing Addendum. It may log them for a limited period solely to detect and prevent prohibited-use violations, maintain service safety and security, and make required legal or regulatory disclosures.".to_owned(),
        regional_data_use_terms: "If you are in the European Economic Area, Switzerland, or the United Kingdom, Google says the Paid Services data-use terms apply to all Gemini API services, including unpaid quota. Check Google's current terms for your account region.".to_owned(),
        eligibility_terms: "Google's terms require API users to be at least 18, use the service for professional or business development rather than consumer use, and access it only in supported regions. If you make an API client available to users in the EEA, Switzerland, or the UK, the terms require Paid Services.".to_owned(),
        billing_uncertainty: "Code Atlas cannot inspect this key's project tier, active billing account, or remaining quota. Google treats Gemini API access through a Cloud project with an active billing account as a Paid Service. Check the project's tier and billing in Google AI Studio before sending; a Free Tier model price does not guarantee that this key's request is free or available.".to_owned(),
        terms_url: "https://ai.google.dev/gemini-api/terms".to_owned(),
        pricing_url: "https://ai.google.dev/gemini-api/docs/pricing".to_owned(),
        regions_url: "https://ai.google.dev/gemini-api/docs/available-regions".to_owned(),
        api_key_url: "https://aistudio.google.com/app/apikey".to_owned(),
        request_scope: "One selected section's exact narration script, optional pronunciation substitutions, the selected prebuilt voice, delivery style, and WAV format. Scripts over 4,000 UTF-8 bytes are split at sentence boundaries where possible and otherwise at whole-word boundaries, then sent as sequential requests. No source excerpt or whole-chapter context is included.".to_owned(),
        voices: PREBUILT_VOICES.iter().map(|voice| (*voice).to_owned()).collect(),
    }
}

/// Local narration has no cloud key, billing tier, or remote script disclosure.
pub fn local_disclosure() -> NarrationDisclosure {
    NarrationDisclosure {
        version: LOCAL_DISCLOSURE_VERSION.to_owned(),
        checked_date: "2026-10-15".to_owned(),
        provider: "Pocket TTS · on this Mac".to_owned(),
        model_id: crate::pocket_tts::MODEL_ID.to_owned(),
        model_name: "Pocket TTS".to_owned(),
        free_tier_terms: "Local synthesis has no provider usage fee.".to_owned(),
        paid_tier_terms: String::new(),
        data_use_terms: "Setup downloads the pinned runtime, model and voice. Once ready, narration uses local assets; your script is not sent to a cloud provider.".to_owned(),
        paid_data_use_terms: String::new(),
        regional_data_use_terms: String::new(),
        eligibility_terms: "Pocket TTS code/model: MIT. Alba voice: Alba MacKenna, CC BY 4.0, distributed by Kyutai.".to_owned(),
        billing_uncertainty: String::new(),
        terms_url: "https://github.com/kyutai-labs/pocket-tts".to_owned(),
        pricing_url: String::new(),
        regions_url: String::new(),
        api_key_url: String::new(),
        request_scope: "Only this section's edited script and enabled pronunciations are synthesized locally. The Alba voice is fixed; style prompting is unsupported. No source files or cloud fallback are used.".to_owned(),
        voices: vec![crate::pocket_tts::VOICE.to_owned()],
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PronunciationEntry {
    pub name: String,
    pub say_as: String,
    pub include: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildNarrationReviewInput {
    pub spoken_text: String,
    pub voice: String,
    pub style: String,
    pub lexicon: Vec<PronunciationEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrationRequestReview {
    pub review_id: String,
    pub exact_text: String,
    pub request_segments: Vec<String>,
    pub spoken_text_sha256: String,
    pub disclosure_version: String,
    pub model_id: String,
    pub voice: String,
    pub style: String,
    pub format: String,
    pub fingerprint: String,
    pub cache_key: String,
    pub cached_audio_available: bool,
    pub expires_at_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrationConsent {
    pub terms_and_request_acknowledged: bool,
}

impl NarrationConsent {
    fn is_complete(&self) -> bool {
        self.terms_and_request_acknowledged
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrationAudio {
    pub fingerprint: String,
    pub mime_type: String,
    pub audio_base64: String,
    pub cache_hit: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NarrationPrewarmState {
    Idle,
    Preparing,
    Ready,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NarrationPrewarmStatus {
    pub state: NarrationPrewarmState,
    pub cached_audio_available: bool,
}

#[derive(Clone, Default)]
pub struct NarrationPrewarmQueue(Arc<Mutex<PrewarmQueueState>>);

#[derive(Default)]
struct PrewarmQueueState {
    closed: bool,
    worker_running: bool,
    pending: VecDeque<String>,
    entries: HashMap<String, PrewarmQueueEntry>,
}

type PrewarmWork = Pin<Box<dyn Future<Output = bool> + Send + 'static>>;

struct PrewarmQueueEntry {
    state: NarrationPrewarmState,
    cancellation: Option<oneshot::Sender<()>>,
    updates: watch::Sender<NarrationPrewarmState>,
    work: Option<PrewarmWork>,
}

struct PrewarmSchedule {
    state: NarrationPrewarmState,
    accepted: bool,
}

impl NarrationPrewarmQueue {
    fn schedule<F, Fut>(
        &self,
        cache_key: String,
        work: F,
    ) -> Result<PrewarmSchedule, ProviderFailure>
    where
        F: FnOnce(oneshot::Receiver<()>) -> Fut + Send + 'static,
        Fut: Future<Output = bool> + Send + 'static,
    {
        let (cancel_sender, cancel_receiver) = oneshot::channel();
        let (updates, _) = watch::channel(NarrationPrewarmState::Preparing);
        let work: PrewarmWork = Box::pin(work(cancel_receiver));
        let mut start_worker = false;
        {
            let mut queue = self.0.lock().map_err(|_| {
                failure(
                    ProviderFailureKind::ReviewUnavailable,
                    "Background narration is unavailable. The written explanation remains ready.",
                )
            })?;
            if queue.closed {
                return Ok(PrewarmSchedule {
                    state: NarrationPrewarmState::Unavailable,
                    accepted: false,
                });
            }
            if let Some(existing) = queue.entries.get(&cache_key) {
                if existing.state == NarrationPrewarmState::Preparing {
                    return Ok(PrewarmSchedule {
                        state: existing.state,
                        accepted: false,
                    });
                }
            }
            queue.entries.remove(&cache_key);
            let pending = queue
                .entries
                .values()
                .filter(|entry| entry.state == NarrationPrewarmState::Preparing)
                .count();
            if pending >= MAX_PREWARM_JOBS {
                remember_prewarm_state(&mut queue, &cache_key, NarrationPrewarmState::Unavailable);
                return Ok(PrewarmSchedule {
                    state: NarrationPrewarmState::Unavailable,
                    accepted: false,
                });
            }
            make_prewarm_status_room(&mut queue);
            queue.entries.insert(
                cache_key.clone(),
                PrewarmQueueEntry {
                    state: NarrationPrewarmState::Preparing,
                    cancellation: Some(cancel_sender),
                    updates,
                    work: Some(work),
                },
            );
            queue.pending.push_back(cache_key.clone());
            if !queue.worker_running {
                queue.worker_running = true;
                start_worker = true;
            }
        }

        if start_worker {
            let queue = self.clone();
            tauri::async_runtime::spawn(async move { queue.run_pending().await });
        }
        Ok(PrewarmSchedule {
            state: NarrationPrewarmState::Preparing,
            accepted: true,
        })
    }

    async fn run_pending(&self) {
        loop {
            let next = {
                let Ok(mut queue) = self.0.lock() else {
                    return;
                };
                if queue.closed {
                    queue.worker_running = false;
                    return;
                }
                loop {
                    let Some(cache_key) = queue.pending.pop_front() else {
                        queue.worker_running = false;
                        return;
                    };
                    let Some(entry) = queue.entries.get_mut(&cache_key) else {
                        continue;
                    };
                    if entry.state != NarrationPrewarmState::Preparing {
                        continue;
                    }
                    if let Some(work) = entry.work.take() {
                        break (cache_key, work);
                    }
                }
            };
            let (cache_key, work) = next;
            let ready = matches!(tauri::async_runtime::spawn(work).await, Ok(true));
            let state = if ready {
                NarrationPrewarmState::Ready
            } else {
                NarrationPrewarmState::Unavailable
            };
            self.finish(&cache_key, state);
        }
    }

    fn status(&self, cache_key: &str) -> Option<NarrationPrewarmState> {
        self.0
            .lock()
            .ok()
            .and_then(|queue| queue.entries.get(cache_key).map(|entry| entry.state))
    }

    /// Remove a stale request only while it is still waiting for the native
    /// worker. Work already dequeued remains cacheable under its original key.
    fn cancel_pending(&self, cache_key: &str) -> bool {
        let mut cancellation = None;
        let mut work = None;
        let cancelled = {
            let Ok(mut queue) = self.0.lock() else {
                return false;
            };
            let Some(position) = queue.pending.iter().position(|key| key == cache_key) else {
                return false;
            };
            queue.pending.remove(position);
            if let Some(entry) = queue.entries.get_mut(cache_key)
                && entry.state == NarrationPrewarmState::Preparing
            {
                entry.state = NarrationPrewarmState::Unavailable;
                entry
                    .updates
                    .send_replace(NarrationPrewarmState::Unavailable);
                cancellation = entry.cancellation.take();
                work = entry.work.take();
            }
            true
        };
        drop(work);
        if let Some(cancellation) = cancellation {
            let _ = cancellation.send(());
        }
        cancelled
    }

    /// Stop synthesis for an explicitly cancelled playback request. Ordinary
    /// navigation cancellation leaves useful background prewarming alone.
    fn cancel_preparation(&self, cache_key: &str) -> bool {
        let mut cancellation = None;
        let mut work = None;
        let cancelled = {
            let Ok(mut queue) = self.0.lock() else {
                return false;
            };
            if !queue
                .entries
                .get(cache_key)
                .is_some_and(|entry| entry.state == NarrationPrewarmState::Preparing)
            {
                return false;
            }
            if let Some(position) = queue.pending.iter().position(|key| key == cache_key) {
                queue.pending.remove(position);
            }
            let Some(entry) = queue.entries.get_mut(cache_key) else {
                return false;
            };
            entry.state = NarrationPrewarmState::Unavailable;
            entry
                .updates
                .send_replace(NarrationPrewarmState::Unavailable);
            cancellation = entry.cancellation.take();
            work = entry.work.take();
            true
        };
        drop(work);
        if let Some(cancellation) = cancellation {
            let _ = cancellation.send(());
        }
        cancelled
    }

    fn finish(&self, cache_key: &str, state: NarrationPrewarmState) {
        if let Ok(mut queue) = self.0.lock() {
            if let Some(entry) = queue.entries.get_mut(cache_key) {
                if entry.state == NarrationPrewarmState::Preparing {
                    entry.state = state;
                    entry.cancellation.take();
                    entry.updates.send_replace(state);
                }
            }
        }
    }

    async fn wait_for(
        &self,
        cache_key: &str,
        cancellation: &mut oneshot::Receiver<()>,
    ) -> Result<Option<NarrationPrewarmState>, ProviderFailure> {
        let mut updates = {
            let queue = self.0.lock().map_err(|_| {
                failure(
                    ProviderFailureKind::ReviewUnavailable,
                    "The narration status is unavailable. The on-demand audio path remains available.",
                )
            })?;
            let Some(entry) = queue.entries.get(cache_key) else {
                return Ok(None);
            };
            if entry.state != NarrationPrewarmState::Preparing {
                return Ok(Some(entry.state));
            }
            entry.updates.subscribe()
        };

        loop {
            let state = *updates.borrow_and_update();
            if state != NarrationPrewarmState::Preparing {
                check_audio_request_cancellation(cancellation)?;
                return Ok(Some(state));
            }
            tokio::select! {
                biased;
                _ = &mut *cancellation => return Err(failure(
                    ProviderFailureKind::RequestCancelled,
                    "The audio wait was cancelled. The transcript and any completed cached audio remain available.",
                )),
                result = updates.changed() => {
                    if result.is_err() {
                        return Ok(Some(NarrationPrewarmState::Unavailable));
                    }
                }
            }
        }
    }

    fn prioritize(&self, cache_key: &str) {
        if let Ok(mut queue) = self.0.lock()
            && let Some(position) = queue.pending.iter().position(|key| key == cache_key)
            && let Some(key) = queue.pending.remove(position)
        {
            queue.pending.push_front(key);
        }
    }

    pub fn shutdown(&self) {
        let mut cancellations = Vec::new();
        if let Ok(mut queue) = self.0.lock() {
            queue.closed = true;
            queue.pending.clear();
            for entry in queue.entries.values_mut() {
                if entry.state == NarrationPrewarmState::Preparing {
                    entry.state = NarrationPrewarmState::Unavailable;
                    entry
                        .updates
                        .send_replace(NarrationPrewarmState::Unavailable);
                    if let Some(cancellation) = entry.cancellation.take() {
                        cancellations.push(cancellation);
                    }
                    entry.work.take();
                }
            }
        }
        for cancellation in cancellations {
            let _ = cancellation.send(());
        }
    }
}

fn make_prewarm_status_room(queue: &mut PrewarmQueueState) {
    while queue.entries.len() >= MAX_PREWARM_STATUS_ENTRIES {
        let completed = queue
            .entries
            .iter()
            .find(|(_, entry)| entry.state != NarrationPrewarmState::Preparing)
            .map(|(key, _)| key.clone());
        let Some(completed) = completed else {
            break;
        };
        queue.entries.remove(&completed);
    }
}

fn remember_prewarm_state(
    queue: &mut PrewarmQueueState,
    cache_key: &str,
    state: NarrationPrewarmState,
) {
    make_prewarm_status_room(queue);
    let (updates, _) = watch::channel(state);
    queue.entries.insert(
        cache_key.to_owned(),
        PrewarmQueueEntry {
            state,
            cancellation: None,
            updates,
            work: None,
        },
    );
}

#[derive(Clone)]
struct PendingNarrationReview {
    review: NarrationRequestReview,
}

#[derive(Clone, Default)]
pub struct PendingNarrationReviews(Arc<Mutex<HashMap<String, PendingNarrationReview>>>);

impl PendingNarrationReviews {
    fn insert(&self, pending: PendingNarrationReview) -> Result<(), ProviderFailure> {
        let mut reviews = self.0.lock().map_err(|_| {
            failure(
                ProviderFailureKind::ReviewUnavailable,
                "The narration review store is temporarily unavailable. No request was sent.",
            )
        })?;
        reviews.retain(|_, review| review.review.expires_at_unix_ms > now_unix_ms());
        if reviews.len() >= MAX_PENDING_REVIEWS {
            return Err(failure(
                ProviderFailureKind::ReviewUnavailable,
                "Too many narration reviews are open. Close an older review and try again.",
            ));
        }
        reviews.insert(pending.review.review_id.clone(), pending);
        Ok(())
    }

    fn consume(
        &self,
        review_id: &str,
        fingerprint: &str,
    ) -> Result<PendingNarrationReview, ProviderFailure> {
        let mut reviews = self.0.lock().map_err(|_| {
            failure(
                ProviderFailureKind::ReviewUnavailable,
                "The narration review store is temporarily unavailable. No request was sent.",
            )
        })?;
        let pending = reviews.remove(review_id).ok_or_else(|| {
            failure(ProviderFailureKind::ReviewUnavailable, "The narration review is no longer available. Review the exact text again before sending.")
        })?;
        if pending.review.expires_at_unix_ms <= now_unix_ms() {
            return Err(failure(
                ProviderFailureKind::ReviewExpired,
                "The narration review expired. Review the exact text again before sending.",
            ));
        }
        if pending.review.fingerprint != fingerprint {
            return Err(failure(
                ProviderFailureKind::ReviewChanged,
                "The narration review fingerprint changed. Review the exact text and settings again before sending.",
            ));
        }
        Ok(pending)
    }

    fn get(
        &self,
        review_id: &str,
        fingerprint: &str,
    ) -> Result<PendingNarrationReview, ProviderFailure> {
        let reviews = self.0.lock().map_err(|_| {
            failure(
                ProviderFailureKind::ReviewUnavailable,
                "The narration review store is temporarily unavailable. No request was sent.",
            )
        })?;
        let pending = reviews.get(review_id).cloned().ok_or_else(|| {
            failure(ProviderFailureKind::ReviewUnavailable, "The narration review is no longer available. Review the exact text again before sending.")
        })?;
        if pending.review.expires_at_unix_ms <= now_unix_ms() {
            return Err(failure(
                ProviderFailureKind::ReviewExpired,
                "The narration review expired. Review the exact text again before sending.",
            ));
        }
        if pending.review.fingerprint != fingerprint {
            return Err(failure(
                ProviderFailureKind::ReviewChanged,
                "The narration review fingerprint changed. Review the exact text and settings again before sending.",
            ));
        }
        Ok(pending)
    }

    fn cache_key_for_cancellation(&self, review_id: &str) -> Option<String> {
        self.0
            .lock()
            .ok()?
            .get(review_id)
            .map(|pending| pending.review.cache_key.clone())
    }

    pub fn remove(&self, review_id: &str) {
        if let Ok(mut reviews) = self.0.lock() {
            reviews.remove(review_id);
        }
    }
}

pub fn build_request_review(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    input: BuildNarrationReviewInput,
) -> ProviderCommandResult<NarrationRequestReview> {
    build_review_for_backend(storage_dir, reviews, input, false)
}

pub fn build_local_request_review(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    input: BuildNarrationReviewInput,
) -> ProviderCommandResult<NarrationRequestReview> {
    build_review_for_backend(storage_dir, reviews, input, true)
}

fn build_review_for_backend(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    input: BuildNarrationReviewInput,
    local: bool,
) -> ProviderCommandResult<NarrationRequestReview> {
    let review = match create_review(storage_dir, input, local) {
        Ok(review) => review,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    match reviews.insert(PendingNarrationReview {
        review: review.clone(),
    }) {
        Ok(()) => ProviderCommandResult::ok(review),
        Err(error) => ProviderCommandResult::failed(error),
    }
}

fn create_review(
    storage_dir: &Path,
    input: BuildNarrationReviewInput,
    local: bool,
) -> Result<NarrationRequestReview, ProviderFailure> {
    let input = validate_input_for_backend(input, local)?;
    let model_id = if local {
        crate::pocket_tts::MODEL_ID
    } else {
        MODEL_ID
    };
    let disclosure_version = if local {
        LOCAL_DISCLOSURE_VERSION
    } else {
        DISCLOSURE_VERSION
    };
    let exact_text = apply_lexicon(&input.spoken_text, &input.lexicon);
    if exact_text.trim().is_empty() || exact_text.len() > MAX_SPOKEN_TEXT_BYTES {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Pronunciation substitutions must leave this section's narration between 1 and 16,000 UTF-8 bytes.",
        ));
    }
    let request_segments = split_spoken_text(&exact_text, MAX_TTS_REQUEST_TEXT_BYTES);
    let spoken_text_sha256 = sha256(exact_text.as_bytes());
    let format = "audio/wav".to_owned();
    let cache_key = audio_cache_key(
        &spoken_text_sha256,
        &request_segments,
        model_id,
        &input.voice,
        &input.style,
        &format,
        crate::pocket_tts::runtime_fingerprint,
        POCKET_SEGMENT_POLICY,
    );
    let fingerprint = request_fingerprint(&cache_key, disclosure_version, &request_segments);
    let cached_audio_available = read_cached_audio(storage_dir, &cache_key, model_id).is_some();
    let expires_at_unix_ms = now_unix_ms().saturating_add(REVIEW_LIFETIME_MS);
    Ok(NarrationRequestReview {
        review_id: Uuid::new_v4().to_string(),
        exact_text,
        request_segments,
        spoken_text_sha256,
        disclosure_version: disclosure_version.to_owned(),
        model_id: model_id.to_owned(),
        voice: input.voice,
        style: input.style,
        format,
        fingerprint,
        cache_key,
        cached_audio_available,
        expires_at_unix_ms,
    })
}

pub fn prewarm_local_audio(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    queue: &NarrationPrewarmQueue,
    input: BuildNarrationReviewInput,
) -> ProviderCommandResult<NarrationPrewarmStatus> {
    let review = match create_review(storage_dir, input, true) {
        Ok(review) => review,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    if review.cached_audio_available {
        queue.finish(&review.cache_key, NarrationPrewarmState::Ready);
        return ProviderCommandResult::ok(NarrationPrewarmStatus {
            state: NarrationPrewarmState::Ready,
            cached_audio_available: true,
        });
    }
    let cache_key = review.cache_key.clone();
    let review_id = review.review_id.clone();
    let fingerprint = review.fingerprint.clone();
    let task_review = review.clone();
    let storage = storage_dir.to_path_buf();
    let task_reviews = reviews.clone();
    let task_key = cache_key.clone();
    let task_model_id = review.model_id.clone();
    let schedule = queue.schedule(cache_key.clone(), move |mut cancellation| async move {
        if task_reviews
            .insert(PendingNarrationReview {
                review: task_review,
            })
            .is_err()
        {
            return false;
        }
        let consent = NarrationConsent {
            terms_and_request_acknowledged: false,
        };
        matches!(
            synthesize_reviewed_audio(
                &storage,
                &task_reviews,
                &review_id,
                &fingerprint,
                &consent,
                &mut cancellation,
            )
            .await,
            ProviderCommandResult::Ok { .. }
        ) && read_cached_audio(&storage, &task_key, &task_model_id).is_some()
    });
    let schedule = match schedule {
        Ok(schedule) => schedule,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    ProviderCommandResult::ok(NarrationPrewarmStatus {
        state: schedule.state,
        cached_audio_available: schedule.accepted && schedule.state == NarrationPrewarmState::Ready,
    })
}

/// Best-effort cleanup for voice changes: a queued prewarm can be discarded,
/// while audio already in synthesis remains valid for its original fingerprint.
pub fn cancel_pending_local_audio(
    storage_dir: &Path,
    queue: &NarrationPrewarmQueue,
    input: BuildNarrationReviewInput,
) -> bool {
    let Ok(review) = create_review(storage_dir, input, true) else {
        return false;
    };
    queue.cancel_pending(&review.cache_key)
}

pub fn cancel_local_preparation_for_review(
    reviews: &PendingNarrationReviews,
    queue: &NarrationPrewarmQueue,
    review_id: &str,
) -> bool {
    reviews
        .cache_key_for_cancellation(review_id)
        .is_some_and(|cache_key| queue.cancel_preparation(&cache_key))
}

pub fn local_audio_preparation_status(
    storage_dir: &Path,
    queue: &NarrationPrewarmQueue,
    input: BuildNarrationReviewInput,
) -> ProviderCommandResult<NarrationPrewarmStatus> {
    let review = match create_review(storage_dir, input, true) {
        Ok(review) => review,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    if review.cached_audio_available {
        queue.finish(&review.cache_key, NarrationPrewarmState::Ready);
        return ProviderCommandResult::ok(NarrationPrewarmStatus {
            state: NarrationPrewarmState::Ready,
            cached_audio_available: true,
        });
    }
    let state = match queue.status(&review.cache_key) {
        Some(NarrationPrewarmState::Preparing) => NarrationPrewarmState::Preparing,
        Some(NarrationPrewarmState::Idle) | None => NarrationPrewarmState::Idle,
        Some(NarrationPrewarmState::Ready | NarrationPrewarmState::Unavailable) => {
            NarrationPrewarmState::Unavailable
        }
    };
    ProviderCommandResult::ok(NarrationPrewarmStatus {
        state,
        cached_audio_available: false,
    })
}

pub fn discard_request_review(reviews: &PendingNarrationReviews, review_id: &str) {
    reviews.remove(review_id);
}

pub fn load_reviewed_audio(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    review_id: &str,
    fingerprint: &str,
) -> ProviderCommandResult<NarrationAudio> {
    let pending = match reviews.get(review_id, fingerprint) {
        Ok(pending) => pending,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    match read_cached_audio(
        storage_dir,
        &pending.review.cache_key,
        &pending.review.model_id,
    ) {
        Some(bytes) => ProviderCommandResult::ok(audio_result(&pending.review, bytes, true)),
        None => ProviderCommandResult::error(
            ProviderFailureKind::ReviewUnavailable,
            "The reviewed narration has no local audio cache. Review it again before generating audio.",
        ),
    }
}

pub async fn load_prewarmed_audio_if_ready(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    queue: &NarrationPrewarmQueue,
    review_id: &str,
    fingerprint: &str,
    cancellation: &mut oneshot::Receiver<()>,
) -> Option<ProviderCommandResult<NarrationAudio>> {
    let pending = reviews.get(review_id, fingerprint).ok()?;
    if pending.review.model_id != crate::pocket_tts::MODEL_ID {
        return None;
    }
    queue.prioritize(&pending.review.cache_key);
    let state = match queue
        .wait_for(&pending.review.cache_key, cancellation)
        .await
    {
        Ok(Some(state)) => state,
        Ok(None) => return None,
        Err(error) => return Some(ProviderCommandResult::failed(error)),
    };
    if state != NarrationPrewarmState::Ready {
        return None;
    }
    let bytes = read_cached_audio(
        storage_dir,
        &pending.review.cache_key,
        &pending.review.model_id,
    )?;
    if let Err(error) = check_audio_request_cancellation(cancellation) {
        return Some(ProviderCommandResult::failed(error));
    }
    Some(ProviderCommandResult::ok(audio_result(
        &pending.review,
        bytes,
        true,
    )))
}

/// Return a completed local prewarm and release its per-playback review slot.
/// When no prepared result is available, the caller may continue through the
/// ordinary synthesis path, which consumes the review itself.
pub async fn load_prewarmed_audio_for_playback(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    queue: &NarrationPrewarmQueue,
    review_id: &str,
    fingerprint: &str,
    cancellation: &mut oneshot::Receiver<()>,
) -> Option<ProviderCommandResult<NarrationAudio>> {
    let result = load_prewarmed_audio_if_ready(
        storage_dir,
        reviews,
        queue,
        review_id,
        fingerprint,
        cancellation,
    )
    .await;
    if result.is_some() {
        discard_request_review(reviews, review_id);
    }
    result
}

pub async fn synthesize_reviewed_audio(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    review_id: &str,
    fingerprint: &str,
    consent: &NarrationConsent,
    cancellation: &mut oneshot::Receiver<()>,
) -> ProviderCommandResult<NarrationAudio> {
    let local = match reviews.get(review_id, fingerprint) {
        Ok(pending) => pending.review.model_id == crate::pocket_tts::MODEL_ID,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    if !local && !consent.is_complete() {
        return ProviderCommandResult::error(
            ProviderFailureKind::ConsentRequired,
            "Confirm the project tier and billing status, Google's data-use and eligibility terms, and authorization to send this exact narration request.",
        );
    }
    let pending = match consume_review(reviews, review_id, fingerprint) {
        Ok(pending) => pending,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    if let Err(error) = check_audio_request_cancellation(cancellation) {
        return ProviderCommandResult::failed(error);
    }
    if let Some(bytes) = read_cached_audio(
        storage_dir,
        &pending.review.cache_key,
        &pending.review.model_id,
    ) {
        if let Err(error) = check_audio_request_cancellation(cancellation) {
            return ProviderCommandResult::failed(error);
        }
        return ProviderCommandResult::ok(audio_result(&pending.review, bytes, true));
    }
    if pending.review.model_id == crate::pocket_tts::MODEL_ID {
        let bytes = match crate::pocket_tts::synthesize_segments(
            storage_dir,
            &pending.review.request_segments,
            cancellation,
        )
        .await
        {
            Ok(bytes) => bytes,
            Err(error) => return ProviderCommandResult::failed(error),
        };
        if let Err(error) = check_audio_request_cancellation(cancellation) {
            return ProviderCommandResult::failed(error);
        }
        let _ = write_cached_audio(
            storage_dir,
            &pending.review.cache_key,
            &pending.review.model_id,
            &bytes,
        );
        return ProviderCommandResult::ok(audio_result(&pending.review, bytes, false));
    }
    if pending.review.model_id != MODEL_ID {
        return ProviderCommandResult::error(
            ProviderFailureKind::ReviewChanged,
            "This narration backend is unavailable. Review the section again; no fallback was used.",
        );
    }
    let api_key = match load_api_key() {
        Ok(key) => key,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    let base_review = pending.review.clone();
    let request_key = api_key.clone();
    let result = synthesize_segments(&base_review.request_segments, cancellation, |segment| {
        let mut segment_review = base_review.clone();
        segment_review.exact_text = segment;
        let segment_key = request_key.clone();
        async move { request_audio(&segment_review, &segment_key).await }
    })
    .await;
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    if let Err(_error) = write_cached_audio(
        storage_dir,
        &pending.review.cache_key,
        &pending.review.model_id,
        &bytes,
    ) {
        // The successfully generated bytes still reach the player if local cache
        // storage is unavailable; a future replay will require a new request.
    }
    ProviderCommandResult::ok(audio_result(&pending.review, bytes, false))
}

async fn await_cancellable_audio_request<T, F>(
    cancellation: &mut oneshot::Receiver<()>,
    request: F,
) -> Result<T, ProviderFailure>
where
    F: Future<Output = Result<T, ProviderFailure>>,
{
    let result = tokio::select! {
        biased;
        _ = &mut *cancellation => Err(failure(
            ProviderFailureKind::RequestCancelled,
            "The narration request was cancelled locally. Google may already have received the reviewed speech text; no response was accepted. Review and confirm again before another send.",
        )),
        result = request => result,
    };
    if result.is_ok() {
        check_audio_request_cancellation(cancellation)?;
    }
    result
}

fn check_audio_request_cancellation(
    cancellation: &mut oneshot::Receiver<()>,
) -> Result<(), ProviderFailure> {
    if !matches!(
        cancellation.try_recv(),
        Err(oneshot::error::TryRecvError::Empty)
    ) {
        return Err(failure(
            ProviderFailureKind::RequestCancelled,
            "The narration request was cancelled locally. No returned audio was accepted. Review again before generating more audio.",
        ));
    }
    Ok(())
}

async fn synthesize_segments<F, Fut>(
    segments: &[String],
    cancellation: &mut oneshot::Receiver<()>,
    mut request_segment: F,
) -> Result<Vec<u8>, ProviderFailure>
where
    F: FnMut(String) -> Fut,
    Fut: Future<Output = Result<Vec<u8>, ProviderFailure>>,
{
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| segment.is_empty() || segment.len() > MAX_TTS_REQUEST_TEXT_BYTES)
    {
        return Err(failure(
            ProviderFailureKind::ReviewChanged,
            "The reviewed narration segments are invalid. Review the exact text again before sending.",
        ));
    }

    let mut audio_segments = Vec::with_capacity(segments.len());
    let mut total_pcm_bytes = 0_usize;
    for segment in segments {
        let audio =
            await_cancellable_audio_request(cancellation, request_segment(segment.clone())).await?;
        if !is_wav(&audio) {
            return Err(failure(
                ProviderFailureKind::MalformedProviderResponse,
                "The narration engine returned an invalid WAV segment. The transcript and previous audio remain available.",
            ));
        }
        let Some(pcm) = wav_pcm_data(&audio) else {
            return Err(failure(
                ProviderFailureKind::MalformedProviderResponse,
                "The narration engine returned WAV audio with no PCM data. The transcript and previous audio remain available.",
            ));
        };
        total_pcm_bytes = total_pcm_bytes.saturating_add(pcm.len());
        let joined_pcm_bytes =
            total_pcm_bytes.saturating_add(9_600_usize.saturating_mul(audio_segments.len()));
        if 44_usize.saturating_add(joined_pcm_bytes) > MAX_RESPONSE_BYTES {
            return Err(failure(
                ProviderFailureKind::ProviderResponseTooLarge,
                "The combined narration exceeded the local audio size limit. The transcript and previous audio remain available.",
            ));
        }
        audio_segments.push(audio);
    }
    let combined = join_wav_segments(&audio_segments)?;
    check_audio_request_cancellation(cancellation)?;
    Ok(combined)
}

fn consume_review(
    reviews: &PendingNarrationReviews,
    review_id: &str,
    fingerprint: &str,
) -> Result<PendingNarrationReview, ProviderFailure> {
    reviews.consume(review_id, fingerprint)
}

fn audio_result(
    review: &NarrationRequestReview,
    bytes: Vec<u8>,
    cache_hit: bool,
) -> NarrationAudio {
    NarrationAudio {
        fingerprint: review.fingerprint.clone(),
        mime_type: "audio/wav".to_owned(),
        audio_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        cache_hit,
    }
}

fn validate_input_for_backend(
    mut input: BuildNarrationReviewInput,
    local: bool,
) -> Result<BuildNarrationReviewInput, ProviderFailure> {
    input.spoken_text = input.spoken_text.trim().to_owned();
    input.style = input.style.trim().to_owned();
    if input.spoken_text.is_empty() || input.spoken_text.len() > MAX_SPOKEN_TEXT_BYTES {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Narration text must contain 1 to 16,000 UTF-8 bytes for this section. Longer scripts need to be divided into separate sections.",
        ));
    }
    if input.style.len() > MAX_STYLE_BYTES
        || input.style.contains('\n')
        || input.style.contains('\r')
    {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Delivery style must be one line of at most 160 UTF-8 bytes.",
        ));
    }
    if local && (input.voice != crate::pocket_tts::VOICE || !input.style.is_empty()) {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Pocket TTS uses the Alba voice and does not support delivery-style prompts. Review local voice settings again.",
        ));
    }
    if !local && !PREBUILT_VOICES.contains(&input.voice.as_str()) {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Choose a currently listed prebuilt Gemini voice.",
        ));
    }
    if input.lexicon.len() > MAX_LEXICON_ENTRIES {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "A section can include at most 12 pronunciation entries.",
        ));
    }
    let mut names = std::collections::HashSet::new();
    for entry in input.lexicon.iter_mut().filter(|entry| entry.include) {
        entry.name = entry.name.trim().to_owned();
        entry.say_as = entry.say_as.trim().to_owned();
        if entry.name.is_empty()
            || entry.name.len() > 120
            || entry.say_as.is_empty()
            || entry.say_as.len() > 160
            || entry.name.chars().any(char::is_whitespace)
            || !names.insert(entry.name.clone())
        {
            return Err(failure(
                ProviderFailureKind::InvalidReview,
                "Included pronunciation entries need a distinct code name of at most 120 bytes and a spoken form of at most 160 bytes.",
            ));
        }
    }
    Ok(input)
}

fn split_spoken_text(text: &str, max_segment_bytes: usize) -> Vec<String> {
    if text.is_empty() || max_segment_bytes == 0 {
        return Vec::new();
    }

    let mut segments = Vec::new();
    let mut remaining = text;
    while remaining.len() > max_segment_bytes {
        let limit = remaining
            .char_indices()
            .map(|(index, character)| index + character.len_utf8())
            .take_while(|end| *end <= max_segment_bytes)
            .last()
            .unwrap_or_else(|| {
                remaining
                    .chars()
                    .next()
                    .map(char::len_utf8)
                    .unwrap_or(remaining.len())
            });
        let bounded_prefix = &remaining[..limit];
        let sentence_boundary = sentence_boundary_in(bounded_prefix, remaining);
        let word_boundary = bounded_prefix
            .char_indices()
            .rev()
            .find(|(_, character)| character.is_whitespace())
            .map(|(index, character)| index + character.len_utf8());
        let boundary = sentence_boundary
            .or(word_boundary)
            .filter(|boundary| *boundary > 0)
            .unwrap_or(limit);
        segments.push(remaining[..boundary].to_owned());
        remaining = &remaining[boundary..];
    }
    if !remaining.is_empty() {
        segments.push(remaining.to_owned());
    }
    segments
}

fn sentence_boundary_in(prefix: &str, text: &str) -> Option<usize> {
    let mut latest = None;
    for (index, character) in prefix.char_indices() {
        if !matches!(character, '.' | '?' | '!') {
            continue;
        }
        let mut boundary = index + character.len_utf8();
        let punctuation_end = boundary;
        for (next_index, next) in text[punctuation_end..].char_indices() {
            if matches!(next, '\"' | '\'' | ')' | ']' | '}') {
                boundary = punctuation_end + next_index + next.len_utf8();
            } else {
                break;
            }
        }
        let sentence_ends = match text[boundary..].chars().next() {
            None => true,
            Some(next) => next.is_whitespace(),
        };
        if !sentence_ends || boundary > prefix.len() {
            continue;
        }
        while boundary < prefix.len() {
            let Some(next) = prefix[boundary..].chars().next() else {
                break;
            };
            if !next.is_whitespace() {
                break;
            }
            boundary += next.len_utf8();
        }
        latest = Some(boundary);
    }
    latest
}

fn join_wav_segments(segments: &[Vec<u8>]) -> Result<Vec<u8>, ProviderFailure> {
    if segments.is_empty() {
        return Err(failure(
            ProviderFailureKind::MalformedProviderResponse,
            "The narration engine returned no narration segments. The transcript and previous audio remain available.",
        ));
    }
    if segments.len() == 1 {
        let audio = &segments[0];
        if audio.len() > MAX_RESPONSE_BYTES || !is_wav(audio) {
            return Err(failure(
                ProviderFailureKind::MalformedProviderResponse,
                "The narration engine returned invalid or oversized WAV audio. The transcript and previous audio remain available.",
            ));
        }
        return Ok(audio.clone());
    }

    const WAV_HEADER_BYTES: usize = 44;
    const SILENCE_BETWEEN_SEGMENTS_BYTES: usize = 9_600; // 200 ms of 24 kHz mono PCM16.
    let mut pcm_segments = Vec::with_capacity(segments.len());
    let mut total_pcm_bytes = 0_usize;
    for audio in segments {
        if !is_wav(audio) || audio.len() > MAX_RESPONSE_BYTES {
            return Err(failure(
                ProviderFailureKind::MalformedProviderResponse,
                "The narration engine returned an invalid WAV segment. The transcript and previous audio remain available.",
            ));
        }
        let pcm = wav_pcm_data(audio).ok_or_else(|| {
            failure(
                ProviderFailureKind::MalformedProviderResponse,
                "The narration engine returned WAV audio with no PCM data. The transcript and previous audio remain available.",
            )
        })?;
        total_pcm_bytes = total_pcm_bytes.saturating_add(pcm.len());
        pcm_segments.push(pcm);
    }
    total_pcm_bytes = total_pcm_bytes.saturating_add(
        SILENCE_BETWEEN_SEGMENTS_BYTES.saturating_mul(segments.len().saturating_sub(1)),
    );
    let total_bytes = WAV_HEADER_BYTES.saturating_add(total_pcm_bytes);
    if total_bytes > MAX_RESPONSE_BYTES || total_pcm_bytes > u32::MAX as usize - 36 {
        return Err(failure(
            ProviderFailureKind::ProviderResponseTooLarge,
            "The combined narration exceeded the local audio size limit. The transcript and previous audio remain available.",
        ));
    }

    let mut wav = Vec::with_capacity(total_bytes);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&((36 + total_pcm_bytes) as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&24_000_u32.to_le_bytes());
    wav.extend_from_slice(&48_000_u32.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(total_pcm_bytes as u32).to_le_bytes());
    for (index, pcm) in pcm_segments.iter().enumerate() {
        wav.extend_from_slice(pcm);
        if index + 1 < pcm_segments.len() {
            wav.resize(wav.len() + SILENCE_BETWEEN_SEGMENTS_BYTES, 0);
        }
    }
    if !is_wav(&wav) {
        return Err(failure(
            ProviderFailureKind::MalformedProviderResponse,
            "The combined narration could not be validated as WAV audio. The transcript and previous audio remain available.",
        ));
    }
    Ok(wav)
}

fn wav_pcm_data(audio: &[u8]) -> Option<&[u8]> {
    let mut cursor = 12;
    while cursor + 8 <= audio.len() {
        let chunk_id = &audio[cursor..cursor + 4];
        let chunk_size =
            u32::from_le_bytes(audio[cursor + 4..cursor + 8].try_into().ok()?) as usize;
        let data_start = cursor.checked_add(8)?;
        let data_end = data_start.checked_add(chunk_size)?;
        if data_end > audio.len() {
            return None;
        }
        if chunk_id == b"data" {
            return Some(&audio[data_start..data_end]);
        }
        let padded_size = chunk_size.checked_add(chunk_size % 2)?;
        cursor = data_start.checked_add(padded_size)?;
    }
    None
}

fn apply_lexicon(text: &str, lexicon: &[PronunciationEntry]) -> String {
    let mut entries = lexicon
        .iter()
        .filter(|entry| entry.include)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.name.len()));
    let mut result = String::with_capacity(text.len());
    let mut cursor = 0;
    while cursor < text.len() {
        let matching_entry = entries.iter().find(|entry| {
            if !text[cursor..].starts_with(&entry.name) {
                return false;
            }
            let end = cursor + entry.name.len();
            let before_is_identifier = text[..cursor]
                .chars()
                .next_back()
                .is_some_and(is_identifier_char);
            let after_is_identifier = text[end..].chars().next().is_some_and(is_identifier_char);
            !before_is_identifier && !after_is_identifier
        });
        if let Some(entry) = matching_entry {
            result.push_str(&entry.say_as);
            cursor += entry.name.len();
        } else {
            let Some(character) = text[cursor..].chars().next() else {
                break;
            };
            result.push(character);
            cursor += character.len_utf8();
        }
    }
    result
}

fn is_identifier_char(character: char) -> bool {
    character.is_alphanumeric() || character == '_' || character == '$'
}

fn audio_cache_key(
    text_hash: &str,
    request_segments: &[String],
    model: &str,
    voice: &str,
    style: &str,
    format: &str,
    pocket_runtime_fingerprint: impl FnOnce() -> String,
    pocket_segment_policy: &str,
) -> String {
    let mut contract = json!({
        "cacheVersion": CACHE_VERSION,
        "spokenTextSha256": text_hash,
        "provider": "Google Gemini Developer API",
        "model": model,
        "voice": voice,
        "style": style,
        "format": format,
    });
    if model == crate::pocket_tts::MODEL_ID {
        contract["cacheVersion"] = json!("pocket-narration-cache-v1");
        contract["provider"] = json!("pocket-tts");
        contract["packageVersion"] = json!(crate::pocket_tts::PACKAGE_VERSION);
        // Include the worker, asset revisions, Python and package pins. A new
        // runtime contract also changes the exact review fingerprint below.
        contract["runtimeFingerprint"] = json!(pocket_runtime_fingerprint());
        contract["pcmFormat"] = json!("pcm16-mono-24000");
        contract["segmentPolicy"] = json!(pocket_segment_policy);
    }
    if request_segments.len() > 1 {
        contract["requestSegmentSha256"] = json!(
            request_segments
                .iter()
                .map(|segment| sha256(segment.as_bytes()))
                .collect::<Vec<_>>()
        );
    }
    sha256(
        serde_json::to_string(&contract)
            .unwrap_or_default()
            .as_bytes(),
    )
}

#[cfg(test)]
pub(crate) fn audio_cache_key_for_script(
    exact_text: &str,
    model: &str,
    voice: &str,
    style: &str,
    format: &str,
) -> String {
    let request_segments = split_spoken_text(exact_text, MAX_TTS_REQUEST_TEXT_BYTES);
    audio_cache_key(
        &sha256(exact_text.as_bytes()),
        &request_segments,
        model,
        voice,
        style,
        format,
        crate::pocket_tts::runtime_fingerprint,
        POCKET_SEGMENT_POLICY,
    )
}

fn request_fingerprint(
    cache_key: &str,
    disclosure_version: &str,
    request_segments: &[String],
) -> String {
    let contract = json!({
        "cacheKey": cache_key,
        "disclosureVersion": disclosure_version,
        "requestSegmentSha256": request_segments.iter().map(|segment| sha256(segment.as_bytes())).collect::<Vec<_>>(),
    });
    sha256(
        serde_json::to_string(&contract)
            .unwrap_or_default()
            .as_bytes(),
    )
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn cache_path_for_version(storage_dir: &Path, fingerprint: &str, version: &str) -> PathBuf {
    storage_dir
        .join("narration-audio")
        .join(version)
        .join(format!("{fingerprint}.wav"))
}

fn cache_path(storage_dir: &Path, fingerprint: &str, model_id: &str) -> PathBuf {
    let version = if model_id == crate::pocket_tts::MODEL_ID {
        POCKET_CACHE_VERSION
    } else {
        CACHE_VERSION
    };
    cache_path_for_version(storage_dir, fingerprint, version)
}

fn read_cached_audio(storage_dir: &Path, fingerprint: &str, model_id: &str) -> Option<Vec<u8>> {
    let current = cache_path(storage_dir, fingerprint, model_id);
    if model_id == crate::pocket_tts::MODEL_ID {
        if let Some(parent) = current.parent() {
            if claim_initial_pocket_cache_sweep(parent) {
                prune_pocket_audio_cache(parent, Some(&current));
            }
        }
    }
    let file = fs::File::open(&current).ok()?;
    let bytes = read_cached_audio_from(&file)?;
    if model_id == crate::pocket_tts::MODEL_ID {
        // Modified time is already available on the cache entry, so LRU
        // ordering does not create another index or per-playback file.
        let _ = file.set_modified(SystemTime::now());
    }
    Some(bytes)
}

fn claim_initial_pocket_cache_sweep(cache_dir: &Path) -> bool {
    static SWEPT_DIRECTORIES: OnceLock<Mutex<std::collections::HashSet<PathBuf>>> = OnceLock::new();
    let directories =
        SWEPT_DIRECTORIES.get_or_init(|| Mutex::new(std::collections::HashSet::new()));
    match directories.lock() {
        Ok(mut swept) => swept.insert(cache_dir.to_path_buf()),
        Err(poisoned) => poisoned.into_inner().insert(cache_dir.to_path_buf()),
    }
}

fn prune_pocket_audio_cache(cache_dir: &Path, keep: Option<&Path>) {
    struct CacheFile {
        path: PathBuf,
        bytes: u64,
        modified: SystemTime,
    }

    let Ok(entries) = fs::read_dir(cache_dir) else {
        return;
    };
    let now = SystemTime::now();
    let mut files = Vec::new();
    let mut total_bytes = 0u64;
    let mut abandoned_temps = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.starts_with('.') && name.ends_with(".tmp") {
            let stale = entry
                .metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| now.duration_since(modified).ok())
                .is_some_and(|age| age >= ABANDONED_CACHE_TEMP_LIFETIME);
            if stale {
                abandoned_temps.push(path);
            }
            continue;
        }
        let Some(stem) = name.strip_suffix(".wav") else {
            continue;
        };
        if stem.len() != 64 || !stem.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            continue;
        }
        let Ok(metadata) = entry.metadata() else {
            continue;
        };
        let bytes = metadata.len();
        total_bytes = total_bytes.saturating_add(bytes);
        files.push(CacheFile {
            path,
            bytes,
            modified: metadata.modified().unwrap_or(UNIX_EPOCH),
        });
    }
    for path in abandoned_temps {
        let _ = fs::remove_file(path);
    }
    if files.len() <= MAX_POCKET_CACHE_FILES && total_bytes <= MAX_POCKET_CACHE_BYTES {
        return;
    }
    files.sort_by(|left, right| {
        left.modified
            .cmp(&right.modified)
            .then_with(|| left.path.cmp(&right.path))
    });
    let mut remaining_files = files.len();
    for file in files {
        if keep.is_some_and(|path| path == file.path) {
            continue;
        }
        if fs::remove_file(&file.path).is_ok() {
            remaining_files = remaining_files.saturating_sub(1);
            total_bytes = total_bytes.saturating_sub(file.bytes);
        }
        if remaining_files <= MAX_POCKET_CACHE_FILES && total_bytes <= MAX_POCKET_CACHE_BYTES {
            break;
        }
    }
}

fn read_cached_audio_from(reader: impl Read) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(MAX_RESPONSE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= MAX_RESPONSE_BYTES && is_wav(&bytes)).then_some(bytes)
}

fn write_cached_audio(
    storage_dir: &Path,
    fingerprint: &str,
    model_id: &str,
    bytes: &[u8],
) -> Result<(), ProviderFailure> {
    if bytes.len() > MAX_RESPONSE_BYTES || !is_wav(bytes) {
        return Err(failure(
            ProviderFailureKind::MalformedProviderResponse,
            "The narration engine returned invalid or oversized WAV audio. The prior audio cache was left untouched.",
        ));
    }
    let path = cache_path(storage_dir, fingerprint, model_id);
    let parent = path.parent().ok_or_else(|| {
        failure(
            ProviderFailureKind::LocalStorageUnavailable,
            "The local audio cache folder could not be located.",
        )
    })?;
    fs::create_dir_all(parent).map_err(|_| failure(ProviderFailureKind::LocalStorageUnavailable, "The local audio cache folder could not be created. The narration can still play for this session."))?;
    let temporary = parent.join(format!(".{}.{}.tmp", fingerprint, Uuid::new_v4()));
    write_cache_temporary(&temporary, || fs::write(&temporary, bytes))
        .map_err(|_| {
            failure(
                ProviderFailureKind::LocalStorageUnavailable,
                "The local audio cache could not be written. The narration can still play for this session.",
            )
        })?;
    fs::rename(&temporary, &path).map_err(|_| {
        let _ = fs::remove_file(&temporary);
        failure(ProviderFailureKind::LocalStorageUnavailable, "The local audio cache could not be finalized. The narration can still play for this session.")
    })?;
    if model_id == crate::pocket_tts::MODEL_ID {
        if let Some(parent) = path.parent() {
            prune_pocket_audio_cache(parent, Some(&path));
        }
    }
    Ok(())
}

fn write_cache_temporary(
    path: &Path,
    write: impl FnOnce() -> std::io::Result<()>,
) -> std::io::Result<()> {
    let result = write();
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}

fn is_wav(bytes: &[u8]) -> bool {
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return false;
    }
    let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if riff_size.checked_add(8) != Some(bytes.len()) {
        return false;
    }

    let mut cursor = 12;
    let mut has_pcm_format = false;
    let mut has_audio_data = false;
    while cursor + 8 <= bytes.len() {
        let chunk_id = &bytes[cursor..cursor + 4];
        let chunk_size =
            u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let data_start = cursor + 8;
        let Some(data_end) = data_start.checked_add(chunk_size) else {
            return false;
        };
        if data_end > bytes.len() {
            return false;
        }

        if chunk_id == b"fmt " {
            if chunk_size < 16 || has_pcm_format {
                return false;
            }
            let format = u16::from_le_bytes(bytes[data_start..data_start + 2].try_into().unwrap());
            let channels =
                u16::from_le_bytes(bytes[data_start + 2..data_start + 4].try_into().unwrap());
            let sample_rate =
                u32::from_le_bytes(bytes[data_start + 4..data_start + 8].try_into().unwrap());
            let byte_rate =
                u32::from_le_bytes(bytes[data_start + 8..data_start + 12].try_into().unwrap());
            let block_align =
                u16::from_le_bytes(bytes[data_start + 12..data_start + 14].try_into().unwrap());
            let bits_per_sample =
                u16::from_le_bytes(bytes[data_start + 14..data_start + 16].try_into().unwrap());
            has_pcm_format = format == 1
                && channels == 1
                && sample_rate == 24_000
                && byte_rate == 48_000
                && block_align == 2
                && bits_per_sample == 16;
            if !has_pcm_format {
                return false;
            }
        } else if chunk_id == b"data" {
            if has_audio_data || chunk_size == 0 || chunk_size % 2 != 0 {
                return false;
            }
            has_audio_data = true;
        }

        let padded_size = chunk_size.checked_add(chunk_size % 2);
        let Some(next) = padded_size.and_then(|size| data_start.checked_add(size)) else {
            return false;
        };
        if next > bytes.len() {
            return false;
        }
        cursor = next;
    }

    cursor == bytes.len() && has_pcm_format && has_audio_data
}

fn build_request_body(review: &NarrationRequestReview) -> Value {
    let text_content = if review.style.is_empty() {
        json!({ "type": "text", "text": review.exact_text })
    } else {
        json!({
            "type": "text",
            "text": review.exact_text,
            "annotations": [{ "type": "speech_metadata", "style": review.style }]
        })
    };
    json!({
        "model": review.model_id,
        "input": [{ "type": "user_input", "content": [text_content] }],
        "response_format": { "type": "audio", "mime_type": "audio/wav" },
        "generation_config": { "speech_config": [{ "voice": review.voice }] },
    })
}

fn decode_audio_response(bytes: &[u8]) -> Result<Vec<u8>, ProviderFailure> {
    let response: Value = serde_json::from_slice(bytes).map_err(|_| {
        failure(ProviderFailureKind::MalformedProviderResponse, "Google returned a narration response Code Atlas could not parse. The transcript and previous audio remain available.")
    })?;
    let encoded = response
        .get("steps")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|step| step.get("type").and_then(Value::as_str) == Some("model_output"))
        .flat_map(|step| step.get("content").and_then(Value::as_array).into_iter().flatten())
        .find(|content| content.get("type").and_then(Value::as_str) == Some("audio"))
        .and_then(|content| content.get("data").and_then(Value::as_str))
        .ok_or_else(|| failure(ProviderFailureKind::MalformedProviderResponse, "Google's response did not contain WAV audio. The transcript and previous audio remain available."))?;
    let audio = base64::engine::general_purpose::STANDARD.decode(encoded).map_err(|_| {
        failure(ProviderFailureKind::MalformedProviderResponse, "Google returned audio Code Atlas could not decode. The transcript and previous audio remain available.")
    })?;
    if audio.len() > MAX_RESPONSE_BYTES || !is_wav(&audio) {
        return Err(failure(
            ProviderFailureKind::MalformedProviderResponse,
            "Google's narration audio was invalid or exceeded the local size limit. The transcript and previous audio remain available.",
        ));
    }
    Ok(audio)
}

async fn request_audio(
    review: &NarrationRequestReview,
    api_key: &str,
) -> Result<Vec<u8>, ProviderFailure> {
    request_audio_at_endpoint(review, api_key, ENDPOINT, true, false).await
}

fn narration_http_client(https_only: bool, no_proxy: bool) -> reqwest::ClientBuilder {
    let builder = reqwest::Client::builder()
        .https_only(https_only)
        // Narration consent covers this endpoint and exact script. Never send
        // the script or API key to a redirect destination.
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(90));
    if no_proxy {
        builder.no_proxy()
    } else {
        builder
    }
}

async fn request_audio_at_endpoint(
    review: &NarrationRequestReview,
    api_key: &str,
    endpoint: &str,
    https_only: bool,
    no_proxy: bool,
) -> Result<Vec<u8>, ProviderFailure> {
    let mut key_header = reqwest::header::HeaderValue::from_str(api_key).map_err(|_| {
        failure(
            ProviderFailureKind::InvalidApiKey,
            "The saved API key has an invalid header format. Replace it in AI and narration setup.",
        )
    })?;
    key_header.set_sensitive(true);
    let body = build_request_body(review);
    let client = narration_http_client(https_only, no_proxy)
        .build()
        .map_err(|_| {
            failure(
                ProviderFailureKind::NetworkUnavailable,
                "The Gemini narration client could not start. No audio was changed.",
            )
        })?;
    let response = client
        .post(endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header("x-goog-api-key", key_header)
        .json(&body)
        .send()
        .await
        .map_err(|error| {
            if error.is_timeout() {
                failure(ProviderFailureKind::RequestTimedOut, "Google did not respond within 90 seconds. The transcript and previous audio remain available; review again before retrying.")
            } else {
                failure(ProviderFailureKind::NetworkUnavailable, "Google could not be reached. The transcript and previous audio remain available; review again after reconnecting.")
            }
        })?;
    let status = response.status();
    let bytes = read_response(response).await?;
    if !status.is_success() {
        return Err(classify_error(status.as_u16(), &bytes));
    }
    decode_audio_response(&bytes)
}

async fn read_response(mut response: reqwest::Response) -> Result<Vec<u8>, ProviderFailure> {
    let mut bytes = Vec::with_capacity(
        response
            .content_length()
            .unwrap_or(0)
            .min(MAX_RESPONSE_BYTES as u64) as usize,
    );
    while let Some(chunk) = response.chunk().await.map_err(|_| {
        failure(ProviderFailureKind::NetworkUnavailable, "Google's narration response was interrupted. The transcript and previous audio remain available.")
    })? {
        if bytes.len().saturating_add(chunk.len()) > MAX_RESPONSE_BYTES {
            return Err(failure(ProviderFailureKind::ProviderResponseTooLarge, "Google's narration response exceeded the local size limit. The transcript and previous audio remain available."));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

fn classify_error(status: u16, bytes: &[u8]) -> ProviderFailure {
    let value: Value = serde_json::from_slice(bytes).unwrap_or(Value::Null);
    let error = value.get("error").unwrap_or(&Value::Null);
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let provider_status = error
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if status == 429
        || provider_status.contains("resource_exhausted")
        || message.contains("quota")
        || message.contains("rate limit")
    {
        failure(
            ProviderFailureKind::QuotaExceeded,
            "Google reported a quota or rate limit. Code Atlas will not switch models or enable paid usage. Check this key's current quota; the transcript and previous audio remain available.",
        )
    } else if message.contains("billing") || provider_status.contains("billing") {
        failure(
            ProviderFailureKind::BillingRequired,
            "Google reported a billing requirement for this key or project. Code Atlas did not enable billing or switch to paid use; the transcript and previous audio remain available.",
        )
    } else if is_region_restriction_error(&provider_status, &message) {
        failure(
            ProviderFailureKind::RegionUnavailable,
            "Google rejected this narration because the service is unavailable in the current region. Check Google's supported-regions disclosure; no fallback was used. The transcript and previous audio remain available.",
        )
    } else if status == 401
        || status == 403
        || provider_status.contains("unauthenticated")
        || message.contains("api key")
    {
        failure(
            ProviderFailureKind::InvalidApiKey,
            "Google rejected the saved API key or access for this project. Replace it in AI and narration setup; no fallback was attempted.",
        )
    } else if status == 404 || provider_status.contains("not_found") {
        failure(
            ProviderFailureKind::UnavailableModel,
            "Google no longer exposes the reviewed TTS model to this key or region. No model was substituted; the transcript and previous audio remain available.",
        )
    } else if status >= 500 {
        failure(
            ProviderFailureKind::NetworkUnavailable,
            "Google's Gemini TTS service is temporarily unavailable. The transcript and previous audio remain available; review again before retrying.",
        )
    } else {
        failure(
            ProviderFailureKind::ProviderRejectedRequest,
            "Google rejected this narration request. The transcript and previous audio remain available; review the request before retrying.",
        )
    }
}

fn failure(kind: ProviderFailureKind, message: impl Into<String>) -> ProviderFailure {
    ProviderFailure {
        kind,
        message: message.into(),
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::time::Duration;

    struct TestStorage(PathBuf);

    impl TestStorage {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("code-atlas-narration-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).expect("temporary narration storage should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestStorage {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    async fn prewarm_jobs_coalesce_and_manual_play_moves_a_queued_job_forward() {
        let queue = NarrationPrewarmQueue::default();
        let (first_started_tx, first_started_rx) = oneshot::channel();
        let (second_started_tx, mut second_started_rx) = oneshot::channel();
        let (third_started_tx, mut third_started_rx) = oneshot::channel();
        let (first_release_tx, first_release_rx) = oneshot::channel();
        let (second_release_tx, second_release_rx) = oneshot::channel();
        let (third_release_tx, third_release_rx) = oneshot::channel();

        let first = queue
            .schedule("first".into(), move |mut cancellation| async move {
                let _ = first_started_tx.send(());
                tokio::select! {
                    _ = &mut cancellation => false,
                    _ = first_release_rx => true,
                }
            })
            .expect("first prewarm should be accepted");
        let second = queue
            .schedule("second".into(), move |mut cancellation| async move {
                let _ = second_started_tx.send(());
                tokio::select! {
                    _ = &mut cancellation => false,
                    _ = second_release_rx => true,
                }
            })
            .expect("second prewarm should be accepted");
        let third = queue
            .schedule("third".into(), move |mut cancellation| async move {
                let _ = third_started_tx.send(());
                tokio::select! {
                    _ = &mut cancellation => false,
                    _ = third_release_rx => true,
                }
            })
            .expect("third prewarm should be accepted");
        assert!(first.accepted && second.accepted && third.accepted);
        first_started_rx
            .await
            .expect("one background task should own the dispatcher");
        assert!(matches!(
            second_started_rx.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
        assert!(matches!(
            third_started_rx.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));

        let duplicate = queue
            .schedule("second".into(), |_| async {
                panic!("a duplicate fingerprint must not start a second worker")
            })
            .expect("duplicate status should be observable");
        assert!(!duplicate.accepted);
        assert_eq!(duplicate.state, NarrationPrewarmState::Preparing);
        queue.prioritize("third");
        first_release_tx
            .send(())
            .expect("the first prewarm should still be active");
        third_started_rx
            .await
            .expect("manual playback should promote its queued prewarm");
        third_release_tx
            .send(())
            .expect("the promoted prewarm should finish");
        second_started_rx
            .await
            .expect("the remaining queued prewarm should continue");
        second_release_tx
            .send(())
            .expect("the remaining prewarm should finish");
        tokio::time::timeout(Duration::from_secs(2), async {
            while queue.status("second") == Some(NarrationPrewarmState::Preparing) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the final queued task should reach a terminal state");
        assert_eq!(queue.status("first"), Some(NarrationPrewarmState::Ready));
        assert_eq!(queue.status("second"), Some(NarrationPrewarmState::Ready));
        assert_eq!(queue.status("third"), Some(NarrationPrewarmState::Ready));
    }

    #[tokio::test]
    async fn full_prewarms_remain_bounded_and_shutdown_cancels_all_pending_work() {
        let queue = NarrationPrewarmQueue::default();
        let (active_started_tx, active_started_rx) = oneshot::channel();
        let mut keys = vec!["active".to_owned()];
        queue
            .schedule("active".into(), move |cancellation| async move {
                let _ = active_started_tx.send(());
                let _ = cancellation.await;
                false
            })
            .expect("first prewarm should be accepted");
        active_started_rx
            .await
            .expect("the first prewarm should begin");

        for index in 1..MAX_PREWARM_JOBS {
            let key = format!("pending-{index}");
            let schedule = queue
                .schedule(key.clone(), |cancellation| async move {
                    let _ = cancellation.await;
                    false
                })
                .expect("bounded prewarm should be scheduled");
            assert!(schedule.accepted);
            keys.push(key);
        }
        let overflow = queue
            .schedule("overflow".into(), |_| async { true })
            .expect("queue saturation is an unavailable status, not an error");
        assert!(!overflow.accepted);
        assert_eq!(overflow.state, NarrationPrewarmState::Unavailable);

        queue.shutdown();
        for key in keys {
            assert_eq!(queue.status(&key), Some(NarrationPrewarmState::Unavailable));
        }
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let worker_running = queue
                    .0
                    .lock()
                    .expect("queue state should remain available")
                    .worker_running;
                if !worker_running {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("shutdown should wait for active work to observe cancellation and exit");
    }

    #[tokio::test]
    async fn a_panicking_preparation_fails_cleanly_and_does_not_strand_later_audio() {
        let queue = NarrationPrewarmQueue::default();
        queue
            .schedule("broken".into(), |_| async {
                panic!("synthetic worker failure")
            })
            .expect("failed work should still be admitted to the bounded queue");
        queue
            .schedule("healthy".into(), |_| async { true })
            .expect("the next clip should remain queued");

        tokio::time::timeout(Duration::from_secs(2), async {
            while queue.status("healthy") != Some(NarrationPrewarmState::Ready) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("a worker failure must not strand later clips");
        assert_eq!(
            queue.status("broken"),
            Some(NarrationPrewarmState::Unavailable)
        );
        assert_eq!(queue.status("healthy"), Some(NarrationPrewarmState::Ready));
    }

    #[test]
    fn local_audio_preparation_status_matches_the_canonical_cache_key() {
        let storage = TestStorage::new();
        let queue = NarrationPrewarmQueue::default();
        let reviews = PendingNarrationReviews::default();
        let input = BuildNarrationReviewInput {
            spoken_text: "The parser returns a result for the next step.".into(),
            voice: crate::pocket_tts::VOICE.into(),
            style: String::new(),
            lexicon: Vec::new(),
        };
        let idle = local_audio_preparation_status(&storage.0, &queue, input.clone());
        let ProviderCommandResult::Ok { data } = idle else {
            panic!("valid local narration should have a preparation status");
        };
        assert_eq!(data.state, NarrationPrewarmState::Idle);
        assert!(!data.cached_audio_available);

        let review = match build_local_request_review(&storage.0, &reviews, input.clone()) {
            ProviderCommandResult::Ok { data } => data,
            ProviderCommandResult::Error { error } => panic!("review should build: {error:?}"),
        };
        write_cached_audio(
            &storage.0,
            &review.cache_key,
            &review.model_id,
            &valid_test_wav(7),
        )
        .expect("valid local audio should be cached");
        let ready = local_audio_preparation_status(&storage.0, &queue, input);
        let ProviderCommandResult::Ok { data } = ready else {
            panic!("cached local narration should have a preparation status");
        };
        assert_eq!(data.state, NarrationPrewarmState::Ready);
        assert!(data.cached_audio_available);
    }

    #[test]
    fn greenfield_current_format_uses_only_the_current_pocket_cache_directory() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let input = BuildNarrationReviewInput {
            spoken_text: "Keep this local clip across the cache layout change.".into(),
            voice: crate::pocket_tts::VOICE.into(),
            style: String::new(),
            lexicon: Vec::new(),
        };
        let pocket = match build_local_request_review(&storage.0, &reviews, input) {
            ProviderCommandResult::Ok { data } => data,
            ProviderCommandResult::Error { error } => {
                panic!("local Pocket review should build: {error:?}")
            }
        };
        let pocket_path = cache_path(&storage.0, &pocket.cache_key, &pocket.model_id);
        assert_eq!(
            pocket_path.parent().and_then(Path::file_name),
            Some(std::ffi::OsStr::new(POCKET_CACHE_VERSION))
        );

        // Older builds wrote Pocket audio under the Gemini-named directory.
        // The current format does not read that app-owned cache layout.
        let previous_audio = valid_test_wav(31);
        let old_path = cache_path_for_version(&storage.0, &pocket.cache_key, CACHE_VERSION);
        fs::create_dir_all(old_path.parent().expect("cache path has a parent"))
            .expect("old cache folder should be creatable");
        fs::write(&old_path, &previous_audio).expect("old cached audio should be retained");
        assert!(read_cached_audio(&storage.0, &pocket.cache_key, &pocket.model_id).is_none());

        let new_audio = valid_test_wav(37);
        write_cached_audio(&storage.0, &pocket.cache_key, &pocket.model_id, &new_audio)
            .expect("new Pocket audio should use its backend directory");
        assert!(pocket_path.is_file());
        assert_eq!(
            read_cached_audio(&storage.0, &pocket.cache_key, &pocket.model_id),
            Some(new_audio)
        );

        let gemini = build_review(
            &storage.0,
            &reviews,
            "Keep Gemini in its own cache namespace.",
            "Kore",
            "",
            vec![],
        );
        let gemini_path = cache_path(&storage.0, &gemini.cache_key, &gemini.model_id);
        assert_eq!(
            gemini_path.parent().and_then(Path::file_name),
            Some(std::ffi::OsStr::new(CACHE_VERSION))
        );
    }

    #[test]
    fn generation_time_prewarm_reuses_cached_audio_without_queuing_synthesis() {
        let storage = TestStorage::new();
        let queue = NarrationPrewarmQueue::default();
        let reviews = PendingNarrationReviews::default();
        let input = BuildNarrationReviewInput {
            spoken_text: "The cached clip is ready as soon as its section appears.".into(),
            voice: crate::pocket_tts::VOICE.into(),
            style: String::new(),
            lexicon: Vec::new(),
        };
        let review = match build_local_request_review(&storage.0, &reviews, input.clone()) {
            ProviderCommandResult::Ok { data } => data,
            ProviderCommandResult::Error { error } => {
                panic!("local review should build: {error:?}")
            }
        };
        write_cached_audio(
            &storage.0,
            &review.cache_key,
            &review.model_id,
            &valid_test_wav(11),
        )
        .expect("the previous section should already have persisted its matching clip");

        let result = prewarm_local_audio(&storage.0, &reviews, &queue, input);
        let ProviderCommandResult::Ok { data } = result else {
            panic!("generation-time prewarm should reuse the matching cached clip");
        };
        assert_eq!(data.state, NarrationPrewarmState::Ready);
        assert!(data.cached_audio_available);
        assert_eq!(queue.status(&review.cache_key), None);
        let queue_state = queue
            .0
            .lock()
            .expect("prewarm queue should remain available");
        assert!(queue_state.pending.is_empty());
        assert!(!queue_state.worker_running);
    }

    #[tokio::test]
    async fn playback_waits_for_a_delayed_prewarm_then_reuses_the_cached_clip() {
        let storage = TestStorage::new();
        let queue = NarrationPrewarmQueue::default();
        let reviews = PendingNarrationReviews::default();
        let review = match build_local_request_review(
            storage.path(),
            &reviews,
            BuildNarrationReviewInput {
                spoken_text: "The prefix selects the font reader.".into(),
                voice: crate::pocket_tts::VOICE.into(),
                style: String::new(),
                lexicon: Vec::new(),
            },
        ) {
            ProviderCommandResult::Ok { data } => data,
            ProviderCommandResult::Error { error } => {
                panic!("local review should build: {error:?}")
            }
        };
        let expected_audio = valid_test_wav(23);
        let expected_audio_for_worker = expected_audio.clone();
        let worker_runs = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let worker_runs_for_job = worker_runs.clone();
        let (started_sender, started_receiver) = oneshot::channel();
        let (release_sender, release_receiver) = oneshot::channel();
        let storage_path = storage.path().to_path_buf();
        let cache_key = review.cache_key.clone();
        let model_id = review.model_id.clone();

        queue
            .schedule(
                review.cache_key.clone(),
                move |mut cancellation| async move {
                    worker_runs_for_job.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let _ = started_sender.send(());
                    tokio::select! {
                        _ = &mut cancellation => false,
                        released = release_receiver => {
                            if released.is_err() {
                                return false;
                            }
                            write_cached_audio(
                                &storage_path,
                                &cache_key,
                                &model_id,
                                &expected_audio_for_worker,
                            )
                            .is_ok()
                        }
                    }
                },
            )
            .expect("the local clip should enter the bounded prewarm queue");
        started_receiver
            .await
            .expect("the delayed local worker should start");

        let storage_path = storage.path().to_path_buf();
        let play_reviews = reviews.clone();
        let play_queue = queue.clone();
        let review_id = review.review_id.clone();
        let fingerprint = review.fingerprint.clone();
        let (_cancel_sender, mut cancellation) = oneshot::channel();
        let play = tokio::spawn(async move {
            load_prewarmed_audio_if_ready(
                &storage_path,
                &play_reviews,
                &play_queue,
                &review_id,
                &fingerprint,
                &mut cancellation,
            )
            .await
        });
        tokio::task::yield_now().await;
        assert!(
            !play.is_finished(),
            "Play should wait while Pocket TTS warms the clip"
        );
        assert_eq!(
            worker_runs.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "the single queued worker should own synthesis"
        );

        release_sender
            .send(())
            .expect("the delayed worker should be released to finish");
        let Some(ProviderCommandResult::Ok { data }) = play
            .await
            .expect("the player should finish waiting for its clip")
        else {
            panic!("Play should load the completed prewarm directly");
        };
        assert!(
            data.cache_hit,
            "the prepared WAV should be returned as a cache hit"
        );
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(data.audio_base64)
                .expect("cached WAV should be base64 encoded"),
            expected_audio
        );

        let (_cancel_sender, mut cancellation) = oneshot::channel();
        let reused = load_prewarmed_audio_if_ready(
            storage.path(),
            &reviews,
            &queue,
            &review.review_id,
            &review.fingerprint,
            &mut cancellation,
        )
        .await;
        assert!(matches!(reused, Some(ProviderCommandResult::Ok { data }) if data.cache_hit));
        assert_eq!(
            worker_runs.load(std::sync::atomic::Ordering::SeqCst),
            1,
            "replaying the same script and settings should not rerun synthesis"
        );
    }

    #[tokio::test]
    async fn completed_prewarm_playback_releases_a_bounded_review_slot() {
        let storage = TestStorage::new();
        let queue = NarrationPrewarmQueue::default();
        let reviews = PendingNarrationReviews::default();
        let mut pending = Vec::new();
        for index in 0..MAX_PENDING_REVIEWS {
            let input = BuildNarrationReviewInput {
                spoken_text: format!("Prepared section {index} has a local clip."),
                voice: crate::pocket_tts::VOICE.into(),
                style: String::new(),
                lexicon: Vec::new(),
            };
            let review = match build_local_request_review(&storage.0, &reviews, input) {
                ProviderCommandResult::Ok { data } => data,
                ProviderCommandResult::Error { error } => {
                    panic!("bounded synthetic review should be admitted: {error:?}")
                }
            };
            pending.push(review);
        }
        let played = &pending[0];
        let storage_path = storage.path().to_path_buf();
        let cache_key = played.cache_key.clone();
        let model_id = played.model_id.clone();
        let audio = valid_test_wav(41);
        let worker_audio = audio.clone();
        let scheduled = queue
            .schedule(cache_key.clone(), move |_| async move {
                write_cached_audio(&storage_path, &cache_key, &model_id, &worker_audio).is_ok()
            })
            .expect("the cached playback fixture should enter the worker");
        assert!(scheduled.accepted);

        tokio::time::timeout(Duration::from_secs(2), async {
            while queue.status(&played.cache_key) != Some(NarrationPrewarmState::Ready) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("the synthetic prewarm should finish");

        let (_cancel_sender, mut cancellation) = oneshot::channel();
        let loaded = load_prewarmed_audio_for_playback(
            storage.path(),
            &reviews,
            &queue,
            &played.review_id,
            &played.fingerprint,
            &mut cancellation,
        )
        .await;
        assert!(matches!(
            loaded,
            Some(ProviderCommandResult::Ok { data }) if data.cache_hit
        ));
        assert!(matches!(
            reviews.get(&played.review_id, &played.fingerprint),
            Err(error) if error.kind == ProviderFailureKind::ReviewUnavailable
        ));

        let next = build_local_request_review(
            storage.path(),
            &reviews,
            BuildNarrationReviewInput {
                spoken_text: "The next section can still receive a review.".into(),
                voice: crate::pocket_tts::VOICE.into(),
                style: String::new(),
                lexicon: Vec::new(),
            },
        );
        assert!(
            matches!(next, ProviderCommandResult::Ok { .. }),
            "cached playback must free a slot for later narration reviews"
        );
    }

    #[tokio::test]
    async fn cancellation_drops_the_pending_audio_request() {
        struct DropSignal(Option<oneshot::Sender<()>>);
        impl Drop for DropSignal {
            fn drop(&mut self) {
                if let Some(sender) = self.0.take() {
                    let _ = sender.send(());
                }
            }
        }

        let (cancel_sender, mut cancellation) = oneshot::channel();
        let (started_sender, started_receiver) = oneshot::channel();
        let (dropped_sender, dropped_receiver) = oneshot::channel();
        let request = async move {
            let _drop_signal = DropSignal(Some(dropped_sender));
            let _ = started_sender.send(());
            std::future::pending::<Result<(), ProviderFailure>>().await
        };
        tokio::spawn(async move {
            let _ = started_receiver.await;
            let _ = cancel_sender.send(());
        });

        let error = await_cancellable_audio_request(&mut cancellation, request)
            .await
            .expect_err("cancellation should discard the pending provider request");
        assert_eq!(error.kind, ProviderFailureKind::RequestCancelled);
        assert!(error.message.contains("reviewed speech text"));
        dropped_receiver
            .await
            .expect("the pending audio request future should be dropped");
    }

    #[tokio::test]
    async fn cancellation_arriving_with_a_completed_segment_discards_the_audio() {
        let segments = vec!["One completed segment.".to_owned()];
        let (cancel_sender, mut cancellation) = oneshot::channel();
        let mut cancel_sender = Some(cancel_sender);

        let error = synthesize_segments(&segments, &mut cancellation, move |_segment| {
            let cancel_sender = cancel_sender
                .take()
                .expect("the single request should own the cancellation sender");
            async move {
                // Signal cancellation during the same poll that completes the
                // request. The completed HTTP response must still be dropped.
                let _ = cancel_sender.send(());
                Ok(valid_test_wav(5))
            }
        })
        .await
        .expect_err("a simultaneously completed request should be discarded");

        assert_eq!(error.kind, ProviderFailureKind::RequestCancelled);
    }

    #[test]
    fn explicit_region_rejection_is_not_reported_as_an_invalid_key() {
        let region = classify_error(
            403,
            br#"{"error":{"status":"PERMISSION_DENIED","message":"Text-to-speech is not available in your region; do not echo this"}}"#,
        );
        assert_eq!(region.kind, ProviderFailureKind::RegionUnavailable);
        assert!(region.message.contains("supported-regions"));
        assert!(region.message.contains("previous audio remain available"));
        assert!(!region.message.contains("do not echo"));

        let invalid_key = classify_error(
            403,
            br#"{"error":{"status":"PERMISSION_DENIED","message":"Invalid API key"}}"#,
        );
        assert_eq!(invalid_key.kind, ProviderFailureKind::InvalidApiKey);
    }

    fn build_review(
        storage: &Path,
        reviews: &PendingNarrationReviews,
        spoken_text: &str,
        voice: &str,
        style: &str,
        lexicon: Vec<PronunciationEntry>,
    ) -> NarrationRequestReview {
        match build_request_review(
            storage,
            reviews,
            BuildNarrationReviewInput {
                spoken_text: spoken_text.to_owned(),
                voice: voice.to_owned(),
                style: style.to_owned(),
                lexicon,
            },
        ) {
            ProviderCommandResult::Ok { data } => data,
            ProviderCommandResult::Error { error } => panic!("review should build: {error:?}"),
        }
    }

    fn valid_test_wav(fill: u8) -> Vec<u8> {
        let mut bytes = vec![fill; 48];
        bytes[..4].copy_from_slice(b"RIFF");
        bytes[4..8].copy_from_slice(&40_u32.to_le_bytes());
        bytes[8..12].copy_from_slice(b"WAVE");
        bytes[12..16].copy_from_slice(b"fmt ");
        bytes[16..20].copy_from_slice(&16_u32.to_le_bytes());
        bytes[20..22].copy_from_slice(&1_u16.to_le_bytes());
        bytes[22..24].copy_from_slice(&1_u16.to_le_bytes());
        bytes[24..28].copy_from_slice(&24_000_u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&48_000_u32.to_le_bytes());
        bytes[32..34].copy_from_slice(&2_u16.to_le_bytes());
        bytes[34..36].copy_from_slice(&16_u16.to_le_bytes());
        bytes[36..40].copy_from_slice(b"data");
        bytes[40..44].copy_from_slice(&4_u32.to_le_bytes());
        bytes
    }

    #[test]
    fn oversized_cached_wav_reads_only_the_limit_plus_sentinel() {
        struct CountingReader {
            remaining: usize,
            bytes_read: usize,
        }

        impl Read for CountingReader {
            fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
                let count = output.len().min(self.remaining);
                output[..count].fill(b'x');
                self.remaining -= count;
                self.bytes_read += count;
                Ok(count)
            }
        }

        let mut reader = CountingReader {
            remaining: MAX_RESPONSE_BYTES + 100,
            bytes_read: 0,
        };
        assert!(read_cached_audio_from(&mut reader).is_none());
        assert_eq!(reader.bytes_read, MAX_RESPONSE_BYTES + 1);
    }

    fn read_narration_http_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        loop {
            let mut byte = [0_u8; 1];
            stream
                .read_exact(&mut byte)
                .expect("the narration request header should be readable");
            request.push(byte[0]);
            assert!(request.len() <= 64 * 1024, "request headers stay bounded");
            if request.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        let header_end = request.len();
        let headers = String::from_utf8_lossy(&request[..header_end]);
        let content_length = headers
            .lines()
            .find(|line| {
                line.split_once(':')
                    .is_some_and(|(name, _)| name.eq_ignore_ascii_case("content-length"))
            })
            .and_then(|line| line.split_once(':').map(|(_, value)| value.trim()))
            .and_then(|value| value.parse::<usize>().ok())
            .expect("the narration request has a content length");
        request.resize(header_end + content_length, 0);
        stream
            .read_exact(&mut request[header_end..])
            .expect("the reviewed narration body should be readable");
        request
    }

    #[tokio::test]
    async fn narration_transport_rejects_redirects_without_a_second_request() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let review = build_review(
            storage.path(),
            &reviews,
            "A reviewed narration script.",
            "Kore",
            "",
            Vec::new(),
        );

        for status in [
            "301 Moved Permanently",
            "302 Found",
            "303 See Other",
            "307 Temporary Redirect",
            "308 Permanent Redirect",
        ] {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}/v1/narration", listener.local_addr().unwrap());
            let api_key = format!("narration-redirect-marker-{status}");
            let expected_body = serde_json::to_vec(&build_request_body(&review)).unwrap();

            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let first_request = read_narration_http_request(&mut stream);
                let response = format!(
                    "HTTP/1.1 {status}\r\nLocation: /collector\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                );
                stream.write_all(response.as_bytes()).unwrap();

                listener.set_nonblocking(true).unwrap();
                let deadline = std::time::Instant::now() + Duration::from_millis(250);
                let followed = loop {
                    match listener.accept() {
                        Ok(_) => break true,
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            if std::time::Instant::now() >= deadline {
                                break false;
                            }
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(error) => {
                            panic!("could not inspect redirected narration requests: {error}")
                        }
                    }
                };
                (first_request, followed)
            });

            let error = request_audio_at_endpoint(&review, &api_key, &endpoint, false, true)
                .await
                .expect_err("a redirect response must not produce narration audio");
            assert!(
                !error.message.contains(&api_key),
                "provider errors must not expose the API key"
            );
            let (first_request, followed) = server.join().unwrap();
            assert!(!followed, "narration client followed {status}");
            let header_end = first_request
                .windows(4)
                .position(|bytes| bytes == b"\r\n\r\n")
                .expect("first request has a header boundary");
            let headers = String::from_utf8_lossy(&first_request[..header_end]);
            assert!(
                headers.contains(api_key.as_str()),
                "the API key stays on the reviewed endpoint"
            );
            assert!(!headers.contains("/collector"));
            assert_eq!(
                &first_request[header_end + 4..],
                expected_body.as_slice(),
                "the first request contains only the reviewed narration body"
            );
        }
    }

    #[tokio::test]
    async fn reviewed_segments_are_synthesized_sequentially_before_audio_is_joined() {
        let segments = vec!["First part.".to_owned(), "Second part.".to_owned()];
        let requests = Arc::new(Mutex::new(Vec::new()));
        let observed_requests = requests.clone();
        let (_cancel_sender, mut cancellation) = oneshot::channel();

        let audio = synthesize_segments(&segments, &mut cancellation, move |segment| {
            let requests = observed_requests.clone();
            async move {
                requests.lock().unwrap().push(segment);
                Ok(valid_test_wav(3))
            }
        })
        .await
        .expect("both reviewed segments should synthesize and join");

        assert_eq!(*requests.lock().unwrap(), segments);
        assert!(is_wav(&audio));
        assert_eq!(wav_pcm_data(&audio).unwrap().len(), 4 + 9_600 + 4);
    }

    #[tokio::test]
    async fn failed_segment_stops_the_sequence_without_producing_combined_audio() {
        let segments = vec!["First part.".to_owned(), "Second part.".to_owned()];
        let requests = Arc::new(Mutex::new(Vec::new()));
        let observed_requests = requests.clone();
        let (_cancel_sender, mut cancellation) = oneshot::channel();

        let error = synthesize_segments(&segments, &mut cancellation, move |segment| {
            let requests = observed_requests.clone();
            async move {
                requests.lock().unwrap().push(segment.clone());
                if segment == "First part." {
                    Err(failure(
                        ProviderFailureKind::QuotaExceeded,
                        "mock quota failure",
                    ))
                } else {
                    Ok(valid_test_wav(4))
                }
            }
        })
        .await
        .expect_err("a rejected first segment should stop synthesis");

        assert_eq!(error.kind, ProviderFailureKind::QuotaExceeded);
        assert_eq!(*requests.lock().unwrap(), vec!["First part.".to_owned()]);
    }

    #[test]
    fn long_speech_is_split_at_sentence_boundaries_without_losing_text() {
        let text = "First sentence. Second sentence. Third sentence.";
        let segments = split_spoken_text(text, 20);
        assert!(segments.len() > 1);
        assert_eq!(segments.concat(), text);
        assert!(segments.iter().all(|segment| segment.len() <= 20));
        assert_eq!(segments[0], "First sentence. ");
    }

    #[test]
    fn speech_splitting_preserves_unicode_and_uses_word_boundaries_when_needed() {
        let text = "Résumé walkthrough. Next section, with details.";
        let segments = split_spoken_text(text, 18);
        assert_eq!(segments.concat(), text);
        assert!(segments.iter().all(|segment| segment.len() <= 18));
        assert_eq!(segments[0], "Résumé ");

        let unbroken = "x".repeat(21);
        let pieces = split_spoken_text(&unbroken, 8);
        assert_eq!(
            pieces,
            vec![
                "xxxxxxxx".to_owned(),
                "xxxxxxxx".to_owned(),
                "xxxxx".to_owned()
            ]
        );

        let multi_byte = "界界界界界";
        let pieces = split_spoken_text(multi_byte, 4);
        assert_eq!(pieces.concat(), multi_byte);
        assert!(pieces.iter().all(|piece| piece.len() <= 4));
    }

    #[test]
    fn sentence_splitting_keeps_dotted_versions_in_one_segment() {
        let text = "Use version v1.2.3, then continue. Next step.";
        let segments = split_spoken_text(text, 18);

        assert_eq!(segments.concat(), text);
        assert_eq!(segments[0], "Use version ");
        assert_eq!(segments[1], "v1.2.3, then ");
        assert_eq!(segments[2], "continue. ");
        assert_eq!(segments[3], "Next step.");
        assert!(segments.iter().all(|segment| segment.len() <= 18));
    }

    #[test]
    fn narration_review_accepts_the_utf8_byte_limit_and_splits_it_for_requests() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let spoken_text = format!("{}x", "界".repeat((MAX_SPOKEN_TEXT_BYTES - 1) / 3));
        assert_eq!(spoken_text.len(), MAX_SPOKEN_TEXT_BYTES);

        let review = build_review(storage.path(), &reviews, &spoken_text, "Zephyr", "", vec![]);

        assert_eq!(review.exact_text, spoken_text);
        assert_eq!(review.request_segments.concat(), spoken_text);
        assert!(review.request_segments.len() > 1);
        assert!(
            review
                .request_segments
                .iter()
                .all(|segment| segment.len() <= MAX_TTS_REQUEST_TEXT_BYTES)
        );
    }

    #[test]
    fn narration_review_rejects_one_utf8_byte_over_the_section_limit() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let spoken_text = format!("{}界", "x".repeat(MAX_SPOKEN_TEXT_BYTES - 2));
        assert_eq!(spoken_text.len(), MAX_SPOKEN_TEXT_BYTES + 1);

        let result = build_request_review(
            storage.path(),
            &reviews,
            BuildNarrationReviewInput {
                spoken_text,
                voice: "Zephyr".to_owned(),
                style: String::new(),
                lexicon: vec![],
            },
        );

        let ProviderCommandResult::Error { error } = result else {
            panic!("a script over the UTF-8 byte limit must be rejected before review");
        };
        assert_eq!(error.kind, ProviderFailureKind::InvalidReview);
        assert!(error.message.contains("16,000 UTF-8 bytes"));
    }

    #[test]
    fn combined_wav_keeps_segment_audio_in_order_with_a_short_pause() {
        let first = valid_test_wav(1);
        let second = valid_test_wav(2);
        let combined = join_wav_segments(&[first.clone(), second.clone()])
            .expect("valid WAV segments should join");
        assert!(is_wav(&combined));
        assert_eq!(wav_pcm_data(&combined).unwrap().len(), 4 + 9_600 + 4);
        assert_eq!(&combined[44..48], wav_pcm_data(&first).unwrap());
        assert!(combined[48..48 + 9_600].iter().all(|byte| *byte == 0));
        assert_eq!(
            &combined[48 + 9_600..52 + 9_600],
            wav_pcm_data(&second).unwrap()
        );
    }

    #[test]
    fn review_exposes_each_exact_request_segment_for_consent() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let text = format!("{} First sentence. Second sentence.", "word ".repeat(900));
        let review = build_review(storage.path(), &reviews, &text, "Kore", "", vec![]);
        assert_eq!(review.request_segments.concat(), review.exact_text);
        assert!(review.request_segments.len() > 1);
        assert!(
            review
                .request_segments
                .iter()
                .all(|segment| segment.len() <= MAX_TTS_REQUEST_TEXT_BYTES)
        );
        for segment in &review.request_segments {
            let mut request_review = review.clone();
            request_review.exact_text = segment.clone();
            assert_eq!(
                build_request_body(&request_review)["input"][0]["content"][0]["text"],
                *segment
            );
        }
    }

    #[test]
    fn outgoing_tts_body_contains_only_the_reviewed_script_and_speech_settings() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let review = build_review(
            storage.path(),
            &reviews,
            "The retry queue waits, then runs the callback.",
            "Charon",
            "Clear and measured.",
            vec![],
        );

        let body = build_request_body(&review);
        assert_eq!(body["model"], MODEL_ID);
        assert_eq!(body["input"][0]["type"], "user_input");
        assert_eq!(body["input"][0]["content"].as_array().unwrap().len(), 1);
        assert_eq!(body["input"][0]["content"][0]["type"], "text");
        assert_eq!(body["input"][0]["content"][0]["text"], review.exact_text);
        assert_eq!(
            body["input"][0]["content"][0]["annotations"][0]["type"],
            "speech_metadata"
        );
        assert_eq!(
            body["input"][0]["content"][0]["annotations"][0]["style"],
            review.style
        );
        assert_eq!(body["response_format"]["type"], "audio");
        assert_eq!(body["response_format"]["mime_type"], "audio/wav");
        assert_eq!(
            body["generation_config"]["speech_config"][0]["voice"],
            review.voice
        );
        assert!(body.get("source").is_none());
        assert!(body.get("files").is_none());
        assert!(body.get("citations").is_none());
    }

    #[test]
    fn decoder_accepts_expected_pcm_wav_and_rejects_incomplete_or_wrong_format_audio() {
        let audio = valid_test_wav(0);
        assert!(is_wav(&audio));
        let payload = json!({
            "steps": [{
                "type": "model_output",
                "content": [{
                    "type": "audio",
                    "data": base64::engine::general_purpose::STANDARD.encode(&audio),
                }],
            }],
        });
        let decoded = decode_audio_response(payload.to_string().as_bytes())
            .expect("documented audio response should decode");
        assert_eq!(decoded, audio);

        let mut wrong_sample_rate = audio.clone();
        wrong_sample_rate[24..28].copy_from_slice(&16_000_u32.to_le_bytes());
        wrong_sample_rate[28..32].copy_from_slice(&32_000_u32.to_le_bytes());
        let malformed = json!({
            "steps": [{
                "type": "model_output",
                "content": [{
                    "type": "audio",
                    "data": base64::engine::general_purpose::STANDARD.encode(wrong_sample_rate),
                }],
            }],
        });
        assert_eq!(
            decode_audio_response(malformed.to_string().as_bytes())
                .unwrap_err()
                .kind,
            ProviderFailureKind::MalformedProviderResponse
        );
    }

    #[test]
    fn review_contains_only_the_edited_speech_and_included_pronunciation_substitutions() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let review = build_review(
            storage.path(),
            &reviews,
            "The retry policy wraps retry.execute and remains unchanged.",
            "Kore",
            "Speak calmly.",
            vec![
                PronunciationEntry {
                    name: "retry.execute".to_owned(),
                    say_as: "retry execute".to_owned(),
                    include: true,
                },
                PronunciationEntry {
                    name: "src/server.rs".to_owned(),
                    say_as: "server path".to_owned(),
                    include: false,
                },
            ],
        );

        assert_eq!(
            review.exact_text,
            "The retry policy wraps retry execute and remains unchanged."
        );
        assert_eq!(
            review.spoken_text_sha256,
            sha256(review.exact_text.as_bytes())
        );
        let serialized = serde_json::to_string(&review).expect("review should serialize");
        assert!(!serialized.contains("sourceExcerpt"));
        assert!(!serialized.contains("filePath"));
        assert!(!serialized.contains("src/server.rs"));
    }

    #[test]
    fn pocket_cache_identity_tracks_runtime_policy_and_output_format() {
        let text = "The local runtime speaks this section.";
        let segments = vec![text.to_owned()];
        let key_for = |model: &str, runtime: &str, policy: &str, format: &str| {
            audio_cache_key(
                &sha256(text.as_bytes()),
                &segments,
                model,
                crate::pocket_tts::VOICE,
                "",
                format,
                || runtime.to_owned(),
                policy,
            )
        };
        let current = key_for(
            crate::pocket_tts::MODEL_ID,
            "runtime-a",
            "segments-a",
            "audio/wav",
        );

        assert_eq!(
            current,
            key_for(
                crate::pocket_tts::MODEL_ID,
                "runtime-a",
                "segments-a",
                "audio/wav"
            )
        );
        assert_ne!(
            current,
            key_for("pocket-tts-next", "runtime-a", "segments-a", "audio/wav")
        );
        assert_ne!(
            current,
            key_for(
                crate::pocket_tts::MODEL_ID,
                "runtime-b",
                "segments-a",
                "audio/wav"
            )
        );
        assert_ne!(
            current,
            key_for(
                crate::pocket_tts::MODEL_ID,
                "runtime-a",
                "segments-b",
                "audio/wav"
            )
        );
        assert_ne!(
            current,
            key_for(
                crate::pocket_tts::MODEL_ID,
                "runtime-a",
                "segments-a",
                "audio/pcm"
            )
        );

        let whole = request_fingerprint(&current, LOCAL_DISCLOSURE_VERSION, &segments);
        let split = request_fingerprint(
            &current,
            LOCAL_DISCLOSURE_VERSION,
            &["The local runtime speaks".into(), "this section.".into()],
        );
        assert_ne!(
            whole, split,
            "the exact request segmentation is bound into the review"
        );
    }

    #[test]
    fn cache_is_reused_for_matching_speech_and_invalidated_by_text_voice_or_style() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let base = build_review(
            storage.path(),
            &reviews,
            "Explain the cache.",
            "Kore",
            "Warm",
            vec![],
        );
        let audio = valid_test_wav(7);
        write_cached_audio(storage.path(), &base.cache_key, &base.model_id, &audio)
            .expect("valid WAV should cache");

        let same = build_review(
            storage.path(),
            &reviews,
            "Explain the cache.",
            "Kore",
            "Warm",
            vec![],
        );
        let changed_text = build_review(
            storage.path(),
            &reviews,
            "Explain this cache.",
            "Kore",
            "Warm",
            vec![],
        );
        let changed_voice = build_review(
            storage.path(),
            &reviews,
            "Explain the cache.",
            "Puck",
            "Warm",
            vec![],
        );
        let changed_style = build_review(
            storage.path(),
            &reviews,
            "Explain the cache.",
            "Kore",
            "Gentle",
            vec![],
        );

        assert_eq!(same.fingerprint, base.fingerprint);
        assert_eq!(same.cache_key, base.cache_key);
        assert!(same.cached_audio_available);
        assert_ne!(
            request_fingerprint(&same.cache_key, "later-disclosure", &same.request_segments),
            same.fingerprint,
            "a changed disclosure must require a fresh confirmation"
        );
        assert!(
            read_cached_audio(storage.path(), &same.cache_key, &same.model_id).is_some(),
            "the same reviewed speech can reuse audio when disclosure metadata changes"
        );
        for changed in [changed_text, changed_voice, changed_style] {
            assert_ne!(changed.cache_key, base.cache_key);
            assert_ne!(changed.fingerprint, base.fingerprint);
            assert!(!changed.cached_audio_available);
        }
    }

    #[test]
    fn pocket_cache_identity_tracks_enabled_pronunciation_edits() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let make_review = |lexicon| match build_local_request_review(
            storage.path(),
            &reviews,
            BuildNarrationReviewInput {
                spoken_text: "PFR selects a reader.".into(),
                voice: crate::pocket_tts::VOICE.into(),
                style: String::new(),
                lexicon,
            },
        ) {
            ProviderCommandResult::Ok { data } => data,
            ProviderCommandResult::Error { error } => {
                panic!("Pocket review should build: {error:?}")
            }
        };
        let original = make_review(Vec::new());
        let same = make_review(Vec::new());
        let disabled = make_review(vec![PronunciationEntry {
            name: "PFR".into(),
            say_as: "P F R".into(),
            include: false,
        }]);
        let changed_pronunciation = make_review(vec![PronunciationEntry {
            name: "PFR".into(),
            say_as: "P F R".into(),
            include: true,
        }]);

        assert_eq!(same.exact_text, original.exact_text);
        assert_eq!(same.cache_key, original.cache_key);
        assert_eq!(disabled.exact_text, original.exact_text);
        assert_eq!(disabled.cache_key, original.cache_key);
        assert_eq!(changed_pronunciation.exact_text, "P F R selects a reader.");
        assert_ne!(changed_pronunciation.cache_key, original.cache_key);
        assert_ne!(changed_pronunciation.fingerprint, original.fingerprint);
    }

    #[test]
    fn cached_audio_can_be_loaded_without_a_provider_request() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let review = build_review(
            storage.path(),
            &reviews,
            "Read from the local cache.",
            "Zephyr",
            "",
            vec![],
        );
        let expected = valid_test_wav(19);
        write_cached_audio(
            storage.path(),
            &review.cache_key,
            &review.model_id,
            &expected,
        )
        .expect("valid WAV should cache");

        let loaded = load_reviewed_audio(
            storage.path(),
            &reviews,
            &review.review_id,
            &review.fingerprint,
        );
        let ProviderCommandResult::Ok { data } = loaded else {
            panic!("cached audio should load without API credentials or network access");
        };
        assert!(data.cache_hit);
        assert_eq!(data.mime_type, "audio/wav");
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(data.audio_base64)
                .expect("cached audio should be base64 encoded"),
            expected
        );
    }

    #[test]
    fn cached_pocket_audio_reloads_after_in_memory_state_is_recreated() {
        let storage = TestStorage::new();
        let first_reviews = PendingNarrationReviews::default();
        let input = BuildNarrationReviewInput {
            spoken_text: "Read this cached Pocket clip after restart.".into(),
            voice: crate::pocket_tts::VOICE.into(),
            style: String::new(),
            lexicon: Vec::new(),
        };
        let first = match build_local_request_review(&storage.0, &first_reviews, input.clone()) {
            ProviderCommandResult::Ok { data } => data,
            ProviderCommandResult::Error { error } => {
                panic!("local Pocket review should build: {error:?}")
            }
        };
        let expected = valid_test_wav(29);
        write_cached_audio(&storage.0, &first.cache_key, &first.model_id, &expected)
            .expect("valid local WAV should be cached");

        // A restart loses reviews and prewarm-queue state but keeps app storage.
        let reopened_reviews = PendingNarrationReviews::default();
        let reopened = match build_local_request_review(&storage.0, &reopened_reviews, input) {
            ProviderCommandResult::Ok { data } => data,
            ProviderCommandResult::Error { error } => {
                panic!("the reopened local Pocket review should build: {error:?}")
            }
        };
        assert_eq!(reopened.cache_key, first.cache_key);
        assert!(reopened.cached_audio_available);

        // A new app process also starts with an empty prewarm queue. It should
        // discover the persisted clip, report it ready, and avoid scheduling a
        // second synthesis job.
        let reopened_queue = NarrationPrewarmQueue::default();
        let reopened_input = BuildNarrationReviewInput {
            spoken_text: "Read this cached Pocket clip after restart.".into(),
            voice: crate::pocket_tts::VOICE.into(),
            style: String::new(),
            lexicon: Vec::new(),
        };
        let status =
            local_audio_preparation_status(&storage.0, &reopened_queue, reopened_input.clone());
        let ProviderCommandResult::Ok { data } = status else {
            panic!("the reopened app should find its cached Pocket clip");
        };
        assert_eq!(data.state, NarrationPrewarmState::Ready);
        assert!(data.cached_audio_available);
        assert_eq!(reopened_queue.status(&first.cache_key), None);

        let prewarm = prewarm_local_audio(
            &storage.0,
            &reopened_reviews,
            &reopened_queue,
            reopened_input,
        );
        let ProviderCommandResult::Ok { data } = prewarm else {
            panic!("generation should reuse the persisted Pocket clip");
        };
        assert_eq!(data.state, NarrationPrewarmState::Ready);
        assert!(data.cached_audio_available);
        let queue_state = reopened_queue
            .0
            .lock()
            .expect("reopened prewarm queue should remain available");
        assert!(queue_state.pending.is_empty());
        assert!(!queue_state.worker_running);
        assert!(queue_state.entries.is_empty());
        drop(queue_state);

        let loaded = load_reviewed_audio(
            &storage.0,
            &reopened_reviews,
            &reopened.review_id,
            &reopened.fingerprint,
        );
        let ProviderCommandResult::Ok { data } = loaded else {
            panic!("the cached Pocket WAV should load using only local storage");
        };
        assert!(data.cache_hit);
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(data.audio_base64)
                .expect("cached Pocket WAV should be base64 encoded"),
            expected
        );
    }

    #[test]
    fn review_fingerprint_is_bound_and_consumed_once() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let review = build_review(
            storage.path(),
            &reviews,
            "One reviewed request.",
            "Leda",
            "",
            vec![],
        );

        let wrong_fingerprint_error = match reviews.consume(&review.review_id, "wrong-fingerprint")
        {
            Ok(_) => panic!("a mismatched fingerprint must not consume a review"),
            Err(error) => error,
        };
        assert_eq!(
            wrong_fingerprint_error.kind,
            ProviderFailureKind::ReviewChanged
        );

        let second_use_error = match reviews.consume(&review.review_id, &review.fingerprint) {
            Ok(_) => panic!("a consumed review must not be reusable"),
            Err(error) => error,
        };
        assert_eq!(
            second_use_error.kind,
            ProviderFailureKind::ReviewUnavailable
        );
    }

    #[test]
    fn invalid_wav_does_not_replace_previously_cached_audio() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let review = build_review(
            storage.path(),
            &reviews,
            "Keep the old audio.",
            "Aoede",
            "",
            vec![],
        );
        let previous = valid_test_wav(23);
        write_cached_audio(
            storage.path(),
            &review.cache_key,
            &review.model_id,
            &previous,
        )
        .expect("valid WAV should cache");

        assert_eq!(
            write_cached_audio(
                storage.path(),
                &review.cache_key,
                &review.model_id,
                b"not-a-wave",
            )
            .unwrap_err()
            .kind,
            ProviderFailureKind::MalformedProviderResponse
        );
        assert_eq!(
            read_cached_audio(storage.path(), &review.cache_key, &review.model_id),
            Some(previous)
        );
    }

    #[test]
    fn partial_cache_temp_write_is_removed_after_failure() {
        let storage = TestStorage::new();
        let temporary = storage.path().join(".partial-audio.tmp");

        let result = write_cache_temporary(&temporary, || {
            fs::write(&temporary, b"partial WAV bytes")?;
            Err(std::io::Error::other("simulated storage failure"))
        });

        assert!(result.is_err());
        assert!(!temporary.exists());
        assert_eq!(fs::read_dir(storage.path()).unwrap().count(), 0);
    }
}
