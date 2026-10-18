//! One explicitly requested follow-up to the already-spent opening operation.
//! The outline and original publication ledger remain immutable history.

use super::*;

pub(super) fn latest(record: &Record) -> Option<&Attempt> {
    record
        .opening_continuations
        .last()
        .or(record.opening.as_ref())
}

fn remaining(record: &Record) -> bool {
    record.plan.is_some()
        && record.opening_continuations.is_empty()
        && record
            .opening
            .as_ref()
            .and_then(|a| a.summary.as_ref())
            .is_some_and(|s| !matches!(s.outcome, Outcome::Working | Outcome::Complete))
}

pub(super) fn remaining_operations(record: &Record) -> usize {
    if record.plan.is_none() {
        return 0;
    }
    let pending = |unit: &Unit| !unit_complete(unit) && unit.attempts.len() < MAX_ATTEMPTS;
    let runnable = record.units.iter().enumerate().any(|(index, unit)| {
        pending(unit) && (independent_research(record) || prerequisite_complete(record, index))
    });
    // A legacy prerequisite may become complete during this same continuation.
    // Consent covers that upper bound as well as units eligible at admission.
    usize::from(remaining(record))
        + if runnable {
            record.units.iter().filter(|unit| pending(unit)).count()
        } else {
            0
        }
}

pub(super) fn eligible(record: &Record) -> bool {
    record.schema_version == 5
        && record.contract == CONTRACT
        && record.status == Status::Building
        && record.active_run_id.is_none()
        && remaining(record)
}

fn intent(record: &Record) -> Result<workspace_generation::progressive::GuideIntent, String> {
    let opening = record.opening.as_ref().ok_or("The guide has no opening.")?;
    let summary = opening
        .summary
        .as_ref()
        .ok_or("The opening has not been reconciled.")?;
    let plan = record
        .plan
        .as_ref()
        .ok_or("The guide has no saved outline.")?;
    Ok(workspace_generation::progressive::GuideIntent {
        guide_id: record.id.clone(),
        question: record.question.clone(),
        unit_key: "opening".into(),
        unit_number: 0,
        unit_count: plan.units.len(),
        research_boundaries: plan.omissions.clone(),
        opening_continuation: Some(workspace_generation::progressive::OpeningContinuation {
            previous_run_id: opening.run_id.clone(),
            scoped: separate_opening_intent(record),
            incomplete_topics: summary.incomplete_topics.clone(),
        }),
    })
}

fn begin(storage: &Path, expected: &Record, run_id: &str) -> Result<Record, String> {
    let question = opening::question(expected)?.to_owned();
    store::update(storage, &expected.id, |current| {
        if current.revision != expected.revision || !eligible(current) {
            return Err("The guide changed before its opening continuation started. No operation was dispatched.".into());
        }
        current.active_run_id = Some(run_id.into());
        current.opening_continuations.push(Attempt {
            run_id: run_id.into(),
            question,
            summary: None,
        });
        Ok(())
    })
}

fn finish(storage: &Path, expected: &Record, answer: Summary) -> Result<Record, String> {
    let run_id = answer.run_id.clone();
    store::update(storage, &expected.id, |current| {
        if current.active_run_id.as_deref() != Some(run_id.as_str())
            || (current.revision != expected.revision && current.status != Status::Stopped)
        {
            return Err(
                "The opening continuation owner changed. Saved sections were preserved.".into(),
            );
        }
        let attempt = current
            .opening_continuations
            .last_mut()
            .ok_or("The opening continuation is unavailable.")?;
        if attempt.run_id != run_id
            || attempt.question != answer.question
            || attempt.summary.is_some()
        {
            return Err(
                "The opening continuation result is stale. Saved sections were preserved.".into(),
            );
        }
        if answer.outcome == Outcome::Stopped {
            current.status = Status::Stopped;
        }
        attempt.summary = Some(answer);
        current.active_run_id = None;
        Ok(())
    })
}

pub(super) fn run(
    storage: &Path,
    record: &Record,
    manager: &codex_agent::CodexAgentManager,
    jobs: RepositoryContextJobs,
    journal: &recovery::InterruptionJournal,
    progress: &impl Fn(codex_agent::CodexProgress),
    notify: Arc<dyn Fn(SectionNotice) + Send + Sync>,
    adopt: &impl Fn(&SectionNotice) -> Result<(), String>,
    changed: &impl Fn(&GuideSummary),
) -> Result<Record, String> {
    checked_manifest(storage, record)?;
    let runtime = manager.review_runtime(storage)?;
    workspace::check_retrieval_grant(
        storage,
        &record.grant_id,
        &record.project_id,
        &record.snapshot_id,
        &runtime,
    )?;
    let intent = intent(record)?;
    let run_id = Uuid::new_v4().to_string();
    let record = begin(storage, record, &run_id)?;
    changed(&summary(storage, &record)?);
    let result = workspace_generation::progressive::run_with_starting_ranges(
        storage,
        Input {
            run_id: run_id.clone(),
            project_id: record.project_id.clone(),
            snapshot_id: record.snapshot_id.clone(),
            grant_id: record.grant_id.clone(),
            question: opening::question(&record)?.into(),
            references: vec![],
            model_id: "gpt-6-luna".into(),
            effort: "max".into(),
        },
        &[],
        Some(&intent),
        manager,
        jobs,
        journal,
        progress,
        notify,
    );
    let error = result.as_ref().err().cloned();
    let answer = result.or_else(|error| {
        reading::scoped(storage, &run_id, &record.project_id, &record.snapshot_id)
            .map(|answer| answer.summary())
            .map_err(|_| error)
    })?;
    for section in &answer.sections {
        adopt(section)?;
    }
    let record = finish(storage, &record, answer)?;
    changed(&summary(storage, &record)?);
    if let Some(error) = error {
        return Err(error);
    }
    let answer = reading::scoped(storage, &run_id, &record.project_id, &record.snapshot_id)?;
    if answer.outcome != Outcome::Stopped
        && (answer.has_recorded_failure()
            || !matches!(answer.outcome, Outcome::Complete | Outcome::Partial))
    {
        return Err("The opening continuation did not finish safely. Saved sections remain available; no later investigation was started.".into());
    }
    Ok(record)
}

#[cfg(test)]
mod tests;
