use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

const JOURNAL_SCHEMA_VERSION: u32 = 2;
const MAX_JOURNAL_BYTES: usize = 1024 * 1024;
const MAX_OPERATION_COUNT: usize = 256;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InterruptedOperationKind {
    Capture,
    ProviderGeneration,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_lens: Option<crate::provider::CodeLensExplanationTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git: Option<InterruptedGitScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) repository_plan: Option<crate::repository_plan::RepositoryPlanReviewScope>,
    #[serde(
        default,
        deserialize_with = "read_planned_chapter_scope",
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) planned_chapter: Option<crate::planned_chapter::PlannedChapterScope>,
}

// An ordinary request omits planned scope. A present null/invalid scope is a
// damaged planned request, never permission to reopen a generic file chapter.
fn read_planned_chapter_scope<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<crate::planned_chapter::PlannedChapterScope>, D::Error> {
    crate::planned_chapter::PlannedChapterScope::deserialize(deserializer).map(Some)
}

/// Immutable Git selection only: never patches, source, settings or consent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InterruptedGitScope {
    pub base_ref: String,
    pub head_ref: String,
    /// Binds the requested refs and their resolved commit object identities.
    pub comparison_id: String,
    pub selected_changes: Vec<crate::pr_guide::SelectedChangeIdentity>,
    pub hunk_index: Option<usize>,
    pub patch_sha256: Vec<String>,
}

#[derive(Clone, Default)]
pub struct InterruptionJournal(Arc<Mutex<()>>);

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
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
        save_remaining_operations(storage_dir, &journal)
    }

    /// Resolve only the native entry that was independently matched to a
    /// reviewed saved result. A concurrent scope change cannot be erased.
    pub(crate) fn discard_if_unchanged(
        &self,
        storage_dir: &Path,
        expected: &InterruptedOperation,
    ) -> Result<bool, String> {
        validate_operation(expected)?;
        let _guard = self.lock()?;
        let mut journal = read_journal(storage_dir)?;
        let Some(current) = journal
            .operations
            .iter()
            .find(|operation| operation.operation_id == expected.operation_id)
        else {
            return Ok(false);
        };
        if current != expected {
            return Err("The recovery notice changed during review. It was preserved; refresh local recovery before resolving it.".to_owned());
        }
        journal
            .operations
            .retain(|operation| operation.operation_id != expected.operation_id);
        save_remaining_operations(storage_dir, &journal)?;
        Ok(true)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, ()>, String> {
        self.0
            .lock()
            .map_err(|_| "Local recovery state is temporarily unavailable.".to_owned())
    }
}

fn save_remaining_operations(storage_dir: &Path, journal: &JournalFile) -> Result<(), String> {
    if journal.operations.is_empty() {
        match fs::remove_file(journal_path(storage_dir)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "The interruption notice could not be cleared: {error}"
            )),
        }
    } else {
        write_journal(storage_dir, journal)
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

/// Retain request scope, including a plan's learning question. Never retain
/// assembled prompts, source bytes, credentials, agent settings or consent.
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
        code_lens: review.code_lens_context.clone(),
        planned_chapter: review
            .generation_config
            .get("plannedChapter")
            .and_then(|_| crate::provider::planned_chapter_review_scope(review).ok()),
        // An invalid or missing native planning scope cannot be journaled:
        // validate_operation below requires it exactly for repositoryPlan.
        repository_plan: (review.purpose == "repositoryPlan")
            .then(|| {
                serde_json::from_value(review.generation_config["repositoryPlan"].clone()).ok()
            })
            .flatten(),
        git: review
            .pr_guide_context
            .as_ref()
            .map(|context| InterruptedGitScope {
                base_ref: context.base_ref.clone(),
                head_ref: context.head_ref.clone(),
                comparison_id: context.comparison_id.clone(),
                selected_changes: context
                    .selected_changes
                    .iter()
                    .map(|change| crate::pr_guide::SelectedChangeIdentity {
                        base_path: change.base_path.clone(),
                        head_path: change.head_path.clone(),
                    })
                    .collect(),
                hunk_index: None,
                patch_sha256: review
                    .selected_files
                    .iter()
                    .map(|file| file.sha256.clone())
                    .collect(),
            })
            .or_else(|| {
                review
                    .pr_change_context
                    .as_ref()
                    .map(|target| InterruptedGitScope {
                        base_ref: target.base_ref.clone(),
                        head_ref: target.head_ref.clone(),
                        comparison_id: target.comparison_id.clone(),
                        selected_changes: vec![target.selected_change.clone()],
                        hunk_index: Some(target.hunk_index),
                        patch_sha256: review
                            .selected_files
                            .iter()
                            .map(|file| file.sha256.clone())
                            .collect(),
                    })
            }),
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
                | "repositoryPlan"
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
            || (request.purpose == "repositoryPlan") != request.repository_plan.is_some()
        {
            return Err("The interrupted provider request scope is invalid.".to_owned());
        }
        if let Some(scope) = &request.planned_chapter {
            if !matches!(request.provider_id.as_str(), "codex" | "claude-agent")
                || request.purpose != "fileChapter"
                || !scope.valid_recorded_shape()
                || request.repository_plan.is_some()
                || request.section_update.is_some()
                || request.code_lens.is_some()
                || request.git.is_some()
                || operation.snapshot_id != scope.snapshot_id
                || operation.target_path.as_deref() != Some(scope.primary_path.as_str())
                || request
                    .selected_paths
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    != scope
                        .selected_paths
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
            {
                return Err("The interrupted accepted-step chapter scope is invalid.".to_owned());
            }
        }
        if let Some(scope) = &request.repository_plan {
            if !matches!(request.provider_id.as_str(), "codex" | "claude-agent")
                || operation.target_path.is_some()
                || request.section_update.is_some()
                || request.code_lens.is_some()
                || request.git.is_some()
                || !scope.valid_recovery_shape(request.selected_paths.len())
            {
                return Err("The interrupted repository planning scope is invalid.".to_owned());
            }
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
        if let Some(target) = &request.code_lens {
            if request.purpose != "codeLensExplanation"
                || request.git.is_some()
                || operation.target_path.as_deref() != Some(target.path.as_str())
                || request.selected_paths != [target.path.clone()]
                || !valid_text(&target.path, 4096)
                || !valid_sha256(&target.source_sha256)
                || target.start_line == 0
                || target.end_line < target.start_line
                || match (target.start_byte, target.end_byte_exclusive) {
                    (None, None) => false,
                    (Some(start), Some(end)) => end <= start || end > (1_u64 << 53) - 1,
                    _ => true,
                }
            {
                return Err("The interrupted source range is invalid.".to_owned());
            }
        }
        if let Some(git) = &request.git {
            let is_hunk = request.purpose == "prChangeExplanation";
            if !matches!(request.purpose.as_str(), "prGuide" | "prChangeExplanation")
                || request.code_lens.is_some()
                || git.selected_changes.is_empty()
                || git.selected_changes.len() > 12
                || !valid_text(&git.base_ref, 4096)
                || !valid_text(&git.head_ref, 4096)
                || git
                    .comparison_id
                    .strip_prefix("cmp:")
                    .is_none_or(|hash| !valid_sha256(hash))
                || operation.snapshot_id != git.comparison_id
                || (request.purpose == "prGuide" && operation.target_path.is_some())
                || (is_hunk && (operation.target_path.is_none() || git.selected_changes.len() != 1))
                || is_hunk != git.hunk_index.is_some()
                || git.patch_sha256.len() != git.selected_changes.len()
                || git.hunk_index.is_some_and(|index| index > 1_000_000)
                || git.patch_sha256.iter().any(|hash| !valid_sha256(hash))
                || git.selected_changes.iter().any(|change| {
                    (change.base_path.is_none() && change.head_path.is_none())
                        || change
                            .base_path
                            .iter()
                            .chain(change.head_path.iter())
                            .any(|path| !valid_text(path, 4096))
                })
                || git
                    .selected_changes
                    .iter()
                    .enumerate()
                    .any(|(index, change)| git.selected_changes[..index].contains(change))
            {
                return Err("The interrupted Git selection is invalid.".to_owned());
            }
        }
    }
    Ok(())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn journal_path(storage_dir: &Path) -> PathBuf {
    storage_dir.join("interrupted-operations.json")
}

fn read_journal(storage_dir: &Path) -> Result<JournalFile, String> {
    let path = journal_path(storage_dir);
    let bytes = match crate::bounded_file::read(&path, MAX_JOURNAL_BYTES) {
        Ok(bytes) => bytes,
        Err(crate::bounded_file::ReadError::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound =>
        {
            return Ok(empty_journal());
        }
        Err(crate::bounded_file::ReadError::Io(error)) => {
            return Err(format!("Interrupted work could not be read: {error}"));
        }
        Err(crate::bounded_file::ReadError::TooLarge) => {
            return Err(
                "Interrupted work data is larger than the local recovery limit.".to_owned(),
            );
        }
    };
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
    if bytes.len() > MAX_JOURNAL_BYTES {
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
    fn recovery_journal_accepts_one_mib_and_rejects_the_next_byte() {
        let directory = TestDirectory::new();
        let mut bytes = serde_json::to_vec(&empty_journal()).unwrap();
        bytes.resize(MAX_JOURNAL_BYTES, b' ');
        fs::write(journal_path(&directory.0), &bytes).unwrap();
        let loaded = read_journal(&directory.0).unwrap();
        assert_eq!(loaded.schema_version, JOURNAL_SCHEMA_VERSION);
        assert!(loaded.operations.is_empty());

        bytes.push(b' ');
        fs::write(journal_path(&directory.0), bytes).unwrap();
        assert!(matches!(
            read_journal(&directory.0),
            Err(error) if error.contains("larger than the local recovery limit")
        ));

        let mut truncated = serde_json::to_vec(&empty_journal()).unwrap();
        truncated.pop();
        for invalid in [Vec::new(), truncated, b"{ broken".to_vec()] {
            fs::write(journal_path(&directory.0), &invalid).unwrap();
            assert!(matches!(read_journal(&directory.0), Err(error) if error.contains("invalid")));
            assert_eq!(fs::read(journal_path(&directory.0)).unwrap(), invalid);
        }
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
