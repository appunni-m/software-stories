//! Application-owned retrieval session. A private Unix socket bridges standard
//! MCP to the same native captured-code service, admission ledger and run owner.

use crate::{
    codex_identity::CodexReviewRuntime,
    context_mcp as mcp,
    evidence_ledger::{Context, Ledger, RangeRequest, Tool},
    index, repository_context as context,
    repository_context_commands::{self, RepositoryContextJobs},
    repository_structure as structure, snapshot,
};
use serde::Deserialize;
use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::{
    fs::{DirBuilderExt, PermissionsExt},
    net::{UnixListener, UnixStream},
};
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Search {
    query: String,
    scope: Option<String>,
    limit: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    ranges: Vec<RangeRequest>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Definitions {
    query: String,
    scope: Option<String>,
    limit: usize,
    offset: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Relationships {
    id: String,
    direction: structure::Direction,
    scope: Option<String>,
    limit: usize,
    offset: usize,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Map {
    scope: Option<String>,
    limit: usize,
    offset: usize,
}

fn parse<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, String> {
    serde_json::from_value(value)
        .map_err(|_| "The agent supplied invalid captured-code arguments.".into())
}
fn scope(value: Option<&str>) -> Option<&str> {
    value.filter(|path| !matches!(*path, "" | "."))
}
fn tool(name: &str) -> Result<Tool, String> {
    match name {
        "search_code" => Ok(Tool::Search),
        "read_code" => Ok(Tool::Read),
        "find_definitions" => Ok(Tool::Definitions),
        "find_relationships" => Ok(Tool::Relationships),
        "list_modules" => Ok(Tool::Map),
        _ => Err("The agent requested an unauthorized tool.".into()),
    }
}

pub(crate) fn arguments_digest(value: &Value) -> Result<String, String> {
    fn sorted(value: &Value) -> Value {
        match value {
            Value::Object(o) => {
                let mut keys = o.keys().collect::<Vec<_>>();
                keys.sort();
                Value::Object(
                    keys.into_iter()
                        .map(|k| (k.clone(), sorted(&o[k])))
                        .collect(),
                )
            }
            Value::Array(a) => Value::Array(a.iter().map(sorted).collect()),
            _ => value.clone(),
        }
    }
    serde_json::to_vec(&sorted(value))
        .map(|b| context::digest(&b))
        .map_err(|_| "Context arguments could not be encoded.".into())
}

pub(crate) struct Session {
    storage: PathBuf,
    pub(crate) manifest: snapshot::SnapshotManifest,
    runtime: CodexReviewRuntime,
    syntax: Option<index::SnapshotIndex>,
    jobs: RepositoryContextJobs,
    pub(crate) ledger: Ledger,
    directory: PathBuf,
    token: String,
    #[cfg(unix)]
    listener: UnixListener,
    #[cfg(unix)]
    pending: Option<Pending>,
    events: HashMap<String, (Tool, String)>,
    consumed: HashSet<String>,
    completed: HashMap<String, (Tool, String, String)>,
}

#[cfg(unix)]
struct Pending {
    stream: UnixStream,
    request: mcp::Request,
    received_at: Instant,
}

impl Session {
    #[cfg(unix)]
    pub(crate) fn new(
        storage: &Path,
        operation_id: &str,
        manifest: snapshot::SnapshotManifest,
        runtime: &CodexReviewRuntime,
        grant: &str,
        jobs: RepositoryContextJobs,
    ) -> Result<Self, String> {
        let ledger = Ledger::new(
            storage,
            operation_id,
            &manifest,
            grant,
            runtime,
            "gpt-6-luna",
            "max",
        )?;
        let syntax = index::load_snapshot_index(
            storage,
            &manifest.summary.project_id,
            &manifest.summary.snapshot_id,
        )
        .unwrap_or(None);
        // macOS Unix socket paths have a short limit. A fresh private directory
        // is created atomically; no shared listener or TCP port is introduced.
        let directory = Path::new("/tmp").join(format!("ca-context-{}", Uuid::new_v4()));
        fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .map_err(|_| "Private code tools could not start.")?;
        let result = (|| {
            let listener = UnixListener::bind(directory.join("context.sock"))
                .map_err(|_| "Private code tools could not start.")?;
            fs::set_permissions(
                directory.join("context.sock"),
                fs::Permissions::from_mode(0o600),
            )
            .map_err(|_| "Private code tool permissions could not be applied.")?;
            listener
                .set_nonblocking(true)
                .map_err(|_| "Private code tools could not start.")?;
            Ok(Self {
                storage: storage.into(),
                manifest,
                runtime: runtime.clone(),
                syntax,
                jobs,
                ledger,
                directory: directory.clone(),
                token: format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple()),
                listener,
                pending: None,
                events: HashMap::new(),
                consumed: HashSet::new(),
                completed: HashMap::new(),
            })
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(directory);
        }
        result
    }

    pub(crate) fn configuration(&self) -> Result<Value, String> {
        #[cfg(test)]
        let executable = std::env::var_os("CODE_ATLAS_CONTEXT_HELPER_BIN")
            .map(PathBuf::from)
            .map(Ok)
            .unwrap_or_else(std::env::current_exe)
            .map_err(|_| "The app's code tool helper is unavailable.")?;
        #[cfg(not(test))]
        let executable =
            std::env::current_exe().map_err(|_| "The app's code tool helper is unavailable.")?;
        Ok(
            json!({"command":executable,"args":["--atlas-context-mcp",self.directory.join("context.sock")],
            "env":{"CODE_ATLAS_CONTEXT_TOKEN":self.token},"enabled":true,"required":true,
            "enabled_tools":mcp::TOOL_NAMES,"startup_timeout_sec":20,"tool_timeout_sec":95,"default_tools_approval_mode":"approve"}),
        )
    }

    pub(crate) fn verify_catalog(&self, value: &Value) -> Result<(), String> {
        let servers = value["data"]
            .as_array()
            .ok_or("Codex did not report its captured-code tools.")?;
        if value.get("nextCursor").is_some_and(|c| !c.is_null()) {
            return Err("Codex returned an incomplete tool inventory.".into());
        }
        let own = servers
            .iter()
            .find(|s| s["name"] == mcp::SERVER)
            .ok_or("Codex did not connect the app's captured-code tools.")?;
        let tools = own["tools"]
            .as_object()
            .ok_or("Codex did not expose the app's captured-code tools.")?;
        let names = tools
            .values()
            .filter_map(|t| t["name"].as_str())
            .collect::<HashSet<_>>();
        if !own.get("toolsError").is_none_or(|e| e.is_null())
            || names.len() != mcp::TOOL_NAMES.len()
            || mcp::TOOL_NAMES.iter().any(|name| !names.contains(name))
            || servers.iter().any(|s| {
                s["name"] != mcp::SERVER && s["tools"].as_object().is_some_and(|t| !t.is_empty())
            })
        {
            return Err(
                "Codex did not apply the owned captured-code tool catalog. No question was sent."
                    .into(),
            );
        }
        Ok(())
    }

    /// Verify observed app-server events against responses admitted by this
    /// native session. An arbitrary MCP event is never accepted as evidence.
    pub(crate) fn event(&mut self, item: &Value, completed: bool) -> Result<(), String> {
        if item["server"] != mcp::SERVER
            || item.get("appContext").is_some_and(|v| !v.is_null())
            || item.get("pluginId").is_some_and(|v| !v.is_null())
        {
            return Err("Codex used a tool outside the captured-code session.".into());
        }
        let id = item["id"]
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .ok_or("Invalid code tool event identity.")?;
        let kind = tool(item["tool"].as_str().unwrap_or(""))?;
        let args = arguments_digest(&item["arguments"])?;
        if !completed {
            if self.events.len() >= 64
                || self.events.contains_key(id)
                || item["status"] != "inProgress"
            {
                return Err("Invalid or repeated code tool start.".into());
            }
            self.events.insert(id.into(), (kind, args));
            return Ok(());
        }
        if self.events.remove(id) != Some((kind, args.clone()))
            || item["status"] != "completed"
            || item.get("error").is_some_and(|e| !e.is_null())
        {
            return Err("The captured-code tool did not complete safely.".into());
        }
        let content = item
            .pointer("/result/content")
            .and_then(Value::as_array)
            .ok_or("Code tool reply is missing.")?;
        if content.len() != 1 || content[0]["type"] != "text" {
            return Err("Unexpected captured-code tool result.".into());
        }
        let text = content[0]["text"]
            .as_str()
            .filter(|t| t.len() <= mcp::MAX_REPLY)
            .ok_or("Code tool reply exceeds its bounds.")?;
        let call = self
            .ledger
            .delivered_call(kind, &args, &context::digest(text.as_bytes()))
            .ok_or("Code tool reply was not admitted by this operation.")?;
        if !self.consumed.insert(call.into()) {
            return Err("A code tool result was replayed.".into());
        }
        self.completed
            .insert(id.into(), (kind, args, context::digest(text.as_bytes())));
        Ok(())
    }

    pub(crate) fn completion_item(&self, item: &Value) -> Result<(), String> {
        let id = item["id"]
            .as_str()
            .ok_or("Invalid code tool completion identity.")?;
        let kind = tool(item["tool"].as_str().unwrap_or(""))?;
        let args = arguments_digest(&item["arguments"])?;
        let content = item
            .pointer("/result/content")
            .and_then(Value::as_array)
            .ok_or("Missing code tool completion result.")?;
        let text = content
            .first()
            .and_then(|c| c["text"].as_str())
            .filter(|t| t.len() <= mcp::MAX_REPLY)
            .ok_or("Invalid code tool completion result.")?;
        if item["server"] != mcp::SERVER
            || item["status"] != "completed"
            || content.len() != 1
            || content[0]["type"] != "text"
            || self.completed.get(id) != Some(&(kind, args, context::digest(text.as_bytes())))
        {
            return Err("An unverified code tool appeared in the final turn.".into());
        }
        Ok(())
    }

    #[cfg(unix)]
    pub(crate) fn poll(
        &mut self,
        cancelled: &Arc<AtomicBool>,
        mut gate: impl FnMut() -> Result<(), String>,
    ) -> Result<(), String> {
        if self.pending.is_none() {
            let (mut stream, _) = match self.listener.accept() {
                Ok(v) => v,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                Err(_) => return Err("The captured-code connection closed.".into()),
            };
            gate()?;
            self.ledger
                .check(&self.manifest, &self.runtime, cancelled)?;
            stream
                .set_read_timeout(Some(Duration::from_millis(200)))
                .map_err(|_| "Code tool transport unavailable.")?;
            stream
                .set_write_timeout(Some(Duration::from_secs(2)))
                .map_err(|_| "Code tool transport unavailable.")?;
            let bytes = mcp::read_packet(&mut stream, mcp::MAX_INPUT)?;
            let request: mcp::Request = parse(
                crate::strict_json::from_slice(&bytes).map_err(|_| "Invalid code tool packet.")?,
            )?;
            if request.token != self.token {
                return Err("Unknown captured-code authority.".into());
            }
            tool(&request.name)?;
            self.pending = Some(Pending {
                stream,
                request,
                received_at: Instant::now(),
            });
        }
        self.ledger
            .check(&self.manifest, &self.runtime, cancelled)?;
        let pending = self
            .pending
            .as_ref()
            .ok_or("Code tool connection unavailable.")?;
        let kind = tool(&pending.request.name)?;
        let args_hash = arguments_digest(&pending.request.arguments)?;
        if !self
            .events
            .values()
            .any(|(k, h)| *k == kind && h == &args_hash)
        {
            // MCP and app-server notifications are separate streams. Retain a
            // bounded private call while the owner drains its started event;
            // never expose code merely because the helper knows the nonce.
            if pending.received_at.elapsed() >= Duration::from_secs(3) {
                self.pending.take();
                return Err("The context call does not own an active Codex tool event.".into());
            }
            return Ok(());
        }
        let Pending {
            mut stream,
            request,
            ..
        } = self
            .pending
            .take()
            .ok_or("Code tool connection unavailable.")?;
        gate()?;
        let fresh = snapshot::load_snapshot_manifest(
            &self.storage,
            &self.manifest.summary.project_id,
            &self.manifest.summary.snapshot_id,
        )?;
        self.ledger.check(&fresh, &self.runtime, cancelled)?;
        let token = self
            .ledger
            .begin_call(kind, &args_hash, &fresh, &self.runtime, cancelled)?;
        let native_request = self
            .jobs
            .begin_with_cancel(&Uuid::new_v4().to_string(), Arc::clone(cancelled))?;
        let budget = native_request.budget();
        let send = |payload: &[u8]| {
            gate()?;
            mcp::write_packet(&mut stream, payload, mcp::MAX_REPLY)
        };
        match kind {
            Tool::Search => {
                let args: Search = parse(request.arguments)?;
                let input = repository_context_commands::SearchInput {
                    request_id: Uuid::new_v4().to_string(),
                    project_id: fresh.summary.project_id.clone(),
                    snapshot_id: fresh.summary.snapshot_id.clone(),
                    query: args.query,
                    path: scope(args.scope.as_deref()).map(str::to_owned),
                    limit: args.limit.clamp(1, 32),
                };
                let result = repository_context_commands::search_snapshot(
                    &self.storage,
                    &input,
                    &native_request,
                )?;
                self.ledger.deliver(
                    token,
                    Context::Search((&result).into()),
                    &[],
                    &fresh,
                    &self.runtime,
                    cancelled,
                    send,
                )
            }
            Tool::Read => {
                let args: Read = parse(request.arguments)?;
                self.ledger.deliver(
                    token,
                    Context::Read {},
                    &args.ranges,
                    &fresh,
                    &self.runtime,
                    cancelled,
                    send,
                )
            }
            Tool::Definitions => {
                let a: Definitions = parse(request.arguments)?;
                let result = structure::definitions(
                    &self.storage,
                    &fresh,
                    self.syntax.as_ref(),
                    &a.query,
                    scope(a.scope.as_deref()),
                    a.limit.clamp(1, 32),
                    a.offset,
                    &budget,
                )?;
                self.ledger.deliver(
                    token,
                    Context::Definitions(&result),
                    &[],
                    &fresh,
                    &self.runtime,
                    cancelled,
                    send,
                )
            }
            Tool::Relationships => {
                let a: Relationships = parse(request.arguments)?;
                if a.id.is_empty() || a.id.len() > 512 {
                    return Err("Invalid captured symbol identity.".into());
                }
                let result = structure::relationships(
                    &self.storage,
                    &fresh,
                    self.syntax.as_ref(),
                    &a.id,
                    a.direction,
                    scope(a.scope.as_deref()),
                    a.limit.clamp(1, 32),
                    a.offset,
                    &budget,
                )?;
                self.ledger.deliver(
                    token,
                    Context::Relationships(&result),
                    &[],
                    &fresh,
                    &self.runtime,
                    cancelled,
                    send,
                )
            }
            Tool::Map => {
                let a: Map = parse(request.arguments)?;
                let result = structure::module_map(
                    &self.storage,
                    &fresh,
                    self.syntax.as_ref(),
                    scope(a.scope.as_deref()),
                    a.limit.clamp(1, 32),
                    a.offset,
                    &budget,
                )?;
                self.ledger.deliver(
                    token,
                    Context::Map(&result),
                    &[],
                    &fresh,
                    &self.runtime,
                    cancelled,
                    send,
                )
            }
        }
    }

    pub(crate) fn check_complete(&self) -> Result<(), String> {
        if !self.events.is_empty() || self.consumed.is_empty() {
            return Err("Codex did not finish its captured-code investigation.".into());
        }
        Ok(())
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = fs::remove_file(self.directory.join("context.sock"));
        let _ = fs::remove_dir(&self.directory);
    }
}

#[cfg(test)]
mod tests;
