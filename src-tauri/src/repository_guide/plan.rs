//! A research outline, not an accepted explanation. Only native-delivered
//! ranges may become starting points; every unit must investigate its own facts.

use super::*;
use crate::{agent_context, context_mcp, native_citations};
use serde_json::{Value, json};
use std::time::Instant;

const PLAN_OPERATION_VERSION: &str = "repository-guide-plan-initial-outline-v20-prerequisites";

const PLAN_INSTRUCTIONS: &str = r#"
Create the initial research outline only. Do not write narration, explanations, HTML, or slide content.

Hard bounds: the title is 8–120 UTF-8 bytes. The sections array contains 1–6 items total: the first is the required opening and at most five are follow-on topics. Never return more than six sections. Add follow-ons only for distinct behavior; do not add topics to meet a minimum. Put remaining work in omissions. Omissions contain at most 64 unique, concise route labels, each 1–240 UTF-8 bytes. Do not repeat labels after case/spacing normalization. Keep keys unique and lowercase with hyphens; each follow-on question is 16–800 characters, is distinct from every other question, and ends in a question mark.

The opening key is `opening`, has no prerequisites, and uses the exact question supplied by the app: the top-level question for a new guide, or `guideContext.openingQuestion` when resuming the same guide. Give it exactly one claim with one to four delivered code references. Each follow-on also has exactly one claim with one to four delivered code references. Follow-on prerequisites are optional and may name only keys of earlier follow-on items; never reference `opening`, the current item, or a later item. List only distinct prerequisite keys that are real reading dependencies. A narrow question may need only this opening. For a broad repository question, include up to five follow-ons only when they explain distinct stages; if useful, normally choose two to five. Keep every other requested or discovered route explicitly pending in omissions, never imply it is covered.

Research only enough captured implementation to ground the opening and useful initial follow-ons. Start with two to five focused terms for the central behavior, then read a few relevant ranges in batches. Do not inventory or deeply investigate every caller, format, or language before returning the outline; later slide research is separately bounded. Arrange broad-guide topics in source-grounded reading order: continue the opening's caller-to-engine path at its next real handoff, finish that central path, then cover alternate callers or languages. Give materially different behavior its own follow-on or pending label; group alternatives only when code shows they behave the same. Code proves behavior. Treat docs as discovery hints and call them stale only when cited implementation contradicts them.

Return only the supplied schema, with no extra fields. Keep the outline compact and valid before trying to maximize coverage."#;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct Plan {
    pub title: String,
    pub opening_ranges: Vec<context::CodeRange>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation_topic: Option<String>,
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
    let follow_on_item = json!({
        "type":"object","additionalProperties":false,
        "required":["key","question","prerequisites","claims"],"properties":{
            "key":{"type":"string","maxLength":80,"pattern":"^[a-z0-9]+(?:-[a-z0-9]+)*$","description":"Unique key for this follow-on. Do not use the reserved opening key."},
            "question":{"type":"string","minLength":16,"maxLength":800,"description":"Distinct question ending in a question mark."},
            "prerequisites":{"type":"array","maxItems":5,"description":"Optional earlier follow-on keys only. The opening has no section key; never reference it, the current item, or a later item. Do not repeat keys.","items":{"type":"string","maxLength":80}},
            "claims":claims
        }
    });
    let opening_item = json!({
        "type":"object","additionalProperties":false,
        "required":["key","question","prerequisites","claims"],"properties":{
            "key":{"type":"string","enum":["opening"]},
            "question":{"type":"string","minLength":1,"maxLength":12000},
            "prerequisites":{"type":"array","maxItems":0},
            "claims":claims
        }
    });
    json!({"type":"object","additionalProperties":false,
      "required":["citationVersion","planVersion","title","sections","omissions"],"properties":{
        "citationVersion":{"type":"string","enum":["nativeEvidenceV2"]},
        "planVersion":{"type":"string","enum":["repositoryGuidePlanV1"]},
        "title":{"type":"string","minLength":8,"maxLength":120},
      "sections":{"type":"array","minItems":1,"maxItems":MAX_INITIAL_UNITS + 1,"description":"Items include one question-first opening at index zero and at most five distinct follow-on topics (six sections total). Additional discovered routes remain explicit pending topics and are appended incrementally under the total-unit guide capacity.","prefixItems":[opening_item],"items":follow_on_item},
        "omissions":{"type":"array","maxItems":MAX_OMISSIONS,"description":"Unique, concise pending route labels not covered by the initial outline; do not repeat the same route.","items":{"type":"string","maxLength":MAX_OMISSION_BYTES}}}})
}

fn starting_ranges(
    resolved: &native_citations::Resolved,
    section_index: usize,
    claim: &StartingClaim,
) -> Result<Vec<context::CodeRange>, String> {
    if !(1..=4).contains(&claim.evidence.len()) {
        return Err(
            "Each guide outline item must cite one to four delivered code references.".into(),
        );
    }
    let spans = resolved
        .claim_spans(section_index, 0)
        .ok_or("Guide starting evidence is missing.")?;
    if spans.len() != claim.evidence.len() {
        return Err("Guide evidence changed.".into());
    }
    let mut ranges = Vec::new();
    for (span, evidence) in spans.iter().zip(&claim.evidence) {
        if span.range().path != evidence.path || evidence.quote.is_empty() {
            return Err("Guide evidence changed.".into());
        }
        if ranges.contains(span.range()) {
            return Err("Guide outline evidence references must be distinct.".into());
        }
        ranges.push(span.range().clone());
    }
    Ok(ranges)
}

fn canonicalize(
    resolved: &native_citations::Resolved,
    expected_opening_question: &str,
) -> Result<Plan, String> {
    let proposal: Proposal = serde_json::from_str(resolved.text())
        .map_err(|_| "The guide outline has an invalid structure.")?;
    if proposal.plan_version != "repositoryGuidePlanV1" {
        return Err("The guide outline used an unsupported plan version.".into());
    }
    if !text(&proposal.title, 8, 120) {
        return Err("The guide title is too short or too long.".into());
    }
    if !(1..=MAX_INITIAL_UNITS + 1).contains(&proposal.sections.len()) {
        return Err(
            "The outline has too many slides; use one opening and no more than five follow-ons."
                .into(),
        );
    }
    if !valid_omissions(&proposal.omissions) {
        return Err("The outline has too many or repeated pending topics.".into());
    }

    let mut proposed_sections = proposal.sections.into_iter();
    let opening = proposed_sections
        .next()
        .ok_or("The guide outline has no opening range lead.")?;
    if opening.key != "opening"
        || opening.question != expected_opening_question
        || !opening.prerequisites.is_empty()
        || opening.claims.len() != 1
    {
        return Err(
            "The first guide outline item must be the app-scoped opening range lead.".into(),
        );
    }
    let opening_ranges = starting_ranges(resolved, 0, &opening.claims[0])?;
    let follow_on_sections: Vec<_> = proposed_sections.collect();
    if follow_on_sections.len() > MAX_INITIAL_UNITS {
        return Err("The initial guide outline exceeds five follow-on sections.".into());
    }
    let mut units = Vec::new();
    let mut keys = std::collections::HashSet::new();
    let mut questions = std::collections::HashSet::from([opening.question.to_lowercase()]);
    for (index, unit) in follow_on_sections.into_iter().enumerate() {
        if unit.key == "opening" {
            return Err(
                "Follow-on guide section keys must be unique and may not use the reserved opening key.".into(),
            );
        }
        if !key(&unit.key) {
            return Err(
                "Guide section keys must use only lowercase letters, digits, and hyphens.".into(),
            );
        }
        if keys.contains(&unit.key) {
            return Err("Guide section keys must be unique.".into());
        }
        if !text(&unit.question, 16, 800) {
            return Err(
                "Each guide section needs a trimmed question between 16 and 800 characters.".into(),
            );
        }
        if !unit.question.ends_with('?') {
            return Err("Each guide section question must end with a question mark.".into());
        }
        if !questions.insert(unit.question.to_lowercase()) {
            return Err("Guide section questions must be distinct.".into());
        }
        if unit.prerequisites.len() > index
            || unit.prerequisites.iter().any(|key| !keys.contains(key))
        {
            return Err(
            "The outline refers to a slide before it appears; list only earlier follow-on topics as prerequisites.".into(),
            );
        }
        if unit
            .prerequisites
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len()
            != unit.prerequisites.len()
        {
            return Err("Guide section prerequisites must be unique.".into());
        }
        if unit.claims.len() != 1 {
            return Err("Each guide section must contain exactly one evidence claim.".into());
        }
        let ranges = starting_ranges(resolved, index + 1, &unit.claims[0])?;
        keys.insert(unit.key.clone());
        units.push(UnitPlan {
            key: unit.key,
            question: unit.question,
            prerequisites: unit.prerequisites,
            starting_ranges: ranges,
            continuation_topic: None,
        });
    }
    Ok(Plan {
        title: proposal.title,
        opening_ranges,
        units,
        omissions: proposal.omissions,
    })
}

pub(super) fn request(job: &Record) -> Result<(String, Value, String, String), String> {
    let opening_question = opening::question(job)?;
    let mut guide_context = json!({"guideId":job.id});
    if opening_question != job.question {
        guide_context["openingQuestion"] = json!(opening_question);
    }
    let prompt = serde_json::to_string(&json!({"question":job.question,"snapshotId":job.snapshot_id,
        "scope":"all eligible captured implementation, researched by ranges; never select whole files for transmission",
        "guideContext":guide_context}))
        .map_err(|_| "The guide question could not be encoded.")?;
    let schema = schema();
    let retry_feedback = if job.planning_retry_count > 0 {
        "This is an automatic retry after the previous outline was rejected by local validation. Keep this attempt minimal and check every bound before returning: title 8–120 UTF-8 bytes; one opening plus at most five follow-ons; distinct follow-on keys and questions; each item has one claim and one to four delivered references; prerequisites may name only earlier follow-on keys, never the opening; and no more than 64 unique omission labels of at most 240 UTF-8 bytes. Prefer fewer follow-ons over risking another invalid outline."
    } else {
        ""
    };
    let instructions = format!(
        "{PLAN_INSTRUCTIONS}\n{retry_feedback}\n{}\n{RESEARCH_INSTRUCTIONS}\nThis repository guide has no wall-clock deadline. Return the compact initial outline as soon as its opening lead and up to five high-value follow-on topics are grounded. Do not finish planning every requested route before the first slide can start. Preserve routes already observed or explicitly requested as pending omissions; after the outline and opening lead are saved, the owned guide worker may promote those omissions to later slide units up to the current total-unit guide capacity. Publish each validated slide independently. Any internal look-ahead must be small and must never gate the outline, first slide, or playback. Native source and call limits still apply.",
        crate::provider::visual_scenes::READER_LANGUAGE_POLICY
    );
    let contract = json!({"contractVersion":job.contract,
        "runId":job.planning_run_id,"guideId":job.id,"grantId":job.grant_id,"manifestSha256":job.manifest_sha256,
        "instructions":instructions,"prompt":prompt,"schema":schema,"tools":context_mcp::research_tools(),
        "operationPolicy":PLAN_OPERATION_VERSION,
        "readerLanguagePolicy":crate::provider::visual_scenes::READER_LANGUAGE_POLICY_VERSION,
        "model":job.model_id,"effort":job.reasoning_effort,
        "maxInitialUnits":MAX_INITIAL_UNITS,"maxTotalUnits":MAX_TOTAL_UNITS,"maxResearchCalls":crate::evidence_ledger::REPOSITORY_PLAN_CALL_LIMIT,
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
            current.planning_failure_code = Some(PlanningFailureCode::OutlineNotPublished);
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
    started_at: Instant,
    progress: impl Fn(codex_agent::CodexProgress),
    notify: Arc<dyn Fn(SectionNotice) + Send + Sync>,
    publish: impl FnOnce(Plan) -> Result<(), String>,
) -> Result<(), String> {
    let mut preparation = crate::generation_timing::Preparation::new(started_at);
    let manifest = preparation.run_stage(
        crate::generation_timing::PreparationKind::SnapshotLoad,
        || checked_manifest(storage, job),
    )?;
    let runtime = preparation.run_stage(
        crate::generation_timing::PreparationKind::RuntimeReview,
        || manager.review_runtime(storage),
    )?;
    preparation.run_stage(
        crate::generation_timing::PreparationKind::RetrievalGrant,
        || {
            workspace::check_retrieval_grant(
                storage,
                &job.grant_id,
                &job.project_id,
                &job.snapshot_id,
                &runtime,
            )
        },
    )?;
    let _worker = preparation.run_stage(
        crate::generation_timing::PreparationKind::WorkerRegistration,
        || manager.hold_answer_worker(&job.planning_run_id),
    )?;
    let (prompt, schema, fingerprint, instructions) = preparation.run_stage(
        crate::generation_timing::PreparationKind::RequestPreparation,
        || request(job),
    )?;
    let session = Arc::new(Mutex::new(preparation.run_stage(
        crate::generation_timing::PreparationKind::RetrievalSession,
        || {
            new_session(
                storage,
                job,
                manifest.clone(),
                &runtime,
                jobs,
                &fingerprint,
                notify,
            )
        },
    )?));
    let preparation_stages = preparation.into_stages();
    let outcome = manager.run_with_result_and_publish_and_preparation(
        storage,
        codex_agent::CodexRunInput {
            run_id: job.planning_run_id.clone(),
            expected_runtime: runtime.clone(),
            prompt_contract_version: format!("{}:{}", job.contract, PLAN_OPERATION_VERSION),
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
        started_at,
        None,
        preparation_stages,
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
            canonicalize(&resolved, opening::question(job)?)
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
