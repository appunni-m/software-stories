use super::{Attempt, Record, freshness};
use crate::{
    chapter_freshness::SectionFreshnessStatus,
    chapter_generation,
    chapter_origin::GuideCarryForwardOrigin,
    chapter_store, chapters,
    evidence_ledger::sections::{SectionNotice, reading::Summary},
};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use uuid::Uuid;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CarryFailure {
    pub(crate) section_id: Option<String>,
    pub(crate) reason: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CarryResult {
    pub(crate) saved_revisions: Vec<chapters::ChapterRevision>,
    pub(crate) failures: Vec<CarryFailure>,
    pub(crate) freshness: freshness::RepositoryGuideFreshness,
}

struct Candidate {
    notice: SectionNotice,
    source_revision: chapters::ChapterRevision,
    source_section: chapters::ChapterSection,
    target_section: chapters::ChapterSection,
}

fn attempts(guide: &Record) -> impl Iterator<Item = &Attempt> {
    guide
        .opening
        .iter()
        .chain(&guide.opening_continuations)
        .chain(guide.units.iter().flat_map(|unit| unit.attempts.iter()))
}

fn notices(guide: &Record) -> impl Iterator<Item = &SectionNotice> {
    attempts(guide).flat_map(|attempt| {
        attempt
            .summary
            .as_ref()
            .into_iter()
            .flat_map(|summary: &Summary| summary.sections.iter())
    })
}

fn id(prefix: &str) -> String {
    format!("{prefix}:{}", Uuid::new_v4())
}

fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |duration| {
            duration.as_millis().min(u64::MAX as u128) as u64
        })
}

/// Carry slides freshly classified as safe, plus fully verified unchanged
/// slides whose evidence is complete. Each target receives one validated local
/// draft containing its eligible slides. A retry skips recorded lineage.
pub(super) fn carry_safe_sections(
    storage: &std::path::Path,
    guide: &Record,
    target_snapshot_id: &str,
) -> Result<CarryResult, String> {
    let initial_freshness = freshness::assess(storage, guide, target_snapshot_id)?;
    let source_manifest = super::checked_manifest(storage, guide)?;
    let source_index =
        crate::index::load_snapshot_index(storage, &guide.project_id, &guide.snapshot_id)?;
    let target_manifest =
        crate::snapshot::load_snapshot_manifest(storage, &guide.project_id, target_snapshot_id)?;
    let target_index =
        crate::index::load_snapshot_index(storage, &guide.project_id, target_snapshot_id)?;
    let revisions = chapter_store::list_for_project(storage, &guide.project_id)?
        .into_iter()
        .map(|revision| (revision.revision_id.clone(), revision))
        .collect::<HashMap<_, _>>();
    let notices = notices(guide).cloned().collect::<Vec<_>>();
    let mut by_section = HashMap::<(String, String, String), SectionNotice>::new();
    for notice in notices {
        let Some(revision) = revisions.get(&notice.revision_id) else {
            continue;
        };
        for section in &revision.sections {
            by_section.insert(
                (
                    revision.revision_id.clone(),
                    section.section_id.clone(),
                    section.section_revision_id.clone(),
                ),
                notice.clone(),
            );
        }
    }

    let mut grouped = BTreeMap::<(String, String), Vec<Candidate>>::new();
    let mut failures = Vec::new();
    for unit in std::iter::once(&initial_freshness.opening).chain(&initial_freshness.units) {
        for assessed in &unit.sections {
            let eligible = assessed.status == SectionFreshnessStatus::SafeToCarry
                || (assessed.status == SectionFreshnessStatus::Current
                    && assessed.evidence_state
                        == crate::chapter_freshness::EvidenceState::Complete);
            if !eligible || assessed.carried_revision_id.is_some() {
                continue;
            }
            let (Some(section_id), Some(section_revision_id)) = (
                assessed.section_id.as_deref(),
                assessed.section_revision_id.as_deref(),
            ) else {
                failures.push(CarryFailure {
                    section_id: None,
                    reason: "The slide has no exact saved section identity.".into(),
                });
                continue;
            };
            let key = (
                assessed.revision_id.clone(),
                section_id.to_owned(),
                section_revision_id.to_owned(),
            );
            let Some(notice) = by_section.get(&key) else {
                failures.push(CarryFailure {
                    section_id: Some(section_id.to_owned()),
                    reason: "The slide no longer has its exact Guide publication receipt.".into(),
                });
                continue;
            };
            let Some(source_revision) = revisions.get(&assessed.revision_id) else {
                failures.push(CarryFailure {
                    section_id: Some(section_id.to_owned()),
                    reason: "The saved slide revision is unavailable.".into(),
                });
                continue;
            };
            if source_revision.project_id != guide.project_id
                || source_revision.snapshot_id != guide.snapshot_id
                || !source_revision
                    .generation_origin
                    .as_ref()
                    .is_some_and(|origin| {
                        origin.request_id == notice.run_id
                            && origin.project_id == guide.project_id
                            && origin.snapshot_id == guide.snapshot_id
                    })
            {
                failures.push(CarryFailure {
                    section_id: Some(section_id.to_owned()),
                    reason: "The saved slide no longer matches its Guide publication receipt."
                        .into(),
                });
                continue;
            }
            let Some(source_section) = source_revision.sections.iter().find(|section| {
                section.section_id == section_id
                    && section.section_revision_id == section_revision_id
            }) else {
                failures.push(CarryFailure {
                    section_id: Some(section_id.to_owned()),
                    reason: "The source section changed after the freshness check.".into(),
                });
                continue;
            };
            let mut target_section = match chapter_generation::remap_carried_section(
                storage,
                &source_manifest,
                source_index.as_ref(),
                &target_manifest,
                target_index.as_ref(),
                source_section,
            ) {
                Ok(section) => section,
                Err(reason) => {
                    failures.push(CarryFailure {
                        section_id: Some(section_id.to_owned()),
                        reason,
                    });
                    continue;
                }
            };
            target_section.section_revision_id = id("sectionrev");
            let Some(origin) = source_revision.generation_origin.as_ref() else {
                failures.push(CarryFailure {
                    section_id: Some(section_id.to_owned()),
                    reason: "The source slide has no recorded generation identity.".into(),
                });
                continue;
            };
            if origin.request_id != notice.run_id {
                failures.push(CarryFailure {
                    section_id: Some(section_id.to_owned()),
                    reason: "The source slide does not match its saved Guide run.".into(),
                });
                continue;
            }
            grouped
                .entry((
                    source_revision.chapter_id.clone(),
                    source_revision.target.target_id.clone(),
                ))
                .or_default()
                .push(Candidate {
                    notice: notice.clone(),
                    source_revision: source_revision.clone(),
                    source_section: source_section.clone(),
                    target_section,
                });
        }
    }

    let mut saved_revisions = Vec::new();
    for (_, mut candidates) in grouped {
        let first = &candidates[0];
        let mut revision = first.source_revision.clone();
        revision.revision_id = id("revision");
        revision.revision_number = 1;
        revision.snapshot_id = target_snapshot_id.to_owned();
        revision.lifecycle = chapters::RevisionLifecycle::Draft;
        revision.validation_status = chapters::ValidationStatus::Valid;
        revision.created_at_unix_ms = unix_time_ms();
        revision.based_on_revision_id = Some(first.source_revision.revision_id.clone());
        revision.decision_note = None;
        revision.generation_origin = None;
        revision.sections = candidates
            .iter_mut()
            .enumerate()
            .map(|(order, candidate)| {
                candidate.target_section.order = order as u32;
                candidate.target_section.clone()
            })
            .collect();
        let lineages = candidates
            .iter()
            .map(|candidate| {
                (
                    candidate.notice.run_id.as_str(),
                    candidate.notice.generation_id.as_str(),
                    &candidate.source_revision,
                    &candidate.source_section,
                )
            })
            .collect::<Vec<_>>();
        revision.guide_carry_forward = Some(GuideCarryForwardOrigin::for_sections(
            &Uuid::new_v4().to_string(),
            &guide.id,
            &guide.id,
            &lineages,
            &revision,
        )?);
        let validation = chapters::validate_chapter_revision(
            storage,
            &target_manifest,
            target_index.as_ref(),
            &revision,
        );
        if validation.status != chapters::ValidationReportStatus::Valid {
            failures.extend(candidates.iter().map(|candidate| CarryFailure {
                section_id: Some(candidate.source_section.section_id.clone()),
                reason: "A slide no longer validates against the latest captured code. Review it in the freshness report.".into(),
            }));
            continue;
        }
        match chapter_store::append_carried_guide_draft(storage, revision) {
            Ok(revision) => saved_revisions.push(revision),
            Err(reason) => failures.extend(candidates.iter().map(|candidate| CarryFailure {
                section_id: Some(candidate.source_section.section_id.clone()),
                reason: reason.clone(),
            })),
        }
    }

    let freshness = freshness::assess(storage, guide, target_snapshot_id)?;
    Ok(CarryResult {
        saved_revisions,
        failures,
        freshness,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        chapter_generation::tests::{Fixture, SIMPLE_PATH, simple_proposal_text},
        chapter_origin::ChapterGenerationOrigin,
        evidence_ledger::sections::Outcome,
        repository_guide::tests::fresh_record,
    };

    fn guide_with_published_slide(fixture: &Fixture) -> Record {
        let mut guide = fresh_record(fixture);
        let run_id = Uuid::new_v4().to_string();
        let generation_id = format!("generation:{}", Uuid::new_v4());
        let sources = fixture.sources(&[SIMPLE_PATH]);
        let mut revision = chapter_generation::canonicalize_file_chapter(
            &fixture.manifest.summary.project_id,
            &fixture.manifest,
            Some(&fixture.index),
            SIMPLE_PATH,
            &sources,
            &std::collections::HashSet::new(),
            &simple_proposal_text(),
        )
        .unwrap();
        revision.chapter_id = "chapter:overview".into();
        revision.target.target_id = "target:overview".into();
        revision.target.scope = chapters::ChapterTargetScope::Overview;
        revision.sections.truncate(1);
        for section in &mut revision.sections {
            section.dependencies.retain(|dependency| {
                !matches!(dependency, chapters::SectionDependency::Relationship { .. })
            });
            for claim in &mut section.claims {
                claim.dependencies.retain(|dependency| {
                    !matches!(dependency, chapters::SectionDependency::Relationship { .. })
                });
            }
        }
        revision.lifecycle = chapters::RevisionLifecycle::Draft;
        let symbol = fixture
            .index
            .symbols
            .iter()
            .find(|symbol| symbol.path == SIMPLE_PATH && symbol.name == "greeting")
            .unwrap();
        let mut range = crate::repository_context::range_from_source(
            &sources[0],
            symbol.anchor.start_byte,
            symbol.anchor.end_byte,
        )
        .unwrap()
        .range;
        range.enclosing = Some(crate::repository_context::SymbolContext {
            symbol_id: symbol.symbol_id.clone(),
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            start_byte: symbol.anchor.start_byte,
            end_byte: symbol.anchor.end_byte,
        });
        revision.generation_origin = Some(
            ChapterGenerationOrigin::from_answer_checkpoint(
                &run_id,
                &"a".repeat(64),
                &"b".repeat(64),
                "gpt-6-luna",
                "max",
                vec![range],
                Vec::new(),
                Some(&fixture.index),
                &sources,
                &revision,
            )
            .unwrap(),
        );
        let validation = chapters::validate_chapter_revision(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            &revision,
        );
        assert_eq!(validation.status, chapters::ValidationReportStatus::Valid);
        revision.validation_status = chapters::ValidationStatus::Valid;
        let revision = chapter_store::save_new_draft(&fixture.storage, revision).unwrap();
        let notice = SectionNotice {
            run_id: run_id.clone(),
            project_id: guide.project_id.clone(),
            snapshot_id: guide.snapshot_id.clone(),
            key: "opening".into(),
            order: 0,
            generation_id,
            revision_id: revision.revision_id,
            supersedes: None,
            context_elapsed_ms: 1,
            run_elapsed_ms: Some(2),
        };
        let project_id = guide.project_id.clone();
        let snapshot_id = guide.snapshot_id.clone();
        let unit = &mut guide.units[0];
        unit.attempts.push(Attempt {
            run_id: run_id.clone(),
            question: "How does the name enter the greeting?".into(),
            automatic_retry: false,
            prerequisite_fingerprints: None,
            summary: Some(Summary {
                run_id,
                project_id,
                snapshot_id,
                question: "How does the name enter the greeting?".into(),
                outcome: Outcome::Complete,
                failure_code: None,
                sections: vec![notice],
                incomplete_topics: Vec::new(),
            }),
        });
        guide
    }

    #[test]
    fn carries_only_safe_slides_once_and_records_the_target_draft() {
        let mut fixture = Fixture::new();
        fixture.index.status = crate::index::SnapshotIndexStatus::Complete;
        fixture.index.relationships.clear();
        crate::index::save_snapshot_index(&fixture.storage, &fixture.index).unwrap();
        let guide = guide_with_published_slide(&fixture);
        fixture.recapture_with_source("README.md", "# A note unrelated to the guide slide.\n");
        fixture.index.status = crate::index::SnapshotIndexStatus::Complete;
        fixture.index.relationships.clear();
        crate::index::save_snapshot_index(&fixture.storage, &fixture.index).unwrap();
        let target_snapshot_id = fixture.manifest.summary.snapshot_id.clone();

        let first = carry_safe_sections(&fixture.storage, &guide, &target_snapshot_id).unwrap();

        assert_eq!(
            first.freshness.units[0].sections[0].status,
            SectionFreshnessStatus::Current,
            "{:?}",
            first.freshness.units[0].sections[0].reasons
        );
        assert_eq!(
            first.freshness.units[0].sections[0].evidence_state,
            crate::chapter_freshness::EvidenceState::Complete
        );
        assert_eq!(first.saved_revisions.len(), 1);
        assert_eq!(first.saved_revisions[0].sections.len(), 1);
        assert!(first.saved_revisions[0].generation_origin.is_none());
        assert_eq!(
            first.freshness.units[0].sections[0]
                .carried_revision_id
                .as_deref(),
            Some(first.saved_revisions[0].revision_id.as_str())
        );
        let second = carry_safe_sections(&fixture.storage, &guide, &target_snapshot_id).unwrap();
        assert!(second.saved_revisions.is_empty());
        assert!(second.failures.is_empty());
    }

    #[test]
    fn stale_guide_slides_are_never_carried() {
        let mut fixture = Fixture::new();
        let guide = guide_with_published_slide(&fixture);
        fixture.recapture_with_source(
            SIMPLE_PATH,
            "export function greeting(name: string): string {\n    return `Welcome, ${name}`;\n}\n",
        );
        let target_snapshot_id = fixture.manifest.summary.snapshot_id.clone();

        let result = carry_safe_sections(&fixture.storage, &guide, &target_snapshot_id).unwrap();

        assert!(result.saved_revisions.is_empty());
        assert!(result.failures.is_empty());
        assert_eq!(
            result.freshness.units[0].sections[0].status,
            SectionFreshnessStatus::Stale
        );
    }
}
