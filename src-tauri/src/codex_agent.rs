//! Account-backed Codex integration over the official app-server stdio protocol.
//!
//! All entry points are blocking and must run on Tauri's blocking pool. One
//! process is shared by this application; calls never use a shell or read a
//! user's existing Codex credentials. The runtime owns its browser login,
//! Keychain credential storage and refresh in an application-specific home.
//! This initial adapter exposes bounded explanation generation with command and web
//! tools disabled, restricted read access, and unexpected tool events rejected.

use crate::codex_identity::{self, CodexReviewRuntime};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};
use uuid::Uuid;

const MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;
const MAX_OPERATION_BYTES: usize = 32 * 1024 * 1024;
const MAX_RESPONSE_BYTES: usize = 300 * 1024;
const MAX_CONTEXT_BYTES: usize = 256 * 1024;
const MAX_SOURCE_FILES: usize = 24;
const RPC_TIMEOUT: Duration = Duration::from_secs(30);
const RUN_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const CANCEL_GRACE: Duration = Duration::from_secs(3);
const CANCEL_TTL: Duration = Duration::from_secs(15 * 60);
const MAX_CANCELLED_RUNS: usize = 256;
const BUSY: &str =
    "Codex is busy with another operation. Wait for it to finish or cancel the active generation.";
const UPGRADE: &str = "Code Atlas could not verify the required explanation protocol in this Codex executable. Sign-in and generation have not started. Check compatibility with this adapter before retrying.";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexAgentStatus {
    pub executable_path: Option<String>,
    pub runtime_running: bool,
    pub authenticated: bool,
    pub account_email: Option<String>,
    pub plan_type: Option<String>,
    pub pending_login_id: Option<String>,
    pub active_run_id: Option<String>,
    pub capability_checked: bool,
    pub message: String,
}

impl Default for CodexAgentStatus {
    fn default() -> Self {
        Self {
            executable_path: None,
            runtime_running: false,
            authenticated: false,
            account_email: None,
            plan_type: None,
            pending_login_id: None,
            active_run_id: None,
            capability_checked: false,
            message:
                "Select a Codex executable and connect your ChatGPT account. No API key is needed."
                    .to_owned(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexLoginStart {
    pub login_id: String,
    pub auth_url: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexModel {
    pub model_id: String,
    pub display_name: String,
    pub default_reasoning_effort: Option<String>,
    pub supported_reasoning_efforts: Vec<String>,
    pub is_default: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CodexSourceFile {
    pub path: String,
    pub sha256: String,
    pub contents: String,
}

/// Construct this only from a freshly revalidated native request review.
pub(crate) struct CodexRunInput {
    pub run_id: String,
    pub expected_runtime: CodexReviewRuntime,
    pub model: String,
    pub effort: String,
    pub approval_policy: String,
    pub system_instruction: String,
    pub user_prompt: String,
    pub output_schema: Value,
    pub sources: Vec<CodexSourceFile>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexProgress {
    pub run_id: String,
    pub phase: String,
    pub thread_id: Option<String>,
    pub turn_id: Option<String>,
    /// Fixed application text only: never source, raw provider output or tokens.
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexRunResult {
    pub thread_id: String,
    pub turn_id: String,
    pub model: String,
    pub effort: String,
    pub response_text: String,
}

/// Available only inside the manager-owned result callback. It cannot start
/// another run or renew review admission; cancellation/deadline still apply.
pub(crate) struct CodexResultAccess<'a> {
    run_id: &'a str,
    fingerprint: &'a str,
    runtime: &'a CodexReviewRuntime,
    shutdown: &'a AtomicBool,
    cancelled: &'a AtomicBool,
    started_at: Instant,
}

impl CodexResultAccess<'_> {
    pub(crate) fn runtime(&self) -> &CodexReviewRuntime {
        self.runtime
    }

    pub(crate) fn check_review(
        &self,
        review_id: &str,
        fingerprint: &str,
        executable_path: &str,
    ) -> Result<(), String> {
        if review_id != self.run_id
            || fingerprint != self.fingerprint
            || executable_path != self.runtime.executable_path
        {
            return Err("The Codex result belongs to a different reviewed operation. Prepare a fresh review.".to_owned());
        }
        check_cancelled(self.shutdown, Some(self.cancelled))?;
        if self.started_at.elapsed() >= RUN_TIMEOUT {
            return Err("The Codex operation exceeded ten minutes, including result validation. No late response was published.".to_owned());
        }
        Ok(())
    }
}

#[derive(Clone, Default)]
pub(crate) struct CodexAgentManager(Arc<ManagerInner>);

#[derive(Default)]
struct ManagerInner {
    process: Mutex<Option<ServerProcess>>,
    status: Mutex<CodexAgentStatus>,
    active: Mutex<Option<ActiveRun>>,
    process_id: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
    cancelled_runs: Mutex<HashMap<String, Instant>>,
}

struct ActiveRun {
    id: String,
    cancelled: Arc<AtomicBool>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RuntimeSettings {
    executable_path: String,
}

impl CodexAgentManager {
    /// Inspect local setup and poll an already-running server; never starts one.
    pub(crate) fn status(&self, storage_dir: &Path) -> Result<CodexAgentStatus, String> {
        if let Ok(mut slot) = self.0.process.try_lock() {
            if let Some(process) = slot.as_mut() {
                if process.has_exited()? {
                    slot.take();
                    self.update_status(|status| {
                        status.runtime_running = false;
                        status.authenticated = false;
                        status.pending_login_id = None;
                        status.message = "Codex stopped. Reconnect to start it again.".to_owned();
                    });
                } else {
                    let account = process.request(
                        "account/read",
                        json!({"refreshToken": false}),
                        RPC_TIMEOUT,
                    )?;
                    self.remember_account(&account);
                }
            } else if let Some(path) = configured_executable(storage_dir)? {
                self.update_status(|status| {
                    status.executable_path = Some(path.to_string_lossy().into_owned())
                });
            }
        }
        let mut status = self
            .0
            .status
            .lock()
            .map_err(|_| "Codex status is unavailable.")?
            .clone();
        status.active_run_id = self
            .0
            .active
            .lock()
            .map_err(|_| BUSY)?
            .as_ref()
            .map(|run| run.id.clone());
        Ok(status)
    }

    /// Observe an already-connected runtime. Preparing a review never starts a
    /// process, renews login or sends source/model input.
    pub(crate) fn review_runtime(&self, storage_dir: &Path) -> Result<CodexReviewRuntime, String> {
        let mut slot = self.lock_process()?;
        let process = slot.as_mut().ok_or_else(codex_identity::changed)?;
        self.observe_runtime(storage_dir, process)
    }

    fn observe_runtime(
        &self,
        storage_dir: &Path,
        process: &mut ServerProcess,
    ) -> Result<CodexReviewRuntime, String> {
        let executable = configured_executable(storage_dir)?.ok_or_else(codex_identity::changed)?;
        if process.has_exited()?
            || process.executable != executable
            || executable_digest(&executable, &self.0.shutdown, process.cancelled.as_deref())?
                != process.executable_sha256
        {
            return Err(codex_identity::changed());
        }
        let account = process.request("account/read", json!({"refreshToken": false}), RPC_TIMEOUT)?;
        self.remember_account(&account);
        CodexReviewRuntime::from_observation(
            &process.connection_id,
            &process.executable,
            &process.executable_sha256,
            process.account_revision,
            &account,
        )
    }

    /// Save a user-selected executable without starting it or signing in.
    pub(crate) fn configure(
        &self,
        storage_dir: &Path,
        executable_path: &str,
    ) -> Result<CodexAgentStatus, String> {
        let path = checked_executable(Path::new(executable_path))?;
        let mut slot = self.lock_process()?;
        let root = runtime_root(storage_dir)?;
        let bytes = serde_json::to_vec(&RuntimeSettings {
            executable_path: path.to_string_lossy().into_owned(),
        })
        .map_err(|_| "Codex setup could not be encoded.")?;
        atomic_private_write(&root.join("runtime.json"), &bytes)?;
        slot.take();
        self.update_status(|status| {
            *status = CodexAgentStatus::default();
            status.executable_path = Some(path.to_string_lossy().into_owned());
            status.message = "Codex executable saved. Connect your ChatGPT account to check runtime compatibility and sign in.".to_owned();
        });
        drop(slot);
        self.status(storage_dir)
    }

    /// The runtime performs browser OAuth and stores credentials in Keychain.
    pub(crate) fn start_login(&self, storage_dir: &Path) -> Result<CodexLoginStart, String> {
        let mut slot = self.lock_process()?;
        let process = self.ensure_started(storage_dir, &mut slot, None)?;
        let account =
            process.request("account/read", json!({"refreshToken": false}), RPC_TIMEOUT)?;
        self.remember_account(&account);
        if let Some(login_id) = self
            .0
            .status
            .lock()
            .map_err(|_| "Codex status is unavailable.")?
            .pending_login_id
            .clone()
        {
            let _ = process.request(
                "account/login/cancel",
                json!({"loginId": login_id}),
                RPC_TIMEOUT,
            );
        }
        let result = process.request(
            "account/login/start",
            json!({"type": "chatgpt", "useHostedLoginSuccessPage": true}),
            RPC_TIMEOUT,
        )?;
        let login_id = bounded_string(&result, "loginId", 256)?;
        let auth_url = bounded_string(&result, "authUrl", 8192)?;
        let url =
            reqwest::Url::parse(&auth_url).map_err(|_| "Codex returned an invalid sign-in URL.")?;
        if url.scheme() != "https"
            || !matches!(url.host_str(), Some("auth.openai.com" | "chatgpt.com"))
            || !url.username().is_empty()
            || url.password().is_some()
        {
            let _ = process.request(
                "account/login/cancel",
                json!({"loginId": login_id}),
                RPC_TIMEOUT,
            );
            return Err(
                "Codex returned an unexpected sign-in destination. No browser was opened."
                    .to_owned(),
            );
        }
        self.update_status(|status| {
            status.pending_login_id = Some(login_id.clone());
            status.message =
                "Complete ChatGPT sign-in in your browser, then refresh the account status."
                    .to_owned();
        });
        Ok(CodexLoginStart { login_id, auth_url })
    }

    pub(crate) fn cancel_login(&self, _storage_dir: &Path, login_id: &str) -> Result<(), String> {
        let mut slot = self.lock_process()?;
        if let Some(process) = slot.as_mut() {
            process.request(
                "account/login/cancel",
                json!({"loginId": login_id}),
                RPC_TIMEOUT,
            )?;
        }
        self.update_status(|status| {
            status.pending_login_id = None;
            status.message = "Codex sign-in was cancelled.".to_owned();
        });
        Ok(())
    }

    pub(crate) fn logout(&self, storage_dir: &Path) -> Result<(), String> {
        let mut slot = self.lock_process()?;
        let process = self.ensure_started(storage_dir, &mut slot, None)?;
        process.request("account/logout", json!({}), RPC_TIMEOUT)?;
        slot.take();
        self.update_status(|status| {
            status.runtime_running = false;
            status.authenticated = false;
            status.account_email = None;
            status.plan_type = None;
            status.pending_login_id = None;
            status.message = "Disconnected from this app's Codex account. Other Codex installations were not changed.".to_owned();
        });
        Ok(())
    }

    pub(crate) fn models(&self, storage_dir: &Path) -> Result<Vec<CodexModel>, String> {
        let mut slot = self.lock_process()?;
        let process = self.ensure_started(storage_dir, &mut slot, None)?;
        self.require_account(process)?;
        process.models()
    }

    pub(crate) fn run<F>(
        &self,
        storage_dir: &Path,
        input: CodexRunInput,
        progress: F,
    ) -> Result<CodexRunResult, String>
    where
        F: Fn(CodexProgress),
    {
        self.run_with_result(
            storage_dir,
            input,
            "",
            Instant::now(),
            progress,
            |response, _| response,
        )
    }

    /// Retain native operation/process ownership through pure result
    /// validation. The callback must not persist state or publish its result: a
    /// cancellation or timeout can still reject its in-memory return value.
    /// The native sender supplies the admission clock before blocking dispatch.
    pub(crate) fn run_with_result<T, F>(
        &self,
        storage_dir: &Path,
        input: CodexRunInput,
        fingerprint: &str,
        started_at: Instant,
        progress: F,
        validate: impl FnOnce(CodexRunResult, &CodexResultAccess<'_>) -> T,
    ) -> Result<T, String>
    where
        F: Fn(CodexProgress),
    {
        self.run_with_result_and_publish(
            storage_dir,
            input,
            fingerprint,
            started_at,
            progress,
            validate,
            |value| value,
        )
    }

    /// Validate without side effects, then publish under the same native lock
    /// used by cancel(). Once publication starts, a later cancel cannot retract
    /// its durable unaccepted result. The publisher must not call this manager.
    pub(crate) fn run_with_result_and_publish<T, U, F>(
        &self,
        storage_dir: &Path,
        input: CodexRunInput,
        fingerprint: &str,
        started_at: Instant,
        progress: F,
        validate: impl FnOnce(CodexRunResult, &CodexResultAccess<'_>) -> T,
        publish: impl FnOnce(T) -> U,
    ) -> Result<U, String>
    where
        F: Fn(CodexProgress),
    {
        validate_run(&input)?;
        let mut slot = self.lock_process()?;
        let cancelled = Arc::new(AtomicBool::new(false));
        {
            // The same lock order in cancel() covers cancellation arriving
            // before this blocking task starts or while it installs its run.
            let mut active = self.0.active.lock().map_err(|_| BUSY)?;
            let mut tombstones = self.0.cancelled_runs.lock().map_err(|_| BUSY)?;
            tombstones.retain(|_, at| at.elapsed() < CANCEL_TTL);
            if tombstones.remove(&input.run_id).is_some() {
                return Err(cancelled_message());
            }
            if active.is_some() {
                return Err(BUSY.to_owned());
            }
            *active = Some(ActiveRun {
                id: input.run_id.clone(),
                cancelled: cancelled.clone(),
            });
        }
        let _active_guard = ActiveGuard {
            inner: self.0.clone(),
            run_id: input.run_id.clone(),
        };
        progress(progress_event(
            &input.run_id,
            "preparing",
            None,
            None,
            "Preparing Codex and the reviewed source snapshot.",
        ));
        let result = (|| {
            check_cancelled(&self.0.shutdown, Some(&cancelled))?;
            if started_at.elapsed() >= RUN_TIMEOUT {
                return Err(
                    "The Codex operation exceeded ten minutes before dispatch. Prepare a fresh review."
                        .to_owned(),
                );
            }
            // A reviewed connection may not be silently replaced/restarted.
            let process = slot.as_mut().ok_or_else(codex_identity::changed)?;
            process.cancelled = Some(cancelled.clone());
            if self.observe_runtime(storage_dir, process)? != input.expected_runtime {
                return Err(codex_identity::changed());
            }
            let models = process.models()?;
            let model = models.iter().find(|model| model.model_id == input.model)
                .ok_or_else(|| "The selected model is not in this Codex account's model catalog. Refresh models and review the request again.".to_owned())?;
            if !model
                .supported_reasoning_efforts
                .iter()
                .any(|effort| effort == &input.effort)
            {
                return Err("The selected model does not advertise that reasoning effort. Choose a supported effort and review again.".to_owned());
            }
            // Model discovery can observe account-change events as well.
            if self.observe_runtime(storage_dir, process)? != input.expected_runtime {
                return Err(codex_identity::changed());
            }
            let stage = StagedContext::create(storage_dir, &input.sources)?;
            if cancelled.load(Ordering::Acquire) {
                return Err(cancelled_message());
            }
            let response = process.generate(&input, &stage.path, &cancelled, started_at, &progress)?;
            if self.observe_runtime(storage_dir, process)? != input.expected_runtime {
                return Err(codex_identity::changed());
            }
            Ok(response)
        })();
        // Clear per-turn transport cancellation before pure local validation.
        if let Some(process) = slot.as_mut() {
            process.cancelled = None;
        }
        let access = CodexResultAccess {
            run_id: &input.run_id,
            fingerprint,
            runtime: &input.expected_runtime,
            shutdown: &self.0.shutdown,
            cancelled: &cancelled,
            started_at,
        };
        let finalized = (|| {
            access.check_review(
                &input.run_id,
                fingerprint,
                &input.expected_runtime.executable_path,
            )?;
            let response = result?;
            let validated = validate(response, &access);
            // Serialize final publication with cancel(). Work after this point is
            // already published; a later cancel cannot retract a returned result.
            let mut active = self.0.active.lock().map_err(|_| BUSY)?;
            access.check_review(
                &input.run_id,
                fingerprint,
                &input.expected_runtime.executable_path,
            )?;
            if !active
                .as_ref()
                .is_some_and(|run| run.id == input.run_id && Arc::ptr_eq(&run.cancelled, &cancelled))
            {
                return Err(
                    "The native Codex operation owner changed. No late response was published."
                        .to_owned(),
                );
            }
            let published = publish(validated);
            active.take();
            Ok(published)
        })();
        // Runtime errors and cancellation/deadline failures during validation
        // leave no reusable process or stale event queue for the next request.
        if finalized.is_err() {
            slot.take();
            self.update_status(|status| {
                status.runtime_running = false;
                status.authenticated = false;
                status.account_email = None;
                status.plan_type = None;
                status.capability_checked = false;
                status.pending_login_id = None;
                status.message =
                    "The Codex run stopped. Reconnect and prepare a fresh review. Existing chapter history was preserved.".to_owned();
            });
        }
        finalized
    }

    pub(crate) fn cancel(&self, run_id: &str) -> bool {
        if run_id.is_empty() || run_id.len() > 256 {
            return false;
        }
        let Ok(active) = self.0.active.lock() else {
            return false;
        };
        if let Some(run) = active.as_ref().filter(|run| run.id == run_id) {
            run.cancelled.store(true, Ordering::Release);
            return true;
        }
        let Ok(mut tombstones) = self.0.cancelled_runs.lock() else {
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

    /// Call during application exit. Reaping follows on the blocking owner.
    pub(crate) fn shutdown(&self) {
        self.0.shutdown.store(true, Ordering::Release);
        if let Ok(active) = self.0.active.lock() {
            if let Some(run) = active.as_ref() {
                run.cancelled.store(true, Ordering::Release);
            }
        }
        terminate_group(self.0.process_id.load(Ordering::Acquire));
        if let Ok(mut slot) = self.0.process.try_lock() {
            slot.take();
        }
    }

    fn lock_process(&self) -> Result<MutexGuard<'_, Option<ServerProcess>>, String> {
        self.0.process.try_lock().map_err(|_| BUSY.to_owned())
    }

    fn ensure_started<'a>(
        &self,
        storage_dir: &Path,
        slot: &'a mut Option<ServerProcess>,
        cancelled: Option<Arc<AtomicBool>>,
    ) -> Result<&'a mut ServerProcess, String> {
        check_cancelled(&self.0.shutdown, cancelled.as_deref())?;
        let executable = configured_executable(storage_dir)?.ok_or_else(|| "Install Codex and select its executable in Settings, then connect your ChatGPT account.".to_owned())?;
        if let Some(process) = slot.as_mut() {
            if process.has_exited()?
                || process.executable != executable
                || executable_digest(&executable, &self.0.shutdown, cancelled.as_deref())?
                    != process.executable_sha256
            {
                slot.take();
            }
        }
        if slot.is_none() {
            let root = runtime_root(storage_dir)?;
            verify_runtime_protocol(
                &executable,
                &root,
                &self.0.process_id,
                &self.0.shutdown,
                cancelled.as_deref(),
            )?;
            let process = ServerProcess::start(
                &executable,
                &root,
                self.0.process_id.clone(),
                self.0.shutdown.clone(),
                cancelled.clone(),
            )?;
            *slot = Some(process);
            self.update_status(|status| {
                status.executable_path = Some(executable.to_string_lossy().into_owned());
                status.runtime_running = true;
                status.capability_checked = true;
                status.message =
                    "Codex is running with read-only staged context and command/web tools disabled."
                        .to_owned();
            });
        }
        let process = slot
            .as_mut()
            .ok_or_else(|| "Codex could not be started.".to_owned())?;
        process.cancelled = cancelled;
        process.check_cancelled()?;
        Ok(process)
    }

    fn require_account(&self, process: &mut ServerProcess) -> Result<(), String> {
        let account =
            process.request("account/read", json!({"refreshToken": true}), RPC_TIMEOUT)?;
        self.remember_account(&account);
        if account.pointer("/account/type").and_then(Value::as_str) != Some("chatgpt") {
            return Err("Connect a ChatGPT account in Codex settings before generating. This route does not use an API key.".to_owned());
        }
        Ok(())
    }

    fn remember_account(&self, value: &Value) {
        let account = value.get("account").unwrap_or(&Value::Null);
        self.update_status(|status| {
            status.authenticated = account.get("type").and_then(Value::as_str) == Some("chatgpt");
            status.account_email = account
                .get("email")
                .and_then(Value::as_str)
                .filter(|value| value.len() <= 320)
                .map(str::to_owned);
            status.plan_type = account
                .get("planType")
                .and_then(Value::as_str)
                .filter(|value| value.len() <= 80)
                .map(str::to_owned);
            if status.authenticated {
                status.pending_login_id = None;
                status.message =
                    "Connected through Codex-managed ChatGPT sign-in. Account plan limits apply."
                        .to_owned();
            }
        });
    }

    fn update_status(&self, update: impl FnOnce(&mut CodexAgentStatus)) {
        if let Ok(mut status) = self.0.status.lock() {
            update(&mut status);
        }
    }
}

struct ActiveGuard {
    inner: Arc<ManagerInner>,
    run_id: String,
}
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = self.inner.active.lock() {
            if active.as_ref().is_some_and(|run| run.id == self.run_id) {
                active.take();
            }
        }
    }
}

struct ServerProcess {
    child: Child,
    writer_sender: Option<mpsc::SyncSender<WriteFrame>>,
    writer: Option<JoinHandle<()>>,
    receiver: Option<mpsc::Receiver<Result<Value, String>>>,
    reader: Option<JoinHandle<()>>,
    next_id: u64,
    process_id: Arc<AtomicU32>,
    shutdown: Arc<AtomicBool>,
    cancelled: Option<Arc<AtomicBool>>,
    executable: PathBuf,
    executable_sha256: String,
    connection_id: String,
    account_revision: u64,
    reaped: bool,
}

struct WriteFrame {
    bytes: Vec<u8>,
    acknowledged: mpsc::SyncSender<Result<(), String>>,
}

impl ServerProcess {
    fn start(
        executable: &Path,
        root: &Path,
        process_id: Arc<AtomicU32>,
        shutdown: Arc<AtomicBool>,
        cancelled: Option<Arc<AtomicBool>>,
    ) -> Result<Self, String> {
        check_cancelled(&shutdown, cancelled.as_deref())?;
        let executable_sha256 = executable_digest(executable, &shutdown, cancelled.as_deref())?;
        let mut command = isolated_command(executable, root)?;
        command.args(["app-server", "--listen", "stdio://"]);
        // Explicit overrides are application-owned and never taken from the
        // repository. Configuration warnings are fatal before any model turn.
        for setting in [
            "model_provider=\"openai\"",
            "forced_login_method=\"chatgpt\"",
            "cli_auth_credentials_store=\"keyring\"",
            "approval_policy=\"never\"",
            "sandbox_mode=\"read-only\"",
            "features.shell_tool=false",
            "features.apps=false",
            "features.hooks=false",
            "features.multi_agent=false",
            "agents.enabled=false",
            "tools.view_image=false",
            "features.goals=false",
            "features.memories=false",
            "features.remote_plugin=false",
            "features.plugins=false",
            "features.shell_snapshot=false",
            "features.skill_mcp_dependency_install=false",
            "web_search=\"disabled\"",
            "project_doc_max_bytes=0",
            "project_doc_fallback_filenames=[]",
            "allow_login_shell=false",
            "history.persistence=\"none\"",
            "analytics.enabled=false",
            "feedback.enabled=false",
            "otel.log_user_prompt=false",
            "check_for_update_on_startup=false",
            "shell_environment_policy.inherit=\"none\"",
        ] {
            command.args(["-c", setting]);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "Codex could not start. Check the executable path and installation.")?;
        let registration = RegisteredChild::new(process_id.clone(), child.id());
        if !executable_digest(executable, &shutdown, cancelled.as_deref())
            .is_ok_and(|digest| digest == executable_sha256)
        {
            kill_and_reap(&mut child);
            return Err(codex_identity::changed());
        }
        if let Err(error) = check_cancelled(&shutdown, cancelled.as_deref()) {
            kill_and_reap(&mut child);
            return Err(error);
        }
        let Some(mut stdin) = child.stdin.take() else {
            kill_and_reap(&mut child);
            return Err("Codex input is unavailable.".to_owned());
        };
        let Some(stdout) = child.stdout.take() else {
            kill_and_reap(&mut child);
            return Err("Codex output is unavailable.".to_owned());
        };
        let (sender, receiver) = mpsc::sync_channel(32);
        let reader = thread::Builder::new()
            .name("code-atlas-codex-jsonl".to_owned())
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                loop {
                    let frame = read_frame(&mut reader);
                    let done = frame.is_err();
                    if sender.send(frame).is_err() || done {
                        break;
                    }
                }
            });
        let reader = match reader {
            Ok(reader) => reader,
            Err(_) => {
                kill_and_reap(&mut child);
                return Err("Codex output handling could not start.".to_owned());
            }
        };
        let (writer_sender, writer_receiver) = mpsc::sync_channel::<WriteFrame>(1);
        let writer = thread::Builder::new()
            .name("code-atlas-codex-input".to_owned())
            .spawn(move || {
                while let Ok(frame) = writer_receiver.recv() {
                    let result = stdin
                        .write_all(&frame.bytes)
                        .and_then(|_| stdin.write_all(b"\n"))
                        .and_then(|_| stdin.flush())
                        .map_err(|_| {
                            "The Codex process disconnected. Reconnect before trying again.".to_owned()
                        });
                    let failed = result.is_err();
                    let _ = frame.acknowledged.send(result);
                    if failed {
                        break;
                    }
                }
            });
        let writer = match writer {
            Ok(writer) => writer,
            Err(_) => {
                drop(receiver);
                kill_and_reap(&mut child);
                let _ = reader.join();
                return Err("Codex input handling could not start.".to_owned());
            }
        };
        let mut process = Self {
            child,
            writer_sender: Some(writer_sender),
            writer: Some(writer),
            receiver: Some(receiver),
            reader: Some(reader),
            next_id: 1,
            process_id,
            shutdown,
            cancelled,
            executable: executable.to_owned(),
            executable_sha256,
            connection_id: Uuid::new_v4().to_string(),
            account_revision: 0,
            reaped: false,
        };
        registration.disarm();
        process.check_cancelled()?;
        process.request("initialize", json!({"clientInfo": {"name": "code_atlas", "title": "Code Atlas", "version": env!("CARGO_PKG_VERSION")}, "capabilities": {"experimentalApi": false}}), RPC_TIMEOUT)?;
        process.write(&json!({"method": "initialized"}))?;
        let configuration =
            process.request("config/read", json!({"includeLayers": false}), RPC_TIMEOUT)?;
        let config = configuration
            .get("config")
            .ok_or("Codex did not return its effective configuration.")?;
        if config
            .pointer("/features/shell_tool")
            .and_then(Value::as_bool)
            != Some(false)
            || config.pointer("/features/apps").and_then(Value::as_bool) != Some(false)
            || config.pointer("/features/hooks").and_then(Value::as_bool) != Some(false)
            || config
                .pointer("/features/multi_agent")
                .and_then(Value::as_bool)
                != Some(false)
            || config.pointer("/features/plugins").and_then(Value::as_bool) != Some(false)
            || config.pointer("/features/goals").and_then(Value::as_bool) != Some(false)
            || config
                .pointer("/features/memories")
                .and_then(Value::as_bool)
                != Some(false)
            || config.pointer("/agents/enabled").and_then(Value::as_bool) != Some(false)
            || config.pointer("/tools/view_image").and_then(Value::as_bool) != Some(false)
            || config.get("web_search").and_then(Value::as_str) != Some("disabled")
            || config.get("approval_policy").and_then(Value::as_str) != Some("never")
            || config.get("sandbox_mode").and_then(Value::as_str) != Some("read-only")
            || config
                .get("mcp_servers")
                .and_then(Value::as_object)
                .is_some_and(|servers| !servers.is_empty())
        {
            return Err("Codex did not apply the required read-only and disabled-tool settings. Remove conflicting configuration from this app's Codex runtime and reconnect.".to_owned());
        }
        Ok(process)
    }

    fn models(&mut self) -> Result<Vec<CodexModel>, String> {
        let mut models = Vec::new();
        let mut cursor = None::<String>;
        let mut seen = HashSet::new();
        for _ in 0..10 {
            let result = self.request(
                "model/list",
                json!({"limit": 100, "includeHidden": false, "cursor": cursor}),
                RPC_TIMEOUT,
            )?;
            for item in result
                .get("data")
                .and_then(Value::as_array)
                .ok_or("Codex returned an invalid model catalog.")?
            {
                let model_id = bounded_string(item, "model", 200)?;
                if !seen.insert(model_id.clone()) {
                    continue;
                }
                let efforts = item.get("supportedReasoningEfforts").and_then(Value::as_array).ok_or("This Codex model catalog does not expose reasoning capabilities. Update Codex.")?
                    .iter().filter_map(|effort| effort.get("reasoningEffort").and_then(Value::as_str))
                    .filter(|effort| effort.len() <= 32).map(str::to_owned).collect::<Vec<_>>();
                if efforts.is_empty() {
                    continue;
                }
                models.push(CodexModel {
                    model_id,
                    display_name: bounded_string(item, "displayName", 200)?,
                    default_reasoning_effort: item
                        .get("defaultReasoningEffort")
                        .and_then(Value::as_str)
                        .filter(|effort| {
                            efforts
                                .iter()
                                .any(|candidate| candidate.as_str() == *effort)
                        })
                        .map(str::to_owned),
                    supported_reasoning_efforts: efforts,
                    is_default: item
                        .get("isDefault")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                });
            }
            cursor = result
                .get("nextCursor")
                .and_then(Value::as_str)
                .map(str::to_owned);
            if cursor.is_none() {
                return Ok(models);
            }
        }
        Err("Codex returned more models than the supported catalog limit.".to_owned())
    }

    fn generate<F>(
        &mut self,
        input: &CodexRunInput,
        stage: &Path,
        cancelled: &AtomicBool,
        started_at: Instant,
        progress: &F,
    ) -> Result<CodexRunResult, String>
    where
        F: Fn(CodexProgress),
    {
        check_cancelled(&self.shutdown, Some(cancelled))?;
        if started_at.elapsed() >= RUN_TIMEOUT {
            return Err("The Codex operation exceeded ten minutes before creating a thread. No generation was started.".to_owned());
        }
        let started = self.request(
            "thread/start",
            thread_start_parameters(&input.model, stage, &input.system_instruction),
            RPC_TIMEOUT,
        )?;
        let thread_id = bounded_string(
            started
                .get("thread")
                .ok_or("Codex did not create a thread.")?,
            "id",
            256,
        )?;
        if started
            .pointer("/thread/ephemeral")
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err("Codex did not create the required private in-memory thread. Update the runtime before trying again.".to_owned());
        }
        if started
            .get("instructionSources")
            .and_then(Value::as_array)
            .is_some_and(|sources| !sources.is_empty())
        {
            return Err("Codex loaded external instruction files. Remove them from this app's isolated runtime before generating.".to_owned());
        }
        if started
            .get("model")
            .and_then(Value::as_str)
            .is_some_and(|model| model != input.model)
        {
            return Err("Codex selected a different model than the reviewed request. No generation was started.".to_owned());
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(cancelled_message());
        }
        if started_at.elapsed() >= RUN_TIMEOUT {
            return Err("The Codex operation exceeded ten minutes before starting the model turn. No generation was started.".to_owned());
        }
        progress(progress_event(
            &input.run_id,
            "starting",
            Some(&thread_id),
            None,
            "Starting the reviewed explanation request.",
        ));
        if self.account_revision != input.expected_runtime.account_revision {
            return Err(codex_identity::changed());
        }
        let request_id = self.send_request("turn/start", json!({
            "threadId": thread_id, "input": [{"type": "text", "text": input.user_prompt}],
            "cwd": stage, "model": input.model, "effort": input.effort, "approvalPolicy": "never",
            "sandboxPolicy": {"type": "readOnly", "access": {"type": "restricted", "includePlatformDefaults": true, "readableRoots": [stage]}, "networkAccess": false},
            "outputSchema": input.output_schema,
        }))?;
        let mut cancel_at = None;
        let mut turn_id = None::<String>;
        let mut response_bytes = 0;
        let mut last_progress = Instant::now();
        let mut messages = HashMap::<String, String>::new();
        let mut final_order = Vec::<String>::new();
        loop {
            if self.shutdown.load(Ordering::Acquire) {
                return Err("Codex stopped because Code Atlas is closing.".to_owned());
            }
            if started_at.elapsed() >= RUN_TIMEOUT {
                return Err("The Codex run exceeded ten minutes and was stopped. Review again before retrying.".to_owned());
            }
            if cancelled.load(Ordering::Acquire) && cancel_at.is_none() {
                cancel_at = Some(Instant::now());
                if let Some(turn_id) = &turn_id {
                    self.send_interrupt(&thread_id, turn_id)?;
                }
                progress(progress_event(
                    &input.run_id,
                    "cancelling",
                    Some(&thread_id),
                    turn_id.as_deref(),
                    "Stopping Codex. No generated response will be accepted.",
                ));
            }
            if cancel_at.is_some_and(|at| at.elapsed() >= CANCEL_GRACE) {
                return Err(cancelled_message());
            }
            let Some(message) = self.receive(Duration::from_millis(100), &mut response_bytes)?
            else {
                continue;
            };
            if message.get("id").and_then(Value::as_u64) == Some(request_id)
                && message.get("method").is_none()
            {
                if message.get("error").is_some() {
                    return Err(rpc_failure("turn/start", &message));
                }
                let result = message
                    .get("result")
                    .ok_or("Codex did not acknowledge the turn.")?;
                turn_id = Some(bounded_string(
                    result.get("turn").ok_or("Codex did not return a turn.")?,
                    "id",
                    256,
                )?);
                if cancel_at.is_some() {
                    if let Some(turn_id) = turn_id.as_deref() {
                        self.send_interrupt(&thread_id, turn_id)?;
                    }
                }
                continue;
            }
            if self.reject_server_request(&message)? {
                continue;
            }
            let method = message.get("method").and_then(Value::as_str).unwrap_or("");
            let params = message.get("params").unwrap_or(&Value::Null);
            if let Some(other_thread) = params.get("threadId").and_then(Value::as_str) {
                if other_thread != thread_id {
                    continue;
                }
            }
            if let (Some(current), Some(other)) =
                (&turn_id, params.get("turnId").and_then(Value::as_str))
            {
                if other != current {
                    continue;
                }
            }
            match method {
                "configWarning" | "warning" => return Err("Codex reported a configuration or runtime warning. The run was stopped rather than using uncertain settings.".to_owned()),
                "account/updated" => return Err(codex_identity::changed()),
                "model/rerouted" => return Err("Codex changed the requested model. The response was discarded; choose and review a model again.".to_owned()),
                "item/started" | "item/completed" => {
                    let item = params.get("item").ok_or("Codex returned an invalid item event.")?;
                    let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
                    if !matches!(kind, "userMessage" | "agentMessage" | "plan" | "reasoning" | "contextCompaction") {
                        return Err("Codex attempted a tool outside this request's permitted explanation scope. The run was stopped.".to_owned());
                    }
                    if method == "item/completed" && kind == "agentMessage" {
                        let id = bounded_string(item, "id", 256)?;
                        let text = bounded_string(item, "text", MAX_RESPONSE_BYTES)?;
                        messages.insert(id.clone(), text);
                        final_order.retain(|candidate| candidate != &id);
                        final_order.push(id);
                    }
                }
                "item/agentMessage/delta" => {
                    if last_progress.elapsed() >= Duration::from_millis(500) {
                        progress(progress_event(&input.run_id, "generating", Some(&thread_id), turn_id.as_deref(), "Codex is generating the structured explanation."));
                        last_progress = Instant::now();
                    }
                }
                "turn/completed" => {
                    let turn = params.get("turn").ok_or("Codex returned an invalid completion event.")?;
                    let completed_id = bounded_string(turn, "id", 256)?;
                    if turn_id.as_deref().is_some_and(|id| id != completed_id) { continue; }
                    if cancelled.load(Ordering::Acquire) { return Err(cancelled_message()); }
                    match turn.get("status").and_then(Value::as_str) {
                        Some("completed") => {}
                        Some("interrupted") => return Err(cancelled_message()),
                        _ => return Err(rpc_failure("turn/completed", turn)),
                    }
                    // Completed-item events are authoritative, not partial
                    // deltas. Some versions also include final items here.
                    if let Some(items) = turn.get("items").and_then(Value::as_array) {
                        for item in items {
                            let kind = item.get("type").and_then(Value::as_str).unwrap_or("");
                            if !matches!(kind, "userMessage" | "agentMessage" | "plan" | "reasoning" | "contextCompaction") {
                                return Err("Codex returned a tool action outside this request's permitted explanation scope. No response was accepted.".to_owned());
                            }
                            if kind != "agentMessage" { continue; }
                            let id = bounded_string(item, "id", 256)?;
                            let text = bounded_string(item, "text", MAX_RESPONSE_BYTES)?;
                            messages.insert(id.clone(), text);
                            final_order.retain(|candidate| candidate != &id);
                            final_order.push(id);
                        }
                    }
                    let response_text = final_order
                        .last()
                        .and_then(|id| messages.get(id))
                        .filter(|text| {
                            crate::strict_json::from_slice(text.as_bytes())
                                .is_ok_and(|value| value.is_object())
                        })
                        .cloned()
                        .ok_or("Codex completed without a complete JSON response. No response was accepted.")?;
                    self.request("thread/unsubscribe", json!({"threadId": thread_id}), RPC_TIMEOUT)?;
                    if cancelled.load(Ordering::Acquire) { return Err(cancelled_message()); }
                    progress(progress_event(&input.run_id, "validating", Some(&thread_id), Some(&completed_id), "Codex completed. Checking the explanation against its exact source evidence."));
                    return Ok(CodexRunResult { thread_id, turn_id: completed_id, model: input.model.clone(), effort: input.effort.clone(), response_text });
                }
                _ => {}
            }
        }
    }

    fn request(&mut self, method: &str, params: Value, timeout: Duration) -> Result<Value, String> {
        let result = self.request_inner(method, params, timeout);
        if result.is_err() {
            self.stop();
        }
        result
    }

    fn request_inner(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value, String> {
        self.check_cancelled()?;
        let id = self.send_request(method, params)?;
        let started = Instant::now();
        let mut received = 0;
        while started.elapsed() < timeout {
            self.check_cancelled()?;
            let Some(message) = self.receive(Duration::from_millis(100), &mut received)? else {
                continue;
            };
            if self.reject_server_request(&message)? {
                continue;
            }
            if message.get("method").and_then(Value::as_str) == Some("configWarning") {
                return Err("Codex reported a configuration warning. Update the runtime before using this integration.".to_owned());
            }
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            if message.get("error").is_some() {
                return Err(rpc_failure(method, &message));
            }
            return message
                .get("result")
                .cloned()
                .ok_or_else(|| "Codex returned an invalid protocol response.".to_owned());
        }
        Err(format!(
            "Codex timed out while handling {method}. Reconnect before retrying."
        ))
    }

    fn send_request(&mut self, method: &str, params: Value) -> Result<u64, String> {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or("Codex request IDs are exhausted. Reconnect.")?;
        self.write(&json!({"id": id, "method": method, "params": params}))?;
        Ok(id)
    }

    fn check_cancelled(&self) -> Result<(), String> {
        check_cancelled(&self.shutdown, self.cancelled.as_deref())
    }

    fn send_interrupt(&mut self, thread_id: &str, turn_id: &str) -> Result<(), String> {
        let id = self.next_id;
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or("Codex request IDs are exhausted. Reconnect.")?;
        self.write_inner(&json!({"id": id, "method": "turn/interrupt", "params": {"threadId": thread_id, "turnId": turn_id}}), true)
    }

    fn write(&mut self, message: &Value) -> Result<(), String> {
        self.write_inner(message, false)
    }

    fn write_inner(&mut self, message: &Value, allow_cancelled: bool) -> Result<(), String> {
        check_cancelled(
            &self.shutdown,
            if allow_cancelled {
                None
            } else {
                self.cancelled.as_deref()
            },
        )?;
        let bytes = serde_json::to_vec(message).map_err(|_| "Codex request encoding failed.")?;
        if bytes.len() > MAX_FRAME_BYTES {
            return Err("The Codex request exceeds the local message limit.".to_owned());
        }
        let (acknowledged, result) = mpsc::sync_channel(1);
        self.writer_sender
            .as_ref()
            .ok_or("Codex input is closed.")?
            .try_send(WriteFrame {
                bytes,
                acknowledged,
            })
            .map_err(|_| "Codex input is unavailable. Reconnect before trying again.")?;
        let started = Instant::now();
        let deadline = if allow_cancelled {
            CANCEL_GRACE
        } else {
            RPC_TIMEOUT
        };
        loop {
            check_cancelled(
                &self.shutdown,
                if allow_cancelled {
                    None
                } else {
                    self.cancelled.as_deref()
                },
            )?;
            match result.recv_timeout(Duration::from_millis(100)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) if started.elapsed() < deadline => {}
                Err(_) => {
                    self.stop();
                    return Err(
                        "Codex stopped accepting input. Reconnect before trying again.".to_owned(),
                    );
                }
            }
        }
    }

    fn receive(&mut self, timeout: Duration, total: &mut usize) -> Result<Option<Value>, String> {
        match self
            .receiver
            .as_ref()
            .ok_or("Codex output is closed.")?
            .recv_timeout(timeout)
        {
            Ok(Ok(value)) => {
                *total = total.saturating_add(
                    serde_json::to_vec(&value)
                        .map_err(|_| "Codex output could not be measured.")?
                        .len(),
                );
                if *total > MAX_OPERATION_BYTES {
                    return Err("Codex output exceeded the local operation limit.".to_owned());
                }
                if value.get("method").and_then(Value::as_str) == Some("account/updated") {
                    self.account_revision = self
                        .account_revision
                        .checked_add(1)
                        .ok_or_else(codex_identity::changed)?;
                }
                Ok(Some(value))
            }
            Ok(Err(error)) => Err(error),
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                Err("Codex closed its output. Reconnect before trying again.".to_owned())
            }
        }
    }

    fn reject_server_request(&mut self, message: &Value) -> Result<bool, String> {
        let (Some(id), Some(method)) = (
            message.get("id"),
            message.get("method").and_then(Value::as_str),
        ) else {
            return Ok(false);
        };
        let result = match method {
            "item/commandExecution/requestApproval" | "item/fileChange/requestApproval" => {
                json!({"decision": "decline"})
            }
            "item/permissions/requestApproval" => json!({"permissions": {}, "scope": "turn"}),
            "mcpServer/elicitation/request" => json!({"action": "decline", "content": null}),
            _ => {
                self.write(&json!({"id": id, "error": {"code": -32601, "message": "This client does not permit this operation."}}))?;
                return Ok(true);
            }
        };
        self.write(&json!({"id": id, "result": result}))?;
        Ok(true)
    }

    fn has_exited(&mut self) -> Result<bool, String> {
        if self.reaped {
            return Ok(true);
        }
        if self
            .child
            .try_wait()
            .map_err(|_| "Codex process status could not be read.")?
            .is_some()
        {
            self.reaped = true;
            self.process_id.store(0, Ordering::Release);
        }
        Ok(self.reaped)
    }

    fn stop(&mut self) {
        if !self.reaped {
            kill_and_reap(&mut self.child);
            self.reaped = true;
            self.process_id.store(0, Ordering::Release);
        }
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        self.receiver.take(); // Unblock a reader waiting on the bounded queue.
        self.writer_sender.take();
        self.stop();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

fn read_frame(reader: &mut impl BufRead) -> Result<Value, String> {
    let mut bytes = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|_| "Codex output could not be read.")?;
        if available.is_empty() {
            return Err("Codex closed its output.".to_owned());
        }
        let count = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .unwrap_or(available.len());
        if bytes.len().saturating_add(count) > MAX_FRAME_BYTES {
            return Err("Codex sent a message above the local size limit.".to_owned());
        }
        let complete = available[count - 1] == b'\n';
        bytes.extend_from_slice(&available[..count]);
        reader.consume(count);
        if complete {
            let value = crate::strict_json::from_slice(&bytes).map_err(|_| {
                "Codex sent malformed or duplicate-key protocol output. Reconnect with a supported runtime."
                    .to_owned()
            })?;
            if !value.is_object() {
                return Err(
                    "Codex sent a non-object protocol frame. No response was accepted.".to_owned(),
                );
            }
            return Ok(value);
        }
    }
}

fn validate_run(input: &CodexRunInput) -> Result<(), String> {
    if !input.expected_runtime.valid_shape() {
        return Err(codex_identity::changed());
    }
    if input.run_id.is_empty()
        || input.run_id.len() > 256
        || input.model.is_empty()
        || input.model.len() > 200
        || input.effort.is_empty()
        || input.effort.len() > 32
    {
        return Err("The Codex request identity, model or reasoning effort is invalid.".to_owned());
    }
    if input.approval_policy != "never" {
        return Err("This explanation route supports only read-only generation with approvals disabled. It cannot grant command, write or network permission.".to_owned());
    }
    if input.system_instruction.is_empty()
        || input.system_instruction.len() > 64 * 1024
        || input.user_prompt.is_empty()
        || input.user_prompt.len() > 768 * 1024
    {
        return Err(
            "The Codex explanation prompt is empty or exceeds the supported limit.".to_owned(),
        );
    }
    if !input.output_schema.is_object()
        || serde_json::to_vec(&input.output_schema)
            .map_err(|_| "Invalid explanation output schema.")?
            .len()
            > 128 * 1024
    {
        return Err("The explanation output schema is invalid or too large.".to_owned());
    }
    if input.sources.is_empty() || input.sources.len() > MAX_SOURCE_FILES {
        return Err("Choose between 1 and 24 reviewed source files for Codex.".to_owned());
    }
    let mut seen = HashSet::new();
    let mut total = 0_usize;
    for source in &input.sources {
        let path = Path::new(&source.path);
        if source.path.is_empty()
            || source.path.len() > 4096
            || path.is_absolute()
            || path
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
            || !seen.insert(&source.path)
        {
            return Err(
                "The Codex source manifest contains an invalid or duplicate path.".to_owned(),
            );
        }
        total = total.saturating_add(source.contents.len());
        if total > MAX_CONTEXT_BYTES || hex_digest(source.contents.as_bytes()) != source.sha256 {
            return Err("The Codex source manifest changed or exceeds 256 KB. Build a fresh reviewed request.".to_owned());
        }
    }
    Ok(())
}

struct StagedContext {
    path: PathBuf,
}
impl StagedContext {
    fn create(storage_dir: &Path, sources: &[CodexSourceFile]) -> Result<Self, String> {
        let root = runtime_root(storage_dir)?.join("staging");
        ensure_private_directory(&root)?;
        let path = root.join(Uuid::new_v4().to_string());
        fs::create_dir(&path).map_err(|_| "The reviewed Codex context could not be staged.")?;
        private_permissions(&path, 0o700)?;
        let stage = Self { path };
        let mut manifest = Vec::new();
        for (index, source) in sources.iter().enumerate() {
            // Preserve original names in metadata only. AGENTS.md, .codex and
            // repository hooks can never become executable configuration.
            let filename = format!("source-{index:04}.txt");
            let target = stage.path.join(&filename);
            create_private_file(&target, source.contents.as_bytes())?;
            private_permissions(&target, 0o400)?;
            manifest.push(
                json!({"path": source.path, "sha256": source.sha256, "stagedFile": filename}),
            );
        }
        let bytes = serde_json::to_vec(&manifest)
            .map_err(|_| "The staged source manifest could not be encoded.")?;
        create_private_file(&stage.path.join("manifest.json"), &bytes)?;
        private_permissions(&stage.path.join("manifest.json"), 0o400)?;
        private_permissions(&stage.path, 0o500)?;
        Ok(stage)
    }
}
impl Drop for StagedContext {
    fn drop(&mut self) {
        let _ = private_permissions(&self.path, 0o700);
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn runtime_root(storage_dir: &Path) -> Result<PathBuf, String> {
    let root = storage_dir.join("codex-agent");
    ensure_private_directory(&root)?;
    Ok(root)
}

fn configured_executable(storage_dir: &Path) -> Result<Option<PathBuf>, String> {
    let settings = storage_dir.join("codex-agent/runtime.json");
    match fs::symlink_metadata(&settings) {
        Ok(metadata) => {
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() > 16 * 1024
            {
                return Err(
                    "The saved Codex configuration is not a bounded regular file.".to_owned(),
                );
            }
            let mut bytes = Vec::new();
            File::open(&settings)
                .map_err(|_| "Codex setup could not be read.")?
                .take(16 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "Codex setup could not be read.")?;
            if bytes.len() > 16 * 1024 {
                return Err("The saved Codex configuration exceeds its size limit.".to_owned());
            }
            let config: RuntimeSettings = serde_json::from_slice(&bytes)
                .map_err(|_| "Codex setup is invalid. Select the executable again.")?;
            Ok(Some(checked_executable(Path::new(
                &config.executable_path,
            ))?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            for candidate in ["/opt/homebrew/bin/codex", "/usr/local/bin/codex"] {
                if let Ok(path) = checked_executable(Path::new(candidate)) {
                    return Ok(Some(path));
                }
            }
            Ok(None)
        }
        Err(_) => Err("Codex setup could not be inspected.".to_owned()),
    }
}

fn checked_executable(path: &Path) -> Result<PathBuf, String> {
    if !path.is_absolute() {
        return Err("Choose the absolute path to the installed Codex executable.".to_owned());
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| "The selected Codex executable is unavailable.")?;
    let metadata = fs::metadata(&canonical)
        .map_err(|_| "The selected Codex executable could not be inspected.")?;
    if !metadata.is_file() {
        return Err("Choose a Codex executable file, not a directory.".to_owned());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err("The selected Codex file is not executable.".to_owned());
        }
    }
    Ok(canonical)
}

/// Bounded native file observation; no executable, package manager or shell is
/// run by hashing. This does not close the path-to-exec replacement race.
fn executable_digest(
    executable: &Path,
    shutdown: &AtomicBool,
    cancelled: Option<&AtomicBool>,
) -> Result<String, String> {
    const LIMIT: u64 = 1024 * 1024 * 1024;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(executable)
        .map_err(|_| codex_identity::changed())?;
    let before = file.metadata().map_err(|_| codex_identity::changed())?;
    if !before.is_file() || before.len() == 0 || before.len() > LIMIT {
        return Err(codex_identity::changed());
    }
    let mut hasher = Sha256::new();
    let mut total = 0_u64;
    let mut bytes = [0_u8; 64 * 1024];
    loop {
        check_cancelled(shutdown, cancelled)?;
        let limit = (LIMIT - total + 1).min(bytes.len() as u64) as usize;
        let count = file
            .read(&mut bytes[..limit])
            .map_err(|_| codex_identity::changed())?;
        if count == 0 {
            break;
        }
        total += count as u64;
        if total > LIMIT {
            return Err(codex_identity::changed());
        }
        hasher.update(&bytes[..count]);
    }
    let after = file.metadata().map_err(|_| codex_identity::changed())?;
    if total != before.len()
        || after.len() != before.len()
        || before.modified().ok() != after.modified().ok()
    {
        return Err(codex_identity::changed());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn isolated_command(executable: &Path, root: &Path) -> Result<Command, String> {
    let home = root.join("home");
    let temporary = root.join("tmp");
    ensure_private_directory(&home)?;
    ensure_private_directory(&temporary)?;
    let mut command = Command::new(executable);
    command
        .current_dir(root)
        .env_clear()
        .env("CODEX_HOME", &home)
        .env("TMPDIR", &temporary)
        .env(
            "PATH",
            "/usr/bin:/bin:/usr/sbin:/sbin:/opt/homebrew/bin:/usr/local/bin",
        )
        .env("LANG", "en_US.UTF-8");
    if let Some(home) = std::env::var_os("HOME") {
        command.env("HOME", home);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    Ok(command)
}

/// Inspect the installed version's published schema before any agent turn. The
/// generator is a finite local command, not a second listening server.
fn verify_runtime_protocol(
    executable: &Path,
    root: &Path,
    process_id: &Arc<AtomicU32>,
    shutdown: &AtomicBool,
    cancelled: Option<&AtomicBool>,
) -> Result<(), String> {
    check_cancelled(shutdown, cancelled)?;
    let path = root.join(format!("schema-{}", Uuid::new_v4()));
    fs::create_dir(&path).map_err(|_| "Codex capability inspection could not start.")?;
    let _cleanup = TemporaryDirectory(path.clone());
    let mut command = isolated_command(executable, root)?;
    let mut child = command
        .args(["app-server", "generate-json-schema", "--out"])
        .arg(&path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "Codex schema inspection could not start. Check the executable.")?;
    let registration = RegisteredChild::new(process_id.clone(), child.id());
    let started = Instant::now();
    loop {
        if let Err(error) = check_cancelled(shutdown, cancelled) {
            kill_and_reap(&mut child);
            return Err(error);
        }
        match child.try_wait() {
            Ok(Some(status)) if status.success() => break,
            Ok(Some(_)) => return Err(UPGRADE.to_owned()),
            Ok(None) if started.elapsed() < Duration::from_secs(20) => {
                thread::sleep(Duration::from_millis(50))
            }
            _ => {
                kill_and_reap(&mut child);
                return Err("Codex capability inspection timed out. Check the installation before trying again.".to_owned());
            }
        }
    }
    drop(registration);
    let mut inspection = SchemaInspection::default();
    inspect_schema_directory(&path, 0, &mut inspection, shutdown, cancelled)?;
    let missing = [
        (!inspection.restricted_access, "restricted read roots"),
        (!inspection.read_only_access, "read-only access controls"),
        (!inspection.turn_output, "structured turn output"),
        (!inspection.thread_ephemeral, "ephemeral threads"),
        (!inspection.thread_instructions, "thread instructions"),
    ]
    .into_iter()
    .filter_map(|(missing, capability)| missing.then_some(capability))
    .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "Code Atlas requires protocol capabilities this Codex executable's published schema does not expose: {}. Sign-in and generation have not started. This adapter needs a compatible runtime integration before it can connect.",
            missing.join(", ")
        ));
    }
    Ok(())
}

#[derive(Default)]
struct SchemaInspection {
    entries: usize,
    bytes: usize,
    restricted_access: bool,
    read_only_access: bool,
    turn_output: bool,
    thread_ephemeral: bool,
    thread_instructions: bool,
}

fn inspect_schema_directory(
    path: &Path,
    depth: usize,
    inspection: &mut SchemaInspection,
    shutdown: &AtomicBool,
    cancelled: Option<&AtomicBool>,
) -> Result<(), String> {
    if depth > 8 {
        return Err(UPGRADE.to_owned());
    }
    for entry in fs::read_dir(path).map_err(|_| UPGRADE)? {
        check_cancelled(shutdown, cancelled)?;
        let entry = entry.map_err(|_| UPGRADE)?;
        inspection.entries += 1;
        if inspection.entries > 6000 {
            return Err(UPGRADE.to_owned());
        }
        let metadata = fs::symlink_metadata(entry.path()).map_err(|_| UPGRADE)?;
        if metadata.file_type().is_symlink() {
            return Err(UPGRADE.to_owned());
        }
        if metadata.is_dir() {
            inspect_schema_directory(&entry.path(), depth + 1, inspection, shutdown, cancelled)?;
        } else if entry.path().extension().is_some_and(|ext| ext == "json") {
            if metadata.len() > 16 * 1024 * 1024 {
                return Err(UPGRADE.to_owned());
            }
            let mut bytes = Vec::new();
            File::open(entry.path())
                .map_err(|_| UPGRADE)?
                .take(16 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| UPGRADE)?;
            inspection.bytes = inspection.bytes.saturating_add(bytes.len());
            if inspection.bytes > 64 * 1024 * 1024 {
                return Err(UPGRADE.to_owned());
            }
            let value: Value = serde_json::from_slice(&bytes).map_err(|_| UPGRADE)?;
            inspect_schema_value(&value, inspection);
        }
    }
    Ok(())
}

fn thread_start_parameters(model: &str, stage: &Path, instructions: &str) -> Value {
    // ThreadStartParams uses kebab-case SandboxMode; TurnStartParams uses
    // camelCase SandboxPolicy. They are different protocol types.
    json!({
        "model": model, "cwd": stage, "approvalPolicy": "never", "sandbox": "read-only",
        "ephemeral": true, "serviceName": "code_atlas", "developerInstructions": instructions,
    })
}

fn inspect_schema_value(value: &Value, inspection: &mut SchemaInspection) {
    if let Some(properties) = value.get("properties").and_then(Value::as_object) {
        let discriminant = properties.get("type");
        let is_variant = |name: &str| {
            discriminant.is_some_and(|tag| {
                tag.get("const").and_then(Value::as_str) == Some(name)
                    || tag
                        .get("enum")
                        .and_then(Value::as_array)
                        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(name)))
            })
        };
        if is_variant("restricted")
            && properties.contains_key("readableRoots")
            && properties.contains_key("includePlatformDefaults")
        {
            inspection.restricted_access = true;
        }
        if is_variant("readOnly") && properties.contains_key("access") {
            inspection.read_only_access = true;
        }
        if properties.contains_key("threadId")
            && properties.contains_key("input")
            && properties.contains_key("sandboxPolicy")
            && properties.contains_key("outputSchema")
        {
            inspection.turn_output = true;
        }
        if properties.contains_key("cwd")
            && properties.contains_key("model")
            && properties.contains_key("sandbox")
        {
            if properties.contains_key("ephemeral") {
                inspection.thread_ephemeral = true;
            }
            if properties.contains_key("developerInstructions") {
                inspection.thread_instructions = true;
            }
        }
    }
    match value {
        Value::Object(items) => {
            for nested in items.values() {
                inspect_schema_value(nested, inspection);
            }
        }
        Value::Array(items) => {
            for nested in items {
                inspect_schema_value(nested, inspection);
            }
        }
        _ => {}
    }
}

struct TemporaryDirectory(PathBuf);
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn ensure_private_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        Ok(_) => return Err("The Codex data path is not a regular local directory.".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path)
                .map_err(|_| "The private Codex data directory could not be created.")?;
        }
        Err(_) => return Err("The private Codex data directory could not be inspected.".to_owned()),
    }
    private_permissions(path, 0o700)
}

fn create_private_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(path)
        .map_err(|_| "The private Codex file could not be created.")?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "The private Codex file could not be saved.".to_owned())
}

fn atomic_private_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err("Codex configuration is not a regular file.".to_owned());
    }
    let temporary = path.with_file_name(format!(".settings-{}.tmp", Uuid::new_v4()));
    create_private_file(&temporary, bytes)?;
    let result = fs::rename(&temporary, path)
        .map_err(|_| "Codex configuration could not be saved.".to_owned());
    let _ = fs::remove_file(&temporary);
    result
}

fn private_permissions(path: &Path, mode: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|_| "Private Codex file permissions could not be applied.".to_owned())
    }
    #[cfg(not(unix))]
    {
        let _ = (path, mode);
        Ok(())
    }
}

/// Publish before checking shutdown so a concurrently spawning child cannot be
/// missed. The process lock permits exactly one managed child at a time.
struct RegisteredChild {
    process_id: Arc<AtomicU32>,
    pid: u32,
    armed: bool,
}
impl RegisteredChild {
    fn new(process_id: Arc<AtomicU32>, pid: u32) -> Self {
        process_id.store(pid, Ordering::Release);
        Self {
            process_id,
            pid,
            armed: true,
        }
    }
    fn disarm(mut self) {
        self.armed = false;
    }
}
impl Drop for RegisteredChild {
    fn drop(&mut self) {
        if self.armed {
            let _ =
                self.process_id
                    .compare_exchange(self.pid, 0, Ordering::AcqRel, Ordering::Acquire);
        }
    }
}

fn check_cancelled(shutdown: &AtomicBool, cancelled: Option<&AtomicBool>) -> Result<(), String> {
    if shutdown.load(Ordering::Acquire) {
        return Err("Codex stopped because Code Atlas is closing.".to_owned());
    }
    if cancelled.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        return Err(cancelled_message());
    }
    Ok(())
}

fn terminate_group(pid: u32) {
    if pid == 0 || pid > i32::MAX as u32 {
        return;
    }
    #[cfg(unix)]
    {
        // SAFETY: each managed command starts a fresh process group whose ID
        // is its child's PID; the positive bounded PID becomes that group ID.
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}

fn kill_and_reap(child: &mut Child) {
    terminate_group(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

fn bounded_string(value: &Value, key: &str, limit: usize) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && value.len() <= limit)
        .map(str::to_owned)
        .ok_or_else(|| format!("Codex returned an invalid {key} field."))
}

fn rpc_failure(method: &str, response: &Value) -> String {
    // Inspect error classes without exposing model output, selected source,
    // URLs containing OAuth codes, or runtime diagnostics in the UI/logs.
    let class = response.to_string().to_ascii_lowercase();
    if class.contains("usage_limit") || class.contains("rate_limit") || class.contains("quota") {
        "Your Codex account reached a usage limit. Wait for the reset or manage your plan; no API fallback was attempted.".to_owned()
    } else if class.contains("unauthorized")
        || class.contains("authentication")
        || class.contains("login")
        || class.contains("token_expired")
    {
        "Codex account access expired or was unavailable. Reconnect your ChatGPT account."
            .to_owned()
    } else if class.contains("invalid params")
        || class.contains("-32602")
        || class.contains("unknown field")
    {
        UPGRADE.to_owned()
    } else {
        format!(
            "Codex could not complete {method}. The request was not retried and existing chapters were preserved."
        )
    }
}

fn progress_event(
    run_id: &str,
    phase: &str,
    thread_id: Option<&str>,
    turn_id: Option<&str>,
    message: &str,
) -> CodexProgress {
    CodexProgress {
        run_id: run_id.to_owned(),
        phase: phase.to_owned(),
        thread_id: thread_id.map(str::to_owned),
        turn_id: turn_id.map(str::to_owned),
        message: message.to_owned(),
    }
}

fn cancelled_message() -> String {
    "The Codex run was cancelled locally. No response was accepted; the provider may already have processed some reviewed context.".to_owned()
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thread_start_uses_sandbox_mode_spelling_and_preserves_reviewed_settings() {
        let parameters = thread_start_parameters(
            "gpt-6-luna",
            Path::new("/synthetic/stage"),
            "Use the reviewed evidence only.",
        );
        assert_eq!(parameters["sandbox"], "read-only");
        assert_eq!(parameters["approvalPolicy"], "never");
        assert_eq!(parameters["ephemeral"], true);
        assert_eq!(parameters["model"], "gpt-6-luna");
        assert_eq!(parameters["cwd"], "/synthetic/stage");
        assert_eq!(parameters["developerInstructions"], "Use the reviewed evidence only.");
    }

    #[test]
    fn ordinary_read_only_schema_does_not_qualify_restricted_reads() {
        let mut inspection = SchemaInspection::default();
        inspect_schema_value(&json!({
            "properties": {
                "type": {"enum": ["readOnly"]},
                "networkAccess": {"type": "boolean"}
            }
        }), &mut inspection);
        assert!(!inspection.read_only_access);
        assert!(!inspection.restricted_access);
        inspect_schema_value(&json!({
            "oneOf": [
                {"properties": {"type": {"enum": ["readOnly"]}, "access": {}}},
                {"properties": {"type": {"enum": ["restricted"]}, "readableRoots": {}, "includePlatformDefaults": {}}}
            ]
        }), &mut inspection);
        assert!(inspection.read_only_access);
        assert!(inspection.restricted_access);
    }
}
