//! Retention for rebuildable search data only. Saved sources and answers live elsewhere.
//!
//! A maintenance lock serializes publishers across processes. Prepared indexes hold
//! shared scope/generation leases so eviction cannot invalidate a later Tantivy reader.

use super::Budget;
use std::{
    collections::{BTreeSet, HashMap},
    fs::{self, File, OpenOptions, TryLockError},
    io,
    path::{Path, PathBuf},
    thread,
    time::{Duration, SystemTime},
};
use uuid::Uuid;

const UNAVAILABLE: &str = "Local code search storage is unavailable.";
const FULL: &str = "Local code search storage is full. Close other searches and try again.";
const SCAN_ENTRIES: usize = 100_000;

#[derive(Clone, Copy)]
struct Limits {
    bytes: u64,
    scopes: usize,
    chunk_bytes: u64,
    chunks: usize,
    generation_bytes: u64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            bytes: 512 * 1024 * 1024,
            scopes: 32,
            chunk_bytes: 64 * 1024 * 1024,
            chunks: 8192,
            generation_bytes: 256 * 1024 * 1024,
        }
    }
}

pub(super) struct ReadLease {
    _generation: File,
    _scope: File,
}

impl Drop for ReadLease {
    fn drop(&mut self) {
        // A concurrent process spawn can briefly inherit the same open file
        // descriptions. End this reader's ownership now, even while those
        // handles remain open; closing File alone waits for the last handle.
        let _ = self._generation.unlock();
        let _ = self._scope.unlock();
    }
}

#[derive(Clone)]
struct Entry {
    bytes: u64,
    used: SystemTime,
}

struct Scope {
    path: PathBuf,
    bytes: u64,
    used: SystemTime,
}

pub(super) struct Cache {
    root: PathBuf,
    pub(super) directory: PathBuf,
    // All preparation, lease acquisition and pruning finish before this handle drops.
    _maintenance: File,
    limits: Limits,
    chunks: HashMap<PathBuf, Entry>,
    chunk_order: BTreeSet<(SystemTime, PathBuf)>,
    chunk_bytes: u64,
}

impl Drop for Cache {
    fn drop(&mut self) {
        // Process creation may temporarily inherit this open file description.
        // Maintenance ends with this owner, even before the inherited handle closes.
        let _ = self._maintenance.unlock();
    }
}

fn directory(path: &Path) -> Result<(), String> {
    match fs::create_dir(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            if fs::symlink_metadata(path).is_ok_and(|m| m.is_dir()) {
                Ok(())
            } else {
                Err(UNAVAILABLE.into())
            }
        }
        Err(_) => Err(UNAVAILABLE.into()),
    }
}

// Ownership guards explicitly unlock at drop. O_CLOEXEC also closes inherited
// handles at exec; process exit closes any remaining handles.
fn lock_file(path: &Path) -> Result<File, String> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    }
    if fs::symlink_metadata(path).is_ok_and(|m| !m.is_file()) {
        return Err(UNAVAILABLE.into());
    }
    let file = options.open(path).map_err(|_| UNAVAILABLE.to_owned())?;
    if !file.metadata().is_ok_and(|m| m.is_file()) {
        return Err(UNAVAILABLE.into());
    }
    Ok(file)
}

fn exclusive(path: &Path) -> Result<Option<File>, String> {
    let file = lock_file(path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(TryLockError::WouldBlock) => Ok(None),
        Err(TryLockError::Error(_)) => Err(UNAVAILABLE.into()),
    }
}

fn shared(path: &Path) -> Result<File, String> {
    let file = lock_file(path)?;
    file.try_lock_shared().map_err(|_| UNAVAILABLE.to_owned())?;
    Ok(file)
}

fn sha_name(name: &str) -> bool {
    name.len() == 64
        && name
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn uuid_name(name: &str) -> bool {
    Uuid::parse_str(name).is_ok_and(|id| id.to_string() == name)
}

fn modified(metadata: &fs::Metadata) -> SystemTime {
    metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH)
}

/// Count apparent file bytes without following links or silently truncating the scan.
/// The scan itself is bounded; unexpected/oversized trees fail closed for maintenance.
fn size(
    path: &Path,
    depth: usize,
    remaining: &mut usize,
    budget: &Budget<'_>,
) -> Result<u64, String> {
    budget.check()?;
    if depth > 8 || *remaining == 0 {
        return Err(UNAVAILABLE.into());
    }
    *remaining -= 1;
    let metadata = fs::symlink_metadata(path).map_err(|_| UNAVAILABLE.to_owned())?;
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Err(UNAVAILABLE.into());
    }
    let mut bytes = 0u64;
    for entry in fs::read_dir(path).map_err(|_| UNAVAILABLE.to_owned())? {
        bytes = bytes.saturating_add(size(
            &entry.map_err(|_| UNAVAILABLE.to_owned())?.path(),
            depth + 1,
            remaining,
            budget,
        )?);
    }
    Ok(bytes)
}

impl Cache {
    /// Open an existing snapshot generation for query without maintenance or
    /// cache pruning. A shared maintenance lease prevents concurrent cleanup
    /// while the generation's reader lease is acquired.
    pub(super) fn open_reader(
        storage: &Path,
        project: &str,
        snapshot: &str,
        budget: &Budget<'_>,
    ) -> Result<Option<Self>, String> {
        budget.check()?;
        let root = storage.join("repository-context");
        if !fs::symlink_metadata(&root).is_ok_and(|metadata| metadata.is_dir()) {
            return Ok(None);
        }
        let maintenance_path = root.join("maintenance.lock");
        if !fs::symlink_metadata(&maintenance_path).is_ok_and(|metadata| metadata.is_file()) {
            return Ok(None);
        }
        let maintenance = shared(&maintenance_path)?;
        let project_dir = root.join(super::digest(project.as_bytes()));
        if !fs::symlink_metadata(&project_dir).is_ok_and(|metadata| metadata.is_dir()) {
            return Ok(None);
        }
        let directory = project_dir.join(super::digest(snapshot.as_bytes()));
        if !fs::symlink_metadata(&directory).is_ok_and(|metadata| metadata.is_dir()) {
            return Ok(None);
        }
        Ok(Some(Self {
            root,
            directory,
            _maintenance: maintenance,
            limits: Limits::default(),
            chunks: HashMap::new(),
            chunk_order: BTreeSet::new(),
            chunk_bytes: 0,
        }))
    }

    pub(super) fn open(
        storage: &Path,
        project: &str,
        snapshot: &str,
        budget: &Budget<'_>,
    ) -> Result<Self, String> {
        Self::with_limits(storage, project, snapshot, budget, Limits::default())
    }

    fn with_limits(
        storage: &Path,
        project: &str,
        snapshot: &str,
        budget: &Budget<'_>,
        limits: Limits,
    ) -> Result<Self, String> {
        budget.check()?;
        let root = storage.join("repository-context");
        directory(&root)?;
        let maintenance = lock_file(&root.join("maintenance.lock"))?;
        loop {
            budget.check()?;
            match maintenance.try_lock() {
                Ok(()) => break,
                Err(TryLockError::WouldBlock) => thread::sleep(Duration::from_millis(10)),
                Err(TryLockError::Error(_)) => return Err(UNAVAILABLE.into()),
            }
        }
        directory(&root.join("chunk-cache"))?;
        let project_dir = root.join(super::digest(project.as_bytes()));
        let scope_dir = project_dir.join(super::digest(snapshot.as_bytes()));
        let mut cache = Self {
            root,
            directory: scope_dir,
            _maintenance: maintenance,
            limits,
            chunks: HashMap::new(),
            chunk_order: BTreeSet::new(),
            chunk_bytes: 0,
        };
        cache.clean_interrupted(budget)?;
        cache.load_chunks(budget)?;
        cache.trim_chunks(0, 0, budget)?;
        let reserve_scope = usize::from(!cache.directory.exists());
        match cache.trim_scopes(reserve_scope, 0, budget) {
            Err(error) if error == FULL => {
                // Old readers may have released extra generations since the last
                // build. Reclaim those before refusing to open the current scope.
                let current =
                    crate::bounded_file::read(&cache.directory.join("current.json"), 4096)
                        .ok()
                        .and_then(|b| serde_json::from_slice::<super::Pointer>(&b).ok())
                        .filter(|p| uuid_name(&p.generation));
                if cache.directory.is_dir() {
                    cache.trim_generations(
                        current
                            .as_ref()
                            .map(|p| p.generation.as_str())
                            .unwrap_or(""),
                        None,
                        budget,
                    )?;
                }
                cache.trim_scopes(reserve_scope, 0, budget)?;
            }
            result => result?,
        }
        directory(&project_dir)?;
        directory(&cache.directory)?;
        cache.touch()?;
        Ok(cache)
    }

    fn scopes(&self, budget: &Budget<'_>) -> Result<Vec<PathBuf>, String> {
        let mut scopes = vec![];
        let mut remaining = SCAN_ENTRIES;
        for project in fs::read_dir(&self.root).map_err(|_| UNAVAILABLE.to_owned())? {
            budget.check()?;
            if remaining == 0 {
                return Err(UNAVAILABLE.into());
            }
            remaining -= 1;
            let project = project.map_err(|_| UNAVAILABLE.to_owned())?;
            if !sha_name(&project.file_name().to_string_lossy()) {
                continue;
            }
            if !project.file_type().is_ok_and(|kind| kind.is_dir()) {
                return Err(UNAVAILABLE.into());
            }
            for scope in fs::read_dir(project.path()).map_err(|_| UNAVAILABLE.to_owned())? {
                budget.check()?;
                if remaining == 0 {
                    return Err(UNAVAILABLE.into());
                }
                remaining -= 1;
                let scope = scope.map_err(|_| UNAVAILABLE.to_owned())?;
                if sha_name(&scope.file_name().to_string_lossy()) {
                    if !scope.file_type().is_ok_and(|kind| kind.is_dir()) {
                        return Err(UNAVAILABLE.into());
                    }
                    scopes.push(scope.path());
                }
            }
        }
        Ok(scopes)
    }

    fn clean_interrupted(&self, budget: &Budget<'_>) -> Result<(), String> {
        // Every new publisher owns maintenance.lock. A complete pointer is never removed here.
        for scope in self.scopes(budget)? {
            let mut remaining = SCAN_ENTRIES;
            for entry in fs::read_dir(scope).map_err(|_| UNAVAILABLE.to_owned())? {
                budget.check()?;
                if remaining == 0 {
                    return Err(UNAVAILABLE.into());
                }
                remaining -= 1;
                let entry = entry.map_err(|_| UNAVAILABLE.to_owned())?;
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.strip_prefix("staging-").is_some_and(uuid_name)
                    && entry.file_type().is_ok_and(|kind| kind.is_dir())
                {
                    fs::remove_dir_all(entry.path()).map_err(|_| UNAVAILABLE.to_owned())?;
                } else if name
                    .strip_prefix("pointer-")
                    .and_then(|n| n.strip_suffix(".tmp"))
                    .is_some_and(uuid_name)
                    && entry.file_type().is_ok_and(|kind| kind.is_file())
                {
                    fs::remove_file(entry.path()).map_err(|_| UNAVAILABLE.to_owned())?;
                }
            }
        }
        for (count, entry) in fs::read_dir(&self.root)
            .map_err(|_| UNAVAILABLE.to_owned())?
            .enumerate()
        {
            budget.check()?;
            if count >= SCAN_ENTRIES {
                return Err(UNAVAILABLE.into());
            }
            let entry = entry.map_err(|_| UNAVAILABLE.to_owned())?;
            if sha_name(&entry.file_name().to_string_lossy())
                && entry.file_type().is_ok_and(|t| t.is_dir())
            {
                let _ = fs::remove_dir(entry.path());
            }
        }
        Ok(())
    }

    fn load_chunks(&mut self, budget: &Budget<'_>) -> Result<(), String> {
        for (count, entry) in fs::read_dir(self.root.join("chunk-cache"))
            .map_err(|_| UNAVAILABLE.to_owned())?
            .enumerate()
        {
            budget.check()?;
            if count >= SCAN_ENTRIES {
                return Err(UNAVAILABLE.into());
            }
            let entry = entry.map_err(|_| UNAVAILABLE.to_owned())?;
            let metadata =
                fs::symlink_metadata(entry.path()).map_err(|_| UNAVAILABLE.to_owned())?;
            if !metadata.is_file() {
                return Err(UNAVAILABLE.into());
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.strip_suffix(".json").is_some_and(sha_name) {
                self.chunk_bytes = self.chunk_bytes.saturating_add(metadata.len());
                self.chunk_order.insert((modified(&metadata), entry.path()));
                self.chunks.insert(
                    entry.path(),
                    Entry {
                        bytes: metadata.len(),
                        used: modified(&metadata),
                    },
                );
            } else if name.strip_suffix(".tmp").is_some_and(|n| {
                n.get(..64).is_some_and(sha_name)
                    && n.get(64..65) == Some("-")
                    && n.get(65..).is_some_and(uuid_name)
            }) {
                fs::remove_file(entry.path()).map_err(|_| UNAVAILABLE.to_owned())?;
            }
        }
        Ok(())
    }

    fn trim_chunks(
        &mut self,
        bytes: u64,
        entries: usize,
        budget: &Budget<'_>,
    ) -> Result<(), String> {
        if bytes > self.limits.chunk_bytes || entries > self.limits.chunks {
            return Err(FULL.into());
        }
        while self.chunk_bytes.saturating_add(bytes) > self.limits.chunk_bytes
            || self.chunks.len().saturating_add(entries) > self.limits.chunks
        {
            budget.check()?;
            let Some((used, path)) = self.chunk_order.first().cloned() else {
                return Err(FULL.into());
            };
            fs::remove_file(&path).map_err(|_| UNAVAILABLE.to_owned())?;
            self.chunk_order.remove(&(used, path.clone()));
            if let Some(entry) = self.chunks.remove(&path) {
                self.chunk_bytes = self.chunk_bytes.saturating_sub(entry.bytes);
            }
        }
        Ok(())
    }

    fn trim_scopes(
        &self,
        reserve: usize,
        extra_bytes: u64,
        budget: &Budget<'_>,
    ) -> Result<(), String> {
        let paths = self.scopes(budget)?;
        let mut remaining = SCAN_ENTRIES;
        // Layout records have their own independent byte/entry cap. Separating
        // them keeps a failed build's optional layout writes within both quotas.
        let mut bytes = size(&self.root, 0, &mut remaining, budget)?
            .saturating_sub(self.chunk_bytes)
            .saturating_add(extra_bytes);
        let mut count = paths.len();
        let mut scopes = vec![];
        for path in paths {
            budget.check()?;
            if path == self.directory {
                continue;
            }
            let bytes = size(&path, 0, &mut remaining, budget)?;
            let used = fs::symlink_metadata(path.join("last-used"))
                .or_else(|_| fs::symlink_metadata(path.join("current.json")))
                .map(|m| modified(&m))
                .unwrap_or(SystemTime::UNIX_EPOCH);
            scopes.push(Scope { path, bytes, used });
        }
        scopes.sort_by(|a, b| a.used.cmp(&b.used).then_with(|| a.path.cmp(&b.path)));
        for scope in scopes {
            if bytes <= self.limits.bytes && count.saturating_add(reserve) <= self.limits.scopes {
                return Ok(());
            }
            budget.check()?;
            let Some(lease) = exclusive(&scope.path.join("readers.lock"))? else {
                continue;
            };
            // Maintenance prevents any new lease until removal finishes. Close the
            // exclusive handle first so Windows can also remove the lock file.
            drop(lease);
            fs::remove_dir_all(&scope.path).map_err(|_| UNAVAILABLE.to_owned())?;
            // Empty project directories have no user data and must not grow forever either.
            if let Some(parent) = scope.path.parent() {
                let _ = fs::remove_dir(parent);
            }
            bytes = bytes.saturating_sub(scope.bytes);
            count -= 1;
        }
        if bytes > self.limits.bytes || count.saturating_add(reserve) > self.limits.scopes {
            Err(FULL.into())
        } else {
            Ok(())
        }
    }

    pub(super) fn touch(&self) -> Result<(), String> {
        lock_file(&self.directory.join("last-used"))?
            .set_times(fs::FileTimes::new().set_modified(SystemTime::now()))
            .map_err(|_| UNAVAILABLE.to_owned())
    }

    pub(super) fn chunk_path(&self, key: &str) -> PathBuf {
        self.root.join("chunk-cache").join(format!("{key}.json"))
    }

    pub(super) fn chunk_used(&mut self, key: &str) {
        let path = self.chunk_path(key);
        if let Some(entry) = self.chunks.get_mut(&path) {
            self.chunk_order.remove(&(entry.used, path.clone()));
            entry.used = SystemTime::now();
            self.chunk_order.insert((entry.used, path.clone()));
            if let Ok(file) = lock_file(&path) {
                let _ = file.set_times(fs::FileTimes::new().set_modified(entry.used));
            }
        }
    }

    pub(super) fn save_chunk(
        &mut self,
        key: &str,
        encoded: &[u8],
        budget: &Budget<'_>,
    ) -> Result<(), String> {
        let path = self.chunk_path(key);
        if let Some(entry) = self.chunks.remove(&path) {
            fs::remove_file(&path).map_err(|_| UNAVAILABLE.to_owned())?;
            self.chunk_order.remove(&(entry.used, path.clone()));
            self.chunk_bytes = self.chunk_bytes.saturating_sub(entry.bytes);
        }
        self.trim_chunks(encoded.len() as u64, 1, budget)?;
        let temporary = self
            .root
            .join("chunk-cache")
            .join(format!("{key}-{}.tmp", Uuid::new_v4()));
        let result = super::write_new(&temporary, encoded)
            .and_then(|_| fs::rename(&temporary, &path).map_err(|_| UNAVAILABLE.to_owned()));
        let _ = fs::remove_file(temporary);
        result?;
        let used = SystemTime::now();
        self.chunk_order.insert((used, path.clone()));
        self.chunk_bytes = self.chunk_bytes.saturating_add(encoded.len() as u64);
        self.chunks.insert(
            path,
            Entry {
                bytes: encoded.len() as u64,
                used,
            },
        );
        Ok(())
    }

    pub(super) fn lease(&self, generation: &Path) -> Result<ReadLease, String> {
        let scope = generation.parent().ok_or(UNAVAILABLE)?;
        if scope.parent() != self.directory.parent()
            || !scope
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(sha_name)
            || !generation
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(uuid_name)
        {
            return Err(UNAVAILABLE.into());
        }
        if !fs::symlink_metadata(generation).is_ok_and(|m| m.is_dir()) {
            return Err(UNAVAILABLE.into());
        }
        Ok(ReadLease {
            _generation: shared(&generation.join("readers.lock"))?,
            _scope: shared(&scope.join("readers.lock"))?,
        })
    }

    /// Prefer the selected scope, then recent snapshots of this same project.
    /// A base is still admitted independently against its metadata and index.
    pub(super) fn candidate_scopes(&self, budget: &Budget<'_>) -> Result<Vec<PathBuf>, String> {
        let mut candidates = self
            .scopes(budget)?
            .into_iter()
            .filter(|p| p.parent() == self.directory.parent())
            .map(|p| {
                let used = fs::symlink_metadata(p.join("last-used"))
                    .map(|m| modified(&m))
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                (p, used)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|(a, at), (b, bt)| {
            (b == &self.directory)
                .cmp(&(a == &self.directory))
                .then_with(|| bt.cmp(at))
                .then_with(|| a.cmp(b))
        });
        Ok(candidates.into_iter().take(8).map(|(p, _)| p).collect())
    }

    pub(super) fn reserve_publication(
        &self,
        staging: &Path,
        pointer_bytes: usize,
        budget: &Budget<'_>,
    ) -> Result<(), String> {
        let mut remaining = SCAN_ENTRIES;
        if size(staging, 0, &mut remaining, budget)? > self.limits.generation_bytes {
            return Err(FULL.into());
        }
        let old_pointer_bytes = fs::symlink_metadata(self.directory.join("current.json"))
            .map(|m| m.len())
            .unwrap_or(0);
        self.trim_scopes(
            0,
            (pointer_bytes as u64).saturating_sub(old_pointer_bytes),
            budget,
        )
    }

    pub(super) fn trim_generations(
        &self,
        current: &str,
        previous: Option<&str>,
        budget: &Budget<'_>,
    ) -> Result<(), String> {
        for (count, entry) in fs::read_dir(&self.directory)
            .map_err(|_| UNAVAILABLE.to_owned())?
            .enumerate()
        {
            budget.check()?;
            if count >= SCAN_ENTRIES {
                return Err(UNAVAILABLE.into());
            }
            let entry = entry.map_err(|_| UNAVAILABLE.to_owned())?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name != current
                && previous != Some(name.as_str())
                && uuid_name(&name)
                && entry.file_type().is_ok_and(|kind| kind.is_dir())
            {
                let Some(lease) = exclusive(&entry.path().join("readers.lock"))? else {
                    continue;
                };
                drop(lease);
                fs::remove_dir_all(entry.path()).map_err(|_| UNAVAILABLE.to_owned())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
