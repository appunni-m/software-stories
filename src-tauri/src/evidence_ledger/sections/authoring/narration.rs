//! Versioned speech for claims that need no visual. Old authoring/replay stays
//! unchanged; fresh proposals map every claim to one authored spoken segment.

use super::{issue, Compiled, Scene};
use crate::chapters::{ChapterRevision, ClaimCategory};
use serde_json::{json, Value};

pub(crate) fn compile(section: &Value) -> Result<Compiled, Vec<Value>> {
    if section["visual"]["kind"] != "none" {
        return super::compile_native_names(section);
    }
    let mut canonical = section.clone();
    let claims = canonical["claims"]
        .as_array_mut()
        .filter(|claims| (1..=8).contains(&claims.len()))
        .ok_or_else(|| issue("/section/claims", "sceneAuthoringShape"))?;
    let mut scripts = Vec::with_capacity(claims.len());
    for (index, claim) in claims.iter_mut().enumerate() {
        let at = format!("/section/claims/{index}/scene");
        let scene: Scene = serde_json::from_value(claim["scene"].clone())
            .map_err(|_| issue(&at, "sceneAuthoringShape"))?;
        if scene.mode != "narration"
            || !scene.nodes.is_empty()
            || !scene.edges.is_empty()
            || !scene.elements.is_empty()
            || !(1..=120).contains(&scene.title.trim().len())
        {
            return Err(issue(&at, "sceneAuthoringShape"));
        }
        let script = scene.narration.trim();
        if !(20..=1_000).contains(&script.len()) {
            return Err(issue(&format!("{at}/narration"), "sceneNarration"));
        }
        scripts.push(script.to_owned());
        claim["scene"] = Value::Null;
    }
    let mut compiled = super::compile_native_names(&canonical)?;
    compiled.narration = Some(scripts);
    Ok(compiled)
}

/// Keep one explicit script per claim without creating diagram nodes, HTML or
/// source claims. Canonical source/identifier validation still runs separately.
pub(crate) fn schema() -> Value {
    let mut section = super::grounded_names_schema();
    let scenes = &mut section["properties"]["claims"]["items"]["properties"]["scene"]["anyOf"];
    let visual_scene = scenes[0].clone();
    *scenes = json!([visual_scene, {
        "type":"object", "additionalProperties":false,
        "required":["title","narration","mode","nodes","edges","elements"],
        "properties":{
            "title":{"type":"string","minLength":1,"maxLength":120},
            "narration":{"type":"string","minLength":20,"maxLength":1000},
            "mode":{"type":"string","enum":["narration"]},
            "nodes":{"type":"array","maxItems":0},
            "edges":{"type":"array","maxItems":0},
            "elements":{"type":"array","maxItems":0}
        }
    }]);
    section
}

impl Compiled {
    /// Only the new checkpoint invokes this method. Old checkpoints retain the
    /// old chapter schema and claim-copy fallback byte for byte on replay.
    pub(crate) fn apply_authored_narration(
        &self,
        revision: &mut ChapterRevision,
    ) -> Result<(), String> {
        if revision.sections.len() != 1 {
            return Err("Authored section narration needs exactly one section.".into());
        }
        if let Some(scripts) = &self.narration {
            let section = &revision.sections[0];
            if scripts.len() != section.claims.len() || section.visual.is_some() {
                return Err("Authored narration does not match its complete claim set.".into());
            }
            let complete = section
                .claims
                .iter()
                .zip(scripts)
                .map(|(claim, script)| match &claim.category {
                    ClaimCategory::Inference { visible_label, .. } => {
                        format!("{}: {script}", visible_label.trim())
                    }
                    _ => script.clone(),
                })
                .collect::<Vec<_>>()
                .join(" ");
            if !(20..=4_000).contains(&complete.len()) {
                return Err(crate::chapter_generation::COMPLETE_CLAIMS_LIMIT_ERROR.into());
            }
            revision.sections[0].spoken_text = complete;
        }
        revision.schema_version = crate::chapters::CHAPTER_SCHEMA_VERSION;
        Ok(())
    }
}
