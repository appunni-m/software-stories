//! App-managed, finite Pocket TTS processes. No listening socket or cloud fallback.
//!
//! Setup downloads a managed interpreter, pinned packages, and immutable model
//! assets into app data. Generation uses those local assets and a private pipe.

use crate::provider::{ProviderFailure, ProviderFailureKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

#[cfg(unix)]
use std::os::unix::{fs::PermissionsExt, process::CommandExt};

pub(crate) const PACKAGE_VERSION: &str = "3.3.0";
pub(crate) const PYTHON_VERSION: &str = "3.13.14";
pub(crate) const MODEL_ID: &str =
    "pocket-tts-3.3.0:english_2026-09:e7205b6e:alba:4e1e0a3e:pcm-v1";
pub(crate) const VOICE: &str = "alba";
const WORKER: &str = include_str!("../resources/pocket_tts_worker.py");
const REQUIREMENTS: [&str; 5] = ["pocket-tts==3.3.0", "torch==2.14.1", "numpy==2.5.4", "huggingface-hub==2.2.0", "pyyaml==6.0.3"];
const MAX_WAV_BYTES: usize = 12 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 16_000;
const ASSET_NAMES: [&str; 3] = ["model.safetensors", "tokenizer.json", "alba.safetensors"];
static BUSY: AtomicBool = AtomicBool::new(false);
static SHUTTING_DOWN: AtomicBool = AtomicBool::new(false);
static ACTIVE_PROCESS: Mutex<Option<u32>> = Mutex::new(None);
static PHASE: Mutex<&'static str> = Mutex::new("idle");

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
}

#[derive(Deserialize)]
struct AssetManifest {
    size: u64,
    sha256: String,
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
    for requirement in REQUIREMENTS { hash.update(requirement.as_bytes()); }
    format!("{:x}", hash.finalize())
}

fn python(root: &Path) -> PathBuf {
    root.join("venv/bin/python")
}

fn supported_platform() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
}

fn installer() -> Option<PathBuf> {
    let mut candidates = vec![PathBuf::from("/opt/homebrew/bin/uv"), PathBuf::from("/usr/local/bin/uv")];
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".local/bin/uv"));
    }
    candidates.into_iter().find(|path| {
        fs::metadata(path).is_ok_and(|metadata| {
            #[cfg(unix)]
            { metadata.is_file() && metadata.permissions().mode() & 0o111 != 0 }
            #[cfg(not(unix))]
            { metadata.is_file() }
        })
    })
}

fn installed(root: &Path) -> bool {
    let Some(bytes) = bounded_read(&root.join("ready.json"), 64 * 1024) else { return false; };
    let Ok(manifest) = serde_json::from_slice::<RuntimeManifest>(&bytes) else { return false; };
    manifest.model_id == MODEL_ID
        && manifest.package_version == PACKAGE_VERSION
        && manifest.python_version == PYTHON_VERSION
        && REQUIREMENTS.iter().all(|requirement| {
            requirement.split_once("==").is_some_and(|(name, version)| {
                manifest.packages.get(name).is_some_and(|installed| installed == version)
            })
        })
        && manifest.config_sha256.len() == 64
        && python(root).is_file()
        && root.join("model.yaml").is_file()
        && ASSET_NAMES.iter().all(|name| {
            manifest.assets.get(*name).is_some_and(|asset| {
                asset.size > 0 && asset.sha256.len() == 64
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
    let setup_available = supported_platform && installer().is_some();
    let message = if !supported_platform {
        "Pocket TTS setup currently targets Apple Silicon Macs. This runtime is unavailable on this platform."
    } else if busy {
        match phase {
            "python" => "Preparing the app's Python runtime…",
            "packages" => "Downloading and installing Pocket TTS dependencies…",
            "model" => "Downloading the pinned English model and Alba voice, then loading them locally…",
            "synthesizing" => "Generating narration locally with Pocket TTS…",
            _ => "Preparing Pocket TTS…",
        }
    } else if ready {
        "Pocket TTS is installed. English narration uses the local Alba voice with no API key."
    } else if setup_available {
        "Set up Pocket TTS once to download Python, dependencies, the English model and Alba voice into app storage. Generation then uses local files."
    } else {
        "Install uv from docs.astral.sh/uv/getting-started/installation/ into its default location, then retry setup. The packaged app does not yet bundle this prerequisite."
    };
    PocketTtsStatus {
        ready, busy, setup_available, supported_platform,
        package_version: PACKAGE_VERSION.into(), python_version: PYTHON_VERSION.into(),
        model_id: MODEL_ID.into(), voice: VOICE.into(), phase: phase.into(), message: message.into(),
    }
}

/// Stop the owned process group before the desktop app exits. Call from Tauri's
/// exit event; the worker thread normally reaps the direct child on its next poll.
pub(crate) fn shutdown() {
    SHUTTING_DOWN.store(true, Ordering::Release);
    if let Ok(active) = ACTIVE_PROCESS.lock() {
        if let Some(pid) = *active { kill_process_group(pid); }
    }
}

/// Explicit setup is the only operation permitted to download runtime assets.
pub(crate) async fn setup(
    storage: &Path,
    cancellation: &mut oneshot::Receiver<()>,
) -> Result<PocketTtsStatus, ProviderFailure> {
    let runtime = root(storage);
    run_exclusive(cancellation, move |cancelled| setup_blocking(&runtime, &cancelled)).await?;
    Ok(status(storage))
}

/// Generate one section with a finite, offline worker and return validated PCM WAV.
pub(crate) async fn synthesize(
    storage: &Path,
    text: &str,
    cancellation: &mut oneshot::Receiver<()>,
) -> Result<Vec<u8>, ProviderFailure> {
    if text.trim().is_empty() || text.len() > MAX_TEXT_BYTES {
        return Err(failure(ProviderFailureKind::InvalidReview, "Pocket TTS needs 1 to 16,000 UTF-8 bytes of narration text."));
    }
    let runtime = root(storage);
    let text = text.to_owned();
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
        run_child(&mut command, Some(text.as_bytes()), &cancelled, Duration::from_secs(300),
            "Pocket TTS could not generate audio. Run setup again if its files changed; the transcript and prior audio remain available.")?;
        let bytes = bounded_read(&output, MAX_WAV_BYTES).ok_or_else(|| failure(
            ProviderFailureKind::ProviderResponseTooLarge, "Pocket TTS returned missing or oversized audio. The transcript and prior audio remain available."))?;
        validate_wav(&bytes)?;
        Ok(bytes)
    }).await
}

async fn run_exclusive<T: Send + 'static>(
    cancellation: &mut oneshot::Receiver<()>,
    operation: impl FnOnce(Arc<AtomicBool>) -> Result<T, ProviderFailure> + Send + 'static,
) -> Result<T, ProviderFailure> {
    if SHUTTING_DOWN.load(Ordering::Acquire) { return Err(cancelled_failure()); }
    if !supported_platform() {
        return Err(failure(ProviderFailureKind::UnavailableModel, "Pocket TTS currently requires an Apple Silicon Mac."));
    }
    BUSY.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .map_err(|_| failure(ProviderFailureKind::ReviewUnavailable, "Pocket TTS is already working. Wait for it or cancel the active request."))?;
    let cancelled = Arc::new(AtomicBool::new(false));
    let _cancel_on_drop = CancelOnDrop(Arc::clone(&cancelled));
    let task_cancelled = Arc::clone(&cancelled);
    let lease = OperationLease;
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

struct OperationLease;
impl Drop for OperationLease {
    fn drop(&mut self) { set_phase("idle"); BUSY.store(false, Ordering::Release); }
}
struct CancelOnDrop(Arc<AtomicBool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) { self.0.store(true, Ordering::Release); }
}
struct OutputCleanup(PathBuf);
impl Drop for OutputCleanup {
    fn drop(&mut self) { let _ = fs::remove_file(&self.0); }
}

fn setup_blocking(runtime: &Path, cancelled: &AtomicBool) -> Result<(), ProviderFailure> {
    let uv = installer().ok_or_else(|| failure(ProviderFailureKind::UnavailableModel,
        "Pocket TTS setup needs uv. Install it from docs.astral.sh/uv/getting-started/installation/ in its default location, then retry."))?;
    fs::create_dir_all(runtime).map_err(|_| storage_failure())?;
    #[cfg(unix)]
    fs::set_permissions(runtime, fs::Permissions::from_mode(0o700)).map_err(|_| storage_failure())?;
    for directory in ["tmp", "uv-cache", "python", "huggingface", "cache"] {
        fs::create_dir_all(runtime.join(directory)).map_err(|_| storage_failure())?;
    }
    let _ = fs::remove_file(runtime.join("ready.json"));
    write_worker(runtime)?;
    set_phase("python");
    let mut venv = Command::new(&uv);
    configure_environment(&mut venv, runtime, false);
    venv.args(["venv", "--clear", "--no-config", "--managed-python", "--python", PYTHON_VERSION])
        .arg(runtime.join("venv"));
    run_child(&mut venv, None, cancelled, Duration::from_secs(900),
        "Pocket TTS could not prepare Python. Check your connection and available disk space, then retry setup.")?;
    set_phase("packages");
    let mut install = Command::new(uv);
    configure_environment(&mut install, runtime, false);
    install.args(["pip", "install", "--no-config", "--only-binary", ":all:", "--python"])
        .arg(python(runtime))
        .args(REQUIREMENTS);
    run_child(&mut install, None, cancelled, Duration::from_secs(1800),
        "Pocket TTS dependencies could not be installed. Check your connection and available disk space, then retry setup.")?;
    set_phase("model");
    let mut download = worker_command(runtime, "setup", false);
    run_child(&mut download, None, cancelled, Duration::from_secs(1200),
        "Pocket TTS could not download or load its model and voice. Check your connection and available memory/disk space, then retry setup.")?;
    if !installed(runtime) {
        return Err(failure(ProviderFailureKind::UnavailableModel, "Pocket TTS setup did not produce a complete runtime. Retry setup."));
    }
    Ok(())
}

fn write_worker(runtime: &Path) -> Result<(), ProviderFailure> {
    fs::write(runtime.join("worker.py"), WORKER).map_err(|_| storage_failure())
}

fn worker_command(runtime: &Path, mode: &str, offline: bool) -> Command {
    let mut command = Command::new(python(runtime));
    configure_environment(&mut command, runtime, offline);
    command.arg("-I").arg(runtime.join("worker.py")).arg(mode).arg(runtime);
    command
}

fn configure_environment(command: &mut Command, runtime: &Path, offline: bool) {
    command.env_clear().current_dir(runtime)
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
    if let Some(home) = std::env::var_os("HOME") { command.env("HOME", home); }
    if offline { command.env("HF_HUB_OFFLINE", "1").env("UV_OFFLINE", "1"); }
    command.stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(unix)]
    command.process_group(0);
}

fn run_child(command: &mut Command, input: Option<&[u8]>, cancelled: &AtomicBool, timeout: Duration, message: &'static str) -> Result<(), ProviderFailure> {
    if cancelled.load(Ordering::Acquire) || SHUTTING_DOWN.load(Ordering::Acquire) { return Err(cancelled_failure()); }
    command.stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() });
    // Serialize registration with shutdown, so exit cannot miss a just-spawned
    // process or signal a PID after this owner has reaped it.
    let mut active = ACTIVE_PROCESS.lock().map_err(|_| failure(ProviderFailureKind::UnavailableModel, message))?;
    if SHUTTING_DOWN.load(Ordering::Acquire) { return Err(cancelled_failure()); }
    let child = command.spawn().map_err(|_| failure(ProviderFailureKind::UnavailableModel, message))?;
    *active = Some(child.id());
    drop(active);
    let mut child = ManagedChild { child, reaped: false };
    if let Some(input) = input {
        let mut stdin = child.child.stdin.take().ok_or_else(|| failure(ProviderFailureKind::UnavailableModel, message))?;
        stdin.write_all(input).map_err(|_| failure(ProviderFailureKind::UnavailableModel, message))?;
    }
    let started = Instant::now();
    loop {
        if cancelled.load(Ordering::Acquire) || SHUTTING_DOWN.load(Ordering::Acquire) { return Err(cancelled_failure()); }
        if started.elapsed() >= timeout { return Err(failure(ProviderFailureKind::RequestTimedOut,
            "Pocket TTS reached its local time limit and was stopped. The transcript and prior audio remain available.")); }
        let mut active = ACTIVE_PROCESS.lock().map_err(|_| failure(ProviderFailureKind::UnavailableModel, message))?;
        match child.child.try_wait() {
            Ok(Some(status)) => {
                *active = None;
                child.reaped = true;
                return if status.success() { Ok(()) } else { Err(failure(ProviderFailureKind::UnavailableModel, message)) };
            }
            Ok(None) => { drop(active); std::thread::sleep(Duration::from_millis(50)); }
            Err(_) => return Err(failure(ProviderFailureKind::UnavailableModel, message)),
        }
    }
}

struct ManagedChild { child: Child, reaped: bool }
impl Drop for ManagedChild {
    fn drop(&mut self) {
        if self.reaped { return; }
        let mut active = ACTIVE_PROCESS.lock().ok();
        kill_process_group(self.child.id());
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(active) = active.as_mut() { **active = None; }
    }
}

fn kill_process_group(process_id: u32) {
    #[cfg(unix)]
    if let Ok(pid) = i32::try_from(process_id) {
        // SAFETY: process_group(0) created an owned group. Its lifecycle lock
        // prevents reaping/reuse until the signal is sent. No arbitrary PID is accepted.
        unsafe { libc::kill(-pid, libc::SIGKILL); }
    }
    #[cfg(not(unix))]
    let _ = process_id;
}

fn bounded_read(path: &Path, limit: usize) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    fs::File::open(path).ok()?.take(limit as u64 + 1).read_to_end(&mut bytes).ok()?;
    (bytes.len() <= limit).then_some(bytes)
}

fn validate_wav(bytes: &[u8]) -> Result<(), ProviderFailure> {
    let valid = bytes.len() >= 46 && bytes.len() <= MAX_WAV_BYTES
        && &bytes[..4] == b"RIFF" && &bytes[8..16] == b"WAVEfmt "
        && bytes[16..20] == 16_u32.to_le_bytes()
        && bytes[20..22] == 1_u16.to_le_bytes() && bytes[22..24] == 1_u16.to_le_bytes()
        && bytes[24..28] == 24_000_u32.to_le_bytes() && bytes[28..32] == 48_000_u32.to_le_bytes()
        && bytes[32..34] == 2_u16.to_le_bytes() && bytes[34..36] == 16_u16.to_le_bytes()
        && &bytes[36..40] == b"data"
        && bytes[4..8] == ((bytes.len() - 8) as u32).to_le_bytes()
        && bytes[40..44] == ((bytes.len() - 44) as u32).to_le_bytes()
        && (bytes.len() - 44) % 2 == 0;
    if valid { Ok(()) } else { Err(failure(ProviderFailureKind::MalformedProviderResponse,
        "Pocket TTS returned invalid audio. The transcript and prior audio remain available.")) }
}

fn set_phase(phase: &'static str) { if let Ok(mut current) = PHASE.lock() { *current = phase; } }
fn failure(kind: ProviderFailureKind, message: impl Into<String>) -> ProviderFailure { ProviderFailure { kind, message: message.into() } }
fn storage_failure() -> ProviderFailure { failure(ProviderFailureKind::LocalStorageUnavailable, "Pocket TTS could not write to app storage. Check available disk space and folder permissions.") }
fn cancelled_failure() -> ProviderFailure { failure(ProviderFailureKind::RequestCancelled, "Pocket TTS was stopped locally. The transcript and prior audio remain available.") }
