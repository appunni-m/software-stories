//! A research outline, not an accepted explanation. Only native-delivered
//! ranges may become starting points; every unit must investigate its own facts.

use super::*;
use crate::{agent_context, context_mcp, native_citations};
use serde_json::{Value, json};
use std::time::Instant;


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
    let question = opening::question(job)?;
    let prompt = serde_json::to_string(&json!({"question":question,"snapshotId":job.snapshot_id,
        "scope":"all eligible captured implementation, researched by ranges; never select whole files for transmission",
        "guideContext":{"guideId":job.id,"question":job.question}}))
        .map_err(|_| "The guide question could not be encoded.")?;
    let schema = opening::schema_for(job, schema());
    let instructions = format!("{}\n{RESEARCH_INSTRUCTIONS}", opening::instructions_for(job));
    let contract = json!({"contractVersion":job.contract,
        "runId":job.planning_run_id,"guideId":job.id,"grantId":job.grant_id,"manifestSha256":job.manifest_sha256,
        "instructions":instructions,"prompt":prompt,"schema":schema,"tools":context_mcp::tools(),
        "correctionPolicy":"owned-section-proposals-no-final-rewrite-v1","openingPolicy":opening::policy(job),
        "model":job.model_id,"effort":job.reasoning_effort,"maxUnits":MAX_UNITS,"maxAttemptsPerUnit":MAX_ATTEMPTS,
        "unitResearchPolicy":RESEARCH_POLICY,"openingIntentSha256":job.opening_context_sha256,
        "sourcePolicy":crate::evidence_ledger::POLICY,"baseInstructions":agent_context::PROGRESSIVE_BASE_INSTRUCTIONS,
        "sceneAuthoringPolicy":crate::evidence_ledger::sections::authoring::POLICY,
        "checkpointVersion":crate::evidence_ledger::sections::NARRATION_CHECKPOINT_VERSION,
        "answerSchemaVersion":crate::evidence_ledger::sections::NARRATION_ANSWER_VERSION,
        "identifierCompilationPolicy":native_citations::names::GROUNDED_POLICY,
        "finalManifestVersion":crate::evidence_ledger::sections::NATIVE_MANIFEST_VERSION,
        "finalManifestPolicy":crate::evidence_ledger::sections::NATIVE_MANIFEST_POLICY});
    let contract = serde_json::to_vec(&contract).map_err(|_| "The guide contract could not be encoded.")?;
    if contract.len() > 128 * 1024 {
        return Err("The guide intent exceeds its initial contract limit. No investigation was started.".into());
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
    agent_context::Session::new_progressive(
        storage, &job.planning_run_id, manifest, runtime, &job.grant_id,
        jobs, opening::question(job)?, fingerprint, &job.model_id, &job.reasoning_effort, notify,
    )
}

pub(super) fn run(
    storage: &Path,
    job: &Record,
    manager: &codex_agent::CodexAgentManager,
    jobs: RepositoryContextJobs,
    progress: impl Fn(codex_agent::CodexProgress),
    notify: Arc<dyn Fn(SectionNotice) + Send + Sync>,
    publish: impl FnOnce(Plan, Option<Summary>) -> Result<(), String>,
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
        storage, job, manifest.clone(), &runtime, jobs, &fingerprint, notify,
    )?));
    let outcome = manager.run_with_result_and_publish(
        storage,
        codex_agent::CodexRunInput {
            run_id: job.planning_run_id.clone(),
            expected_runtime: runtime.clone(),
            model: job.model_id.clone(),
            effort: job.reasoning_effort.clone(),
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
        |response, access| -> Result<(Plan, String), String> {
            access.check_review(&job.planning_run_id, &fingerprint, &runtime.executable_path)?;
            let owned = session
                .lock()
                .map_err(|_| "The guide research session is unavailable.")?;
            owned.check_complete()?;
            let (outline, answer) = opening::split(&response.response_text)?;
            let resolved = match owned.ledger.resolve_citations(
                &outline,
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
            let plan = canonicalize(&resolved)?;
            Ok((plan, answer))
        },
        |result| {
            let (plan, manifest) = result?;
            session.lock().map_err(|_| "The guide research session is unavailable.")?
                .finish_answer_manifest(&manifest)?;
            let answer = Some(crate::evidence_ledger::sections::inspect(storage, &job.planning_run_id)?.summary());
            publish(plan, answer)
        },
    );
    {
        if let Err(error) | Ok(Err(error)) = &outcome {
            let cancelled = session
                .lock()
                .map_err(|_| "The guide research session is unavailable.")?
                .answer_cancelled()
                || store::read(storage, &job.id)?.status == Status::Stopped;
            reading::interrupt_with_failure(
                storage,
                &job.planning_run_id,
                if cancelled {
                    Outcome::Stopped
                } else {
                    Outcome::Partial
                },
                Some(if cancelled {
                    crate::evidence_ledger::sections::FailureCode::Stopped
                } else {
                    crate::evidence_ledger::sections::FailureCode::from_error(error)
                }),
            )?;
        }
    }
    outcome?
}

#[cfg(test)]
mod tests;
