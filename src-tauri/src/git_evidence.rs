//! Bounded, read-only evidence for comparing two locally available Git commits.
//!
//! This module deliberately operates on immutable commit object IDs after ref
//! resolution. It does not checkout either tree, touch the index, or update refs.

use crate::project::git_command;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Output, Stdio};
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
    pub omitted_change_count: usize,
    pub omitted_reason: Option<String>,
    pub message: Option<String>,
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
    pub skipped_reason: Option<String>,
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
        omitted_change_count,
        omitted_reason: limit_reason.clone(),
        message: limit_reason,
    }
}

#[derive(Debug)]
struct RefResolutionError {
    kind: RefProblemKind,
    message: String,
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

    let candidates = match ref_candidates(requested) {
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

fn ref_candidates(requested: &str) -> Result<Vec<String>, String> {
    if requested.is_empty()
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
            || !valid_git_ref(requested)
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
    if candidates.iter().any(|candidate| !valid_git_ref(candidate)) {
        return Err("The ref name is not a valid local Git ref.".to_owned());
    }
    Ok(candidates.into_iter().collect())
}

fn valid_git_ref(reference: &str) -> bool {
    if reference.contains("//") || reference.ends_with(".lock") || reference.ends_with(".") {
        return false;
    }
    let output = git_command()
        .arg("check-ref-format")
        .arg(reference)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    output.is_ok_and(|status| status.success())
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
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        GitEvidenceError::Git(format!(
            "Git could not start for the local comparison: {error}"
        ))
    })?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| GitEvidenceError::Git("Git stdout was unavailable.".to_owned()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| GitEvidenceError::Git("Git stderr was unavailable.".to_owned()))?;
    let stdout_reader = thread::spawn(move || read_limited(stdout, max_stdout_bytes));
    let stderr_reader = thread::spawn(move || read_limited(stderr, MAX_GIT_STDERR_BYTES));

    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(10)),
            Ok(None) => {
                timed_out = true;
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(GitEvidenceError::Git(format!(
                    "Git process state could not be read: {error}"
                )));
            }
        }
    }
    let status = child.wait().map_err(|error| {
        GitEvidenceError::Git(format!("Git process could not be joined: {error}"))
    })?;
    let stdout = stdout_reader
        .join()
        .map_err(|_| GitEvidenceError::Git("Git output reader stopped unexpectedly.".to_owned()))?
        .map_err(|error| GitEvidenceError::Git(format!("Git output could not be read: {error}")))?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| GitEvidenceError::Git("Git error reader stopped unexpectedly.".to_owned()))?
        .map_err(|error| {
            GitEvidenceError::Git(format!("Git error output could not be read: {error}"))
        })?;
    if timed_out {
        return Err(GitEvidenceError::TimedOut(
            "The local Git comparison reached its 30-second limit; remaining evidence was omitted."
                .to_owned(),
        ));
    }
    if stdout.len() > max_stdout_bytes || stderr.len() > MAX_GIT_STDERR_BYTES {
        return Err(GitEvidenceError::OutputLimit(
            "Git output exceeded a local evidence size limit; remaining evidence was omitted."
                .to_owned(),
        ));
    }
    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

fn read_limited<R: Read>(reader: R, limit: usize) -> io::Result<Vec<u8>> {
    let mut output = Vec::with_capacity(limit.min(64 * 1024));
    reader
        .take(limit.saturating_add(1) as u64)
        .read_to_end(&mut output)?;
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
    fn message(self) -> String {
        match self {
            Self::Git(message)
            | Self::OutputLimit(message)
            | Self::TimedOut(message)
            | Self::InvalidOutput(message) => message,
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
