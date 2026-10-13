use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentProject {
    pub project_id: String,
    pub display_name: String,
    pub selected_path: String,
    pub repository_root: String,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub access_status: AccessStatus,
    pub last_opened_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AccessStatus {
    Available,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectOpenResult {
    pub status: ProjectOpenStatus,
    pub project: Option<RecentProject>,
    pub selected_path: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectOpenStatus {
    Cancelled,
    Opened,
    NotGitRepository,
    InvalidDirectory,
    PermissionDenied,
    GitUnavailable,
    GitError,
    StorageError,
    RecentProjectUnavailable,
}

#[derive(Debug)]
enum InspectionError {
    InvalidDirectory { path: String, message: String },
    PermissionDenied { path: String, message: String },
    NotGitRepository { path: String },
    GitUnavailable { message: String },
    GitError { path: String, message: String },
}

static RECENT_PROJECTS_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub fn inspect_and_record_project(storage_dir: &Path, selected_path: &str) -> ProjectOpenResult {
    let project = match inspect_project(selected_path) {
        Ok(project) => project,
        Err(error) => return error.into_result(),
    };

    let mut recent_project = project.clone();
    recent_project.selected_path = project.repository_root.clone();
    match upsert_recent_project(storage_dir, recent_project) {
        Ok(()) => ProjectOpenResult::opened(project),
        Err(message) => ProjectOpenResult {
            status: ProjectOpenStatus::StorageError,
            project: Some(project),
            selected_path: None,
            message: Some(message),
        },
    }
}

pub fn open_recent_project(storage_dir: &Path, project_id: &str) -> ProjectOpenResult {
    let projects = match load_recent_projects(storage_dir) {
        Ok(projects) => projects,
        Err(message) => {
            return ProjectOpenResult {
                status: ProjectOpenStatus::StorageError,
                project: None,
                selected_path: None,
                message: Some(message),
            };
        }
    };

    let Some(project) = projects
        .iter()
        .find(|project| project.project_id == project_id)
    else {
        return ProjectOpenResult {
            status: ProjectOpenStatus::RecentProjectUnavailable,
            project: None,
            selected_path: None,
            message: Some(
                "This repository is no longer in Recents. Choose it again to add it back."
                    .to_owned(),
            ),
        };
    };

    inspect_and_record_project(storage_dir, &project.selected_path)
}

pub fn list_recent_projects(storage_dir: &Path) -> Result<Vec<RecentProject>, String> {
    load_recent_projects(storage_dir)
}

pub fn remove_recent_project(
    storage_dir: &Path,
    project_id: &str,
) -> Result<Vec<RecentProject>, String> {
    with_recent_lock(|| {
        let mut projects = load_recent_projects(storage_dir)?;
        let original_count = projects.len();
        projects.retain(|project| project.project_id != project_id);
        if projects.len() != original_count {
            write_recent_projects(storage_dir, &projects)?;
        }
        Ok(projects)
    })
}

fn inspect_project(selected_path: &str) -> Result<RecentProject, InspectionError> {
    let input_path = Path::new(selected_path);
    let canonical_selected_path =
        fs::canonicalize(input_path).map_err(|error| directory_error(input_path, error))?;

    let metadata = fs::metadata(&canonical_selected_path)
        .map_err(|error| directory_error(&canonical_selected_path, error))?;
    if !metadata.is_dir() {
        return Err(InspectionError::InvalidDirectory {
            path: canonical_selected_path.display().to_string(),
            message: "The selected path is not a folder.".to_owned(),
        });
    }

    // Opening the directory checks access without enumerating entries or reading source bytes.
    File::open(&canonical_selected_path)
        .map_err(|error| directory_error(&canonical_selected_path, error))?;

    let worktree_output = git_output(
        &canonical_selected_path,
        &["rev-parse", "--is-inside-work-tree"],
    )?;
    if !worktree_output.status.success() {
        let message = output_message(&worktree_output);
        if message.to_lowercase().contains("not a git repository") {
            return Err(InspectionError::NotGitRepository {
                path: canonical_selected_path.display().to_string(),
            });
        }
        return Err(git_output_error(&canonical_selected_path, message));
    }
    if String::from_utf8_lossy(&worktree_output.stdout).trim() != "true" {
        return Err(InspectionError::NotGitRepository {
            path: canonical_selected_path.display().to_string(),
        });
    }

    let root_output = git_output(&canonical_selected_path, &["rev-parse", "--show-toplevel"])?;
    if !root_output.status.success() {
        let message = output_message(&root_output);
        if message.to_lowercase().contains("not a git repository")
            || message
                .to_lowercase()
                .contains("must be run in a work tree")
        {
            return Err(InspectionError::NotGitRepository {
                path: canonical_selected_path.display().to_string(),
            });
        }
        return Err(git_output_error(&canonical_selected_path, message));
    }

    let reported_root = String::from_utf8_lossy(&root_output.stdout)
        .trim()
        .to_owned();
    let canonical_root =
        fs::canonicalize(&reported_root).map_err(|error| InspectionError::GitError {
            path: canonical_selected_path.display().to_string(),
            message: format!("Git reported a repository root that cannot be accessed: {error}"),
        })?;
    let repository_root = canonical_root.display().to_string();
    let branch = git_optional(
        &canonical_root,
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
    )?;
    let head = git_optional(&canonical_root, &["rev-parse", "--verify", "HEAD"])?;
    let display_name = canonical_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("Repository")
        .to_owned();

    Ok(RecentProject {
        // Canonical root identity survives relaunch and is independent of the display name.
        project_id: repository_root.clone(),
        display_name,
        selected_path: canonical_selected_path.display().to_string(),
        repository_root,
        branch,
        head,
        access_status: AccessStatus::Available,
        last_opened_unix_ms: unix_time_ms(),
    })
}

fn directory_error(path: &Path, error: io::Error) -> InspectionError {
    let path = path.display().to_string();
    let message = match error.kind() {
        io::ErrorKind::NotFound => "The selected folder no longer exists.".to_owned(),
        io::ErrorKind::PermissionDenied => "macOS denied access to this folder. Choose it again or grant folder access in System Settings.".to_owned(),
        _ => format!("The folder could not be opened: {error}"),
    };

    if error.kind() == io::ErrorKind::PermissionDenied {
        InspectionError::PermissionDenied { path, message }
    } else {
        InspectionError::InvalidDirectory { path, message }
    }
}

fn git_output(path: &Path, args: &[&str]) -> Result<Output, InspectionError> {
    git_command()
        .args(args)
        .current_dir(path)
        .output()
        .map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                InspectionError::GitUnavailable {
                    message: "Git was not found. Install Git or the Xcode Command Line Tools, then try again.".to_owned(),
                }
            } else {
                InspectionError::GitError {
                    path: path.display().to_string(),
                    message: format!("Git could not inspect this folder: {error}"),
                }
            }
        })
}

pub(crate) fn git_command() -> Command {
    #[cfg(target_os = "macos")]
    let executable = [
        "/opt/homebrew/bin/git",
        "/usr/local/bin/git",
        "/usr/bin/git",
        "git",
    ]
    .into_iter()
    .find(|candidate| *candidate == "git" || Path::new(candidate).is_file())
    .unwrap_or("git");

    #[cfg(not(target_os = "macos"))]
    let executable = "git";

    let mut command = Command::new(executable);
    for variable in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_PREFIX",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    ] {
        command.env_remove(variable);
    }
    command
}

fn git_optional(path: &Path, args: &[&str]) -> Result<Option<String>, InspectionError> {
    let output = git_output(path, args)?;
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok((!value.is_empty()).then_some(value))
}

fn output_message(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if stderr.is_empty() {
        "Git returned an error without details.".to_owned()
    } else {
        stderr
    }
}

fn git_output_error(path: &Path, message: String) -> InspectionError {
    let lowered = message.to_lowercase();
    if lowered.contains("permission denied") || lowered.contains("operation not permitted") {
        InspectionError::PermissionDenied {
            path: path.display().to_string(),
            message: format!("macOS denied access to Git metadata for this folder: {message}"),
        }
    } else {
        InspectionError::GitError {
            path: path.display().to_string(),
            message,
        }
    }
}

impl InspectionError {
    fn into_result(self) -> ProjectOpenResult {
        match self {
            Self::InvalidDirectory { path, message } => ProjectOpenResult::failure(
                ProjectOpenStatus::InvalidDirectory,
                Some(path),
                message,
            ),
            Self::PermissionDenied { path, message } => ProjectOpenResult::failure(
                ProjectOpenStatus::PermissionDenied,
                Some(path),
                message,
            ),
            Self::NotGitRepository { path } => ProjectOpenResult::failure(
                ProjectOpenStatus::NotGitRepository,
                Some(path),
                "This folder is not inside a Git working copy. Choose a repository folder or one of its subfolders.".to_owned(),
            ),
            Self::GitUnavailable { message } => ProjectOpenResult::failure(
                ProjectOpenStatus::GitUnavailable,
                None,
                message,
            ),
            Self::GitError { path, message } => ProjectOpenResult::failure(
                ProjectOpenStatus::GitError,
                Some(path),
                message,
            ),
        }
    }
}

impl ProjectOpenResult {
    pub fn cancelled() -> Self {
        Self {
            status: ProjectOpenStatus::Cancelled,
            project: None,
            selected_path: None,
            message: None,
        }
    }

    fn opened(project: RecentProject) -> Self {
        Self {
            status: ProjectOpenStatus::Opened,
            project: Some(project),
            selected_path: None,
            message: None,
        }
    }

    fn failure(status: ProjectOpenStatus, selected_path: Option<String>, message: String) -> Self {
        Self {
            status,
            project: None,
            selected_path,
            message: Some(message),
        }
    }

    pub fn storage_error(message: String) -> Self {
        Self::failure(ProjectOpenStatus::StorageError, None, message)
    }

    pub fn invalid_selected_path(message: String) -> Self {
        Self::failure(ProjectOpenStatus::InvalidDirectory, None, message)
    }
}

fn recent_projects_path(storage_dir: &Path) -> PathBuf {
    storage_dir.join("recent-projects.json")
}

fn load_recent_projects(storage_dir: &Path) -> Result<Vec<RecentProject>, String> {
    let path = recent_projects_path(storage_dir);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Recent projects could not be read: {error}")),
    };
    let mut projects: Vec<RecentProject> = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Recent projects data is invalid: {error}"))?;
    projects.sort_by(|left, right| right.last_opened_unix_ms.cmp(&left.last_opened_unix_ms));
    Ok(projects)
}

fn upsert_recent_project(storage_dir: &Path, project: RecentProject) -> Result<(), String> {
    with_recent_lock(|| {
        let mut projects = load_recent_projects(storage_dir)?;
        projects.retain(|saved| saved.project_id != project.project_id);
        projects.insert(0, project);
        write_recent_projects(storage_dir, &projects)
    })
}

fn write_recent_projects(storage_dir: &Path, projects: &[RecentProject]) -> Result<(), String> {
    fs::create_dir_all(storage_dir).map_err(|error| {
        format!("The local Code Atlas data folder could not be created: {error}")
    })?;
    let target_path = recent_projects_path(storage_dir);
    let temporary_path = storage_dir.join(format!("recent-projects.{}.tmp", std::process::id()));
    let bytes = serde_json::to_vec_pretty(projects)
        .map_err(|error| format!("Recent projects could not be encoded: {error}"))?;

    let write_result = (|| {
        let mut file = File::create(&temporary_path)
            .map_err(|error| format!("Recent projects could not be saved: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("Recent projects could not be saved: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Recent projects could not be saved: {error}"))?;
        fs::rename(&temporary_path, &target_path)
            .map_err(|error| format!("Recent projects could not be finalized: {error}"))
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
}

fn with_recent_lock<T>(operation: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
    let lock = RECENT_PROJECTS_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock
        .lock()
        .map_err(|_| "Recent projects storage is temporarily unavailable.".to_owned())?;
    operation()
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after UNIX epoch")
                .as_nanos();
            let path = std::env::temp_dir()
                .join(format!("code-atlas-{label}-{}-{nonce}", std::process::id()));
            fs::create_dir_all(&path).expect("temporary directory should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn git(path: &Path, args: &[&str]) -> Output {
        git_command()
            .args(args)
            .current_dir(path)
            .output()
            .expect("git should be available for repository fixture tests")
    }

    fn create_repository(path: &Path) {
        fs::create_dir_all(path).expect("repository folder should be created");
        let init = git(path, &["init", "--quiet", "--initial-branch=main"]);
        assert!(
            init.status.success(),
            "git init failed: {}",
            output_message(&init)
        );
        for (key, value) in [
            ("user.name", "Code Atlas Tests"),
            ("user.email", "tests@example.invalid"),
        ] {
            let config = git(path, &["config", key, value]);
            assert!(
                config.status.success(),
                "git config failed: {}",
                output_message(&config)
            );
        }
        fs::write(path.join("README.md"), "fixture bytes\n")
            .expect("fixture file should be written");
        let add = git(path, &["add", "README.md"]);
        assert!(
            add.status.success(),
            "git add failed: {}",
            output_message(&add)
        );
        let commit = git(path, &["commit", "--quiet", "-m", "fixture"]);
        assert!(
            commit.status.success(),
            "git commit failed: {}",
            output_message(&commit)
        );
    }

    #[test]
    fn nested_folder_resolves_root_and_reads_no_repository_bytes() {
        let temporary = TestDirectory::new("nested-repo");
        let repository = temporary.path().join("repository");
        create_repository(&repository);
        let nested = repository.join("src").join("module");
        fs::create_dir_all(&nested).expect("nested folder should be created");
        let source = nested.join("lib.rs");
        fs::write(&source, "pub fn answer() -> u8 { 42 }\n")
            .expect("source fixture should be written");

        let head_before = git(&repository, &["rev-parse", "HEAD"]);
        let status_before = git(&repository, &["status", "--porcelain"]);
        let project = inspect_project(nested.to_str().expect("fixture path should be UTF-8"))
            .expect("nested repository folder should be accepted");
        let head_after = git(&repository, &["rev-parse", "HEAD"]);
        let status_after = git(&repository, &["status", "--porcelain"]);

        assert_eq!(
            project.repository_root,
            repository.canonicalize().unwrap().display().to_string()
        );
        assert_eq!(
            project.selected_path,
            nested.canonicalize().unwrap().display().to_string()
        );
        assert_eq!(project.branch.as_deref(), Some("main"));
        assert_eq!(
            project.head.as_deref(),
            Some(String::from_utf8_lossy(&head_before.stdout).trim())
        );
        assert_eq!(head_before.stdout, head_after.stdout);
        assert_eq!(status_before.stdout, status_after.stdout);
        assert_eq!(
            fs::read_to_string(source).unwrap(),
            "pub fn answer() -> u8 { 42 }\n"
        );
    }

    #[test]
    fn plain_folder_and_missing_path_have_distinct_recoverable_outcomes() {
        let temporary = TestDirectory::new("non-repo");
        let selected = temporary
            .path()
            .to_str()
            .expect("fixture path should be UTF-8");
        assert!(matches!(
            inspect_project(selected),
            Err(InspectionError::NotGitRepository { .. })
        ));

        let missing = temporary.path().join("missing");
        assert!(matches!(
            inspect_project(missing.to_str().expect("fixture path should be UTF-8")),
            Err(InspectionError::InvalidDirectory { .. })
        ));
    }

    #[test]
    fn recent_project_identity_and_removal_touch_only_app_metadata() {
        let temporary = TestDirectory::new("recent-project");
        let repository = temporary.path().join("repository");
        let storage = temporary.path().join("app-data");
        create_repository(&repository);
        let project_file = repository.join("README.md");
        let original_contents = fs::read(&project_file).unwrap();

        let first = inspect_project(repository.to_str().unwrap()).unwrap();
        upsert_recent_project(&storage, first.clone()).unwrap();
        let loaded = load_recent_projects(&storage).unwrap();
        assert_eq!(loaded, vec![first.clone()]);

        let reopened_result = open_recent_project(&storage, &first.project_id);
        assert_eq!(reopened_result.status, ProjectOpenStatus::Opened);
        let reopened = reopened_result
            .project
            .expect("recent repository should reopen");
        assert_eq!(first.project_id, reopened.project_id);
        assert_eq!(first.display_name, reopened.display_name);

        let after_remove = remove_recent_project(&storage, &first.project_id).unwrap();
        assert!(after_remove.is_empty());
        assert_eq!(fs::read(project_file).unwrap(), original_contents);
        assert!(!repository.join("recent-projects.json").exists());
    }

    #[test]
    fn permission_denial_maps_to_its_own_status() {
        let error = directory_error(
            Path::new("/private/example"),
            io::Error::from(io::ErrorKind::PermissionDenied),
        );
        assert!(matches!(error, InspectionError::PermissionDenied { .. }));
    }

    #[test]
    fn cancelling_folder_selection_has_no_path_or_project_to_mutate() {
        let result = ProjectOpenResult::cancelled();
        assert_eq!(result.status, ProjectOpenStatus::Cancelled);
        assert!(result.project.is_none());
        assert!(result.selected_path.is_none());
        assert!(result.message.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_folder_reports_permission_denied_when_not_running_as_root() {
        use std::os::unix::fs::PermissionsExt;

        let identity = Command::new("id")
            .arg("-u")
            .output()
            .expect("id command should be available");
        if String::from_utf8_lossy(&identity.stdout).trim() == "0" {
            return;
        }

        let temporary = TestDirectory::new("permission-denied");
        let folder = temporary.path().join("private");
        fs::create_dir(&folder).expect("private folder should be created");
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o000))
            .expect("folder permissions should be restricted");
        let result = inspect_project(folder.to_str().expect("fixture path should be UTF-8"));
        fs::set_permissions(&folder, fs::Permissions::from_mode(0o700))
            .expect("folder permissions should be restored");

        assert!(matches!(
            result,
            Err(InspectionError::PermissionDenied { .. })
        ));
    }
}
