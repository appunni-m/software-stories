//! Observed app-server/account identity bound to a native request review.
//! This is local change detection, not entitlement or immutable-execution proof.

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::{Component, Path};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum CodexRuntimeVersionSource {
    ServerInfoVersion,
    UserAgent,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CodexReviewRuntime {
    pub connection_id: String,
    pub executable_path: String,
    pub executable_sha256: String,
    /// Codex app-server version observed during initialize.
    #[serde(deserialize_with = "required_nullable_field")]
    pub runtime_version: Option<String>,
    /// Provenance for the observed runtime version.
    #[serde(deserialize_with = "required_nullable_field")]
    pub runtime_version_source: Option<CodexRuntimeVersionSource>,
    pub account_identity_sha256: String,
    pub account_revision: u64,
}

impl CodexReviewRuntime {
    pub(crate) fn from_observation(
        connection_id: &str,
        executable_path: &Path,
        executable_sha256: &str,
        runtime_version: &str,
        runtime_version_source: CodexRuntimeVersionSource,
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
            runtime_version: Some(runtime_version.to_owned()),
            runtime_version_source: Some(runtime_version_source),
            account_identity_sha256: format!("{:x}", Sha256::digest(identity)),
            account_revision,
        };
        if !runtime.current_review_shape() {
            return Err(changed());
        }
        Ok(runtime)
    }

    /// New source-bearing reviews require the observed app-server version.
    pub(crate) fn current_review_shape(&self) -> bool {
        self.valid_shape()
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
            && self
                .runtime_version
                .as_deref()
                .is_some_and(|version| text(version, 128))
            && self.runtime_version_source.is_some()
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

fn required_nullable_field<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(deserializer)
}

#[cfg(test)]
mod tests {
    use super::{CodexReviewRuntime, CodexRuntimeVersionSource};
    use serde_json::{Value, json};
    use std::path::Path;

    #[test]
    fn runtime_version_provenance_is_required_by_the_current_review_contract() {
        let account = json!({
            "requiresOpenaiAuth": true,
            "account": {
                "type": "chatgpt",
                "email": "codex-test@example.invalid",
                "planType": "plus"
            }
        });
        let runtime = CodexReviewRuntime::from_observation(
            "00000000-0000-4000-8000-000000000001",
            Path::new("/usr/local/bin/codex"),
            &"a".repeat(64),
            "0.166.1",
            CodexRuntimeVersionSource::ServerInfoVersion,
            3,
            &account,
        )
        .unwrap();

        assert_eq!(
            runtime.runtime_version_source,
            Some(CodexRuntimeVersionSource::ServerInfoVersion)
        );
        assert!(runtime.current_review_shape());
        let serialized = serde_json::to_value(&runtime).unwrap();
        assert_eq!(serialized["runtimeVersionSource"], "serverInfoVersion");

        let mut missing_source = serialized.clone();
        missing_source
            .as_object_mut()
            .unwrap()
            .remove("runtimeVersionSource");
        assert!(serde_json::from_value::<CodexReviewRuntime>(missing_source).is_err());

        let mut missing_version = serialized;
        missing_version
            .as_object_mut()
            .unwrap()
            .remove("runtimeVersion");
        assert!(serde_json::from_value::<CodexReviewRuntime>(missing_version).is_err());

        let mut malformed: Value = serde_json::to_value(&runtime).unwrap();
        malformed["runtimeVersion"] = Value::Null;
        assert!(
            !serde_json::from_value::<CodexReviewRuntime>(malformed)
                .unwrap()
                .valid_shape()
        );
    }
}
