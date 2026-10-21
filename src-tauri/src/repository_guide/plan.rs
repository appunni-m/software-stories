//! A research outline, not an accepted explanation. Only native-delivered
//! ranges may become starting points; every unit must investigate its own facts.

use super::*;
use crate::{agent_context, context_mcp, native_citations};
use serde_json::{Value, json};
use std::time::Instant;

const PLAN_INSTRUCTIONS: &str = "Create the guide's cited slide outline only. The opening slide will be generated independently after this outline is saved. Return two to six focused follow-on behavior questions that together with the opening cover the exact original question. The first follow-on starts immediately after the opening entry-point transition. Keep units small enough for one independently bounded investigation each. Use delivered source references only as starting-range hints; do not write explanations, narration or slide content, and do not call propose_answer_section. Name unsupported boundaries as omissions. Return only the supplied plan schema.";

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Plan {
    pub title: String,
    pub units: Vec<UnitPlan>,
    pub omissions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct UnitPlan {
    pub key: String,
    pub question: String,
    pub prerequisites: Vec<String>,
    pub starting_ranges: Vec<context::CodeRange>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Proposal {
    plan_version: String,
    title: String,
    sections: Vec<ProposedUnit>,
    omissions: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposedUnit {
    key: String,
    question: String,
    prerequisites: Vec<String>,
    claims: Vec<StartingClaim>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartingClaim {
    evidence: Vec<StartingEvidence>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StartingEvidence {
    path: String,
    quote: String,
}

pub(super) fn schema() -> Value {
    let evidence = json!({"type":"array","minItems":1,"maxItems":4,"items":{
        "type":"object","additionalProperties":false,"required":["referenceId"],"properties":{
            "referenceId":{"type":"string","maxLength":8,"pattern":"^code:[1-9][0-9]{0,2}$"}}}});
    let claims = json!({"type":"array","minItems":1,"maxItems":1,"items":{
        "type":"object","additionalProperties":false,"required":["evidence"],"properties":{"evidence":evidence}}});
    json!({"type":"object","additionalProperties":false,
      "required":["citationVersion","planVersion","title","sections","omissions"],"properties":{
        "citationVersion":{"type":"string","enum":["nativeEvidenceV2"]},
        "planVersion":{"type":"string","enum":["repositoryGuidePlanV1"]},
        "title":{"type":"string","minLength":8,"maxLength":120},
        "sections":{"type":"array","minItems":2,"maxItems":MAX_UNITS,"items":{
          "type":"object","additionalProperties":false,"required":["key","question","prerequisites","claims"],"properties":{
            "key":{"type":"string","maxLength":80},"question":{"type":"string","minLength":16,"maxLength":800},
            "prerequisites":{"type":"array","maxItems":5,"items":{"type":"string","maxLength":80}},
            "claims":claims}}},
        "omissions":{"type":"array","maxItems":8,"items":{"type":"string","maxLength":240}}}})
}

fn canonicalize(resolved: &native_citations::Resolved) -> Result<Plan, String> {
    let proposal: Proposal = serde_json::from_str(resolved.text())
        .map_err(|_| "The guide outline has an invalid structure.")?;
    if proposal.plan_version != "repositoryGuidePlanV1"
        || !text(&proposal.title, 8, 120)
        || !(2..=MAX_UNITS).contains(&proposal.sections.len())
        || proposal.omissions.len() > 8
        || proposal.omissions.iter().any(|s| !text(s, 1, 240))
    {
        return Err("The guide outline exceeds its fixed limits.".into());
    }
    let mut units = Vec::new();
    let mut keys = std::collections::HashSet::new();
    let mut questions = std::collections::HashSet::new();
    for (index, unit) in proposal.sections.into_iter().enumerate() {
        if !key(&unit.key)
            || keys.contains(&unit.key)
            || !text(&unit.question, 16, 800)
            || !unit.question.ends_with('?')
            || !questions.insert(unit.question.to_lowercase())
            || unit.prerequisites.len() > index
            || unit.prerequisites.iter().any(|k| !keys.contains(k))
            || unit
                .prerequisites
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != unit.prerequisites.len()
            || unit.claims.len() != 1
            || !(1..=4).contains(&unit.claims[0].evidence.len())
        {
            return Err(
                "Guide questions must be distinct, ordered and supported by delivered code.".into(),
            );
        }
        let spans = resolved
            .claim_spans(index, 0)
            .ok_or("Guide starting evidence is missing.")?;
        if spans.len() != unit.claims[0].evidence.len() {
            return Err("Guide evidence changed.".into());
        }
        let mut ranges = Vec::new();
        for (span, evidence) in spans.iter().zip(&unit.claims[0].evidence) {
            // Resolved JSON is native-only; still compare it with its trusted span.
            if span.range().path != evidence.path || evidence.quote.is_empty() {
                return Err("Guide evidence changed.".into());
            }
            if !ranges.contains(span.range()) {
                ranges.push(span.range().clone());
            }
        }
        keys.insert(unit.key.clone());
        units.push(UnitPlan {
            key: unit.key,
            question: unit.question,
            prerequisites: unit.prerequisites,
            starting_ranges: ranges,
        });
    }
    Ok(Plan {
        title: proposal.title,
        units,
        omissions: proposal.omissions,
    })
}

pub(super) fn request(job: &Record) -> Result<(String, Value, String, String), String> {
    opening::question(job)?;
    let prompt = serde_json::to_string(&json!({"question":job.question,"snapshotId":job.snapshot_id,
        "scope":"all eligible captured implementation, researched by ranges; never select whole files for transmission",
        "guideContext":{"guideId":job.id,"question":job.question,"openingQuestion":opening::LEARNING_QUESTION}}))
        .map_err(|_| "The guide question could not be encoded.")?;
    let schema = schema();
    let instructions = format!(
        "{PLAN_INSTRUCTIONS}\n{RESEARCH_INSTRUCTIONS}\nThis repository guide has no wall-clock deadline. Continue researching the outline until it is complete or the user stops the guide. Native source and call limits still apply."
    );
    let contract = json!({"contractVersion":job.contract,
        "runId":job.planning_run_id,"guideId":job.id,"grantId":job.grant_id,"manifestSha256":job.manifest_sha256,
        "instructions":instructions,"prompt":prompt,"schema":schema,"tools":context_mcp::research_tools(),
        "operationPolicy":"repository-guide-plan-only-v1",
        "model":job.model_id,"effort":job.reasoning_effort,"maxUnits":MAX_UNITS,
        "unitResearchPolicy":RESEARCH_POLICY,"openingIntentSha256":job.opening_context_sha256,
        "runDeadlinePolicy":"user-controlled-stop-v1",
        "sourcePolicy":crate::evidence_ledger::POLICY,"baseInstructions":agent_context::RESEARCH_BASE_INSTRUCTIONS});
    let contract =
        serde_json::to_vec(&contract).map_err(|_| "The guide contract could not be encoded.")?;
    if contract.len() > 128 * 1024 {
        return Err(
            "The guide intent exceeds its initial contract limit. No investigation was started."
                .into(),
        );
    }
    Ok((prompt, schema, context::digest(&contract), instructions))
}

fn new_session(
    storage: &Path,
    job: &Record,
    manifest: snapshot::SnapshotManifest,
    runtime: &crate::codex_identity::CodexReviewRuntime,
    jobs: RepositoryContextJobs,
    fingerprint: &str,
    notify: Arc<dyn Fn(SectionNotice) + Send + Sync>,
) -> Result<agent_context::Session, String> {
    agent_context::Session::new_research(
        storage,
        &job.planning_run_id,
        manifest,
        runtime,
        &job.grant_id,
        jobs,
        fingerprint,
        &job.model_id,
        &job.reasoning_effort,
        notify,
    )
}

pub(super) fn retain_failure(storage: &Path, job: &Record) -> Result<(), String> {
    store::update(storage, &job.id, |current| {
        if current.revision != job.revision
            || current.plan.is_some()
            || current.active_run_id.as_deref() != Some(job.planning_run_id.as_str())
        {
            return Err("The guide outline owner changed. Existing progress was preserved.".into());
        }
        if current.status != Status::Stopped {
            current.status = Status::Partial;
        }
        current.active_run_id = None;
        Ok(())
    })?;
    Ok(())
}

pub(super) fn run(
    storage: &Path,
    job: &Record,
    manager: &codex_agent::CodexAgentManager,
    jobs: RepositoryContextJobs,
    progress: impl Fn(codex_agent::CodexProgress),
    notify: Arc<dyn Fn(SectionNotice) + Send + Sync>,
    publish: impl FnOnce(Plan) -> Result<(), String>,
) -> Result<(), String> {
    let started = Instant::now();
    let manifest = checked_manifest(storage, job)?;
    let runtime = manager.review_runtime(storage)?;
    workspace::check_retrieval_grant(
        storage,
        &job.grant_id,
        &job.project_id,
        &job.snapshot_id,
        &runtime,
    )?;
    let _worker = manager.hold_answer_worker(&job.planning_run_id)?;
    let (prompt, schema, fingerprint, instructions) = request(job)?;
    let session = Arc::new(Mutex::new(new_session(
        storage,
        job,
        manifest.clone(),
        &runtime,
        jobs,
        &fingerprint,
        notify,
    )?));
    let outcome = manager.run_with_result_and_publish(
        storage,
        codex_agent::CodexRunInput {
            run_id: job.planning_run_id.clone(),
            expected_runtime: runtime.clone(),
            model: job.model_id.clone(),
            effort: job.reasoning_effort.clone(),
            run_deadline: crate::codex_agent::RunDeadline::UserControlled,
            approval_policy: "never".into(),
            system_instruction: instructions,
            user_prompt: prompt,
            output_schema: schema,
            correction_policy: crate::claim_alignment::CorrectionPolicy::Disabled,
            retrieval: Some(Arc::clone(&session)),
            sources: vec![],
        },
        &fingerprint,
        started,
        progress,
        |response, access| -> Result<Plan, String> {
            access.check_review(&job.planning_run_id, &fingerprint, &runtime.executable_path)?;
            let owned = session
                .lock()
                .map_err(|_| "The guide research session is unavailable.")?;
            owned.check_complete()?;
            let resolved = match owned.ledger.resolve_citations(
                &response.response_text,
                &manifest,
                access.runtime(),
                access.cancelled(),
            )? {
                native_citations::Resolution::Ready(resolved) => resolved,
                native_citations::Resolution::Invalid(_) => {
                    return Err(
                        "The guide outline cited unavailable code. No outline was published."
                            .into(),
                    );
                }
            };
            canonicalize(&resolved)
        },
        |result| {
            session
                .lock()
                .map_err(|_| "The guide research session is unavailable.")?
                .finish_research()?;
            publish(result?)
        },
    );
    outcome?
}

#[cfg(test)]
mod tests;
