//! Publish a grounded opening inside the original outline operation, before
//! its independent deeper units exist. This never grants a second operation.

use super::*;
use serde_json::{Value, json};

const LEGACY_POLICY: &str = "first-transition-with-native-publications-v2";
const SCOPED_POLICY: &str = "first-transition-with-separate-learning-intent-v3";
pub(super) const LEARNING_QUESTION: &str = "How does the repository's real entry point accept its input, make the first consequential decision, and return or propagate that transition's result?";
const SCOPED_ROLE: &str = "This owned investigation has two distinct questions. The top-level question is the opening answer's learning question: teach the actual first input-to-decision/result transition and its guards, using captured implementation. guideContext.question is the exact original whole-guide question: preserve its entire scope in the deeper outline, including alternatives and failures. Publish a small illustrated opening early, then finish researching that outline within this same operation and original budgets. Source selection is yours; neither question establishes implementation facts.";
const SCOPED_COMPLETION: &str = "Assess openingAnswer.incompleteTopics against the top-level opening question, while outline coverage is assessed against the complete guideContext.question. Returning a selected driver's Result can fully explain caller dispatch without claiming the driver's work succeeds; unclaimed callee internals belong to later research when required by the whole-guide question. Missing conditions, fall-through or returned/propagated outcomes within the opening's own chosen transition still remain incomplete. Do not remove facts, hide an unresolved opening guard or shrink the whole-guide outline to obtain completion. Completing the opening never completes the whole guide.";
const ROLE: &str = r#"Investigate the original learning question using captured implementation code. Read the actual entry point and first input-to-decision/result transition early. Publish a small complete opening section as soon as its implementation evidence has been read, then focus on the remaining research outline. Preserve the question's causal order and full scope in that outline, including requested alternate paths and failures. The opening teaches its real transition and all relevant guards and consequences; it is not a plan or file list. For a predicate explain both true and false conditions. Do not continue publishing every deeper stage as part of the opening. Those stages belong to the distinct focused investigations you are planning within this same operation and original budgets."#;

const FINAL_INSTRUCTIONS: &str = r#"This operation opens a complete repository guide and researches its deeper outline. The original learning question remains the guide's full scope. Publish a useful illustrated opening through propose_answer_section as soon as you have read the first real input-to-decision/result transition. Do not wait for a complete repository survey or outline. The opening must teach implementation behavior; a file list, generic introduction or promise to investigate is not an answer. Its narrower role is the first concrete transition, not every alternate representation and failure family. Explain its supported guards and consequences completely. Keep the opening small, normally one section, while preserving every fact you choose to teach. Continue researching the outline in the SAME owned investigation and original budgets; never start a second operation to get an opening.

Return exactly {outline,openingAnswer}. openingAnswer is {manifestVersion:"answerManifestV2",incompleteTopics:[...]}. The app retains every latest published section automatically, in its native publication order. Do not copy sectionKey or generationId into this manifest or select a subset of sections. Name concrete incompleteTopics within the opening's own transition. The outline's deeper investigations are separate planned coverage, not completed facts and not by themselves an incomplete opening. An unresolved or unsupported part of the opening remains incomplete even if you plan to investigate it later. Do not locally repair output or hide missing evidence.

outline is a repositoryGuidePlanV1 object using citationVersion:"nativeEvidenceV2", a short title, sections and omissions. Plan 2–6 distinct concrete learning questions that together with the opening cover the ORIGINAL question, including requested alternate paths and failures. These are later independent bounded investigations, not one unit per file. Each unit has exactly key,question,prerequisites,claims:[{evidence:[{referenceId:"code:1"}]}]. Use 1–4 exact citeable native references per unit as starting points; no source prose, paths, quotations or established claims. Each question ends with a question mark, names a coherent narrow behavior, and should fit one focused investigation with a few useful sections. Do not make the first unit an exhaustive parser/constructor/error catalogue and block every later part behind it. Partition requested alternatives coherently across units without omitting or compressing requested behavior. Add a prerequisite ONLY when understanding this unit actually requires the earlier unit's established concept; shared files or a broad pipeline order alone do not make every earlier unit a prerequisite. A prerequisite key must name an earlier unit. Do not repeat the opening as a generic orientation unit; deeper work may revisit its source for genuinely new behavior. Name concrete remaining unsupported boundaries in omissions. No omissions never means every file is explained. Neither a published opening nor an outline marks the whole guide complete. Finish with this exact combined schema, no Markdown or extra fields."#;

// Keep the scoped wording independent of legacy instructions: the top-level
// learning question now names the opening, not the original whole-guide scope.
// Full request fingerprints already bind these exact instructions. No stored
// question, checkpoint, completion declaration or historical policy is changed.
const SCOPED_FINAL_INSTRUCTIONS: &str = r#"The top-level question defines only the opening's first input-to-decision/result transition. guideContext.question defines the entire guide, including every requested alternative and failure. Read the real input boundary and first consequential branch, then publish a small complete illustrated section through propose_answer_section early. Explain its predicate, both alternatives, and the actual return or error propagation at that boundary. Returning a callee's Result does not establish the callee's successful construction. Unclaimed callee internals and later selectors belong in the deeper outline; their absence alone is not an incomplete opening. If a published claim depends on an unverified helper, guard or outcome, read and verify that implementation or retain the concrete unresolved topic. Do not infer behavior such as byte order or validation from a helper's name. Never erase a supported fact, hide an actual evidence gap, or shrink guideContext.question to declare completion.

Continue researching the deeper outline in this SAME owned operation and original budgets. Return exactly {outline,openingAnswer}, without Markdown or extra fields. openingAnswer is {manifestVersion:"answerManifestV2",incompleteTopics:[...]}. The app retains every latest published section automatically in native publication order; do not select a subset or repeat sectionKey/generationId in the manifest. incompleteTopics names unresolved behavior needed for the opening's actual claims and first transition. It is not a list of everything the guide's deeper investigations will teach. A file list, plan or promise to investigate is not an opening answer.

outline is repositoryGuidePlanV1 with citationVersion:"nativeEvidenceV2", a short title, sections and omissions. Plan 2–6 focused learning questions that, together with the opening, cover the COMPLETE guideContext.question. Preserve its requested alternatives, failure paths and untraced callee internals in these questions; do not omit or compress requested behavior. Each unit has exactly key,question,prerequisites,claims:[{evidence:[{referenceId:"code:1"}]}], with 1–4 exact citeable native references as starting points, never source prose, paths, quotations or established facts. Each question ends with a question mark and fits one focused investigation with a few useful sections. Do not repeat the opening as generic orientation or make unit one an exhaustive catalogue. Add an earlier prerequisite only when understanding actually requires that concept; shared files or pipeline order alone do not create a prerequisite. Name unsupported boundaries in omissions. An outline is planned coverage, not proof that every file or behavior has been explained."#;

pub(super) fn instructions() -> String {
    workspace_generation::progressive::instructions_for_role(ROLE, FINAL_INSTRUCTIONS)
}

pub(super) fn policy(job: &Record) -> &'static str {
    if separate_opening_intent(job) { SCOPED_POLICY } else { LEGACY_POLICY }
}

pub(super) fn context_sha256(job: &Record) -> Result<String, String> {
    let bytes = serde_json::to_vec(&json!({
        "policy":SCOPED_POLICY,"guideId":job.id,"runId":job.planning_run_id,
        "projectId":job.project_id,"snapshotId":job.snapshot_id,
        "guideQuestion":job.question,"openingQuestion":LEARNING_QUESTION,
    })).map_err(|_| "The opening learning intent could not be encoded.")?;
    Ok(context::digest(&bytes))
}

pub(super) fn question(job: &Record) -> Result<&str, String> {
    let opening = job.opening.as_ref().ok_or("The guide has no opening owner.")?;
    let expected = if separate_opening_intent(job) { LEARNING_QUESTION } else { &job.question };
    if opening.run_id != job.planning_run_id || opening.question != expected
        || (separate_opening_intent(job)
            && job.opening_context_sha256.as_deref() != Some(context_sha256(job)?.as_str()))
        || (!separate_opening_intent(job) && job.opening_context_sha256.is_some())
    {
        return Err("The guide opening changed its owner or learning intent.".into());
    }
    Ok(&opening.question)
}

pub(super) fn instructions_for(job: &Record) -> String {
    if separate_opening_intent(job) {
        workspace_generation::progressive::instructions_for_role(
            SCOPED_ROLE, &format!("{SCOPED_FINAL_INSTRUCTIONS}\n\n{SCOPED_COMPLETION}"),
        )
    } else { instructions() }
}

pub(super) fn schema_for(job: &Record, outline: Value) -> Value {
    let mut result = schema(outline);
    if separate_opening_intent(job) {
        result["properties"]["openingAnswer"]["description"] = json!("Unfinished behavior within the top-level opening learning question; this is not a completion declaration for guideContext.question.");
        result["properties"]["outline"]["description"] = json!("Research questions covering the complete original guideContext.question together with the opening; never narrow the original requested behavior.");
    }
    result
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
        if saved.run_id != opening.run_id || saved.question != opening.question
            || current.question != job.question
            || current.opening_context_sha256 != job.opening_context_sha256
        {
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
