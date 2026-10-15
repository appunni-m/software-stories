//! Bounded, read-only evidence for comparing two locally available Git commits.
//!
//! This module deliberately operates on immutable commit object IDs after ref
//! resolution. It does not checkout either tree, touch the index, or update refs.

use crate::bounded_process::{self, Limits, RunError};
use crate::project::git_command;
use crate::{
    index::{self, RelationshipConfidence, RelationshipKind, SnapshotIndex, SnapshotIndexStatus},
    snapshot::{
        CaptureLimits, CaptureStatus, CapturedContentKind, CapturedFile, GitChangeState,
        InventoryPathKind, SnapshotManifest, SnapshotSource, SnapshotSummary,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Output;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use tree_sitter::{Language, ParseOptions, Parser, Point};

const MAX_CHANGED_FILES: usize = 128;
const MAX_RAW_DIFF_BYTES: usize = 1024 * 1024;
const MAX_FILE_BYTES: u64 = 512 * 1024;
const MAX_FILE_PATCH_BYTES: usize = 128 * 1024;
const MAX_TOTAL_PATCH_BYTES: usize = 2 * 1024 * 1024;
const MAX_HUNKS_PER_FILE: usize = 128;
const MAX_HUNK_TEXT_BYTES: usize = 8 * 1024;
const MAX_AST_NODES_PER_FILE: usize = 250_000;
const MAX_SYMBOLS_PER_FILE: usize = 5_000;
const MAX_PARSE_TIME: Duration = Duration::from_millis(1_500);
const MAX_COMPARISON_TIME: Duration = Duration::from_secs(30);
const MAX_RELATIONSHIP_SCAN_TIME: Duration = Duration::from_secs(8);
const MAX_RELATIONSHIP_TREE_BYTES: usize = 8 * 1024 * 1024;
const MAX_RELATIONSHIP_TREE_FILES: usize = 10_000;
const MAX_RELATIONSHIP_EVIDENCE_PER_FILE_SIDE: usize = 16;
const MAX_GIT_STDERR_BYTES: usize = 16 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GitComparisonStatus {
    Ready,
    ChooseAgain,
    Limited,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RefSide {
    Base,
    Head,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RefProblemKind {
    Invalid,
    Missing,
    Ambiguous,
    NotCommit,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RefProblem {
    pub side: RefSide,
    pub requested: String,
    pub kind: RefProblemKind,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResolvedGitRef {
    pub requested: String,
    pub object_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitComparisonResult {
    pub status: GitComparisonStatus,
    pub comparison_id: Option<String>,
    pub base: Option<ResolvedGitRef>,
    pub head: Option<ResolvedGitRef>,
    pub ref_problem: Option<RefProblem>,
    pub changes: Vec<GitFileChange>,
    pub relationship_impact: GitRelationshipImpact,
    pub omitted_change_count: usize,
    pub omitted_reason: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GitRelationshipImpactStatus {
    NotNeeded,
    Complete,
    Partial,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitRelationshipImpact {
    pub status: GitRelationshipImpactStatus,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum GitChangeKind {
    Added,
    Deleted,
    Modified,
    Renamed,
    Copied,
    TypeChanged,
    Other,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub enum EvidenceClassification {
    Text,
    Binary,
    Oversized,
    Unsupported,
    ParserUncertain,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitFileChange {
    pub kind: GitChangeKind,
    pub base_path: Option<String>,
    pub head_path: Option<String>,
    pub classification: EvidenceClassification,
    pub base_blob_id: Option<String>,
    pub head_blob_id: Option<String>,
    pub base_size_bytes: Option<u64>,
    pub head_size_bytes: Option<u64>,
    pub hunks: Vec<GitDiffHunk>,
    #[serde(default)]
    pub base_relationships: Vec<GitRelationshipEvidence>,
    #[serde(default)]
    pub head_relationships: Vec<GitRelationshipEvidence>,
    pub skipped_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitRelationshipEvidence {
    pub kind: String,
    pub confidence: String,
    pub source_path: String,
    pub source_name: Option<String>,
    pub target_path: String,
    pub target_name: Option<String>,
    pub evidence_anchor: GitSourceAnchor,
    pub target_anchor: Option<GitSourceAnchor>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitDiffHunk {
    pub base_anchor: Option<GitSourceAnchor>,
    pub head_anchor: Option<GitSourceAnchor>,
    pub base_changed_ranges: Vec<GitChangedRange>,
    pub head_changed_ranges: Vec<GitChangedRange>,
    pub base_symbols: Vec<GitSymbolEvidence>,
    pub head_symbols: Vec<GitSymbolEvidence>,
    /// Bounded unified context for this hunk. It is evidence text from Git,
    /// not a claim that the changed code has any particular runtime effect.
    pub patch_excerpt: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitChangedRange {
    pub start_line: u64,
    pub end_line: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitSymbolEvidence {
    pub kind: String,
    pub name: String,
    pub anchor: GitSourceAnchor,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitSourceAnchor {
    pub revision_id: String,
    pub path: String,
    pub blob_id: String,
    pub start_line: u64,
    pub end_line: u64,
}

#[derive(Debug)]
enum GitEvidenceError {
    Git(String),
    OutputLimit(String),
    TimedOut(String),
    InvalidOutput(String),
}

#[derive(Clone, Debug)]
struct RawGitChange {
    kind: GitChangeKind,
    base_path_bytes: Option<Vec<u8>>,
    head_path_bytes: Option<Vec<u8>>,
    base_mode: String,
    head_mode: String,
    base_blob_id: Option<String>,
    head_blob_id: Option<String>,
}

#[derive(Clone, Debug)]
struct TextBlob {
    bytes: Vec<u8>,
}

#[derive(Clone, Debug)]
struct ParsedGitSymbol {
    kind: &'static str,
    name: String,
    start_line: u64,
    end_line: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParseLanguage {
    Rust,
    TypeScript,
    Tsx,
    JavaScript,
}

impl ParseLanguage {
    fn for_path(path: &str) -> Option<Self> {
        match Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("rs") => Some(Self::Rust),
            Some("ts") | Some("mts") | Some("cts") => Some(Self::TypeScript),
            Some("tsx") => Some(Self::Tsx),
            Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => Some(Self::JavaScript),
            _ => None,
        }
    }

    fn grammar(self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        }
    }
}

impl GitComparisonResult {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: GitComparisonStatus::Error,
            comparison_id: None,
            base: None,
            head: None,
            ref_problem: None,
            changes: Vec::new(),
            relationship_impact: GitRelationshipImpact {
                status: GitRelationshipImpactStatus::NotNeeded,
                message: "No resolved comparison is available for relationship analysis."
                    .to_owned(),
            },
            omitted_change_count: 0,
            omitted_reason: None,
            message: Some(message.into()),
        }
    }
}

/// Resolve two user-selected local refs and compare the resulting immutable commits.
///
/// Short names are accepted only when they match exactly one local branch, tag,
/// or remote-tracking ref. Fully qualified local refs and full object IDs are
/// also accepted. Revision expressions, abbreviations, and revision ranges are
/// rejected before Git receives them.
pub fn compare_local_revisions(
    repository: &Path,
    base_ref: &str,
    head_ref: &str,
) -> GitComparisonResult {
    let repository = match fs::canonicalize(repository) {
        Ok(path) if path.is_dir() => path,
        _ => return GitComparisonResult::error("The selected repository folder is unavailable."),
    };
    let deadline = Instant::now() + MAX_COMPARISON_TIME;
    let object_format = match git_text(
        &repository,
        &["rev-parse", "--show-object-format"],
        deadline,
    ) {
        Ok(format) if format == "sha1" || format == "sha256" => format,
        Ok(_) => {
            return GitComparisonResult::error(
                "This repository uses an unsupported Git object format.",
            );
        }
        Err(error) => return GitComparisonResult::error(error.message()),
    };

    let base = match resolve_ref(&repository, base_ref, &object_format, deadline) {
        Ok(revision) => revision,
        Err(problem) => return choose_again(RefSide::Base, base_ref, problem),
    };
    let head = match resolve_ref(&repository, head_ref, &object_format, deadline) {
        Ok(revision) => revision,
        Err(problem) => {
            let mut result = choose_again(RefSide::Head, head_ref, problem);
            result.base = Some(base);
            return result;
        }
    };
    let comparison_id = comparison_id(
        &base.requested,
        &base.object_id,
        &head.requested,
        &head.object_id,
    );
    if base.object_id == head.object_id {
        return ready_result(comparison_id, base, head, Vec::new());
    }

    let raw_output = match git_output(
        &repository,
        &[
            "diff-tree",
            "--no-commit-id",
            "-r",
            "--raw",
            "-z",
            "--no-abbrev",
            "--find-renames=50%",
            "--no-ext-diff",
            "--no-textconv",
            &base.object_id,
            &head.object_id,
            "--",
        ],
        MAX_RAW_DIFF_BYTES,
        deadline,
    ) {
        Ok(output) if output.status.success() => output.stdout,
        Ok(output) => {
            return GitComparisonResult::error(git_failure_message(&output));
        }
        Err(GitEvidenceError::OutputLimit(message) | GitEvidenceError::TimedOut(message)) => {
            return limited_result(comparison_id, base, head, message);
        }
        Err(error) => return GitComparisonResult::error(error.message()),
    };
    let mut raw_changes = match parse_raw_diff(&raw_output) {
        Ok(changes) => changes,
        Err(message) => return GitComparisonResult::error(message),
    };
    raw_changes.sort_by(|left, right| {
        display_path(left.base_path_bytes.as_deref())
            .cmp(&display_path(right.base_path_bytes.as_deref()))
            .then_with(|| {
                display_path(left.head_path_bytes.as_deref())
                    .cmp(&display_path(right.head_path_bytes.as_deref()))
            })
            .then_with(|| change_kind_order(&left.kind).cmp(&change_kind_order(&right.kind)))
    });

    let omitted_change_count = raw_changes.len().saturating_sub(MAX_CHANGED_FILES);
    raw_changes.truncate(MAX_CHANGED_FILES);
    let mut changes = Vec::with_capacity(raw_changes.len());
    let mut total_patch_bytes = 0_usize;
    let mut limit_reason = (omitted_change_count > 0).then(|| {
        format!(
            "The comparison contains more than {MAX_CHANGED_FILES} changed files; the remaining files were omitted."
        )
    });

    for raw in raw_changes {
        if Instant::now() >= deadline {
            limit_reason = Some(
                "The comparison reached its 30-second time limit; remaining evidence was omitted."
                    .to_owned(),
            );
            break;
        }
        match build_file_change(
            &repository,
            &base.object_id,
            &head.object_id,
            raw,
            &mut total_patch_bytes,
            deadline,
        ) {
            Ok(change) => changes.push(change),
            Err(GitEvidenceError::OutputLimit(message) | GitEvidenceError::TimedOut(message)) => {
                limit_reason = Some(message);
                break;
            }
            Err(error) => return GitComparisonResult::error(error.message()),
        }
    }

    let relationship_impact = if changes.is_empty() {
        GitRelationshipImpact {
            status: GitRelationshipImpactStatus::NotNeeded,
            message: "No changed files need relationship analysis.".to_owned(),
        }
    } else {
        add_cross_file_relationships(
            &repository,
            &base.object_id,
            &head.object_id,
            &mut changes,
            deadline,
        )
    };

    GitComparisonResult {
        status: if limit_reason.is_some() {
            GitComparisonStatus::Limited
        } else {
            GitComparisonStatus::Ready
        },
        comparison_id: Some(comparison_id),
        base: Some(base),
        head: Some(head),
        ref_problem: None,
        changes,
        relationship_impact,
        omitted_change_count,
        omitted_reason: limit_reason.clone(),
        message: limit_reason,
    }
}

fn add_cross_file_relationships(
    repository: &Path,
    base_revision_id: &str,
    head_revision_id: &str,
    changes: &mut [GitFileChange],
    comparison_deadline: Instant,
) -> GitRelationshipImpact {
    let now = Instant::now();
    if now >= comparison_deadline {
        return relationship_impact_unavailable(
            "The file diff used the comparison time budget before relationship indexing began.",
        );
    }
    let scan_deadline = (now + MAX_RELATIONSHIP_SCAN_TIME).min(comparison_deadline);
    let base_priority_paths = changes
        .iter()
        .filter_map(|change| change.base_path.clone())
        .collect::<BTreeSet<_>>();
    let head_priority_paths = changes
        .iter()
        .filter_map(|change| change.head_path.clone())
        .collect::<BTreeSet<_>>();
    let cancellation = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicBool::new(false));
    let timer_cancellation = Arc::clone(&cancellation);
    let timer_finished = Arc::clone(&finished);
    let timer = thread::spawn(move || {
        while Instant::now() < scan_deadline && !timer_finished.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(10));
        }
        if !timer_finished.load(Ordering::Relaxed) {
            timer_cancellation.store(true, Ordering::Relaxed);
        }
    });

    let base_index = build_revision_index(
        repository,
        base_revision_id,
        &base_priority_paths,
        scan_deadline,
        &cancellation,
    );
    let head_index = if Instant::now() < scan_deadline && !cancellation.load(Ordering::Relaxed) {
        build_revision_index(
            repository,
            head_revision_id,
            &head_priority_paths,
            scan_deadline,
            &cancellation,
        )
    } else {
        Err(GitEvidenceError::TimedOut(
            "The relationship index reached its bounded scan time; head-side relationships are unavailable.".to_owned(),
        ))
    };
    finished.store(true, Ordering::Relaxed);
    let _ = timer.join();

    let mut relationship_evidence_limited = false;
    for change in changes {
        if let (Some(path), Ok(index)) = (change.base_path.as_deref(), base_index.as_ref()) {
            let (relationships, limited) = relationships_for_change(
                &index.index,
                &index.blob_ids_by_path,
                base_revision_id,
                path,
                &change
                    .hunks
                    .iter()
                    .flat_map(|hunk| hunk.base_symbols.iter())
                    .cloned()
                    .collect::<Vec<_>>(),
            );
            change.base_relationships = relationships;
            relationship_evidence_limited |= limited;
        }
        if let (Some(path), Ok(index)) = (change.head_path.as_deref(), head_index.as_ref()) {
            let (relationships, limited) = relationships_for_change(
                &index.index,
                &index.blob_ids_by_path,
                head_revision_id,
                path,
                &change
                    .hunks
                    .iter()
                    .flat_map(|hunk| hunk.head_symbols.iter())
                    .cloned()
                    .collect::<Vec<_>>(),
            );
            change.head_relationships = relationships;
            relationship_evidence_limited |= limited;
        }
    }

    let base_partial = base_index
        .as_ref()
        .map(|index| index.partial)
        .unwrap_or(true);
    let head_partial = head_index
        .as_ref()
        .map(|index| index.partial)
        .unwrap_or(true);
    match (&base_index, &head_index) {
        (Ok(_), Ok(_)) if !base_partial && !head_partial && !relationship_evidence_limited => GitRelationshipImpact {
            status: GitRelationshipImpactStatus::Complete,
            message: "High-confidence static imports, calls, and test references are attached when they target a changed path or declaration. Analysis covers the shipped Rust, TypeScript, TSX, and JavaScript parsers; dynamic or unresolved edges are omitted, and no runtime impact is inferred.".to_owned(),
        },
        (Ok(_), Ok(_)) if relationship_evidence_limited => GitRelationshipImpact {
            status: GitRelationshipImpactStatus::Partial,
            message: format!("Resolved relationships are sorted deterministically and capped at {MAX_RELATIONSHIP_EVIDENCE_PER_FILE_SIDE} records for each changed file and revision. Additional matches may exist. A parser, tree, or source-byte limit may also have left source unindexed. Dynamic or unresolved edges are omitted, and no runtime impact is inferred."),
        },
        (Ok(_), Ok(_)) => GitRelationshipImpact {
            status: GitRelationshipImpactStatus::Partial,
            message: "Resolved relationships are shown only for indexed source. A parser, tree, or source-byte limit left part of at least one revision unindexed; omitted callers may exist. Dynamic or unresolved edges are omitted, and no runtime impact is inferred.".to_owned(),
        },
        (Ok(_), Err(error)) => GitRelationshipImpact {
            status: GitRelationshipImpactStatus::Partial,
            message: format!("Base-side relationships were indexed, but head-side relationships are unavailable: {}. Results are incomplete; no runtime impact is inferred.", error.message()),
        },
        (Err(error), Ok(_)) => GitRelationshipImpact {
            status: GitRelationshipImpactStatus::Partial,
            message: format!("Head-side relationships were indexed, but base-side relationships are unavailable: {}. Results are incomplete; no runtime impact is inferred.", error.message()),
        },
        (Err(base_error), Err(head_error)) => relationship_impact_unavailable(&format!(
            "Base relationship index unavailable: {} Head relationship index unavailable: {}",
            base_error.message(),
            head_error.message()
        )),
    }
}

fn relationship_impact_unavailable(message: &str) -> GitRelationshipImpact {
    GitRelationshipImpact {
        status: GitRelationshipImpactStatus::Unavailable,
        message: message.to_owned(),
    }
}

fn build_revision_index(
    repository: &Path,
    revision_id: &str,
    priority_paths: &BTreeSet<String>,
    deadline: Instant,
    cancellation: &AtomicBool,
) -> Result<RevisionIndex, GitEvidenceError> {
    let output = git_output(
        repository,
        &["ls-tree", "-r", "-l", "-z", "--full-tree", revision_id],
        MAX_RELATIONSHIP_TREE_BYTES,
        deadline,
    )?;
    if !output.status.success() {
        return Err(GitEvidenceError::Git(git_failure_message(&output)));
    }
    let (tree_entries, tree_partial) =
        parse_tree_entries(&output.stdout, priority_paths, revision_id.len())?;
    let mut index_order = tree_entries.iter().collect::<Vec<_>>();
    index_order.sort_by(|left, right| {
        priority_paths
            .contains(&right.path)
            .cmp(&priority_paths.contains(&left.path))
            .then_with(|| left.path.cmp(&right.path))
    });

    let mut sources_to_read = Vec::new();
    let mut selected_source_bytes = 0_u64;
    for (file_index, entry) in index_order
        .iter()
        .take(index::MAX_INDEXED_FILES)
        .enumerate()
    {
        let _ = file_index;
        if ParseLanguage::for_path(&entry.path).is_none()
            || entry.size_bytes > index::MAX_SOURCE_BYTES_PER_FILE
        {
            continue;
        }
        if selected_source_bytes.saturating_add(entry.size_bytes) > index::MAX_TOTAL_SOURCE_BYTES {
            break;
        }
        selected_source_bytes = selected_source_bytes.saturating_add(entry.size_bytes);
        sources_to_read.push((*entry).clone());
    }
    let source_bytes = read_tree_blob_batch(repository, &sources_to_read, deadline)?;
    let mut blob_ids_by_path = HashMap::with_capacity(tree_entries.len());
    let mut snapshot_sources = HashMap::with_capacity(source_bytes.len());
    let mut source_kinds = HashMap::with_capacity(source_bytes.len());
    for entry in &tree_entries {
        blob_ids_by_path.insert(entry.path.clone(), entry.object_id.clone());
    }
    for (entry, bytes) in sources_to_read.iter().zip(source_bytes) {
        let hash = hex_digest(&Sha256::digest(&bytes));
        let content_kind = if bytes.contains(&0) {
            CapturedContentKind::Binary
        } else if std::str::from_utf8(&bytes).is_ok() {
            CapturedContentKind::Text
        } else {
            CapturedContentKind::InvalidEncoding
        };
        source_kinds.insert(entry.path.clone(), content_kind.clone());
        if content_kind == CapturedContentKind::Text {
            let contents = String::from_utf8(bytes).map_err(|_| {
                GitEvidenceError::InvalidOutput(
                    "A validated Git text blob could not be decoded as UTF-8.".to_owned(),
                )
            })?;
            snapshot_sources.insert(
                entry.path.clone(),
                SnapshotSource {
                    snapshot_id: revision_id.to_owned(),
                    path: entry.path.clone(),
                    sha256: hash.clone(),
                    size_bytes: entry.size_bytes,
                    contents,
                },
            );
        }
    }

    let files = tree_entries
        .iter()
        .map(|entry| CapturedFile {
            path: entry.path.clone(),
            kind: InventoryPathKind::File,
            size_bytes: entry.size_bytes,
            sha256: snapshot_sources
                .get(&entry.path)
                .map(|source| source.sha256.clone())
                .unwrap_or_else(|| "0".repeat(64)),
            blob_path: String::new(),
            content_kind: source_kinds
                .get(&entry.path)
                .cloned()
                .unwrap_or(CapturedContentKind::Unknown),
            secret_like: false,
        })
        .collect::<Vec<_>>();
    let total_bytes = files
        .iter()
        .fold(0_u64, |total, file| total.saturating_add(file.size_bytes));
    let summary = SnapshotSummary {
        snapshot_id: revision_id.to_owned(),
        project_id: "git-revision".to_owned(),
        parent_snapshot_id: None,
        created_at_unix_ms: 0,
        branch: None,
        head: Some(revision_id.to_owned()),
        index_state: GitChangeState::Clean,
        worktree_state: GitChangeState::Clean,
        status: CaptureStatus::Complete,
        files_captured: files.len(),
        paths_skipped: 0,
        total_bytes,
        paths_visited: files.len() as u64,
        policy_version: "git-revision-index-v1".to_owned(),
    };
    let manifest = SnapshotManifest {
        schema_version: 2,
        summary,
        repository_root: repository.display().to_string(),
        selected_path: ".".to_owned(),
        capture_limits: CaptureLimits::default(),
        directories: Vec::new(),
        files,
        skipped: Vec::new(),
    };
    let priority_paths = priority_paths
        .iter()
        .filter(|path| blob_ids_by_path.contains_key(*path))
        .cloned()
        .collect::<BTreeSet<_>>();
    let index = index::build_snapshot_index_from_sources(
        &manifest,
        &snapshot_sources,
        &priority_paths,
        revision_id,
        cancellation,
        |_| {},
    );
    let parser_limited = index.status == SnapshotIndexStatus::Cancelled
        || index.files.iter().any(|file| {
            file.language.is_some() && !matches!(file.status, index::IndexFileStatus::Indexed)
        });
    Ok(RevisionIndex {
        index,
        blob_ids_by_path,
        partial: tree_partial || parser_limited || cancellation.load(Ordering::Relaxed),
    })
}

fn parse_tree_entries(
    output: &[u8],
    priority_paths: &BTreeSet<String>,
    expected_object_id_len: usize,
) -> Result<(Vec<GitTreeEntry>, bool), GitEvidenceError> {
    let mut entries = Vec::new();
    let mut partial = false;
    let mut regular_file_count = 0_usize;
    for record in output
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let Some(separator) = record.iter().position(|byte| *byte == b'\t') else {
            return Err(GitEvidenceError::InvalidOutput(
                "Git returned a malformed revision tree entry.".to_owned(),
            ));
        };
        let metadata = &record[..separator];
        let path_bytes = &record[separator + 1..];
        let fields = metadata
            .split(|byte| byte.is_ascii_whitespace())
            .filter(|field| !field.is_empty())
            .collect::<Vec<_>>();
        if fields.len() != 4 {
            return Err(GitEvidenceError::InvalidOutput(
                "Git returned a revision tree entry without a bounded blob size.".to_owned(),
            ));
        }
        let mode = std::str::from_utf8(fields[0]).map_err(|_| {
            GitEvidenceError::InvalidOutput("Git returned an invalid tree mode.".to_owned())
        })?;
        let object_type = fields[1];
        if object_type != b"blob" || !matches!(mode, "100644" | "100755") {
            partial = true;
            continue;
        }
        let object_id = std::str::from_utf8(fields[2]).map_err(|_| {
            GitEvidenceError::InvalidOutput("Git returned an invalid blob object ID.".to_owned())
        })?;
        if object_id.len() != expected_object_id_len
            || !object_id.bytes().all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(GitEvidenceError::InvalidOutput(
                "Git returned a malformed revision blob object ID.".to_owned(),
            ));
        }
        let size_bytes = std::str::from_utf8(fields[3])
            .ok()
            .and_then(|size| size.parse::<u64>().ok())
            .ok_or_else(|| {
                GitEvidenceError::InvalidOutput("Git returned an invalid blob size.".to_owned())
            })?;
        let path = match std::str::from_utf8(path_bytes) {
            Ok(path) if !path.is_empty() && !path.starts_with('/') => path.to_owned(),
            _ => {
                partial = true;
                continue;
            }
        };
        regular_file_count = regular_file_count.saturating_add(1);
        if regular_file_count > MAX_RELATIONSHIP_TREE_FILES && !priority_paths.contains(&path) {
            partial = true;
            continue;
        }
        entries.push(GitTreeEntry {
            object_id: object_id.to_owned(),
            path,
            size_bytes,
        });
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    Ok((entries, partial))
}

fn read_tree_blob_batch(
    repository: &Path,
    entries: &[GitTreeEntry],
    deadline: Instant,
) -> Result<Vec<Vec<u8>>, GitEvidenceError> {
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    let mut input = String::new();
    for entry in entries {
        input.push_str(&entry.object_id);
        input.push('\n');
    }
    let max_output_bytes = index::MAX_TOTAL_SOURCE_BYTES as usize + 512 * 1024;
    let output = git_output_with_input(
        repository,
        &["cat-file", "--batch"],
        max_output_bytes,
        deadline,
        input.as_bytes(),
    )?;
    if !output.status.success() {
        return Err(GitEvidenceError::Git(git_failure_message(&output)));
    }
    let mut result = Vec::with_capacity(entries.len());
    let mut cursor = 0_usize;
    for entry in entries {
        let header_end = output.stdout[cursor..]
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|offset| cursor + offset)
            .ok_or_else(|| {
                GitEvidenceError::InvalidOutput(
                    "Git returned an incomplete cat-file batch header.".to_owned(),
                )
            })?;
        let header = &output.stdout[cursor..header_end];
        let fields = header.split(|byte| *byte == b' ').collect::<Vec<_>>();
        if fields.len() != 3 || fields[0] != entry.object_id.as_bytes() || fields[1] != b"blob" {
            return Err(GitEvidenceError::InvalidOutput(
                "Git returned a mismatched cat-file batch object.".to_owned(),
            ));
        }
        let size = std::str::from_utf8(fields[2])
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .ok_or_else(|| {
                GitEvidenceError::InvalidOutput(
                    "Git returned an invalid cat-file batch size.".to_owned(),
                )
            })?;
        if size as u64 != entry.size_bytes {
            return Err(GitEvidenceError::InvalidOutput(
                "The revision tree blob size did not match its cat-file data.".to_owned(),
            ));
        }
        let data_start = header_end.saturating_add(1);
        let data_end = data_start.saturating_add(size);
        if data_end >= output.stdout.len() || output.stdout[data_end] != b'\n' {
            return Err(GitEvidenceError::InvalidOutput(
                "Git returned truncated cat-file batch data.".to_owned(),
            ));
        }
        result.push(output.stdout[data_start..data_end].to_vec());
        cursor = data_end.saturating_add(1);
    }
    if cursor != output.stdout.len() {
        return Err(GitEvidenceError::InvalidOutput(
            "Git returned trailing cat-file batch data.".to_owned(),
        ));
    }
    Ok(result)
}

fn relationships_for_change(
    index: &SnapshotIndex,
    blob_ids_by_path: &HashMap<String, String>,
    revision_id: &str,
    changed_path: &str,
    changed_symbols: &[GitSymbolEvidence],
) -> (Vec<GitRelationshipEvidence>, bool) {
    let changed_symbol_ids = index
        .symbols
        .iter()
        .filter(|symbol| symbol.path == changed_path)
        .filter(|symbol| {
            changed_symbols.iter().any(|evidence| {
                evidence.name == symbol.name
                    && evidence.anchor.start_line == symbol.anchor.start_line as u64
                    && evidence.anchor.end_line == symbol.anchor.end_line as u64
            })
        })
        .map(|symbol| symbol.symbol_id.clone())
        .collect::<HashSet<_>>();
    let fully_indexed_paths = index
        .files
        .iter()
        .filter(|file| file.status == index::IndexFileStatus::Indexed)
        .map(|file| file.path.as_str())
        .collect::<HashSet<_>>();
    let symbols_by_id = index
        .symbols
        .iter()
        .map(|symbol| (symbol.symbol_id.as_str(), symbol))
        .collect::<HashMap<_, _>>();
    let mut output = Vec::new();
    for relationship in &index.relationships {
        if relationship.confidence != RelationshipConfidence::High
            || relationship.unresolved_reason.is_some()
            || relationship.evidence.path == changed_path
            || !fully_indexed_paths.contains(relationship.evidence.path.as_str())
        {
            continue;
        }
        let target_symbol = relationship
            .target_id
            .as_deref()
            .and_then(|symbol_id| symbols_by_id.get(symbol_id).copied());
        let target_path = relationship
            .target_path
            .as_deref()
            .or_else(|| target_symbol.map(|symbol| symbol.path.as_str()));
        let Some(target_path) = target_path else {
            continue;
        };
        let targets_changed_file =
            target_path == changed_path && relationship.kind == RelationshipKind::Imports;
        let targets_changed_symbol = target_path == changed_path
            && matches!(
                relationship.kind,
                RelationshipKind::Calls | RelationshipKind::TestReference
            )
            && relationship
                .target_id
                .as_ref()
                .is_some_and(|target_id| changed_symbol_ids.contains(target_id));
        if !targets_changed_file && !targets_changed_symbol {
            continue;
        }
        let (Ok(evidence_start_line), Ok(evidence_end_line)) = (
            u64::try_from(relationship.evidence.start_line),
            u64::try_from(relationship.evidence.end_line),
        ) else {
            continue;
        };
        let Some(evidence_anchor) = git_anchor_from_source(
            revision_id,
            &relationship.evidence.path,
            evidence_start_line,
            evidence_end_line,
            blob_ids_by_path,
        ) else {
            continue;
        };
        let target_anchor = target_symbol.and_then(|symbol| {
            git_anchor_from_source(
                revision_id,
                &symbol.path,
                u64::try_from(symbol.anchor.start_line).ok()?,
                u64::try_from(symbol.anchor.end_line).ok()?,
                blob_ids_by_path,
            )
        });
        output.push(GitRelationshipEvidence {
            kind: relationship_kind_name(&relationship.kind).to_owned(),
            confidence: "high".to_owned(),
            source_path: relationship.evidence.path.clone(),
            source_name: symbols_by_id
                .get(relationship.source_id.as_str())
                .map(|symbol| symbol.name.clone()),
            target_path: target_path.to_owned(),
            target_name: target_symbol
                .map(|symbol| symbol.name.clone())
                .or_else(|| relationship.target_label.clone()),
            evidence_anchor,
            target_anchor,
        });
    }
    output.sort_by(|left, right| {
        left.source_path
            .cmp(&right.source_path)
            .then(
                left.evidence_anchor
                    .start_line
                    .cmp(&right.evidence_anchor.start_line),
            )
            .then(left.kind.cmp(&right.kind))
            .then(left.target_path.cmp(&right.target_path))
            .then(left.target_name.cmp(&right.target_name))
    });
    output.dedup();
    let limited = output.len() > MAX_RELATIONSHIP_EVIDENCE_PER_FILE_SIDE;
    output.truncate(MAX_RELATIONSHIP_EVIDENCE_PER_FILE_SIDE);
    (output, limited)
}

fn relationship_kind_name(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Defines => "defines",
        RelationshipKind::Exports => "exports",
        RelationshipKind::Imports => "imports",
        RelationshipKind::Calls => "calls",
        RelationshipKind::TestReference => "testReference",
    }
}

fn git_anchor_from_source(
    revision_id: &str,
    path: &str,
    start_line: u64,
    end_line: u64,
    blob_ids_by_path: &HashMap<String, String>,
) -> Option<GitSourceAnchor> {
    Some(GitSourceAnchor {
        revision_id: revision_id.to_owned(),
        path: path.to_owned(),
        blob_id: blob_ids_by_path.get(path)?.clone(),
        start_line,
        end_line,
    })
}

#[derive(Debug)]
struct RefResolutionError {
    kind: RefProblemKind,
    message: String,
}

#[derive(Clone, Debug)]
struct GitTreeEntry {
    object_id: String,
    path: String,
    size_bytes: u64,
}

struct RevisionIndex {
    index: SnapshotIndex,
    blob_ids_by_path: HashMap<String, String>,
    partial: bool,
}

fn resolve_ref(
    repository: &Path,
    requested: &str,
    object_format: &str,
    deadline: Instant,
) -> Result<ResolvedGitRef, RefResolutionError> {
    if requested == "HEAD" {
        return resolve_commit_expression(repository, requested, "HEAD", deadline);
    }
    let expected_hex_len = if object_format == "sha1" { 40 } else { 64 };
    if requested.len() == expected_hex_len && requested.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        let expression = format!("{}^{{commit}}", requested.to_ascii_lowercase());
        return resolve_commit_expression(repository, requested, &expression, deadline);
    }

    let candidates = match ref_candidates(requested, deadline) {
        Ok(candidates) => candidates,
        Err(message) => {
            return Err(RefResolutionError {
                kind: RefProblemKind::Invalid,
                message,
            });
        }
    };
    let mut found = Vec::new();
    for candidate in candidates {
        match git_output(
            repository,
            &["show-ref", "--verify", "--quiet", &candidate],
            1024,
            deadline,
        ) {
            Ok(output) if output.status.success() => found.push(candidate),
            Ok(output) if output.status.code() == Some(1) => {}
            Ok(output) => {
                return Err(RefResolutionError {
                    kind: RefProblemKind::Invalid,
                    message: git_failure_message(&output),
                });
            }
            Err(error) => {
                return Err(RefResolutionError {
                    kind: RefProblemKind::Invalid,
                    message: error.message(),
                });
            }
        }
    }
    match found.as_slice() {
        [] => Err(RefResolutionError {
            kind: RefProblemKind::Missing,
            message: "No matching local branch, tag, remote-tracking ref, or full commit ID was found. Choose another ref.".to_owned(),
        }),
        [reference] => {
            let expression = format!("{reference}^{{commit}}");
            resolve_commit_expression(repository, requested, &expression, deadline)
        }
        _ => Err(RefResolutionError {
            kind: RefProblemKind::Ambiguous,
            message: format!(
                "The short ref matches multiple local refs: {}. Choose a fully qualified ref.",
                found.join(", ")
            ),
        }),
    }
}

fn ref_candidates(requested: &str, deadline: Instant) -> Result<Vec<String>, String> {
    if requested.is_empty()
        || requested.len() > 1_024
        || requested.len() > 1024
        || requested.starts_with('-')
        || requested
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
        || requested.contains("..")
        || requested.contains("@{")
        || requested.contains('~')
        || requested.contains('^')
        || requested.contains(':')
        || requested.contains('?')
        || requested.contains('*')
        || requested.contains('[')
        || requested.contains('\\')
    {
        return Err(
            "Enter a ref name or full commit ID, without revision operators or ranges.".to_owned(),
        );
    }
    if requested.starts_with("refs/") {
        if !(requested.starts_with("refs/heads/")
            || requested.starts_with("refs/tags/")
            || requested.starts_with("refs/remotes/"))
            || !valid_git_ref(requested, deadline)
        {
            return Err(
                "Only fully qualified local branch, tag, and remote-tracking refs are accepted."
                    .to_owned(),
            );
        }
        return Ok(vec![requested.to_owned()]);
    }
    if requested.starts_with('/') || requested.ends_with('/') || requested.ends_with('.') {
        return Err("The ref name is not a valid local Git ref.".to_owned());
    }
    let candidates = [
        format!("refs/heads/{requested}"),
        format!("refs/tags/{requested}"),
        format!("refs/remotes/{requested}"),
    ];
    if candidates
        .iter()
        .any(|candidate| !valid_git_ref(candidate, deadline))
    {
        return Err("The ref name is not a valid local Git ref.".to_owned());
    }
    Ok(candidates.into_iter().collect())
}

fn valid_git_ref(reference: &str, deadline: Instant) -> bool {
    if reference.contains("//") || reference.ends_with(".lock") || reference.ends_with(".") {
        return false;
    }
    let mut command = git_command();
    command.args(["check-ref-format", reference]);
    bounded_process::run(
        &mut command,
        None,
        Limits {
            command_timeout: deadline.saturating_duration_since(Instant::now()),
            operation_deadline: deadline,
            stdout_bytes: 0,
            stderr_bytes: 0,
            stdin_bytes: 0,
        },
        None,
    )
    .is_ok_and(|output| output.status.success())
}

fn resolve_commit_expression(
    repository: &Path,
    requested: &str,
    expression: &str,
    deadline: Instant,
) -> Result<ResolvedGitRef, RefResolutionError> {
    let output = match git_output(
        repository,
        &["rev-parse", "--verify", "--end-of-options", expression],
        1024,
        deadline,
    ) {
        Ok(output) => output,
        Err(error) => {
            return Err(RefResolutionError {
                kind: RefProblemKind::Invalid,
                message: error.message(),
            });
        }
    };
    if !output.status.success() {
        let message = git_failure_message(&output);
        let kind = if message.to_ascii_lowercase().contains("commit")
            || message.to_ascii_lowercase().contains("object")
        {
            RefProblemKind::NotCommit
        } else {
            RefProblemKind::Missing
        };
        return Err(RefResolutionError {
            kind,
            message: "The selected ref does not resolve to a locally available commit. Choose another ref.".to_owned(),
        });
    }
    let object_id = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_ascii_lowercase();
    if object_id.is_empty() || !object_id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(RefResolutionError {
            kind: RefProblemKind::NotCommit,
            message: "Git did not resolve this ref to a commit object. Choose another ref."
                .to_owned(),
        });
    }
    Ok(ResolvedGitRef {
        requested: requested.to_owned(),
        object_id,
    })
}

fn choose_again(
    side: RefSide,
    requested: &str,
    problem: RefResolutionError,
) -> GitComparisonResult {
    GitComparisonResult {
        status: GitComparisonStatus::ChooseAgain,
        comparison_id: None,
        base: None,
        head: None,
        ref_problem: Some(RefProblem {
            side,
            requested: requested.to_owned(),
            kind: problem.kind,
            message: problem.message.clone(),
        }),
        changes: Vec::new(),
        relationship_impact: GitRelationshipImpact {
            status: GitRelationshipImpactStatus::NotNeeded,
            message: "Resolve both local refs before relationship analysis.".to_owned(),
        },
        omitted_change_count: 0,
        omitted_reason: None,
        message: Some(problem.message),
    }
}

fn ready_result(
    comparison_id: String,
    base: ResolvedGitRef,
    head: ResolvedGitRef,
    changes: Vec<GitFileChange>,
) -> GitComparisonResult {
    GitComparisonResult {
        status: GitComparisonStatus::Ready,
        comparison_id: Some(comparison_id),
        base: Some(base),
        head: Some(head),
        ref_problem: None,
        changes,
        relationship_impact: GitRelationshipImpact {
            status: GitRelationshipImpactStatus::NotNeeded,
            message: "No changed files need relationship analysis.".to_owned(),
        },
        omitted_change_count: 0,
        omitted_reason: None,
        message: None,
    }
}

fn limited_result(
    comparison_id: String,
    base: ResolvedGitRef,
    head: ResolvedGitRef,
    message: String,
) -> GitComparisonResult {
    GitComparisonResult {
        status: GitComparisonStatus::Limited,
        comparison_id: Some(comparison_id),
        base: Some(base),
        head: Some(head),
        ref_problem: None,
        changes: Vec::new(),
        relationship_impact: GitRelationshipImpact {
            status: GitRelationshipImpactStatus::Unavailable,
            message: "The bounded diff did not produce files to analyze for relationship impact."
                .to_owned(),
        },
        omitted_change_count: 0,
        omitted_reason: Some(message.clone()),
        message: Some(message),
    }
}

fn build_file_change(
    repository: &Path,
    base_revision_id: &str,
    head_revision_id: &str,
    raw: RawGitChange,
    total_patch_bytes: &mut usize,
    deadline: Instant,
) -> Result<GitFileChange, GitEvidenceError> {
    let base_path = raw
        .base_path_bytes
        .as_deref()
        .map(|path| display_path(Some(path)));
    let head_path = raw
        .head_path_bytes
        .as_deref()
        .map(|path| display_path(Some(path)));
    let base_size_bytes = blob_size(repository, raw.base_blob_id.as_deref(), deadline)?;
    let head_size_bytes = blob_size(repository, raw.head_blob_id.as_deref(), deadline)?;

    let mut change = GitFileChange {
        kind: raw.kind.clone(),
        base_path: base_path.clone(),
        head_path: head_path.clone(),
        classification: EvidenceClassification::Text,
        base_blob_id: raw.base_blob_id.clone(),
        head_blob_id: raw.head_blob_id.clone(),
        base_size_bytes,
        head_size_bytes,
        hunks: Vec::new(),
        base_relationships: Vec::new(),
        head_relationships: Vec::new(),
        skipped_reason: None,
    };
    let mut parser_symbols: Option<(Vec<ParsedGitSymbol>, Vec<ParsedGitSymbol>)> = None;

    let unsupported_mode = [&raw.base_mode, &raw.head_mode]
        .into_iter()
        .any(|mode| !mode.is_empty() && !matches!(mode.as_str(), "000000" | "100644" | "100755"));
    if unsupported_mode {
        change.classification = EvidenceClassification::Unsupported;
        change.skipped_reason = Some("The changed path uses a symlink, submodule, or non-regular Git mode; parser-backed source evidence was skipped.".to_owned());
        return Ok(change);
    }
    if base_size_bytes.is_some_and(|size| size > MAX_FILE_BYTES)
        || head_size_bytes.is_some_and(|size| size > MAX_FILE_BYTES)
    {
        change.classification = EvidenceClassification::Oversized;
        change.skipped_reason = Some(format!(
            "At least one source blob exceeds the {MAX_FILE_BYTES}-byte per-file evidence limit."
        ));
        return Ok(change);
    }

    let base_blob = read_text_blob(
        repository,
        raw.base_blob_id.as_deref(),
        base_size_bytes,
        deadline,
    )?;
    let head_blob = read_text_blob(
        repository,
        raw.head_blob_id.as_deref(),
        head_size_bytes,
        deadline,
    )?;
    if base_blob.as_ref().is_some_and(|blob| !is_text(&blob.bytes))
        || head_blob.as_ref().is_some_and(|blob| !is_text(&blob.bytes))
    {
        change.classification = EvidenceClassification::Binary;
        change.skipped_reason = Some(
            "A changed blob contains NUL bytes or invalid UTF-8; line anchors are unavailable."
                .to_owned(),
        );
        return Ok(change);
    }

    if raw
        .base_path_bytes
        .as_ref()
        .is_some_and(|path| std::str::from_utf8(path).is_err())
        || raw
            .head_path_bytes
            .as_ref()
            .is_some_and(|path| std::str::from_utf8(path).is_err())
    {
        change.classification = EvidenceClassification::Unsupported;
        change.skipped_reason = Some("The changed path is not valid UTF-8, so safe source-path anchors cannot be represented.".to_owned());
        return Ok(change);
    }

    let base_language = base_path.as_deref().and_then(ParseLanguage::for_path);
    let head_language = head_path.as_deref().and_then(ParseLanguage::for_path);
    let language = match (base_language, head_language) {
        (Some(language), None) if raw.head_path_bytes.is_none() => Some(language),
        (None, Some(language)) if raw.base_path_bytes.is_none() => Some(language),
        (Some(base), Some(head)) if base == head => Some(base),
        _ => None,
    };
    match language {
        None => {
            change.classification = EvidenceClassification::Unsupported;
            change.skipped_reason = Some("No bundled Tree-sitter grammar supports this file extension; the textual hunk can still be inspected.".to_owned());
        }
        Some(language) => {
            let (base_symbols, base_ok) = parsed_symbols_for_blob(base_blob.as_ref(), language);
            let (head_symbols, head_ok) = parsed_symbols_for_blob(head_blob.as_ref(), language);
            if !base_ok || !head_ok {
                change.classification = EvidenceClassification::ParserUncertain;
                change.skipped_reason = Some("Tree-sitter reported syntax errors or reached its bounded parse limit on at least one revision; no parser-backed symbols or relationships are attached.".to_owned());
            } else {
                parser_symbols = Some((base_symbols, head_symbols));
            }
        }
    }

    if *total_patch_bytes >= MAX_TOTAL_PATCH_BYTES {
        change.classification = EvidenceClassification::Oversized;
        change.skipped_reason =
            Some("The comparison reached its total unified-diff evidence budget.".to_owned());
        return Ok(change);
    }
    let patch_args = patch_args(base_revision_id, head_revision_id, &raw);
    let patch_args = patch_args.iter().map(String::as_str).collect::<Vec<_>>();
    let patch = match git_output(repository, &patch_args, MAX_FILE_PATCH_BYTES, deadline) {
        Ok(output) if output.status.success() => output.stdout,
        Ok(output) => return Err(GitEvidenceError::Git(git_failure_message(&output))),
        Err(GitEvidenceError::OutputLimit(_)) => {
            change.classification = EvidenceClassification::Oversized;
            change.skipped_reason = Some(format!(
                "The unified diff for this file exceeds the {MAX_FILE_PATCH_BYTES}-byte per-file evidence limit."
            ));
            return Ok(change);
        }
        Err(error) => return Err(error),
    };
    if total_patch_bytes.saturating_add(patch.len()) > MAX_TOTAL_PATCH_BYTES {
        change.classification = EvidenceClassification::Oversized;
        change.skipped_reason =
            Some("The unified diff would exceed the total comparison evidence budget.".to_owned());
        return Ok(change);
    }
    *total_patch_bytes += patch.len();
    let (mut hunks, hunk_limit_hit) = parse_patch_hunks(
        &patch,
        base_revision_id,
        head_revision_id,
        base_path.as_deref(),
        head_path.as_deref(),
        raw.base_blob_id.as_deref(),
        raw.head_blob_id.as_deref(),
    );
    if let Some((base_symbols, head_symbols)) = parser_symbols {
        for hunk in &mut hunks {
            hunk.base_symbols = symbols_for_hunk(
                &base_symbols,
                hunk.base_anchor.as_ref(),
                &hunk.base_changed_ranges,
            );
            hunk.head_symbols = symbols_for_hunk(
                &head_symbols,
                hunk.head_anchor.as_ref(),
                &hunk.head_changed_ranges,
            );
        }
    }
    change.hunks = hunks;
    if hunk_limit_hit {
        change.classification = EvidenceClassification::Oversized;
        change.skipped_reason = Some(format!(
            "The file contains more than {MAX_HUNKS_PER_FILE} diff hunks; remaining hunks were omitted."
        ));
    } else if change.hunks.is_empty() && change.skipped_reason.is_none() {
        change.skipped_reason =
            Some("Git reported a file or mode change without line-level hunks.".to_owned());
    }
    Ok(change)
}

fn patch_args(base_revision_id: &str, head_revision_id: &str, raw: &RawGitChange) -> Vec<String> {
    let mut arguments = vec![
        "--literal-pathspecs".to_owned(),
        "diff".to_owned(),
        "--no-ext-diff".to_owned(),
        "--no-textconv".to_owned(),
        "--no-color".to_owned(),
        "--unified=3".to_owned(),
        "--find-renames=50%".to_owned(),
        base_revision_id.to_owned(),
        head_revision_id.to_owned(),
        "--".to_owned(),
    ];
    if let Some(path) = &raw.base_path_bytes {
        if let Ok(path) = std::str::from_utf8(path) {
            arguments.push(path.to_owned());
        }
    }
    if let Some(path) = &raw.head_path_bytes {
        if let Ok(path) = std::str::from_utf8(path) {
            let literal = path.to_owned();
            if !arguments.contains(&literal) {
                arguments.push(literal);
            }
        }
    }
    arguments
}

fn parse_patch_hunks(
    patch: &[u8],
    base_revision_id: &str,
    head_revision_id: &str,
    base_path: Option<&str>,
    head_path: Option<&str>,
    base_blob_id: Option<&str>,
    head_blob_id: Option<&str>,
) -> (Vec<GitDiffHunk>, bool) {
    let patch = String::from_utf8_lossy(patch);
    let mut hunks = Vec::new();
    let mut active_header: Option<(Option<LineRange>, Option<LineRange>)> = None;
    let mut active_excerpt = String::new();
    let mut base_cursor = None;
    let mut head_cursor = None;
    let mut base_changed_ranges = Vec::new();
    let mut head_changed_ranges = Vec::new();
    let mut hunk_limit_hit = false;
    let flush = |header: Option<(Option<LineRange>, Option<LineRange>)>,
                 excerpt: &mut String,
                 base_ranges: &mut Vec<GitChangedRange>,
                 head_ranges: &mut Vec<GitChangedRange>,
                 hunks: &mut Vec<GitDiffHunk>,
                 hit: &mut bool| {
        let Some((base_range, head_range)) = header else {
            excerpt.clear();
            base_ranges.clear();
            head_ranges.clear();
            return;
        };
        if hunks.len() >= MAX_HUNKS_PER_FILE {
            *hit = true;
            excerpt.clear();
            base_ranges.clear();
            head_ranges.clear();
            return;
        }
        let base_anchor = base_range.and_then(|range| {
            Some(GitSourceAnchor {
                revision_id: base_revision_id.to_owned(),
                path: base_path?.to_owned(),
                blob_id: base_blob_id?.to_owned(),
                start_line: range.start,
                end_line: range.end,
            })
        });
        let head_anchor = head_range.and_then(|range| {
            Some(GitSourceAnchor {
                revision_id: head_revision_id.to_owned(),
                path: head_path?.to_owned(),
                blob_id: head_blob_id?.to_owned(),
                start_line: range.start,
                end_line: range.end,
            })
        });
        hunks.push(GitDiffHunk {
            base_anchor,
            head_anchor,
            base_changed_ranges: std::mem::take(base_ranges),
            head_changed_ranges: std::mem::take(head_ranges),
            base_symbols: Vec::new(),
            head_symbols: Vec::new(),
            patch_excerpt: excerpt.trim_end().to_owned(),
        });
        excerpt.clear();
    };

    for line in patch.lines() {
        if line.starts_with("@@ ") {
            flush(
                active_header.take(),
                &mut active_excerpt,
                &mut base_changed_ranges,
                &mut head_changed_ranges,
                &mut hunks,
                &mut hunk_limit_hit,
            );
            if hunks.len() >= MAX_HUNKS_PER_FILE {
                hunk_limit_hit = true;
                break;
            }
            active_header = parse_hunk_header(line);
            if let Some((base_range, head_range)) = active_header {
                base_cursor = base_range.map(|range| range.start);
                head_cursor = head_range.map(|range| range.start);
                push_bounded_line(&mut active_excerpt, line);
            }
        } else if active_header.is_some() {
            track_hunk_line(
                line,
                &mut base_cursor,
                &mut head_cursor,
                &mut base_changed_ranges,
                &mut head_changed_ranges,
            );
            push_bounded_line(&mut active_excerpt, line);
        }
    }
    flush(
        active_header,
        &mut active_excerpt,
        &mut base_changed_ranges,
        &mut head_changed_ranges,
        &mut hunks,
        &mut hunk_limit_hit,
    );
    (hunks, hunk_limit_hit)
}

#[derive(Clone, Copy)]
struct LineRange {
    start: u64,
    end: u64,
}

fn track_hunk_line(
    line: &str,
    base_cursor: &mut Option<u64>,
    head_cursor: &mut Option<u64>,
    base_ranges: &mut Vec<GitChangedRange>,
    head_ranges: &mut Vec<GitChangedRange>,
) {
    match line.as_bytes().first() {
        Some(b' ') => {
            advance_line(base_cursor);
            advance_line(head_cursor);
        }
        Some(b'-') => {
            push_changed_line(base_ranges, *base_cursor);
            advance_line(base_cursor);
        }
        Some(b'+') => {
            push_changed_line(head_ranges, *head_cursor);
            advance_line(head_cursor);
        }
        _ => {}
    }
}

fn advance_line(cursor: &mut Option<u64>) {
    if let Some(line) = cursor.as_mut() {
        *line = line.saturating_add(1);
    }
}

fn push_changed_line(ranges: &mut Vec<GitChangedRange>, line: Option<u64>) {
    let Some(line) = line else {
        return;
    };
    if let Some(last) = ranges.last_mut() {
        if line <= last.end_line.saturating_add(1) {
            last.end_line = last.end_line.max(line);
            return;
        }
    }
    ranges.push(GitChangedRange {
        start_line: line,
        end_line: line,
    });
}

fn parse_hunk_header(line: &str) -> Option<(Option<LineRange>, Option<LineRange>)> {
    let mut parts = line.split_whitespace();
    if parts.next()? != "@@" {
        return None;
    }
    let base = parse_hunk_range(parts.next()?)?;
    let head = parse_hunk_range(parts.next()?)?;
    Some((base, head))
}

fn parse_hunk_range(value: &str) -> Option<Option<LineRange>> {
    let value = value.strip_prefix(['-', '+'])?;
    let (start, count) = match value.split_once(',') {
        Some((start, count)) => (start.parse::<u64>().ok()?, count.parse::<u64>().ok()?),
        None => (value.parse::<u64>().ok()?, 1),
    };
    if count == 0 {
        return Some(None);
    }
    Some(Some(LineRange {
        start,
        end: start.saturating_add(count.saturating_sub(1)),
    }))
}

fn push_bounded_line(output: &mut String, line: &str) {
    if output.len() >= MAX_HUNK_TEXT_BYTES {
        return;
    }
    let available = MAX_HUNK_TEXT_BYTES.saturating_sub(output.len());
    let line = if line.len() > available {
        let mut end = available;
        while !line.is_char_boundary(end) {
            end = end.saturating_sub(1);
        }
        &line[..end]
    } else {
        line
    };
    output.push_str(line);
    output.push('\n');
}

fn blob_size(
    repository: &Path,
    object_id: Option<&str>,
    deadline: Instant,
) -> Result<Option<u64>, GitEvidenceError> {
    let Some(object_id) = object_id else {
        return Ok(None);
    };
    let output = git_output(repository, &["cat-file", "-s", object_id], 128, deadline)?;
    if !output.status.success() {
        return Ok(None);
    }
    let size = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .map_err(|_| {
            GitEvidenceError::InvalidOutput("Git returned an invalid object size.".to_owned())
        })?;
    Ok(Some(size))
}

fn read_text_blob(
    repository: &Path,
    object_id: Option<&str>,
    size_bytes: Option<u64>,
    deadline: Instant,
) -> Result<Option<TextBlob>, GitEvidenceError> {
    let (Some(object_id), Some(size_bytes)) = (object_id, size_bytes) else {
        return Ok(None);
    };
    if size_bytes > MAX_FILE_BYTES {
        return Ok(None);
    }
    let output = git_output(
        repository,
        &["cat-file", "blob", object_id],
        MAX_FILE_BYTES as usize,
        deadline,
    )?;
    if !output.status.success() {
        return Err(GitEvidenceError::Git(git_failure_message(&output)));
    }
    Ok(Some(TextBlob {
        bytes: output.stdout,
    }))
}

fn is_text(bytes: &[u8]) -> bool {
    !bytes.contains(&0) && std::str::from_utf8(bytes).is_ok()
}

fn parsed_symbols_for_blob(
    blob: Option<&TextBlob>,
    language: ParseLanguage,
) -> (Vec<ParsedGitSymbol>, bool) {
    let Some(blob) = blob else {
        return (Vec::new(), true);
    };
    match parse_git_symbols(language, &blob.bytes) {
        Some(symbols) => (symbols, true),
        None => (Vec::new(), false),
    }
}

fn parse_git_symbols(language: ParseLanguage, source: &[u8]) -> Option<Vec<ParsedGitSymbol>> {
    let mut parser = Parser::new();
    parser.set_language(&language.grammar()).ok()?;
    let started = Instant::now();
    let mut exceeded_time = false;
    let mut progress = |_state: &tree_sitter::ParseState| {
        if started.elapsed() > MAX_PARSE_TIME {
            exceeded_time = true;
            std::ops::ControlFlow::Break(())
        } else {
            std::ops::ControlFlow::Continue(())
        }
    };
    let options = ParseOptions::new().progress_callback(&mut progress);
    let mut read_chunk = |offset: usize, _position: Point| source.get(offset..).unwrap_or(&[]);
    let tree = parser.parse_with_options(&mut read_chunk, None, Some(options))?;
    if exceeded_time || started.elapsed() > MAX_PARSE_TIME {
        return None;
    }
    let source_text = std::str::from_utf8(source).ok()?;
    let mut stack = vec![tree.root_node()];
    let mut visited = 0_usize;
    let mut symbols = Vec::new();
    while let Some(node) = stack.pop() {
        visited += 1;
        if visited > MAX_AST_NODES_PER_FILE || node.kind() == "ERROR" || node.is_missing() {
            return None;
        }
        if let Some(kind) = git_symbol_kind(node.kind(), language) {
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = source_text.get(name_node.start_byte()..name_node.end_byte())?;
                if !name.is_empty() && name.len() <= 512 {
                    let start = node.start_position();
                    let end = node.end_position();
                    let start_line = start.row as u64 + 1;
                    let end_line = if end.row > start.row && end.column == 0 {
                        end.row as u64
                    } else {
                        end.row as u64 + 1
                    };
                    symbols.push(ParsedGitSymbol {
                        kind,
                        name: name.to_owned(),
                        start_line,
                        end_line: end_line.max(start_line),
                    });
                    if symbols.len() > MAX_SYMBOLS_PER_FILE {
                        return None;
                    }
                }
            }
        }
        for index in 0..node.child_count() {
            if let Some(child) = node.child(index) {
                stack.push(child);
            }
        }
    }
    symbols.sort_by(|left, right| {
        left.start_line
            .cmp(&right.start_line)
            .then(left.end_line.cmp(&right.end_line))
            .then(left.kind.cmp(right.kind))
            .then(left.name.cmp(&right.name))
    });
    Some(symbols)
}

fn git_symbol_kind(kind: &str, language: ParseLanguage) -> Option<&'static str> {
    match language {
        ParseLanguage::Rust => match kind {
            "function_item" | "function_signature" => Some("function"),
            "struct_item" | "union_item" => Some("struct"),
            "enum_item" => Some("enum"),
            "trait_item" => Some("trait"),
            "type_item" => Some("type alias"),
            "mod_item" => Some("module"),
            "const_item" | "static_item" => Some("constant"),
            "macro_definition" => Some("macro"),
            _ => None,
        },
        ParseLanguage::TypeScript | ParseLanguage::Tsx => match kind {
            "function_declaration" | "function_signature" | "generator_function_declaration" => {
                Some("function")
            }
            "method_definition" | "method_signature" | "abstract_method_signature" => {
                Some("method")
            }
            "class_declaration" | "abstract_class_declaration" | "class" => Some("class"),
            "interface_declaration" => Some("interface"),
            "type_alias_declaration" => Some("type alias"),
            "enum_declaration" => Some("enum"),
            "internal_module" | "module" => Some("module"),
            "variable_declarator" => Some("variable"),
            _ => None,
        },
        ParseLanguage::JavaScript => match kind {
            "function_declaration" | "generator_function_declaration" => Some("function"),
            "method_definition" => Some("method"),
            "class_declaration" | "class" => Some("class"),
            "variable_declarator" => Some("variable"),
            _ => None,
        },
    }
}

fn symbols_for_hunk(
    symbols: &[ParsedGitSymbol],
    hunk_anchor: Option<&GitSourceAnchor>,
    changed_ranges: &[GitChangedRange],
) -> Vec<GitSymbolEvidence> {
    let Some(hunk_anchor) = hunk_anchor else {
        return Vec::new();
    };
    symbols
        .iter()
        .filter(|symbol| {
            changed_ranges.iter().any(|range| {
                symbol.start_line <= range.end_line && symbol.end_line >= range.start_line
            })
        })
        .map(|symbol| GitSymbolEvidence {
            kind: symbol.kind.to_owned(),
            name: symbol.name.clone(),
            anchor: GitSourceAnchor {
                revision_id: hunk_anchor.revision_id.clone(),
                path: hunk_anchor.path.clone(),
                blob_id: hunk_anchor.blob_id.clone(),
                start_line: symbol.start_line,
                end_line: symbol.end_line,
            },
        })
        .collect()
}

fn parse_raw_diff(output: &[u8]) -> Result<Vec<RawGitChange>, String> {
    let fields = output.split(|byte| *byte == 0).collect::<Vec<_>>();
    let mut index = 0;
    let mut changes = Vec::new();
    while index < fields.len() {
        let header = fields[index];
        index += 1;
        if header.is_empty() {
            continue;
        }
        let header = std::str::from_utf8(header)
            .map_err(|_| "Git returned non-UTF-8 diff metadata.".to_owned())?;
        let header_fields = header.split_ascii_whitespace().collect::<Vec<_>>();
        if header_fields.len() != 5 || !header_fields[0].starts_with(':') {
            return Err("Git returned malformed raw diff metadata.".to_owned());
        }
        let base_mode = header_fields[0].trim_start_matches(':').to_owned();
        let head_mode = header_fields[1].to_owned();
        let base_blob_id = normalize_blob_id(header_fields[2]);
        let head_blob_id = normalize_blob_id(header_fields[3]);
        let code = header_fields[4]
            .as_bytes()
            .first()
            .copied()
            .unwrap_or_default();
        let kind = match code {
            b'A' => GitChangeKind::Added,
            b'D' => GitChangeKind::Deleted,
            b'M' => GitChangeKind::Modified,
            b'R' => GitChangeKind::Renamed,
            b'C' => GitChangeKind::Copied,
            b'T' => GitChangeKind::TypeChanged,
            _ => GitChangeKind::Other,
        };
        let first_path = next_raw_field(&fields, &mut index)?;
        let (base_path_bytes, head_path_bytes) = match kind {
            GitChangeKind::Added => (None, Some(first_path)),
            GitChangeKind::Deleted => (Some(first_path), None),
            GitChangeKind::Renamed | GitChangeKind::Copied => {
                let second_path = next_raw_field(&fields, &mut index)?;
                (Some(first_path), Some(second_path))
            }
            _ => (Some(first_path.clone()), Some(first_path)),
        };
        changes.push(RawGitChange {
            kind,
            base_path_bytes,
            head_path_bytes,
            base_mode,
            head_mode,
            base_blob_id,
            head_blob_id,
        });
    }
    Ok(changes)
}

fn next_raw_field(fields: &[&[u8]], field_index: &mut usize) -> Result<Vec<u8>, String> {
    let path = fields
        .get(*field_index)
        .ok_or_else(|| "Git returned incomplete raw diff metadata.".to_owned())?;
    *field_index += 1;
    Ok(path.to_vec())
}

fn normalize_blob_id(value: &str) -> Option<String> {
    if value.bytes().all(|byte| byte == b'0') {
        None
    } else {
        Some(value.to_ascii_lowercase())
    }
}

fn display_path(path: Option<&[u8]>) -> String {
    let Some(path) = path else {
        return String::new();
    };
    match std::str::from_utf8(path) {
        Ok(path) => path.to_owned(),
        Err(_) => path
            .iter()
            .map(|byte| match *byte {
                b' '..=b'~' if *byte != b'\\' => (*byte as char).to_string(),
                b'\\' => "\\\\".to_owned(),
                _ => format!("\\x{byte:02x}"),
            })
            .collect(),
    }
}

fn comparison_id(
    base_requested: &str,
    base_object_id: &str,
    head_requested: &str,
    head_object_id: &str,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"code-atlas-local-git-comparison-v1\0");
    digest.update(base_requested.as_bytes());
    digest.update([0]);
    digest.update(base_object_id.as_bytes());
    digest.update([0]);
    digest.update(head_requested.as_bytes());
    digest.update([0]);
    digest.update(head_object_id.as_bytes());
    format!("cmp:{}", hex_digest(digest.finalize().as_slice()))
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn change_kind_order(kind: &GitChangeKind) -> u8 {
    match kind {
        GitChangeKind::Added => 0,
        GitChangeKind::Deleted => 1,
        GitChangeKind::Modified => 2,
        GitChangeKind::Renamed => 3,
        GitChangeKind::Copied => 4,
        GitChangeKind::TypeChanged => 5,
        GitChangeKind::Other => 6,
    }
}

fn git_text(
    repository: &Path,
    args: &[&str],
    deadline: Instant,
) -> Result<String, GitEvidenceError> {
    let output = git_output(repository, args, 4096, deadline)?;
    if !output.status.success() {
        return Err(GitEvidenceError::Git(git_failure_message(&output)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn git_output(
    repository: &Path,
    args: &[&str],
    max_stdout_bytes: usize,
    deadline: Instant,
) -> Result<Output, GitEvidenceError> {
    run_git_output(repository, args, max_stdout_bytes, deadline, None)
}

fn git_output_with_input(
    repository: &Path,
    args: &[&str],
    max_stdout_bytes: usize,
    deadline: Instant,
    input: &[u8],
) -> Result<Output, GitEvidenceError> {
    run_git_output(repository, args, max_stdout_bytes, deadline, Some(input))
}

fn run_git_output(
    repository: &Path,
    args: &[&str],
    max_stdout_bytes: usize,
    deadline: Instant,
    input: Option<&[u8]>,
) -> Result<Output, GitEvidenceError> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err(GitEvidenceError::TimedOut(
            "The bounded local Git command reached its time limit; remaining evidence was omitted."
                .to_owned(),
        ));
    }
    let mut command = git_command();
    command
        .args(args)
        .current_dir(repository)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_NO_REPLACE_OBJECTS", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null");
    let output = bounded_process::run(
        &mut command,
        input.map(<[u8]>::to_vec),
        Limits {
            command_timeout: remaining,
            operation_deadline: deadline,
            stdout_bytes: max_stdout_bytes,
            stderr_bytes: MAX_GIT_STDERR_BYTES,
            stdin_bytes: 1024 * 1024,
        },
        None,
    )
    .map_err(|error| match error {
        RunError::TimedOut => GitEvidenceError::TimedOut(
            "The bounded local Git command reached its time limit; remaining evidence was omitted."
                .to_owned(),
        ),
        RunError::OutputLimit | RunError::InputLimit => GitEvidenceError::OutputLimit(
            "Git input or output exceeded a local evidence size limit; remaining evidence was omitted."
                .to_owned(),
        ),
        other => GitEvidenceError::Git(format!(
            "Git could not complete the local comparison: {other}"
        )),
    })?;
    Ok(output)
}

fn git_failure_message(output: &Output) -> String {
    let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
    if message.is_empty() {
        "Git could not produce local comparison evidence.".to_owned()
    } else {
        message.chars().take(1_000).collect()
    }
}

impl GitEvidenceError {
    fn message(&self) -> String {
        match self {
            Self::Git(message)
            | Self::OutputLimit(message)
            | Self::TimedOut(message)
            | Self::InvalidOutput(message) => message.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use uuid::Uuid;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path =
                std::env::temp_dir().join(format!("code-atlas-git-evidence-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).expect("temporary folder should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn git(path: &Path, args: &[&str]) -> Output {
        let output = git_command()
            .args(args)
            .current_dir(path)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .expect("git should be available for isolated fixture tests");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn init_repository(path: &Path) {
        fs::create_dir_all(path).expect("repository fixture should be created");
        git(path, &["init", "--quiet", "--initial-branch=main"]);
        git(path, &["config", "user.name", "Code Atlas Tests"]);
        git(path, &["config", "user.email", "tests@example.invalid"]);
    }

    fn commit_fixture(path: &Path, message: &str) -> String {
        git(path, &["add", "--all"]);
        git(path, &["commit", "--quiet", "-m", message]);
        String::from_utf8_lossy(&git(path, &["rev-parse", "HEAD"]).stdout)
            .trim()
            .to_owned()
    }

    fn blob_at(path: &Path, revision_id: &str, relative_path: &str) -> String {
        let object = format!("{revision_id}:{relative_path}");
        String::from_utf8_lossy(&git(path, &["rev-parse", &object]).stdout)
            .trim()
            .to_owned()
    }

    fn stage_ref(path: &Path, name: &str, commit_id: &str) {
        let reference = format!("refs/heads/{name}");
        git(path, &["update-ref", &reference, commit_id]);
    }

    fn refs(path: &Path) -> Vec<u8> {
        git(path, &["show-ref", "--head", "--dereference"]).stdout
    }

    fn tree_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
        fn visit(root: &Path, folder: &Path, files: &mut BTreeMap<String, Vec<u8>>) {
            for entry in fs::read_dir(folder).expect("fixture folder should be readable") {
                let entry = entry.expect("fixture entry should be readable");
                let path = entry.path();
                if path.file_name().is_some_and(|name| name == ".git") {
                    continue;
                }
                if path.is_dir() {
                    visit(root, &path, files);
                } else if path.is_file() {
                    let relative = path
                        .strip_prefix(root)
                        .expect("fixture path should be under repository")
                        .to_string_lossy()
                        .into_owned();
                    files.insert(
                        relative,
                        fs::read(path).expect("fixture file should be readable"),
                    );
                }
            }
        }
        let mut files = BTreeMap::new();
        visit(path, path, &mut files);
        files
    }

    fn file_change<'a>(result: &'a GitComparisonResult, path: &str) -> &'a GitFileChange {
        result
            .changes
            .iter()
            .find(|change| {
                change.head_path.as_deref() == Some(path)
                    || change.base_path.as_deref() == Some(path)
            })
            .expect("expected path change")
    }

    #[test]
    fn older_pr_guide_file_change_records_default_relationship_evidence_to_empty() {
        let change: GitFileChange = serde_json::from_value(serde_json::json!({
            "kind": "modified",
            "basePath": "src/lib.rs",
            "headPath": "src/lib.rs",
            "classification": "text",
            "baseBlobId": null,
            "headBlobId": null,
            "baseSizeBytes": null,
            "headSizeBytes": null,
            "hunks": [],
            "skippedReason": null
        }))
        .expect("older saved file evidence should remain readable");

        assert!(change.base_relationships.is_empty());
        assert!(change.head_relationships.is_empty());
    }

    #[test]
    fn resolves_refs_strictly_and_returns_choose_again_for_invalid_missing_and_ambiguous() {
        let temporary = TestDirectory::new();
        let repository = temporary.path();
        init_repository(repository);
        fs::write(repository.join("src.rs"), "fn value() { 1 }\n").unwrap();
        let base = commit_fixture(repository, "base");
        fs::write(repository.join("src.rs"), "fn value() { 2 }\n").unwrap();
        let head = commit_fixture(repository, "head");
        stage_ref(repository, "release", &head);
        git(repository, &["tag", "release", &head]);

        let ambiguous = compare_local_revisions(repository, "release", "main");
        assert_eq!(ambiguous.status, GitComparisonStatus::ChooseAgain);
        assert_eq!(
            ambiguous.ref_problem.as_ref().unwrap().kind,
            RefProblemKind::Ambiguous
        );

        let missing = compare_local_revisions(repository, "missing-branch", "main");
        assert_eq!(missing.status, GitComparisonStatus::ChooseAgain);
        assert_eq!(
            missing.ref_problem.as_ref().unwrap().kind,
            RefProblemKind::Missing
        );

        let invalid = compare_local_revisions(repository, "main..HEAD", "main");
        assert_eq!(invalid.status, GitComparisonStatus::ChooseAgain);
        assert_eq!(
            invalid.ref_problem.as_ref().unwrap().kind,
            RefProblemKind::Invalid
        );

        let valid = compare_local_revisions(repository, &base, &head);
        assert_eq!(valid.status, GitComparisonStatus::Ready);
        assert_eq!(valid.base.as_ref().unwrap().object_id, base);
        assert_eq!(valid.head.as_ref().unwrap().object_id, head);
    }

    #[test]
    fn reports_deterministic_hunks_with_distinct_base_and_head_anchors_without_mutation() {
        let temporary = TestDirectory::new();
        let repository = temporary.path();
        init_repository(repository);
        fs::create_dir_all(repository.join("src")).unwrap();
        fs::write(
            repository.join("src/lib.rs"),
            "pub fn first() {\n    let before = 1;\n}\n\npub fn second() {\n    let stable = 9;\n}\n",
        )
        .unwrap();
        let base = commit_fixture(repository, "base");
        fs::write(
            repository.join("src/lib.rs"),
            "pub fn first() {\n    let after = 2;\n}\n\npub fn second() {\n    let stable = 9;\n}\n",
        )
        .unwrap();
        let head = commit_fixture(repository, "head");
        fs::write(repository.join("untracked.txt"), "user work remains here\n").unwrap();

        let refs_before = refs(repository);
        let tree_before = tree_files(repository);
        let index_before = fs::read(repository.join(".git/index")).unwrap();
        let first = compare_local_revisions(repository, &base, &head);
        let second = compare_local_revisions(repository, &base, &head);
        let refs_after = refs(repository);
        let tree_after = tree_files(repository);
        let index_after = fs::read(repository.join(".git/index")).unwrap();

        assert_eq!(first.status, GitComparisonStatus::Ready);
        assert_eq!(first.changes, second.changes);
        assert_eq!(first.comparison_id, second.comparison_id);
        let change = file_change(&first, "src/lib.rs");
        assert_eq!(change.kind, GitChangeKind::Modified);
        assert_eq!(change.classification, EvidenceClassification::Text);
        let hunk = change
            .hunks
            .first()
            .expect("modified code should have a hunk");
        let base_anchor = hunk.base_anchor.as_ref().unwrap();
        let head_anchor = hunk.head_anchor.as_ref().unwrap();
        assert_eq!(base_anchor.revision_id, base);
        assert_eq!(head_anchor.revision_id, head);
        assert_eq!(base_anchor.path, "src/lib.rs");
        assert_eq!(head_anchor.path, "src/lib.rs");
        assert!(hunk.patch_excerpt.contains("before"));
        assert!(hunk.patch_excerpt.contains("after"));
        assert_eq!(hunk.base_changed_ranges.len(), 1);
        assert_eq!(hunk.base_changed_ranges[0].start_line, 2);
        assert_eq!(hunk.base_changed_ranges[0].end_line, 2);
        assert_eq!(hunk.head_changed_ranges.len(), 1);
        assert_eq!(hunk.head_changed_ranges[0].start_line, 2);
        assert_eq!(hunk.head_changed_ranges[0].end_line, 2);
        assert_eq!(hunk.base_symbols.len(), 1);
        assert_eq!(hunk.head_symbols.len(), 1);
        assert_eq!(hunk.base_symbols[0].name, "first");
        assert_eq!(hunk.head_symbols[0].name, "first");
        assert_eq!(hunk.base_symbols[0].anchor.revision_id, base);
        assert_eq!(hunk.head_symbols[0].anchor.revision_id, head);
        assert!(
            !hunk
                .base_symbols
                .iter()
                .any(|symbol| symbol.name == "second")
        );
        assert_eq!(refs_before, refs_after);
        assert_eq!(tree_before, tree_after);
        assert_eq!(index_before, index_after);
    }

    #[test]
    fn attaches_revision_exact_cross_file_import_and_call_evidence_to_changed_declarations() {
        let temporary = TestDirectory::new();
        let repository = temporary.path();
        init_repository(repository);
        fs::create_dir_all(repository.join("src")).unwrap();
        fs::write(
            repository.join("src/worker.ts"),
            "export function targetFn(value: number): number {\n  return value + 1;\n}\n",
        )
        .unwrap();
        fs::write(
            repository.join("src/client.ts"),
            "import { targetFn } from './worker';\n\nexport function caller(): number {\n  return targetFn(1);\n}\n",
        )
        .unwrap();
        fs::write(
            repository.join("src/unrelated.ts"),
            "export function unrelated(): number { return 0; }\n",
        )
        .unwrap();
        let base = commit_fixture(repository, "base");

        fs::write(
            repository.join("src/worker.ts"),
            "export function targetFn(value: number): number {\n  return value + 2;\n}\n",
        )
        .unwrap();
        let head = commit_fixture(repository, "head");

        let result = compare_local_revisions(repository, &base, &head);

        assert_eq!(result.status, GitComparisonStatus::Ready);
        assert_eq!(
            result.relationship_impact.status,
            GitRelationshipImpactStatus::Complete,
            "{}",
            result.relationship_impact.message
        );
        let change = file_change(&result, "src/worker.ts");
        assert_eq!(change.head_relationships.len(), 2);
        for (revision_id, relationships) in [
            (base.as_str(), &change.base_relationships),
            (head.as_str(), &change.head_relationships),
        ] {
            let client_blob_id = blob_at(repository, revision_id, "src/client.ts");
            let worker_blob_id = blob_at(repository, revision_id, "src/worker.ts");
            assert_eq!(relationships.len(), 2);
            assert!(relationships.iter().all(|relationship| {
                relationship.source_path == "src/client.ts"
                    && relationship.target_path == "src/worker.ts"
                    && relationship.confidence == "high"
                    && relationship.evidence_anchor.revision_id == revision_id
                    && relationship.evidence_anchor.blob_id == client_blob_id
            }));
            let import = relationships
                .iter()
                .find(|relationship| relationship.kind == "imports")
                .expect("resolved import to the changed file should be attached");
            assert_eq!(import.evidence_anchor.start_line, 1);
            assert_eq!(import.target_anchor, None);
            let call = relationships
                .iter()
                .find(|relationship| relationship.kind == "calls")
                .expect("resolved caller of the changed declaration should be attached");
            assert_eq!(call.target_name.as_deref(), Some("targetFn"));
            let target_anchor = call
                .target_anchor
                .as_ref()
                .expect("changed declaration should carry a target source anchor");
            assert_eq!(target_anchor.revision_id, revision_id);
            assert_eq!(target_anchor.path, "src/worker.ts");
            assert_eq!(target_anchor.blob_id, worker_blob_id);
            assert_eq!(target_anchor.start_line, 1);
            assert_eq!(target_anchor.end_line, 3);
        }
        assert!(
            change
                .base_relationships
                .iter()
                .chain(&change.head_relationships)
                .all(|relationship| relationship.source_path != "src/unrelated.ts")
        );
    }

    #[test]
    fn base_and_head_revision_indexes_apply_lexical_shadowing_independently() {
        let temporary = TestDirectory::new();
        let repository = temporary.path();
        init_repository(repository);
        fs::create_dir_all(repository.join("src")).unwrap();
        fs::write(
            repository.join("src/worker.ts"),
            "export function targetFn(value: number): number { return value + 1; }\n",
        )
        .unwrap();
        fs::write(
            repository.join("src/client.ts"),
            concat!(
                "import { targetFn } from './worker';\n",
                "export function caller(targetFn: (value: number) => number) { return targetFn(1); }\n",
            ),
        )
        .unwrap();
        let base = commit_fixture(repository, "shadowed base");

        fs::write(
            repository.join("src/worker.ts"),
            "export function targetFn(value: number): number { return value + 2; }\n",
        )
        .unwrap();
        fs::write(
            repository.join("src/client.ts"),
            concat!(
                "import { targetFn } from './worker';\n",
                "export function caller() { return targetFn(1); }\n",
            ),
        )
        .unwrap();
        let head = commit_fixture(repository, "unshadowed head");

        let result = compare_local_revisions(repository, &base, &head);
        assert_eq!(result.status, GitComparisonStatus::Ready);
        assert_eq!(
            result.relationship_impact.status,
            GitRelationshipImpactStatus::Complete,
            "{}",
            result.relationship_impact.message
        );
        let change = file_change(&result, "src/worker.ts");
        assert_eq!(change.base_relationships.len(), 1);
        assert_eq!(change.base_relationships[0].kind, "imports");
        assert_eq!(change.base_relationships[0].confidence, "high");
        assert_eq!(change.head_relationships.len(), 2);
        let head_call = change
            .head_relationships
            .iter()
            .find(|relationship| relationship.kind == "calls")
            .expect("the unshadowed head call should resolve to the changed declaration");
        assert_eq!(head_call.confidence, "high");
        assert_eq!(head_call.target_name.as_deref(), Some("targetFn"));
        let target = head_call
            .target_anchor
            .as_ref()
            .expect("the resolved target must retain its exact head anchor");
        assert_eq!(target.revision_id, head);
        assert_eq!(target.path, "src/worker.ts");
        assert_eq!(target.blob_id, blob_at(repository, &head, "src/worker.ts"));
        assert_eq!(head_call.evidence_anchor.revision_id, head);
        assert_eq!(
            head_call.evidence_anchor.blob_id,
            blob_at(repository, &head, "src/client.ts")
        );
    }

    #[test]
    fn labels_relationship_evidence_partial_when_a_supported_file_is_parser_uncertain() {
        let temporary = TestDirectory::new();
        let repository = temporary.path();
        init_repository(repository);
        fs::create_dir_all(repository.join("src")).unwrap();
        fs::write(
            repository.join("src/worker.ts"),
            "export function targetFn(value: number): number { return value + 1; }\n",
        )
        .unwrap();
        fs::write(
            repository.join("src/client.ts"),
            "import { targetFn } from './worker';\nexport function caller() { return targetFn(1); }\n",
        )
        .unwrap();
        fs::write(
            repository.join("src/uncertain.ts"),
            "export function broken( {\n",
        )
        .unwrap();
        let base = commit_fixture(repository, "base");

        fs::write(
            repository.join("src/worker.ts"),
            "export function targetFn(value: number): number { return value + 2; }\n",
        )
        .unwrap();
        let head = commit_fixture(repository, "head");

        let result = compare_local_revisions(repository, &base, &head);
        assert_eq!(
            result.relationship_impact.status,
            GitRelationshipImpactStatus::Partial
        );
        assert!(
            result
                .relationship_impact
                .message
                .contains("omitted callers may exist")
        );
        let change = file_change(&result, "src/worker.ts");
        assert!(
            change
                .head_relationships
                .iter()
                .any(|relationship| relationship.kind == "calls")
        );
    }

    #[test]
    fn caps_relationship_evidence_per_changed_file_and_reports_omitted_matches() {
        let temporary = TestDirectory::new();
        let repository = temporary.path();
        init_repository(repository);
        fs::create_dir_all(repository.join("src")).unwrap();
        fs::write(
            repository.join("src/worker.ts"),
            "export function targetFn(value: number): number { return value + 1; }\n",
        )
        .unwrap();
        for index in 0..20 {
            fs::write(
                repository.join(format!("src/client-{index:02}.ts")),
                "import { targetFn } from './worker';\nexport function caller() { return targetFn(1); }\n",
            )
            .unwrap();
        }
        let base = commit_fixture(repository, "base");

        fs::write(
            repository.join("src/worker.ts"),
            "export function targetFn(value: number): number { return value + 2; }\n",
        )
        .unwrap();
        let head = commit_fixture(repository, "head");

        let result = compare_local_revisions(repository, &base, &head);
        assert_eq!(
            result.relationship_impact.status,
            GitRelationshipImpactStatus::Partial
        );
        assert!(
            result
                .relationship_impact
                .message
                .contains("capped at 16 records")
        );
        let change = file_change(&result, "src/worker.ts");
        assert_eq!(
            change.base_relationships.len(),
            MAX_RELATIONSHIP_EVIDENCE_PER_FILE_SIDE
        );
        assert_eq!(
            change.head_relationships.len(),
            MAX_RELATIONSHIP_EVIDENCE_PER_FILE_SIDE
        );
        assert!(
            change
                .base_relationships
                .windows(2)
                .all(|pair| pair[0].source_path <= pair[1].source_path)
        );
        assert!(
            change
                .head_relationships
                .windows(2)
                .all(|pair| pair[0].source_path <= pair[1].source_path)
        );
    }

    #[test]
    fn classifies_add_delete_rename_binary_oversized_unsupported_and_parser_uncertain() {
        let temporary = TestDirectory::new();
        let repository = temporary.path();
        init_repository(repository);
        fs::create_dir_all(repository.join("src")).unwrap();
        fs::write(
            repository.join("src/edit.rs"),
            "fn edit() { let before = 1; }\n",
        )
        .unwrap();
        fs::write(repository.join("src/delete.rs"), "fn removed() {}\n").unwrap();
        fs::write(repository.join("src/rename.rs"), "fn moved() {}\n").unwrap();
        fs::write(repository.join("docs.txt"), "old text\n").unwrap();
        fs::write(repository.join("broken.rs"), "fn broken( {\n").unwrap();
        fs::write(repository.join("binary.dat"), [0_u8, 1, 2, 3]).unwrap();
        let base = commit_fixture(repository, "base");

        fs::write(
            repository.join("src/edit.rs"),
            "fn edit() { let after = 2; }\n",
        )
        .unwrap();
        fs::remove_file(repository.join("src/delete.rs")).unwrap();
        fs::rename(
            repository.join("src/rename.rs"),
            repository.join("src/renamed.rs"),
        )
        .unwrap();
        fs::write(repository.join("docs.txt"), "new text\n").unwrap();
        fs::write(repository.join("broken.rs"), "fn broken( { let = ;\n").unwrap();
        fs::write(repository.join("binary.dat"), [0_u8, 4, 5, 6]).unwrap();
        fs::write(
            repository.join("large.rs"),
            vec![b'x'; MAX_FILE_BYTES as usize + 1],
        )
        .unwrap();
        fs::write(repository.join("new.rs"), "pub fn added() {}\n").unwrap();
        let head = commit_fixture(repository, "head");

        let result = compare_local_revisions(repository, &base, &head);
        assert_eq!(result.status, GitComparisonStatus::Ready);
        assert_eq!(
            file_change(&result, "src/edit.rs").classification,
            EvidenceClassification::Text
        );
        assert_eq!(
            file_change(&result, "src/delete.rs").kind,
            GitChangeKind::Deleted
        );
        assert_eq!(
            file_change(&result, "src/delete.rs").classification,
            EvidenceClassification::Text
        );
        let deleted_hunk = file_change(&result, "src/delete.rs")
            .hunks
            .first()
            .expect("deletion should retain its base-side hunk");
        assert!(deleted_hunk.base_anchor.is_some());
        assert!(deleted_hunk.head_anchor.is_none());
        assert_eq!(
            file_change(&result, "src/renamed.rs").kind,
            GitChangeKind::Renamed
        );
        assert_eq!(
            file_change(&result, "binary.dat").classification,
            EvidenceClassification::Binary
        );
        assert_eq!(
            file_change(&result, "large.rs").classification,
            EvidenceClassification::Oversized
        );
        assert_eq!(
            file_change(&result, "docs.txt").classification,
            EvidenceClassification::Unsupported
        );
        assert!(
            file_change(&result, "docs.txt")
                .hunks
                .iter()
                .all(|hunk| hunk.base_symbols.is_empty() && hunk.head_symbols.is_empty())
        );
        assert_eq!(
            file_change(&result, "broken.rs").classification,
            EvidenceClassification::ParserUncertain
        );
        assert!(
            file_change(&result, "broken.rs")
                .hunks
                .iter()
                .all(|hunk| hunk.base_symbols.is_empty() && hunk.head_symbols.is_empty())
        );
        assert_eq!(file_change(&result, "new.rs").kind, GitChangeKind::Added);
        let added_hunk = file_change(&result, "new.rs")
            .hunks
            .first()
            .expect("addition should retain its head-side hunk");
        assert!(added_hunk.base_anchor.is_none());
        assert!(added_hunk.head_anchor.is_some());
        assert_eq!(added_hunk.head_changed_ranges[0].start_line, 1);
        assert_eq!(added_hunk.head_changed_ranges[0].end_line, 1);
        assert_eq!(added_hunk.head_symbols.len(), 1);
        assert_eq!(added_hunk.head_symbols[0].name, "added");
    }

    #[test]
    fn short_oid_prefix_is_not_accepted_as_a_commit_id_and_full_ref_is_supported() {
        let temporary = TestDirectory::new();
        let repository = temporary.path();
        init_repository(repository);
        fs::write(repository.join("src.rs"), "fn value() {}\n").unwrap();
        let commit = commit_fixture(repository, "base");
        let short = &commit[..8];
        let result = compare_local_revisions(repository, short, "refs/heads/main");
        assert_eq!(result.status, GitComparisonStatus::ChooseAgain);
        assert_eq!(
            result.ref_problem.as_ref().unwrap().kind,
            RefProblemKind::Missing
        );

        let result = compare_local_revisions(repository, "refs/heads/main", "HEAD");
        assert_eq!(result.status, GitComparisonStatus::Ready);
        assert_eq!(result.base.as_ref().unwrap().object_id, commit);
        assert_eq!(result.head.as_ref().unwrap().object_id, commit);
        let branch_to_head = compare_local_revisions(repository, "refs/heads/main", "HEAD");
        let head_to_branch = compare_local_revisions(repository, "HEAD", "refs/heads/main");
        assert_ne!(branch_to_head.comparison_id, head_to_branch.comparison_id);
    }
}
