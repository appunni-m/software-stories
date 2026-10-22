//! Repository guides use the installed Codex harness. A bounded research
//! outline drives independently owned investigations. Restart never dispatches.

use crate::{
    ProjectSessions, app_data_directory, codex_agent,
    evidence_ledger::sections::{
        Outcome, SectionNotice,
        reading::{self, Summary},
    },
    recovery, repository_context as context,
    repository_context_commands::RepositoryContextJobs,
    snapshot, workspace,
    workspace_generation::{self, Input, Reference},
};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tauri::{AppHandle, Emitter, Manager, State};
use uuid::Uuid;

mod opening;
mod opening_continuation;
mod plan;
mod recovering;
mod store;
use plan::Plan;
use recovering::reconcile;

const CONTRACT: &str = "repository-guide-v8-incremental-retries";
const RECORD_SCHEMA_VERSION: u32 = 7;
#[cfg(test)]
const DEFAULT_MODEL_ID: &str = "gpt-6-luna";
#[cfg(test)]
const DEFAULT_REASONING_EFFORT: &str = "max";
const RESEARCH_POLICY: &str = "ordered-independent-research-v1";
const RESEARCH_INSTRUCTIONS: &str = "Prerequisites describe reading order, not permission to investigate. Each outlined unit will research its own captured code in outline order even if an earlier explanation remains incomplete. Earlier questions and starting ranges supply leads, never implementation facts or inherited citations. Preserve requested alternatives and failures; independently verify every explanation. A partial unit stays partial and cannot establish a later unit's behavior.";
const MAX_UNITS: usize = 6;
const MAX_ATTEMPTS: usize = 3;
const MAX_AUTOMATIC_RETRIES_PER_UNIT: u8 = 1;
/// Initial builds and explicit resumes fill at most five slides; playback
/// refills use a single slide. Every worker entry point requires a limit.
const GUIDE_SLIDE_BUFFER: usize = 5;
const MAX_OPENING_CONTINUATIONS: usize = 2;
pub(crate) const MAX_GUIDE_ANSWER_RECORDS: usize =
    1 + MAX_OPENING_CONTINUATIONS + MAX_UNITS * MAX_ATTEMPTS;

/// Storage accounting only. This does not authorize retrieval or inference.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct AnswerReservation {
    pub(crate) guide_id: String,
    pub(crate) project_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) manifest_sha256: String,
    pub(crate) run_ids: Vec<String>,
}

fn answer_reservation(record: &Record) -> AnswerReservation {
    AnswerReservation {
        guide_id: record.id.clone(),
        project_id: record.project_id.clone(),
        snapshot_id: record.snapshot_id.clone(),
        manifest_sha256: record.manifest_sha256.clone(),
        run_ids: record
            .opening
            .iter()
            .chain(&record.opening_continuations)
            .chain(record.units.iter().flat_map(|unit| &unit.attempts))
            .map(|attempt| attempt.run_id.clone())
            .collect(),
    }
}

pub(crate) fn answer_reservations(storage: &Path) -> Result<Vec<AnswerReservation>, String> {
    store::answer_reservations(storage)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Status {
    Planning,
    Building,
    WaitingForPlayback,
    Ready,
    Partial,
    Stopped,
    Interrupted,
    Failed,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Attempt {
    run_id: String,
    question: String,
    automatic_retry: bool,
    summary: Option<Summary>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Unit {
    key: String,
    attempts: Vec<Attempt>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    revision: u64,
    schema_version: u32,
    contract: String,
    id: String,
    project_id: String,
    snapshot_id: String,
    manifest_sha256: String,
    grant_id: String,
    question: String,
    model_id: String,
    reasoning_effort: String,
    planning_run_id: String,
    planning_retry_count: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    opening: Option<Attempt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    opening_context_sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    opening_continuations: Vec<Attempt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    consumed_slide_revisions: Vec<String>,
    plan: Option<Plan>,
    units: Vec<Unit>,
    status: Status,
    active_run_id: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuideSummary {
    revision: u64,
    id: String,
    project_id: String,
    snapshot_id: String,
    question: String,
    title: String,
    status: Status,
    units: Vec<UnitSummary>,
    answers: Vec<Summary>,
    opening_run_id: Option<String>,
    opening_run_ids: Vec<String>,
    can_continue: bool,
    continuation_operations: usize,
    omissions: Vec<String>,
    captured_files: usize,
    starting_files: usize,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuideList {
    guides: Vec<GuideSummary>,
    unavailable_count: usize,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UnitSummary {
    key: String,
    question: String,
    prerequisites: Vec<String>,
    status: String,
    run_ids: Vec<String>,
}

fn uuid(id: &str) -> bool {
    Uuid::parse_str(id).is_ok_and(|u| u.to_string() == id)
}
fn text(s: &str, min: usize, max: usize) -> bool {
    (min..=max).contains(&s.len()) && s.trim() == s && !s.chars().any(char::is_control)
}
fn model_text(s: &str, max: usize) -> bool {
    text(s, 1, max)
        && s.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.:/".contains(&byte))
}
fn validate_model_selection(
    models: &[codex_agent::CodexModel],
    model_id: &str,
    effort: &str,
) -> Result<(), String> {
    if models.iter().any(|model| {
        model.model_id == model_id
            && model
                .supported_reasoning_efforts
                .iter()
                .any(|supported| supported == effort)
    }) {
        Ok(())
    } else {
        Err("The selected Codex model or reasoning effort is no longer available. Choose an available option in Connections.".into())
    }
}
fn key(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn checked_manifest(storage: &Path, job: &Record) -> Result<snapshot::SnapshotManifest, String> {
    let manifest = snapshot::load_snapshot_manifest(storage, &job.project_id, &job.snapshot_id)?;
    let digest = context::digest(
        &serde_json::to_vec(&manifest)
            .map_err(|_| "The captured inventory could not be checked.")?,
    );
    if digest != job.manifest_sha256 {
        return Err(
            "This guide's captured inventory changed. Saved chapters were preserved.".into(),
        );
    }
    if let Some(plan) = &job.plan {
        for range in plan
            .opening_ranges
            .iter()
            .chain(plan.units.iter().flat_map(|u| &u.starting_ranges))
        {
            let source = context::source(storage, &manifest, &range.path)?;
            if context::range_from_source(&source, range.start_byte, range.end_byte)?.range
                != *range
            {
                return Err(
                    "A guide starting range changed. No investigation was dispatched.".into(),
                );
            }
        }
    }
    Ok(manifest)
}
fn scoped(storage: &Path, id: &str, project: &str, snapshot: &str) -> Result<Record, String> {
    let record = store::read(storage, id)?;
    if record.project_id != project || record.snapshot_id != snapshot {
        return Err("This guide belongs to another repository snapshot.".into());
    }
    checked_manifest(storage, &record)?;
    Ok(record)
}
fn unit_complete(unit: &Unit) -> bool {
    unit.attempts
        .last()
        .and_then(|a| a.summary.as_ref())
        .is_some_and(|s| s.outcome == Outcome::Complete && !s.sections.is_empty())
}
fn guide_complete(record: &Record) -> bool {
    opening_continuation::latest(record).is_some_and(|a| {
        a.summary
            .as_ref()
            .is_some_and(|s| s.outcome == Outcome::Complete && !s.sections.is_empty())
    }) && record.units.iter().all(unit_complete)
}
fn published_slide_count(record: &Record) -> usize {
    record
        .opening
        .iter()
        .chain(&record.opening_continuations)
        .chain(record.units.iter().flat_map(|unit| &unit.attempts))
        .filter_map(|attempt| attempt.summary.as_ref())
        .flat_map(|answer| &answer.sections)
        .map(|section| section.revision_id.as_str())
        .collect::<std::collections::HashSet<_>>()
        .len()
}
fn contains_slide(record: &Record, revision_id: &str) -> bool {
    record
        .opening
        .iter()
        .chain(&record.opening_continuations)
        .chain(record.units.iter().flat_map(|unit| &unit.attempts))
        .filter_map(|attempt| attempt.summary.as_ref())
        .flat_map(|answer| &answer.sections)
        .any(|section| section.revision_id == revision_id)
}
fn unit_research_eligible(record: &Record, index: usize) -> bool {
    record.status != Status::Stopped
        && record
            .units
            .get(index)
            .is_some_and(|unit| !unit_complete(unit) && unit.attempts.len() < MAX_ATTEMPTS)
}

fn automatic_slide_retry_allowed(
    answer: Option<&crate::evidence_ledger::sections::Answer>,
    error: Option<&str>,
    status: &Status,
) -> bool {
    if operation_requires_halt(error, answer, status) {
        return false;
    }
    true
}

fn operation_requires_halt(
    error: Option<&str>,
    answer: Option<&crate::evidence_ledger::sections::Answer>,
    status: &Status,
) -> bool {
    *status == Status::Stopped
        || error.is_some_and(|message| {
            message == crate::codex_identity::changed()
                || message.contains("permission")
                || message.contains("Permission")
        })
        || answer.is_some_and(|answer| {
            answer.outcome == Outcome::Stopped
                || answer.failure_code()
                    == Some(&crate::evidence_ledger::sections::FailureCode::Stopped)
                || answer.failure_code()
                    == Some(&crate::evidence_ledger::sections::FailureCode::RuntimeIdentity)
        })
}

fn synthetic_failed_slide(
    run_id: &str,
    project_id: &str,
    snapshot_id: &str,
    question: &str,
) -> Summary {
    Summary {
        run_id: run_id.into(),
        project_id: project_id.into(),
        snapshot_id: snapshot_id.into(),
        question: question.into(),
        outcome: Outcome::Failed,
        sections: vec![],
        incomplete_topics: vec![
            "This slide did not publish. The guide preserved its other progress.".into(),
        ],
    }
}

fn begin_unit(
    storage: &Path,
    expected: &Record,
    index: usize,
    run_id: &str,
    question: &str,
) -> Result<Record, String> {
    begin_unit_with_retry(storage, expected, index, run_id, question, false)
}

fn begin_unit_with_retry(
    storage: &Path,
    expected: &Record,
    index: usize,
    run_id: &str,
    question: &str,
    automatic_retry: bool,
) -> Result<Record, String> {
    store::update(storage, &expected.id, |current| {
        if current.status == Status::Stopped {
            return Err("The guide was stopped. Saved sections remain available.".into());
        }
        if current.revision != expected.revision
            || current.active_run_id.is_some()
            || !unit_research_eligible(current, index)
        {
            return Err("The guide changed before its investigation started. Saved progress was preserved; no operation was dispatched.".into());
        }
        let unit = current
            .units
            .get_mut(index)
            .ok_or("The guide unit is unavailable.")?;
        if automatic_retry {
            if unit.attempts.is_empty()
                || unit
                    .attempts
                    .iter()
                    .filter(|attempt| attempt.automatic_retry)
                    .count()
                    >= usize::from(MAX_AUTOMATIC_RETRIES_PER_UNIT)
            {
                return Err("This slide has no eligible automatic recovery attempt.".into());
            }
        }
        current.status = Status::Building;
        current.active_run_id = Some(run_id.into());
        unit.attempts.push(Attempt {
            run_id: run_id.into(),
            question: question.into(),
            automatic_retry,
            summary: None,
        });
        Ok(())
    })
}

fn check_research_outcome(
    _record: &Record,
    answer: &crate::evidence_ledger::sections::Answer,
) -> Result<(), String> {
    if answer.failure_code()
        == Some(&crate::evidence_ledger::sections::FailureCode::RuntimeIdentity)
    {
        return Err(
            "The Codex account or runtime changed. The guide stopped before another investigation."
                .into(),
        );
    }
    if !matches!(
        answer.outcome,
        Outcome::Complete | Outcome::Partial | Outcome::Failed | Outcome::Stopped
    ) {
        return Err("The guide investigation did not finish safely. Saved sections remain available; no later investigation was started.".into());
    }
    Ok(())
}
fn saved_section(storage: &Path, section: &SectionNotice) -> Result<bool, String> {
    Ok(
        crate::chapter_store::list_for_snapshot(
            storage,
            &section.project_id,
            &section.snapshot_id,
        )?
        .iter()
        .any(|revision| {
            revision.revision_id == section.revision_id
                && revision
                    .generation_origin
                    .as_ref()
                    .is_some_and(|origin| origin.request_id == section.run_id)
        }),
    )
}
fn unit_status(unit: &Unit) -> &'static str {
    match unit.attempts.last() {
        None => "pending",
        Some(a) => match a.summary.as_ref().map(|s| &s.outcome) {
            None | Some(Outcome::Working) => "working",
            Some(Outcome::Complete) => "complete",
            Some(Outcome::Partial) => "partial",
            Some(Outcome::Stopped) => "stopped",
            Some(Outcome::Interrupted) => "interrupted",
            Some(Outcome::Failed) => "failed",
        },
    }
}
fn summary(storage: &Path, record: &Record) -> Result<GuideSummary, String> {
    let manifest = checked_manifest(storage, record)?;
    let mut answers = Vec::new();
    for attempt in record
        .opening
        .iter()
        .chain(&record.opening_continuations)
        .chain(record.units.iter().flat_map(|u| &u.attempts))
    {
        match if reading::exists(storage, &attempt.run_id)? {
            Some(reading::scoped(
                storage,
                &attempt.run_id,
                &record.project_id,
                &record.snapshot_id,
            )?)
        } else {
            None
        } {
            Some(answer) => {
                let answer = answer.summary();
                if answer.question != attempt.question {
                    return Err("A guide investigation question changed.".into());
                }
                answers.push(answer);
            }
            None => {
                if let Some(saved) = &attempt.summary {
                    let chapters = crate::chapter_store::list_for_snapshot(
                        storage,
                        &record.project_id,
                        &record.snapshot_id,
                    )?;
                    if saved.sections.iter().any(|s| {
                        !chapters.iter().any(|r| {
                            r.revision_id == s.revision_id
                                && r.generation_origin
                                    .as_ref()
                                    .is_some_and(|o| o.request_id == s.run_id)
                        })
                    }) {
                        return Err("A saved guide section is unavailable. Existing guide data was preserved.".into());
                    }
                    answers.push(saved.clone());
                }
            }
        }
    }
    let plan = record.plan.as_ref();
    let units = record
        .units
        .iter()
        .enumerate()
        .map(|(i, u)| UnitSummary {
            key: u.key.clone(),
            question: plan.unwrap().units[i].question.clone(),
            prerequisites: plan.unwrap().units[i].prerequisites.clone(),
            status: unit_status(u).into(),
            run_ids: u.attempts.iter().map(|a| a.run_id.clone()).collect(),
        })
        .collect();
    Ok(GuideSummary {
        revision: record.revision,
        id: record.id.clone(),
        project_id: record.project_id.clone(),
        snapshot_id: record.snapshot_id.clone(),
        question: record.question.clone(),
        title: plan.map_or_else(|| "Repository guide".into(), |p| p.title.clone()),
        status: record.status.clone(),
        units,
        answers,
        opening_run_id: opening_continuation::latest(record).map(|a| a.run_id.clone()),
        opening_run_ids: record
            .opening
            .iter()
            .chain(&record.opening_continuations)
            .map(|a| a.run_id.clone())
            .collect(),
        can_continue: opening_continuation::remaining_operations(record) > 0,
        continuation_operations: opening_continuation::remaining_operations(record),
        omissions: plan.map_or_else(Vec::new, |p| p.omissions.clone()),
        captured_files: manifest
            .files
            .iter()
            .filter(|f| context::exclusion(f).is_none())
            .count(),
        starting_files: plan.map_or(0, |p| {
            p.opening_ranges
                .iter()
                .chain(p.units.iter().flat_map(|u| &u.starting_ranges))
                .map(|r| &r.path)
                .collect::<std::collections::HashSet<_>>()
                .len()
        }),
    })
}
fn continuation(storage: &Path, record: &Record, index: usize) -> Result<String, String> {
    let unit = &record
        .plan
        .as_ref()
        .ok_or("The guide has no outline.")?
        .units[index];
    let mut question = format!(
        "{}\nThis is unit {} of a repository guide. Investigate this unit's behavior only. Its preceding learning questions are context, not evidence: {}.",
        unit.question,
        index + 1,
        record.plan.as_ref().unwrap().units[..index]
            .iter()
            .map(|u| u.question.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    );
    question.push('\n');
    question.push_str(RESEARCH_INSTRUCTIONS);
    if !record.units[index].attempts.is_empty() {
        let chapters = crate::chapter_store::list_for_snapshot(
            storage,
            &record.project_id,
            &record.snapshot_id,
        )?;
        let attempts = &record.units[index].attempts;
        let headings = attempts
            .iter()
            .filter_map(|a| a.summary.as_ref())
            .flat_map(|s| &s.sections)
            .filter_map(|s| chapters.iter().find(|r| r.revision_id == s.revision_id))
            .flat_map(|r| &r.sections)
            .map(|s| s.title.as_str())
            .take(12)
            .collect::<Vec<_>>();
        let missing = attempts
            .last()
            .and_then(|a| a.summary.as_ref())
            .map(|s| s.incomplete_topics.join(" "))
            .unwrap_or_default();
        question.push_str(&format!("\nExplicit continuation: preserve the already saved sections; do not repeat them. This is a NEW isolated investigation with its own publication ledger. Its first new section MUST use order:0 and previousGenerationId:null. Earlier saved sections are outside this investigation: do not count them, revise them, cite their generation IDs or include them in this investigation's final manifest. After a rejection, use nextNewSectionOrder from the native reply; a rejection never advances publication order. Published headings (navigation hints, not source evidence): {}. Investigate the remaining behavior: {}. Reread necessary captured code with the owned tools. State uninvestigated boundaries honestly.",headings.join("; "),missing));
    }
    if question.len() > 12000 {
        return Err("The guide continuation exceeds its intent limit.".into());
    }
    Ok(question)
}

fn unit_intent(
    record: &Record,
    index: usize,
) -> Result<workspace_generation::progressive::GuideIntent, String> {
    let plan = record.plan.as_ref().ok_or("The guide has no outline.")?;
    let unit = plan
        .units
        .get(index)
        .ok_or("The guide unit is unavailable.")?;
    Ok(workspace_generation::progressive::GuideIntent {
        guide_id: record.id.clone(),
        question: record.question.clone(),
        unit_key: unit.key.clone(),
        unit_number: index + 2,
        unit_count: plan.units.len() + 1,
        research_boundaries: plan.omissions.clone(),
        opening_continuation: None,
    })
}
fn save_outline(
    storage: &Path,
    expected: &Record,
    plan: Plan,
    answer: Option<Summary>,
) -> Result<(), String> {
    store::update(storage, &expected.id, |current| {
        if current.status == Status::Stopped {
            return Err("The guide was stopped. No outline was published.".into());
        }
        if current.revision != expected.revision
            || current.plan.is_some()
            || current.active_run_id.as_deref() != Some(expected.planning_run_id.as_str())
        {
            return Err("The guide changed while its outline was being saved. Existing progress was preserved.".into());
        }
        current.units = plan
            .units
            .iter()
            .map(|u| Unit {
                key: u.key.clone(),
                attempts: vec![],
            })
            .collect();
        current.plan = Some(plan);
        if let Some(answer) = answer {
            current
                .opening
                .as_mut()
                .ok_or("The guide opening owner changed. Saved sections were preserved.")?
                .summary = Some(answer);
        }
        current.status = Status::Building;
        current.active_run_id = None;
        Ok(())
    })?;
    Ok(())
}

fn begin_initial_opening(storage: &Path, expected: &Record) -> Result<Record, String> {
    let opening = expected
        .opening
        .as_ref()
        .ok_or("The guide has no opening owner.")?;
    store::update(storage, &expected.id, |current| {
        if current.status == Status::Stopped {
            return Err("The guide was stopped. Its saved outline remains available.".into());
        }
        if current.revision != expected.revision
            || current.plan.is_none()
            || current.active_run_id.is_some()
            || current
                .opening
                .as_ref()
                .is_none_or(|saved| saved.run_id != opening.run_id || saved.summary.is_some())
        {
            return Err(
                "The guide changed before its opening slide started. Saved progress was preserved."
                    .into(),
            );
        }
        current.status = Status::Building;
        current.active_run_id = Some(opening.run_id.clone());
        Ok(())
    })
}

fn finish_initial_opening(
    storage: &Path,
    expected: &Record,
    answer: Summary,
) -> Result<Record, String> {
    let run_id = answer.run_id.clone();
    store::update(storage, &expected.id, |current| {
        if current.active_run_id.as_deref() != Some(run_id.as_str())
            || (current.revision != expected.revision && current.status != Status::Stopped)
        {
            return Err("The opening slide owner changed. Saved sections were preserved.".into());
        }
        let opening = current
            .opening
            .as_mut()
            .ok_or("The guide opening is unavailable.")?;
        if opening.run_id != run_id
            || opening.question != answer.question
            || opening.summary.is_some()
        {
            return Err("The opening slide result is stale. Saved sections were preserved.".into());
        }
        if answer.outcome == Outcome::Stopped {
            current.status = Status::Stopped;
        }
        opening.summary = Some(answer);
        current.active_run_id = None;
        Ok(())
    })
}

fn run_initial_opening(
    storage: &Path,
    record: &Record,
    manager: &codex_agent::CodexAgentManager,
    jobs: RepositoryContextJobs,
    journal: &recovery::InterruptionJournal,
    progress: &impl Fn(codex_agent::CodexProgress),
    notify: Arc<dyn Fn(SectionNotice) + Send + Sync>,
    adopt: &impl Fn(&SectionNotice) -> Result<(), String>,
    changed: &impl Fn(&GuideSummary),
    renderer_queue_wait_ms: Option<u64>,
) -> Result<Record, String> {
    checked_manifest(storage, record)?;
    let runtime = manager.review_runtime(storage)?;
    workspace::check_retrieval_grant(
        storage,
        &record.grant_id,
        &record.project_id,
        &record.snapshot_id,
        &runtime,
    )?;
    let plan = record
        .plan
        .as_ref()
        .ok_or("The guide has no saved outline.")?;
    // Use the separately cited opening lead to avoid an opening-wide rescan;
    // the progressive request rechecks it and treats it only as a research hint.
    let starting_ranges = opening::starting_ranges(record)?.to_vec();
    let opening = record
        .opening
        .as_ref()
        .ok_or("The guide has no opening owner.")?;
    let intent = workspace_generation::progressive::GuideIntent {
        guide_id: record.id.clone(),
        question: record.question.clone(),
        unit_key: "opening".into(),
        unit_number: 1,
        unit_count: plan.units.len() + 1,
        research_boundaries: plan.omissions.clone(),
        opening_continuation: None,
    };
    let run_id = opening.run_id.clone();
    let question = opening::question(record)?.to_owned();
    let references = starting_ranges
        .iter()
        .map(|range| Reference {
            path: range.path.clone(),
            sha256: Some(range.source_sha256.clone()),
            from: Some(range.start_line as u32),
            to: Some(range.end_line as u32),
        })
        .collect();
    let record = begin_initial_opening(storage, record)?;
    changed(&summary(storage, &record)?);
    let result = workspace_generation::progressive::run_with_starting_ranges(
        storage,
        Input {
            run_id: run_id.clone(),
            project_id: record.project_id.clone(),
            snapshot_id: record.snapshot_id.clone(),
            grant_id: record.grant_id.clone(),
            question: question.clone(),
            references,
            model_id: record.model_id.clone(),
            effort: record.reasoning_effort.clone(),
            renderer_queue_wait_ms,
        },
        &starting_ranges,
        Some(&intent),
        manager,
        jobs,
        journal,
        progress,
        notify,
    );
    let operation_error = result.as_ref().err().cloned();
    let answer = match result {
        Ok(answer) => answer,
        Err(error) => {
            #[cfg(test)]
            eprintln!("GUIDE_OPENING_FAILURE {run_id}: {error}");
            if reading::exists(storage, &run_id)? {
                reading::scoped(storage, &run_id, &record.project_id, &record.snapshot_id)?
                    .summary()
            } else {
                synthetic_failed_slide(&run_id, &record.project_id, &record.snapshot_id, &question)
            }
        }
    };
    for section in &answer.sections {
        adopt(section)?;
    }
    let saved_answer = if reading::exists(storage, &run_id)? {
        Some(reading::scoped(
            storage,
            &run_id,
            &record.project_id,
            &record.snapshot_id,
        )?)
    } else {
        None
    };
    let record = finish_initial_opening(storage, &record, answer)?;
    changed(&summary(storage, &record)?);
    if operation_requires_halt(
        operation_error.as_deref(),
        saved_answer.as_ref(),
        &record.status,
    ) {
        if record.status == Status::Stopped {
            return Ok(record);
        }
        return Err(operation_error.unwrap_or_else(|| {
            "The opening investigation could not continue safely. Saved sections remain available.".into()
        }));
    }
    if let Some(answer) = saved_answer.as_ref() {
        check_research_outcome(&record, answer)?;
    }
    Ok(record)
}

fn build(
    storage: &Path,
    id: &str,
    continue_opening: bool,
    manager: &codex_agent::CodexAgentManager,
    jobs: RepositoryContextJobs,
    journal: &recovery::InterruptionJournal,
    max_new_slides: usize,
    renderer_queue_wait_ms: Option<u64>,
    progress: impl Fn(codex_agent::CodexProgress),
    notify: Arc<dyn Fn(SectionNotice) + Send + Sync>,
    adopt: impl Fn(&SectionNotice) -> Result<(), String>,
    changed: impl Fn(&GuideSummary),
) -> Result<GuideSummary, String> {
    let result = build_inner(
        storage,
        id,
        continue_opening,
        manager,
        jobs,
        journal,
        max_new_slides,
        renderer_queue_wait_ms,
        progress,
        notify,
        adopt,
        &changed,
    );
    if result.is_err() {
        // Keep the original operation error even if local cleanup fails. The
        // next read can still reconcile without dispatching another request.
        if let Ok(record) = recovering::settle_failed_build(storage, id, manager) {
            if let Ok(saved) = summary(storage, &record) {
                changed(&saved);
            }
        }
    }
    result
}

fn build_inner(
    storage: &Path,
    id: &str,
    continue_opening: bool,
    manager: &codex_agent::CodexAgentManager,
    jobs: RepositoryContextJobs,
    journal: &recovery::InterruptionJournal,
    max_new_slides: usize,
    mut renderer_queue_wait_ms: Option<u64>,
    progress: impl Fn(codex_agent::CodexProgress),
    notify: Arc<dyn Fn(SectionNotice) + Send + Sync>,
    adopt: impl Fn(&SectionNotice) -> Result<(), String>,
    changed: impl Fn(&GuideSummary),
) -> Result<GuideSummary, String> {
    let mut record = store::read(storage, id)?;
    let starting_slide_count = published_slide_count(&record);
    crate::evidence_ledger::sections::capacity::with_guide_reservation(
        storage,
        &answer_reservation(&record),
        || Ok(()),
    )?;
    checked_manifest(storage, &record)?;
    if record.plan.is_none() {
        if record.status != Status::Planning {
            return Err("This guide stopped before its outline was saved. Start a new guide; no investigation was replayed.".into());
        }
        // The reader must know the opening owner before its first publication.
        // This observation starts no additional model operation.
        changed(&summary(storage, &record)?);
        loop {
            let owner = record.clone();
            let result = plan::run(
                storage,
                &owner,
                manager,
                jobs.clone(),
                &progress,
                Arc::clone(&notify),
                |plan| save_outline(storage, &owner, plan, None),
            );
            if let Err(error) = result {
                plan::retain_failure(storage, &owner)?;
                let saved_record = store::read(storage, id)?;
                let saved = summary(storage, &saved_record)?;
                for section in saved.answers.iter().flat_map(|a| &a.sections) {
                    adopt(section)?;
                }
                if owner.planning_retry_count == 0
                    && automatic_slide_retry_allowed(None, Some(&error), &saved_record.status)
                {
                    record = store::prepare_plan_retry(storage, id, saved_record.revision)?;
                    changed(&summary(storage, &record)?);
                    continue;
                }
                changed(&saved);
                return Err(error);
            }
            break;
        }
        record = store::read(storage, id)?;
        for section in summary(storage, &record)?
            .answers
            .iter()
            .flat_map(|a| &a.sections)
        {
            adopt(section)?;
        }
        changed(&summary(storage, &record)?);
    }
    record = store::read(storage, id)?;
    if record.status != Status::Stopped
        && record
            .opening
            .as_ref()
            .is_some_and(|opening| opening.summary.is_none())
    {
        record = run_initial_opening(
            storage,
            &record,
            manager,
            jobs.clone(),
            journal,
            &progress,
            Arc::clone(&notify),
            &adopt,
            &changed,
            renderer_queue_wait_ms.take(),
        )?;
    }
    if continue_opening && opening_continuation::eligible(&record) {
        opening_continuation::run(
            storage,
            &record,
            manager,
            jobs.clone(),
            journal,
            &progress,
            Arc::clone(&notify),
            &adopt,
            &changed,
            false,
            renderer_queue_wait_ms.take(),
        )?;
    }
    record = store::read(storage, id)?;
    if opening_continuation::automatic_retry_eligible(&record) {
        record = opening_continuation::run(
            storage,
            &record,
            manager,
            jobs.clone(),
            journal,
            &progress,
            Arc::clone(&notify),
            &adopt,
            &changed,
            true,
            renderer_queue_wait_ms.take(),
        )?;
    }
    for index in 0..record.units.len() {
        record = store::read(storage, id)?;
        if record.status == Status::Stopped {
            break;
        }
        if published_slide_count(&record).saturating_sub(starting_slide_count) >= max_new_slides {
            break;
        }
        for section in record.units[index]
            .attempts
            .iter()
            .filter_map(|a| a.summary.as_ref())
            .flat_map(|s| &s.sections)
        {
            adopt(section)?;
        }
        let mut automatic_retry = false;
        loop {
            record = store::read(storage, id)?;
            if record.status == Status::Stopped || !unit_research_eligible(&record, index) {
                break;
            }
            checked_manifest(storage, &record)?;
            let runtime = manager.review_runtime(storage)?;
            workspace::check_retrieval_grant(
                storage,
                &record.grant_id,
                &record.project_id,
                &record.snapshot_id,
                &runtime,
            )?;
            let question = continuation(storage, &record, index)?;
            let guide_intent = unit_intent(&record, index)?;
            let run_id = Uuid::new_v4().to_string();
            record = begin_unit_with_retry(
                storage,
                &record,
                index,
                &run_id,
                &question,
                automatic_retry,
            )?;
            changed(&summary(storage, &record)?);
            let references = record.plan.as_ref().unwrap().units[index]
                .starting_ranges
                .iter()
                .map(|r| Reference {
                    path: r.path.clone(),
                    sha256: Some(r.source_sha256.clone()),
                    from: Some(r.start_line as u32),
                    to: Some(r.end_line as u32),
                })
                .collect();
            let result = workspace_generation::progressive::run_with_starting_ranges(
                storage,
                Input {
                    run_id: run_id.clone(),
                    project_id: record.project_id.clone(),
                    snapshot_id: record.snapshot_id.clone(),
                    grant_id: record.grant_id.clone(),
                    question: question.clone(),
                    references,
                    model_id: record.model_id.clone(),
                    effort: record.reasoning_effort.clone(),
                    renderer_queue_wait_ms: renderer_queue_wait_ms.take(),
                },
                &record.plan.as_ref().unwrap().units[index].starting_ranges,
                Some(&guide_intent),
                manager,
                jobs.clone(),
                journal,
                &progress,
                Arc::clone(&notify),
            );
            let operation_error = result.as_ref().err().cloned();
            let answer = match result {
                Ok(answer) => answer,
                Err(error) => {
                    #[cfg(test)]
                    eprintln!("GUIDE_UNIT_FAILURE {run_id}: {error}");
                    if reading::exists(storage, &run_id)? {
                        reading::scoped(storage, &run_id, &record.project_id, &record.snapshot_id)?
                            .summary()
                    } else {
                        synthetic_failed_slide(
                            &run_id,
                            &record.project_id,
                            &record.snapshot_id,
                            &question,
                        )
                    }
                }
            };
            for section in &answer.sections {
                adopt(section)?;
            }
            let retryable_content_gap =
                answer.outcome != Outcome::Complete && answer.sections.is_empty();
            record = store::update(storage, id, |current| {
                let attempt = current.units[index]
                    .attempts
                    .last_mut()
                    .ok_or("The guide unit disappeared.")?;
                if attempt.run_id != run_id {
                    return Err("The guide's investigation owner changed.".into());
                }
                if answer.outcome == Outcome::Stopped {
                    current.status = Status::Stopped;
                }
                attempt.summary = Some(answer);
                current.active_run_id = None;
                Ok(())
            })?;
            changed(&summary(storage, &record)?);

            let saved_answer = if reading::exists(storage, &run_id)? {
                Some(reading::scoped(
                    storage,
                    &run_id,
                    &record.project_id,
                    &record.snapshot_id,
                )?)
            } else {
                None
            };
            if operation_requires_halt(
                operation_error.as_deref(),
                saved_answer.as_ref(),
                &record.status,
            ) {
                if record.status == Status::Stopped {
                    break;
                }
                return Err(operation_error.unwrap_or_else(|| {
                    "The guide could not continue safely. Saved sections remain available.".into()
                }));
            }
            if let Some(answer) = saved_answer.as_ref() {
                check_research_outcome(&record, answer)?;
            }
            let can_retry = retryable_content_gap
                && record.units[index]
                    .attempts
                    .iter()
                    .filter(|attempt| attempt.automatic_retry)
                    .count()
                    < usize::from(MAX_AUTOMATIC_RETRIES_PER_UNIT)
                && automatic_slide_retry_allowed(
                    saved_answer.as_ref(),
                    operation_error.as_deref(),
                    &record.status,
                );
            if can_retry {
                automatic_retry = true;
                continue;
            }
            break;
        }
    }
    record = store::update(storage, id, |current| {
        if current.status != Status::Stopped {
            current.status = if guide_complete(current) {
                Status::Ready
            } else if published_slide_count(current).saturating_sub(starting_slide_count)
                >= max_new_slides
                && opening_continuation::remaining_operations(current) > 0
            {
                Status::WaitingForPlayback
            } else {
                Status::Partial
            };
        }
        current.active_run_id = None;
        Ok(())
    })?;
    let result = summary(storage, &record)?;
    changed(&result);
    Ok(result)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Start {
    id: String,
    project_id: String,
    snapshot_id: String,
    grant_id: String,
    question: String,
    model_id: String,
    reasoning_effort: String,
    consent: bool,
    renderer_queue_wait_ms: Option<u64>,
}

fn bounded_renderer_queue_wait_ms(value: Option<u64>) -> Option<u64> {
    value.filter(|elapsed| *elapsed <= crate::generation_timing::MAX_ELAPSED)
}

fn new_record(input: Start, manifest: &snapshot::SnapshotManifest) -> Result<Record, String> {
    let planning_run_id = input.id.clone();
    let opening_run_id = Uuid::new_v4().to_string();
    let mut record = Record {
        revision: 0,
        schema_version: RECORD_SCHEMA_VERSION,
        contract: CONTRACT.into(),
        id: input.id,
        project_id: input.project_id,
        snapshot_id: input.snapshot_id,
        manifest_sha256: context::digest(
            &serde_json::to_vec(&manifest).map_err(|_| "The inventory could not be encoded.")?,
        ),
        grant_id: input.grant_id,
        model_id: input.model_id,
        reasoning_effort: input.reasoning_effort,
        opening: Some(Attempt {
            run_id: opening_run_id,
            question: opening::LEARNING_QUESTION.into(),
            automatic_retry: false,
            summary: None,
        }),
        opening_context_sha256: None,
        opening_continuations: vec![],
        consumed_slide_revisions: vec![],
        question: input.question,
        planning_run_id,
        planning_retry_count: 0,
        plan: None,
        units: vec![],
        status: Status::Planning,
        active_run_id: None,
    };
    record.opening_context_sha256 = Some(opening::context_sha256(&record)?);
    Ok(record)
}

#[tauri::command]
pub(crate) async fn start_repository_guide(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    input: Start,
) -> Result<GuideSummary, String> {
    sessions.get(&input.project_id)?;
    if !input.consent
        || !uuid(&input.id)
        || !uuid(&input.grant_id)
        || !text(&input.question, 1, 12000)
        || !model_text(&input.model_id, 200)
        || !model_text(&input.reasoning_effort, 32)
    {
        return Err("Review and allow this repository guide first.".into());
    }
    let storage = app_data_directory(&app)?;
    let manifest =
        snapshot::load_snapshot_manifest(&storage, &input.project_id, &input.snapshot_id)?;
    let runtime = app
        .state::<codex_agent::CodexAgentManager>()
        .review_runtime(&storage)?;
    workspace::check_retrieval_grant(
        &storage,
        &input.grant_id,
        &input.project_id,
        &input.snapshot_id,
        &runtime,
    )?;
    let models = app
        .state::<codex_agent::CodexAgentManager>()
        .models(&storage)?;
    validate_model_selection(&models, &input.model_id, &input.reasoning_effort)?;
    let renderer_queue_wait_ms = bounded_renderer_queue_wait_ms(input.renderer_queue_wait_ms);
    let record = new_record(input, &manifest)?;
    let lease =
        store::lease(&storage, &record.id)?.ok_or("This guide already has a native worker.")?;
    store::create(&storage, &record)?;
    let id = record.id.clone();
    store::update(&storage, &id, |current| {
        current.active_run_id = Some(current.planning_run_id.clone());
        Ok(())
    })?;
    run_command(
        app,
        storage,
        id,
        lease,
        false,
        GUIDE_SLIDE_BUFFER,
        renderer_queue_wait_ms,
    )
    .await
}
async fn run_command(
    app: AppHandle,
    storage: std::path::PathBuf,
    id: String,
    lease: crate::file_lease::Lease,
    continue_opening: bool,
    max_new_slides: usize,
    renderer_queue_wait_ms: Option<u64>,
) -> Result<GuideSummary, String> {
    tauri::async_runtime::spawn_blocking(move||{
        let _lease=lease;
        let manager=app.state::<codex_agent::CodexAgentManager>().inner().clone();
        let jobs=app.state::<RepositoryContextJobs>().inner().clone();
        let journal=app.state::<recovery::InterruptionJournal>().inner().clone();
        let emitter=app.clone();
        let guide_id_for_sections=id.clone();
        let result=build(&storage,&id,continue_opening,&manager,jobs,&journal,max_new_slides,renderer_queue_wait_ms,|p|{let _=app.emit("codex-agent-progress",p);},
            Arc::new(move|section|{
                let _=emitter.emit("workspace-answer-section",section.clone());
                let _=emitter.emit("repository-guide-section-published",serde_json::json!({"guideId":&guide_id_for_sections,"section":section}));
            }),
            |section|{if saved_section(&storage, section)? { return Ok(()); } let lock=app.state::<crate::ChapterStoreLock>();let _guard=lock.0.lock().map_err(|_|"Chapter storage is unavailable.")?;
                reading::adopt(&storage,&section.run_id,&section.project_id,&section.snapshot_id,&section.generation_id)?;Ok(())},
            |guide|{let _=app.emit("repository-guide-changed",guide);});
        result
    }).await.map_err(|_|"The guide worker stopped unexpectedly. Inspect saved work before explicitly resuming; nothing was replayed.".to_owned())?
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn resume_repository_guide(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    id: String,
    project_id: String,
    snapshot_id: String,
    consent: bool,
    renderer_queue_wait_ms: Option<u64>,
) -> Result<GuideSummary, String> {
    sessions.get(&project_id)?;
    if !consent {
        return Err("Allow continuation before building more guide content.".into());
    }
    let storage = app_data_directory(&app)?;
    let renderer_queue_wait_ms = bounded_renderer_queue_wait_ms(renderer_queue_wait_ms);
    let record = scoped(&storage, &id, &project_id, &snapshot_id)?;
    let lease = store::lease(&storage, &id)?
        .ok_or("This guide is already being built. No second worker was started.")?;
    let manager = app.state::<codex_agent::CodexAgentManager>();
    let record = reconcile(&storage, &record, &manager)?;
    let expected_revision = record.revision;
    let runtime = manager.review_runtime(&storage)?;
    workspace::check_retrieval_grant(
        &storage,
        &record.grant_id,
        &project_id,
        &snapshot_id,
        &runtime,
    )?;
    crate::evidence_ledger::sections::capacity::with_guide_reservation(
        &storage,
        &answer_reservation(&record),
        || Ok(()),
    )?;
    if record.plan.is_none() {
        store::prepare_plan_retry(&storage, &id, expected_revision)?;
        return run_command(
            app,
            storage,
            id,
            lease,
            false,
            GUIDE_SLIDE_BUFFER,
            renderer_queue_wait_ms,
        )
        .await;
    }
    store::prepare_continuation(&storage, &id, expected_revision)?;
    run_command(
        app,
        storage,
        id,
        lease,
        true,
        GUIDE_SLIDE_BUFFER,
        renderer_queue_wait_ms,
    )
    .await
}

/// Admit one new slide after the reader finishes a narrated slide.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn advance_repository_guide(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    id: String,
    project_id: String,
    snapshot_id: String,
    consumed_revision_id: String,
    renderer_queue_wait_ms: Option<u64>,
) -> Result<GuideSummary, String> {
    sessions.get(&project_id)?;
    if !text(&consumed_revision_id, 1, 256) {
        return Err("The completed guide slide could not be identified.".into());
    }
    let storage = app_data_directory(&app)?;
    let renderer_queue_wait_ms = bounded_renderer_queue_wait_ms(renderer_queue_wait_ms);
    let record = scoped(&storage, &id, &project_id, &snapshot_id)?;
    if !contains_slide(&record, &consumed_revision_id) {
        return Err(
            "The completed slide does not belong to this guide. Saved progress was preserved."
                .into(),
        );
    }
    if record
        .consumed_slide_revisions
        .contains(&consumed_revision_id)
    {
        return summary(&storage, &record);
    }
    let manager = app
        .state::<codex_agent::CodexAgentManager>()
        .inner()
        .clone();
    let Some(lease) = store::lease(&storage, &id)? else {
        // A worker owned by another webview still controls this guide. Do not
        // claim the playback receipt; the reader can submit it after that run.
        return summary(&storage, &record);
    };
    let record = reconcile(&storage, &record, &manager)?;
    if record.status == Status::Stopped || opening_continuation::remaining_operations(&record) == 0
    {
        return summary(&storage, &record);
    }
    if !contains_slide(&record, &consumed_revision_id) {
        return Err(
            "The completed slide is no longer part of this guide. Saved progress was preserved."
                .into(),
        );
    }
    if record
        .consumed_slide_revisions
        .contains(&consumed_revision_id)
    {
        return summary(&storage, &record);
    }
    crate::evidence_ledger::sections::capacity::with_guide_reservation(
        &storage,
        &answer_reservation(&record),
        || Ok(()),
    )?;
    let runtime = manager.review_runtime(&storage)?;
    workspace::check_retrieval_grant(
        &storage,
        &record.grant_id,
        &project_id,
        &snapshot_id,
        &runtime,
    )?;
    let mut duplicate = false;
    let record = store::update_if_changed(&storage, &id, |current| {
        if !contains_slide(current, &consumed_revision_id) {
            return Err("The completed slide changed while its progress was saved.".into());
        }
        if current
            .consumed_slide_revisions
            .contains(&consumed_revision_id)
        {
            duplicate = true;
            return Ok(false);
        }
        if current.consumed_slide_revisions.len() >= MAX_GUIDE_ANSWER_RECORDS {
            return Err(
                "The guide reached its slide progression limit. Saved slides remain available."
                    .into(),
            );
        }
        current
            .consumed_slide_revisions
            .push(consumed_revision_id.clone());
        Ok(true)
    })?;
    if duplicate {
        return summary(&storage, &record);
    }
    let expected_revision = record.revision;
    store::prepare_continuation(&storage, &id, expected_revision)?;
    run_command(app, storage, id, lease, false, 1, renderer_queue_wait_ms).await
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) fn stop_repository_guide(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
    id: String,
    project_id: String,
    snapshot_id: String,
) -> Result<(), String> {
    let storage = app_data_directory(&app)?;
    stop_saved_guide(&storage, &manager, &id, &project_id, &snapshot_id)
}

fn stop_saved_guide(
    storage: &Path,
    manager: &codex_agent::CodexAgentManager,
    id: &str,
    project_id: &str,
    snapshot_id: &str,
) -> Result<(), String> {
    if !store::exists(storage, id)? {
        manager.cancel(id);
        return Ok(());
    }
    // Admit Stop and select the current owner in the same transaction. Reading
    // an owner first could miss a just-admitted investigation between actions.
    let stopped = store::update(storage, id, |record| {
        if record.project_id != project_id || record.snapshot_id != snapshot_id {
            return Err("This guide belongs to another repository snapshot.".into());
        }
        record.status = Status::Stopped;
        Ok(())
    })?;
    if let Some(run) = &stopped.active_run_id {
        manager.cancel(run);
    }
    Ok(())
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn list_repository_guides(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
    project_id: String,
    snapshot_id: String,
) -> Result<GuideList, String> {
    let storage = app_data_directory(&app)?;
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        list_saved_guides_with_count(&storage, &project_id, &snapshot_id, &manager)
    })
    .await
    .map_err(|_| "Saved guides could not be read. Nothing was regenerated.".to_owned())?
    .map(|(guides, unavailable_count)| GuideList {
        guides,
        unavailable_count,
    })
}

fn list_saved_guides(
    storage: &Path,
    project_id: &str,
    snapshot_id: &str,
    manager: &codex_agent::CodexAgentManager,
) -> Result<Vec<GuideSummary>, String> {
    list_saved_guides_with_count(storage, project_id, snapshot_id, manager)
        .map(|(guides, _)| guides)
}

fn list_saved_guides_with_count(
    storage: &Path,
    project_id: &str,
    snapshot_id: &str,
    manager: &codex_agent::CodexAgentManager,
) -> Result<(Vec<GuideSummary>, usize), String> {
    let records = store::list(storage, project_id, snapshot_id)?;
    let mut guides = Vec::with_capacity(records.len());
    let mut unavailable_count = 0;
    for record in records {
        let guide = (|| {
            let lease = store::lease(storage, &record.id)?;
            let record = if lease.is_some()
                && matches!(record.status, Status::Planning | Status::Building)
                || lease.is_some() && record.active_run_id.is_some()
            {
                reconcile(storage, &record, manager)?
            } else {
                record
            };
            summary(storage, &record)
        })();
        match guide {
            Ok(guide) => guides.push(guide),
            Err(error) => {
                // A single old or damaged saved scene must not hide other
                // independently recoverable guides in this repository.
                eprintln!("A saved repository guide could not be reopened: {error}");
                unavailable_count += 1;
            }
        }
    }
    Ok((guides, unavailable_count))
}

#[cfg(test)]
mod tests;
