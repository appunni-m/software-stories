use super::{Attempt, Record, SectionNotice, Unit, unit_attempt_complete};
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
    pub(crate) opening: GuideUnitFreshness,
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
    // Freshness is only meaningful when the source being compared is the
    // exact immutable inventory this Guide originally reviewed. Reuse the
    // same digest and starting-range checks required by normal Guide reads;
    // otherwise changed local snapshot metadata could make old evidence look
    // safe to carry into a new snapshot.
    let from_manifest = super::checked_manifest(storage, guide)?;
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

    let opening_question = if guide.opening.is_some() {
        super::opening::question(guide)?
    } else {
        super::opening::LEARNING_QUESTION
    };
    let opening = assess_opening(
        storage,
        &from_manifest,
        from_index.as_ref(),
        &to_manifest,
        to_index.as_ref(),
        guide,
        opening_question,
        &revisions,
    );

    let Some(plan) = guide.plan.as_ref() else {
        return Ok(RepositoryGuideFreshness {
            guide_id: guide.id.clone(),
            project_id: guide.project_id.clone(),
            from_snapshot_id: guide.snapshot_id.clone(),
            to_snapshot_id: to_snapshot_id.to_owned(),
            opening,
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
        opening,
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

    output.sections = assess_notices(
        storage,
        from_manifest,
        from_index,
        to_manifest,
        to_index,
        answer.sections.iter(),
        revisions,
    );

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

/// Opening continuations add newly published slides while keeping earlier
/// slides in the same Guide. Unlike a normal unit retry, freshness therefore
/// has to assess every published attempt, not only the latest complete one.
fn assess_opening(
    storage: &Path,
    from_manifest: &SnapshotManifest,
    from_index: Option<&crate::index::SnapshotIndex>,
    to_manifest: &SnapshotManifest,
    to_index: Option<&crate::index::SnapshotIndex>,
    guide: &Record,
    question: &str,
    revisions: &HashMap<String, ChapterRevision>,
) -> GuideUnitFreshness {
    let attempts = guide
        .opening
        .iter()
        .chain(&guide.opening_continuations)
        .collect::<Vec<_>>();
    let mut output = GuideUnitFreshness {
        key: "opening".into(),
        question: question.to_owned(),
        prerequisites: Vec::new(),
        status: GuideUnitFreshnessStatus::Pending,
        sections: Vec::new(),
        reasons: Vec::new(),
    };

    if attempts.is_empty() {
        return output;
    }

    output.sections = assess_notices(
        storage,
        from_manifest,
        from_index,
        to_manifest,
        to_index,
        attempts
            .iter()
            .filter_map(|attempt| attempt.summary.as_ref())
            .flat_map(|answer| answer.sections.iter()),
        revisions,
    );

    let Some(latest) = attempts.last() else {
        return output;
    };
    let latest_is_complete = unit_attempt_complete(latest);
    if output.sections.is_empty() {
        output.status = GuideUnitFreshnessStatus::Incomplete;
        output.reasons.push(if latest.summary.is_none() {
            "The latest saved attempt has no completed source receipt.".into()
        } else {
            "This Guide topic has no published section yet.".into()
        });
        return output;
    }

    output.status = aggregate_sections(&output.sections);
    if output.status == GuideUnitFreshnessStatus::Current && !latest_is_complete {
        output.status = GuideUnitFreshnessStatus::Incomplete;
        output
            .reasons
            .push("Only a partial opening attempt has published content.".into());
    }
    output.reasons.extend(
        output
            .sections
            .iter()
            .flat_map(|section| section.reasons.iter().cloned()),
    );
    output
}

fn assess_notices<'a>(
    storage: &Path,
    from_manifest: &SnapshotManifest,
    from_index: Option<&crate::index::SnapshotIndex>,
    to_manifest: &SnapshotManifest,
    to_index: Option<&crate::index::SnapshotIndex>,
    notices: impl Iterator<Item = &'a SectionNotice>,
    revisions: &HashMap<String, ChapterRevision>,
) -> Vec<GuideSectionFreshness> {
    let mut assessed = Vec::new();
    for notice in notices {
        let Some(revision) = revisions.get(&notice.revision_id) else {
            assessed.push(GuideSectionFreshness {
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
            Ok(report) if !report.sections.is_empty() => {
                assessed.extend(report.sections.into_iter().map(|section| {
                    GuideSectionFreshness {
                        revision_id: revision.revision_id.clone(),
                        status: section.status,
                        evidence_state: section.evidence_state,
                        reasons: section
                            .reasons
                            .into_iter()
                            .map(|reason| reason.message)
                            .collect(),
                    }
                }));
            }
            Ok(_) | Err(_) => assessed.push(GuideSectionFreshness {
                revision_id: notice.revision_id.clone(),
                status: SectionFreshnessStatus::NeedsReview,
                evidence_state: EvidenceState::Unavailable,
                reasons: vec!["The saved source scope could not be verified.".into()],
            }),
        }
    }
    assessed
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

    #[test]
    fn source_change_assessment_rejects_a_changed_saved_inventory_digest() {
        let fixture = crate::chapter_generation::tests::Fixture::new();
        let mut guide = crate::repository_guide::tests::fresh_record(&fixture);
        guide.manifest_sha256 = "f".repeat(64);

        let error = assess(&fixture.storage, &guide, &guide.snapshot_id).unwrap_err();

        assert!(error.contains("captured inventory changed"));
    }

    #[test]
    fn source_change_assessment_accepts_the_original_guide_inventory() {
        let fixture = crate::chapter_generation::tests::Fixture::new();
        let guide = crate::repository_guide::tests::fresh_record(&fixture);

        let report = assess(&fixture.storage, &guide, &guide.snapshot_id).unwrap();

        assert_eq!(report.from_snapshot_id, guide.snapshot_id);
        assert_eq!(report.to_snapshot_id, guide.snapshot_id);
        assert_eq!(report.opening.key, "opening");
        assert_eq!(
            report.opening.question,
            super::super::opening::LEARNING_QUESTION
        );
        assert_eq!(report.opening.status, GuideUnitFreshnessStatus::Incomplete);
        assert_eq!(report.units.len(), guide.units.len());
        assert!(
            report
                .units
                .iter()
                .all(|unit| { unit.status == GuideUnitFreshnessStatus::Pending })
        );
    }

    #[test]
    fn source_change_assessment_includes_the_opening_without_an_outline() {
        let fixture = crate::chapter_generation::tests::Fixture::new();
        let mut guide = crate::repository_guide::tests::fresh_record(&fixture);
        guide.plan = None;
        guide.units.clear();

        let report = assess(&fixture.storage, &guide, &guide.snapshot_id).unwrap();

        assert_eq!(report.opening.key, "opening");
        assert_eq!(report.opening.status, GuideUnitFreshnessStatus::Incomplete);
        assert!(report.units.is_empty());
    }

    #[test]
    fn opening_freshness_includes_slides_from_partial_attempts_and_continuations() {
        let fixture = crate::chapter_generation::tests::Fixture::new();
        let mut guide = crate::repository_guide::tests::fresh_record(&fixture);
        let mut partial =
            crate::repository_guide::tests::attempt(&guide, super::super::Outcome::Complete);
        partial.summary.as_mut().unwrap().outcome = super::super::Outcome::Partial;
        guide.opening = Some(partial);
        guide
            .opening_continuations
            .push(crate::repository_guide::tests::attempt(
                &guide,
                super::super::Outcome::Complete,
            ));

        let report = assess_opening(
            &fixture.storage,
            &fixture.manifest,
            None,
            &fixture.manifest,
            None,
            &guide,
            super::super::opening::LEARNING_QUESTION,
            &HashMap::new(),
        );

        assert_eq!(report.sections.len(), 2);
        assert_ne!(
            report.sections[0].revision_id,
            report.sections[1].revision_id
        );
        assert_eq!(report.status, GuideUnitFreshnessStatus::NeedsReview);
    }

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
