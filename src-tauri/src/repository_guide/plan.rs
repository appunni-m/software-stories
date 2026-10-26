//! A research outline, not an accepted explanation. Only native-delivered
//! ranges may become starting points; every unit must investigate its own facts.

use super::*;
use crate::{agent_context, context_mcp, native_citations};
use serde_json::{Value, json};
use std::time::Instant;

const PLAN_INSTRUCTIONS: &str = r#"
Create the guide's cited research outline only. The fresh-outline schema allows two to four focused behavior questions for the initial core path; the separate app-authored opening makes the first buffer at most five slides. Do not delay that first buffer to map every caller, language, binding, or side feature. Put every requested or observed route outside the initial path in `omissions` as a distinct, concrete topic. These topics are explicitly not yet covered; later units can be appended up to the saved guide's existing total-unit limit, and any route that cannot be represented must remain visible as uncovered. The completed guide, not this initial outline, must cover the user's requested scope. Return an opening-range lead followed by two to four focused behavior questions for the first useful path. The opening lead must use key `opening`, exactly match the app-provided `openingQuestion`, have no prerequisites, and contain one claim citing one to four delivered code references for the real outermost input boundary. The app authors this opening as a separate slide.

For repository-wide requests, do not mistake an internal helper that receives an already constructed object or completed state for the repository entry point. If source shows a central engine, reserve at least two follow-on units for its primary path: caller input into the central domain representation, then the main operation through useful results and important failures. Ground these units in engine implementation, not only wrappers.

For complete repository-wide coverage, make a source-grounded inventory of caller-controlled languages and entry surfaces visible in implementation, checked-in examples, and build manifests. The first outline does not wait for this whole-repository inventory: identify routes relevant to the initial core path, and list each other requested or observed route as a distinct omission topic. Verify every example and binding in source when its later unit is investigated. Do not count a language as covered because another language's example reaches the same engine. Name each materially different surface in its own question, including C, JavaScript/Node, WASM, CLI, server, or public API surfaces when present; name omitted surfaces as explicit pending topics, never as covered. Do not combine object creation/opening with a later operation that consumes the object when their inputs, results, or failure behavior differ; for example, creating a face from memory and loading a glyph into an existing face are separate units even when they share one ABI and engine. Group adapters only when source shows they accept the same caller intent and follow the same consequential behavior; name every grouped surface in the question.

After the first buffer, prioritize distinct public language/API surfaces and their checked-in callers before auxiliary formats or side features. Never present one representative path as whole-project coverage. Within each route, follow external input through consequential transformations to useful results and important failures. If more than twelve units are needed, prioritize the core path and materially different language/API surfaces, group only adapters proven to share behavior, and record every excluded route as not yet covered. Never hide a supported surface in omissions or imply complete coverage; each omission must name pending work explicitly. For a narrow question, scope the plan to that behavior and do not enumerate unrelated routes.

Every item needs a unique lowercase key using only letters, digits, and hyphens; one distinct, trimmed question ending in `?`; and exactly one claim citing one to four delivered code references relevant to that item. Prerequisites are optional and may name only unique keys from earlier follow-on units. Never name the opening, current unit, or a later unit as a prerequisite.

Keep each unit small enough for one answer section's visual budget: target at most eight distinct evidence nodes and two to four claims/scenes, with hard limits of 12 nodes and 20 edges. One unit explains one stage or decision, not an entire large function. Split long dispatches, branch chains, separate API operations, or multi-stage behavior at handoffs before those limits are approached. Preserve source order, outcomes, continuation, and fallback; do not collapse them into dense scenes or silently omit branches. Use a few related checks per unit so each has a clear visual form and complete narration.

Use delivered source references only as starting-range hints. Do not write explanations, narration, or slide content, and do not call `propose_answer_section`. Return only the supplied plan schema.
"#;

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
    json!({"type":"object","additionalProperties":false,
      "required":["citationVersion","planVersion","title","sections","omissions"],"properties":{
        "citationVersion":{"type":"string","enum":["nativeEvidenceV2"]},
        "planVersion":{"type":"string","enum":["repositoryGuidePlanV1"]},
        "title":{"type":"string","minLength":8,"maxLength":120},
      "sections":{"type":"array","minItems":3,"maxItems":MAX_INITIAL_UNITS + 1,"description":"Item zero is the opening range lead; the remaining two to four items establish the initial core path.","items":{
          "type":"object","additionalProperties":false,"required":["key","question","prerequisites","claims"],"properties":{
            "key":{"type":"string","maxLength":80},"question":{"type":"string","minLength":16,"maxLength":800},
            "prerequisites":{"type":"array","maxItems":5,"items":{"type":"string","maxLength":80}},
            "claims":claims}}},
        "omissions":{"type":"array","maxItems":MAX_OMISSIONS,"items":{"type":"string","maxLength":MAX_OMISSION_BYTES}}}})
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

fn canonicalize(resolved: &native_citations::Resolved) -> Result<Plan, String> {
    let proposal: Proposal = serde_json::from_str(resolved.text())
        .map_err(|_| "The guide outline has an invalid structure.")?;
    if proposal.plan_version != "repositoryGuidePlanV1"
        || !text(&proposal.title, 8, 120)
        || !(3..=MAX_INITIAL_UNITS + 1).contains(&proposal.sections.len())
        || !valid_omissions(&proposal.omissions)
    {
        return Err("The guide outline exceeds its fixed limits.".into());
    }

    let mut proposed_sections = proposal.sections.into_iter();
    let opening = proposed_sections
        .next()
        .ok_or("The guide outline has no opening range lead.")?;
    if opening.key != "opening"
        || opening.question != opening::LEARNING_QUESTION
        || !opening.prerequisites.is_empty()
        || opening.claims.len() != 1
    {
        return Err(
            "The first guide outline item must be the app-scoped opening range lead.".into(),
        );
    }
    let opening_ranges = starting_ranges(resolved, 0, &opening.claims[0])?;
    let follow_on_sections: Vec<_> = proposed_sections.collect();
    if !(2..=MAX_INITIAL_UNITS).contains(&follow_on_sections.len()) {
        return Err(
            "The initial guide outline needs two to four follow-on sections after its opening lead.".into(),
        );
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
                "Guide prerequisites may name only sections that appear earlier in the outline; the separate opening slide has no section key.".into(),
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
    opening::question(job)?;
    let prompt = serde_json::to_string(&json!({"question":job.question,"snapshotId":job.snapshot_id,
        "scope":"all eligible captured implementation, researched by ranges; never select whole files for transmission",
        "guideContext":{"guideId":job.id,"question":job.question,"openingQuestion":opening::LEARNING_QUESTION}}))
        .map_err(|_| "The guide question could not be encoded.")?;
    let schema = schema();
    let instructions = format!(
        "{PLAN_INSTRUCTIONS}\n{}\n{RESEARCH_INSTRUCTIONS}\nThis repository guide has no wall-clock deadline. Stop the initial plan once the opening lead, two to four core-path questions, and explicit pending omission topics are source-grounded; later units will investigate the omitted topics independently. Native source and call limits still apply.",
        crate::provider::visual_scenes::READER_LANGUAGE_POLICY
    );
    let contract = json!({"contractVersion":job.contract,
        "runId":job.planning_run_id,"guideId":job.id,"grantId":job.grant_id,"manifestSha256":job.manifest_sha256,
        "instructions":instructions,"prompt":prompt,"schema":schema,"tools":context_mcp::research_tools(),
        "operationPolicy":"repository-guide-plan-focused-discovery-v6",
        "readerLanguagePolicy":crate::provider::visual_scenes::READER_LANGUAGE_POLICY_VERSION,
        "model":job.model_id,"effort":job.reasoning_effort,
        "maxInitialUnits":MAX_INITIAL_UNITS,"maxTotalUnits":MAX_TOTAL_UNITS,"firstBufferSlides":5,
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
    let session = preparation.run_stage(
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
    )?;
    let preparation_stages = preparation.into_stages();
    let outcome = manager.run_with_result_and_publish_and_preparation(
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
