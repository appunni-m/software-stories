//! Generation admission from release-owned qualification records.
//!
//! Observed installation/account/catalog metadata is not a qualification. Only
//! this module can construct a generation runtime, and no command accepts a
//! profile, permission flag or runtime identity from the renderer. The bundled
//! registry is deliberately empty until the exact integration has evidence.

use crate::{
    claude_agent::{CANDIDATE_RUNTIME_VERSION, ClaudeReviewRuntime},
    claude_discovery::{ClaudeModelDiscoveryObservation, SDK_MODULE_SHA256},
    claude_models::CANDIDATE_SDK_VERSION,
    provider::{ProviderFailure, ProviderFailureKind},
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};

/// Captured only from a successful native managed catalog observation.
/// This is not deserializable or supplied by a command caller.
pub(crate) struct ClaudeCatalogInstallation {
    node_sha256: String,
    node_version: String,
    sdk_sha256: String,
    subscription_type: Option<String>,
}

impl ClaudeCatalogInstallation {
    pub(crate) fn observed(value: &ClaudeModelDiscoveryObservation) -> Self {
        Self {
            node_sha256: value.node_executable_sha256.clone(),
            node_version: value.node_version.clone(),
            sdk_sha256: value.sdk_module_sha256.to_owned(),
            subscription_type: value
                .connection
                .account
                .as_ref()
                .and_then(|account| account.subscription_type.clone()),
        }
    }
}

/// Cannot be constructed from a connection ID, catalog or renderer DTO.
/// Its inner runtime is the same native Arc used by the revocable connection.
pub(crate) struct ClaudeGenerationRuntime(Arc<ClaudeReviewRuntime>);

impl ClaudeGenerationRuntime {
    pub(crate) fn into_review_runtime(self) -> Arc<ClaudeReviewRuntime> {
        self.0
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeGenerationAvailability {
    available: bool,
    qualification_id: Option<String>,
    adapter_sha256: &'static str,
    message: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Registry {
    schema_version: u32,
    profiles: Vec<Qualification>,
}

/// Release-owned evidence references are identifiers, not executable policy.
/// Filling a record is a release decision backed by the documented acceptance
/// audit; a nonempty string or a successful metadata probe is not that audit.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Qualification {
    id: String,
    app_version: String,
    adapter_sha256: String,
    platform: String,
    architecture: String,
    runtime_version: String,
    cli_sha256: String,
    node_version: String,
    node_sha256: String,
    sdk_version: String,
    sdk_sha256: String,
    account_route: String,
    subscription_types: Vec<String>,
    account_route_evidence_sha256: String,
    runtime_boundary_evidence_sha256: String,
    operation_acceptance_evidence_sha256: String,
}

const UNAVAILABLE: &str = "Automatic Claude generation is unavailable in this build. Its exact runtime, account route and operation behavior have not completed qualification. Your local request preview remains available; no agent request was started.";

pub(crate) fn availability(
    runtime: &ClaudeReviewRuntime,
    installation: &ClaudeCatalogInstallation,
) -> ClaudeGenerationAvailability {
    match qualification(runtime, installation) {
        Some(profile) => ClaudeGenerationAvailability {
            available: true,
            qualification_id: Some(profile.id.clone()),
            adapter_sha256: adapter_sha256(),
            message: "This installed Claude route matches this build's qualified runtime profile. Every run still requires fresh source review and explicit consent; account limits apply.",
        },
        None => ClaudeGenerationAvailability {
            available: false,
            qualification_id: None,
            adapter_sha256: adapter_sha256(),
            message: UNAVAILABLE,
        },
    }
}

pub(crate) fn admit(
    runtime: &Arc<ClaudeReviewRuntime>,
    installation: &ClaudeCatalogInstallation,
) -> Result<ClaudeGenerationRuntime, ProviderFailure> {
    qualification(runtime, installation)
        .map(|_| ClaudeGenerationRuntime(runtime.clone()))
        .ok_or_else(|| ProviderFailure {
            kind: ProviderFailureKind::ReviewUnavailable,
            message: UNAVAILABLE.to_owned(),
        })
}

fn qualification(
    runtime: &ClaudeReviewRuntime,
    installation: &ClaudeCatalogInstallation,
) -> Option<&'static Qualification> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    let registry = registry()?;
    let mut matches = registry.profiles.iter().filter(|profile| {
        profile.app_version == env!("CARGO_PKG_VERSION")
            && profile.adapter_sha256 == adapter_sha256()
            && profile.platform == std::env::consts::OS
            && profile.architecture == std::env::consts::ARCH
            && profile.runtime_version == runtime.runtime_version
            && profile.cli_sha256 == runtime.executable_sha256
            && profile.node_version == installation.node_version
            && profile.node_sha256 == installation.node_sha256
            && profile.sdk_sha256 == installation.sdk_sha256
            && installation
                .subscription_type
                .as_ref()
                .is_some_and(|plan| profile.subscription_types.contains(plan))
    });
    let profile = matches.next()?;
    // Ambiguous release configuration cannot widen runtime admission.
    matches.next().is_none().then_some(profile)
}

fn registry() -> Option<&'static Registry> {
    static VALUE: OnceLock<Option<Registry>> = OnceLock::new();
    VALUE
        .get_or_init(|| {
            let registry: Registry = serde_json::from_str(include_str!(
                "../resources/claude_generation_qualifications.json"
            ))
            .ok()?;
            if registry.schema_version != 1 || registry.profiles.len() > 16 {
                return None;
            }
            let mut ids = std::collections::HashSet::new();
            for profile in &registry.profiles {
                if profile.id.is_empty()
                    || profile.id.len() > 120
                    || !profile
                        .id
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
                    || !ids.insert(profile.id.clone())
                    || profile.runtime_version != CANDIDATE_RUNTIME_VERSION
                    || profile.sdk_version != CANDIDATE_SDK_VERSION
                    || profile.sdk_sha256 != SDK_MODULE_SHA256
                    || profile.account_route != "existingClaudeAccount"
                    || profile.subscription_types.is_empty()
                    || profile.subscription_types.len() > 5
                    || profile.subscription_types.iter().any(|plan| {
                        !matches!(
                            plan.as_str(),
                            "free" | "pro" | "max" | "team" | "enterprise"
                        )
                    })
                    || profile
                        .subscription_types
                        .iter()
                        .collect::<std::collections::HashSet<_>>()
                        .len()
                        != profile.subscription_types.len()
                    || profile.platform != "macos"
                    || !matches!(profile.architecture.as_str(), "aarch64" | "x86_64")
                    || [
                        &profile.adapter_sha256,
                        &profile.cli_sha256,
                        &profile.node_sha256,
                        &profile.sdk_sha256,
                        &profile.account_route_evidence_sha256,
                        &profile.runtime_boundary_evidence_sha256,
                        &profile.operation_acceptance_evidence_sha256,
                    ]
                    .iter()
                    .any(|value| !digest(value))
                {
                    return None;
                }
            }
            Some(registry)
        })
        .as_ref()
}

fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Bind a release receipt to this native implementation, including review,
/// lifecycle, model admission and validation. Changing it requires new evidence.
/// Receipt contents are excluded so adding evidence cannot be self-referential.
fn adapter_sha256() -> &'static str {
    env!("CODE_ATLAS_CLAUDE_ADAPTER_SHA256")
}
