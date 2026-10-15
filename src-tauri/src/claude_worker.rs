//! Managed execution for the official Claude programmatic CLI.
//!
//! There is no registered generation command for this worker yet. Its native
//! caller must first qualify the account/runtime/isolation route, re-inspect
//! the account and model catalog, and revalidate an explicitly consented review.
//! This is process supervision, not an agent loop or a permission boundary.

use crate::claude_agent::{
    self, ClaudeGenerationSettings, ClaudeOutputStream, ClaudeProgressPhase, ClaudeReviewRuntime,
    ClaudeStructuredResult,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CANCEL_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_CANCELLED_RUNS: usize = 256;
const BUSY: &str =
    "Claude Agent is busy. Finish or cancel the active operation before starting another.";

pub(crate) struct ClaudeWorkerInput {
    pub run_id: String,
    pub settings: ClaudeGenerationSettings,
    pub system_instruction: String,
    pub user_prompt: String,
    pub output_schema: Value,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeWorkerProgress {
    pub run_id: String,
    pub phase: &'static str,
    /// Application-authored text only; never raw stdout, stderr or source.
    pub message: &'static str,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ClaudeWorkerFailureKind {
    Busy,
    Cancelled,
    TimedOut,
    RuntimeChanged,
    PreparationFailed,
    ProcessFailed,
    ProtocolRejected,
    OutputLimit,
    CleanupFailed,
    UnsupportedPlatform,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClaudeWorkerFailure {
    pub kind: ClaudeWorkerFailureKind,
    /// Fixed native diagnostics only. Never format an OS or agent error here.
    pub message: String,
}

fn failure(kind: ClaudeWorkerFailureKind, message: impl Into<String>) -> ClaudeWorkerFailure {
    ClaudeWorkerFailure {
        kind,
        message: message.into(),
    }
}

#[derive(Clone, Default)]
pub(crate) struct ClaudeWorkerManager(Arc<ManagerInner>);

#[derive(Default)]
struct ManagerInner {
    operation: Mutex<()>,
    active: Mutex<Option<ActiveRun>>,
    cancelled: Mutex<HashMap<String, Instant>>,
    /// Only the child owner reaps. Shutdown signals under this same lock, so a
    /// reaped/reused PID can never remain published to another signal sender.
    child_pid: Mutex<Option<u32>>,
    shutdown: AtomicBool,
    cleanup_failed: AtomicBool,
}

struct ActiveRun {
    id: String,
    cancelled: Arc<AtomicBool>,
}

impl ClaudeWorkerManager {
    /// Blocking: run on the native blocking pool after fresh review/account
    /// qualification. No public/native command currently calls this method.
    pub(crate) fn run(
        &self,
        storage_dir: &Path,
        runtime: &ClaudeReviewRuntime,
        input: &ClaudeWorkerInput,
        mut progress: impl FnMut(ClaudeWorkerProgress),
    ) -> Result<ClaudeStructuredResult, ClaudeWorkerFailure> {
        self.with_operation(&input.run_id, |cancelled| {
            progress(ClaudeWorkerProgress {
                run_id: input.run_id.clone(), phase: "preparing",
                message: "Preparing the reviewed Claude request in private temporary storage.",
            });
            #[cfg(unix)]
            { native::run(&self.0, storage_dir, runtime, input, cancelled, &mut progress) }
            #[cfg(not(unix))]
            {
                let _ = (storage_dir, runtime, input, progress, cancelled);
                Err(failure(ClaudeWorkerFailureKind::UnsupportedPlatform,
                    "This Claude worker requires the reviewed Unix process and pipe controls. No process was started."))
            }
        })
    }

    /// Explicit, source-free metadata inspection. This does not authenticate,
    /// discover models, establish eligibility or enable automatic generation.
    pub(crate) fn inspect_connection(
        &self,
        storage_dir: &Path,
        executable: &Path,
        run_id: &str,
    ) -> Result<crate::claude_connection::ClaudeConnectionObservation, ClaudeWorkerFailure> {
        self.with_operation(run_id, |cancelled| {
            #[cfg(unix)]
            { native::inspect_connection(&self.0, storage_dir, executable, cancelled) }
            #[cfg(not(unix))]
            {
                let _ = (storage_dir, executable, cancelled);
                Err(failure(ClaudeWorkerFailureKind::UnsupportedPlatform,
                    "Claude CLI inspection requires the reviewed Unix process controls. No process was started."))
            }
        })
    }

    /// Candidate discovery for a native-owned, verified runtime installation.
    /// No renderer command currently calls this. Catalog metadata never
    /// establishes account eligibility or enables explanation generation.
    pub(crate) fn discover_models(
        &self,
        storage_dir: &Path,
        runtime: &crate::claude_discovery::ClaudeDiscoveryRuntime,
        run_id: &str,
    ) -> Result<crate::claude_discovery::ClaudeModelDiscoveryObservation, ClaudeWorkerFailure> {
        self.with_operation(run_id, |cancelled| {
            #[cfg(unix)]
            { native::discover_models(&self.0, storage_dir, runtime, cancelled) }
            #[cfg(not(unix))]
            {
                let _ = (storage_dir, runtime, cancelled);
                Err(failure(ClaudeWorkerFailureKind::UnsupportedPlatform,
                    "Claude model discovery requires the reviewed Unix process controls. No process was started."))
            }
        })
    }

    fn with_operation<T>(
        &self,
        run_id: &str,
        operation: impl FnOnce(&AtomicBool) -> Result<T, ClaudeWorkerFailure>,
    ) -> Result<T, ClaudeWorkerFailure> {
        use ClaudeWorkerFailureKind as Kind;
        if uuid::Uuid::parse_str(run_id).is_err() {
            return Err(failure(
                Kind::PreparationFailed,
                "The Claude run identity is invalid. Prepare a fresh review.",
            ));
        }
        let _operation = self
            .0
            .operation
            .try_lock()
            .map_err(|_| failure(Kind::Busy, BUSY))?;
        if self.0.cleanup_failed.load(Ordering::Acquire) {
            return Err(failure(
                Kind::CleanupFailed,
                "A previous Claude cleanup could not be confirmed. Resolve the native process/storage state before restarting this app and generating again.",
            ));
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        {
            // cancel() uses the same lock order, covering cancellation before
            // a queued blocking task can register its active operation.
            let mut active = self
                .0
                .active
                .lock()
                .map_err(|_| failure(Kind::Busy, BUSY))?;
            let mut tombstones = self
                .0
                .cancelled
                .lock()
                .map_err(|_| failure(Kind::Busy, BUSY))?;
            tombstones.retain(|_, at| at.elapsed() < CANCEL_TTL);
            if tombstones.remove(run_id).is_some() {
                return Err(cancelled_failure());
            }
            if active.is_some() {
                return Err(failure(Kind::Busy, BUSY));
            }
            *active = Some(ActiveRun {
                id: run_id.to_owned(),
                cancelled: cancelled.clone(),
            });
        }
        let _active = ActiveGuard(self.0.clone());
        check_cancelled(&self.0, &cancelled)?;
        let result = operation(&cancelled);
        if result
            .as_ref()
            .err()
            .is_some_and(|error| matches!(error.kind, Kind::CleanupFailed))
        {
            self.0.cleanup_failed.store(true, Ordering::Release);
            return result;
        }
        if self.0.cleanup_failed.load(Ordering::Acquire) {
            return Err(failure(
                Kind::CleanupFailed,
                "Claude process cleanup could not be confirmed. No response was accepted and further runs are disabled for this app session.",
            ));
        }
        // A cancel arriving during final draining/cleanup still wins over a
        // valid response; the caller must never save it as a generated draft.
        check_cancelled(&self.0, &cancelled)?;
        result
    }

    pub(crate) fn cancel(&self, run_id: &str) -> bool {
        if uuid::Uuid::parse_str(run_id).is_err() {
            return false;
        }
        let Ok(active) = self.0.active.lock() else {
            return false;
        };
        if let Some(run) = active.as_ref().filter(|run| run.id == run_id) {
            run.cancelled.store(true, Ordering::Release);
            return true;
        }
        let Ok(mut tombstones) = self.0.cancelled.lock() else {
            return false;
        };
        tombstones.retain(|_, at| at.elapsed() < CANCEL_TTL);
        if tombstones.len() >= MAX_CANCELLED_RUNS {
            if let Some(oldest) = tombstones
                .iter()
                .min_by_key(|(_, at)| **at)
                .map(|(id, _)| id.clone())
            {
                tombstones.remove(&oldest);
            }
        }
        tombstones.insert(run_id.to_owned(), Instant::now());
        true
    }

    /// Signal the owned group immediately; the blocking owner interrupts,
    /// escalates and reaps it. This starts no process and reads no credentials.
    pub(crate) fn shutdown(&self) {
        self.0.shutdown.store(true, Ordering::Release);
        if let Ok(active) = self.0.active.lock() {
            if let Some(run) = active.as_ref() {
                run.cancelled.store(true, Ordering::Release);
            }
        }
        #[cfg(unix)]
        {
            let pid = self
                .0
                .child_pid
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(pid) = *pid {
                let _ = native::signal_group(pid, libc::SIGINT);
            }
        }
    }
}

struct ActiveGuard(Arc<ManagerInner>);
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        let mut active = self
            .0
            .active
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        active.take();
    }
}

fn cancelled_failure() -> ClaudeWorkerFailure {
    failure(
        ClaudeWorkerFailureKind::Cancelled,
        "The Claude run was cancelled locally. No response was accepted; account usage may already have occurred.",
    )
}

fn check_cancelled(
    manager: &ManagerInner,
    cancelled: &AtomicBool,
) -> Result<(), ClaudeWorkerFailure> {
    if manager.shutdown.load(Ordering::Acquire) || cancelled.load(Ordering::Acquire) {
        Err(cancelled_failure())
    } else {
        Ok(())
    }
}

#[cfg(unix)]
mod native {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::fs::{self, OpenOptions};
    use std::io::{self, Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    use std::path::PathBuf;
    use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, ExitStatus, Stdio};
    use std::thread;

    const RUN_TIMEOUT: Duration = Duration::from_secs(600);
    const POLL_INTERVAL: Duration = Duration::from_millis(10);
    const INTERRUPT_GRACE: Duration = Duration::from_secs(2);
    const TERMINATE_GRACE: Duration = Duration::from_secs(1);
    const PIPE_DRAIN_TIMEOUT: Duration = Duration::from_secs(2);
    const MAX_EXECUTABLE_BYTES: u64 = 256 * 1024 * 1024;
    const MAX_STDERR_BYTES: usize = 256 * 1024;
    const IO_CHUNK: usize = 16 * 1024;
    const IO_BURST: usize = 128 * 1024;
    const INSPECTION_TIMEOUT: Duration = Duration::from_secs(45);

    pub(super) fn inspect_connection(
        manager: &Arc<ManagerInner>,
        storage_dir: &Path,
        executable: &Path,
        cancelled: &AtomicBool,
    ) -> Result<crate::claude_connection::ClaudeConnectionObservation, ClaudeWorkerFailure> {
        inspect_connection_at(manager, storage_dir, executable, cancelled, Instant::now())
    }

    fn inspect_connection_at(
        manager: &Arc<ManagerInner>,
        storage_dir: &Path,
        executable: &Path,
        cancelled: &AtomicBool,
        started: Instant,
    ) -> Result<crate::claude_connection::ClaudeConnectionObservation, ClaudeWorkerFailure> {
        use crate::claude_agent::ClaudeInspectionCommand as Inspection;
        use crate::claude_connection::{self, ClaudeAuthMethod, ClaudeConnectionObservation};
        use ClaudeWorkerFailureKind as Kind;
        let check = || inspection_deadline(manager, cancelled, started);
        check()?;
        let (path, digest) = inspect_executable(executable, check)?;
        let stage = PrivateStage::create(storage_dir, &manager.cleanup_failed)?;
        let account_home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| {
                failure(
                    Kind::PreparationFailed,
                    "The native account home is unavailable. No Claude inspection was started.",
                )
            })?;
        let (version_bytes, version_exit) = inspect_command(
            manager,
            &path,
            Inspection::Version,
            &stage,
            &account_home,
            cancelled,
            started,
            128,
        )?;
        if !version_exit.success() {
            return Err(failure(
                Kind::ProcessFailed,
                "The selected Claude executable did not complete its version check.",
            ));
        }
        let version = claude_connection::parse_version(&version_bytes)
            .map_err(|message| failure(Kind::ProtocolRejected, message))?;
        let matches = version == claude_agent::CANDIDATE_RUNTIME_VERSION;
        let mut account = None;
        let message = if matches {
            let identity = inspect_executable(&path, check)?;
            if identity.0 != path || identity.1 != digest {
                return Err(failure(
                    Kind::RuntimeChanged,
                    "The Claude executable changed during inspection. Check installation again.",
                ));
            }
            let (bytes, status) = inspect_command(
                manager,
                &path,
                Inspection::AccountStatus,
                &stage,
                &account_home,
                cancelled,
                started,
                64 * 1024,
            )?;
            let observed = claude_connection::parse_account_status(&bytes, status.code())
                .map_err(|message| failure(Kind::ProtocolRejected, message))?;
            let message = if !observed.logged_in {
                "Claude reports no active account. Automatic generation is still unavailable."
            } else if matches!(
                observed.auth_method,
                ClaudeAuthMethod::ApiKey
                    | ClaudeAuthMethod::ApiKeyHelper
                    | ClaudeAuthMethod::ThirdParty
            ) {
                "Claude reports API-key or alternate-provider authentication. This does not qualify the account-backed generation route. No prompt was sent."
            } else if observed.account_identity_sha256.is_some() {
                "Claude reports an existing account login. Model discovery, product eligibility and automatic generation remain pending."
            } else {
                "Claude reported incomplete or unsupported account metadata. Automatic generation remains unavailable."
            };
            account = Some(observed);
            message
        } else {
            "This Claude version differs from the candidate adapter version. Account inspection was skipped. Automatic generation remains unavailable."
        };
        let identity = inspect_executable(&path, check)?;
        if identity.0 != path || identity.1 != digest {
            return Err(failure(
                Kind::RuntimeChanged,
                "The Claude executable changed during inspection. Check installation again.",
            ));
        }
        stage.cleanup()?;
        check()?;
        Ok(ClaudeConnectionObservation {
            executable_path: path
                .to_str()
                .ok_or_else(|| {
                    failure(
                        Kind::PreparationFailed,
                        "The Claude executable path is not valid UTF-8.",
                    )
                })?
                .to_owned(),
            executable_sha256: digest,
            runtime_version: version,
            candidate_version_matches: matches,
            account,
            automatic_generation_available: false,
            message,
        })
    }

    pub(super) fn discover_models(
        manager: &Arc<ManagerInner>,
        storage_dir: &Path,
        runtime: &crate::claude_discovery::ClaudeDiscoveryRuntime,
        cancelled: &AtomicBool,
    ) -> Result<crate::claude_discovery::ClaudeModelDiscoveryObservation, ClaudeWorkerFailure> {
        use crate::{claude_discovery as discovery, claude_models};
        use ClaudeWorkerFailureKind as Kind;
        let started = Instant::now();
        let check = || inspection_deadline(manager, cancelled, started);
        verify_discovery_executables(runtime, check)?;
        let before = inspect_connection_at(
            manager,
            storage_dir,
            &runtime.claude_executable,
            cancelled,
            started,
        )?;
        let account_identity = discovery_account(&before)?.to_owned();
        if Path::new(&before.executable_path) != runtime.claude_executable
            || before.executable_sha256 != runtime.claude_sha256
        {
            return Err(failure(
                Kind::RuntimeChanged,
                "Claude changed before model discovery. Prepare fresh native observations.",
            ));
        }
        let sdk = read_discovery_sdk(&runtime.sdk_module, check)?;
        let stage = PrivateStage::create(storage_dir, &manager.cleanup_failed)?;
        let result = (|| {
            stage.write_discovery_files(&sdk, check)?;
            let account_home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
                .ok_or_else(|| {
                    failure(
                        Kind::PreparationFailed,
                        "The native account home is unavailable. No SDK discovery was started.",
                    )
                })?;
            let (mut command, input) =
                discovery::prepare(runtime, &stage.path, &stage.temporary_path(), &account_home)
                    .map_err(|message| failure(Kind::PreparationFailed, message))?;
            let (bytes, status) = run_metadata_command(
                manager,
                &mut command,
                &input,
                cancelled,
                started,
                claude_models::MAX_CATALOG_BYTES,
            )?;
            if !status.success() {
                let (kind, message) = match status.code() {
                    Some(20) => (
                        Kind::ProtocolRejected,
                        "The Claude SDK discovery contract was incompatible. No catalog was accepted.",
                    ),
                    Some(21) => (
                        Kind::Cancelled,
                        "Claude model discovery was cancelled. No catalog was accepted.",
                    ),
                    Some(22) => (
                        Kind::TimedOut,
                        "Claude SDK model discovery timed out. No catalog was accepted.",
                    ),
                    Some(24) => (
                        Kind::CleanupFailed,
                        "Claude SDK child cleanup could not be confirmed. Further operations are disabled for this app session.",
                    ),
                    Some(25) => (
                        Kind::OutputLimit,
                        "Claude SDK metadata exceeded a raw pipe limit. No catalog was accepted.",
                    ),
                    _ => (
                        Kind::ProcessFailed,
                        "Claude SDK discovery did not complete successfully. No catalog was accepted.",
                    ),
                };
                return Err(failure(kind, message));
            }
            let catalog = claude_models::parse_model_catalog(&bytes)
                .map_err(|message| failure(Kind::ProtocolRejected, message))?;
            // Staging is read-only, not an OS sandbox. Reject observed mutation
            // and qualify pre-execution confinement separately before release.
            read_discovery_sdk(&stage.path.join("sdk.mjs"), check)?;
            verify_discovery_executables(runtime, check)?;
            let after = inspect_connection_at(
                manager,
                storage_dir,
                &runtime.claude_executable,
                cancelled,
                started,
            )?;
            if discovery_account(&after)? != account_identity
                || after.executable_path != before.executable_path
                || after.executable_sha256 != before.executable_sha256
            {
                return Err(failure(
                    Kind::RuntimeChanged,
                    "Claude account or executable metadata changed during discovery. Refresh the connection.",
                ));
            }
            check()?;
            Ok(discovery::ClaudeModelDiscoveryObservation {
                connection: after,
                catalog,
                node_executable_path: runtime
                    .node_executable
                    .to_str()
                    .ok_or_else(|| {
                        failure(
                            Kind::PreparationFailed,
                            "The native SDK host path is not valid UTF-8.",
                        )
                    })?
                    .to_owned(),
                node_executable_sha256: runtime.node_sha256.clone(),
                sdk_module_sha256: discovery::SDK_MODULE_SHA256,
            })
        })();
        stage.cleanup()?;
        if result
            .as_ref()
            .err()
            .is_some_and(|error| matches!(error.kind, Kind::CleanupFailed))
        {
            return result;
        }
        check()?;
        result
    }

    fn discovery_account(
        observation: &crate::claude_connection::ClaudeConnectionObservation,
    ) -> Result<&str, ClaudeWorkerFailure> {
        use crate::claude_connection::ClaudeAuthMethod;
        let identity = observation
            .account
            .as_ref()
            .filter(|account| {
                observation.candidate_version_matches
                    && account.logged_in
                    && account.auth_method == ClaudeAuthMethod::ClaudeAccount
                    && account.first_party_provider
            })
            .and_then(|account| account.account_identity_sha256.as_deref());
        identity.ok_or_else(|| failure(ClaudeWorkerFailureKind::PreparationFailed,
            "An unchanged candidate runtime and reported first-party Claude account are required for this catalog check. No API fallback was used."))
    }

    fn verify_discovery_executables(
        runtime: &crate::claude_discovery::ClaudeDiscoveryRuntime,
        mut check: impl FnMut() -> Result<(), ClaudeWorkerFailure>,
    ) -> Result<(), ClaudeWorkerFailure> {
        for (path, expected) in [
            (&runtime.node_executable, &runtime.node_sha256),
            (&runtime.claude_executable, &runtime.claude_sha256),
        ] {
            let (canonical, digest) = inspect_executable(path, &mut check)?;
            if canonical != *path || digest != *expected {
                return Err(failure(
                    ClaudeWorkerFailureKind::RuntimeChanged,
                    "A native SDK host or Claude executable identity changed. Refresh the qualified installation.",
                ));
            }
        }
        Ok(())
    }

    fn read_discovery_sdk(
        path: &Path,
        mut check: impl FnMut() -> Result<(), ClaudeWorkerFailure>,
    ) -> Result<Vec<u8>, ClaudeWorkerFailure> {
        use crate::claude_discovery::{MAX_SDK_MODULE_BYTES, SDK_MODULE_SHA256};
        let invalid = || {
            failure(
                ClaudeWorkerFailureKind::RuntimeChanged,
                "The inspected Claude SDK module is missing, changed or incompatible. No SDK catalog was accepted.",
            )
        };
        if !path.is_absolute()
            || path.to_str().is_none()
            || path.as_os_str().len() > 4096
            || path
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(invalid());
        }
        check()?;
        if fs::canonicalize(path).map_err(|_| invalid())? != path {
            return Err(invalid());
        }
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .map_err(|_| invalid())?;
        let metadata = file.metadata().map_err(|_| invalid())?;
        if !metadata.is_file()
            || metadata.len() == 0
            || metadata.len() > MAX_SDK_MODULE_BYTES as u64
        {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; IO_CHUNK];
        loop {
            check()?;
            let limit = MAX_SDK_MODULE_BYTES
                .saturating_sub(bytes.len())
                .saturating_add(1)
                .min(chunk.len());
            let count = file.read(&mut chunk[..limit]).map_err(|_| invalid())?;
            if count == 0 {
                break;
            }
            if bytes.len().saturating_add(count) > MAX_SDK_MODULE_BYTES {
                return Err(invalid());
            }
            bytes.extend_from_slice(&chunk[..count]);
        }
        if bytes.len() as u64 != metadata.len()
            || format!("{:x}", Sha256::digest(&bytes)) != SDK_MODULE_SHA256
        {
            return Err(invalid());
        }
        Ok(bytes)
    }

    fn inspection_deadline(
        manager: &ManagerInner,
        cancelled: &AtomicBool,
        started: Instant,
    ) -> Result<(), ClaudeWorkerFailure> {
        check_cancelled(manager, cancelled)?;
        if started.elapsed() >= INSPECTION_TIMEOUT {
            Err(failure(
                ClaudeWorkerFailureKind::TimedOut,
                "Claude installation/account inspection exceeded 45 seconds. No prompt was sent.",
            ))
        } else {
            Ok(())
        }
    }

    fn inspect_command(
        manager: &ManagerInner,
        executable: &Path,
        operation: claude_agent::ClaudeInspectionCommand,
        stage: &PrivateStage<'_>,
        account_home: &Path,
        cancelled: &AtomicBool,
        started: Instant,
        output_limit: usize,
    ) -> Result<(Vec<u8>, ExitStatus), ClaudeWorkerFailure> {
        use ClaudeWorkerFailureKind as Kind;
        inspection_deadline(manager, cancelled, started)?;
        let mut command = claude_agent::prepare_inspection_command(
            executable,
            operation,
            &stage.path,
            &stage.temporary_path(),
            account_home,
        )
        .map_err(|message| failure(Kind::PreparationFailed, message))?;
        run_metadata_command(manager, &mut command, &[], cancelled, started, output_limit)
    }

    fn run_metadata_command(
        manager: &ManagerInner,
        command: &mut Command,
        input: &[u8],
        cancelled: &AtomicBool,
        started: Instant,
        output_limit: usize,
    ) -> Result<(Vec<u8>, ExitStatus), ClaudeWorkerFailure> {
        use ClaudeWorkerFailureKind as Kind;
        if input.len() > crate::claude_discovery::MAX_DISCOVERY_INPUT_BYTES {
            return Err(failure(
                Kind::PreparationFailed,
                "Claude metadata configuration exceeded its local limit.",
            ));
        }
        inspection_deadline(manager, cancelled, started)?;
        let mut child = OwnedChild::spawn(manager, command, cancelled)?;
        // Metadata commands have empty input. SDK discovery receives only
        // native paths; no source, prompt or vendor RPC is sent by this owner.
        if input.is_empty() {
            child.stdin.take();
        }
        let mut written = 0;
        let mut output = Vec::new();
        let mut diagnostics = 0;
        let mut stdout_eof = false;
        let mut stderr_eof = false;
        let mut exit = None;
        let mut drain_started = None;
        let result = (|| {
            loop {
                inspection_deadline(manager, cancelled, started)?;
                if !stdout_eof {
                    stdout_eof =
                        drain_inspection_output(child.stdout()?, &mut output, output_limit)?;
                }
                if !stderr_eof {
                    stderr_eof = drain_stderr(child.stderr()?, &mut diagnostics)?;
                }
                inspection_deadline(manager, cancelled, started)?;
                if written < input.len() {
                    let end = written.saturating_add(IO_CHUNK).min(input.len());
                    let stdin = child.stdin.as_mut().ok_or_else(|| {
                        failure(
                            Kind::ProcessFailed,
                            "The Claude metadata input pipe is unavailable.",
                        )
                    })?;
                    match stdin.write(&input[written..end]) {
                        Ok(0) => {
                            return Err(failure(
                                Kind::ProcessFailed,
                                "Claude metadata input closed before configuration was delivered.",
                            ));
                        }
                        Ok(count) => written += count,
                        Err(error)
                            if matches!(
                                error.kind(),
                                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                            ) => {}
                        Err(_) => {
                            return Err(failure(
                                Kind::ProcessFailed,
                                "The native Claude metadata configuration could not be delivered.",
                            ));
                        }
                    }
                    if written == input.len() {
                        child.stdin.take();
                    }
                }
                if exit.is_none() {
                    if let Some(status) = child.try_finish()? {
                        exit = Some(status);
                        drain_started = Some(Instant::now());
                    }
                }
                if stdout_eof && stderr_eof && exit.is_some() {
                    break;
                }
                if drain_started.is_some_and(|at: Instant| at.elapsed() >= PIPE_DRAIN_TIMEOUT) {
                    return Err(failure(
                        Kind::ProcessFailed,
                        "Claude inspection exited but an output pipe did not close.",
                    ));
                }
                thread::sleep(POLL_INTERVAL);
            }
            let status = exit.ok_or_else(|| {
                failure(
                    Kind::ProcessFailed,
                    "The Claude inspection exit status is unavailable.",
                )
            })?;
            if written != input.len() {
                return Err(failure(
                    Kind::ProcessFailed,
                    "Claude metadata inspection exited before configuration delivery completed.",
                ));
            }
            Ok((output, status))
        })();
        if result.is_err() && !child.reaped {
            let graceful = result
                .as_ref()
                .err()
                .is_some_and(|error: &ClaudeWorkerFailure| {
                    matches!(error.kind, Kind::Cancelled | Kind::TimedOut)
                });
            child.stop_and_reap(graceful)?;
        }
        drop(child);
        result
    }

    fn drain_inspection_output(
        stdout: &mut ChildStdout,
        output: &mut Vec<u8>,
        limit: usize,
    ) -> Result<bool, ClaudeWorkerFailure> {
        let mut bytes = [0_u8; IO_CHUNK];
        let mut burst = 0;
        while burst < IO_BURST {
            let length = limit
                .saturating_sub(output.len())
                .saturating_add(1)
                .min(bytes.len());
            match stdout.read(&mut bytes[..length]) {
                Ok(0) => return Ok(true),
                Ok(count) => {
                    if output.len().saturating_add(count) > limit {
                        return Err(failure(
                            ClaudeWorkerFailureKind::OutputLimit,
                            "Claude inspection output exceeded its local metadata limit.",
                        ));
                    }
                    output.extend_from_slice(&bytes[..count]);
                    burst += count;
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    return Ok(false);
                }
                Err(_) => {
                    return Err(failure(
                        ClaudeWorkerFailureKind::ProcessFailed,
                        "Claude inspection output could not be read.",
                    ));
                }
            }
        }
        Ok(false)
    }

    pub(super) fn run(
        manager: &Arc<ManagerInner>,
        storage_dir: &Path,
        runtime: &ClaudeReviewRuntime,
        input: &ClaudeWorkerInput,
        cancelled: &AtomicBool,
        progress: &mut impl FnMut(ClaudeWorkerProgress),
    ) -> Result<ClaudeStructuredResult, ClaudeWorkerFailure> {
        use ClaudeWorkerFailureKind as Kind;
        let started = Instant::now();
        runtime
            .validate_settings(&input.settings)
            .map_err(|message| failure(Kind::PreparationFailed, message))?;
        verify_executable(manager, runtime, cancelled, started)?;
        let stage = PrivateStage::create(storage_dir, &manager.cleanup_failed)?;
        let invocation = claude_agent::prepare_invocation(
            runtime,
            &input.settings,
            &input.system_instruction,
            &input.user_prompt,
            &input.output_schema,
            &stage.system_path(),
        )
        .map_err(|message| failure(Kind::PreparationFailed, message))?;
        stage.write_system_instruction(&invocation.system_instruction)?;
        let account_home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| {
                failure(
                    Kind::PreparationFailed,
                    "The native account home is unavailable. No Claude process was started.",
                )
            })?;
        let mut command = claude_agent::prepare_command(
            runtime,
            &invocation,
            &stage.path,
            &stage.temporary_path(),
            &account_home,
        )
        .map_err(|message| failure(Kind::PreparationFailed, message))?;
        check_deadline(manager, cancelled, started)?;
        let mut child = OwnedChild::spawn(manager, &mut command, cancelled)?;
        let mut stream = ClaudeOutputStream::new(&input.settings)
            .map_err(|message| failure(Kind::PreparationFailed, message))?;
        let mut framing = Framing::default();
        let mut written = 0;
        let mut stderr_bytes = 0_usize;
        let mut stdout_eof = false;
        let mut stderr_eof = false;
        let mut exit_status: Option<ExitStatus> = None;
        let mut drain_started = None;
        let mut last_phase: Option<ClaudeProgressPhase> = None;
        let result = (|| {
            loop {
                check_deadline(manager, cancelled, started)?;
                if !stdout_eof {
                    stdout_eof = framing.drain(child.stdout()?, &mut stream, |phase| {
                        if last_phase != Some(phase) {
                            last_phase = Some(phase);
                            let (phase, message) = phase_message(phase);
                            progress(ClaudeWorkerProgress {
                                run_id: input.run_id.clone(),
                                phase,
                                message,
                            });
                        }
                    })?;
                }
                if !stderr_eof {
                    stderr_eof = drain_stderr(child.stderr()?, &mut stderr_bytes)?;
                }
                // A progress callback can trigger cancellation. Recheck before
                // delivering another source chunk after draining the pipes.
                check_deadline(manager, cancelled, started)?;
                if child.stdin.is_some() {
                    if written < invocation.stdin.len() {
                        let end = written.saturating_add(IO_CHUNK).min(invocation.stdin.len());
                        match child
                            .stdin
                            .as_mut()
                            .ok_or_else(|| {
                                failure(
                                    Kind::ProcessFailed,
                                    "The Claude input pipe became unavailable.",
                                )
                            })?
                            .write(&invocation.stdin[written..end])
                        {
                            Ok(0) => {
                                return Err(failure(
                                    Kind::ProcessFailed,
                                    "Claude closed its input before receiving the complete reviewed request.",
                                ));
                            }
                            Ok(count) => written += count,
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                                ) => {}
                            Err(_) => {
                                return Err(failure(
                                    Kind::ProcessFailed,
                                    "The complete reviewed request could not be delivered to Claude.",
                                ));
                            }
                        }
                    }
                    if written == invocation.stdin.len() {
                        child.stdin.take();
                    }
                }
                if exit_status.is_none() {
                    if let Some(status) = child.try_finish()? {
                        exit_status = Some(status);
                        drain_started = Some(Instant::now());
                    }
                }
                if stdout_eof && stderr_eof && exit_status.is_some() {
                    break;
                }
                if drain_started.is_some_and(|at: Instant| at.elapsed() >= PIPE_DRAIN_TIMEOUT) {
                    return Err(failure(
                        Kind::ProcessFailed,
                        "Claude exited but an output pipe did not close. No response was accepted.",
                    ));
                }
                thread::sleep(POLL_INTERVAL);
            }
            framing.finish(&mut stream)?;
            if written != invocation.stdin.len() {
                return Err(failure(
                    Kind::ProcessFailed,
                    "Claude exited before the complete reviewed request was delivered.",
                ));
            }
            stream
                .finish(exit_status.is_some_and(|status| status.success()))
                .map_err(|message| failure(Kind::ProtocolRejected, message))
        })();
        if result.is_err() && !child.reaped {
            progress(ClaudeWorkerProgress {
                run_id: input.run_id.clone(),
                phase: "stopping",
                message: "Stopping Claude and releasing this run's local resources.",
            });
            let graceful = result
                .as_ref()
                .err()
                .is_some_and(|error| matches!(error.kind, Kind::Cancelled | Kind::TimedOut));
            child.stop_and_reap(graceful)?;
        }
        // Drop closes pipes before private storage is deleted. No reader/writer
        // threads are spawned, so inherited pipes cannot strand a worker thread.
        drop(child);
        stage.cleanup()?;
        check_deadline(manager, cancelled, started)?;
        result
    }

    fn phase_message(phase: ClaudeProgressPhase) -> (&'static str, &'static str) {
        match phase {
            ClaudeProgressPhase::Started => (
                "started",
                "Claude reported the reviewed model and restricted tool configuration.",
            ),
            ClaudeProgressPhase::Generating => {
                ("generating", "Claude is preparing the explanation.")
            }
            ClaudeProgressPhase::Retry => (
                "retrying",
                "The Claude runtime reported a retry on the selected backend.",
            ),
            ClaudeProgressPhase::ResponseReceived => (
                "responseReceived",
                "The structured response arrived; waiting for clean process completion and local validation.",
            ),
        }
    }

    fn check_deadline(
        manager: &ManagerInner,
        cancelled: &AtomicBool,
        started: Instant,
    ) -> Result<(), ClaudeWorkerFailure> {
        check_cancelled(manager, cancelled)?;
        if started.elapsed() >= RUN_TIMEOUT {
            Err(failure(
                ClaudeWorkerFailureKind::TimedOut,
                "The Claude run exceeded ten minutes. No response was accepted; account usage may already have occurred.",
            ))
        } else {
            Ok(())
        }
    }

    fn verify_executable(
        manager: &ManagerInner,
        runtime: &ClaudeReviewRuntime,
        cancelled: &AtomicBool,
        started: Instant,
    ) -> Result<(), ClaudeWorkerFailure> {
        let (path, digest) = inspect_executable(&runtime.executable_path, || {
            check_deadline(manager, cancelled, started)
        })?;
        if path != runtime.executable_path || digest != runtime.executable_sha256 {
            return Err(failure(
                ClaudeWorkerFailureKind::RuntimeChanged,
                "The reviewed Claude executable changed. Prepare a fresh review.",
            ));
        }
        Ok(())
    }

    fn inspect_executable(
        executable: &Path,
        mut check: impl FnMut() -> Result<(), ClaudeWorkerFailure>,
    ) -> Result<(PathBuf, String), ClaudeWorkerFailure> {
        use ClaudeWorkerFailureKind as Kind;
        if !executable.is_absolute()
            || executable.to_str().is_none()
            || executable.as_os_str().len() > 4096
            || executable
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(failure(
                Kind::PreparationFailed,
                "Choose an absolute path to an installed Claude CLI executable.",
            ));
        }
        let changed = || {
            failure(
                Kind::RuntimeChanged,
                "The reviewed Claude executable changed or is unavailable. Prepare a fresh review with the qualified runtime.",
            )
        };
        let canonical = fs::canonicalize(executable).map_err(|_| changed())?;
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&canonical)
            .map_err(|_| changed())?;
        let metadata = file.metadata().map_err(|_| changed())?;
        if !metadata.is_file()
            || metadata.len() == 0
            || metadata.len() > MAX_EXECUTABLE_BYTES
            || metadata.permissions().mode() & 0o111 == 0
        {
            return Err(changed());
        }
        let mut digest = Sha256::new();
        let mut total = 0_u64;
        let mut bytes = [0_u8; 64 * 1024];
        loop {
            check()?;
            let read_limit = MAX_EXECUTABLE_BYTES
                .saturating_sub(total)
                .saturating_add(1)
                .min(bytes.len() as u64) as usize;
            let count = file.read(&mut bytes[..read_limit]).map_err(|_| changed())?;
            if count == 0 {
                break;
            }
            total = total.saturating_add(count as u64);
            if total > MAX_EXECUTABLE_BYTES {
                return Err(changed());
            }
            digest.update(&bytes[..count]);
        }
        if total != metadata.len() {
            return Err(changed());
        }
        Ok((canonical, format!("{:x}", digest.finalize())))
    }

    #[derive(Default)]
    struct Framing {
        frame: Vec<u8>,
        total: usize,
    }
    impl Framing {
        fn drain(
            &mut self,
            stdout: &mut ChildStdout,
            stream: &mut ClaudeOutputStream,
            mut progress: impl FnMut(ClaudeProgressPhase),
        ) -> Result<bool, ClaudeWorkerFailure> {
            let mut bytes = [0_u8; IO_CHUNK];
            let mut burst = 0;
            while burst < IO_BURST {
                let count = match stdout.read(&mut bytes) {
                    Ok(0) => return Ok(true),
                    Ok(count) => count,
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(false),
                    Err(error) if error.kind() == io::ErrorKind::Interrupted => return Ok(false),
                    Err(_) => {
                        return Err(failure(
                            ClaudeWorkerFailureKind::ProcessFailed,
                            "Claude output could not be read.",
                        ));
                    }
                };
                burst += count;
                self.total = self.total.saturating_add(count);
                if self.total > claude_agent::MAX_STREAM_BYTES {
                    return Err(failure(
                        ClaudeWorkerFailureKind::OutputLimit,
                        "Claude output exceeded 32 MB.",
                    ));
                }
                for byte in &bytes[..count] {
                    if *byte == b'\n' {
                        if let Some(phase) = stream.push(&self.frame).map_err(|message| {
                            failure(ClaudeWorkerFailureKind::ProtocolRejected, message)
                        })? {
                            progress(phase);
                        }
                        self.frame.clear();
                    } else {
                        if self.frame.len() >= claude_agent::MAX_FRAME_BYTES {
                            return Err(failure(
                                ClaudeWorkerFailureKind::OutputLimit,
                                "A Claude output frame exceeded 2 MB.",
                            ));
                        }
                        self.frame.push(*byte);
                    }
                }
            }
            Ok(false)
        }
        fn finish(self, stream: &mut ClaudeOutputStream) -> Result<(), ClaudeWorkerFailure> {
            if !self.frame.is_empty() {
                stream.push(&self.frame).map_err(|message| {
                    failure(ClaudeWorkerFailureKind::ProtocolRejected, message)
                })?;
            }
            Ok(())
        }
    }

    fn drain_stderr(
        stderr: &mut ChildStderr,
        total: &mut usize,
    ) -> Result<bool, ClaudeWorkerFailure> {
        let mut buffer = [0_u8; IO_CHUNK];
        let mut burst = 0;
        while burst < IO_BURST {
            match stderr.read(&mut buffer) {
                Ok(0) => return Ok(true),
                Ok(count) => {
                    burst += count;
                    *total = total.saturating_add(count);
                    if *total > MAX_STDERR_BYTES {
                        return Err(failure(
                            ClaudeWorkerFailureKind::OutputLimit,
                            "Claude diagnostic output exceeded its local limit.",
                        ));
                    }
                }
                Err(error)
                    if matches!(
                        error.kind(),
                        io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                    ) =>
                {
                    return Ok(false);
                }
                Err(_) => {
                    return Err(failure(
                        ClaudeWorkerFailureKind::ProcessFailed,
                        "Claude diagnostics could not be drained.",
                    ));
                }
            }
        }
        Ok(false)
    }

    fn nonblocking(pipe: &impl AsRawFd) -> Result<(), ClaudeWorkerFailure> {
        let fd = pipe.as_raw_fd();
        // SAFETY: fd is borrowed from a live owned pipe for both calls; fcntl
        // takes no pointer here. F_GETFL has no variadic arg; F_SETFL takes an
        // integer flag word. Preserve existing flags while adding O_NONBLOCK.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 {
            return Err(failure(
                ClaudeWorkerFailureKind::PreparationFailed,
                "Claude pipe flags could not be inspected. The process will be stopped.",
            ));
        }
        // SAFETY: fd is still borrowed from the same live pipe. F_GETFL
        // succeeded, and F_SETFL receives only its flags plus O_NONBLOCK.
        if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return Err(failure(
                ClaudeWorkerFailureKind::PreparationFailed,
                "Claude pipes could not be made nonblocking. The process will be stopped.",
            ));
        }
        Ok(())
    }

    struct OwnedChild<'a> {
        child: Child,
        registry: &'a Mutex<Option<u32>>,
        cleanup_failed: &'a AtomicBool,
        stdin: Option<ChildStdin>,
        stdout: Option<ChildStdout>,
        stderr: Option<ChildStderr>,
        reaped: bool,
    }
    impl<'a> OwnedChild<'a> {
        fn spawn(
            manager: &'a ManagerInner,
            command: &mut Command,
            cancelled: &AtomicBool,
        ) -> Result<Self, ClaudeWorkerFailure> {
            let mut published = manager.child_pid.lock().map_err(|_| {
                failure(
                    ClaudeWorkerFailureKind::PreparationFailed,
                    "Claude process ownership could not be established.",
                )
            })?;
            check_cancelled(manager, cancelled)?;
            command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let child = command.spawn().map_err(|_| failure(ClaudeWorkerFailureKind::ProcessFailed, "The selected Claude executable could not start. No alternate backend was used."))?;
            *published = Some(child.id());
            let mut owned = Self {
                child,
                registry: &manager.child_pid,
                cleanup_failed: &manager.cleanup_failed,
                stdin: None,
                stdout: None,
                stderr: None,
                reaped: false,
            };
            // Publish ownership before a concurrent shutdown can signal. Drop
            // the registry lock before any fallible setup, so RAII can reacquire.
            drop(published);
            owned.stdin = owned.child.stdin.take();
            owned.stdout = owned.child.stdout.take();
            owned.stderr = owned.child.stderr.take();
            if let (Some(stdin), Some(stdout), Some(stderr)) =
                (&owned.stdin, &owned.stdout, &owned.stderr)
            {
                nonblocking(stdin)?;
                nonblocking(stdout)?;
                nonblocking(stderr)?;
            } else {
                return Err(failure(
                    ClaudeWorkerFailureKind::PreparationFailed,
                    "Claude did not provide all required process pipes.",
                ));
            }
            Ok(owned)
        }
        fn stdout(&mut self) -> Result<&mut ChildStdout, ClaudeWorkerFailure> {
            self.stdout.as_mut().ok_or_else(|| {
                failure(
                    ClaudeWorkerFailureKind::ProcessFailed,
                    "The Claude output pipe is unavailable.",
                )
            })
        }
        fn stderr(&mut self) -> Result<&mut ChildStderr, ClaudeWorkerFailure> {
            self.stderr.as_mut().ok_or_else(|| {
                failure(
                    ClaudeWorkerFailureKind::ProcessFailed,
                    "The Claude diagnostic pipe is unavailable.",
                )
            })
        }
        fn signal(&self, signal: i32) -> io::Result<()> {
            let published = self
                .registry
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if *published == Some(self.child.id()) && !self.reaped {
                signal_group(self.child.id(), signal)
            } else {
                Ok(())
            }
        }
        fn try_finish(&mut self) -> Result<Option<ExitStatus>, ClaudeWorkerFailure> {
            if self.reaped {
                return Ok(None);
            }
            let mut published = self
                .registry
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            // SAFETY: siginfo_t is a C output record with integer/pointer
            // storage, valid zero-initialized. waitid writes through a uniquely
            // borrowed, correctly aligned live record. P_PID selects our owned
            // unreaped child; WNOWAIT preserves PID ownership until wait below.
            let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
            // SAFETY: the output pointer refers to the live zeroed record
            // above, and the selected positive PID belongs to this Child.
            let observed = unsafe {
                libc::waitid(
                    libc::P_PID,
                    self.child.id() as libc::id_t,
                    &mut info,
                    libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
                )
            };
            if observed < 0 {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::Interrupted {
                    return Ok(None);
                }
                if error.raw_os_error() == Some(libc::ECHILD) {
                    // Another reaper or process policy invalidated ownership.
                    // Never signal a PID after that observation: it may be reused.
                    *published = None;
                    self.reaped = true;
                    self.cleanup_failed.store(true, Ordering::Release);
                    return Err(failure(
                        ClaudeWorkerFailureKind::CleanupFailed,
                        "Claude child ownership was lost before cleanup could be confirmed. Further runs are disabled for this app session.",
                    ));
                }
                return Err(failure(
                    ClaudeWorkerFailureKind::ProcessFailed,
                    "Claude process completion could not be inspected.",
                ));
            }
            if info.si_signo == 0 {
                return Ok(None);
            }
            if info.si_signo != libc::SIGCHLD {
                return Err(failure(
                    ClaudeWorkerFailureKind::ProcessFailed,
                    "Claude returned an unexpected child status record.",
                ));
            }
            // Stop remaining members while the leader is still unreaped; its
            // process-group identifier cannot have been reused in this window.
            let signalled = signal_group(self.child.id(), libc::SIGKILL);
            let waited = self.child.wait();
            self.reaped = true;
            *published = None;
            if signalled.is_err() || waited.is_err() {
                self.cleanup_failed.store(true, Ordering::Release);
                return Err(failure(
                    ClaudeWorkerFailureKind::CleanupFailed,
                    "Claude exited but process cleanup could not be confirmed. Further runs are disabled for this app session.",
                ));
            }
            waited.map(Some).map_err(|_| {
                failure(
                    ClaudeWorkerFailureKind::CleanupFailed,
                    "Claude exit status is unavailable.",
                )
            })
        }
        fn stop_and_reap(&mut self, graceful: bool) -> Result<(), ClaudeWorkerFailure> {
            self.stdin.take();
            if graceful {
                self.signal(libc::SIGINT).map_err(|_| {
                    failure(
                        ClaudeWorkerFailureKind::CleanupFailed,
                        "Claude could not be interrupted.",
                    )
                })?;
                let interrupt = Instant::now();
                while interrupt.elapsed() < INTERRUPT_GRACE {
                    self.discard_available();
                    if self.try_finish()?.is_some() {
                        return Ok(());
                    }
                    thread::sleep(POLL_INTERVAL);
                }
                self.signal(libc::SIGTERM).map_err(|_| {
                    failure(
                        ClaudeWorkerFailureKind::CleanupFailed,
                        "Claude could not be terminated.",
                    )
                })?;
                let terminate = Instant::now();
                while terminate.elapsed() < TERMINATE_GRACE {
                    self.discard_available();
                    if self.try_finish()?.is_some() {
                        return Ok(());
                    }
                    thread::sleep(POLL_INTERVAL);
                }
            }
            self.kill_and_reap()
        }
        fn discard_available(&mut self) {
            let mut bytes = [0_u8; IO_CHUNK];
            discard_pipe(&mut self.stdout, &mut bytes);
            discard_pipe(&mut self.stderr, &mut bytes);
        }
        fn kill_and_reap(&mut self) -> Result<(), ClaudeWorkerFailure> {
            if self.reaped {
                return Ok(());
            }
            let mut published = self
                .registry
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let group = signal_group(self.child.id(), libc::SIGKILL);
            let _ = self.child.kill();
            let waited = self.child.wait();
            // Never retain a possibly reaped PID for a concurrent shutdown.
            *published = None;
            self.reaped = true;
            if group.is_err() || waited.is_err() {
                self.cleanup_failed.store(true, Ordering::Release);
                return Err(failure(
                    ClaudeWorkerFailureKind::CleanupFailed,
                    "Claude process cleanup could not be confirmed. Check the native process state before retrying.",
                ));
            }
            Ok(())
        }
    }
    impl Drop for OwnedChild<'_> {
        fn drop(&mut self) {
            if !self.reaped {
                let _ = self.kill_and_reap();
            }
        }
    }

    fn discard_pipe<T: Read>(pipe: &mut Option<T>, bytes: &mut [u8]) {
        if let Some(pipe) = pipe {
            for _ in 0..8 {
                if !matches!(pipe.read(bytes), Ok(count) if count > 0) {
                    break;
                }
            }
        }
    }

    pub(super) fn signal_group(pid: u32, signal: i32) -> io::Result<()> {
        if pid == 0 || pid > i32::MAX as u32 {
            return Err(io::Error::other("invalid owned process identity"));
        }
        // SAFETY: the caller holds the registry lock for a still-owned,
        // unreaped child. prepare_command sets process_group(0), so -pid names
        // only that child's process group, never the app's group or PID zero.
        if unsafe { libc::kill(-(pid as libc::pid_t), signal) } < 0 {
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ESRCH) {
                return Err(error);
            }
        }
        Ok(())
    }

    struct PrivateStage<'a> {
        path: PathBuf,
        removed: bool,
        cleanup_failed: &'a AtomicBool,
    }
    impl<'a> PrivateStage<'a> {
        fn create(
            storage_dir: &Path,
            cleanup_failed: &'a AtomicBool,
        ) -> Result<Self, ClaudeWorkerFailure> {
            if !storage_dir.is_absolute() {
                return Err(failure(
                    ClaudeWorkerFailureKind::PreparationFailed,
                    "Claude requires an absolute app data directory.",
                ));
            }
            let storage = fs::canonicalize(storage_dir).map_err(|_| {
                failure(
                    ClaudeWorkerFailureKind::PreparationFailed,
                    "The Claude app data directory is unavailable.",
                )
            })?;
            let root = storage.join("claude-agent");
            private_directory(&root)?;
            let root = root.join("runs");
            private_directory(&root)?;
            let path = root.join(uuid::Uuid::new_v4().to_string());
            fs::create_dir(&path).map_err(|_| {
                failure(
                    ClaudeWorkerFailureKind::PreparationFailed,
                    "The private Claude run directory could not be created.",
                )
            })?;
            let stage = Self {
                path,
                removed: false,
                cleanup_failed,
            };
            fs::set_permissions(&stage.path, fs::Permissions::from_mode(0o700)).map_err(|_| {
                failure(
                    ClaudeWorkerFailureKind::PreparationFailed,
                    "Claude run permissions could not be restricted.",
                )
            })?;
            private_directory(&stage.temporary_path())?;
            Ok(stage)
        }
        fn system_path(&self) -> PathBuf {
            self.path.join("instructions.txt")
        }
        fn temporary_path(&self) -> PathBuf {
            self.path.join("tmp")
        }
        fn write_system_instruction(&self, bytes: &[u8]) -> Result<(), ClaudeWorkerFailure> {
            let mut file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(self.system_path())
                .map_err(|_| {
                    failure(
                        ClaudeWorkerFailureKind::PreparationFailed,
                        "The private Claude instruction file could not be created.",
                    )
                })?;
            file.write_all(bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| {
                    failure(
                        ClaudeWorkerFailureKind::PreparationFailed,
                        "The private Claude instructions could not be saved.",
                    )
                })?;
            file.set_permissions(fs::Permissions::from_mode(0o400))
                .and_then(|_| fs::set_permissions(&self.path, fs::Permissions::from_mode(0o500)))
                .map_err(|_| {
                    failure(
                        ClaudeWorkerFailureKind::PreparationFailed,
                        "The staged Claude instructions could not be made read-only.",
                    )
                })
        }
        fn write_discovery_files(
            &self,
            sdk: &[u8],
            mut check: impl FnMut() -> Result<(), ClaudeWorkerFailure>,
        ) -> Result<(), ClaudeWorkerFailure> {
            use crate::claude_discovery::{DISCOVERY_LIBRARY, DISCOVERY_SCRIPT};
            use ClaudeWorkerFailureKind as Kind;
            for (name, bytes) in [
                ("worker.mjs", DISCOVERY_SCRIPT.as_bytes()),
                ("discovery.mjs", DISCOVERY_LIBRARY.as_bytes()),
                ("sdk.mjs", sdk),
            ] {
                check()?;
                let mut file = OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .mode(0o600)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(self.path.join(name))
                    .map_err(|_| {
                        failure(
                            Kind::PreparationFailed,
                            "The private SDK discovery files could not be created.",
                        )
                    })?;
                for chunk in bytes.chunks(IO_CHUNK) {
                    check()?;
                    file.write_all(chunk).map_err(|_| {
                        failure(
                            Kind::PreparationFailed,
                            "The private SDK discovery files could not be written.",
                        )
                    })?;
                }
                file.sync_all()
                    .and_then(|_| file.set_permissions(fs::Permissions::from_mode(0o400)))
                    .map_err(|_| {
                        failure(
                            Kind::PreparationFailed,
                            "The private SDK discovery files could not be made read-only.",
                        )
                    })?;
            }
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o500)).map_err(|_| {
                failure(
                    Kind::PreparationFailed,
                    "The private SDK discovery directory could not be made read-only.",
                )
            })
        }
        fn cleanup(mut self) -> Result<(), ClaudeWorkerFailure> {
            self.remove()?;
            self.removed = true;
            Ok(())
        }
        fn remove(&self) -> Result<(), ClaudeWorkerFailure> {
            fs::set_permissions(&self.path, fs::Permissions::from_mode(0o700)).and_then(|_| fs::remove_dir_all(&self.path))
                .map_err(|_| failure(ClaudeWorkerFailureKind::CleanupFailed, "Private Claude run storage could not be removed. No response was accepted."))
        }
    }
    impl Drop for PrivateStage<'_> {
        fn drop(&mut self) {
            // Preparation errors and unwinding must disable later runs too
            // when storage cleanup cannot be confirmed. Explicit cleanup
            // remains required before any successful response is returned.
            if !self.removed && self.remove().is_err() {
                self.cleanup_failed.store(true, Ordering::Release);
            }
        }
    }
    fn private_directory(path: &Path) -> Result<(), ClaudeWorkerFailure> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                fs::create_dir(path).map_err(|_| {
                    failure(
                        ClaudeWorkerFailureKind::PreparationFailed,
                        "Private Claude storage could not be created.",
                    )
                })?;
            }
            _ => {
                return Err(failure(
                    ClaudeWorkerFailureKind::PreparationFailed,
                    "The Claude data path is not a regular local directory.",
                ));
            }
        }
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|_| {
            failure(
                ClaudeWorkerFailureKind::PreparationFailed,
                "Private Claude storage permissions could not be restricted.",
            )
        })
    }
}
