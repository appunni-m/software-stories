//! Reuse committed Tantivy segments in private staging, then apply normal deletes/adds.
//! Every current source is still verified before any old document is admitted.

use super::*;
use std::io;

pub(super) fn file_key(
    path: &str,
    sha256: &str,
    parser: &Option<index::ParserVersion>,
    symbols: &[&index::IndexedSymbol],
) -> Result<String, String> {
    // IDs can change between snapshots. Indexed declaration names and positions
    // determine postings; freshly built range metadata retains the current IDs.
    let mut declarations = symbols
        .iter()
        .map(|s| {
            (
                s.name.as_str(),
                s.qualified_name.as_str(),
                s.anchor.start_byte,
                s.anchor.end_byte,
            )
        })
        .collect::<Vec<_>>();
    declarations.sort_unstable();
    Ok(digest(&json(&(
        VERSION,
        path,
        sha256,
        parser,
        declarations,
    ))?))
}

pub(super) fn document_key(file_key: &str, start: usize, end: usize, sha256: &str) -> String {
    digest(format!("{file_key}:{start}:{end}:{sha256}").as_bytes())
}

pub(super) fn safe_component(path: &Path) -> bool {
    path.components().count() == 1
        && matches!(
            path.components().next(),
            Some(std::path::Component::Normal(_))
        )
}

pub(super) fn lookup(metadata: &Metadata) -> Option<HashMap<String, usize>> {
    if metadata.document_keys.len() != metadata.ranges.len() || metadata.files.len() > 25_000 {
        return None;
    }
    let mut files = HashMap::new();
    for file in &metadata.files {
        if (!safe_path(&file.path) && file.reason.as_deref() != Some("invalidPath"))
            || files.insert(file.path.as_str(), file).is_some()
            || (file.reason.is_some() && (file.chunks > 0 || file.index_key.is_some()))
            || (file.reason.is_none()
                && !file
                    .index_key
                    .as_ref()
                    .is_some_and(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())))
        {
            return None;
        }
    }
    let mut lookup = HashMap::new();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for (ordinal, (key, range)) in metadata
        .document_keys
        .iter()
        .zip(&metadata.ranges)
        .enumerate()
    {
        let file = files.get(range.path.as_str())?;
        if file.source_sha256 != range.source_sha256
            || *key
                != document_key(
                    file.index_key.as_deref()?,
                    range.start_byte,
                    range.end_byte,
                    &range.chunk_sha256,
                )
            || lookup.insert(key.clone(), ordinal).is_some()
        {
            return None;
        }
        *counts.entry(range.path.as_str()).or_default() += 1;
    }
    if metadata
        .files
        .iter()
        .any(|f| counts.get(f.path.as_str()).copied().unwrap_or(0) != f.chunks)
    {
        return None;
    }
    Some(lookup)
}

pub(super) fn base(
    cache: &cache::Cache,
    budget: &Budget<'_>,
) -> Result<Option<PreparedIndex>, String> {
    for scope in cache.candidate_scopes(budget)? {
        budget.check()?;
        if let Some(prepared) = load_generation(cache, &scope, None, budget) {
            return Ok(Some(prepared));
        }
    }
    budget.check()?;
    Ok(None)
}

pub(super) fn copy(
    base: &PreparedIndex,
    staging: &Path,
    budget: &Budget<'_>,
) -> Result<u64, String> {
    // Use Tantivy's own segment/managed-file inventories. No hard links: the
    // destination writer must never modify the base's metadata or delete files.
    let metas = base
        .index
        .load_metas()
        .map_err(|_| "The previous code index is unavailable.".to_owned())?;
    let managed = base.index.directory().list_managed_files();
    let active = metas
        .segments
        .iter()
        .flat_map(|s| s.list_files())
        .collect::<BTreeSet<_>>();
    let mut files = active
        .into_iter()
        .filter(|p| managed.contains(p))
        .collect::<BTreeSet<_>>();
    files.insert(PathBuf::from("meta.json"));
    files.insert(PathBuf::from(".managed.json"));
    if files.len() > 100_000 {
        return Err("The previous code index exceeds its file limit.".into());
    }
    let mut bytes = 0u64;
    for relative in files {
        budget.check()?;
        if !safe_component(&relative) {
            return Err("The previous code index has an invalid file path.".into());
        }
        let source = base.generation.join(&relative);
        let metadata = match fs::symlink_metadata(&source) {
            Ok(metadata) => metadata,
            // Managed paths may include optional components not present on disk.
            Err(error)
                if error.kind() == io::ErrorKind::NotFound
                    && relative != Path::new("meta.json")
                    && relative != Path::new(".managed.json") =>
            {
                continue;
            }
            Err(_) => return Err("The previous code index could not be copied.".into()),
        };
        if !metadata.is_file() {
            return Err("The previous code index has an invalid file.".into());
        }
        bytes = bytes.saturating_add(metadata.len());
        if bytes > 256 * 1024 * 1024 {
            return Err("The previous code index exceeds its byte limit.".into());
        }
        if fs::copy(&source, staging.join(relative))
            .map_err(|_| "The previous code index could not be copied.".to_owned())?
            != metadata.len()
        {
            return Err("The previous code index changed while copying.".into());
        }
    }
    budget.check()?;
    Ok(bytes)
}

#[cfg(test)]
mod tests;
