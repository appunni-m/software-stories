//! Native Rust/Tauri application backend for Code Atlas.
//!
//! This crate exposes [`run`] as the desktop entry point used by `src/main.rs`.
//! Chapter records, validation, and remapping are private implementation
//! details shared by the native command layer. Their persisted JSON format is
//! described in `docs/CHAPTER_SCHEMA.md`. This crate is built as part of the
//! desktop app and does not promise a separately versioned Rust SDK.

#![deny(missing_docs)]

mod bounded_file;
mod bounded_process;
mod chapter_freshness;
mod claim_alignment;
mod chapter_generation;
mod chapter_review;
mod chapter_store;
mod chapters;
mod scene_presentation;
mod agent_handoff;
mod agent_installation;
mod claude_agent;
mod claude_commands;
mod claude_connection;
mod claude_connections;
mod claude_discovery;
mod claude_generation;
mod claude_models;
mod claude_runtime;
mod claude_worker;
mod codex_agent;
mod codex_identity;
mod codex_commands;
mod context_mcp;
mod chapter_quality;
mod generation_timing;
mod local_export;
mod agent_context;
mod evidence_ledger;
mod native_citations;
mod git_evidence;
mod index;
mod narration;
mod pending_chapter_previews;
mod pocket_tts;
mod pr_guide;
mod pr_guide_store;
mod project;
mod provider;
mod provider_api;
mod recovery;
mod recovery_commands;
mod repository_context;
mod repository_context_commands;
mod repository_structure;
mod repository_structure_commands;
mod repository_plan;
mod planned_chapter;
mod chapter_origin;
mod repository_plan_commands;
mod repository_plan_history;
mod repository_plan_store;
mod snapshot;
mod source_discovery;
mod strict_json;
mod workspace;
mod workspace_generation;

use project::{ProjectOpenResult, RecentProject};
use serde::Serialize;
use snapshot::{
    CaptureLimits, CaptureStatus, ProjectSnapshots, SnapshotJobs, SnapshotManifest,
    SnapshotProgress, SnapshotSource, SnapshotStart,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;

#[derive(Clone, Default)]
struct ProjectSessions(Arc<Mutex<HashMap<String, RecentProject>>>);

#[derive(Default)]
struct ChapterStoreLock(Mutex<()>);

impl ProjectSessions {
    fn remember_result(&self, result: &ProjectOpenResult) {
        if let Some(project) = &result.project {
            if let Ok(mut sessions) = self.0.lock() {
                sessions.insert(project.project_id.clone(), project.clone());
            }
        }
    }

    fn get(&self, project_id: &str) -> Result<RecentProject, String> {
        self.0
            .lock()
            .map_err(|_| "The active project is temporarily unavailable.".to_owned())?
            .get(project_id)
            .cloned()
            .ok_or_else(|| "Open this repository again before using its local snapshot.".to_owned())
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplicationStatus {
    product_name: &'static str,
    version: &'static str,
    native: bool,
}

#[tauri::command]
fn get_application_status() -> ApplicationStatus {
    ApplicationStatus {
        product_name: "Code Atlas",
        version: env!("CARGO_PKG_VERSION"),
        native: true,
    }
}

#[tauri::command]
fn get_capture_limits() -> CaptureLimits {
    CaptureLimits::default()
}

fn app_data_directory(app: &AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|error| format!("Code Atlas could not locate its local data folder: {error}"))
}

#[tauri::command(rename_all = "camelCase")]
async fn pick_project_folder(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
) -> Result<ProjectOpenResult, String> {
    let Some(selected_path) = app
        .dialog()
        .file()
        .set_title("Choose a local Git repository")
        .blocking_pick_folder()
    else {
        return Ok(ProjectOpenResult::cancelled());
    };

    let path = match selected_path.into_path() {
        Ok(path) => path,
        Err(error) => {
            return Ok(ProjectOpenResult::invalid_selected_path(format!(
                "The selected folder path could not be read: {error}"
            )));
        }
    };

    let result = match app_data_directory(&app) {
        Ok(storage_dir) => {
            project::inspect_and_record_project(&storage_dir, &path.display().to_string())
        }
        Err(message) => ProjectOpenResult::storage_error(message),
    };
    sessions.remember_result(&result);
    Ok(result)
}

#[tauri::command(rename_all = "camelCase")]
fn open_recent_project(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
) -> ProjectOpenResult {
    let result = match app_data_directory(&app) {
        Ok(storage_dir) => project::open_recent_project(&storage_dir, &project_id),
        Err(message) => ProjectOpenResult::storage_error(message),
    };
    sessions.remember_result(&result);
    result
}

#[tauri::command(rename_all = "camelCase")]
fn list_recent_projects(app: AppHandle) -> Result<Vec<RecentProject>, String> {
    let storage_dir = app_data_directory(&app)?;
    project::list_recent_projects(&storage_dir)
}

#[tauri::command(rename_all = "camelCase")]
fn remove_recent_project(app: AppHandle, project_id: String) -> Result<Vec<RecentProject>, String> {
    let storage_dir = app_data_directory(&app)?;
    project::remove_recent_project(&storage_dir, &project_id)
}

#[tauri::command(rename_all = "camelCase")]
fn list_project_snapshots(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
) -> Result<ProjectSnapshots, String> {
    let _project = sessions.get(&project_id)?;
    snapshot::list_project_snapshots(&app_data_directory(&app)?, &project_id)
}

#[tauri::command(rename_all = "camelCase")]
fn load_snapshot_manifest(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    snapshot_id: String,
) -> Result<SnapshotManifest, String> {
    let _project = sessions.get(&project_id)?;
    snapshot::load_snapshot_manifest(&app_data_directory(&app)?, &project_id, &snapshot_id)
}

#[tauri::command(rename_all = "camelCase")]
fn read_snapshot_source(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    snapshot_id: String,
    relative_path: String,
) -> Result<SnapshotSource, String> {
    let _project = sessions.get(&project_id)?;
    snapshot::read_snapshot_source(
        &app_data_directory(&app)?,
        &project_id,
        &snapshot_id,
        &relative_path,
    )
}

#[tauri::command(rename_all = "camelCase")]
async fn start_snapshot(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    jobs: State<'_, SnapshotJobs>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    project_id: String,
) -> Result<SnapshotStart, String> {
    let project = sessions.get(&project_id)?;
    let storage_dir = app_data_directory(&app)?;
    let progress =
        snapshot::new_capture_progress(&storage_dir, &project, CaptureLimits::default())?;
    let start = SnapshotStart {
        snapshot_id: progress.snapshot_id.clone(),
        parent_snapshot_id: progress.parent_snapshot_id.clone(),
        limits: progress.limits.clone(),
    };
    let cancel = interruptions.begin(
        &storage_dir,
        recovery::new_capture_operation(project_id.clone(), progress.snapshot_id.clone()),
        || jobs.begin(progress.clone()),
    )?;
    let _ = app.emit("snapshot-progress", progress.clone());
    let app_for_task = app.clone();
    let jobs_for_task = (*jobs).clone();
    let interruptions_for_task = (*interruptions).clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut latest_progress = progress.clone();
        let mut last_emitted_at = Instant::now();
        let mut last_emitted_files = 0_usize;
        let mut last_emitted_phase = progress.phase.clone();
        let mut publish = |update: SnapshotProgress| {
            latest_progress = update.clone();
            jobs_for_task.update(update.clone());
            let should_emit = update.status != CaptureStatus::Running
                || update.phase != last_emitted_phase
                || update.files_captured.saturating_sub(last_emitted_files) >= 32
                || last_emitted_at.elapsed() >= Duration::from_millis(120);
            if should_emit {
                last_emitted_at = Instant::now();
                last_emitted_files = update.files_captured;
                last_emitted_phase = update.phase.clone();
                let _ = app_for_task.emit("snapshot-progress", update);
            }
        };
        let result = snapshot::capture_snapshot_with_progress(
            &storage_dir,
            &project,
            progress.clone(),
            &cancel,
            &mut publish,
        );
        drop(publish);
        if let Err(error) = result {
            let (status, message) = match &error {
                snapshot::CaptureError::Cancelled => (
                    CaptureStatus::Cancelled,
                    "Capture cancelled. No snapshot was added to history.".to_owned(),
                ),
                snapshot::CaptureError::Inconsistent { message, path } => (
                    CaptureStatus::Inconsistent,
                    path.as_ref()
                        .map(|path| format!("{message} Path: {path}"))
                        .unwrap_or_else(|| message.clone()),
                ),
                snapshot::CaptureError::Failed(message) => (CaptureStatus::Failed, message.clone()),
            };
            let mut terminal = latest_progress;
            terminal.status = status;
            terminal.message = Some(message);
            terminal.current_path = None;
            jobs_for_task.update(terminal.clone());
            let _ = app_for_task.emit("snapshot-progress", terminal);
        }
        if let Err(error) = interruptions_for_task.discard(&storage_dir, &progress.snapshot_id) {
            eprintln!("Capture finished but its recovery notice could not be cleared: {error}");
        }
    });
    Ok(start)
}

#[tauri::command]
fn list_interrupted_operations(
    app: AppHandle,
    interruptions: State<'_, recovery::InterruptionJournal>,
) -> Result<Vec<recovery::InterruptedOperation>, String> {
    interruptions.list(&app_data_directory(&app)?)
}

#[tauri::command(rename_all = "camelCase")]
fn discard_interrupted_operation(
    app: AppHandle,
    interruptions: State<'_, recovery::InterruptionJournal>,
    operation_id: String,
) -> Result<(), String> {
    let storage_dir = app_data_directory(&app)?;
    if let Some(operation) = interruptions
        .list(&storage_dir)?
        .into_iter()
        .find(|operation| operation.operation_id == operation_id)
    {
        if matches!(&operation.kind, recovery::InterruptedOperationKind::Capture) {
            snapshot::cleanup_interrupted_staging(&storage_dir, &operation.snapshot_id)?;
        }
    }
    interruptions.discard(&storage_dir, &operation_id)
}

#[tauri::command(rename_all = "camelCase")]
fn cancel_snapshot(jobs: State<'_, SnapshotJobs>, snapshot_id: String) -> Result<(), String> {
    jobs.cancel(&snapshot_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_snapshot_progress(
    jobs: State<'_, SnapshotJobs>,
    snapshot_id: String,
) -> Option<SnapshotProgress> {
    jobs.get(&snapshot_id)
}

#[tauri::command(rename_all = "camelCase")]
fn start_snapshot_indexing(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    jobs: State<'_, index::SnapshotIndexJobs>,
    project_id: String,
    snapshot_id: String,
) -> Result<index::IndexStart, String> {
    let _project = sessions.get(&project_id)?;
    let storage_dir = app_data_directory(&app)?;
    let manifest = snapshot::load_snapshot_manifest(&storage_dir, &project_id, &snapshot_id)?;
    let (initial_progress, cancellation) =
        jobs.begin(&project_id, &snapshot_id, manifest.files.len())?;
    let start = index::IndexStart {
        job_id: initial_progress.job_id.clone(),
        project_id: project_id.clone(),
        snapshot_id: snapshot_id.clone(),
        status: index::IndexJobStatus::Running,
    };
    let _ = app.emit("index-progress", initial_progress.clone());
    let app_for_task = app.clone();
    let jobs_for_task = (*jobs).clone();
    let job_id = initial_progress.job_id;
    tauri::async_runtime::spawn_blocking(move || {
        let mut last_emitted_at = Instant::now();
        let mut last_emitted_files = 0_usize;
        let result = index::build_snapshot_index(
            &storage_dir,
            &manifest,
            &job_id,
            &cancellation,
            |update| {
                jobs_for_task.update(update.clone());
                let should_emit = update.status != index::IndexJobStatus::Running
                    || update.files_processed.saturating_sub(last_emitted_files) >= 32
                    || last_emitted_at.elapsed() >= Duration::from_millis(120);
                if should_emit {
                    last_emitted_at = Instant::now();
                    last_emitted_files = update.files_processed;
                    let _ = app_for_task.emit("index-progress", update.clone());
                }
            },
        );
        let save_error = if cancellation.load(std::sync::atomic::Ordering::Relaxed)
            || result.status == index::SnapshotIndexStatus::Cancelled
        {
            None
        } else {
            index::save_snapshot_index(&storage_dir, &result).err()
        };
        let Some(mut final_progress) = jobs_for_task.get(&job_id) else {
            return;
        };
        final_progress.current_path = None;
        if let Some(error) = save_error {
            final_progress.status = index::IndexJobStatus::Failed;
            final_progress.message = Some(error);
        } else if result.status == index::SnapshotIndexStatus::Cancelled
            || cancellation.load(std::sync::atomic::Ordering::Relaxed)
        {
            final_progress.status = index::IndexJobStatus::Cancelled;
            final_progress.message =
                Some("Indexing was cancelled. No partial index was saved.".to_owned());
        } else if result.status == index::SnapshotIndexStatus::Partial {
            final_progress.status = index::IndexJobStatus::Partial;
            final_progress.message = Some("Index complete with unsupported files, parse gaps, or bounded work omitted. Unresolved edges remain listed with their reasons.".to_owned());
        } else {
            final_progress.status = index::IndexJobStatus::Complete;
            final_progress.message = Some(
                "Index complete for every captured file supported by the shipped parsers."
                    .to_owned(),
            );
        }
        final_progress.files_processed = result.files.len();
        final_progress.files_indexed = result
            .files
            .iter()
            .filter(|file| {
                matches!(
                    file.status,
                    index::IndexFileStatus::Indexed | index::IndexFileStatus::PartialParse
                )
            })
            .count();
        final_progress.symbols_found = result.symbols.len();
        final_progress.relationships_found = result.relationships.len();
        jobs_for_task.update(final_progress.clone());
        let _ = app_for_task.emit("index-progress", final_progress);
    });
    Ok(start)
}

#[tauri::command(rename_all = "camelCase")]
fn cancel_snapshot_indexing(
    jobs: State<'_, index::SnapshotIndexJobs>,
    job_id: String,
) -> Result<(), String> {
    jobs.cancel(&job_id)
}

#[tauri::command(rename_all = "camelCase")]
fn get_snapshot_index_progress(
    jobs: State<'_, index::SnapshotIndexJobs>,
    job_id: String,
) -> Option<index::IndexProgress> {
    jobs.get(&job_id)
}

#[tauri::command(rename_all = "camelCase")]
fn load_snapshot_index(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    snapshot_id: String,
) -> Result<Option<index::SnapshotIndex>, String> {
    let _project = sessions.get(&project_id)?;
    let storage_dir = app_data_directory(&app)?;
    snapshot::load_snapshot_manifest(&storage_dir, &project_id, &snapshot_id)?;
    index::load_snapshot_index(&storage_dir, &project_id, &snapshot_id)
}

#[tauri::command(rename_all = "camelCase")]
fn validate_chapter_revision(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    revision_json: String,
) -> Result<chapters::ChapterValidationReport, String> {
    let _project = sessions.get(&project_id)?;
    let revision = chapters::decode_chapter_revision(revision_json.as_bytes())?;
    let storage_dir = app_data_directory(&app)?;
    let manifest =
        snapshot::load_snapshot_manifest(&storage_dir, &project_id, &revision.snapshot_id)?;
    let snapshot_index =
        index::load_snapshot_index(&storage_dir, &project_id, &revision.snapshot_id)?;
    Ok(chapters::validate_chapter_revision(
        &storage_dir,
        &manifest,
        snapshot_index.as_ref(),
        &revision,
    ))
}

#[tauri::command(rename_all = "camelCase")]
fn list_chapter_revisions(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    snapshot_id: String,
) -> Result<Vec<chapters::ChapterRevision>, String> {
    let _project = sessions.get(&project_id)?;
    let storage_dir = app_data_directory(&app)?;
    snapshot::load_snapshot_manifest(&storage_dir, &project_id, &snapshot_id)?;
    chapter_store::list_for_snapshot(&storage_dir, &project_id, &snapshot_id)
}

#[tauri::command(rename_all = "camelCase")]
fn list_project_chapter_revisions(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
) -> Result<Vec<chapters::ChapterRevision>, String> {
    let _project = sessions.get(&project_id)?;
    chapter_store::list_for_project(&app_data_directory(&app)?, &project_id)
}

#[tauri::command(rename_all = "camelCase")]
fn save_chapter_edits(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    write_lock: State<'_, ChapterStoreLock>,
    project_id: String,
    base_revision_id: String,
    revision_json: String,
) -> Result<chapter_review::SavedChapterReview, String> {
    let _project = sessions.get(&project_id)?;
    let proposal = chapters::decode_chapter_revision(revision_json.as_bytes())?;
    chapter_review::save_edits(
        &app_data_directory(&app)?,
        &write_lock.0,
        &project_id,
        &base_revision_id,
        proposal,
    )
}

#[tauri::command(rename_all = "camelCase")]
fn restore_chapter_revision(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    write_lock: State<'_, ChapterStoreLock>,
    project_id: String,
    base_revision_id: String,
    source_revision_id: String,
) -> Result<chapter_review::SavedChapterReview, String> {
    let _project = sessions.get(&project_id)?;
    chapter_review::restore_revision(
        &app_data_directory(&app)?,
        &write_lock.0,
        &project_id,
        &base_revision_id,
        &source_revision_id,
    )
}

#[tauri::command(rename_all = "camelCase")]
fn accept_chapter_draft(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    write_lock: State<'_, ChapterStoreLock>,
    project_id: String,
    revision_id: String,
) -> Result<chapter_review::SavedChapterReview, String> {
    let _project = sessions.get(&project_id)?;
    chapter_review::accept_draft(
        &app_data_directory(&app)?,
        &write_lock.0,
        &project_id,
        &revision_id,
    )
}

#[tauri::command(rename_all = "camelCase")]
fn reject_chapter_draft(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    write_lock: State<'_, ChapterStoreLock>,
    project_id: String,
    revision_id: String,
    reason: String,
) -> Result<chapters::ChapterRevision, String> {
    let _project = sessions.get(&project_id)?;
    chapter_review::reject_draft(
        &app_data_directory(&app)?,
        &write_lock.0,
        &project_id,
        &revision_id,
        &reason,
    )
}

#[tauri::command(rename_all = "camelCase")]
fn save_generated_chapter_draft(
    app: AppHandle,
    drafts: State<'_, chapters::PendingChapterDrafts>,
    write_lock: State<'_, ChapterStoreLock>,
    generation_id: String,
) -> Result<chapters::ChapterRevision, String> {
    let preview = pending_chapter_previews::load(&app_data_directory(&app)?, &generation_id)?;
    drafts.restore(&generation_id, preview.revision.clone())?;
    let storage_dir = app_data_directory(&app)?;
    save_pending_chapter_draft(&storage_dir, &drafts, &write_lock.0, &generation_id)
}

#[tauri::command(rename_all = "camelCase")]
fn list_pending_generated_chapter_previews(
    app: AppHandle,
) -> Result<Vec<pending_chapter_previews::PendingChapterPreviewSummary>, String> {
    pending_chapter_previews::list(&app_data_directory(&app)?)
}

#[tauri::command(rename_all = "camelCase")]
fn load_pending_generated_chapter_preview(
    app: AppHandle,
    drafts: State<'_, chapters::PendingChapterDrafts>,
    generation_id: String,
) -> Result<chapters::ChapterDraftPreview, String> {
    let storage_dir = app_data_directory(&app)?;
    let mut preview = pending_chapter_previews::load(&storage_dir, &generation_id)?;
    let manifest = snapshot::load_snapshot_manifest(
        &storage_dir,
        &preview.revision.project_id,
        &preview.revision.snapshot_id,
    )?;
    let snapshot_index = index::load_snapshot_index(
        &storage_dir,
        &preview.revision.project_id,
        &preview.revision.snapshot_id,
    )
    .ok()
    .flatten();
    let report = chapters::validate_chapter_revision(
        &storage_dir,
        &manifest,
        snapshot_index.as_ref(),
        &preview.revision,
    );
    if report.status != chapters::ValidationReportStatus::Valid {
        let detail = report
            .issues
            .first()
            .map(|issue| issue.message.as_str())
            .unwrap_or("The chapter failed local source validation.");
        return Err(format!(
            "This recovered preview no longer passes local evidence validation: {detail} The saved recovery copy remains available to discard."
        ));
    }
    preview.validation = report;
    drafts.restore(&generation_id, preview.revision.clone())?;
    Ok(preview)
}

fn save_pending_chapter_draft(
    storage_dir: &std::path::Path,
    drafts: &chapters::PendingChapterDrafts,
    write_lock: &Mutex<()>,
    generation_id: &str,
) -> Result<chapters::ChapterRevision, String> {
    let _guard = write_lock
        .lock()
        .map_err(|_| "The local chapter store is temporarily unavailable.".to_owned())?;
    let preview = pending_chapter_previews::load(storage_dir, generation_id)?;
    let revision = preview.revision;
    let manifest = snapshot::load_snapshot_manifest(
        &storage_dir,
        &revision.project_id,
        &revision.snapshot_id,
    )?;
    let snapshot_index =
        index::load_snapshot_index(&storage_dir, &revision.project_id, &revision.snapshot_id)?;
    let report = chapters::validate_chapter_revision(
        &storage_dir,
        &manifest,
        snapshot_index.as_ref(),
        &revision,
    );
    if report.status != chapters::ValidationReportStatus::Valid {
        let detail = report
            .issues
            .first()
            .map(|issue| issue.message.as_str())
            .unwrap_or("The chapter failed local source validation.");
        return Err(format!(
            "This generated draft no longer passes local evidence validation: {detail}"
        ));
    }
    let mut revision = revision;
    revision.validation_status = chapters::ValidationStatus::Valid;
    let saved = if revision.based_on_revision_id.is_some() {
        chapter_store::append_review_revision(storage_dir, revision)?
    } else {
        chapter_store::save_new_draft(storage_dir, revision)?
    };
    pending_chapter_previews::remove(storage_dir, generation_id)?;
    drafts.remove(generation_id);
    Ok(saved)
}

#[tauri::command(rename_all = "camelCase")]
fn discard_generated_chapter_draft(
    app: AppHandle,
    drafts: State<'_, chapters::PendingChapterDrafts>,
    generation_id: String,
) -> Result<(), String> {
    pending_chapter_previews::remove(&app_data_directory(&app)?, &generation_id)?;
    drafts.remove(&generation_id);
    Ok(())
}

#[tauri::command(rename_all = "camelCase")]
fn remap_chapter_anchor(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    target_snapshot_id: String,
    anchor_json: String,
) -> Result<chapters::AnchorRemapResult, String> {
    let _project = sessions.get(&project_id)?;
    let anchor = chapters::decode_chapter_anchor(anchor_json.as_bytes())?;
    let storage_dir = app_data_directory(&app)?;
    let old_manifest =
        snapshot::load_snapshot_manifest(&storage_dir, &project_id, &anchor.snapshot_id)?;
    let new_manifest =
        snapshot::load_snapshot_manifest(&storage_dir, &project_id, &target_snapshot_id)?;
    let old_index = index::load_snapshot_index(&storage_dir, &project_id, &anchor.snapshot_id)?;
    let new_index = index::load_snapshot_index(&storage_dir, &project_id, &target_snapshot_id)?;
    Ok(chapters::remap_anchor_between_snapshots(
        &storage_dir,
        &old_manifest,
        old_index.as_ref(),
        &new_manifest,
        new_index.as_ref(),
        &anchor,
    ))
}

#[tauri::command(rename_all = "camelCase")]
fn assess_chapter_freshness(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    revision_id: String,
    target_snapshot_id: String,
) -> Result<chapter_freshness::ChapterFreshnessReport, String> {
    let _project = sessions.get(&project_id)?;
    let storage_dir = app_data_directory(&app)?;
    let revision = chapter_store::list_for_project(&storage_dir, &project_id)?
        .into_iter()
        .find(|revision| revision.revision_id == revision_id)
        .ok_or_else(|| "The selected chapter revision is no longer available.".to_owned())?;
    if revision.lifecycle != chapters::RevisionLifecycle::Accepted {
        return Err("Freshness can only be assessed for an accepted chapter revision.".to_owned());
    }
    let from_manifest =
        snapshot::load_snapshot_manifest(&storage_dir, &project_id, &revision.snapshot_id)?;
    let to_manifest =
        snapshot::load_snapshot_manifest(&storage_dir, &project_id, &target_snapshot_id)?;
    let from_index = index::load_snapshot_index(&storage_dir, &project_id, &revision.snapshot_id)?;
    let to_index = index::load_snapshot_index(&storage_dir, &project_id, &target_snapshot_id)?;
    chapter_freshness::classify_chapter_freshness(
        &storage_dir,
        &from_manifest,
        from_index.as_ref(),
        &to_manifest,
        to_index.as_ref(),
        &revision,
    )
}

#[tauri::command(rename_all = "camelCase")]
fn compare_local_git_revisions(
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    base_ref: String,
    head_ref: String,
) -> git_evidence::GitComparisonResult {
    let project = match sessions.get(&project_id) {
        Ok(project) => project,
        Err(message) => return git_evidence::GitComparisonResult::error(message),
    };
    git_evidence::compare_local_revisions(Path::new(&project.repository_root), &base_ref, &head_ref)
}

#[tauri::command]
fn get_provider_catalog() -> Vec<provider::ProviderDisclosure> {
    provider::provider_catalog()
}

#[tauri::command]
fn get_narration_disclosure() -> narration::NarrationDisclosure {
    narration::disclosure()
}

#[tauri::command]
fn get_local_narration_disclosure() -> narration::NarrationDisclosure {
    narration::local_disclosure()
}

#[tauri::command]
fn get_pocket_tts_status(app: AppHandle) -> Result<pocket_tts::PocketTtsStatus, String> {
    Ok(pocket_tts::status(&app_data_directory(&app)?))
}

#[tauri::command(rename_all = "camelCase")]
async fn setup_pocket_tts(
    app: AppHandle,
    cancellations: State<'_, provider::ProviderRequestCancellations>,
    request_id: String,
) -> Result<provider::ProviderCommandResult<pocket_tts::PocketTtsStatus>, String> {
    uuid::Uuid::parse_str(&request_id)
        .map_err(|_| "The setup operation ID is invalid.".to_owned())?;
    let storage_dir = app_data_directory(&app)?;
    let mut cancellation = match cancellations.begin(&request_id) {
        Ok(receiver) => receiver,
        Err(error) => return Ok(provider::ProviderCommandResult::failed(error)),
    };
    let result = pocket_tts::setup(&storage_dir, &mut cancellation).await;
    cancellations.finish(&request_id);
    Ok(match result {
        Ok(status) => provider::ProviderCommandResult::ok(status),
        Err(error) => provider::ProviderCommandResult::failed(error),
    })
}

#[tauri::command(rename_all = "camelCase")]
fn build_local_narration_request_review(
    app: AppHandle,
    reviews: State<'_, narration::PendingNarrationReviews>,
    input: narration::BuildNarrationReviewInput,
) -> Result<provider::ProviderCommandResult<narration::NarrationRequestReview>, String> {
    Ok(narration::build_local_request_review(
        &app_data_directory(&app)?,
        &reviews,
        input,
    ))
}

#[tauri::command(rename_all = "camelCase")]
fn build_narration_request_review(
    app: AppHandle,
    reviews: State<'_, narration::PendingNarrationReviews>,
    input: narration::BuildNarrationReviewInput,
) -> provider::ProviderCommandResult<narration::NarrationRequestReview> {
    let storage_dir = match app_data_directory(&app) {
        Ok(path) => path,
        Err(_) => {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::LocalStorageUnavailable,
                "Code Atlas local audio storage is unavailable. No narration request was sent.",
            );
        }
    };
    narration::build_request_review(&storage_dir, &reviews, input)
}

#[tauri::command(rename_all = "camelCase")]
fn discard_narration_request_review(
    reviews: State<'_, narration::PendingNarrationReviews>,
    review_id: String,
) {
    narration::discard_request_review(&reviews, &review_id);
}

#[tauri::command(rename_all = "camelCase")]
fn load_reviewed_narration_audio(
    app: AppHandle,
    reviews: State<'_, narration::PendingNarrationReviews>,
    review_id: String,
    fingerprint: String,
) -> provider::ProviderCommandResult<narration::NarrationAudio> {
    let storage_dir = match app_data_directory(&app) {
        Ok(path) => path,
        Err(_) => {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::LocalStorageUnavailable,
                "Code Atlas local audio storage is unavailable. The transcript remains available.",
            );
        }
    };
    narration::load_reviewed_audio(&storage_dir, &reviews, &review_id, &fingerprint)
}

#[tauri::command(rename_all = "camelCase")]
async fn synthesize_reviewed_narration_audio(
    app: AppHandle,
    reviews: State<'_, narration::PendingNarrationReviews>,
    cancellations: State<'_, provider::ProviderRequestCancellations>,
    review_id: String,
    fingerprint: String,
    consent: narration::NarrationConsent,
) -> Result<provider::ProviderCommandResult<narration::NarrationAudio>, String> {
    let storage_dir = match app_data_directory(&app) {
        Ok(path) => path,
        Err(_) => {
            return Ok(provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::LocalStorageUnavailable,
                "Code Atlas local audio storage is unavailable. No narration request was sent.",
            ));
        }
    };
    let mut cancellation = match cancellations.begin(&review_id) {
        Ok(cancellation) => cancellation,
        Err(error) => return Ok(provider::ProviderCommandResult::failed(error)),
    };
    let result = narration::synthesize_reviewed_audio(
        &storage_dir,
        &reviews,
        &review_id,
        &fingerprint,
        &consent,
        &mut cancellation,
    )
    .await;
    cancellations.finish(&review_id);
    Ok(result)
}

#[tauri::command]
fn get_provider_credential_status(
    provider_id: String,
) -> provider::ProviderCommandResult<provider::ProviderCredentialStatus> {
    provider::credential_status_for(&provider_id)
}

#[tauri::command(rename_all = "camelCase")]
fn store_provider_api_key(
    provider_id: String,
    api_key: String,
) -> provider::ProviderCommandResult<provider::ProviderCredentialStatus> {
    provider::store_api_key_for(&provider_id, api_key)
}

#[tauri::command(rename_all = "camelCase")]
fn remove_provider_api_key(
    provider_id: String,
) -> provider::ProviderCommandResult<provider::ProviderCredentialStatus> {
    provider::remove_api_key_for(&provider_id)
}

#[tauri::command(rename_all = "camelCase")]
fn build_provider_request_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    provider_id: String,
    input: provider::BuildProviderReviewInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    if sessions.get(&input.project_id).is_err() {
        return provider::ProviderCommandResult::error(
            provider::ProviderFailureKind::InvalidReview,
            "Reopen the selected repository before building a provider request review.",
        );
    }
    let pending = match provider::build_request_review_for_provider(
        &match app_data_directory(&app) {
            Ok(path) => path,
            Err(_) => {
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::LocalStorageUnavailable,
                    "Code Atlas local snapshot storage is unavailable. No request was sent.",
                );
            }
        },
        &input,
        &provider_id,
    ) {
        Ok(pending) => pending,
        Err(error) => return provider::ProviderCommandResult::failed(error),
    };
    match reviews.insert(pending) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    }
}

#[tauri::command(rename_all = "camelCase")]
fn build_code_lens_provider_request_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    provider_id: String,
    input: provider::BuildCodeLensReviewInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    if sessions.get(&input.project_id).is_err() {
        return provider::ProviderCommandResult::error(
            provider::ProviderFailureKind::InvalidReview,
            "Reopen the selected repository before building a Code Lens request review.",
        );
    }
    let storage_dir = match app_data_directory(&app) {
        Ok(path) => path,
        Err(_) => {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::LocalStorageUnavailable,
                "Code Atlas local snapshot storage is unavailable. No request was sent.",
            );
        }
    };
    let pending = match provider::build_code_lens_request_review_for_provider(
        &storage_dir,
        &input,
        &provider_id,
    ) {
        Ok(pending) => pending,
        Err(error) => return provider::ProviderCommandResult::failed(error),
    };
    match reviews.insert(pending) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    }
}

#[tauri::command(rename_all = "camelCase")]
fn build_provider_pr_guide_request_review(
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    provider_id: String,
    input: pr_guide::BuildReviewInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    let project = match sessions.get(&input.project_id) {
        Ok(project) => project,
        Err(message) => {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::InvalidReview,
                message,
            );
        }
    };
    let pending = match provider::build_pr_guide_request_review_for_provider(
        Path::new(&project.repository_root),
        &input,
        &provider_id,
    ) {
        Ok(pending) => pending,
        Err(error) => return provider::ProviderCommandResult::failed(error),
    };
    match reviews.insert(pending) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    }
}

#[tauri::command(rename_all = "camelCase")]
fn build_provider_pr_change_explanation_request_review(
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    provider_id: String,
    input: provider::BuildPrChangeExplanationReviewInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    let project = match sessions.get(&input.project_id) {
        Ok(project) => project,
        Err(message) => {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::InvalidReview,
                message,
            );
        }
    };
    let pending = match provider::build_pr_change_explanation_request_review_for_provider(
        Path::new(&project.repository_root),
        &input,
        &provider_id,
    ) {
        Ok(pending) => pending,
        Err(error) => return provider::ProviderCommandResult::failed(error),
    };
    match reviews.insert(pending) {
        Ok(review) => provider::ProviderCommandResult::ok(review),
        Err(error) => provider::ProviderCommandResult::failed(error),
    }
}

#[tauri::command(rename_all = "camelCase")]
fn discard_provider_request_review(
    reviews: State<'_, provider::PendingProviderReviews>,
    review_id: String,
) {
    reviews.remove(&review_id);
}

#[tauri::command(rename_all = "camelCase")]
fn cancel_provider_request(
    cancellations: State<'_, provider::ProviderRequestCancellations>,
    review_id: String,
) -> bool {
    cancellations.cancel(&review_id)
}

#[tauri::command(rename_all = "camelCase")]
async fn send_provider_request(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    cancellations: State<'_, provider::ProviderRequestCancellations>,
    drafts: State<'_, chapters::PendingChapterDrafts>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    review_id: String,
    fingerprint: String,
    consent: provider::ProviderConsent,
) -> Result<provider::ProviderCommandResult<chapters::ChapterDraftPreview>, String> {
    let interruptions = (*interruptions).clone();
    let result = async move {
        if !consent.is_complete() {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::ConsentRequired,
                "Confirm the privacy/data-use, billing uncertainty, and adult/supported-region statements before sending.",
            );
        }
        let pending = match reviews.get(&review_id) {
            Ok(pending) => pending,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        if let Err(error) = provider::require_direct_api_operation(
            &pending.review,
            provider::DirectApiOperation::Chapter,
        ) {
            return provider::ProviderCommandResult::failed(error);
        }
        if pending.review.fingerprint != fingerprint {
            reviews.remove(&review_id);
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::ReviewChanged,
                "The reviewed request fingerprint changed. Build and review the exact context again.",
            );
        }
        if sessions.get(&pending.review.project_id).is_err() {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::InvalidReview,
                "Reopen the repository and review the request again before sending.",
            );
        }
        let storage_dir = match app_data_directory(&app) {
            Ok(path) => path,
            Err(_) => {
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::LocalStorageUnavailable,
                    "Code Atlas local snapshot storage is unavailable. No request was sent.",
                );
            }
        };
        if let Err(error) = provider::revalidate_request_review(&storage_dir, &pending.review) {
            reviews.remove(&review_id);
            return provider::ProviderCommandResult::failed(error);
        }
        let api_key = match provider::load_api_key_for(&pending.review.provider_id) {
            Ok(api_key) => api_key,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let pending = match reviews.consume(&review_id, &fingerprint) {
            Ok(pending) => pending,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let mut cancellation = match cancellations.begin(&review_id) {
            Ok(cancellation) => cancellation,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let result = match interruptions
            .begin_async(
                &storage_dir,
                recovery::new_reviewed_provider_operation(&pending.review),
                || {
                    provider::await_cancellable_request(
                        &mut cancellation,
                        provider::dispatch_reviewed_request_via_rig(
                            &pending,
                            api_key,
                            &consent,
                        ),
                    )
                },
            )
            .await
        {
            Ok(result) => result,
            Err(error) => {
                cancellations.finish(&review_id);
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::LocalStorageUnavailable,
                    format!("Local recovery state could not be saved, so no provider request was sent: {error}"),
                );
            }
        };
        cancellations.finish(&review_id);
        let command_result = match result {
            Ok(result) => match prepare_chapter_draft_preview(&storage_dir, &pending.review, result, &drafts) {
                Ok(preview) => provider::ProviderCommandResult::ok(preview),
                Err(error) => provider::ProviderCommandResult::failed(error),
            },
            Err(error) => provider::ProviderCommandResult::failed(error),
        };
        if let Err(error) = interruptions.discard(&storage_dir, &review_id) {
            let preview_was_saved = matches!(
                &command_result,
                provider::ProviderCommandResult::Ok { .. }
            );
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::LocalStorageUnavailable,
                if preview_was_saved {
                    format!("The request finished and its validated preview was saved locally, but the interruption notice could not be cleared: {error} Review the saved preview locally or clear the notice. A new provider request still requires fresh review and consent.")
                } else {
                    format!("The request finished, but its local recovery notice could not be cleared: {error} Review a new exact-context request with fresh consent before trying again.")
                },
            );
        }
        command_result
    }
    .await;
    Ok(result)
}

#[tauri::command(rename_all = "camelCase")]
async fn send_code_lens_provider_request(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    cancellations: State<'_, provider::ProviderRequestCancellations>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    review_id: String,
    fingerprint: String,
    consent: provider::ProviderConsent,
) -> Result<provider::ProviderCommandResult<provider::CodeLensExplanationPreview>, String> {
    let interruptions = (*interruptions).clone();
    let result = async move {
        if !consent.is_complete() {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::ConsentRequired,
                "Confirm the privacy/data-use, billing uncertainty, and adult/supported-region statements before sending.",
            );
        }
        let pending = match reviews.get(&review_id) {
            Ok(pending) => pending,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        if let Err(error) = provider::require_direct_api_operation(
            &pending.review,
            provider::DirectApiOperation::CodeLens,
        ) {
            return provider::ProviderCommandResult::failed(error);
        }
        if pending.review.purpose != "codeLensExplanation"
            || pending.review.fingerprint != fingerprint
        {
            reviews.remove(&review_id);
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::ReviewChanged,
                "The reviewed Code Lens request fingerprint changed. Build and review the exact source range again.",
            );
        }
        if sessions.get(&pending.review.project_id).is_err() {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::InvalidReview,
                "Reopen the repository and review the Code Lens request again before sending.",
            );
        }
        let storage_dir = match app_data_directory(&app) {
            Ok(path) => path,
            Err(_) => {
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::LocalStorageUnavailable,
                    "Code Atlas local snapshot storage is unavailable. No request was sent.",
                );
            }
        };
        if let Err(error) = provider::revalidate_request_review(&storage_dir, &pending.review) {
            reviews.remove(&review_id);
            return provider::ProviderCommandResult::failed(error);
        }
        let api_key = match provider::load_api_key_for(&pending.review.provider_id) {
            Ok(api_key) => api_key,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let pending = match reviews.consume(&review_id, &fingerprint) {
            Ok(pending) => pending,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let mut cancellation = match cancellations.begin(&review_id) {
            Ok(cancellation) => cancellation,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let dispatched = match interruptions
            .begin_async(
                &storage_dir,
                recovery::new_reviewed_provider_operation(&pending.review),
                || {
                    provider::await_cancellable_request(
                        &mut cancellation,
                        provider::dispatch_reviewed_request_via_rig(
                            &pending,
                            api_key,
                            &consent,
                        ),
                    )
                },
            )
            .await
        {
            Ok(result) => result,
            Err(error) => {
                cancellations.finish(&review_id);
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::LocalStorageUnavailable,
                    format!("Local recovery state could not be saved, so no provider request was sent: {error}"),
                );
            }
        };
        cancellations.finish(&review_id);
        let command_result = match dispatched {
            Ok(result) => match provider::canonicalize_code_lens_explanation(
                &storage_dir,
                &pending.review,
                &result,
            ) {
                Ok(preview) => provider::ProviderCommandResult::ok(preview),
                Err(error) => provider::ProviderCommandResult::failed(error),
            },
            Err(error) => provider::ProviderCommandResult::failed(error),
        };
        if let Err(error) = interruptions.discard(&storage_dir, &review_id) {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::LocalStorageUnavailable,
                format!("The request finished, but its local recovery notice could not be cleared: {error} Review a new exact-range request with fresh consent before trying again."),
            );
        }
        command_result
    }
    .await;
    Ok(result)
}

#[tauri::command(rename_all = "camelCase")]
async fn send_provider_pr_change_explanation_request(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    cancellations: State<'_, provider::ProviderRequestCancellations>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    review_id: String,
    fingerprint: String,
    consent: provider::ProviderConsent,
) -> Result<provider::ProviderCommandResult<provider::PrChangeExplanationPreview>, String> {
    let interruptions = (*interruptions).clone();
    let result = async move {
        if !consent.is_complete() {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::ConsentRequired,
                "Confirm the privacy/data-use, billing uncertainty, and adult/supported-region statements before sending.",
            );
        }
        let pending = match reviews.get(&review_id) {
            Ok(pending) => pending,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        if let Err(error) = provider::require_direct_api_operation(
            &pending.review,
            provider::DirectApiOperation::PrChange,
        ) {
            return provider::ProviderCommandResult::failed(error);
        }
        if pending.review.purpose != "prChangeExplanation"
            || pending.review.fingerprint != fingerprint
        {
            reviews.remove(&review_id);
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::ReviewChanged,
                "The reviewed PR change fingerprint changed. Build and review the exact hunk again.",
            );
        }
        let project = match sessions.get(&pending.review.project_id) {
            Ok(project) => project,
            Err(message) => {
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::InvalidReview,
                    message,
                );
            }
        };
        let repository = Path::new(&project.repository_root);
        if let Err(error) =
            provider::revalidate_pr_change_explanation_request(repository, &pending.review)
        {
            reviews.remove(&review_id);
            return provider::ProviderCommandResult::failed(error);
        }
        let storage_dir = match app_data_directory(&app) {
            Ok(path) => path,
            Err(_) => {
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::LocalStorageUnavailable,
                    "Code Atlas local recovery storage is unavailable. No request was sent.",
                );
            }
        };
        let api_key = match provider::load_api_key_for(&pending.review.provider_id) {
            Ok(api_key) => api_key,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let pending = match reviews.consume(&review_id, &fingerprint) {
            Ok(pending) => pending,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let mut cancellation = match cancellations.begin(&review_id) {
            Ok(cancellation) => cancellation,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let dispatched = match interruptions
            .begin_async(
                &storage_dir,
                recovery::new_reviewed_provider_operation(&pending.review),
                || {
                    provider::await_cancellable_request(
                        &mut cancellation,
                        provider::dispatch_reviewed_request_via_rig(
                            &pending,
                            api_key,
                            &consent,
                        ),
                    )
                },
            )
            .await
        {
            Ok(result) => result,
            Err(error) => {
                cancellations.finish(&review_id);
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::LocalStorageUnavailable,
                    format!("Local recovery state could not be saved, so no provider request was sent: {error}"),
                );
            }
        };
        cancellations.finish(&review_id);
        let command_result = match dispatched {
            Ok(response) => match provider::canonicalize_pr_change_explanation(
                repository,
                &pending.review,
                &response,
            ) {
                Ok(preview) => provider::ProviderCommandResult::ok(preview),
                Err(error) => provider::ProviderCommandResult::failed(error),
            },
            Err(error) => provider::ProviderCommandResult::failed(error),
        };
        if let Err(error) = interruptions.discard(&storage_dir, &review_id) {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::LocalStorageUnavailable,
                format!("The provider request finished, but its local interruption notice could not be cleared: {error}. Build a new hunk review before trying again."),
            );
        }
        command_result
    }
    .await;
    Ok(result)
}

#[tauri::command(rename_all = "camelCase")]
async fn send_provider_pr_guide_request(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    cancellations: State<'_, provider::ProviderRequestCancellations>,
    interruptions: State<'_, recovery::InterruptionJournal>,
    review_id: String,
    fingerprint: String,
    consent: provider::ProviderConsent,
) -> Result<provider::ProviderCommandResult<pr_guide::PrGuidePreview>, String> {
    let interruptions = (*interruptions).clone();
    let result = async move {
        if !consent.is_complete() {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::ConsentRequired,
                "Confirm the privacy/data-use, billing uncertainty, and adult/supported-region statements before sending.",
            );
        }
        let pending = match reviews.get(&review_id) {
            Ok(pending) => pending,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        if let Err(error) = provider::require_direct_api_operation(
            &pending.review,
            provider::DirectApiOperation::PrGuide,
        ) {
            return provider::ProviderCommandResult::failed(error);
        }
        if pending.review.fingerprint != fingerprint {
            reviews.remove(&review_id);
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::ReviewChanged,
                "The reviewed request fingerprint changed. Build and review the exact Git evidence again.",
            );
        }
        let project = match sessions.get(&pending.review.project_id) {
            Ok(project) => project,
            Err(message) => {
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::InvalidReview,
                    message,
                );
            }
        };
        let storage_dir = match app_data_directory(&app) {
            Ok(path) => path,
            Err(_) => {
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::LocalStorageUnavailable,
                    "Code Atlas local PR Guide storage is unavailable. No request was sent.",
                );
            }
        };
        if let Err(error) = provider::revalidate_pr_guide_request(
            Path::new(&project.repository_root),
            &pending.review,
        ) {
            reviews.remove(&review_id);
            return provider::ProviderCommandResult::failed(error);
        }
        let api_key = match provider::load_api_key_for(&pending.review.provider_id) {
            Ok(api_key) => api_key,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let pending = match reviews.consume(&review_id, &fingerprint) {
            Ok(pending) => pending,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let mut cancellation = match cancellations.begin(&review_id) {
            Ok(cancellation) => cancellation,
            Err(error) => return provider::ProviderCommandResult::failed(error),
        };
        let result = match interruptions
            .begin_async(
                &storage_dir,
                recovery::new_reviewed_provider_operation(&pending.review),
                || {
                    provider::await_cancellable_request(
                        &mut cancellation,
                        provider::dispatch_reviewed_request_via_rig(
                            &pending,
                            api_key,
                            &consent,
                        ),
                    )
                },
            )
            .await
        {
            Ok(result) => result,
            Err(error) => {
                cancellations.finish(&review_id);
                return provider::ProviderCommandResult::error(
                    provider::ProviderFailureKind::LocalStorageUnavailable,
                    format!("Local recovery state could not be saved, so no provider request was sent: {error}"),
                );
            }
        };
        cancellations.finish(&review_id);
        let command_result = match result {
            Ok(result) => match prepare_pr_guide_preview(&storage_dir, &pending.review, result) {
                Ok(preview) => provider::ProviderCommandResult::ok(preview),
                Err(error) => provider::ProviderCommandResult::failed(error),
            },
            Err(error) => provider::ProviderCommandResult::failed(error),
        };
        if let Err(error) = interruptions.discard(&storage_dir, &review_id) {
            return provider::ProviderCommandResult::error(
                provider::ProviderFailureKind::LocalStorageUnavailable,
                format!("The provider request finished, but its local interruption notice could not be cleared: {error}. Check PR Guide history before sending another request."),
            );
        }
        command_result
    }
    .await;
    Ok(result)
}

/// A validated in-memory candidate; publication remains a separate decision.
struct PreparedPrGuidePreview {
    preview: pr_guide::PrGuidePreview,
}

fn prepare_pr_guide_preview(
    storage_dir: &Path,
    review: &provider::ProviderRequestReview,
    result: provider::ProviderSendResult,
) -> Result<pr_guide::PrGuidePreview, provider::ProviderFailure> {
    publish_pr_guide_candidate(
        storage_dir,
        validate_pr_guide_preview(review, result)?,
        PreviewPublication::Ordinary,
    )
}

fn publish_pr_guide_candidate(
    storage_dir: &Path,
    candidate: PreparedPrGuidePreview,
    publication: PreviewPublication,
) -> Result<pr_guide::PrGuidePreview, provider::ProviderFailure> {
    let mut preview = candidate.preview;
    preview.revision = match publication {
        PreviewPublication::Ordinary => pr_guide_store::save_draft(storage_dir, preview.revision),
        PreviewPublication::AgentOwned => pr_guide_store::try_save_draft(storage_dir, preview.revision),
    }.map_err(|message| provider::ProviderFailure {
        kind: provider::ProviderFailureKind::LocalStorageUnavailable,
        message: format!("The PR Guide passed local evidence validation, but its separate history could not be saved: {message}"),
    })?;
    Ok(preview)
}

fn validate_pr_guide_preview(
    review: &provider::ProviderRequestReview,
    result: provider::ProviderSendResult,
) -> Result<PreparedPrGuidePreview, provider::ProviderFailure> {
    let failure = |kind, message: String| provider::ProviderFailure { kind, message };
    if result.review_id != review.review_id
        || result.fingerprint != review.fingerprint
        || result.model_id != review.model_id
    {
        return Err(failure(
            provider::ProviderFailureKind::InvalidReview,
            "The returned guide does not match the reviewed request identity. No PR Guide was saved.".to_owned(),
        ));
    }
    let context = review.pr_guide_context.clone().ok_or_else(|| {
        failure(
            provider::ProviderFailureKind::InvalidReview,
            "The reviewed request has no immutable Git comparison. No PR Guide was saved."
                .to_owned(),
        )
    })?;
    let revision =
        pr_guide::canonicalize_response(&review.project_id, context, &result.response_text)
            .map_err(|message| {
                failure(
                    provider::ProviderFailureKind::MalformedProviderResponse,
                    message,
                )
            })?;
    Ok(PreparedPrGuidePreview {
        preview: pr_guide::PrGuidePreview {
            revision,
            model_id: result.model_id,
            model_version: result.model_version,
            input_tokens: result.input_tokens,
            output_tokens: result.output_tokens,
        },
    })
}

#[tauri::command(rename_all = "camelCase")]
fn list_pr_guide_revisions(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    comparison_id: String,
) -> Result<Vec<pr_guide::PrGuideRevision>, String> {
    let _project = sessions.get(&project_id)?;
    let storage_dir = app_data_directory(&app)?;
    pr_guide_store::list_for_comparison(&storage_dir, &project_id, &comparison_id)
}

#[tauri::command(rename_all = "camelCase")]
fn accept_pr_guide_draft(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    comparison_id: String,
    revision_id: String,
    included_claim_ids: Vec<String>,
) -> Result<pr_guide::PrGuideRevision, String> {
    let _project = sessions.get(&project_id)?;
    let storage_dir = app_data_directory(&app)?;
    pr_guide_store::accept_draft(
        &storage_dir,
        &project_id,
        &comparison_id,
        &revision_id,
        &included_claim_ids,
    )
}

#[tauri::command(rename_all = "camelCase")]
fn reject_pr_guide_draft(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    project_id: String,
    comparison_id: String,
    revision_id: String,
    reason: String,
) -> Result<pr_guide::PrGuideRevision, String> {
    let _project = sessions.get(&project_id)?;
    let storage_dir = app_data_directory(&app)?;
    pr_guide_store::reject_draft(
        &storage_dir,
        &project_id,
        &comparison_id,
        &revision_id,
        &reason,
    )
}

/// Agent publication already holds its cancellation owner; never queue
/// behind a local result-store lock while holding that ownership lock.
#[derive(Clone, Copy)]
enum PreviewPublication {
    Ordinary,
    AgentOwned,
}

struct PreparedChapterDraftPreview {
    revision: chapters::ChapterRevision,
    validation: chapters::ChapterValidationReport,
    model_id: String,
    model_version: Option<String>,
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    update_notes: Vec<String>,
}

/// Only native agent result closures call this after independent chapter
/// validation. Direct API and manual imports cannot create agent lineage.
fn attach_agent_generation_origin(
    candidate: &mut PreparedChapterDraftPreview,
    review: &provider::ProviderRequestReview,
) -> Result<(), provider::ProviderFailure> {
    let origin =
        chapter_origin::ChapterGenerationOrigin::from_agent_review(review, &candidate.revision)
            .map_err(|message| provider::ProviderFailure {
                kind: provider::ProviderFailureKind::InvalidReview,
                message,
            })?;
    candidate.revision.generation_origin = Some(origin);
    Ok(())
}

fn prepare_chapter_draft_preview(
    storage_dir: &Path,
    review: &provider::ProviderRequestReview,
    result: provider::ProviderSendResult,
    drafts: &chapters::PendingChapterDrafts,
) -> Result<chapters::ChapterDraftPreview, provider::ProviderFailure> {
    let candidate = validate_chapter_draft_preview(storage_dir, review, result)?;
    publish_chapter_candidate(storage_dir, candidate, drafts, PreviewPublication::Ordinary)
}

fn publish_chapter_candidate(
    storage_dir: &Path,
    candidate: PreparedChapterDraftPreview,
    drafts: &chapters::PendingChapterDrafts,
    publication: PreviewPublication,
) -> Result<chapters::ChapterDraftPreview, provider::ProviderFailure> {
    let failure = |kind, message: String| provider::ProviderFailure { kind, message };
    let PreparedChapterDraftPreview {
        revision,
        validation,
        model_id,
        model_version,
        input_tokens,
        output_tokens,
        update_notes,
    } = candidate;
    let pending_revision = revision.clone();
    let preview = |generation_id| chapters::ChapterDraftPreview {
        generation_id,
        revision,
        validation,
        model_id,
        model_version,
        input_tokens,
        output_tokens,
        update_notes,
    };
    if matches!(publication, PreviewPublication::AgentOwned) {
        // No memory insertion or eviction occurs unless durable publication
        // succeeds. Uncertain file publication remains discoverable on disk.
        // Both local locks use try_lock under the agent cancellation owner.
        return drafts.try_publish(pending_revision, |generation_id| {
            let preview = preview(generation_id);
            pending_chapter_previews::try_save(storage_dir, &preview)?;
            Ok(preview)
        }).map_err(|message| failure(provider::ProviderFailureKind::LocalStorageUnavailable,
            format!("The generated chapter passed local validation, but preview publication could not be confirmed: {message} Inspect pending previews and recovery before another request; a recovery copy may already exist.")));
    }
    let generation_id = drafts
        .insert(pending_revision)
        .map_err(|message| failure(provider::ProviderFailureKind::ReviewUnavailable, message))?;
    let preview = preview(generation_id);
    if let Err(message) = pending_chapter_previews::save(storage_dir, &preview) {
        drafts.remove(&preview.generation_id);
        return Err(failure(
            provider::ProviderFailureKind::LocalStorageUnavailable,
            format!(
                "The generated chapter passed local validation, but its recovery copy could not be persisted: {message}"
            ),
        ));
    }
    Ok(preview)
}

fn validate_chapter_draft_preview(
    storage_dir: &std::path::Path,
    review: &provider::ProviderRequestReview,
    result: provider::ProviderSendResult,
) -> Result<PreparedChapterDraftPreview, provider::ProviderFailure> {
    let failure = |kind, message: String| provider::ProviderFailure { kind, message };
    if result.review_id != review.review_id
        || result.fingerprint != review.fingerprint
        || result.model_id != review.model_id
    {
        return Err(failure(provider::ProviderFailureKind::InvalidReview, "The returned chapter does not match the reviewed request identity. No draft was created.".to_owned()));
    }
    let manifest = snapshot::load_snapshot_manifest(storage_dir, &review.project_id, &review.snapshot_id)
        .map_err(|_| failure(provider::ProviderFailureKind::InvalidReview, "The exact source snapshot became unavailable. The provider returned text, but no chapter draft was created.".to_owned()))?;
    let snapshot_index =
        index::load_snapshot_index(storage_dir, &review.project_id, &review.snapshot_id)
            .ok()
            .flatten();
    let sources = review.selected_files.iter().map(|selected| {
        snapshot::read_snapshot_source(storage_dir, &review.project_id, &review.snapshot_id, &selected.path)
            .map_err(|_| failure(provider::ProviderFailureKind::InvalidReview, format!("Selected source '{}' could not be re-read from the immutable snapshot. No draft was created.", selected.path)))
            .and_then(|source| {
                if source.sha256 != selected.sha256 || source.contents.len() as u64 != selected.size_bytes {
                    Err(failure(provider::ProviderFailureKind::InvalidReview, "A selected source no longer matches the reviewed fingerprint. No draft was created.".to_owned()))
                } else { Ok(source) }
            })
    }).collect::<Result<Vec<_>, _>>()?;
    let allowed_relationships = review
        .selected_relationships
        .iter()
        .map(|relationship| relationship.relationship_id.clone())
        .collect::<std::collections::HashSet<_>>();
    let (mut revision, update_notes) = match (review.purpose.as_str(), review.target_path.as_deref()) {
        ("fileChapter", Some(target_path)) => chapter_generation::canonicalize_file_chapter(
            &review.project_id,
            &manifest,
            snapshot_index.as_ref(),
            target_path,
            &sources,
            &allowed_relationships,
            &result.response_text,
        ).map(|revision| (revision, Vec::new())),
        ("repositoryOverview", None) if review.provider_id == "codex"
            && review.generation_config.get("overviewPresentation").and_then(serde_json::Value::as_str) == Some("anchored-claims-v1") => chapter_generation::canonicalize_workspace_overview(
            &review.project_id,
            &manifest,
            snapshot_index.as_ref(),
            &sources,
            &allowed_relationships,
            &result.response_text,
        ).map(|revision| (revision, Vec::new())),
        ("repositoryOverview", None) => chapter_generation::canonicalize_repository_overview(
            &review.project_id,
            &manifest,
            snapshot_index.as_ref(),
            &sources,
            &allowed_relationships,
            &result.response_text,
        ).map(|revision| (revision, Vec::new())),
        ("sectionUpdate", Some(_)) => {
            let context = review.section_update_context.as_ref().ok_or_else(|| failure(
                provider::ProviderFailureKind::InvalidReview,
                "The one-section request is missing its selected chapter identity.".to_owned(),
            ))?;
            let history = chapter_store::list_for_project(storage_dir, &review.project_id)
                .map_err(|message| failure(provider::ProviderFailureKind::ReviewUnavailable, message))?;
            let base = history.iter().find(|revision| revision.revision_id == context.chapter_revision_id)
                .ok_or_else(|| failure(provider::ProviderFailureKind::ReviewUnavailable, "The accepted chapter revision is no longer available. No draft was created.".to_owned()))?;
            chapter_generation::canonicalize_section_update(
                storage_dir,
                base,
                &manifest,
                snapshot_index.as_ref(),
                &sources,
                &context.section_id,
                &context.section_revision_id,
                &allowed_relationships,
                &result.response_text,
            )
        },
        _ => Err("The reviewed request purpose and target do not match.".to_owned()),
    }
    .map_err(|message| {
        failure(
            provider::ProviderFailureKind::MalformedProviderResponse,
            message,
        )
    })?;
    provider::visual_scenes::prepare_generated_sections(
        &mut revision,
        review.section_update_context.as_ref().map(|context| context.section_id.as_str()),
    ).map_err(|message| failure(provider::ProviderFailureKind::MalformedProviderResponse, message))?;
    let validation = chapters::validate_chapter_revision(
        storage_dir,
        &manifest,
        snapshot_index.as_ref(),
        &revision,
    );
    if validation.status != chapters::ValidationReportStatus::Valid {
        let detail = validation
            .issues
            .first()
            .map(|issue| issue.message.clone())
            .unwrap_or_else(|| "The chapter failed local citation validation.".to_owned());
        return Err(failure(
            provider::ProviderFailureKind::MalformedProviderResponse,
            format!(
                "The generated chapter failed local source validation: {detail} No draft was saved."
            ),
        ));
    }
    revision.validation_status = chapters::ValidationStatus::Valid;
    Ok(PreparedChapterDraftPreview {
        revision,
        validation,
        model_id: result.model_id,
        model_version: result.model_version,
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
        update_notes,
    })
}

/// Runs the private MCP stdio entry point when this process was launched as a
/// context helper. Returns false for the ordinary desktop invocation.
pub fn run_context_helper() -> bool { context_mcp::run_if_requested() }

/// Runs the bounded, offline benchmark validator without starting the desktop UI.
pub fn run_quality_helper() -> bool { chapter_quality::run_if_requested() }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Starts the Code Atlas Tauri application and its registered command layer.
///
/// This function initializes the native plugins and in-memory command state,
/// then starts the Tauri event loop.
///
/// # Panics
///
/// Panics if the application cannot start.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(ProjectSessions::default())
        .manage(codex_agent::CodexAgentManager::default())
        .manage(claude_worker::ClaudeWorkerManager::default())
        .manage(claude_connections::ClaudeConnections::default())
        .manage(SnapshotJobs::default())
        .manage(index::SnapshotIndexJobs::default())
        .manage(repository_context_commands::RepositoryContextJobs::default())
        .manage(provider::PendingProviderReviews::default())
        .manage(provider::ProviderRequestCancellations::default())
        .manage(narration::PendingNarrationReviews::default())
        .manage(chapters::PendingChapterDrafts::default())
        .manage(source_discovery::DiscoveryReviews::default())
        .manage(recovery::InterruptionJournal::default())
        .manage(ChapterStoreLock::default())
        .invoke_handler(tauri::generate_handler![
            agent_installation::pick_agent_installation_file,
            agent_handoff::build_agent_handoff_review,
            agent_handoff::export_agent_handoff,
            agent_handoff::import_agent_handoff_result,
            workspace::authorize_workspace_exploration,
            workspace::revoke_workspace_exploration,
            workspace::list_workspace_commits,
            repository_context_commands::search_workspace_source,
            repository_context_commands::search_repository_code,
            repository_context_commands::read_repository_range,
            repository_context_commands::cancel_repository_context,
            repository_structure_commands::inspect_repository_structure,
            source_discovery::prepare_source_discovery,
            source_discovery::get_documentation_notices,
            source_discovery::discard_source_discovery,
            source_discovery::send_source_discovery,
            codex_commands::get_codex_agent_status,
            codex_commands::resume_codex_connection,
            codex_commands::configure_codex_agent,
            codex_commands::start_codex_login,
            codex_commands::cancel_codex_login,
            codex_commands::logout_codex_agent,
            codex_commands::list_codex_models,
            codex_commands::build_codex_chapter_review,
            codex_commands::build_codex_planned_chapter_review,
            codex_commands::build_codex_repository_plan_review,
            codex_commands::build_codex_code_lens_review,
            codex_commands::build_codex_pr_guide_review,
            codex_commands::build_codex_pr_change_review,
            codex_commands::send_codex_code_lens_request,
            codex_commands::send_codex_chapter_request,
            codex_commands::send_codex_repository_plan_request,
            codex_commands::send_codex_pr_guide_request,
            codex_commands::send_codex_pr_change_request,
            codex_commands::cancel_codex_chapter_request,
            workspace_generation::generate_workspace_answer,
            workspace_generation::progressive::generate_progressive_workspace_answer,
            workspace_generation::progressive::load_workspace_answer,
            workspace_generation::progressive::save_workspace_answer_section,
            workspace_generation::progressive::list_workspace_answers,
            workspace_generation::progressive::load_workspace_answer_timing,
            workspace_generation::progressive::save_workspace_answer_timing,
            local_export::save_offline_guide,
            workspace_generation::progressive::list_workspace_answer_records,
            workspace_generation::progressive::remove_workspace_answer_record,
            claude_commands::inspect_claude_agent_connection,
            claude_commands::inspect_claude_agent_model_catalog,
            claude_commands::cancel_claude_agent_operation,
            claude_commands::forget_claude_agent_connection,
            claude_commands::build_claude_chapter_review,
            claude_commands::build_claude_planned_chapter_review,
            claude_commands::build_claude_repository_plan_review,
            claude_commands::build_claude_code_lens_review,
            claude_commands::build_claude_pr_guide_review,
            claude_commands::build_claude_pr_change_review,
            claude_commands::send_claude_chapter_request,
            claude_commands::send_claude_repository_plan_request,
            claude_commands::send_claude_code_lens_request,
            claude_commands::send_claude_pr_guide_request,
            claude_commands::send_claude_pr_change_request,
            repository_plan_commands::prepare_repository_plan_context,
            repository_plan_commands::prepare_planned_chapter_scope,
            repository_plan_commands::list_repository_plan_drafts,
            repository_plan_commands::load_repository_plan_draft,
            repository_plan_commands::discard_repository_plan_draft,
            repository_plan_commands::list_repository_plan_revisions,
            repository_plan_commands::load_repository_plan_revision,
            repository_plan_commands::accept_repository_plan_draft,
            repository_plan_commands::resolve_repository_plan_notice,
            get_application_status,
            get_capture_limits,
            pick_project_folder,
            open_recent_project,
            list_recent_projects,
            remove_recent_project,
            list_project_snapshots,
            load_snapshot_manifest,
            read_snapshot_source,
            start_snapshot,
            list_interrupted_operations,
            recovery_commands::load_interrupted_generation_scope,
            discard_interrupted_operation,
            cancel_snapshot,
            get_snapshot_progress,
            start_snapshot_indexing,
            cancel_snapshot_indexing,
            get_snapshot_index_progress,
            load_snapshot_index,
            validate_chapter_revision,
            list_chapter_revisions,
            list_project_chapter_revisions,
            save_chapter_edits,
            restore_chapter_revision,
            accept_chapter_draft,
            reject_chapter_draft,
            save_generated_chapter_draft,
            list_pending_generated_chapter_previews,
            load_pending_generated_chapter_preview,
            discard_generated_chapter_draft,
            remap_chapter_anchor,
            assess_chapter_freshness,
            compare_local_git_revisions,
            list_pr_guide_revisions,
            accept_pr_guide_draft,
            reject_pr_guide_draft,
            get_provider_catalog,
            get_narration_disclosure,
            get_local_narration_disclosure,
            get_pocket_tts_status,
            setup_pocket_tts,
            build_local_narration_request_review,
            build_narration_request_review,
            discard_narration_request_review,
            load_reviewed_narration_audio,
            synthesize_reviewed_narration_audio,
            get_provider_credential_status,
            store_provider_api_key,
            remove_provider_api_key,
            build_provider_request_review,
            build_code_lens_provider_request_review,
            build_provider_pr_guide_request_review,
            build_provider_pr_change_explanation_request_review,
            discard_provider_request_review,
            cancel_provider_request,
            send_provider_request,
            send_code_lens_provider_request,
            send_provider_pr_guide_request,
            send_provider_pr_change_explanation_request
        ])
        .build(tauri::generate_context!())
        .expect("failed to build Code Atlas")
        .run(|app, event| {
            if matches!(event, tauri::RunEvent::Exit) {
                app.state::<codex_agent::CodexAgentManager>().shutdown();
                app.state::<claude_worker::ClaudeWorkerManager>().shutdown();
                pocket_tts::shutdown();
            }
        });
}

#[cfg(test)]
mod chapter_pipeline_tests {
    use super::*;
    use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, simple_proposal};

    #[test]
    fn fresh_file_and_overview_previews_validate_scenes_and_keep_complete_narration() {
        let fixture = Fixture::new();
        for overview in [false, true] {
            let review = provider::build_request_review(&fixture.storage,
                &provider::BuildProviderReviewInput {
                    project_id: fixture.manifest.summary.project_id.clone(),
                    snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                    target_path: if overview { None } else { Some(SIMPLE_PATH.to_owned()) },
                    selected_paths: vec![SIMPLE_PATH.to_owned()],
                    model_id: "gemini-3.8-flash".to_owned(), section_update: None,
                }).unwrap().review;
            assert!(review.system_instruction.contains("ai-html-compositions-v3"));
            let mut proposal = provider::visual_scenes::tests::composed_proposal(3);
            if overview { proposal.as_object_mut().unwrap().remove("evidenceLimits"); }
            let original = proposal["sections"][0]["claims"][0]["prose"].as_str().unwrap().to_owned();
            let full = format!("{original} An incoming name becomes part of the returned greeting.");
            proposal["sections"][0]["claims"][0]["prose"] = serde_json::json!(&full);
            let prepared = validate_chapter_draft_preview(
                &fixture.storage, &review, response(&review, proposal.to_string())).unwrap();
            assert_eq!(prepared.revision.sections[0].spoken_text, proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["narration"].as_str().unwrap());
            assert_eq!(prepared.revision.sections[0].prose, full);
            let presentation = prepared.revision.sections[0].visual.as_ref().unwrap()
                .typed_visual().unwrap().presentation.as_ref().unwrap();
            assert_eq!(presentation.schema_version, 3);
            assert_eq!(presentation.scenes.len(), 1);
            assert!(chapter_store::list_for_project(&fixture.storage, &review.project_id).unwrap().is_empty());
            // A historical shape still decodes, but cannot downgrade a fresh result.
            proposal["sections"][0]["visual"]["presentation"]["schemaVersion"] = serde_json::json!(1);
            proposal["sections"][0]["visual"]["presentation"]["scenes"][0].as_object_mut().unwrap().remove("narration");
            let error = validate_chapter_draft_preview(
                &fixture.storage, &review, response(&review, proposal.to_string())).err().unwrap();
            assert_eq!(error.kind, provider::ProviderFailureKind::MalformedProviderResponse);
            assert!(error.message.contains("version 3"));
        }
    }

    #[test]
    #[ignore = "Explicit existing-account GPT-6 Luna/max Fontdone file-scene qualification; consumes quota."]
    fn fontdone_file_scene_account_probe() {
        use crate::codex_agent::{CodexAgentManager, CodexRunInput, CodexSourceFile};
        use std::io::Write;
        let storage = std::path::PathBuf::from(std::env::var_os("CODE_ATLAS_FONTDONE_DATA_DIR")
            .expect("set isolated CODE_ATLAS_FONTDONE_DATA_DIR"));
        let output = std::path::PathBuf::from(std::env::var_os("CODE_ATLAS_FILE_SCENE_OUTPUT")
            .expect("set a fresh absolute CODE_ATLAS_FILE_SCENE_OUTPUT"));
        assert!(storage.is_absolute() && output.is_absolute());
        let mut artifact = std::fs::OpenOptions::new().write(true).create_new(true)
            .open(&output).expect("never overwrite another attempt");
        let project = "/Users/lazytrot/work/fontdone";
        let snapshot = "snap-1792140879637485000-11155-0";
        let path = "src/tt/loca.rs";
        let question = "How does this file turn a glyph ID into its byte range? Explain the short and long offset encodings, missing entries, and the bounds and ordering cases that determine glyph length. Use a concrete illustrative example where it helps.";
        let manager = CodexAgentManager::default();
        assert!(manager.resume_connection(&storage).unwrap().authenticated);
        let runtime = manager.review_runtime(&storage).unwrap();
        let review = provider::build_codex_chapter_review(&storage,
            &provider::BuildCodexReviewInput {
                project_id: project.into(), snapshot_id: snapshot.into(),
                target_path: Some(path.into()), selected_paths: vec![path.into()],
                model_id: "gpt-6-luna".into(), effort: "max".into(), approval_policy: "never".into(),
                section_update: None, question: Some(question.into()), focus: vec![], workspace_grant: None,
            }, &runtime).unwrap().review;
        let source = snapshot::read_snapshot_source(&storage, project, snapshot, path).unwrap();
        let started = std::time::Instant::now();
        let run = manager.run(&storage, CodexRunInput {
            run_id: format!("file-scenes-{}", uuid::Uuid::new_v4()), expected_runtime: runtime,
            model: "gpt-6-luna".into(), effort: "max".into(), approval_policy: "never".into(),
            system_instruction: review.system_instruction.clone(), user_prompt: review.user_prompt.clone(),
            output_schema: review.generation_config["outputSchema"].clone(),
            correction_policy: crate::claim_alignment::CorrectionPolicy::from_generation_config(&review.generation_config), retrieval: None,
            sources: vec![CodexSourceFile { path: source.path.clone(), sha256: source.sha256.clone(), contents: source.contents.clone() }],
        }, |_| {});
        let elapsed = started.elapsed().as_millis();
        manager.shutdown();
        let validated = run.as_ref().map_err(|e| e.clone()).and_then(|reply| {
            if reply.model != "gpt-6-luna" || reply.effort != "max" {
                return Err("The account runtime changed the requested model or effort.".to_owned());
            }
            validate_chapter_draft_preview(&storage, &review,
                provider::ProviderSendResult {
                    review_id: review.review_id.clone(), fingerprint: review.fingerprint.clone(),
                    model_id: reply.model.clone(), model_version: None,
                    response_text: reply.response_text.clone(), input_tokens: None, output_tokens: None,
                }).map_err(|e| e.message)
        });
        let report = serde_json::json!({
            "schemaVersion":1, "scope":"One real Fontdone file; not whole-repository qualification",
            "sourceRevision":std::env::var("CODE_ATLAS_QUALIFICATION_REVISION").ok(),
            "model":"gpt-6-luna", "effort":"max", "elapsedMs":elapsed,
            "source":{"projectId":project,"snapshotId":snapshot,"path":path,"sha256":source.sha256,"bytes":source.contents.len()},
            "request":{"fingerprint":review.fingerprint,"systemInstruction":review.system_instruction,
                "userPrompt":review.user_prompt,"generationConfig":review.generation_config},
            "responseText":run.as_ref().ok().map(|r|&r.response_text),
            "runtimeError":run.as_ref().err(), "validationError":validated.as_ref().err(),
            "revision":validated.as_ref().ok().map(|v|&v.revision),
            "validation":validated.as_ref().ok().map(|v|&v.validation),
            "accepted":false, "humanQualityOrListeningPass":false,
        });
        artifact.write_all(serde_json::to_string_pretty(&report).unwrap().as_bytes()).unwrap();
        artifact.sync_all().unwrap();
        println!("FILE_SCENE_QUALIFICATION {} {}ms", output.display(), elapsed);
        let validated = validated.unwrap();
        assert!(validated.revision.sections.iter().any(|s| s.visual.as_ref()
            .and_then(|v|v.typed_visual()).and_then(|v|v.presentation.as_ref()).is_some()),
            "A scene-free answer does not qualify AI visual generation; retain the original result.");
    }

    fn review(fixture: &Fixture) -> provider::ProviderRequestReview {
        provider::build_request_review(
            &fixture.storage,
            &provider::BuildProviderReviewInput {
                project_id: fixture.manifest.summary.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                target_path: Some(SIMPLE_PATH.to_owned()),
                selected_paths: vec![SIMPLE_PATH.to_owned()],
                model_id: "gemini-3.8-flash".to_owned(),
                section_update: None,
            },
        )
        .unwrap()
        .review
    }

    fn response(
        review: &provider::ProviderRequestReview,
        text: String,
    ) -> provider::ProviderSendResult {
        provider::ProviderSendResult {
            review_id: review.review_id.clone(),
            fingerprint: review.fingerprint.clone(),
            model_id: review.model_id.clone(),
            model_version: Some("fixture-response-no-network".to_owned()),
            response_text: text,
            input_tokens: Some(100),
            output_tokens: Some(50),
        }
    }

    async fn assert_incomplete_consent_stops_section_dispatch(
        request: &provider::PendingProviderRequest,
    ) {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_from_transport = std::sync::Arc::clone(&calls);
        let error = provider::dispatch_reviewed_request(
            request,
            zeroize::Zeroizing::new("fixture-key".to_owned()),
            &provider::ProviderConsent {
                privacy_and_data_use_acknowledged: true,
                billing_uncertainty_acknowledged: false,
                adult_supported_region_acknowledged: true,
            },
            move |_, _, _| async move {
                calls_from_transport.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(provider::ProviderHttpResult {
                    model_version: None,
                    response_text: "{}".to_owned(),
                    input_tokens: None,
                    output_tokens: None,
                })
            },
        )
        .await
        .err()
        .expect("incomplete consent must stop the section request");
        assert_eq!(error.kind, provider::ProviderFailureKind::ConsentRequired);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    async fn assert_review_change_stops_section_dispatch(
        request: &provider::PendingProviderRequest,
    ) {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_from_transport = std::sync::Arc::clone(&calls);
        let error = provider::dispatch_reviewed_request(
            request,
            zeroize::Zeroizing::new("fixture-key".to_owned()),
            &provider::ProviderConsent {
                privacy_and_data_use_acknowledged: true,
                billing_uncertainty_acknowledged: true,
                adult_supported_region_acknowledged: true,
            },
            move |_, _, _| async move {
                calls_from_transport.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                Ok(provider::ProviderHttpResult {
                    model_version: None,
                    response_text: "{}".to_owned(),
                    input_tokens: None,
                    output_tokens: None,
                })
            },
        )
        .await
        .err()
        .expect("a changed section request must be rejected before transport");
        assert_eq!(error.kind, provider::ProviderFailureKind::ReviewChanged);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    }

    async fn assert_section_quota_failure_is_not_retried(
        request: &provider::PendingProviderRequest,
    ) {
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let calls_from_transport = std::sync::Arc::clone(&calls);
        let expected_model = request.review.model_id.clone();
        let expected_body = request.review.request_body_preview.clone();
        let error = provider::dispatch_reviewed_request(
            request,
            zeroize::Zeroizing::new("fixture-key".to_owned()),
            &provider::ProviderConsent {
                privacy_and_data_use_acknowledged: true,
                billing_uncertainty_acknowledged: true,
                adult_supported_region_acknowledged: true,
            },
            move |model_id, _, body| {
                assert_eq!(model_id, expected_model);
                assert_eq!(body, expected_body);
                calls_from_transport.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                async move {
                    Err(provider::ProviderFailure {
                        kind: provider::ProviderFailureKind::QuotaExceeded,
                        message: "fixture quota failure".to_owned(),
                    })
                }
            },
        )
        .await
        .err()
        .expect("the selected section provider's failure must be returned");
        assert_eq!(error.kind, provider::ProviderFailureKind::QuotaExceeded);
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn selected_stale_section_regenerates_without_changing_other_section_or_asset_ids() {
        let mut fixture = Fixture::new();
        let mut proposal = simple_proposal();
        let stable_quote = "overlapping = \"aaaaaaaaa\"";
        let stable_prose = "overlapping stores the repeated literal in one local constant without changing repository state.";
        proposal.sections.push(chapters::GeneratedChapterSection {
            title: "The unrelated overlap marker".to_owned(),
            prose: stable_prose.to_owned(),
            spoken_text:
                "The overlapping constant holds one repeated literal for this local example."
                    .to_owned(),
            claims: vec![chapters::GeneratedChapterClaim {
                category: "codeBehavior".to_owned(),
                aspect: "stateEffect".to_owned(),
                prose: stable_prose.to_owned(),
                inference_basis: String::new(),
                identifiers: vec!["overlapping".to_owned()],
                evidence: vec![chapters::GeneratedChapterEvidence {
                    path: SIMPLE_PATH.to_owned(),
                    quote: stable_quote.to_owned(),
                }],
                relationship_ids: Vec::new(),
            }],
        });
        proposal.evidence_limits = vec![chapters::GeneratedChapterEvidenceLimit {
            prose: "overlapping holds one repeated literal here; this captured source does not show how another module uses it.".to_owned(),
            basis: "Only the selected static source was captured and indexed for this local example.".to_owned(),
            identifiers: vec!["overlapping".to_owned()],
            evidence: vec![chapters::GeneratedChapterEvidence {
                path: SIMPLE_PATH.to_owned(),
                quote: stable_quote.to_owned(),
            }],
        }];
        let relationships = std::collections::HashSet::new();
        let mut base = chapter_generation::canonicalize_file_chapter(
            &fixture.manifest.summary.project_id,
            &fixture.manifest,
            Some(&fixture.index),
            SIMPLE_PATH,
            &fixture.sources(&[SIMPLE_PATH]),
            &relationships,
            &serde_json::to_string(&proposal).unwrap(),
        )
        .unwrap();
        let stable_anchor = base.sections[1].claims[0].anchors[0].anchor_id.clone();
        base.sections[1].spoken_text =
            "The overlapping constant keeps this sample's repeated literal.".to_owned();
        base.sections[1].visual =
            Some(chapters::ChapterVisualPayload::from(chapters::VisualSpec {
                presentation: None,
                schema_version: 2,
                kind: chapters::VisualKind::DataFlow,
                title: "The repeated literal".to_owned(),
                layout_hint: Some(chapters::VisualLayoutHint {
                    direction: chapters::VisualLayoutDirection::LeftToRight,
                }),
                nodes: vec![chapters::VisualNode {
                    node_id: "node:overlapping".to_owned(),
                    label: "overlapping".to_owned(),
                    detail: Some("one module constant".to_owned()),
                    hotspot_id: None,
                    evidence_anchor_ids: vec![stable_anchor],
                }],
                edges: Vec::new(),
            }));
        let initial_validation = chapters::validate_chapter_revision(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            &base,
        );
        assert_eq!(
            initial_validation.status,
            chapters::ValidationReportStatus::Valid,
            "{initial_validation:#?}"
        );
        base.validation_status = chapters::ValidationStatus::Valid;
        let base_draft = chapter_store::save_new_draft(&fixture.storage, base).unwrap();
        let accepted = chapter_review::accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &base_draft.project_id,
            &base_draft.revision_id,
        )
        .unwrap()
        .revision;
        let accepted_bytes = chapters::encode_chapter_revision(&accepted).unwrap();
        let history_before =
            chapter_store::list_for_project(&fixture.storage, &accepted.project_id).unwrap();
        let history_before_bytes = history_before
            .iter()
            .map(chapters::encode_chapter_revision)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let old_selected_section = accepted.sections[0].clone();
        let old_unrelated_section = accepted.sections[1].clone();
        let old_visual_assets = chapter_freshness::asset_ids_for_revision(&accepted)
            .into_iter()
            .filter(|asset| asset.starts_with("visual:"))
            .collect::<Vec<_>>();
        assert_eq!(old_visual_assets.len(), 1);
        let old_unrelated_audio = narration::audio_cache_key_for_script(
            &old_unrelated_section.spoken_text,
            "gemini-tts-test",
            "Kore",
            "plain",
            "wav",
        );

        let multiple_stale_source = chapter_generation::tests::SIMPLE_SOURCE
            .replace(
                "return `Hello, ${name}`;",
                "return `Hello, ${name.trim()}`;",
            )
            .replace(
                "const overlapping = \"aaaaaaaaa\";",
                "const overlapping = \"bbbbbbbbb\";",
            );
        fixture.recapture_with_source(SIMPLE_PATH, &multiple_stale_source);
        let target = provider::SectionUpdateReviewTarget {
            chapter_revision_id: accepted.revision_id.clone(),
            section_id: old_selected_section.section_id.clone(),
            section_revision_id: old_selected_section.section_revision_id.clone(),
        };
        let blocked = provider::build_request_review(
            &fixture.storage,
            &provider::BuildProviderReviewInput {
                project_id: accepted.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                target_path: Some(SIMPLE_PATH.to_owned()),
                selected_paths: vec![SIMPLE_PATH.to_owned()],
                model_id: "gemini-3.8-flash".to_owned(),
                section_update: Some(target.clone()),
            },
        )
        .err()
        .expect("a second stale section must be resolved before preparing a paid request");
        assert!(blocked.message.contains("The unrelated overlap marker"));
        assert!(blocked.message.contains("No request was sent"));

        let current_source = chapter_generation::tests::SIMPLE_SOURCE.replace(
            "return `Hello, ${name}`;",
            "return `Hello, ${name.trim()}`;",
        );
        fixture.recapture_with_source(SIMPLE_PATH, &current_source);
        let freshness = chapter_freshness::classify_chapter_freshness(
            &fixture.storage,
            &snapshot::load_snapshot_manifest(
                &fixture.storage,
                &accepted.project_id,
                &accepted.snapshot_id,
            )
            .unwrap(),
            Some(
                &index::load_snapshot_index(
                    &fixture.storage,
                    &accepted.project_id,
                    &accepted.snapshot_id,
                )
                .unwrap()
                .unwrap(),
            ),
            &fixture.manifest,
            Some(&fixture.index),
            &accepted,
        )
        .unwrap();
        assert_eq!(
            freshness.sections[0].status,
            chapter_freshness::SectionFreshnessStatus::Stale,
            "{freshness:#?}"
        );
        assert_eq!(
            freshness.sections[1].status,
            chapter_freshness::SectionFreshnessStatus::SafeToCarry,
            "{freshness:#?}"
        );

        let codex_runtime = codex_identity::CodexReviewRuntime {
            connection_id: uuid::Uuid::new_v4().to_string(), executable_path: "/usr/local/bin/codex".into(),
            executable_sha256: "a".repeat(64), account_identity_sha256: "b".repeat(64), account_revision: 1,
        };
        let codex_review = provider::build_codex_chapter_review(&fixture.storage,
            &provider::BuildCodexReviewInput {
                project_id:accepted.project_id.clone(), snapshot_id:fixture.manifest.summary.snapshot_id.clone(),
                target_path:Some(SIMPLE_PATH.to_owned()), selected_paths:vec![SIMPLE_PATH.to_owned()],
                model_id:"gpt-6-luna".into(), effort:"max".into(), approval_policy:"never".into(),
                section_update:Some(target.clone()), question:None, focus:vec![], workspace_grant:None,
            }, &codex_runtime).unwrap().review;
        assert_eq!(codex_review.purpose,"sectionUpdate");
        assert_eq!(crate::claim_alignment::CorrectionPolicy::from_generation_config(&codex_review.generation_config),
            crate::claim_alignment::CorrectionPolicy::ChapterScenesV4);
        provider::revalidate_codex_request_review(&fixture.storage,&codex_review,&codex_runtime).unwrap();

        for disclosure in provider::provider_catalog() {
            let model_id = disclosure.models[0].model_id.clone();
            let input = provider::BuildProviderReviewInput {
                project_id: accepted.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                target_path: Some(SIMPLE_PATH.to_owned()),
                selected_paths: vec![SIMPLE_PATH.to_owned()],
                model_id: model_id.clone(),
                section_update: Some(target.clone()),
            };
            assert!(input.section_update.is_some());
            assert!(
                provider::disclosure_for(&disclosure.provider_id)
                    .unwrap()
                    .models
                    .iter()
                    .any(|model| model.model_id == model_id && model.supports_structured_output)
            );
            let provider_request = provider::build_request_review_for_provider(
                &fixture.storage,
                &input,
                &disclosure.provider_id,
            )
            .unwrap_or_else(|error| {
                panic!(
                    "{}/{} section update review: {error:?}",
                    disclosure.provider_id, model_id
                )
            });
            assert_eq!(provider_request.review.purpose, "sectionUpdate");
            assert_eq!(provider_request.review.provider_id, disclosure.provider_id);
            assert_eq!(provider_request.review.model_id, model_id);
            let wire = crate::provider_api::encode_request_preview(
                &provider_request.review.provider_id,
                &provider_request.review.model_id,
                &provider_request.review.system_instruction,
                &provider_request.review.user_prompt,
                &provider_request.review.generation_config,
            )
            .unwrap();
            assert_eq!(provider_request.review.endpoint_url, wire.endpoint_url);
            assert_eq!(provider_request.review.request_body_preview, wire.body);
            assert_eq!(
                provider::revalidate_request_review(&fixture.storage, &provider_request.review)
                    .unwrap()
                    .review
                    .fingerprint,
                provider_request.review.fingerprint
            );

            let dispatch_calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
            let calls_during_dispatch = std::sync::Arc::clone(&dispatch_calls);
            let expected_model = provider_request.review.model_id.clone();
            let expected_body = wire.body;
            let dispatched = provider::dispatch_reviewed_request(
                &provider_request,
                zeroize::Zeroizing::new("fixture-key".to_owned()),
                &provider::ProviderConsent {
                    privacy_and_data_use_acknowledged: true,
                    billing_uncertainty_acknowledged: true,
                    adult_supported_region_acknowledged: true,
                },
                move |model_id, _, body| async move {
                    assert_eq!(model_id, expected_model);
                    assert_eq!(body, expected_body);
                    calls_during_dispatch.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(provider::ProviderHttpResult {
                        model_version: Some(format!("{model_id}-fixture")),
                        response_text: "{}".to_owned(),
                        input_tokens: Some(100),
                        output_tokens: Some(40),
                    })
                },
            )
            .await
            .unwrap_or_else(|error| {
                panic!(
                    "{}/{} section update dispatch: {error:?}",
                    disclosure.provider_id, model_id
                )
            });
            assert_eq!(dispatch_calls.load(std::sync::atomic::Ordering::SeqCst), 1);
            assert_eq!(dispatched.model_id, model_id);
            assert_eq!(dispatched.fingerprint, provider_request.review.fingerprint);
            assert_incomplete_consent_stops_section_dispatch(&provider_request).await;

            let (alternate_provider, alternate_model) = match disclosure.provider_id.as_str() {
                "gemini" => ("openai", "gpt-6-luna"),
                "openai" => ("anthropic", "claude-haiku-5-5"),
                _ => ("gemini", "gemini-3.8-flash"),
            };
            let mut changed_provider = provider_request.clone();
            changed_provider.review.provider_id = alternate_provider.to_owned();
            changed_provider.review.model_id = alternate_model.to_owned();
            let mut changed_model = provider_request.clone();
            changed_model.review.model_id = format!("unlisted-{}-model", disclosure.provider_id);
            let mut changed_endpoint = provider_request.clone();
            changed_endpoint.review.endpoint_url = "https://api.example.invalid/v1".to_owned();
            for changed in [changed_provider, changed_model, changed_endpoint] {
                assert_review_change_stops_section_dispatch(&changed).await;
            }
            assert_section_quota_failure_is_not_retried(&provider_request).await;
            let history_after =
                chapter_store::list_for_project(&fixture.storage, &accepted.project_id).unwrap();
            assert_eq!(history_after, history_before);
            assert_eq!(
                history_after
                    .iter()
                    .map(chapters::encode_chapter_revision)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap(),
                history_before_bytes,
                "a failed selective section request must not change stored chapter history"
            );
        }

        let request = provider::build_request_review(
            &fixture.storage,
            &provider::BuildProviderReviewInput {
                project_id: accepted.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                target_path: Some(SIMPLE_PATH.to_owned()),
                selected_paths: vec![SIMPLE_PATH.to_owned()],
                model_id: "gemini-3.8-flash".to_owned(),
                section_update: Some(target),
            },
        )
        .unwrap();
        assert_eq!(request.review.purpose, "sectionUpdate");
        assert_eq!(
            request
                .review
                .selected_files
                .iter()
                .map(|file| file.path.as_str())
                .collect::<Vec<_>>(),
            vec![SIMPLE_PATH]
        );
        assert!(
            request
                .review
                .user_prompt
                .contains("Return exactly one JSON object")
        );
        assert!(
            !request
                .review
                .user_prompt
                .contains("The unrelated overlap marker")
        );
        assert_eq!(
            provider::revalidate_request_review(&fixture.storage, &request.review)
                .unwrap()
                .review
                .fingerprint,
            request.review.fingerprint,
        );

        let updated_quote =
            "function greeting(name: string): string {\n    return `Hello, ${name.trim()}`;\n}";
        let updated_prose = "greeting trims name before placing it in the Hello template string returned from the function.";
        let generated_section = chapters::GeneratedChapterSection {
            title: "Trim the name before formatting".to_owned(),
            prose: updated_prose.to_owned(),
            spoken_text: "greeting trims the name before returning the Hello string.".to_owned(),
            claims: vec![chapters::GeneratedChapterClaim {
                category: "codeBehavior".to_owned(),
                aspect: "execution".to_owned(),
                prose: updated_prose.to_owned(),
                inference_basis: String::new(),
                identifiers: vec!["greeting".to_owned(), "name".to_owned()],
                evidence: vec![chapters::GeneratedChapterEvidence {
                    path: SIMPLE_PATH.to_owned(),
                    quote: updated_quote.to_owned(),
                }],
                relationship_ids: Vec::new(),
            }],
        };
        let pending = chapters::PendingChapterDrafts::default();
        let preview = prepare_chapter_draft_preview(
            &fixture.storage,
            &request.review,
            response(
                &request.review,
                serde_json::to_string(&generated_section).unwrap(),
            ),
            &pending,
        )
        .unwrap();
        assert_eq!(
            preview.validation.status,
            chapters::ValidationReportStatus::Valid,
            "{:#?}",
            preview.validation
        );
        assert_eq!(
            preview.revision.snapshot_id,
            fixture.manifest.summary.snapshot_id
        );
        assert_eq!(
            preview.revision.based_on_revision_id.as_deref(),
            Some(accepted.revision_id.as_str())
        );
        let next_selected = &preview.revision.sections[0];
        let next_unrelated = &preview.revision.sections[1];
        assert_eq!(next_selected.section_id, old_selected_section.section_id);
        assert_ne!(
            next_selected.section_revision_id,
            old_selected_section.section_revision_id
        );
        assert_eq!(next_unrelated.section_id, old_unrelated_section.section_id);
        assert_eq!(
            next_unrelated.section_revision_id,
            old_unrelated_section.section_revision_id
        );
        assert_eq!(
            next_unrelated.spoken_text,
            old_unrelated_section.spoken_text
        );
        assert_eq!(next_unrelated.visual, old_unrelated_section.visual);
        assert_eq!(
            chapter_freshness::asset_ids_for_revision(&preview.revision)
                .into_iter()
                .filter(|asset| asset.starts_with("visual:"))
                .collect::<Vec<_>>(),
            old_visual_assets,
        );
        assert_eq!(
            narration::audio_cache_key_for_script(
                &next_unrelated.spoken_text,
                "gemini-tts-test",
                "Kore",
                "plain",
                "wav"
            ),
            old_unrelated_audio,
        );
        assert_ne!(
            narration::audio_cache_key_for_script(
                &next_selected.spoken_text,
                "gemini-tts-test",
                "Kore",
                "plain",
                "wav"
            ),
            narration::audio_cache_key_for_script(
                &old_selected_section.spoken_text,
                "gemini-tts-test",
                "Kore",
                "plain",
                "wav"
            ),
        );
        assert_eq!(
            chapters::encode_chapter_revision(&accepted).unwrap(),
            accepted_bytes
        );

        let saved = save_pending_chapter_draft(
            &fixture.storage,
            &pending,
            &Mutex::new(()),
            &preview.generation_id,
        )
        .unwrap();
        assert_eq!(saved.lifecycle, chapters::RevisionLifecycle::Draft);
        assert_eq!(
            saved.based_on_revision_id.as_deref(),
            Some(accepted.revision_id.as_str())
        );
        let new_accepted = chapter_review::accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &saved.project_id,
            &saved.revision_id,
        )
        .unwrap();
        assert_eq!(
            new_accepted.validation.status,
            chapters::ValidationReportStatus::Valid
        );
        assert_eq!(
            chapters::encode_chapter_revision(&accepted).unwrap(),
            accepted_bytes
        );
        let history =
            chapter_store::list_for_project(&fixture.storage, &accepted.project_id).unwrap();
        assert!(history.contains(&accepted));
        assert!(history.contains(&saved));
        assert!(history.contains(&new_accepted.revision));
    }

    #[test]
    fn preview_is_durable_and_only_explicit_save_appends_one_unaccepted_target() {
        let fixture = Fixture::new();
        let review = review(&fixture);
        let pending = chapters::PendingChapterDrafts::default();
        let preview = prepare_chapter_draft_preview(
            &fixture.storage,
            &review,
            response(&review, serde_json::to_string(&simple_proposal()).unwrap()),
            &pending,
        )
        .unwrap();
        assert_eq!(
            preview.validation.status,
            chapters::ValidationReportStatus::Valid
        );
        assert_eq!(
            preview.revision.lifecycle,
            chapters::RevisionLifecycle::Draft
        );
        assert!(
            chapter_store::list_for_project(&fixture.storage, &review.project_id)
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            pending.get(&preview.generation_id).unwrap(),
            preview.revision
        );
        assert_eq!(
            pending_chapter_previews::load(&fixture.storage, &preview.generation_id).unwrap(),
            preview
        );

        let saved = save_pending_chapter_draft(
            &fixture.storage,
            &pending,
            &Mutex::new(()),
            &preview.generation_id,
        )
        .unwrap();
        assert_eq!(saved.lifecycle, chapters::RevisionLifecycle::Draft);
        assert_eq!(
            saved.target.scope,
            chapters::ChapterTargetScope::File {
                path: SIMPLE_PATH.to_owned()
            }
        );
        assert_eq!(
            chapter_store::list_for_snapshot(
                &fixture.storage,
                &review.project_id,
                &review.snapshot_id
            )
            .unwrap(),
            vec![saved.clone()]
        );
        assert!(pending.get(&preview.generation_id).is_err());
        assert!(pending_chapter_previews::load(&fixture.storage, &preview.generation_id).is_err());
        assert!(
            save_pending_chapter_draft(
                &fixture.storage,
                &pending,
                &Mutex::new(()),
                &preview.generation_id
            )
            .is_err()
        );
        assert_eq!(
            chapter_store::list_for_project(&fixture.storage, &review.project_id).unwrap(),
            vec![saved]
        );
    }

    #[test]
    fn repository_overview_preview_uses_the_pending_save_and_review_lifecycle() {
        let fixture = Fixture::new();
        let review = provider::build_request_review(
            &fixture.storage,
            &provider::BuildProviderReviewInput {
                project_id: fixture.manifest.summary.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                target_path: None,
                selected_paths: vec![SIMPLE_PATH.to_owned(), "src/names.ts".to_owned()],
                model_id: "gemini-3.8-flash".to_owned(),
                section_update: None,
            },
        )
        .unwrap()
        .review;
        assert_eq!(review.purpose, "repositoryOverview");
        let mut proposal = serde_json::to_value(simple_proposal()).unwrap();
        proposal.as_object_mut().unwrap().remove("evidenceLimits");
        proposal["title"] = serde_json::json!("Greeting behavior across selected modules");
        proposal["sections"][0]["claims"][0]["category"] = serde_json::json!("inference");
        proposal["sections"][0]["claims"][0]["inferenceBasis"] = serde_json::json!(
            "The overview interprets the captured code structure and exact selected source excerpts."
        );
        let inferred_copy = format!(
            "Inference: {}",
            proposal["sections"][0]["claims"][0]["prose"]
                .as_str()
                .unwrap()
        );
        proposal["sections"][0]["prose"] = serde_json::json!(&inferred_copy);
        proposal["sections"][0]["spokenText"] = serde_json::json!(inferred_copy);
        let pending = chapters::PendingChapterDrafts::default();
        let preview = prepare_chapter_draft_preview(
            &fixture.storage,
            &review,
            response(&review, proposal.to_string()),
            &pending,
        )
        .unwrap();
        assert_eq!(preview.revision.chapter_id, "chapter:overview");
        assert_eq!(preview.revision.target.target_id, "target:overview");
        assert!(
            chapter_store::list_for_project(&fixture.storage, &review.project_id)
                .unwrap()
                .is_empty()
        );
        let saved = save_pending_chapter_draft(
            &fixture.storage,
            &pending,
            &Mutex::new(()),
            &preview.generation_id,
        )
        .unwrap();
        assert_eq!(saved.target.scope, chapters::ChapterTargetScope::Overview);
        assert_eq!(
            chapter_store::list_for_project(&fixture.storage, &review.project_id).unwrap(),
            vec![saved]
        );
        assert!(pending.get(&preview.generation_id).is_err());
    }

    #[test]
    fn adding_an_overview_leaves_accepted_file_history_unchanged() {
        let fixture = Fixture::new();
        let file_review = review(&fixture);
        let file_pending = chapters::PendingChapterDrafts::default();
        let file_preview = prepare_chapter_draft_preview(
            &fixture.storage,
            &file_review,
            response(
                &file_review,
                serde_json::to_string(&simple_proposal()).unwrap(),
            ),
            &file_pending,
        )
        .unwrap();
        let file_draft = save_pending_chapter_draft(
            &fixture.storage,
            &file_pending,
            &Mutex::new(()),
            &file_preview.generation_id,
        )
        .unwrap();
        let accepted_file = chapter_review::accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &file_review.project_id,
            &file_draft.revision_id,
        )
        .unwrap()
        .revision;
        let before = chapter_store::list_for_snapshot(
            &fixture.storage,
            &file_review.project_id,
            &file_review.snapshot_id,
        )
        .unwrap();
        assert_eq!(before, vec![file_draft.clone(), accepted_file.clone()]);
        assert!(before.iter().all(|revision| matches!(
            &revision.target.scope,
            chapters::ChapterTargetScope::File { .. }
        )));
        let file_history_bytes = before
            .iter()
            .map(chapters::encode_chapter_revision)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let overview_review = provider::build_request_review(
            &fixture.storage,
            &provider::BuildProviderReviewInput {
                project_id: fixture.manifest.summary.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                target_path: None,
                selected_paths: vec![SIMPLE_PATH.to_owned(), "src/names.ts".to_owned()],
                model_id: "gemini-3.8-flash".to_owned(),
                section_update: None,
            },
        )
        .unwrap()
        .review;
        let mut proposal = serde_json::to_value(simple_proposal()).unwrap();
        proposal.as_object_mut().unwrap().remove("evidenceLimits");
        proposal["title"] = serde_json::json!("Greeting behavior across selected modules");
        proposal["sections"][0]["claims"][0]["category"] = serde_json::json!("inference");
        proposal["sections"][0]["claims"][0]["inferenceBasis"] = serde_json::json!(
            "This is a structural interpretation of the captured greeting implementation."
        );
        let inferred_copy = format!(
            "Inference: {}",
            proposal["sections"][0]["claims"][0]["prose"]
                .as_str()
                .unwrap()
        );
        proposal["sections"][0]["prose"] = serde_json::json!(&inferred_copy);
        proposal["sections"][0]["spokenText"] = serde_json::json!(inferred_copy);
        let overview_pending = chapters::PendingChapterDrafts::default();
        let overview_preview = prepare_chapter_draft_preview(
            &fixture.storage,
            &overview_review,
            response(&overview_review, proposal.to_string()),
            &overview_pending,
        )
        .unwrap();
        save_pending_chapter_draft(
            &fixture.storage,
            &overview_pending,
            &Mutex::new(()),
            &overview_preview.generation_id,
        )
        .unwrap();

        let after = chapter_store::list_for_snapshot(
            &fixture.storage,
            &file_review.project_id,
            &file_review.snapshot_id,
        )
        .unwrap();
        let file_history_after = after
            .iter()
            .filter(|revision| {
                matches!(
                    &revision.target.scope,
                    chapters::ChapterTargetScope::File { .. }
                )
            })
            .map(chapters::encode_chapter_revision)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(file_history_after, file_history_bytes);
        assert_eq!(
            accepted_file.lifecycle,
            chapters::RevisionLifecycle::Accepted
        );
        assert_eq!(
            after
                .iter()
                .filter(|revision| matches!(
                    &revision.target.scope,
                    chapters::ChapterTargetScope::Overview
                ))
                .count(),
            1
        );
    }

    #[test]
    fn discard_malformed_and_changed_evidence_preserve_existing_revisions() {
        let fixture = Fixture::new();
        let review = review(&fixture);
        let pending = chapters::PendingChapterDrafts::default();
        let proposal = serde_json::to_string(&simple_proposal()).unwrap();
        let preview = prepare_chapter_draft_preview(
            &fixture.storage,
            &review,
            response(&review, proposal.clone()),
            &pending,
        )
        .unwrap();
        let saved = save_pending_chapter_draft(
            &fixture.storage,
            &pending,
            &Mutex::new(()),
            &preview.generation_id,
        )
        .unwrap();
        let before = chapter_store::list_for_project(&fixture.storage, &review.project_id).unwrap();

        let discarded = prepare_chapter_draft_preview(
            &fixture.storage,
            &review,
            response(&review, proposal.clone()),
            &pending,
        )
        .unwrap();
        pending_chapter_previews::remove(&fixture.storage, &discarded.generation_id).unwrap();
        pending.remove(&discarded.generation_id);
        assert!(
            save_pending_chapter_draft(
                &fixture.storage,
                &pending,
                &Mutex::new(()),
                &discarded.generation_id
            )
            .is_err()
        );
        let malformed = prepare_chapter_draft_preview(
            &fixture.storage,
            &review,
            response(&review, "{\"unexpected\":true}".to_owned()),
            &pending,
        );
        assert!(matches!(
            malformed,
            Err(provider::ProviderFailure {
                kind: provider::ProviderFailureKind::MalformedProviderResponse,
                ..
            })
        ));
        assert_eq!(
            chapter_store::list_for_project(&fixture.storage, &review.project_id).unwrap(),
            before
        );

        let stale = prepare_chapter_draft_preview(
            &fixture.storage,
            &review,
            response(&review, proposal),
            &pending,
        )
        .unwrap();
        let captured = fixture
            .manifest
            .files
            .iter()
            .find(|file| file.path == SIMPLE_PATH)
            .unwrap();
        std::fs::write(
            fixture.storage.join(&captured.blob_path),
            b"changed source bytes",
        )
        .unwrap();
        assert!(
            save_pending_chapter_draft(
                &fixture.storage,
                &pending,
                &Mutex::new(()),
                &stale.generation_id
            )
            .is_err()
        );
        assert_eq!(
            chapter_store::list_for_project(&fixture.storage, &review.project_id).unwrap(),
            vec![saved]
        );
    }

    #[cfg(unix)]
    #[test]
    fn preview_storage_failure_is_returned_without_adding_a_local_draft() {
        let fixture = Fixture::new();
        let review = review(&fixture);
        let outside = std::env::temp_dir().join(format!(
            "code-atlas-preview-blocked-{}",
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, fixture.storage.join("pending-chapter-previews"))
            .unwrap();
        let pending = chapters::PendingChapterDrafts::default();

        let failure = prepare_chapter_draft_preview(
            &fixture.storage,
            &review,
            response(&review, serde_json::to_string(&simple_proposal()).unwrap()),
            &pending,
        )
        .unwrap_err();

        assert_eq!(
            failure.kind,
            provider::ProviderFailureKind::LocalStorageUnavailable
        );
        assert!(
            failure
                .message
                .contains("recovery copy could not be persisted")
        );
        assert!(
            chapter_store::list_for_project(&fixture.storage, &review.project_id)
                .unwrap()
                .is_empty()
        );
        assert!(std::fs::read_dir(&outside).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(outside);
    }
}
