//! Versioned native record serialization. Schema 1 existed before and after
//! shared enum fields changed from snake_case to camelCase. Preserve those two
//! known encodings so accepting/loading a route never changes its digest or
//! chapter lineage. New records use schema 2 with current field names.

use super::{LEGACY_SAVED_PLAN_RECORD_SCHEMA_VERSION, SAVED_PLAN_RECORD_SCHEMA_VERSION};
use super::{RepositoryPlanReviewScope, SavedPlanAgentSettings, SavedPlanRecord};
use crate::provider::RepositoryPlanPreview;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use serde_json::{Map, Value, value::RawValue};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum RecordEncoding {
    Legacy,
    Current,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RecordWire {
    schema_version: u32,
    preview: Box<RawValue>,
    scope: RepositoryPlanReviewScope,
    selected_paths: Vec<String>,
    settings: SavedPlanAgentSettings,
    response_text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordOutput<'a> {
    schema_version: u32,
    preview: Value,
    scope: &'a RepositoryPlanReviewScope,
    selected_paths: &'a [String],
    settings: &'a SavedPlanAgentSettings,
    response_text: &'a str,
}

impl<'de> Deserialize<'de> for SavedPlanRecord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = RecordWire::deserialize(deserializer)?;
        // Decode strict typed DTOs before inspecting aliases. Unknown/duplicate
        // enum fields are errors; raw saved JSON is never a substitute validator.
        let preview: RepositoryPlanPreview =
            serde_json::from_str(wire.preview.get()).map_err(D::Error::custom)?;
        // Inspect only after strict decoding, which rejects duplicate fields.
        let preview_value: Value =
            serde_json::from_str(wire.preview.get()).map_err(D::Error::custom)?;
        let encoding = detect_encoding(&preview_value).map_err(D::Error::custom)?;
        if !matches!(
            wire.schema_version,
            LEGACY_SAVED_PLAN_RECORD_SCHEMA_VERSION | SAVED_PLAN_RECORD_SCHEMA_VERSION
        ) || (wire.schema_version == SAVED_PLAN_RECORD_SCHEMA_VERSION
            && encoding != RecordEncoding::Current)
        {
            return Err(D::Error::custom("Unsupported saved plan record encoding."));
        }
        Ok(Self {
            schema_version: wire.schema_version,
            preview,
            scope: wire.scope,
            selected_paths: wire.selected_paths,
            settings: wire.settings,
            response_text: wire.response_text,
            encoding,
        })
    }
}

impl Serialize for SavedPlanRecord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut preview = serde_json::to_value(&self.preview).map_err(serde::ser::Error::custom)?;
        if self.encoding == RecordEncoding::Legacy {
            for claim in claims_mut(&mut preview) {
                if let Some(category) = claim.get_mut("category").and_then(Value::as_object_mut) {
                    rename_in_place(category, &[("visibleLabel", "visible_label")]);
                }
                if let Some(dependencies) =
                    claim.get_mut("dependencies").and_then(Value::as_array_mut)
                {
                    for dependency in dependencies {
                        if let Some(fields) = dependency.as_object_mut() {
                            rename_in_place(
                                fields,
                                &[
                                    ("sourceSha256", "source_sha256"),
                                    ("symbolId", "symbol_id"),
                                    ("relationshipId", "relationship_id"),
                                ],
                            );
                        }
                    }
                }
            }
        }
        RecordOutput {
            schema_version: self.schema_version,
            preview,
            scope: &self.scope,
            selected_paths: &self.selected_paths,
            settings: &self.settings,
            response_text: &self.response_text,
        }
        .serialize(serializer)
    }
}

fn detect_encoding(preview: &Value) -> Result<RecordEncoding, &'static str> {
    let mut legacy = false;
    let mut current = false;
    let plan = &preview["plan"];
    for claim in plan["steps"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|step| &step["claim"])
        .chain(plan["unknowns"].as_array().into_iter().flatten())
    {
        let mut observe = |object: &Value, old: &str, new: &str| {
            legacy |= object.get(old).is_some();
            current |= object.get(new).is_some();
        };
        observe(&claim["category"], "visible_label", "visibleLabel");
        for dependency in claim["dependencies"].as_array().into_iter().flatten() {
            observe(dependency, "source_sha256", "sourceSha256");
            observe(dependency, "symbol_id", "symbolId");
            observe(dependency, "relationship_id", "relationshipId");
        }
    }
    match (legacy, current) {
        (true, true) => Err("Mixed saved plan enum encodings are unsupported."),
        (true, false) => Ok(RecordEncoding::Legacy),
        _ => Ok(RecordEncoding::Current),
    }
}

fn claims_mut(preview: &mut Value) -> Vec<&mut Value> {
    let Some(plan) = preview.get_mut("plan").and_then(Value::as_object_mut) else {
        return vec![];
    };
    let mut claims = Vec::new();
    for (key, collection) in plan {
        if key == "steps" {
            for step in collection.as_array_mut().into_iter().flatten() {
                if let Some(claim) = step.get_mut("claim") {
                    claims.push(claim);
                }
            }
        } else if key == "unknowns" {
            claims.extend(collection.as_array_mut().into_iter().flatten());
        }
    }
    claims
}

fn rename_in_place(fields: &mut Map<String, Value>, names: &[(&str, &str)]) {
    // Rebuild in the same order; removing and appending would change the exact
    // canonical native digest even though the JSON object has the same values.
    let original = std::mem::take(fields);
    for (name, value) in original {
        let renamed = names
            .iter()
            .find(|(from, _)| *from == name)
            .map(|(_, to)| *to)
            .unwrap_or(&name);
        fields.insert(renamed.to_owned(), value);
    }
}
