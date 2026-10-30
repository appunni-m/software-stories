//! Local, snapshot-scoped code retrieval. Syntax improves chunks; it never gates text search.
//!
//! This layer does not invoke AI, read a live working tree, or claim runtime/type resolution.
//! Immutable generations are published only after both metadata and Tantivy commit succeed.

use crate::{bounded_file, index, snapshot};
use grep_matcher::Matcher;
use grep_regex::{RegexMatcher, RegexMatcherBuilder};
use grep_searcher::{Searcher, sinks::UTF8};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap},
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tantivy::{
    Index, IndexWriter, Term,
    query::{BooleanQuery, BoostQuery, Occur, Query, TermQuery},
    schema::{
        FAST, Field, IndexRecordOption, STORED, STRING, Schema, TEXT, TantivyDocument,
        TextFieldIndexing, TextOptions, Value,
    },
    tokenizer::TokenStream,
};
use uuid::Uuid;

const VERSION: &str = "atlas-ranges-v6-english-stems-v1-identifiers-v2-policy-v2-stable-order-v1";
// Search identity/ordering fields do not change source policy or chunk layouts.
const CHUNK_VERSION: &str = "atlas-ranges-v1-identifiers-v1-policy-v2";
pub(crate) const CHUNK_BYTES: usize = 8192;
const MAX_CHUNKS: usize = 50_000;
const MAX_SOURCE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 32 * 1024 * 1024;
const MAX_CACHE_BYTES: usize = 4 * 1024 * 1024;
const MAX_CANDIDATES: usize = 256;
const GLOBAL_RANKING_CANDIDATE_LIMIT: usize = MAX_CANDIDATES * 2;
const GLOBAL_DIVERSIFIED_PATH_LIMIT: usize = 64;
pub(crate) const MAX_SEARCH_PAGE_SIZE: usize = 32;
const MAX_INDEX_OMISSION_PATHS: usize = 8;
pub(crate) const MAX_RANGE_BYTES: usize = 32_768;

/// Broad, unscoped natural-language searches work better as file discovery;
/// code-like and scoped searches should keep returning exact source ranges.
pub(crate) fn should_discover_files(query: &str, scope: Option<&str>) -> bool {
    let query = query.trim();
    let query_chars = query.chars().collect::<Vec<_>>();
    let has_code_syntax = query
        .chars()
        .any(|ch| matches!(ch, '/' | '\\' | '(' | ')' | '[' | ']' | '{' | '}' | ';'))
        || query_chars.iter().enumerate().any(|(index, ch)| {
            *ch == '.'
                && index > 0
                && index + 1 < query_chars.len()
                && query_chars[index - 1].is_alphanumeric()
                && query_chars[index + 1].is_alphanumeric()
        })
        || query.contains("::")
        || query.contains("->")
        || query.contains("=>")
        || query.contains("==")
        || query.contains('_')
        || query.split_whitespace().any(|word| {
            word.chars()
                .zip(word.chars().skip(1))
                .any(|(left, right)| left.is_lowercase() && right.is_uppercase())
        });
    scope.is_none() && !has_code_syntax && query.split_whitespace().count() >= 3
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) struct Budget<'a> {
    cancelled: &'a AtomicBool,
    deadline: Instant,
}

impl<'a> Budget<'a> {
    pub(crate) fn new(cancelled: &'a AtomicBool) -> Self {
        Self {
            cancelled,
            deadline: Instant::now() + Duration::from_secs(90),
        }
    }

    pub(crate) fn check(&self) -> Result<(), String> {
        if self.cancelled.load(Ordering::Acquire) {
            Err("Code search stopped.".to_owned())
        } else if Instant::now() >= self.deadline {
            Err("Code search reached its local time limit.".to_owned())
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SymbolContext {
    pub(crate) symbol_id: String,
    pub(crate) name: String,
    pub(crate) qualified_name: String,
    pub(crate) start_byte: usize,
    pub(crate) end_byte: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Chunk {
    start_byte: usize,
    end_byte: usize,
    start_line: usize,
    end_line: usize,
    chunk_sha256: String,
    // The innermost containing declaration survives splitting an oversized function.
    enclosing: Option<SymbolContext>,
    parent: Option<SymbolContext>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CodeRange {
    pub(crate) snapshot_id: String,
    pub(crate) path: String,
    pub(crate) source_sha256: String,
    pub(crate) start_byte: usize,
    /// Exclusive UTF-8 byte offset. Lines are one-based and inclusive.
    pub(crate) end_byte: usize,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) chunk_sha256: String,
    pub(crate) enclosing: Option<SymbolContext>,
    pub(crate) parent: Option<SymbolContext>,
    pub(crate) parser: Option<index::ParserVersion>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct FileCoverage {
    path: String,
    source_sha256: String,
    bytes: u64,
    chunks: usize,
    parser: Option<index::ParserVersion>,
    #[serde(default)]
    status: Option<index::IndexFileStatus>,
    reason: Option<String>,
    index_key: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Coverage {
    pub(crate) files_indexed: usize,
    pub(crate) files_unavailable: usize,
    #[serde(default)]
    pub(crate) source_byte_limited_files: usize,
    pub(crate) bytes_indexed: u64,
    pub(crate) chunks: usize,
    pub(crate) snapshot_paths_skipped: usize,
    pub(crate) syntax_files: usize,
    pub(crate) text_fallback_files: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IndexLimitOmissions {
    pub(crate) source_byte_limited_files: usize,
    pub(crate) range_limited_files: usize,
    pub(crate) files: Vec<IndexLimitOmission>,
    pub(crate) files_truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IndexLimitOmission {
    pub(crate) path: String,
    pub(crate) reason: String,
}

fn index_limit_omissions(
    files: &[FileCoverage],
    budget: &Budget<'_>,
) -> Result<IndexLimitOmissions, String> {
    let mut summary = IndexLimitOmissions::default();
    let mut range_limited = Vec::new();
    let mut source_byte_limited = Vec::new();
    for (index, file) in files.iter().enumerate() {
        if index % 128 == 0 {
            budget.check()?;
        }
        match file.reason.as_deref() {
            Some("localRangeLimit") => {
                summary.range_limited_files += 1;
                if range_limited.len() < MAX_INDEX_OMISSION_PATHS
                    && safe_path(&file.path)
                    && !snapshot::looks_secret_like(&file.path, &[])
                {
                    range_limited.push(IndexLimitOmission {
                        path: file.path.clone(),
                        reason: "localRangeLimit".to_owned(),
                    });
                }
            }
            Some("localSourceByteLimit") => {
                summary.source_byte_limited_files += 1;
                if source_byte_limited.len() < MAX_INDEX_OMISSION_PATHS
                    && safe_path(&file.path)
                    && !snapshot::looks_secret_like(&file.path, &[])
                {
                    source_byte_limited.push(IndexLimitOmission {
                        path: file.path.clone(),
                        reason: "localSourceByteLimit".to_owned(),
                    });
                }
            }
            _ => {}
        }
    }
    summary.files = range_limited
        .into_iter()
        .chain(source_byte_limited)
        .take(MAX_INDEX_OMISSION_PATHS)
        .collect();
    summary.files_truncated = summary.files.len()
        < summary
            .range_limited_files
            .saturating_add(summary.source_byte_limited_files);
    Ok(summary)
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BuildStats {
    pub(crate) elapsed_ms: u64,
    pub(crate) source_bytes_verified: u64,
    pub(crate) files_chunked: usize,
    pub(crate) files_chunk_cache_reused: usize,
    pub(crate) files_documents_reused: usize,
    pub(crate) documents_reused: usize,
    pub(crate) documents_added: usize,
    pub(crate) documents_deleted: usize,
    pub(crate) segment_bytes_copied: u64,
    pub(crate) incremental_base_snapshot: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Metadata {
    version: String,
    fingerprint: String,
    snapshot_id: String,
    project_id: String,
    ranges: Vec<CodeRange>,
    document_keys: Vec<String>,
    files: Vec<FileCoverage>,
    coverage: Coverage,
    build: BuildStats,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Pointer {
    generation: String,
    fingerprint: String,
    metadata_sha256: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChunkCache {
    version: String,
    key: String,
    chunks_sha256: String,
    chunks: Vec<Chunk>,
}

pub(crate) struct PreparedIndex {
    index: Index,
    metadata: Metadata,
    ranges_by_key: HashMap<String, usize>,
    generation: PathBuf,
    pub(crate) reused_generation: bool,
    // Drop after Index: its directory remains protected for every later reader().
    _lease: cache::ReadLease,
}

#[derive(PartialEq, Eq)]
pub(crate) struct CacheTag {
    manifest_sha256: String,
    syntax: Option<(u64, std::time::SystemTime)>,
    pointer: Option<(u64, std::time::SystemTime)>,
}

pub(crate) fn cache_tag(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
) -> Result<CacheTag, String> {
    let directory = root(
        storage,
        &manifest.summary.project_id,
        &manifest.summary.snapshot_id,
    );
    let pointer = fs::metadata(directory.join("current.json"))
        .ok()
        .and_then(|m| Some((m.len(), m.modified().ok()?)));
    Ok(CacheTag {
        manifest_sha256: digest(&json(manifest)?),
        syntax: index::snapshot_index_stamp(
            storage,
            &manifest.summary.project_id,
            &manifest.summary.snapshot_id,
        ),
        pointer,
    })
}

impl CacheTag {
    pub(crate) fn retain_syntax_from(&mut self, original: &Self) {
        self.syntax = original.syntax;
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchHit {
    pub(crate) range: CodeRange,
    pub(crate) line: usize,
    pub(crate) text: String,
    pub(crate) exact_identifier: bool,
    pub(crate) declaration_match: bool,
    pub(crate) score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) query_coverage: Option<QueryCoverage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueryCoverage {
    pub(crate) matched_terms: Vec<String>,
    pub(crate) total_terms: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResult {
    pub(crate) matches: Vec<SearchHit>,
    pub(crate) limited: bool,
    pub(crate) next_offset: Option<usize>,
    pub(crate) coverage: Coverage,
    pub(crate) index_limit_omissions: IndexLimitOmissions,
    pub(crate) reused_generation: bool,
    pub(crate) build: BuildStats,
    pub(crate) elapsed_ms: u64,
    pub(crate) result_bytes: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RangeRead {
    pub(crate) range: CodeRange,
    pub(crate) contents: String,
}

pub(crate) fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.contains('\\')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|p| !matches!(p, "" | "." | "..") && !p.contains(':'))
}

pub(crate) fn exclusion(file: &snapshot::CapturedFile) -> Option<&'static str> {
    if !safe_path(&file.path) {
        return Some("invalidPath");
    }
    if file.path.split('/').any(|part| {
        matches!(
            part,
            ".git"
                | "deprecated"
                | "node_modules"
                | "vendor"
                | "target"
                | "dist"
                | "build"
                | "coverage"
                | "out"
                | "generated"
                | ".next"
                | ".nuxt"
                | ".turbo"
                | ".cache"
                | ".venv"
                | "venv"
        )
    }) {
        return Some("excludedPath");
    }
    if file.kind != snapshot::InventoryPathKind::File {
        return Some("notFile");
    }
    if file.secret_like || snapshot::looks_secret_like(&file.path, &[]) {
        return Some("secretLike");
    }
    if !matches!(
        file.content_kind,
        snapshot::CapturedContentKind::Text | snapshot::CapturedContentKind::Unknown
    ) {
        return Some("unavailableText");
    }
    None
}

pub(crate) fn source(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    path: &str,
) -> Result<snapshot::SnapshotSource, String> {
    source_with_check(storage, manifest, path, &mut || Ok(()))
}

fn source_with_budget(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    path: &str,
    budget: &Budget<'_>,
) -> Result<snapshot::SnapshotSource, String> {
    source_with_check(storage, manifest, path, &mut || budget.check())
}

fn source_with_check(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    path: &str,
    check: &mut impl FnMut() -> Result<(), String>,
) -> Result<snapshot::SnapshotSource, String> {
    let file = manifest
        .files
        .iter()
        .find(|f| f.path == path)
        .ok_or("This path is not in the selected snapshot.")?;
    if exclusion(file).is_some() {
        return Err("This captured path is excluded from code retrieval.".to_owned());
    }
    let source =
        snapshot::read_snapshot_source_from_manifest_checked(storage, manifest, path, check)?;
    if snapshot::looks_secret_like(path, source.contents.as_bytes()) {
        return Err("This captured text is excluded by secret filtering.".to_owned());
    }
    Ok(source)
}

fn schema() -> Schema {
    let mut schema = Schema::builder();
    schema.add_text_field("range_key", STRING | STORED | FAST);
    schema.add_text_field("source_path", STRING | FAST);
    schema.add_u64_field("start_byte", FAST);
    schema.add_u64_field("end_byte", FAST);
    schema.add_text_field("body", TEXT);
    schema.add_text_field(
        "body_stem",
        TextOptions::default().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("en_stem")
                .set_index_option(IndexRecordOption::WithFreqs),
        ),
    );
    schema.add_text_field("path", TEXT);
    schema.add_text_field("scope", STRING);
    schema.add_text_field("exact", STRING);
    schema.add_text_field("declaration", STRING);
    schema.add_text_field("symbol", TEXT);
    schema.add_text_field("identifier_parts", STRING);
    schema.build()
}

fn field(index: &Index, name: &str) -> Result<Field, String> {
    index
        .schema()
        .get_field(name)
        .map_err(|_| "The code index schema is invalid. Rebuild it.".to_owned())
}

fn english_stems(index: &Index, text: &str) -> Result<BTreeSet<String>, String> {
    let mut analyzer = index
        .tokenizers()
        .get("en_stem")
        .ok_or("English code-search tokenizer is unavailable.")?;
    let mut stream = analyzer.token_stream(text);
    let mut stems = BTreeSet::new();
    while stream.advance() {
        let stem = stream.token().text.clone();
        if !stem.is_empty() {
            stems.insert(stem);
        }
    }
    Ok(stems)
}

/// Original spellings plus identifier parts; the index has a separate English inflection field.
fn terms(text: &str) -> Vec<String> {
    let mut output = BTreeSet::new();
    for word in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if word.is_empty() || word.len() > 240 {
            continue;
        }
        output.insert(word.to_lowercase());
        let chars: Vec<char> = word.chars().collect();
        let mut start = 0;
        for i in 0..chars.len() {
            let boundary = chars[i] == '_'
                || (i > start
                    && chars[i].is_uppercase()
                    && (chars[i - 1].is_lowercase()
                        || (chars[i - 1].is_uppercase()
                            && chars.get(i + 1).is_some_and(|c| c.is_lowercase()))));
            if boundary {
                if start < i {
                    output.insert(chars[start..i].iter().collect::<String>().to_lowercase());
                }
                start = if chars[i] == '_' { i + 1 } else { i };
            }
        }
        if start < chars.len() {
            output.insert(chars[start..].iter().collect::<String>().to_lowercase());
        }
    }
    output.into_iter().collect()
}

/// Keep broad natural-language relevance focused on repository terms instead
/// of question scaffolding. Exact search still uses every original term.
fn coverage_terms(query: &str, scope: Option<&str>, words: &[String]) -> Vec<String> {
    if !should_discover_files(query, scope) {
        return words.to_vec();
    }
    words
        .iter()
        .filter(|word| {
            word.len() > 2
                && !crate::source_discovery::QUERY_STOPWORDS
                .iter()
                .any(|stopword| word.eq_ignore_ascii_case(stopword))
        })
        .cloned()
        .collect()
}

fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0)
        .chain(
            text.bytes()
                .enumerate()
                .filter_map(|(i, b)| (b == b'\n').then_some(i + 1)),
        )
        .collect()
}

fn line_at(starts: &[usize], byte: usize) -> usize {
    starts.partition_point(|&start| start <= byte)
}

fn context(symbol: &index::IndexedSymbol) -> SymbolContext {
    SymbolContext {
        symbol_id: symbol.symbol_id.clone(),
        name: symbol.name.clone(),
        qualified_name: symbol.qualified_name.clone(),
        start_byte: symbol.anchor.start_byte,
        end_byte: symbol.anchor.end_byte,
    }
}

fn chunks(
    text: &str,
    symbols: &[&index::IndexedSymbol],
    budget: &Budget<'_>,
) -> Result<Vec<Chunk>, String> {
    let starts = line_starts(text);
    let mut boundaries = symbols
        .iter()
        .flat_map(|s| [s.anchor.start_byte, s.anchor.end_byte])
        .filter(|&p| p <= text.len() && text.is_char_boundary(p))
        .collect::<Vec<_>>();
    boundaries.sort_unstable();
    boundaries.dedup();
    let mut result = vec![];
    let mut start = 0;
    while start < text.len() {
        budget.check()?;
        let mut end = (start + CHUNK_BYTES).min(text.len());
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        if end < text.len() {
            let floor = start + CHUNK_BYTES / 2;
            if let Some(&boundary) = boundaries.iter().rev().find(|&&p| p >= floor && p <= end) {
                end = boundary;
            } else if let Some(newline) = text[start..end]
                .rfind('\n')
                .filter(|&p| p >= CHUNK_BYTES / 2)
            {
                end = start + newline + 1;
            }
        }
        let mut containing = symbols
            .iter()
            .copied()
            .filter(|s| s.anchor.start_byte <= start && s.anchor.end_byte >= end)
            .collect::<Vec<_>>();
        containing.sort_by_key(|s| {
            (
                s.anchor.end_byte - s.anchor.start_byte,
                s.symbol_id.as_str(),
            )
        });
        result.push(Chunk {
            start_byte: start,
            end_byte: end,
            start_line: line_at(&starts, start),
            end_line: line_at(&starts, end - 1),
            chunk_sha256: digest(text[start..end].as_bytes()),
            enclosing: containing.first().map(|s| context(s)),
            parent: containing.get(1).map(|s| context(s)),
        });
        start = end;
    }
    Ok(result)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "Local code index could not be saved.".to_owned())?;
    file.write_all(bytes)
        .and_then(|_| file.sync_all())
        .map_err(|_| "Local code index could not be saved.".to_owned())
}

fn json<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value)
        .map_err(|_| "Local code index metadata could not be encoded.".to_owned())
}

fn root(storage: &Path, project: &str, snapshot_id: &str) -> PathBuf {
    storage
        .join("repository-context")
        .join(digest(project.as_bytes()))
        .join(digest(snapshot_id.as_bytes()))
}

fn load_generation(
    cache: &cache::Cache,
    directory: &Path,
    fingerprint: Option<&str>,
    budget: &Budget<'_>,
) -> Option<PreparedIndex> {
    budget.check().ok()?;
    let pointer: Pointer =
        serde_json::from_slice(&bounded_file::read(&directory.join("current.json"), 4096).ok()?)
            .ok()?;
    if fingerprint.is_some_and(|f| pointer.fingerprint != f)
        || Uuid::parse_str(&pointer.generation).ok()?.to_string() != pointer.generation
    {
        return None;
    }
    let generation = directory.join(&pointer.generation);
    let bytes = bounded_file::read(&generation.join("ranges.json"), MAX_METADATA_BYTES).ok()?;
    if digest(&bytes) != pointer.metadata_sha256 {
        return None;
    }
    let metadata: Metadata = serde_json::from_slice(&bytes).ok()?;
    if metadata.version != VERSION
        || metadata.fingerprint != pointer.fingerprint
        || metadata.ranges.len() > MAX_CHUNKS
        || directory.file_name()?.to_str()? != digest(metadata.snapshot_id.as_bytes())
        || directory.parent()?.file_name()?.to_str()? != digest(metadata.project_id.as_bytes())
    {
        return None;
    }
    if metadata.ranges.iter().any(|r| {
        !safe_path(&r.path)
            || r.snapshot_id != metadata.snapshot_id
            || r.start_byte >= r.end_byte
            || r.end_byte.saturating_sub(r.start_byte) > CHUNK_BYTES
            || r.start_line == 0
            || r.end_line < r.start_line
            || ![&r.source_sha256, &r.chunk_sha256]
                .iter()
                .all(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
    }) {
        return None;
    }
    let index = Index::open_in_dir(&generation).ok()?;
    if index.schema() != schema() {
        return None;
    }
    // A corrupt managed-file inventory must never become a writer's GC input.
    let managed = index.directory().list_managed_files();
    if managed.len() > 100_000 || managed.iter().any(|p| !incremental::safe_component(p)) {
        return None;
    }
    if !index.validate_checksum().ok()?.is_empty() {
        return None;
    }
    let reader = index.reader().ok()?;
    let searcher = reader.searcher();
    if searcher.num_docs() as usize != metadata.ranges.len() {
        return None;
    }
    let ranges_by_key = incremental::lookup(&metadata)?;
    let range_key = field(&index, "range_key").ok()?;
    let mut seen = BTreeSet::new();
    for (ordinal, segment) in searcher.segment_readers().iter().enumerate() {
        let range_keys = segment.fast_fields().str("range_key").ok()??;
        let paths = segment.fast_fields().str("source_path").ok()??;
        let starts = segment.fast_fields().u64("start_byte").ok()?;
        let ends = segment.fast_fields().u64("end_byte").ok()?;
        let mut path = String::new();
        let mut fast_range_key = String::new();
        for doc_id in segment.doc_ids_alive() {
            budget.check().ok()?;
            let doc = searcher
                .doc::<TantivyDocument>(tantivy::DocAddress::new(ordinal as u32, doc_id))
                .ok()?;
            let key = doc.get_first(range_key)?.as_str()?;
            let range = metadata.ranges.get(*ranges_by_key.get(key)?)?;
            let mut range_key_values = range_keys.ords().values_for_doc(doc_id);
            let mut path_values = paths.ords().values_for_doc(doc_id);
            let mut start_values = starts.values_for_doc(doc_id);
            let mut end_values = ends.values_for_doc(doc_id);
            if !range_keys
                .ord_to_str(range_key_values.next()?, &mut fast_range_key)
                .ok()?
                || range_key_values.next().is_some()
                || fast_range_key != key
                || !paths.ord_to_str(path_values.next()?, &mut path).ok()?
                || path_values.next().is_some()
                || start_values.next()? != range.start_byte as u64
                || start_values.next().is_some()
                || end_values.next()? != range.end_byte as u64
                || end_values.next().is_some()
                || path != range.path
                || !seen.insert(key.to_owned())
            {
                return None;
            }
        }
    }
    Some(PreparedIndex {
        index,
        metadata,
        ranges_by_key,
        generation: generation.clone(),
        reused_generation: true,
        _lease: cache.lease(&generation).ok()?,
    })
}

struct Staging {
    path: PathBuf,
    published: bool,
}
impl Drop for Staging {
    fn drop(&mut self) {
        if !self.published {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}

fn fingerprint(
    manifest: &snapshot::SnapshotManifest,
    syntax: Option<&index::SnapshotIndex>,
) -> Result<String, String> {
    Ok(digest(&json(&(VERSION, manifest, syntax))?))
}

pub(crate) fn prepare(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    syntax: Option<&index::SnapshotIndex>,
    budget: &Budget<'_>,
) -> Result<PreparedIndex, String> {
    budget.check()?;
    if manifest.files.len() > 25_000 {
        return Err("This snapshot exceeds the local code index path limit.".to_owned());
    }
    if syntax.is_some_and(|s| {
        s.project_id != manifest.summary.project_id || s.snapshot_id != manifest.summary.snapshot_id
    }) {
        return Err("The syntax index belongs to a different snapshot.".to_owned());
    }
    let cache = cache::Cache::open(
        storage,
        &manifest.summary.project_id,
        &manifest.summary.snapshot_id,
        budget,
    )?;
    prepare_cached(storage, manifest, syntax, budget, cache)
}

/// Search the existing snapshot index without triggering a generation build.
/// Source discovery uses this to rank its bounded AI-review shortlist from
/// actual captured code while keeping discovery startup independent from index
/// creation. A missing or stale generation is a normal fallback condition.
pub(crate) fn search_cached_files(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    syntax: Option<&index::SnapshotIndex>,
    query: &str,
    limit: usize,
    budget: &Budget<'_>,
) -> Result<Option<SearchResult>, String> {
    budget.check()?;
    if manifest.files.len() > 25_000 {
        return Err("This snapshot exceeds the local code index path limit.".to_owned());
    }
    let fingerprint = fingerprint(manifest, syntax)?;
    let Some(cache) = cache::Cache::open_reader(
        storage,
        &manifest.summary.project_id,
        &manifest.summary.snapshot_id,
        budget,
    )?
    else {
        return Ok(None);
    };
    let Some(prepared) = load_generation(&cache, &cache.directory, Some(&fingerprint), budget)
    else {
        return Ok(None);
    };
    search_files_with_mode_page(
        storage,
        manifest,
        &prepared,
        query,
        None,
        limit,
        0,
        budget,
        SearchMode::Code,
    )
    .map(Some)
}

fn prepare_cached(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    syntax: Option<&index::SnapshotIndex>,
    budget: &Budget<'_>,
    mut cache: cache::Cache,
) -> Result<PreparedIndex, String> {
    let fingerprint = fingerprint(manifest, syntax)?;
    let directory = cache.directory.clone();
    if let Some(prepared) = load_generation(&cache, &directory, Some(&fingerprint), budget) {
        budget.check()?;
        return Ok(prepared);
    }
    let started = Instant::now();
    let base = incremental::base(&cache, budget)?;
    let generation_id = Uuid::new_v4().to_string();
    let mut staging = Staging {
        path: directory.join(format!("staging-{generation_id}")),
        published: false,
    };
    fs::create_dir(&staging.path)
        .map_err(|_| "Local code index storage is unavailable.".to_owned())?;
    let copied = if let Some(base) = &base {
        incremental::copy(base, &staging.path, budget)?
    } else {
        0
    };
    let index = if base.is_some() {
        Index::open_in_dir(&staging.path)
    } else {
        Index::create_in_dir(&staging.path, schema())
    }
    .map_err(|_| "Local code index could not be created.".to_owned())?;
    let mut writer: IndexWriter = index
        .writer_with_num_threads(1, 30_000_000)
        .map_err(|_| "Local code index writer is unavailable.".to_owned())?;
    // BM25 scores can shift when the same corpus is split across a different
    // number of Tantivy segments. A canonical single segment keeps broad
    // result ordering stable between cold and incremental generations.
    writer.set_merge_policy(Box::new(tantivy::indexer::NoMergePolicy));
    let range_key = field(&index, "range_key")?;
    let source_path = field(&index, "source_path")?;
    let start_byte = field(&index, "start_byte")?;
    let end_byte = field(&index, "end_byte")?;
    let body = field(&index, "body")?;
    let body_stem = field(&index, "body_stem")?;
    let path_field = field(&index, "path")?;
    let exact = field(&index, "exact")?;
    let scope = field(&index, "scope")?;
    let declaration = field(&index, "declaration")?;
    let symbol_field = field(&index, "symbol")?;
    let identifier_parts_field = field(&index, "identifier_parts")?;
    let mut metadata = Metadata {
        version: VERSION.to_owned(),
        fingerprint: fingerprint.clone(),
        project_id: manifest.summary.project_id.clone(),
        snapshot_id: manifest.summary.snapshot_id.clone(),
        ranges: vec![],
        document_keys: vec![],
        files: vec![],
        coverage: Coverage {
            snapshot_paths_skipped: manifest.skipped.len(),
            ..Default::default()
        },
        build: BuildStats {
            segment_bytes_copied: copied,
            incremental_base_snapshot: base.as_ref().map(|b| b.metadata.snapshot_id.clone()),
            ..BuildStats::default()
        },
    };
    let mut files = manifest.files.iter().collect::<Vec<_>>();
    // Keep implementation searchable first when the bounded local index cannot
    // admit every captured text file. Path order remains deterministic inside
    // each group.
    files.sort_by(|left, right| {
        is_code_path(&right.path)
            .cmp(&is_code_path(&left.path))
            .then_with(|| left.path.cmp(&right.path))
    });
    let mut symbols_by_path: HashMap<&str, Vec<&index::IndexedSymbol>> = HashMap::new();
    if let Some(syntax) = syntax {
        for symbol in &syntax.symbols {
            symbols_by_path
                .entry(&symbol.path)
                .or_default()
                .push(symbol);
        }
    }
    let mut admitted_bytes = 0u64;
    let mut admitted_paths = BTreeSet::new();
    let base_files = base
        .as_ref()
        .map(|b| {
            b.metadata
                .files
                .iter()
                .map(|f| (f.path.as_str(), f))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    for file in files {
        budget.check()?;
        let mut coverage = FileCoverage {
            path: file.path.clone(),
            source_sha256: file.sha256.clone(),
            bytes: file.size_bytes,
            chunks: 0,
            parser: None,
            status: None,
            reason: exclusion(file).map(str::to_owned),
            index_key: None,
        };
        if coverage.reason.is_some() {
            metadata.files.push(coverage);
            continue;
        }
        let next_admitted_bytes = admitted_bytes.saturating_add(file.size_bytes);
        if next_admitted_bytes > MAX_SOURCE_BYTES {
            coverage.reason = Some("localSourceByteLimit".to_owned());
            metadata.coverage.source_byte_limited_files += 1;
            metadata.files.push(coverage);
            continue;
        }
        admitted_bytes = next_admitted_bytes;
        let source = match source_with_budget(storage, manifest, &file.path, budget) {
            Ok(source) => source,
            Err(_) => {
                budget.check()?;
                admitted_bytes = admitted_bytes.saturating_sub(file.size_bytes);
                coverage.reason = Some("unavailableOrFilteredText".to_owned());
                metadata.files.push(coverage);
                continue;
            }
        };
        let next_verified_bytes = metadata
            .build
            .source_bytes_verified
            .saturating_add(source.size_bytes);
        if next_verified_bytes > MAX_SOURCE_BYTES {
            admitted_bytes = admitted_bytes.saturating_sub(file.size_bytes);
            coverage.reason = Some("localSourceByteLimit".to_owned());
            metadata.coverage.source_byte_limited_files += 1;
            metadata.files.push(coverage);
            continue;
        }
        metadata.build.source_bytes_verified = next_verified_bytes;
        let symbols = symbols_by_path
            .get(file.path.as_str())
            .map(Vec::as_slice)
            .unwrap_or(&[])
            .iter()
            .copied()
            .filter(|s| {
                s.anchor.path == file.path
                    && s.anchor.source_sha256 == file.sha256
                    && s.anchor.start_byte < s.anchor.end_byte
                    && s.anchor.end_byte <= source.contents.len()
                    && source.contents.is_char_boundary(s.anchor.start_byte)
                    && source.contents.is_char_boundary(s.anchor.end_byte)
            })
            .collect::<Vec<_>>();
        let indexed_file = syntax
            .and_then(|s| {
                s.files
                    .iter()
                    .find(|f| f.path == file.path && f.source_sha256 == file.sha256)
        });
        coverage.parser = indexed_file.and_then(|file| file.parser.clone());
        coverage.status = indexed_file.map(|file| file.status.clone());
        let key = digest(&json(&(
            CHUNK_VERSION,
            &source.sha256,
            &symbols,
            &coverage.parser,
        ))?);
        let cache_path = cache.chunk_path(&key);
        let cached = bounded_file::read(&cache_path, MAX_CACHE_BYTES)
            .ok()
            .and_then(|b| serde_json::from_slice::<ChunkCache>(&b).ok())
            .filter(|c| {
                c.version == CHUNK_VERSION
                    && c.key == key
                    && json(&c.chunks).is_ok_and(|bytes| digest(&bytes) == c.chunks_sha256)
            });
        // Validate the complete partition and coordinates against freshly hash-checked source.
        let local_chunks = if let Some(cached_chunk) =
            cached.filter(|cache| valid_chunks(&cache.chunks, &source.contents))
        {
            metadata.build.files_chunk_cache_reused += 1;
            cache.chunk_used(&key);
            cached_chunk.chunks
        } else {
            metadata.build.files_chunked += 1;
            let chunks = chunks(&source.contents, &symbols, budget)?;
            let encoded = json(&ChunkCache {
                version: CHUNK_VERSION.to_owned(),
                key: key.clone(),
                chunks_sha256: digest(&json(&chunks)?),
                chunks: chunks.clone(),
            })?;
            if encoded.len() <= MAX_CACHE_BYTES {
                cache.save_chunk(&key, &encoded, budget)?;
            }
            chunks
        };
        if metadata.ranges.len().saturating_add(local_chunks.len()) > MAX_CHUNKS {
            // Keep the generation useful when a valid snapshot exceeds the
            // aggregate range budget. This file remains in the inventory with
            // an explicit reason, while later files may still fit the budget.
            // Since it is not admitted below, an incremental base's postings
            // for this path are removed by the normal deleted-file pass.
            coverage.reason = Some("localRangeLimit".to_owned());
            metadata.files.push(coverage);
            continue;
        }
        coverage.chunks = local_chunks.len();
        let file_key =
            incremental::file_key(&file.path, &source.sha256, &coverage.parser, &symbols)?;
        let keys = local_chunks
            .iter()
            .map(|c| {
                incremental::document_key(&file_key, c.start_byte, c.end_byte, &c.chunk_sha256)
            })
            .collect::<Vec<_>>();
        coverage.index_key = Some(file_key);
        let old_file = base_files.get(file.path.as_str());
        let reused = old_file.is_some_and(|old| {
            old.index_key == coverage.index_key && old.chunks == local_chunks.len()
        }) && base
            .as_ref()
            .is_some_and(|base| keys.iter().all(|key| base.ranges_by_key.contains_key(key)));
        admitted_paths.insert(file.path.as_str());
        if reused {
            metadata.build.files_documents_reused += 1;
            metadata.build.documents_reused += local_chunks.len();
        } else if let Some(old) = old_file.filter(|old| old.chunks > 0) {
            writer.delete_term(Term::from_field_text(source_path, &file.path));
            metadata.build.documents_deleted += old.chunks;
        }
        for (chunk, key) in local_chunks.into_iter().zip(keys) {
            budget.check()?;
            if !reused {
                let text = &source.contents[chunk.start_byte..chunk.end_byte];
                let mut doc = TantivyDocument::default();
                doc.add_text(range_key, &key);
                doc.add_text(source_path, &file.path);
                doc.add_u64(start_byte, chunk.start_byte as u64);
                doc.add_u64(end_byte, chunk.end_byte as u64);
                let body_terms = terms(text).join(" ");
                doc.add_text(body, &body_terms);
                doc.add_text(body_stem, body_terms);
                doc.add_text(path_field, terms(&file.path).join(" "));
                let identifier_keys = text
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .filter(|word| !word.is_empty() && word.len() <= 240)
                    .filter_map(identifier_parts_key)
                    .collect::<BTreeSet<_>>();
                for identifier_key in identifier_keys {
                    doc.add_text(identifier_parts_field, &identifier_key);
                }
                for word in text
                    .split(|c: char| !c.is_alphanumeric() && c != '_')
                    .filter(|w| !w.is_empty() && w.len() <= 240)
                    .map(str::to_lowercase)
                    .collect::<BTreeSet<_>>()
                {
                    doc.add_text(exact, word);
                }
                for symbol in &symbols {
                    if symbol.anchor.start_byte >= chunk.start_byte
                        && symbol.anchor.start_byte < chunk.end_byte
                    {
                        doc.add_text(exact, symbol.qualified_name.to_lowercase());
                        doc.add_text(exact, symbol.name.to_lowercase());
                        doc.add_text(declaration, symbol.qualified_name.to_lowercase());
                        doc.add_text(declaration, symbol.name.to_lowercase());
                        doc.add_text(symbol_field, terms(&symbol.qualified_name).join(" "));
                    }
                }
                doc.add_text(scope, &file.path);
                for (offset, _) in file.path.match_indices('/') {
                    doc.add_text(scope, &file.path[..offset]);
                }
                writer
                    .add_document(doc)
                    .map_err(|_| "A captured code range could not be indexed.".to_owned())?;
                metadata.build.documents_added += 1;
            }
            metadata.document_keys.push(key);
            metadata.ranges.push(CodeRange {
                snapshot_id: manifest.summary.snapshot_id.clone(),
                path: file.path.clone(),
                source_sha256: file.sha256.clone(),
                start_byte: chunk.start_byte,
                end_byte: chunk.end_byte,
                start_line: chunk.start_line,
                end_line: chunk.end_line,
                chunk_sha256: chunk.chunk_sha256,
                enclosing: chunk.enclosing,
                parent: chunk.parent,
                parser: coverage.parser.clone(),
            });
        }
        metadata.files.push(coverage);
    }
    for old in base_files
        .values()
        .filter(|old| old.chunks > 0 && !admitted_paths.contains(old.path.as_str()))
    {
        budget.check()?;
        writer.delete_term(Term::from_field_text(source_path, &old.path));
        metadata.build.documents_deleted += old.chunks;
    }
    budget.check()?;
    writer
        .commit()
        .map_err(|_| "Local code index could not be committed.".to_owned())?;
    let segments = writer
        .index()
        .load_metas()
        .map_err(|_| "Local code index could not be opened.".to_owned())?
        .segments
        .into_iter()
        .map(|segment| segment.id())
        .collect::<Vec<_>>();
    if segments.len() > 1 {
        budget.check()?;
        writer
            .merge(&segments)
            .wait()
            .map_err(|_| "Local code index could not be consolidated.".to_owned())?;
    }
    writer
        .wait_merging_threads()
        .map_err(|_| "Local code index could not finish publishing.".to_owned())?;
    budget.check()?;
    for file in &metadata.files {
        if file.reason.is_some() {
            metadata.coverage.files_unavailable += 1;
        } else {
            metadata.coverage.files_indexed += 1;
            metadata.coverage.bytes_indexed += file.bytes;
            if file.parser.is_some() {
                metadata.coverage.syntax_files += 1;
            } else {
                metadata.coverage.text_fallback_files += 1;
            }
        }
    }
    metadata.coverage.chunks = metadata.ranges.len();
    metadata.build.elapsed_ms = started.elapsed().as_millis() as u64;
    let ranges_by_key =
        incremental::lookup(&metadata).ok_or("Local code range identities are invalid.")?;
    let encoded = json(&metadata)?;
    if encoded.len() > MAX_METADATA_BYTES {
        return Err("Local code index metadata exceeds its size limit.".to_owned());
    }
    write_new(&staging.path.join("ranges.json"), &encoded)?;
    let pointer = Pointer {
        generation: generation_id.clone(),
        fingerprint,
        metadata_sha256: digest(&encoded),
    };
    let previous = bounded_file::read(&directory.join("current.json"), 4096)
        .ok()
        .and_then(|b| serde_json::from_slice::<Pointer>(&b).ok());
    // Reclaim only unleased older generations. The current pointer survives every
    // fallible capacity/build step, including an all-readers-pinned cache.
    cache.trim_generations(
        &generation_id,
        previous.as_ref().map(|p| p.generation.as_str()),
        budget,
    )?;
    let encoded_pointer = json(&pointer)?;
    cache.reserve_publication(&staging.path, encoded_pointer.len(), budget)?;
    // No reader can see staging. Rename on the same volume, then replace the tiny pointer.
    let published = directory.join(&generation_id);
    fs::rename(&staging.path, &published)
        .map_err(|_| "Local code index could not be published.".to_owned())?;
    staging.path = published.clone();
    let index = Index::open_in_dir(&published)
        .map_err(|_| "Published code index is unavailable.".to_owned())?;
    let lease = cache.lease(&published)?;
    let temporary = directory.join(format!("pointer-{generation_id}.tmp"));
    if let Err(error) = write_new(&temporary, &encoded_pointer).and_then(|_| {
        budget.check()?;
        fs::rename(&temporary, directory.join("current.json"))
            .map_err(|_| "Local code index could not be published.".to_owned())
    }) {
        let _ = fs::remove_file(temporary);
        return Err(error);
    }
    staging.published = true;
    Ok(PreparedIndex {
        index,
        ranges_by_key,
        generation: published,
        metadata,
        reused_generation: false,
        _lease: lease,
    })
}

fn valid_chunks(chunks: &[Chunk], text: &str) -> bool {
    let starts = line_starts(text);
    let mut cursor = 0;
    for chunk in chunks {
        if chunk.start_byte != cursor
            || chunk.start_byte >= chunk.end_byte
            || chunk.end_byte > text.len()
            || chunk.end_byte - chunk.start_byte > CHUNK_BYTES
            || !text.is_char_boundary(chunk.start_byte)
            || !text.is_char_boundary(chunk.end_byte)
        {
            return false;
        }
        if chunk.start_line != line_at(&starts, chunk.start_byte)
            || chunk.end_line != line_at(&starts, chunk.end_byte - 1)
            || chunk.chunk_sha256 != digest(text[chunk.start_byte..chunk.end_byte].as_bytes())
        {
            return false;
        }
        cursor = chunk.end_byte;
    }
    cursor == text.len()
}

pub(crate) fn read_range(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    path: &str,
    expected_sha256: &str,
    start_byte: usize,
    end_byte: usize,
) -> Result<RangeRead, String> {
    if start_byte >= end_byte || end_byte.saturating_sub(start_byte) > MAX_RANGE_BYTES {
        return Err("Read a nonempty code range of at most 32,768 bytes.".to_owned());
    }
    let source = source(storage, manifest, path)?;
    if source.sha256 != expected_sha256 {
        return Err("This code range belongs to a different source version.".to_owned());
    }
    range_from_source(&source, start_byte, end_byte)
}

pub(crate) fn range_from_source(
    source: &snapshot::SnapshotSource,
    start_byte: usize,
    end_byte: usize,
) -> Result<RangeRead, String> {
    if start_byte >= end_byte || end_byte.saturating_sub(start_byte) > MAX_RANGE_BYTES {
        return Err("Read a nonempty code range of at most 32,768 bytes.".to_owned());
    }
    let starts = line_starts(&source.contents);
    range_from_source_at_lines(source, start_byte, end_byte, &starts)
}

fn range_from_source_at_lines(
    source: &snapshot::SnapshotSource,
    start_byte: usize,
    end_byte: usize,
    starts: &[usize],
) -> Result<RangeRead, String> {
    if start_byte >= end_byte || end_byte.saturating_sub(start_byte) > MAX_RANGE_BYTES {
        return Err("Read a nonempty code range of at most 32,768 bytes.".to_owned());
    }
    let contents = source
        .contents
        .get(start_byte..end_byte)
        .ok_or("The code range must lie on UTF-8 boundaries inside this file.")?
        .to_owned();
    Ok(RangeRead {
        range: CodeRange {
            snapshot_id: source.snapshot_id.clone(),
            path: source.path.clone(),
            source_sha256: source.sha256.clone(),
            start_byte,
            end_byte,
            start_line: line_at(&starts, start_byte),
            end_line: line_at(&starts, end_byte - 1),
            chunk_sha256: digest(contents.as_bytes()),
            enclosing: None,
            parent: None,
            parser: None,
        },
        contents,
    })
}

fn term_query(field: Field, value: &str) -> Box<dyn Query> {
    Box::new(TermQuery::new(
        Term::from_field_text(field, value),
        IndexRecordOption::WithFreqs,
    ))
}

fn scoped(
    query: Box<dyn Query>,
    index: &Index,
    path: Option<&str>,
) -> Result<Box<dyn Query>, String> {
    if let Some(path) = path {
        if !safe_path(path) {
            return Err("Search within a captured file or folder.".to_owned());
        }
        Ok(Box::new(BooleanQuery::new(vec![
            (Occur::Must, query),
            (Occur::Must, term_query(field(index, "scope")?, path)),
        ])))
    } else {
        Ok(query)
    }
}

/// Regex is opt-in; existing callers retain ranked identifier and literal search.
#[derive(Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum SearchMode {
    #[default]
    Code,
    Regex,
}

pub(crate) const INVALID_LITERAL_SEARCH_QUERY: &str = "Code search terms are invalid.";

fn pattern_matcher(query: &str, mode: SearchMode) -> Result<RegexMatcher, String> {
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(true)
        .fixed_strings(mode == SearchMode::Code)
        .size_limit(2 * 1024 * 1024)
        .dfa_size_limit(2 * 1024 * 1024)
        .nest_limit(64)
        .build(query)
        .map_err(|_| if mode == SearchMode::Regex {
            "Unsupported regular expression. Check the pattern; look-around and backreferences are not supported."
        } else { INVALID_LITERAL_SEARCH_QUERY }.to_owned())?;
    if mode == SearchMode::Regex
        && matcher
            .find(b"")
            .map_err(|_| "The regular expression could not be searched.".to_owned())?
            .is_some()
    {
        return Err("Use a regular expression that matches at least one character.".to_owned());
    }
    Ok(matcher)
}

pub(crate) fn validate_search_mode(
    query: &str,
    path: Option<&str>,
    limit: usize,
    mode: SearchMode,
) -> Result<Vec<String>, String> {
    let minimum = if mode == SearchMode::Regex { 1 } else { 2 };
    if !(minimum..=240).contains(&query.trim().chars().count())
        || query.chars().any(char::is_control)
        || !(1..=32).contains(&limit)
    {
        return Err(if mode == SearchMode::Regex {
            "Search with a 1–240 character regular expression and at most 32 results."
        } else {
            "Search with 2–240 characters and at most 32 results."
        }
        .to_owned());
    }
    if path.is_some_and(|p| !safe_path(p)) {
        return Err("Search within a captured file or folder.".to_owned());
    }
    if mode == SearchMode::Regex {
        pattern_matcher(query.trim(), mode)?;
        return Ok(vec![]);
    }
    let words = terms(query);
    if words.len() > 32 {
        return Err("Use up to 32 searchable terms.".to_owned());
    }
    Ok(words)
}

pub(crate) fn validate_search(
    query: &str,
    path: Option<&str>,
    limit: usize,
) -> Result<Vec<String>, String> {
    validate_search_mode(query, path, limit, SearchMode::Code)
}

pub(crate) fn validate_search_page(limit: usize, offset: usize) -> Result<usize, String> {
    if !(1..=MAX_SEARCH_PAGE_SIZE).contains(&limit) {
        return Err("Search pages contain between 1 and 32 results.".to_owned());
    }
    offset
        .checked_add(limit)
        .filter(|end| *end <= MAX_CANDIDATES)
        .ok_or_else(|| "Search page is outside the 256-result candidate window.".to_owned())
}

fn search_windows_overlap(left: &CodeRange, right: &CodeRange) -> bool {
    left.snapshot_id == right.snapshot_id
        && left.path == right.path
        && left.source_sha256 == right.source_sha256
        && left.start_byte < right.end_byte
        && right.start_byte < left.end_byte
}

fn expanded_search_fetch_limit(
    fetch_limit: usize,
    has_more: bool,
    has_page_sentinel: bool,
) -> Option<usize> {
    (has_more && !has_page_sentinel && fetch_limit < MAX_CANDIDATES)
        .then(|| fetch_limit.saturating_mul(2).min(MAX_CANDIDATES))
        .filter(|next| *next > fetch_limit)
}

fn is_identifier_query(query: &str) -> bool {
    let mut chars = query.chars();
    chars
        .next()
        .is_some_and(|first| first == '_' || first.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// Canonicalize one identifier while preserving component order. This lets a
/// CamelCase query match its snake_case spelling without combining components
/// that occur in separate identifiers in the same source range.
fn identifier_parts_key(identifier: &str) -> Option<String> {
    let chars = identifier.chars().collect::<Vec<_>>();
    let mut parts = Vec::new();
    let mut start = 0;
    for i in 0..chars.len() {
        let boundary = chars[i] == '_'
            || (i > start
                && chars[i].is_uppercase()
                && (chars[i - 1].is_lowercase()
                    || (chars[i - 1].is_uppercase()
                        && chars.get(i + 1).is_some_and(|c| c.is_lowercase()))));
        if boundary {
            if start < i {
                parts.push(chars[start..i].iter().collect::<String>().to_lowercase());
            }
            start = if chars[i] == '_' { i + 1 } else { i };
        }
    }
    if start < chars.len() {
        parts.push(chars[start..].iter().collect::<String>().to_lowercase());
    }
    (parts.len() > 1).then(|| parts.join("\u{1f}"))
}

fn identifier_query_key(query: &str) -> Option<String> {
    is_identifier_query(query)
        .then(|| identifier_parts_key(query))
        .flatten()
}

fn is_literal_operator_query(query: &str) -> bool {
    ["::", "->", "=>", "==", "!=", "<=", ">=", "&&", "||"]
        .iter()
        .any(|operator| query.contains(operator))
}

fn is_code_path(path: &str) -> bool {
    crate::source_discovery::is_code_path(path)
}

/// Recognize a small, language-neutral set of same-line function declaration forms.
/// This gives parserless oversized source a useful exact-name result without treating
/// call sites such as `self.name(...)` or `let value = name(...)` as definitions.
fn function_declaration_at(source: &str, start: usize, end: usize) -> bool {
    let line_start = source[..start].rfind('\n').map_or(0, |at| at + 1);
    let line_end = source[end..]
        .find('\n')
        .map_or(source.len(), |offset| end + offset);
    let Some(line) = source.get(line_start..line_end) else {
        return false;
    };
    let Some(prefix) = line.get(..start - line_start) else {
        return false;
    };
    let Some(suffix) = line.get(end - line_start..) else {
        return false;
    };
    let prefix = prefix.trim();
    let suffix = suffix.trim_start();
    if prefix.is_empty()
        || !suffix.starts_with('(')
        || prefix.starts_with("//")
        || prefix.starts_with("/*")
        || prefix.starts_with('*')
        || (prefix.starts_with('#') && !prefix.starts_with("#["))
        || prefix.starts_with("--")
        || prefix.starts_with("<!--")
        || prefix.starts_with("(*")
        || prefix.contains("::")
        || prefix
            .chars()
            .any(|c| matches!(c, '=' | '.' | '!' | '{' | '}' | ';'))
    {
        return false;
    }
    if ["fn", "function", "def", "func"]
        .iter()
        .any(|marker| prefix.split_whitespace().any(|word| word == *marker))
    {
        return true;
    }
    let words = prefix.split_whitespace().collect::<Vec<_>>();
    let type_word = words.last().is_some_and(|word| {
        let word = word.trim_matches(|c| matches!(c, '*' | '&' | ':'));
        matches!(
            word,
            "void"
                | "char"
                | "short"
                | "int"
                | "long"
                | "float"
                | "double"
                | "bool"
                | "size_t"
                | "ssize_t"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "usize"
                | "i8"
                | "i16"
                | "i32"
                | "i64"
                | "isize"
        ) || word.contains('_')
            || [
                "Error", "Handle", "Face", "Slot", "Glyph", "Font", "Bitmap", "Buffer", "Mode",
                "Type", "Status", "Result", "State", "Value",
            ]
            .iter()
            .any(|suffix| word.ends_with(suffix))
    });
    words.len() <= 5
        && !words.is_empty()
        && type_word
        && !words.iter().any(|word| {
            matches!(
                *word,
                "return" | "match" | "let" | "const" | "var" | "if" | "while" | "for"
            )
        })
        && prefix
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c.is_whitespace() || "_*&<>:\"'".contains(c))
}

fn needs_identifier_declaration_fallback(file: &FileCoverage) -> bool {
    file.chunks > 0
        && (file.parser.is_none()
            || file.status.as_ref().is_some_and(|status| {
                matches!(
                    status,
                    index::IndexFileStatus::PartialParse | index::IndexFileStatus::ParseError
                )
            }))
        && is_code_path(&file.path)
}

fn reserve_verified_source_bytes(
    verified_bytes: &mut u64,
    next_file_bytes: u64,
) -> Result<(), String> {
    *verified_bytes = verified_bytes
        .checked_add(next_file_bytes)
        .filter(|total| *total <= MAX_SOURCE_BYTES)
        .ok_or_else(|| "The captured text exceeds the local code index byte limit.".to_owned())?;
    Ok(())
}

fn collect_identifier_declarations_from_source(
    source: &snapshot::SnapshotSource,
    ranges: &[&CodeRange],
    snapshot_id: &str,
    matcher: &RegexMatcher,
    declarations: &mut Vec<SearchHit>,
    limit: usize,
    budget: &Budget<'_>,
) -> Result<bool, String> {
    let starts = line_starts(&source.contents);
    let mut cursor = 0;
    loop {
        budget.check()?;
        let Some(found) = matcher
            .find_at(source.contents.as_bytes(), cursor)
            .map_err(|_| "A captured code snippet could not be searched.".to_owned())?
        else {
            return Ok(false);
        };
        if found.start() == found.end() {
            return Err("Use a regular expression that matches at least one character.".to_owned());
        }
        if !source.contents.is_char_boundary(found.start())
            || !source.contents.is_char_boundary(found.end())
        {
            return Err(
                "The expression selected part of a UTF-8 character. Use a Unicode pattern."
                    .to_owned(),
            );
        }
        if function_declaration_at(&source.contents, found.start(), found.end()) {
            if declarations.len() == limit {
                return Ok(true);
            }
            let range = ranges
                .get(ranges.partition_point(|range| range.end_byte <= found.start()))
                .ok_or("The code search index is invalid.")?;
            let original = source
                .contents
                .get(range.start_byte..range.end_byte)
                .ok_or("The code search index is invalid.")?;
            if range.start_byte >= range.end_byte
                || range.end_byte - range.start_byte > CHUNK_BYTES
                || range.start_byte > found.start()
                || range.snapshot_id != snapshot_id
                || range.source_sha256 != source.sha256
                || range.chunk_sha256 != digest(original.as_bytes())
                || range.start_line != line_at(&starts, range.start_byte)
                || range.end_line != line_at(&starts, range.end_byte - 1)
            {
                return Err("A saved code range changed. Rebuild the local code index.".to_owned());
            }
            let line_start = source.contents[..found.start()]
                .rfind('\n')
                .map_or(0, |at| at + 1);
            let line_end = source.contents[found.end()..]
                .find('\n')
                .map_or(source.contents.len(), |offset| found.end() + offset);
            let text = source.contents[line_start..line_end]
                .trim()
                .chars()
                .take(360)
                .collect();
            declarations.push(SearchHit {
                range: (**range).clone(),
                line: line_at(&starts, found.start()),
                text,
                exact_identifier: true,
                declaration_match: true,
                score: 0.0,
                query_coverage: None,
            });
        }
        cursor = found.end();
    }
}

/// Bounded literal or explicit regex search over verified captured text.
/// Search one eligible immutable file at a time, retaining the existing byte,
/// result and cancellation limits. A literal may cross an index chunk boundary.
fn search_pattern(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    prepared: &PreparedIndex,
    query: &str,
    path: Option<&str>,
    limit: usize,
    budget: &Budget<'_>,
    started: Instant,
    mode: SearchMode,
) -> Result<SearchResult, String> {
    let matcher = pattern_matcher(query, mode)?;
    let seek_declarations = mode == SearchMode::Code && is_identifier_query(query);
    let mut files = prepared
        .metadata
        .files
        .iter()
        .filter(|file| {
            file.chunks > 0
                && path.is_none_or(|scope| {
                    file.path == scope
                        || file
                            .path
                            .strip_prefix(scope)
                            .is_some_and(|rest| rest.starts_with('/'))
                })
        })
        .collect::<Vec<_>>();
    files.sort_by_key(|file| &file.path);
    let mut ranges_by_path: HashMap<&str, Vec<&CodeRange>> = HashMap::new();
    for range in &prepared.metadata.ranges {
        ranges_by_path.entry(&range.path).or_default().push(range);
    }
    for ranges in ranges_by_path.values_mut() {
        ranges.sort_by_key(|range| range.start_byte);
    }
    let mut matches = vec![];
    let mut declarations = vec![];
    let mut pattern_limited = false;
    let mut declaration_limited = false;
    let mut verified_bytes = 0u64;
    'files: for file in files {
        let declaration_fallback = seek_declarations && needs_identifier_declaration_fallback(file);
        if seek_declarations && pattern_limited && declaration_limited {
            break 'files;
        }
        if seek_declarations && pattern_limited && !declaration_fallback {
            continue;
        }
        if !seek_declarations && pattern_limited {
            break;
        }
        budget.check()?;
        reserve_verified_source_bytes(&mut verified_bytes, file.bytes)?;
        let captured = source_with_budget(storage, manifest, &file.path, budget)?;
        if captured.sha256 != file.source_sha256 || captured.size_bytes != file.bytes {
            return Err("A code result belongs to a different source version.".to_owned());
        }
        let ranges = ranges_by_path
            .get(file.path.as_str())
            .ok_or("The code search index is invalid.")?;
        let mut starts = None;
        let mut cursor = 0;
        while !pattern_limited {
            budget.check()?;
            let Some(found) = matcher
                .find_at(captured.contents.as_bytes(), cursor)
                .map_err(|_| "A captured code snippet could not be searched.".to_owned())?
            else {
                break;
            };
            budget.check()?;
            if found.start() == found.end() {
                return Err(
                    "Use a regular expression that matches at least one character.".to_owned(),
                );
            }
            if !captured.contents.is_char_boundary(found.start())
                || !captured.contents.is_char_boundary(found.end())
            {
                return Err(
                    "The expression selected part of a UTF-8 character. Use a Unicode pattern."
                        .to_owned(),
                );
            }
            if matches.len() == limit {
                pattern_limited = true;
                if !seek_declarations {
                    break 'files;
                }
                break;
            }
            let range = ranges
                .get(ranges.partition_point(|range| range.end_byte <= found.start()))
                .ok_or("The code search index is invalid.")?;
            if range.start_byte >= range.end_byte || range.end_byte - range.start_byte > CHUNK_BYTES
            {
                return Err("The code search index is invalid.".to_owned());
            }
            let lines = starts.get_or_insert_with(|| line_starts(&captured.contents));
            let original = captured
                .contents
                .get(range.start_byte..range.end_byte)
                .ok_or("The code search index is invalid.")?;
            if range.start_byte > found.start()
                || range.snapshot_id != manifest.summary.snapshot_id
                || range.source_sha256 != captured.sha256
                || range.chunk_sha256 != digest(original.as_bytes())
                || range.start_line != line_at(lines, range.start_byte)
                || range.end_line != line_at(lines, range.end_byte - 1)
            {
                return Err("A saved code range changed. Rebuild the local code index.".to_owned());
            }
            let start = range.start_byte.max(cursor);
            let end = range.end_byte.max(found.end());
            if end.saturating_sub(start) > MAX_RANGE_BYTES {
                return Err(
                    "A code match exceeds the 32 KiB range limit. Use a narrower pattern."
                        .to_owned(),
                );
            }
            let contents = captured
                .contents
                .get(start..end)
                .ok_or("The code range must lie on UTF-8 boundaries inside this file.")?;
            let mut hit_range = (**range).clone();
            hit_range.start_byte = start;
            hit_range.start_line = line_at(lines, start);
            hit_range.end_byte = end;
            hit_range.end_line = line_at(lines, end - 1);
            hit_range.chunk_sha256 = digest(contents.as_bytes());
            hit_range.enclosing = hit_range.enclosing.filter(|symbol| {
                symbol.start_byte <= hit_range.start_byte && end <= symbol.end_byte
            });
            hit_range.parent = hit_range.parent.filter(|symbol| {
                symbol.start_byte <= hit_range.start_byte && end <= symbol.end_byte
            });
            let line_start = captured.contents[..found.start()]
                .rfind('\n')
                .map_or(0, |at| at + 1);
            let before = &captured.contents[line_start..found.start()];
            let snippet_start = before
                .char_indices()
                .rev()
                .nth(79)
                .map_or(line_start, |(at, _)| line_start + at)
                .max(start);
            let text = captured.contents[snippet_start..end]
                .lines()
                .next()
                .unwrap_or("")
                .chars()
                .take(360)
                .collect::<String>();
            let hit = SearchHit {
                range: hit_range,
                line: line_at(lines, found.start()),
                text,
                exact_identifier: false,
                declaration_match: false,
                score: 0.0,
                query_coverage: None,
            };
            matches.push(hit);
            // One result per non-overlapping evidence window, even on long lines.
            cursor = end;
        }
        if declaration_fallback && !declaration_limited {
            declaration_limited = collect_identifier_declarations_from_source(
                &captured,
                ranges,
                &manifest.summary.snapshot_id,
                &matcher,
                &mut declarations,
                limit,
                budget,
            )?;
        }
        if seek_declarations && pattern_limited && declaration_limited {
            break 'files;
        }
    }
    if !declarations.is_empty() {
        declarations.append(&mut matches);
        declarations.truncate(limit);
        matches = declarations;
    }
    let limited = pattern_limited || declaration_limited;
    budget.check()?;
    let result_bytes = json(&matches)?.len();
    Ok(SearchResult {
        matches,
        limited,
        next_offset: None,
        coverage: prepared.metadata.coverage.clone(),
        index_limit_omissions: index_limit_omissions(&prepared.metadata.files, budget)?,
        reused_generation: prepared.reused_generation,
        build: prepared.metadata.build.clone(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        result_bytes,
    })
}

pub(crate) fn search(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    prepared: &PreparedIndex,
    query: &str,
    path: Option<&str>,
    limit: usize,
    budget: &Budget<'_>,
) -> Result<SearchResult, String> {
    search_with_mode(
        storage,
        manifest,
        prepared,
        query,
        path,
        limit,
        budget,
        SearchMode::Code,
    )
}

/// Exact declarations and identifiers lead. Ranked lexical results precede
/// substring fallback, except when a code operator asks for a literal match.
/// Merge evidence windows by immutable identity and overlap, never by display text.
pub(crate) fn search_with_mode(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    prepared: &PreparedIndex,
    query: &str,
    path: Option<&str>,
    limit: usize,
    budget: &Budget<'_>,
    mode: SearchMode,
) -> Result<SearchResult, String> {
    search_with_mode_page(
        storage, manifest, prepared, query, path, limit, 0, budget, mode,
    )
}

/// Return a stable page from the bounded, deterministic search candidate set.
/// The extra sentinel result tells the caller whether another page exists.
pub(crate) fn search_with_mode_page(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    prepared: &PreparedIndex,
    query: &str,
    path: Option<&str>,
    limit: usize,
    offset: usize,
    budget: &Budget<'_>,
    mode: SearchMode,
) -> Result<SearchResult, String> {
    let started = Instant::now();
    budget.check()?;
    if prepared.metadata.project_id != manifest.summary.project_id
        || prepared.metadata.snapshot_id != manifest.summary.snapshot_id
    {
        return Err("The code index belongs to a different snapshot.".to_owned());
    }
    let words = validate_search_mode(query, path, limit, mode)?;
    let page_end = validate_search_page(limit, offset)?;
    let mut fetch_limit = page_end.saturating_add(1).min(MAX_CANDIDATES);
    let query = query.trim();
    if mode == SearchMode::Regex || words.is_empty() {
        let mut result = search_pattern(
            storage,
            manifest,
            prepared,
            query,
            path,
            fetch_limit,
            budget,
            started,
            mode,
        )?;
        let has_more = result.limited || result.matches.len() > page_end;
        result.next_offset =
            (page_end < MAX_CANDIDATES && result.matches.len() > page_end).then_some(page_end);
        result.matches = result
            .matches
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();
        result.limited = has_more;
        result.result_bytes = json(&result.matches)?.len();
        return Ok(result);
    }
    loop {
        let mut ranked = search_ranked(
            storage,
            manifest,
            prepared,
            query,
            path,
            fetch_limit,
            budget,
        )?;
        let literal = search_pattern(
            storage,
            manifest,
            prepared,
            query,
            path,
            fetch_limit,
            budget,
            started,
            SearchMode::Code,
        )?;
        let (declarations, remaining): (Vec<_>, Vec<_>) = ranked
            .matches
            .into_iter()
            .partition(|hit| hit.declaration_match);
        let (exact, other): (Vec<_>, Vec<_>) =
            remaining.into_iter().partition(|hit| hit.exact_identifier);
        let (literal_declarations, literal_other): (Vec<_>, Vec<_>) = literal
            .matches
            .into_iter()
            .partition(|hit| hit.declaration_match);
        let (preferred_fallback, remaining_fallback) = if is_literal_operator_query(query) {
            (literal_other, other)
        } else {
            (other, literal_other)
        };
        let mut matches: Vec<SearchHit> = vec![];
        let mut limited = ranked.limited || literal.limited;
        for hit in declarations
            .into_iter()
            .chain(literal_declarations)
            .chain(exact)
            .chain(preferred_fallback)
            .chain(remaining_fallback)
        {
            budget.check()?;
            if matches
                .iter()
                .any(|old: &SearchHit| search_windows_overlap(&old.range, &hit.range))
            {
                continue;
            }
            if matches.len() == fetch_limit {
                limited = true;
                continue;
            }
            matches.push(hit);
        }
        let has_more = limited || matches.len() > page_end;
        let has_page_sentinel = matches.len() > page_end;
        // The ranked and literal searches can return overlapping windows. If
        // those duplicates consume their per-search sentinel, grow both
        // bounded prefixes until the merged unique results contain a sentinel
        // or the shared candidate window is exhausted. Otherwise a truncated
        // page can incorrectly report no next offset after deduplication.
        if let Some(expanded_limit) =
            expanded_search_fetch_limit(fetch_limit, has_more, has_page_sentinel)
        {
            fetch_limit = expanded_limit;
            continue;
        }
        let next_offset = (page_end < MAX_CANDIDATES && has_page_sentinel).then_some(page_end);
        ranked.matches = matches;
        ranked.next_offset = next_offset;
        ranked.limited = has_more;
        ranked.matches = ranked
            .matches
            .into_iter()
            .skip(offset)
            .take(limit)
            .collect();
        ranked.result_bytes = json(&ranked.matches)?.len();
        ranked.elapsed_ms = started.elapsed().as_millis() as u64;
        return Ok(ranked);
    }
}

/// Return one relevant source range per captured file, ranked by distinct
/// query-term coverage and then inverse-document-frequency score. This is a
/// discovery view for broad questions: the caller can then scope a normal
/// range search to a promising file instead of letting many chunks from one
/// large file hide other relevant modules.
pub(crate) fn search_files_with_mode_page(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    prepared: &PreparedIndex,
    query: &str,
    path: Option<&str>,
    limit: usize,
    offset: usize,
    budget: &Budget<'_>,
    mode: SearchMode,
) -> Result<SearchResult, String> {
    let started = Instant::now();
    budget.check()?;
    if prepared.metadata.project_id != manifest.summary.project_id
        || prepared.metadata.snapshot_id != manifest.summary.snapshot_id
    {
        return Err("The code index belongs to a different snapshot.".to_owned());
    }
    if mode != SearchMode::Code {
        return Err("File discovery supports ranked code search only.".to_owned());
    }
    let query_words = validate_search_mode(query, path, limit, mode)?;
    let relevant_words = coverage_terms(query, path, &query_words);
    let page_end = validate_search_page(limit, offset)?;
    if relevant_words.is_empty() {
        let matches = Vec::new();
        return Ok(SearchResult {
            result_bytes: json(&matches)?.len(),
            matches,
            limited: false,
            next_offset: None,
            coverage: prepared.metadata.coverage.clone(),
            index_limit_omissions: index_limit_omissions(&prepared.metadata.files, budget)?,
            reused_generation: prepared.reused_generation,
            build: prepared.metadata.build.clone(),
            elapsed_ms: started.elapsed().as_millis() as u64,
        });
    }
    let words = relevant_words;
    let in_scope = |candidate: &str| {
        path.is_none_or(|scope| {
            candidate == scope
                || candidate
                    .strip_prefix(scope)
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
    };
    let body = field(&prepared.index, "body")?;
    let body_stem = field(&prepared.index, "body_stem")?;
    let symbol_field = field(&prepared.index, "symbol")?;
    let mut term_groups = Vec::with_capacity(words.len());
    let mut hint_term_groups = Vec::with_capacity(words.len());
    for word in &words {
        let stems = english_stems(&prepared.index, word)?;
        let mut hint_terms = BTreeSet::from([word.clone()]);
        hint_terms.extend(stems.iter().cloned());
        let mut variants = vec![(body, word.clone())];
        variants.extend(
            stems
                .into_iter()
                .filter(|stem| stem != word)
                .map(|stem| (body_stem, stem)),
        );
        term_groups.push(variants);
        hint_term_groups.push(hint_terms);
    }

    let searcher = prepared
        .index
        .reader()
        .map_err(|_| "Code search index could not be opened. Try rebuilding it.".to_owned())?
        .searcher();
    // A term has one IDF weight for the entire query, so each path/range only
    // needs to retain which positive-weight terms matched. A 32-bit mask keeps
    // the bounded 50,000-range index from allocating a heap vector per hit.
    let mut path_terms: HashMap<&str, u32> = HashMap::new();
    // Paths and indexed declarations are useful for ranking a file shortlist,
    // but they are not source evidence. Keep their coverage separate from the
    // body matches that select the returned, verifiable source range.
    let mut path_hint_terms: HashMap<&str, u32> = HashMap::new();
    let mut declaration_hint_terms: HashMap<&str, u32> = HashMap::new();
    let mut declaration_range_terms: HashMap<usize, u32> = HashMap::new();
    let mut range_terms: HashMap<usize, u32> = HashMap::new();
    let mut term_weights = vec![0.0f32; term_groups.len()];
    let mut declaration_term_weights = vec![0.0f32; term_groups.len()];
    // File discovery returns one result per path, so IDF must count files,
    // not chunks. Otherwise one large file with many matching ranges can
    // distort term rarity for every other file in the repository.
    let total_paths = prepared
        .metadata
        .files
        .iter()
        .filter(|file| file.chunks > 0 && in_scope(&file.path))
        .count()
        .max(1);
    let range_key_field = field(&prepared.index, "range_key")?;
    for (term_index, variants) in term_groups.iter().enumerate() {
        budget.check()?;
        let mut matching_ranges = BTreeSet::new();
        for (term_field, term) in variants {
            let query = scoped(
                term_query(*term_field, term),
                &prepared.index,
                path,
            )?;
            let documents = searcher
                .search(&query, &tantivy::collector::DocSetCollector)
                .map_err(|_| "Code search failed. Try rebuilding its local index.".to_owned())?;
            for (index, address) in documents.into_iter().enumerate() {
                if index % 128 == 0 {
                    budget.check()?;
                }
                let document = searcher
                    .doc::<TantivyDocument>(address)
                    .map_err(|_| "A code search result is unavailable.".to_owned())?;
                let key = document
                    .get_first(range_key_field)
                    .and_then(|value| value.as_str())
                    .ok_or("The code search index is invalid.")?;
                let range_id = *prepared
                    .ranges_by_key
                    .get(key)
                    .ok_or("The code search index is invalid.")?;
                let range = prepared
                    .metadata
                    .ranges
                    .get(range_id)
                    .ok_or("The code search index is invalid.")?;
                if in_scope(&range.path) {
                    matching_ranges.insert(range_id);
                }
            }
        }
        if matching_ranges.is_empty() {
            continue;
        }
        let matching_paths = matching_ranges
            .iter()
            .map(|range_id| prepared.metadata.ranges[*range_id].path.as_str())
            .collect::<BTreeSet<_>>();
        let inverse_document_frequency =
            (((total_paths + 1) as f32) / ((matching_paths.len() + 1) as f32)).ln();
        term_weights[term_index] = inverse_document_frequency;
        let term_bit = if inverse_document_frequency > 0.0 {
            1u32 << term_index
        } else {
            0
        };
        for range_id in matching_ranges {
            let range = &prepared.metadata.ranges[range_id];
            *path_terms.entry(range.path.as_str()).or_default() |= term_bit;
            *range_terms.entry(range_id).or_default() |= term_bit;
        }

        // Declaration names are a strong signal for broad questions: a code
        // file can contain many generic body matches while its relevant entry
        // point is visible only in the indexed symbol name. Keep declarations
        // separate from paths. Match both the original term and its indexed
        // English stem so questions such as "rendering" can find a declared
        // `render` entry point without changing the exact source evidence.
        // Body matches remain mandatory before returning any source result.
        // A common body term can still identify a rare declaration, so score
        // declaration frequency independently from body-term IDF.
        let declarations = BooleanQuery::union(
            hint_term_groups[term_index]
                .iter()
                .map(|term| term_query(symbol_field, term))
                .collect(),
        );
        let declarations = scoped(Box::new(declarations), &prepared.index, path)?;
        let declaration_docs = searcher
            .search(&declarations, &tantivy::collector::DocSetCollector)
            .map_err(|_| "Code search failed. Try rebuilding its local index.".to_owned())?;
        let mut declaration_ranges = BTreeSet::new();
        let mut declaration_paths = BTreeSet::new();
        for (index, address) in declaration_docs.into_iter().enumerate() {
            if index % 128 == 0 {
                budget.check()?;
            }
            let document = searcher
                .doc::<TantivyDocument>(address)
                .map_err(|_| "A code search result is unavailable.".to_owned())?;
            let key = document
                .get_first(range_key_field)
                .and_then(|value| value.as_str())
                .ok_or("The code search index is invalid.")?;
            let range_id = *prepared
                .ranges_by_key
                .get(key)
                .ok_or("The code search index is invalid.")?;
            let range = prepared
                .metadata
                .ranges
                .get(range_id)
                .ok_or("The code search index is invalid.")?;
            if in_scope(&range.path) {
                declaration_ranges.insert(range_id);
                declaration_paths.insert(range.path.as_str());
            }
        }
        let declaration_weight = if declaration_paths.is_empty()
            || declaration_paths.len() >= total_paths
        {
            0.0
        } else {
            (((total_paths + 1) as f32) / ((declaration_paths.len() + 1) as f32)).ln()
        };
        declaration_term_weights[term_index] = declaration_weight;
        if declaration_weight > 0.0 {
            let declaration_bit = 1u32 << term_index;
            for range_id in declaration_ranges {
                let range = &prepared.metadata.ranges[range_id];
                *declaration_hint_terms
                    .entry(range.path.as_str())
                    .or_default() |= declaration_bit;
                *declaration_range_terms.entry(range_id).or_default() |= declaration_bit;
            }
        }
    }

    // File paths are already present in the bounded local file inventory, so
    // derive their low-priority hint score directly instead of querying every
    // matching range document a second time. Compare the same raw and stemmed
    // term variants used for declarations against the indexed path tokens.
    for (file_index, file) in prepared.metadata.files.iter().enumerate() {
        if file_index % 128 == 0 {
            budget.check()?;
        }
        if file.chunks == 0 || !in_scope(&file.path) {
            continue;
        }
        let file_terms = terms(&file.path);
        for (term_index, hint_terms) in hint_term_groups.iter().enumerate() {
            if hint_terms
                .iter()
                .any(|term| file_terms.iter().any(|file_term| file_term == term))
            {
                *path_hint_terms.entry(file.path.as_str()).or_default() |= 1u32 << term_index;
            }
        }
    }

    let score_terms = |mask: u32, weights: &[f32]| {
        let mut score = 0.0f32;
        let mut coverage = 0usize;
        for (index, weight) in weights.iter().enumerate() {
            if mask & (1u32 << index) != 0 {
                score += *weight;
                coverage += 1;
            }
        }
        (score, coverage)
    };
    let mut best_ranges: HashMap<&str, (usize, f32, usize, f32, usize)> = HashMap::new();
    for (range_id, mask) in &range_terms {
        let range = &prepared.metadata.ranges[*range_id];
        let (score, coverage) = score_terms(*mask, &term_weights);
        let declaration_mask = declaration_range_terms
            .get(range_id)
            .copied()
            .unwrap_or_default();
        let (declaration_score, declaration_coverage) =
            score_terms(declaration_mask, &declaration_term_weights);
        let replace = best_ranges.get(range.path.as_str()).is_none_or(
            |(
                best_id,
                best_score,
                best_coverage,
                best_declaration_score,
                best_declaration_coverage,
            )| {
                let relevance = declaration_score
                    .total_cmp(best_declaration_score)
                    .then_with(|| declaration_coverage.cmp(best_declaration_coverage))
                    .then_with(|| score.total_cmp(best_score))
                    .then_with(|| coverage.cmp(best_coverage));
                relevance.is_gt()
                    || (relevance.is_eq()
                        && (range.start_byte, range.end_byte, *range_id)
                            < (
                                prepared.metadata.ranges[*best_id].start_byte,
                                prepared.metadata.ranges[*best_id].end_byte,
                                *best_id,
                            ))
            },
        );
        if replace {
            best_ranges.insert(
                range.path.as_str(),
                (
                    *range_id,
                    score,
                    coverage,
                    declaration_score,
                    declaration_coverage,
                ),
            );
        }
    }

    let mut ranked_files = path_terms
        .into_iter()
        .filter_map(|(file_path, mask)| {
            let (range_id, _, _, _, _) = best_ranges.get(file_path)?;
            let (score, coverage) = score_terms(mask, &term_weights);
            let matched_terms = words
                .iter()
                .enumerate()
                .filter_map(|(index, word)| {
                    (mask & (1u32 << index) != 0).then(|| word.clone())
                })
                .collect::<Vec<_>>();
            let hint_coverage = path_hint_terms
                .get(file_path)
                .copied()
                .unwrap_or_default()
                .count_ones() as usize;
            let declaration_mask = declaration_hint_terms
                .get(file_path)
                .copied()
                .unwrap_or_default();
            let (declaration_score, declaration_coverage) =
                score_terms(declaration_mask, &declaration_term_weights);
            Some((
                file_path,
                *range_id,
                score,
                coverage,
                declaration_score,
                declaration_coverage,
                hint_coverage,
                matched_terms,
            ))
        })
        .collect::<Vec<_>>();
    ranked_files.sort_by(|left, right| {
        right
            .4
            .total_cmp(&left.4)
            .then_with(|| right.5.cmp(&left.5))
            .then_with(|| right.3.cmp(&left.3))
            .then_with(|| right.6.cmp(&left.6))
            .then_with(|| right.2.total_cmp(&left.2))
            .then_with(|| left.0.cmp(&right.0))
    });
    let total_files = ranked_files.len();
    ranked_files.truncate(MAX_CANDIDATES);
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(true)
        .fixed_strings(true)
        .build_literals(&words)
        .map_err(|_| INVALID_LITERAL_SEARCH_QUERY.to_owned())?;
    let mut matches = Vec::new();
    for (file_path, range_id, score, _, _, _, _, matched_terms) in
        ranked_files.into_iter().skip(offset).take(limit)
    {
        budget.check()?;
        let range = prepared
            .metadata
            .ranges
            .get(range_id)
            .ok_or("The code search index is invalid.")?;
        if range.path != file_path {
            return Err("The code search index is invalid.".to_owned());
        }
        let captured = source_with_budget(storage, manifest, file_path, budget)?;
        // Source verification reads and hashes the complete captured blob. A
        // request may be cancelled while that bounded read is in progress.
        budget.check()?;
        if captured.sha256 != range.source_sha256 {
            return Err("A code result belongs to a different source version.".to_owned());
        }
        let read = range_from_source(&captured, range.start_byte, range.end_byte)?;
        if read.range.chunk_sha256 != range.chunk_sha256
            || read.range.start_line != range.start_line
            || read.range.end_line != range.end_line
        {
            return Err("A saved code range changed. Rebuild the local code index.".to_owned());
        }
        let mut line = range.start_line;
        let mut text = read
            .contents
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .chars()
            .take(360)
            .collect::<String>();
        Searcher::new()
            .search_slice(
                &matcher,
                read.contents.as_bytes(),
                UTF8(|number, found| {
                    line = range.start_line + number as usize - 1;
                    text = found.trim().chars().take(360).collect();
                    Ok(false)
                }),
            )
            .map_err(|_| "A code search result could not be inspected.".to_owned())?;
        budget.check()?;
        matches.push(SearchHit {
            range: read.range,
            line,
            text,
            exact_identifier: false,
            declaration_match: false,
            score,
            query_coverage: Some(QueryCoverage {
                matched_terms,
                total_terms: words.len(),
            }),
        });
    }
    budget.check()?;
    let next_offset = (page_end < MAX_CANDIDATES && total_files > page_end).then_some(page_end);
    let limited = total_files > page_end;
    let result_bytes = json(&matches)?.len();
    Ok(SearchResult {
        matches,
        result_bytes,
        limited,
        next_offset,
        coverage: prepared.metadata.coverage.clone(),
        index_limit_omissions: index_limit_omissions(&prepared.metadata.files, budget)?,
        reused_generation: prepared.reused_generation,
        build: prepared.metadata.build.clone(),
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

fn merge_query_term_candidate(
    candidates: &mut HashMap<usize, (bool, bool, f32)>,
    term_coverage: &mut HashMap<usize, u32>,
    range_id: usize,
    score: f32,
    term_bit: u32,
) {
    let candidate = candidates
        .entry(range_id)
        .or_insert((false, false, score));
    candidate.2 = candidate.2.max(score);
    *term_coverage.entry(range_id).or_default() |= term_bit;
}

fn search_ranked(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    prepared: &PreparedIndex,
    query: &str,
    path: Option<&str>,
    limit: usize,
    budget: &Budget<'_>,
) -> Result<SearchResult, String> {
    let started = Instant::now();
    budget.check()?;
    if prepared.metadata.project_id != manifest.summary.project_id
        || prepared.metadata.snapshot_id != manifest.summary.snapshot_id
    {
        return Err("The code index belongs to a different snapshot.".to_owned());
    }
    let query = query.trim();
    let words = terms(query);
    let identifier_key = identifier_query_key(query);
    // Only multiword natural-language searches need this second relevance
    // signal. Keep paths and identifier lookups on their established ranking,
    // and cap the content terms (after prompt scaffolding is removed) so
    // coverage work stays bounded for long questions too.
    let coverage_words = coverage_terms(query, path, &words);
    let natural_query_coverage = identifier_key.is_none()
        && !query.contains('/')
        && !query.contains('\\')
        && query.split_whitespace().count() > 1
        && (3..=12).contains(&coverage_words.len())
        && !coverage_words.is_empty();
    let include_path_terms = query.split_whitespace().count() <= 2
        || query.contains('/')
        || query.contains('\\')
        || identifier_key.is_some();
    let body = field(&prepared.index, "body")?;
    let body_stem = field(&prepared.index, "body_stem")?;
    let path_field = field(&prepared.index, "path")?;
    let symbol_field = field(&prepared.index, "symbol")?;
    let identifier_parts_field = field(&prepared.index, "identifier_parts")?;
    let mut clauses = vec![];
    for word in &words {
        clauses.push((Occur::Should, term_query(body, word)));
        clauses.push((
            Occur::Should,
            Box::new(BoostQuery::new(term_query(symbol_field, word), 3.0)) as Box<dyn Query>,
        ));
        if include_path_terms {
            clauses.push((
                Occur::Should,
                Box::new(BoostQuery::new(term_query(path_field, word), 1.5)) as Box<dyn Query>,
            ));
        }
    }
    if identifier_key.is_none() {
        for stem in english_stems(&prepared.index, &words.join(" "))? {
            if !words.contains(&stem) {
                clauses.push((
                    Occur::Should,
                    Box::new(BoostQuery::new(term_query(body_stem, &stem), 0.5)) as Box<dyn Query>,
                ));
            }
        }
    }
    if let Some(identifier_key) = identifier_key {
        clauses.push((
            Occur::Must,
            term_query(identifier_parts_field, &identifier_key),
        ));
    }
    let lexical = scoped(Box::new(BooleanQuery::new(clauses)), &prepared.index, path)?;
    let reader = prepared
        .index
        .reader()
        .map_err(|_| "Code search index could not be opened. Try rebuilding it.".to_owned())?;
    let searcher = reader.searcher();
    // A repository-wide TopDocs cap can be filled by a single large file
    // before path diversification runs. A second bounded pool lets us recover
    // other paths only when the first 256 results are path-concentrated.
    let ranking_limit = if path.is_none() {
        GLOBAL_RANKING_CANDIDATE_LIMIT
    } else {
        MAX_CANDIDATES
    };
    let (mut found, total) = ranking::candidates_with_limit(&searcher, &lexical, ranking_limit)
        .map_err(|_| "Code search failed. Try rebuilding its local index.".to_owned())?;
    let exact_query = if query.split_whitespace().count() == 1 {
        let last = query.rsplit("::").next().unwrap_or(query).to_lowercase();
        Some(scoped(
            Box::new(BooleanQuery::union(vec![
                term_query(field(&prepared.index, "exact")?, &query.to_lowercase()),
                term_query(field(&prepared.index, "exact")?, &last),
            ])),
            &prepared.index,
            path,
        )?)
    } else {
        None
    };
    let mut exact_docs = BTreeSet::new();
    let mut declaration_docs = BTreeSet::new();
    let mut exact_limited = false;
    if let Some(exact) = exact_query {
        let (hits, count) = ranking::candidates_with_limit(&searcher, &exact, ranking_limit)
            .map_err(|_| "Exact identifier search failed.".to_owned())?;
        exact_limited = count > MAX_CANDIDATES;
        for (score, address) in hits {
            exact_docs.insert(address);
            found.push((score, address));
        }
        let last = query.rsplit("::").next().unwrap_or(query).to_lowercase();
        let declarations = scoped(
            Box::new(BooleanQuery::union(vec![
                term_query(
                    field(&prepared.index, "declaration")?,
                    &query.to_lowercase(),
                ),
                term_query(field(&prepared.index, "declaration")?, &last),
            ])),
            &prepared.index,
            path,
        )?;
        let (hits, count) = ranking::candidates_with_limit(&searcher, &declarations, ranking_limit)
            .map_err(|_| "Declaration search failed.".to_owned())?;
        exact_limited |= count > MAX_CANDIDATES;
        for (score, address) in hits {
            declaration_docs.insert(address);
            exact_docs.insert(address);
            found.push((score, address));
        }
    }
    budget.check()?;
    let range_key = field(&prepared.index, "range_key")?;
    let mut candidates = HashMap::new();
    let mut candidate_address_ids = HashMap::new();
    for (score, address) in found {
        let doc = searcher
            .doc::<TantivyDocument>(address)
            .map_err(|_| "A code search result is unavailable.".to_owned())?;
        let id = doc
            .get_first(range_key)
            .and_then(|v| v.as_str())
            .and_then(|key| prepared.ranges_by_key.get(key).copied())
            .ok_or("The code search index is invalid.")?;
        if id >= prepared.metadata.ranges.len() {
            return Err("The code search index is invalid.".to_owned());
        }
        let exact = exact_docs.contains(&address);
        let declaration = declaration_docs.contains(&address);
        candidate_address_ids.insert(address, id);
        candidates
            .entry(id)
            .and_modify(|s: &mut (bool, bool, f32)| {
                s.0 |= exact;
                s.1 |= declaration;
                s.2 = s.2.max(score);
            })
            .or_insert((exact, declaration, score));
    }
    let mut term_coverage = HashMap::<usize, u32>::new();
    let mut symbol_term_coverage = HashMap::<usize, u32>::new();
    let mut coverage_term_weights = vec![0.0f32; coverage_words.len()];
    let mut symbol_term_weights = vec![0.0f32; coverage_words.len()];
    let mut coverage_limited = false;
    if natural_query_coverage {
        let in_scope = |candidate: &str| {
            path.is_none_or(|scope| {
                candidate == scope
                    || candidate
                        .strip_prefix(scope)
                        .is_some_and(|suffix| suffix.starts_with('/'))
            })
        };
        let coverage_range_count = prepared
            .metadata
            .ranges
            .iter()
            .filter(|range| in_scope(&range.path))
            .count()
            .max(1) as f32;
        let body_field = field(&prepared.index, "body")?;
        let body_stem_field = field(&prepared.index, "body_stem")?;
        let symbol_field = field(&prepared.index, "symbol")?;
        for (term_index, word) in coverage_words.iter().enumerate() {
            budget.check()?;
            let mut symbol_terms = english_stems(&prepared.index, word)?;
            symbol_terms.insert(word.clone());
            let symbol_query = scoped(
                Box::new(BooleanQuery::union(
                    symbol_terms
                        .iter()
                        .map(|term| term_query(symbol_field, term))
                        .collect(),
                )),
                &prepared.index,
                path,
            )?;
            let (symbol_matches, symbol_count) =
                ranking::candidates_with_limit(&searcher, &symbol_query, MAX_CHUNKS).map_err(
                    |_| "Code search failed. Try rebuilding its local index.".to_owned(),
                )?;
            coverage_limited |= symbol_count > MAX_CHUNKS;
            let symbol_document_frequency = (symbol_count as f32).min(coverage_range_count);
            symbol_term_weights[term_index] = ((coverage_range_count + 1.0)
                / (symbol_document_frequency + 1.0))
                .ln()
                .max(0.0);
            let mut clauses = vec![
                (Occur::Should, term_query(body_field, word)),
                (
                    Occur::Should,
                    Box::new(BoostQuery::new(term_query(symbol_field, word), 3.0))
                        as Box<dyn Query>,
                ),
            ];
            for stem in english_stems(&prepared.index, word)? {
                if stem != *word {
                    clauses.push((
                        Occur::Should,
                        Box::new(BoostQuery::new(term_query(body_stem_field, &stem), 0.5))
                            as Box<dyn Query>,
                    ));
                }
            }
            let term = scoped(Box::new(BooleanQuery::new(clauses)), &prepared.index, path)?;
            // Coverage ranking must see every matching range before the final
            // 256-result window is selected. A BM25 TopDocs cap here can hide
            // a low-score match that covers a distinctive query term. The
            // immutable range index itself is bounded by MAX_CHUNKS.
            let (term_matches, count) = ranking::candidates_with_limit(&searcher, &term, MAX_CHUNKS)
                .map_err(|_| "Code search failed. Try rebuilding its local index.".to_owned())?;
            coverage_limited |= count > MAX_CHUNKS;
            // Weight distinct query terms by their rarity across the scoped
            // range corpus. This mirrors file discovery's IDF ranking and
            // keeps generic words from outranking specific implementation
            // terms merely because both matched the same number of ranges.
            let document_frequency = (count as f32).min(coverage_range_count);
            coverage_term_weights[term_index] = ((coverage_range_count + 1.0)
                / (document_frequency + 1.0))
                .ln()
                .max(0.0);
            let term_bit = 1u32 << term_index;
            for (index, (score, address)) in term_matches.into_iter().enumerate() {
                if index % 128 == 0 {
                    budget.check()?;
                }
                let range_id = if let Some(range_id) = candidate_address_ids.get(&address) {
                    *range_id
                } else {
                    let doc = searcher
                        .doc::<TantivyDocument>(address)
                        .map_err(|_| "A code search result is unavailable.".to_owned())?;
                    let id = doc
                        .get_first(range_key)
                        .and_then(|value| value.as_str())
                        .and_then(|key| prepared.ranges_by_key.get(key).copied())
                        .ok_or("The code search index is invalid.")?;
                    if id >= prepared.metadata.ranges.len() {
                        return Err("The code search index is invalid.".to_owned());
                    }
                    candidate_address_ids.insert(address, id);
                    id
                };
                merge_query_term_candidate(
                    &mut candidates,
                    &mut term_coverage,
                    range_id,
                    score,
                    term_bit,
                );
            }
            for (index, (score, address)) in symbol_matches.into_iter().enumerate() {
                if index % 128 == 0 {
                    budget.check()?;
                }
                let range_id = if let Some(range_id) = candidate_address_ids.get(&address) {
                    *range_id
                } else {
                    let doc = searcher
                        .doc::<TantivyDocument>(address)
                        .map_err(|_| "A code search result is unavailable.".to_owned())?;
                    let id = doc
                        .get_first(range_key)
                        .and_then(|value| value.as_str())
                        .and_then(|key| prepared.ranges_by_key.get(key).copied())
                        .ok_or("The code search index is invalid.")?;
                    if id >= prepared.metadata.ranges.len() {
                        return Err("The code search index is invalid.".to_owned());
                    }
                    candidate_address_ids.insert(address, id);
                    id
                };
                *symbol_term_coverage.entry(range_id).or_default() |= term_bit;
                merge_query_term_candidate(
                    &mut candidates,
                    &mut term_coverage,
                    range_id,
                    score,
                    term_bit,
                );
            }
        }
        coverage_limited |= candidates.len() > MAX_CANDIDATES;
    }
    let mut ranked = candidates
        .into_iter()
        .map(|(id, (exact, declaration, score))| {
            let coverage_mask = term_coverage.get(&id).copied().unwrap_or_default();
            let coverage = coverage_mask.count_ones() as usize;
            let weighted_coverage = coverage_term_weights
                .iter()
                .enumerate()
                .filter_map(|(term_index, weight)| {
                    (coverage_mask & (1u32 << term_index) != 0).then_some(*weight)
                })
                .sum::<f32>();
            let symbol_mask = symbol_term_coverage.get(&id).copied().unwrap_or_default();
            let symbol_coverage = symbol_mask.count_ones() as usize;
            let weighted_symbol_coverage = symbol_term_weights
                .iter()
                .enumerate()
                .filter_map(|(term_index, weight)| {
                    (symbol_mask & (1u32 << term_index) != 0).then_some(*weight)
                })
                .sum::<f32>();
            (
                id,
                (
                    exact,
                    declaration,
                    weighted_symbol_coverage,
                    symbol_coverage,
                    weighted_coverage,
                    coverage,
                    score,
                ),
            )
        })
        .collect::<Vec<_>>();
    ranked.sort_by(
        |(a, (ae, ad, asw, asc, aw, ac, ascore)), (b, (be, bd, bsw, bsc, bw, bc, bscore))| {
        bd.cmp(ad)
            .then_with(|| be.cmp(ae))
            .then_with(|| bsw.total_cmp(asw))
            .then_with(|| bsc.cmp(asc))
            .then_with(|| bc.cmp(ac))
            .then_with(|| bw.total_cmp(aw))
            .then_with(|| bscore.total_cmp(ascore))
            .then_with(|| {
                prepared.metadata.ranges[*a]
                    .path
                    .cmp(&prepared.metadata.ranges[*b].path)
            })
            .then_with(|| {
                prepared.metadata.ranges[*a]
                    .start_byte
                    .cmp(&prepared.metadata.ranges[*b].start_byte)
            })
            .then_with(|| a.cmp(b))
        },
    );
    if path.is_none() {
        let mut leading_paths = BTreeSet::new();
        for (id, _) in ranked.iter().take(MAX_CANDIDATES) {
            leading_paths.insert(prepared.metadata.ranges[*id].path.as_str());
        }
        let use_expanded_candidates =
            ranked.len() > MAX_CANDIDATES && leading_paths.len() < GLOBAL_DIVERSIFIED_PATH_LIMIT;
        if !use_expanded_candidates {
            ranked.truncate(MAX_CANDIDATES);
        }
        let path_limit = if use_expanded_candidates {
            GLOBAL_DIVERSIFIED_PATH_LIMIT
        } else {
            usize::MAX
        };
        let (declarations, remaining): (Vec<_>, Vec<_>) = ranked
            .into_iter()
            .partition(|(_, (_, declaration, _, _, _, _, _))| *declaration);
        let (exact, other): (Vec<_>, Vec<_>) = remaining
            .into_iter()
            .partition(|(_, (exact, _, _, _, _, _, _))| *exact);
        let path_for = |id: usize| prepared.metadata.ranges[id].path.clone();
        ranked = diversify_by_path_limited(declarations, &path_for, path_limit)
            .into_iter()
            .chain(diversify_by_path_limited(exact, &path_for, path_limit))
            .chain(diversify_by_path_limited(other, &path_for, path_limit))
            .collect();
    }
    let limited = exact_limited || coverage_limited || total > limit || ranked.len() > limit;
    let exact_terms = vec![query.rsplit("::").next().unwrap_or(query).to_owned()];
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(true)
        .fixed_strings(true)
        .build_literals(&words)
        .map_err(|_| INVALID_LITERAL_SEARCH_QUERY.to_owned())?;
    let exact_matcher = RegexMatcherBuilder::new()
        .case_insensitive(true)
        .fixed_strings(true)
        .build_literals(&exact_terms)
        .map_err(|_| INVALID_LITERAL_SEARCH_QUERY.to_owned())?;
    // Keep relevance order while verifying only one full captured file at a
    // time. Retaining a source blob for every result path can multiply peak
    // memory by the page size when a broad query spans large files.
    let selected = ranked.into_iter().take(limit).collect::<Vec<_>>();
    let mut positions_by_path = HashMap::<String, Vec<usize>>::new();
    for (position, (id, _)) in selected.iter().enumerate() {
        let path = &prepared.metadata.ranges[*id].path;
        positions_by_path
            .entry(path.clone())
            .or_default()
            .push(position);
    }
    let mut ordered_matches = std::iter::repeat_with(|| None)
        .take(selected.len())
        .collect::<Vec<Option<SearchHit>>>();
    for (path, positions) in positions_by_path {
        budget.check()?;
        let captured = source_with_budget(storage, manifest, &path, budget)?;
        // A result page can contain many chunks from one large file. Reuse its
        // line index instead of rescanning the complete source for every hit.
        let starts = line_starts(&captured.contents);
        for position in positions {
            budget.check()?;
            let (id, (exact_identifier, declaration_match, _, _, _, _, score)) = selected[position];
            let range = &prepared.metadata.ranges[id];
            if range.path != path {
                return Err("The code search index is invalid.".to_owned());
            }
            if captured.sha256 != range.source_sha256 {
                return Err("A code result belongs to a different source version.".to_owned());
            }
            let read = range_from_source_at_lines(
                &captured,
                range.start_byte,
                range.end_byte,
                &starts,
            )?;
            if read.range.chunk_sha256 != range.chunk_sha256
                || read.range.start_line != range.start_line
                || read.range.end_line != range.end_line
            {
                return Err("A saved code range changed. Rebuild the local code index.".to_owned());
            }
            let mut line = range.start_line;
            let mut text = read
                .contents
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .chars()
                .take(360)
                .collect::<String>();
            Searcher::new()
                .search_slice(
                    if exact_identifier {
                        &exact_matcher
                    } else {
                        &matcher
                    },
                    read.contents.as_bytes(),
                    UTF8(|number, found| {
                        line = range.start_line + number as usize - 1;
                        text = found.trim().chars().take(360).collect();
                        Ok(false)
                    }),
                )
                .map_err(|_| "A captured code snippet could not be searched.".to_owned())?;
            ordered_matches[position] = Some(SearchHit {
                range: range.clone(),
                line,
                text,
                exact_identifier,
                declaration_match,
                score,
                query_coverage: natural_query_coverage.then(|| {
                    let matched_terms = term_coverage
                        .get(&id)
                        .copied()
                        .unwrap_or_default();
                    QueryCoverage {
                        matched_terms: coverage_words
                            .iter()
                            .enumerate()
                            .filter_map(|(term_index, word)| {
                                (matched_terms & (1u32 << term_index) != 0)
                                    .then(|| word.clone())
                            })
                            .collect(),
                        total_terms: coverage_words.len(),
                    }
                }),
            });
        }
    }
    let matches = ordered_matches
        .into_iter()
        .map(|hit| hit.ok_or_else(|| "A code search result is unavailable.".to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    budget.check()?;
    let result_bytes = json(&matches)?.len();
    Ok(SearchResult {
        matches,
        limited,
        next_offset: None,
        coverage: prepared.metadata.coverage.clone(),
        index_limit_omissions: index_limit_omissions(&prepared.metadata.files, budget)?,
        reused_generation: prepared.reused_generation,
        build: prepared.metadata.build.clone(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        result_bytes,
    })
}

/// Preserve relevance order while giving a repository-wide search one result
/// from each path before returning additional ranges from already-represented files.
/// Round-robin only across the highest-ranked paths. A very broad query can
/// match hundreds of low-value files; letting all of them consume the page
/// would hide useful follow-up ranges from the strongest matches.
fn diversify_by_path_limited<T: Copy>(
    candidates: Vec<(usize, T)>,
    path_for: &impl Fn(usize) -> String,
    max_paths: usize,
) -> Vec<(usize, T)> {
    let mut groups: Vec<(String, Vec<(usize, T)>)> = Vec::new();
    let mut group_by_path = HashMap::new();
    for candidate in candidates {
        let path = path_for(candidate.0);
        let group = if let Some(group) = group_by_path.get(&path) {
            *group
        } else {
            if groups.len() >= max_paths {
                continue;
            }
            groups.push((path.clone(), Vec::new()));
            let group = groups.len() - 1;
            group_by_path.insert(path, group);
            group
        };
        groups[group].1.push(candidate);
    }
    let rounds = groups
        .iter()
        .map(|(_, group)| group.len())
        .max()
        .unwrap_or(0);
    let mut diversified = Vec::new();
    for round in 0..rounds {
        for (_, group) in &groups {
            if let Some(candidate) = group.get(round) {
                diversified.push(*candidate);
            }
        }
    }
    diversified
}

#[cfg(test)]
mod tests;

mod cache;
mod incremental;
mod ranking;
