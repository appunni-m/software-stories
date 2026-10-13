use crate::chapter_store;
use crate::chapters::{
    self, ChapterRevision, RevisionLifecycle, ValidationReportStatus, ValidationStatus,
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
    if base.project_id != proposal.project_id
        || base.snapshot_id != proposal.snapshot_id
        || base.chapter_id != proposal.chapter_id
        || base.target != proposal.target
    {
        return Err(
            "Chapter edits must keep the selected project, snapshot, and target.".to_owned(),
        );
    }

    let manifest = snapshot::load_snapshot_manifest(storage_dir, project_id, &base.snapshot_id)?;
    let snapshot_index = index::load_snapshot_index(storage_dir, project_id, &base.snapshot_id)?;
    let mut revision = base.clone();
    revision.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
    revision.revision_id = new_id("revision");
    revision.revision_number = base
        .revision_number
        .checked_add(1)
        .ok_or_else(|| "The chapter revision number is exhausted.".to_owned())?;
    revision.title = proposal.title;
    revision.sections = version_edited_sections(&base, proposal.sections);
    revision.lifecycle = RevisionLifecycle::Draft;
    revision.validation_status = ValidationStatus::Unvalidated;
    revision.created_at_unix_ms = unix_time_ms();
    revision.based_on_revision_id = Some(base.revision_id.clone());
    revision.decision_note = None;

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
