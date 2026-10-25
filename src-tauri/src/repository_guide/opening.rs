//! The opening is its own bounded slide operation, after the outline is saved.

use super::*;
use serde_json::json;

const SCOPED_POLICY: &str = "first-transition-with-separate-learning-intent-v3";
pub(super) const LEARNING_QUESTION: &str = "How does the repository's real entry point accept its input, make the first consequential decision, and return or propagate that transition's result?";

pub(super) fn context_sha256(job: &Record) -> Result<String, String> {
    let opening = job
        .opening
        .as_ref()
        .ok_or("The guide has no opening owner.")?;
    let bytes = serde_json::to_vec(&json!({
        "policy":SCOPED_POLICY,"guideId":job.id,"runId":opening.run_id,
        "projectId":job.project_id,"snapshotId":job.snapshot_id,
        "guideQuestion":job.question,"openingQuestion":LEARNING_QUESTION,
        "model":job.model_id,"effort":job.reasoning_effort,
    }))
    .map_err(|_| "The opening learning intent could not be encoded.")?;
    Ok(context::digest(&bytes))
}

pub(super) fn question(job: &Record) -> Result<&str, String> {
    let opening = job
        .opening
        .as_ref()
        .ok_or("The guide has no opening owner.")?;
    if opening.question != LEARNING_QUESTION
        || job.opening_context_sha256.as_deref() != Some(context_sha256(job)?.as_str())
    {
        return Err("The guide opening changed its owner or learning intent.".into());
    }
    Ok(&opening.question)
}

pub(super) fn starting_ranges(job: &Record) -> Result<&[context::CodeRange], String> {
    let plan = job.plan.as_ref().ok_or("The guide has no outline.")?;
    let ranges = plan.opening_ranges.as_slice();
    if ranges.is_empty() {
        Err("The guide outline has no opening lead.".into())
    } else {
        Ok(ranges)
    }
}

/// Finalize existing publication receipts only. The worker has already joined;
/// source replay and adoption happen outside native operation ownership.
pub(super) fn retain_failure(storage: &Path, job: &Record) -> Result<(), String> {
    let Some(opening) = &job.opening else {
        return Ok(());
    };
    let answer = if reading::exists(storage, &opening.run_id)? {
        Some(
            reading::scoped(storage, &opening.run_id, &job.project_id, &job.snapshot_id)?.summary(),
        )
    } else {
        None
    };
    store::update(storage, &job.id, |current| {
        if current
            .active_run_id
            .as_ref()
            .is_some_and(|run| run != &opening.run_id)
        {
            return Err(
                "The guide has a newer investigation owner. Saved work was preserved.".into(),
            );
        }
        let saved = current
            .opening
            .as_mut()
            .ok_or("The guide opening owner changed.")?;
        if saved.run_id != opening.run_id
            || saved.question != opening.question
            || current.question != job.question
            || current.opening_context_sha256 != job.opening_context_sha256
        {
            return Err("The guide opening scope changed. Saved work was preserved.".into());
        }
        saved.summary = answer;
        if current.status != Status::Stopped {
            current.status = Status::Partial;
        }
        current.active_run_id = None;
        Ok(())
    })?;
    Ok(())
}

#[cfg(test)]
pub(super) mod tests;
