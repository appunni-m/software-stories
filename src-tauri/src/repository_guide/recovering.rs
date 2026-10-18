//! Recover inactive investigations without holding Stop across source replay.

use super::*;

/// Retire a failed build only after its worker has released native ownership.
/// Preserve spent attempts and terminal receipts; replay no investigation.
pub(super) fn settle_failed_build(
    storage: &Path,
    id: &str,
    manager: &codex_agent::CodexAgentManager,
) -> Result<Record, String> {
    let record = store::read(storage, id)?;
    reconcile_with_reader(storage, &record, manager, Status::Partial, |attempt| {
        observe_attempt(storage, &record, attempt)
    })
}

pub(super) fn reconcile(
    storage: &Path,
    record: &Record,
    manager: &codex_agent::CodexAgentManager,
) -> Result<Record, String> {
    reconcile_with_reader(storage, record, manager, Status::Interrupted, |attempt| {
        observe_attempt(storage, record, attempt)
    })
}

fn observe_attempt(
    storage: &Path,
    record: &Record,
    attempt: &Attempt,
) -> Result<Option<reading::Observation>, String> {
    if !reading::exists(storage, &attempt.run_id)? {
        return Ok(None);
    }
    reading::observe(storage, &attempt.run_id, &record.project_id, &record.snapshot_id).map(Some)
}

fn reconcile_with_reader(
    storage: &Path,
    record: &Record,
    manager: &codex_agent::CodexAgentManager,
    terminal_status: Status,
    mut read: impl FnMut(&Attempt) -> Result<Option<reading::Observation>, String>,
) -> Result<Record, String> {
    let mut next = record.clone();
    for attempt in next
        .opening
        .iter_mut()
        .chain(&mut next.opening_continuations)
        .chain(next.units.iter_mut().flat_map(|unit| &mut unit.attempts))
    {
        if attempt
            .summary
            .as_ref()
            .is_some_and(|s| s.outcome != Outcome::Working)
        {
            continue;
        }
        let mut recovered = None;
        for _ in 0..3 {
            // Check source/checkpoints before entering native ownership order.
            let observed = read(attempt)?;
            if observed
                .as_ref()
                .is_some_and(|answer| answer.question() != attempt.question)
            {
                return Err("A guide investigation question changed.".into());
            }
            let current = manager.with_operation_state(&attempt.run_id, |active| {
                if active {
                    return Err("This guide still has an owned Codex investigation. Wait for it to finish or stop it.".into());
                }
                match observed {
                    Some(answer) => reading::reconcile_observed(storage, answer, false)
                        .map(|answer| answer.map(Some)),
                    // A missing observation cannot hide a receipt published
                    // before ownership was checked. Re-read locally instead.
                    None if reading::exists(storage, &attempt.run_id)? => Ok(None),
                    None => Ok(Some(None)),
                }
            })?;
            if let Some(answer) = current {
                recovered = Some(answer);
                break;
            }
        }
        let recovered = recovered.ok_or(
            "The saved guide answer changed while opening. Open it again; nothing was regenerated.",
        )?;
        attempt.summary = Some(recovered.unwrap_or_else(|| Summary {
            run_id: attempt.run_id.clone(),
            project_id: record.project_id.clone(),
            snapshot_id: record.snapshot_id.clone(),
            question: attempt.question.clone(),
            outcome: Outcome::Interrupted,
            sections: vec![],
            incomplete_topics: vec![
                "The investigation stopped before a saved answer was available.".into(),
            ],
        }));
    }
    next.active_run_id = None;
    if matches!(next.status, Status::Planning | Status::Building) {
        next.status = terminal_status;
    }
    let save = || store::update_if_changed(storage, &record.id, |current| {
        if current.revision != record.revision {
            // Stop wins without copying any stale attempts, plan or owner.
            if current.status == Status::Stopped {
                return Ok(false);
            }
            return Err(
                "The guide changed while opening. Open it again; saved progress was preserved."
                    .into(),
            );
        }
        if current.status == Status::Stopped {
            next.status = Status::Stopped;
        }
        // Compare after checking the persisted revision: a no-op open must
        // preserve the original bytes, including the revision later archived
        // by an explicit continuation.
        let saved = serde_json::to_value(&*current)
            .map_err(|_| "The saved guide could not be compared. Existing work was preserved.")?;
        let recovered = serde_json::to_value(&next).map_err(
            |_| "The recovered guide could not be compared. Existing work was preserved.",
        )?;
        if saved == recovered {
            return Ok(false);
        }
        *current = next;
        Ok(true)
    });
    // A durable terminal receipt can precede worker retirement. Even when all
    // summaries were already terminal, keep ownership through this last write.
    match record.active_run_id.as_deref() {
        Some(run) => manager.with_operation_state(run, |active| {
            if active {
                return Err("This guide still has an owned Codex investigation. Wait for it to finish or stop it.".into());
            }
            save()
        }),
        None => save(),
    }
}

#[cfg(test)]
mod tests;
