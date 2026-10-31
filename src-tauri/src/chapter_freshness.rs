use crate::chapters::{
    self, ChapterHotspot, ChapterRevision, ChapterSection, ChapterSourceAnchor,
    ChapterVisualPayload, HotspotDestination, RevisionLifecycle, SectionDependency, VisualSpec,
};
use crate::index::{
    IndexFileStatus, IndexedRelationship, IndexedSymbol, RelationshipKind, SnapshotIndex,
    SnapshotIndexStatus, SourceAnchor,
};
use crate::snapshot::{self, CaptureStatus, InventoryPathKind, SnapshotManifest, SnapshotSource};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterFreshnessReport {
    pub chapter_revision_id: String,
    pub from_snapshot_id: String,
    pub to_snapshot_id: String,
    pub indexes: FreshnessIndexPair,
    pub sections: Vec<SectionFreshness>,
    /// Snapshot-local reverse links used to trace which explanations and visuals
    /// are affected by a source range, symbol, relationship, or file.
    pub reverse_dependencies: Vec<ReverseDependency>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessIndexPair {
    pub from: FreshnessIndexSummary,
    pub to: FreshnessIndexSummary,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessIndexSummary {
    pub status: FreshnessIndexStatus,
    pub indexed_files: Option<usize>,
    pub total_files: Option<usize>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FreshnessIndexStatus {
    Complete,
    Partial,
    Cancelled,
    Unavailable,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionFreshness {
    pub section_id: String,
    pub section_revision_id: String,
    pub status: SectionFreshnessStatus,
    pub evidence_state: EvidenceState,
    pub reasons: Vec<FreshnessReason>,
    pub source_changes: Vec<FreshnessSourceChange>,
    pub narration_script_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SectionFreshnessStatus {
    Current,
    SafeToCarry,
    Stale,
    NeedsReview,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EvidenceState {
    Complete,
    Partial,
    Unavailable,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessReason {
    pub code: String,
    pub message: String,
    pub path: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessSourceChange {
    pub path: String,
    pub kind: String,
    pub old_range: Option<FreshnessLineRange>,
    pub new_range: Option<FreshnessLineRange>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FreshnessLineRange {
    /// Path containing this particular side of the change. It can differ from
    /// `FreshnessSourceChange::path` when a relationship endpoint is removed.
    pub path: String,
    /// Immutable snapshot whose captured source bytes this range identifies.
    pub snapshot_id: String,
    /// Hash of the captured file bytes, used by Code Lens to reject stale ranges.
    pub source_sha256: String,
    pub start_line: u32,
    pub end_line: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseDependency {
    pub evidence_key: String,
    pub path: String,
    pub dependent_section_ids: Vec<String>,
    pub asset_ids: Vec<String>,
}

#[derive(Default)]
struct SectionAssessment {
    reasons: Vec<FreshnessReason>,
    changes: Vec<FreshnessSourceChange>,
    proven_stale: bool,
    safe_to_carry: bool,
    unknown_count: usize,
    checked_count: usize,
}

impl SectionAssessment {
    fn stale(
        &mut self,
        code: &str,
        message: impl Into<String>,
        path: Option<&str>,
        change: Option<FreshnessSourceChange>,
    ) {
        self.proven_stale = true;
        self.checked_count += 1;
        self.reasons.push(FreshnessReason {
            code: code.to_owned(),
            message: message.into(),
            path: path.map(str::to_owned),
        });
        if let Some(change) = change {
            push_unique_change(&mut self.changes, change);
        }
    }

    fn unknown(&mut self, code: &str, message: impl Into<String>, path: Option<&str>) {
        self.unknown_count += 1;
        self.reasons.push(FreshnessReason {
            code: code.to_owned(),
            message: message.into(),
            path: path.map(str::to_owned),
        });
    }

    fn safe(&mut self, change: Option<FreshnessSourceChange>) {
        self.safe_to_carry = true;
        self.checked_count += 1;
        if let Some(change) = change {
            push_unique_change(&mut self.changes, change);
        }
    }

    fn current(&mut self) {
        self.checked_count += 1;
    }

    fn finish(self, section: &ChapterSection) -> SectionFreshness {
        let status = if self.proven_stale {
            SectionFreshnessStatus::Stale
        } else if self.unknown_count > 0 {
            SectionFreshnessStatus::NeedsReview
        } else if self.safe_to_carry {
            SectionFreshnessStatus::SafeToCarry
        } else {
            SectionFreshnessStatus::Current
        };
        let evidence_state = if self.unknown_count == 0 {
            EvidenceState::Complete
        } else if self.checked_count > 0 {
            EvidenceState::Partial
        } else {
            EvidenceState::Unavailable
        };
        SectionFreshness {
            section_id: section.section_id.clone(),
            section_revision_id: section.section_revision_id.clone(),
            status,
            evidence_state,
            reasons: self.reasons,
            source_changes: self.changes,
            narration_script_sha256: if section.spoken_text.is_empty() {
                None
            } else {
                let digest = Sha256::digest(section.spoken_text.as_bytes());
                Some(digest.iter().map(|byte| format!("{byte:02x}")).collect())
            },
        }
    }
}

pub fn classify_chapter_freshness(
    storage_dir: &Path,
    from_manifest: &SnapshotManifest,
    from_index: Option<&SnapshotIndex>,
    to_manifest: &SnapshotManifest,
    to_index: Option<&SnapshotIndex>,
    revision: &ChapterRevision,
) -> Result<ChapterFreshnessReport, String> {
    let from_id = from_manifest.summary.snapshot_id.as_str();
    let to_id = to_manifest.summary.snapshot_id.as_str();
    if revision.lifecycle != RevisionLifecycle::Accepted {
        return Err("Freshness can only be assessed for an accepted chapter revision.".to_owned());
    }
    if revision.project_id != from_manifest.summary.project_id
        || revision.project_id != to_manifest.summary.project_id
        || revision.snapshot_id != from_id
        || from_manifest.summary.project_id != to_manifest.summary.project_id
    {
        return Err("The accepted chapter and comparison snapshots must belong to the same project, and the source snapshot must match the chapter.".to_owned());
    }
    crate::chapter_origin::validate_revision_origin(revision)?;
    if let Some(index) = from_index {
        if index.project_id != revision.project_id || index.snapshot_id != from_id {
            return Err(
                "The source symbol index does not match the accepted chapter snapshot.".to_owned(),
            );
        }
    }
    if let Some(index) = to_index {
        if index.project_id != revision.project_id || index.snapshot_id != to_id {
            return Err("The target symbol index does not match the selected snapshot.".to_owned());
        }
    }
    let retrieval_ranges = revision
        .generation_origin
        .as_ref()
        .and_then(crate::chapter_origin::ChapterGenerationOrigin::retrieval_ranges);
    let retrieval_index_identity = revision
        .generation_origin
        .as_ref()
        .and_then(crate::chapter_origin::ChapterGenerationOrigin::retrieval_index_identity);
    let retrieval_index_dependencies =
        retrieved_index_dependencies(revision, from_manifest, from_index);

    let sections = revision
        .sections
        .iter()
        .map(|section| {
            let assessment = assess_section(
                storage_dir,
                from_manifest,
                from_index,
                to_manifest,
                to_index,
                section,
                retrieval_ranges,
                retrieval_index_identity.as_deref(),
                &retrieval_index_dependencies,
            );
            assessment.finish(section)
        })
        .collect();

    Ok(ChapterFreshnessReport {
        chapter_revision_id: revision.revision_id.clone(),
        from_snapshot_id: from_id.to_owned(),
        to_snapshot_id: to_id.to_owned(),
        indexes: FreshnessIndexPair {
            from: index_summary(from_index, from_manifest),
            to: index_summary(to_index, to_manifest),
        },
        sections,
        reverse_dependencies: build_reverse_dependencies(revision, &retrieval_index_dependencies),
    })
}

fn index_summary(
    index: Option<&SnapshotIndex>,
    manifest: &SnapshotManifest,
) -> FreshnessIndexSummary {
    let Some(index) = index else {
        return FreshnessIndexSummary {
            status: FreshnessIndexStatus::Unavailable,
            indexed_files: None,
            total_files: Some(manifest.files.len()),
        };
    };
    let status = match &index.status {
        SnapshotIndexStatus::Complete => FreshnessIndexStatus::Complete,
        SnapshotIndexStatus::Partial => FreshnessIndexStatus::Partial,
        SnapshotIndexStatus::Cancelled => FreshnessIndexStatus::Cancelled,
    };
    FreshnessIndexSummary {
        status,
        indexed_files: Some(
            index
                .files
                .iter()
                .filter(|file| {
                    file.status == IndexFileStatus::Indexed
                        && manifest.files.iter().any(|captured| {
                            captured.path == file.path && captured.sha256 == file.source_sha256
                        })
                })
                .count(),
        ),
        total_files: Some(manifest.files.len()),
    }
}

fn assess_section(
    storage_dir: &Path,
    from_manifest: &SnapshotManifest,
    from_index: Option<&SnapshotIndex>,
    to_manifest: &SnapshotManifest,
    to_index: Option<&SnapshotIndex>,
    section: &ChapterSection,
    retrieval_ranges: Option<&[crate::repository_context::CodeRange]>,
    retrieval_index_identity: Option<&str>,
    retrieval_index_dependencies: &[SectionDependency],
) -> SectionAssessment {
    let mut result = SectionAssessment::default();
    let mut anchors = HashMap::<String, &ChapterSourceAnchor>::new();
    let mut dependencies = BTreeSet::<DependencyKey>::new();
    for anchor in section.claims.iter().flat_map(|claim| &claim.anchors) {
        anchors.insert(anchor.anchor_id.clone(), anchor);
    }
    dependencies.extend(section.dependencies.iter().map(DependencyKey::from));
    for claim in &section.claims {
        dependencies.extend(claim.dependencies.iter().map(DependencyKey::from));
    }
    dependencies.extend(retrieval_index_dependencies.iter().map(DependencyKey::from));

    let mut anchored_paths = HashSet::new();
    for anchor in anchors.values() {
        anchored_paths.insert(anchor.path.as_str());
        assess_anchor(
            storage_dir,
            from_manifest,
            from_index,
            to_manifest,
            to_index,
            anchor,
            &mut result,
        );
    }

    // Progressive explanations retain the exact chunks read while generating
    // each section. Check those chunks and their enclosing symbols so an edit
    // elsewhere in the same file does not invalidate the whole explanation.
    let mut verified_ranges = BTreeMap::new();
    let mut retrieval_file_hashes = BTreeMap::<String, String>::new();
    for range in retrieval_ranges.unwrap_or_default() {
        let previous_hash = retrieval_file_hashes
            .entry(range.path.clone())
            .or_insert_with(|| range.source_sha256.clone());
        if previous_hash != &range.source_sha256 {
            result.unknown(
                "retrievalEvidenceConflict",
                "The section's recorded retrieval ranges disagree about a source file version.",
                Some(&range.path),
            );
            continue;
        }
        verified_ranges.insert(
            (
                range.path.clone(),
                range.source_sha256.clone(),
                range.start_byte,
                range.end_byte,
                range.chunk_sha256.clone(),
            ),
            range,
        );
    }
    for range in verified_ranges.values() {
        assess_retrieval_range_dependency(
            storage_dir,
            from_manifest,
            to_manifest,
            range,
            &mut result,
        );
    }
    if let Some(ranges) = retrieval_ranges {
        assess_retrieval_index_context(
            from_manifest,
            from_index,
            to_manifest,
            to_index,
            ranges,
            retrieval_index_identity,
            &mut result,
        );
    }

    for dependency in dependencies {
        match dependency {
            DependencyKey::SourceFile {
                path,
                source_sha256,
            } => {
                if anchored_paths.contains(path.as_str()) {
                    // The exact anchors on this file carry the more precise dependency.
                    continue;
                }
                assess_file_dependency(
                    storage_dir,
                    from_manifest,
                    to_manifest,
                    &path,
                    &source_sha256,
                    &mut result,
                );
            }
            DependencyKey::Symbol {
                path,
                source_sha256,
                symbol_id,
            } => assess_symbol_dependency(
                storage_dir,
                from_manifest,
                from_index,
                to_manifest,
                to_index,
                &path,
                &source_sha256,
                &symbol_id,
                &mut result,
            ),
            DependencyKey::Relationship {
                path,
                source_sha256,
                relationship_id,
            } => assess_relationship_dependency(
                storage_dir,
                from_manifest,
                from_index,
                to_manifest,
                to_index,
                &path,
                &source_sha256,
                &relationship_id,
                &mut result,
            ),
        }
    }

    // An explanation with no source or index dependencies contains no evidence
    // claim for this comparison and remains current.
    if result.checked_count == 0 && result.unknown_count == 0 && !result.proven_stale {
        result.current();
    }
    result
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DependencyKey {
    SourceFile {
        path: String,
        source_sha256: String,
    },
    Symbol {
        path: String,
        source_sha256: String,
        symbol_id: String,
    },
    Relationship {
        path: String,
        source_sha256: String,
        relationship_id: String,
    },
}

impl From<&SectionDependency> for DependencyKey {
    fn from(value: &SectionDependency) -> Self {
        match value {
            SectionDependency::SourceFile {
                path,
                source_sha256,
            } => Self::SourceFile {
                path: path.clone(),
                source_sha256: source_sha256.clone(),
            },
            SectionDependency::Symbol {
                path,
                source_sha256,
                symbol_id,
            } => Self::Symbol {
                path: path.clone(),
                source_sha256: source_sha256.clone(),
                symbol_id: symbol_id.clone(),
            },
            SectionDependency::Relationship {
                path,
                source_sha256,
                relationship_id,
            } => Self::Relationship {
                path: path.clone(),
                source_sha256: source_sha256.clone(),
                relationship_id: relationship_id.clone(),
            },
        }
    }
}

fn assess_anchor(
    storage_dir: &Path,
    from_manifest: &SnapshotManifest,
    from_index: Option<&SnapshotIndex>,
    to_manifest: &SnapshotManifest,
    to_index: Option<&SnapshotIndex>,
    anchor: &ChapterSourceAnchor,
    result: &mut SectionAssessment,
) {
    let from_file = from_manifest
        .files
        .iter()
        .find(|file| file.path == anchor.path);
    let Some(from_file) = from_file else {
        result.unknown(
            "sourceUnavailable",
            "The cited source is absent from the accepted revision's snapshot manifest.",
            Some(&anchor.path),
        );
        return;
    };
    if from_file.sha256 != anchor.source_sha256 {
        result.unknown(
            "sourceHashMismatch",
            "The accepted citation hash does not match its captured snapshot manifest.",
            Some(&anchor.path),
        );
        return;
    }
    let old_source = match read_source(storage_dir, from_manifest, &anchor.path) {
        Ok(source) => source,
        Err(message) => {
            result.unknown("sourceUnavailable", message, Some(&anchor.path));
            return;
        }
    };
    if !anchor_matches_source(anchor, &old_source) {
        result.unknown(
            "citationInvalid",
            "The saved citation does not match its captured source bytes.",
            Some(&anchor.path),
        );
        return;
    }
    let Some(new_file) = to_manifest
        .files
        .iter()
        .find(|file| file.path == anchor.path)
    else {
        if path_was_skipped(to_manifest, &anchor.path)
            || to_manifest.summary.status != CaptureStatus::Complete
        {
            result.unknown("sourceNotCaptured", "The target snapshot could not confirm whether this path changed because the path was skipped or capture was incomplete.", Some(&anchor.path));
        } else {
            result.stale(
                "sourceDeleted",
                "The cited source file is absent from the complete target snapshot.",
                Some(&anchor.path),
                Some(FreshnessSourceChange {
                    path: anchor.path.clone(),
                    kind: "deleted".to_owned(),
                    old_range: Some(chapter_anchor_range(anchor)),
                    new_range: None,
                }),
            );
        }
        return;
    };

    if new_file.sha256 == anchor.source_sha256 {
        match read_source(storage_dir, to_manifest, &anchor.path) {
            Ok(source) if anchor_matches_source(anchor, &source) => result.current(),
            Ok(_) => result.unknown("citationInvalid", "The target snapshot has the same declared file hash, but the cited bytes do not verify.", Some(&anchor.path)),
            Err(message) => result.unknown("sourceUnavailable", message, Some(&anchor.path)),
        }
        return;
    }

    match read_source(storage_dir, to_manifest, &anchor.path) {
        Ok(source) if source.sha256 == new_file.sha256 => {}
        Ok(_) => {
            result.unknown(
                "sourceHashMismatch",
                "The target snapshot source bytes do not match the manifest hash.",
                Some(&anchor.path),
            );
            return;
        }
        Err(message) => {
            result.unknown("sourceUnavailable", message, Some(&anchor.path));
            return;
        }
    }
    if anchor.symbol_id.is_none() {
        result.unknown("stableSymbolUnavailable", "This citation has no stable symbol identity, so changed source cannot be remapped safely.", Some(&anchor.path));
        return;
    }

    let remapped = chapters::remap_anchor_between_snapshots(
        storage_dir,
        from_manifest,
        from_index,
        to_manifest,
        to_index,
        anchor,
    );
    if let Some(new_anchor) = remapped
        .anchor
        .filter(|_| matches!(remapped.status, chapters::AnchorRemapStatus::Remapped))
    {
        let moved = anchor.start_byte != new_anchor.start_byte
            || anchor.end_byte_inclusive != new_anchor.end_byte_inclusive;
        if moved {
            result.safe(Some(FreshnessSourceChange {
                path: anchor.path.clone(),
                kind: "shifted".to_owned(),
                old_range: Some(chapter_anchor_range(anchor)),
                new_range: Some(chapter_anchor_range(&new_anchor)),
            }));
        } else {
            result.current();
        }
        return;
    }

    let issue = remapped.issues.first();
    match issue.map(|issue| &issue.code) {
        Some(chapters::ChapterIssueCode::IndexUnavailable) => result.unknown(
            "indexIncomplete",
            issue.map(|issue| issue.message.clone()).unwrap_or_else(|| "A relevant source index is incomplete.".to_owned()),
            Some(&anchor.path),
        ),
        Some(chapters::ChapterIssueCode::SourceUnavailable | chapters::ChapterIssueCode::WrongSnapshot) => result.unknown(
            "sourceUnavailable",
            issue.map(|issue| issue.message.clone()).unwrap_or_else(|| "Source evidence could not be verified.".to_owned()),
            Some(&anchor.path),
        ),
        Some(chapters::ChapterIssueCode::MissingPath) => result.unknown(
            "sourceUnavailable",
            "The target manifest lists this file, but its captured source bytes could not be read and verified.",
            Some(&anchor.path),
        ),
        Some(chapters::ChapterIssueCode::MissingSymbol) => {
            let old_symbol_present = from_index.is_some_and(|index| {
                index.symbols.iter().any(|symbol| symbol.symbol_id == anchor.symbol_id.as_deref().unwrap_or_default() && symbol.path == anchor.path)
            });
            let old_index_clean = from_index.is_some_and(|index| file_indexed_cleanly(index, from_manifest, &anchor.path));
            let new_index_clean = to_index.is_some_and(|index| file_indexed_cleanly(index, to_manifest, &anchor.path));
            if !old_symbol_present || !old_index_clean || !new_index_clean {
                result.unknown(
                    "indexIncomplete",
                    "The target symbol is absent from an index that is not complete enough to prove its removal.",
                    Some(&anchor.path),
                );
            } else {
                result.stale(
                    "symbolChanged",
                    "The cited stable symbol no longer resolves to the same declaration in the completely indexed target file.",
                    Some(&anchor.path),
                    Some(FreshnessSourceChange {
                        path: anchor.path.clone(),
                        kind: "modified".to_owned(),
                        old_range: Some(chapter_anchor_range(anchor)),
                        new_range: None,
                    }),
                );
            }
        }
        Some(chapters::ChapterIssueCode::ExcerptMismatch) => result.stale(
            "citedCodeChanged",
            "The exact cited bytes no longer appear in the same stable symbol.",
            Some(&anchor.path),
            Some(FreshnessSourceChange {
                path: anchor.path.clone(),
                kind: "modified".to_owned(),
                old_range: Some(chapter_anchor_range(anchor)),
                new_range: None,
            }),
        ),
        Some(chapters::ChapterIssueCode::AmbiguousRemap) => result.unknown(
            "ambiguousRemap",
            "The exact excerpt appears more than once in the stable symbol, so no destination range can be chosen safely.",
            Some(&anchor.path),
        ),
        Some(chapters::ChapterIssueCode::RangeMismatch | chapters::ChapterIssueCode::HashMismatch) => result.unknown(
            "remapEvidenceMismatch",
            "The saved range or hash evidence could not be verified against captured source bytes.",
            Some(&anchor.path),
        ),
        _ => result.unknown(
            "needsReview",
            issue.map(|issue| issue.message.clone()).unwrap_or_else(|| "The target citation could not be compared with sufficient evidence.".to_owned()),
            Some(&anchor.path),
        ),
    }
}

fn assess_file_dependency(
    storage_dir: &Path,
    from_manifest: &SnapshotManifest,
    to_manifest: &SnapshotManifest,
    path: &str,
    source_sha256: &str,
    result: &mut SectionAssessment,
) {
    let Some(old_file) = from_manifest.files.iter().find(|file| file.path == path) else {
        result.unknown(
            "sourceUnavailable",
            "The broad file dependency is missing from the accepted snapshot manifest.",
            Some(path),
        );
        return;
    };
    if old_file.sha256 != source_sha256 {
        result.unknown(
            "sourceHashMismatch",
            "The broad file dependency hash does not match its accepted snapshot.",
            Some(path),
        );
        return;
    }
    let Some(new_file) = to_manifest.files.iter().find(|file| file.path == path) else {
        if path_was_skipped(to_manifest, path)
            || to_manifest.summary.status != CaptureStatus::Complete
        {
            result.unknown("sourceNotCaptured", "The target snapshot did not capture this broad file dependency, so its state is unknown.", Some(path));
        } else {
            result.stale(
                "sourceDeleted",
                "A broad source-file dependency is absent from the complete target snapshot.",
                Some(path),
                Some(FreshnessSourceChange {
                    path: path.to_owned(),
                    kind: "deleted".to_owned(),
                    old_range: None,
                    new_range: None,
                }),
            );
        }
        return;
    };
    if new_file.sha256 == source_sha256 {
        match read_source(storage_dir, to_manifest, path) {
            Ok(source) if source.sha256 == source_sha256 => result.current(),
            Ok(_) => result.unknown(
                "sourceHashMismatch",
                "The broad file dependency failed target snapshot hash verification.",
                Some(path),
            ),
            Err(message) => result.unknown("sourceUnavailable", message, Some(path)),
        }
        return;
    }
    // A broad dependency has no cited range to remap, so a changed file cannot
    // safely be carried on anchor evidence alone. First verify both blobs so a
    // storage or hash failure is never misreported as a proven source change.
    let (old_source, new_source) = match (
        read_source(storage_dir, from_manifest, path),
        read_source(storage_dir, to_manifest, path),
    ) {
        (Ok(old_source), Ok(new_source)) => (old_source, new_source),
        _ => {
            result.unknown(
                "sourceUnavailable",
                "A captured file blob needed for this broad dependency comparison could not be verified.",
                Some(path),
            );
            return;
        }
    };
    if old_source.sha256 != old_file.sha256 || new_source.sha256 != new_file.sha256 {
        result.unknown(
            "sourceHashMismatch",
            "A captured file hash did not match its manifest during the broad dependency comparison.",
            Some(path),
        );
        return;
    }
    let old_len = old_source.contents.lines().count() as u32;
    let new_len = new_source.contents.lines().count() as u32;
    result.stale(
        "sourceFileChanged",
        "This section depends on the file as a whole and its captured content changed.",
        Some(path),
        Some(FreshnessSourceChange {
            path: path.to_owned(),
            kind: "modified".to_owned(),
            old_range: (old_len > 0).then(|| {
                range(
                    path,
                    &from_manifest.summary.snapshot_id,
                    &old_file.sha256,
                    1,
                    old_len,
                )
            }),
            new_range: (new_len > 0).then(|| {
                range(
                    path,
                    &to_manifest.summary.snapshot_id,
                    &new_file.sha256,
                    1,
                    new_len,
                )
            }),
        }),
    );
}

fn assess_retrieval_range_dependency(
    storage_dir: &Path,
    from_manifest: &SnapshotManifest,
    to_manifest: &SnapshotManifest,
    retrieved: &crate::repository_context::CodeRange,
    result: &mut SectionAssessment,
) {
    let path = retrieved.path.as_str();
    if retrieved.snapshot_id != from_manifest.summary.snapshot_id {
        result.unknown(
            "retrievalSnapshotMismatch",
            "A retrieved code range belongs to a different accepted snapshot.",
            Some(path),
        );
        return;
    }
    let Some(old_file) = from_manifest.files.iter().find(|file| file.path == path) else {
        result.unknown(
            "sourceUnavailable",
            "A retrieved code range is missing from the accepted snapshot manifest.",
            Some(path),
        );
        return;
    };
    if old_file.sha256 != retrieved.source_sha256 {
        result.unknown(
            "sourceHashMismatch",
            "A retrieved code range does not match its accepted snapshot file hash.",
            Some(path),
        );
        return;
    }
    let old_source = match read_source(storage_dir, from_manifest, path) {
        Ok(source) if source.sha256 == old_file.sha256 => source,
        Ok(_) => {
            result.unknown(
                "sourceHashMismatch",
                "The accepted source bytes for a retrieved code range do not match the manifest.",
                Some(path),
            );
            return;
        }
        Err(message) => {
            result.unknown("sourceUnavailable", message, Some(path));
            return;
        }
    };
    let Some(old_text) = old_source
        .contents
        .get(retrieved.start_byte..retrieved.end_byte)
    else {
        result.unknown(
            "retrievalRangeMismatch",
            "A retrieved code range has invalid or non-UTF-8 byte boundaries.",
            Some(path),
        );
        return;
    };
    if old_text.is_empty()
        || format!("{:x}", Sha256::digest(old_text.as_bytes())) != retrieved.chunk_sha256
    {
        result.unknown(
            "retrievalRangeMismatch",
            "The saved retrieved range does not match its accepted source bytes.",
            Some(path),
        );
        return;
    }
    let old_start_line = byte_position(old_source.contents.as_bytes(), retrieved.start_byte).0;
    let old_end_line = byte_position(old_source.contents.as_bytes(), retrieved.end_byte - 1).0;
    if (old_start_line as usize) != retrieved.start_line
        || (old_end_line as usize) != retrieved.end_line
    {
        result.unknown(
            "retrievalRangeMismatch",
            "The saved retrieved range line numbers do not match its accepted source bytes.",
            Some(path),
        );
        return;
    }

    let Some(new_file) = to_manifest.files.iter().find(|file| file.path == path) else {
        if path_was_skipped(to_manifest, path)
            || to_manifest.summary.status != CaptureStatus::Complete
        {
            result.unknown(
                "sourceNotCaptured",
                "The target snapshot could not confirm the retrieved code range because the file was skipped or capture was incomplete.",
                Some(path),
            );
        } else {
            result.stale(
                "retrievedSourceDeleted",
                "The source file containing this retrieved code range is absent from the complete target snapshot.",
                Some(path),
                Some(FreshnessSourceChange {
                    path: path.to_owned(),
                    kind: "deleted".to_owned(),
                    old_range: Some(range(
                        path,
                        &from_manifest.summary.snapshot_id,
                        &old_file.sha256,
                        old_start_line,
                        old_end_line,
                    )),
                    new_range: None,
                }),
            );
        }
        return;
    };
    let new_source = match read_source(storage_dir, to_manifest, path) {
        Ok(source) if source.sha256 == new_file.sha256 => source,
        Ok(_) => {
            result.unknown(
                "sourceHashMismatch",
                "The target source bytes for a retrieved code range do not match the manifest.",
                Some(path),
            );
            return;
        }
        Err(message) => {
            result.unknown("sourceUnavailable", message, Some(path));
            return;
        }
    };
    if new_file.sha256 == retrieved.source_sha256 {
        result.current();
        return;
    }

    let matches = new_source
        .contents
        .match_indices(old_text)
        .map(|(start, _)| start)
        .take(2)
        .collect::<Vec<_>>();
    let [new_start] = matches.as_slice() else {
        if matches.is_empty() {
            result.stale(
                "retrievedRangeChanged",
                "The exact code range used for this explanation no longer appears in the target source.",
                Some(path),
                Some(FreshnessSourceChange {
                    path: path.to_owned(),
                    kind: "modified".to_owned(),
                    old_range: Some(range(
                        path,
                        &from_manifest.summary.snapshot_id,
                        &old_file.sha256,
                        old_start_line,
                        old_end_line,
                    )),
                    new_range: None,
                }),
            );
        } else {
            result.unknown(
                "retrievalRangeAmbiguous",
                "The exact code range appears more than once in the target source, so its new location is ambiguous.",
                Some(path),
            );
        }
        return;
    };
    let new_end = *new_start + old_text.len();
    let new_start_line = byte_position(new_source.contents.as_bytes(), *new_start).0;
    let new_end_line = byte_position(new_source.contents.as_bytes(), new_end - 1).0;
    result.safe(Some(FreshnessSourceChange {
        path: path.to_owned(),
        kind: if *new_start == retrieved.start_byte {
            "unrelatedEdit".to_owned()
        } else {
            "shifted".to_owned()
        },
        old_range: Some(range(
            path,
            &from_manifest.summary.snapshot_id,
            &old_file.sha256,
            old_start_line,
            old_end_line,
        )),
        new_range: Some(range(
            path,
            &to_manifest.summary.snapshot_id,
            &new_file.sha256,
            new_start_line,
            new_end_line,
        )),
    }));
}

fn assess_retrieval_index_context(
    from_manifest: &SnapshotManifest,
    from_index: Option<&SnapshotIndex>,
    to_manifest: &SnapshotManifest,
    to_index: Option<&SnapshotIndex>,
    ranges: &[crate::repository_context::CodeRange],
    recorded_index_identity: Option<&str>,
    result: &mut SectionAssessment,
) {
    let mut parsers = BTreeMap::<String, Vec<crate::index::ParserVersion>>::new();
    let mut symbols = BTreeMap::<String, BTreeSet<String>>::new();
    for range in ranges {
        let path = &range.path;
        if let Some(parser) = &range.parser {
            let expected = parsers.entry(path.clone()).or_default();
            if !expected.contains(parser) {
                expected.push(parser.clone());
            }
        }
        for context in [&range.enclosing, &range.parent].into_iter().flatten() {
            symbols
                .entry(path.clone())
                .or_default()
                .insert(context.symbol_id.clone());
        }
    }
    let paths = parsers
        .keys()
        .chain(symbols.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    if paths.is_empty() {
        return;
    }

    let example_path = paths.iter().next().map(String::as_str);
    let (Some(from_index), Some(to_index)) = (from_index, to_index) else {
        result.unknown(
            "retrievalIndexUnavailable",
            "The indexes needed to compare this section's retrieved code context are unavailable.",
            example_path,
        );
        return;
    };
    let Some(recorded_index_identity) = recorded_index_identity else {
        result.unknown(
            "retrievalIndexIdentityUnavailable",
            "This section did not record the local index identity used during generation.",
            example_path,
        );
        return;
    };
    let from_identity = format!(
        "{}:{}",
        from_index.schema_version, from_index.indexer_version
    );
    let to_identity = format!("{}:{}", to_index.schema_version, to_index.indexer_version);
    if from_identity != recorded_index_identity || to_identity != recorded_index_identity {
        result.unknown(
            "retrievalIndexVersionChanged",
            "The local code index changed versions since this section was generated; review its retrieved context.",
            example_path,
        );
        return;
    }
    if from_index.status != SnapshotIndexStatus::Complete
        || to_index.status != SnapshotIndexStatus::Complete
    {
        result.unknown(
            "retrievalIndexIncomplete",
            "A complete local code index is needed to confirm the retrieved symbol and relationship context.",
            example_path,
        );
        return;
    }

    let mut changed_relationship_paths = BTreeSet::new();
    for path in paths {
        let expected_parsers = parsers.get(&path).cloned().unwrap_or_default();
        if expected_parsers.len() > 1 {
            result.unknown(
                "retrievalIndexConflict",
                "The section's recorded retrieval ranges disagree about a file's parser identity.",
                Some(&path),
            );
            continue;
        }
        let Some(old_file) = from_index.files.iter().find(|file| file.path == path) else {
            result.unknown(
                "retrievalIndexUnavailable",
                "The source index has no record for a file used by this section.",
                Some(&path),
            );
            continue;
        };
        let Some(new_file) = to_index.files.iter().find(|file| file.path == path) else {
            result.unknown(
                "retrievalIndexUnavailable",
                "The target index has no record for a file used by this section.",
                Some(&path),
            );
            continue;
        };
        let old_hash = from_manifest
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.sha256.as_str());
        let new_hash = to_manifest
            .files
            .iter()
            .find(|file| file.path == path)
            .map(|file| file.sha256.as_str());
        if old_hash != Some(old_file.source_sha256.as_str())
            || new_hash != Some(new_file.source_sha256.as_str())
        {
            result.unknown(
                "retrievalIndexSourceMismatch",
                "The index entry for a retrieved file does not match its captured source.",
                Some(&path),
            );
            continue;
        }
        if old_file.status != new_file.status
            || old_file.parser != new_file.parser
            || expected_parsers
                .iter()
                .any(|parser| old_file.parser.as_ref() != Some(parser))
        {
            result.unknown(
                "retrievalParserChanged",
                "The parser or indexing status for a retrieved file changed; review the section's source context.",
                Some(&path),
            );
            continue;
        }
        if old_file.status != IndexFileStatus::Indexed {
            result.unknown(
                "retrievalIndexIncomplete",
                "The retrieved file does not have a complete structural index in both snapshots.",
                Some(&path),
            );
            continue;
        }

        let relevant_symbols = symbols.get(&path).cloned().unwrap_or_default();
        let mut symbol_unavailable = false;
        for symbol_id in &relevant_symbols {
            let old_symbol = from_index
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == *symbol_id && symbol.path == path);
            if old_symbol.is_none() {
                symbol_unavailable = true;
                break;
            }
        }
        if symbol_unavailable {
            result.unknown(
                "retrievalSymbolUnavailable",
                "The accepted index is missing a symbol recorded in this section's retrieval context.",
                Some(&path),
            );
        }
        if !relevant_symbols.is_empty() {
            let old_relationships = retrieval_relationships(from_index, &relevant_symbols);
            let new_relationships = retrieval_relationships(to_index, &relevant_symbols);
            // Compare semantic edge counts per evidence file. Moving an
            // unchanged edge within a file must not make the Guide stale.
            for relationship in old_relationships.keys().chain(new_relationships.keys()) {
                if old_relationships.get(relationship) != new_relationships.get(relationship) {
                    changed_relationship_paths.insert(relationship.1.clone());
                }
            }
        }
    }
    for path in changed_relationship_paths {
        result.unknown(
            "retrievalRelationshipsChanged",
            "Indexed calls or imports touching a retrieved symbol changed; review the section's context.",
            Some(&path),
        );
    }
}

fn retrieval_relationships(
    index: &SnapshotIndex,
    symbol_ids: &BTreeSet<String>,
) -> BTreeMap<(String, String), usize> {
    let mut relationships = BTreeMap::new();
    for relationship in index.relationships.iter().filter(|relationship| {
        symbol_ids.contains(&relationship.source_id)
            || relationship
                .target_id
                .as_ref()
                .is_some_and(|target| symbol_ids.contains(target))
    }) {
        let key = (
            relationship_descriptor(index, relationship),
            relationship.evidence.path.clone(),
        );
        *relationships.entry(key).or_insert(0) += 1;
    }
    relationships
}

fn assess_symbol_dependency(
    storage_dir: &Path,
    from_manifest: &SnapshotManifest,
    from_index: Option<&SnapshotIndex>,
    to_manifest: &SnapshotManifest,
    to_index: Option<&SnapshotIndex>,
    path: &str,
    source_sha256: &str,
    symbol_id: &str,
    result: &mut SectionAssessment,
) {
    let Some(old_file) = from_manifest.files.iter().find(|file| file.path == path) else {
        result.unknown(
            "sourceUnavailable",
            "The symbol dependency is absent from the accepted snapshot manifest.",
            Some(path),
        );
        return;
    };
    if old_file.sha256 != source_sha256 {
        result.unknown(
            "sourceHashMismatch",
            "The symbol dependency hash does not match its accepted snapshot.",
            Some(path),
        );
        return;
    }
    let Some(new_file) = to_manifest.files.iter().find(|file| file.path == path) else {
        if path_was_skipped(to_manifest, path)
            || to_manifest.summary.status != CaptureStatus::Complete
        {
            result.unknown("sourceNotCaptured", "The target snapshot did not capture this symbol dependency, so its state is unknown.", Some(path));
        } else {
            result.stale(
                "symbolRemoved",
                "The symbol's source file is absent from the complete target snapshot.",
                Some(path),
                Some(FreshnessSourceChange {
                    path: path.to_owned(),
                    kind: "deleted".to_owned(),
                    old_range: None,
                    new_range: None,
                }),
            );
        }
        return;
    };
    if new_file.sha256 == source_sha256 {
        match (
            read_source(storage_dir, from_manifest, path),
            read_source(storage_dir, to_manifest, path),
        ) {
            (Ok(old_source), Ok(new_source)) if old_source.sha256 == new_source.sha256 => {
                result.current()
            }
            _ => result.unknown(
                "sourceUnavailable",
                "The unchanged symbol source could not be verified in both snapshots.",
                Some(path),
            ),
        }
        return;
    }

    let Some(old_index) = from_index else {
        result.unknown(
            "indexUnavailable",
            "The accepted snapshot symbol index is unavailable for this dependency.",
            Some(path),
        );
        return;
    };
    let Some(new_index) = to_index else {
        result.unknown(
            "indexUnavailable",
            "The target snapshot symbol index is unavailable for this dependency.",
            Some(path),
        );
        return;
    };
    if !file_indexed_cleanly(old_index, from_manifest, path)
        || !file_indexed_cleanly(new_index, to_manifest, path)
    {
        result.unknown("indexIncomplete", "The symbol dependency cannot be compared because a relevant file was not indexed completely.", Some(path));
        return;
    }
    let Some(old_symbol) = old_index
        .symbols
        .iter()
        .find(|symbol| symbol.symbol_id == symbol_id && symbol.path == path)
    else {
        result.unknown(
            "symbolUnavailable",
            "The accepted index does not contain the saved stable symbol dependency.",
            Some(path),
        );
        return;
    };
    let Some(new_symbol) = new_index
        .symbols
        .iter()
        .find(|symbol| symbol.symbol_id == symbol_id && symbol.path == path)
    else {
        result.stale(
            "symbolRemoved",
            "The stable symbol dependency no longer exists in the target snapshot.",
            Some(path),
            Some(FreshnessSourceChange {
                path: path.to_owned(),
                kind: "deleted".to_owned(),
                old_range: Some(index_anchor_range(from_manifest, &old_symbol.anchor)),
                new_range: None,
            }),
        );
        return;
    };
    let (old_text, new_text) = match (
        symbol_text(storage_dir, from_manifest, old_symbol),
        symbol_text(storage_dir, to_manifest, new_symbol),
    ) {
        (Ok(old_text), Ok(new_text)) => (old_text, new_text),
        _ => {
            result.unknown(
                "sourceUnavailable",
                "A captured declaration needed for the symbol comparison could not be verified.",
                Some(path),
            );
            return;
        }
    };
    if old_text != new_text {
        result.stale(
            "symbolChanged",
            "The complete indexed declaration for this dependency changed.",
            Some(path),
            Some(FreshnessSourceChange {
                path: path.to_owned(),
                kind: "modified".to_owned(),
                old_range: Some(index_anchor_range(from_manifest, &old_symbol.anchor)),
                new_range: Some(index_anchor_range(to_manifest, &new_symbol.anchor)),
            }),
        );
    } else if old_symbol.anchor.start_line != new_symbol.anchor.start_line {
        result.safe(Some(FreshnessSourceChange {
            path: path.to_owned(),
            kind: "shifted".to_owned(),
            old_range: Some(index_anchor_range(from_manifest, &old_symbol.anchor)),
            new_range: Some(index_anchor_range(to_manifest, &new_symbol.anchor)),
        }));
    } else {
        result.current();
    }
}

fn assess_relationship_dependency(
    storage_dir: &Path,
    from_manifest: &SnapshotManifest,
    from_index: Option<&SnapshotIndex>,
    to_manifest: &SnapshotManifest,
    to_index: Option<&SnapshotIndex>,
    path: &str,
    source_sha256: &str,
    relationship_id: &str,
    result: &mut SectionAssessment,
) {
    let Some(old_file) = from_manifest.files.iter().find(|file| file.path == path) else {
        result.unknown(
            "sourceUnavailable",
            "The relationship evidence path is absent from the accepted snapshot manifest.",
            Some(path),
        );
        return;
    };
    if old_file.sha256 != source_sha256 {
        result.unknown(
            "sourceHashMismatch",
            "The relationship dependency hash does not match its accepted snapshot.",
            Some(path),
        );
        return;
    }
    let Some(old_index) = from_index else {
        result.unknown(
            "indexUnavailable",
            "The accepted snapshot relationship index is unavailable.",
            Some(path),
        );
        return;
    };
    let Some(new_index) = to_index else {
        result.unknown(
            "indexUnavailable",
            "The target snapshot relationship index is unavailable.",
            Some(path),
        );
        return;
    };
    if !file_indexed_cleanly(old_index, from_manifest, path)
        || !file_indexed_cleanly(new_index, to_manifest, path)
    {
        result.unknown(
            "indexIncomplete",
            "A relevant relationship source file was not indexed completely.",
            Some(path),
        );
        return;
    }
    let Some(old_relationship) = old_index.relationships.iter().find(|relationship| {
        relationship.relationship_id == relationship_id && relationship.evidence.path == path
    }) else {
        result.unknown(
            "relationshipUnavailable",
            "The accepted index does not contain the saved relationship dependency.",
            Some(path),
        );
        return;
    };
    if !relationship_endpoints_indexed(old_index, from_manifest, old_relationship) {
        result.unknown(
            "indexIncomplete",
            "A source or target endpoint for this relationship was not indexed completely.",
            Some(path),
        );
        return;
    }
    if !relationship_endpoints_captured(storage_dir, old_index, from_manifest, old_relationship) {
        result.unknown(
            "sourceUnavailable",
            "A captured source file for this relationship could not be verified against its manifest.",
            Some(path),
        );
        return;
    }
    let source_path = old_index
        .symbols
        .iter()
        .find(|symbol| symbol.symbol_id == old_relationship.source_id)
        .map(|symbol| symbol.path.as_str())
        .unwrap_or(old_relationship.evidence.path.as_str());
    let target_path = old_relationship.target_path.as_deref().or_else(|| {
        old_relationship.target_id.as_deref().and_then(|target| {
            old_index
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == target)
                .map(|symbol| symbol.path.as_str())
        })
    });
    for endpoint_path in std::iter::once(source_path).chain(target_path) {
        if !to_manifest
            .files
            .iter()
            .any(|file| file.path == endpoint_path)
        {
            if !path_was_skipped(to_manifest, endpoint_path)
                && to_manifest.summary.status == CaptureStatus::Complete
            {
                result.stale("relationshipEndpointRemoved", "A source or destination file for this relationship is absent from the complete target snapshot.", Some(endpoint_path), Some(FreshnessSourceChange {
                    path: endpoint_path.to_owned(),
                    kind: "deleted".to_owned(),
                    // The old lines are relationship evidence, not necessarily
                    // in the endpoint file whose absence made the edge stale.
                    old_range: Some(index_anchor_range(from_manifest, &old_relationship.evidence)),
                    new_range: None,
                }));
            } else {
                result.unknown("sourceNotCaptured", "A relationship endpoint was not captured in the target snapshot, so its state is unknown.", Some(endpoint_path));
            }
            return;
        }
        if !file_indexed_cleanly(new_index, to_manifest, endpoint_path) {
            result.unknown("indexIncomplete", "A source or destination endpoint for this relationship was not indexed completely.", Some(endpoint_path));
            return;
        }
        if !file_source_matches_manifest(storage_dir, to_manifest, endpoint_path) {
            result.unknown(
                "sourceUnavailable",
                "A captured target endpoint could not be read and verified against its manifest.",
                Some(endpoint_path),
            );
            return;
        }
    }
    let descriptor = relationship_descriptor(old_index, old_relationship);
    let candidates: Vec<&IndexedRelationship> = new_index
        .relationships
        .iter()
        .filter(|relationship| relationship_descriptor(new_index, relationship) == descriptor)
        .collect();
    let old_range = Some(index_anchor_range(
        from_manifest,
        &old_relationship.evidence,
    ));

    for candidate in &candidates {
        if !relationship_endpoints_indexed(new_index, to_manifest, candidate) {
            result.unknown(
                "indexIncomplete",
                "A matching target relationship has an incomplete evidence or endpoint index.",
                Some(path),
            );
            return;
        }
        if !relationship_endpoints_captured(storage_dir, new_index, to_manifest, candidate) {
            result.unknown("sourceUnavailable", "A matching target relationship endpoint could not be verified against its captured source.", Some(path));
            return;
        }
    }

    let evidence_remap =
        remap_relationship_evidence(storage_dir, from_manifest, old_relationship, to_manifest);
    let mapped_occurrence = match &evidence_remap {
        RelationshipEvidenceRemap::Mapped(location) => Some(location),
        _ => None,
    };
    if let Some(location) = mapped_occurrence {
        for candidate in new_index.relationships.iter().filter(|candidate| {
            candidate.kind == old_relationship.kind
                && candidate.evidence.path == location.path
                && candidate.evidence.start_byte == location.start_byte
                && candidate.evidence.end_byte == location.end_byte_exclusive
        }) {
            if !relationship_endpoints_indexed(new_index, to_manifest, candidate) {
                result.unknown("indexIncomplete", "A relationship at the cited target occurrence has an incomplete endpoint index.", Some(path));
                return;
            }
            if !relationship_endpoints_captured(storage_dir, new_index, to_manifest, candidate) {
                result.unknown("sourceUnavailable", "A relationship at the cited target occurrence has an unverifiable endpoint source.", Some(path));
                return;
            }
        }
    }

    let exact_candidates: Vec<&IndexedRelationship> = mapped_occurrence
        .map(|location| {
            candidates
                .iter()
                .copied()
                .filter(|candidate| {
                    candidate.evidence.path == location.path
                        && candidate.evidence.start_byte == location.start_byte
                        && candidate.evidence.end_byte == location.end_byte_exclusive
                })
                .collect()
        })
        .unwrap_or_default();
    if exact_candidates.len() > 1 {
        result.unknown("relationshipOccurrenceAmbiguous", "More than one target relationship matches the cited occurrence, so no edge can be selected safely.", Some(path));
        return;
    }
    if mapped_occurrence.is_some() && exact_candidates.is_empty() && !candidates.is_empty() {
        result.unknown("relationshipOccurrenceAmbiguous", "The same relationship exists in the target snapshot, but its exact occurrence could not be mapped without guessing.", Some(path));
        return;
    }
    let matched = exact_candidates.first().copied();

    if let Some(new_relationship) = matched {
        if old_relationship.kind == RelationshipKind::TestReference {
            let old_source_symbol = old_index
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == old_relationship.source_id);
            let new_source_symbol = new_index
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == new_relationship.source_id);
            match (old_source_symbol, new_source_symbol) {
                (Some(old_symbol), Some(new_symbol)) => match (
                    symbol_text(storage_dir, from_manifest, old_symbol),
                    symbol_text(storage_dir, to_manifest, new_symbol),
                ) {
                    (Ok(old_text), Ok(new_text)) if old_text == new_text => {}
                    (Ok(_), Ok(_)) => {
                        result.stale("testEvidenceChanged", "The indexed test declaration changed, including its assertions or setup.", Some(path), Some(FreshnessSourceChange {
                            path: old_symbol.path.clone(),
                            kind: "relationshipChanged".to_owned(),
                            old_range: Some(index_anchor_range(from_manifest, &old_symbol.anchor)),
                            new_range: Some(index_anchor_range(to_manifest, &new_symbol.anchor)),
                        }));
                        return;
                    }
                    _ => {
                        result.unknown("sourceUnavailable", "The test declaration bytes needed for this relationship comparison could not be verified.", Some(path));
                        return;
                    }
                },
                _ => {
                    result.unknown("symbolUnavailable", "The test relationship source symbol could not be resolved in both indexes.", Some(path));
                    return;
                }
            }
        }
        let new_range = Some(index_anchor_range(to_manifest, &new_relationship.evidence));
        if old_range != new_range {
            result.safe(Some(FreshnessSourceChange {
                path: path.to_owned(),
                kind: "shifted".to_owned(),
                old_range,
                new_range,
            }));
        } else {
            result.current();
        }
        return;
    }

    match evidence_remap {
        RelationshipEvidenceRemap::Ambiguous => {
            result.unknown("relationshipEvidenceAmbiguous", "The exact relationship evidence appears more than once in the target source, so its occurrence cannot be chosen safely.", Some(path));
            return;
        }
        RelationshipEvidenceRemap::Unavailable(message) => {
            result.unknown("relationshipEvidenceUnavailable", message, Some(path));
            return;
        }
        RelationshipEvidenceRemap::Mapped(_) | RelationshipEvidenceRemap::Changed => {}
    }

    let source_path = old_index
        .symbols
        .iter()
        .find(|symbol| symbol.symbol_id == old_relationship.source_id)
        .map(|symbol| symbol.path.as_str())
        .unwrap_or(path);
    let source_file_complete = file_indexed_cleanly(new_index, to_manifest, source_path);
    let old_target_path = old_relationship.target_path.as_deref().or_else(|| {
        old_relationship.target_id.as_deref().and_then(|target| {
            old_index
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == target)
                .map(|symbol| symbol.path.as_str())
        })
    });
    let target_complete = old_target_path
        .is_none_or(|target_path| file_indexed_cleanly(new_index, to_manifest, target_path));
    if !source_file_complete || !target_complete {
        result.unknown("indexIncomplete", "The target snapshot lacks complete source or destination index evidence to confirm whether this relationship changed.", Some(path));
        return;
    }
    // If exactly one edge of the same kind remains in the same caller, show it
    // as comparison context. Never choose among multiple possible destinations.
    let same_caller_edges: Vec<&IndexedRelationship> = new_index
        .relationships
        .iter()
        .filter(|candidate| {
            candidate.kind == old_relationship.kind
                && candidate.source_id == old_relationship.source_id
                && candidate.evidence.path == old_relationship.evidence.path
        })
        .collect();
    for candidate in &same_caller_edges {
        if !relationship_endpoints_indexed(new_index, to_manifest, candidate) {
            result.unknown("indexIncomplete", "A possible target relationship has an incomplete endpoint index, so the change cannot be classified conclusively.", Some(path));
            return;
        }
        if !relationship_endpoints_captured(storage_dir, new_index, to_manifest, candidate) {
            result.unknown("sourceUnavailable", "A possible target relationship endpoint could not be verified against its captured source.", Some(path));
            return;
        }
    }
    let contextual_range = (same_caller_edges.len() == 1)
        .then(|| index_anchor_range(to_manifest, &same_caller_edges[0].evidence));
    result.stale(
        if old_relationship.kind == RelationshipKind::TestReference {
            "testRelationshipChanged"
        } else {
            "relationshipChanged"
        },
        if old_relationship.kind == RelationshipKind::TestReference {
            "The cited test relationship or resolved target changed in the target snapshot."
        } else {
            "The cited relationship or its resolved destination changed in the target snapshot."
        },
        Some(path),
        Some(FreshnessSourceChange {
            path: path.to_owned(),
            kind: "relationshipChanged".to_owned(),
            old_range,
            new_range: contextual_range,
        }),
    );
}

fn relationship_descriptor(index: &SnapshotIndex, relationship: &IndexedRelationship) -> String {
    let symbol_identity = |symbol_id: Option<&str>| -> String {
        let Some(symbol_id) = symbol_id else {
            return "<none>".to_owned();
        };
        index
            .symbols
            .iter()
            .find(|symbol| symbol.symbol_id == symbol_id)
            .map(|symbol| format!("{}:{}", symbol.path, symbol.declaration_identity))
            .unwrap_or_else(|| format!("<missing:{symbol_id}>"))
    };
    format!(
        "{:?}|{}|{}|{}|{}|{:?}|{:?}",
        relationship.kind,
        symbol_identity(Some(&relationship.source_id)),
        symbol_identity(relationship.target_id.as_deref()),
        relationship.target_path.as_deref().unwrap_or(""),
        relationship.target_label.as_deref().unwrap_or(""),
        relationship.confidence,
        relationship.unresolved_reason,
    )
}

fn relationship_endpoints_indexed(
    index: &SnapshotIndex,
    manifest: &SnapshotManifest,
    relationship: &IndexedRelationship,
) -> bool {
    if !file_indexed_cleanly(index, manifest, &relationship.evidence.path) {
        return false;
    }
    let source_path = index
        .symbols
        .iter()
        .find(|symbol| symbol.symbol_id == relationship.source_id)
        .map(|symbol| symbol.path.as_str());
    let target_path = relationship.target_path.as_deref().or_else(|| {
        relationship.target_id.as_deref().and_then(|target| {
            index
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == target)
                .map(|symbol| symbol.path.as_str())
        })
    });
    source_path.is_none_or(|path| file_indexed_cleanly(index, manifest, path))
        && target_path.is_none_or(|path| file_indexed_cleanly(index, manifest, path))
}

#[derive(Clone, Debug)]
struct RelationshipEvidenceLocation {
    path: String,
    start_byte: usize,
    end_byte_exclusive: usize,
}

#[derive(Clone, Debug)]
enum RelationshipEvidenceRemap {
    Mapped(RelationshipEvidenceLocation),
    Changed,
    Ambiguous,
    Unavailable(String),
}

fn relationship_evidence_text(
    storage_dir: &Path,
    manifest: &SnapshotManifest,
    relationship: &IndexedRelationship,
) -> Result<String, String> {
    let evidence = &relationship.evidence;
    let Some(file) = manifest
        .files
        .iter()
        .find(|file| file.path == evidence.path)
    else {
        return Err(
            "The relationship evidence path is absent from its snapshot manifest.".to_owned(),
        );
    };
    if evidence.source_sha256 != file.sha256 {
        return Err(
            "The relationship evidence hash does not match its snapshot manifest.".to_owned(),
        );
    }
    let source = read_source(storage_dir, manifest, &evidence.path)?;
    if source.sha256 != file.sha256 || source.path != evidence.path {
        return Err(
            "The relationship source bytes do not match their snapshot manifest.".to_owned(),
        );
    }
    let bytes = source.contents.as_bytes();
    if evidence.start_byte >= evidence.end_byte || evidence.end_byte > bytes.len() {
        return Err(
            "The relationship evidence range is empty or outside captured source bytes.".to_owned(),
        );
    }
    let excerpt = source
        .contents
        .get(evidence.start_byte..evidence.end_byte)
        .ok_or_else(|| {
            "The relationship evidence range is not aligned to UTF-8 boundaries.".to_owned()
        })?;
    if byte_position(bytes, evidence.start_byte)
        != (evidence.start_line as u32, evidence.start_column as u32)
        || byte_position(bytes, evidence.end_byte - 1)
            != (evidence.end_line as u32, evidence.end_column as u32)
    {
        return Err(
            "The relationship evidence line or byte-column range does not match its byte offsets."
                .to_owned(),
        );
    }
    Ok(excerpt.to_owned())
}

fn remap_relationship_evidence(
    storage_dir: &Path,
    from_manifest: &SnapshotManifest,
    old_relationship: &IndexedRelationship,
    to_manifest: &SnapshotManifest,
) -> RelationshipEvidenceRemap {
    let excerpt = match relationship_evidence_text(storage_dir, from_manifest, old_relationship) {
        Ok(excerpt) => excerpt,
        Err(message) => return RelationshipEvidenceRemap::Unavailable(message),
    };
    let evidence_path = old_relationship.evidence.path.as_str();
    let Some(new_file) = to_manifest
        .files
        .iter()
        .find(|file| file.path == evidence_path)
    else {
        return RelationshipEvidenceRemap::Unavailable(
            "The relationship evidence path is absent from the target snapshot manifest."
                .to_owned(),
        );
    };
    let new_source = match read_source(storage_dir, to_manifest, evidence_path) {
        Ok(source) if source.sha256 == new_file.sha256 => source,
        Ok(_) => {
            return RelationshipEvidenceRemap::Unavailable(
                "The target relationship evidence bytes do not match their snapshot manifest."
                    .to_owned(),
            );
        }
        Err(message) => return RelationshipEvidenceRemap::Unavailable(message),
    };
    let bytes = new_source.contents.as_bytes();
    let start = if new_file.sha256 == old_relationship.evidence.source_sha256 {
        old_relationship.evidence.start_byte
    } else {
        if excerpt.is_empty() || excerpt.len() > bytes.len() {
            return RelationshipEvidenceRemap::Changed;
        }
        let mut matches = bytes
            .windows(excerpt.len())
            .enumerate()
            .filter_map(|(offset, candidate)| (candidate == excerpt.as_bytes()).then_some(offset));
        let Some(first) = matches.next() else {
            return RelationshipEvidenceRemap::Changed;
        };
        if matches.next().is_some() {
            return RelationshipEvidenceRemap::Ambiguous;
        }
        first
    };
    let end = start + excerpt.len();
    if start >= end
        || end > bytes.len()
        || new_source.contents.get(start..end) != Some(excerpt.as_str())
    {
        return RelationshipEvidenceRemap::Unavailable(
            "The remapped relationship range failed target source verification.".to_owned(),
        );
    }
    RelationshipEvidenceRemap::Mapped(RelationshipEvidenceLocation {
        path: evidence_path.to_owned(),
        start_byte: start,
        end_byte_exclusive: end,
    })
}

fn relationship_endpoints_captured(
    storage_dir: &Path,
    index: &SnapshotIndex,
    manifest: &SnapshotManifest,
    relationship: &IndexedRelationship,
) -> bool {
    if relationship_evidence_text(storage_dir, manifest, relationship).is_err() {
        return false;
    }
    let mut endpoint_paths = BTreeSet::from([relationship.evidence.path.as_str()]);
    if let Some(source_path) = index
        .symbols
        .iter()
        .find(|symbol| symbol.symbol_id == relationship.source_id)
        .map(|symbol| symbol.path.as_str())
    {
        endpoint_paths.insert(source_path);
    }
    let target_path = relationship.target_path.as_deref().or_else(|| {
        relationship.target_id.as_deref().and_then(|target| {
            index
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == target)
                .map(|symbol| symbol.path.as_str())
        })
    });
    if let Some(target_path) = target_path {
        endpoint_paths.insert(target_path);
    }
    endpoint_paths
        .into_iter()
        .all(|path| file_source_matches_manifest(storage_dir, manifest, path))
}

fn file_source_matches_manifest(
    storage_dir: &Path,
    manifest: &SnapshotManifest,
    path: &str,
) -> bool {
    let Some(file) = manifest.files.iter().find(|file| file.path == path) else {
        return false;
    };
    read_source(storage_dir, manifest, path)
        .is_ok_and(|source| source.path == path && source.sha256 == file.sha256)
}

fn file_indexed_cleanly(index: &SnapshotIndex, manifest: &SnapshotManifest, path: &str) -> bool {
    let Some(captured) = manifest.files.iter().find(|file| file.path == path) else {
        return false;
    };
    index.files.iter().any(|file| {
        file.path == path
            && file.status == IndexFileStatus::Indexed
            && file.source_sha256 == captured.sha256
    })
}

fn symbol_text(
    storage_dir: &Path,
    manifest: &SnapshotManifest,
    symbol: &IndexedSymbol,
) -> Result<String, String> {
    let source = read_source(storage_dir, manifest, &symbol.path)?;
    let start = symbol.anchor.start_byte;
    let end = symbol.anchor.end_byte;
    if symbol.anchor.source_sha256 != source.sha256 || start >= end || end > source.contents.len() {
        return Err(
            "The indexed declaration range failed its captured source hash or bounds check."
                .to_owned(),
        );
    }
    source
        .contents
        .get(start..end)
        .map(str::to_owned)
        .ok_or_else(|| {
            "The indexed declaration is not aligned to UTF-8 source boundaries.".to_owned()
        })
}

fn read_source(
    storage_dir: &Path,
    manifest: &SnapshotManifest,
    path: &str,
) -> Result<SnapshotSource, String> {
    snapshot::read_snapshot_source_from_manifest(storage_dir, manifest, path)
}

fn anchor_matches_source(anchor: &ChapterSourceAnchor, source: &SnapshotSource) -> bool {
    if anchor.path != source.path || anchor.source_sha256 != source.sha256 {
        return false;
    }
    let (Ok(start), Ok(end_inclusive)) = (
        usize::try_from(anchor.start_byte),
        usize::try_from(anchor.end_byte_inclusive),
    ) else {
        return false;
    };
    let Some(end_exclusive) = end_inclusive.checked_add(1) else {
        return false;
    };
    if start >= end_exclusive || end_exclusive > source.contents.len() {
        return false;
    }
    let Some(excerpt) = source.contents.get(start..end_exclusive) else {
        return false;
    };
    if excerpt != anchor.excerpt {
        return false;
    }
    let (start_line, start_column) = byte_position(source.contents.as_bytes(), start);
    let (end_line, end_column) = byte_position(source.contents.as_bytes(), end_inclusive);
    (start_line, start_column, end_line, end_column)
        == (
            anchor.start_line,
            anchor.start_column,
            anchor.end_line,
            anchor.end_column,
        )
}

fn byte_position(bytes: &[u8], offset: usize) -> (u32, u32) {
    let prefix = &bytes[..offset.min(bytes.len())];
    let line = prefix.iter().filter(|byte| **byte == b'\n').count() as u32 + 1;
    let column = prefix
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(prefix.len(), |index| prefix.len() - index - 1) as u32
        + 1;
    (line, column)
}

fn path_was_skipped(manifest: &SnapshotManifest, path: &str) -> bool {
    manifest.skipped.iter().any(|skipped| {
        skipped.path == path
            || (matches!(
                skipped.kind,
                InventoryPathKind::Directory | InventoryPathKind::Unknown
            ) && path
                .strip_prefix(&skipped.path)
                .is_some_and(|suffix| suffix.starts_with('/')))
    })
}

fn range(
    path: &str,
    snapshot_id: &str,
    source_sha256: &str,
    start_line: u32,
    end_line: u32,
) -> FreshnessLineRange {
    FreshnessLineRange {
        path: path.to_owned(),
        snapshot_id: snapshot_id.to_owned(),
        source_sha256: source_sha256.to_owned(),
        start_line,
        end_line,
    }
}

fn chapter_anchor_range(anchor: &ChapterSourceAnchor) -> FreshnessLineRange {
    range(
        &anchor.path,
        &anchor.snapshot_id,
        &anchor.source_sha256,
        anchor.start_line,
        anchor.end_line,
    )
}

fn index_anchor_range(manifest: &SnapshotManifest, anchor: &SourceAnchor) -> FreshnessLineRange {
    range(
        &anchor.path,
        &manifest.summary.snapshot_id,
        &anchor.source_sha256,
        anchor.start_line as u32,
        anchor.end_line as u32,
    )
}

fn push_unique_change(changes: &mut Vec<FreshnessSourceChange>, change: FreshnessSourceChange) {
    if !changes.iter().any(|saved| {
        saved.path == change.path
            && saved.kind == change.kind
            && saved.old_range == change.old_range
            && saved.new_range == change.new_range
    }) {
        changes.push(change);
    }
}

fn build_reverse_dependencies(
    revision: &ChapterRevision,
    retrieval_index_dependencies: &[SectionDependency],
) -> Vec<ReverseDependency> {
    type DependencyMap = BTreeMap<String, (String, BTreeSet<String>, BTreeSet<String>)>;
    let mut dependencies = DependencyMap::new();
    let anchors_by_id: HashMap<String, &ChapterSourceAnchor> = revision
        .sections
        .iter()
        .flat_map(|section| &section.claims)
        .flat_map(|claim| &claim.anchors)
        .map(|anchor| (anchor.anchor_id.clone(), anchor))
        .collect();

    for section in &revision.sections {
        let visual = section
            .visual
            .as_ref()
            .and_then(ChapterVisualPayload::typed_visual);
        let visual_asset = visual.and_then(|visual| {
            visual_asset_identity(&revision.chapter_id, section, visual, &anchors_by_id)
        });
        let visual_asset_id = visual_asset.as_ref().map(|(asset_id, _)| asset_id.clone());
        let visual_dependency_anchor_ids = visual_asset
            .as_ref()
            .map(|(_, anchor_ids)| anchor_ids.clone())
            .unwrap_or_default();

        let mut hotspot_anchor_assets = HashMap::<String, BTreeSet<String>>::new();
        for hotspot in &section.hotspots {
            if let HotspotDestination::CodeLens { anchor_id } = &hotspot.destination {
                hotspot_anchor_assets
                    .entry(anchor_id.clone())
                    .or_default()
                    .insert(format!("hotspot:{}", hotspot.hotspot_id));
            }
        }

        let mut add = |key: String, path: String, mut asset_ids: BTreeSet<String>| {
            let entry = dependencies
                .entry(key)
                .or_insert_with(|| (path, BTreeSet::new(), BTreeSet::new()));
            entry.1.insert(section.section_id.clone());
            entry.2.append(&mut asset_ids);
        };

        for claim in &section.claims {
            for anchor in &claim.anchors {
                let mut asset_ids = hotspot_anchor_assets
                    .get(&anchor.anchor_id)
                    .cloned()
                    .unwrap_or_default();
                if visual_dependency_anchor_ids.contains(&anchor.anchor_id) {
                    if let Some(visual_asset_id) = &visual_asset_id {
                        asset_ids.insert(visual_asset_id.clone());
                    }
                }
                add(
                    anchor_dependency_key(anchor),
                    anchor.path.clone(),
                    asset_ids,
                );
            }
        }

        // A hotspot may point to an anchor in a different section. Link that
        // route to the exact target evidence, and link the diagram only when
        // the hotspot is one of its typed inputs.
        for hotspot in &section.hotspots {
            let HotspotDestination::CodeLens { anchor_id } = &hotspot.destination else {
                continue;
            };
            let Some(anchor) = anchors_by_id.get(anchor_id) else {
                continue;
            };
            let mut asset_ids = BTreeSet::from([format!("hotspot:{}", hotspot.hotspot_id)]);
            if visual_dependency_anchor_ids.contains(anchor_id) {
                if let Some(visual_asset_id) = &visual_asset_id {
                    asset_ids.insert(visual_asset_id.clone());
                }
            }
            add(
                anchor_dependency_key(anchor),
                anchor.path.clone(),
                asset_ids,
            );
        }

        if revision.sections.len() == 1 {
            for range in revision
                .generation_origin
                .as_ref()
                .and_then(crate::chapter_origin::ChapterGenerationOrigin::retrieval_ranges)
                .unwrap_or_default()
            {
                add(
                    format!(
                        "retrievalRange:{}:{}:{}:{}:{}",
                        range.path,
                        range.source_sha256,
                        range.start_byte,
                        range.end_byte,
                        range.chunk_sha256,
                    ),
                    range.path.clone(),
                    BTreeSet::new(),
                );
            }
        }
        for dependency in section
            .dependencies
            .iter()
            .chain(section.claims.iter().flat_map(|claim| &claim.dependencies))
            .chain(retrieval_index_dependencies.iter())
        {
            let (key, path) = match dependency {
                SectionDependency::SourceFile {
                    path,
                    source_sha256,
                } => (format!("sourceFile:{path}:{source_sha256}"), path),
                SectionDependency::Symbol {
                    path,
                    source_sha256,
                    symbol_id,
                } => (format!("symbol:{path}:{symbol_id}:{source_sha256}"), path),
                SectionDependency::Relationship {
                    path,
                    source_sha256,
                    relationship_id,
                } => (
                    format!("relationship:{path}:{relationship_id}:{source_sha256}"),
                    path,
                ),
            };
            let asset_ids = section
                .hotspots
                .iter()
                .filter(|hotspot| hotspot_matches_dependency(hotspot, dependency))
                .map(|hotspot| format!("hotspot:{}", hotspot.hotspot_id))
                .collect();
            add(key, path.clone(), asset_ids);
        }
    }
    dependencies
        .into_iter()
        .map(
            |(evidence_key, (path, sections, assets))| ReverseDependency {
                evidence_key,
                path,
                dependent_section_ids: sections.into_iter().collect(),
                asset_ids: assets.into_iter().collect(),
            },
        )
        .collect()
}

/// Bind enclosing declarations and one-hop relationships around retrieved
/// ranges to the section. Declaration checks remain local to each symbol, and
/// external endpoints retain exact symbol dependencies.
fn retrieved_index_dependencies(
    revision: &ChapterRevision,
    source_manifest: &SnapshotManifest,
    source_index: Option<&SnapshotIndex>,
) -> Vec<SectionDependency> {
    if revision.sections.len() != 1 {
        return Vec::new();
    }
    let Some(source_index) = source_index else {
        return Vec::new();
    };
    if source_index.status != SnapshotIndexStatus::Complete {
        return Vec::new();
    }
    let Some(recorded_identity) = revision
        .generation_origin
        .as_ref()
        .and_then(crate::chapter_origin::ChapterGenerationOrigin::retrieval_index_identity)
    else {
        return Vec::new();
    };
    if recorded_identity
        != format!(
            "{}:{}",
            source_index.schema_version, source_index.indexer_version
        )
    {
        return Vec::new();
    }
    let ranges = revision
        .generation_origin
        .as_ref()
        .and_then(crate::chapter_origin::ChapterGenerationOrigin::retrieval_ranges)
        .unwrap_or_default();
    let retrieved_paths = ranges
        .iter()
        .map(|range| range.path.clone())
        .collect::<BTreeSet<_>>();
    let symbol_ids = ranges
        .iter()
        .flat_map(|range| [&range.enclosing, &range.parent])
        .flatten()
        .map(|context| context.symbol_id.clone())
        .collect::<BTreeSet<_>>();
    if symbol_ids.is_empty() {
        return Vec::new();
    }

    let mut dependencies = BTreeMap::new();
    for (path, symbol_id) in ranges
        .iter()
        .flat_map(|range| {
            [&range.enclosing, &range.parent]
                .into_iter()
                .flatten()
                .map(move |context| (range.path.as_str(), context.symbol_id.as_str()))
        })
        .collect::<BTreeSet<_>>()
    {
        let Some(symbol) = source_index
            .symbols
            .iter()
            .find(|symbol| symbol.symbol_id == symbol_id && symbol.path == path)
        else {
            continue;
        };
        let source_sha256 = &symbol.anchor.source_sha256;
        let index_file = source_index.files.iter().find(|file| file.path == path);
        let manifest_file = source_manifest.files.iter().find(|file| file.path == path);
        if !index_file.is_some_and(|file| {
            file.status == IndexFileStatus::Indexed && file.source_sha256 == *source_sha256
        }) || !manifest_file.is_some_and(|file| file.sha256 == *source_sha256)
        {
            continue;
        }
        dependencies.insert(
            (
                0_u8,
                path.to_owned(),
                symbol.symbol_id.clone(),
                source_sha256.clone(),
            ),
            SectionDependency::Symbol {
                path: path.to_owned(),
                source_sha256: source_sha256.clone(),
                symbol_id: symbol.symbol_id.clone(),
            },
        );
    }

    for relationship in &source_index.relationships {
        if !symbol_ids.contains(&relationship.source_id)
            && !relationship
                .target_id
                .as_ref()
                .is_some_and(|target| symbol_ids.contains(target))
        {
            continue;
        }
        let evidence = &relationship.evidence;
        let index_file = source_index
            .files
            .iter()
            .find(|file| file.path == evidence.path);
        let manifest_file = source_manifest
            .files
            .iter()
            .find(|file| file.path == evidence.path);
        if !index_file.is_some_and(|file| {
            file.status == IndexFileStatus::Indexed && file.source_sha256 == evidence.source_sha256
        }) || !manifest_file.is_some_and(|file| file.sha256 == evidence.source_sha256)
        {
            continue;
        }
        dependencies.insert(
            (
                1_u8,
                evidence.path.clone(),
                relationship.relationship_id.clone(),
                evidence.source_sha256.clone(),
            ),
            SectionDependency::Relationship {
                path: evidence.path.clone(),
                source_sha256: evidence.source_sha256.clone(),
                relationship_id: relationship.relationship_id.clone(),
            },
        );

        for endpoint_id in
            std::iter::once(&relationship.source_id).chain(relationship.target_id.iter())
        {
            if symbol_ids.contains(endpoint_id) {
                continue;
            }
            let Some(symbol) = source_index
                .symbols
                .iter()
                .find(|symbol| &symbol.symbol_id == endpoint_id)
            else {
                continue;
            };
            if retrieved_paths.contains(&symbol.path) {
                continue;
            }
            let index_file = source_index
                .files
                .iter()
                .find(|file| file.path == symbol.path);
            let manifest_file = source_manifest
                .files
                .iter()
                .find(|file| file.path == symbol.path);
            let source_sha256 = &symbol.anchor.source_sha256;
            if !index_file.is_some_and(|file| {
                file.status == IndexFileStatus::Indexed && file.source_sha256 == *source_sha256
            }) || !manifest_file.is_some_and(|file| file.sha256 == *source_sha256)
            {
                continue;
            }
            dependencies.insert(
                (
                    2_u8,
                    symbol.path.clone(),
                    symbol.symbol_id.clone(),
                    source_sha256.clone(),
                ),
                SectionDependency::Symbol {
                    path: symbol.path.clone(),
                    source_sha256: source_sha256.clone(),
                    symbol_id: symbol.symbol_id.clone(),
                },
            );
        }
    }
    dependencies.into_values().collect()
}

#[cfg(test)]
pub(crate) fn asset_ids_for_revision(revision: &ChapterRevision) -> Vec<String> {
    let mut ids = build_reverse_dependencies(revision, &[])
        .into_iter()
        .flat_map(|dependency| dependency.asset_ids)
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn anchor_dependency_key(anchor: &ChapterSourceAnchor) -> String {
    format!(
        "anchor:{}:{}:{}:{}-{}",
        anchor.snapshot_id,
        anchor.path,
        anchor.source_sha256,
        anchor.start_byte,
        anchor.end_byte_inclusive
    )
}

fn hotspot_matches_dependency(hotspot: &ChapterHotspot, dependency: &SectionDependency) -> bool {
    match (&hotspot.destination, dependency) {
        (
            HotspotDestination::AtlasFile { path: target },
            SectionDependency::SourceFile { path, .. },
        ) => target == path,
        (
            HotspotDestination::AtlasSymbol {
                path: target_path,
                symbol_id: target_symbol,
            },
            SectionDependency::Symbol {
                path, symbol_id, ..
            },
        ) => target_path == path && target_symbol == symbol_id,
        (
            HotspotDestination::Relationship {
                relationship_id: target,
            },
            SectionDependency::Relationship {
                relationship_id, ..
            },
        ) => target == relationship_id,
        _ => false,
    }
}

fn visual_asset_identity(
    chapter_id: &str,
    section: &ChapterSection,
    visual: &VisualSpec,
    anchors_by_id: &HashMap<String, &ChapterSourceAnchor>,
) -> Option<(String, BTreeSet<String>)> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct AssetAnchorFingerprint<'a> {
        path: &'a str,
        symbol_id: Option<&'a str>,
        excerpt: &'a str,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct FingerprintInput<'a> {
        renderer_contract_version: u32,
        visual: &'a VisualSpec,
        evidence_anchors: Vec<AssetAnchorFingerprint<'a>>,
        used_hotspots: Vec<&'a ChapterHotspot>,
        destination_anchors: Vec<AssetAnchorFingerprint<'a>>,
    }

    let evidence_anchor_ids = visual
        .nodes
        .iter()
        .flat_map(|node| &node.evidence_anchor_ids)
        .chain(
            visual
                .edges
                .iter()
                .flat_map(|edge| &edge.evidence_anchor_ids),
        )
        .cloned()
        .collect::<BTreeSet<_>>();
    let used_hotspot_ids = visual
        .nodes
        .iter()
        .filter_map(|node| node.hotspot_id.clone())
        .collect::<BTreeSet<_>>();
    let mut used_hotspots = section
        .hotspots
        .iter()
        .filter(|hotspot| used_hotspot_ids.contains(&hotspot.hotspot_id))
        .collect::<Vec<_>>();
    used_hotspots.sort_by(|left, right| left.hotspot_id.cmp(&right.hotspot_id));
    let destination_anchor_ids = used_hotspots
        .iter()
        .filter_map(|hotspot| match &hotspot.destination {
            HotspotDestination::CodeLens { anchor_id } => Some(anchor_id.clone()),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let all_anchor_ids = evidence_anchor_ids
        .iter()
        .chain(destination_anchor_ids.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let evidence_anchors = evidence_anchor_ids
        .iter()
        .filter_map(|anchor_id| anchors_by_id.get(anchor_id).copied())
        .map(|anchor| AssetAnchorFingerprint {
            path: &anchor.path,
            symbol_id: anchor.symbol_id.as_deref(),
            excerpt: &anchor.excerpt,
        })
        .collect::<Vec<_>>();
    let destination_anchors = destination_anchor_ids
        .iter()
        .filter_map(|anchor_id| anchors_by_id.get(anchor_id).copied())
        .map(|anchor| AssetAnchorFingerprint {
            path: &anchor.path,
            symbol_id: anchor.symbol_id.as_deref(),
            excerpt: &anchor.excerpt,
        })
        .collect::<Vec<_>>();
    let input = FingerprintInput {
        renderer_contract_version: chapters::VISUAL_RENDERER_CONTRACT_VERSION,
        visual,
        evidence_anchors,
        used_hotspots,
        destination_anchors,
    };
    let bytes = serde_json::to_vec(&input).ok()?;
    let digest = Sha256::digest(bytes);
    Some((
        format!("visual:{chapter_id}:{}:{digest:x}", section.section_id),
        all_anchor_ids,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapters::{
        ChapterClaim, ChapterHotspot, ChapterTarget, ChapterTargetScope, ClaimCategory,
        RevisionLifecycle, ValidationStatus, VisualEdge, VisualKind, VisualLayoutDirection,
        VisualLayoutHint, VisualNode,
    };
    use crate::index::{
        IndexedFile, IndexedRelationship, IndexedSymbol, RelationshipConfidence, RelationshipKind,
        SourceAnchor, SymbolKind,
    };
    use crate::snapshot::{
        CaptureLimits, CapturedContentKind, CapturedFile, GitChangeState, InventoryPathKind,
        SkippedPath, SnapshotSummary,
    };
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::path::PathBuf;
    use uuid::Uuid;

    const PROJECT: &str = "freshness-test-project";

    struct TempStorage(PathBuf);

    impl TempStorage {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("code-atlas-freshness-{}", Uuid::new_v4()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempStorage {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn hash(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn put_file(storage: &Path, path: &str, text: &str) -> CapturedFile {
        let sha = hash(text.as_bytes());
        let blob = storage
            .join("objects/sha256")
            .join(&sha[..2])
            .join(&sha[2..]);
        fs::create_dir_all(blob.parent().unwrap()).unwrap();
        fs::write(blob, text.as_bytes()).unwrap();
        CapturedFile {
            path: path.to_owned(),
            kind: InventoryPathKind::File,
            size_bytes: text.len() as u64,
            sha256: sha.clone(),
            blob_path: format!("objects/sha256/{}/{}", &sha[..2], &sha[2..]),
            content_kind: CapturedContentKind::Text,
            secret_like: false,
        }
    }

    fn manifest(id: &str, files: Vec<CapturedFile>) -> SnapshotManifest {
        SnapshotManifest {
            schema_version: 2,
            summary: SnapshotSummary {
                snapshot_id: id.to_owned(),
                project_id: PROJECT.to_owned(),
                parent_snapshot_id: None,
                created_at_unix_ms: 1,
                branch: Some("main".to_owned()),
                head: None,
                index_state: GitChangeState::Clean,
                worktree_state: GitChangeState::Clean,
                status: CaptureStatus::Complete,
                files_captured: files.len(),
                paths_skipped: 0,
                total_bytes: files.iter().map(|file| file.size_bytes).sum(),
                paths_visited: files.len() as u64,
                policy_version: "fixture".to_owned(),
            },
            repository_root: "/repo".to_owned(),
            selected_path: "/repo".to_owned(),
            capture_limits: CaptureLimits::default(),
            directories: Vec::new(),
            files,
            skipped: Vec::new(),
        }
    }

    fn position(text: &str, byte: usize) -> (usize, usize) {
        let bytes = text.as_bytes();
        let prefix = &bytes[..byte.min(bytes.len())];
        let line = prefix.iter().filter(|byte| **byte == b'\n').count() + 1;
        let column = prefix
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(prefix.len(), |last| prefix.len() - last - 1)
            + 1;
        (line, column)
    }

    fn source_anchor(path: &str, text: &str, start: usize, end: usize) -> SourceAnchor {
        let (start_line, start_column) = position(text, start);
        let (end_line, end_column) = position(text, end.saturating_sub(1));
        SourceAnchor {
            path: path.to_owned(),
            source_sha256: hash(text.as_bytes()),
            start_byte: start,
            end_byte: end,
            start_line,
            end_line,
            start_column,
            end_column,
        }
    }

    fn symbol(path: &str, id: &str, name: &str, text: &str, is_test: bool) -> IndexedSymbol {
        let start = text.find("fn ").unwrap_or(0);
        IndexedSymbol {
            symbol_id: id.to_owned(),
            path: path.to_owned(),
            language: "rust".to_owned(),
            kind: SymbolKind::Function,
            name: name.to_owned(),
            qualified_name: name.to_owned(),
            declaration_identity: format!("function:{name}"),
            signature_sha256: format!("signature:{name}"),
            is_exported: false,
            is_test,
            anchor: source_anchor(path, text, start, text.len()),
        }
    }

    fn index_file(file: &CapturedFile, status: IndexFileStatus) -> IndexedFile {
        IndexedFile {
            path: file.path.clone(),
            file_id: format!("file:{}", file.path),
            language: Some("rust".to_owned()),
            status,
            parser: None,
            source_sha256: file.sha256.clone(),
            source_size_bytes: file.size_bytes,
            parse_error_count: 0,
            symbol_count: 1,
            relationship_count: 0,
            message: None,
        }
    }

    fn index(
        id: &str,
        manifest: &SnapshotManifest,
        symbols: Vec<IndexedSymbol>,
        relationships: Vec<IndexedRelationship>,
        partial_paths: &HashSet<String>,
    ) -> SnapshotIndex {
        SnapshotIndex {
            schema_version: 1,
            indexer_version: "fixture".to_owned(),
            project_id: PROJECT.to_owned(),
            snapshot_id: id.to_owned(),
            status: if partial_paths.is_empty() {
                SnapshotIndexStatus::Complete
            } else {
                SnapshotIndexStatus::Partial
            },
            parsers: Vec::new(),
            files: manifest
                .files
                .iter()
                .map(|file| {
                    index_file(
                        file,
                        if partial_paths.contains(&file.path) {
                            IndexFileStatus::PartialParse
                        } else {
                            IndexFileStatus::Indexed
                        },
                    )
                })
                .collect(),
            symbols,
            relationships,
        }
    }

    fn chapter_anchor(
        path: &str,
        text: &str,
        symbol_id: &str,
        excerpt: &str,
        id: &str,
    ) -> ChapterSourceAnchor {
        let start = text.find(excerpt).unwrap();
        let end = start + excerpt.len();
        let (start_line, start_column) = position(text, start);
        let (end_line, end_column) = position(text, end - 1);
        ChapterSourceAnchor {
            anchor_id: id.to_owned(),
            path: path.to_owned(),
            symbol_id: Some(symbol_id.to_owned()),
            snapshot_id: "old".to_owned(),
            source_sha256: hash(text.as_bytes()),
            start_byte: start as u64,
            end_byte_inclusive: (end - 1) as u64,
            start_line: start_line as u32,
            end_line: end_line as u32,
            start_column: start_column as u32,
            end_column: end_column as u32,
            excerpt: excerpt.to_owned(),
        }
    }

    fn relationship(
        id: &str,
        kind: RelationshipKind,
        source_id: &str,
        target_id: &str,
        target_path: &str,
        evidence_path: &str,
        text: &str,
        excerpt: &str,
    ) -> IndexedRelationship {
        let start = text.find(excerpt).unwrap();
        let end = start + excerpt.len();
        let mut evidence = source_anchor(evidence_path, text, start, end);
        evidence.path = evidence_path.to_owned();
        IndexedRelationship {
            relationship_id: id.to_owned(),
            kind,
            source_id: source_id.to_owned(),
            target_id: Some(target_id.to_owned()),
            target_path: Some(target_path.to_owned()),
            target_label: Some(target_id.to_owned()),
            confidence: RelationshipConfidence::High,
            unresolved_reason: None,
            evidence,
        }
    }

    fn anchored_section(
        id: &str,
        path: &str,
        text: &str,
        symbol_id: &str,
        excerpt: &str,
    ) -> ChapterSection {
        let anchor = chapter_anchor(path, text, symbol_id, excerpt, &format!("anchor:{id}"));
        let sha = anchor.source_sha256.clone();
        ChapterSection {
            section_id: id.to_owned(),
            section_revision_id: format!("section-revision:{id}"),
            order: 0,
            title: id.to_owned(),
            prose: "Evidence-backed explanation".to_owned(),
            spoken_text: String::new(),
            claims: vec![ChapterClaim {
                claim_id: format!("claim:{id}"),
                category: ClaimCategory::CodeBehavior,
                prose: "This source range supports the explanation.".to_owned(),
                anchors: vec![anchor],
                dependencies: vec![SectionDependency::Symbol {
                    path: path.to_owned(),
                    source_sha256: sha.clone(),
                    symbol_id: symbol_id.to_owned(),
                }],
            }],
            dependencies: vec![SectionDependency::SourceFile {
                path: path.to_owned(),
                source_sha256: sha,
            }],
            visual: None,
            hotspots: Vec::new(),
        }
    }

    fn relation_section(
        id: &str,
        path: &str,
        relationship: &IndexedRelationship,
    ) -> ChapterSection {
        ChapterSection {
            section_id: id.to_owned(),
            section_revision_id: format!("section-revision:{id}"),
            order: 0,
            title: id.to_owned(),
            prose: "Relationship explanation".to_owned(),
            spoken_text: String::new(),
            claims: vec![ChapterClaim {
                claim_id: format!("claim:{id}"),
                category: ClaimCategory::CodeBehavior,
                prose: "This relationship supports the explanation.".to_owned(),
                anchors: Vec::new(),
                dependencies: vec![SectionDependency::Relationship {
                    path: path.to_owned(),
                    source_sha256: relationship.evidence.source_sha256.clone(),
                    relationship_id: relationship.relationship_id.clone(),
                }],
            }],
            dependencies: Vec::new(),
            visual: None,
            hotspots: Vec::new(),
        }
    }

    fn revision(sections: Vec<ChapterSection>) -> ChapterRevision {
        ChapterRevision {
            schema_version: 2,
            chapter_id: "chapter:fixture".to_owned(),
            revision_id: "revision:accepted".to_owned(),
            revision_number: 1,
            project_id: PROJECT.to_owned(),
            snapshot_id: "old".to_owned(),
            target: ChapterTarget {
                target_id: "target:fixture".to_owned(),
                scope: ChapterTargetScope::File {
                    path: "src/line.rs".to_owned(),
                },
            },
            title: "Fixture".to_owned(),
            lifecycle: RevisionLifecycle::Accepted,
            validation_status: ValidationStatus::Valid,
            created_at_unix_ms: 1,
            based_on_revision_id: None,
            decision_note: None,
            generation_origin: None,
            sections,
        }
    }

    fn assert_range_identity(
        range: &FreshnessLineRange,
        path: &str,
        snapshot_id: &str,
        source_sha256: &str,
    ) {
        assert_eq!(range.path, path);
        assert_eq!(range.snapshot_id, snapshot_id);
        assert_eq!(range.source_sha256, source_sha256);
    }

    fn visual_section() -> ChapterSection {
        let mut section = anchored_section(
            "visual-section",
            "src/visual.rs",
            "fn run() { worker(); }\n",
            "symbol:run",
            "worker()",
        );
        let evidence_anchor_id = section.claims[0].anchors[0].anchor_id.clone();
        section.hotspots = vec![
            ChapterHotspot {
                hotspot_id: "hotspot:used".to_owned(),
                label: "Open the worker section".to_owned(),
                destination: HotspotDestination::Section {
                    section_id: "visual-section".to_owned(),
                },
            },
            ChapterHotspot {
                hotspot_id: "hotspot:unused".to_owned(),
                label: "Open an unused file".to_owned(),
                destination: HotspotDestination::AtlasFile {
                    path: "src/unused.rs".to_owned(),
                },
            },
        ];
        section.visual = Some(ChapterVisualPayload::Typed(VisualSpec {
            presentation: serde_json::from_value(serde_json::json!({
                "schemaVersion":3,"scenes":[{"claimIndex":0,"title":"The entry point calls its worker",
                    "narration":"The entry point calls the worker and then returns to its caller.",
                    "mode":"graph","elements":[]}]
            })).unwrap(),
            schema_version: 3,
            kind: VisualKind::DataFlow,
            title: "Worker flow".to_owned(),
            layout_hint: Some(VisualLayoutHint {
                direction: VisualLayoutDirection::LeftToRight,
            }),
            nodes: vec![
                VisualNode {
                    node_id: "node:run".to_owned(),
                    label: "run".to_owned(),
                    detail: Some("entry point".to_owned()),
                    hotspot_id: Some("hotspot:used".to_owned()),
                    evidence_anchor_ids: vec![evidence_anchor_id.clone()],
                    source_anchor_id: None,
                    source_value: None,
                },
                VisualNode {
                    node_id: "node:worker".to_owned(),
                    label: "worker".to_owned(),
                    detail: None,
                    hotspot_id: None,
                    evidence_anchor_ids: vec![evidence_anchor_id.clone()],
                    source_anchor_id: None,
                    source_value: None,
                },
            ],
            edges: vec![VisualEdge {
                edge_id: "edge:calls".to_owned(),
                from_node_id: "node:run".to_owned(),
                to_node_id: "node:worker".to_owned(),
                label: "calls".to_owned(),
                evidence_anchor_ids: vec![evidence_anchor_id],
            }],
        }));
        section
    }

    fn visual_asset_id(revision: &ChapterRevision, section_id: &str) -> String {
        let anchors_by_id = revision
            .sections
            .iter()
            .flat_map(|section| &section.claims)
            .flat_map(|claim| &claim.anchors)
            .map(|anchor| (anchor.anchor_id.clone(), anchor))
            .collect::<HashMap<_, _>>();
        let section = revision
            .sections
            .iter()
            .find(|section| section.section_id == section_id)
            .unwrap();
        let Some(visual) = section
            .visual
            .as_ref()
            .and_then(ChapterVisualPayload::typed_visual)
        else {
            panic!("fixture should contain a typed visual");
        };
        visual_asset_identity(&revision.chapter_id, section, visual, &anchors_by_id)
            .unwrap()
            .0
    }

    #[test]
    fn visual_asset_fingerprint_tracks_only_rendered_data_evidence_and_used_hotspots() {
        let original = revision(vec![visual_section()]);
        let original_id = visual_asset_id(&original, "visual-section");

        let mut prose_only = original.clone();
        let section = &mut prose_only.sections[0];
        section.title = "Renamed section title".to_owned();
        section.prose = "Revised explanation text".to_owned();
        section.spoken_text = "Revised narration".to_owned();
        section.order = 42;
        section.section_revision_id = "section-revision:next".to_owned();
        section.claims[0].prose = "Reworded claim".to_owned();
        let mut unused_anchor = section.claims[0].anchors[0].clone();
        unused_anchor.anchor_id = "anchor:unlinked".to_owned();
        unused_anchor.excerpt = "fn run()".to_owned();
        unused_anchor.start_byte = 0;
        unused_anchor.end_byte_inclusive = 7;
        section.claims[0].anchors.push(unused_anchor);
        section.hotspots[1].label = "Changed unused hotspot".to_owned();
        section.hotspots[1].destination = HotspotDestination::AtlasFile {
            path: "src/another-unused.rs".to_owned(),
        };
        let mut unrelated = anchored_section(
            "unrelated-section",
            "src/other.rs",
            "fn other() {}\n",
            "symbol:other",
            "other()",
        );
        unrelated.title = "Unrelated section updated".to_owned();
        prose_only.sections.push(unrelated);
        assert_eq!(visual_asset_id(&prose_only, "visual-section"), original_id);

        let mut changed_node = original.clone();
        let Some(visual) = changed_node.sections[0]
            .visual
            .as_mut()
            .and_then(ChapterVisualPayload::typed_visual_mut)
        else {
            panic!("fixture should contain a typed visual");
        };
        visual.nodes[0].label = "renamed entry".to_owned();
        assert_ne!(
            visual_asset_id(&changed_node, "visual-section"),
            original_id
        );

        let mut changed_anchor = original.clone();
        changed_anchor.sections[0].claims[0].anchors[0].excerpt = "worker() changed".to_owned();
        assert_ne!(
            visual_asset_id(&changed_anchor, "visual-section"),
            original_id
        );

        let mut changed_snapshot_link = original.clone();
        changed_snapshot_link.sections[0].claims[0].anchors[0].snapshot_id =
            "snapshot:next".to_owned();
        assert_eq!(
            visual_asset_id(&changed_snapshot_link, "visual-section"),
            original_id,
            "an unchanged visual keeps its identity when its exact citation is carried to a new snapshot"
        );

        let mut changed_used_hotspot = original.clone();
        changed_used_hotspot.sections[0].hotspots[0].destination = HotspotDestination::AtlasFile {
            path: "src/worker.rs".to_owned(),
        };
        assert_ne!(
            visual_asset_id(&changed_used_hotspot, "visual-section"),
            original_id
        );
    }

    #[test]
    fn visual_asset_reverse_links_only_explicit_evidence_and_used_code_lens_inputs() {
        let mut section = visual_section();
        let mut unlinked_anchor = chapter_anchor(
            "src/visual.rs",
            "fn run() { worker(); }\n",
            "symbol:run",
            "fn run()",
            "anchor:unlinked",
        );
        unlinked_anchor.start_byte = 0;
        section.claims[0].anchors.push(unlinked_anchor.clone());
        let unrelated_anchor = chapter_anchor(
            "src/visual.rs",
            "fn run() { worker(); }\n",
            "symbol:run",
            "fn run() { worker(); }",
            "anchor:unrelated",
        );
        section.claims[0].anchors.push(unrelated_anchor.clone());
        section.hotspots[0].destination = HotspotDestination::CodeLens {
            anchor_id: unlinked_anchor.anchor_id.clone(),
        };
        let revision = revision(vec![section]);
        let visual_id = visual_asset_id(&revision, "visual-section");
        let dependencies = build_reverse_dependencies(&revision, &[]);
        let linked_anchor = &revision.sections[0].claims[0].anchors[0];
        let linked = dependencies
            .iter()
            .find(|item| item.evidence_key == anchor_dependency_key(linked_anchor))
            .unwrap();
        let code_lens_target = dependencies
            .iter()
            .find(|item| item.evidence_key == anchor_dependency_key(&unlinked_anchor))
            .unwrap();
        let unrelated = dependencies
            .iter()
            .find(|item| item.evidence_key == anchor_dependency_key(&unrelated_anchor))
            .unwrap();
        assert!(linked.asset_ids.contains(&visual_id));
        assert!(code_lens_target.asset_ids.contains(&visual_id));
        assert!(!unrelated.asset_ids.contains(&visual_id));
        let source_file = dependencies
            .iter()
            .find(|item| item.evidence_key.starts_with("sourceFile:"))
            .unwrap();
        assert!(!source_file.asset_ids.contains(&visual_id));

        let mut no_visual = revision.clone();
        no_visual.sections[0].visual = None;
        assert!(
            build_reverse_dependencies(&no_visual, &[])
                .iter()
                .all(|item| {
                    item.asset_ids
                        .iter()
                        .all(|asset| !asset.starts_with("visual:"))
                })
        );
    }

    #[test]
    fn multi_file_assessment_only_stales_dependent_sections_and_preserves_accepted_bytes() {
        let storage = TempStorage::new();
        let old_line = "fn line_example() { let value = 7; }\n";
        let new_line = "// inserted\nfn line_example() { let value = 7; }\n";
        let old_edit = "fn edit_example() { let value = 1; }\n";
        let new_edit = "fn edit_example() { let value = 2; }\n";
        let old_test =
            "#[test]\nfn test_math() { let result = calculate(); assert_eq!(result, 1); }\n";
        let new_test =
            "#[test]\nfn test_math() { let result = calculate(); assert_eq!(result, 2); }\n";
        let old_caller = "fn caller() { alpha(); }\n";
        let new_caller = "fn caller() { beta(); }\n";
        let alpha = "fn alpha() {}\n";
        let beta = "fn beta() {}\n";
        let math = "fn calculate() -> i32 { 1 }\n";
        let untouched = "fn untouched() { true }\n";
        let unrelated_old = "fn unrelated() { 1 }\n";
        let unrelated_new = "fn unrelated() { 2 }\n";

        let old_manifest = manifest(
            "old",
            vec![
                put_file(&storage.0, "src/line.rs", old_line),
                put_file(&storage.0, "src/edit.rs", old_edit),
                put_file(&storage.0, "tests/math.rs", old_test),
                put_file(&storage.0, "src/caller.rs", old_caller),
                put_file(&storage.0, "src/alpha.rs", alpha),
                put_file(&storage.0, "src/beta.rs", beta),
                put_file(&storage.0, "src/math.rs", math),
                put_file(&storage.0, "src/untouched.rs", untouched),
                put_file(&storage.0, "src/unrelated.rs", unrelated_old),
            ],
        );
        let new_manifest = manifest(
            "new",
            vec![
                put_file(&storage.0, "src/line.rs", new_line),
                put_file(&storage.0, "src/edit.rs", new_edit),
                put_file(&storage.0, "tests/math.rs", new_test),
                put_file(&storage.0, "src/caller.rs", new_caller),
                put_file(&storage.0, "src/alpha.rs", alpha),
                put_file(&storage.0, "src/beta.rs", beta),
                put_file(&storage.0, "src/math.rs", math),
                put_file(&storage.0, "src/untouched.rs", untouched),
                put_file(&storage.0, "src/unrelated.rs", unrelated_new),
            ],
        );
        let old_symbols = vec![
            symbol("src/line.rs", "sym-line", "line_example", old_line, false),
            symbol("src/edit.rs", "sym-edit", "edit_example", old_edit, false),
            symbol("tests/math.rs", "sym-test", "test_math", old_test, true),
            symbol("src/caller.rs", "sym-caller", "caller", old_caller, false),
            symbol("src/alpha.rs", "sym-alpha", "alpha", alpha, false),
            symbol("src/beta.rs", "sym-beta", "beta", beta, false),
            symbol("src/math.rs", "sym-calculate", "calculate", math, false),
            symbol(
                "src/untouched.rs",
                "sym-untouched",
                "untouched",
                untouched,
                false,
            ),
            symbol(
                "src/unrelated.rs",
                "sym-unrelated",
                "unrelated",
                unrelated_old,
                false,
            ),
        ];
        let new_symbols = vec![
            symbol("src/line.rs", "sym-line", "line_example", new_line, false),
            symbol("src/edit.rs", "sym-edit", "edit_example", new_edit, false),
            symbol("tests/math.rs", "sym-test", "test_math", new_test, true),
            symbol("src/caller.rs", "sym-caller", "caller", new_caller, false),
            symbol("src/alpha.rs", "sym-alpha", "alpha", alpha, false),
            symbol("src/beta.rs", "sym-beta", "beta", beta, false),
            symbol("src/math.rs", "sym-calculate", "calculate", math, false),
            symbol(
                "src/untouched.rs",
                "sym-untouched",
                "untouched",
                untouched,
                false,
            ),
            symbol(
                "src/unrelated.rs",
                "sym-unrelated",
                "unrelated",
                unrelated_new,
                false,
            ),
        ];
        let old_test_rel = relationship(
            "test:old-offset",
            RelationshipKind::TestReference,
            "sym-test",
            "sym-calculate",
            "src/math.rs",
            "tests/math.rs",
            old_test,
            "calculate()",
        );
        let new_test_rel = relationship(
            "test:new-offset",
            RelationshipKind::TestReference,
            "sym-test",
            "sym-calculate",
            "src/math.rs",
            "tests/math.rs",
            new_test,
            "calculate()",
        );
        let old_call_rel = relationship(
            "call:old",
            RelationshipKind::Calls,
            "sym-caller",
            "sym-alpha",
            "src/alpha.rs",
            "src/caller.rs",
            old_caller,
            "alpha()",
        );
        let new_call_rel = relationship(
            "call:new",
            RelationshipKind::Calls,
            "sym-caller",
            "sym-beta",
            "src/beta.rs",
            "src/caller.rs",
            new_caller,
            "beta()",
        );
        let old_index = index(
            "old",
            &old_manifest,
            old_symbols,
            vec![old_test_rel.clone(), old_call_rel.clone()],
            &HashSet::new(),
        );
        let new_index = index(
            "new",
            &new_manifest,
            new_symbols,
            vec![new_test_rel, new_call_rel],
            &HashSet::new(),
        );

        let revision = revision(vec![
            anchored_section(
                "line-shift",
                "src/line.rs",
                old_line,
                "sym-line",
                "let value = 7;",
            ),
            anchored_section(
                "cited-edit",
                "src/edit.rs",
                old_edit,
                "sym-edit",
                "let value = 1;",
            ),
            relation_section("test-evidence", "tests/math.rs", &old_test_rel),
            relation_section("caller-edge", "src/caller.rs", &old_call_rel),
            anchored_section(
                "unrelated-file-edit",
                "src/untouched.rs",
                untouched,
                "sym-untouched",
                "true",
            ),
            ChapterSection {
                section_id: "unreferenced".to_owned(),
                section_revision_id: "section-revision:unreferenced".to_owned(),
                order: 5,
                title: "Repository context".to_owned(),
                prose: "No source dependency".to_owned(),
                spoken_text: String::new(),
                claims: Vec::new(),
                dependencies: Vec::new(),
                visual: None,
                hotspots: Vec::<ChapterHotspot>::new(),
            },
        ]);
        let before = serde_json::to_vec(&revision).unwrap();

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &revision,
        )
        .unwrap();
        let statuses: HashMap<&str, SectionFreshnessStatus> = report
            .sections
            .iter()
            .map(|section| (section.section_id.as_str(), section.status))
            .collect();
        assert_eq!(
            statuses["line-shift"],
            SectionFreshnessStatus::SafeToCarry,
            "{:#?}",
            report.sections
        );
        assert_eq!(statuses["cited-edit"], SectionFreshnessStatus::Stale);
        assert_eq!(statuses["test-evidence"], SectionFreshnessStatus::Stale);
        assert_eq!(statuses["caller-edge"], SectionFreshnessStatus::Stale);
        assert_eq!(
            statuses["unrelated-file-edit"],
            SectionFreshnessStatus::Current
        );
        assert_eq!(statuses["unreferenced"], SectionFreshnessStatus::Current);
        let line_change = report
            .sections
            .iter()
            .find(|section| section.section_id == "line-shift")
            .unwrap()
            .source_changes
            .first()
            .unwrap();
        let old_range = line_change.old_range.as_ref().unwrap();
        let new_range = line_change.new_range.as_ref().unwrap();
        assert_eq!((old_range.start_line, old_range.end_line), (1, 1));
        assert_eq!((new_range.start_line, new_range.end_line), (2, 2));
        assert_range_identity(
            old_range,
            "src/line.rs",
            "old",
            &old_manifest
                .files
                .iter()
                .find(|file| file.path == "src/line.rs")
                .unwrap()
                .sha256,
        );
        assert_range_identity(
            new_range,
            "src/line.rs",
            "new",
            &new_manifest
                .files
                .iter()
                .find(|file| file.path == "src/line.rs")
                .unwrap()
                .sha256,
        );
        assert_eq!(
            serde_json::to_vec(&revision).unwrap(),
            before,
            "freshness must not mutate the accepted revision"
        );
        assert!(report.reverse_dependencies.iter().any(|dependency| {
            dependency.path == "src/line.rs"
                && dependency
                    .dependent_section_ids
                    .contains(&"line-shift".to_owned())
        }));
    }

    #[test]
    fn incomplete_index_is_needs_review_instead_of_stale() {
        let storage = TempStorage::new();
        let old_text = "fn line_example() { let value = 7; }\n";
        let new_text = "// inserted\nfn line_example() { let value = 7; }\n";
        let old_manifest = manifest("old", vec![put_file(&storage.0, "src/line.rs", old_text)]);
        let new_manifest = manifest("new", vec![put_file(&storage.0, "src/line.rs", new_text)]);
        let old_index = index(
            "old",
            &old_manifest,
            vec![symbol(
                "src/line.rs",
                "sym-line",
                "line_example",
                old_text,
                false,
            )],
            Vec::new(),
            &HashSet::new(),
        );
        let partial_path = HashSet::from(["src/line.rs".to_owned()]);
        let new_index = index(
            "new",
            &new_manifest,
            vec![symbol(
                "src/line.rs",
                "sym-line",
                "line_example",
                new_text,
                false,
            )],
            Vec::new(),
            &partial_path,
        );
        let revision = revision(vec![anchored_section(
            "line-shift",
            "src/line.rs",
            old_text,
            "sym-line",
            "let value = 7;",
        )]);
        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &revision,
        )
        .unwrap();
        assert_eq!(
            report.sections[0].status,
            SectionFreshnessStatus::NeedsReview
        );
        assert!(
            report.sections[0]
                .reasons
                .iter()
                .any(|reason| reason.code == "indexIncomplete")
        );
        assert!(report.sections[0].source_changes.is_empty());
    }

    #[test]
    fn unavailable_target_index_is_needs_review_with_unavailable_cross_snapshot_evidence() {
        let storage = TempStorage::new();
        let old_text = "fn cited() { let value = 7; }\n";
        let new_text = "fn cited() { let value = 8; }\n";
        let old_manifest = manifest("old", vec![put_file(&storage.0, "src/cited.rs", old_text)]);
        let new_manifest = manifest("new", vec![put_file(&storage.0, "src/cited.rs", new_text)]);
        let old_index = index(
            "old",
            &old_manifest,
            vec![symbol(
                "src/cited.rs",
                "sym-cited",
                "cited",
                old_text,
                false,
            )],
            Vec::new(),
            &HashSet::new(),
        );
        let accepted = revision(vec![anchored_section(
            "changed-citation",
            "src/cited.rs",
            old_text,
            "sym-cited",
            "value = 7;",
        )]);

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            None,
            &accepted,
        )
        .unwrap();

        assert!(matches!(
            report.indexes.from.status,
            FreshnessIndexStatus::Complete
        ));
        assert!(matches!(
            report.indexes.to.status,
            FreshnessIndexStatus::Unavailable
        ));
        assert_eq!(
            report.sections[0].status,
            SectionFreshnessStatus::NeedsReview
        );
        assert_eq!(
            report.sections[0].evidence_state,
            EvidenceState::Unavailable
        );
        assert!(report.sections[0].source_changes.is_empty());
        assert!(report.sections[0].reasons.iter().any(|reason| {
            reason.code == "indexIncomplete" && reason.path.as_deref() == Some("src/cited.rs")
        }));
    }

    #[test]
    fn no_symbol_anchor_is_current_when_unchanged_and_stale_when_deleted() {
        let storage = TempStorage::new();
        let text = "fn cited() { let value = 7; }\n";
        let old_file = put_file(&storage.0, "src/cited.rs", text);
        let old_manifest = manifest("old", vec![old_file]);
        let old_index = index(
            "old",
            &old_manifest,
            vec![symbol("src/cited.rs", "sym-cited", "cited", text, false)],
            Vec::new(),
            &HashSet::new(),
        );
        let mut section =
            anchored_section("no-symbol", "src/cited.rs", text, "sym-cited", "value = 7;");
        section.claims[0].anchors[0].symbol_id = None;
        section.claims[0].dependencies.clear();
        section.dependencies.clear();
        let accepted = revision(vec![section]);

        let unchanged_manifest = manifest(
            "unchanged",
            vec![put_file(&storage.0, "src/cited.rs", text)],
        );
        let unchanged = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &unchanged_manifest,
            None,
            &accepted,
        )
        .unwrap();
        assert_eq!(
            unchanged.sections[0].status,
            SectionFreshnessStatus::Current
        );

        let deleted_manifest = manifest("deleted", Vec::new());
        let deleted = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &deleted_manifest,
            None,
            &accepted,
        )
        .unwrap();
        assert_eq!(deleted.sections[0].status, SectionFreshnessStatus::Stale);
        assert_eq!(deleted.sections[0].reasons[0].code, "sourceDeleted");

        let changed_manifest = manifest(
            "changed",
            vec![put_file(
                &storage.0,
                "src/cited.rs",
                "fn cited() { let value = 8; }\n",
            )],
        );
        let changed = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &changed_manifest,
            None,
            &accepted,
        )
        .unwrap();
        assert_eq!(
            changed.sections[0].status,
            SectionFreshnessStatus::NeedsReview
        );
        assert_eq!(
            changed.sections[0].reasons[0].code,
            "stableSymbolUnavailable"
        );
    }

    #[test]
    fn present_but_unreadable_target_file_is_needs_review_not_symbol_removal() {
        let storage = TempStorage::new();
        let old_text = "fn cited() { let value = 7; }\n";
        let new_text = "fn replacement() { let value = 8; }\n";
        let old_manifest = manifest("old", vec![put_file(&storage.0, "src/cited.rs", old_text)]);
        let new_file = put_file(&storage.0, "src/cited.rs", new_text);
        let blob_path = storage.0.join(&new_file.blob_path);
        let new_manifest = manifest("new", vec![new_file]);
        let old_index = index(
            "old",
            &old_manifest,
            vec![symbol(
                "src/cited.rs",
                "sym-cited",
                "cited",
                old_text,
                false,
            )],
            Vec::new(),
            &HashSet::new(),
        );
        let new_index = index(
            "new",
            &new_manifest,
            Vec::new(),
            Vec::new(),
            &HashSet::new(),
        );
        let mut section = anchored_section(
            "unreadable-target",
            "src/cited.rs",
            old_text,
            "sym-cited",
            "value = 7;",
        );
        section.claims[0].dependencies.clear();
        section.dependencies.clear();
        fs::remove_file(blob_path).unwrap();

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &revision(vec![section]),
        )
        .unwrap();
        assert_eq!(
            report.sections[0].status,
            SectionFreshnessStatus::NeedsReview
        );
        assert_eq!(report.sections[0].reasons[0].code, "sourceUnavailable");
        assert!(report.sections[0].source_changes.is_empty());
    }

    #[test]
    fn duplicate_excerpt_is_needs_review_in_freshness_assessment() {
        let storage = TempStorage::new();
        let old_text = "fn cited() { let value = 7; }\n";
        let new_text = "fn cited() { let value = 7; let again = 7; }\n";
        let old_manifest = manifest("old", vec![put_file(&storage.0, "src/cited.rs", old_text)]);
        let new_manifest = manifest("new", vec![put_file(&storage.0, "src/cited.rs", new_text)]);
        let old_index = index(
            "old",
            &old_manifest,
            vec![symbol(
                "src/cited.rs",
                "sym-cited",
                "cited",
                old_text,
                false,
            )],
            Vec::new(),
            &HashSet::new(),
        );
        let new_index = index(
            "new",
            &new_manifest,
            vec![symbol(
                "src/cited.rs",
                "sym-cited",
                "cited",
                new_text,
                false,
            )],
            Vec::new(),
            &HashSet::new(),
        );
        let mut section = anchored_section(
            "ambiguous-excerpt",
            "src/cited.rs",
            old_text,
            "sym-cited",
            "7",
        );
        section.claims[0].dependencies.clear();
        section.dependencies.clear();

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &revision(vec![section]),
        )
        .unwrap();
        assert_eq!(
            report.sections[0].status,
            SectionFreshnessStatus::NeedsReview
        );
        assert_eq!(report.sections[0].reasons[0].code, "ambiguousRemap");
        assert!(report.sections[0].source_changes.is_empty());
    }

    #[test]
    fn changed_call_text_with_same_semantic_edge_is_stale_not_current() {
        let storage = TempStorage::new();
        let old_caller = "fn caller() { alpha(); }\n";
        let new_caller = "fn caller() { alpha(1); }\n";
        let alpha = "fn alpha() {}\n";
        let old_manifest = manifest(
            "old",
            vec![
                put_file(&storage.0, "src/caller.rs", old_caller),
                put_file(&storage.0, "src/alpha.rs", alpha),
            ],
        );
        let new_manifest = manifest(
            "new",
            vec![
                put_file(&storage.0, "src/caller.rs", new_caller),
                put_file(&storage.0, "src/alpha.rs", alpha),
            ],
        );
        let old_relationship = relationship(
            "call:old",
            RelationshipKind::Calls,
            "sym-caller",
            "sym-alpha",
            "src/alpha.rs",
            "src/caller.rs",
            old_caller,
            "alpha()",
        );
        let new_relationship = relationship(
            "call:new",
            RelationshipKind::Calls,
            "sym-caller",
            "sym-alpha",
            "src/alpha.rs",
            "src/caller.rs",
            new_caller,
            "alpha(1)",
        );
        let old_index = index(
            "old",
            &old_manifest,
            vec![
                symbol("src/caller.rs", "sym-caller", "caller", old_caller, false),
                symbol("src/alpha.rs", "sym-alpha", "alpha", alpha, false),
            ],
            vec![old_relationship.clone()],
            &HashSet::new(),
        );
        let new_index = index(
            "new",
            &new_manifest,
            vec![
                symbol("src/caller.rs", "sym-caller", "caller", new_caller, false),
                symbol("src/alpha.rs", "sym-alpha", "alpha", alpha, false),
            ],
            vec![new_relationship],
            &HashSet::new(),
        );

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &revision(vec![relation_section(
                "changed-call",
                "src/caller.rs",
                &old_relationship,
            )]),
        )
        .unwrap();
        assert_eq!(report.sections[0].status, SectionFreshnessStatus::Stale);
        assert_eq!(report.sections[0].reasons[0].code, "relationshipChanged");
    }

    #[test]
    fn removed_relationship_endpoint_range_targets_the_old_evidence_snapshot_and_path() {
        let storage = TempStorage::new();
        let caller = "fn caller() { gone(); }\n";
        let endpoint = "pub fn gone() {}\n";
        let caller_file = put_file(&storage.0, "src/caller.rs", caller);
        let endpoint_file = put_file(&storage.0, "src/gone.rs", endpoint);
        let old_manifest = manifest("old", vec![caller_file, endpoint_file]);
        let new_manifest = manifest("new", vec![put_file(&storage.0, "src/caller.rs", caller)]);
        let old_relationship = relationship(
            "call:removed-endpoint",
            RelationshipKind::Calls,
            "sym-caller",
            "sym-gone",
            "src/gone.rs",
            "src/caller.rs",
            caller,
            "gone()",
        );
        let old_index = index(
            "old",
            &old_manifest,
            vec![
                symbol("src/caller.rs", "sym-caller", "caller", caller, false),
                symbol("src/gone.rs", "sym-gone", "gone", endpoint, false),
            ],
            vec![old_relationship.clone()],
            &HashSet::new(),
        );
        let new_index = index(
            "new",
            &new_manifest,
            vec![symbol(
                "src/caller.rs",
                "sym-caller",
                "caller",
                caller,
                false,
            )],
            Vec::new(),
            &HashSet::new(),
        );

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &revision(vec![relation_section(
                "removed-endpoint",
                "src/caller.rs",
                &old_relationship,
            )]),
        )
        .unwrap();

        assert_eq!(report.sections[0].status, SectionFreshnessStatus::Stale);
        assert_eq!(
            report.sections[0].reasons[0].code,
            "relationshipEndpointRemoved"
        );
        let change = &report.sections[0].source_changes[0];
        assert_eq!(change.path, "src/gone.rs");
        let old_range = change.old_range.as_ref().unwrap();
        assert_range_identity(old_range, "src/caller.rs", "old", &hash(caller.as_bytes()));
        assert_eq!((old_range.start_line, old_range.end_line), (1, 1));
        assert!(change.new_range.is_none());

        let serialized = serde_json::to_value(old_range).unwrap();
        assert_eq!(serialized["path"], "src/caller.rs");
        assert_eq!(serialized["snapshotId"], "old");
        assert_eq!(serialized["sourceSha256"], hash(caller.as_bytes()));
    }

    #[test]
    fn file_level_import_relationship_remaps_without_an_enclosing_symbol() {
        let storage = TempStorage::new();
        let old_source = "use crate::alpha;\n";
        let new_source = "// moved down\nuse crate::alpha;\n";
        let alpha = "pub fn alpha() {}\n";
        let old_manifest = manifest(
            "old",
            vec![
                put_file(&storage.0, "src/lib.rs", old_source),
                put_file(&storage.0, "src/alpha.rs", alpha),
            ],
        );
        let new_manifest = manifest(
            "new",
            vec![
                put_file(&storage.0, "src/lib.rs", new_source),
                put_file(&storage.0, "src/alpha.rs", alpha),
            ],
        );
        let old_relationship = relationship(
            "import:old",
            RelationshipKind::Imports,
            "file:src/lib.rs",
            "file:src/alpha.rs",
            "src/alpha.rs",
            "src/lib.rs",
            old_source,
            "use crate::alpha;",
        );
        let new_relationship = relationship(
            "import:new",
            RelationshipKind::Imports,
            "file:src/lib.rs",
            "file:src/alpha.rs",
            "src/alpha.rs",
            "src/lib.rs",
            new_source,
            "use crate::alpha;",
        );
        let old_index = index(
            "old",
            &old_manifest,
            vec![symbol("src/alpha.rs", "sym-alpha", "alpha", alpha, false)],
            vec![old_relationship.clone()],
            &HashSet::new(),
        );
        let new_index = index(
            "new",
            &new_manifest,
            vec![symbol("src/alpha.rs", "sym-alpha", "alpha", alpha, false)],
            vec![new_relationship],
            &HashSet::new(),
        );

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &revision(vec![relation_section(
                "file-import",
                "src/lib.rs",
                &old_relationship,
            )]),
        )
        .unwrap();
        assert_eq!(
            report.sections[0].status,
            SectionFreshnessStatus::SafeToCarry
        );
        let change = &report.sections[0].source_changes[0];
        let old_range = change.old_range.as_ref().unwrap();
        let new_range = change.new_range.as_ref().unwrap();
        assert_eq!((old_range.start_line, old_range.end_line), (1, 1));
        assert_eq!((new_range.start_line, new_range.end_line), (2, 2));
        assert_range_identity(old_range, "src/lib.rs", "old", &hash(old_source.as_bytes()));
        assert_range_identity(new_range, "src/lib.rs", "new", &hash(new_source.as_bytes()));
    }

    #[test]
    fn ambiguous_file_level_relationship_excerpt_needs_review() {
        let storage = TempStorage::new();
        let old_source = "use crate::alpha;\n";
        let new_source = "use crate::alpha;\nuse crate::alpha;\n";
        let alpha = "pub fn alpha() {}\n";
        let old_manifest = manifest(
            "old",
            vec![
                put_file(&storage.0, "src/lib.rs", old_source),
                put_file(&storage.0, "src/alpha.rs", alpha),
            ],
        );
        let new_manifest = manifest(
            "new",
            vec![
                put_file(&storage.0, "src/lib.rs", new_source),
                put_file(&storage.0, "src/alpha.rs", alpha),
            ],
        );
        let old_relationship = relationship(
            "import:old",
            RelationshipKind::Imports,
            "file:src/lib.rs",
            "file:src/alpha.rs",
            "src/alpha.rs",
            "src/lib.rs",
            old_source,
            "use crate::alpha;",
        );
        let new_relationship = relationship(
            "import:new",
            RelationshipKind::Imports,
            "file:src/lib.rs",
            "file:src/alpha.rs",
            "src/alpha.rs",
            "src/lib.rs",
            new_source,
            "use crate::alpha;",
        );
        let old_index = index(
            "old",
            &old_manifest,
            vec![symbol("src/alpha.rs", "sym-alpha", "alpha", alpha, false)],
            vec![old_relationship.clone()],
            &HashSet::new(),
        );
        let new_index = index(
            "new",
            &new_manifest,
            vec![symbol("src/alpha.rs", "sym-alpha", "alpha", alpha, false)],
            vec![new_relationship],
            &HashSet::new(),
        );

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &revision(vec![relation_section(
                "ambiguous-import",
                "src/lib.rs",
                &old_relationship,
            )]),
        )
        .unwrap();
        assert_eq!(
            report.sections[0].status,
            SectionFreshnessStatus::NeedsReview
        );
        assert_eq!(
            report.sections[0].reasons[0].code,
            "relationshipEvidenceAmbiguous"
        );
        assert!(report.sections[0].source_changes.is_empty());
    }

    #[test]
    fn unresolved_edge_with_partial_new_target_is_needs_review() {
        let storage = TempStorage::new();
        let old_caller = "fn caller() { unknown(); }\n";
        let new_caller = "fn caller() { beta(); }\n";
        let beta = "fn beta() {}\n";
        let old_manifest = manifest(
            "old",
            vec![
                put_file(&storage.0, "src/caller.rs", old_caller),
                put_file(&storage.0, "src/beta.rs", beta),
            ],
        );
        let new_manifest = manifest(
            "new",
            vec![
                put_file(&storage.0, "src/caller.rs", new_caller),
                put_file(&storage.0, "src/beta.rs", beta),
            ],
        );
        let mut old_relationship = relationship(
            "call:old",
            RelationshipKind::Calls,
            "sym-caller",
            "sym-unknown",
            "src/unknown.rs",
            "src/caller.rs",
            old_caller,
            "unknown()",
        );
        old_relationship.target_id = None;
        old_relationship.target_path = None;
        old_relationship.confidence = RelationshipConfidence::Unresolved;
        old_relationship.unresolved_reason = Some("not_resolved".to_owned());
        let new_relationship = relationship(
            "call:new",
            RelationshipKind::Calls,
            "sym-caller",
            "sym-beta",
            "src/beta.rs",
            "src/caller.rs",
            new_caller,
            "beta()",
        );
        let old_index = index(
            "old",
            &old_manifest,
            vec![symbol(
                "src/caller.rs",
                "sym-caller",
                "caller",
                old_caller,
                false,
            )],
            vec![old_relationship.clone()],
            &HashSet::new(),
        );
        let partial_paths = HashSet::from(["src/beta.rs".to_owned()]);
        let new_index = index(
            "new",
            &new_manifest,
            vec![
                symbol("src/caller.rs", "sym-caller", "caller", new_caller, false),
                symbol("src/beta.rs", "sym-beta", "beta", beta, false),
            ],
            vec![new_relationship],
            &partial_paths,
        );

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &revision(vec![relation_section(
                "partial-target",
                "src/caller.rs",
                &old_relationship,
            )]),
        )
        .unwrap();
        assert_eq!(
            report.sections[0].status,
            SectionFreshnessStatus::NeedsReview
        );
        assert!(report.sections[0].source_changes.is_empty());
    }

    #[test]
    fn offset_sensitive_relationship_id_survives_an_exact_line_shift() {
        let storage = TempStorage::new();
        let old_test = "#[test]\nfn test_math() { let value = calculate(); }\n";
        let new_test = "// moved\n#[test]\nfn test_math() { let value = calculate(); }\n";
        let math = "fn calculate() -> i32 { 1 }\n";
        let old_manifest = manifest(
            "old",
            vec![
                put_file(&storage.0, "tests/math.rs", old_test),
                put_file(&storage.0, "src/math.rs", math),
            ],
        );
        let new_manifest = manifest(
            "new",
            vec![
                put_file(&storage.0, "tests/math.rs", new_test),
                put_file(&storage.0, "src/math.rs", math),
            ],
        );
        let old_relationship = relationship(
            "relation:old-offset",
            RelationshipKind::TestReference,
            "sym-test",
            "sym-math",
            "src/math.rs",
            "tests/math.rs",
            old_test,
            "calculate()",
        );
        let new_relationship = relationship(
            "relation:new-offset",
            RelationshipKind::TestReference,
            "sym-test",
            "sym-math",
            "src/math.rs",
            "tests/math.rs",
            new_test,
            "calculate()",
        );
        assert_ne!(
            old_relationship.relationship_id,
            new_relationship.relationship_id
        );
        let old_index = index(
            "old",
            &old_manifest,
            vec![
                symbol("tests/math.rs", "sym-test", "test_math", old_test, true),
                symbol("src/math.rs", "sym-math", "calculate", math, false),
            ],
            vec![old_relationship.clone()],
            &HashSet::new(),
        );
        let new_index = index(
            "new",
            &new_manifest,
            vec![
                symbol("tests/math.rs", "sym-test", "test_math", new_test, true),
                symbol("src/math.rs", "sym-math", "calculate", math, false),
            ],
            vec![new_relationship],
            &HashSet::new(),
        );
        let accepted = revision(vec![relation_section(
            "test-reference",
            "tests/math.rs",
            &old_relationship,
        )]);

        let report = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &new_manifest,
            Some(&new_index),
            &accepted,
        )
        .unwrap();
        assert_eq!(
            report.sections[0].status,
            SectionFreshnessStatus::SafeToCarry
        );
        let change = &report.sections[0].source_changes[0];
        assert_eq!(change.kind, "shifted");
        let old_range = change.old_range.as_ref().unwrap();
        let new_range = change.new_range.as_ref().unwrap();
        assert_eq!((old_range.start_line, old_range.end_line), (2, 2));
        assert_eq!((new_range.start_line, new_range.end_line), (3, 3));
        assert_range_identity(
            old_range,
            "tests/math.rs",
            "old",
            &hash(old_test.as_bytes()),
        );
        assert_range_identity(
            new_range,
            "tests/math.rs",
            "new",
            &hash(new_test.as_bytes()),
        );
    }

    #[test]
    fn complete_absence_is_deleted_but_skipped_source_is_inconclusive() {
        let storage = TempStorage::new();
        let text = "fn cited() { 1 }\n";
        let old_manifest = manifest("old", vec![put_file(&storage.0, "src/cited.rs", text)]);
        let old_index = index(
            "old",
            &old_manifest,
            vec![symbol("src/cited.rs", "sym-cited", "cited", text, false)],
            Vec::new(),
            &HashSet::new(),
        );
        let revision = revision(vec![anchored_section(
            "cited",
            "src/cited.rs",
            text,
            "sym-cited",
            "1",
        )]);
        let complete_empty = manifest("new", Vec::new());
        let stale = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &complete_empty,
            None,
            &revision,
        )
        .unwrap();
        assert_eq!(stale.sections[0].status, SectionFreshnessStatus::Stale);
        assert!(
            stale.sections[0]
                .reasons
                .iter()
                .any(|reason| reason.code == "sourceDeleted")
        );

        let mut partial_empty = manifest("partial", Vec::new());
        partial_empty.summary.status = CaptureStatus::Partial;
        partial_empty.skipped.push(SkippedPath {
            path: "src".to_owned(),
            kind: InventoryPathKind::Directory,
            size_bytes: None,
            reason: "permissionDenied".to_owned(),
            details: "not captured".to_owned(),
        });
        let unknown = classify_chapter_freshness(
            &storage.0,
            &old_manifest,
            Some(&old_index),
            &partial_empty,
            None,
            &revision,
        )
        .unwrap();
        assert_eq!(
            unknown.sections[0].status,
            SectionFreshnessStatus::NeedsReview
        );
        assert!(
            unknown.sections[0]
                .reasons
                .iter()
                .any(|reason| reason.code == "sourceNotCaptured")
        );
    }
}
