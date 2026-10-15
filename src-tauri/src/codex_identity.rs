//! Observed app-server/account identity bound to a native request review.
//! This is local change detection, not entitlement or immutable-execution proof.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CodexReviewRuntime {
    pub connection_id: String,
    pub executable_path: String,
    pub executable_sha256: String,
    pub account_identity_sha256: String,
    pub account_revision: u64,
}

impl CodexReviewRuntime {
    pub(crate) fn from_observation(
        connection_id: &str,
        executable_path: &Path,
        executable_sha256: &str,
        account_revision: u64,
        account: &Value,
    ) -> Result<Self, String> {
        if account.get("requiresOpenaiAuth").and_then(Value::as_bool) != Some(true) {
            return Err(changed());
        }
        let account = account.get("account").ok_or_else(changed)?;
        if account.get("type").and_then(Value::as_str) != Some("chatgpt") {
            return Err("Connect a ChatGPT account before reviewing a Codex request. This route does not use an API key.".to_owned());
        }
        // The public stable account surface exposes email and plan, not a
        // guaranteed immutable workspace/account ID. Do not invent that ID.
        let email = account.get("email").and_then(Value::as_str)
            .filter(|value| text(value, 320) && value.contains('@')
                && !value.chars().any(char::is_whitespace))
            .ok_or("Codex did not expose enough account metadata to bind this review. Reconnect before preparing it again.")?;
        let plan = account
            .get("planType")
            .and_then(Value::as_str)
            .filter(|value| text(value, 80))
            .ok_or_else(changed)?;
        let identity = serde_json::to_vec(&json!({
            "version": 1, "type": "chatgpt", "email": email, "planType": plan,
        }))
        .map_err(|_| changed())?;
        let runtime = Self {
            connection_id: connection_id.to_owned(),
            executable_path: executable_path.to_str().ok_or_else(changed)?.to_owned(),
            executable_sha256: executable_sha256.to_owned(),
            account_identity_sha256: format!("{:x}", Sha256::digest(identity)),
            account_revision,
        };
        if !runtime.valid_shape() {
            return Err(changed());
        }
        Ok(runtime)
    }

    pub(crate) fn valid_shape(&self) -> bool {
        Uuid::parse_str(&self.connection_id)
            .ok()
            .is_some_and(|id| id.to_string() == self.connection_id)
            && text(&self.executable_path, 4096)
            && Path::new(&self.executable_path).is_absolute()
            && !Path::new(&self.executable_path)
                .components()
                .any(|part| matches!(part, Component::ParentDir))
            && digest(&self.executable_sha256)
            && digest(&self.account_identity_sha256)
    }
}

pub(crate) fn changed() -> String {
    "The Codex account or runtime changed or is unavailable. Reconnect, load models and prepare a fresh review before sending source.".to_owned()
}

fn text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
