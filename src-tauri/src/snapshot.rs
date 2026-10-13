use crate::project::RecentProject;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

const POLICY_VERSION: &str = "capture-policy-v1";
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 100 * 1024 * 1024;
const MAX_PATHS: u64 = 25_000;
const MAX_PATH_BYTES: usize = 1_024;
const MAX_ENTRIES_PER_DIRECTORY: u64 = 1_000;
const READ_CHUNK_BYTES: usize = 64 * 1024;
const MAX_RETAINED_JOBS: usize = 32;

static SNAPSHOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const DEFAULT_EXCLUDED_DIRECTORIES: &[&str] = &[
    "node_modules",
    "vendor",
    "target",
    "dist",
    "build",
    "coverage",
    "out",
    "generated",
    ".next",
    ".nuxt",
    ".turbo",
    ".cache",
    ".venv",
    "venv",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureLimits {
    pub policy_version: String,
    pub max_file_bytes: u64,
    pub max_total_bytes: u64,
    pub max_paths: u64,
    pub max_path_bytes: usize,
    pub max_entries_per_directory: u64,
    pub excluded_directories: Vec<String>,
    pub other_exclusions: Vec<String>,
}

impl Default for CaptureLimits {
    fn default() -> Self {
        Self {
            policy_version: POLICY_VERSION.to_owned(),
            max_file_bytes: MAX_FILE_BYTES,
            max_total_bytes: MAX_TOTAL_BYTES,
            max_paths: MAX_PATHS,
            max_path_bytes: MAX_PATH_BYTES,
            max_entries_per_directory: MAX_ENTRIES_PER_DIRECTORY,
            excluded_directories: DEFAULT_EXCLUDED_DIRECTORIES
                .iter()
                .map(|name| (*name).to_owned())
                .collect(),
            other_exclusions: vec![
                "Git-ignored paths; tracked files remain eligible".to_owned(),
                "Symlinks and their targets".to_owned(),
                "The repository .git marker and its contents".to_owned(),
                "Nested Git repositories".to_owned(),
                "Non-regular filesystem objects".to_owned(),
            ],
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureStatus {
    Running,
    Complete,
    Partial,
    Cancelled,
    Inconsistent,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CapturePhase {
    Scanning,
    CheckingIgnoreRules,
    ReadingFiles,
    Finalizing,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GitChangeState {
    Clean,
    Modified,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InventoryPathKind {
    File,
    Directory,
    Symlink,
    Other,
    Unknown,
}

impl Default for InventoryPathKind {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CapturedContentKind {
    Text,
    Binary,
    InvalidEncoding,
    Unknown,
}

impl Default for CapturedContentKind {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedFile {
    pub path: String,
    #[serde(default)]
    pub kind: InventoryPathKind,
    pub size_bytes: u64,
    pub sha256: String,
    pub blob_path: String,
    #[serde(default)]
    pub content_kind: CapturedContentKind,
    #[serde(default)]
    pub secret_like: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedPath {
    pub path: String,
    #[serde(default)]
    pub kind: InventoryPathKind,
    #[serde(default)]
    pub size_bytes: Option<u64>,
    pub reason: String,
    pub details: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSource {
    pub snapshot_id: String,
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub contents: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryDirectory {
    pub path: String,
    pub kind: InventoryPathKind,
    pub size_bytes: Option<u64>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSummary {
    pub snapshot_id: String,
    pub project_id: String,
    pub parent_snapshot_id: Option<String>,
    pub created_at_unix_ms: u64,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub index_state: GitChangeState,
    pub worktree_state: GitChangeState,
    pub status: CaptureStatus,
    pub files_captured: usize,
    pub paths_skipped: usize,
    pub total_bytes: u64,
    pub paths_visited: u64,
    pub policy_version: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotManifest {
    pub schema_version: u32,
    pub summary: SnapshotSummary,
    pub repository_root: String,
    pub selected_path: String,
    pub capture_limits: CaptureLimits,
    #[serde(default)]
    pub directories: Vec<InventoryDirectory>,
    pub files: Vec<CapturedFile>,
    pub skipped: Vec<SkippedPath>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotProgress {
    pub project_id: String,
    pub snapshot_id: String,
    pub parent_snapshot_id: Option<String>,
    pub status: CaptureStatus,
    pub phase: CapturePhase,
    pub paths_visited: u64,
    pub candidate_files: usize,
    pub files_processed: usize,
    pub files_captured: usize,
    pub paths_skipped: usize,
    pub bytes_read: u64,
    pub current_path: Option<String>,
    pub message: Option<String>,
    pub limits: CaptureLimits,
    pub summary: Option<SnapshotSummary>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotStart {
    pub snapshot_id: String,
    pub parent_snapshot_id: Option<String>,
    pub limits: CaptureLimits,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureError {
    Cancelled,
    Inconsistent {
        path: Option<String>,
        message: String,
    },
    Failed(String),
}

#[derive(Clone, Default)]
pub struct SnapshotJobs {
    inner: Arc<Mutex<HashMap<String, SnapshotJob>>>,
}

#[derive(Clone)]
struct SnapshotJob {
    cancel: Arc<AtomicBool>,
    progress: SnapshotProgress,
}

impl SnapshotJobs {
    pub fn begin(&self, progress: SnapshotProgress) -> Result<Arc<AtomicBool>, String> {
        let mut jobs = self
            .inner
            .lock()
            .map_err(|_| "Snapshot jobs are temporarily unavailable.".to_owned())?;
        if jobs.values().any(|job| {
            job.progress.project_id == progress.project_id
                && job.progress.status == CaptureStatus::Running
        }) {
            return Err("A snapshot is already being captured for this repository.".to_owned());
        }
        jobs.retain(|_, job| job.progress.status == CaptureStatus::Running);
        if jobs.len() >= MAX_RETAINED_JOBS {
            return Err(
                "Too many repositories are being captured at once. Wait for one capture to finish."
                    .to_owned(),
            );
        }
        let cancel = Arc::new(AtomicBool::new(false));
        jobs.insert(
            progress.snapshot_id.clone(),
            SnapshotJob {
                cancel: Arc::clone(&cancel),
                progress,
            },
        );
        Ok(cancel)
    }

    pub fn update(&self, progress: SnapshotProgress) {
        if let Ok(mut jobs) = self.inner.lock() {
            if let Some(job) = jobs.get_mut(&progress.snapshot_id) {
                job.progress = progress;
            }
        }
    }

    pub fn get(&self, snapshot_id: &str) -> Option<SnapshotProgress> {
        self.inner
            .lock()
            .ok()?
            .get(snapshot_id)
            .map(|job| job.progress.clone())
    }

    pub fn cancel(&self, snapshot_id: &str) -> Result<(), String> {
        let jobs = self
            .inner
            .lock()
            .map_err(|_| "Snapshot jobs are temporarily unavailable.".to_owned())?;
        let Some(job) = jobs.get(snapshot_id) else {
            return Err("This snapshot capture is no longer running.".to_owned());
        };
        if job.progress.status != CaptureStatus::Running {
            return Err("This snapshot capture has already finished.".to_owned());
        }
        job.cancel.store(true, Ordering::Release);
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSnapshots {
    pub project_id: String,
    pub snapshots: Vec<SnapshotSummary>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotCatalog {
    schema_version: u32,
    snapshots: Vec<SnapshotSummary>,
}

#[derive(Clone, Debug)]
struct Candidate {
    absolute_path: PathBuf,
    relative_path: String,
    is_directory: bool,
    fingerprint: FileFingerprint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FileFingerprint {
    length: u64,
    modified_seconds: i64,
    modified_nanos: i64,
    changed_seconds: i64,
    changed_nanos: i64,
    device: u64,
    inode: u64,
    mode: u32,
}

struct ScanResult {
    candidates: Vec<Candidate>,
    directories: Vec<(PathBuf, FileFingerprint, String)>,
    skipped: Vec<SkippedPath>,
    paths_visited: u64,
    limits: CaptureLimits,
}

struct StagingDirectory {
    path: PathBuf,
}

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn new_snapshot_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let sequence = SNAPSHOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("snap-{timestamp}-{}-{sequence}", std::process::id())
}

/// Remove only the staging directory named by a validated capture snapshot ID.
/// This is used after a user dismisses a recovery notice; immutable snapshot
/// manifests and content-addressed objects are deliberately left untouched.
pub fn cleanup_interrupted_staging(storage_dir: &Path, snapshot_id: &str) -> Result<(), String> {
    if !is_capture_snapshot_id(snapshot_id) {
        return Err(
            "The interrupted capture ID is invalid; its staging data was left untouched."
                .to_owned(),
        );
    }
    let staging_root = storage_dir.join("staging");
    let root_metadata = match fs::symlink_metadata(&staging_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Interrupted capture staging could not be checked: {error}"
            ));
        }
    };
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(
            "Interrupted capture staging is not a local directory; its data was left untouched."
                .to_owned(),
        );
    }

    let operation_directory = staging_root.join(snapshot_id);
    let metadata = match fs::symlink_metadata(&operation_directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(format!(
                "Interrupted capture staging could not be checked: {error}"
            ));
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("Interrupted capture staging is not a real operation directory; its data was left untouched.".to_owned());
    }
    fs::remove_dir_all(&operation_directory)
        .map_err(|error| format!("Interrupted capture staging could not be removed: {error}"))
}

fn is_capture_snapshot_id(snapshot_id: &str) -> bool {
    let Some(components) = snapshot_id.strip_prefix("snap-") else {
        return false;
    };
    let mut parts = components.split('-');
    let (Some(timestamp), Some(process_id), Some(sequence), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    [timestamp, process_id, sequence]
        .into_iter()
        .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(0)
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn project_storage_key(project_id: &str) -> String {
    sha256(project_id.as_bytes())
}

fn snapshots_directory(storage_dir: &Path, project_id: &str) -> PathBuf {
    storage_dir
        .join("snapshots")
        .join(project_storage_key(project_id))
}

fn catalog_path(storage_dir: &Path, project_id: &str) -> PathBuf {
    snapshots_directory(storage_dir, project_id).join("catalog.json")
}

fn manifest_path(storage_dir: &Path, project_id: &str, snapshot_id: &str) -> PathBuf {
    snapshots_directory(storage_dir, project_id)
        .join("manifests")
        .join(format!("{snapshot_id}.json"))
}

fn blob_relative_path(hash: &str) -> String {
    format!("objects/sha256/{}/{}", &hash[..2], &hash[2..])
}

fn blob_absolute_path(storage_dir: &Path, hash: &str) -> PathBuf {
    storage_dir.join(blob_relative_path(hash))
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Snapshot storage path has no parent folder.".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Snapshot storage folder could not be created: {error}"))?;
    let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("snapshot-data");
    let temporary_path = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        sequence
    ));

    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| format!("Snapshot metadata could not be written: {error}"))?;
        file.write_all(bytes)
            .map_err(|error| format!("Snapshot metadata could not be written: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Snapshot metadata could not be synced: {error}"))?;
        fs::rename(&temporary_path, path)
            .map_err(|error| format!("Snapshot metadata could not be finalized: {error}"))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn read_catalog(storage_dir: &Path, project_id: &str) -> Result<SnapshotCatalog, String> {
    let path = catalog_path(storage_dir, project_id);
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(SnapshotCatalog {
                schema_version: 1,
                snapshots: Vec::new(),
            });
        }
        Err(error) => return Err(format!("Snapshot history could not be read: {error}")),
    };
    let catalog: SnapshotCatalog = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Snapshot history is invalid: {error}"))?;
    if catalog.schema_version != 1 {
        return Err(format!(
            "Snapshot history schema {} is not supported by this app version.",
            catalog.schema_version
        ));
    }
    Ok(catalog)
}

pub fn list_project_snapshots(
    storage_dir: &Path,
    project_id: &str,
) -> Result<ProjectSnapshots, String> {
    let catalog = read_catalog(storage_dir, project_id)?;
    Ok(ProjectSnapshots {
        project_id: project_id.to_owned(),
        snapshots: catalog.snapshots,
    })
}

pub fn load_snapshot_manifest(
    storage_dir: &Path,
    project_id: &str,
    snapshot_id: &str,
) -> Result<SnapshotManifest, String> {
    let catalog = read_catalog(storage_dir, project_id)?;
    if !catalog
        .snapshots
        .iter()
        .any(|snapshot| snapshot.snapshot_id == snapshot_id)
    {
        return Err("This snapshot does not belong to the selected repository.".to_owned());
    }
    let bytes = fs::read(manifest_path(storage_dir, project_id, snapshot_id))
        .map_err(|error| format!("Snapshot details could not be read: {error}"))?;
    let manifest: SnapshotManifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Snapshot details are invalid: {error}"))?;
    if manifest.schema_version != 1 && manifest.schema_version != 2 {
        return Err(format!(
            "Snapshot manifest schema {} is not supported by this app version.",
            manifest.schema_version
        ));
    }
    if manifest.summary.project_id != project_id || manifest.summary.snapshot_id != snapshot_id {
        return Err("Snapshot identity does not match its catalog entry.".to_owned());
    }
    Ok(manifest)
}

pub fn read_snapshot_source(
    storage_dir: &Path,
    project_id: &str,
    snapshot_id: &str,
    relative_path: &str,
) -> Result<SnapshotSource, String> {
    let manifest = load_snapshot_manifest(storage_dir, project_id, snapshot_id)?;
    read_snapshot_source_from_manifest(storage_dir, &manifest, relative_path)
}

pub fn read_snapshot_source_from_manifest(
    storage_dir: &Path,
    manifest: &SnapshotManifest,
    relative_path: &str,
) -> Result<SnapshotSource, String> {
    let file = manifest
        .files
        .iter()
        .find(|file| file.path == relative_path)
        .ok_or_else(|| "This path was not captured in the selected snapshot.".to_owned())?;
    if file.sha256.len() != 64 || !file.sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("The snapshot contains an invalid content hash for this path.".to_owned());
    }
    let bytes = fs::read(blob_absolute_path(storage_dir, &file.sha256)).map_err(|error| {
        format!("Captured source bytes are unavailable in local snapshot storage: {error}")
    })?;
    if bytes.len() as u64 != file.size_bytes || sha256(&bytes) != file.sha256 {
        return Err("Captured source failed its size or SHA-256 integrity check.".to_owned());
    }
    if file.content_kind == CapturedContentKind::Binary {
        return Err("This captured file is binary and cannot be opened as source text.".to_owned());
    }
    let contents = String::from_utf8(bytes)
        .map_err(|_| "This captured file is not valid UTF-8 source text.".to_owned())?;
    Ok(SnapshotSource {
        snapshot_id: manifest.summary.snapshot_id.clone(),
        path: file.path.clone(),
        sha256: file.sha256.clone(),
        size_bytes: file.size_bytes,
        contents,
    })
}

fn file_fingerprint(metadata: &Metadata) -> FileFingerprint {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        FileFingerprint {
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanos: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanos: metadata.ctime_nsec(),
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
        }
    }
    #[cfg(not(unix))]
    {
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .unwrap_or_default();
        FileFingerprint {
            length: metadata.len(),
            modified_seconds: modified.as_secs() as i64,
            modified_nanos: modified.subsec_nanos() as i64,
            changed_seconds: 0,
            changed_nanos: 0,
            device: 0,
            inode: 0,
            mode: 0,
        }
    }
}

fn metadata_no_follow(path: &Path) -> io::Result<Metadata> {
    fs::symlink_metadata(path)
}

fn relative_name(root: &Path, path: &Path) -> Option<String> {
    path.strip_prefix(root)
        .ok()?
        .components()
        .map(|component| component.as_os_str().to_str())
        .collect::<Option<Vec<_>>>()
        .map(|parts| parts.join("/"))
}

fn escaped_relative_name(root: &Path, path: &Path) -> String {
    let relative = path.strip_prefix(root).unwrap_or(path);
    #[cfg(unix)]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        relative.as_os_str().as_bytes()
    };
    #[cfg(not(unix))]
    let owned_bytes = relative.to_string_lossy().as_bytes().to_vec();
    #[cfg(not(unix))]
    let bytes = owned_bytes.as_slice();
    let mut escaped = String::with_capacity(bytes.len());
    for byte in bytes {
        if byte.is_ascii_alphanumeric() || matches!(*byte, b'/' | b'.' | b'_' | b'-') {
            escaped.push(*byte as char);
        } else {
            escaped.push_str(&format!("%{byte:02X}"));
        }
    }
    escaped
}

fn skipped(path: impl Into<String>, reason: &str, details: impl Into<String>) -> SkippedPath {
    skipped_with_kind(path, InventoryPathKind::Unknown, None, reason, details)
}

fn skipped_with_kind(
    path: impl Into<String>,
    kind: InventoryPathKind,
    size_bytes: Option<u64>,
    reason: &str,
    details: impl Into<String>,
) -> SkippedPath {
    SkippedPath {
        path: path.into(),
        kind,
        size_bytes,
        reason: reason.to_owned(),
        details: details.into(),
    }
}

fn remainder_path(directory: &str) -> String {
    if directory.is_empty() {
        "…".to_owned()
    } else {
        format!("{directory}/…")
    }
}

fn kind_from_metadata(metadata: &Metadata) -> InventoryPathKind {
    if metadata.file_type().is_symlink() {
        InventoryPathKind::Symlink
    } else if metadata.is_dir() {
        InventoryPathKind::Directory
    } else if metadata.is_file() {
        InventoryPathKind::File
    } else {
        InventoryPathKind::Other
    }
}

fn classify_content(bytes: &[u8]) -> CapturedContentKind {
    if bytes.contains(&0) {
        CapturedContentKind::Binary
    } else if std::str::from_utf8(bytes).is_ok() {
        CapturedContentKind::Text
    } else {
        CapturedContentKind::InvalidEncoding
    }
}

fn looks_secret_like(path: &str, bytes: &[u8]) -> bool {
    let path = path.to_ascii_lowercase();
    let file_name = path.rsplit('/').next().unwrap_or(&path);
    if file_name == ".env"
        || file_name.starts_with(".env.")
        || [
            "credentials",
            "secret",
            "token",
            "id_rsa",
            "id_ed25519",
            ".pem",
            ".p12",
            ".pfx",
            ".key",
        ]
        .iter()
        .any(|needle| path.contains(needle))
    {
        return true;
    }
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    [
        "-----begin private key-----",
        "-----begin rsa private key-----",
        "-----begin ec private key-----",
        "akia",
        "ghp_",
        "github_pat_",
        "AIza",
        "sk_live_",
    ]
    .iter()
    .any(|needle| text.contains(&needle.to_ascii_lowercase()))
}

fn is_expected_exclusion(reason: &str) -> bool {
    matches!(
        reason,
        "excludedByPolicy"
            | "ignoredByGit"
            | "symlink"
            | "nestedRepository"
            | "gitMetadata"
            | "nonRegularFile"
    )
}

fn git_output(root: &Path, args: &[&str]) -> Result<std::process::Output, String> {
    crate::project::git_command()
        .arg("--no-optional-locks")
        .args(args)
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .map_err(|error| {
            format!(
                "Git could not inspect repository state{}: {error}",
                if error.kind() == io::ErrorKind::NotFound {
                    " (Git was not found)"
                } else {
                    ""
                }
            )
        })
}

fn git_optional(root: &Path, args: &[&str]) -> Result<Option<String>, String> {
    let output = git_output(root, args)?;
    if !output.status.success() {
        return Ok(None);
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    Ok((!value.is_empty()).then_some(value))
}

fn git_change_state(
    root: &Path,
) -> Result<
    (
        Option<String>,
        Option<String>,
        GitChangeState,
        GitChangeState,
    ),
    String,
> {
    let branch = git_optional(root, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    let head = git_optional(root, &["rev-parse", "--verify", "HEAD"])?;
    let output = git_output(
        root,
        &[
            "-c",
            "core.fsmonitor=false",
            "-c",
            "core.untrackedCache=false",
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=normal",
            "--ignore-submodules=all",
        ],
    )?;
    if !output.status.success() {
        return Err(format!(
            "Git could not read the repository change state: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let mut index_changed = false;
    let mut worktree_changed = false;
    // Porcelain v1 -z starts each record with its two status bytes. NUL safely
    // separates paths that contain spaces, quotes, tabs, or newlines.
    let mut records = output.stdout.split(|byte| *byte == 0);
    while let Some(record) = records.next() {
        if record.len() < 2 {
            continue;
        }
        let x = record[0];
        let y = record[1];
        if x == b'?' && y == b'?' {
            worktree_changed = true;
        } else {
            index_changed |= x != b' ';
            worktree_changed |= y != b' ';
        }
        // Rename/copy records carry a second NUL-delimited destination path.
        if x == b'R' || x == b'C' || y == b'R' || y == b'C' {
            let _ = records.next();
        }
    }
    Ok((
        branch,
        head,
        if index_changed {
            GitChangeState::Modified
        } else {
            GitChangeState::Clean
        },
        if worktree_changed {
            GitChangeState::Modified
        } else {
            GitChangeState::Clean
        },
    ))
}

fn ignored_paths(root: &Path, paths: &[String]) -> Result<HashSet<String>, String> {
    if paths.is_empty() {
        return Ok(HashSet::new());
    }
    let mut child = crate::project::git_command()
        .arg("--no-optional-locks")
        .args(["check-ignore", "-z", "-v", "--stdin"])
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Git could not check ignore rules: {error}"))?;
    let mut stdout = child.stdout.take().expect("piped stdout");
    let mut stderr = child.stderr.take().expect("piped stderr");
    // Drain both output pipes while writing stdin. A repository with many
    // ignored paths can otherwise fill stdout and deadlock the writer.
    let stdout_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout.read_to_end(&mut bytes);
        (bytes, result)
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stderr.read_to_end(&mut bytes);
        (bytes, result)
    });
    let mut stdin = child.stdin.take().expect("piped stdin");
    for path in paths {
        stdin
            .write_all(path.as_bytes())
            .and_then(|_| stdin.write_all(&[0]))
            .map_err(|error| format!("Git ignore rules could not be checked: {error}"))?;
    }
    drop(stdin);
    let status = child
        .wait()
        .map_err(|error| format!("Git ignore check did not finish: {error}"))?;
    let (bytes, stdout_result) = stdout_reader
        .join()
        .map_err(|_| "Git ignore check output could not be read.".to_owned())?;
    let (stderr_bytes, stderr_result) = stderr_reader
        .join()
        .map_err(|_| "Git ignore check error output could not be read.".to_owned())?;
    stdout_result.map_err(|error| format!("Git ignore check output failed: {error}"))?;
    stderr_result.map_err(|error| format!("Git ignore check error output failed: {error}"))?;
    if !status.success() && status.code() != Some(1) {
        return Err(format!(
            "Git could not check ignore rules: {}",
            String::from_utf8_lossy(&stderr_bytes).trim()
        ));
    }
    let fields: Vec<&[u8]> = bytes.split(|byte| *byte == 0).collect();
    let mut ignored = HashSet::new();
    for record in fields.chunks_exact(4) {
        if let Ok(path) = std::str::from_utf8(record[3]) {
            ignored.insert(path.to_owned());
        }
    }
    Ok(ignored)
}

fn directory_has_tracked_paths(root: &Path, relative_path: &str) -> Result<bool, String> {
    // A directory can match an ignore rule even when it contains force-tracked
    // files. Keep walking only those ignored folders that Git's index says
    // contain tracked paths; untracked ignored descendants are still filtered
    // individually by check-ignore.
    let prefix = format!("{relative_path}/");
    let output = git_output(
        root,
        &["--literal-pathspecs", "ls-files", "-z", "--", &prefix],
    )?;
    if !output.status.success() {
        return Err(format!(
            "Git could not check tracked files under {relative_path}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(!output.stdout.is_empty())
}

fn send_progress<F>(progress: &mut SnapshotProgress, callback: &mut F)
where
    F: FnMut(SnapshotProgress),
{
    callback(progress.clone());
}

fn scan_repository<F>(
    root: &Path,
    snapshot_id: &str,
    project_id: &str,
    parent_snapshot_id: Option<String>,
    limits: CaptureLimits,
    cancel: &AtomicBool,
    progress: &mut SnapshotProgress,
    callback: &mut F,
) -> Result<ScanResult, CaptureError>
where
    F: FnMut(SnapshotProgress),
{
    let excluded: HashSet<&str> = limits
        .excluded_directories
        .iter()
        .map(String::as_str)
        .collect();
    let mut queue = VecDeque::from([root.to_path_buf()]);
    let mut candidates = Vec::new();
    let mut directories = Vec::new();
    let mut skipped_paths = Vec::new();
    let mut paths_visited = 0_u64;
    let mut stop = false;

    while !queue.is_empty() && !stop {
        if cancel.load(Ordering::Acquire) {
            return Err(CaptureError::Cancelled);
        }
        let level_count = queue.len();
        let mut level_candidates = Vec::new();
        for _ in 0..level_count {
            if stop {
                break;
            }
            let directory = queue.pop_front().expect("known non-empty queue");
            let directory_relative = if directory == root {
                String::new()
            } else {
                relative_name(root, &directory).unwrap_or_default()
            };
            let directory_metadata = metadata_no_follow(&directory).map_err(|error| {
                CaptureError::Failed(format!(
                    "A repository folder could not be inspected: {error}"
                ))
            })?;
            if !directory_metadata.is_dir() {
                return Err(CaptureError::Inconsistent {
                    path: Some(directory_relative),
                    message: "A folder changed type during the initial scan.".to_owned(),
                });
            }
            directories.push((
                directory.clone(),
                file_fingerprint(&directory_metadata),
                directory_relative.clone(),
            ));

            let read_dir = match fs::read_dir(&directory) {
                Ok(read_dir) => read_dir,
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                    skipped_paths.push(skipped_with_kind(
                        if directory_relative.is_empty() {
                            ".".to_owned()
                        } else {
                            directory_relative
                        },
                        InventoryPathKind::Directory,
                        None,
                        "unreadableDirectory",
                        error.to_string(),
                    ));
                    continue;
                }
                Err(error) => {
                    return Err(CaptureError::Failed(format!(
                        "A repository folder could not be enumerated: {error}"
                    )));
                }
            };
            let mut entries = Vec::new();
            let mut iterator = read_dir;
            let remaining_paths = limits.max_paths.saturating_sub(paths_visited);
            if remaining_paths == 0 {
                if iterator.next().is_some() {
                    skipped_paths.push(skipped_with_kind(
                        remainder_path(&directory_relative),
                        InventoryPathKind::Unknown,
                        None,
                        "pathLimit",
                        format!(
                            "The capture reached its limit of {} visited paths.",
                            limits.max_paths
                        ),
                    ));
                    stop = true;
                }
                continue;
            }
            let directory_allowance =
                limits.max_entries_per_directory.min(remaining_paths) as usize;
            for _ in 0..directory_allowance {
                match iterator.next() {
                    Some(Ok(entry)) => entries.push(entry),
                    Some(Err(error)) if error.kind() == io::ErrorKind::PermissionDenied => {
                        skipped_paths.push(skipped_with_kind(
                            remainder_path(&directory_relative),
                            InventoryPathKind::Unknown,
                            None,
                            "unreadableDirectoryEntry",
                            error.to_string(),
                        ));
                        break;
                    }
                    Some(Err(error)) => {
                        return Err(CaptureError::Failed(format!(
                            "A repository directory entry could not be read: {error}"
                        )));
                    }
                    None => break,
                }
            }
            let has_more_entries = match iterator.next() {
                Some(Ok(_)) => true,
                Some(Err(error)) if error.kind() == io::ErrorKind::PermissionDenied => {
                    skipped_paths.push(skipped_with_kind(
                        remainder_path(&directory_relative),
                        InventoryPathKind::Unknown,
                        None,
                        "unreadableDirectoryEntry",
                        error.to_string(),
                    ));
                    false
                }
                Some(Err(error)) => {
                    return Err(CaptureError::Failed(format!(
                        "A repository directory entry could not be read: {error}"
                    )));
                }
                None => false,
            };
            entries.sort_by_key(|entry| entry.file_name());
            if has_more_entries && remaining_paths <= limits.max_entries_per_directory {
                skipped_paths.push(skipped_with_kind(
                    remainder_path(&directory_relative),
                    InventoryPathKind::Unknown,
                    None,
                    "pathLimit",
                    format!(
                        "The capture reached its limit of {} visited paths.",
                        limits.max_paths
                    ),
                ));
                stop = true;
            } else if has_more_entries {
                skipped_paths.push(skipped_with_kind(
                    remainder_path(&directory_relative),
                    InventoryPathKind::Unknown,
                    None,
                    "directoryEntryLimit",
                    format!(
                        "Only the first {} entries in this folder were inspected.",
                        limits.max_entries_per_directory
                    ),
                ));
            }

            for entry in entries {
                if cancel.load(Ordering::Acquire) {
                    return Err(CaptureError::Cancelled);
                }
                paths_visited += 1;
                progress.paths_visited = paths_visited;
                let absolute = entry.path();
                let relative = match relative_name(root, &absolute) {
                    Some(relative) => relative,
                    None => {
                        let escaped = escaped_relative_name(root, &absolute);
                        let (kind, size) = metadata_no_follow(&absolute)
                            .map(|metadata| {
                                let kind = kind_from_metadata(&metadata);
                                (kind, metadata.is_file().then_some(metadata.len()))
                            })
                            .unwrap_or((InventoryPathKind::Unknown, None));
                        skipped_paths.push(skipped_with_kind(
                            escaped,
                            kind,
                            size,
                            "nonUtf8Path",
                            "The path name is not Unicode; its raw bytes are shown as percent escapes.",
                        ));
                        continue;
                    }
                };
                let metadata = match metadata_no_follow(&absolute) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                        skipped_paths.push(skipped(relative, "unreadablePath", error.to_string()));
                        continue;
                    }
                    Err(error) => {
                        return Err(CaptureError::Failed(format!(
                            "Path metadata could not be read: {error}"
                        )));
                    }
                };
                let name = entry.file_name();
                let name = name.to_string_lossy();
                let kind = kind_from_metadata(&metadata);
                if relative.len() > limits.max_path_bytes {
                    skipped_paths.push(skipped_with_kind(
                        relative,
                        kind.clone(),
                        metadata.is_file().then_some(metadata.len()),
                        "pathByteLimit",
                        format!("The path exceeds {} UTF-8 bytes.", limits.max_path_bytes),
                    ));
                    continue;
                }
                if name == ".git" {
                    skipped_paths.push(skipped_with_kind(
                        relative,
                        kind,
                        metadata.is_file().then_some(metadata.len()),
                        "gitMetadata",
                        "Git metadata is never captured.",
                    ));
                    continue;
                }
                if metadata.file_type().is_symlink() {
                    skipped_paths.push(skipped_with_kind(
                        relative,
                        InventoryPathKind::Symlink,
                        None,
                        "symlink",
                        "Symlinks and their targets are excluded.",
                    ));
                    continue;
                }
                let is_directory = metadata.is_dir();
                if is_directory {
                    if excluded.contains(name.as_ref()) {
                        skipped_paths.push(skipped_with_kind(
                            relative,
                            InventoryPathKind::Directory,
                            None,
                            "excludedByPolicy",
                            "This directory is excluded by the visible capture policy.",
                        ));
                        continue;
                    }
                    match metadata_no_follow(&absolute.join(".git")) {
                        Ok(_) => {
                            skipped_paths.push(skipped_with_kind(
                                relative,
                                InventoryPathKind::Directory,
                                None,
                                "nestedRepository",
                                "Nested repositories are excluded.",
                            ));
                            continue;
                        }
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                        Err(error) => {
                            skipped_paths.push(skipped_with_kind(
                                relative,
                                InventoryPathKind::Directory,
                                None,
                                "uninspectedDirectory",
                                format!(
                                    "The nested-repository marker could not be checked: {error}"
                                ),
                            ));
                            continue;
                        }
                    }
                } else if !metadata.is_file() {
                    skipped_paths.push(skipped_with_kind(
                        relative,
                        InventoryPathKind::Other,
                        Some(metadata.len()),
                        "nonRegularFile",
                        "Only regular files are captured.",
                    ));
                    continue;
                }
                level_candidates.push(Candidate {
                    absolute_path: absolute,
                    relative_path: relative,
                    is_directory,
                    fingerprint: file_fingerprint(&metadata),
                });
            }
        }
        if level_candidates.is_empty() {
            continue;
        }
        progress.phase = CapturePhase::CheckingIgnoreRules;
        progress.current_path = None;
        send_progress(progress, callback);
        let paths = level_candidates
            .iter()
            .map(|candidate| candidate.relative_path.clone())
            .collect::<Vec<_>>();
        let ignored = ignored_paths(root, &paths).map_err(CaptureError::Failed)?;
        for candidate in level_candidates {
            if ignored.contains(&candidate.relative_path) {
                if candidate.is_directory
                    && directory_has_tracked_paths(root, &candidate.relative_path)
                        .map_err(CaptureError::Failed)?
                {
                    if stop {
                        skipped_paths.push(skipped_with_kind(
                            candidate.relative_path,
                            InventoryPathKind::Directory,
                            None,
                            "pathLimit",
                            "This folder was not descended into because the repository path limit was reached.",
                        ));
                    } else {
                        queue.push_back(candidate.absolute_path);
                    }
                } else {
                    skipped_paths.push(skipped_with_kind(
                        candidate.relative_path,
                        if candidate.is_directory {
                            InventoryPathKind::Directory
                        } else {
                            InventoryPathKind::File
                        },
                        (!candidate.is_directory).then_some(candidate.fingerprint.length),
                        "ignoredByGit",
                        "Git ignore rules exclude this path; tracked files remain eligible.",
                    ));
                }
                continue;
            }
            if candidate.is_directory {
                if stop {
                    skipped_paths.push(skipped_with_kind(
                        candidate.relative_path,
                        InventoryPathKind::Directory,
                        None,
                        "pathLimit",
                        "This folder was not descended into because the repository path limit was reached.",
                    ));
                } else {
                    queue.push_back(candidate.absolute_path);
                }
            } else {
                candidates.push(candidate);
            }
        }
    }

    candidates.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    progress.phase = CapturePhase::ReadingFiles;
    progress.paths_visited = paths_visited;
    progress.candidate_files = candidates.len();
    progress.paths_skipped = skipped_paths.len();
    progress.current_path = None;
    send_progress(progress, callback);
    let _ = (snapshot_id, project_id, parent_snapshot_id);
    Ok(ScanResult {
        candidates,
        directories,
        skipped: skipped_paths,
        paths_visited,
        limits,
    })
}

fn open_file_no_follow(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    options.open(path)
}

fn status_from_error(error: &CaptureError) -> CaptureStatus {
    match error {
        CaptureError::Cancelled => CaptureStatus::Cancelled,
        CaptureError::Inconsistent { .. } => CaptureStatus::Inconsistent,
        CaptureError::Failed(_) => CaptureStatus::Failed,
    }
}

fn fail_progress<F>(
    progress: &mut SnapshotProgress,
    status: CaptureStatus,
    message: String,
    callback: &mut F,
) where
    F: FnMut(SnapshotProgress),
{
    progress.status = status;
    progress.message = Some(message);
    progress.current_path = None;
    send_progress(progress, callback);
}

#[cfg(test)]
fn capture_snapshot<F>(
    storage_dir: &Path,
    project: &RecentProject,
    cancel: &AtomicBool,
    callback: &mut F,
) -> Result<SnapshotManifest, CaptureError>
where
    F: FnMut(SnapshotProgress),
{
    let progress = new_capture_progress(storage_dir, project, CaptureLimits::default())
        .map_err(CaptureError::Failed)?;
    capture_snapshot_with_progress(storage_dir, project, progress, cancel, callback)
}

pub fn new_capture_progress(
    storage_dir: &Path,
    project: &RecentProject,
    limits: CaptureLimits,
) -> Result<SnapshotProgress, String> {
    let catalog = read_catalog(storage_dir, &project.project_id)?;
    let parent_snapshot_id = catalog
        .snapshots
        .last()
        .map(|snapshot| snapshot.snapshot_id.clone());
    Ok(SnapshotProgress {
        project_id: project.project_id.clone(),
        snapshot_id: new_snapshot_id(),
        parent_snapshot_id,
        status: CaptureStatus::Running,
        phase: CapturePhase::Scanning,
        paths_visited: 0,
        candidate_files: 0,
        files_processed: 0,
        files_captured: 0,
        paths_skipped: 0,
        bytes_read: 0,
        current_path: None,
        message: None,
        limits,
        summary: None,
    })
}

pub fn capture_snapshot_with_progress<F>(
    storage_dir: &Path,
    project: &RecentProject,
    progress: SnapshotProgress,
    cancel: &AtomicBool,
    callback: &mut F,
) -> Result<SnapshotManifest, CaptureError>
where
    F: FnMut(SnapshotProgress),
{
    capture_snapshot_with_chunk_hook(
        storage_dir,
        project,
        progress,
        cancel,
        callback,
        &mut |_, _| {},
    )
}

fn capture_snapshot_with_chunk_hook<F, H>(
    storage_dir: &Path,
    project: &RecentProject,
    mut progress: SnapshotProgress,
    cancel: &AtomicBool,
    callback: &mut F,
    after_chunk: &mut H,
) -> Result<SnapshotManifest, CaptureError>
where
    F: FnMut(SnapshotProgress),
    H: FnMut(&Path, u64),
{
    let snapshot_id = progress.snapshot_id.clone();
    let repository_root = PathBuf::from(&project.repository_root);
    let initial_git = git_change_state(&repository_root).map_err(CaptureError::Failed)?;
    let catalog = read_catalog(storage_dir, &project.project_id).map_err(CaptureError::Failed)?;
    let parent_snapshot_id = progress.parent_snapshot_id.clone();
    if catalog
        .snapshots
        .last()
        .map(|snapshot| snapshot.snapshot_id.clone())
        != parent_snapshot_id
    {
        return Err(CaptureError::Failed("Snapshot history changed before this capture began. Start the capture again to preserve a clear parent revision.".to_owned()));
    }
    let limits = progress.limits.clone();
    send_progress(&mut progress, callback);

    let scan = match scan_repository(
        &repository_root,
        &snapshot_id,
        &project.project_id,
        parent_snapshot_id.clone(),
        limits,
        cancel,
        &mut progress,
        callback,
    ) {
        Ok(scan) => scan,
        Err(error) => {
            fail_progress(
                &mut progress,
                status_from_error(&error),
                format_capture_error(&error),
                callback,
            );
            return Err(error);
        }
    };
    let staging_path = storage_dir.join("staging").join(&snapshot_id);
    fs::create_dir_all(&staging_path).map_err(|error| {
        CaptureError::Failed(format!("Snapshot staging could not be created: {error}"))
    })?;
    let staging = StagingDirectory {
        path: staging_path.clone(),
    };
    let mut captured = Vec::new();
    let mut staged = Vec::new();
    let mut skipped_paths = scan.skipped;
    let mut bytes_read = 0_u64;

    for (index, candidate) in scan.candidates.iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            let error = CaptureError::Cancelled;
            fail_progress(
                &mut progress,
                CaptureStatus::Cancelled,
                format_capture_error(&error),
                callback,
            );
            return Err(error);
        }
        progress.phase = CapturePhase::ReadingFiles;
        progress.current_path = Some(candidate.relative_path.clone());
        progress.files_processed = index;
        progress.files_captured = captured.len();
        progress.paths_skipped = skipped_paths.len();
        send_progress(&mut progress, callback);
        if candidate.fingerprint.length > scan.limits.max_file_bytes {
            skipped_paths.push(skipped_with_kind(
                candidate.relative_path.clone(),
                InventoryPathKind::File,
                Some(candidate.fingerprint.length),
                "fileByteLimit",
                format!(
                    "The file is {} bytes; the per-file limit is {} bytes.",
                    candidate.fingerprint.length, scan.limits.max_file_bytes
                ),
            ));
            continue;
        }
        if bytes_read.saturating_add(candidate.fingerprint.length) > scan.limits.max_total_bytes {
            skipped_paths.push(skipped_with_kind(
                candidate.relative_path.clone(),
                InventoryPathKind::File,
                Some(candidate.fingerprint.length),
                "totalByteLimit",
                format!(
                    "Capturing this file would exceed the {} byte snapshot limit.",
                    scan.limits.max_total_bytes
                ),
            ));
            continue;
        }
        let path_before = match metadata_no_follow(&candidate.absolute_path) {
            Ok(metadata) => file_fingerprint(&metadata),
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                skipped_paths.push(skipped_with_kind(
                    candidate.relative_path.clone(),
                    InventoryPathKind::File,
                    Some(candidate.fingerprint.length),
                    "unreadableFile",
                    error.to_string(),
                ));
                continue;
            }
            Err(error) => {
                return Err(CaptureError::Inconsistent {
                    path: Some(candidate.relative_path.clone()),
                    message: format!("The path changed before it could be read: {error}"),
                });
            }
        };
        if path_before != candidate.fingerprint {
            let error = CaptureError::Inconsistent {
                path: Some(candidate.relative_path.clone()),
                message: "The file changed after the repository scan began.".to_owned(),
            };
            fail_progress(
                &mut progress,
                CaptureStatus::Inconsistent,
                format_capture_error(&error),
                callback,
            );
            return Err(error);
        }
        let mut input = match open_file_no_follow(&candidate.absolute_path) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                skipped_paths.push(skipped_with_kind(
                    candidate.relative_path.clone(),
                    InventoryPathKind::File,
                    Some(candidate.fingerprint.length),
                    "unreadableFile",
                    error.to_string(),
                ));
                continue;
            }
            #[cfg(unix)]
            Err(error) if error.raw_os_error() == Some(libc::ELOOP) => {
                skipped_paths.push(skipped_with_kind(
                    candidate.relative_path.clone(),
                    InventoryPathKind::Symlink,
                    None,
                    "symlink",
                    "The path became a symlink before it could be opened.",
                ));
                continue;
            }
            Err(error) => {
                return Err(CaptureError::Failed(format!(
                    "A captured file could not be opened: {error}"
                )));
            }
        };
        let opened_before = file_fingerprint(&input.metadata().map_err(|error| {
            CaptureError::Failed(format!("Opened file metadata could not be read: {error}"))
        })?);
        if opened_before != candidate.fingerprint {
            let error = CaptureError::Inconsistent {
                path: Some(candidate.relative_path.clone()),
                message: "The file identity changed while it was being opened.".to_owned(),
            };
            fail_progress(
                &mut progress,
                CaptureStatus::Inconsistent,
                format_capture_error(&error),
                callback,
            );
            return Err(error);
        }
        let staged_path = staging_path.join(format!("{index}.blob"));
        let mut output = File::create(&staged_path).map_err(|error| {
            CaptureError::Failed(format!(
                "Snapshot staging file could not be created: {error}"
            ))
        })?;
        let mut hasher = Sha256::new();
        let mut file_bytes = 0_u64;
        let mut file_contents = Vec::with_capacity(candidate.fingerprint.length as usize);
        let mut buffer = [0_u8; READ_CHUNK_BYTES];
        loop {
            if cancel.load(Ordering::Acquire) {
                let error = CaptureError::Cancelled;
                fail_progress(
                    &mut progress,
                    CaptureStatus::Cancelled,
                    format_capture_error(&error),
                    callback,
                );
                return Err(error);
            }
            let count = input.read(&mut buffer).map_err(|error| {
                if error.kind() == io::ErrorKind::PermissionDenied {
                    CaptureError::Failed(format!(
                        "Permission was revoked while reading {}.",
                        candidate.relative_path
                    ))
                } else {
                    CaptureError::Failed(format!(
                        "{} could not be read: {error}",
                        candidate.relative_path
                    ))
                }
            })?;
            if count == 0 {
                break;
            }
            file_bytes = file_bytes.saturating_add(count as u64);
            if file_bytes > candidate.fingerprint.length || file_bytes > scan.limits.max_file_bytes
            {
                let error = CaptureError::Inconsistent {
                    path: Some(candidate.relative_path.clone()),
                    message: "The file grew beyond its scanned size during capture.".to_owned(),
                };
                fail_progress(
                    &mut progress,
                    CaptureStatus::Inconsistent,
                    format_capture_error(&error),
                    callback,
                );
                return Err(error);
            }
            output.write_all(&buffer[..count]).map_err(|error| {
                CaptureError::Failed(format!(
                    "Snapshot staging bytes could not be written: {error}"
                ))
            })?;
            hasher.update(&buffer[..count]);
            file_contents.extend_from_slice(&buffer[..count]);
            after_chunk(&candidate.absolute_path, file_bytes);
        }
        output.sync_all().map_err(|error| {
            CaptureError::Failed(format!(
                "Snapshot staging bytes could not be synced: {error}"
            ))
        })?;
        let opened_after = file_fingerprint(&input.metadata().map_err(|error| {
            CaptureError::Failed(format!(
                "Opened file metadata could not be checked: {error}"
            ))
        })?);
        let path_after = metadata_no_follow(&candidate.absolute_path)
            .map(|metadata| file_fingerprint(&metadata));
        if file_bytes != candidate.fingerprint.length
            || opened_after != candidate.fingerprint
            || path_after.as_ref().ok() != Some(&candidate.fingerprint)
        {
            let error = CaptureError::Inconsistent {
                path: Some(candidate.relative_path.clone()),
                message: "The file changed while it was being read; no snapshot was committed."
                    .to_owned(),
            };
            fail_progress(
                &mut progress,
                CaptureStatus::Inconsistent,
                format_capture_error(&error),
                callback,
            );
            return Err(error);
        }
        let hash_bytes = hasher.finalize();
        let hash = hash_bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let blob_path = blob_relative_path(&hash);
        captured.push(CapturedFile {
            path: candidate.relative_path.clone(),
            kind: InventoryPathKind::File,
            size_bytes: file_bytes,
            sha256: hash.clone(),
            blob_path,
            content_kind: classify_content(&file_contents),
            secret_like: looks_secret_like(&candidate.relative_path, &file_contents),
        });
        staged.push((staged_path, hash));
        bytes_read = bytes_read.saturating_add(file_bytes);
        progress.bytes_read = bytes_read;
        progress.files_captured = captured.len();
        progress.files_processed = index + 1;
    }

    for (directory, fingerprint, relative) in &scan.directories {
        let after = metadata_no_follow(directory).map(|metadata| file_fingerprint(&metadata));
        if after.as_ref().ok() != Some(fingerprint) {
            let error = CaptureError::Inconsistent { path: Some(if relative.is_empty() { ".".to_owned() } else { relative.clone() }), message: "A repository directory changed during snapshot capture; no snapshot was committed.".to_owned() };
            fail_progress(
                &mut progress,
                CaptureStatus::Inconsistent,
                format_capture_error(&error),
                callback,
            );
            return Err(error);
        }
    }
    if cancel.load(Ordering::Acquire) {
        let error = CaptureError::Cancelled;
        fail_progress(
            &mut progress,
            CaptureStatus::Cancelled,
            format_capture_error(&error),
            callback,
        );
        return Err(error);
    }
    let final_git = git_change_state(&repository_root).map_err(CaptureError::Failed)?;
    if initial_git != final_git {
        let error = CaptureError::Inconsistent { path: None, message: "Git HEAD, branch, index, or worktree state changed during capture; no snapshot was committed.".to_owned() };
        fail_progress(
            &mut progress,
            CaptureStatus::Inconsistent,
            format_capture_error(&error),
            callback,
        );
        return Err(error);
    }

    progress.phase = CapturePhase::Finalizing;
    progress.current_path = None;
    send_progress(&mut progress, callback);
    let objects_dir = storage_dir.join("objects").join("sha256");
    fs::create_dir_all(&objects_dir).map_err(|error| {
        CaptureError::Failed(format!(
            "Snapshot object storage could not be created: {error}"
        ))
    })?;
    for (staged_path, hash) in staged {
        if cancel.load(Ordering::Acquire) {
            let error = CaptureError::Cancelled;
            fail_progress(
                &mut progress,
                CaptureStatus::Cancelled,
                format_capture_error(&error),
                callback,
            );
            return Err(error);
        }
        let blob_path = blob_absolute_path(storage_dir, &hash);
        if blob_path.exists() {
            let existing = fs::read(&blob_path).map_err(|error| {
                CaptureError::Failed(format!(
                    "An existing snapshot object could not be verified: {error}"
                ))
            })?;
            if sha256(&existing) != hash {
                return Err(CaptureError::Failed(
                    "An existing content-addressed snapshot object did not match its hash."
                        .to_owned(),
                ));
            }
            fs::remove_file(staged_path).map_err(|error| {
                CaptureError::Failed(format!(
                    "Snapshot staging object could not be removed: {error}"
                ))
            })?;
        } else {
            let parent = blob_path
                .parent()
                .expect("content addressed blob has parent");
            fs::create_dir_all(parent).map_err(|error| {
                CaptureError::Failed(format!(
                    "Snapshot object folder could not be created: {error}"
                ))
            })?;
            match fs::rename(&staged_path, &blob_path) {
                Ok(()) => {}
                Err(error) if blob_path.exists() => {
                    let existing = fs::read(&blob_path).map_err(|read_error| CaptureError::Failed(format!("Snapshot object publication failed ({error}) and the existing object could not be read: {read_error}")))?;
                    if sha256(&existing) != hash {
                        return Err(CaptureError::Failed(
                            "A concurrent snapshot object did not match its content hash."
                                .to_owned(),
                        ));
                    }
                    let _ = fs::remove_file(&staged_path);
                }
                Err(error) => {
                    return Err(CaptureError::Failed(format!(
                        "Snapshot object could not be finalized: {error}"
                    )));
                }
            }
        }
    }

    let created_at_unix_ms = unix_time_ms();
    let summary = SnapshotSummary {
        snapshot_id: snapshot_id.clone(),
        project_id: project.project_id.clone(),
        parent_snapshot_id: parent_snapshot_id.clone(),
        created_at_unix_ms,
        branch: initial_git.0.clone(),
        head: initial_git.1.clone(),
        index_state: initial_git.2.clone(),
        worktree_state: initial_git.3.clone(),
        status: if skipped_paths
            .iter()
            .any(|item| !is_expected_exclusion(&item.reason))
        {
            CaptureStatus::Partial
        } else {
            CaptureStatus::Complete
        },
        files_captured: captured.len(),
        paths_skipped: skipped_paths.len(),
        total_bytes: bytes_read,
        paths_visited: scan.paths_visited,
        policy_version: scan.limits.policy_version.clone(),
    };
    let skipped_directory_paths = skipped_paths
        .iter()
        .filter(|path| path.kind == InventoryPathKind::Directory)
        .map(|path| path.path.as_str())
        .collect::<HashSet<_>>();
    let inventory_directories = scan
        .directories
        .iter()
        .filter_map(|(_, _, path)| {
            let path = if path.is_empty() { "." } else { path.as_str() };
            (!skipped_directory_paths.contains(path)).then(|| InventoryDirectory {
                path: path.to_owned(),
                kind: InventoryPathKind::Directory,
                size_bytes: None,
            })
        })
        .collect();
    let manifest = SnapshotManifest {
        schema_version: 2,
        summary: summary.clone(),
        repository_root: project.repository_root.clone(),
        selected_path: project.selected_path.clone(),
        capture_limits: scan.limits,
        directories: inventory_directories,
        files: captured,
        skipped: skipped_paths,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(|error| {
        CaptureError::Failed(format!("Snapshot manifest could not be encoded: {error}"))
    })?;
    atomic_write(
        &manifest_path(storage_dir, &project.project_id, &snapshot_id),
        &manifest_bytes,
    )
    .map_err(CaptureError::Failed)?;
    let mut catalog =
        read_catalog(storage_dir, &project.project_id).map_err(CaptureError::Failed)?;
    catalog.schema_version = 1;
    catalog.snapshots.push(summary.clone());
    let catalog_bytes = serde_json::to_vec_pretty(&catalog).map_err(|error| {
        CaptureError::Failed(format!("Snapshot history could not be encoded: {error}"))
    })?;
    atomic_write(
        &catalog_path(storage_dir, &project.project_id),
        &catalog_bytes,
    )
    .map_err(CaptureError::Failed)?;
    progress.status = summary.status.clone();
    progress.summary = Some(summary);
    progress.files_captured = manifest.files.len();
    progress.files_processed = progress.candidate_files;
    progress.paths_skipped = manifest.skipped.len();
    progress.bytes_read = manifest.summary.total_bytes;
    progress.message = Some(if progress.status == CaptureStatus::Partial {
        "Snapshot saved with paths skipped by a capture bound or read error.".to_owned()
    } else {
        "Snapshot saved. Captured files are immutable and stored locally.".to_owned()
    });
    send_progress(&mut progress, callback);
    drop(staging);
    Ok(manifest)
}

fn format_capture_error(error: &CaptureError) -> String {
    match error {
        CaptureError::Cancelled => {
            "Capture cancelled. No snapshot was added to history.".to_owned()
        }
        CaptureError::Inconsistent {
            path: Some(path),
            message,
        } => format!("{message} Path: {path}"),
        CaptureError::Inconsistent {
            path: None,
            message,
        } => message.clone(),
        CaptureError::Failed(message) => message.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::AccessStatus;
    use std::process::Command;
    use std::sync::atomic::AtomicBool;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!("code-atlas-snapshot-{label}-{nonce}"));
            fs::create_dir_all(&path).unwrap();
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

    fn git(root: &Path, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap()
    }

    fn create_repository(root: &Path, files: &[(&str, &[u8])]) {
        fs::create_dir_all(root).unwrap();
        assert!(git(root, &["init", "-q"]).status.success());
        assert!(
            git(
                root,
                &["config", "user.email", "snapshot-test@example.invalid"]
            )
            .status
            .success()
        );
        assert!(
            git(root, &["config", "user.name", "Snapshot Test"])
                .status
                .success()
        );
        for (path, bytes) in files {
            let absolute = root.join(path);
            fs::create_dir_all(absolute.parent().unwrap()).unwrap();
            fs::write(absolute, bytes).unwrap();
        }
        assert!(git(root, &["add", "--all"]).status.success());
        assert!(git(root, &["commit", "-qm", "fixture"]).status.success());
    }

    fn project(root: &Path) -> RecentProject {
        let path = root.canonicalize().unwrap().display().to_string();
        RecentProject {
            project_id: path.clone(),
            display_name: root.file_name().unwrap().to_string_lossy().into_owned(),
            selected_path: path.clone(),
            repository_root: path,
            branch: Some("main".to_owned()),
            head: None,
            access_status: AccessStatus::Available,
            last_opened_unix_ms: 0,
        }
    }

    fn capture_with_limits(
        storage: &Path,
        project: &RecentProject,
        limits: CaptureLimits,
        cancel: &AtomicBool,
    ) -> Result<SnapshotManifest, CaptureError> {
        let progress = new_capture_progress(storage, project, limits).unwrap();
        capture_snapshot_with_progress(storage, project, progress, cancel, &mut |_| {})
    }

    fn default_capture(storage: &Path, project: &RecentProject) -> SnapshotManifest {
        capture_snapshot(storage, project, &AtomicBool::new(false), &mut |_| {}).unwrap()
    }

    #[test]
    fn interrupted_staging_cleanup_is_confined_and_keeps_immutable_snapshot_data() {
        let temporary = TestDirectory::new("recovery-staging-cleanup");
        let storage = temporary.path().join("app-data");
        let snapshot_id = "snap-123456-42-7";
        let staging = storage.join("staging").join(snapshot_id);
        fs::create_dir_all(staging.join("nested")).unwrap();
        fs::write(staging.join("nested/source.blob"), b"partial staged bytes").unwrap();
        let outside = temporary.path().join("outside.txt");
        fs::write(&outside, b"leave this alone").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let external_directory = temporary.path().join("external");
            fs::create_dir_all(&external_directory).unwrap();
            fs::write(external_directory.join("keep.txt"), b"external bytes").unwrap();
            symlink(external_directory, staging.join("external-link")).unwrap();
        }
        let accepted_manifest = storage.join("snapshots/project/manifests/snap-accepted.json");
        fs::create_dir_all(accepted_manifest.parent().unwrap()).unwrap();
        fs::write(&accepted_manifest, b"accepted snapshot manifest").unwrap();

        cleanup_interrupted_staging(&storage, snapshot_id).unwrap();

        assert!(!staging.exists());
        assert_eq!(fs::read(outside).unwrap(), b"leave this alone");
        #[cfg(unix)]
        assert_eq!(
            fs::read(temporary.path().join("external/keep.txt")).unwrap(),
            b"external bytes"
        );
        assert_eq!(
            fs::read(accepted_manifest).unwrap(),
            b"accepted snapshot manifest"
        );
    }

    #[test]
    fn interrupted_staging_cleanup_rejects_traversal_and_symlinked_paths() {
        let temporary = TestDirectory::new("recovery-staging-symlink");
        let storage = temporary.path().join("app-data");
        let outside = temporary.path().join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("keep.txt"), b"keep").unwrap();
        fs::create_dir_all(storage.join("staging/snap-123-4-5")).unwrap();

        assert!(cleanup_interrupted_staging(&storage, "../../outside").is_err());
        assert!(storage.join("staging/snap-123-4-5").exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_root = temporary.path().join("linked-app-data");
            fs::create_dir(&link_root).unwrap();
            symlink(&outside, link_root.join("staging")).unwrap();
            assert!(cleanup_interrupted_staging(&link_root, "snap-123-4-5").is_err());

            let linked_child = storage.join("staging/snap-123-4-6");
            symlink(&outside, &linked_child).unwrap();
            assert!(cleanup_interrupted_staging(&storage, "snap-123-4-6").is_err());
        }
        assert_eq!(fs::read(outside.join("keep.txt")).unwrap(), b"keep");
    }

    #[test]
    fn captured_hashes_are_exact_repeat_capture_has_parent_and_repository_is_unchanged() {
        let temporary = TestDirectory::new("hash-repeat");
        let repository = temporary.path().join("repository");
        let storage = temporary.path().join("app-data");
        create_repository(
            &repository,
            &[("src/main.rs", b"fn main() { println!(\"atlas\"); }\n")],
        );
        let project = project(&repository);
        let original_status = git(&repository, &["status", "--porcelain=v1", "-z"]).stdout;
        let original_source = fs::read(repository.join("src/main.rs")).unwrap();

        let first = default_capture(&storage, &project);
        let file = first
            .files
            .iter()
            .find(|file| file.path == "src/main.rs")
            .unwrap();
        assert_eq!(file.sha256, sha256(&original_source));
        assert_eq!(
            fs::read(storage.join(&file.blob_path)).unwrap(),
            original_source
        );

        let second = default_capture(&storage, &project);
        assert_ne!(first.summary.snapshot_id, second.summary.snapshot_id);
        assert_eq!(
            second.summary.parent_snapshot_id.as_deref(),
            Some(first.summary.snapshot_id.as_str())
        );
        assert_eq!(second.files[0].sha256, first.files[0].sha256);
        assert_eq!(
            git(&repository, &["status", "--porcelain=v1", "-z"]).stdout,
            original_status
        );
        assert_eq!(
            fs::read(repository.join("src/main.rs")).unwrap(),
            original_source
        );
        assert_eq!(
            list_project_snapshots(&storage, &project.project_id)
                .unwrap()
                .snapshots
                .len(),
            2
        );
        assert_eq!(
            load_snapshot_manifest(&storage, &project.project_id, &first.summary.snapshot_id)
                .unwrap(),
            first
        );
    }

    #[test]
    fn inventory_covers_file_kinds_and_source_open_reads_only_the_selected_snapshot() {
        let temporary = TestDirectory::new("inventory-classification");
        let repository = temporary.path().join("repository");
        let storage = temporary.path().join("app-data");
        create_repository(
            &repository,
            &[
                (
                    "src/main.rs",
                    b"fn snapshot_source() { println!(\"frozen\"); }\n",
                ),
                ("src/invalid.rs", &[0xff, 0xfe]),
                ("data/logo.bin", b"\0\x01\x02"),
                ("config/credentials.txt", b"stored locally only"),
                ("docs/readme.zzz", b"an unregistered language extension"),
                ("tests/check.rs", b"#[test] fn evidence() {}"),
            ],
        );
        fs::create_dir_all(repository.join("generated")).unwrap();
        fs::write(repository.join("generated/auto.rs"), b"generated").unwrap();
        fs::write(
            repository.join("oversized.rs"),
            vec![b'x'; MAX_FILE_BYTES as usize + 1],
        )
        .unwrap();
        let project = project(&repository);
        let manifest = default_capture(&storage, &project);

        assert_eq!(manifest.schema_version, 2);
        assert!(
            manifest
                .directories
                .iter()
                .any(|directory| directory.path == ".")
        );
        assert!(
            manifest
                .directories
                .iter()
                .any(|directory| directory.path == "src")
        );
        let inventory_paths = manifest
            .files
            .iter()
            .map(|file| file.path.as_str())
            .chain(manifest.skipped.iter().map(|path| path.path.as_str()))
            .chain(
                manifest
                    .directories
                    .iter()
                    .map(|directory| directory.path.as_str()),
            )
            .collect::<HashSet<_>>();
        assert_eq!(
            inventory_paths.len(),
            manifest.files.len() + manifest.skipped.len() + manifest.directories.len()
        );
        let find = |path: &str| {
            manifest
                .files
                .iter()
                .find(|file| file.path == path)
                .unwrap()
        };
        assert_eq!(
            find("src/invalid.rs").content_kind,
            CapturedContentKind::InvalidEncoding
        );
        assert_eq!(
            find("data/logo.bin").content_kind,
            CapturedContentKind::Binary
        );
        assert!(find("config/credentials.txt").secret_like);
        assert_eq!(
            find("docs/readme.zzz").content_kind,
            CapturedContentKind::Text
        );
        assert!(
            manifest
                .skipped
                .iter()
                .any(|path| path.path == "oversized.rs"
                    && path.reason == "fileByteLimit"
                    && path.size_bytes == Some(MAX_FILE_BYTES + 1))
        );
        assert!(
            manifest
                .skipped
                .iter()
                .any(|path| path.path == "generated" && path.kind == InventoryPathKind::Directory)
        );

        fs::write(repository.join("src/main.rs"), b"changed live working tree").unwrap();
        let loaded = read_snapshot_source(
            &storage,
            &project.project_id,
            &manifest.summary.snapshot_id,
            "src/main.rs",
        )
        .unwrap();
        assert_eq!(
            loaded.contents,
            "fn snapshot_source() { println!(\"frozen\"); }\n"
        );
        assert!(read_snapshot_source(
            &storage,
            &project.project_id,
            "../outside",
            "src/main.rs",
        )
        .unwrap_err()
        .contains("does not belong to the selected repository"));
        assert!(read_snapshot_source(
            &storage,
            &project.project_id,
            &manifest.summary.snapshot_id,
            "src/invalid.rs",
        )
        .unwrap_err()
        .contains("not valid UTF-8"));
        let captured = manifest
            .files
            .iter()
            .find(|file| file.path == "src/main.rs")
            .unwrap();
        let blob = blob_absolute_path(&storage, &captured.sha256);
        let original_blob = fs::read(&blob).unwrap();
        let mut changed_blob = original_blob.clone();
        changed_blob[0] ^= 1;
        fs::write(&blob, changed_blob).unwrap();
        assert!(read_snapshot_source(
            &storage,
            &project.project_id,
            &manifest.summary.snapshot_id,
            "src/main.rs",
        )
        .unwrap_err()
        .contains("size or SHA-256 integrity check"));
        fs::write(&blob, &original_blob[..original_blob.len() - 1]).unwrap();
        assert!(read_snapshot_source(
            &storage,
            &project.project_id,
            &manifest.summary.snapshot_id,
            "src/main.rs",
        )
        .unwrap_err()
        .contains("size or SHA-256 integrity check"));
        assert!(
            read_snapshot_source(
                &storage,
                &project.project_id,
                &manifest.summary.snapshot_id,
                "data/logo.bin",
            )
            .unwrap_err()
            .contains("binary")
        );
        assert!(
            read_snapshot_source(
                &storage,
                &project.project_id,
                &manifest.summary.snapshot_id,
                "../outside.txt",
            )
            .is_err()
        );
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_path_bytes_have_a_stable_percent_escaped_display() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let raw_name = OsString::from_vec(vec![b'n', 0xff, b'.', b't', b'x', b't']);
        let root = PathBuf::from("/tmp/repository");
        assert_eq!(
            escaped_relative_name(&root, &root.join(raw_name)),
            "n%FF.txt"
        );
    }

    #[cfg(unix)]
    #[test]
    fn policy_skips_ignored_generated_symlink_and_nested_repository_but_keeps_tracked_ignored_file()
    {
        use std::os::unix::fs::symlink;
        let temporary = TestDirectory::new("policy-exclusions");
        let repository = temporary.path().join("repository");
        let storage = temporary.path().join("app-data");
        fs::create_dir_all(&repository).unwrap();
        assert!(git(&repository, &["init", "-q"]).status.success());
        assert!(
            git(
                &repository,
                &["config", "user.email", "snapshot-test@example.invalid"]
            )
            .status
            .success()
        );
        assert!(
            git(&repository, &["config", "user.name", "Snapshot Test"])
                .status
                .success()
        );
        fs::write(
            repository.join(".gitignore"),
            "*.log\nignored.txt\nignored-dir/\n",
        )
        .unwrap();
        fs::write(repository.join("tracked.log"), b"tracked despite ignore").unwrap();
        fs::create_dir_all(repository.join("ignored-dir")).unwrap();
        fs::write(
            repository.join("ignored-dir/keep.rs"),
            b"tracked in ignored dir",
        )
        .unwrap();
        fs::write(repository.join("ignored.txt"), b"do not capture").unwrap();
        assert!(git(&repository, &["add", ".gitignore"]).status.success());
        assert!(
            git(&repository, &["add", "-f", "tracked.log"])
                .status
                .success()
        );
        assert!(
            git(&repository, &["add", "-f", "ignored-dir/keep.rs"])
                .status
                .success()
        );
        assert!(
            git(&repository, &["commit", "-qm", "fixture"])
                .status
                .success()
        );
        fs::write(repository.join("ignored.log"), b"ignored file").unwrap();
        fs::write(
            repository.join("ignored-dir/untracked.rs"),
            b"untracked ignored",
        )
        .unwrap();
        fs::create_dir_all(repository.join("vendor")).unwrap();
        fs::write(repository.join("vendor/dependency.rs"), b"vendor").unwrap();
        fs::write(repository.join("real.txt"), b"target").unwrap();
        symlink(repository.join("real.txt"), repository.join("linked.txt")).unwrap();
        let nested = repository.join("nested");
        create_repository(&nested, &[("inside.rs", b"nested")]);

        let result = default_capture(&storage, &project(&repository));
        let paths = result
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<HashSet<_>>();
        assert!(paths.contains("tracked.log"));
        assert!(paths.contains("ignored-dir/keep.rs"));
        assert!(paths.contains("real.txt"));
        assert!(!paths.contains("ignored.txt"));
        assert!(!paths.contains("ignored.log"));
        assert!(!paths.contains("ignored-dir/untracked.rs"));
        assert!(!paths.contains("vendor/dependency.rs"));
        assert!(!paths.contains("linked.txt"));
        assert!(!paths.contains("nested/inside.rs"));
        assert!(
            result
                .skipped
                .iter()
                .any(|item| item.reason == "ignoredByGit")
        );
        assert!(result.skipped.iter().any(|item| item.reason == "symlink"));
        assert!(
            result
                .skipped
                .iter()
                .any(|item| item.reason == "nestedRepository")
        );
        assert!(
            result
                .skipped
                .iter()
                .any(|item| item.reason == "excludedByPolicy")
        );
    }

    #[test]
    fn all_capture_bounds_record_the_path_that_was_not_captured() {
        let temporary = TestDirectory::new("capture-bounds");
        let repository = temporary.path().join("repository");
        create_repository(
            &repository,
            &[
                ("a.txt", b"1234"),
                ("b.txt", b"5678"),
                ("long-name.txt", b"x"),
            ],
        );
        let project = project(&repository);

        let file_storage = temporary.path().join("file-limit");
        let mut limits = CaptureLimits::default();
        limits.max_file_bytes = 3;
        let file_limited =
            capture_with_limits(&file_storage, &project, limits, &AtomicBool::new(false)).unwrap();
        assert!(
            file_limited
                .skipped
                .iter()
                .any(|item| item.reason == "fileByteLimit" && item.path == "a.txt")
        );

        let total_storage = temporary.path().join("total-limit");
        let mut limits = CaptureLimits::default();
        limits.max_total_bytes = 5;
        let total_limited =
            capture_with_limits(&total_storage, &project, limits, &AtomicBool::new(false)).unwrap();
        assert!(
            total_limited
                .skipped
                .iter()
                .any(|item| item.reason == "totalByteLimit")
        );

        let paths_storage = temporary.path().join("path-limit");
        let mut limits = CaptureLimits::default();
        limits.max_paths = 1;
        let path_limited =
            capture_with_limits(&paths_storage, &project, limits, &AtomicBool::new(false)).unwrap();
        assert!(path_limited.summary.paths_visited <= 1);
        assert!(
            path_limited
                .skipped
                .iter()
                .any(|item| item.reason == "pathLimit")
        );

        let entries_storage = temporary.path().join("entry-limit");
        let mut limits = CaptureLimits::default();
        limits.max_entries_per_directory = 1;
        let entries_limited =
            capture_with_limits(&entries_storage, &project, limits, &AtomicBool::new(false))
                .unwrap();
        assert!(
            entries_limited
                .skipped
                .iter()
                .any(|item| item.reason == "directoryEntryLimit")
        );

        let path_bytes_storage = temporary.path().join("path-byte-limit");
        let mut limits = CaptureLimits::default();
        limits.max_path_bytes = 6;
        let path_bytes_limited = capture_with_limits(
            &path_bytes_storage,
            &project,
            limits,
            &AtomicBool::new(false),
        )
        .unwrap();
        assert!(
            path_bytes_limited
                .skipped
                .iter()
                .any(|item| item.reason == "pathByteLimit" && item.path == "long-name.txt")
        );
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_file_is_recorded_as_a_partial_capture_when_permissions_apply() {
        use std::os::unix::fs::PermissionsExt;
        let user = Command::new("id").arg("-u").output().unwrap();
        if String::from_utf8_lossy(&user.stdout).trim() == "0" {
            return;
        }
        let temporary = TestDirectory::new("unreadable");
        let repository = temporary.path().join("repository");
        create_repository(&repository, &[("private.txt", b"private")]);
        let private_file = repository.join("private.txt");
        fs::set_permissions(&private_file, fs::Permissions::from_mode(0o000)).unwrap();
        let result = default_capture(&temporary.path().join("app-data"), &project(&repository));
        fs::set_permissions(&private_file, fs::Permissions::from_mode(0o600)).unwrap();
        assert!(
            result
                .skipped
                .iter()
                .any(|item| item.path == "private.txt" && item.reason == "unreadableFile")
        );
        assert_eq!(result.summary.status, CaptureStatus::Partial);
    }

    #[test]
    fn cancellation_keeps_the_previous_manifest_and_history_entry() {
        let temporary = TestDirectory::new("cancel");
        let repository = temporary.path().join("repository");
        create_repository(&repository, &[("a.rs", b"fn a() {}")]);
        let project = project(&repository);
        let storage = temporary.path().join("app-data");
        let previous = default_capture(&storage, &project);
        let before = list_project_snapshots(&storage, &project.project_id).unwrap();
        let cancel = AtomicBool::new(true);
        let result = capture_with_limits(&storage, &project, CaptureLimits::default(), &cancel);
        assert_eq!(result.unwrap_err(), CaptureError::Cancelled);
        assert_eq!(
            list_project_snapshots(&storage, &project.project_id).unwrap(),
            before
        );
        assert!(
            load_snapshot_manifest(&storage, &project.project_id, &previous.summary.snapshot_id)
                .is_ok()
        );
    }

    #[test]
    fn change_during_chunked_read_is_rejected_without_a_manifest() {
        let temporary = TestDirectory::new("mid-read-change");
        let repository = temporary.path().join("repository");
        let initial = vec![b'a'; READ_CHUNK_BYTES * 3];
        create_repository(&repository, &[("large.txt", &initial)]);
        let project = project(&repository);
        let storage = temporary.path().join("app-data");
        let progress = new_capture_progress(&storage, &project, CaptureLimits::default()).unwrap();
        let mut changed = false;
        let result = capture_snapshot_with_chunk_hook(
            &storage,
            &project,
            progress,
            &AtomicBool::new(false),
            &mut |_| {},
            &mut |path, bytes_read| {
                if !changed && bytes_read >= READ_CHUNK_BYTES as u64 {
                    fs::write(path, vec![b'b'; READ_CHUNK_BYTES * 3]).unwrap();
                    changed = true;
                }
            },
        );
        assert!(matches!(result, Err(CaptureError::Inconsistent { .. })));
        assert!(
            list_project_snapshots(&storage, &project.project_id)
                .unwrap()
                .snapshots
                .is_empty()
        );
    }
}
