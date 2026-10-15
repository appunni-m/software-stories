//! Native-owned inputs and fixed preparation for candidate SDK discovery.
//!
//! Runtime installation/provenance and account-route qualification belong to
//! the native caller. No WebView command can deserialize this runtime record.

use crate::claude_agent;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) const SDK_MODULE_SHA256: &str =
    "c1ccee152f98ff6899c2989b6151086e5d0ff34aedd93bd98cc4576619c41d5f";
pub(crate) const MAX_SDK_MODULE_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_DISCOVERY_INPUT_BYTES: usize = 32 * 1024;
pub(crate) const DISCOVERY_SCRIPT: &str = include_str!("../resources/claude_discovery_worker.mjs");
pub(crate) const DISCOVERY_LIBRARY: &str = include_str!("../resources/claude_model_discovery.mjs");

pub(crate) struct ClaudeDiscoveryRuntime {
    pub node_executable: PathBuf,
    pub node_sha256: String,
    pub claude_executable: PathBuf,
    pub claude_sha256: String,
    /// Exact inspected SDK module from a trusted native installation. The
    /// worker verifies and copies its bytes into its private read-only stage.
    pub sdk_module: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeModelDiscoveryObservation {
    pub connection: crate::claude_connection::ClaudeConnectionObservation,
    pub catalog: crate::claude_models::ClaudeModelCatalog,
    pub node_executable_path: String,
    pub node_executable_sha256: String,
    pub sdk_module_sha256: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveryInput<'a> {
    protocol_version: u32,
    executable: &'a Path,
    stage: &'a Path,
    temporary: &'a Path,
    account_home: &'a Path,
}

pub(crate) fn prepare(
    runtime: &ClaudeDiscoveryRuntime,
    stage: &Path,
    temporary: &Path,
    account_home: &Path,
) -> Result<(Command, Vec<u8>), String> {
    if ![&runtime.node_sha256, &runtime.claude_sha256]
        .iter()
        .all(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        || !valid_path(&runtime.claude_executable)
        || !valid_path(&runtime.sdk_module)
    {
        return Err("The native Claude discovery runtime identity is incomplete.".to_owned());
    }
    let mut command =
        claude_agent::isolated_command(&runtime.node_executable, stage, temporary, account_home)?;
    command.arg(stage.join("worker.mjs"));
    let input = serde_json::to_vec(&DiscoveryInput {
        protocol_version: 1,
        executable: &runtime.claude_executable,
        stage,
        temporary,
        account_home,
    })
    .map_err(|_| "The native Claude discovery paths could not be encoded.".to_owned())?;
    if input.len() > MAX_DISCOVERY_INPUT_BYTES {
        return Err(
            "The native Claude discovery configuration exceeds its local limit.".to_owned(),
        );
    }
    Ok((command, input))
}

fn valid_path(path: &Path) -> bool {
    path.is_absolute()
        && path.to_str().is_some()
        && path.as_os_str().len() <= 4096
        && !path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
}
