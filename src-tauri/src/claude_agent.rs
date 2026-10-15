//! Protocol preparation for the official Claude Code programmatic CLI.
//!
//! This module does not start a process, authenticate, or make model requests.
//! Native launch/account/isolation qualification is still required before an
//! automatic Claude Agent route can be enabled. In particular, a requested
//! model catalog is not evidence of subscription entitlement or approval.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Candidate only; no execution or compatibility acceptance has been recorded.
pub(crate) const CANDIDATE_RUNTIME_VERSION: &str = "2.1.300";
const MAX_PROMPT_BYTES: usize = 768 * 1024;
const MAX_SYSTEM_BYTES: usize = 64 * 1024;
const MAX_SCHEMA_BYTES: usize = 120 * 1024;
pub(crate) const MAX_FRAME_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const MAX_STREAM_BYTES: usize = 32 * 1024 * 1024;
const MAX_RESULT_BYTES: usize = 1_000_000;
const MAX_EVENTS: usize = 100_000;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ClaudeGenerationSettings {
    pub model_id: String,
    /// None means omit --effort and explicitly review the runtime default.
    pub effort: Option<String>,
    pub approval_policy: String,
    pub max_turns: u32,
}

/// Native-owned capability/identity data, never deserialized from the WebView.
/// The connection owner obtains these observations itself. Review identity is
/// separate from account/runtime qualification and does not authorize a run.
pub(crate) struct ClaudeReviewRuntime {
    pub connection_id: String,
    pub executable_path: PathBuf,
    pub executable_sha256: String,
    pub runtime_version: String,
    /// Opaque digest of the native account identity, never a credential.
    pub account_identity_sha256: String,
    pub models: Vec<ClaudeModelCapability>,
}

pub(crate) struct ClaudeModelCapability {
    pub model_id: String,
    pub supported_efforts: Vec<String>,
}

impl ClaudeReviewRuntime {
    pub(crate) fn validate_settings(
        &self,
        settings: &ClaudeGenerationSettings,
    ) -> Result<(), String> {
        if uuid::Uuid::parse_str(&self.connection_id).is_err()
            || !self.executable_path.is_absolute()
            || self.executable_path.to_str().is_none()
            || self.executable_path.as_os_str().len() > 4096
            || !is_digest(&self.executable_sha256)
            || !is_digest(&self.account_identity_sha256)
            || self.runtime_version != CANDIDATE_RUNTIME_VERSION
        {
            return Err("The Claude runtime/account identity is missing or changed. Prepare a fresh review with a qualified runtime.".to_owned());
        }
        validate_settings_shape(settings)?;
        let matches: Vec<_> = self
            .models
            .iter()
            .filter(|model| model.model_id == settings.model_id)
            .collect();
        if matches.len() != 1 {
            return Err("The selected Claude model is not uniquely advertised by this runtime. Refresh the native model catalog.".to_owned());
        }
        if let Some(effort) = &settings.effort {
            if !matches[0].supported_efforts.contains(effort) {
                return Err("This Claude model does not advertise the selected reasoning effort. Choose an advertised setting or the runtime default.".to_owned());
            }
        }
        Ok(())
    }
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_settings_shape(settings: &ClaudeGenerationSettings) -> Result<(), String> {
    if !is_exact_model_id(&settings.model_id) {
        return Err("Choose a full Claude model identifier from the runtime catalog; aliases and alternate-provider model names are unavailable.".to_owned());
    }
    if settings
        .effort
        .as_deref()
        .is_some_and(|effort| !matches!(effort, "low" | "medium" | "high" | "xhigh" | "max"))
        || settings.approval_policy != "dontAsk"
        || !(1..=8).contains(&settings.max_turns)
    {
        return Err("Use an advertised Claude reasoning effort, the fixed Dont ask approval policy and a turn limit from 1 to 8. Tool, command and write approvals are unavailable for explanation generation.".to_owned());
    }
    Ok(())
}

/// A supported full wire selector, not a guarantee that model weights are
/// immutable. Named aliases require the SDK's explicit canonical resolution.
pub(crate) fn is_exact_model_id(value: &str) -> bool {
    value.starts_with("claude-")
        && value.len() > "claude-".len()
        && value.len() <= 200
        && !value.ends_with("-latest")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

/// Prepared argv/stdin only. Creation grants no right to execute the runtime.
/// The launcher must separately enforce account eligibility and OS/runtime
/// isolation, revalidate the executable digest and use private staging files.
pub(crate) struct ClaudeInvocation {
    args: Vec<String>,
    settings: ClaudeGenerationSettings,
    executable_path: PathBuf,
    executable_sha256: String,
    account_identity_sha256: String,
    system_path: PathBuf,
    pub stdin: Vec<u8>,
    pub system_instruction: Vec<u8>,
}

pub(crate) fn prepare_invocation(
    runtime: &ClaudeReviewRuntime,
    settings: &ClaudeGenerationSettings,
    system_instruction: &str,
    user_prompt: &str,
    output_schema: &Value,
    staged_system_path: &Path,
) -> Result<ClaudeInvocation, String> {
    runtime.validate_settings(settings)?;
    if system_instruction.is_empty()
        || system_instruction.len() > MAX_SYSTEM_BYTES
        || user_prompt.is_empty()
        || user_prompt.len() > MAX_PROMPT_BYTES
        || !output_schema.is_object()
        || !staged_system_path.is_absolute()
        || staged_system_path.to_str().is_none()
        || staged_system_path.as_os_str().len() > 4096
    {
        return Err(
            "The reviewed Claude prompt, output schema or private staging path is invalid."
                .to_owned(),
        );
    }
    let schema = serde_json::to_string(output_schema)
        .map_err(|_| "The Claude output schema could not be encoded.".to_owned())?;
    if schema.len() > MAX_SCHEMA_BYTES {
        return Err("The Claude output schema exceeds 120 KB.".to_owned());
    }
    let mut args: Vec<String> = [
        "--print",
        "--output-format",
        "stream-json",
        "--verbose",
        "--include-partial-messages",
        "--safe-mode",
        "--restricted",
        "--setting-sources",
        "",
        "--tools",
        "",
        "--disallowedTools",
        "mcp__*",
        "--strict-mcp-config",
        "--mcp-config",
        "{\"mcpServers\":{}}",
        "--disable-slash-commands",
        "--no-chrome",
        "--no-session-persistence",
        "--permission-mode",
        "dontAsk",
        "--permission-prompts",
        "none",
        "--settings",
        "{\"disableAllHooks\":true,\"fallbackModel\":[]}",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    args.extend([
        "--model".to_owned(),
        settings.model_id.clone(),
        "--max-turns".to_owned(),
        settings.max_turns.to_string(),
        "--json-schema".to_owned(),
        schema,
        "--system-prompt-file".to_owned(),
        staged_system_path.to_string_lossy().into_owned(),
    ]);
    if let Some(effort) = &settings.effort {
        args.extend(["--effort".to_owned(), effort.clone()]);
    }
    Ok(ClaudeInvocation {
        args,
        settings: settings.clone(),
        executable_path: runtime.executable_path.clone(),
        executable_sha256: runtime.executable_sha256.clone(),
        account_identity_sha256: runtime.account_identity_sha256.clone(),
        system_path: staged_system_path.to_owned(),
        // Source goes only over stdin, never through argv or a shell.
        stdin: user_prompt.as_bytes().to_vec(),
        system_instruction: system_instruction.as_bytes().to_vec(),
    })
}

/// Construct a command without spawning it. No inherited credential, endpoint,
/// cloud-provider, shell startup, Node option or proxy environment is retained.
/// The vendor's own account store remains available through the real HOME;
/// the app must never copy/read its tokens. Managed policy remains a separate
/// qualification concern even with safe/restricted mode and hooks disabled.
pub(crate) fn prepare_command(
    runtime: &ClaudeReviewRuntime,
    invocation: &ClaudeInvocation,
    stage: &Path,
    temporary: &Path,
    account_home: &Path,
) -> Result<Command, String> {
    runtime.validate_settings(&invocation.settings)?;
    if invocation.executable_path != runtime.executable_path
        || invocation.executable_sha256 != runtime.executable_sha256
        || invocation.account_identity_sha256 != runtime.account_identity_sha256
        || invocation.system_path.parent() != Some(stage)
    {
        return Err("The prepared Claude invocation belongs to a different executable, account or private stage. Prepare a new review.".to_owned());
    }
    let mut command = isolated_command(&runtime.executable_path, stage, temporary, account_home)?;
    command.args(&invocation.args);
    Ok(command)
}

#[derive(Clone, Copy)]
pub(crate) enum ClaudeInspectionCommand {
    Version,
    AccountStatus,
}

/// Supported metadata commands only. There is no prompt, custom argv, login,
/// install/update action or model request in this preparation path.
pub(crate) fn prepare_inspection_command(
    executable: &Path,
    operation: ClaudeInspectionCommand,
    stage: &Path,
    temporary: &Path,
    account_home: &Path,
) -> Result<Command, String> {
    let mut command = isolated_command(executable, stage, temporary, account_home)?;
    match operation {
        ClaudeInspectionCommand::Version => {
            command.arg("--version");
        }
        ClaudeInspectionCommand::AccountStatus => {
            command.args(["auth", "status"]);
        }
    }
    Ok(command)
}

pub(crate) fn isolated_command(
    executable: &Path,
    stage: &Path,
    temporary: &Path,
    account_home: &Path,
) -> Result<Command, String> {
    if [executable, stage, temporary, account_home]
        .iter()
        .any(|path| {
            !path.is_absolute()
                || path.to_str().is_none()
                || path.as_os_str().len() > 4096
                || path
                    .components()
                    .any(|part| matches!(part, std::path::Component::ParentDir))
        })
    {
        return Err(
            "Claude requires absolute executable, private staging and account paths.".to_owned(),
        );
    }
    let mut command = Command::new(executable);
    command
        .current_dir(stage)
        .env_clear()
        .env("HOME", account_home)
        .env("TMPDIR", temporary)
        .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
        .env("LANG", "en_US.UTF-8")
        // Inspection/generation must not start the vendor's background
        // updater and replace the executable whose identity was reviewed.
        .env("DISABLE_AUTOUPDATER", "1");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    Ok(command)
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum ClaudeProgressPhase {
    Started,
    Generating,
    Retry,
    ResponseReceived,
}

/// A completed protocol result is still untrusted until the existing native
/// chapter/section citation and preservation validators have accepted it.
pub(crate) struct ClaudeStructuredResult {
    pub model_id: String,
    pub response_text: String,
    pub session_id: String,
}

/// Strict consumer for the documented newline-delimited stream. The future
/// supervisor supplies bounded complete frames, checks process exit, and calls
/// finish only after EOF. It must not publish a result before a successful exit.
pub(crate) struct ClaudeOutputStream {
    expected_model: String,
    expected_max_turns: u32,
    session_id: Option<String>,
    total_bytes: usize,
    event_count: usize,
    result: Option<ClaudeStructuredResult>,
    structured_tool_calls: HashSet<String>,
    failed: bool,
}

impl ClaudeOutputStream {
    pub(crate) fn new(settings: &ClaudeGenerationSettings) -> Result<Self, String> {
        validate_settings_shape(settings)?;
        Ok(Self {
            expected_model: settings.model_id.clone(),
            expected_max_turns: settings.max_turns,
            session_id: None,
            total_bytes: 0,
            event_count: 0,
            result: None,
            structured_tool_calls: HashSet::new(),
            failed: false,
        })
    }

    pub(crate) fn push(&mut self, frame: &[u8]) -> Result<Option<ClaudeProgressPhase>, String> {
        if self.failed {
            return Err(
                "The Claude output stream has already failed; no result can be accepted."
                    .to_owned(),
            );
        }
        let result = self.push_inner(frame);
        if result.is_err() {
            self.failed = true;
            self.result = None;
        }
        result
    }

    fn push_inner(&mut self, frame: &[u8]) -> Result<Option<ClaudeProgressPhase>, String> {
        self.total_bytes = self.total_bytes.saturating_add(frame.len());
        self.event_count = self.event_count.saturating_add(1);
        if frame.is_empty()
            || frame.len() > MAX_FRAME_BYTES
            || self.total_bytes > MAX_STREAM_BYTES
            || self.event_count > MAX_EVENTS
        {
            return Err("Claude output exceeded a local frame, event or stream limit.".to_owned());
        }
        if self.result.is_some() {
            return Err(
                "Claude sent output after its final result. No response was accepted.".to_owned(),
            );
        }
        let event = crate::strict_json::from_slice(frame)
            .map_err(|_| "Claude returned malformed or duplicate-key protocol output.".to_owned())?;
        if !event.is_object()
            || event
                .get("parent_tool_use_id")
                .is_some_and(|id| !id.is_null())
        {
            return Err("Claude returned an unexpected nested-agent event.".to_owned());
        }
        match (event["type"].as_str(), event["subtype"].as_str()) {
            (Some("system"), Some("init")) => self.initialize(&event),
            _ if self.session_id.is_none() => Err("Claude did not begin with the expected runtime initialization event.".to_owned()),
            (Some("result"), _) => self.complete(&event),
            (Some("assistant"), _) => {
                self.check_session(&event)?;
                if event["message"]["model"].as_str().is_some_and(|model| model != self.expected_model) {
                    return Err("Claude switched away from the reviewed model. No response was accepted.".to_owned());
                }
                let content = event["message"]["content"].as_array()
                    .ok_or_else(|| "Claude returned an invalid assistant message.".to_owned())?;
                for block in content {
                    if let Some(id) = check_content_block(block)? {
                        self.structured_tool_calls.insert(id);
                    }
                }
                Ok(Some(ClaudeProgressPhase::Generating))
            }
            (Some("stream_event"), _) => {
                self.check_session(&event)?;
                let delta = &event["event"];
                if delta["type"] == "content_block_start" {
                    if let Some(id) = check_content_block(&delta["content_block"])? {
                        self.structured_tool_calls.insert(id);
                    }
                }
                if delta["type"] == "message_start" && delta["message"]["model"].as_str().is_some_and(|model| model != self.expected_model) {
                    return Err("Claude started a response using another model.".to_owned());
                }
                Ok(None)
            }
            (Some("system"), Some("api_retry")) => {
                self.check_session(&event)?;
                // Report a fixed progress phase only, never the raw error or source.
                Ok(Some(ClaudeProgressPhase::Retry))
            }
            (Some("user"), _) => {
                self.check_session(&event)?;
                let content = event["message"]["content"].as_array()
                    .filter(|blocks| !blocks.is_empty())
                    .ok_or_else(|| "Claude returned an unexpected user message.".to_owned())?;
                for block in content {
                    let id = bounded_id(block, "tool_use_id")?;
                    if block["type"] != "tool_result" || !self.structured_tool_calls.remove(&id) {
                        return Err("Claude returned an unrequested external tool result or injected user content.".to_owned());
                    }
                }
                Ok(None)
            }
            _ => Err("Claude emitted an unsupported tool, permission, hook or session event. No response was accepted.".to_owned()),
        }
    }

    fn initialize(&mut self, event: &Value) -> Result<Option<ClaudeProgressPhase>, String> {
        if self.session_id.is_some()
            || event["model"].as_str() != Some(self.expected_model.as_str())
            || event["permissionMode"].as_str() != Some("dontAsk")
        {
            return Err(
                "Claude's effective model, permission mode or session does not match the review."
                    .to_owned(),
            );
        }
        // --json-schema can expose the internal StructuredOutput formatter
        // even with built-in tools disabled. It grants no filesystem, command
        // or MCP capability. External lookalikes are never allowed.
        if !event["tools"].as_array().is_some_and(|tools| {
            tools.len() <= 1
                && tools
                    .iter()
                    .all(|tool| tool.as_str() == Some("StructuredOutput"))
        }) || !event["mcp_servers"].as_array().is_some_and(Vec::is_empty)
        {
            return Err(
                "Claude exposed unexpected tools or MCP servers for a context-only explanation."
                    .to_owned(),
            );
        }
        for field in [
            "plugins",
            "agents",
            "skills",
            "plugin_errors",
            "mcp_server_errors",
        ] {
            if event
                .get(field)
                .is_some_and(|value| !value.as_array().is_some_and(Vec::is_empty))
            {
                return Err(
                    "Claude loaded unexpected customization, subagents or startup errors."
                        .to_owned(),
                );
            }
        }
        let session_id = bounded_id(event, "session_id")?;
        self.session_id = Some(session_id);
        Ok(Some(ClaudeProgressPhase::Started))
    }

    fn check_session(&self, event: &Value) -> Result<(), String> {
        if event["session_id"].as_str() != self.session_id.as_deref() {
            return Err("Claude returned an event from a different session.".to_owned());
        }
        Ok(())
    }

    fn complete(&mut self, event: &Value) -> Result<Option<ClaudeProgressPhase>, String> {
        self.check_session(event)?;
        if event["subtype"] != "success" || event["is_error"] != false {
            return Err("Claude did not complete a successful structured response. Check the account, quota and turn limit before reviewing a new request.".to_owned());
        }
        if !event["num_turns"]
            .as_u64()
            .is_some_and(|turns| (1..=u64::from(self.expected_max_turns)).contains(&turns))
        {
            return Err("Claude returned a missing, invalid or over-limit turn count. No response was accepted; review the runtime and turn setting before retrying.".to_owned());
        }
        if event
            .get("permission_denials")
            .is_some_and(|value| !value.as_array().is_some_and(Vec::is_empty))
        {
            return Err(
                "Claude requested permissions outside this explanation. No response was accepted."
                    .to_owned(),
            );
        }
        if !event["structured_output"].is_object() {
            return Err("Claude completed without a structured output object. Free-form result text cannot become a chapter.".to_owned());
        }
        if let Some(models) = event.get("modelUsage") {
            let models = models
                .as_object()
                .ok_or_else(|| "Claude returned invalid model usage metadata.".to_owned())?;
            if models.keys().any(|model| model != &self.expected_model) {
                return Err(
                    "Claude reported usage by an unreviewed model. No response was accepted."
                        .to_owned(),
                );
            }
        }
        let response_text = serde_json::to_string(&event["structured_output"])
            .map_err(|_| "Claude's structured response could not be decoded.".to_owned())?;
        if response_text.len() > MAX_RESULT_BYTES {
            return Err("Claude's structured response exceeded 1 MB.".to_owned());
        }
        self.result = Some(ClaudeStructuredResult {
            model_id: self.expected_model.clone(),
            response_text,
            session_id: bounded_id(event, "session_id")?,
        });
        Ok(Some(ClaudeProgressPhase::ResponseReceived))
    }

    pub(crate) fn finish(self, process_succeeded: bool) -> Result<ClaudeStructuredResult, String> {
        if self.failed || !process_succeeded {
            return Err(
                "The Claude process failed or its output was rejected. No draft was created."
                    .to_owned(),
            );
        }
        self.result.ok_or_else(|| {
            "Claude stopped before returning a structured result. No draft was created.".to_owned()
        })
    }
}

fn bounded_id(value: &Value, field: &str) -> Result<String, String> {
    value[field]
        .as_str()
        .filter(|id| !id.is_empty() && id.len() <= 256 && !id.chars().any(char::is_control))
        .map(str::to_owned)
        .ok_or_else(|| "Claude returned an invalid session identity.".to_owned())
}

fn check_content_block(block: &Value) -> Result<Option<String>, String> {
    match block["type"].as_str() {
        Some("text" | "thinking" | "redacted_thinking") => Ok(None),
        Some("tool_use") if block["name"] == "StructuredOutput" && block["input"].is_object() => {
            bounded_id(block, "id").map(Some)
        }
        // Formatter input/results cannot become a draft: consume only the
        // final structured_output field, then apply independent validation.
        _ => Err("Claude attempted an unexpected tool or content operation.".to_owned()),
    }
}
