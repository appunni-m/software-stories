//! Direct model API request encoding through the pinned Rig provider wires.
//!
//! Request reviews use a non-secret placeholder credential. The preview keeps
//! only the sanitized endpoint and the encoded JSON body; authentication never
//! enters the frontend review or application metadata.

use rig_core::completion::CompletionRequest;
use rig_core::driver::Model;
use rig_core::operation::Completion;
use rig_core::wire::{Body, Encoded, Mode, Wire, WireFrame};
use serde_json::Value;
use sha2::{Digest, Sha256};

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
    let request = if provider_id == GEMINI_PROVIDER {
        request.additional_params(serde_json::json!({
            "generationConfig": { "candidateCount": 1 }
        }))
    } else {
        request
    };

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
    use super::encode_request_preview;
    use serde_json::{Value, json};

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
}
