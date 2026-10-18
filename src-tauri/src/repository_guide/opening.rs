//! Publish a grounded opening inside the original outline operation, before
//! its independent deeper units exist. This never grants a second operation.

use super::*;
use serde_json::{Value, json};

const SCOPED_POLICY: &str = "first-transition-with-separate-learning-intent-v3";
pub(super) const LEARNING_QUESTION: &str = "How does the repository's real entry point accept its input, make the first consequential decision, and return or propagate that transition's result?";
const SCOPED_ROLE: &str = "This owned investigation has two distinct questions. The top-level question teaches the opening's actual first input-to-decision/result transition and guards. guideContext.question is the exact original whole-guide question: preserve all requested alternatives and failures in the deeper outline. Publish a small illustrated opening early, then research that outline within this operation's original budgets. Select source independently; questions are not implementation evidence.";
const SCOPED_COMPLETION: &str = "Assess openingAnswer.incompleteTopics against the top-level opening question and outline coverage against all of guideContext.question. Missing conditions, fall-through or returned/propagated outcomes within the opening remain incomplete. Returning a driver's Result explains dispatch, not successful construction; unclaimed callee internals belong to required deeper research. Do not hide gaps or shrink the outline for completion. Completing the opening never completes the whole guide.";
const SCOPED_OPENING_BOUNDARY: &str = r#"Choose exactly one entry point where guideContext.question's input first enters the journey. Follow wrappers and predicate helpers through that transition's first decision, guards and immediate return or error propagation. A later public API starts a separate transition.

Normally publish one focused opening section with the grounded claims/scenes its conditions require, then finish the deeper outline. Keep later API entry points, selectors and flag normalization in focused outline questions outside this first transition. Preserve full guide coverage; do not add a later API's first guard as another opening or repeat this same behavior in the outline. Deeper units may revisit source for genuinely new behavior. Never discard a publication or hide an unresolved condition to obtain completion. Native publication, evidence and lifetime budgets are unchanged."#;


// The top-level learning question names the opening. guideContext retains the
// full guide scope; request fingerprints bind both questions and these instructions.
const SCOPED_FINAL_INSTRUCTIONS: &str = r#"Read the real input boundary and first consequential branch, then publish a small complete illustrated section through propose_answer_section early. Explain its predicate, both alternatives, and the actual return or error propagation at that boundary. Returning a callee's Result does not establish the callee's successful construction. Unclaimed callee internals and later selectors belong in the deeper outline; their absence alone is not an incomplete opening. If a published claim depends on an unverified helper, guard or outcome, read and verify that implementation or retain the concrete unresolved topic. Do not infer behavior such as byte order or validation from a helper's name. Never erase a supported fact, hide an actual evidence gap, or shrink guideContext.question to declare completion.

Continue researching the deeper outline in this SAME owned operation and original budgets. Return exactly {outline,openingAnswer}, without Markdown or extra fields. openingAnswer is {manifestVersion:"answerManifestV2",incompleteTopics:[...]}. The app retains every latest published section automatically in native publication order; do not select a subset or repeat sectionKey/generationId in the manifest. incompleteTopics names unresolved behavior needed for the opening's actual claims and first transition. It is not a list of everything the guide's deeper investigations will teach. A file list, plan or promise to investigate is not an opening answer.

outline is repositoryGuidePlanV1 with citationVersion:"nativeEvidenceV2", a short title, sections and omissions. Plan 2–6 focused learning questions that, together with the opening, cover the COMPLETE guideContext.question. Preserve its requested alternatives, failure paths and untraced callee internals in these questions; do not omit or compress requested behavior. Each unit has exactly key,question,prerequisites,claims:[{evidence:[{referenceId:"code:1"}]}], with 1–4 exact citeable native references as starting points, never source prose, paths, quotations or established facts. Each question ends with a question mark and fits one focused investigation with a few useful sections. Do not repeat the opening as generic orientation or make unit one an exhaustive catalogue. Add an earlier prerequisite only when understanding actually requires that concept; shared files or pipeline order alone do not create a prerequisite. Name unsupported boundaries in omissions. An outline is planned coverage, not proof that every file or behavior has been explained."#;

pub(super) fn instructions() -> String {
    workspace_generation::progressive::instructions_for_role(
        SCOPED_ROLE, &format!("{SCOPED_FINAL_INSTRUCTIONS}\n\n{SCOPED_OPENING_BOUNDARY}\n\n{SCOPED_COMPLETION}"),
    )
}

pub(super) fn policy(_job: &Record) -> &'static str { SCOPED_POLICY
}

pub(super) fn context_sha256(job: &Record) -> Result<String, String> {
    let bytes = serde_json::to_vec(&json!({
        "policy":SCOPED_POLICY,"guideId":job.id,"runId":job.planning_run_id,
        "projectId":job.project_id,"snapshotId":job.snapshot_id,
        "guideQuestion":job.question,"openingQuestion":LEARNING_QUESTION,
        "model":job.model_id,"effort":job.reasoning_effort,
    })).map_err(|_| "The opening learning intent could not be encoded.")?;
    Ok(context::digest(&bytes))
}

pub(super) fn question(job: &Record) -> Result<&str, String> {
    let opening = job.opening.as_ref().ok_or("The guide has no opening owner.")?;
    if opening.run_id != job.planning_run_id || opening.question != LEARNING_QUESTION
        || job.opening_context_sha256.as_deref() != Some(context_sha256(job)?.as_str()) {
        return Err("The guide opening changed its owner or learning intent.".into());
    }
    Ok(&opening.question)
}

pub(super) fn instructions_for(_job: &Record) -> String { instructions()
}

pub(super) fn schema_for(_job: &Record, outline: Value) -> Value {
    let mut result = schema(outline);
    result["properties"]["openingAnswer"]["description"] = json!("Unfinished behavior within the top-level opening learning question; this is not a completion declaration for guideContext.question.");
    result["properties"]["outline"]["description"] = json!("Research questions covering the complete original guideContext.question together with the opening; never narrow the original requested behavior.");
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
pub(super) mod tests;
