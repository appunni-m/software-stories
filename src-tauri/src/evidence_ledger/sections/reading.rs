//! Read and adopt independently verified sections without reconnecting an agent.
//! Saved records never recreate an active publisher or grant source access.

use super::*;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SectionIdentity {
    pub(crate) section_key: String,
    pub(crate) generation_id: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Summary {
    pub(crate) run_id: String,
    pub(crate) project_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) question: String,
    pub(crate) outcome: Outcome,
    pub(crate) sections: Vec<SectionNotice>,
    pub(crate) incomplete_topics: Vec<String>,
}

impl Answer {
    pub(super) fn latest_identities(&self) -> Vec<SectionIdentity> {
        let mut latest = BTreeMap::new();
        for section in &self.sections {
            latest.insert(section.order, section);
        }
        latest
            .values()
            .map(|section| SectionIdentity {
                section_key: section.key.clone(),
                generation_id: section.preview.generation_id.clone(),
            })
            .collect()
    }

    pub(crate) fn summary(&self) -> Summary {
        Summary {
            run_id: self.binding.operation_id.clone(),
            project_id: self.binding.project_id.clone(),
            snapshot_id: self.binding.snapshot_id.clone(),
            question: self.question.clone(),
            outcome: self.outcome.clone(),
            incomplete_topics: self.incomplete_topics.clone(),
            sections: self
                .sections
                .iter()
                .map(|section| SectionNotice {
                    run_id: self.binding.operation_id.clone(),
                    project_id: self.binding.project_id.clone(),
                    snapshot_id: self.binding.snapshot_id.clone(),
                    key: section.key.clone(),
                    order: section.order,
                    generation_id: section.preview.generation_id.clone(),
                    revision_id: section.preview.revision.revision_id.clone(),
                    supersedes: section.supersedes.clone(),
                    context_elapsed_ms: section.published_elapsed_ms,
                    run_elapsed_ms: section.published_run_elapsed_ms,
                })
                .collect(),
        }
    }
}

pub(crate) fn scoped(
    storage: &Path,
    run: &str,
    project: &str,
    snapshot: &str,
) -> Result<Answer, String> {
    let answer = inspect(storage, run)?;
    if answer.binding.project_id != project || answer.binding.snapshot_id != snapshot {
        return Err("This answer belongs to another repository or snapshot.".into());
    }
    Ok(answer)
}

/// Cached guide metadata may cover an explicitly removed receipt, never a
/// damaged existing record whose independent evidence replay has failed.
pub(crate) fn exists(storage: &Path, run: &str) -> Result<bool, String> {
    match std::fs::symlink_metadata(record_path(storage, run)?) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err("The saved answer record could not be inspected.".into()),
    }
}

/// Caller holds the manager's active-operation lock, or has joined the worker.
/// Terminal recovery updates only outcome metadata; it cannot publish content.
pub(crate) fn interrupt(storage: &Path, run: &str, outcome: Outcome) -> Result<Summary, String> {
    interrupt_with_failure(storage, run, outcome, None)
}

/// Called by the joined worker with a fixed classification of its actual error.
/// A historical terminal record is preserved; its missing cause stays unknown.
pub(crate) fn interrupt_with_failure(
    storage: &Path,
    run: &str,
    outcome: Outcome,
    failure: Option<FailureCode>,
) -> Result<Summary, String> {
    if !matches!(
        outcome,
        Outcome::Stopped | Outcome::Interrupted | Outcome::Partial | Outcome::Failed
    ) || (failure == Some(FailureCode::Stopped) && outcome != Outcome::Stopped) {
        return Err("The answer recovery outcome is invalid.".into());
    }
    let mut answer = inspect(storage, run)?;
    if answer.outcome != Outcome::Working {
        return Ok(answer.summary());
    }
    let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
    let path = record_path(storage, run)?;
    if read_bytes(&path)? != encode_answer(&answer)? {
        return Err("The saved answer changed during recovery. It was preserved.".into());
    }
    answer.outcome = match outcome {
        Outcome::Failed | Outcome::Partial if answer.sections.is_empty() => Outcome::Failed,
        Outcome::Failed | Outcome::Partial => Outcome::Partial,
        other => other,
    };
    answer.failure_code = failure;
    if answer.outcome == Outcome::Partial {
        answer.incomplete_topics = vec!["The investigation ended before all requested details could be completed. Only the published sections are available.".into()];
    }
    codex_agent::atomic_private_write(&path, &encode_answer(&answer)?)?;
    Ok(answer.summary())
}

/// Call under the shared chapter write lock. The same immutable revision ID is
/// reused on retry after a lost reply, including after the answer has stopped.
pub(crate) fn adopt(
    storage: &Path,
    run: &str,
    project: &str,
    snapshot: &str,
    generation: &str,
) -> Result<chapters::ChapterRevision, String> {
    let answer = scoped(storage, run, project, snapshot)?;
    let revision = answer
        .sections
        .iter()
        .find(|s| s.preview.generation_id == generation)
        .ok_or("This section was not published by this answer.")?
        .preview
        .revision
        .clone();
    let existing = crate::chapter_store::list_for_snapshot(storage, project, snapshot)?;
    if let Some(saved) = existing
        .into_iter()
        .find(|r| r.revision_id == revision.revision_id)
    {
        let mut expected = revision.clone();
        expected.revision_number = saved.revision_number;
        if serde_json::to_vec(&saved).map_err(|_| "The saved section could not be encoded.")?
            != serde_json::to_vec(&expected)
                .map_err(|_| "The expected section could not be encoded.")?
        {
            return Err("The saved section's immutable revision changed.".into());
        }
        return Ok(saved);
    }
    crate::chapter_store::save_new_draft(storage, revision)
}

/// Explicit retention action. All published revisions must already exist in
/// the chapter library; removing the bounded investigation receipt cannot
/// remove a reader's drafts. Never called automatically to make storage room.
pub(crate) fn remove(
    storage: &Path,
    run: &str,
    project: &str,
    snapshot: &str,
) -> Result<(), String> {
    let answer = scoped(storage, run, project, snapshot)?;
    if answer.outcome == Outcome::Working {
        return Err("Stop this answer before removing its investigation record.".into());
    }
    let saved = crate::chapter_store::list_for_snapshot(storage, project, snapshot)?;
    for section in &answer.sections {
        if !saved.iter().any(|r| {
            let mut expected = section.preview.revision.clone();
            expected.revision_number = r.revision_number;
            r == &expected
        }) {
            return Err(
                "Save all published sections before removing this investigation record.".into(),
            );
        }
    }
    let _guard = STORE.lock().map_err(|_| "Answer storage is unavailable.")?;
    let path = record_path(storage, run)?;
    if read_bytes(&path)? != encode_answer(&answer)? {
        return Err("The saved answer changed; its record was preserved.".into());
    }
    fs::remove_file(path).map_err(|_| "The investigation record could not be removed.".to_owned())
}

pub(crate) fn list(storage: &Path, project: &str, snapshot: &str) -> Result<Vec<Summary>, String> {
    Ok(list_all(storage)?
        .into_iter()
        .filter(|s| s.project_id == project && s.snapshot_id == snapshot)
        .collect())
}

pub(crate) fn list_all(storage: &Path) -> Result<Vec<Summary>, String> {
    let paths = fs::read_dir(directory(storage)?)
        .map_err(|_| "Saved answers could not be inspected.")?
        .take(MAX_DIRECTORY_ENTRIES + 1)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Saved answers could not be inspected.")?;
    if paths.len() > MAX_DIRECTORY_ENTRIES {
        return Err("The saved answer directory exceeds its limit.".into());
    }
    let mut result = vec![];
    for entry in paths {
        let path = entry.path();
        if path.extension().is_some_and(|v| v == "json") {
            let run = path
                .file_stem()
                .and_then(|v| v.to_str())
                .ok_or("Invalid saved answer identity.")?;
            let answer = inspect(storage, run)?;
            result.push(answer.summary());
        }
    }
    result.sort_by(|a, b| a.run_id.cmp(&b.run_id));
    Ok(result)
}
