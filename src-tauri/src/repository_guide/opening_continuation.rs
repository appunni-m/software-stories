//! Fresh bounded retries for an incomplete opening slide. The outline and all
//! earlier publication ledgers remain immutable history.

use super::*;

pub(super) fn latest(record: &Record) -> Option<&Attempt> {
    record
        .opening_continuations
        .last()
        .or(record.opening.as_ref())
}

fn incomplete(record: &Record) -> bool {
    record.plan.is_some()
        && latest(record)
            .and_then(|attempt| attempt.summary.as_ref())
            .is_some_and(|summary| !matches!(summary.outcome, Outcome::Working | Outcome::Complete))
}

fn automatic_retry_used(record: &Record) -> bool {
    record
        .opening_continuations
        .iter()
        .any(|attempt| attempt.automatic_retry)
}

fn explicit_retry_used(record: &Record) -> bool {
    record
        .opening_continuations
        .iter()
        .any(|attempt| !attempt.automatic_retry)
}

pub(super) fn automatic_retry_eligible(record: &Record) -> bool {
    record.status != Status::Stopped
        && incomplete(record)
        && !automatic_retry_used(record)
        && latest(record)
            .and_then(|attempt| attempt.summary.as_ref())
            .is_some_and(|summary| summary.sections.is_empty())
}

fn explicit_retry_eligible(record: &Record) -> bool {
    incomplete(record) && !automatic_retry_eligible(record) && !explicit_retry_used(record)
}

pub(super) fn remaining_operations(record: &Record) -> usize {
    if record.plan.is_none() {
        return usize::from(
            record.status != Status::Stopped
                && record.active_run_id.is_none()
                && record.planning_retry_count < 2,
        );
    }
    let pending = |unit: &Unit| !unit_complete(unit) && unit.attempts.len() < MAX_ATTEMPTS;
    let initial_opening = record
        .opening
        .as_ref()
        .is_some_and(|attempt| attempt.summary.is_none());
    usize::from(initial_opening)
        + usize::from(automatic_retry_eligible(record))
        + usize::from(explicit_retry_eligible(record))
        + record.units.iter().filter(|unit| pending(unit)).count()
}

pub(super) fn eligible(record: &Record) -> bool {
    record.schema_version == RECORD_SCHEMA_VERSION
        && record.contract == CONTRACT
        && record.status == Status::Building
        && record.active_run_id.is_none()
        && explicit_retry_eligible(record)
}

fn intent(record: &Record) -> Result<workspace_generation::progressive::GuideIntent, String> {
    record.opening.as_ref().ok_or("The guide has no opening.")?;
    let previous = latest(record).ok_or("The guide has no previous opening attempt.")?;
    let summary = previous
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
            previous_run_id: previous.run_id.clone(),
            incomplete_topics: summary.incomplete_topics.clone(),
        }),
    })
}

fn begin(storage: &Path, expected: &Record, run_id: &str) -> Result<Record, String> {
    begin_with_retry(storage, expected, run_id, false)
}

fn begin_with_retry(
    storage: &Path,
    expected: &Record,
    run_id: &str,
    automatic_retry: bool,
) -> Result<Record, String> {
    let question = opening::question(expected)?.to_owned();
    store::update(storage, &expected.id, |current| {
        let permitted = if automatic_retry {
            automatic_retry_eligible(current)
        } else {
            eligible(current)
        };
        if current.revision != expected.revision
            || current.active_run_id.is_some()
            || current.status != Status::Building
            || !permitted
        {
            return Err("The guide changed before its opening continuation started. No operation was dispatched.".into());
        }
        current.active_run_id = Some(run_id.into());
        current.opening_continuations.push(Attempt {
            run_id: run_id.into(),
            question,
            automatic_retry,
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
    automatic_retry: bool,
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
    let record = begin_with_retry(storage, record, &run_id, automatic_retry)?;
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
            model_id: record.model_id.clone(),
            effort: record.reasoning_effort.clone(),
            renderer_queue_wait_ms: None,
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
    let answer = match result {
        Ok(answer) => answer,
        Err(error) => {
            #[cfg(test)]
            eprintln!("GUIDE_OPENING_CONTINUATION_FAILURE {run_id}: {error}");
            if reading::exists(storage, &run_id)? {
                reading::scoped(storage, &run_id, &record.project_id, &record.snapshot_id)?
                    .summary()
            } else {
                synthetic_failed_slide(
                    &run_id,
                    &record.project_id,
                    &record.snapshot_id,
                    &opening::question(&record)?,
                )
            }
        }
    };
    for section in &answer.sections {
        adopt(section)?;
    }
    let record = finish(storage, &record, answer)?;
    changed(&summary(storage, &record)?);
    let saved_answer = if reading::exists(storage, &run_id)? {
        Some(reading::scoped(
            storage,
            &run_id,
            &record.project_id,
            &record.snapshot_id,
        )?)
    } else {
        None
    };
    if operation_requires_halt(error.as_deref(), saved_answer.as_ref(), &record.status) {
        if record.status == Status::Stopped {
            return Ok(record);
        }
        return Err(error.unwrap_or_else(|| {
            "The opening continuation could not proceed safely. Saved sections remain available.".into()
        }));
    }
    if let Some(answer) = saved_answer.as_ref() {
        check_research_outcome(&record, answer)?;
    }
    Ok(record)
}

#[cfg(test)]
mod tests;
