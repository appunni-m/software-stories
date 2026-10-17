//! Publish a grounded opening inside the original outline operation, before
//! its independent deeper units exist. This never grants a second operation.

use super::*;
use serde_json::{Value, json};

pub(super) const POLICY: &str = "first-transition-with-owned-outline-v1";
const ROLE: &str = r#"Investigate the original learning question using captured implementation code. Read the actual entry point and first input-to-decision/result transition early. Publish a small complete opening section as soon as its implementation evidence has been read, then focus on the remaining research outline. Preserve the question's causal order and full scope in that outline, including requested alternate paths and failures. The opening teaches its real transition and all relevant guards and consequences; it is not a plan or file list. For a predicate explain both true and false conditions. Do not continue publishing every deeper stage as part of the opening. Those stages belong to the distinct focused investigations you are planning within this same operation and original budgets."#;

const FINAL_INSTRUCTIONS: &str = r#"This operation opens a complete repository guide and researches its deeper outline. The original learning question remains the guide's full scope. Publish a useful illustrated opening through propose_answer_section as soon as you have read the first real input-to-decision/result transition. Do not wait for a complete repository survey or outline. The opening must teach implementation behavior; a file list, generic introduction or promise to investigate is not an answer. Its narrower role is the first concrete transition, not every alternate representation and failure family. Explain its supported guards and consequences completely. Keep the opening small, normally one section, while preserving every fact you choose to teach. Continue researching the outline in the SAME owned investigation and original budgets; never start a second operation to get an opening.

Return exactly {outline,openingAnswer}. openingAnswer is answerManifestV1: list this investigation's latest published sectionKey/generationId pairs in order and concrete incompleteTopics within the opening's own transition. The outline's deeper investigations are separate planned coverage, not completed facts and not by themselves an incomplete opening. An unresolved or unsupported part of the opening remains incomplete even if you plan to investigate it later. Do not invent identities, locally repair output or hide missing evidence.

outline is a repositoryGuidePlanV1 object using citationVersion:"nativeEvidenceV2", a short title, sections and omissions. Plan 2–6 distinct concrete learning questions that together with the opening cover the ORIGINAL question, including requested alternate paths and failures. These are later independent bounded investigations, not one unit per file. Each unit has exactly key,question,prerequisites,claims:[{evidence:[{referenceId:"code:1"}]}]. Use 1–4 exact citeable native references per unit as starting points; no source prose, paths, quotations or established claims. Each question ends with a question mark, names a coherent narrow behavior, and should fit one focused investigation with a few useful sections. Do not make the first unit an exhaustive parser/constructor/error catalogue and block every later part behind it. Partition requested alternatives coherently across units without omitting or compressing requested behavior. Add a prerequisite ONLY when understanding this unit actually requires the earlier unit's established concept; shared files or a broad pipeline order alone do not make every earlier unit a prerequisite. A prerequisite key must name an earlier unit. Do not repeat the opening as a generic orientation unit; deeper work may revisit its source for genuinely new behavior. Name concrete remaining unsupported boundaries in omissions. No omissions never means every file is explained. Neither a published opening nor an outline marks the whole guide complete. Finish with this exact combined schema, no Markdown or extra fields."#;

pub(super) fn instructions() -> String {
    workspace_generation::progressive::instructions_for_role(ROLE, FINAL_INSTRUCTIONS)
}

pub(super) fn schema(outline: Value) -> Value {
    json!({"type":"object","additionalProperties":false,"required":["outline","openingAnswer"],
        "properties":{"outline":outline,"openingAnswer":workspace_generation::progressive::schema()}})
}

/// Strict parsing precedes both native citation resolution and manifest replay.
/// The final response cannot smuggle a new explanation into the outline.
pub(super) fn split(text: &str) -> Result<(String, String), String> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct ResultParts {
        outline: Box<serde_json::value::RawValue>,
        opening_answer: Box<serde_json::value::RawValue>,
    }
    crate::strict_json::from_slice(text.as_bytes())
        .map_err(|_| "The guide result has duplicate or invalid JSON fields.")?;
    let parts: ResultParts = serde_json::from_str(text)
        .map_err(|_| "The guide result must retain its outline and published opening manifest.")?;
    // Keep the exact native-owned manifest substring, including whitespace,
    // instead of manufacturing a normalized final response for its receipt.
    Ok((
        parts.outline.get().into(),
        parts.opening_answer.get().into(),
    ))
}

/// Finalize existing publication receipts only. The worker has already joined;
/// source replay and adoption happen outside native operation ownership.
pub(super) fn retain_failure(storage: &Path, job: &Record) -> Result<(), String> {
    let Some(opening) = &job.opening else {
        return Ok(());
    };
    let answer = if reading::exists(storage, &opening.run_id)? {
        Some(
            reading::scoped(storage, &opening.run_id, &job.project_id, &job.snapshot_id)?.summary(),
        )
    } else {
        None
    };
    store::update(storage, &job.id, |current| {
        if current
            .active_run_id
            .as_ref()
            .is_some_and(|run| run != &opening.run_id)
        {
            return Err(
                "The guide has a newer investigation owner. Saved work was preserved.".into(),
            );
        }
        let saved = current
            .opening
            .as_mut()
            .ok_or("The guide opening owner changed.")?;
        if saved.run_id != opening.run_id || saved.question != opening.question {
            return Err("The guide opening scope changed. Saved work was preserved.".into());
        }
        saved.summary = answer;
        if current.status != Status::Stopped {
            current.status = Status::Partial;
        }
        current.active_run_id = None;
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
mod tests;
