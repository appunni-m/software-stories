//! One accepted reading step becomes one question-focused agent chapter.
//! Plan acceptance is not chapter acceptance, agent consent or source coverage.

use crate::{repository_plan, repository_plan_history};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlannedChapterTarget {
    pub revision_id: String,
    pub record_sha256: String,
    pub step_id: String,
}

impl PlannedChapterTarget {
    pub(crate) fn valid_shape(&self) -> bool {
        repository_plan::canonical_uuid_id(&self.revision_id, "plan-revision")
            && self.record_sha256.len() == 64
            && self
                .record_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            && !self.step_id.is_empty()
            && self.step_id.len() <= 256
            && !self.step_id.chars().any(char::is_control)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlannedChapterPrerequisite {
    step_id: String,
    question: String,
}

/// Native-derived context, included in the exact agent review and interruption
/// journal. All prose remains untrusted data; only the captured source is evidence.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlannedChapterScope {
    pub schema_version: u32,
    pub target: PlannedChapterTarget,
    pub snapshot_id: String,
    pub primary_path: String,
    pub selected_paths: Vec<String>,
    pub plan_question: String,
    pub question: String,
    pub prerequisites: Vec<PlannedChapterPrerequisite>,
    pub plan_context: repository_plan::RepositoryPlanReviewScope,
}

impl PlannedChapterScope {
    /// Bounded metadata only. Accepted-record replay remains authoritative.
    pub(crate) fn valid_recorded_shape(&self) -> bool {
        let text = |value: &str, max: usize| {
            !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
        };
        let paths = self.selected_paths.iter().collect::<BTreeSet<_>>();
        let prerequisites = self
            .prerequisites
            .iter()
            .map(|item| &item.step_id)
            .collect::<BTreeSet<_>>();
        self.schema_version == 1
            && self.target.valid_shape()
            && text(&self.snapshot_id, 256)
            && text(&self.primary_path, 4096)
            && text(&self.question, 720)
            && self.plan_question == self.plan_context.question
            && self.plan_context.valid_recorded_shape()
            && !paths.is_empty()
            && paths.len() <= 12
            && paths.len() == self.selected_paths.len()
            && paths.contains(&self.primary_path)
            && self.selected_paths.iter().all(|path| text(path, 4096))
            && self.prerequisites.len() <= 23
            && prerequisites.len() == self.prerequisites.len()
            && self.prerequisites.iter().all(|item| {
                item.step_id != self.target.step_id
                    && text(&item.step_id, 256)
                    && text(&item.question, 720)
            })
    }
}

/// Replays the accepted original proposal and its immutable evidence. Current
/// head changes do not silently substitute another revision or reading step.
pub(crate) fn prepare(
    storage: &Path,
    project_id: &str,
    target: &PlannedChapterTarget,
) -> Result<PlannedChapterScope, String> {
    if !target.valid_shape() {
        return Err("Open the exact accepted plan step before preparing its chapter.".to_owned());
    }
    let record = repository_plan_history::load_accepted_record(
        storage,
        project_id,
        &target.revision_id,
        &target.record_sha256,
    )?;
    let plan = &record.preview.plan;
    let step = plan
        .steps
        .iter()
        .find(|step| step.step_id == target.step_id)
        .ok_or("This step is not in the reviewed accepted plan. Open that revision again.")?;
    let selected_paths = std::iter::once(step.primary_path.clone())
        .chain(step.claim.anchors.iter().map(|anchor| anchor.path.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if selected_paths.is_empty()
        || selected_paths.len() > 12
        || selected_paths
            .iter()
            .any(|path| !record.selected_paths.contains(path))
    {
        return Err(
            "The accepted step's captured source is unavailable or exceeds the chapter limit."
                .to_owned(),
        );
    }
    let prerequisites = step
        .prerequisite_step_ids
        .iter()
        .map(|id| {
            let prerequisite = plan
                .steps
                .iter()
                .find(|item| item.step_id == *id)
                .ok_or("An accepted reading prerequisite is unavailable.")?;
            Ok(PlannedChapterPrerequisite {
                step_id: id.clone(),
                question: prerequisite.question.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(PlannedChapterScope {
        schema_version: 1,
        target: target.clone(),
        snapshot_id: plan.snapshot_id.clone(),
        primary_path: step.primary_path.clone(),
        selected_paths,
        plan_question: plan.question.clone(),
        question: step.question.clone(),
        prerequisites,
        plan_context: record.scope.clone(),
    })
}
