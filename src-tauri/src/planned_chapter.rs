//! One accepted reading step becomes one question-focused agent chapter.
//! Plan acceptance is not chapter acceptance, agent consent or source coverage.

use crate::{repository_plan, repository_plan_history, repository_plan_store};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

const MAX_PLANNED_CHAPTERS: usize = 24;

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

/// The accepted route identity used to prepare its complete ordered chapter
/// batch. Step IDs and prompts remain native-derived from that immutable route.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlannedChapterBatchTarget {
    pub revision_id: String,
    pub record_sha256: String,
}

/// Native review scope for every accepted route step, in original plan order.
/// Preparing this value reads no agent runtime and starts no provider request.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlannedChapterBatchScope {
    pub schema_version: u32,
    pub project_id: String,
    pub revision_id: String,
    pub record_sha256: String,
    pub snapshot_id: String,
    pub steps: Vec<PlannedChapterScope>,
}

impl PlannedChapterBatchScope {
    /// Structural guard for the renderer's whole-route review. The accepted
    /// record remains authoritative when each provider request is rebuilt.
    pub(crate) fn valid_recorded_shape(&self) -> bool {
        let digest = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        let project = !self.project_id.trim().is_empty()
            && self.project_id.len() <= 4096
            && !self.project_id.chars().any(char::is_control);
        let snapshot = !self.snapshot_id.trim().is_empty()
            && self.snapshot_id.len() <= 256
            && !self.snapshot_id.chars().any(char::is_control);
        let mut step_ids = BTreeSet::new();
        self.schema_version == 1
            && project
            && snapshot
            && repository_plan::canonical_uuid_id(&self.revision_id, "plan-revision")
            && digest(&self.record_sha256)
            && (1..=MAX_PLANNED_CHAPTERS).contains(&self.steps.len())
            && self.steps.iter().all(|step| {
                step.target.revision_id == self.revision_id
                    && step.target.record_sha256 == self.record_sha256
                    && step.snapshot_id == self.snapshot_id
                    && step.valid_recorded_shape()
                    && step_ids.insert(step.target.step_id.as_str())
            })
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
    prepare_from_record(&record, project_id, target)
}

/// Replays one accepted record once and prepares every step in its saved
/// order. A single unavailable or over-limit step rejects the batch before
/// any provider request can be built or sent.
pub(crate) fn prepare_batch(
    storage: &Path,
    project_id: &str,
    target: &PlannedChapterBatchTarget,
) -> Result<PlannedChapterBatchScope, String> {
    if !repository_plan::canonical_uuid_id(&target.revision_id, "plan-revision")
        || target.record_sha256.len() != 64
        || !target
            .record_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("Open the exact accepted route before preparing its chapters.".to_owned());
    }
    let record = repository_plan_history::load_accepted_record(
        storage,
        project_id,
        &target.revision_id,
        &target.record_sha256,
    )?;
    let plan = &record.preview.plan;
    if plan.steps.is_empty() || plan.steps.len() > MAX_PLANNED_CHAPTERS {
        return Err("The accepted route has no buildable steps or exceeds the chapter limit.".to_owned());
    }
    let mut steps = Vec::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let step_target = PlannedChapterTarget {
            revision_id: target.revision_id.clone(),
            record_sha256: target.record_sha256.clone(),
            step_id: step.step_id.clone(),
        };
        steps.push(prepare_from_record(&record, project_id, &step_target)?);
    }
    let batch = PlannedChapterBatchScope {
        schema_version: 1,
        project_id: project_id.to_owned(),
        revision_id: target.revision_id.clone(),
        record_sha256: target.record_sha256.clone(),
        snapshot_id: plan.snapshot_id.clone(),
        steps,
    };
    if !batch.valid_recorded_shape() {
        return Err("The accepted route's chapter batch could not be verified.".to_owned());
    }
    Ok(batch)
}

fn prepare_from_record(
    record: &repository_plan_store::SavedPlanRecord,
    project_id: &str,
    target: &PlannedChapterTarget,
) -> Result<PlannedChapterScope, String> {
    let plan = &record.preview.plan;
    if plan.project_id != project_id {
        return Err("The accepted route belongs to a different repository.".to_owned());
    }
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
