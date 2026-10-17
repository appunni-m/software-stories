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

mod plan;
mod store;
use plan::Plan;

const CONTRACT: &str = "repository-guide-v2-short-reference-planning";
const LEGACY_CONTRACT: &str = "repository-guide-v1-owned-range-planning";
const MAX_UNITS: usize = 6;
const MAX_ATTEMPTS: usize = 2;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
enum Status {
    Planning,
    Building,
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
    planning_run_id: String,
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
    omissions: Vec<String>,
    captured_files: usize,
    starting_files: usize,
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
        for range in plan.units.iter().flat_map(|u| &u.starting_ranges) {
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
fn prerequisite_complete(record: &Record, index: usize) -> bool {
    record.plan.as_ref().is_some_and(|p| {
        p.units[index].prerequisites.iter().all(|key| {
            record
                .units
                .iter()
                .any(|u| &u.key == key && unit_complete(u))
        })
    })
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
    for attempt in record.units.iter().flat_map(|u| &u.attempts) {
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
        omissions: plan.map_or_else(Vec::new, |p| p.omissions.clone()),
        captured_files: manifest
            .files
            .iter()
            .filter(|f| context::exclusion(f).is_none())
            .count(),
        starting_files: plan.map_or(0, |p| {
            p.units
                .iter()
                .flat_map(|u| &u.starting_ranges)
                .map(|r| &r.path)
                .collect::<std::collections::HashSet<_>>()
                .len()
        }),
    })
}
fn reconcile(
    storage: &Path,
    record: &Record,
    manager: &codex_agent::CodexAgentManager,
) -> Result<Record, String> {
    let mut next = record.clone();
    for attempt in next.units.iter_mut().flat_map(|u| &mut u.attempts) {
        if attempt
            .summary
            .as_ref()
            .is_some_and(|s| s.outcome != Outcome::Working)
        {
            continue;
        }
        let recovered=manager.with_operation_state(&attempt.run_id,|active|{
            if !reading::exists(storage,&attempt.run_id)? { return Ok(None); }
            let answer=reading::scoped(storage,&attempt.run_id,&record.project_id,&record.snapshot_id)?;
            if answer.question!=attempt.question{return Err("A guide investigation question changed.".into());}
            if active{return Err("This guide still has an owned Codex investigation. Wait for it to finish or stop it.".into());}
            if answer.outcome==Outcome::Working {Ok(Some(reading::interrupt(storage,&attempt.run_id,Outcome::Interrupted)?))}
            else {Ok(Some(answer.summary()))}
        })?;
        attempt.summary = Some(recovered.unwrap_or_else(|| Summary {
            run_id: attempt.run_id.clone(),
            project_id: record.project_id.clone(),
            snapshot_id: record.snapshot_id.clone(),
            question: attempt.question.clone(),
            outcome: Outcome::Interrupted,
            sections: vec![],
            incomplete_topics: vec![
                "The investigation stopped before a saved answer was available.".into(),
            ],
        }));
    }
    next.active_run_id = None;
    if matches!(next.status, Status::Planning | Status::Building) {
        next.status = Status::Interrupted;
    }
    store::update(storage, &record.id, |current| {
        if current.status == Status::Stopped {
            next.status = Status::Stopped;
        }
        *current = next.clone();
        Ok(())
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
        question.push_str(&format!("\nExplicit continuation: preserve the already saved sections; do not repeat them. Published headings (navigation hints, not source evidence): {}. Investigate the remaining behavior: {}. Reread necessary captured code with the owned tools. State uninvestigated boundaries honestly.",headings.join("; "),missing));
    }
    if question.len() > 12000 {
        return Err("The guide continuation exceeds its intent limit.".into());
    }
    Ok(question)
}
fn build(
    storage: &Path,
    id: &str,
    manager: &codex_agent::CodexAgentManager,
    jobs: RepositoryContextJobs,
    journal: &recovery::InterruptionJournal,
    progress: impl Fn(codex_agent::CodexProgress),
    notify: Arc<dyn Fn(SectionNotice) + Send + Sync>,
    adopt: impl Fn(&SectionNotice) -> Result<(), String>,
    changed: impl Fn(&GuideSummary),
) -> Result<GuideSummary, String> {
    let mut record = store::read(storage, id)?;
    checked_manifest(storage, &record)?;
    if record.plan.is_none() {
        if record.status != Status::Planning {
            return Err("This guide stopped before its outline was saved. Start a new guide; no investigation was replayed.".into());
        }
        plan::run(storage, &record, manager, jobs.clone(), &progress, |plan| {
            store::update(storage, id, |current| {
                if current.status == Status::Stopped {
                    return Err("The guide was stopped. No outline was published.".into());
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
                current.status = Status::Building;
                current.active_run_id = None;
                Ok(())
            })?;
            Ok(())
        })?;
        record = store::read(storage, id)?;
        changed(&summary(storage, &record)?);
    }
    for index in 0..record.units.len() {
        record = store::read(storage, id)?;
        if record.status == Status::Stopped {
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
        if unit_complete(&record.units[index])
            || record.units[index].attempts.len() >= MAX_ATTEMPTS
            || !prerequisite_complete(&record, index)
        {
            continue;
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
        let run_id = Uuid::new_v4().to_string();
        record = store::update(storage, id, |current| {
            if current.status == Status::Stopped {
                return Err("The guide was stopped. Saved sections remain available.".into());
            }
            current.status = Status::Building;
            current.active_run_id = Some(run_id.clone());
            current.units[index].attempts.push(Attempt {
                run_id: run_id.clone(),
                question: question.clone(),
                summary: None,
            });
            Ok(())
        })?;
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
                model_id: "gpt-6-luna".into(),
                effort: "max".into(),
            },
            &record.plan.as_ref().unwrap().units[index].starting_ranges,
            manager,
            jobs.clone(),
            journal,
            &progress,
            Arc::clone(&notify),
        );
        let answer = result.or_else(|error| {
            // Qualification logs are private and opt-in. Production receipts
            // retain fixed failure codes, never arbitrary agent or OS text.
            #[cfg(test)]
            eprintln!("GUIDE_UNIT_FAILURE {run_id}: {error}");
            reading::scoped(storage, &run_id, &record.project_id, &record.snapshot_id)
                .map(|a| a.summary())
                .map_err(|_| error)
        })?;
        for section in &answer.sections {
            adopt(section)?;
        }
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
    }
    record = store::update(storage, id, |current| {
        if current.status != Status::Stopped {
            current.status = if current.units.iter().all(unit_complete) {
                Status::Ready
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
    consent: bool,
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
    let planning_run_id = input.id.clone();
    let record = Record {
        revision: 0,
        schema_version: 1,
        contract: CONTRACT.into(),
        id: input.id,
        project_id: input.project_id,
        snapshot_id: input.snapshot_id,
        manifest_sha256: context::digest(
            &serde_json::to_vec(&manifest).map_err(|_| "The inventory could not be encoded.")?,
        ),
        grant_id: input.grant_id,
        question: input.question,
        planning_run_id,
        plan: None,
        units: vec![],
        status: Status::Planning,
        active_run_id: None,
    };
    let lease =
        store::lease(&storage, &record.id)?.ok_or("This guide already has a native worker.")?;
    store::create(&storage, &record)?;
    let id = record.id.clone();
    store::update(&storage, &id, |current| {
        current.active_run_id = Some(current.planning_run_id.clone());
        Ok(())
    })?;
    run_command(app, storage, id, lease).await
}
async fn run_command(
    app: AppHandle,
    storage: std::path::PathBuf,
    id: String,
    lease: std::fs::File,
) -> Result<GuideSummary, String> {
    tauri::async_runtime::spawn_blocking(move||{
        let _lease=lease;
        let manager=app.state::<codex_agent::CodexAgentManager>().inner().clone();
        let jobs=app.state::<RepositoryContextJobs>().inner().clone();
        let journal=app.state::<recovery::InterruptionJournal>().inner().clone();
        let emitter=app.clone();
        let result=build(&storage,&id,&manager,jobs,&journal,|p|{let _=app.emit("codex-agent-progress",p);},
            Arc::new(move|section|{let _=emitter.emit("workspace-answer-section",section);}),
            |section|{if saved_section(&storage, section)? { return Ok(()); } let lock=app.state::<crate::ChapterStoreLock>();let _guard=lock.0.lock().map_err(|_|"Chapter storage is unavailable.")?;
                reading::adopt(&storage,&section.run_id,&section.project_id,&section.snapshot_id,&section.generation_id)?;Ok(())},
            |guide|{let _=app.emit("repository-guide-changed",guide);});
        if result.is_err(){let _=store::update(&storage,&id,|record|{if record.status!=Status::Stopped{record.status=Status::Partial;}Ok(())});}
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
) -> Result<GuideSummary, String> {
    sessions.get(&project_id)?;
    if !consent {
        return Err("Allow continuation before building more guide content.".into());
    }
    let storage = app_data_directory(&app)?;
    let record = scoped(&storage, &id, &project_id, &snapshot_id)?;
    let lease = store::lease(&storage, &id)?
        .ok_or("This guide is already being built. No second worker was started.")?;
    if record.plan.is_none() {
        return Err(
            "This guide has no saved outline. Start a new guide; no request was replayed.".into(),
        );
    }
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
    store::update(&storage, &id, |current| {
        if current.revision != expected_revision {
            return Err("The guide changed while continuation was being prepared. Inspect its saved progress before continuing.".into());
        }
        current.status = Status::Building;
        Ok(())
    })?;
    run_command(app, storage, id, lease).await
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
    if !store::exists(&storage, &id)? {
        manager.cancel(&id);
        return Ok(());
    }
    let existing = store::read(&storage, &id)?;
    if existing.project_id != project_id || existing.snapshot_id != snapshot_id {
        return Err("This guide belongs to another repository snapshot.".into());
    }
    if let Some(run) = &existing.active_run_id {
        manager.cancel(run);
    }
    store::update(&storage, &id, |record| {
        record.status = Status::Stopped;
        Ok(())
    })?;
    Ok(())
}
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn list_repository_guides(
    app: AppHandle,
    manager: State<'_, codex_agent::CodexAgentManager>,
    project_id: String,
    snapshot_id: String,
) -> Result<Vec<GuideSummary>, String> {
    let storage = app_data_directory(&app)?;
    let manager = manager.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        list_saved_guides(&storage, &project_id, &snapshot_id, &manager)
    })
    .await
    .map_err(|_| "Saved guides could not be read. Nothing was regenerated.".to_owned())?
}

fn list_saved_guides(
    storage: &Path,
    project_id: &str,
    snapshot_id: &str,
    manager: &codex_agent::CodexAgentManager,
) -> Result<Vec<GuideSummary>, String> {
    let records = store::list(storage, project_id, snapshot_id)?;
    records
        .into_iter()
        .map(|record| {
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
        })
        .collect()
}

#[cfg(test)]
mod tests;
