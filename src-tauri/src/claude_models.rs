//! Candidate SDK model metadata, admitted only from a native managed worker.
//!
//! A catalog is neither account entitlement nor runtime qualification. This
//! module has no command accepting catalogs from the WebView and starts no
//! process. Execution still requires the separately qualified account route.

use crate::claude_agent::{CANDIDATE_RUNTIME_VERSION, ClaudeModelCapability, is_exact_model_id};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const CANDIDATE_SDK_VERSION: &str = "0.3.300";
pub(crate) const MAX_CATALOG_BYTES: usize = 128 * 1024;
const MAX_MODEL_ROWS: usize = 256;
const EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
const INVALID: &str =
    "Claude model metadata was incomplete or incompatible. No generation settings were accepted.";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeModelCatalog {
    sdk_version: String,
    runtime_version: String,
    models: Vec<ClaudeModelObservation>,
    /// Valid metadata rows lacking an exact supported model ID are omitted.
    omitted_rows: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeModelObservation {
    model_id: String,
    display_name: String,
    aliases: Vec<String>,
    effort_support_advertised: bool,
    supported_efforts: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CatalogWire {
    protocol_version: u32,
    sdk_version: String,
    runtime_version: String,
    models: Vec<ModelWire>,
}

/// Only fields projected by our SDK adapter. A changed bridge contract must be
/// qualified explicitly; raw vendor initialization/account objects are invalid.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ModelWire {
    value: String,
    resolved_model: Option<String>,
    display_name: String,
    supports_effort: Option<bool>,
    supported_effort_levels: Option<Vec<String>>,
}

struct ModelGroup {
    names_by_value: BTreeMap<String, String>,
    effort_support_advertised: bool,
    supported_efforts: Vec<String>,
}

pub(crate) fn parse_model_catalog(bytes: &[u8]) -> Result<ClaudeModelCatalog, &'static str> {
    if bytes.is_empty() || bytes.len() > MAX_CATALOG_BYTES {
        return Err(INVALID);
    }
    let wire: CatalogWire = serde_json::from_slice(bytes).map_err(|_| INVALID)?;
    if wire.protocol_version != 1
        || wire.sdk_version != CANDIDATE_SDK_VERSION
        || wire.runtime_version != CANDIDATE_RUNTIME_VERSION
        || wire.models.is_empty()
        || wire.models.len() > MAX_MODEL_ROWS
    {
        return Err(INVALID);
    }
    let mut values = BTreeSet::new();
    let mut groups: BTreeMap<String, ModelGroup> = BTreeMap::new();
    let mut omitted_rows = 0;
    for row in wire.models {
        if !bounded_text(&row.value, 200)
            || row.value.bytes().any(|byte| byte.is_ascii_whitespace())
            || !bounded_text(&row.display_name, 160)
            || row.resolved_model.as_ref().is_some_and(|id| {
                !bounded_text(id, 200) || id.bytes().any(|byte| byte.is_ascii_whitespace())
            })
            || !values.insert(row.value.clone())
        {
            return Err(INVALID);
        }
        let advertised = row.supports_effort == Some(true);
        let levels = row.supported_effort_levels.unwrap_or_default();
        if levels.len() > EFFORTS.len() || (!levels.is_empty() && !advertised) {
            return Err(INVALID);
        }
        let mut unique_levels = BTreeSet::new();
        for level in levels {
            if !EFFORTS.contains(&level.as_str()) || !unique_levels.insert(level) {
                return Err(INVALID);
            }
        }
        // A support boolean without enumerated levels permits only the
        // explicitly reviewed runtime default, never an invented effort list.
        let efforts: Vec<String> = EFFORTS
            .iter()
            .filter(|effort| unique_levels.contains(**effort))
            .map(|effort| (*effort).to_owned())
            .collect();
        let model_id = match row.resolved_model.as_deref() {
            Some(resolved) if is_exact_model_id(resolved) => resolved.to_owned(),
            None if is_exact_model_id(&row.value) => row.value.clone(),
            _ => {
                omitted_rows += 1;
                continue;
            }
        };
        let group = groups.entry(model_id).or_insert_with(|| ModelGroup {
            names_by_value: BTreeMap::new(),
            effort_support_advertised: advertised,
            supported_efforts: efforts.clone(),
        });
        if group.effort_support_advertised != advertised || group.supported_efforts != efforts {
            return Err(INVALID);
        }
        group.names_by_value.insert(row.value, row.display_name);
    }
    if groups.is_empty() {
        return Err(
            "Claude did not advertise any exact model IDs this adapter can use. Refresh the qualified runtime catalog.",
        );
    }
    let mut models = Vec::with_capacity(groups.len());
    for (model_id, group) in groups {
        // Prefer the explicit row's label, otherwise select deterministically
        // by alias. Input order must not change a catalog's effective controls.
        let display_name = group
            .names_by_value
            .get(&model_id)
            .or_else(|| group.names_by_value.first_key_value().map(|(_, name)| name))
            .ok_or(INVALID)?
            .clone();
        let aliases = group
            .names_by_value
            .into_keys()
            .filter(|value| value != &model_id)
            .collect();
        models.push(ClaudeModelObservation {
            model_id,
            display_name,
            aliases,
            effort_support_advertised: group.effort_support_advertised,
            supported_efforts: group.supported_efforts,
        });
    }
    Ok(ClaudeModelCatalog {
        sdk_version: wire.sdk_version,
        runtime_version: wire.runtime_version,
        models,
        omitted_rows,
    })
}

impl ClaudeModelCatalog {
    /// Candidate capabilities only. The native caller must separately bind
    /// fresh executable/account identity and establish runtime qualification.
    pub(crate) fn capabilities(&self) -> Vec<ClaudeModelCapability> {
        self.models
            .iter()
            .map(|model| ClaudeModelCapability {
                model_id: model.model_id.clone(),
                supported_efforts: model.supported_efforts.clone(),
            })
            .collect()
    }
}

fn bounded_text(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}
