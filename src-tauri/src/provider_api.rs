//! Direct model API request encoding through the pinned Rig provider wires.
//!
//! Request reviews use a non-secret placeholder credential. The preview keeps
//! only the sanitized endpoint and the encoded JSON body; authentication never
//! enters the frontend review or application metadata.

use crate::provider::{MAX_PROVIDER_ERROR_BYTES, MAX_PROVIDER_RESPONSE_BYTES};
use rig_core::completion::CompletionRequest;
use rig_core::driver::Model;
use rig_core::http_client::{
    self, Error as HttpError, HttpClientExt, LazyBody, MultipartForm, Request, Response,
    StreamingResponse,
};
use rig_core::operation::Completion;
use rig_core::wire::{Body, Encoded, Mode, Wire, WireFrame};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::future::Future;
use std::io;
use std::sync::Arc;
use std::time::Duration;
use zeroize::Zeroizing;

const GEMINI_PROVIDER: &str = "gemini";
const OPENAI_PROVIDER: &str = "openai";
const ANTHROPIC_PROVIDER: &str = "anthropic";
const PREVIEW_CREDENTIAL: &str = "code-atlas-review-placeholder";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EncodedRequestPreview {
    pub endpoint_url: String,
    pub body: Value,
    pub body_sha256: String,
    pub body_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RigTextResponse {
    pub model_version: Option<String>,
    pub response_text: String,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RigRequestFailure {
    RequestChanged,
    UnsupportedProvider,
    ClientUnavailable,
    InvalidApiKey,
    QuotaExceeded,
    BillingRequired,
    RegionUnavailable,
    UnavailableModel,
    ProviderRejected,
    RequestTimedOut,
    NetworkUnavailable,
    ResponseTooLarge,
    MalformedResponse,
}

/// Encode one explicitly selected provider request without opening a network
/// connection. `generation_config` is the app's Gemini-shaped canonical
/// settings record; Rig maps those settings and the JSON schema to the selected
/// provider's wire format.
pub fn encode_request_preview(
    provider_id: &str,
    model_id: &str,
    system_instruction: &str,
    user_prompt: &str,
    generation_config: &Value,
) -> Result<EncodedRequestPreview, &'static str> {
    let request = completion_request(
        provider_id,
        system_instruction,
        user_prompt,
        generation_config,
    )?;

    let encoded = match provider_id {
        GEMINI_PROVIDER => {
            let model: Model<_> =
                rig_core::providers::gemini::Gemini::new(PREVIEW_CREDENTIAL).completion(model_id);
            encode_wire(model.wire, request)?
        }
        OPENAI_PROVIDER => {
            let model: Model<_> =
                rig_core::providers::openai::OpenAI::new(PREVIEW_CREDENTIAL).responses(model_id);
            encode_wire(model.wire, request)?
        }
        ANTHROPIC_PROVIDER => {
            let model: Model<_> =
                rig_core::providers::anthropic::Anthropic::new(PREVIEW_CREDENTIAL)
                    .completion(model_id);
            encode_wire(model.wire, request)?
        }
        _ => return Err("provider is not in the direct API adapter catalog"),
    };

    let endpoint_url = sanitized_endpoint(provider_id, &encoded)?;
    let body_bytes = match encoded.request.into_body() {
        Body::Bytes(bytes) => bytes,
        Body::Multipart(_) => return Err("text generation produced a non-JSON request body"),
    };
    let body: Value = serde_json::from_slice(&body_bytes)
        .map_err(|_| "provider encoder produced invalid JSON")?;
    // `send_to_gemini` currently uses Reqwest's JSON serializer after the
    // review boundary. Hash that canonical serialization so the reviewed hash
    // is byte-for-byte the body the transport will send.
    let canonical_body = serde_json::to_vec(&body)
        .map_err(|_| "provider request body could not be fingerprinted")?;
    let body_sha256 = hex_digest(&canonical_body);

    Ok(EncodedRequestPreview {
        endpoint_url,
        body,
        body_sha256,
        body_bytes: canonical_body.len(),
    })
}

fn completion_request(
    provider_id: &str,
    system_instruction: &str,
    user_prompt: &str,
    generation_config: &Value,
) -> Result<CompletionRequest, &'static str> {
    let temperature = generation_config
        .get("temperature")
        .and_then(Value::as_f64)
        .ok_or("missing request temperature")?;
    let max_tokens = generation_config
        .get("maxOutputTokens")
        .and_then(Value::as_u64)
        .ok_or("missing output token limit")?;
    if generation_config
        .get("candidateCount")
        .and_then(Value::as_u64)
        != Some(1)
    {
        return Err("only one response candidate is supported");
    }
    if generation_config
        .get("responseMimeType")
        .and_then(Value::as_str)
        != Some("application/json")
    {
        return Err("the request must require JSON output");
    }
    let mut schema_json = generation_config
        .get("responseSchema")
        .cloned()
        .ok_or("missing response schema")?;
    lowercase_schema_types(&mut schema_json);
    let schema: rig_core::schemars::Schema =
        serde_json::from_value(schema_json).map_err(|_| "response schema is invalid")?;
    let request = CompletionRequest::new(user_prompt.to_owned())
        .preamble(system_instruction.to_owned())
        .temperature(temperature)
        .max_tokens(max_tokens)
        .output_schema(schema);
    match provider_id {
        GEMINI_PROVIDER => Ok(request.additional_params(serde_json::json!({
            "generationConfig": { "candidateCount": 1 }
        }))),
        OPENAI_PROVIDER => Ok(request.additional_params(serde_json::json!({
            "store": false
        }))),
        ANTHROPIC_PROVIDER => Ok(request),
        _ => Err("provider is not in the direct API adapter catalog"),
    }
}

/// Execute a reviewed unary completion through the selected Rig provider wire.
/// The transport accepts a request only when its endpoint and serialized body
/// match this review; it also moves credentials from Rig's placeholder into
/// the provider's required authentication header after that comparison.
pub async fn complete_reviewed_request(
    provider_id: &str,
    model_id: &str,
    endpoint_url: &str,
    api_key: Zeroizing<String>,
    system_instruction: &str,
    user_prompt: &str,
    generation_config: &Value,
    reviewed_body: &Value,
) -> Result<RigTextResponse, RigRequestFailure> {
    let encoded = encode_request_preview(
        provider_id,
        model_id,
        system_instruction,
        user_prompt,
        generation_config,
    )
    .map_err(|_| RigRequestFailure::RequestChanged)?;
    if encoded.endpoint_url != endpoint_url || &encoded.body != reviewed_body {
        return Err(RigRequestFailure::RequestChanged);
    }
    let request = completion_request(
        provider_id,
        system_instruction,
        user_prompt,
        generation_config,
    )
    .map_err(|_| RigRequestFailure::RequestChanged)?;
    let http = SafeRigHttpClient::new(
        provider_id,
        endpoint_url,
        api_key,
        serde_json::to_vec(reviewed_body).map_err(|_| RigRequestFailure::RequestChanged)?,
    )
    .map_err(|_| RigRequestFailure::ClientUnavailable)?;
    let response = match provider_id {
        GEMINI_PROVIDER => {
            rig_core::providers::gemini::Gemini::new(PREVIEW_CREDENTIAL)
                .with_http(http)
                .completion(model_id)
                .call(request)
                .await
        }
        OPENAI_PROVIDER => {
            rig_core::providers::openai::OpenAI::new(PREVIEW_CREDENTIAL)
                .with_http(http)
                .responses(model_id)
                .call(request)
                .await
        }
        ANTHROPIC_PROVIDER => {
            rig_core::providers::anthropic::Anthropic::new(PREVIEW_CREDENTIAL)
                .with_http(http)
                .completion(model_id)
                .call(request)
                .await
        }
        _ => return Err(RigRequestFailure::UnsupportedProvider),
    }
    .map_err(|error| classify_rig_failure(error, provider_id))?;
    if response.error.is_some()
        || response.aborted.is_some()
        || !matches!(
            response.finish_reason(),
            Some(rig_core::completion::FinishReason::Stop)
        )
    {
        return Err(RigRequestFailure::ProviderRejected);
    }
    let mut response_text = String::new();
    for content in response.choice {
        if let rig_core::completion::AssistantContent::Text(text) = content {
            response_text.push_str(&text.text);
        }
    }
    if response_text.trim().is_empty() {
        return Err(RigRequestFailure::MalformedResponse);
    }
    Ok(RigTextResponse {
        model_version: response.origin.response_model,
        response_text,
        input_tokens: response.usage.input_tokens,
        output_tokens: response.usage.output_tokens,
    })
}

#[derive(Clone)]
struct SafeRigHttpClient {
    provider_id: String,
    endpoint_url: String,
    api_key: Arc<Zeroizing<String>>,
    reviewed_body: bytes::Bytes,
    client: reqwest::Client,
}

impl SafeRigHttpClient {
    fn new(
        provider_id: &str,
        endpoint_url: &str,
        api_key: Zeroizing<String>,
        reviewed_body: Vec<u8>,
    ) -> Result<Self, &'static str> {
        if !matches!(
            provider_id,
            GEMINI_PROVIDER | OPENAI_PROVIDER | ANTHROPIC_PROVIDER
        ) {
            return Err("provider is not in the direct API adapter catalog");
        }
        let client = reqwest::Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(|_| "the provider request client could not start")?;
        Self::with_client(provider_id, endpoint_url, api_key, reviewed_body, client)
    }

    #[cfg(test)]
    fn new_for_loopback_test(
        provider_id: &str,
        endpoint_url: &str,
        api_key: Zeroizing<String>,
        reviewed_body: Vec<u8>,
    ) -> Result<Self, &'static str> {
        let endpoint = reqwest::Url::parse(endpoint_url)
            .map_err(|_| "test transport requires a valid loopback URL")?;
        if endpoint.scheme() != "http"
            || endpoint.host_str() != Some("127.0.0.1")
            || endpoint.query().is_some()
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.fragment().is_some()
        {
            return Err("test transport accepts only credential-free loopback HTTP URLs");
        }
        let client = reqwest::Client::builder()
            .no_proxy()
            .https_only(false)
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(|_| "the loopback test client could not start")?;
        Self::with_client(provider_id, endpoint_url, api_key, reviewed_body, client)
    }

    fn with_client(
        provider_id: &str,
        endpoint_url: &str,
        api_key: Zeroizing<String>,
        reviewed_body: Vec<u8>,
        client: reqwest::Client,
    ) -> Result<Self, &'static str> {
        if !matches!(
            provider_id,
            GEMINI_PROVIDER | OPENAI_PROVIDER | ANTHROPIC_PROVIDER
        ) {
            return Err("provider is not in the direct API adapter catalog");
        }
        Ok(Self {
            provider_id: provider_id.to_owned(),
            endpoint_url: endpoint_url.to_owned(),
            api_key: Arc::new(api_key),
            reviewed_body: bytes::Bytes::from(reviewed_body),
            client,
        })
    }

    fn prepare_request(
        &self,
        request: Request<bytes::Bytes>,
    ) -> Result<reqwest::Request, &'static str> {
        let (parts, body) = request.into_parts();
        if parts.method != reqwest::Method::POST {
            return Err("direct text generation requires POST");
        }
        if body.len() > crate::provider::MAX_REQUEST_BODY_BYTES {
            return Err("the reviewed request exceeds the local request-size limit");
        }
        if body != self.reviewed_body {
            return Err("the encoded body differs from the body the user reviewed");
        }
        if parts
            .headers
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            != Some("application/json")
        {
            return Err("the provider request is not JSON");
        }
        let request_uri = parts.uri.to_string();
        let mut request = Request::from_parts(parts, body);
        match self.provider_id.as_str() {
            GEMINI_PROVIDER => {
                let (url, query) = request_uri
                    .split_once('?')
                    .ok_or("Gemini request omitted its Rig credential placeholder")?;
                if url != self.endpoint_url || query != format!("key={PREVIEW_CREDENTIAL}") {
                    return Err("Gemini request endpoint or credential placeholder changed");
                }
                *request.uri_mut() = self
                    .endpoint_url
                    .parse()
                    .map_err(|_| "Gemini endpoint could not be reconstructed")?;
                let mut key = reqwest::header::HeaderValue::from_str(self.api_key.as_str())
                    .map_err(|_| "the saved API key has an invalid header format")?;
                key.set_sensitive(true);
                request.headers_mut().insert("x-goog-api-key", key);
            }
            OPENAI_PROVIDER => {
                if request_uri != self.endpoint_url || request.uri().query().is_some() {
                    return Err("OpenAI request endpoint changed after review");
                }
                let placeholder = format!("Bearer {PREVIEW_CREDENTIAL}");
                if request
                    .headers()
                    .get(reqwest::header::AUTHORIZATION)
                    .and_then(|value| value.to_str().ok())
                    != Some(placeholder.as_str())
                {
                    return Err("OpenAI request authentication did not match the Rig adapter");
                }
                let mut key = reqwest::header::HeaderValue::from_str(&format!(
                    "Bearer {}",
                    self.api_key.as_str()
                ))
                .map_err(|_| "the saved API key has an invalid header format")?;
                key.set_sensitive(true);
                request
                    .headers_mut()
                    .insert(reqwest::header::AUTHORIZATION, key);
            }
            ANTHROPIC_PROVIDER => {
                if request_uri != self.endpoint_url || request.uri().query().is_some() {
                    return Err("Anthropic request endpoint changed after review");
                }
                if request
                    .headers()
                    .get("x-api-key")
                    .and_then(|value| value.to_str().ok())
                    != Some(PREVIEW_CREDENTIAL)
                {
                    return Err("Anthropic request authentication did not match the Rig adapter");
                }
                let mut key = reqwest::header::HeaderValue::from_str(self.api_key.as_str())
                    .map_err(|_| "the saved API key has an invalid header format")?;
                key.set_sensitive(true);
                request.headers_mut().insert("x-api-key", key);
            }
            _ => return Err("provider is not in the direct API adapter catalog"),
        }
        reqwest::Request::try_from(request)
            .map_err(|_| "the provider request could not be prepared")
    }
}

impl HttpClientExt for SafeRigHttpClient {
    fn send<T, U>(
        &self,
        request: Request<T>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
    where
        T: Into<bytes::Bytes> + Send,
        U: From<bytes::Bytes> + Send + 'static,
    {
        let prepared = self.prepare_request(request.map(Into::into));
        let client = self.client.clone();
        async move {
            let request = prepared.map_err(|_| {
                HttpError::instance(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "the request did not match its reviewed provider route",
                ))
            })?;
            let response = client.execute(request).await.map_err(HttpError::instance)?;
            let status = response.status();
            let version = response.version();
            let headers = response.headers().clone();
            if !status.is_success() {
                let error_body = read_bounded_body(response, MAX_PROVIDER_ERROR_BYTES).await;
                let body = match error_body {
                    Ok((bytes, false)) => String::from_utf8_lossy(&bytes).into_owned(),
                    Ok((_, true)) => {
                        "Provider error response exceeded the local size limit.".to_owned()
                    }
                    Err(_) => "Provider error response could not be read.".to_owned(),
                };
                return Err(HttpError::non_success_with_details(status, headers, body));
            }
            if response
                .content_length()
                .is_some_and(|length| length > MAX_PROVIDER_RESPONSE_BYTES as u64)
            {
                return Err(HttpError::instance(LocalResponseLimitExceeded));
            }
            let (bytes, too_large) = read_bounded_body(response, MAX_PROVIDER_RESPONSE_BYTES)
                .await
                .map_err(HttpError::instance)?;
            if too_large {
                return Err(HttpError::instance(LocalResponseLimitExceeded));
            }
            let mut builder = Response::builder().status(status).version(version);
            if let Some(response_headers) = builder.headers_mut() {
                *response_headers = headers;
            }
            let body: LazyBody<U> = Box::pin(async move { Ok(U::from(bytes::Bytes::from(bytes))) });
            builder.body(body).map_err(HttpError::Protocol)
        }
    }

    fn send_multipart<U>(
        &self,
        _request: Request<MultipartForm>,
    ) -> impl Future<Output = http_client::Result<Response<LazyBody<U>>>> + Send + 'static
    where
        U: From<bytes::Bytes> + Send + 'static,
    {
        async {
            Err(HttpError::instance(io::Error::new(
                io::ErrorKind::Unsupported,
                "direct text generation does not send multipart requests",
            )))
        }
    }

    fn send_streaming<T>(
        &self,
        _request: Request<T>,
    ) -> impl Future<Output = http_client::Result<StreamingResponse>> + Send
    where
        T: Into<bytes::Bytes> + Send,
    {
        async {
            Err(HttpError::instance(io::Error::new(
                io::ErrorKind::Unsupported,
                "streaming is not enabled for this direct text-generation operation",
            )))
        }
    }
}

async fn read_bounded_body(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<(Vec<u8>, bool), reqwest::Error> {
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Ok((bytes, true));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok((bytes, false))
}

#[derive(Debug)]
struct LocalResponseLimitExceeded;

impl std::fmt::Display for LocalResponseLimitExceeded {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("provider response exceeded the local size limit")
    }
}

impl std::error::Error for LocalResponseLimitExceeded {}

fn classify_rig_failure(
    error: rig_core::error::ProviderError,
    provider_id: &str,
) -> RigRequestFailure {
    if let Some(status) = error.provider_response_status() {
        let status = status.as_u16();
        let body = error.provider_response_body().unwrap_or_default();
        return match provider_id {
            GEMINI_PROVIDER => {
                let mapped = crate::provider::classify_provider_error(status, body.as_bytes());
                match mapped.kind {
                    crate::provider::ProviderFailureKind::InvalidApiKey => {
                        RigRequestFailure::InvalidApiKey
                    }
                    crate::provider::ProviderFailureKind::QuotaExceeded => {
                        RigRequestFailure::QuotaExceeded
                    }
                    crate::provider::ProviderFailureKind::BillingRequired => {
                        RigRequestFailure::BillingRequired
                    }
                    crate::provider::ProviderFailureKind::RegionUnavailable => {
                        RigRequestFailure::RegionUnavailable
                    }
                    crate::provider::ProviderFailureKind::UnavailableModel => {
                        RigRequestFailure::UnavailableModel
                    }
                    _ => RigRequestFailure::ProviderRejected,
                }
            }
            OPENAI_PROVIDER => classify_openai_failure(status, body),
            ANTHROPIC_PROVIDER => classify_anthropic_failure(status, body),
            _ => classify_http_status(status),
        };
    }
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(&error);
    while let Some(cause) = source {
        if cause.downcast_ref::<LocalResponseLimitExceeded>().is_some() {
            return RigRequestFailure::ResponseTooLarge;
        }
        if cause
            .downcast_ref::<reqwest::Error>()
            .is_some_and(reqwest::Error::is_timeout)
        {
            return RigRequestFailure::RequestTimedOut;
        }
        if cause
            .downcast_ref::<io::Error>()
            .is_some_and(|error| error.kind() == io::ErrorKind::InvalidInput)
        {
            return RigRequestFailure::RequestChanged;
        }
        source = cause.source();
    }
    RigRequestFailure::NetworkUnavailable
}

fn classify_openai_failure(status: u16, body: &str) -> RigRequestFailure {
    let (code, message) = provider_error_fields(body);
    if status == 403 && crate::provider::is_region_restriction_error(&code, &message) {
        return RigRequestFailure::RegionUnavailable;
    }
    if status == 429
        && matches!(
            code.as_str(),
            "credit_balance_exhausted"
                | "organization_spend_limit_exceeded"
                | "project_spend_limit_exceeded"
        )
    {
        return RigRequestFailure::BillingRequired;
    }
    if status == 429 && code == "organization_usage_limit_exceeded" {
        return RigRequestFailure::QuotaExceeded;
    }
    if status == 401
        && (code.contains("authentication")
            || code.contains("api_key")
            || message.contains("authentication")
            || message.contains("api key"))
    {
        return RigRequestFailure::InvalidApiKey;
    }
    classify_http_status(status)
}

fn classify_anthropic_failure(status: u16, body: &str) -> RigRequestFailure {
    let (code, message) = provider_error_fields(body);
    if status == 403 && crate::provider::is_region_restriction_error(&code, &message) {
        return RigRequestFailure::RegionUnavailable;
    }
    match status {
        401 => RigRequestFailure::InvalidApiKey,
        402 => RigRequestFailure::BillingRequired,
        429 => RigRequestFailure::QuotaExceeded,
        _ => classify_http_status(status),
    }
}

fn classify_http_status(status: u16) -> RigRequestFailure {
    match status {
        402 => RigRequestFailure::BillingRequired,
        429 => RigRequestFailure::QuotaExceeded,
        400..=499 => RigRequestFailure::ProviderRejected,
        500..=599 => RigRequestFailure::NetworkUnavailable,
        _ => RigRequestFailure::ProviderRejected,
    }
}

fn provider_error_fields(body: &str) -> (String, String) {
    let value: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let error = value.get("error").unwrap_or(&value);
    let code = error
        .get("code")
        .and_then(Value::as_str)
        .or_else(|| error.get("type").and_then(Value::as_str))
        .or_else(|| error.get("status").and_then(Value::as_str))
        .unwrap_or_default()
        .to_ascii_lowercase();
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    (code, message)
}

fn encode_wire<W>(wire: W, request: CompletionRequest) -> Result<Encoded, &'static str>
where
    W: Wire<Op = Completion, Payload = Encoded, Frame = WireFrame>,
{
    wire.encode(request, Mode::Unary)
        .map_err(|_| "provider could not encode the reviewed request")
}

fn sanitized_endpoint(provider_id: &str, encoded: &Encoded) -> Result<String, &'static str> {
    let uri = encoded.request.uri();
    let endpoint = uri.to_string();
    if provider_id == GEMINI_PROVIDER {
        let (without_query, query) = endpoint
            .split_once('?')
            .ok_or("Gemini encoder did not produce its expected API-key query")?;
        if !query.starts_with("key=") || query.contains('&') {
            return Err("Gemini encoder produced an unexpected query string");
        }
        return Ok(without_query.to_owned());
    }
    if uri.query().is_some() {
        return Err("provider endpoint query parameters are not supported by this catalog");
    }
    Ok(endpoint)
}

fn lowercase_schema_types(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if let Some(Value::String(schema_type)) = object.get_mut("type") {
                *schema_type = schema_type.to_ascii_lowercase();
            }
            for nested in object.values_mut() {
                lowercase_schema_types(nested);
            }
        }
        Value::Array(items) => {
            for nested in items {
                lowercase_schema_types(nested);
            }
        }
        _ => {}
    }
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        PREVIEW_CREDENTIAL, RigRequestFailure, SafeRigHttpClient, classify_rig_failure,
        encode_request_preview,
    };
    use rig_core::http_client::{HttpClientExt, Request};
    use serde_json::{Value, json};
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::mpsc;
    use std::time::Duration;
    use zeroize::Zeroizing;

    fn generation_config() -> Value {
        json!({
            "temperature": 0.2,
            "maxOutputTokens": 1024,
            "candidateCount": 1,
            "responseMimeType": "application/json",
            "responseSchema": {
                "type": "OBJECT",
                "properties": {
                    "summary": { "type": "STRING" },
                    "evidence": {
                        "type": "ARRAY",
                        "items": { "type": "STRING" }
                    }
                },
                "required": ["summary", "evidence"],
                "additionalProperties": false
            }
        })
    }

    fn request_with_placeholder(
        provider_id: &str,
        endpoint: &str,
        body: &[u8],
    ) -> Request<bytes::Bytes> {
        let uri = if provider_id == "gemini" {
            format!("{endpoint}?key={PREVIEW_CREDENTIAL}")
        } else {
            endpoint.to_owned()
        };
        let mut builder = Request::builder()
            .method("POST")
            .uri(uri)
            .header(reqwest::header::CONTENT_TYPE, "application/json");
        match provider_id {
            "openai" => {
                builder = builder.header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {PREVIEW_CREDENTIAL}"),
                );
            }
            "anthropic" => {
                builder = builder
                    .header("x-api-key", PREVIEW_CREDENTIAL)
                    .header("anthropic-version", "2023-06-01");
            }
            _ => {}
        }
        builder
            .body(bytes::Bytes::copy_from_slice(body))
            .expect("test request is valid")
    }

    fn read_http_request(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = stream.read(&mut buffer)?;
            if read == 0 {
                return Ok(request);
            }
            request.extend_from_slice(&buffer[..read]);
            if request.len() > 64 * 1024 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "loopback test request exceeded its bound",
                ));
            }
            let Some(header_end) = request.windows(4).position(|bytes| bytes == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .filter_map(|line| line.split_once(':'))
                .find_map(|(name, value)| {
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "loopback test request has no content length",
                    )
                })?;
            if request.len() >= header_end + 4 + content_length {
                return Ok(request);
            }
        }
    }

    #[test]
    fn each_catalog_provider_encodes_to_its_explicit_route_without_exposing_credentials() {
        let cases = [
            (
                "gemini",
                "test-gemini-model",
                "https://generativelanguage.googleapis.com/v1beta/models/test-gemini-model:generateContent",
            ),
            (
                "openai",
                "test-openai-model",
                "https://api.openai.com/v1/responses",
            ),
            (
                "anthropic",
                "test-anthropic-model",
                "https://api.anthropic.com/v1/messages",
            ),
        ];

        for (provider_id, model_id, expected_endpoint) in cases {
            let preview = encode_request_preview(
                provider_id,
                model_id,
                "Explain only the supplied evidence.",
                "Describe one selected source file.",
                &generation_config(),
            )
            .unwrap_or_else(|error| panic!("{provider_id} did not encode: {error}"));

            assert_eq!(
                preview.endpoint_url, expected_endpoint,
                "{provider_id} route"
            );
            if provider_id == "gemini" {
                assert!(
                    preview.endpoint_url.contains(model_id),
                    "Gemini model is encoded in the endpoint"
                );
            } else {
                assert_eq!(preview.body["model"], model_id, "{provider_id} model");
            }
            assert!(preview.body_bytes > 0, "{provider_id} body is present");
            assert_eq!(
                preview.body_bytes,
                serde_json::to_vec(&preview.body).unwrap().len(),
                "{provider_id} body size matches the reviewed JSON body"
            );
            assert!(!preview.endpoint_url.contains("key="));
            assert!(
                !preview
                    .body
                    .to_string()
                    .contains("code-atlas-review-placeholder")
            );
        }
    }

    #[test]
    fn an_unknown_provider_fails_closed_instead_of_selecting_a_default() {
        let error = encode_request_preview(
            "unknown-provider",
            "model-name",
            "system",
            "prompt",
            &generation_config(),
        )
        .unwrap_err();

        assert_eq!(error, "provider is not in the direct API adapter catalog");
    }

    #[test]
    fn guarded_rig_transport_only_replaces_credentials_after_exact_review_match() {
        let cases = [
            (
                "gemini",
                "test-gemini-model",
                "https://generativelanguage.googleapis.com/v1beta/models/test-gemini-model:generateContent",
            ),
            (
                "openai",
                "test-openai-model",
                "https://api.openai.com/v1/responses",
            ),
            (
                "anthropic",
                "test-anthropic-model",
                "https://api.anthropic.com/v1/messages",
            ),
        ];

        for (provider_id, model_id, endpoint) in cases {
            let preview = encode_request_preview(
                provider_id,
                model_id,
                "system",
                "prompt",
                &generation_config(),
            )
            .unwrap();
            let client = SafeRigHttpClient::new(
                provider_id,
                endpoint,
                Zeroizing::new("live-test-key-should-never-escape".to_owned()),
                serde_json::to_vec(&preview.body).unwrap(),
            )
            .unwrap();
            let request_uri = if provider_id == "gemini" {
                format!("{endpoint}?key={PREVIEW_CREDENTIAL}")
            } else {
                endpoint.to_owned()
            };
            let mut builder = Request::builder().method("POST").uri(request_uri);
            match provider_id {
                "openai" => {
                    builder = builder.header(
                        reqwest::header::AUTHORIZATION,
                        "Bearer code-atlas-review-placeholder",
                    );
                }
                "anthropic" => {
                    builder = builder
                        .header("x-api-key", PREVIEW_CREDENTIAL)
                        .header("anthropic-version", "2023-06-01");
                }
                _ => {}
            }
            let request = builder
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(bytes::Bytes::from(
                    serde_json::to_vec(&preview.body).unwrap(),
                ))
                .unwrap();
            let prepared = client.prepare_request(request).unwrap();

            assert_eq!(prepared.url().as_str(), endpoint);
            assert!(!prepared.url().as_str().contains("live-test-key"));
            assert!(!preview.body.to_string().contains("live-test-key"));
            let credential_header = match provider_id {
                "gemini" => "x-goog-api-key",
                "openai" => "authorization",
                "anthropic" => "x-api-key",
                _ => unreachable!(),
            };
            let value = prepared.headers().get(credential_header).unwrap();
            assert!(
                value.is_sensitive(),
                "{provider_id} credential is sensitive"
            );
            assert!(
                value.to_str().unwrap().contains("live-test-key"),
                "{provider_id} receives the credential only in its auth header"
            );
        }
    }

    #[tokio::test]
    async fn actual_loopback_transport_keeps_each_provider_key_in_its_auth_header() {
        let cases = [
            ("gemini", "test-gemini-model", "x-goog-api-key"),
            ("openai", "test-openai-model", "authorization"),
            ("anthropic", "test-anthropic-model", "x-api-key"),
        ];

        for (provider_id, model_id, credential_header) in cases {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let endpoint = format!("http://{}/v1/test", listener.local_addr().unwrap());
            let preview = encode_request_preview(
                provider_id,
                model_id,
                "system instruction",
                "reviewed source context",
                &generation_config(),
            )
            .unwrap();
            let reviewed_body = serde_json::to_vec(&preview.body).unwrap();
            let api_key = format!("private-marker-{provider_id}");
            let client = SafeRigHttpClient::new_for_loopback_test(
                provider_id,
                &endpoint,
                Zeroizing::new(api_key.clone()),
                reviewed_body.clone(),
            )
            .unwrap();
            let request = request_with_placeholder(provider_id, &endpoint, &reviewed_body);

            let server = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let request = read_http_request(&mut stream).unwrap();
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 2\r\nconnection: close\r\n\r\n{}",
                    )
                    .unwrap();
                request
            });

            let response = client
                .send::<bytes::Bytes, bytes::Bytes>(request)
                .await
                .unwrap();
            let response_body = response.into_body().await.unwrap();
            assert_eq!(response_body.as_ref(), b"{}");
            let wire_request = server.join().unwrap();
            let header_end = wire_request
                .windows(4)
                .position(|bytes| bytes == b"\r\n\r\n")
                .expect("request has a header boundary");
            let headers = String::from_utf8_lossy(&wire_request[..header_end]);
            let body = &wire_request[header_end + 4..];
            let request_line = headers.lines().next().unwrap_or_default();
            assert!(request_line.starts_with("POST /v1/test "), "{request_line}");
            assert_eq!(body, reviewed_body, "{provider_id} sends the reviewed body");

            let auth_line = headers
                .lines()
                .find(|line| {
                    line.split_once(':')
                        .is_some_and(|(name, _)| name.eq_ignore_ascii_case(credential_header))
                })
                .unwrap_or_else(|| panic!("{provider_id} auth header is present"));
            assert!(auth_line.contains(&api_key), "{provider_id} auth header");
            let other_headers = headers
                .lines()
                .filter(|line| {
                    !line
                        .split_once(':')
                        .is_some_and(|(name, _)| name.eq_ignore_ascii_case(credential_header))
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                !other_headers.contains(&api_key),
                "{provider_id} other headers"
            );
            assert!(
                !request_line.contains(&api_key),
                "{provider_id} request target"
            );
            assert!(!String::from_utf8_lossy(body).contains(&api_key));
            assert!(!headers.contains(PREVIEW_CREDENTIAL));
        }
    }

    #[tokio::test]
    async fn cancelling_a_loopback_rig_request_closes_its_in_flight_socket() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!("http://{}/v1/test", listener.local_addr().unwrap());
        let preview = encode_request_preview(
            "openai",
            "test-openai-model",
            "system instruction",
            "reviewed source context",
            &generation_config(),
        )
        .unwrap();
        let reviewed_body = serde_json::to_vec(&preview.body).unwrap();
        let client = SafeRigHttpClient::new_for_loopback_test(
            "openai",
            &endpoint,
            Zeroizing::new("private-cancellation-marker".to_owned()),
            reviewed_body.clone(),
        )
        .unwrap();
        let request = request_with_placeholder("openai", &endpoint, &reviewed_body);
        let (request_seen_tx, request_seen_rx) = mpsc::channel();
        let (connection_closed_tx, connection_closed_rx) = mpsc::channel();

        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let request = read_http_request(&mut stream).unwrap();
            request_seen_tx.send(request).unwrap();
            let mut byte = [0_u8; 1];
            let closed = match stream.read(&mut byte) {
                Ok(0) => true,
                Err(error) => matches!(
                    error.kind(),
                    std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::UnexpectedEof
                ),
                Ok(_) => false,
            };
            connection_closed_tx.send(closed).unwrap();
        });

        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        let request_task = tokio::spawn(async move {
            let request = client.send::<bytes::Bytes, bytes::Bytes>(request);
            tokio::select! {
                biased;
                _ = cancel_rx => false,
                _ = request => true,
            }
        });
        let wire_request = tokio::task::spawn_blocking(move || {
            request_seen_rx
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
        })
        .await
        .unwrap();
        assert!(String::from_utf8_lossy(&wire_request).contains("private-cancellation-marker"));
        cancel_tx.send(()).unwrap();
        assert!(
            !request_task.await.unwrap(),
            "cancellation wins over the pending response"
        );
        let connection_closed = tokio::task::spawn_blocking(move || {
            connection_closed_rx
                .recv_timeout(Duration::from_secs(6))
                .unwrap()
        })
        .await
        .unwrap();
        assert!(
            connection_closed,
            "dropping the request future closes its socket"
        );
        server.join().unwrap();
    }

    #[test]
    fn guarded_rig_transport_rejects_changed_body_or_route_before_network_access() {
        let preview = encode_request_preview(
            "openai",
            "test-openai-model",
            "system",
            "prompt",
            &generation_config(),
        )
        .unwrap();
        let client = SafeRigHttpClient::new(
            "openai",
            &preview.endpoint_url,
            Zeroizing::new("live-test-key".to_owned()),
            serde_json::to_vec(&preview.body).unwrap(),
        )
        .unwrap();
        let request = Request::builder()
            .method("POST")
            .uri("https://api.openai.com/v1/chat/completions")
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                reqwest::header::AUTHORIZATION,
                "Bearer code-atlas-review-placeholder",
            )
            .body(bytes::Bytes::from(
                serde_json::to_vec(&preview.body).unwrap(),
            ))
            .unwrap();
        assert!(client.prepare_request(request).is_err());

        let request = Request::builder()
            .method("POST")
            .uri(&preview.endpoint_url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                reqwest::header::AUTHORIZATION,
                "Bearer code-atlas-review-placeholder",
            )
            .body(bytes::Bytes::from_static(b"{}"))
            .unwrap();
        assert!(client.prepare_request(request).is_err());
    }

    #[test]
    fn provider_http_failures_use_vendor_specific_safe_classification() {
        let cases = [
            (
                "gemini",
                401,
                r#"{"error":{"status":"UNAUTHENTICATED","message":"private api key detail"}}"#,
                RigRequestFailure::InvalidApiKey,
            ),
            (
                "gemini",
                403,
                r#"{"error":{"status":"PERMISSION_DENIED","message":"This API is not available in your region"}}"#,
                RigRequestFailure::RegionUnavailable,
            ),
            (
                "gemini",
                403,
                r#"{"error":{"status":"PERMISSION_DENIED","message":"generic access denied"}}"#,
                RigRequestFailure::ProviderRejected,
            ),
            (
                "gemini",
                402,
                r#"{"error":{"status":"PAYMENT_REQUIRED","message":"prepay credits are depleted"}}"#,
                RigRequestFailure::BillingRequired,
            ),
            (
                "gemini",
                404,
                r#"{"error":{"status":"NOT_FOUND","message":"unknown model"}}"#,
                RigRequestFailure::UnavailableModel,
            ),
            (
                "gemini",
                429,
                r#"{"error":{"status":"RESOURCE_EXHAUSTED","message":"quota exceeded"}}"#,
                RigRequestFailure::QuotaExceeded,
            ),
            (
                "openai",
                401,
                r#"{"error":{"type":"invalid_request_error","message":"Incorrect API key provided: private-key-marker"}}"#,
                RigRequestFailure::InvalidApiKey,
            ),
            (
                "openai",
                401,
                r#"{"error":{"type":"invalid_request_error","message":"You must be a member of an organization"}}"#,
                RigRequestFailure::ProviderRejected,
            ),
            (
                "openai",
                403,
                r#"{"error":{"type":"unsupported_country_region_territory","message":"Country, region, or territory not supported"}}"#,
                RigRequestFailure::RegionUnavailable,
            ),
            (
                "openai",
                403,
                r#"{"error":{"type":"permission_error","message":"The project cannot use this resource"}}"#,
                RigRequestFailure::ProviderRejected,
            ),
            (
                "openai",
                429,
                r#"{"error":{"code":"credit_balance_exhausted","message":"private account detail"}}"#,
                RigRequestFailure::BillingRequired,
            ),
            (
                "openai",
                429,
                r#"{"error":{"type":"rate_limit_error","code":"rate_limit_exceeded","message":"slow down"}}"#,
                RigRequestFailure::QuotaExceeded,
            ),
            (
                "openai",
                429,
                r#"{"error":{"code":"organization_usage_limit_exceeded","message":"usage limit reached"}}"#,
                RigRequestFailure::QuotaExceeded,
            ),
            (
                "openai",
                503,
                r#"{"error":{"type":"service_unavailable_error","message":"temporarily overloaded"}}"#,
                RigRequestFailure::NetworkUnavailable,
            ),
            (
                "openai",
                200,
                r#"{"error":{"type":"api_error","message":"error envelope in success response"}}"#,
                RigRequestFailure::ProviderRejected,
            ),
            (
                "anthropic",
                401,
                r#"{"type":"error","error":{"type":"authentication_error","message":"private key detail"}}"#,
                RigRequestFailure::InvalidApiKey,
            ),
            (
                "anthropic",
                402,
                r#"{"type":"error","error":{"type":"billing_error","message":"billing required"}}"#,
                RigRequestFailure::BillingRequired,
            ),
            (
                "anthropic",
                403,
                r#"{"type":"error","error":{"type":"permission_error","message":"resource access denied"}}"#,
                RigRequestFailure::ProviderRejected,
            ),
            (
                "anthropic",
                429,
                r#"{"type":"error","error":{"type":"rate_limit_error","message":"request limit reached"}}"#,
                RigRequestFailure::QuotaExceeded,
            ),
            (
                "anthropic",
                529,
                r#"{"type":"error","error":{"type":"overloaded_error","message":"overloaded"}}"#,
                RigRequestFailure::NetworkUnavailable,
            ),
        ];

        for (provider_id, status, body, expected) in cases {
            let error = rig_core::error::ProviderError::from_http_response(
                reqwest::StatusCode::from_u16(status).unwrap(),
                body,
            );
            let classified = classify_rig_failure(error, provider_id);
            assert_eq!(classified, expected, "{provider_id} HTTP {status}");
            assert!(
                !format!("{classified:?}").contains("private"),
                "{provider_id} HTTP {status} must not retain the provider body"
            );
        }
    }
}
