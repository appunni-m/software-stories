//! App-managed, finite Pocket TTS processes. No listening socket or cloud fallback.
//!
//! Setup downloads a managed interpreter, pinned packages, and immutable model
//! assets into app data. Generation uses those local assets and a private pipe.

use crate::provider::{ProviderFailure, ProviderFailureKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

mod queue;

#[cfg(unix)]
use std::os::unix::{fs::PermissionsExt, process::CommandExt};

pub(crate) const PACKAGE_VERSION: &str = "3.3.0";
pub(crate) const PYTHON_VERSION: &str = "3.13.14";
const UV_VERSION: &str = "0.13.4";
const UV_ARCHIVE_URL: &str =
    "https://releases.astral.sh/github/uv/releases/download/0.13.4/uv-aarch64-apple-darwin.tar.gz";
const UV_ARCHIVE_SHA256: &str = "e0b1d4743ce64c55d9f0f4a444afa6b81125680070ee04e853fcd0f6c1ebf8f1";
const UV_ARCHIVE_LIMIT: usize = 20 * 1024 * 1024;
const UV_BINARY_LIMIT: usize = 64 * 1024 * 1024;
const REQUIREMENTS_LOCK_FILE: &str = "requirements.lock";
const MAX_REQUIREMENTS_LOCK_BYTES: usize = 128 * 1024;
pub(crate) const MODEL_ID: &str = "pocket-tts-3.3.0:english_2026-09:e7205b6e:alba:4e1e0a3e:pcm-v1";
pub(crate) const VOICE: &str = "alba";
const WORKER: &str = include_str!("../resources/pocket_tts_worker.py");
#[cfg(test)]
const REQUIREMENTS_INPUT: &str =
    include_str!("../resources/pocket-tts-requirements-macos-aarch64-py313.in");
const REQUIREMENTS_LOCK: &str =
    include_str!("../resources/pocket-tts-requirements-macos-aarch64-py313.lock");
const REQUIREMENTS: [&str; 5] = [
    "pocket-tts==3.3.0",
    "torch==2.14.1",
    "numpy==2.5.4",
    "huggingface-hub==2.2.0",
    "pyyaml==6.0.3",
];
const MAX_WAV_BYTES: usize = 12 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 16_000;
const MAX_SEGMENT_BYTES: usize = 4_000;
const MAX_SEGMENTS: usize = 64;
const MAX_PACKET_BYTES: usize = MAX_TEXT_BYTES * 6 + 1024;
const ASSET_NAMES: [&str; 3] = ["model.safetensors", "tokenizer.json", "alba.safetensors"];
static BUSY: AtomicBool = AtomicBool::new(false);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
static ACTIVE_PROCESS: Mutex<Option<u32>> = Mutex::new(None);
static PHASE: Mutex<&'static str> = Mutex::new("idle");
static WORKER_QUEUE: queue::WorkerQueue = queue::WorkerQueue::new();

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PocketTtsStatus {
    pub ready: bool,
    pub busy: bool,
    pub setup_available: bool,
    pub supported_platform: bool,
    pub package_version: String,
    pub python_version: String,
    pub model_id: String,
    pub voice: String,
    pub phase: String,
    pub message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeManifest {
    model_id: String,
    package_version: String,
    python_version: String,
    packages: HashMap<String, String>,
    assets: HashMap<String, AssetManifest>,
    config_sha256: String,
    requirements_sha256: String,
}

#[derive(Deserialize)]
struct AssetManifest {
    size: u64,
    sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UvRuntimeManifest {
    version: String,
    archive_sha256: String,
    binary_size: u64,
    binary_sha256: String,
}

fn root(storage: &Path) -> PathBuf {
    storage.join("pocket-tts").join("runtime-3.3.0-v1")
}

/// Cache identity includes the exact worker, all asset revision pins it embeds,
/// the interpreter and core packages. Changes require fresh audio generation.
pub(crate) fn runtime_fingerprint() -> String {
    let mut hash = Sha256::new();
    hash.update(WORKER.as_bytes());
    hash.update(PYTHON_VERSION.as_bytes());
    hash.update(REQUIREMENTS_LOCK.as_bytes());
    for requirement in REQUIREMENTS {
        hash.update(requirement.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn python(root: &Path) -> PathBuf {
    root.join("venv/bin/python")
}

fn supported_platform() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

fn uv_tool_dir(runtime: &Path) -> PathBuf {
    runtime.join("tooling").join(format!("uv-{UV_VERSION}"))
}

fn verified_uv(runtime: &Path) -> Option<PathBuf> {
    let directory = uv_tool_dir(runtime);
    if !fs::symlink_metadata(&directory).ok()?.file_type().is_dir() {
        return None;
    }
    let manifest_bytes = bounded_read(&directory.join("manifest.json"), 4 * 1024)?;
    let manifest = serde_json::from_slice::<UvRuntimeManifest>(&manifest_bytes).ok()?;
    if manifest.version != UV_VERSION || manifest.archive_sha256 != UV_ARCHIVE_SHA256 {
        return None;
    }
    let binary = directory.join("uv");
    let metadata = fs::symlink_metadata(&binary).ok()?;
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > UV_BINARY_LIMIT as u64
    {
        return None;
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o111 == 0 {
        return None;
    }
    let bytes = bounded_read(&binary, UV_BINARY_LIMIT)?;
    if bytes.len() as u64 != manifest.binary_size || sha256_hex(&bytes) != manifest.binary_sha256 {
        return None;
    }
    Some(binary)
}

fn installed(root: &Path) -> bool {
    let Some(bytes) = bounded_read(&root.join("ready.json"), 64 * 1024) else {
        return false;
    };
    let Ok(manifest) = serde_json::from_slice::<RuntimeManifest>(&bytes) else {
        return false;
    };
    let Some(lock_bytes) = bounded_read(
        &root.join(REQUIREMENTS_LOCK_FILE),
        MAX_REQUIREMENTS_LOCK_BYTES,
    ) else {
        return false;
    };
    manifest.model_id == MODEL_ID
        && manifest.package_version == PACKAGE_VERSION
        && manifest.python_version == PYTHON_VERSION
        && manifest.requirements_sha256 == requirements_sha256()
        && sha256_hex(&lock_bytes) == requirements_sha256()
        && REQUIREMENTS.iter().all(|requirement| {
            requirement.split_once("==").is_some_and(|(name, version)| {
                manifest
                    .packages
                    .get(name)
                    .is_some_and(|installed| installed == version)
            })
        })
        && manifest.config_sha256.len() == 64
        && python(root).is_file()
        && root.join("model.yaml").is_file()
        && ASSET_NAMES.iter().all(|name| {
            manifest.assets.get(*name).is_some_and(|asset| {
                asset.size > 0
                    && asset.sha256.len() == 64
                    && fs::metadata(root.join("assets").join(*name))
                        .is_ok_and(|metadata| metadata.is_file() && metadata.len() == asset.size)
            })
        })
}

/// Read installation state only. This never starts Python or downloads assets.
pub(crate) fn status(storage: &Path) -> PocketTtsStatus {
    let supported_platform = supported_platform();
    let ready = supported_platform && installed(&root(storage));
    let busy = BUSY.load(Ordering::Acquire);
    let phase = PHASE.lock().map(|phase| *phase).unwrap_or("unavailable");
    let setup_available = supported_platform;
    let message = if !supported_platform {
        "Pocket TTS setup currently targets Apple Silicon Macs. This runtime is unavailable on this platform."
    } else if busy {
        match phase {
            "tool" => "Preparing the verified Pocket TTS setup tool…",
            "python" => "Preparing the app's Python runtime…",
            "packages" => "Downloading and installing Pocket TTS dependencies…",
            "model" => {
                "Downloading the pinned English model and Alba voice, then loading them locally…"
            }
            "synthesizing" => "Generating narration locally with Pocket TTS…",
            _ => "Preparing Pocket TTS…",
        }
    } else if ready {
        "Pocket TTS is installed. English narration uses the local Alba voice with no API key."
    } else if setup_available {
        "Set up Pocket TTS once to download its verified setup tool, Python, dependencies, English model and Alba voice into app storage. Generation then uses local files."
    } else {
        "Pocket TTS setup currently targets Apple Silicon Macs. This runtime is unavailable on this platform."
    };
    PocketTtsStatus {
        ready,
        busy,
        setup_available,
        supported_platform,
        package_version: PACKAGE_VERSION.into(),
        python_version: PYTHON_VERSION.into(),
        model_id: MODEL_ID.into(),
        voice: VOICE.into(),
        phase: phase.into(),
        message: message.into(),
    }
}

/// Stop the owned process group before the desktop app exits. Call from Tauri's
/// exit event; the worker thread normally reaps the direct child on its next poll.
pub(crate) fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::Release);
    WORKER_QUEUE.close();
    if let Ok(active) = ACTIVE_PROCESS.lock() {
        if let Some(pid) = *active {
            kill_process_group(pid);
        }
    }
}

/// Explicit setup is the only operation permitted to download runtime assets.
pub(crate) async fn setup(
    storage: &Path,
    cancellation: &mut oneshot::Receiver<()>,
) -> Result<PocketTtsStatus, ProviderFailure> {
    let runtime = root(storage);
    run_exclusive(cancellation, move |cancelled| {
        setup_blocking(&runtime, &cancelled)
    })
    .await?;
    Ok(status(storage))
}

/// Generate one section with a finite, offline worker and return validated PCM WAV.
pub(crate) async fn synthesize_segments(
    storage: &Path,
    segments: &[String],
    cancellation: &mut oneshot::Receiver<()>,
) -> Result<Vec<u8>, ProviderFailure> {
    let packet = section_packet(segments)?;
    let runtime = root(storage);
    run_exclusive(cancellation, move |cancelled| {
        if !installed(&runtime) {
            return Err(failure(ProviderFailureKind::UnavailableModel, "Set up Pocket TTS before generating narration. Missing or changed runtime files require setup again."));
        }
        set_phase("synthesizing");
        write_worker(&runtime)?;
        let output = runtime.join("active.wav");
        let _cleanup = OutputCleanup(output.clone());
        let _ = fs::remove_file(&output);
        let mut command = worker_command(&runtime, "synthesize", true);
        run_child(&mut command, Some(&packet), &cancelled, Duration::from_secs(300),
            "Pocket TTS could not generate audio. Run setup again if its files changed; the transcript and prior audio remain available.")?;
        let bytes = bounded_read(&output, MAX_WAV_BYTES).ok_or_else(|| failure(
            ProviderFailureKind::ProviderResponseTooLarge, "Pocket TTS returned missing or oversized audio. The transcript and prior audio remain available."))?;
        validate_wav(&bytes)?;
        Ok(bytes)
    }).await
}

fn section_packet(segments: &[String]) -> Result<Vec<u8>, ProviderFailure> {
    if segments.is_empty()
        || segments.len() > MAX_SEGMENTS
        || segments
            .iter()
            .try_fold(0_usize, |sum, text| sum.checked_add(text.len()))
            .is_none_or(|total| total > MAX_TEXT_BYTES)
        || segments
            .iter()
            .any(|text| text.trim().is_empty() || text.len() > MAX_SEGMENT_BYTES)
    {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "Pocket TTS needs the reviewed segments, each at most 4,000 UTF-8 bytes and at most 16,000 bytes in total.",
        ));
    }
    let packet = serde_json::to_vec(&serde_json::json!({"schemaVersion":1,"segments":segments}))
        .map_err(|_| {
            failure(
                ProviderFailureKind::InvalidReview,
                "The local narration request could not be encoded.",
            )
        })?;
    if packet.len() > MAX_PACKET_BYTES {
        return Err(failure(
            ProviderFailureKind::InvalidReview,
            "The local narration request exceeds its limit.",
        ));
    }
    Ok(packet)
}

async fn run_exclusive<T: Send + 'static>(
    cancellation: &mut oneshot::Receiver<()>,
    operation: impl FnOnce(Arc<AtomicBool>) -> Result<T, ProviderFailure> + Send + 'static,
) -> Result<T, ProviderFailure> {
    if SHUTTING_DOWN.load(Ordering::Acquire) {
        return Err(cancelled_failure());
    }
    if !supported_platform() {
        return Err(failure(
            ProviderFailureKind::UnavailableModel,
            "Pocket TTS currently requires an Apple Silicon Mac.",
        ));
    }
    let permit = WORKER_QUEUE.acquire(cancellation).await?;
    if SHUTTING_DOWN.load(Ordering::Acquire) {
        return Err(cancelled_failure());
    }
    BUSY.store(true, Ordering::Release);
    let cancelled = Arc::new(AtomicBool::new(false));
    let _cancel_on_drop = CancelOnDrop(Arc::clone(&cancelled));
    let task_cancelled = Arc::clone(&cancelled);
    let lease = OperationLease { _permit: permit };
    let mut task = tauri::async_runtime::spawn_blocking(move || {
        let _lease = lease;
        operation(task_cancelled)
    });
    tokio::select! {
        biased;
        _ = cancellation => {
            cancelled.store(true, Ordering::Release);
            let _ = task.await;
            Err(cancelled_failure())
        }
        result = &mut task => result.map_err(|_| failure(
            ProviderFailureKind::LocalStorageUnavailable, "The Pocket TTS worker stopped unexpectedly. Retry setup if the problem continues."))?,
    }
}

struct OperationLease {
    _permit: queue::WorkerPermit<'static>,
}
impl Drop for OperationLease {
    fn drop(&mut self) {
        set_phase("idle");
        BUSY.store(false, Ordering::Release);
    }
}
struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}
struct OutputCleanup(PathBuf);
impl Drop for OutputCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
struct DirectoryCleanup(PathBuf);
impl Drop for DirectoryCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn setup_blocking(runtime: &Path, cancelled: &AtomicBool) -> Result<(), ProviderFailure> {
    fs::create_dir_all(runtime).map_err(|_| storage_failure())?;
    #[cfg(unix)]
    fs::set_permissions(runtime, fs::Permissions::from_mode(0o700))
        .map_err(|_| storage_failure())?;
    for directory in ["tmp", "uv-cache", "python", "huggingface", "cache"] {
        fs::create_dir_all(runtime.join(directory)).map_err(|_| storage_failure())?;
    }
    let uv = ensure_uv(runtime, cancelled)?;
    let _ = fs::remove_file(runtime.join("ready.json"));
    write_worker(runtime)?;
    set_phase("python");
    let mut venv = venv_command(&uv, runtime);
    run_child(
        &mut venv,
        None,
        cancelled,
        Duration::from_secs(900),
        "Pocket TTS could not prepare Python. Check your connection and available disk space, then retry setup.",
    )?;
    set_phase("packages");
    write_requirements_lock(runtime)?;
    let mut install = package_install_command(&uv, runtime);
    run_child(
        &mut install,
        None,
        cancelled,
        Duration::from_secs(1800),
        "Pocket TTS dependencies could not be installed. Check your connection and available disk space, then retry setup.",
    )?;
    set_phase("model");
    let mut download = worker_command(runtime, "setup", false);
    run_child(
        &mut download,
        None,
        cancelled,
        Duration::from_secs(1200),
        "Pocket TTS could not download or load its model and voice. Check your connection and available memory/disk space, then retry setup.",
    )?;
    if !installed(runtime) {
        return Err(failure(
            ProviderFailureKind::UnavailableModel,
            "Pocket TTS setup did not produce a complete runtime. Retry setup.",
        ));
    }
    cleanup_setup_caches(runtime);
    Ok(())
}

/// Drop setup-only copies after the verified runtime owns its model assets.
/// Failed setup keeps these caches so an explicit retry can reuse downloads.
fn cleanup_setup_caches(runtime: &Path) {
    for directory in ["uv-cache", "huggingface"] {
        let _ = remove_path_if_present(&runtime.join(directory));
    }
}

fn write_worker(runtime: &Path) -> Result<(), ProviderFailure> {
    fs::write(runtime.join("worker.py"), WORKER).map_err(|_| storage_failure())
}

fn write_requirements_lock(runtime: &Path) -> Result<(), ProviderFailure> {
    fs::write(runtime.join(REQUIREMENTS_LOCK_FILE), REQUIREMENTS_LOCK)
        .map_err(|_| storage_failure())
}

fn requirements_sha256() -> String {
    sha256_hex(REQUIREMENTS_LOCK.as_bytes())
}

fn ensure_uv(runtime: &Path, cancelled: &AtomicBool) -> Result<PathBuf, ProviderFailure> {
    if let Some(binary) = verified_uv(runtime) {
        return Ok(binary);
    }

    set_phase("tool");
    let tooling = runtime.join("tooling");
    fs::create_dir_all(&tooling).map_err(|_| storage_failure())?;
    #[cfg(unix)]
    fs::set_permissions(&tooling, fs::Permissions::from_mode(0o700))
        .map_err(|_| storage_failure())?;

    let archive = runtime
        .join("tmp")
        .join(format!("uv-{UV_VERSION}-{}.tar.gz", uuid::Uuid::new_v4()));
    let _archive_cleanup = OutputCleanup(archive.clone());
    let mut download = Command::new("/usr/bin/curl");
    configure_environment(&mut download, runtime, false);
    download
        .args([
            "--disable",
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--tlsv1.2",
            "--max-redirs",
            "5",
            "--max-filesize",
            "20000000",
            "--connect-timeout",
            "30",
            "--max-time",
            "300",
            "--output",
        ])
        .arg(&archive)
        .arg(UV_ARCHIVE_URL);
    run_child(
        &mut download,
        None,
        cancelled,
        Duration::from_secs(300),
        "Pocket TTS could not download its setup tool. Check your connection and retry.",
    )?;
    let archive_bytes = bounded_read(&archive, UV_ARCHIVE_LIMIT).ok_or_else(|| {
        failure(
            ProviderFailureKind::UnavailableModel,
            "The Pocket TTS setup tool download was incomplete or too large. Retry setup.",
        )
    })?;
    if sha256_hex(&archive_bytes) != UV_ARCHIVE_SHA256 {
        return Err(failure(
            ProviderFailureKind::UnavailableModel,
            "The Pocket TTS setup tool did not match its pinned SHA-256. Retry setup later.",
        ));
    }

    let staging = tooling.join(format!(".uv-stage-{}", uuid::Uuid::new_v4()));
    fs::create_dir(&staging).map_err(|_| storage_failure())?;
    #[cfg(unix)]
    fs::set_permissions(&staging, fs::Permissions::from_mode(0o700))
        .map_err(|_| storage_failure())?;
    let _staging_cleanup = DirectoryCleanup(staging.clone());
    let mut extract = Command::new("/usr/bin/tar");
    configure_environment(&mut extract, runtime, false);
    extract
        .args(["-xzf"])
        .arg(&archive)
        .arg("-C")
        .arg(&staging)
        .args(["--strip-components=1", "uv-aarch64-apple-darwin/uv"]);
    run_child(
        &mut extract,
        None,
        cancelled,
        Duration::from_secs(30),
        "Pocket TTS could not unpack its verified setup tool. Retry setup.",
    )?;

    let binary = staging.join("uv");
    let metadata = fs::symlink_metadata(&binary).map_err(|_| {
        failure(
            ProviderFailureKind::UnavailableModel,
            "The Pocket TTS setup tool archive did not contain its expected executable.",
        )
    })?;
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > UV_BINARY_LIMIT as u64
    {
        return Err(failure(
            ProviderFailureKind::UnavailableModel,
            "The Pocket TTS setup tool archive contained an invalid executable.",
        ));
    }
    #[cfg(unix)]
    fs::set_permissions(&binary, fs::Permissions::from_mode(0o700))
        .map_err(|_| storage_failure())?;
    let binary_bytes = bounded_read(&binary, UV_BINARY_LIMIT).ok_or_else(storage_failure)?;
    let manifest = UvRuntimeManifest {
        version: UV_VERSION.to_owned(),
        archive_sha256: UV_ARCHIVE_SHA256.to_owned(),
        binary_size: binary_bytes.len() as u64,
        binary_sha256: sha256_hex(&binary_bytes),
    };
    let manifest_bytes = serde_json::to_vec(&manifest).map_err(|_| storage_failure())?;
    fs::write(staging.join("manifest.json"), manifest_bytes).map_err(|_| storage_failure())?;

    let destination = uv_tool_dir(runtime);
    remove_path_if_present(&destination)?;
    fs::rename(&staging, &destination).map_err(|_| storage_failure())?;
    verified_uv(runtime).ok_or_else(|| {
        failure(
            ProviderFailureKind::UnavailableModel,
            "The installed Pocket TTS setup tool could not be verified. Retry setup.",
        )
    })
}

fn remove_path_if_present(path: &Path) -> Result<(), ProviderFailure> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(storage_failure()),
    };
    if metadata.file_type().is_symlink() || metadata.is_file() {
        fs::remove_file(path).map_err(|_| storage_failure())
    } else if metadata.is_dir() {
        fs::remove_dir_all(path).map_err(|_| storage_failure())
    } else {
        Err(storage_failure())
    }
}

fn worker_command(runtime: &Path, mode: &str, offline: bool) -> Command {
    let mut command = Command::new(python(runtime));
    configure_environment(&mut command, runtime, offline);
    command
        .env("CODE_ATLAS_REQUIREMENTS_SHA256", requirements_sha256())
        .arg("-I")
        .arg(runtime.join("worker.py"))
        .arg(mode)
        .arg(runtime);
    command
}

fn package_install_command(uv: &Path, runtime: &Path) -> Command {
    let mut command = Command::new(uv);
    configure_environment(&mut command, runtime, false);
    command
        .args([
            "pip",
            "install",
            "--no-config",
            "--require-hashes",
            "--only-binary",
            ":all:",
            "--python",
        ])
        .arg(python(runtime))
        .arg("-r")
        .arg(runtime.join(REQUIREMENTS_LOCK_FILE));
    command
}

fn venv_command(uv: &Path, runtime: &Path) -> Command {
    let mut command = Command::new(uv);
    configure_environment(&mut command, runtime, false);
    // UV_PYTHON_PREFERENCE=only-managed already enforces a managed interpreter.
    // uv rejects --managed-python together with that environment setting.
    command
        .args(["venv", "--clear", "--no-config", "--python", PYTHON_VERSION])
        .arg(runtime.join("venv"));
    command
}

fn configure_environment(command: &mut Command, runtime: &Path, offline: bool) {
    command
        .env_clear()
        .current_dir(runtime)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("LANG", "en_US.UTF-8")
        .env("TMPDIR", runtime.join("tmp"))
        .env("UV_CACHE_DIR", runtime.join("uv-cache"))
        .env("UV_PYTHON_INSTALL_DIR", runtime.join("python"))
        .env("UV_PYTHON_PREFERENCE", "only-managed")
        .env("UV_NO_CONFIG", "1")
        .env("UV_NO_PROGRESS", "1")
        .env("HF_HOME", runtime.join("huggingface"))
        .env("HF_HUB_DISABLE_IMPLICIT_TOKEN", "1")
        .env("HF_HUB_DISABLE_TELEMETRY", "1")
        .env("DO_NOT_TRACK", "1")
        .env("XDG_CACHE_HOME", runtime.join("cache"));
    if let Some(home) = std::env::var_os("HOME") {
        command.env("HOME", home);
    }
    if offline {
        command.env("HF_HUB_OFFLINE", "1").env("UV_OFFLINE", "1");
    }
    command.stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    command.process_group(0);
}

fn run_child(
    command: &mut Command,
    input: Option<&[u8]>,
    cancelled: &AtomicBool,
    timeout: Duration,
    message: &'static str,
) -> Result<(), ProviderFailure> {
    let started = Instant::now();
    check_process_boundary(cancelled, started, timeout)?;
    command.stdin(if input.is_some() {
        Stdio::piped()
    } else {
        Stdio::null()
    });
    // Serialize registration with shutdown, so exit cannot miss a just-spawned
    // process or signal a PID after this owner has reaped it.
    let mut active = ACTIVE_PROCESS
        .lock()
        .map_err(|_| failure(ProviderFailureKind::UnavailableModel, message))?;
    if SHUTTING_DOWN.load(Ordering::Acquire) {
        return Err(cancelled_failure());
    }
    let child = command
        .spawn()
        .map_err(|_| failure(ProviderFailureKind::UnavailableModel, message))?;
    *active = Some(child.id());
    drop(active);
    let mut child = ManagedChild {
        child,
        reaped: false,
    };
    if let Some(input) = input {
        write_child_input(&mut child, input, cancelled, started, timeout, message)?;
    }
    loop {
        check_process_boundary(cancelled, started, timeout)?;
        let mut active = ACTIVE_PROCESS
            .lock()
            .map_err(|_| failure(ProviderFailureKind::UnavailableModel, message))?;
        match child.child.try_wait() {
            Ok(Some(status)) => {
                *active = None;
                child.reaped = true;
                return if status.success() {
                    Ok(())
                } else {
                    Err(failure(ProviderFailureKind::UnavailableModel, message))
                };
            }
            Ok(None) => {
                drop(active);
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return Err(failure(ProviderFailureKind::UnavailableModel, message)),
        }
    }
}

fn check_process_boundary(
    cancelled: &AtomicBool,
    started: Instant,
    timeout: Duration,
) -> Result<(), ProviderFailure> {
    if cancelled.load(Ordering::Acquire) || SHUTTING_DOWN.load(Ordering::Acquire) {
        return Err(cancelled_failure());
    }
    if started.elapsed() >= timeout {
        return Err(failure(
            ProviderFailureKind::RequestTimedOut,
            "Pocket TTS reached its local time limit and was stopped. The transcript and prior audio remain available.",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn write_child_input(
    child: &mut ManagedChild,
    input: &[u8],
    cancelled: &AtomicBool,
    started: Instant,
    timeout: Duration,
    message: &'static str,
) -> Result<(), ProviderFailure> {
    use std::os::fd::AsRawFd;
    let mut stdin = child
        .child
        .stdin
        .take()
        .ok_or_else(|| failure(ProviderFailureKind::UnavailableModel, message))?;
    let fd = stdin.as_raw_fd();
    // SAFETY: `stdin` exclusively owns this live descriptor for both calls and
    // all writes below. fcntl takes integer arguments here, no pointers. Keep
    // existing flags; no other thread closes or reuses this descriptor.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(failure(ProviderFailureKind::UnavailableModel, message));
    }
    let mut cursor = 0;
    while cursor < input.len() {
        check_process_boundary(cancelled, started, timeout)?;
        match stdin.write(&input[cursor..]) {
            Ok(0) => return Err(failure(ProviderFailureKind::UnavailableModel, message)),
            Ok(bytes) => cursor += bytes,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(_) => return Err(failure(ProviderFailureKind::UnavailableModel, message)),
        }
    }
    Ok(()) // Closing stdin delivers EOF before waiting for the finite worker.
}

#[cfg(not(unix))]
fn write_child_input(
    _: &mut ManagedChild,
    _: &[u8],
    _: &AtomicBool,
    _: Instant,
    _: Duration,
    message: &'static str,
) -> Result<(), ProviderFailure> {
    Err(failure(ProviderFailureKind::UnavailableModel, message))
}

struct ManagedChild {
    child: Child,
    reaped: bool,
}
impl Drop for ManagedChild {
    fn drop(&mut self) {
        if self.reaped {
            return;
        }
        let mut active = ACTIVE_PROCESS.lock().ok();
        kill_process_group(self.child.id());
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(active) = active.as_mut() {
            **active = None;
        }
    }
}

fn kill_process_group(process_id: u32) {
    #[cfg(unix)]
    if let Ok(pid) = i32::try_from(process_id) {
        // SAFETY: process_group(0) created an owned group. Its lifecycle lock
        // prevents reaping/reuse until the signal is sent. No arbitrary PID is accepted.
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
    }
    #[cfg(not(unix))]
    let _ = process_id;
}

fn bounded_read(path: &Path, limit: usize) -> Option<Vec<u8>> {
    crate::bounded_file::read(path, limit).ok()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{digest:x}")
}

fn validate_wav(bytes: &[u8]) -> Result<(), ProviderFailure> {
    let valid = bytes.len() >= 46
        && bytes.len() <= MAX_WAV_BYTES
        && &bytes[..4] == b"RIFF"
        && &bytes[8..16] == b"WAVEfmt "
        && bytes[16..20] == 16_u32.to_le_bytes()
        && bytes[20..22] == 1_u16.to_le_bytes()
        && bytes[22..24] == 1_u16.to_le_bytes()
        && bytes[24..28] == 24_000_u32.to_le_bytes()
        && bytes[28..32] == 48_000_u32.to_le_bytes()
        && bytes[32..34] == 2_u16.to_le_bytes()
        && bytes[34..36] == 16_u16.to_le_bytes()
        && &bytes[36..40] == b"data"
        && bytes[4..8] == ((bytes.len() - 8) as u32).to_le_bytes()
        && bytes[40..44] == ((bytes.len() - 44) as u32).to_le_bytes()
        && (bytes.len() - 44) % 2 == 0;
    if valid {
        Ok(())
    } else {
        Err(failure(
            ProviderFailureKind::MalformedProviderResponse,
            "Pocket TTS returned invalid audio. The transcript and prior audio remain available.",
        ))
    }
}

fn set_phase(phase: &'static str) {
    if let Ok(mut current) = PHASE.lock() {
        *current = phase;
    }
}
fn failure(kind: ProviderFailureKind, message: impl Into<String>) -> ProviderFailure {
    ProviderFailure {
        kind,
        message: message.into(),
    }
}
fn storage_failure() -> ProviderFailure {
    failure(
        ProviderFailureKind::LocalStorageUnavailable,
        "Pocket TTS could not write to app storage. Check available disk space and folder permissions.",
    )
}
fn cancelled_failure() -> ProviderFailure {
    failure(
        ProviderFailureKind::RequestCancelled,
        "Pocket TTS was stopped locally. The transcript and prior audio remain available.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestAudioFile(PathBuf);

    impl TestAudioFile {
        fn new() -> Self {
            Self(std::env::temp_dir().join(format!(
                "code-atlas-pocket-tts-bounded-read-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            )))
        }
    }

    impl Drop for TestAudioFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    struct TestRuntime(PathBuf);

    impl TestRuntime {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "code-atlas-pocket-tts-runtime-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestRuntime {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn managed_uv_is_reused_only_when_release_and_binary_hashes_match() {
        let runtime = TestRuntime::new();
        let directory = uv_tool_dir(&runtime.0);
        fs::create_dir_all(&directory).unwrap();
        let binary = directory.join("uv");
        let bytes = b"synthetic pinned uv executable";
        fs::write(&binary, bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        let manifest = UvRuntimeManifest {
            version: UV_VERSION.to_owned(),
            archive_sha256: UV_ARCHIVE_SHA256.to_owned(),
            binary_size: bytes.len() as u64,
            binary_sha256: sha256_hex(bytes),
        };
        fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert_eq!(verified_uv(&runtime.0).as_deref(), Some(binary.as_path()));

        fs::write(&binary, b"changed executable").unwrap();
        assert!(verified_uv(&runtime.0).is_none());
    }

    #[test]
    fn managed_uv_rejects_manifest_for_a_different_release() {
        let runtime = TestRuntime::new();
        let directory = uv_tool_dir(&runtime.0);
        fs::create_dir_all(&directory).unwrap();
        let binary = directory.join("uv");
        let bytes = b"synthetic pinned uv executable";
        fs::write(&binary, bytes).unwrap();
        #[cfg(unix)]
        fs::set_permissions(&binary, fs::Permissions::from_mode(0o700)).unwrap();
        let manifest = UvRuntimeManifest {
            version: UV_VERSION.to_owned(),
            archive_sha256: "not-the-pinned-release".to_owned(),
            binary_size: bytes.len() as u64,
            binary_sha256: sha256_hex(bytes),
        };
        fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verified_uv(&runtime.0).is_none());
    }

    #[test]
    fn pocket_setup_availability_does_not_depend_on_a_global_uv_install() {
        let runtime = TestRuntime::new();
        let status = status(&runtime.0);
        assert_eq!(status.setup_available, supported_platform());
    }

    #[cfg(unix)]
    #[test]
    fn managed_uv_rejects_symlinked_executable() {
        use std::os::unix::fs::symlink;

        let runtime = TestRuntime::new();
        let directory = uv_tool_dir(&runtime.0);
        fs::create_dir_all(&directory).unwrap();
        let target = directory.join("outside-uv");
        let bytes = b"synthetic pinned uv executable";
        fs::write(&target, bytes).unwrap();
        fs::set_permissions(&target, fs::Permissions::from_mode(0o700)).unwrap();
        let binary = directory.join("uv");
        symlink(&target, &binary).unwrap();
        let manifest = UvRuntimeManifest {
            version: UV_VERSION.to_owned(),
            archive_sha256: UV_ARCHIVE_SHA256.to_owned(),
            binary_size: bytes.len() as u64,
            binary_sha256: sha256_hex(bytes),
        };
        fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        assert!(verified_uv(&runtime.0).is_none());
    }

    #[test]
    fn generated_audio_reader_accepts_12_mib_and_rejects_the_next_byte() {
        let file = TestAudioFile::new();
        let exact = valid_wav(MAX_WAV_BYTES);
        fs::write(&file.0, &exact).unwrap();
        let loaded = bounded_read(&file.0, MAX_WAV_BYTES).unwrap();
        assert_eq!(loaded.len(), MAX_WAV_BYTES);
        assert!(validate_wav(&loaded).is_ok());

        let mut oversized = vec![0; MAX_WAV_BYTES];
        oversized.push(0);
        fs::write(&file.0, oversized).unwrap();
        assert!(bounded_read(&file.0, MAX_WAV_BYTES).is_none());

        assert!(validate_wav(&[]).is_err());
        let truncated = exact.get(..exact.len() - 1).unwrap();
        assert!(validate_wav(truncated).is_err());
        let mut corrupted = exact;
        corrupted[0] = b'X';
        assert!(validate_wav(&corrupted).is_err());
    }

    fn valid_wav(byte_len: usize) -> Vec<u8> {
        assert!(byte_len >= 46 && byte_len <= MAX_WAV_BYTES && (byte_len - 44) % 2 == 0);
        let mut bytes = vec![0; byte_len];
        bytes[..4].copy_from_slice(b"RIFF");
        bytes[4..8].copy_from_slice(&((byte_len - 8) as u32).to_le_bytes());
        bytes[8..16].copy_from_slice(b"WAVEfmt ");
        bytes[16..20].copy_from_slice(&16_u32.to_le_bytes());
        bytes[20..22].copy_from_slice(&1_u16.to_le_bytes());
        bytes[22..24].copy_from_slice(&1_u16.to_le_bytes());
        bytes[24..28].copy_from_slice(&24_000_u32.to_le_bytes());
        bytes[28..32].copy_from_slice(&48_000_u32.to_le_bytes());
        bytes[32..34].copy_from_slice(&2_u16.to_le_bytes());
        bytes[34..36].copy_from_slice(&16_u16.to_le_bytes());
        bytes[36..40].copy_from_slice(b"data");
        bytes[40..44].copy_from_slice(&((byte_len - 44) as u32).to_le_bytes());
        bytes
    }

    #[test]
    fn reviewed_packet_keeps_order_text_and_utf8_bounds() {
        let segments = vec!["é".repeat(2_000); 4];
        let bytes = section_packet(&segments).unwrap();
        let decoded: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            decoded,
            serde_json::json!({"schemaVersion":1,"segments":segments})
        );
        // JSON escaping can expand valid source text without widening its limit.
        let escaped = vec!["\u{0001}".repeat(4_000); 4];
        let bytes = section_packet(&escaped).unwrap();
        assert!(bytes.len() > MAX_TEXT_BYTES);
        assert!(bytes.len() <= MAX_PACKET_BYTES);
        for invalid in [
            vec![],
            vec![" ".into()],
            vec!["é".repeat(2_001)],
            vec!["a".repeat(4_000); 5],
            vec!["a".into(); MAX_SEGMENTS + 1],
        ] {
            assert_eq!(
                section_packet(&invalid).unwrap_err().kind,
                ProviderFailureKind::InvalidReview
            );
        }
    }

    #[cfg(unix)]
    static PROCESS_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[cfg(unix)]
    fn blocked_input_failure(cancel_after: Option<Duration>) -> ProviderFailure {
        let _serial = PROCESS_TEST_LOCK.lock().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let mut command = Command::new("/bin/sleep");
        command
            .arg("30")
            .process_group(0)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let process = command.spawn().unwrap();
        let pid = process.id();
        *ACTIVE_PROCESS.lock().unwrap() = Some(pid);
        let mut child = ManagedChild {
            child: process,
            reaped: false,
        };
        let trigger = cancel_after.map(|delay| {
            let cancelled = Arc::clone(&cancelled);
            std::thread::spawn(move || {
                std::thread::sleep(delay);
                cancelled.store(true, Ordering::Release);
            })
        });
        let started = Instant::now();
        let result = write_child_input(
            &mut child,
            &vec![b'x'; 1_048_576],
            &cancelled,
            started,
            Duration::from_millis(300),
            "fixed synthetic failure",
        );
        drop(child);
        if let Some(trigger) = trigger {
            trigger.join().unwrap();
        }
        assert!(started.elapsed() < Duration::from_secs(3));
        assert_eq!(*ACTIVE_PROCESS.lock().unwrap(), None);
        let mut status = 0;
        // SAFETY: the captured PID belongs to the direct child above. waitpid
        // writes only to this valid local integer. Drop must already reap it.
        assert_eq!(
            unsafe { libc::waitpid(pid as i32, &mut status, libc::WNOHANG) },
            -1
        );
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
        result.unwrap_err()
    }

    #[test]
    #[cfg(unix)]
    fn stalled_input_deadline_stops_and_reaps_owned_child() {
        assert_eq!(
            blocked_input_failure(None).kind,
            ProviderFailureKind::RequestTimedOut
        );
    }

    #[test]
    #[cfg(unix)]
    fn stalled_input_cancellation_stops_and_reaps_owned_child() {
        assert_eq!(
            blocked_input_failure(Some(Duration::from_millis(60))).kind,
            ProviderFailureKind::RequestCancelled
        );
    }

    #[test]
    fn streaming_worker_uses_thread_compatible_no_grad_context() {
        assert!(WORKER.contains("with torch.no_grad(), wave.open("));
        assert!(!WORKER.contains("torch.inference_mode("));
    }

    #[test]
    fn managed_python_setup_uses_one_non_conflicting_preference() {
        let command = venv_command(Path::new("/test/uv"), Path::new("/test/runtime"));
        let args: Vec<_> = command.get_args().collect();
        assert!(!args.contains(&std::ffi::OsStr::new("--managed-python")));
        assert_eq!(
            args.last(),
            Some(&std::ffi::OsStr::new("/test/runtime/venv"))
        );
        assert!(command.get_envs().any(|(key, value)| {
            key == "UV_PYTHON_PREFERENCE" && value == Some(std::ffi::OsStr::new("only-managed"))
        }));
        assert!(args.windows(2).any(|pair| {
            pair == [
                std::ffi::OsStr::new("--python"),
                std::ffi::OsStr::new(PYTHON_VERSION),
            ]
        }));
    }

    #[test]
    fn package_lock_pins_and_hashes_every_dependency() {
        let direct_inputs: Vec<_> = REQUIREMENTS_INPUT
            .lines()
            .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with('#'))
            .collect();
        assert_eq!(direct_inputs, REQUIREMENTS.to_vec());

        let mut package_count = 0;
        let mut current_package_has_hash = false;
        for line in REQUIREMENTS_LOCK.lines() {
            let trimmed = line.trim();
            if !line.chars().next().is_some_and(char::is_whitespace)
                && !trimmed.starts_with('#')
                && trimmed.contains("==")
            {
                if package_count > 0 {
                    assert!(current_package_has_hash, "unhashed lock entry: {line}");
                }
                package_count += 1;
                current_package_has_hash = false;
                assert!(
                    trimmed.ends_with('\\'),
                    "expected generated lock entry: {line}"
                );
            } else if trimmed.starts_with("--hash=sha256:") {
                assert!(package_count > 0, "hash precedes a package entry");
                current_package_has_hash = true;
            }
        }
        assert!(
            package_count >= 40,
            "lock unexpectedly lost transitive packages"
        );
        assert!(
            current_package_has_hash,
            "last lock entry has no SHA-256 hash"
        );
        for requirement in REQUIREMENTS {
            assert!(REQUIREMENTS_LOCK.contains(requirement));
        }
    }

    #[test]
    fn package_install_uses_the_checked_hash_lock() {
        let runtime = TestRuntime::new();
        let command = package_install_command(Path::new("/test/uv"), &runtime.0);
        let args: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert!(args.iter().any(|arg| arg == "--require-hashes"));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--only-binary", ":all:"])
        );
        let lock_index = args.iter().position(|arg| arg == "-r").unwrap();
        assert_eq!(
            args[lock_index + 1],
            runtime.0.join(REQUIREMENTS_LOCK_FILE).display().to_string()
        );
        assert!(!args.iter().any(|arg| REQUIREMENTS.contains(&arg.as_str())));
    }

    #[test]
    fn runtime_readiness_is_bound_to_the_embedded_hash_lock() {
        let runtime = TestRuntime::new();
        fs::create_dir_all(runtime.0.join("venv/bin")).unwrap();
        fs::create_dir_all(runtime.0.join("assets")).unwrap();
        fs::write(python(&runtime.0), b"synthetic python").unwrap();
        fs::write(runtime.0.join("model.yaml"), b"synthetic model config").unwrap();
        fs::write(runtime.0.join(REQUIREMENTS_LOCK_FILE), REQUIREMENTS_LOCK).unwrap();

        let packages: HashMap<_, _> = REQUIREMENTS
            .iter()
            .map(|requirement| {
                let (name, version) = requirement.split_once("==").unwrap();
                (name.to_owned(), version.to_owned())
            })
            .collect();
        let assets: HashMap<_, _> = ASSET_NAMES
            .iter()
            .map(|name| {
                let path = runtime.0.join("assets").join(name);
                fs::write(&path, b"synthetic asset").unwrap();
                (
                    (*name).to_owned(),
                    serde_json::json!({
                        "size": fs::metadata(path).unwrap().len(),
                        "sha256": "a".repeat(64),
                    }),
                )
            })
            .collect();
        fs::write(
            runtime.0.join("ready.json"),
            serde_json::to_vec(&serde_json::json!({
                "modelId": MODEL_ID,
                "packageVersion": PACKAGE_VERSION,
                "pythonVersion": PYTHON_VERSION,
                "packages": packages,
                "assets": assets,
                "configSha256": "b".repeat(64),
                "requirementsSha256": requirements_sha256(),
            }))
            .unwrap(),
        )
        .unwrap();
        assert!(installed(&runtime.0));

        fs::write(runtime.0.join(REQUIREMENTS_LOCK_FILE), b"different lock").unwrap();
        assert!(!installed(&runtime.0));
    }

    #[test]
    fn worker_process_receives_the_compiled_requirements_hash() {
        let runtime = TestRuntime::new();
        let command = worker_command(&runtime.0, "setup", false);
        assert!(command.get_envs().any(|(key, value)| {
            key == "CODE_ATLAS_REQUIREMENTS_SHA256"
                && value == Some(std::ffi::OsStr::new(&requirements_sha256()))
        }));
    }
}
