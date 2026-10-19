//! Sanitized, native-owned Claude CLI installation/account observations.
//!
//! Inspection is not login, entitlement verification, model discovery or
//! authorization to send source. Never expose raw command output or credentials.

use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::path::{Component, Path};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeConnectionObservation {
    pub executable_path: String,
    pub executable_sha256: String,
    pub runtime_version: String,
    pub candidate_version_matches: bool,
    pub account: Option<ClaudeAccountObservation>,
    // Kept explicit: installation/auth metadata does not enable generation.
    pub automatic_generation_available: bool,
    pub message: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeAccountObservation {
    pub logged_in: bool,
    pub auth_method: ClaudeAuthMethod,
    pub first_party_provider: bool,
    pub account_email: Option<String>,
    pub subscription_type: Option<String>,
    pub config_directory: Option<String>,
    /// Reported identity only; neither a credential nor entitlement proof.
    pub account_identity_sha256: Option<String>,
}

#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ClaudeAuthMethod {
    None,
    ClaudeAccount,
    #[serde(rename = "oauthToken")]
    OAuthToken,
    ApiKey,
    ApiKeyHelper,
    ThirdParty,
    Unrecognized,
}

pub(crate) fn parse_version(bytes: &[u8]) -> Result<String, &'static str> {
    if bytes.len() > 128 {
        return Err("Claude version output exceeded its local limit.");
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "Claude version output was not valid UTF-8.")?
        .trim();
    let version = text
        .strip_suffix(" (Claude Code)")
        .ok_or("The selected executable did not report the expected Claude Code version format.")?;
    let parts: Vec<_> = version.split('.').collect();
    if parts.len() != 3
        || parts.iter().any(|part| {
            part.is_empty() || part.len() > 5 || !part.bytes().all(|byte| byte.is_ascii_digit())
        })
    {
        return Err("The selected executable reported an unsupported version format.");
    }
    Ok(version.to_owned())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccountStatusWire {
    logged_in: bool,
    auth_method: String,
    api_provider: Option<String>,
    email: Option<String>,
    org_id: Option<String>,
    subscription_type: Option<String>,
    config_directory: Option<String>,
}

/// `auth status` is the vendor's metadata surface. Unknown fields, including
/// any future credential fields, are dropped rather than forwarded to the UI.
pub(crate) fn parse_account_status(
    bytes: &[u8],
    exit_code: Option<i32>,
) -> Result<ClaudeAccountObservation, &'static str> {
    if bytes.len() > 64 * 1024 {
        return Err("Claude account status exceeded its local limit.");
    }
    let value: AccountStatusWire = serde_json::from_slice(bytes)
        .map_err(|_| "Claude account status was not a valid JSON object.")?;
    let logged_in = value.logged_in;
    if exit_code != Some(if logged_in { 0 } else { 1 }) {
        return Err("Claude account status and process exit disagreed. Check the CLI setup again.");
    }
    let auth_method = match value.auth_method.as_str() {
        "none" => ClaudeAuthMethod::None,
        "claude.ai" => ClaudeAuthMethod::ClaudeAccount,
        "oauth_token" => ClaudeAuthMethod::OAuthToken,
        "api_key" => ClaudeAuthMethod::ApiKey,
        "api_key_helper" => ClaudeAuthMethod::ApiKeyHelper,
        "third_party" => ClaudeAuthMethod::ThirdParty,
        _ => ClaudeAuthMethod::Unrecognized,
    };
    if logged_in && auth_method == ClaudeAuthMethod::None
        || !logged_in && auth_method != ClaudeAuthMethod::None
    {
        return Err("Claude reported inconsistent login metadata. Check the CLI setup again.");
    }
    let first_party_provider = value.api_provider.as_deref() == Some("firstParty");
    let account_email = bounded_text(value.email.as_deref(), 320).filter(|email| {
        email.contains('@') && !email.bytes().any(|byte| byte.is_ascii_whitespace())
    });
    let subscription_type = value.subscription_type.as_deref().and_then(|plan| {
        matches!(plan, "free" | "pro" | "max" | "team" | "enterprise").then(|| plan.to_owned())
    });
    let config_directory =
        bounded_text(value.config_directory.as_deref(), 4096).filter(|directory| {
            let path = Path::new(directory);
            path.is_absolute()
                && !path
                    .components()
                    .any(|part| matches!(part, Component::ParentDir))
        });
    // Missing identity/configuration data is visible as unbound metadata, not
    // replaced with a global account identity or a fabricated account catalog.
    let org_id = bounded_text(value.org_id.as_deref(), 256);
    let raw_plan = bounded_text(value.subscription_type.as_deref(), 128);
    let optional_identity_valid = (value.org_id.is_none() || org_id.is_some())
        && (value.subscription_type.is_none() || raw_plan.is_some());
    let account_identity_sha256 = if optional_identity_valid
        && logged_in
        && auth_method == ClaudeAuthMethod::ClaudeAccount
        && first_party_provider
        && account_email.is_some()
        && config_directory.is_some()
    {
        let identity = json!({
            "version": 1, "authMethod": "claude.ai", "apiProvider": "firstParty",
            "email": account_email, "orgId": org_id,
            "subscriptionType": raw_plan, "configDirectory": config_directory,
        });
        Some(format!(
            "{:x}",
            Sha256::digest(identity.to_string().as_bytes())
        ))
    } else {
        None
    };
    Ok(ClaudeAccountObservation {
        logged_in,
        auth_method,
        first_party_provider,
        account_email,
        subscription_type,
        config_directory,
        account_identity_sha256,
    })
}

fn bounded_text(value: Option<&str>, max_bytes: usize) -> Option<String> {
    value
        .filter(|text| {
            !text.is_empty() && text.len() <= max_bytes && !text.chars().any(char::is_control)
        })
        .map(str::to_owned)
}
