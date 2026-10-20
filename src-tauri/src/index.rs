use crate::snapshot::{self, CapturedContentKind, CapturedFile, SnapshotManifest, SnapshotSource};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tree_sitter::{Language, Node, ParseOptions, Parser, Point};
use uuid::Uuid;

const INDEX_SCHEMA_VERSION: u32 = 1;
const INDEXER_VERSION: &str = "atlas-symbol-index-v4";
const TREE_SITTER_RUNTIME_VERSION: &str = "0.27.1";
pub(crate) const MAX_INDEXED_FILES: usize = 5_000;
pub(crate) const MAX_SOURCE_BYTES_PER_FILE: u64 = 1024 * 1024;
pub(crate) const MAX_TOTAL_SOURCE_BYTES: u64 = 50 * 1024 * 1024;
const MAX_AST_NODES_PER_FILE: usize = 250_000;
const MAX_SCOPE_DEPTH: usize = 128;
const MAX_INDEXED_SYMBOLS: usize = 50_000;
const MAX_INDEXED_RELATIONSHIPS: usize = 100_000;
const MAX_INDEXED_IMPORT_BINDINGS: usize = 50_000;
const MAX_IMPORT_BINDINGS_PER_FILE: usize = 10_000;
const MAX_INDEX_JSON_BYTES: usize = 32 * 1024 * 1024;
const MAX_PARSE_TIME: Duration = Duration::from_millis(1_500);
const MAX_RETAINED_JOBS: usize = 32;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SnapshotIndexStatus {
    Complete,
    Partial,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IndexFileStatus {
    Indexed,
    PartialParse,
    ParseError,
    UnsupportedLanguage,
    Binary,
    InvalidEncoding,
    SourceUnavailable,
    FileTooLarge,
    IndexLimitExceeded,
    AnalysisLimitExceeded,
    ParserUnavailable,
    Cancelled,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IndexJobStatus {
    Running,
    Complete,
    Partial,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SymbolKind {
    Function,
    Method,
    Class,
    Struct,
    Enum,
    Trait,
    Interface,
    TypeAlias,
    Module,
    Constant,
    Variable,
    Macro,
}

impl SymbolKind {
    fn is_callable(&self) -> bool {
        matches!(self, Self::Function | Self::Method)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationshipKind {
    Defines,
    Exports,
    Imports,
    Calls,
    TestReference,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RelationshipConfidence {
    High,
    Unresolved,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ParserVersion {
    pub engine: String,
    pub engine_version: String,
    pub engine_language_abi: usize,
    pub grammar: String,
    pub grammar_version: String,
    pub grammar_abi: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceAnchor {
    pub path: String,
    pub source_sha256: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
    /// One-based UTF-8 byte columns, matching Tree-sitter's byte-oriented points.
    pub start_column: usize,
    /// One-based UTF-8 byte columns, matching Tree-sitter's byte-oriented points.
    pub end_column: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedFile {
    pub path: String,
    pub file_id: String,
    pub language: Option<String>,
    pub status: IndexFileStatus,
    pub parser: Option<ParserVersion>,
    pub source_sha256: String,
    pub source_size_bytes: u64,
    pub parse_error_count: usize,
    pub symbol_count: usize,
    pub relationship_count: usize,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedSymbol {
    pub symbol_id: String,
    pub path: String,
    pub language: String,
    pub kind: SymbolKind,
    pub name: String,
    pub qualified_name: String,
    pub declaration_identity: String,
    pub signature_sha256: String,
    pub is_exported: bool,
    pub is_test: bool,
    pub anchor: SourceAnchor,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexedRelationship {
    pub relationship_id: String,
    pub kind: RelationshipKind,
    pub source_id: String,
    pub target_id: Option<String>,
    pub target_path: Option<String>,
    pub target_label: Option<String>,
    pub confidence: RelationshipConfidence,
    pub unresolved_reason: Option<String>,
    pub evidence: SourceAnchor,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotIndex {
    pub schema_version: u32,
    pub indexer_version: String,
    pub project_id: String,
    pub snapshot_id: String,
    pub status: SnapshotIndexStatus,
    pub parsers: Vec<ParserVersion>,
    pub files: Vec<IndexedFile>,
    pub symbols: Vec<IndexedSymbol>,
    pub relationships: Vec<IndexedRelationship>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexProgress {
    pub job_id: String,
    pub project_id: String,
    pub snapshot_id: String,
    pub status: IndexJobStatus,
    pub files_total: usize,
    pub files_processed: usize,
    pub files_indexed: usize,
    pub symbols_found: usize,
    pub relationships_found: usize,
    pub current_path: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStart {
    pub job_id: String,
    pub project_id: String,
    pub snapshot_id: String,
    pub status: IndexJobStatus,
}

#[derive(Clone, Debug)]
struct WalkContext {
    scopes: Arc<Vec<String>>,
    owner_symbol_id: Option<Arc<str>>,
    owner_is_test: bool,
}

impl Default for WalkContext {
    fn default() -> Self {
        Self {
            scopes: Arc::new(Vec::new()),
            owner_symbol_id: None,
            owner_is_test: false,
        }
    }
}

struct CallCandidate {
    path: String,
    source_id: String,
    binding: Option<BindingTarget>,
    rust_self_method: Option<(String, String)>,
    is_test: bool,
    name: String,
    direct_identifier: bool,
    unresolved_reason: Option<String>,
    anchor: SourceAnchor,
}

struct ImportCandidate {
    path: String,
    module_specifier: Option<String>,
    unresolved_reason: Option<String>,
    bindings: Vec<ImportBinding>,
    anchor: SourceAnchor,
    is_require_call: bool,
}

struct ImportBinding {
    local_name: String,
    imported_name: String,
    is_value_binding: bool,
}

#[derive(Clone)]
enum BindingTarget {
    Symbol(String),
    Import,
    Unknown(&'static str),
}

struct LexicalBinding {
    target: BindingTarget,
    available_from: usize,
}

struct LexicalScope {
    parent: Option<usize>,
    function_scope: usize,
    bindings: HashMap<String, Vec<LexicalBinding>>,
    opaque: bool,
}

struct ParsedFile {
    file: IndexedFile,
    symbols: Vec<IndexedSymbol>,
    calls: Vec<CallCandidate>,
    imports: Vec<ImportCandidate>,
    callable_exports: BTreeSet<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SupportedLanguage {
    Rust,
    TypeScript,
    Tsx,
    JavaScript,
}

impl SupportedLanguage {
    fn label(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::JavaScript => "javascript",
        }
    }

    fn grammar_name(self) -> &'static str {
        match self {
            Self::Rust => "tree-sitter-rust",
            Self::TypeScript | Self::Tsx => "tree-sitter-typescript",
            Self::JavaScript => "tree-sitter-javascript",
        }
    }

    fn grammar_version(self) -> &'static str {
        match self {
            Self::Rust => "0.24.2",
            Self::TypeScript | Self::Tsx => "0.23.2",
            Self::JavaScript => "0.25.0",
        }
    }

    fn language(self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            Self::Tsx => tree_sitter_typescript::LANGUAGE_TSX.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        }
    }

    fn from_path(path: &str) -> Option<Self> {
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
}

#[derive(Clone, Default)]
pub struct SnapshotIndexJobs(Arc<Mutex<HashMap<String, JobRecord>>>);

#[derive(Clone)]
struct JobRecord {
    progress: IndexProgress,
    cancellation: Arc<AtomicBool>,
}

impl SnapshotIndexJobs {
    pub fn begin(
        &self,
        project_id: &str,
        snapshot_id: &str,
        files_total: usize,
    ) -> Result<(IndexProgress, Arc<AtomicBool>), String> {
        let mut jobs = self
            .0
            .lock()
            .map_err(|_| "The parser job store is temporarily unavailable.".to_owned())?;
        if jobs.values().any(|job| {
            job.progress.project_id == project_id
                && job.progress.snapshot_id == snapshot_id
                && job.progress.status == IndexJobStatus::Running
        }) {
            return Err("This snapshot is already being indexed.".to_owned());
        }
        while jobs.len() >= MAX_RETAINED_JOBS {
            let removable = jobs
                .iter()
                .find(|(_, job)| job.progress.status != IndexJobStatus::Running)
                .map(|(id, _)| id.clone());
            if let Some(id) = removable {
                jobs.remove(&id);
            } else {
                return Err(
                    "Too many snapshot indexes are running. Wait for one to finish.".to_owned(),
                );
            }
        }
        let job_id = Uuid::new_v4().to_string();
        let cancellation = Arc::new(AtomicBool::new(false));
        let progress = IndexProgress {
            job_id: job_id.clone(),
            project_id: project_id.to_owned(),
            snapshot_id: snapshot_id.to_owned(),
            status: IndexJobStatus::Running,
            files_total,
            files_processed: 0,
            files_indexed: 0,
            symbols_found: 0,
            relationships_found: 0,
            current_path: None,
            message: None,
        };
        jobs.insert(
            job_id,
            JobRecord {
                progress: progress.clone(),
                cancellation: Arc::clone(&cancellation),
            },
        );
        Ok((progress, cancellation))
    }

    pub fn update(&self, progress: IndexProgress) {
        if let Ok(mut jobs) = self.0.lock() {
            if let Some(job) = jobs.get_mut(&progress.job_id) {
                job.progress = progress;
            }
        }
    }

    pub fn get(&self, job_id: &str) -> Option<IndexProgress> {
        self.0
            .lock()
            .ok()?
            .get(job_id)
            .map(|job| job.progress.clone())
    }

    pub fn cancel(&self, job_id: &str) -> Result<(), String> {
        let jobs = self
            .0
            .lock()
            .map_err(|_| "The parser job store is temporarily unavailable.".to_owned())?;
        let job = jobs
            .get(job_id)
            .ok_or_else(|| "This indexing job is no longer available.".to_owned())?;
        if job.progress.status != IndexJobStatus::Running {
            return Err("This indexing job has already finished.".to_owned());
        }
        job.cancellation.store(true, Ordering::Relaxed);
        Ok(())
    }
}

pub fn build_snapshot_index<F>(
    storage_dir: &Path,
    manifest: &SnapshotManifest,
    job_id: &str,
    cancellation: &AtomicBool,
    on_progress: F,
) -> SnapshotIndex
where
    F: FnMut(&IndexProgress),
{
    build_snapshot_index_with_reader(
        manifest,
        job_id,
        cancellation,
        &BTreeSet::new(),
        on_progress,
        |captured| {
            snapshot::read_snapshot_source_from_manifest(storage_dir, manifest, &captured.path)
        },
    )
}

pub(crate) fn build_snapshot_index_from_sources<F>(
    manifest: &SnapshotManifest,
    sources: &HashMap<String, SnapshotSource>,
    priority_paths: &BTreeSet<String>,
    job_id: &str,
    cancellation: &AtomicBool,
    on_progress: F,
) -> SnapshotIndex
where
    F: FnMut(&IndexProgress),
{
    build_snapshot_index_with_reader(
        manifest,
        job_id,
        cancellation,
        priority_paths,
        on_progress,
        |captured| {
            sources.get(&captured.path).cloned().ok_or_else(|| {
                "The immutable Git revision source was omitted by its scan limit.".to_owned()
            })
        },
    )
}

fn build_snapshot_index_with_reader<R, F>(
    manifest: &SnapshotManifest,
    job_id: &str,
    cancellation: &AtomicBool,
    priority_paths: &BTreeSet<String>,
    mut on_progress: F,
    mut read_source: R,
) -> SnapshotIndex
where
    F: FnMut(&IndexProgress),
    R: FnMut(&CapturedFile) -> Result<SnapshotSource, String>,
{
    let mut progress = IndexProgress {
        job_id: job_id.to_owned(),
        project_id: manifest.summary.project_id.clone(),
        snapshot_id: manifest.summary.snapshot_id.clone(),
        status: IndexJobStatus::Running,
        files_total: manifest.files.len(),
        files_processed: 0,
        files_indexed: 0,
        symbols_found: 0,
        relationships_found: 0,
        current_path: None,
        message: None,
    };
    let supported_parsers = parser_catalog();
    let files_by_path = manifest
        .files
        .iter()
        .map(|file| file.path.clone())
        .collect::<BTreeSet<_>>();
    let mut files = Vec::with_capacity(manifest.files.len());
    let mut symbols = BTreeMap::<String, IndexedSymbol>::new();
    let mut call_candidates = Vec::new();
    let mut import_candidates = Vec::new();
    let mut callable_exports = BTreeSet::new();
    let mut total_source_bytes = 0_u64;
    let mut total_import_bindings = 0_usize;
    let mut partial = false;
    let mut hit_global_limit = false;
    let mut ordered_files = manifest.files.iter().collect::<Vec<_>>();
    ordered_files.sort_by(|left, right| {
        priority_paths
            .contains(&right.path)
            .cmp(&priority_paths.contains(&left.path))
            .then_with(|| left.path.cmp(&right.path))
    });

    for (file_index, captured) in ordered_files.iter().enumerate() {
        progress.current_path = Some(captured.path.clone());
        if cancellation.load(Ordering::Relaxed) {
            progress.status = IndexJobStatus::Cancelled;
            progress.message =
                Some("Indexing was cancelled. No partial index was saved.".to_owned());
            on_progress(&progress);
            return snapshot_index(
                manifest,
                SnapshotIndexStatus::Cancelled,
                supported_parsers,
                files,
                symbols.into_values().collect(),
                Vec::new(),
            );
        }

        if file_index >= MAX_INDEXED_FILES || hit_global_limit {
            partial = true;
            files.push(unprocessed_file(
                captured,
                IndexFileStatus::IndexLimitExceeded,
                "The snapshot index reached its 5,000-file or 50 MB source limit. The file remains available in the repository inventory.",
            ));
            progress.files_processed += 1;
            continue;
        }
        if captured.size_bytes > MAX_SOURCE_BYTES_PER_FILE {
            partial = true;
            files.push(unprocessed_file(
                captured,
                IndexFileStatus::FileTooLarge,
                "This file exceeds the 1 MiB per-file parser limit. Source browsing remains available.",
            ));
            progress.files_processed += 1;
            update_progress_counts(
                &mut progress,
                &files,
                &symbols,
                &call_candidates,
                &import_candidates,
            );
            on_progress(&progress);
            continue;
        }
        match captured.content_kind {
            CapturedContentKind::Binary => {
                partial = true;
                files.push(unprocessed_file(
                    captured,
                    IndexFileStatus::Binary,
                    "Binary content is not parsed.",
                ));
                progress.files_processed += 1;
                on_progress(&progress);
                continue;
            }
            CapturedContentKind::InvalidEncoding => {
                partial = true;
                files.push(unprocessed_file(
                    captured,
                    IndexFileStatus::InvalidEncoding,
                    "Invalid UTF-8 content is not parsed.",
                ));
                progress.files_processed += 1;
                on_progress(&progress);
                continue;
            }
            CapturedContentKind::Text | CapturedContentKind::Unknown => {}
        }

        let Some(language) = SupportedLanguage::from_path(&captured.path) else {
            partial = true;
            files.push(unprocessed_file(
                captured,
                IndexFileStatus::UnsupportedLanguage,
                "No parser is shipped for this file extension. The file remains available in the repository inventory.",
            ));
            progress.files_processed += 1;
            on_progress(&progress);
            continue;
        };
        if total_source_bytes.saturating_add(captured.size_bytes) > MAX_TOTAL_SOURCE_BYTES {
            partial = true;
            hit_global_limit = true;
            files.push(unprocessed_file(
                captured,
                IndexFileStatus::IndexLimitExceeded,
                "The snapshot index reached its 50 MB total source limit. Remaining files stay available in the repository inventory.",
            ));
            progress.files_processed += 1;
            on_progress(&progress);
            continue;
        }
        let source = match read_source(captured) {
            Ok(source) => source,
            Err(_) => {
                partial = true;
                files.push(unprocessed_file(
                    captured,
                    IndexFileStatus::SourceUnavailable,
                    "The captured source blob is unavailable or failed its integrity check. Re-capture this repository to rebuild the index.",
                ));
                progress.files_processed += 1;
                on_progress(&progress);
                continue;
            }
        };
        total_source_bytes = total_source_bytes.saturating_add(source.size_bytes);
        let parsed = parse_file(
            &source.path,
            &source.sha256,
            &source.contents,
            language,
            cancellation,
        );
        match parsed {
            Err(ParseFailure::Cancelled) => {
                progress.status = IndexJobStatus::Cancelled;
                progress.message =
                    Some("Indexing was cancelled. No partial index was saved.".to_owned());
                on_progress(&progress);
                return snapshot_index(
                    manifest,
                    SnapshotIndexStatus::Cancelled,
                    supported_parsers,
                    files,
                    symbols.into_values().collect(),
                    Vec::new(),
                );
            }
            Err(ParseFailure::ParserUnavailable(message)) => {
                partial = true;
                files.push(unprocessed_file(
                    captured,
                    IndexFileStatus::ParserUnavailable,
                    &message,
                ));
            }
            Err(ParseFailure::AnalysisLimit) => {
                partial = true;
                files.push(unprocessed_file(
                    captured,
                    IndexFileStatus::AnalysisLimitExceeded,
                    "The parser exceeded its per-file time or syntax-node limit. The file remains available in the repository inventory.",
                ));
            }
            Ok(parsed) => {
                partial |= matches!(
                    parsed.file.status,
                    IndexFileStatus::PartialParse | IndexFileStatus::ParseError
                );
                let parsed_import_binding_count = parsed
                    .imports
                    .iter()
                    .map(|candidate| candidate.bindings.len())
                    .sum::<usize>();
                if symbols.len() + parsed.symbols.len() > MAX_INDEXED_SYMBOLS
                    || call_candidates.len()
                        + import_candidates.len()
                        + parsed.calls.len()
                        + parsed.imports.len()
                        > MAX_INDEXED_RELATIONSHIPS
                    || total_import_bindings.saturating_add(parsed_import_binding_count)
                        > MAX_INDEXED_IMPORT_BINDINGS
                {
                    partial = true;
                    hit_global_limit = true;
                    files.push(unprocessed_file(
                        captured,
                        IndexFileStatus::IndexLimitExceeded,
                        "The snapshot index reached its symbol or relationship limit. The file remains available in the repository inventory.",
                    ));
                } else {
                    total_import_bindings += parsed_import_binding_count;
                    for symbol in parsed.symbols {
                        symbols.entry(symbol.symbol_id.clone()).or_insert(symbol);
                    }
                    call_candidates.extend(parsed.calls);
                    import_candidates.extend(parsed.imports);
                    callable_exports.extend(parsed.callable_exports);
                    files.push(parsed.file);
                }
            }
        }
        progress.files_processed += 1;
        update_progress_counts(
            &mut progress,
            &files,
            &symbols,
            &call_candidates,
            &import_candidates,
        );
        on_progress(&progress);
    }

    let mut relationships = build_definition_relationships(&symbols);
    let (import_relationships, imported_call_targets) = resolve_imports(
        import_candidates,
        &files_by_path,
        &files,
        &symbols,
        &callable_exports,
    );
    relationships.extend(import_relationships);
    relationships.extend(resolve_calls(
        call_candidates,
        &symbols,
        &imported_call_targets,
    ));
    relationships.sort_by(relationship_order);
    relationships.dedup_by(|left, right| left.relationship_id == right.relationship_id);
    if relationships.len() > MAX_INDEXED_RELATIONSHIPS {
        let omitted = relationships.split_off(MAX_INDEXED_RELATIONSHIPS);
        let omitted_paths = omitted
            .into_iter()
            .map(|edge| edge.evidence.path)
            .collect::<BTreeSet<_>>();
        for file in files
            .iter_mut()
            .filter(|file| omitted_paths.contains(&file.path))
        {
            file.status = IndexFileStatus::IndexLimitExceeded;
            file.message = Some(
                "The index reached its 100,000-relationship limit. Some edges anchored in this file were omitted; all captured source remains browseable."
                    .to_owned(),
            );
        }
        partial = true;
    }
    let mut relationships_by_path = HashMap::<String, usize>::new();
    for edge in &relationships {
        *relationships_by_path
            .entry(edge.evidence.path.clone())
            .or_default() += 1;
    }
    for file in &mut files {
        file.relationship_count = relationships_by_path
            .get(&file.path)
            .copied()
            .unwrap_or_default();
    }
    if files
        .iter()
        .any(|file| file.status != IndexFileStatus::Indexed)
    {
        partial = true;
    }
    progress.status = if partial {
        IndexJobStatus::Partial
    } else {
        IndexJobStatus::Complete
    };
    progress.current_path = None;
    progress.files_indexed = files
        .iter()
        .filter(|file| {
            matches!(
                file.status,
                IndexFileStatus::Indexed | IndexFileStatus::PartialParse
            )
        })
        .count();
    progress.symbols_found = symbols.len();
    progress.relationships_found = relationships.len();
    progress.message = Some(if partial {
        "Index complete with unsupported files, parse gaps, or bounded work omitted. Unresolved edges remain listed with their reasons.".to_owned()
    } else {
        "Index complete for every captured file supported by the shipped parsers.".to_owned()
    });
    on_progress(&progress);
    if cancellation.load(Ordering::Relaxed) {
        progress.status = IndexJobStatus::Cancelled;
        progress.message = Some("Indexing was cancelled. No partial index was saved.".to_owned());
        on_progress(&progress);
        return snapshot_index(
            manifest,
            SnapshotIndexStatus::Cancelled,
            supported_parsers,
            files,
            symbols.into_values().collect(),
            Vec::new(),
        );
    }
    snapshot_index(
        manifest,
        if partial {
            SnapshotIndexStatus::Partial
        } else {
            SnapshotIndexStatus::Complete
        },
        supported_parsers,
        files,
        symbols.into_values().collect(),
        relationships,
    )
}

fn parser_catalog() -> Vec<ParserVersion> {
    [
        SupportedLanguage::JavaScript,
        SupportedLanguage::Rust,
        SupportedLanguage::Tsx,
        SupportedLanguage::TypeScript,
    ]
    .into_iter()
    .map(parser_version)
    .collect()
}

fn parser_version(language: SupportedLanguage) -> ParserVersion {
    let grammar = language.language();
    let grammar_version = grammar
        .metadata()
        .map(|metadata| {
            format!(
                "{}.{}.{}",
                metadata.major_version, metadata.minor_version, metadata.patch_version
            )
        })
        .unwrap_or_else(|| language.grammar_version().to_owned());
    ParserVersion {
        engine: "tree-sitter".to_owned(),
        engine_version: TREE_SITTER_RUNTIME_VERSION.to_owned(),
        engine_language_abi: tree_sitter::LANGUAGE_VERSION,
        grammar: language.grammar_name().to_owned(),
        grammar_version,
        grammar_abi: grammar.abi_version(),
    }
}

fn snapshot_index(
    manifest: &SnapshotManifest,
    status: SnapshotIndexStatus,
    parsers: Vec<ParserVersion>,
    mut files: Vec<IndexedFile>,
    mut symbols: Vec<IndexedSymbol>,
    mut relationships: Vec<IndexedRelationship>,
) -> SnapshotIndex {
    files.sort_by(|left, right| left.path.cmp(&right.path));
    symbols.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then(left.qualified_name.cmp(&right.qualified_name))
            .then(symbol_kind_label(&left.kind).cmp(symbol_kind_label(&right.kind)))
            .then(left.symbol_id.cmp(&right.symbol_id))
    });
    relationships.sort_by(relationship_order);
    SnapshotIndex {
        schema_version: INDEX_SCHEMA_VERSION,
        indexer_version: INDEXER_VERSION.to_owned(),
        project_id: manifest.summary.project_id.clone(),
        snapshot_id: manifest.summary.snapshot_id.clone(),
        status,
        parsers,
        files,
        symbols,
        relationships,
    }
}

fn unprocessed_file(
    captured: &snapshot::CapturedFile,
    status: IndexFileStatus,
    message: &str,
) -> IndexedFile {
    IndexedFile {
        path: captured.path.clone(),
        file_id: file_id(&captured.path),
        language: SupportedLanguage::from_path(&captured.path)
            .map(|language| language.label().to_owned()),
        status,
        parser: None,
        source_sha256: captured.sha256.clone(),
        source_size_bytes: captured.size_bytes,
        parse_error_count: 0,
        symbol_count: 0,
        relationship_count: 0,
        message: Some(message.to_owned()),
    }
}

#[derive(Debug)]
enum ParseFailure {
    Cancelled,
    ParserUnavailable(String),
    AnalysisLimit,
}

fn parse_file(
    path: &str,
    source_sha256: &str,
    source: &str,
    language: SupportedLanguage,
    cancellation: &AtomicBool,
) -> Result<ParsedFile, ParseFailure> {
    let language_object = language.language();
    let mut parser = Parser::new();
    parser.set_language(&language_object).map_err(|_| {
        ParseFailure::ParserUnavailable(
            "The bundled parser grammar is incompatible with this Tree-sitter runtime.".to_owned(),
        )
    })?;
    let started = Instant::now();
    let mut user_cancelled = false;
    let mut exceeded_time = false;
    let mut progress_callback = |_state: &tree_sitter::ParseState| {
        if cancellation.load(Ordering::Relaxed) {
            user_cancelled = true;
            ControlFlow::Break(())
        } else if started.elapsed() > MAX_PARSE_TIME {
            exceeded_time = true;
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let options = ParseOptions::new().progress_callback(&mut progress_callback);
    let source_bytes = source.as_bytes();
    let mut read_chunk =
        |offset: usize, _position: Point| source_bytes.get(offset..).unwrap_or(&[]);
    let tree = parser.parse_with_options(&mut read_chunk, None, Some(options));
    let Some(tree) = tree else {
        return if user_cancelled || cancellation.load(Ordering::Relaxed) {
            Err(ParseFailure::Cancelled)
        } else if exceeded_time {
            Err(ParseFailure::AnalysisLimit)
        } else {
            Err(ParseFailure::ParserUnavailable(
                "The parser could not produce a syntax tree.".to_owned(),
            ))
        };
    };
    if cancellation.load(Ordering::Relaxed) {
        return Err(ParseFailure::Cancelled);
    }
    collect_file_index(path, source_sha256, source, language, tree, cancellation)
}

fn collect_file_index(
    path: &str,
    source_sha256: &str,
    source: &str,
    language: SupportedLanguage,
    tree: tree_sitter::Tree,
    cancellation: &AtomicBool,
) -> Result<ParsedFile, ParseFailure> {
    let root = tree.root_node();
    let parse_error_count = count_parse_errors(root, cancellation)?;
    let file_id = file_id(path);
    let parser = parser_version(language);
    let file_is_test = is_test_path(path);
    let mut symbols = Vec::new();
    let mut declaration_occurrences = HashMap::<String, usize>::new();
    let mut calls = Vec::new();
    let mut imports = Vec::new();
    let mut import_binding_count = 0_usize;
    let mut stack = vec![(root, WalkContext::default(), 0_usize)];
    let mut visited = 0_usize;
    let mut analysis_limited = false;

    while let Some((node, context, depth)) = stack.pop() {
        if cancellation.load(Ordering::Relaxed) {
            return Err(ParseFailure::Cancelled);
        }
        visited += 1;
        if visited > MAX_AST_NODES_PER_FILE || depth > MAX_SCOPE_DEPTH {
            analysis_limited = true;
            break;
        }
        if let Some((kind, name_node)) = declaration(&node, language, &context) {
            let name = node_text(name_node, source);
            if !name.is_empty() && name.len() <= 512 {
                let exported = direct_module_export(node, language);
                let is_test = kind.is_callable()
                    && (file_is_test
                        || (language == SupportedLanguage::Rust
                            && rust_test_attribute(&node, source)));
                let qualified_name = qualify(&context.scopes, &name);
                let signature = signature_bytes(&node, name_node, source, &kind);
                let signature_sha256 = digest(signature.as_bytes());
                let declaration_identity = format!(
                    "{}:{}:{}:{}:{}",
                    language.label(),
                    path,
                    qualified_name,
                    symbol_kind_label(&kind),
                    signature_sha256
                );
                let occurrence = declaration_occurrences
                    .entry(declaration_identity.clone())
                    .or_default();
                *occurrence += 1;
                // Keep the first declaration's stable identity while retaining
                // repeated occurrences instead of silently collapsing ambiguity.
                let occurrence_identity = if *occurrence == 1 {
                    declaration_identity.clone()
                } else {
                    format!("{declaration_identity}:occurrence:{occurrence}")
                };
                let symbol_id = format!("sym:{}", digest(occurrence_identity.as_bytes()));
                let indexed_symbol = IndexedSymbol {
                    symbol_id: symbol_id.clone(),
                    path: path.to_owned(),
                    language: language.label().to_owned(),
                    kind: kind.clone(),
                    name: name.clone(),
                    qualified_name,
                    declaration_identity,
                    signature_sha256,
                    is_exported: exported,
                    is_test,
                    anchor: anchor(path, source_sha256, node),
                };
                symbols.push(indexed_symbol);

                let child_scope = opens_symbol_scope(&kind, language);
                if child_scope {
                    let mut scopes = context.scopes.as_ref().clone();
                    scopes.push(name);
                    let mut child_context = context.clone();
                    child_context.scopes = Arc::new(scopes);
                    if kind.is_callable() {
                        child_context.owner_symbol_id = Some(Arc::from(symbol_id));
                        child_context.owner_is_test = is_test;
                    }
                    push_children(node, child_context, depth, &mut stack);
                    continue;
                }
            }
        }

        if node.kind() == "impl_item" {
            if let Some(implementation_type) = node.child_by_field_name("type") {
                let mut scopes = context.scopes.as_ref().clone();
                scopes.push(format!("impl:{}", node_text(implementation_type, source)));
                let mut child_context = context.clone();
                child_context.scopes = Arc::new(scopes);
                push_children(node, child_context, depth, &mut stack);
                continue;
            }
        }

        if is_import_node(node, language) {
            if let Some(candidate) = import_candidate(node, path, source, source_sha256, language) {
                import_binding_count += candidate.bindings.len();
                if import_binding_count > MAX_IMPORT_BINDINGS_PER_FILE {
                    analysis_limited = true;
                    break;
                }
                imports.push(candidate);
            }
        }
        if node.kind() == "call_expression" {
            if language == SupportedLanguage::Rust || !is_static_import_call(node, source) {
                calls.push(call_candidate(
                    node,
                    path,
                    source,
                    source_sha256,
                    &context,
                    file_is_test,
                    language,
                    &file_id,
                ));
            } else if let Some(candidate) =
                import_candidate(node, path, source, source_sha256, language)
            {
                // `require` is an ordinary identifier and can be shadowed.
                // Keep its call until lexical binding analysis decides whether
                // this is the conventional module loader or a local function.
                if candidate.is_require_call {
                    calls.push(call_candidate(
                        node,
                        path,
                        source,
                        source_sha256,
                        &context,
                        file_is_test,
                        language,
                        &file_id,
                    ));
                }
                import_binding_count += candidate.bindings.len();
                if import_binding_count > MAX_IMPORT_BINDINGS_PER_FILE {
                    analysis_limited = true;
                    break;
                }
                imports.push(candidate);
            }
        }
        push_children(node, context, depth, &mut stack);
    }

    if analysis_limited {
        return Err(ParseFailure::AnalysisLimit);
    }
    let mut callable_exports =
        bind_calls_lexically(root, source, language, &symbols, &mut calls, cancellation)?;
    if parse_error_count > 0 {
        callable_exports.clear();
        for call in &mut calls {
            if call.direct_identifier {
                call.binding = Some(BindingTarget::Unknown("source_has_parse_errors"));
            }
        }
    }
    let require_calls = imports
        .iter()
        .filter(|import| import.is_require_call)
        .map(|import| import.anchor.start_byte)
        .collect::<BTreeSet<_>>();
    let shadowed_require_calls = calls
        .iter()
        .filter(|call| {
            require_calls.contains(&call.anchor.start_byte)
                && !matches!(
                    call.binding.as_ref(),
                    Some(BindingTarget::Unknown("no_visible_lexical_binding"))
                )
        })
        .map(|call| call.anchor.start_byte)
        .collect::<BTreeSet<_>>();
    imports.retain(|import| {
        !import.is_require_call || !shadowed_require_calls.contains(&import.anchor.start_byte)
    });
    calls.retain(|call| {
        !require_calls.contains(&call.anchor.start_byte)
            || shadowed_require_calls.contains(&call.anchor.start_byte)
    });
    let status = if parse_error_count > 0 {
        if symbols.is_empty() {
            IndexFileStatus::ParseError
        } else {
            IndexFileStatus::PartialParse
        }
    } else {
        IndexFileStatus::Indexed
    };
    let message = match status {
        IndexFileStatus::PartialParse => Some("The parser recovered from syntax errors. Indexed declarations and relationships may be incomplete.".to_owned()),
        IndexFileStatus::ParseError => Some("The parser found syntax errors and no usable declarations.".to_owned()),
        _ => None,
    };
    let symbol_count = symbols.len();
    Ok(ParsedFile {
        file: IndexedFile {
            path: path.to_owned(),
            file_id,
            language: Some(language.label().to_owned()),
            status,
            parser: Some(parser),
            source_sha256: source_sha256.to_owned(),
            source_size_bytes: source.len() as u64,
            parse_error_count,
            symbol_count,
            relationship_count: 0,
            message,
        },
        symbols,
        calls,
        imports,
        callable_exports,
    })
}

fn direct_module_export(node: Node<'_>, language: SupportedLanguage) -> bool {
    if language == SupportedLanguage::Rust {
        return node
            .parent()
            .is_some_and(|parent| parent.kind() == "source_file")
            && rust_public(&node);
    }
    let mut declaration = node;
    if node.kind() == "variable_declarator" {
        let Some(parent) = node.parent() else {
            return false;
        };
        declaration = parent;
    }
    let Some(export) = declaration.parent() else {
        return false;
    };
    matches!(export.kind(), "export_statement" | "export_declaration")
        && export
            .parent()
            .is_some_and(|parent| parent.kind() == "program")
        && {
            let mut cursor = export.walk();
            !export
                .children(&mut cursor)
                .any(|child| child.kind() == "default")
        }
}

fn lexical_function(kind: &str) -> bool {
    matches!(
        kind,
        "function_item"
            | "function_signature_item"
            | "function_declaration"
            | "function_signature"
            | "generator_function_declaration"
            | "function_expression"
            | "generator_function"
            | "arrow_function"
            | "method_definition"
            | "method_signature"
            | "abstract_method_signature"
            | "closure_expression"
    )
}

fn lexical_block(kind: &str) -> bool {
    matches!(
        kind,
        "block"
            | "statement_block"
            | "switch_body"
            | "catch_clause"
            | "for_statement"
            | "for_in_statement"
            | "for_expression"
            | "match_arm"
            | "if_expression"
            | "while_expression"
            | "class_declaration"
            | "abstract_class_declaration"
            | "class"
            | "impl_item"
            | "trait_item"
            | "mod_item"
            | "internal_module"
            | "module"
            | "with_statement"
    )
}

// Only binding positions are visited. Property keys, type names and default
// value expressions must never become variables in the enclosing scope.
fn pattern_binding_names(pattern: Node<'_>, source: &str) -> (Vec<String>, bool) {
    let mut names = BTreeSet::new();
    let mut stack = vec![pattern];
    let mut understood = true;
    while let Some(node) = stack.pop() {
        match node.kind() {
            "identifier"
            | "shorthand_property_identifier_pattern"
            | "shorthand_field_identifier" => {
                names.insert(node_text(node, source));
            }
            "required_parameter" | "optional_parameter" | "parameter" => {
                if let Some(binding) = node
                    .child_by_field_name("pattern")
                    .or_else(|| node.child_by_field_name("name"))
                {
                    stack.push(binding);
                } else {
                    understood = false;
                }
            }
            "assignment_pattern" | "object_assignment_pattern" => {
                if let Some(left) = node.child_by_field_name("left") {
                    stack.push(left);
                }
            }
            "pair_pattern" => {
                if let Some(value) = node.child_by_field_name("value") {
                    stack.push(value);
                }
            }
            "field_pattern" => {
                if let Some(binding) = node
                    .child_by_field_name("pattern")
                    .or_else(|| node.child_by_field_name("name"))
                {
                    if binding.kind() != "field_identifier" {
                        stack.push(binding);
                    }
                }
            }
            "formal_parameters"
            | "parameters"
            | "closure_parameters"
            | "array_pattern"
            | "object_pattern"
            | "rest_pattern"
            | "tuple_pattern"
            | "slice_pattern"
            | "or_pattern"
            | "reference_pattern"
            | "ref_pattern"
            | "mut_pattern"
            | "captured_pattern"
            | "struct_pattern"
            | "tuple_struct_pattern"
            | "match_pattern" => {
                let type_node = node.child_by_field_name("type");
                let condition = node.child_by_field_name("condition");
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    if type_node.is_some_and(|item| item.id() == child.id())
                        || condition.is_some_and(|item| item.id() == child.id())
                    {
                        continue;
                    }
                    stack.push(child);
                }
            }
            "self"
            | "self_parameter"
            | "this"
            | "_"
            | "remaining_field_pattern"
            | "mutable_specifier"
            | "lifetime"
            | "scoped_identifier"
            | "scoped_type_identifier"
            | "type_identifier"
            | "range_pattern"
            | "integer_literal"
            | "float_literal"
            | "string_literal"
            | "char_literal"
            | "boolean_literal"
            | "negative_literal" => {}
            _ => understood = false,
        }
    }
    (names.into_iter().collect(), understood)
}

fn add_pattern_bindings(
    scopes: &mut [LexicalScope],
    scope: usize,
    pattern: Node<'_>,
    source: &str,
    target: BindingTarget,
    available_from: usize,
) {
    let (names, understood) = pattern_binding_names(pattern, source);
    scopes[scope].opaque |= !understood;
    for name in names {
        scopes[scope]
            .bindings
            .entry(name)
            .or_default()
            .push(LexicalBinding {
                target: target.clone(),
                available_from,
            });
    }
}

fn binding_scope(scopes: &[LexicalScope], mut scope: usize, name: &str) -> Option<usize> {
    loop {
        if scopes[scope].bindings.contains_key(name) || scopes[scope].opaque {
            return Some(scope);
        }
        scope = scopes[scope].parent?;
    }
}

fn add_rust_import_bindings(scope: &mut LexicalScope, node: Node<'_>, source: &str) {
    let mut stack = vec![node];
    while let Some(node) = stack.pop() {
        let name = match node.kind() {
            "identifier" => Some(node),
            "use_as_clause" => node.child_by_field_name("alias"),
            "scoped_identifier" => node.child_by_field_name("name"),
            "use_declaration" => {
                if let Some(argument) = node.child_by_field_name("argument") {
                    stack.push(argument);
                }
                None
            }
            "scoped_use_list" => {
                if let Some(list) = node.child_by_field_name("list") {
                    stack.push(list);
                }
                None
            }
            "use_list" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    stack.push(child);
                }
                None
            }
            _ => {
                // Glob imports and context-dependent `self` imports cannot be
                // enumerated without resolving Rust's module graph.
                scope.opaque = true;
                None
            }
        };
        if let Some(name) = name {
            scope
                .bindings
                .entry(node_text(name, source))
                .or_default()
                .push(LexicalBinding {
                    target: BindingTarget::Unknown("rust_import_binding_not_resolved"),
                    available_from: 0,
                });
        }
    }
}

fn bind_calls_lexically(
    root: Node<'_>,
    source: &str,
    language: SupportedLanguage,
    symbols: &[IndexedSymbol],
    calls: &mut [CallCandidate],
    cancellation: &AtomicBool,
) -> Result<BTreeSet<String>, ParseFailure> {
    let symbols_by_span = symbols
        .iter()
        .map(|symbol| ((symbol.anchor.start_byte, symbol.anchor.end_byte), symbol))
        .collect::<HashMap<_, _>>();
    let mut scopes = vec![LexicalScope {
        parent: None,
        function_scope: 0,
        bindings: HashMap::new(),
        opaque: false,
    }];
    let mut call_scopes = HashMap::new();
    let mut assignments = Vec::new();
    let mut stack = vec![(root, 0_usize, 0_usize)];
    let mut visited = 0_usize;
    while let Some((node, inherited_scope, depth)) = stack.pop() {
        if cancellation.load(Ordering::Relaxed) {
            return Err(ParseFailure::Cancelled);
        }
        visited += 1;
        if visited > MAX_AST_NODES_PER_FILE || depth > MAX_SCOPE_DEPTH {
            return Err(ParseFailure::AnalysisLimit);
        }
        let kind = node.kind();
        let symbol = symbols_by_span
            .get(&(node.start_byte(), node.end_byte()))
            .copied();
        // Declarations bind outside their own body; methods do not introduce
        // bare lexical names, and named function/class expressions bind inside.
        if let Some(symbol) = symbol {
            if !matches!(kind, "variable_declarator" | "class") && symbol.kind != SymbolKind::Method
            {
                scopes[inherited_scope]
                    .bindings
                    .entry(symbol.name.clone())
                    .or_default()
                    .push(LexicalBinding {
                        target: if symbol.kind.is_callable() {
                            BindingTarget::Symbol(symbol.symbol_id.clone())
                        } else {
                            BindingTarget::Unknown("nearest_binding_is_not_a_static_callable")
                        },
                        available_from: 0,
                    });
            }
        }
        let function = lexical_function(kind);
        let mut scope = inherited_scope;
        if function || lexical_block(kind) {
            scope = scopes.len();
            let module = matches!(kind, "mod_item" | "internal_module" | "module");
            scopes.push(LexicalScope {
                // A Rust module has its own namespace; outer items require an
                // explicit use/super path, unlike a closure's lexical lookup.
                parent: if language == SupportedLanguage::Rust && kind == "mod_item" {
                    None
                } else {
                    Some(inherited_scope)
                },
                function_scope: if function || module {
                    scope
                } else {
                    scopes[inherited_scope].function_scope
                },
                bindings: HashMap::new(),
                opaque: kind == "with_statement",
            });
        }
        if function {
            if let Some(parameters) = node
                .child_by_field_name("parameters")
                .or_else(|| node.child_by_field_name("parameter"))
            {
                add_pattern_bindings(
                    &mut scopes,
                    scope,
                    parameters,
                    source,
                    BindingTarget::Unknown("call_is_shadowed_by_parameter"),
                    0,
                );
            }
            if matches!(kind, "function_expression" | "generator_function") {
                if let Some(name) = node.child_by_field_name("name") {
                    add_pattern_bindings(
                        &mut scopes,
                        scope,
                        name,
                        source,
                        BindingTarget::Unknown("named_function_expression_not_resolved"),
                        0,
                    );
                }
            }
        }
        if kind == "class" {
            if let Some(name) = node.child_by_field_name("name") {
                // TypeScript class names use type_identifier, but they occupy
                // the value namespace too.
                scopes[scope]
                    .bindings
                    .entry(node_text(name, source))
                    .or_default()
                    .push(LexicalBinding {
                        target: BindingTarget::Unknown("class_binding_is_not_a_direct_call"),
                        available_from: 0,
                    });
            }
        }
        match kind {
            "variable_declarator" => {
                if let Some(pattern) = node.child_by_field_name("name") {
                    let destination = if node
                        .parent()
                        .is_some_and(|parent| parent.kind() == "variable_declaration")
                    {
                        scopes[scope].function_scope
                    } else {
                        scope
                    };
                    let target = symbol
                        .filter(|symbol| symbol.kind.is_callable())
                        .map(|symbol| BindingTarget::Symbol(symbol.symbol_id.clone()))
                        .unwrap_or(BindingTarget::Unknown("call_is_shadowed_by_local_binding"));
                    add_pattern_bindings(
                        &mut scopes,
                        destination,
                        pattern,
                        source,
                        target,
                        node.end_byte(),
                    );
                }
            }
            "let_declaration" | "let_condition" | "for_expression" | "match_arm" => {
                if let Some(pattern) = node.child_by_field_name("pattern") {
                    add_pattern_bindings(
                        &mut scopes,
                        scope,
                        pattern,
                        source,
                        BindingTarget::Unknown("call_is_shadowed_by_pattern_binding"),
                        0,
                    );
                }
            }
            "catch_clause" => {
                if let Some(pattern) = node.child_by_field_name("parameter") {
                    add_pattern_bindings(
                        &mut scopes,
                        scope,
                        pattern,
                        source,
                        BindingTarget::Unknown("call_is_shadowed_by_catch_binding"),
                        0,
                    );
                }
            }
            "for_in_statement" => {
                if let Some(pattern) = node.child_by_field_name("left") {
                    if node.child_by_field_name("kind").is_none() {
                        let (names, _) = pattern_binding_names(pattern, source);
                        assignments.push((scope, names));
                    } else {
                        let destination = if node
                            .child_by_field_name("kind")
                            .is_some_and(|kind| kind.kind() == "var")
                        {
                            scopes[scope].function_scope
                        } else {
                            scope
                        };
                        add_pattern_bindings(
                            &mut scopes,
                            destination,
                            pattern,
                            source,
                            BindingTarget::Unknown("call_is_shadowed_by_loop_binding"),
                            0,
                        );
                    }
                }
            }
            "import_statement" => {
                for binding in import_bindings(node, source) {
                    scopes[scope]
                        .bindings
                        .entry(binding.local_name)
                        .or_default()
                        .push(LexicalBinding {
                            target: if binding.is_value_binding
                                && binding.imported_name != "default"
                                && binding.imported_name != "*"
                            {
                                BindingTarget::Import
                            } else {
                                BindingTarget::Unknown("import_binding_not_statically_resolved")
                            },
                            available_from: 0,
                        });
                }
            }
            "use_declaration" if language == SupportedLanguage::Rust => {
                // Rust use trees need crate/module resolution; until that is
                // available, do not let an unresolved import expose an outer
                // declaration with the same name as a high-confidence call.
                add_rust_import_bindings(&mut scopes[scope], node, source);
            }
            "assignment_expression"
            | "augmented_assignment_expression"
            | "compound_assignment_expr" => {
                if let Some(left) = node.child_by_field_name("left") {
                    let (names, _) = pattern_binding_names(left, source);
                    assignments.push((scope, names));
                }
            }
            "update_expression" => {
                if let Some(argument) = node.child_by_field_name("argument") {
                    let (names, _) = pattern_binding_names(argument, source);
                    assignments.push((scope, names));
                }
            }
            "call_expression" => {
                call_scopes.insert(node.start_byte(), scope);
                if language != SupportedLanguage::Rust
                    && node.child_by_field_name("function").is_some_and(|callee| {
                        callee.kind() == "identifier" && node_text(callee, source) == "eval"
                    })
                {
                    let function_scope = scopes[scope].function_scope;
                    scopes[function_scope].opaque = true;
                }
            }
            _ => {}
        }
        push_named_children_reverse(node, scope, depth + 1, &mut stack);
    }
    // An assignment anywhere in the lexical lifetime makes a static target
    // uncertain. This intentionally avoids pretending to perform data flow.
    for (scope, names) in assignments {
        for name in names {
            if let Some(destination) = binding_scope(&scopes, scope, &name) {
                if let Some(bindings) = scopes[destination].bindings.get_mut(&name) {
                    for binding in bindings {
                        binding.target = BindingTarget::Unknown("binding_is_reassigned");
                    }
                }
            }
        }
    }
    for call in calls.iter_mut().filter(|call| call.direct_identifier) {
        if cancellation.load(Ordering::Relaxed) {
            return Err(ParseFailure::Cancelled);
        }
        let Some(scope) = call_scopes.get(&call.anchor.start_byte).copied() else {
            call.binding = Some(BindingTarget::Unknown("lexical_scope_unavailable"));
            continue;
        };
        let Some(destination) = binding_scope(&scopes, scope, &call.name) else {
            call.binding = Some(BindingTarget::Unknown("no_visible_lexical_binding"));
            continue;
        };
        call.binding = Some(if scopes[destination].opaque {
            BindingTarget::Unknown("lexical_scope_contains_unresolved_bindings")
        } else {
            match scopes[destination]
                .bindings
                .get(&call.name)
                .map(Vec::as_slice)
            {
                Some([binding]) if call.anchor.start_byte >= binding.available_from => {
                    binding.target.clone()
                }
                Some([_]) => BindingTarget::Unknown("binding_not_initialized_at_call_site"),
                Some(_) => BindingTarget::Unknown("multiple_bindings_in_lexical_scope"),
                None => BindingTarget::Unknown("no_visible_lexical_binding"),
            }
        });
    }
    Ok(symbols
        .iter()
        .filter(|symbol| {
            symbol.is_exported
                && symbol.kind.is_callable()
                && !scopes[0].opaque
                && matches!(scopes[0].bindings.get(&symbol.name).map(Vec::as_slice),
                Some([LexicalBinding { target: BindingTarget::Symbol(target), .. }])
                    if target == &symbol.symbol_id)
        })
        .map(|symbol| symbol.symbol_id.clone())
        .collect())
}

fn push_children<'tree>(
    node: Node<'tree>,
    child_context: WalkContext,
    depth: usize,
    stack: &mut Vec<(Node<'tree>, WalkContext, usize)>,
) {
    let mut cursor = node.walk();
    if cursor.goto_last_child() {
        loop {
            stack.push((cursor.node(), child_context.clone(), depth + 1));
            if !cursor.goto_previous_sibling() {
                break;
            }
        }
    }
}

fn push_named_children_reverse<'tree>(
    node: Node<'tree>,
    scope: usize,
    depth: usize,
    stack: &mut Vec<(Node<'tree>, usize, usize)>,
) {
    let mut cursor = node.walk();
    if cursor.goto_last_child() {
        loop {
            let child = cursor.node();
            if child.is_named() {
                stack.push((child, scope, depth));
            }
            if !cursor.goto_previous_sibling() {
                break;
            }
        }
    }
}

fn declaration<'tree>(
    node: &Node<'tree>,
    language: SupportedLanguage,
    context: &WalkContext,
) -> Option<(SymbolKind, Node<'tree>)> {
    let kind = node.kind();
    let symbol_kind = match language {
        SupportedLanguage::Rust => match kind {
            "function_item" | "function_signature" | "function_signature_item" => {
                if context
                    .scopes
                    .last()
                    .is_some_and(|scope| scope.starts_with("impl:"))
                    || node
                        .parent()
                        .and_then(|parent| parent.parent())
                        .is_some_and(|parent| parent.kind() == "trait_item")
                {
                    SymbolKind::Method
                } else {
                    SymbolKind::Function
                }
            }
            "struct_item" => SymbolKind::Struct,
            "enum_item" => SymbolKind::Enum,
            "union_item" => SymbolKind::Struct,
            "trait_item" => SymbolKind::Trait,
            "type_item" => SymbolKind::TypeAlias,
            "mod_item" => SymbolKind::Module,
            "const_item" | "static_item" => SymbolKind::Constant,
            "macro_definition" => SymbolKind::Macro,
            _ => return None,
        },
        SupportedLanguage::TypeScript | SupportedLanguage::Tsx => match kind {
            "function_declaration" | "function_signature" | "generator_function_declaration" => {
                SymbolKind::Function
            }
            "method_definition" | "method_signature" | "abstract_method_signature" => {
                SymbolKind::Method
            }
            "class_declaration" | "abstract_class_declaration" => SymbolKind::Class,
            "interface_declaration" => SymbolKind::Interface,
            "type_alias_declaration" => SymbolKind::TypeAlias,
            "enum_declaration" => SymbolKind::Enum,
            "internal_module" | "module" => SymbolKind::Module,
            "variable_declarator" => {
                let parent_kind = node
                    .parent()
                    .map(|parent| parent.kind())
                    .unwrap_or_default();
                if !matches!(parent_kind, "lexical_declaration" | "variable_declaration") {
                    return None;
                }
                let value_kind = node.child_by_field_name("value").map(|value| value.kind());
                match value_kind {
                    Some("arrow_function" | "function_expression" | "generator_function") => {
                        SymbolKind::Function
                    }
                    _ => SymbolKind::Variable,
                }
            }
            _ => return None,
        },
        SupportedLanguage::JavaScript => match kind {
            "function_declaration" | "generator_function_declaration" => SymbolKind::Function,
            "method_definition" => SymbolKind::Method,
            "class_declaration" | "class" => SymbolKind::Class,
            "variable_declarator" => {
                let parent_kind = node
                    .parent()
                    .map(|parent| parent.kind())
                    .unwrap_or_default();
                if !matches!(parent_kind, "lexical_declaration" | "variable_declaration") {
                    return None;
                }
                let value_kind = node.child_by_field_name("value").map(|value| value.kind());
                match value_kind {
                    Some("arrow_function" | "function_expression" | "generator_function") => {
                        SymbolKind::Function
                    }
                    _ => SymbolKind::Variable,
                }
            }
            _ => return None,
        },
    };
    let name = node.child_by_field_name("name")?;
    if name.kind().contains("pattern") || name.kind() == "object_pattern" {
        return None;
    }
    Some((symbol_kind, name))
}

fn opens_symbol_scope(kind: &SymbolKind, language: SupportedLanguage) -> bool {
    kind.is_callable()
        || matches!(
            kind,
            SymbolKind::Class
                | SymbolKind::Struct
                | SymbolKind::Enum
                | SymbolKind::Trait
                | SymbolKind::Interface
                | SymbolKind::Module
        )
        || (language == SupportedLanguage::Rust && matches!(kind, SymbolKind::TypeAlias))
}

fn rust_public(node: &Node<'_>) -> bool {
    // Restricted visibility (`pub(crate)`, `pub(super)`, `pub(in path)`) is not
    // a crate-root export and must not become a high-confidence export edge.
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .any(|child| child.kind() == "visibility_modifier" && child.named_child_count() == 0)
}

fn rust_test_attribute(node: &Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    node.children(&mut cursor).any(|child| {
        child.kind() == "attribute_item" && {
            let text = node_text(child, source);
            text.contains("test")
        }
    })
}

fn is_test_path(path: &str) -> bool {
    let normalized = path.to_ascii_lowercase();
    let file = normalized.rsplit('/').next().unwrap_or(&normalized);
    normalized
        .split('/')
        .any(|part| part == "tests" || part == "__tests__")
        || file.contains(".test.")
        || file.contains(".spec.")
        || file.ends_with("_test.rs")
}

fn signature_bytes(node: &Node<'_>, name: Node<'_>, source: &str, kind: &SymbolKind) -> String {
    if !kind.is_callable() {
        return node_text(name, source);
    }
    let end = node
        .child_by_field_name("body")
        .map(|body| body.start_byte())
        .unwrap_or_else(|| node.end_byte());
    source
        .get(name.start_byte()..end)
        .map(str::to_owned)
        .unwrap_or_else(|| node_text(name, source))
}

fn qualify(scopes: &[String], name: &str) -> String {
    if scopes.is_empty() {
        name.to_owned()
    } else {
        format!("{}::{name}", scopes.join("::"))
    }
}

fn symbol_kind_label(kind: &SymbolKind) -> &'static str {
    match kind {
        SymbolKind::Function => "function",
        SymbolKind::Method => "method",
        SymbolKind::Class => "class",
        SymbolKind::Struct => "struct",
        SymbolKind::Enum => "enum",
        SymbolKind::Trait => "trait",
        SymbolKind::Interface => "interface",
        SymbolKind::TypeAlias => "typeAlias",
        SymbolKind::Module => "module",
        SymbolKind::Constant => "constant",
        SymbolKind::Variable => "variable",
        SymbolKind::Macro => "macro",
    }
}

fn node_text(node: Node<'_>, source: &str) -> String {
    source
        .get(node.start_byte()..node.end_byte())
        .unwrap_or_default()
        .to_owned()
}

fn count_parse_errors(root: Node<'_>, cancellation: &AtomicBool) -> Result<usize, ParseFailure> {
    let mut count = 0;
    let mut stack = vec![root];
    let mut visited = 0_usize;
    while let Some(node) = stack.pop() {
        if cancellation.load(Ordering::Relaxed) {
            return Err(ParseFailure::Cancelled);
        }
        visited += 1;
        if visited > MAX_AST_NODES_PER_FILE {
            return Err(ParseFailure::AnalysisLimit);
        }
        if node.kind() == "ERROR" || node.is_missing() {
            count += 1;
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            stack.push(child);
        }
    }
    Ok(count)
}

fn call_candidate(
    node: Node<'_>,
    path: &str,
    source: &str,
    source_sha256: &str,
    context: &WalkContext,
    file_is_test: bool,
    language: SupportedLanguage,
    file_id: &str,
) -> CallCandidate {
    let function = node.child_by_field_name("function");
    let function_kind = function.map(|function| function.kind()).unwrap_or_default();
    let rust_self_method = if language == SupportedLanguage::Rust {
        function.and_then(|function| {
            if function.kind() != "scoped_identifier" {
                return None;
            }
            let method_text = node_text(function, source);
            let method = method_text.strip_prefix("Self::")?;
            if method.is_empty() || method.contains(':') || method.contains('<') {
                return None;
            }
            let implementation = context
                .scopes
                .iter()
                .rposition(|scope| scope.starts_with("impl:"))?;
            let implementation_scope = context.scopes[..=implementation].join("::");
            Some((implementation_scope, method.to_owned()))
        })
    } else {
        None
    };
    let (label, direct_identifier, unresolved_reason) = match function {
        Some(function) if function.kind() == "identifier" => {
            (node_text(function, source), true, None)
        }
        Some(function)
            if language == SupportedLanguage::Rust && function.kind() == "scoped_identifier" =>
        {
            (
                node_text(function, source),
                false,
                Some("qualified_call_not_resolved".to_owned()),
            )
        }
        Some(function)
            if matches!(
                function.kind(),
                "member_expression" | "field_expression" | "subscript_expression"
            ) =>
        {
            (
                node_text(function, source),
                false,
                Some("member_or_index_dispatch_not_resolved".to_owned()),
            )
        }
        _ => (
            function
                .map(|function| node_text(function, source))
                .unwrap_or_else(|| "<unknown callee>".to_owned()),
            false,
            Some(
                if function_kind == "" {
                    "callee_missing"
                } else {
                    "dynamic_callee_not_resolved"
                }
                .to_owned(),
            ),
        ),
    };
    let safe_name = label.chars().take(256).collect::<String>();
    CallCandidate {
        path: path.to_owned(),
        source_id: context
            .owner_symbol_id
            .as_deref()
            .unwrap_or(file_id)
            .to_owned(),
        binding: None,
        rust_self_method,
        is_test: context.owner_is_test || file_is_test,
        name: safe_name,
        direct_identifier,
        unresolved_reason,
        anchor: anchor(path, source_sha256, node),
    }
}

fn is_import_node(node: Node<'_>, language: SupportedLanguage) -> bool {
    match language {
        SupportedLanguage::Rust => node.kind() == "use_declaration",
        SupportedLanguage::TypeScript | SupportedLanguage::Tsx | SupportedLanguage::JavaScript => {
            matches!(node.kind(), "import_statement" | "export_statement")
        }
    }
}

fn is_static_import_call(node: Node<'_>, source: &str) -> bool {
    let Some(function) = node.child_by_field_name("function") else {
        return false;
    };
    matches!(node_text(function, source).as_str(), "require" | "import")
}

fn import_candidate(
    node: Node<'_>,
    path: &str,
    source: &str,
    source_sha256: &str,
    language: SupportedLanguage,
) -> Option<ImportCandidate> {
    if language == SupportedLanguage::Rust {
        return Some(ImportCandidate {
            path: path.to_owned(),
            module_specifier: None,
            unresolved_reason: Some("rust_use_path_not_resolved_in_m0".to_owned()),
            bindings: Vec::new(),
            anchor: anchor(path, source_sha256, node),
            is_require_call: false,
        });
    }

    let source_node = if node.kind() == "call_expression" {
        node.child_by_field_name("arguments")?.named_child(0)?
    } else {
        node.child_by_field_name("source")?
    };
    let raw = node_text(source_node, source);
    let specifier = static_string_value(source_node.kind(), &raw);
    let bindings = if node.kind() == "import_statement" {
        import_bindings(node, source)
    } else {
        Vec::new()
    };
    Some(ImportCandidate {
        path: path.to_owned(),
        unresolved_reason: if specifier.is_some() {
            None
        } else {
            Some("module_specifier_is_not_a_plain_static_string".to_owned())
        },
        module_specifier: specifier,
        bindings,
        anchor: anchor(path, source_sha256, node),
        is_require_call: node.kind() == "call_expression"
            && node.child_by_field_name("function").is_some_and(|callee| {
                callee.kind() == "identifier" && node_text(callee, source) == "require"
            }),
    })
}

fn import_bindings(node: Node<'_>, source: &str) -> Vec<ImportBinding> {
    let mut children_cursor = node.walk();
    let Some(clause) = node
        .children(&mut children_cursor)
        .find(|child| child.kind() == "import_clause")
    else {
        return Vec::new();
    };
    let mut children_cursor = node.walk();
    let declaration_is_type_only = node
        .children(&mut children_cursor)
        .any(|child| !child.is_named() && child.kind() == "type");
    let mut bindings = Vec::new();
    let mut clause_cursor = clause.walk();
    for child in clause.named_children(&mut clause_cursor) {
        match child.kind() {
            "identifier" => {
                // Default exports have syntax-specific names; keep their import edge visible,
                // but do not guess which declaration a default import refers to.
                bindings.push(ImportBinding {
                    local_name: node_text(child, source).to_owned(),
                    imported_name: "default".to_owned(),
                    is_value_binding: !declaration_is_type_only,
                });
            }
            "named_imports" => {
                let mut specifiers_cursor = child.walk();
                for specifier in child.named_children(&mut specifiers_cursor) {
                    if specifier.kind() != "import_specifier" {
                        continue;
                    }
                    let Some(name) = specifier.child_by_field_name("name") else {
                        continue;
                    };
                    let imported_name = node_text(name, source).to_owned();
                    let local_name = specifier
                        .child_by_field_name("alias")
                        .map(|alias| node_text(alias, source).to_owned())
                        .unwrap_or_else(|| imported_name.clone());
                    if !local_name.is_empty() && !imported_name.is_empty() {
                        let mut specifier_cursor = specifier.walk();
                        let specifier_is_type_only = specifier
                            .children(&mut specifier_cursor)
                            .any(|child| !child.is_named() && child.kind() == "type");
                        bindings.push(ImportBinding {
                            local_name,
                            imported_name,
                            is_value_binding: !declaration_is_type_only && !specifier_is_type_only,
                        });
                    }
                }
            }
            // Namespace-member calls need type-aware dispatch to be safe. The import edge
            // remains indexed while calls such as `api.run()` remain explicitly unresolved.
            "namespace_import" => {
                let mut namespace_cursor = child.walk();
                if let Some(name) = child.named_children(&mut namespace_cursor).next() {
                    bindings.push(ImportBinding {
                        local_name: node_text(name, source),
                        imported_name: "*".to_owned(),
                        is_value_binding: !declaration_is_type_only,
                    });
                }
            }
            _ => {}
        }
    }
    bindings
}

fn static_string_value(kind: &str, raw: &str) -> Option<String> {
    if !matches!(kind, "string" | "string_literal") || raw.len() < 2 {
        return None;
    }
    let first = raw.as_bytes()[0];
    let last = *raw.as_bytes().last()?;
    if !matches!(first, b'\'' | b'"') || first != last || raw[1..raw.len() - 1].contains('\\') {
        return None;
    }
    let value = raw[1..raw.len() - 1].to_owned();
    (!value.trim().is_empty()).then_some(value)
}

fn build_definition_relationships(
    symbols: &BTreeMap<String, IndexedSymbol>,
) -> Vec<IndexedRelationship> {
    let mut result = Vec::new();
    for symbol in symbols.values() {
        let file_id = file_id(&symbol.path);
        result.push(relationship(
            RelationshipKind::Defines,
            &file_id,
            Some(&symbol.symbol_id),
            Some(&symbol.path),
            Some(&symbol.qualified_name),
            RelationshipConfidence::High,
            None,
            symbol.anchor.clone(),
        ));
        if symbol.is_exported {
            result.push(relationship(
                RelationshipKind::Exports,
                &file_id,
                Some(&symbol.symbol_id),
                Some(&symbol.path),
                Some(&symbol.name),
                RelationshipConfidence::High,
                None,
                symbol.anchor.clone(),
            ));
        }
    }
    result
}

fn resolve_imports(
    candidates: Vec<ImportCandidate>,
    captured_paths: &BTreeSet<String>,
    files: &[IndexedFile],
    symbols: &BTreeMap<String, IndexedSymbol>,
    callable_exports: &BTreeSet<String>,
) -> (
    Vec<IndexedRelationship>,
    HashMap<(String, String), Vec<String>>,
) {
    let fully_parsed = files
        .iter()
        .filter(|file| file.status == IndexFileStatus::Indexed)
        .map(|file| file.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut exported_callables = HashMap::<(String, String), Vec<String>>::new();
    for symbol in symbols.values().filter(|symbol| {
        callable_exports.contains(&symbol.symbol_id)
            && symbol.kind.is_callable()
            && fully_parsed.contains(symbol.path.as_str())
    }) {
        exported_callables
            .entry((symbol.path.clone(), symbol.name.clone()))
            .or_default()
            .push(symbol.symbol_id.clone());
    }
    for targets in exported_callables.values_mut() {
        targets.sort();
        targets.dedup();
    }
    let mut relationships = Vec::new();
    let mut imported_targets = HashMap::<(String, String), Vec<String>>::new();
    for candidate in candidates {
        let (target_path, reason) = match (&candidate.module_specifier, candidate.unresolved_reason)
        {
            (Some(specifier), None) => {
                resolve_relative_import(&candidate.path, specifier, captured_paths)
            }
            (_, Some(reason)) => (None, reason),
            _ => (None, "module_specifier_not_resolved".to_owned()),
        };
        let target_id = target_path.as_deref().map(file_id);
        relationships.push(relationship(
            RelationshipKind::Imports,
            &file_id(&candidate.path),
            target_id.as_deref(),
            target_path.as_deref(),
            candidate.module_specifier.as_deref(),
            if target_id.is_some() {
                RelationshipConfidence::High
            } else {
                RelationshipConfidence::Unresolved
            },
            target_id.is_none().then_some(reason),
            candidate.anchor,
        ));
        let Some(target_path) = target_path else {
            continue;
        };
        // A recovered syntax tree cannot prove that its exported declarations are complete.
        if !fully_parsed.contains(target_path.as_str()) {
            continue;
        }
        for binding in candidate.bindings {
            if !binding.is_value_binding
                || matches!(binding.imported_name.as_str(), "default" | "*")
            {
                continue;
            }
            if let Some(matching_symbols) =
                exported_callables.get(&(target_path.clone(), binding.imported_name))
            {
                imported_targets
                    .entry((candidate.path.clone(), binding.local_name))
                    .or_default()
                    .extend(matching_symbols.iter().cloned());
            }
        }
    }
    for targets in imported_targets.values_mut() {
        targets.sort();
        targets.dedup();
    }
    (relationships, imported_targets)
}

fn resolve_relative_import(
    importer: &str,
    specifier: &str,
    captured_paths: &BTreeSet<String>,
) -> (Option<String>, String) {
    if !specifier.starts_with('.') || specifier.contains('?') || specifier.contains('#') {
        return (
            None,
            "external_or_non_relative_module_not_resolved".to_owned(),
        );
    }
    let mut components = importer.split('/').collect::<Vec<_>>();
    components.pop();
    for component in specifier.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.pop().is_none() {
                    return (None, "relative_import_escapes_snapshot_root".to_owned());
                }
            }
            value => components.push(value),
        }
    }
    let base = components.join("/");
    if base.is_empty() {
        return (None, "relative_import_resolves_to_empty_path".to_owned());
    }
    let has_extension = Path::new(&base).extension().is_some();
    let candidates = if has_extension {
        vec![base]
    } else {
        ["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs", "json"]
            .into_iter()
            .map(|extension| format!("{base}.{extension}"))
            .chain(
                ["ts", "tsx", "js", "jsx", "mts", "cts", "mjs", "cjs", "json"]
                    .into_iter()
                    .map(|extension| format!("{base}/index.{extension}")),
            )
            .collect()
    };
    let found = candidates
        .into_iter()
        .filter(|candidate| captured_paths.contains(candidate))
        .collect::<BTreeSet<_>>();
    match found.len() {
        1 => (found.into_iter().next(), String::new()),
        0 => (None, "relative_module_not_captured_in_snapshot".to_owned()),
        _ => (None, "relative_module_resolution_is_ambiguous".to_owned()),
    }
}

fn resolve_calls(
    candidates: Vec<CallCandidate>,
    symbols: &BTreeMap<String, IndexedSymbol>,
    imported_targets: &HashMap<(String, String), Vec<String>>,
) -> Vec<IndexedRelationship> {
    candidates
        .into_iter()
        .map(|candidate| {
            let mut target = None;
            let mut unresolved_reason = candidate.unresolved_reason.clone();
            if let Some((implementation_scope, method_name)) = &candidate.rust_self_method {
                let qualified_name = format!("{implementation_scope}::{method_name}");
                let mut matches = symbols.values().filter(|symbol| {
                    symbol.path == candidate.path
                        && symbol.kind == SymbolKind::Method
                        && symbol.qualified_name == qualified_name
                });
                match (matches.next(), matches.next()) {
                    (Some(symbol), None) => {
                        target = Some(symbol.symbol_id.clone());
                        unresolved_reason = None;
                    }
                    (Some(_), Some(_)) => {
                        unresolved_reason = Some("multiple_self_method_matches_in_impl".to_owned());
                    }
                    (None, _) => {
                        unresolved_reason = Some("self_method_not_found_in_impl".to_owned());
                    }
                }
            } else if candidate.direct_identifier {
                match candidate.binding {
                    Some(BindingTarget::Symbol(symbol_id)) if symbols.contains_key(&symbol_id) => {
                        target = Some(symbol_id);
                        unresolved_reason = None;
                    }
                    Some(BindingTarget::Import) => {
                        match imported_targets
                            .get(&(candidate.path.clone(), candidate.name.clone()))
                        {
                            Some(matches) if matches.len() == 1 => {
                                target = matches.first().cloned();
                                unresolved_reason = None;
                            }
                            Some(matches) if matches.len() > 1 => {
                                unresolved_reason =
                                    Some("multiple_imported_export_matches".to_owned());
                            }
                            _ => {
                                unresolved_reason = Some("no_unique_imported_export".to_owned());
                            }
                        }
                    }
                    Some(BindingTarget::Unknown(reason)) => {
                        unresolved_reason = Some(reason.to_owned());
                    }
                    _ => unresolved_reason = Some("lexical_target_unavailable".to_owned()),
                }
            }
            let kind = if candidate.is_test {
                RelationshipKind::TestReference
            } else {
                RelationshipKind::Calls
            };
            relationship(
                kind,
                &candidate.source_id,
                target.as_deref(),
                target
                    .as_deref()
                    .and_then(|target_id| symbols.get(target_id))
                    .map(|symbol| symbol.path.as_str()),
                Some(&candidate.name),
                if target.is_some() {
                    RelationshipConfidence::High
                } else {
                    RelationshipConfidence::Unresolved
                },
                unresolved_reason,
                candidate.anchor,
            )
        })
        .collect()
}

fn relationship(
    kind: RelationshipKind,
    source_id: &str,
    target_id: Option<&str>,
    target_path: Option<&str>,
    target_label: Option<&str>,
    confidence: RelationshipConfidence,
    unresolved_reason: Option<String>,
    evidence: SourceAnchor,
) -> IndexedRelationship {
    let key = format!(
        "{}:{}:{}:{}:{}:{}",
        relationship_kind_label(&kind),
        source_id,
        target_id.unwrap_or_default(),
        target_label.unwrap_or_default(),
        evidence.path,
        evidence.start_byte
    );
    IndexedRelationship {
        relationship_id: format!("rel:{}", digest(key.as_bytes())),
        kind,
        source_id: source_id.to_owned(),
        target_id: target_id.map(str::to_owned),
        target_path: target_path.map(str::to_owned),
        target_label: target_label.map(str::to_owned),
        confidence,
        unresolved_reason,
        evidence,
    }
}

fn relationship_kind_label(kind: &RelationshipKind) -> &'static str {
    match kind {
        RelationshipKind::Defines => "defines",
        RelationshipKind::Exports => "exports",
        RelationshipKind::Imports => "imports",
        RelationshipKind::Calls => "calls",
        RelationshipKind::TestReference => "testReference",
    }
}

fn relationship_order(
    left: &IndexedRelationship,
    right: &IndexedRelationship,
) -> std::cmp::Ordering {
    relationship_kind_label(&left.kind)
        .cmp(relationship_kind_label(&right.kind))
        .then(left.source_id.cmp(&right.source_id))
        .then(left.target_id.cmp(&right.target_id))
        .then(left.evidence.path.cmp(&right.evidence.path))
        .then(left.evidence.start_byte.cmp(&right.evidence.start_byte))
        .then(left.relationship_id.cmp(&right.relationship_id))
}

fn update_progress_counts(
    progress: &mut IndexProgress,
    files: &[IndexedFile],
    symbols: &BTreeMap<String, IndexedSymbol>,
    calls: &[CallCandidate],
    imports: &[ImportCandidate],
) {
    progress.files_indexed = files
        .iter()
        .filter(|file| {
            matches!(
                file.status,
                IndexFileStatus::Indexed | IndexFileStatus::PartialParse
            )
        })
        .count();
    progress.symbols_found = symbols.len();
    progress.relationships_found = symbols.len() + calls.len() + imports.len();
}

fn anchor(path: &str, source_sha256: &str, node: Node<'_>) -> SourceAnchor {
    let start = node.start_position();
    let end = node.end_position();
    SourceAnchor {
        path: path.to_owned(),
        source_sha256: source_sha256.to_owned(),
        start_byte: node.start_byte(),
        end_byte: node.end_byte(),
        start_line: start.row + 1,
        end_line: end.row + 1,
        start_column: start.column + 1,
        end_column: end.column + 1,
    }
}

fn file_id(path: &str) -> String {
    format!("file:{}", digest(path.as_bytes()))
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{AccessStatus, RecentProject};
    use crate::snapshot::{self, CaptureLimits};
    use std::process::Command;
    use std::sync::atomic::AtomicBool;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "code-atlas-index-{}-{}",
                std::process::id(),
                Uuid::new_v4()
            ));
            fs::create_dir_all(&path).expect("fixture directory should be created");
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

    fn git(root: &Path, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .expect("git should be available for snapshot fixtures");
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn write_fixture(root: &Path, relative_path: &str, source: &str) {
        let path = root.join(relative_path);
        fs::create_dir_all(path.parent().expect("fixture has a parent directory"))
            .expect("fixture directories should be created");
        fs::write(path, source).expect("fixture source should be written");
    }

    fn commit_fixture(root: &Path, message: &str) {
        git(root, &["add", "--all"]);
        git(root, &["commit", "-qm", message]);
    }

    fn fixture_project(root: &Path) -> RecentProject {
        let path = root
            .canonicalize()
            .expect("fixture repository should resolve")
            .display()
            .to_string();
        RecentProject {
            project_id: path.clone(),
            display_name: "fixture".to_owned(),
            selected_path: path.clone(),
            repository_root: path,
            branch: Some("main".to_owned()),
            head: None,
            access_status: AccessStatus::Available,
            last_opened_unix_ms: 0,
        }
    }

    fn capture(storage: &Path, project: &RecentProject) -> SnapshotManifest {
        let progress = snapshot::new_capture_progress(storage, project, CaptureLimits::default())
            .expect("snapshot capture should start");
        snapshot::capture_snapshot_with_progress(
            storage,
            project,
            progress,
            &AtomicBool::new(false),
            &mut |_| {},
        )
        .expect("snapshot fixture should be captured")
    }

    fn build(storage: &Path, manifest: &SnapshotManifest) -> SnapshotIndex {
        build_snapshot_index(
            storage,
            manifest,
            "test-index-job",
            &AtomicBool::new(false),
            |_| {},
        )
    }

    /// Measures parser coverage of the two large Fontdone FFI files against
    /// the captured snapshot without running repository code or calling a model.
    #[test]
    #[ignore = "Explicit Fontdone parser coverage measurement; requires CODE_ATLAS_FONTDONE_DATA_DIR."]
    fn fontdone_large_file_index_profile() {
        let started = std::time::Instant::now();
        let storage = PathBuf::from(
            std::env::var("CODE_ATLAS_FONTDONE_DATA_DIR")
                .expect("set CODE_ATLAS_FONTDONE_DATA_DIR"),
        );
        let corpus: serde_json::Value = serde_json::from_str(include_str!(
            "../../docs/acceptance/fixtures/fontdone-retrieval-ranges.json"
        ))
        .unwrap();
        let snapshot_id = corpus["snapshotId"].as_str().unwrap();
        let manifest = snapshot::load_snapshot_manifest(
            &storage,
            "/Users/lazytrot/work/fontdone",
            snapshot_id,
        )
        .unwrap();
        let indexed = build_snapshot_index(
            &storage,
            &manifest,
            "fontdone-large-file-index-profile",
            &AtomicBool::new(false),
            |_| {},
        );
        save_snapshot_index(&storage, &indexed).unwrap();
        for path in ["src/ffi/handles.rs", "fontdone-c-abi/src/implementation.rs"] {
            let file = indexed
                .files
                .iter()
                .find(|file| file.path == path)
                .expect("captured FFI file should have an index status");
            let declarations = indexed
                .symbols
                .iter()
                .filter(|symbol| symbol.path == path)
                .map(|symbol| symbol.name.as_str())
                .filter(|name| matches!(*name, "FT_Load_Glyph" | "FT_Render_Glyph"))
                .collect::<Vec<_>>();
            println!(
                "FONTDONE_LARGE_FILE_INDEX {}",
                serde_json::json!({
                    "path":path,
                    "sourceBytes":file.source_size_bytes,
                    "status":format!("{:?}", file.status),
                    "symbolCount":file.symbol_count,
                    "declarations":declarations,
                    "profileElapsedMs":started.elapsed().as_millis(),
                    "indexStatus":format!("{:?}", indexed.status),
                })
            );
        }
    }

    #[test]
    fn saved_snapshot_index_accepts_32_mib_and_rejects_the_next_byte() {
        let directory = TestDirectory::new();
        let project_id = "fixture-project";
        let snapshot_id = "snapshot-a";
        let index = SnapshotIndex {
            schema_version: INDEX_SCHEMA_VERSION,
            indexer_version: INDEXER_VERSION.to_owned(),
            project_id: project_id.to_owned(),
            snapshot_id: snapshot_id.to_owned(),
            status: SnapshotIndexStatus::Partial,
            parsers: Vec::new(),
            files: Vec::new(),
            symbols: Vec::new(),
            relationships: Vec::new(),
        };
        let mut bytes = serde_json::to_vec(&index).unwrap();
        bytes.resize(MAX_INDEX_JSON_BYTES, b' ');

        let path = relationship_path(directory.path(), project_id, snapshot_id).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, &bytes).unwrap();
        assert_eq!(
            load_snapshot_index(directory.path(), project_id, snapshot_id).unwrap(),
            Some(index.clone())
        );

        bytes.push(b' ');
        fs::write(&path, &bytes).unwrap();
        assert!(
            load_snapshot_index(directory.path(), project_id, snapshot_id)
                .unwrap_err()
                .contains("exceeds the supported size")
        );

        let mut truncated = serde_json::to_vec(&index).unwrap();
        truncated.pop();
        for invalid in [Vec::new(), truncated, b"{ broken".to_vec()] {
            fs::write(&path, &invalid).unwrap();
            assert!(
                load_snapshot_index(directory.path(), project_id, snapshot_id)
                    .unwrap_err()
                    .contains("is invalid")
            );
            assert_eq!(fs::read(&path).unwrap(), invalid);
        }
    }

    fn relationship_for<'a>(
        index: &'a SnapshotIndex,
        kind: RelationshipKind,
        source_path: &str,
        label: &str,
    ) -> &'a IndexedRelationship {
        index
            .relationships
            .iter()
            .find(|edge| {
                edge.kind == kind
                    && edge.evidence.path == source_path
                    && edge.target_label.as_deref() == Some(label)
            })
            .unwrap_or_else(|| panic!("missing {kind:?} relationship for {source_path}:{label}"))
    }

    fn call_at_line<'a>(
        index: &'a SnapshotIndex,
        source_path: &str,
        line: usize,
        label: &str,
    ) -> &'a IndexedRelationship {
        index
            .relationships
            .iter()
            .find(|edge| {
                edge.kind == RelationshipKind::Calls
                    && edge.evidence.path == source_path
                    && edge.evidence.start_line == line
                    && edge.target_label.as_deref() == Some(label)
            })
            .unwrap_or_else(|| panic!("missing call relationship for {source_path}:{line}:{label}"))
    }

    fn assert_unresolved_call(
        index: &SnapshotIndex,
        source_path: &str,
        line: usize,
        label: &str,
        reason: &str,
    ) {
        let call = call_at_line(index, source_path, line, label);
        assert_eq!(
            call.confidence,
            RelationshipConfidence::Unresolved,
            "{source_path}:{line}:{label} reason={reason}: {call:#?}"
        );
        assert_eq!(call.target_id, None);
        assert_eq!(call.target_path, None);
        assert_eq!(call.unresolved_reason.as_deref(), Some(reason));
    }

    #[test]
    fn lexical_resolution_is_conservative_across_the_shipped_grammars() {
        let temporary = TestDirectory::new();
        let root = temporary.path().join("repository");
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "--initial-branch=main"]);
        git(&root, &["config", "user.name", "Code Atlas Index Tests"]);
        git(
            &root,
            &["config", "user.email", "index-tests@example.invalid"],
        );

        write_fixture(
            &root,
            "src/ts-worker.ts",
            concat!(
                "export function imported(value: string) { return value; }\n",
                "export function duplicateExport() {}\n",
                "export function duplicateExport() {}\n",
            ),
        );
        let typescript = concat!(
            "import { imported as run, duplicateExport } from './ts-worker';\n",
            "export function direct() { return run('direct'); }\n",
            "export function parameter(run: (value: string) => string) { return run('parameter'); }\n",
            "export function defaultParameter(run = (value: string) => value) { return run('default'); }\n",
            "export function destructured({ run }: { run: (value: string) => string }) { return run('destructured'); }\n",
            "export function destructuredDefault({ run = (value: string) => value }: { run?: (value: string) => string }) { return run('destructured default'); }\n",
            "export function blockScoped() { { const run = (value: string) => value; run('block'); } }\n",
            "export function nonCallableShadow() { { const run = 'not a function'; run(); } }\n",
            "export function caught() { try { throw (value: string) => value; } catch (run) { run('catch'); } }\n",
            "export function looped(values: Array<(value: string) => string>) { for (const run of values) { run('loop'); } }\n",
            "export function reassigned() { let local = (value: string) => value; local = run; return local('changed'); }\n",
            "export function temporal() { late(); const late = () => ''; }\n",
            "export function duplicate() {}\n",
            "export function duplicate() {}\n",
            "export function callsDuplicate() { duplicate(); }\n",
            "export function enclosing() { function nested() {} class Inner { method() {} } nested(); }\n",
            "export function callsAmbiguousImport() { duplicateExport(); }\n",
        );
        write_fixture(&root, "src/shadow.ts", typescript);
        write_fixture(
            &root,
            "src/worker.js",
            "export function imported(value) { return value; }\n",
        );
        write_fixture(
            &root,
            "src/client.js",
            concat!(
                "import { imported as run } from './worker.js';\n",
                "export function direct() { return run('direct'); }\n",
                "export function parameter(run) { return run('parameter'); }\n",
                "export function defaultParameter(run = value => value) { return run('default'); }\n",
                "export function destructured({ run }) { return run('destructured'); }\n",
                "export function blockScoped() { { const run = value => value; run('block'); } }\n",
            ),
        );
        write_fixture(
            &root,
            "src/component.tsx",
            concat!(
                "import { imported as run } from './ts-worker';\n",
                "export function direct() { return run('tsx'); }\n",
                "export function parameter(run: (value: string) => string) { return run('parameter'); }\n",
                "export const Handler = ({ run }: { run: (value: string) => string }) => <button onClick={() => run('handler')} />;\n",
            ),
        );
        write_fixture(
            &root,
            "src/main.rs",
            concat!(
                "fn helper() {}\n",
                "pub fn direct() { helper(); }\n",
                "pub fn parameter(helper: fn()) { helper(); }\n",
                "pub fn local() { let helper = || {}; helper(); }\n",
                "fn duplicate() {}\n",
                "fn duplicate() {}\n",
                "pub fn calls_duplicate() { duplicate(); }\n",
                "pub fn enclosing() { fn nested() {} nested(); }\n",
                "pub(crate) fn crate_only() {}\n",
                "pub fn public_api() {}\n",
                "pub mod nested { pub fn inline_item() {} pub(super) fn restricted_item() {} }\n",
                "struct Face;\n",
                "impl Face {\n",
                "    fn self_calls(&self) {\n",
                "        Self::helper();\n",
                "        Self::missing();\n",
                "    }\n",
                "    fn helper() {}\n",
                "}\n",
            ),
        );
        commit_fixture(&root, "lexical resolution fixtures");

        let storage = temporary.path().join("app-data");
        let project = fixture_project(&root);
        let manifest = capture(&storage, &project);
        let index = build(&storage, &manifest);
        assert_eq!(index.indexer_version, INDEXER_VERSION);
        let legacy_path = storage
            .join("indexes")
            .join(digest(project.project_id.as_bytes()))
            .join("atlas-symbol-index-v1")
            .join(format!("{}.json", manifest.summary.snapshot_id));
        fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
        let mut legacy_index = index.clone();
        legacy_index.indexer_version = "atlas-symbol-index-v1".to_owned();
        fs::write(&legacy_path, serde_json::to_vec(&legacy_index).unwrap()).unwrap();
        assert!(
            load_snapshot_index(&storage, &project.project_id, &manifest.summary.snapshot_id)
                .unwrap()
                .is_none(),
            "v1 cache entries must not be reused under indexer v2"
        );

        let imported = index
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/ts-worker.ts" && symbol.name == "imported")
            .expect("the named export should be indexed");
        let direct = call_at_line(&index, "src/shadow.ts", 2, "run");
        assert_eq!(
            direct.confidence,
            RelationshipConfidence::High,
            "{direct:#?}"
        );
        assert_eq!(
            direct.target_id.as_deref(),
            Some(imported.symbol_id.as_str())
        );
        assert_eq!(direct.target_path.as_deref(), Some("src/ts-worker.ts"));
        let typescript_bytes = typescript.as_bytes();
        assert_eq!(
            &typescript_bytes[direct.evidence.start_byte..direct.evidence.end_byte],
            b"run('direct')",
            "resolved call anchors must retain the exact source expression"
        );
        for (line, reason) in [
            (3, "call_is_shadowed_by_parameter"),
            (4, "call_is_shadowed_by_parameter"),
            (5, "call_is_shadowed_by_parameter"),
            (6, "call_is_shadowed_by_parameter"),
            (8, "call_is_shadowed_by_local_binding"),
            (9, "call_is_shadowed_by_catch_binding"),
            (10, "call_is_shadowed_by_loop_binding"),
            (11, "binding_is_reassigned"),
            (12, "binding_not_initialized_at_call_site"),
            (15, "multiple_bindings_in_lexical_scope"),
        ] {
            let label = if line == 11 {
                "local"
            } else if line == 12 {
                "late"
            } else if line == 15 {
                "duplicate"
            } else {
                "run"
            };
            assert_unresolved_call(&index, "src/shadow.ts", line, label, reason);
        }
        let local_arrow_call = call_at_line(&index, "src/shadow.ts", 7, "run");
        let local_arrow = index
            .symbols
            .iter()
            .find(|symbol| {
                symbol.path == "src/shadow.ts"
                    && symbol.name == "run"
                    && symbol.anchor.start_line == 7
            })
            .expect("a local arrow function should have a symbol identity");
        assert_eq!(local_arrow_call.confidence, RelationshipConfidence::High);
        assert_eq!(
            local_arrow_call.target_id.as_deref(),
            Some(local_arrow.symbol_id.as_str())
        );

        let duplicate_symbols = index
            .symbols
            .iter()
            .filter(|symbol| symbol.path == "src/shadow.ts" && symbol.name == "duplicate")
            .collect::<Vec<_>>();
        assert_eq!(duplicate_symbols.len(), 2);
        assert_ne!(
            duplicate_symbols[0].symbol_id,
            duplicate_symbols[1].symbol_id
        );
        for nested_name in ["nested", "method"] {
            let symbol = index
                .symbols
                .iter()
                .find(|symbol| symbol.path == "src/shadow.ts" && symbol.name == nested_name)
                .expect("nested declarations should remain indexed");
            assert!(!symbol.is_exported, "{nested_name} is not a module export");
        }
        let duplicate_ids = duplicate_symbols
            .iter()
            .map(|symbol| symbol.symbol_id.as_str())
            .collect::<BTreeSet<_>>();
        let duplicate_exports = index
            .relationships
            .iter()
            .filter(|edge| {
                edge.kind == RelationshipKind::Exports
                    && edge.evidence.path == "src/shadow.ts"
                    && edge.target_label.as_deref() == Some("duplicate")
            })
            .collect::<Vec<_>>();
        assert_eq!(duplicate_exports.len(), 2);
        assert!(duplicate_exports.iter().all(|edge| {
            edge.confidence == RelationshipConfidence::High
                && duplicate_ids.contains(edge.target_id.as_deref().unwrap_or_default())
        }));
        assert_unresolved_call(
            &index,
            "src/shadow.ts",
            17,
            "duplicateExport",
            "no_unique_imported_export",
        );

        let js_imported = index
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/worker.js" && symbol.name == "imported")
            .expect("the JavaScript export should be indexed");
        let js_direct = call_at_line(&index, "src/client.js", 2, "run");
        assert_eq!(js_direct.confidence, RelationshipConfidence::High);
        assert_eq!(
            js_direct.target_id.as_deref(),
            Some(js_imported.symbol_id.as_str())
        );
        for (line, reason) in [
            (3, "call_is_shadowed_by_parameter"),
            (4, "call_is_shadowed_by_parameter"),
            (5, "call_is_shadowed_by_parameter"),
        ] {
            assert_unresolved_call(&index, "src/client.js", line, "run", reason);
        }
        let js_local_arrow = call_at_line(&index, "src/client.js", 6, "run");
        assert_eq!(js_local_arrow.confidence, RelationshipConfidence::High);
        assert_eq!(js_local_arrow.target_path.as_deref(), Some("src/client.js"));

        let tsx_imported = index
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/ts-worker.ts" && symbol.name == "imported")
            .expect("the TSX named export should be indexed");
        let tsx_direct = call_at_line(&index, "src/component.tsx", 2, "run");
        assert_eq!(tsx_direct.confidence, RelationshipConfidence::High);
        assert_eq!(
            tsx_direct.target_id.as_deref(),
            Some(tsx_imported.symbol_id.as_str())
        );
        assert_unresolved_call(
            &index,
            "src/component.tsx",
            3,
            "run",
            "call_is_shadowed_by_parameter",
        );
        assert_unresolved_call(
            &index,
            "src/component.tsx",
            4,
            "run",
            "call_is_shadowed_by_parameter",
        );

        let rust_direct = call_at_line(&index, "src/main.rs", 2, "helper");
        let helper = index
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/main.rs" && symbol.name == "helper")
            .expect("the root helper should be indexed");
        assert_eq!(rust_direct.confidence, RelationshipConfidence::High);
        assert_eq!(
            rust_direct.target_id.as_deref(),
            Some(helper.symbol_id.as_str())
        );
        assert_unresolved_call(
            &index,
            "src/main.rs",
            3,
            "helper",
            "call_is_shadowed_by_parameter",
        );
        assert_unresolved_call(
            &index,
            "src/main.rs",
            4,
            "helper",
            "call_is_shadowed_by_pattern_binding",
        );
        assert_unresolved_call(
            &index,
            "src/main.rs",
            7,
            "duplicate",
            "multiple_bindings_in_lexical_scope",
        );
        let nested_call = call_at_line(&index, "src/main.rs", 8, "nested");
        assert_eq!(nested_call.confidence, RelationshipConfidence::High);
        assert_eq!(nested_call.target_path.as_deref(), Some("src/main.rs"));
        let self_method_call = call_at_line(&index, "src/main.rs", 15, "Self::helper");
        let self_method = index
            .symbols
            .iter()
            .find(|symbol| {
                symbol.path == "src/main.rs"
                    && symbol.kind == SymbolKind::Method
                    && symbol.name == "helper"
                    && symbol.qualified_name == "impl:Face::helper"
            })
            .expect("the inherent helper method should be indexed");
        assert_eq!(self_method_call.confidence, RelationshipConfidence::High);
        assert_eq!(
            self_method_call.target_id.as_deref(),
            Some(self_method.symbol_id.as_str())
        );
        assert_unresolved_call(
            &index,
            "src/main.rs",
            16,
            "Self::missing",
            "self_method_not_found_in_impl",
        );

        let duplicate_rust_symbols = index
            .symbols
            .iter()
            .filter(|symbol| symbol.path == "src/main.rs" && symbol.name == "duplicate")
            .collect::<Vec<_>>();
        assert_eq!(duplicate_rust_symbols.len(), 2);
        assert_ne!(
            duplicate_rust_symbols[0].symbol_id,
            duplicate_rust_symbols[1].symbol_id
        );
        let rust_exports = index
            .relationships
            .iter()
            .filter(|edge| {
                edge.kind == RelationshipKind::Exports && edge.evidence.path == "src/main.rs"
            })
            .filter_map(|edge| edge.target_label.as_deref())
            .collect::<BTreeSet<_>>();
        assert!(rust_exports.contains("public_api"));
        assert!(!rust_exports.contains("crate_only"));
        assert!(!rust_exports.contains("inline_item"));
        assert!(!rust_exports.contains("restricted_item"));
    }

    #[test]
    fn captured_snapshot_index_is_deterministic_anchored_and_conservative() {
        let temporary = TestDirectory::new();
        let root = temporary.path().join("repository");
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "--initial-branch=main"]);
        git(&root, &["config", "user.name", "Code Atlas Index Tests"]);
        git(
            &root,
            &["config", "user.email", "index-tests@example.invalid"],
        );
        let worker_source = "export function work(): number { return 1; }\n";
        write_fixture(
            &root,
            "src/app.ts",
            "import { work as runTask } from './worker';\nexport function main() { return runTask(); }\nfunction local() { return main(); }\nexport function localCall() { return local(); }\n",
        );
        write_fixture(&root, "src/worker.ts", worker_source);
        write_fixture(
            &root,
            "src/widget.tsx",
            "export const Widget = () => <button>Atlas</button>;\n",
        );
        write_fixture(
            &root,
            "src/view.jsx",
            "export function view() { return <div />; }\n",
        );
        write_fixture(
            &root,
            "src/dynamic.ts",
            "export function dispatch(api: any) { getHandler()(); api.run(); }\n",
        );
        write_fixture(
            &root,
            "src/no-execution.ts",
            "throw new Error('repository source must never execute');\nexport function stillIndexable() { return true; }\n",
        );
        write_fixture(
            &root,
            "tests/work.test.ts",
            "import { work } from '../src/worker';\ntest('work', () => { work(); });\n",
        );
        write_fixture(
            &root,
            "src/recovered.js",
            "export function usable() { return 1; }\nfunction broken( {\n",
        );
        write_fixture(&root, "src/invalid.ts", "function (\n");
        write_fixture(
            &root,
            "src/main.rs",
            "pub fn rust_entry() { helper(); }\nfn helper() {}\n",
        );
        let medium_source = format!("pub fn large_boundary() {{}}\n{}", " ".repeat(768 * 1024));
        write_fixture(&root, "src/medium.rs", &medium_source);
        write_fixture(
            &root,
            "notes.md",
            "A captured file with no shipped parser.\n",
        );
        write_fixture(
            &root,
            "src/large.ts",
            &" ".repeat(MAX_SOURCE_BYTES_PER_FILE as usize + 1),
        );
        commit_fixture(&root, "index fixture");

        let storage = temporary.path().join("app-data");
        let project = fixture_project(&root);
        let manifest = capture(&storage, &project);
        let indexed = build(&storage, &manifest);
        let repeated = build(&storage, &manifest);
        assert_eq!(indexed, repeated, "index output should be deterministic");
        assert_eq!(indexed.status, SnapshotIndexStatus::Partial);
        save_snapshot_index(&storage, &indexed).expect("completed index should persist locally");
        assert_eq!(
            load_snapshot_index(&storage, &project.project_id, &manifest.summary.snapshot_id)
                .expect("saved index should load")
                .as_ref(),
            Some(&indexed)
        );
        assert!(indexed.parsers.iter().any(|parser| {
            parser.grammar == "tree-sitter-typescript" && parser.grammar_version == "0.23.2"
        }));
        assert_eq!(
            indexed
                .files
                .iter()
                .find(|file| file.path == "src/widget.tsx")
                .unwrap()
                .status,
            IndexFileStatus::Indexed
        );
        assert_eq!(
            indexed
                .files
                .iter()
                .find(|file| file.path == "src/view.jsx")
                .unwrap()
                .status,
            IndexFileStatus::Indexed
        );

        let work = indexed
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/worker.ts" && symbol.name == "work")
            .expect("exported worker function should be indexed");
        let source = snapshot::read_snapshot_source(
            &storage,
            &project.project_id,
            &manifest.summary.snapshot_id,
            "src/worker.ts",
        )
        .expect("captured worker source should read");
        assert_eq!(work.anchor.source_sha256, source.sha256);
        assert_eq!(
            &source.contents.as_bytes()[work.anchor.start_byte..work.anchor.end_byte],
            worker_source
                .trim_end()
                .strip_prefix("export ")
                .unwrap()
                .as_bytes(),
            "symbol byte anchors should select exact captured bytes"
        );

        let imported_call =
            relationship_for(&indexed, RelationshipKind::Calls, "src/app.ts", "runTask");
        assert_eq!(
            imported_call.target_id.as_deref(),
            Some(work.symbol_id.as_str())
        );
        assert_eq!(imported_call.target_path.as_deref(), Some("src/worker.ts"));
        assert_eq!(imported_call.confidence, RelationshipConfidence::High);
        let local_call = relationship_for(&indexed, RelationshipKind::Calls, "src/app.ts", "local");
        assert_eq!(local_call.confidence, RelationshipConfidence::High);
        let rust_call =
            relationship_for(&indexed, RelationshipKind::Calls, "src/main.rs", "helper");
        assert_eq!(rust_call.confidence, RelationshipConfidence::High);
        let test_call = relationship_for(
            &indexed,
            RelationshipKind::TestReference,
            "tests/work.test.ts",
            "work",
        );
        assert_eq!(
            test_call.target_id.as_deref(),
            Some(work.symbol_id.as_str())
        );

        let unresolved = indexed
            .relationships
            .iter()
            .filter(|edge| {
                edge.evidence.path == "src/dynamic.ts" && edge.kind == RelationshipKind::Calls
            })
            .collect::<Vec<_>>();
        assert_eq!(unresolved.len(), 3);
        assert!(unresolved.iter().all(|edge| {
            edge.confidence == RelationshipConfidence::Unresolved && edge.target_id.is_none()
        }));
        assert!(
            unresolved.iter().any(
                |edge| edge.unresolved_reason.as_deref() == Some("dynamic_callee_not_resolved")
            )
        );
        assert!(
            unresolved
                .iter()
                .any(|edge| edge.unresolved_reason.as_deref()
                    == Some("member_or_index_dispatch_not_resolved"))
        );
        assert_eq!(
            indexed
                .files
                .iter()
                .find(|file| file.path == "src/recovered.js")
                .unwrap()
                .status,
            IndexFileStatus::PartialParse
        );
        assert_eq!(
            indexed
                .files
                .iter()
                .find(|file| file.path == "src/invalid.ts")
                .unwrap()
                .status,
            IndexFileStatus::ParseError
        );
        assert!(
            indexed
                .symbols
                .iter()
                .any(|symbol| symbol.path == "src/main.rs" && symbol.name == "rust_entry")
        );
        assert_eq!(
            indexed
                .files
                .iter()
                .find(|file| file.path == "notes.md")
                .unwrap()
                .status,
            IndexFileStatus::UnsupportedLanguage
        );
        assert_eq!(
            indexed
                .files
                .iter()
                .find(|file| file.path == "src/medium.rs")
                .unwrap()
                .status,
            IndexFileStatus::Indexed
        );
        let large_boundary = indexed
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/medium.rs" && symbol.name == "large_boundary")
            .expect("a 768 KiB Rust file should be structurally indexed");
        assert!(
            medium_source
                .get(large_boundary.anchor.start_byte..large_boundary.anchor.end_byte)
                .is_some_and(|source| source.contains("large_boundary"))
        );
        assert_eq!(
            indexed
                .files
                .iter()
                .find(|file| file.path == "src/large.ts")
                .unwrap()
                .status,
            IndexFileStatus::FileTooLarge
        );
        for file in &indexed.files {
            assert_eq!(
                file.relationship_count,
                indexed
                    .relationships
                    .iter()
                    .filter(|edge| edge.evidence.path == file.path)
                    .count(),
                "relationship counts should match source evidence path for {}",
                file.path
            );
        }
        let repository_state = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&root)
            .output()
            .unwrap();
        assert!(repository_state.status.success());
        assert!(
            repository_state.stdout.is_empty(),
            "indexing must not execute or modify the repository"
        );

        let cancelled = build_snapshot_index(
            &storage,
            &manifest,
            "cancelled-job",
            &AtomicBool::new(true),
            |_| {},
        );
        assert_eq!(cancelled.status, SnapshotIndexStatus::Cancelled);
        assert!(cancelled.relationships.is_empty());
    }

    #[test]
    #[ignore = "manual large-repository performance report; run by name with --ignored --nocapture"]
    fn report_representative_large_repository_performance() {
        use std::time::Instant;

        let temporary = TestDirectory::new();
        let root = temporary.path().join("representative-large-repository");
        let source_dir = root.join("src");
        fs::create_dir_all(&source_dir).unwrap();
        git(&root, &["init", "-q"]);
        git(&root, &["config", "user.name", "Code Atlas Benchmark"]);
        git(
            &root,
            &["config", "user.email", "benchmark@example.invalid"],
        );
        git(&root, &["branch", "-M", "main"]);

        const FILE_COUNT: usize = 2_000;
        for index in 0..FILE_COUNT {
            let group_dir = source_dir.join(format!("group_{:02}", index / 100));
            if index % 100 == 0 {
                fs::create_dir_all(&group_dir).unwrap();
            }
            let source = format!(
                "export function transform_{index:04}(input: string): string {{\n  return input.trim();\n}}\n"
            );
            fs::write(group_dir.join(format!("module_{index:04}.ts")), source).unwrap();
        }
        commit_fixture(&root, "representative large fixture");

        let storage = temporary.path().join("app-data");
        let project = fixture_project(&root);

        // Warm the filesystem and parser once; report three subsequent complete runs.
        let warm_manifest = capture(&storage, &project);
        let warm_index = build(&storage, &warm_manifest);
        save_snapshot_index(&storage, &warm_index).unwrap();

        let mut capture_ms = Vec::with_capacity(3);
        let mut index_ms = Vec::with_capacity(3);
        let mut save_ms = Vec::with_capacity(3);
        let mut final_manifest = None;
        let mut final_index = None;
        for _ in 0..3 {
            let started = Instant::now();
            let manifest = capture(&storage, &project);
            capture_ms.push(started.elapsed().as_millis());

            let started = Instant::now();
            let index = build(&storage, &manifest);
            index_ms.push(started.elapsed().as_millis());

            let started = Instant::now();
            save_snapshot_index(&storage, &index).unwrap();
            save_ms.push(started.elapsed().as_millis());

            final_manifest = Some(manifest);
            final_index = Some(index);
        }

        let manifest = final_manifest.unwrap();
        let index = final_index.unwrap();
        assert_eq!(manifest.files.len(), FILE_COUNT);
        assert_eq!(index.status, SnapshotIndexStatus::Complete);
        assert_eq!(index.files.len(), FILE_COUNT);
        assert!(index.symbols.len() >= FILE_COUNT);

        fn median(values: &[u128]) -> u128 {
            let mut sorted = values.to_vec();
            sorted.sort_unstable();
            sorted[sorted.len() / 2]
        }

        eprintln!(
            "ATLAS_PERF os={} arch={} files={} source_bytes={} indexed_files={} symbols={} relationships={} warmup=1 samples=3 capture_ms={:?} capture_median_ms={} index_ms={:?} index_median_ms={} save_ms={:?} save_median_ms={}",
            std::env::consts::OS,
            std::env::consts::ARCH,
            manifest.summary.files_captured,
            manifest.summary.total_bytes,
            index.files.len(),
            index.symbols.len(),
            index.relationships.len(),
            capture_ms,
            median(&capture_ms),
            index_ms,
            median(&index_ms),
            save_ms,
            median(&save_ms),
        );
    }

    #[test]
    fn symbol_identity_survives_line_shifts_while_anchor_moves() {
        let temporary = TestDirectory::new();
        let root = temporary.path().join("repository");
        fs::create_dir_all(&root).unwrap();
        git(&root, &["init", "-q", "--initial-branch=main"]);
        git(&root, &["config", "user.name", "Code Atlas Index Tests"]);
        git(
            &root,
            &["config", "user.email", "index-tests@example.invalid"],
        );
        write_fixture(
            &root,
            "src/worker.ts",
            "export function work(): number { return 1; }\n",
        );
        commit_fixture(&root, "initial source");
        let storage = temporary.path().join("app-data");
        let project = fixture_project(&root);
        let first_manifest = capture(&storage, &project);
        let first = build(&storage, &first_manifest);
        let first_symbol = first
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/worker.ts" && symbol.name == "work")
            .unwrap();

        write_fixture(
            &root,
            "src/worker.ts",
            "// inserted before declaration\n\nexport function work(): number { return 1; }\n",
        );
        commit_fixture(&root, "move source declaration");
        let second_manifest = capture(&storage, &project);
        let second = build(&storage, &second_manifest);
        let second_symbol = second
            .symbols
            .iter()
            .find(|symbol| symbol.path == "src/worker.ts" && symbol.name == "work")
            .unwrap();

        assert_eq!(first_symbol.symbol_id, second_symbol.symbol_id);
        assert_eq!(
            first_symbol.anchor.start_line + 2,
            second_symbol.anchor.start_line
        );
        assert_ne!(
            first_symbol.anchor.source_sha256,
            second_symbol.anchor.source_sha256
        );
    }
}

fn relationship_path(
    storage_dir: &Path,
    project_id: &str,
    snapshot_id: &str,
) -> Result<PathBuf, String> {
    if snapshot_id.is_empty()
        || !snapshot_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("The snapshot identifier is not safe for local index storage.".to_owned());
    }
    Ok(storage_dir
        .join("indexes")
        .join(digest(project_id.as_bytes()))
        .join(INDEXER_VERSION)
        .join(format!("{snapshot_id}.json")))
}

pub fn save_snapshot_index(storage_dir: &Path, index: &SnapshotIndex) -> Result<(), String> {
    if index.schema_version != INDEX_SCHEMA_VERSION || index.indexer_version != INDEXER_VERSION {
        return Err("The snapshot index version does not match this app.".to_owned());
    }
    if index.status == SnapshotIndexStatus::Cancelled {
        return Err("A cancelled snapshot index cannot replace a saved index.".to_owned());
    }
    let bytes = serde_json::to_vec(index)
        .map_err(|_| "The snapshot index could not be serialized.".to_owned())?;
    if bytes.len() > MAX_INDEX_JSON_BYTES {
        return Err("The snapshot index exceeded its 32 MB storage limit.".to_owned());
    }
    let path = relationship_path(storage_dir, &index.project_id, &index.snapshot_id)?;
    let parent = path
        .parent()
        .ok_or_else(|| "The local index path is invalid.".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|_| "The local index folder could not be created.".to_owned())?;
    let temporary = parent.join(format!(".index-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "The local index could not be written.".to_owned())?;
        file.write_all(&bytes)
            .map_err(|_| "The local index could not be written.".to_owned())?;
        file.sync_all()
            .map_err(|_| "The local index could not be synced.".to_owned())?;
        fs::rename(&temporary, &path)
            .map_err(|_| "The local index could not be finalized.".to_owned())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

/// A cheap cache invalidation hint. Source evidence still requires full blob/hash checks.
pub(crate) fn snapshot_index_stamp(
    storage: &Path,
    project: &str,
    snapshot: &str,
) -> Option<(u64, std::time::SystemTime)> {
    let metadata = fs::metadata(relationship_path(storage, project, snapshot).ok()?).ok()?;
    Some((metadata.len(), metadata.modified().ok()?))
}

pub fn load_snapshot_index(
    storage_dir: &Path,
    project_id: &str,
    snapshot_id: &str,
) -> Result<Option<SnapshotIndex>, String> {
    let path = relationship_path(storage_dir, project_id, snapshot_id)?;
    let bytes = match crate::bounded_file::read(&path, MAX_INDEX_JSON_BYTES) {
        Ok(bytes) => bytes,
        Err(crate::bounded_file::ReadError::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(None);
        }
        Err(crate::bounded_file::ReadError::Io(_)) => {
            return Err("The saved snapshot index could not be read.".to_owned());
        }
        Err(crate::bounded_file::ReadError::TooLarge) => {
            return Err("The saved snapshot index exceeds the supported size.".to_owned());
        }
    };
    let index: SnapshotIndex = serde_json::from_slice(&bytes).map_err(|_| {
        "The saved snapshot index is invalid. Rebuild the index for this snapshot.".to_owned()
    })?;
    if index.schema_version != INDEX_SCHEMA_VERSION
        || index.indexer_version != INDEXER_VERSION
        || index.project_id != project_id
        || index.snapshot_id != snapshot_id
    {
        return Err("The saved snapshot index was produced by a different parser version. Rebuild it to continue.".to_owned());
    }
    Ok(Some(index))
}
