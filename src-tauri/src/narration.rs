//! Reviewed per-section narration using the managed local Pocket TTS runtime.

use crate::provider::{ProviderCommandResult, ProviderFailure, ProviderFailureKind};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
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

const POCKET_CACHE_VERSION: &str = "pocket-narration-cache-v1";
const POCKET_REVIEW_VERSION: &str = "pocket-narration-2026-10-15-v1";
const REVIEW_LIFETIME_MS: u64 = 15 * 60 * 1_000;
const MAX_PENDING_REVIEWS: usize = 16;
const MAX_SPOKEN_TEXT_BYTES: usize = 16_000;
const MAX_POCKET_SEGMENT_BYTES: usize = 4_000;
const MAX_LEXICON_ENTRIES: usize = 12;
const MAX_AUDIO_BYTES: usize = 12 * 1024 * 1024;
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
        let (cancellation, work) = {
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
            (entry.cancellation.take(), entry.work.take())
        };
        drop(work);
        if let Some(cancellation) = cancellation {
            let _ = cancellation.send(());
        }
        true
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

pub fn build_local_request_review(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    input: BuildNarrationReviewInput,
) -> ProviderCommandResult<NarrationRequestReview> {
    let review = match create_review(storage_dir, input) {
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
) -> Result<NarrationRequestReview, ProviderFailure> {
    let input = validate_local_input(input)?;
    let model_id = crate::pocket_tts::MODEL_ID;
    let exact_text = apply_lexicon(&input.spoken_text, &input.lexicon);
    if exact_text.trim().is_empty() || exact_text.len() > MAX_SPOKEN_TEXT_BYTES {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Pronunciation substitutions must leave this section's narration between 1 and 16,000 UTF-8 bytes.",
        ));
    }
    let request_segments = split_spoken_text(&exact_text, MAX_POCKET_SEGMENT_BYTES);
    let spoken_text_sha256 = sha256(exact_text.as_bytes());
    let format = "audio/wav".to_owned();
    let cache_key = audio_cache_key(
        &spoken_text_sha256,
        &request_segments,
        &input.voice,
        &input.style,
        &format,
        crate::pocket_tts::runtime_fingerprint,
        POCKET_SEGMENT_POLICY,
    );
    let fingerprint = request_fingerprint(&cache_key, POCKET_REVIEW_VERSION, &request_segments);
    let cached_audio_available = read_cached_audio(storage_dir, &cache_key, model_id).is_some();
    let expires_at_unix_ms = now_unix_ms().saturating_add(REVIEW_LIFETIME_MS);
    Ok(NarrationRequestReview {
        review_id: Uuid::new_v4().to_string(),
        exact_text,
        request_segments,
        spoken_text_sha256,
        disclosure_version: POCKET_REVIEW_VERSION.to_owned(),
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
    let review = match create_review(storage_dir, input) {
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
        matches!(
            synthesize_reviewed_audio(
                &storage,
                &task_reviews,
                &review_id,
                &fingerprint,
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
    let Ok(review) = create_review(storage_dir, input) else {
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
    let review = match create_review(storage_dir, input) {
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
    if let Err(error) = check_audio_request_cancellation(cancellation) {
        return Some(ProviderCommandResult::failed(error));
    }
    if let Some(bytes) = read_cached_audio(
        storage_dir,
        &pending.review.cache_key,
        &pending.review.model_id,
    ) {
        if let Err(error) = check_audio_request_cancellation(cancellation) {
            return Some(ProviderCommandResult::failed(error));
        }
        return Some(ProviderCommandResult::ok(audio_result(
            &pending.review,
            bytes,
            true,
        )));
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

/// Return the prepared local clip and release its per-playback review slot.
/// If Play reaches this path before a renderer-queued prewarm enters the native
/// queue, admit the selected clip there and move it ahead of background work.
/// If bounded admission is unavailable, the caller can retain ordinary on-
/// demand synthesis as its recovery path.
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

    let result = if result.is_some() {
        result
    } else {
        let review = reviews.get(review_id, fingerprint).ok()?.review;
        if review.model_id != crate::pocket_tts::MODEL_ID {
            return None;
        }

        let cache_key = review.cache_key.clone();
        let task_cache_key = cache_key.clone();
        let task_model_id = review.model_id.clone();
        let task_segments = review.request_segments.clone();
        let task_storage = storage_dir.to_path_buf();
        if queue
            .schedule(
                cache_key.clone(),
                move |mut queue_cancellation| async move {
                    let Ok(bytes) = crate::pocket_tts::synthesize_segments(
                        &task_storage,
                        &task_segments,
                        &mut queue_cancellation,
                    )
                    .await
                    else {
                        return false;
                    };
                    if check_audio_request_cancellation(&mut queue_cancellation).is_err()
                        || write_cached_audio(
                            &task_storage,
                            &task_cache_key,
                            &task_model_id,
                            &bytes,
                        )
                        .is_err()
                    {
                        return false;
                    }
                    read_cached_audio(&task_storage, &task_cache_key, &task_model_id).is_some()
                },
            )
            .is_err()
        {
            None
        } else {
            queue.prioritize(&cache_key);
            load_prewarmed_audio_if_ready(
                storage_dir,
                reviews,
                queue,
                review_id,
                fingerprint,
                cancellation,
            )
            .await
        }
    };

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
    cancellation: &mut oneshot::Receiver<()>,
) -> ProviderCommandResult<NarrationAudio> {
    let current = match reviews.get(review_id, fingerprint) {
        Ok(pending) => pending,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    if current.review.model_id != crate::pocket_tts::MODEL_ID {
        return ProviderCommandResult::error(
            ProviderFailureKind::ReviewChanged,
            "This saved narration request is unsupported. Create a new Pocket TTS request.",
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

fn validate_local_input(
    mut input: BuildNarrationReviewInput,
) -> Result<BuildNarrationReviewInput, ProviderFailure> {
    input.spoken_text = input.spoken_text.trim().to_owned();
    if input.spoken_text.is_empty() || input.spoken_text.len() > MAX_SPOKEN_TEXT_BYTES {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Narration text must contain 1 to 16,000 UTF-8 bytes for this section. Longer scripts need to be divided into separate sections.",
        ));
    }
    if input.voice != crate::pocket_tts::VOICE || !input.style.is_empty() {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Pocket TTS uses the Alba voice and does not support delivery-style prompts. Review local voice settings again.",
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
    voice: &str,
    style: &str,
    format: &str,
    pocket_runtime_fingerprint: impl FnOnce() -> String,
    pocket_segment_policy: &str,
) -> String {
    let mut contract = json!({
        "cacheVersion": POCKET_CACHE_VERSION,
        "spokenTextSha256": text_hash,
        "provider": "pocket-tts",
        "model": crate::pocket_tts::MODEL_ID,
        "voice": voice,
        "style": style,
        "format": format,
        "packageVersion": crate::pocket_tts::PACKAGE_VERSION,
        // Include the worker, asset revisions, Python and package pins. A new
        // runtime contract also changes the exact review fingerprint below.
        "runtimeFingerprint": pocket_runtime_fingerprint(),
        "pcmFormat": "pcm16-mono-24000",
        "segmentPolicy": pocket_segment_policy,
    });
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
pub(crate) fn audio_cache_key_for_script(exact_text: &str) -> String {
    let request_segments = split_spoken_text(exact_text, MAX_POCKET_SEGMENT_BYTES);
    audio_cache_key(
        &sha256(exact_text.as_bytes()),
        &request_segments,
        crate::pocket_tts::VOICE,
        "",
        "audio/wav",
        crate::pocket_tts::runtime_fingerprint,
        POCKET_SEGMENT_POLICY,
    )
}

fn request_fingerprint(
    cache_key: &str,
    review_contract_version: &str,
    request_segments: &[String],
) -> String {
    let contract = json!({
        "cacheKey": cache_key,
        "reviewContractVersion": review_contract_version,
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

fn cache_path(storage_dir: &Path, fingerprint: &str, _model_id: &str) -> PathBuf {
    cache_path_for_version(storage_dir, fingerprint, POCKET_CACHE_VERSION)
}

fn read_cached_audio(storage_dir: &Path, fingerprint: &str, model_id: &str) -> Option<Vec<u8>> {
    if model_id != crate::pocket_tts::MODEL_ID {
        return None;
    }
    let current = cache_path(storage_dir, fingerprint, model_id);
    if let Some(parent) = current.parent() {
        if claim_initial_pocket_cache_sweep(parent) {
            prune_pocket_audio_cache(parent, Some(&current));
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
        .take(MAX_AUDIO_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= MAX_AUDIO_BYTES && is_wav(&bytes)).then_some(bytes)
}

fn write_cached_audio(
    storage_dir: &Path,
    fingerprint: &str,
    model_id: &str,
    bytes: &[u8],
) -> Result<(), ProviderFailure> {
    if model_id != crate::pocket_tts::MODEL_ID {
        return Err(failure(
            ProviderFailureKind::ReviewChanged,
            "This saved narration request is unsupported. Create a new Pocket TTS request.",
        ));
    }
    if bytes.len() > MAX_AUDIO_BYTES || !is_wav(bytes) {
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
    if let Some(parent) = path.parent() {
        prune_pocket_audio_cache(parent, Some(&path));
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
    use std::io::Read;
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

    #[tokio::test]
    async fn cancelling_active_prewarm_keeps_the_worker_available_for_later_audio() {
        let queue = NarrationPrewarmQueue::default();
        let (started_tx, started_rx) = oneshot::channel();
        let (cancelled_tx, cancelled_rx) = oneshot::channel();
        queue
            .schedule("cancelled".into(), move |cancellation| async move {
                let _ = started_tx.send(());
                let _ = cancellation.await;
                let _ = cancelled_tx.send(());
                false
            })
            .expect("the active prewarm should be admitted");
        started_rx
            .await
            .expect("the active prewarm should start before cancellation");

        assert!(queue.cancel_preparation("cancelled"));
        assert_eq!(
            queue.status("cancelled"),
            Some(NarrationPrewarmState::Unavailable)
        );
        let following = queue
            .schedule("following".into(), |_| async { true })
            .expect("later audio should remain admissible after cancellation");
        assert!(following.accepted);

        tokio::time::timeout(Duration::from_secs(2), async {
            cancelled_rx
                .await
                .expect("the cancelled worker should observe its signal");
            while queue.status("following") != Some(NarrationPrewarmState::Ready) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("cancellation should release the worker for queued audio");
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
    fn pocket_cache_does_not_read_the_pre_greenfield_cache_directory() {
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

        // The current Pocket cache contract does not search an older directory.
        let previous_audio = valid_test_wav(31);
        let old_path =
            cache_path_for_version(&storage.0, &pocket.cache_key, "gemini-narration-cache-v1");
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
        assert!(read_cached_audio(&storage.0, &pocket.cache_key, "unsupported-model").is_none());
        assert_eq!(
            write_cached_audio(
                &storage.0,
                &pocket.cache_key,
                "unsupported-model",
                &previous_audio,
            )
            .unwrap_err()
            .kind,
            ProviderFailureKind::ReviewChanged
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
    async fn playback_reuses_cached_pocket_audio_without_a_native_queue_entry() {
        let storage = TestStorage::new();
        let queue = NarrationPrewarmQueue::default();
        let reviews = PendingNarrationReviews::default();
        let review = match build_local_request_review(
            storage.path(),
            &reviews,
            BuildNarrationReviewInput {
                spoken_text: "A clip can survive an app restart.".into(),
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
        let expected_audio = valid_test_wav(43);
        write_cached_audio(
            storage.path(),
            &review.cache_key,
            &review.model_id,
            &expected_audio,
        )
        .expect("the existing clip should be cached");

        let (_cancel_sender, mut cancellation) = oneshot::channel();
        let result = load_prewarmed_audio_for_playback(
            storage.path(),
            &reviews,
            &queue,
            &review.review_id,
            &review.fingerprint,
            &mut cancellation,
        )
        .await;

        let Some(ProviderCommandResult::Ok { data }) = result else {
            panic!("cached local audio should play without queueing synthesis");
        };
        assert!(data.cache_hit);
        assert_eq!(
            base64::engine::general_purpose::STANDARD
                .decode(data.audio_base64)
                .expect("returned audio should be valid base64"),
            expected_audio,
        );
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

    fn build_review(
        storage: &Path,
        reviews: &PendingNarrationReviews,
        spoken_text: &str,
        voice: &str,
        style: &str,
        lexicon: Vec<PronunciationEntry>,
    ) -> NarrationRequestReview {
        assert_eq!(voice, crate::pocket_tts::VOICE);
        assert!(style.is_empty());
        match build_local_request_review(
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
            remaining: MAX_AUDIO_BYTES + 100,
            bytes_read: 0,
        };
        assert!(read_cached_audio_from(&mut reader).is_none());
        assert_eq!(reader.bytes_read, MAX_AUDIO_BYTES + 1);
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

        let review = build_review(
            storage.path(),
            &reviews,
            &spoken_text,
            crate::pocket_tts::VOICE,
            "",
            vec![],
        );

        assert_eq!(review.exact_text, spoken_text);
        assert_eq!(review.request_segments.concat(), spoken_text);
        assert!(review.request_segments.len() > 1);
        assert!(
            review
                .request_segments
                .iter()
                .all(|segment| segment.len() <= MAX_POCKET_SEGMENT_BYTES)
        );
    }

    #[test]
    fn narration_review_rejects_one_utf8_byte_over_the_section_limit() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let spoken_text = format!("{}界", "x".repeat(MAX_SPOKEN_TEXT_BYTES - 2));
        assert_eq!(spoken_text.len(), MAX_SPOKEN_TEXT_BYTES + 1);

        let result = build_local_request_review(
            storage.path(),
            &reviews,
            BuildNarrationReviewInput {
                spoken_text,
                voice: crate::pocket_tts::VOICE.to_owned(),
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
    fn review_contains_only_the_edited_speech_and_included_pronunciation_substitutions() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let review = build_review(
            storage.path(),
            &reviews,
            "The retry policy wraps retry.execute and remains unchanged.",
            crate::pocket_tts::VOICE,
            "",
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
        let key_for = |runtime: &str, policy: &str, format: &str| {
            audio_cache_key(
                &sha256(text.as_bytes()),
                &segments,
                crate::pocket_tts::VOICE,
                "",
                format,
                || runtime.to_owned(),
                policy,
            )
        };
        let current = key_for("runtime-a", "segments-a", "audio/wav");

        assert_eq!(current, key_for("runtime-a", "segments-a", "audio/wav"));
        assert_ne!(current, key_for("runtime-b", "segments-a", "audio/wav"));
        assert_ne!(current, key_for("runtime-a", "segments-b", "audio/wav"));
        assert_ne!(current, key_for("runtime-a", "segments-a", "audio/pcm"));

        let whole = request_fingerprint(&current, POCKET_REVIEW_VERSION, &segments);
        let split = request_fingerprint(
            &current,
            POCKET_REVIEW_VERSION,
            &["The local runtime speaks".into(), "this section.".into()],
        );
        assert_ne!(
            whole, split,
            "the exact request segmentation is bound into the review"
        );
    }

    #[test]
    fn cache_is_reused_for_matching_speech_and_invalidated_by_text() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        let base = build_review(
            storage.path(),
            &reviews,
            "Explain the cache.",
            crate::pocket_tts::VOICE,
            "",
            vec![],
        );
        let audio = valid_test_wav(7);
        write_cached_audio(storage.path(), &base.cache_key, &base.model_id, &audio)
            .expect("valid WAV should cache");

        let same = build_review(
            storage.path(),
            &reviews,
            "Explain the cache.",
            crate::pocket_tts::VOICE,
            "",
            vec![],
        );
        let changed_text = build_review(
            storage.path(),
            &reviews,
            "Explain this cache.",
            crate::pocket_tts::VOICE,
            "",
            vec![],
        );
        assert_eq!(same.fingerprint, base.fingerprint);
        assert_eq!(same.cache_key, base.cache_key);
        assert!(same.cached_audio_available);
        assert!(
            read_cached_audio(storage.path(), &same.cache_key, &same.model_id).is_some(),
            "the same reviewed speech should reuse local audio"
        );
        assert_ne!(changed_text.cache_key, base.cache_key);
        assert_ne!(changed_text.fingerprint, base.fingerprint);
        assert!(!changed_text.cached_audio_available);
    }

    #[test]
    fn pocket_review_rejects_unsupported_voice_and_style() {
        let storage = TestStorage::new();
        let reviews = PendingNarrationReviews::default();
        for (voice, style) in [("Kore", ""), (crate::pocket_tts::VOICE, "Warm style")] {
            let result = build_local_request_review(
                storage.path(),
                &reviews,
                BuildNarrationReviewInput {
                    spoken_text: "The same script stays local.".into(),
                    voice: voice.into(),
                    style: style.into(),
                    lexicon: Vec::new(),
                },
            );
            assert!(matches!(
                result,
                ProviderCommandResult::Error { error }
                    if error.kind == ProviderFailureKind::InvalidReview
            ));
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
            crate::pocket_tts::VOICE,
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
            crate::pocket_tts::VOICE,
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
            crate::pocket_tts::VOICE,
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

    #[test]
    fn pocket_audio_cache_pruning_keeps_recent_entry_within_file_limit() {
        let storage = TestStorage::new();
        let cache_dir = storage.path().join("pocket-cache");
        fs::create_dir_all(&cache_dir).expect("cache directory should be created");

        let oldest = cache_dir.join(format!("{:064x}.wav", 0));
        let newest = cache_dir.join(format!("{:064x}.wav", MAX_POCKET_CACHE_FILES));
        for index in 0..=MAX_POCKET_CACHE_FILES {
            let path = cache_dir.join(format!("{index:064x}.wav"));
            fs::write(&path, [index as u8]).expect("cache entry should be written");
            fs::File::open(&path)
                .expect("cache entry should be open")
                .set_modified(UNIX_EPOCH + Duration::from_secs(index as u64 + 1))
                .expect("cache access time should be controllable");
        }

        prune_pocket_audio_cache(&cache_dir, Some(&newest));

        let remaining = fs::read_dir(&cache_dir)
            .expect("cache directory should remain readable")
            .count();
        assert_eq!(remaining, MAX_POCKET_CACHE_FILES);
        assert!(!oldest.exists(), "oldest cached audio should be pruned");
        assert!(newest.exists(), "the just-used audio should be preserved");
    }
}
