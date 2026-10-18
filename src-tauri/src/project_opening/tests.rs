use super::*;
use crate::{chapter_generation::tests::Fixture, project::ProjectOpenStatus, repository_context};
use std::{collections::BTreeMap, fs, path::PathBuf, sync::atomic::AtomicBool};

fn files(root: &Path) -> BTreeMap<PathBuf, String> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, String>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), result);
            } else if entry.file_type().unwrap().is_file() {
                result.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    repository_context::digest(&fs::read(entry.path()).unwrap()),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

#[test]
fn cancelled_inspection_never_updates_recents_sessions_or_snapshot_data() {
    let fixture = Fixture::new();
    let sessions = ProjectSessions::default();
    let jobs = RepositoryContextJobs::default();
    let id = uuid::Uuid::new_v4().to_string();
    let request = jobs.begin(&id).unwrap();
    let before = files(&fixture.storage);
    let inspected = project::inspect_selected_project(
        &fixture.manifest.summary.project_id,
        request.cancellation(),
    );
    assert_eq!(inspected.status, ProjectOpenStatus::Opened);
    jobs.cancel(&id).unwrap();
    let result = publish(&fixture.storage, inspected, &request, &sessions).unwrap();
    assert_eq!(result.status, ProjectOpenStatus::Cancelled);
    assert!(result.project.is_none());
    assert!(sessions.0.lock().unwrap().is_empty());
    assert_eq!(files(&fixture.storage), before);
}

#[test]
fn cancellation_discards_staged_recent_projects_before_publication() {
    let fixture = Fixture::new();
    let sessions = ProjectSessions::default();
    let jobs = RepositoryContextJobs::default();
    let id = uuid::Uuid::new_v4().to_string();
    let request = jobs.begin(&id).unwrap();
    let inspected = project::inspect_selected_project(
        &fixture.manifest.summary.project_id,
        request.cancellation(),
    );
    let before = files(&fixture.storage);
    let prepared =
        project::prepare_inspected_project(&fixture.storage, inspected, request.cancellation());
    assert!(
        fs::read_dir(&fixture.storage).unwrap().any(|entry| entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")),
        "recent-project metadata should be staged before the short commit boundary"
    );

    // Stop can acquire the request lock while the prepared disk update is held
    // outside it; rejecting publication then drops the staged file.
    jobs.cancel(&id).unwrap();
    assert!(request.commit(|| prepared.publish()).is_err());
    assert!(sessions.0.lock().unwrap().is_empty());
    assert_eq!(files(&fixture.storage), before);
}

#[test]
fn live_owner_publishes_one_project_without_changing_repository_or_saved_snapshots() {
    let fixture = Fixture::new();
    let path = Path::new(&fixture.manifest.summary.project_id);
    let before_repository = files(path);
    let before_storage = files(&fixture.storage);
    let sessions = ProjectSessions::default();
    let jobs = RepositoryContextJobs::default();
    let request = jobs.begin(&uuid::Uuid::new_v4().to_string()).unwrap();
    let inspected =
        project::inspect_selected_project(path.to_str().unwrap(), request.cancellation());
    assert_eq!(inspected.status, ProjectOpenStatus::Opened);
    assert!(
        !fixture.storage.join("recent-projects.json").exists(),
        "inspection is read only"
    );
    let result = publish(&fixture.storage, inspected, &request, &sessions).unwrap();
    assert_eq!(result.status, ProjectOpenStatus::Opened);
    assert_eq!(sessions.0.lock().unwrap().len(), 1);
    assert!(sessions.get(path.to_str().unwrap()).is_ok());
    let recents = project::list_recent_projects(&fixture.storage).unwrap();
    assert_eq!(recents.len(), 1);
    assert_eq!(recents[0].project_id, path.to_str().unwrap());
    assert_eq!(files(path), before_repository);
    let mut after = files(&fixture.storage);
    assert!(after.remove(Path::new("recent-projects.json")).is_some());
    assert_eq!(after, before_storage);
}

#[test]
fn a_pre_cancelled_request_and_folder_error_have_no_publication_side_effects() {
    let fixture = Fixture::new();
    let sessions = ProjectSessions::default();
    let jobs = RepositoryContextJobs::default();
    let id = uuid::Uuid::new_v4().to_string();
    jobs.cancel(&id).unwrap();
    assert!(jobs.begin(&id).is_err());
    let before = files(&fixture.storage);
    let cancelled = AtomicBool::new(true);
    let result =
        project::inspect_selected_project("/code-atlas-project-cancel-no-such-path", &cancelled);
    assert_eq!(result.status, ProjectOpenStatus::Cancelled);
    let request = jobs.begin(&uuid::Uuid::new_v4().to_string()).unwrap();
    let result = project::inspect_selected_project(
        "/code-atlas-project-cancel-no-such-path",
        request.cancellation(),
    );
    assert_eq!(result.status, ProjectOpenStatus::InvalidDirectory);
    let result = publish(&fixture.storage, result, &request, &sessions).unwrap();
    assert_eq!(result.status, ProjectOpenStatus::InvalidDirectory);
    assert_eq!(files(&fixture.storage), before);
    assert!(sessions.0.lock().unwrap().is_empty());
}

#[test]
fn stopping_recent_reinspection_preserves_previous_recents_and_session_selection() {
    let fixture = Fixture::new();
    let path = &fixture.manifest.summary.project_id;
    let sessions = ProjectSessions::default();
    let jobs = RepositoryContextJobs::default();
    let first = jobs.begin(&uuid::Uuid::new_v4().to_string()).unwrap();
    let inspected = project::inspect_selected_project(path, first.cancellation());
    publish(&fixture.storage, inspected, &first, &sessions).unwrap();
    drop(first);
    let before = files(&fixture.storage);
    let remembered = sessions.get(path).unwrap();
    let id = uuid::Uuid::new_v4().to_string();
    let request = jobs.begin(&id).unwrap();
    let inspected = project::inspect_recent_project(&fixture.storage, path, request.cancellation());
    assert_eq!(inspected.status, ProjectOpenStatus::Opened);
    jobs.cancel(&id).unwrap();
    assert_eq!(
        publish(&fixture.storage, inspected, &request, &sessions)
            .unwrap()
            .status,
        ProjectOpenStatus::Cancelled
    );
    assert_eq!(sessions.get(path).unwrap(), remembered);
    assert_eq!(files(&fixture.storage), before);
}

#[cfg(unix)]
#[test]
fn stop_interrupts_real_project_inspection_while_git_reads_config() {
    use std::{
        fs::OpenOptions,
        os::unix::fs::OpenOptionsExt,
        process::Command,
        thread,
        time::{Duration, Instant},
    };
    let fixture = Fixture::new();
    let path = Path::new(&fixture.manifest.summary.project_id);
    let pipe = path.join("held-config");
    assert!(
        Command::new("mkfifo")
            .args(["-m", "600"])
            .arg(&pipe)
            .status()
            .unwrap()
            .success()
    );
    let config_path = path.join(".git/config");
    let mut config = fs::read(&config_path).unwrap();
    config.extend_from_slice(format!("\n[include]\n\tpath = {}\n", pipe.display()).as_bytes());
    fs::write(&config_path, &config).unwrap();
    let before_storage = files(&fixture.storage);
    let head = fs::read(path.join(".git/HEAD")).unwrap();
    let index = fs::read(path.join(".git/index")).unwrap();
    let jobs = RepositoryContextJobs::default();
    let id = uuid::Uuid::new_v4().to_string();
    let request = jobs.begin(&id).unwrap();
    let sessions = ProjectSessions::default();
    thread::scope(|scope| {
        let worker = scope.spawn(|| {
            project::inspect_selected_project(path.to_str().unwrap(), request.cancellation())
        });
        let until = Instant::now() + Duration::from_secs(2);
        let mut writer = None;
        while Instant::now() < until && !worker.is_finished() {
            if let Ok(file) = OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&pipe)
            {
                writer = Some(file);
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let observed = writer.is_some();
        if observed {
            let hold = Instant::now() + Duration::from_millis(40);
            while Instant::now() < hold {
                assert!(!worker.is_finished());
                thread::sleep(Duration::from_millis(2));
            }
        }
        let stopped = Instant::now();
        jobs.cancel(&id).unwrap();
        let inspected = worker.join().unwrap();
        assert!(
            observed,
            "the real inspection must reach and remain held on the config stream"
        );
        assert_eq!(inspected.status, ProjectOpenStatus::Cancelled);
        assert!(stopped.elapsed() < Duration::from_secs(2));
        assert_eq!(
            publish(&fixture.storage, inspected, &request, &sessions)
                .unwrap()
                .status,
            ProjectOpenStatus::Cancelled
        );
        drop(writer);
    });
    assert!(sessions.0.lock().unwrap().is_empty());
    assert_eq!(files(&fixture.storage), before_storage);
    assert_eq!(fs::read(&config_path).unwrap(), config);
    assert_eq!(fs::read(path.join(".git/HEAD")).unwrap(), head);
    assert_eq!(fs::read(path.join(".git/index")).unwrap(), index);
}
