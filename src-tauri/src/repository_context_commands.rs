//! Native ownership and cancellation for local context operations; no model connection required.

use crate::{ProjectSessions, app_data_directory, index, repository_context as context, snapshot};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, State};
use uuid::Uuid;

#[derive(Default)]
struct Jobs {
    work: Mutex<IndexCache>,
    requests: Mutex<HashMap<String, RequestEntry>>,
}

struct RequestEntry {
    cancelled: Arc<AtomicBool>,
    owned: bool,
    at: Instant,
}

fn make_room(requests: &mut HashMap<String, RequestEntry>) -> bool {
    requests.retain(|_, entry| entry.owned || entry.at.elapsed() < Duration::from_secs(120));
    if requests.len() >= 64 {
        let oldest = requests
            .iter()
            .filter(|(_, e)| !e.owned)
            .min_by_key(|(_, e)| e.at)
            .map(|(id, _)| id.clone());
        if let Some(id) = oldest {
            requests.remove(&id);
        }
    }
    requests.len() < 64
}

#[derive(Default)]
struct IndexCache {
    prepared: Option<context::PreparedIndex>,
    tag: Option<context::CacheTag>,
    checked: Option<Instant>,
}

#[derive(Clone, Default)]
pub(crate) struct RepositoryContextJobs(Arc<Jobs>);

pub(crate) struct Request {
    id: String,
    cancelled: Arc<AtomicBool>,
    jobs: RepositoryContextJobs,
}

impl Drop for Request {
    fn drop(&mut self) {
        if let Ok(mut requests) = self.jobs.0.requests.lock() {
            requests.remove(&self.id);
        }
    }
}

impl RepositoryContextJobs {
    pub(crate) fn begin(&self, id: &str) -> Result<Request, String> {
        self.begin_with_cancel(id, Arc::new(AtomicBool::new(false)))
    }

    pub(crate) fn begin_with_cancel(
        &self,
        id: &str,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Request, String> {
        if Uuid::parse_str(id).is_err() {
            return Err("A valid code-search request ID is required.".to_owned());
        }
        let mut requests = self
            .0
            .requests
            .lock()
            .map_err(|_| "Code search is temporarily unavailable.".to_owned())?;
        requests.retain(|_, entry| entry.owned || entry.at.elapsed() < Duration::from_secs(120));
        if let Some(entry) = requests.get(id) {
            return Err(if entry.cancelled.load(Ordering::Acquire) {
                "Code search stopped."
            } else {
                "This code search is already running."
            }
            .to_owned());
        }
        if requests.values().filter(|entry| entry.owned).count() >= 8 || !make_room(&mut requests) {
            return Err("Wait for a local code search to finish.".to_owned());
        }
        requests.insert(
            id.to_owned(),
            RequestEntry {
                cancelled: Arc::clone(&cancelled),
                owned: true,
                at: Instant::now(),
            },
        );
        Ok(Request {
            id: id.to_owned(),
            cancelled,
            jobs: self.clone(),
        })
    }

    pub(crate) fn cancel(&self, id: &str) -> Result<(), String> {
        if Uuid::parse_str(id).is_err() {
            return Err("A valid code-search request ID is required.".to_owned());
        }
        let mut requests = self
            .0
            .requests
            .lock()
            .map_err(|_| "Code search is temporarily unavailable.".to_owned())?;
        if let Some(entry) = requests.get_mut(id) {
            entry.cancelled.store(true, Ordering::Release);
            entry.at = Instant::now();
        } else if make_room(&mut requests) {
            // Cancellation can arrive before IPC has dispatched the original request.
            requests.insert(
                id.to_owned(),
                RequestEntry {
                    cancelled: Arc::new(AtomicBool::new(true)),
                    owned: false,
                    at: Instant::now(),
                },
            );
        }
        Ok(())
    }
}

impl Request {
    /// Serialize a short local publication with cancellation. Expensive source
    /// or Git work must finish before entering this commit boundary.
    pub(crate) fn commit<T>(&self, publish: impl FnOnce() -> T) -> Result<T, String> {
        let requests = self
            .jobs
            .0
            .requests
            .lock()
            .map_err(|_| "The local request owner is unavailable.".to_owned())?;
        if !requests.get(&self.id).is_some_and(|entry| {
            entry.owned
                && Arc::ptr_eq(&entry.cancelled, &self.cancelled)
                && !entry.cancelled.load(Ordering::Acquire)
        }) {
            return Err("The local request was stopped before publication.".into());
        }
        Ok(publish())
    }

    pub(crate) fn cancellation(&self) -> &AtomicBool {
        &self.cancelled
    }

    pub(crate) fn budget(&self) -> context::Budget<'_> {
        context::Budget::new(&self.cancelled)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SearchInput {
    pub(crate) request_id: String,
    pub(crate) project_id: String,
    pub(crate) snapshot_id: String,
    pub(crate) query: String,
    pub(crate) path: Option<String>,
    pub(crate) limit: usize,
}

pub(crate) fn search_snapshot(
    storage: &Path,
    input: &SearchInput,
    request: &Request,
) -> Result<context::SearchResult, String> {
    context::validate_search(&input.query, input.path.as_deref(), input.limit)?;
    let budget = context::Budget::new(&request.cancelled);
    let mut work = loop {
        budget.check()?;
        match request.jobs.0.work.try_lock() {
            Ok(guard) => break guard,
            Err(std::sync::TryLockError::WouldBlock) => {
                std::thread::sleep(Duration::from_millis(10))
            }
            Err(_) => return Err("Code search is temporarily unavailable.".to_owned()),
        }
    };
    let manifest =
        snapshot::load_snapshot_manifest(storage, &input.project_id, &input.snapshot_id)?;
    let tag = context::cache_tag(storage, &manifest)?;
    let reuse = work.tag.as_ref() == Some(&tag)
        && work
            .checked
            .is_some_and(|at| at.elapsed() < Duration::from_secs(60))
        && work.prepared.is_some();
    if !reuse {
        // An absent or damaged syntax index must not make captured text unavailable.
        let syntax = index::load_snapshot_index(storage, &input.project_id, &input.snapshot_id)
            .unwrap_or(None);
        work.prepared = Some(context::prepare(
            storage,
            &manifest,
            syntax.as_ref(),
            &budget,
        )?);
        let mut published_tag = context::cache_tag(storage, &manifest)?;
        // If parsing finishes during this build, the next search refreshes syntax.
        published_tag.retain_syntax_from(&tag);
        work.tag = Some(published_tag);
        work.checked = Some(Instant::now());
    } else if let Some(prepared) = &mut work.prepared {
        prepared.reused_generation = true;
    }
    let prepared = work
        .prepared
        .as_ref()
        .ok_or("Local code index is unavailable.")?;
    let result = context::search(
        storage,
        &manifest,
        prepared,
        &input.query,
        input.path.as_deref(),
        input.limit,
        &budget,
    );
    if result.is_err() {
        work.tag = None;
        work.prepared = None;
    }
    result
}

#[tauri::command]
pub(crate) async fn search_repository_code(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    jobs: State<'_, RepositoryContextJobs>,
    input: SearchInput,
) -> Result<context::SearchResult, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    let request = jobs.begin(&input.request_id)?;
    tauri::async_runtime::spawn_blocking(move || search_snapshot(&storage, &input, &request))
        .await
        .map_err(|_| "Code search stopped unexpectedly.".to_owned())?
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) fn cancel_repository_context(
    jobs: State<'_, RepositoryContextJobs>,
    request_id: String,
) -> Result<(), String> {
    jobs.cancel(&request_id)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReadInput {
    project_id: String,
    snapshot_id: String,
    path: String,
    source_sha256: String,
    start_byte: usize,
    end_byte: usize,
}

#[tauri::command]
pub(crate) async fn read_repository_range(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    input: ReadInput,
) -> Result<context::RangeRead, String> {
    sessions.get(&input.project_id)?;
    let storage = app_data_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let manifest =
            snapshot::load_snapshot_manifest(&storage, &input.project_id, &input.snapshot_id)?;
        context::read_range(
            &storage,
            &manifest,
            &input.path,
            &input.source_sha256,
            input.start_byte,
            input.end_byte,
        )
    })
    .await
    .map_err(|_| "The code range could not be read.".to_owned())?
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceSearchResult {
    matches: Vec<WorkspaceSearchHit>,
    limited: bool,
    unavailable_files: usize,
}

#[derive(Serialize)]
struct WorkspaceSearchHit {
    path: String,
    line: usize,
    text: String,
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn search_workspace_source(
    app: AppHandle,
    sessions: State<'_, ProjectSessions>,
    jobs: State<'_, RepositoryContextJobs>,
    project_id: String,
    snapshot_id: String,
    query: String,
    request_id: String,
) -> Result<WorkspaceSearchResult, String> {
    sessions.get(&project_id)?;
    let storage = app_data_directory(&app)?;
    let request = jobs.begin(&request_id)?;
    tauri::async_runtime::spawn_blocking(move || {
        let input = SearchInput {
            request_id,
            project_id,
            snapshot_id,
            query,
            path: None,
            limit: 32,
        };
        let result = search_snapshot(&storage, &input, &request)?;
        Ok(WorkspaceSearchResult {
            matches: result
                .matches
                .into_iter()
                .map(|hit| WorkspaceSearchHit {
                    path: hit.range.path,
                    line: hit.line,
                    text: hit.text,
                })
                .collect(),
            limited: result.limited,
            unavailable_files: result.coverage.files_unavailable,
        })
    })
    .await
    .map_err(|_| "Code search stopped unexpectedly.".to_owned())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancels_before_dispatch_and_reaps_completed_ownership() {
        let jobs = RepositoryContextJobs::default();
        let id = Uuid::new_v4().to_string();
        jobs.cancel(&id).unwrap();
        assert!(jobs.begin(&id).is_err());
        let id = Uuid::new_v4().to_string();
        let request = jobs.begin(&id).unwrap();
        assert!(jobs.begin(&id).is_err());
        jobs.cancel(&id).unwrap();
        assert!(request.cancelled.load(Ordering::Acquire));
        drop(request);
        assert!(!jobs.0.requests.lock().unwrap().contains_key(&id));
    }

    #[test]
    fn cancellation_tombstones_do_not_lock_out_new_searches_or_evict_live_jobs() {
        let jobs = RepositoryContextJobs::default();
        for _ in 0..100 {
            jobs.cancel(&Uuid::new_v4().to_string()).unwrap();
        }
        let requests = (0..8)
            .map(|_| jobs.begin(&Uuid::new_v4().to_string()).unwrap())
            .collect::<Vec<_>>();
        for request in &requests {
            jobs.cancel(&request.id).unwrap();
        }
        assert!(jobs.begin(&Uuid::new_v4().to_string()).is_err());
        for _ in 0..100 {
            jobs.cancel(&Uuid::new_v4().to_string()).unwrap();
        }
        assert!(
            requests
                .iter()
                .all(|r| jobs.0.requests.lock().unwrap().contains_key(&r.id))
        );
        assert!(jobs.0.requests.lock().unwrap().len() <= 64);
        drop(requests);
        assert!(jobs.begin(&Uuid::new_v4().to_string()).is_ok());
    }

    #[test]
    fn native_search_reuses_one_cached_generation_and_keeps_queries_scoped() {
        let fixture = crate::chapter_generation::tests::Fixture::new();
        let jobs = RepositoryContextJobs::default();
        for query in ["greeting", "name"] {
            let request = jobs.begin(&Uuid::new_v4().to_string()).unwrap();
            let input = SearchInput {
                request_id: request.id.clone(),
                project_id: fixture.manifest.summary.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                query: query.to_owned(),
                path: None,
                limit: 8,
            };
            let result = search_snapshot(&fixture.storage, &input, &request).unwrap();
            assert!(!result.matches.is_empty());
            if query == "name" {
                assert!(result.reused_generation);
            }
        }
        assert!(jobs.0.work.lock().unwrap().prepared.is_some());
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedRange {
        id: String,
        path: String,
        query: String,
        source_sha256: String,
        start_byte: usize,
        end_byte: usize,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Corpus {
        snapshot_id: String,
        expected: Vec<ExpectedRange>,
    }

    fn found(result: &context::SearchResult, expected: &ExpectedRange) -> bool {
        result.matches.iter().any(|h| {
            h.range.path == expected.path
                && h.range.source_sha256 == expected.source_sha256
                && h.range.start_byte <= expected.start_byte
                && h.range.end_byte >= expected.end_byte
        })
    }

    #[test]
    #[ignore = "Explicit Fontdone native-service retrieval measurement; no model call."]
    fn fontdone_service_retrieval_profile() {
        let storage = std::path::PathBuf::from(
            std::env::var("CODE_ATLAS_FONTDONE_DATA_DIR")
                .expect("set CODE_ATLAS_FONTDONE_DATA_DIR"),
        );
        let corpus: Corpus = serde_json::from_str(include_str!(
            "../../docs/acceptance/fixtures/fontdone-retrieval-ranges.json"
        ))
        .unwrap();
        let jobs = RepositoryContextJobs::default();
        let run = |query: &str, path: Option<&str>| {
            let request = jobs.begin(&Uuid::new_v4().to_string()).unwrap();
            let input = SearchInput {
                request_id: request.id.clone(),
                project_id: "/Users/lazytrot/work/fontdone".to_owned(),
                snapshot_id: corpus.snapshot_id.clone(),
                query: query.to_owned(),
                path: path.map(str::to_owned),
                limit: 32,
            };
            let started = Instant::now();
            let result = search_snapshot(&storage, &input, &request).unwrap();
            (result, started.elapsed().as_millis())
        };
        let (first, first_ms) = run("memory_face", None);
        let (warm, warm_ms) = run("memory_face", None);
        assert!(
            first
                .matches
                .iter()
                .any(|h| h.declaration_match && h.range.path == "src/font.rs")
        );
        assert!(warm.reused_generation);
        println!(
            "FONTDONE_SERVICE {}",
            serde_json::json!({"diskLoadAndSearchMs":first_ms,"memoryCachedSearchMs":warm_ms,"coverage":warm.coverage,"serializedMatchBytes":warm.result_bytes,"topPath":warm.matches[0].range.path,"topLine":warm.matches[0].line})
        );
        let mut named = 0;
        for expected in &corpus.expected {
            let (global, global_ms) = run(&expected.query, None);
            let (scoped, scoped_ms) = run(&expected.query, Some(&expected.path));
            named += usize::from(found(&global, expected));
            println!(
                "FONTDONE_NAMED {}",
                serde_json::json!({"id":expected.id,"globalFound":found(&global,expected),"scopedFound":found(&scoped,expected),"globalMs":global_ms,"scopedMs":scoped_ms,"globalMatchBytes":global.result_bytes,"scopedMatchBytes":scoped.result_bytes})
            );
            assert!(
                found(&scoped, expected),
                "missing independently selected declaration {}",
                expected.id
            );
        }
        for query in [
            "font bytes rendered glyph bitmap entry parsing outline hinting rasterization",
            "font buffer glyph image format outline hinting pixel coverage",
        ] {
            let (result, ms) = run(query, None);
            let recovered = corpus
                .expected
                .iter()
                .filter(|e| found(&result, e))
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>();
            println!(
                "FONTDONE_BROAD {}",
                serde_json::json!({"query":query,"elapsedMs":ms,"retrieved":recovered,"expectedCount":corpus.expected.len(),"serializedMatchBytes":result.result_bytes,"limited":result.limited,"namedGlobalRecall":format!("{named}/{}",corpus.expected.len())})
            );
        }
    }
}
