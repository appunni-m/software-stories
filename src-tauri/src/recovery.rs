use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::future::Future;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const JOURNAL_SCHEMA_VERSION: u32 = 1;
const MAX_JOURNAL_BYTES: u64 = 1024 * 1024;
const MAX_OPERATION_COUNT: usize = 256;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InterruptedOperationKind {
    Capture,
    ProviderGeneration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InterruptedOperation {
    pub operation_id: String,
    pub kind: InterruptedOperationKind,
    pub project_id: String,
    pub snapshot_id: String,
    pub target_path: Option<String>,
    pub started_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_request: Option<InterruptedProviderRequest>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterruptedProviderRequest {
    pub provider_id: String,
    pub purpose: String,
    pub selected_paths: Vec<String>,
    pub section_update: Option<crate::provider::SectionUpdateReviewTarget>,
}

#[derive(Clone, Default)]
pub struct InterruptionJournal(Arc<Mutex<()>>);

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct JournalFile {
    schema_version: u32,
    operations: Vec<InterruptedOperation>,
}

impl InterruptionJournal {
    pub fn list(&self, storage_dir: &Path) -> Result<Vec<InterruptedOperation>, String> {
        let _guard = self.lock()?;
        read_journal(storage_dir).map(|journal| journal.operations)
    }

    pub fn record(
        &self,
        storage_dir: &Path,
        operation: InterruptedOperation,
    ) -> Result<(), String> {
        validate_operation(&operation)?;
        let _guard = self.lock()?;
        let mut journal = read_journal(storage_dir)?;
        journal
            .operations
            .retain(|saved| saved.operation_id != operation.operation_id);
        if journal.operations.len() >= MAX_OPERATION_COUNT {
            return Err("Too many interrupted operations are saved locally. Discard an older notice before starting more work.".to_owned());
        }
        journal.operations.push(operation);
        write_journal(storage_dir, &journal)
    }

    pub fn begin<T>(
        &self,
        storage_dir: &Path,
        operation: InterruptedOperation,
        start: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        let operation_id = operation.operation_id.clone();
        self.record(storage_dir, operation)?;
        match start() {
            Ok(value) => Ok(value),
            Err(error) => match self.discard(storage_dir, &operation_id) {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(format!(
                    "{error} Recovery state could not be cleared: {cleanup_error}"
                )),
            },
        }
    }

    pub async fn begin_async<T, F, Fut>(
        &self,
        storage_dir: &Path,
        operation: InterruptedOperation,
        start: F,
    ) -> Result<T, String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        self.record(storage_dir, operation)?;
        Ok(start().await)
    }

    pub fn discard(&self, storage_dir: &Path, operation_id: &str) -> Result<(), String> {
        if operation_id.is_empty() || operation_id.len() > 256 {
            return Err("The interrupted operation ID is invalid.".to_owned());
        }
        let _guard = self.lock()?;
        let mut journal = read_journal(storage_dir)?;
        journal
            .operations
            .retain(|operation| operation.operation_id != operation_id);
        if journal.operations.is_empty() {
            match fs::remove_file(journal_path(storage_dir)) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(format!(
                    "The interruption notice could not be cleared: {error}"
                )),
            }
        } else {
            write_journal(storage_dir, &journal)
        }
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, String> {
        self.0
            .lock()
            .map_err(|_| "Local recovery state is temporarily unavailable.".to_owned())
    }
}

pub fn new_capture_operation(project_id: String, snapshot_id: String) -> InterruptedOperation {
    let mut operation = operation(
        InterruptedOperationKind::Capture,
        project_id,
        snapshot_id.clone(),
        None,
    );
    operation.operation_id = snapshot_id;
    operation
}

pub fn new_provider_operation(
    project_id: String,
    snapshot_id: String,
    target_path: Option<String>,
    operation_id: String,
) -> InterruptedOperation {
    let mut operation = operation(
        InterruptedOperationKind::ProviderGeneration,
        project_id,
        snapshot_id,
        target_path,
    );
    operation.operation_id = operation_id;
    operation
}

/// Retain request scope, never prompts, source bytes, credentials or consent.
pub fn new_reviewed_provider_operation(
    review: &crate::provider::ProviderRequestReview,
) -> InterruptedOperation {
    let mut operation = new_provider_operation(
        review.project_id.clone(),
        review.snapshot_id.clone(),
        review.target_path.clone(),
        review.review_id.clone(),
    );
    operation.provider_request = Some(InterruptedProviderRequest {
        provider_id: review.provider_id.clone(),
        purpose: review.purpose.clone(),
        selected_paths: review
            .selected_files
            .iter()
            .map(|file| file.path.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect(),
        section_update: review.section_update_context.clone(),
    });
    operation
}

fn operation(
    kind: InterruptedOperationKind,
    project_id: String,
    snapshot_id: String,
    target_path: Option<String>,
) -> InterruptedOperation {
    let started_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default();
    InterruptedOperation {
        operation_id: Uuid::new_v4().to_string(),
        kind,
        project_id,
        snapshot_id,
        target_path,
        started_at_unix_ms,
        provider_request: None,
    }
}

fn validate_operation(operation: &InterruptedOperation) -> Result<(), String> {
    let valid_text = |value: &str, max_bytes: usize| {
        !value.is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
    };
    if !valid_text(&operation.operation_id, 256)
        || !valid_text(&operation.project_id, 4096)
        || !valid_text(&operation.snapshot_id, 256)
        || operation
            .target_path
            .as_deref()
            .is_some_and(|path| !valid_text(path, 4096))
    {
        return Err("The interrupted operation details are invalid.".to_owned());
    }
    if matches!(&operation.kind, InterruptedOperationKind::Capture)
        && (operation.target_path.is_some() || operation.provider_request.is_some())
    {
        return Err("A capture recovery record cannot include a provider target.".to_owned());
    }
    if let Some(request) = &operation.provider_request {
        if !matches!(
            request.provider_id.as_str(),
            "codex" | "claude-agent" | "gemini" | "openai" | "anthropic"
        ) || !matches!(
            request.purpose.as_str(),
            "fileChapter"
                | "repositoryOverview"
                | "sectionUpdate"
                | "codeLensExplanation"
                | "prGuide"
                | "prChangeExplanation"
        ) || request.selected_paths.len() > 24
            || request
                .selected_paths
                .iter()
                .any(|path| !valid_text(path, 4096))
            || request
                .selected_paths
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != request.selected_paths.len()
            || (request.purpose == "sectionUpdate") != request.section_update.is_some()
        {
            return Err("The interrupted provider request scope is invalid.".to_owned());
        }
        if matches!(
            request.purpose.as_str(),
            "fileChapter" | "repositoryOverview" | "sectionUpdate"
        ) {
            if request.selected_paths.is_empty()
                || request.selected_paths.len() > 12
                || (request.purpose == "repositoryOverview") != operation.target_path.is_none()
                || operation
                    .target_path
                    .as_ref()
                    .is_some_and(|path| !request.selected_paths.contains(path))
            {
                return Err("The interrupted chapter's source scope is invalid.".to_owned());
            }
        }
        if request.section_update.as_ref().is_some_and(|target| {
            !valid_text(&target.chapter_revision_id, 256)
                || !valid_text(&target.section_id, 256)
                || !valid_text(&target.section_revision_id, 256)
        }) {
            return Err("The interrupted section identity is invalid.".to_owned());
        }
    }
    Ok(())
}

fn journal_path(storage_dir: &Path) -> PathBuf {
    storage_dir.join("interrupted-operations.json")
}

fn read_journal(storage_dir: &Path) -> Result<JournalFile, String> {
    let path = journal_path(storage_dir);
    let file = match File::open(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(empty_journal());
        }
        Err(error) => return Err(format!("Interrupted work could not be read: {error}")),
    };
    let mut bytes = Vec::new();
    file.take(MAX_JOURNAL_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("Interrupted work could not be read: {error}"))?;
    if bytes.len() as u64 > MAX_JOURNAL_BYTES {
        return Err("Interrupted work data is larger than the local recovery limit.".to_owned());
    }
    let journal: JournalFile = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Interrupted work data is invalid: {error}"))?;
    if journal.schema_version != JOURNAL_SCHEMA_VERSION
        || journal.operations.len() > MAX_OPERATION_COUNT
    {
        return Err(
            "Interrupted work uses an unsupported or oversized recovery format.".to_owned(),
        );
    }
    for operation in &journal.operations {
        validate_operation(operation)?;
    }
    Ok(journal)
}

fn empty_journal() -> JournalFile {
    JournalFile {
        schema_version: JOURNAL_SCHEMA_VERSION,
        operations: Vec::new(),
    }
}

fn write_journal(storage_dir: &Path, journal: &JournalFile) -> Result<(), String> {
    fs::create_dir_all(storage_dir)
        .map_err(|error| format!("The local recovery folder could not be created: {error}"))?;
    let path = journal_path(storage_dir);
    let temporary_path =
        storage_dir.join(format!(".interrupted-operations.{}.tmp", Uuid::new_v4()));
    let bytes = serde_json::to_vec_pretty(journal)
        .map_err(|error| format!("Interrupted work could not be encoded: {error}"))?;
    if bytes.len() as u64 > MAX_JOURNAL_BYTES {
        return Err("Interrupted work would exceed the local recovery storage limit.".to_owned());
    }

    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary_path)
            .map_err(|error| format!("Interrupted work could not be saved: {error}"))?;
        file.write_all(&bytes)
            .map_err(|error| format!("Interrupted work could not be saved: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Interrupted work could not be synced: {error}"))?;
        fs::rename(&temporary_path, &path)
            .map_err(|error| format!("Interrupted work could not be finalized: {error}"))
    })();

    if result.is_err() {
        let _ = fs::remove_file(temporary_path);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("code-atlas-recovery-{}", Uuid::new_v4()));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn interrupted_operation_survives_reload_and_discard_only_removes_it() {
        let directory = TestDirectory::new();
        let journal = InterruptionJournal::default();
        let capture = new_capture_operation("/repo/a".to_owned(), "snap-a".to_owned());
        let provider = new_provider_operation(
            "/repo/b".to_owned(),
            "snap-b".to_owned(),
            Some("src/lib.rs".to_owned()),
            "review-b".to_owned(),
        );
        journal.record(&directory.0, capture.clone()).unwrap();
        journal.record(&directory.0, provider.clone()).unwrap();

        let reloaded = InterruptionJournal::default().list(&directory.0).unwrap();
        assert_eq!(reloaded, vec![capture.clone(), provider.clone()]);

        journal
            .discard(&directory.0, &capture.operation_id)
            .unwrap();
        assert_eq!(journal.list(&directory.0).unwrap(), vec![provider]);
    }

    #[test]
    fn unsupported_or_malformed_recovery_data_is_not_silently_overwritten() {
        let directory = TestDirectory::new();
        fs::write(
            journal_path(&directory.0),
            br#"{"schemaVersion":9,"operations":[]}"#,
        )
        .unwrap();
        let journal = InterruptionJournal::default();
        assert!(journal.list(&directory.0).is_err());
        assert!(
            journal
                .record(
                    &directory.0,
                    new_capture_operation("/repo/a".to_owned(), "snap-a".to_owned()),
                )
                .is_err()
        );
        assert_eq!(
            fs::read(journal_path(&directory.0)).unwrap(),
            br#"{"schemaVersion":9,"operations":[]}"#
        );
    }

    #[test]
    fn oversized_update_preserves_the_previous_recovery_journal() {
        let directory = TestDirectory::new();
        let journal = InterruptionJournal::default();
        let original = new_capture_operation("/repo/kept".to_owned(), "snap-kept".to_owned());
        journal.record(&directory.0, original.clone()).unwrap();

        let oversized_project_id = "p".repeat(4096);
        let oversized_target_path = "f".repeat(4096);
        let mut overflowed = false;
        for _ in 0..MAX_OPERATION_COUNT {
            let operation = new_provider_operation(
                oversized_project_id.clone(),
                "snap-overflow".to_owned(),
                Some(oversized_target_path.clone()),
                Uuid::new_v4().to_string(),
            );
            if journal.record(&directory.0, operation).is_err() {
                overflowed = true;
                break;
            }
        }

        assert!(
            overflowed,
            "journal byte cap should reject the oversized update"
        );
        let retained = journal.list(&directory.0).unwrap();
        assert_eq!(retained.first(), Some(&original));
        assert!(retained.len() < MAX_OPERATION_COUNT);
    }

    #[test]
    fn capture_write_failure_does_not_start_capture_work() {
        let directory = TestDirectory::new();
        let invalid_storage = directory.0.join("storage-is-a-file");
        fs::write(&invalid_storage, b"file").unwrap();
        let started = std::cell::Cell::new(false);
        let result = InterruptionJournal::default().begin(
            &invalid_storage,
            new_capture_operation("/repo/a".to_owned(), "snap-a".to_owned()),
            || {
                started.set(true);
                Ok(())
            },
        );
        assert!(result.is_err());
        assert!(!started.get());
    }

    #[tokio::test]
    async fn provider_write_failure_does_not_start_provider_work() {
        let directory = TestDirectory::new();
        let invalid_storage = directory.0.join("storage-is-a-file");
        fs::write(&invalid_storage, b"file").unwrap();
        let started = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let started_by_request = std::sync::Arc::clone(&started);
        let result = InterruptionJournal::default()
            .begin_async(
                &invalid_storage,
                new_provider_operation(
                    "/repo/a".to_owned(),
                    "snap-a".to_owned(),
                    Some("src/lib.rs".to_owned()),
                    "review-a".to_owned(),
                ),
                || async move {
                    started_by_request.store(true, std::sync::atomic::Ordering::SeqCst);
                },
            )
            .await;
        assert!(result.is_err());
        assert!(!started.load(std::sync::atomic::Ordering::SeqCst));
    }

    #[tokio::test]
    async fn provider_operation_is_durable_before_send_and_survives_reopen_until_cleared() {
        let directory = TestDirectory::new();
        let journal = InterruptionJournal::default();
        let operation = new_provider_operation(
            "project-a".to_owned(),
            "snapshot-a".to_owned(),
            Some("src/lib.rs".to_owned()),
            "review-a".to_owned(),
        );
        let operation_id = operation.operation_id.clone();
        let storage = directory.0.clone();
        let operation_seen_before_send = operation.clone();

        journal
            .begin_async(&directory.0, operation.clone(), || async move {
                let saved_before_send = InterruptionJournal::default().list(&storage).unwrap();
                assert_eq!(saved_before_send, vec![operation_seen_before_send]);
                "provider completed"
            })
            .await
            .unwrap();

        let serialized = fs::read_to_string(journal_path(&directory.0)).unwrap();
        assert!(!serialized.contains("consent"));
        assert!(!serialized.contains("apiKey"));
        assert!(!serialized.contains("userPrompt"));
        assert_eq!(
            InterruptionJournal::default().list(&directory.0).unwrap(),
            vec![operation]
        );

        journal.discard(&directory.0, &operation_id).unwrap();
        assert!(
            InterruptionJournal::default()
                .list(&directory.0)
                .unwrap()
                .is_empty()
        );
    }
}
