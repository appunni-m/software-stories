//! Local-only recovery commands. No connection, grant, model or inference path.

use crate::{
    app_data_directory, codex_agent::CodexAgentManager, evidence_ledger::attempts::management,
    local_read,
};
use tauri::{AppHandle, State};

fn when_idle<T>(
    manager: &CodexAgentManager,
    work: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    manager.with_generation_state(|busy| {
        if busy {
            return Err(
                "Finish or stop the current explanation before removing saved attempts.".into(),
            );
        }
        work()
    })
}

#[tauri::command]
pub(crate) async fn list_rejected_answers(
    app: AppHandle,
    manager: State<'_, CodexAgentManager>,
) -> Result<management::Inventory, String> {
    let storage = app_data_directory(&app)?;
    let manager = manager.inner().clone();
    local_read::run(move || {
        let mut inventory = management::list(&storage)?;
        manager.with_generation_state(|busy| {
            inventory.busy = busy;
            Ok(inventory)
        })
    })
    .await
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn read_rejected_answer(
    app: AppHandle,
    entry_id: String,
    review_token: String,
) -> Result<management::Review, String> {
    let storage = app_data_directory(&app)?;
    local_read::run(move || management::review(&storage, &entry_id, &review_token)).await
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn trash_rejected_answer(
    app: AppHandle,
    manager: State<'_, CodexAgentManager>,
    entry_id: String,
    review_token: String,
) -> Result<(), String> {
    let storage = app_data_directory(&app)?;
    let manager = manager.inner().clone();
    local_read::run(move || {
        when_idle(&manager, || {
            management::move_to_trash(&storage, &entry_id, &review_token)
        })
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codex_identity::CodexReviewRuntime;

    #[test]
    fn active_generation_and_lingering_answer_workers_block_recovery_mutation() {
        let runtime = CodexReviewRuntime {
            connection_id: uuid::Uuid::new_v4().to_string(),
            executable_path: "/fixture/codex".into(),
            executable_sha256: "a".repeat(64),
            account_identity_sha256: "b".repeat(64),
            account_revision: 0,
        };
        let (manager, _owner, _) = crate::codex_agent::section_owner_for_test(
            &uuid::Uuid::new_v4().to_string(),
            &"d".repeat(64),
            &runtime,
        );
        assert!(
            when_idle(&manager, || -> Result<(), String> {
                panic!("active generation")
            })
            .is_err()
        );
        let idle = CodexAgentManager::default();
        let worker = idle.hold_answer_worker("fixture-worker").unwrap();
        assert!(when_idle(&idle, || -> Result<(), String> { panic!("answer worker") }).is_err());
        drop(worker);
        assert_eq!(when_idle(&idle, || Ok(7)).unwrap(), 7);
    }

    #[test]
    fn recovery_serializes_with_a_new_answer_worker() {
        use std::{sync::mpsc, time::Duration};
        let manager = CodexAgentManager::default();
        let other = manager.clone();
        let (entered, started) = mpsc::channel();
        let (released, done) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            started.recv().unwrap();
            let held = other.hold_answer_worker("fixture-new-worker").unwrap();
            released.send(()).unwrap();
            held
        });
        when_idle(&manager, || {
            entered.send(()).unwrap();
            assert!(done.recv_timeout(Duration::from_millis(30)).is_err());
            Ok(())
        })
        .unwrap();
        done.recv_timeout(Duration::from_secs(2)).unwrap();
        drop(worker.join().unwrap());
    }
}
