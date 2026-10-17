//! Bounded structural context over immutable evidence. Syntax links are never runtime proof.

use crate::{index, repository_context as context, snapshot};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
};

const MAX_PAGE: usize = 32;
const MAX_OFFSET: usize = 100_000;
const MAX_VERIFIED_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Direction {
    Incoming,
    Outgoing,
    Both,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Page<T> {
    pub(crate) snapshot_id: String,
    pub(crate) items: Vec<T>,
    pub(crate) next_offset: Option<usize>,
    pub(crate) unavailable: usize,
    pub(crate) syntax_available: bool,
    pub(crate) syntax_status: Option<index::SnapshotIndexStatus>,
    pub(crate) indexer_version: Option<String>,
    pub(crate) scoped_syntax_status: Option<index::IndexFileStatus>,
    pub(crate) snapshot_paths_skipped: usize,
    pub(crate) interpretation: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Symbol {
    pub(crate) symbol_id: String,
    pub(crate) name: String,
    pub(crate) qualified_name: String,
    pub(crate) kind: index::SymbolKind,
    pub(crate) language: String,
    pub(crate) range: context::CodeRange,
    pub(crate) declaration_end_byte: usize,
    pub(crate) match_kind: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Endpoint {
    pub(crate) id: String,
    pub(crate) path: String,
    pub(crate) label: String,
    pub(crate) source_sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Relationship {
    pub(crate) relationship_id: String,
    pub(crate) kind: index::RelationshipKind,
    pub(crate) confidence: index::RelationshipConfidence,
    pub(crate) resolution: &'static str,
    pub(crate) source: Option<Endpoint>,
    pub(crate) target: Option<Endpoint>,
    pub(crate) target_label: Option<String>,
    pub(crate) unresolved_reason: Option<String>,
    pub(crate) evidence: context::CodeRange,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Module {
    pub(crate) path: String,
    pub(crate) directory: bool,
    pub(crate) captured_text_files: usize,
    pub(crate) code_files: usize,
    pub(crate) captured_bytes: u64,
    pub(crate) syntax_files: usize,
    pub(crate) public_symbols: Vec<String>,
}

fn validate_page(path: Option<&str>, limit: usize, offset: usize) -> Result<(), String> {
    if path.is_some_and(|p| !context::safe_path(p))
        || !(1..=MAX_PAGE).contains(&limit)
        || offset > MAX_OFFSET
    {
        return Err(
            "Choose a captured file/folder and a page of at most 32 context items.".to_owned(),
        );
    }
    Ok(())
}

fn within(path: &str, scope: Option<&str>) -> bool {
    scope.is_none_or(|scope| {
        path == scope
            || path
                .strip_prefix(scope)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

fn checked_syntax<'a>(
    manifest: &snapshot::SnapshotManifest,
    syntax: Option<&'a index::SnapshotIndex>,
) -> Result<Option<&'a index::SnapshotIndex>, String> {
    if syntax.is_some_and(|s| {
        s.project_id != manifest.summary.project_id || s.snapshot_id != manifest.summary.snapshot_id
    }) {
        return Err("The syntax context belongs to a different snapshot.".to_owned());
    }
    Ok(syntax)
}

fn page<T>(
    manifest: &snapshot::SnapshotManifest,
    syntax: Option<&index::SnapshotIndex>,
    scope: Option<&str>,
    items: Vec<T>,
    next_offset: Option<usize>,
    unavailable: usize,
    interpretation: &'static str,
) -> Page<T> {
    Page {
        snapshot_id: manifest.summary.snapshot_id.clone(),
        items,
        next_offset,
        unavailable,
        syntax_available: syntax.is_some(),
        syntax_status: syntax.map(|s| s.status.clone()),
        indexer_version: syntax.map(|s| s.indexer_version.clone()),
        scoped_syntax_status: scope.and_then(|scope| {
            let captured = manifest.files.iter().find(|f| f.path == scope)?;
            if context::exclusion(captured).is_some() {
                return None;
            }
            syntax?
                .files
                .iter()
                .find(|f| f.path == scope && f.source_sha256 == captured.sha256)
                .map(|f| f.status.clone())
        }),
        snapshot_paths_skipped: manifest.skipped.len(),
        interpretation,
    }
}

struct Sources<'a> {
    storage: &'a Path,
    manifest: &'a snapshot::SnapshotManifest,
    sources: HashMap<String, Result<snapshot::SnapshotSource, String>>,
    admitted: u64,
}

impl<'a> Sources<'a> {
    fn new(storage: &'a Path, manifest: &'a snapshot::SnapshotManifest) -> Self {
        Self {
            storage,
            manifest,
            sources: HashMap::new(),
            admitted: 0,
        }
    }

    fn get(&mut self, path: &str) -> Result<&snapshot::SnapshotSource, String> {
        if !self.sources.contains_key(path) {
            let bytes = self
                .manifest
                .files
                .iter()
                .find(|f| f.path == path)
                .ok_or("Uncaptured context path.")?
                .size_bytes;
            let result = if bytes > MAX_VERIFIED_BYTES.saturating_sub(self.admitted) {
                Err("Structural context exceeded its local source budget.".to_owned())
            } else {
                self.admitted += bytes;
                context::source(self.storage, self.manifest, path)
            };
            self.sources.insert(path.to_owned(), result);
        }
        self.sources
            .get(path)
            .ok_or("Captured context is unavailable.".to_owned())?
            .as_ref()
            .map_err(Clone::clone)
    }

    fn anchor(&mut self, anchor: &index::SourceAnchor) -> Result<context::CodeRange, String> {
        let source = self.get(&anchor.path)?;
        if source.sha256 != anchor.source_sha256
            || anchor.start_byte >= anchor.end_byte
            || anchor.end_byte > source.contents.len()
            || !source.contents.is_char_boundary(anchor.start_byte)
            || !source.contents.is_char_boundary(anchor.end_byte)
        {
            return Err("The syntax evidence no longer matches its captured source.".to_owned());
        }
        let mut end = anchor
            .end_byte
            .min(anchor.start_byte + context::CHUNK_BYTES);
        while !source.contents.is_char_boundary(end) {
            end -= 1;
        }
        let range = context::range_from_source(source, anchor.start_byte, end)?.range;
        // Tree-sitter uses the exclusive end point, sometimes on the following line.
        let line = source.contents[..anchor.start_byte]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1;
        let line_start = source.contents[..anchor.start_byte]
            .rfind('\n')
            .map_or(0, |p| p + 1);
        let end_line = source.contents[..anchor.end_byte]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1;
        let end_line_start = source.contents[..anchor.end_byte]
            .rfind('\n')
            .map_or(0, |p| p + 1);
        if anchor.start_line != line
            || anchor.start_column != anchor.start_byte - line_start + 1
            || anchor.end_line != end_line
            || anchor.end_column != anchor.end_byte - end_line_start + 1
        {
            return Err("The syntax evidence has invalid source coordinates.".to_owned());
        }
        Ok(range)
    }
}

fn match_rank(symbol: &index::IndexedSymbol, query: &str) -> Option<(u8, &'static str)> {
    if symbol.qualified_name == query {
        Some((0, "exactQualified"))
    } else if symbol.name == query {
        Some((1, "exactName"))
    } else if symbol.qualified_name.eq_ignore_ascii_case(query)
        || symbol.name.eq_ignore_ascii_case(query)
    {
        Some((2, "caseFolded"))
    } else if query
        .rsplit("::")
        .next()
        .is_some_and(|name| name == symbol.name)
    {
        Some((3, "unqualifiedName"))
    } else if symbol
        .qualified_name
        .to_lowercase()
        .contains(&query.to_lowercase())
    {
        Some((4, "partialName"))
    } else {
        None
    }
}

pub(crate) fn definitions(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    syntax: Option<&index::SnapshotIndex>,
    query: &str,
    scope: Option<&str>,
    limit: usize,
    offset: usize,
    budget: &context::Budget<'_>,
) -> Result<Page<Symbol>, String> {
    validate_page(scope, limit, offset)?;
    budget.check()?;
    if !(2..=240).contains(&query.chars().count()) || query.chars().any(char::is_control) {
        return Err("Find a declaration using 2–240 characters.".to_owned());
    }
    let syntax = checked_syntax(manifest, syntax)?;
    let Some(index) = syntax else {
        return Ok(page(
            manifest,
            None,
            scope,
            vec![],
            None,
            0,
            "No syntax index is available; use captured text search.",
        ));
    };
    let mut candidates = index
        .symbols
        .iter()
        .filter(|s| within(&s.path, scope))
        .filter_map(|s| match_rank(s, query).map(|rank| (s, rank)))
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(s, (rank, _))| {
        (
            *rank,
            s.path.as_str(),
            s.anchor.start_byte,
            s.symbol_id.as_str(),
        )
    });
    let mut sources = Sources::new(storage, manifest);
    let mut items = vec![];
    let mut unavailable = 0;
    let total = candidates.len();
    let mut scanned = offset.min(total);
    for (symbol, (_, match_kind)) in candidates.into_iter().skip(offset) {
        budget.check()?;
        scanned += 1;
        if symbol.path != symbol.anchor.path
            || symbol.name.len() > 240
            || symbol.qualified_name.len() > 1024
        {
            unavailable += 1;
            continue;
        }
        let mut range = match sources.anchor(&symbol.anchor) {
            Ok(range) => range,
            Err(_) => {
                unavailable += 1;
                continue;
            }
        };
        range.parser = index
            .files
            .iter()
            .find(|f| f.path == symbol.path && f.source_sha256 == range.source_sha256)
            .and_then(|f| f.parser.clone());
        range.enclosing = Some(context::SymbolContext {
            symbol_id: symbol.symbol_id.clone(),
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            start_byte: symbol.anchor.start_byte,
            end_byte: symbol.anchor.end_byte,
        });
        items.push(Symbol {
            symbol_id: symbol.symbol_id.clone(),
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            kind: symbol.kind.clone(),
            language: symbol.language.clone(),
            declaration_end_byte: symbol.anchor.end_byte,
            range,
            match_kind,
        });
        if items.len() == limit {
            break;
        }
    }
    budget.check()?;
    Ok(page(
        manifest,
        syntax,
        scope,
        items,
        (scanned < total).then_some(scanned),
        unavailable,
        "Syntactic declarations from the captured parser index; not compiler or runtime resolution.",
    ))
}

fn endpoint(
    id: &str,
    syntax: &index::SnapshotIndex,
    sources: &mut Sources<'_>,
) -> Option<Endpoint> {
    if let Some(symbol) = syntax.symbols.iter().find(|s| s.symbol_id == id) {
        sources.anchor(&symbol.anchor).ok()?;
        let source = sources.get(&symbol.path).ok()?;
        if symbol.path != symbol.anchor.path || symbol.anchor.source_sha256 != source.sha256 {
            return None;
        }
        return Some(Endpoint {
            id: id.to_owned(),
            path: symbol.path.clone(),
            label: symbol.qualified_name.chars().take(1024).collect(),
            source_sha256: source.sha256.clone(),
        });
    }
    let file = syntax.files.iter().find(|f| f.file_id == id)?;
    let source = sources.get(&file.path).ok()?;
    if file.source_sha256 != source.sha256 {
        return None;
    }
    Some(Endpoint {
        id: id.to_owned(),
        path: file.path.clone(),
        label: file.path.clone(),
        source_sha256: source.sha256.clone(),
    })
}

fn relationship_rank(kind: &index::RelationshipKind) -> u8 {
    match kind {
        index::RelationshipKind::Calls => 0,
        index::RelationshipKind::Imports => 1,
        index::RelationshipKind::TestReference => 2,
        index::RelationshipKind::Exports => 3,
        index::RelationshipKind::Defines => 4,
    }
}

pub(crate) fn relationships(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    syntax: Option<&index::SnapshotIndex>,
    id: &str,
    direction: Direction,
    scope: Option<&str>,
    limit: usize,
    offset: usize,
    budget: &context::Budget<'_>,
) -> Result<Page<Relationship>, String> {
    validate_page(scope, limit, offset)?;
    budget.check()?;
    if id.is_empty() || id.len() > 512 || id.chars().any(char::is_control) {
        return Err("Choose a captured symbol or file ID.".to_owned());
    }
    let syntax = checked_syntax(manifest, syntax)?;
    let Some(index) = syntax else {
        return Ok(page(
            manifest,
            None,
            scope,
            vec![],
            None,
            0,
            "No static relationships are available; use captured text search.",
        ));
    };
    let mut sources = Sources::new(storage, manifest);
    if endpoint(id, index, &mut sources).is_none() {
        return Ok(page(
            manifest,
            syntax,
            scope,
            vec![],
            None,
            1,
            "The requested endpoint is unavailable; no relationships were disclosed.",
        ));
    }
    let mut candidates = index
        .relationships
        .iter()
        .filter(|r| {
            within(&r.evidence.path, scope)
                && match direction {
                    Direction::Incoming => r.target_id.as_deref() == Some(id),
                    Direction::Outgoing => r.source_id == id,
                    Direction::Both => r.source_id == id || r.target_id.as_deref() == Some(id),
                }
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|r| {
        (
            relationship_rank(&r.kind),
            r.evidence.path.as_str(),
            r.evidence.start_byte,
            r.relationship_id.as_str(),
        )
    });
    let total = candidates.len();
    let mut scanned = offset.min(total);
    let mut items = vec![];
    let mut unavailable = 0;
    for relation in candidates.into_iter().skip(offset) {
        budget.check()?;
        scanned += 1;
        let evidence = match sources.anchor(&relation.evidence) {
            Ok(range) => range,
            Err(_) => {
                unavailable += 1;
                continue;
            }
        };
        let source = endpoint(&relation.source_id, index, &mut sources);
        if source.is_none() {
            unavailable += 1;
            continue;
        }
        let target = relation
            .target_id
            .as_deref()
            .and_then(|id| endpoint(id, index, &mut sources));
        let missing_target = relation.target_id.is_some() && target.is_none();
        let unresolved =
            relation.confidence == index::RelationshipConfidence::Unresolved || target.is_none();
        let target_label = if unresolved && relation.target_id.is_some() {
            None
        } else {
            relation
                .target_label
                .as_ref()
                .map(|s| s.chars().filter(|c| !c.is_control()).take(512).collect())
        };
        items.push(Relationship {
            relationship_id: relation.relationship_id.clone(),
            kind: relation.kind.clone(),
            confidence: if unresolved {
                index::RelationshipConfidence::Unresolved
            } else {
                relation.confidence.clone()
            },
            resolution: if unresolved {
                "unresolved"
            } else {
                "staticCandidate"
            },
            source,
            target,
            target_label,
            unresolved_reason: if unresolved {
                Some(if missing_target {
                    "The recorded target is unavailable in eligible captured text.".to_owned()
                } else {
                    relation
                        .unresolved_reason
                        .as_ref()
                        .map(|s| s.chars().filter(|c| !c.is_control()).take(512).collect())
                        .unwrap_or_else(|| {
                            "The parser did not resolve a captured target.".to_owned()
                        })
                })
            } else {
                None
            },
            evidence,
        });
        if items.len() == limit {
            break;
        }
    }
    budget.check()?;
    Ok(page(
        manifest,
        syntax,
        scope,
        items,
        (scanned < total).then_some(scanned),
        unavailable,
        "Static parser candidates and unresolved links; these do not prove runtime execution or compiler resolution.",
    ))
}

/// An orientation map; even hash-verified files do not establish module execution order.
pub(crate) fn module_map(
    storage: &Path,
    manifest: &snapshot::SnapshotManifest,
    syntax: Option<&index::SnapshotIndex>,
    scope: Option<&str>,
    limit: usize,
    offset: usize,
    budget: &context::Budget<'_>,
) -> Result<Page<Module>, String> {
    validate_page(scope, limit, offset)?;
    budget.check()?;
    let syntax = checked_syntax(manifest, syntax)?;
    let mut sources = Sources::new(storage, manifest);
    let mut syntax_files = HashMap::new();
    let mut exported = HashMap::<&str, Vec<&index::IndexedSymbol>>::new();
    if let Some(syntax) = syntax {
        for file in &syntax.files {
            syntax_files.insert(file.path.as_str(), file);
        }
        for symbol in &syntax.symbols {
            if symbol.is_exported {
                exported.entry(&symbol.path).or_default().push(symbol);
            }
        }
        for symbols in exported.values_mut() {
            symbols.sort_by_key(|s| (&s.qualified_name, s.anchor.start_byte));
        }
    }
    let mut modules = BTreeMap::<String, Module>::new();
    let mut unavailable = 0;
    let mut files = manifest
        .files
        .iter()
        .filter(|f| within(&f.path, scope))
        .collect::<Vec<_>>();
    files.sort_by_key(|f| &f.path);
    for file in files {
        budget.check()?;
        if context::exclusion(file).is_some() {
            unavailable += 1;
            continue;
        }
        if sources.get(&file.path).is_err() {
            unavailable += 1;
            continue;
        }
        let relative = scope
            .and_then(|scope| file.path.strip_prefix(&format!("{scope}/")))
            .unwrap_or(&file.path);
        let first = relative.split('/').next().unwrap_or(relative);
        let path = if scope == Some(file.path.as_str()) {
            file.path.clone()
        } else {
            scope.map_or_else(|| first.to_owned(), |scope| format!("{scope}/{first}"))
        };
        let directory = path != file.path;
        let entry = modules.entry(path.clone()).or_insert_with(|| Module {
            path,
            directory,
            captured_text_files: 0,
            code_files: 0,
            captured_bytes: 0,
            syntax_files: 0,
            public_symbols: vec![],
        });
        entry.captured_text_files += 1;
        entry.captured_bytes = entry.captured_bytes.saturating_add(file.size_bytes);
        entry.code_files += usize::from(crate::source_discovery::is_code_path(&file.path));
        if syntax.is_some() {
            entry.syntax_files += usize::from(
                syntax_files
                    .get(file.path.as_str())
                    .is_some_and(|f| f.source_sha256 == file.sha256 && f.parser.is_some()),
            );
            for symbol in exported.get(file.path.as_str()).into_iter().flatten() {
                if entry.public_symbols.len() < 4
                    && symbol.path == symbol.anchor.path
                    && symbol.anchor.source_sha256 == file.sha256
                    && symbol.qualified_name.len() <= 1024
                    && sources.anchor(&symbol.anchor).is_ok()
                {
                    entry.public_symbols.push(symbol.qualified_name.clone());
                }
            }
        }
    }
    let mut modules = modules.into_values().collect::<Vec<_>>();
    modules.sort_by(|a, b| {
        b.code_files
            .cmp(&a.code_files)
            .then_with(|| a.path.cmp(&b.path))
    });
    let total = modules.len();
    let next = offset.saturating_add(limit);
    let items = modules.into_iter().skip(offset).take(limit).collect();
    budget.check()?;
    Ok(page(
        manifest,
        syntax,
        scope,
        items,
        (next < total).then_some(next),
        unavailable,
        "Hash-verified captured text inventory and static parser metadata; not module execution order or complete architecture.",
    ))
}

#[cfg(test)]
mod tests;
