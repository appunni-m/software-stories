use super::{unit_attempt_complete, Attempt, Record, Unit};
use crate::{
    chapter_freshness::{self, EvidenceState, SectionFreshnessStatus},
    chapters::ChapterRevision,
    snapshot::{self, SnapshotManifest},
};
use serde::Serialize;
use std::{collections::HashMap, path::Path};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryGuideFreshness {
    pub(crate) guide_id: String,
    pub(crate) project_id: String,
    pub(crate) from_snapshot_id: String,
    pub(crate) to_snapshot_id: String,
    pub(crate) units: Vec<GuideUnitFreshness>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuideUnitFreshness {
    pub(crate) key: String,
    pub(crate) question: String,
    pub(crate) prerequisites: Vec<String>,
    pub(crate) status: GuideUnitFreshnessStatus,
    pub(crate) sections: Vec<GuideSectionFreshness>,
    pub(crate) reasons: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuideSectionFreshness {
    pub(crate) revision_id: String,
    pub(crate) status: SectionFreshnessStatus,
    pub(crate) evidence_state: EvidenceState,
    pub(crate) reasons: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum GuideUnitFreshnessStatus {
    Current,
    SafeToCarry,
    Stale,
    NeedsReview,
    Incomplete,
    Pending,
}

pub(crate) fn assess(
    storage: &Path,
    guide: &Record,
    to_snapshot_id: &str,
) -> Result<RepositoryGuideFreshness, String> {
    let from_manifest =
        snapshot::load_snapshot_manifest(storage, &guide.project_id, &guide.snapshot_id)?;
    let to_manifest = snapshot::load_snapshot_manifest(storage, &guide.project_id, to_snapshot_id)?;
    if from_manifest.summary.project_id != to_manifest.summary.project_id
        || from_manifest.summary.project_id != guide.project_id
    {
        return Err("The Guide and comparison snapshot must belong to the same repository.".into());
    }
    let from_index =
        crate::index::load_snapshot_index(storage, &guide.project_id, &guide.snapshot_id)?;
    let to_index = crate::index::load_snapshot_index(storage, &guide.project_id, to_snapshot_id)?;
    let revisions =
        crate::chapter_store::list_for_snapshot(storage, &guide.project_id, &guide.snapshot_id)?;
    let revisions = revisions
        .into_iter()
        .map(|revision| (revision.revision_id.clone(), revision))
        .collect::<HashMap<_, _>>();

    let Some(plan) = guide.plan.as_ref() else {
        return Ok(RepositoryGuideFreshness {
            guide_id: guide.id.clone(),
            project_id: guide.project_id.clone(),
            from_snapshot_id: guide.snapshot_id.clone(),
            to_snapshot_id: to_snapshot_id.to_owned(),
            units: Vec::new(),
        });
    };
    if plan.units.len() != guide.units.len() {
        return Err(
            "The saved Guide outline is inconsistent. Existing explanations were preserved.".into(),
        );
    }

    let mut units = guide
        .units
        .iter()
        .zip(&plan.units)
        .map(|(unit, planned)| {
            assess_unit(
                storage,
                &from_manifest,
                from_index.as_ref(),
                &to_manifest,
                to_index.as_ref(),
                unit,
                planned.question.as_str(),
                &planned.prerequisites,
                &revisions,
            )
        })
        .collect::<Vec<_>>();
    apply_attempt_prerequisite_freshness(&mut units, guide);
    propagate_prerequisite_freshness(&mut units);

    Ok(RepositoryGuideFreshness {
        guide_id: guide.id.clone(),
        project_id: guide.project_id.clone(),
        from_snapshot_id: guide.snapshot_id.clone(),
        to_snapshot_id: to_snapshot_id.to_owned(),
        units,
    })
}

fn assess_unit(
    storage: &Path,
    from_manifest: &SnapshotManifest,
    from_index: Option<&crate::index::SnapshotIndex>,
    to_manifest: &SnapshotManifest,
    to_index: Option<&crate::index::SnapshotIndex>,
    unit: &Unit,
    question: &str,
    prerequisites: &[String],
    revisions: &HashMap<String, ChapterRevision>,
) -> GuideUnitFreshness {
    let selected = selected_attempt(unit);
    let mut output = GuideUnitFreshness {
        key: unit.key.clone(),
        question: question.to_owned(),
        prerequisites: prerequisites.to_vec(),
        status: GuideUnitFreshnessStatus::Pending,
        sections: Vec::new(),
        reasons: Vec::new(),
    };
    let Some((attempt, complete)) = selected else {
        output.status = if unit.attempts.is_empty() {
            GuideUnitFreshnessStatus::Pending
        } else {
            GuideUnitFreshnessStatus::Incomplete
        };
        if output.status == GuideUnitFreshnessStatus::Incomplete {
            output
                .reasons
                .push("No published section is available to compare.".into());
        }
        return output;
    };
    let Some(answer) = attempt.summary.as_ref() else {
        output.status = GuideUnitFreshnessStatus::Incomplete;
        output
            .reasons
            .push("The latest saved attempt has no completed source receipt.".into());
        return output;
    };
    if answer.sections.is_empty() {
        output.status = GuideUnitFreshnessStatus::Incomplete;
        output
            .reasons
            .push("This Guide topic has no published section yet.".into());
        return output;
    }

    for notice in &answer.sections {
        let Some(revision) = revisions.get(&notice.revision_id) else {
            output.sections.push(GuideSectionFreshness {
                revision_id: notice.revision_id.clone(),
                status: SectionFreshnessStatus::NeedsReview,
                evidence_state: EvidenceState::Unavailable,
                reasons: vec!["The saved source-linked slide is unavailable.".into()],
            });
            continue;
        };
        match chapter_freshness::classify_guide_draft_freshness(
            storage,
            from_manifest,
            from_index,
            to_manifest,
            to_index,
            revision,
        ) {
            Ok(report) => {
                for section in report.sections {
                    output.sections.push(GuideSectionFreshness {
                        revision_id: revision.revision_id.clone(),
                        status: section.status,
                        evidence_state: section.evidence_state,
                        reasons: section
                            .reasons
                            .into_iter()
                            .map(|reason| reason.message)
                            .collect(),
                    });
                }
            }
            Err(_) => output.sections.push(GuideSectionFreshness {
                revision_id: notice.revision_id.clone(),
                status: SectionFreshnessStatus::NeedsReview,
                evidence_state: EvidenceState::Unavailable,
                reasons: vec!["The saved source scope could not be verified.".into()],
            }),
        }
    }

    output.status = aggregate_sections(&output.sections);
    if output.status == GuideUnitFreshnessStatus::Current && !complete {
        output.status = GuideUnitFreshnessStatus::Incomplete;
        output
            .reasons
            .push("Only a partial Guide attempt has published content.".into());
    }
    output.reasons.extend(
        output
            .sections
            .iter()
            .flat_map(|section| section.reasons.iter().cloned()),
    );
    output
}

fn attempt_has_sections(attempt: &Attempt) -> bool {
    attempt
        .summary
        .as_ref()
        .is_some_and(|answer| !answer.sections.is_empty())
}

fn selected_attempt(unit: &Unit) -> Option<(&Attempt, bool)> {
    unit.attempts
        .iter()
        .rev()
        .find(|attempt| unit_attempt_complete(attempt))
        .map(|attempt| (attempt, true))
        .or_else(|| {
            unit.attempts
                .iter()
                .rev()
                .find(|attempt| attempt_has_sections(attempt))
                .map(|attempt| (attempt, false))
        })
}

fn aggregate_sections(sections: &[GuideSectionFreshness]) -> GuideUnitFreshnessStatus {
    if sections.is_empty() {
        return GuideUnitFreshnessStatus::NeedsReview;
    }
    if sections
        .iter()
        .any(|section| section.status == SectionFreshnessStatus::Stale)
    {
        GuideUnitFreshnessStatus::Stale
    } else if sections
        .iter()
        .any(|section| section.status == SectionFreshnessStatus::NeedsReview)
    {
        GuideUnitFreshnessStatus::NeedsReview
    } else if sections
        .iter()
        .any(|section| section.status == SectionFreshnessStatus::SafeToCarry)
    {
        GuideUnitFreshnessStatus::SafeToCarry
    } else {
        GuideUnitFreshnessStatus::Current
    }
}

fn apply_attempt_prerequisite_freshness(units: &mut [GuideUnitFreshness], guide: &Record) {
    let Some(plan) = guide.plan.as_ref() else {
        return;
    };
    for (index, freshness) in units.iter_mut().enumerate() {
        let Some((attempt, _)) = guide.units.get(index).and_then(selected_attempt) else {
            continue;
        };
        let Some(planned) = plan.units.get(index) else {
            continue;
        };
        if planned.prerequisites.is_empty() {
            continue;
        }
        let current = planned
            .prerequisites
            .iter()
            .map(|key| {
                let prerequisite_index = plan.units.iter().position(|unit| &unit.key == key)?;
                let prerequisite = guide.units.get(prerequisite_index)?;
                Some(super::PrerequisiteFingerprint {
                    key: key.clone(),
                    revision_ids: selected_revision_ids(prerequisite),
                })
            })
            .collect::<Option<Vec<_>>>();
        let status = current.map_or(GuideUnitFreshnessStatus::NeedsReview, |current| {
            prerequisite_fingerprint_status(attempt.prerequisite_fingerprints.as_deref(), &current)
        });
        match status {
            GuideUnitFreshnessStatus::Stale => {
                freshness.status = GuideUnitFreshnessStatus::Stale;
                freshness
                    .reasons
                    .push("A saved prerequisite has a newer or different Guide slide.".into());
            }
            GuideUnitFreshnessStatus::NeedsReview
                if matches!(
                    freshness.status,
                    GuideUnitFreshnessStatus::Current | GuideUnitFreshnessStatus::SafeToCarry
                ) =>
            {
                freshness.status = GuideUnitFreshnessStatus::NeedsReview;
                freshness.reasons.push("This saved attempt predates prerequisite tracking, so its earlier topic dependencies are unknown.".into());
            }
            _ => {}
        }
    }
}

fn prerequisite_fingerprint_status(
    saved: Option<&[super::PrerequisiteFingerprint]>,
    current: &[super::PrerequisiteFingerprint],
) -> GuideUnitFreshnessStatus {
    match saved {
        None => GuideUnitFreshnessStatus::NeedsReview,
        Some(saved) if saved == current => GuideUnitFreshnessStatus::Current,
        Some(_) => GuideUnitFreshnessStatus::Stale,
    }
}

fn selected_revision_ids(unit: &Unit) -> Vec<String> {
    selected_attempt(unit)
        .and_then(|(attempt, _)| attempt.summary.as_ref())
        .map(|summary| {
            summary
                .sections
                .iter()
                .map(|section| section.revision_id.clone())
                .collect()
        })
        .unwrap_or_default()
}

fn propagate_prerequisite_freshness(units: &mut [GuideUnitFreshness]) {
    let mut resolved = HashMap::<String, GuideUnitFreshnessStatus>::new();
    for unit in units {
        let mut inherited = None;
        for prerequisite in &unit.prerequisites {
            match resolved.get(prerequisite) {
                Some(GuideUnitFreshnessStatus::Stale) => {
                    inherited = Some(GuideUnitFreshnessStatus::Stale);
                    break;
                }
                Some(
                    GuideUnitFreshnessStatus::NeedsReview
                    | GuideUnitFreshnessStatus::Pending
                    | GuideUnitFreshnessStatus::Incomplete,
                ) => {
                    inherited = Some(GuideUnitFreshnessStatus::NeedsReview);
                }
                _ => {}
            }
        }
        if let Some(inherited) = inherited {
            if matches!(
                unit.status,
                GuideUnitFreshnessStatus::Current | GuideUnitFreshnessStatus::SafeToCarry
            ) {
                unit.status = inherited;
                unit.reasons.push("A declared prerequisite needs another look before this topic can be carried forward.".into());
            }
        }
        resolved.insert(unit.key.clone(), unit.status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(
        key: &str,
        prerequisites: &[&str],
        status: GuideUnitFreshnessStatus,
    ) -> GuideUnitFreshness {
        GuideUnitFreshness {
            key: key.into(),
            question: key.into(),
            prerequisites: prerequisites.iter().map(|value| (*value).into()).collect(),
            status,
            sections: Vec::new(),
            reasons: Vec::new(),
        }
    }

    #[test]
    fn prerequisite_changes_propagate_only_through_declared_guide_edges() {
        let mut units = vec![
            unit("input", &[], GuideUnitFreshnessStatus::Stale),
            unit("output", &["input"], GuideUnitFreshnessStatus::Current),
            unit("unrelated", &[], GuideUnitFreshnessStatus::Current),
        ];
        propagate_prerequisite_freshness(&mut units);
        assert_eq!(units[1].status, GuideUnitFreshnessStatus::Stale);
        assert_eq!(units[2].status, GuideUnitFreshnessStatus::Current);
    }

    #[test]
    fn unknown_prerequisite_marks_dependent_material_for_review() {
        let mut units = vec![
            unit("input", &[], GuideUnitFreshnessStatus::NeedsReview),
            unit("output", &["input"], GuideUnitFreshnessStatus::SafeToCarry),
        ];
        propagate_prerequisite_freshness(&mut units);
        assert_eq!(units[1].status, GuideUnitFreshnessStatus::NeedsReview);
    }

    #[test]
    fn changed_prerequisite_fingerprint_marks_only_its_dependent_material_stale() {
        let saved = [super::super::PrerequisiteFingerprint {
            key: "input".into(),
            revision_ids: vec!["revision:old".into()],
        }];
        let current = [super::super::PrerequisiteFingerprint {
            key: "input".into(),
            revision_ids: vec!["revision:new".into()],
        }];
        assert_eq!(
            prerequisite_fingerprint_status(Some(&saved), &current),
            GuideUnitFreshnessStatus::Stale
        );
        assert_eq!(
            prerequisite_fingerprint_status(None, &current),
            GuideUnitFreshnessStatus::NeedsReview
        );
    }
}
