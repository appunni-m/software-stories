//! Native Rust/Tauri application backend for Code Atlas.
//!
//! This crate exposes [`run`] as the desktop entry point used by `src/main.rs`.
//! Chapter records, validation, and remapping are private implementation
//! details shared by the native command layer. Their persisted JSON format is
//! described in `docs/CHAPTER_SCHEMA.md`. This crate is built as part of the
//! desktop app and does not promise a separately versioned Rust SDK.

#![deny(missing_docs)]

mod chapter_freshness;
mod chapter_generation;
mod chapter_review;
mod chapter_store;
mod chapters;
mod git_evidence;
mod index;
mod narration;
mod pending_chapter_previews;
mod pr_guide;
mod pr_guide_store;
mod project;
mod provider;
mod recovery;
mod snapshot;

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
    let saved = chapter_store::save_new_draft(storage_dir, revision)?;
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
fn get_provider_disclosure() -> provider::ProviderDisclosure {
    provider::disclosure()
}

#[tauri::command]
fn get_narration_disclosure() -> narration::NarrationDisclosure {
    narration::disclosure()
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
fn get_provider_credential_status()
-> provider::ProviderCommandResult<provider::ProviderCredentialStatus> {
    provider::credential_status()
}

#[tauri::command(rename_all = "camelCase")]
fn store_gemini_api_key(
    api_key: String,
) -> provider::ProviderCommandResult<provider::ProviderCredentialStatus> {
    provider::store_api_key(api_key)
}

#[tauri::command]
fn remove_gemini_api_key() -> provider::ProviderCommandResult<provider::ProviderCredentialStatus> {
    provider::remove_api_key()
}

#[tauri::command(rename_all = "camelCase")]
fn build_gemini_request_review(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
    input: provider::BuildProviderReviewInput,
) -> provider::ProviderCommandResult<provider::ProviderRequestReview> {
    if sessions.get(&input.project_id).is_err() {
        return provider::ProviderCommandResult::error(
            provider::ProviderFailureKind::InvalidReview,
            "Reopen the selected repository before building a provider request review.",
        );
    }
    let pending = match provider::build_request_review(
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
fn build_gemini_pr_guide_request_review(
    sessions: State<'_, ProjectSessions>,
    reviews: State<'_, provider::PendingProviderReviews>,
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
    let pending = match provider::build_pr_guide_request_review(
        Path::new(&project.repository_root),
        &input,
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
fn discard_gemini_request_review(
    reviews: State<'_, provider::PendingProviderReviews>,
    review_id: String,
) {
    reviews.remove(&review_id);
}

#[tauri::command(rename_all = "camelCase")]
fn cancel_gemini_request(
    cancellations: State<'_, provider::ProviderRequestCancellations>,
    review_id: String,
) -> bool {
    cancellations.cancel(&review_id)
}

#[tauri::command(rename_all = "camelCase")]
async fn send_gemini_request(
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
        let api_key = match provider::load_api_key() {
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
                recovery::new_provider_operation(
                    pending.review.project_id.clone(),
                    pending.review.snapshot_id.clone(),
                    pending.review.target_path.clone(),
                    pending.review.review_id.clone(),
                ),
                || {
                    provider::await_cancellable_request(
                        &mut cancellation,
                        provider::dispatch_reviewed_request(
                            &pending,
                            api_key,
                            &consent,
                            |model_id, api_key, body| async move {
                                provider::send_to_gemini(&model_id, &api_key, body).await
                            },
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
async fn send_gemini_pr_guide_request(
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
        let api_key = match provider::load_api_key() {
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
                recovery::new_provider_operation(
                    pending.review.project_id.clone(),
                    pending.review.snapshot_id.clone(),
                    None,
                    pending.review.review_id.clone(),
                ),
                || {
                    provider::await_cancellable_request(
                        &mut cancellation,
                        provider::dispatch_reviewed_request(
                            &pending,
                            api_key,
                            &consent,
                            |model_id, api_key, body| async move {
                                provider::send_to_gemini(&model_id, &api_key, body).await
                            },
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

fn prepare_pr_guide_preview(
    storage_dir: &Path,
    review: &provider::ProviderRequestReview,
    result: provider::ProviderSendResult,
) -> Result<pr_guide::PrGuidePreview, provider::ProviderFailure> {
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
    let revision = pr_guide_store::save_draft(storage_dir, revision).map_err(|message| {
        failure(
            provider::ProviderFailureKind::LocalStorageUnavailable,
            format!("The PR Guide passed local evidence validation, but its separate history could not be saved: {message}"),
        )
    })?;
    Ok(pr_guide::PrGuidePreview {
        revision,
        model_id: result.model_id,
        model_version: result.model_version,
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
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

fn prepare_chapter_draft_preview(
    storage_dir: &std::path::Path,
    review: &provider::ProviderRequestReview,
    result: provider::ProviderSendResult,
    drafts: &chapters::PendingChapterDrafts,
) -> Result<chapters::ChapterDraftPreview, provider::ProviderFailure> {
    let failure = |kind, message: String| provider::ProviderFailure { kind, message };
    if result.review_id != review.review_id
        || result.fingerprint != review.fingerprint
        || result.model_id != review.model_id
    {
        return Err(failure(provider::ProviderFailureKind::InvalidReview, "The returned chapter does not match the reviewed request identity. No draft was created.".to_owned()));
    }
    let manifest = snapshot::load_snapshot_manifest(storage_dir, &review.project_id, &review.snapshot_id)
        .map_err(|_| failure(provider::ProviderFailureKind::InvalidReview, "The exact source snapshot became unavailable. Gemini returned text, but no chapter draft was created.".to_owned()))?;
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
    let mut revision = match (review.purpose.as_str(), review.target_path.as_deref()) {
        ("fileChapter", Some(target_path)) => chapter_generation::canonicalize_file_chapter(
            &review.project_id,
            &manifest,
            snapshot_index.as_ref(),
            target_path,
            &sources,
            &allowed_relationships,
            &result.response_text,
        ),
        ("repositoryOverview", None) => chapter_generation::canonicalize_repository_overview(
            &review.project_id,
            &manifest,
            snapshot_index.as_ref(),
            &sources,
            &allowed_relationships,
            &result.response_text,
        ),
        _ => Err("The reviewed request purpose and target do not match.".to_owned()),
    }
    .map_err(|message| {
        failure(
            provider::ProviderFailureKind::MalformedProviderResponse,
            message,
        )
    })?;
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
    let generation_id = drafts
        .insert(revision.clone())
        .map_err(|message| failure(provider::ProviderFailureKind::ReviewUnavailable, message))?;
    let preview = chapters::ChapterDraftPreview {
        generation_id,
        revision,
        validation,
        model_id: result.model_id,
        model_version: result.model_version,
        input_tokens: result.input_tokens,
        output_tokens: result.output_tokens,
    };
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
        .manage(SnapshotJobs::default())
        .manage(index::SnapshotIndexJobs::default())
        .manage(provider::PendingProviderReviews::default())
        .manage(provider::ProviderRequestCancellations::default())
        .manage(narration::PendingNarrationReviews::default())
        .manage(chapters::PendingChapterDrafts::default())
        .manage(recovery::InterruptionJournal::default())
        .manage(ChapterStoreLock::default())
        .invoke_handler(tauri::generate_handler![
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
            get_provider_disclosure,
            get_narration_disclosure,
            build_narration_request_review,
            discard_narration_request_review,
            load_reviewed_narration_audio,
            synthesize_reviewed_narration_audio,
            get_provider_credential_status,
            store_gemini_api_key,
            remove_gemini_api_key,
            build_gemini_request_review,
            build_gemini_pr_guide_request_review,
            discard_gemini_request_review,
            cancel_gemini_request,
            send_gemini_request,
            send_gemini_pr_guide_request
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Code Atlas");
}

#[cfg(test)]
mod chapter_pipeline_tests {
    use super::*;
    use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, simple_proposal};

    fn review(fixture: &Fixture) -> provider::ProviderRequestReview {
        provider::build_request_review(
            &fixture.storage,
            &provider::BuildProviderReviewInput {
                project_id: fixture.manifest.summary.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                target_path: Some(SIMPLE_PATH.to_owned()),
                selected_paths: vec![SIMPLE_PATH.to_owned()],
                model_id: "gemini-3.8-flash".to_owned(),
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
