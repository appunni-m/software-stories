use crate::chapter_store;
use crate::chapters::{
    self, ChapterRevision, RevisionLifecycle, SectionDependency, ValidationReportStatus,
    ValidationStatus,
};
use crate::{index, snapshot};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedChapterReview {
    pub revision: ChapterRevision,
    pub validation: chapters::ChapterValidationReport,
}

/// Validate the immutable native record without a renderer JSON round-trip.
/// Historical revisions remain inspectable; project ownership is enforced by
/// the store. This read does not reserve a base for a later save or acceptance.
pub(crate) fn validate_saved(
    storage_dir: &Path,
    project_id: &str,
    revision_id: &str,
) -> Result<chapters::ChapterValidationReport, String> {
    let revision = chapter_store::list_for_project(storage_dir, project_id)?
        .into_iter()
        .find(|revision| revision.revision_id == revision_id)
        .ok_or_else(|| {
            "This saved chapter is no longer available. Reload its history.".to_owned()
        })?;
    let manifest =
        snapshot::load_snapshot_manifest(storage_dir, project_id, &revision.snapshot_id)?;
    let snapshot_index =
        index::load_snapshot_index(storage_dir, project_id, &revision.snapshot_id)?;
    Ok(chapters::validate_chapter_revision(
        storage_dir,
        &manifest,
        snapshot_index.as_ref(),
        &revision,
    ))
}

/// Read-only citation repair scoped to a trusted saved chapter and its snapshot.
pub(crate) fn repair_citation(
    storage_dir: &Path,
    project_id: &str,
    base_revision_id: &str,
    anchor_id: &str,
    relative_path: &str,
    start_line: u32,
    end_line: u32,
) -> Result<chapters::ChapterSourceAnchor, String> {
    let base = find_latest_base(storage_dir, project_id, base_revision_id)?;
    if !base
        .sections
        .iter()
        .flat_map(|s| &s.claims)
        .flat_map(|c| &c.anchors)
        .any(|a| a.anchor_id == anchor_id)
    {
        return Err("This citation does not belong to the selected saved chapter.".to_owned());
    }
    let source =
        snapshot::read_snapshot_source(storage_dir, project_id, &base.snapshot_id, relative_path)?;
    let snapshot_index = index::load_snapshot_index(storage_dir, project_id, &base.snapshot_id)?;
    chapters::anchor_from_lines(
        &source,
        snapshot_index.as_ref(),
        anchor_id,
        start_line,
        end_line,
    )
}

pub fn save_edits(
    storage_dir: &Path,
    write_lock: &Mutex<()>,
    project_id: &str,
    base_revision_id: &str,
    proposal: ChapterRevision,
) -> Result<SavedChapterReview, String> {
    let _guard = write_lock
        .lock()
        .map_err(|_| "The local chapter store is temporarily unavailable.".to_owned())?;
    let base = find_latest_base(storage_dir, project_id, base_revision_id)?;
    let mut revision = prepare_edit(&base, proposal)?;
    let manifest = snapshot::load_snapshot_manifest(storage_dir, project_id, &base.snapshot_id)?;
    let snapshot_index = index::load_snapshot_index(storage_dir, project_id, &base.snapshot_id)?;

    let validation = chapters::validate_chapter_revision(
        storage_dir,
        &manifest,
        snapshot_index.as_ref(),
        &revision,
    );
    revision.validation_status = if validation.status == ValidationReportStatus::Valid {
        ValidationStatus::Valid
    } else {
        ValidationStatus::Invalid
    };
    let revision = chapter_store::append_review_revision(storage_dir, revision)?;
    Ok(SavedChapterReview {
        revision,
        validation,
    })
}

/// Read-only preview of the same derived content Save will independently
/// revalidate under its write lock. No chapter or asset is written here.
pub(crate) fn validate_edits(
    storage_dir: &Path,
    project_id: &str,
    base_revision_id: &str,
    proposal: ChapterRevision,
) -> Result<chapters::ChapterValidationReport, String> {
    let base = find_latest_base(storage_dir, project_id, base_revision_id)?;
    let revision = prepare_edit(&base, proposal)?;
    let manifest = snapshot::load_snapshot_manifest(storage_dir, project_id, &base.snapshot_id)?;
    let snapshot_index = index::load_snapshot_index(storage_dir, project_id, &base.snapshot_id)?;
    Ok(chapters::validate_chapter_revision(
        storage_dir,
        &manifest,
        snapshot_index.as_ref(),
        &revision,
    ))
}

fn prepare_edit(
    base: &ChapterRevision,
    proposal: ChapterRevision,
) -> Result<ChapterRevision, String> {
    if base.project_id != proposal.project_id
        || base.snapshot_id != proposal.snapshot_id
        || base.chapter_id != proposal.chapter_id
        || base.target != proposal.target
    {
        return Err(
            "Chapter edits must keep the selected project, snapshot, and target.".to_owned(),
        );
    }

    let mut revision = base.clone();
    revision.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
    revision.revision_id = new_id("revision");
    revision.revision_number = base
        .revision_number
        .checked_add(1)
        .ok_or_else(|| "The chapter revision number is exhausted.".to_owned())?;
    revision.title = proposal.title;
    revision.sections = version_edited_sections(base, proposal.sections);
    revision.lifecycle = RevisionLifecycle::Draft;
    revision.validation_status = ValidationStatus::Unvalidated;
    revision.created_at_unix_ms = unix_time_ms();
    revision.based_on_revision_id = Some(base.revision_id.clone());
    revision.decision_note = None;

    Ok(revision)
}

/// Restore immutable content from an earlier revision into a new draft based on
/// the latest revision for the selected target. Evidence is remapped only when
/// the same stable symbol and exact excerpt can be proven in the target snapshot.
/// Unavailable anchors remain attached to the draft so validation can identify
/// the precise repair locations; they never become accepted by this operation.
pub fn restore_revision(
    storage_dir: &Path,
    write_lock: &Mutex<()>,
    project_id: &str,
    base_revision_id: &str,
    source_revision_id: &str,
) -> Result<SavedChapterReview, String> {
    let _guard = write_lock
        .lock()
        .map_err(|_| "The local chapter store is temporarily unavailable.".to_owned())?;
    let base = find_latest_base(storage_dir, project_id, base_revision_id)?;
    let history = chapter_store::list_for_project(storage_dir, project_id)?;
    let source = history
        .iter()
        .find(|revision| revision.revision_id == source_revision_id)
        .cloned()
        .ok_or_else(|| "The selected restore revision is no longer available.".to_owned())?;
    if source.chapter_id != base.chapter_id
        || source.target != base.target
        || source.revision_id == base.revision_id
        || source.revision_number >= base.revision_number
    {
        return Err(
            "Restore requires an earlier immutable revision of the same chapter target.".to_owned(),
        );
    }

    let source_manifest =
        snapshot::load_snapshot_manifest(storage_dir, project_id, &source.snapshot_id)?;
    let target_manifest =
        snapshot::load_snapshot_manifest(storage_dir, project_id, &base.snapshot_id)?;
    let source_index = index::load_snapshot_index(storage_dir, project_id, &source.snapshot_id)?;
    let target_index = index::load_snapshot_index(storage_dir, project_id, &base.snapshot_id)?;

    let mut restored = source.clone();
    restored.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
    restored.revision_id = new_id("revision");
    restored.revision_number = base
        .revision_number
        .checked_add(1)
        .ok_or_else(|| "The chapter revision number is exhausted.".to_owned())?;
    restored.project_id = base.project_id.clone();
    restored.snapshot_id = base.snapshot_id.clone();
    restored.target = base.target.clone();
    restored.lifecycle = RevisionLifecycle::Draft;
    restored.validation_status = ValidationStatus::Unvalidated;
    restored.created_at_unix_ms = unix_time_ms();
    restored.based_on_revision_id = Some(base.revision_id.clone());
    restored.decision_note = None;

    if source.snapshot_id != base.snapshot_id {
        for section in &mut restored.sections {
            for claim in &mut section.claims {
                for anchor in &mut claim.anchors {
                    let remap = chapters::remap_anchor_between_snapshots(
                        storage_dir,
                        &source_manifest,
                        source_index.as_ref(),
                        &target_manifest,
                        target_index.as_ref(),
                        anchor,
                    );
                    if let Some(updated) = remap
                        .anchor
                        .filter(|_| matches!(remap.status, chapters::AnchorRemapStatus::Remapped))
                    {
                        *anchor = updated;
                    }
                }
                for dependency in &mut claim.dependencies {
                    rebase_dependency(dependency, &target_manifest, target_index.as_ref());
                }
            }
            for dependency in &mut section.dependencies {
                rebase_dependency(dependency, &target_manifest, target_index.as_ref());
            }
            section.section_revision_id = new_id("sectionrev");
        }
    } else {
        for section in &mut restored.sections {
            section.section_revision_id = new_id("sectionrev");
        }
    }

    let validation = chapters::validate_chapter_revision(
        storage_dir,
        &target_manifest,
        target_index.as_ref(),
        &restored,
    );
    restored.validation_status = if validation.status == ValidationReportStatus::Valid {
        ValidationStatus::Valid
    } else {
        ValidationStatus::Invalid
    };
    let revision = chapter_store::append_review_revision(storage_dir, restored)?;
    Ok(SavedChapterReview {
        revision,
        validation,
    })
}

fn rebase_dependency(
    dependency: &mut SectionDependency,
    manifest: &snapshot::SnapshotManifest,
    index: Option<&index::SnapshotIndex>,
) {
    match dependency {
        SectionDependency::SourceFile {
            path,
            source_sha256,
        } => {
            if let Some(file) = manifest.files.iter().find(|file| file.path == *path) {
                *source_sha256 = file.sha256.clone();
            }
        }
        SectionDependency::Symbol {
            path,
            source_sha256,
            symbol_id,
        } => {
            if let Some(symbol) = index.and_then(|index| {
                index
                    .symbols
                    .iter()
                    .find(|symbol| symbol.path == *path && symbol.symbol_id == *symbol_id)
            }) {
                *source_sha256 = symbol.anchor.source_sha256.clone();
            }
        }
        SectionDependency::Relationship {
            path,
            source_sha256,
            relationship_id,
        } => {
            if let Some(relationship) = index.and_then(|index| {
                index.relationships.iter().find(|edge| {
                    edge.relationship_id == *relationship_id && edge.evidence.path == *path
                })
            }) {
                *source_sha256 = relationship.evidence.source_sha256.clone();
            }
        }
    }
}

pub fn accept_draft(
    storage_dir: &Path,
    write_lock: &Mutex<()>,
    project_id: &str,
    revision_id: &str,
) -> Result<SavedChapterReview, String> {
    let _guard = write_lock
        .lock()
        .map_err(|_| "The local chapter store is temporarily unavailable.".to_owned())?;
    let draft = find_latest_base(storage_dir, project_id, revision_id)?;
    if draft.lifecycle != RevisionLifecycle::Draft {
        return Err("Only the latest saved draft can be accepted.".to_owned());
    }
    let manifest = snapshot::load_snapshot_manifest(storage_dir, project_id, &draft.snapshot_id)?;
    let snapshot_index = index::load_snapshot_index(storage_dir, project_id, &draft.snapshot_id)?;
    let mut accepted = draft.clone();
    accepted.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
    accepted.revision_id = new_id("revision");
    accepted.revision_number = draft
        .revision_number
        .checked_add(1)
        .ok_or_else(|| "The chapter revision number is exhausted.".to_owned())?;
    accepted.lifecycle = RevisionLifecycle::Accepted;
    accepted.validation_status = ValidationStatus::Valid;
    accepted.created_at_unix_ms = unix_time_ms();
    accepted.based_on_revision_id = Some(draft.revision_id.clone());
    accepted.decision_note = None;
    let validation = chapters::validate_chapter_revision(
        storage_dir,
        &manifest,
        snapshot_index.as_ref(),
        &accepted,
    );
    if validation.status != ValidationReportStatus::Valid {
        let detail = validation
            .issues
            .first()
            .map(|issue| issue.message.as_str())
            .unwrap_or("The chapter failed local evidence validation.");
        return Err(format!(
            "This draft cannot be accepted until every issue is fixed: {detail}"
        ));
    }
    let accepted = chapter_store::append_review_revision(storage_dir, accepted)?;
    Ok(SavedChapterReview {
        revision: accepted,
        validation,
    })
}

pub fn reject_draft(
    storage_dir: &Path,
    write_lock: &Mutex<()>,
    project_id: &str,
    revision_id: &str,
    reason: &str,
) -> Result<ChapterRevision, String> {
    let reason = reason.trim();
    if reason.is_empty() || reason.len() > 4_000 {
        return Err("Enter a rejection reason of 1 to 4,000 bytes.".to_owned());
    }
    let _guard = write_lock
        .lock()
        .map_err(|_| "The local chapter store is temporarily unavailable.".to_owned())?;
    let draft = find_latest_base(storage_dir, project_id, revision_id)?;
    if draft.lifecycle != RevisionLifecycle::Draft {
        return Err("Only the latest saved draft can be rejected.".to_owned());
    }
    let mut rejected = draft.clone();
    rejected.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
    rejected.revision_id = new_id("revision");
    rejected.revision_number = draft
        .revision_number
        .checked_add(1)
        .ok_or_else(|| "The chapter revision number is exhausted.".to_owned())?;
    rejected.lifecycle = RevisionLifecycle::Rejected;
    rejected.created_at_unix_ms = unix_time_ms();
    rejected.based_on_revision_id = Some(draft.revision_id.clone());
    rejected.decision_note = Some(reason.to_owned());
    chapter_store::append_review_revision(storage_dir, rejected)
}

fn find_latest_base(
    storage_dir: &Path,
    project_id: &str,
    revision_id: &str,
) -> Result<ChapterRevision, String> {
    let revisions = chapter_store::list_for_project(storage_dir, project_id)?;
    let base = revisions
        .iter()
        .find(|revision| revision.revision_id == revision_id)
        .cloned()
        .ok_or_else(|| {
            "This chapter revision is no longer available. Reload Chapter Studio.".to_owned()
        })?;
    let latest = revisions
        .iter()
        .filter(|revision| {
            revision.snapshot_id == base.snapshot_id
                && revision.target.target_id == base.target.target_id
        })
        .max_by_key(|revision| revision.revision_number);
    if latest.map(|revision| revision.revision_id.as_str()) != Some(revision_id) {
        return Err(
            "This chapter changed since it was opened. Reload its latest revision.".to_owned(),
        );
    }
    Ok(base)
}

fn version_edited_sections(
    base: &ChapterRevision,
    mut proposed: Vec<chapters::ChapterSection>,
) -> Vec<chapters::ChapterSection> {
    for section in &mut proposed {
        let prior = base
            .sections
            .iter()
            .find(|old| old.section_id == section.section_id);
        let changed = prior.is_none_or(|old| {
            old.title != section.title
                || old.prose != section.prose
                || old.spoken_text != section.spoken_text
                || old.claims != section.claims
                || old.dependencies != section.dependencies
                || old.visual != section.visual
                || old.hotspots != section.hotspots
        });
        if changed {
            section.section_revision_id = new_id("section-revision");
        } else if let Some(prior) = prior {
            // The UI cannot edit this identity; preserve the previous section revision.
            section.section_revision_id = prior.section_revision_id.clone();
        }
    }
    proposed
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}:{}", Uuid::new_v4())
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_generation::{
        self,
        tests::{Fixture, SIMPLE_PATH, simple_proposal},
    };
    use crate::chapter_store;
    use std::collections::HashSet;

    fn first_saved_draft(fixture: &Fixture) -> ChapterRevision {
        let proposal = simple_proposal();
        let source = fixture.sources(&[SIMPLE_PATH]);
        let relationships: HashSet<String> = proposal
            .sections
            .iter()
            .flat_map(|section| &section.claims)
            .flat_map(|claim| &claim.relationship_ids)
            .cloned()
            .collect();
        let revision = chapter_generation::canonicalize_file_chapter(
            &fixture.manifest.summary.project_id,
            &fixture.manifest,
            Some(&fixture.index),
            SIMPLE_PATH,
            &source,
            &relationships,
            &serde_json::to_string(&proposal).unwrap(),
        )
        .unwrap();
        let report = chapters::validate_chapter_revision(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            &revision,
        );
        assert_eq!(report.status, ValidationReportStatus::Valid, "{report:#?}");
        let mut revision = revision;
        revision.validation_status = ValidationStatus::Valid;
        chapter_store::save_new_draft(&fixture.storage, revision).unwrap()
    }

    #[test]
    fn saved_validation_preserves_native_integer_ranges_and_historical_project_ownership() {
        let fixture = Fixture::new();
        let base = first_saved_draft(&fixture);
        let mut proposal = base.clone();
        proposal.sections[0].claims[0].anchors[0].end_byte_inclusive = u64::MAX;
        let invalid = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &base.project_id,
            &base.revision_id,
            proposal,
        )
        .unwrap();
        let before = chapter_store::list_for_project(&fixture.storage, &base.project_id).unwrap();
        let report = validate_saved(
            &fixture.storage,
            &base.project_id,
            &invalid.revision.revision_id,
        )
        .unwrap();
        assert_eq!(report.status, ValidationReportStatus::Invalid);
        assert!(!report.issues.is_empty());
        assert_eq!(
            validate_saved(&fixture.storage, &base.project_id, &base.revision_id)
                .unwrap()
                .status,
            ValidationReportStatus::Valid
        );
        assert!(
            validate_saved(&fixture.storage, "foreign", &invalid.revision.revision_id).is_err()
        );
        assert!(validate_saved(&fixture.storage, &base.project_id, "revision:missing").is_err());
        assert_eq!(
            chapter_store::list_for_project(&fixture.storage, &base.project_id).unwrap(),
            before
        );
        assert_eq!(
            before
                .iter()
                .find(|r| r.revision_id == invalid.revision.revision_id)
                .unwrap()
                .sections[0]
                .claims[0]
                .anchors[0]
                .end_byte_inclusive,
            u64::MAX
        );
    }

    #[test]
    fn citation_repair_reads_only_the_saved_snapshot_and_never_writes_a_revision() {
        let fixture = Fixture::new();
        let base = first_saved_draft(&fixture);
        let previous = chapter_store::list_for_project(&fixture.storage, &base.project_id).unwrap();
        let anchor = &base.sections[0].claims[0].anchors[0];
        let source = fixture.sources(&[SIMPLE_PATH]).remove(0);
        // Editing the working copy cannot replace captured evidence.
        std::fs::write(
            Path::new(&base.project_id).join(SIMPLE_PATH),
            "unrelated live bytes",
        )
        .unwrap();
        let repaired = repair_citation(
            &fixture.storage,
            &base.project_id,
            &base.revision_id,
            &anchor.anchor_id,
            SIMPLE_PATH,
            3,
            3,
        )
        .unwrap();
        assert_eq!(repaired.anchor_id, anchor.anchor_id);
        assert_eq!(repaired.snapshot_id, base.snapshot_id);
        assert_eq!(repaired.source_sha256, source.sha256);
        assert_eq!(
            repaired.excerpt,
            source.contents.split_inclusive('\n').nth(2).unwrap()
        );
        assert!(repaired.symbol_id.is_some());
        assert_eq!(
            chapter_store::list_for_project(&fixture.storage, &base.project_id).unwrap(),
            previous
        );
        for (project, owner, id, path, from, to) in [
            (
                "foreign",
                base.revision_id.as_str(),
                anchor.anchor_id.as_str(),
                SIMPLE_PATH,
                1,
                1,
            ),
            (
                base.project_id.as_str(),
                "revision:missing",
                anchor.anchor_id.as_str(),
                SIMPLE_PATH,
                1,
                1,
            ),
            (
                base.project_id.as_str(),
                base.revision_id.as_str(),
                "anchor:foreign",
                SIMPLE_PATH,
                1,
                1,
            ),
            (
                base.project_id.as_str(),
                base.revision_id.as_str(),
                anchor.anchor_id.as_str(),
                "../outside",
                1,
                1,
            ),
            (
                base.project_id.as_str(),
                base.revision_id.as_str(),
                anchor.anchor_id.as_str(),
                SIMPLE_PATH,
                0,
                1,
            ),
            (
                base.project_id.as_str(),
                base.revision_id.as_str(),
                anchor.anchor_id.as_str(),
                SIMPLE_PATH,
                1,
                u32::MAX,
            ),
        ] {
            assert!(repair_citation(&fixture.storage, project, owner, id, path, from, to).is_err());
        }
        assert_eq!(
            chapter_store::list_for_project(&fixture.storage, &base.project_id).unwrap(),
            previous
        );
    }

    #[test]
    fn citation_repair_rejects_stale_bases_and_damaged_captured_objects() {
        let fixture = Fixture::new();
        let base = first_saved_draft(&fixture);
        let anchor = &base.sections[0].claims[0].anchors[0];
        let mut proposal = base.clone();
        proposal.title = "A newer draft".into();
        let newer = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &base.project_id,
            &base.revision_id,
            proposal,
        )
        .unwrap();
        assert!(
            repair_citation(
                &fixture.storage,
                &base.project_id,
                &base.revision_id,
                &anchor.anchor_id,
                SIMPLE_PATH,
                1,
                1
            )
            .is_err()
        );
        let source = fixture.sources(&[SIMPLE_PATH]).remove(0);
        let blob = fixture
            .storage
            .join("objects/sha256")
            .join(&source.sha256[..2])
            .join(&source.sha256[2..]);
        std::fs::write(blob, "damaged").unwrap();
        assert!(
            repair_citation(
                &fixture.storage,
                &base.project_id,
                &newer.revision.revision_id,
                &anchor.anchor_id,
                SIMPLE_PATH,
                1,
                1
            )
            .is_err()
        );
        assert_eq!(
            chapter_store::list_for_project(&fixture.storage, &base.project_id)
                .unwrap()
                .len(),
            2
        );
    }

    #[test]
    fn invalid_citation_is_repaired_saved_and_accepted_without_changing_prior_revisions() {
        let fixture = Fixture::new();
        let base = first_saved_draft(&fixture);
        let original = chapters::encode_chapter_revision(&base).unwrap();
        let mut proposal = base.clone();
        proposal.sections[0].claims[0].anchors[0].end_byte_inclusive = u64::MAX;
        let invalid = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &base.project_id,
            &base.revision_id,
            proposal,
        )
        .unwrap();
        assert_eq!(invalid.validation.status, ValidationReportStatus::Invalid);
        assert!(
            accept_draft(
                &fixture.storage,
                &Mutex::new(()),
                &base.project_id,
                &invalid.revision.revision_id
            )
            .is_err()
        );
        let anchor_id = invalid.revision.sections[0].claims[0].anchors[0]
            .anchor_id
            .clone();
        let anchor = repair_citation(
            &fixture.storage,
            &base.project_id,
            &invalid.revision.revision_id,
            &anchor_id,
            SIMPLE_PATH,
            2,
            4,
        )
        .unwrap();
        let mut proposal = invalid.revision.clone();
        proposal.sections[0].claims[0].anchors[0] = anchor;
        let repaired = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &base.project_id,
            &invalid.revision.revision_id,
            proposal,
        )
        .unwrap();
        assert_eq!(
            repaired.validation.status,
            ValidationReportStatus::Valid,
            "{:?}",
            repaired.validation
        );
        let accepted = accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &base.project_id,
            &repaired.revision.revision_id,
        )
        .unwrap();
        assert_eq!(accepted.revision.lifecycle, RevisionLifecycle::Accepted);
        let history = chapter_store::list_for_project(&fixture.storage, &base.project_id).unwrap();
        assert_eq!(history.len(), 4);
        assert_eq!(
            chapters::encode_chapter_revision(
                history
                    .iter()
                    .find(|r| r.revision_id == base.revision_id)
                    .unwrap()
            )
            .unwrap(),
            original
        );
        assert_eq!(
            history
                .iter()
                .find(|r| r.revision_id == invalid.revision.revision_id)
                .unwrap()
                .validation_status,
            ValidationStatus::Invalid
        );
    }

    fn saved_agent_overview(fixture: &Fixture) -> ChapterRevision {
        let mut revision = first_saved_draft(fixture);
        revision.revision_id = new_id("revision");
        revision.chapter_id = "chapter:overview".into();
        revision.target.target_id = "target:overview".into();
        revision.target.scope = chapters::ChapterTargetScope::Overview;
        for section in &mut revision.sections {
            section.prose = chapters::overview_claim_sentences(&section.claims).join(" ");
            section.spoken_text = section.prose.clone();
        }
        let sources = fixture.sources(&[SIMPLE_PATH]);
        let range = crate::repository_context::read_range(
            &fixture.storage,
            &fixture.manifest,
            SIMPLE_PATH,
            &sources[0].sha256,
            0,
            crate::chapter_generation::tests::SIMPLE_SOURCE.len(),
        )
        .unwrap()
        .range;
        revision.generation_origin = Some(
            crate::chapter_origin::ChapterGenerationOrigin::from_answer_checkpoint(
                &Uuid::new_v4().to_string(),
                &"a".repeat(64),
                &"b".repeat(64),
                vec![range],
                &sources,
                &revision,
            )
            .unwrap(),
        );
        chapter_store::save_new_draft(&fixture.storage, revision).unwrap()
    }

    #[test]
    fn authored_nonvisual_narration_survives_edit_accept_restore_and_reject_immutably() {
        let fixture = Fixture::new();
        let original = saved_agent_overview(&fixture);
        let original_bytes = chapters::encode_chapter_revision(&original).unwrap();
        let mut proposal = original.clone();
        proposal.sections[0].visual = None;
        let speech = "Start with a person's name. It becomes part of a friendly greeting that is returned to the caller.";
        proposal.sections[0].spoken_text = speech.into();
        let lock = Mutex::new(());
        let edited = save_edits(
            &fixture.storage,
            &lock,
            &original.project_id,
            &original.revision_id,
            proposal,
        )
        .unwrap();
        assert_eq!(edited.validation.status, ValidationReportStatus::Valid);
        assert_eq!(edited.revision.schema_version, 5);
        let edited_bytes = chapters::encode_chapter_revision(&edited.revision).unwrap();
        let accepted = accept_draft(
            &fixture.storage,
            &lock,
            &original.project_id,
            &edited.revision.revision_id,
        )
        .unwrap();
        assert_eq!(accepted.validation.status, ValidationReportStatus::Valid);
        let accepted_bytes = chapters::encode_chapter_revision(&accepted.revision).unwrap();
        let restored = restore_revision(
            &fixture.storage,
            &lock,
            &original.project_id,
            &accepted.revision.revision_id,
            &edited.revision.revision_id,
        )
        .unwrap();
        let rejected = reject_draft(
            &fixture.storage,
            &lock,
            &original.project_id,
            &restored.revision.revision_id,
            "Keep the narrated draft for review.",
        )
        .unwrap();
        for revision in [
            &edited.revision,
            &accepted.revision,
            &restored.revision,
            &rejected,
        ] {
            assert_eq!(revision.schema_version, 5);
            assert_eq!(revision.sections[0].spoken_text, speech);
            assert!(revision.sections[0].visual.is_none());
            let bytes = chapters::encode_chapter_revision(revision).unwrap();
            assert_eq!(
                chapters::decode_chapter_revision(&bytes).unwrap(),
                *revision
            );
        }
        let history =
            chapter_store::list_for_project(&fixture.storage, &original.project_id).unwrap();
        for (revision, bytes) in [
            (&original, original_bytes),
            (&edited.revision, edited_bytes),
            (&accepted.revision, accepted_bytes),
        ] {
            assert_eq!(
                chapters::encode_chapter_revision(
                    history
                        .iter()
                        .find(|r| r.revision_id == revision.revision_id)
                        .unwrap()
                )
                .unwrap(),
                bytes
            );
        }
    }

    #[test]
    fn edited_agent_wire_content_validates_without_writing_and_saves_with_trusted_lineage() {
        let fixture = Fixture::new();
        let original = saved_agent_overview(&fixture);
        let original_bytes = chapters::encode_chapter_revision(&original).unwrap();
        let before =
            chapter_store::list_for_project(&fixture.storage, &original.project_id).unwrap();
        let mut proposal = original.clone();
        proposal.sections[0].title.push_str(" — reviewed path");
        // The immutable import path must continue rejecting modified originals.
        let wire = serde_json::to_vec(&proposal).unwrap();
        assert!(chapters::decode_chapter_revision(&wire).is_err());
        let mut decoded = chapters::decode_chapter_edit_proposal(&wire).unwrap();
        decoded.generation_origin.as_mut().unwrap().content_sha256 = "c".repeat(64);
        let report = validate_edits(
            &fixture.storage,
            &original.project_id,
            &original.revision_id,
            decoded.clone(),
        )
        .unwrap();
        assert_eq!(report.status, ValidationReportStatus::Valid, "{report:#?}");
        assert_eq!(
            chapter_store::list_for_project(&fixture.storage, &original.project_id).unwrap(),
            before
        );
        let saved = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &original.project_id,
            &original.revision_id,
            decoded,
        )
        .unwrap();
        assert_eq!(saved.validation.status, ValidationReportStatus::Valid);
        assert_eq!(saved.revision.generation_origin, original.generation_origin);
        assert_eq!(
            saved.revision.based_on_revision_id.as_deref(),
            Some(original.revision_id.as_str())
        );
        assert_ne!(saved.revision.revision_id, original.revision_id);
        let stored =
            chapter_store::list_for_project(&fixture.storage, &original.project_id).unwrap();
        assert_eq!(
            chapters::encode_chapter_revision(
                stored
                    .iter()
                    .find(|r| r.revision_id == original.revision_id)
                    .unwrap()
            )
            .unwrap(),
            original_bytes
        );
    }

    #[test]
    fn edited_agent_validation_reports_content_issues_and_rejects_stale_or_foreign_bases() {
        let fixture = Fixture::new();
        let original = saved_agent_overview(&fixture);
        let mut proposal = original.clone();
        proposal.sections[0].claims[0].anchors[0].source_sha256 = "0".repeat(64);
        let wire = serde_json::to_vec(&proposal).unwrap();
        let decoded = chapters::decode_chapter_edit_proposal(&wire).unwrap();
        let report = validate_edits(
            &fixture.storage,
            &original.project_id,
            &original.revision_id,
            decoded.clone(),
        )
        .unwrap();
        assert_eq!(report.status, ValidationReportStatus::Invalid);
        assert!(!report.issues.is_empty());
        assert!(
            report
                .issues
                .iter()
                .all(|i| !i.message.contains("generation metadata"))
        );
        let mut foreign = decoded.clone();
        foreign.snapshot_id = "foreign-snapshot".into();
        assert!(
            validate_edits(
                &fixture.storage,
                &original.project_id,
                &original.revision_id,
                foreign
            )
            .is_err()
        );
        let invalid = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &original.project_id,
            &original.revision_id,
            decoded.clone(),
        )
        .unwrap();
        assert_eq!(invalid.validation.status, ValidationReportStatus::Invalid);
        assert!(
            accept_draft(
                &fixture.storage,
                &Mutex::new(()),
                &original.project_id,
                &invalid.revision.revision_id
            )
            .is_err()
        );
        assert!(
            validate_edits(
                &fixture.storage,
                &original.project_id,
                &original.revision_id,
                decoded
            )
            .is_err()
        );
    }

    #[test]
    fn edit_wire_decoder_retains_strict_json_schema_and_size_guards() {
        let fixture = Fixture::new();
        let original = saved_agent_overview(&fixture);
        let encoded = serde_json::to_string(&original).unwrap();
        let duplicate = encoded.replacen('{', "{\"schemaVersion\":4,", 1);
        assert!(chapters::decode_chapter_edit_proposal(duplicate.as_bytes()).is_err());
        let mut value = serde_json::to_value(&original).unwrap();
        value["unknownRoot"] = true.into();
        assert!(
            chapters::decode_chapter_edit_proposal(&serde_json::to_vec(&value).unwrap()).is_err()
        );
        value.as_object_mut().unwrap().remove("unknownRoot");
        value["schemaVersion"] = 99.into();
        assert!(
            chapters::decode_chapter_edit_proposal(&serde_json::to_vec(&value).unwrap()).is_err()
        );
        value["schemaVersion"] = 4.into();
        assert!(
            chapters::decode_chapter_edit_proposal(&serde_json::to_vec(&value).unwrap())
                .unwrap_err()
                .contains("not supported")
        );
        value["schemaVersion"] = chapters::CHAPTER_SCHEMA_VERSION.into();
        value["generationOrigin"] = serde_json::Value::Null;
        assert!(
            chapters::decode_chapter_edit_proposal(&serde_json::to_vec(&value).unwrap()).is_err()
        );
        assert!(chapters::decode_chapter_edit_proposal(&vec![b' '; 4 * 1024 * 1024 + 1]).is_err());
    }

    #[test]
    fn edited_sections_accept_as_append_only_revisions_without_changing_old_payloads_or_source() {
        let fixture = Fixture::new();
        let original = first_saved_draft(&fixture);
        let original_bytes = chapters::encode_chapter_revision(&original).unwrap();
        let mut proposal = original.clone();
        proposal.title = "How greeting constructs its return value".to_owned();
        proposal.sections[0].spoken_text =
            "greeting returns a Hello template string with the name.".to_owned();
        let result = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &original.project_id,
            &original.revision_id,
            proposal,
        )
        .unwrap();
        assert_eq!(result.validation.status, ValidationReportStatus::Valid);
        assert_eq!(result.revision.lifecycle, RevisionLifecycle::Draft);
        assert_eq!(
            result.revision.based_on_revision_id.as_deref(),
            Some(original.revision_id.as_str())
        );
        assert_ne!(
            result.revision.sections[0].section_revision_id,
            original.sections[0].section_revision_id,
            "only the section with edited content should receive a new section revision"
        );
        assert_eq!(
            result.revision.sections[1].section_revision_id,
            original.sections[1].section_revision_id,
            "an untouched section should keep its content revision"
        );
        assert_eq!(
            chapters::encode_chapter_revision(&original).unwrap(),
            original_bytes
        );

        let accepted = accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &original.project_id,
            &result.revision.revision_id,
        )
        .unwrap();
        assert_eq!(accepted.validation.status, ValidationReportStatus::Valid);
        assert_eq!(accepted.validation.chapter_id, accepted.revision.chapter_id);
        assert_eq!(
            accepted.validation.revision_id,
            accepted.revision.revision_id
        );
        assert_eq!(
            accepted.validation.snapshot_id,
            accepted.revision.snapshot_id
        );
        assert_ne!(accepted.validation.revision_id, result.revision.revision_id);
        assert_eq!(accepted.revision.lifecycle, RevisionLifecycle::Accepted);
        assert_eq!(
            accepted.revision.based_on_revision_id.as_deref(),
            Some(result.revision.revision_id.as_str())
        );
        let history =
            chapter_store::list_for_project(&fixture.storage, &original.project_id).unwrap();
        assert_eq!(history.len(), 3);
        assert_eq!(history[0], original);
        assert_eq!(history[1], result.revision);
        assert_eq!(history[2], accepted.revision);
        let source = snapshot::read_snapshot_source_from_manifest(
            &fixture.storage,
            &fixture.manifest,
            SIMPLE_PATH,
        )
        .unwrap();
        assert!(source.contents.contains("return `Hello, ${name}`;"));
    }

    #[test]
    fn restore_appends_a_new_draft_and_keeps_both_accepted_payloads_and_assets_immutable() {
        let fixture = Fixture::new();
        let initial_draft = first_saved_draft(&fixture);
        let mut with_visual = initial_draft.clone();
        let anchor_id = with_visual.sections[0].claims[0].anchors[0]
            .anchor_id
            .clone();
        with_visual.sections[0].visual =
            Some(chapters::ChapterVisualPayload::from(chapters::VisualSpec {
                presentation: None,
                schema_version: 2,
                kind: chapters::VisualKind::DataFlow,
                title: "Greeting return flow".to_owned(),
                layout_hint: Some(chapters::VisualLayoutHint {
                    direction: chapters::VisualLayoutDirection::LeftToRight,
                }),
                nodes: vec![chapters::VisualNode {
                    node_id: "node:greeting".to_owned(),
                    label: "greeting".to_owned(),
                    detail: Some("returns the Hello template".to_owned()),
                    hotspot_id: None,
                    evidence_anchor_ids: vec![anchor_id],
                    source_anchor_id: None,
                    source_value: None,
                }],
                edges: Vec::new(),
            }));
        let visual_draft = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &initial_draft.project_id,
            &initial_draft.revision_id,
            with_visual,
        )
        .unwrap();
        assert_eq!(
            visual_draft.validation.status,
            ValidationReportStatus::Valid
        );
        let first_accepted = accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &initial_draft.project_id,
            &visual_draft.revision.revision_id,
        )
        .unwrap()
        .revision;
        let first_bytes = chapters::encode_chapter_revision(&first_accepted).unwrap();
        let first_assets = crate::chapter_freshness::asset_ids_for_revision(&first_accepted);
        assert_eq!(first_assets.len(), 1);

        let mut changed = first_accepted.clone();
        changed.title = "The revised greeting explanation".to_owned();
        changed.sections[0].prose =
            "greeting returns the name inside a more carefully described Hello template string."
                .to_owned();
        changed.sections[0].spoken_text =
            "greeting places the name in its returned Hello template string.".to_owned();
        if let Some(visual) = changed.sections[0]
            .visual
            .as_mut()
            .and_then(chapters::ChapterVisualPayload::typed_visual_mut)
        {
            visual.nodes[0].label = "formatted greeting".to_owned();
        }
        let second_draft = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &first_accepted.project_id,
            &first_accepted.revision_id,
            changed,
        )
        .unwrap();
        assert_eq!(
            second_draft.validation.status,
            ValidationReportStatus::Valid
        );
        let second_accepted = accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &first_accepted.project_id,
            &second_draft.revision.revision_id,
        )
        .unwrap()
        .revision;
        let second_bytes = chapters::encode_chapter_revision(&second_accepted).unwrap();
        assert_ne!(
            crate::chapter_freshness::asset_ids_for_revision(&second_accepted),
            first_assets
        );

        let restored = restore_revision(
            &fixture.storage,
            &Mutex::new(()),
            &first_accepted.project_id,
            &second_accepted.revision_id,
            &first_accepted.revision_id,
        )
        .unwrap();
        assert_eq!(restored.validation.status, ValidationReportStatus::Valid);
        assert_eq!(restored.revision.lifecycle, RevisionLifecycle::Draft);
        assert_eq!(restored.revision.title, first_accepted.title);
        assert_eq!(
            restored.revision.sections[0].visual,
            first_accepted.sections[0].visual
        );
        assert_eq!(
            crate::chapter_freshness::asset_ids_for_revision(&restored.revision),
            first_assets
        );
        assert_eq!(
            restored.revision.based_on_revision_id.as_deref(),
            Some(second_accepted.revision_id.as_str())
        );
        assert_ne!(restored.revision.revision_id, first_accepted.revision_id);
        assert_ne!(restored.revision.revision_id, second_accepted.revision_id);
        assert_ne!(
            restored.revision.sections[0].section_revision_id,
            first_accepted.sections[0].section_revision_id
        );
        assert_eq!(
            chapters::encode_chapter_revision(&first_accepted).unwrap(),
            first_bytes
        );
        assert_eq!(
            chapters::encode_chapter_revision(&second_accepted).unwrap(),
            second_bytes
        );
        let history =
            chapter_store::list_for_project(&fixture.storage, &first_accepted.project_id).unwrap();
        assert!(history.contains(&first_accepted));
        assert!(history.contains(&second_accepted));
        assert!(history.contains(&restored.revision));
    }

    #[test]
    fn restore_from_an_old_snapshot_keeps_unmappable_citations_invalid_and_unaccepted() {
        let mut fixture = Fixture::new();
        let old_draft = first_saved_draft(&fixture);
        let old_accepted = accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &old_draft.project_id,
            &old_draft.revision_id,
        )
        .unwrap()
        .revision;
        let old_bytes = chapters::encode_chapter_revision(&old_accepted).unwrap();

        let changed_source = crate::chapter_generation::tests::SIMPLE_SOURCE.replace(
            "return `Hello, ${name}`;",
            "return `Hello, ${name.trim()}`;",
        );
        fixture.recapture_with_source(SIMPLE_PATH, &changed_source);
        let mut proposal = simple_proposal();
        let updated_quote =
            "function greeting(name: string): string {\n    return `Hello, ${name.trim()}`;\n}";
        let updated_prose =
            "greeting trims name before placing it in the returned Hello template string.";
        proposal.sections[0].prose = updated_prose.to_owned();
        proposal.sections[0].spoken_text = updated_prose.to_owned();
        proposal.sections[0].claims[0].prose = updated_prose.to_owned();
        proposal.sections[0].claims[0].evidence[0].quote = updated_quote.to_owned();
        proposal.evidence_limits[0].evidence[0].quote = updated_quote.to_owned();
        let current = chapter_generation::canonicalize_file_chapter(
            &fixture.manifest.summary.project_id,
            &fixture.manifest,
            Some(&fixture.index),
            SIMPLE_PATH,
            &fixture.sources(&[SIMPLE_PATH]),
            &HashSet::new(),
            &serde_json::to_string(&proposal).unwrap(),
        )
        .unwrap();
        let mut current = current;
        current.validation_status = ValidationStatus::Valid;
        let current_draft = chapter_store::save_new_draft(&fixture.storage, current).unwrap();
        let current_accepted = accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &old_accepted.project_id,
            &current_draft.revision_id,
        )
        .unwrap()
        .revision;
        let current_bytes = chapters::encode_chapter_revision(&current_accepted).unwrap();

        let restored = restore_revision(
            &fixture.storage,
            &Mutex::new(()),
            &old_accepted.project_id,
            &current_accepted.revision_id,
            &old_accepted.revision_id,
        )
        .unwrap();
        assert_eq!(restored.revision.lifecycle, RevisionLifecycle::Draft);
        assert_eq!(restored.validation.status, ValidationReportStatus::Invalid);
        assert!(restored.validation.issues.iter().any(|issue| {
            issue.code == chapters::ChapterIssueCode::WrongSnapshot
                && issue.section_id.as_deref() == Some(old_accepted.sections[0].section_id.as_str())
                && issue.anchor_id.is_some()
        }));
        assert!(
            accept_draft(
                &fixture.storage,
                &Mutex::new(()),
                &old_accepted.project_id,
                &restored.revision.revision_id,
            )
            .unwrap_err()
            .contains("cannot be accepted")
        );
        assert_eq!(
            chapters::encode_chapter_revision(&old_accepted).unwrap(),
            old_bytes
        );
        assert_eq!(
            chapters::encode_chapter_revision(&current_accepted).unwrap(),
            current_bytes
        );
        assert!(
            chapter_store::list_for_project(&fixture.storage, &old_accepted.project_id)
                .unwrap()
                .contains(&restored.revision)
        );
    }

    #[test]
    fn rejection_reason_is_preserved_and_invalid_claims_block_accept_until_repaired() {
        let fixture = Fixture::new();
        let original = first_saved_draft(&fixture);
        let original_payload = chapters::encode_chapter_revision(&original).unwrap();
        let rejected = reject_draft(
            &fixture.storage,
            &Mutex::new(()),
            &original.project_id,
            &original.revision_id,
            "The explanation should state the return behavior more directly.",
        )
        .unwrap();
        assert_eq!(rejected.lifecycle, RevisionLifecycle::Rejected);
        assert_eq!(
            rejected.decision_note.as_deref(),
            Some("The explanation should state the return behavior more directly.")
        );
        assert!(
            accept_draft(
                &fixture.storage,
                &Mutex::new(()),
                &original.project_id,
                &rejected.revision_id,
            )
            .is_err()
        );

        let mut invalid_candidate = rejected.clone();
        invalid_candidate.sections[0].claims[0].anchors[0].end_byte_inclusive = u64::MAX;
        let invalid = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &original.project_id,
            &rejected.revision_id,
            invalid_candidate,
        )
        .unwrap();
        assert_eq!(invalid.revision.lifecycle, RevisionLifecycle::Draft);
        assert_eq!(
            invalid.revision.validation_status,
            ValidationStatus::Invalid
        );
        assert!(
            invalid
                .validation
                .issues
                .iter()
                .any(|issue| issue.code == chapters::ChapterIssueCode::RangeMismatch)
        );
        assert!(
            accept_draft(
                &fixture.storage,
                &Mutex::new(()),
                &original.project_id,
                &invalid.revision.revision_id,
            )
            .is_err()
        );

        let mut repaired_candidate = invalid.revision.clone();
        repaired_candidate.sections[0].claims[0].anchors[0] =
            original.sections[0].claims[0].anchors[0].clone();
        let repaired = save_edits(
            &fixture.storage,
            &Mutex::new(()),
            &original.project_id,
            &invalid.revision.revision_id,
            repaired_candidate,
        )
        .unwrap();
        assert_eq!(repaired.validation.status, ValidationReportStatus::Valid);
        let accepted = accept_draft(
            &fixture.storage,
            &Mutex::new(()),
            &original.project_id,
            &repaired.revision.revision_id,
        )
        .unwrap();
        assert_eq!(accepted.revision.lifecycle, RevisionLifecycle::Accepted);

        let history =
            chapter_store::list_for_project(&fixture.storage, &original.project_id).unwrap();
        assert_eq!(history[0], original);
        assert_eq!(
            chapters::encode_chapter_revision(&history[0]).unwrap(),
            original_payload
        );
        assert_eq!(history[1], rejected);
        assert_eq!(history[2], invalid.revision);
        assert_eq!(history[3], repaired.revision);
        assert_eq!(history[4], accepted.revision);
    }
}
