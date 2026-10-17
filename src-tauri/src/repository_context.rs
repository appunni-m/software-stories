//! Local, snapshot-scoped code retrieval. Syntax improves chunks; it never gates text search.
//!
//! This layer does not invoke AI, read a live working tree, or claim runtime/type resolution.
//! Immutable generations are published only after both metadata and Tantivy commit succeed.

use crate::{bounded_file, index, snapshot};
use grep_regex::RegexMatcherBuilder;
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
    collector::{Count, TopDocs},
    query::{BooleanQuery, BoostQuery, Occur, Query, TermQuery},
    schema::{Field, IndexRecordOption, STORED, STRING, Schema, TEXT, TantivyDocument, Value},
};
use uuid::Uuid;

const VERSION: &str = "atlas-ranges-v1-identifiers-v1-policy-v2";
pub(crate) const CHUNK_BYTES: usize = 8192;
const MAX_CHUNKS: usize = 50_000;
const MAX_SOURCE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_METADATA_BYTES: usize = 32 * 1024 * 1024;
const MAX_CACHE_BYTES: usize = 4 * 1024 * 1024;
const MAX_CANDIDATES: usize = 256;
pub(crate) const MAX_RANGE_BYTES: usize = 32_768;

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
    reason: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Coverage {
    pub(crate) files_indexed: usize,
    pub(crate) files_unavailable: usize,
    pub(crate) bytes_indexed: u64,
    pub(crate) chunks: usize,
    pub(crate) snapshot_paths_skipped: usize,
    pub(crate) syntax_files: usize,
    pub(crate) text_fallback_files: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BuildStats {
    pub(crate) elapsed_ms: u64,
    pub(crate) source_bytes_verified: u64,
    pub(crate) files_chunked: usize,
    pub(crate) files_chunk_cache_reused: usize,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Metadata {
    version: String,
    fingerprint: String,
    snapshot_id: String,
    project_id: String,
    ranges: Vec<CodeRange>,
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
    pub(crate) reused_generation: bool,
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
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResult {
    pub(crate) matches: Vec<SearchHit>,
    pub(crate) limited: bool,
    pub(crate) coverage: Coverage,
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
    let file = manifest
        .files
        .iter()
        .find(|f| f.path == path)
        .ok_or("This path is not in the selected snapshot.")?;
    if exclusion(file).is_some() {
        return Err("This captured path is excluded from code retrieval.".to_owned());
    }
    let source = snapshot::read_snapshot_source_from_manifest(storage, manifest, path)?;
    if snapshot::looks_secret_like(path, source.contents.as_bytes()) {
        return Err("This captured text is excluded by secret filtering.".to_owned());
    }
    Ok(source)
}

fn schema() -> Schema {
    let mut schema = Schema::builder();
    schema.add_u64_field("ordinal", STORED);
    schema.add_text_field("body", TEXT);
    schema.add_text_field("path", TEXT);
    schema.add_text_field("scope", STRING);
    schema.add_text_field("exact", STRING);
    schema.add_text_field("declaration", STRING);
    schema.add_text_field("symbol", TEXT);
    schema.build()
}

fn field(index: &Index, name: &str) -> Result<Field, String> {
    index
        .schema()
        .get_field(name)
        .map_err(|_| "The code index schema is invalid. Rebuild it.".to_owned())
}

/// Original spellings (lowercased) plus snake/camel/acronym components; no model or stemmer.
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

fn load_generation(directory: &Path, fingerprint: &str) -> Option<PreparedIndex> {
    let pointer: Pointer =
        serde_json::from_slice(&bounded_file::read(&directory.join("current.json"), 4096).ok()?)
            .ok()?;
    if pointer.fingerprint != fingerprint
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
        || metadata.fingerprint != fingerprint
        || metadata.ranges.len() > MAX_CHUNKS
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
    if !index.validate_checksum().ok()?.is_empty() {
        return None;
    }
    let reader = index.reader().ok()?;
    if reader.searcher().num_docs() as usize != metadata.ranges.len() {
        return None;
    }
    Some(PreparedIndex {
        index,
        metadata,
        reused_generation: true,
    })
}

struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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
    let fingerprint = fingerprint(manifest, syntax)?;
    let directory = root(
        storage,
        &manifest.summary.project_id,
        &manifest.summary.snapshot_id,
    );
    if let Some(prepared) = load_generation(&directory, &fingerprint) {
        budget.check()?;
        return Ok(prepared);
    }
    let started = Instant::now();
    fs::create_dir_all(&directory)
        .map_err(|_| "Local code index storage is unavailable.".to_owned())?;
    let generation_id = Uuid::new_v4().to_string();
    let staging = Staging(directory.join(format!("staging-{generation_id}")));
    fs::create_dir(&staging.0)
        .map_err(|_| "Local code index storage is unavailable.".to_owned())?;
    let index = Index::create_in_dir(&staging.0, schema())
        .map_err(|_| "Local code index could not be created.".to_owned())?;
    let mut writer: IndexWriter = index
        .writer_with_num_threads(1, 30_000_000)
        .map_err(|_| "Local code index writer is unavailable.".to_owned())?;
    let ordinal = field(&index, "ordinal")?;
    let body = field(&index, "body")?;
    let path_field = field(&index, "path")?;
    let exact = field(&index, "exact")?;
    let scope = field(&index, "scope")?;
    let declaration = field(&index, "declaration")?;
    let symbol_field = field(&index, "symbol")?;
    let cache_dir = storage.join("repository-context").join("chunk-cache");
    fs::create_dir_all(&cache_dir)
        .map_err(|_| "Local code chunk storage is unavailable.".to_owned())?;
    let mut metadata = Metadata {
        version: VERSION.to_owned(),
        fingerprint: fingerprint.clone(),
        project_id: manifest.summary.project_id.clone(),
        snapshot_id: manifest.summary.snapshot_id.clone(),
        ranges: vec![],
        files: vec![],
        coverage: Coverage {
            snapshot_paths_skipped: manifest.skipped.len(),
            ..Default::default()
        },
        build: BuildStats::default(),
    };
    let mut files = manifest.files.iter().collect::<Vec<_>>();
    files.sort_by_key(|f| &f.path);
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
    for file in files {
        budget.check()?;
        let mut coverage = FileCoverage {
            path: file.path.clone(),
            source_sha256: file.sha256.clone(),
            bytes: file.size_bytes,
            chunks: 0,
            parser: None,
            reason: exclusion(file).map(str::to_owned),
        };
        if coverage.reason.is_some() {
            metadata.files.push(coverage);
            continue;
        }
        admitted_bytes = admitted_bytes.saturating_add(file.size_bytes);
        if admitted_bytes > MAX_SOURCE_BYTES {
            return Err("The captured text exceeds the local code index byte limit.".to_owned());
        }
        let source = match source(storage, manifest, &file.path) {
            Ok(source) => source,
            Err(_) => {
                coverage.reason = Some("unavailableOrFilteredText".to_owned());
                metadata.files.push(coverage);
                continue;
            }
        };
        metadata.build.source_bytes_verified += source.size_bytes;
        if metadata.build.source_bytes_verified > MAX_SOURCE_BYTES {
            return Err("The captured text exceeds the local code index byte limit.".to_owned());
        }
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
        coverage.parser = syntax
            .and_then(|s| {
                s.files
                    .iter()
                    .find(|f| f.path == file.path && f.source_sha256 == file.sha256)
            })
            .and_then(|f| f.parser.clone());
        let key = digest(&json(&(
            VERSION,
            &source.sha256,
            &symbols,
            &coverage.parser,
        ))?);
        let cache_path = cache_dir.join(format!("{key}.json"));
        let cached = bounded_file::read(&cache_path, MAX_CACHE_BYTES)
            .ok()
            .and_then(|b| serde_json::from_slice::<ChunkCache>(&b).ok())
            .filter(|c| {
                c.version == VERSION
                    && c.key == key
                    && json(&c.chunks).is_ok_and(|bytes| digest(&bytes) == c.chunks_sha256)
            });
        // Validate the complete partition and coordinates against freshly hash-checked source.
        let local_chunks = if let Some(cache) =
            cached.filter(|cache| valid_chunks(&cache.chunks, &source.contents))
        {
            metadata.build.files_chunk_cache_reused += 1;
            cache.chunks
        } else {
            metadata.build.files_chunked += 1;
            let chunks = chunks(&source.contents, &symbols, budget)?;
            let encoded = json(&ChunkCache {
                version: VERSION.to_owned(),
                key: key.clone(),
                chunks_sha256: digest(&json(&chunks)?),
                chunks: chunks.clone(),
            })?;
            if encoded.len() <= MAX_CACHE_BYTES {
                let temporary = cache_dir.join(format!("{key}-{}.tmp", Uuid::new_v4()));
                if write_new(&temporary, &encoded).is_ok() {
                    let _ = fs::rename(&temporary, &cache_path);
                }
                let _ = fs::remove_file(temporary);
            }
            chunks
        };
        coverage.chunks = local_chunks.len();
        if metadata.ranges.len().saturating_add(local_chunks.len()) > MAX_CHUNKS {
            return Err("The captured text exceeds the local code range limit.".to_owned());
        }
        for chunk in local_chunks {
            budget.check()?;
            let text = &source.contents[chunk.start_byte..chunk.end_byte];
            let mut doc = TantivyDocument::default();
            doc.add_u64(ordinal, metadata.ranges.len() as u64);
            doc.add_text(body, terms(text).join(" "));
            doc.add_text(path_field, terms(&file.path).join(" "));
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
    budget.check()?;
    writer
        .commit()
        .map_err(|_| "Local code index could not be committed.".to_owned())?;
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
    let encoded = json(&metadata)?;
    if encoded.len() > MAX_METADATA_BYTES {
        return Err("Local code index metadata exceeds its size limit.".to_owned());
    }
    write_new(&staging.0.join("ranges.json"), &encoded)?;
    let pointer = Pointer {
        generation: generation_id.clone(),
        fingerprint,
        metadata_sha256: digest(&encoded),
    };
    let previous = bounded_file::read(&directory.join("current.json"), 4096)
        .ok()
        .and_then(|b| serde_json::from_slice::<Pointer>(&b).ok());
    // No reader can see staging. Rename on the same volume, then replace the tiny pointer.
    let published = directory.join(&generation_id);
    fs::rename(&staging.0, &published)
        .map_err(|_| "Local code index could not be published.".to_owned())?;
    let temporary = directory.join(format!("pointer-{generation_id}.tmp"));
    if let Err(error) = write_new(&temporary, &json(&pointer)?).and_then(|_| {
        budget.check()?;
        fs::rename(&temporary, directory.join("current.json"))
            .map_err(|_| "Local code index could not be published.".to_owned())
    }) {
        let _ = fs::remove_file(temporary);
        let _ = fs::remove_dir_all(published);
        return Err(error);
    }
    // Keep the previous generation until the next build. Native operations are serialized;
    // a crash leaves either complete pointer intact, and immutable snapshots can be rebuilt.
    for entry in fs::read_dir(&directory)
        .into_iter()
        .flatten()
        .flatten()
        .take(128)
    {
        if entry.file_type().is_ok_and(|t| t.is_dir())
            && entry.file_name() != generation_id.as_str()
            && !previous
                .as_ref()
                .is_some_and(|p| entry.file_name() == p.generation.as_str())
            && Uuid::parse_str(&entry.file_name().to_string_lossy()).is_ok()
        {
            let _ = fs::remove_dir_all(entry.path());
        }
    }
    Ok(PreparedIndex {
        index: Index::open_in_dir(published)
            .map_err(|_| "Published code index is unavailable.".to_owned())?,
        metadata,
        reused_generation: false,
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
    let contents = source
        .contents
        .get(start_byte..end_byte)
        .ok_or("The code range must lie on UTF-8 boundaries inside this file.")?
        .to_owned();
    let starts = line_starts(&source.contents);
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

pub(crate) fn validate_search(
    query: &str,
    path: Option<&str>,
    limit: usize,
) -> Result<Vec<String>, String> {
    if !(2..=240).contains(&query.trim().chars().count())
        || query.chars().any(char::is_control)
        || !(1..=32).contains(&limit)
    {
        return Err("Search with 2–240 characters and at most 32 results.".to_owned());
    }
    if path.is_some_and(|p| !safe_path(p)) {
        return Err("Search within a captured file or folder.".to_owned());
    }
    let words = terms(query);
    if words.is_empty() || words.len() > 32 {
        return Err("Use up to 32 searchable terms.".to_owned());
    }
    Ok(words)
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
    let started = Instant::now();
    budget.check()?;
    if prepared.metadata.project_id != manifest.summary.project_id
        || prepared.metadata.snapshot_id != manifest.summary.snapshot_id
    {
        return Err("The code index belongs to a different snapshot.".to_owned());
    }
    let words = validate_search(query, path, limit)?;
    let query = query.trim();
    let body = field(&prepared.index, "body")?;
    let path_field = field(&prepared.index, "path")?;
    let symbol_field = field(&prepared.index, "symbol")?;
    let mut clauses = vec![];
    for word in &words {
        clauses.push((Occur::Should, term_query(body, word)));
        clauses.push((
            Occur::Should,
            Box::new(BoostQuery::new(term_query(symbol_field, word), 3.0)) as Box<dyn Query>,
        ));
        clauses.push((
            Occur::Should,
            Box::new(BoostQuery::new(term_query(path_field, word), 1.5)) as Box<dyn Query>,
        ));
    }
    let lexical = scoped(Box::new(BooleanQuery::new(clauses)), &prepared.index, path)?;
    let reader = prepared
        .index
        .reader()
        .map_err(|_| "Code search index could not be opened. Try rebuilding it.".to_owned())?;
    let searcher = reader.searcher();
    let (mut found, total) = searcher
        .search(
            &lexical,
            &(TopDocs::with_limit(MAX_CANDIDATES).order_by_score(), Count),
        )
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
        let (hits, count) = searcher
            .search(
                &exact,
                &(TopDocs::with_limit(MAX_CANDIDATES).order_by_score(), Count),
            )
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
        let (hits, count) = searcher
            .search(
                &declarations,
                &(TopDocs::with_limit(MAX_CANDIDATES).order_by_score(), Count),
            )
            .map_err(|_| "Declaration search failed.".to_owned())?;
        exact_limited |= count > MAX_CANDIDATES;
        for (score, address) in hits {
            declaration_docs.insert(address);
            exact_docs.insert(address);
            found.push((score, address));
        }
    }
    budget.check()?;
    let ordinal = field(&prepared.index, "ordinal")?;
    let mut candidates = HashMap::new();
    for (score, address) in found {
        let doc = searcher
            .doc::<TantivyDocument>(address)
            .map_err(|_| "A code search result is unavailable.".to_owned())?;
        let id = doc
            .get_first(ordinal)
            .and_then(|v| v.as_u64())
            .and_then(|v| usize::try_from(v).ok())
            .ok_or("The code search index is invalid.")?;
        if id >= prepared.metadata.ranges.len() {
            return Err("The code search index is invalid.".to_owned());
        }
        let exact = exact_docs.contains(&address);
        let declaration = declaration_docs.contains(&address);
        candidates
            .entry(id)
            .and_modify(|s: &mut (bool, bool, f32)| {
                s.0 |= exact;
                s.1 |= declaration;
                s.2 = s.2.max(score);
            })
            .or_insert((exact, declaration, score));
    }
    let mut ranked = candidates.into_iter().collect::<Vec<_>>();
    ranked.sort_by(|(a, (ae, ad, ascore)), (b, (be, bd, bscore))| {
        bd.cmp(ad)
            .then_with(|| be.cmp(ae))
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
    });
    let limited = exact_limited || total > limit || ranked.len() > limit;
    let exact_terms = vec![query.rsplit("::").next().unwrap_or(query).to_owned()];
    let matcher = RegexMatcherBuilder::new()
        .case_insensitive(true)
        .fixed_strings(true)
        .build_literals(&words)
        .map_err(|_| "Code search terms are invalid.".to_owned())?;
    let exact_matcher = RegexMatcherBuilder::new()
        .case_insensitive(true)
        .fixed_strings(true)
        .build_literals(&exact_terms)
        .map_err(|_| "Code search terms are invalid.".to_owned())?;
    let mut matches = vec![];
    let mut sources = HashMap::new();
    for (id, (exact_identifier, declaration_match, score)) in ranked.into_iter().take(limit) {
        budget.check()?;
        let range = &prepared.metadata.ranges[id];
        if !sources.contains_key(&range.path) {
            sources.insert(range.path.clone(), source(storage, manifest, &range.path)?);
        }
        let captured = sources
            .get(&range.path)
            .ok_or("Captured source is unavailable.")?;
        if captured.sha256 != range.source_sha256 {
            return Err("A code result belongs to a different source version.".to_owned());
        }
        let read = range_from_source(captured, range.start_byte, range.end_byte)?;
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
        matches.push(SearchHit {
            range: range.clone(),
            line,
            text,
            exact_identifier,
            declaration_match,
            score,
        });
    }
    budget.check()?;
    let result_bytes = json(&matches)?.len();
    Ok(SearchResult {
        matches,
        limited,
        coverage: prepared.metadata.coverage.clone(),
        reused_generation: prepared.reused_generation,
        build: prepared.metadata.build.clone(),
        elapsed_ms: started.elapsed().as_millis() as u64,
        result_bytes,
    })
}

#[cfg(test)]
mod tests;
