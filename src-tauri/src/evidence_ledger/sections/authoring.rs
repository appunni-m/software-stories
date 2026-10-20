//! Compile scene-local authored identities without inventing evidence/content.
//! The existing canonical claim/source/scene validators still admit the result.

use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;

pub(crate) mod narration;

pub(crate) const POLICY: &str = "claim-owned-scene-local-identities-v3";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Settings {
    kind: String,
    title: String,
    direction: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Scene {
    title: String,
    narration: String,
    mode: String,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    elements: Vec<Element>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Node {
    key: String,
    label: String,
    detail: String,
    references: Vec<String>,
    #[serde(default)]
    source: Option<SourceBinding>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceBinding {
    reference_id: String,
    value: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Edge {
    from_key: String,
    to_key: String,
    label: String,
    references: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Element {
    key: String,
    kind: String,
    #[serde(deserialize_with = "nullable_key")]
    parent_key: Option<String>,
    #[serde(deserialize_with = "nullable_key")]
    node_key: Option<String>,
    tone: String,
    value: String,
    value_origin: String,
}

fn nullable_key<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}

pub(super) struct Compiled {
    pub(super) section: Value,
    locations: Vec<(String, String)>,
    narration: Option<Vec<String>>,
}

impl Compiled {
    /// Findings retain fixed positions/codes, expressed in the authored shape.
    pub(super) fn remap(&self, findings: &mut [Value]) {
        for finding in findings {
            let Some(location) = finding["location"].as_str() else {
                continue;
            };
            for (canonical, authored) in &self.locations {
                if location == canonical
                    || location
                        .strip_prefix(canonical)
                        .is_some_and(|s| s.starts_with('/'))
                {
                    let suffix = &location[canonical.len()..];
                    let suffix = suffix
                        .replace("/nodeIndex", "/nodeKey")
                        .replace("/parentIndex", "/parentKey")
                        .replace("/fromNodeIndex", "/fromKey")
                        .replace("/toNodeIndex", "/toKey")
                        .replace("/evidence", "/references");
                    finding["location"] = json!(format!("{authored}{suffix}"));
                    break;
                }
            }
        }
    }
}

fn issue(location: &str, code: &str) -> Vec<Value> {
    vec![json!({"location":location,"code":code})]
}

fn references(
    ids: &[String],
    evidence: &[Value],
    claim: usize,
    at: &str,
) -> Result<Vec<Value>, Vec<Value>> {
    if !(1..=8).contains(&ids.len()) {
        return Err(issue(at, "sceneAuthoringReference"));
    }
    let mut seen = std::collections::HashSet::new();
    ids.iter()
        .map(|id| {
            if !seen.insert(id) {
                return Err(issue(at, "sceneAuthoringReference"));
            }
            let positions: Vec<_> = evidence
                .iter()
                .enumerate()
                .filter(|(_, reference)| reference["referenceId"].as_str() == Some(id.as_str()))
                .map(|(index, _)| index)
                .collect();
            if positions.len() != 1 {
                return Err(issue(at, "sceneAuthoringReference"));
            }
            Ok(json!({"claimIndex":claim,"evidenceIndex":positions[0]}))
        })
        .collect()
}

pub(super) fn compile(section: &Value) -> Result<Compiled, Vec<Value>> {
    let shape = || issue("/section", "sceneAuthoringShape");
    let object = section.as_object().ok_or_else(shape)?;
    if object.len() != 3
        || !["title", "claims", "visual"]
            .iter()
            .all(|key| object.contains_key(*key))
    {
        return Err(shape());
    }
    let settings: Settings =
        serde_json::from_value(object["visual"].clone()).map_err(|_| shape())?;
    let claims = object["claims"]
        .as_array()
        .filter(|v| (1..=8).contains(&v.len()))
        .ok_or_else(shape)?;
    let mut canonical_claims = Vec::with_capacity(claims.len());
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut scenes = Vec::new();
    let mut locations = Vec::new();
    for (claim_index, claim) in claims.iter().enumerate() {
        let base = format!("/section/claims/{claim_index}/scene");
        let mut canonical = claim.as_object().cloned().ok_or_else(shape)?;
        let scene = canonical
            .remove("scene")
            .ok_or_else(|| issue(&base, "sceneAuthoringShape"))?;
        let evidence_at = format!("/section/claims/{claim_index}/evidence");
        let evidence = canonical
            .get("evidence")
            .and_then(Value::as_array)
            .ok_or_else(|| issue(&evidence_at, "sceneAuthoringShape"))?;
        if !(1..=8).contains(&evidence.len()) {
            return Err(issue(&evidence_at, "sceneAuthoringLimit"));
        }
        if settings.kind == "none" {
            if !scene.is_null() {
                return Err(issue(&base, "sceneAuthoringShape"));
            }
            canonical_claims.push(Value::Object(canonical));
            continue;
        }
        let scene: Scene =
            serde_json::from_value(scene).map_err(|_| issue(&base, "sceneAuthoringShape"))?;
        if scene.nodes.is_empty()
            || scene.nodes.len() + nodes.len() > 12
            || scene.edges.len() + edges.len() > 20
            || scene.elements.len() > 48
        {
            return Err(issue(&base, "sceneAuthoringLimit"));
        }
        let mut node_keys = HashMap::new();
        for (index, node) in scene.nodes.into_iter().enumerate() {
            let at = format!("{base}/nodes/{index}");
            if !super::valid_key(&node.key) || node_keys.insert(node.key, nodes.len()).is_some() {
                return Err(issue(&format!("{at}/key"), "sceneAuthoringKey"));
            }
            let refs = references(
                &node.references,
                evidence,
                claim_index,
                &format!("{at}/references"),
            )?;
            let source = match node.source {
                Some(source) => {
                    if !node.references.contains(&source.reference_id) {
                        return Err(issue(
                            &format!("{at}/source/referenceId"),
                            "sceneAuthoringReference",
                        ));
                    }
                    let source_evidence = references(
                        &[source.reference_id],
                        evidence,
                        claim_index,
                        &format!("{at}/source/referenceId"),
                    )?
                    .into_iter()
                    .next()
                    .ok_or_else(|| issue(&format!("{at}/source/referenceId"), "sceneAuthoringReference"))?;
                    json!({"evidence":source_evidence,"value":source.value})
                }
                None => Value::Null,
            };
            locations.push((format!("/section/visual/nodes/{}", nodes.len()), at));
            nodes.push(json!({"label":node.label,"detail":node.detail,"evidence":refs,"source":source}));
        }
        for (index, edge) in scene.edges.into_iter().enumerate() {
            let at = format!("{base}/edges/{index}");
            let from = node_keys
                .get(&edge.from_key)
                .ok_or_else(|| issue(&format!("{at}/fromKey"), "sceneAuthoringKey"))?;
            let to = node_keys
                .get(&edge.to_key)
                .ok_or_else(|| issue(&format!("{at}/toKey"), "sceneAuthoringKey"))?;
            let refs = references(
                &edge.references,
                evidence,
                claim_index,
                &format!("{at}/references"),
            )?;
            locations.push((format!("/section/visual/edges/{}", edges.len()), at));
            edges.push(
                json!({"fromNodeIndex":from,"toNodeIndex":to,"label":edge.label,"evidence":refs}),
            );
        }
        let mut elements = Vec::with_capacity(scene.elements.len());
        let mut element_keys = HashMap::new();
        for (index, element) in scene.elements.into_iter().enumerate() {
            let at = format!("{base}/elements/{index}");
            if !super::valid_key(&element.key) || element_keys.contains_key(&element.key) {
                return Err(issue(&format!("{at}/key"), "sceneAuthoringKey"));
            }
            let parent = element
                .parent_key
                .as_ref()
                .map(|key| {
                    element_keys
                        .get(key)
                        .copied()
                        .ok_or_else(|| issue(&format!("{at}/parentKey"), "sceneAuthoringKey"))
                })
                .transpose()?;
            let node = element
                .node_key
                .as_ref()
                .map(|key| {
                    node_keys
                        .get(key)
                        .copied()
                        .ok_or_else(|| issue(&format!("{at}/nodeKey"), "sceneAuthoringKey"))
                })
                .transpose()?;
            element_keys.insert(element.key, index);
            elements.push(
                json!({"kind":element.kind,"parentIndex":parent,"nodeIndex":node,
                "tone":element.tone,"value":element.value,"valueOrigin":element.value_origin}),
            );
        }
        locations.push((
            format!("/section/visual/presentation/scenes/{claim_index}"),
            base,
        ));
        scenes.push(json!({"claimIndex":claim_index,"title":scene.title,"narration":scene.narration,"mode":scene.mode,"elements":elements}));
        canonical_claims.push(Value::Object(canonical));
    }
    let mut visual = json!({"kind":settings.kind,"title":settings.title,"direction":settings.direction,"nodes":nodes,"edges":edges});
    if !scenes.is_empty() {
        visual["presentation"] = json!({"schemaVersion":3,"scenes":scenes});
        // Aggregate script findings refer to all claims, not to a canonical
        // presentation property that the agent never authored. Keep this after
        // the more specific per-scene prefixes above.
        locations.push((
            "/section/visual/presentation/scenes".into(),
            "/section/claims".into(),
        ));
    }
    let compiled_section =
        json!({"title":object["title"],"claims":canonical_claims,"visual":visual});
    for (claim_index, scene) in compiled_section["visual"]["presentation"]["scenes"]
        .as_array()
        .into_iter()
        .flatten()
        .enumerate()
    {
        if scene["mode"] == "graph"
            && crate::claim_alignment::graph_scene_node_count(&compiled_section, claim_index)
                .is_some_and(|count| count > crate::scene_presentation::MAX_FOCUSED_GRAPH_NODES)
        {
            return Err(issue(
                &format!("/section/claims/{claim_index}/scene/nodes"),
                "sceneAuthoringLimit",
            ));
        }
    }
    Ok(Compiled {
        section: compiled_section,
        locations,
        narration: None,
    })
}

/// Names are compiled only after every native citation has passed ownership,
/// delivery and captured-byte checks.
pub(super) fn compile_current_section(section: &Value) -> Result<Compiled, Vec<Value>> {
    if let Some(claims) = section["claims"].as_array() {
        for (index, claim) in claims.iter().enumerate() {
            if claim.get("identifiers").is_some() {
                return Err(issue(
                    &format!("/section/claims/{index}/identifiers"),
                    "namedIdentifier",
                ));
            }
        }
    }
    compile(section)
}

/// The sole current proposal schema, including claim-owned visual keys.
pub(crate) fn section_schema() -> Value {
    let mut section =
        crate::provider::research_response_schema()["properties"]["sections"]["items"].clone();
    let properties = section["properties"].as_object_mut().unwrap();
    properties.remove("prose");
    properties.remove("spokenText");
    properties.get_mut("claims").unwrap()["maxItems"] = json!(8);
    let claim = &mut properties.get_mut("claims").unwrap()["items"];
    claim["properties"]
        .as_object_mut()
        .unwrap()
        .remove("identifiers");
    claim["required"]
        .as_array_mut()
        .unwrap()
        .retain(|field| field != "identifiers");
    claim["properties"]["prose"]["description"] = json!(
        "Mark code names/configuration keys with single backticks. Each claim's cited fragment must contain a whole marked name or literal qualified-name suffix also present in prose. Native name metadata is not namespace or semantic proof; evidence must support the whole claim. Omit identifiers."
    );
    claim["properties"]["evidence"]["items"]["properties"]["referenceId"]["pattern"] =
        json!("^code:[1-9][0-9]{0,2}$");
    let text = json!({"type":"string"});
    let key = json!({"type":"string","minLength":1,"maxLength":80,"pattern":"^[a-z0-9-]+$"});
    let nullable_key = json!({"anyOf":[key.clone(),{"type":"null"}]});
    let refs = json!({"type":"array","minItems":1,"maxItems":8,"items":{"type":"string","pattern":"^code:[1-9][0-9]{0,2}$"}});
    let node = json!({"type":"object","additionalProperties":false,"required":["key","label","detail","references","source"],
        "properties":{"key":key,"label":text,"detail":text,"references":refs,
        "source":{"anyOf":[{"type":"object","additionalProperties":false,"required":["referenceId","value"],
            "properties":{"referenceId":{"type":"string","pattern":"^code:[1-9][0-9]{0,2}$"},"value":{"type":"string","minLength":1,"maxLength":240}}},{"type":"null"}]}}});
    let edge = json!({"type":"object","additionalProperties":false,"required":["fromKey","toKey","label","references"],
        "properties":{"fromKey":key,"toKey":key,"label":text,"references":refs}});
    let mut element = crate::scene_presentation::proposal_schema()["properties"]["scenes"]["items"]
        ["properties"]["elements"]["items"]
        .clone();
    let fields = element["properties"].as_object_mut().unwrap();
    fields.remove("parentIndex");
    fields.remove("nodeIndex");
    fields.insert("key".into(), key.clone());
    fields.insert("parentKey".into(), nullable_key.clone());
    fields.insert("nodeKey".into(), nullable_key);
    element["required"] = json!([
        "key",
        "kind",
        "parentKey",
        "nodeKey",
        "tone",
        "value",
        "valueOrigin"
    ]);
    claim["properties"]["scene"] = json!({"anyOf":[{"type":"object","additionalProperties":false,
        "required":["title","narration","mode","nodes","edges","elements"],"properties":{
        "title":text,"narration":{"type":"string","minLength":1,"maxLength":4000},"mode":{"type":"string","enum":["composition","graph"]},
        "nodes":{"type":"array","minItems":1,"maxItems":12,"items":node},"edges":{"type":"array","maxItems":20,"items":edge},
        "elements":{"type":"array","maxItems":48,"items":element}}},{"type":"null"}]});
    claim["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("scene"));
    let mut visual = properties["visual"].clone();
    let fields = visual["properties"].as_object_mut().unwrap();
    fields.remove("nodes");
    fields.remove("edges");
    fields.remove("presentation");
    visual["required"] = json!(["kind", "title", "direction"]);
    properties.insert("visual".into(), visual);
    section["required"] = json!(["title", "claims", "visual"]);
    section
}

#[cfg(test)]
pub(crate) mod tests;
