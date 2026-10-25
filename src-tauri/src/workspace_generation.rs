//! Agent-led workspace questions. The initial request contains intent and
//! starting points; all source access goes through the owned MCP session.

use crate::{
    ProjectSessions, agent_context, app_data_directory, chapters, codex_agent, context_mcp,
    provider, recovery, repository_context as context,
    repository_context_commands::RepositoryContextJobs, snapshot, workspace,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Instant,
};
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Reference {
    pub(crate) path: String,
    pub(crate) sha256: Option<String>,
    pub(crate) from: Option<u32>,
    pub(crate) to: Option<u32>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Input {
    pub(crate) run_id: String,
    #[serde(default)]
    pub(crate) turn_id: Option<String>,
    pub(crate) project_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) grant_id: String,
    pub(crate) question: String,
    pub(crate) references: Vec<Reference>,
    pub(crate) model_id: String,
    pub(crate) effort: String,
    #[serde(default)]
    pub(crate) renderer_queue_wait_ms: Option<u64>,
}

fn question_prompt(input: &Input, manifest: &snapshot::SnapshotManifest) -> Result<String, String> {
    if Uuid::parse_str(&input.run_id).is_err()
        || input
            .turn_id
            .as_deref()
            .is_some_and(|turn_id| Uuid::parse_str(turn_id).is_err())
        || input.model_id.trim().is_empty()
        || input.model_id.len() > 200
        || input.effort.trim().is_empty()
        || input.effort.len() > 32
        || input.question.trim().is_empty()
        || input.question.len() > 12000
        || input.references.len() > 12
        || input.project_id != manifest.summary.project_id
        || input.snapshot_id != manifest.summary.snapshot_id
    {
        return Err("The workspace question or Codex settings are invalid.".into());
    }
    for reference in &input.references {
        let file = manifest
            .files
            .iter()
            .find(|f| f.path == reference.path && context::exclusion(f).is_none())
            .ok_or("An attached path is not available in this captured snapshot.")?;
        if reference.sha256.as_ref().is_some_and(|h| h != &file.sha256)
            || reference.from.is_some() != reference.to.is_some()
            || reference
                .from
                .zip(reference.to)
                .is_some_and(|(from, to)| from == 0 || from > to)
        {
            return Err("An attached code reference changed. Open it again before asking.".into());
        }
    }
    let prompt=serde_json::to_string(&json!({"question":input.question,"snapshotId":input.snapshot_id,"startingPoints":input.references,
        "evidencePolicy":crate::evidence_ledger::POLICY,"scope":"captured implementation; use tools to investigate; starting points are not a source selection"})).map_err(|_|"The question could not be encoded.")?;
    Ok(prompt)
}

struct Owner {
    manager: codex_agent::CodexAgentManager,
    run_id: String,
    armed: bool,
}
impl Drop for Owner {
    fn drop(&mut self) {
        if self.armed {
            self.manager.cancel(&self.run_id);
        }
    }
}

pub(crate) mod jobs;
pub(crate) mod progressive;

#[cfg(test)]
mod tests;
