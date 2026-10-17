//! Recover inactive investigations without holding Stop across source replay.

use super::*;

pub(super) fn reconcile(
    storage: &Path,
    record: &Record,
    manager: &codex_agent::CodexAgentManager,
) -> Result<Record, String> {
    reconcile_with_reader(storage, record, manager, |attempt| {
        if !reading::exists(storage, &attempt.run_id)? {
            return Ok(None);
        }
        reading::observe(
            storage,
            &attempt.run_id,
            &record.project_id,
            &record.snapshot_id,
        )
        .map(Some)
    })
}

fn reconcile_with_reader(
    storage: &Path,
    record: &Record,
    manager: &codex_agent::CodexAgentManager,
    mut read: impl FnMut(&Attempt) -> Result<Option<reading::Observation>, String>,
) -> Result<Record, String> {
    let mut next = record.clone();
    for attempt in next
        .opening
        .iter_mut()
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
        next.status = Status::Interrupted;
    }
    store::update(storage, &record.id, |current| {
        if current.revision != record.revision {
            // Stop wins without copying any stale attempts, plan or owner.
            if current.status == Status::Stopped {
                return Ok(());
            }
            return Err(
                "The guide changed while opening. Open it again; saved progress was preserved."
                    .into(),
            );
        }
        if current.status == Status::Stopped {
            next.status = Status::Stopped;
        }
        *current = next;
        Ok(())
    })
}

#[cfg(test)]
mod tests;
