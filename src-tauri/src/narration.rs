//! Explicitly reviewed, per-section Google Gemini text-to-speech requests.
//!
//! Only the edited speech transcript and optional pronunciation substitutions
//! leave the machine. Source files, evidence excerpts, and paths are never part
//! of a narration request.

use crate::provider::{
    ProviderCommandResult, ProviderFailure, ProviderFailureKind, is_region_restriction_error,
    load_api_key,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::oneshot;
use uuid::Uuid;

const MODEL_ID: &str = "gemini-3.8-flash-lite-tts";
const ENDPOINT: &str = "https://generativelanguage.googleapis.com/v1beta/interactions";
const CACHE_VERSION: &str = "gemini-narration-cache-v1";
const DISCLOSURE_VERSION: &str = "gemini-tts-disclosure-2026-10-13-v1";
const REVIEW_LIFETIME_MS: u64 = 15 * 60 * 1_000;
const MAX_PENDING_REVIEWS: usize = 16;
const MAX_SPOKEN_TEXT_BYTES: usize = 4_000;
const MAX_STYLE_BYTES: usize = 160;
const MAX_LEXICON_ENTRIES: usize = 12;
const MAX_RESPONSE_BYTES: usize = 12 * 1024 * 1024;

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
        data_use_terms: provider_disclosure.unpaid_service_data_use,
        paid_data_use_terms: "For Paid Services, Google says it does not use prompts or responses to improve its products and processes them under its Data Processing Addendum. It may log them for a limited period solely to detect and prevent prohibited-use violations, maintain service safety and security, and make required legal or regulatory disclosures.".to_owned(),
        regional_data_use_terms: "If you are in the European Economic Area, Switzerland, or the United Kingdom, Google says the Paid Services data-use terms apply to all Gemini API services, including unpaid quota. Check Google's current terms for your account region.".to_owned(),
        eligibility_terms: "Google's terms require API users to be at least 18, use the service for professional or business development rather than consumer use, and access it only in supported regions. If you make an API client available to users in the EEA, Switzerland, or the UK, the terms require Paid Services.".to_owned(),
        billing_uncertainty: "Code Atlas cannot inspect this key's project tier, active billing account, or remaining quota. Google treats Gemini API access through a Cloud project with an active billing account as a Paid Service. Check the project's tier and billing in Google AI Studio before sending; a Free Tier model price does not guarantee that this key's request is free or available.".to_owned(),
        terms_url: "https://ai.google.dev/gemini-api/terms".to_owned(),
        pricing_url: "https://ai.google.dev/gemini-api/docs/pricing".to_owned(),
        regions_url: "https://ai.google.dev/gemini-api/docs/available-regions".to_owned(),
        api_key_url: "https://aistudio.google.com/app/apikey".to_owned(),
        request_scope: "One selected section's exact narration script, optional pronunciation substitutions, the selected prebuilt voice, delivery style, and WAV format. No source excerpt or whole-chapter context is included.".to_owned(),
        voices: PREBUILT_VOICES.iter().map(|voice| (*voice).to_owned()).collect(),
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
    pub spoken_text_sha256: String,
    pub model_id: String,
    pub voice: String,
    pub style: String,
    pub format: String,
    pub fingerprint: String,
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
    let input = match validate_input(input) {
        Ok(input) => input,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    let exact_text = apply_lexicon(&input.spoken_text, &input.lexicon);
    if exact_text.trim().is_empty() || exact_text.len() > MAX_SPOKEN_TEXT_BYTES {
        return ProviderCommandResult::error(
            ProviderFailureKind::InvalidReview,
            "Pronunciation substitutions must leave this section's narration between 1 and 4,000 UTF-8 bytes.",
        );
    }
    let spoken_text_sha256 = sha256(exact_text.as_bytes());
    let format = "audio/wav".to_owned();
    let fingerprint = request_fingerprint(
        &spoken_text_sha256,
        MODEL_ID,
        &input.voice,
        &input.style,
        &format,
    );
    let cached_audio_available = read_cached_audio(storage_dir, &fingerprint).is_some();
    let expires_at_unix_ms = now_unix_ms().saturating_add(REVIEW_LIFETIME_MS);
    let review = NarrationRequestReview {
        review_id: Uuid::new_v4().to_string(),
        exact_text,
        spoken_text_sha256,
        model_id: MODEL_ID.to_owned(),
        voice: input.voice,
        style: input.style,
        format,
        fingerprint,
        cached_audio_available,
        expires_at_unix_ms,
    };
    match reviews.insert(PendingNarrationReview {
        review: review.clone(),
    }) {
        Ok(()) => ProviderCommandResult::ok(review),
        Err(error) => ProviderCommandResult::failed(error),
    }
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
    match read_cached_audio(storage_dir, &pending.review.fingerprint) {
        Some(bytes) => ProviderCommandResult::ok(audio_result(&pending.review, bytes, true)),
        None => ProviderCommandResult::error(
            ProviderFailureKind::ReviewUnavailable,
            "The reviewed narration has no local audio cache. Review it again before generating audio.",
        ),
    }
}

pub async fn synthesize_reviewed_audio(
    storage_dir: &Path,
    reviews: &PendingNarrationReviews,
    review_id: &str,
    fingerprint: &str,
    consent: &NarrationConsent,
    cancellation: &mut oneshot::Receiver<()>,
) -> ProviderCommandResult<NarrationAudio> {
    if !consent.is_complete() {
        return ProviderCommandResult::error(
            ProviderFailureKind::ConsentRequired,
            "Confirm the project tier and billing status, Google's data-use and eligibility terms, and authorization to send this exact narration request.",
        );
    }
    let pending = match consume_review(reviews, review_id, fingerprint) {
        Ok(pending) => pending,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    if let Some(bytes) = read_cached_audio(storage_dir, &pending.review.fingerprint) {
        return ProviderCommandResult::ok(audio_result(&pending.review, bytes, true));
    }
    let api_key = match load_api_key() {
        Ok(key) => key,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    let result =
        await_cancellable_audio_request(cancellation, request_audio(&pending.review, &api_key))
            .await;
    let bytes = match result {
        Ok(bytes) => bytes,
        Err(error) => return ProviderCommandResult::failed(error),
    };
    if let Err(_error) = write_cached_audio(storage_dir, &pending.review.fingerprint, &bytes) {
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
    tokio::select! {
        biased;
        _ = cancellation => Err(failure(
            ProviderFailureKind::RequestCancelled,
            "The narration request was cancelled locally. Google may already have received the reviewed speech text; no response was accepted. Review and confirm again before another send.",
        )),
        result = request => result,
    }
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

fn validate_input(
    mut input: BuildNarrationReviewInput,
) -> Result<BuildNarrationReviewInput, ProviderFailure> {
    input.spoken_text = input.spoken_text.trim().to_owned();
    input.style = input.style.trim().to_owned();
    if input.spoken_text.is_empty() || input.spoken_text.len() > MAX_SPOKEN_TEXT_BYTES {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Narration text must contain 1 to 4,000 UTF-8 bytes for this section. Long material is requested one section at a time.",
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
    if !PREBUILT_VOICES.contains(&input.voice.as_str()) {
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

fn request_fingerprint(
    text_hash: &str,
    model: &str,
    voice: &str,
    style: &str,
    format: &str,
) -> String {
    let contract = json!({
        "cacheVersion": CACHE_VERSION,
        "spokenTextSha256": text_hash,
        "provider": "Google Gemini Developer API",
        "model": model,
        "voice": voice,
        "style": style,
        "format": format,
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

fn cache_path(storage_dir: &Path, fingerprint: &str) -> PathBuf {
    storage_dir
        .join("narration-audio")
        .join(CACHE_VERSION)
        .join(format!("{fingerprint}.wav"))
}

fn read_cached_audio(storage_dir: &Path, fingerprint: &str) -> Option<Vec<u8>> {
    let bytes = fs::read(cache_path(storage_dir, fingerprint)).ok()?;
    is_wav(&bytes).then_some(bytes)
}

fn write_cached_audio(
    storage_dir: &Path,
    fingerprint: &str,
    bytes: &[u8],
) -> Result<(), ProviderFailure> {
    if !is_wav(bytes) {
        return Err(failure(
            ProviderFailureKind::MalformedProviderResponse,
            "Google returned audio without a valid WAV container. The prior audio cache was left untouched.",
        ));
    }
    let path = cache_path(storage_dir, fingerprint);
    let parent = path.parent().ok_or_else(|| {
        failure(
            ProviderFailureKind::LocalStorageUnavailable,
            "The local audio cache folder could not be located.",
        )
    })?;
    fs::create_dir_all(parent).map_err(|_| failure(ProviderFailureKind::LocalStorageUnavailable, "The local audio cache folder could not be created. The narration can still play for this session."))?;
    let temporary = parent.join(format!(".{}.{}.tmp", fingerprint, Uuid::new_v4()));
    fs::write(&temporary, bytes).map_err(|_| failure(ProviderFailureKind::LocalStorageUnavailable, "The local audio cache could not be written. The narration can still play for this session."))?;
    fs::rename(&temporary, &path).map_err(|_| {
        let _ = fs::remove_file(&temporary);
        failure(ProviderFailureKind::LocalStorageUnavailable, "The local audio cache could not be finalized. The narration can still play for this session.")
    })
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
    let mut key_header = reqwest::header::HeaderValue::from_str(api_key).map_err(|_| {
        failure(
            ProviderFailureKind::InvalidApiKey,
            "The saved API key has an invalid header format. Replace it in AI and narration setup.",
        )
    })?;
    key_header.set_sensitive(true);
    let body = build_request_body(review);
    let client = reqwest::Client::builder()
        .https_only(true)
        .timeout(std::time::Duration::from_secs(90))
        .build()
        .map_err(|_| {
            failure(
                ProviderFailureKind::NetworkUnavailable,
                "The Gemini narration client could not start. No audio was changed.",
            )
        })?;
    let response = client
        .post(ENDPOINT)
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
        write_cached_audio(storage.path(), &base.fingerprint, &audio)
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
        assert!(same.cached_audio_available);
        for changed in [changed_text, changed_voice, changed_style] {
            assert_ne!(changed.fingerprint, base.fingerprint);
            assert!(!changed.cached_audio_available);
        }
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
        write_cached_audio(storage.path(), &review.fingerprint, &expected)
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
        write_cached_audio(storage.path(), &review.fingerprint, &previous)
            .expect("valid WAV should cache");

        assert_eq!(
            write_cached_audio(storage.path(), &review.fingerprint, b"not-a-wave")
                .unwrap_err()
                .kind,
            ProviderFailureKind::MalformedProviderResponse
        );
        assert_eq!(
            read_cached_audio(storage.path(), &review.fingerprint),
            Some(previous)
        );
    }
}
