//! Active-project native entry point for structural lookup. Agent authorization is separate.

use crate::{
    ProjectSessions, app_data_directory, index, repository_context_commands::RepositoryContextJobs,
    repository_structure as structure, snapshot,
};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Operation {
    Definitions {
        query: String,
    },
    Relationships {
        id: String,
        direction: structure::Direction,
    },
    Map {},
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Input {
    request_id: String,
    project_id: String,
    snapshot_id: String,
    scope: Option<String>,
    limit: usize,
    #[serde(default)]
    offset: usize,
    operation: Operation,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "result", rename_all = "camelCase")]
pub(crate) enum Output {
    Definitions(structure::Page<structure::Symbol>),
    Relationships(structure::Page<structure::Relationship>),
    Map(structure::Page<structure::Module>),
}

#[tauri::command]
pub(crate) async fn inspect_repository_structure(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    jobs: State<'_, RepositoryContextJobs>,
    input: Input,
) -> Result<Output, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    let request = jobs.begin(&input.request_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let budget = request.budget();
        budget.check()?;
        let manifest =
            snapshot::load_snapshot_manifest(&storage, &input.project_id, &input.snapshot_id)?;
        let syntax = index::load_snapshot_index(&storage, &input.project_id, &input.snapshot_id)
            .unwrap_or(None);
        match input.operation {
            Operation::Definitions { query } => structure::definitions(
                &storage,
                &manifest,
                syntax.as_ref(),
                &query,
                input.scope.as_deref(),
                input.limit,
                input.offset,
                &budget,
            )
            .map(Output::Definitions),
            Operation::Relationships { id, direction } => structure::relationships(
                &storage,
                &manifest,
                syntax.as_ref(),
                &id,
                direction,
                input.scope.as_deref(),
                input.limit,
                input.offset,
                &budget,
            )
            .map(Output::Relationships),
            Operation::Map {} => structure::module_map(
                &storage,
                &manifest,
                syntax.as_ref(),
                input.scope.as_deref(),
                input.limit,
                input.offset,
                &budget,
            )
            .map(Output::Map),
        }
    })
    .await
    .map_err(|_| "Repository context stopped unexpectedly.".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(operation: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "requestId": "1ba42cd8-295d-4ed2-9eb9-f1dec1cd0e6b",
            "projectId": "/captured/project", "snapshotId": "snap-fixture",
            "limit": 8, "operation": operation
        })
    }

    #[test]
    fn structural_input_accepts_only_the_declared_operations_and_fields() {
        for operation in [
            serde_json::json!({"kind":"definitions", "query":"memory_face"}),
            serde_json::json!({"kind":"relationships", "id":"symbol", "direction":"incoming"}),
            serde_json::json!({"kind":"map"}),
        ] {
            let parsed: Input = serde_json::from_value(input(operation)).unwrap();
            assert_eq!(parsed.offset, 0);
            assert!(parsed.scope.is_none());
        }
        for operation in [
            serde_json::json!({"kind":"shell", "command":"anything"}),
            serde_json::json!({"kind":"definitions", "query":"memory_face", "contents":"injected"}),
            serde_json::json!({"kind":"relationships", "id":"symbol", "direction":"runtime"}),
            serde_json::json!({"kind":"map", "contents":"injected"}),
        ] {
            assert!(
                serde_json::from_value::<Input>(input(operation.clone())).is_err(),
                "unexpectedly accepted operation {operation}"
            );
        }
        let mut unknown = input(serde_json::json!({"kind":"map"}));
        unknown["cwd"] = serde_json::json!("/uncaptured/checkout");
        assert!(serde_json::from_value::<Input>(unknown).is_err());
    }
}
