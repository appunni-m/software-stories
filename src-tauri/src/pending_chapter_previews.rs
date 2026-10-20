use crate::chapters::{
    self, ChapterDraftPreview, ChapterTargetScope, RevisionLifecycle, ValidationReportStatus,
    ValidationStatus,
};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use uuid::Uuid;

const PREVIEW_SCHEMA_VERSION: u32 = 1;
const MAX_PREVIEW_COUNT: usize = 16;
const MAX_PREVIEW_BYTES: usize = 2 * 1024 * 1024;
const MAX_TOTAL_PREVIEW_BYTES: usize = 16 * 1024 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 128;
const MAX_MODEL_METADATA_BYTES: usize = 512;

static PREVIEW_STORE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PendingChapterPreviewSummary {
    pub generation_id: String,
    pub project_id: Option<String>,
    pub snapshot_id: Option<String>,
    pub title: String,
    pub model_id: Option<String>,
    pub target_path: Option<String>,
    pub unavailable_reason: Option<String>,
    pub generation_origin: Option<crate::chapter_origin::ChapterGenerationOrigin>,
    pub origin_content_matches: Option<bool>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewFile {
    schema_version: u32,
    preview: ChapterDraftPreview,
}

pub fn save(storage_dir: &Path, preview: &ChapterDraftPreview) -> Result<(), String> {
    let _guard = lock_store()?;
    save_locked(storage_dir, preview)
}

/// Called only from an admitted agent's final publication callback.
pub(crate) fn try_save(storage_dir: &Path, preview: &ChapterDraftPreview) -> Result<(), String> {
    let _guard = lock_store()?;
    save_locked(storage_dir, preview)
}

fn save_locked(storage_dir: &Path, preview: &ChapterDraftPreview) -> Result<(), String> {
    validate_preview(preview)?;
    let bytes = serde_json::to_vec(&PreviewFile {
        schema_version: PREVIEW_SCHEMA_VERSION,
        preview: preview.clone(),
    })
    .map_err(|_| "The generated preview could not be serialized for local recovery.".to_owned())?;
    if bytes.len() > MAX_PREVIEW_BYTES {
        return Err("The generated preview exceeds its 2 MB local recovery limit.".to_owned());
    }

    let directory = preview_directory(storage_dir)?;
    let existing = read_all(&directory)?;
    // An unsupported or malformed earlier preview remains available for its
    // recovery notice, but it must not prevent saving an unrelated new draft.
    // `read_all` still enforces regular files, canonical names and bounded
    // storage before capacity is calculated; the new preview is validated
    // above before publication.
    if existing.len() >= MAX_PREVIEW_COUNT {
        return Err("Too many generated previews are saved locally. Save or discard one before generating another.".to_owned());
    }
    let existing_bytes = existing.iter().try_fold(0usize, |sum, entry| {
        sum.checked_add(entry.bytes).ok_or_else(|| {
            "The generated preview recovery folder exceeds its supported size.".to_owned()
        })
    })?;
    if existing_bytes.saturating_add(bytes.len()) > MAX_TOTAL_PREVIEW_BYTES {
        return Err("Generated previews exceed the 16 MB local recovery limit. Save or discard one before generating another.".to_owned());
    }

    let (uuid, _) = parse_generation_id(&preview.generation_id)?;
    let final_path = directory.join(format!("{uuid}.json"));
    match fs::symlink_metadata(&final_path) {
        Ok(_) => return Err("This generated preview already exists in local recovery.".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err("The generated preview destination could not be inspected.".to_owned());
        }
    }

    let temporary_path = directory.join(format!(".preview-{}.tmp", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary_path)
        .map_err(|_| "The generated preview could not be staged for local recovery.".to_owned())?;
    let staged = StagedPreview(temporary_path);
    use std::io::Write;
    file.write_all(&bytes)
        .map_err(|_| "The generated preview could not be written for local recovery.".to_owned())?;
    file.sync_all()
        .map_err(|_| "The generated preview could not be synced for local recovery.".to_owned())?;
    // A hard link publishes the fully synced record without replacing an existing entry.
    fs::hard_link(&staged.0, &final_path).map_err(|_| {
        "The generated preview could not be added without replacing local recovery data.".to_owned()
    })?;
    drop(file);
    drop(staged);
    Ok(())
}

pub fn load(storage_dir: &Path, generation_id: &str) -> Result<ChapterDraftPreview, String> {
    let _guard = lock_store()?;
    let path = record_path(storage_dir, generation_id)?;
    let preview = read_preview_file(&path, generation_id)?;
    Ok(preview)
}

pub fn list(storage_dir: &Path) -> Result<Vec<PendingChapterPreviewSummary>, String> {
    let _guard = lock_store()?;
    let directory = preview_directory(storage_dir)?;
    let entries = read_all(&directory)?;
    let mut summaries = entries
        .into_iter()
        .map(
            |entry| match decode_preview(&entry.bytes_vec, &entry.path) {
                Ok(preview) => {
                    let revision = preview.revision;
                    let origin_content_matches = revision
                        .generation_origin
                        .as_ref()
                        .map(|origin| origin.matches_content(&revision));
                    let target_path = match revision.target.scope {
                        ChapterTargetScope::File { path } => Some(path),
                        ChapterTargetScope::Group { paths } => paths.first().cloned(),
                        ChapterTargetScope::Overview => None,
                    };
                    Ok(PendingChapterPreviewSummary {
                        generation_id: preview.generation_id,
                        project_id: Some(revision.project_id),
                        snapshot_id: Some(revision.snapshot_id),
                        title: revision.title,
                        model_id: Some(preview.model_id),
                        target_path,
                        unavailable_reason: None,
                        generation_origin: revision.generation_origin,
                        origin_content_matches,
                    })
                }
                Err(reason) => {
                    let filename = entry
                        .path
                        .file_stem()
                        .and_then(|value| value.to_str())
                        .ok_or_else(|| "The generated preview filename is invalid.".to_owned())?;
                    let generation_id = format!("generation:{filename}");
                    parse_generation_id(&generation_id)?;
                    Ok(PendingChapterPreviewSummary {
                        generation_id,
                        project_id: None,
                        snapshot_id: None,
                        title: "Saved preview needs attention".to_owned(),
                        model_id: None,
                        target_path: None,
                        unavailable_reason: Some(reason),
                        generation_origin: None,
                        origin_content_matches: None,
                    })
                }
            },
        )
        .collect::<Result<Vec<_>, String>>()?;
    summaries.sort_by(|left, right| left.generation_id.cmp(&right.generation_id));
    Ok(summaries)
}

pub fn remove(storage_dir: &Path, generation_id: &str) -> Result<(), String> {
    let _guard = lock_store()?;
    let path = record_path(storage_dir, generation_id)?;
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_file() && !metadata.file_type().is_symlink() => {
            fs::remove_file(path).map_err(|_| {
                "The generated preview could not be removed from local recovery.".to_owned()
            })
        }
        Ok(_) => Err("The generated preview recovery entry is not a regular file.".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("The generated preview recovery entry could not be inspected.".to_owned()),
    }
}

fn lock_store() -> Result<MutexGuard<'static, ()>, String> {
    PREVIEW_STORE_LOCK
        .lock()
        .map_err(|_| "The local generated preview store is temporarily unavailable.".to_owned())
}

fn preview_directory(storage_dir: &Path) -> Result<PathBuf, String> {
    ensure_real_directory(storage_dir)?;
    let directory = storage_dir.join("pending-chapter-previews");
    match fs::create_dir(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err("The local generated preview folder is unavailable.".to_owned()),
    }
    ensure_real_directory(&directory)?;
    Ok(directory)
}

pub(crate) fn ensure_real_directory(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() => {
            Ok(())
        }
        Ok(_) => Err("The local generated preview folder is not a regular directory.".to_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path).map_err(|_| {
                "The local generated preview folder could not be created.".to_owned()
            })?;
            let metadata = fs::symlink_metadata(path).map_err(|_| {
                "The local generated preview folder could not be inspected.".to_owned()
            })?;
            if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
                Ok(())
            } else {
                Err("The local generated preview folder is not a regular directory.".to_owned())
            }
        }
        Err(_) => Err("The local generated preview folder could not be inspected.".to_owned()),
    }
}

struct PreviewBytes {
    path: PathBuf,
    bytes: usize,
    bytes_vec: Vec<u8>,
}

fn read_all(directory: &Path) -> Result<Vec<PreviewBytes>, String> {
    let mut entries = Vec::new();
    let mut visited = 0usize;
    let directory_entries = fs::read_dir(directory)
        .map_err(|_| "The local generated preview folder could not be read.".to_owned())?;
    for entry in directory_entries {
        visited += 1;
        if visited > MAX_DIRECTORY_ENTRIES {
            return Err("The local generated preview folder contains too many entries.".to_owned());
        }
        let entry = entry
            .map_err(|_| "The local generated preview folder could not be read.".to_owned())?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(".preview-") && name.ends_with(".tmp") {
            let metadata = fs::symlink_metadata(&path)
                .map_err(|_| "A staged generated preview could not be inspected.".to_owned())?;
            if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
                return Err("A staged generated preview is not a regular file.".to_owned());
            }
            continue;
        }
        if !name.ends_with(".json") {
            return Err(
                "The local generated preview folder contains an unsupported entry.".to_owned(),
            );
        }
        let id = name.strip_suffix(".json").unwrap_or_default();
        let uuid = Uuid::parse_str(id)
            .map_err(|_| "A generated preview filename is invalid.".to_owned())?;
        if uuid.to_string() != id {
            return Err("A generated preview filename is not canonical.".to_owned());
        }
        let file = open_without_following_symlinks(&path)
            .map_err(|_| "A generated preview is not a readable regular file.".to_owned())?;
        let metadata = file
            .metadata()
            .map_err(|_| "A generated preview could not be inspected.".to_owned())?;
        if !metadata.file_type().is_file() {
            return Err("A generated preview is not a regular file.".to_owned());
        }
        let size = checked_size(&metadata)?;
        let total = entries
            .iter()
            .map(|item: &PreviewBytes| item.bytes)
            .sum::<usize>();
        if size > MAX_PREVIEW_BYTES || total.saturating_add(size) > MAX_TOTAL_PREVIEW_BYTES {
            return Err(
                "The local generated preview folder exceeds its supported size.".to_owned(),
            );
        }
        let mut bytes_vec = Vec::with_capacity(size);
        file.take((MAX_PREVIEW_BYTES + 1) as u64)
            .read_to_end(&mut bytes_vec)
            .map_err(|_| "A generated preview could not be read.".to_owned())?;
        if bytes_vec.len() != size || bytes_vec.len() > MAX_PREVIEW_BYTES {
            return Err("A generated preview changed while it was being read.".to_owned());
        }
        entries.push(PreviewBytes {
            path,
            bytes: size,
            bytes_vec,
        });
    }
    if entries.len() > MAX_PREVIEW_COUNT {
        return Err("The local generated preview folder contains too many records.".to_owned());
    }
    Ok(entries)
}

fn checked_size(metadata: &Metadata) -> Result<usize, String> {
    usize::try_from(metadata.len())
        .map_err(|_| "A generated preview has an unsupported size.".to_owned())
}

pub(crate) fn open_without_following_symlinks(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // A path swapped for a FIFO must not block open before the caller can
        // inspect the handle and reject a nonregular file. Regular reads retain
        // their existing behavior.
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    options.open(path)
}

fn record_path(storage_dir: &Path, generation_id: &str) -> Result<PathBuf, String> {
    let (uuid, _) = parse_generation_id(generation_id)?;
    Ok(preview_directory(storage_dir)?.join(format!("{uuid}.json")))
}

fn parse_generation_id(generation_id: &str) -> Result<(Uuid, String), String> {
    let value = generation_id
        .strip_prefix("generation:")
        .ok_or_else(|| "The generated preview ID is invalid.".to_owned())?;
    let uuid =
        Uuid::parse_str(value).map_err(|_| "The generated preview ID is invalid.".to_owned())?;
    if uuid.to_string() != value {
        return Err("The generated preview ID is not canonical.".to_owned());
    }
    Ok((uuid, value.to_owned()))
}

fn read_preview_file(
    path: &Path,
    expected_generation_id: &str,
) -> Result<ChapterDraftPreview, String> {
    let file = open_without_following_symlinks(path).map_err(|_| {
        "This generated preview is not available as a regular local recovery file.".to_owned()
    })?;
    let metadata = file
        .metadata()
        .map_err(|_| "The generated preview recovery entry could not be inspected.".to_owned())?;
    if !metadata.file_type().is_file() {
        return Err("The generated preview recovery entry is not a regular file.".to_owned());
    }
    let size = checked_size(&metadata)?;
    if size > MAX_PREVIEW_BYTES {
        return Err("The generated preview exceeds its 2 MB local recovery limit.".to_owned());
    }
    let mut bytes = Vec::with_capacity(size);
    file.take((MAX_PREVIEW_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "The generated preview could not be read from local recovery.".to_owned())?;
    if bytes.len() != size || bytes.len() > MAX_PREVIEW_BYTES {
        return Err("The generated preview changed while it was being read.".to_owned());
    }
    let preview = decode_preview(&bytes, path)?;
    if preview.generation_id != expected_generation_id {
        return Err("The generated preview ID does not match its recovery file.".to_owned());
    }
    Ok(preview)
}

fn decode_preview(bytes: &[u8], path: &Path) -> Result<ChapterDraftPreview, String> {
    let record: PreviewFile = serde_json::from_slice(bytes).map_err(|_| {
        format!(
            "The generated preview in '{}' is malformed.",
            path.display()
        )
    })?;
    if record.schema_version != PREVIEW_SCHEMA_VERSION {
        return Err(format!(
            "Generated preview schema version {} is not supported by this app.",
            record.schema_version
        ));
    }
    validate_preview(&record.preview)?;
    let filename = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "The generated preview filename is invalid.".to_owned())?;
    let (uuid, _) = parse_generation_id(&record.preview.generation_id)?;
    if uuid.to_string() != filename {
        return Err("The generated preview ID does not match its recovery filename.".to_owned());
    }
    Ok(record.preview)
}

fn validate_preview(preview: &ChapterDraftPreview) -> Result<(), String> {
    parse_generation_id(&preview.generation_id)?;
    if preview
        .revision
        .generation_origin
        .as_ref()
        .is_some_and(|origin| {
            !origin.matches_content(&preview.revision) || origin.model_id() != preview.model_id
        })
    {
        return Err(
            "The recoverable preview does not match its recorded original agent content/model."
                .to_owned(),
        );
    }
    if preview.revision.schema_version != chapters::CHAPTER_SCHEMA_VERSION
        || preview.revision.lifecycle != RevisionLifecycle::Draft
        || preview.revision.validation_status != ValidationStatus::Valid
    {
        return Err(
            "Only supported, locally validated, unaccepted chapter drafts can be recovered."
                .to_owned(),
        );
    }
    if preview.validation.status != ValidationReportStatus::Valid
        || !preview.validation.issues.is_empty()
        || preview.validation.schema_version != preview.revision.schema_version
        || preview.validation.chapter_id != preview.revision.chapter_id
        || preview.validation.revision_id != preview.revision.revision_id
        || preview.validation.snapshot_id != preview.revision.snapshot_id
    {
        return Err(
            "The generated preview's validation report does not match its draft.".to_owned(),
        );
    }
    if preview.model_id.trim().is_empty()
        || preview.model_id.len() > MAX_MODEL_METADATA_BYTES
        || preview
            .model_version
            .as_ref()
            .is_some_and(|value| value.len() > MAX_MODEL_METADATA_BYTES)
    {
        return Err("The generated preview has invalid or oversized model metadata.".to_owned());
    }
    if preview.revision.title.trim().is_empty()
        || preview.revision.title.len() > 160
        || preview.revision.sections.is_empty()
        || preview.revision.sections.len() > 500
    {
        return Err("The generated preview does not contain a bounded chapter draft.".to_owned());
    }
    chapters::encode_chapter_revision(&preview.revision)
        .map_err(|_| "The generated preview contains an invalid chapter revision.".to_owned())?;
    Ok(())
}

struct StagedPreview(PathBuf);

impl Drop for StagedPreview {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, simple_proposal};
    use std::collections::HashSet;

    fn temporary_storage() -> PathBuf {
        let directory =
            std::env::temp_dir().join(format!("code-atlas-pending-preview-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn preview() -> ChapterDraftPreview {
        let fixture = Fixture::new();
        let proposal = serde_json::to_string(&simple_proposal()).unwrap();
        let mut revision = crate::chapter_generation::canonicalize_file_chapter(
            &fixture.manifest.summary.project_id,
            &fixture.manifest,
            Some(&fixture.index),
            SIMPLE_PATH,
            &fixture.sources(&[SIMPLE_PATH]),
            &HashSet::new(),
            &proposal,
        )
        .unwrap();
        let validation = chapters::validate_chapter_revision(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            &revision,
        );
        assert_eq!(validation.status, ValidationReportStatus::Valid);
        revision.validation_status = ValidationStatus::Valid;
        ChapterDraftPreview {
            generation_id: format!("generation:{}", Uuid::new_v4()),
            revision,
            validation,
            model_id: "gemini-test".to_owned(),
            model_version: Some("model-version".to_owned()),
            input_tokens: Some(4),
            output_tokens: Some(8),
            update_notes: Vec::new(),
        }
    }

    #[test]
    fn preview_round_trips_as_a_separate_unaccepted_record_without_request_secrets() {
        let directory = temporary_storage();
        let value = preview();
        save(&directory, &value).unwrap();

        let restored = load(&directory, &value.generation_id).unwrap();
        assert_eq!(restored, value);
        assert_eq!(restored.revision.lifecycle, RevisionLifecycle::Draft);
        assert_eq!(list(&directory).unwrap().len(), 1);
        let serialized =
            fs::read_to_string(record_path(&directory, &value.generation_id).unwrap()).unwrap();
        assert!(!serialized.contains("apiKey"));
        assert!(!serialized.contains("consent"));
        assert!(!serialized.contains("systemInstruction"));
        assert!(!serialized.contains("userPrompt"));

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn unsupported_schema_and_malformed_records_are_preserved_for_recovery_diagnostics() {
        let directory = temporary_storage();
        let value = preview();
        save(&directory, &value).unwrap();
        let path = record_path(&directory, &value.generation_id).unwrap();
        let original = fs::read(&path).unwrap();
        let mut record: serde_json::Value = serde_json::from_slice(&original).unwrap();
        record["schemaVersion"] = serde_json::json!(99);
        fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert!(
            load(&directory, &value.generation_id)
                .unwrap_err()
                .contains("schema version 99")
        );
        assert_ne!(fs::read(&path).unwrap(), original);

        record["schemaVersion"] = serde_json::json!(PREVIEW_SCHEMA_VERSION);
        record["preview"]["consent"] = serde_json::json!({"previouslyAcknowledged": true});
        fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert!(
            load(&directory, &value.generation_id)
                .unwrap_err()
                .contains("malformed")
        );

        fs::write(&path, vec![b'x'; MAX_PREVIEW_BYTES + 1]).unwrap();
        assert!(
            load(&directory, &value.generation_id)
                .unwrap_err()
                .contains("2 MB")
        );
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn unsupported_existing_preview_does_not_block_a_new_agent_preview() {
        let directory = temporary_storage();
        let mut unsupported = preview();
        unsupported.revision.schema_version = chapters::CHAPTER_SCHEMA_VERSION - 1;
        let unsupported_id = unsupported.generation_id.clone();
        let unsupported_path = record_path(&directory, &unsupported_id).unwrap();
        let unsupported_bytes = serde_json::to_vec(&PreviewFile {
            schema_version: PREVIEW_SCHEMA_VERSION,
            preview: unsupported,
        })
        .unwrap();
        fs::write(&unsupported_path, &unsupported_bytes).unwrap();

        let pending = chapters::PendingChapterDrafts::default();
        let candidate = preview();
        let mut published = candidate.clone();
        let saved = pending.try_publish(candidate.revision.clone(), |generation_id| {
            published.generation_id = generation_id;
            try_save(&directory, &published)?;
            Ok(published.clone())
        });

        let saved = saved.expect("an unsupported unrelated preview must not block publication");
        assert_eq!(
            pending.get(&saved.generation_id).unwrap(),
            candidate.revision
        );
        assert_eq!(load(&directory, &saved.generation_id).unwrap(), saved);
        assert_eq!(fs::read(&unsupported_path).unwrap(), unsupported_bytes);

        let summaries = list(&directory).unwrap();
        assert_eq!(summaries.len(), 2);
        assert!(summaries.iter().any(|entry| {
            entry.generation_id == unsupported_id && entry.unavailable_reason.is_some()
        }));
        assert!(summaries.iter().any(|entry| {
            entry.generation_id == saved.generation_id && entry.unavailable_reason.is_none()
        }));

        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn duplicate_id_and_oversized_metadata_fail_without_replacing_existing_preview() {
        let directory = temporary_storage();
        let value = preview();
        save(&directory, &value).unwrap();
        let original = fs::read(record_path(&directory, &value.generation_id).unwrap()).unwrap();
        assert!(save(&directory, &value).is_err());
        assert_eq!(
            fs::read(record_path(&directory, &value.generation_id).unwrap()).unwrap(),
            original
        );

        let mut oversized = preview();
        oversized.model_id = "x".repeat(MAX_MODEL_METADATA_BYTES + 1);
        assert!(
            save(&directory, &oversized)
                .unwrap_err()
                .contains("model metadata")
        );
        assert_eq!(list(&directory).unwrap().len(), 1);

        let mut oversized_record = preview();
        oversized_record.revision.sections[0].prose = "x".repeat(MAX_PREVIEW_BYTES);
        assert!(
            save(&directory, &oversized_record)
                .unwrap_err()
                .contains("2 MB")
        );
        assert_eq!(list(&directory).unwrap().len(), 1);
        let _ = fs::remove_dir_all(directory);
    }

    #[test]
    fn aggregate_byte_cap_refuses_a_new_preview_without_corrupting_existing_records() {
        let directory = temporary_storage();
        let mut template = preview();
        template.revision.sections[0].prose = "x".repeat(1_050_000);
        let mut ids = Vec::new();
        for _ in 0..15 {
            let mut value = template.clone();
            value.generation_id = format!("generation:{}", Uuid::new_v4());
            ids.push(value.generation_id.clone());
            save(&directory, &value).unwrap();
        }
        let mut extra = template;
        extra.generation_id = format!("generation:{}", Uuid::new_v4());
        assert!(save(&directory, &extra).unwrap_err().contains("16 MB"));
        assert_eq!(list(&directory).unwrap().len(), 15);
        assert!(ids.iter().all(|id| load(&directory, id).is_ok()));
        let _ = fs::remove_dir_all(directory);
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_preview_directory_is_rejected_without_writing_outside_the_app_folder() {
        let directory = temporary_storage();
        let outside = temporary_storage();
        let sentinel = outside.join("keep.txt");
        fs::write(&sentinel, b"keep").unwrap();
        std::os::unix::fs::symlink(&outside, directory.join("pending-chapter-previews")).unwrap();
        assert!(
            save(&directory, &preview())
                .unwrap_err()
                .contains("not a regular directory")
        );
        assert_eq!(fs::read(&sentinel).unwrap(), b"keep");
        let _ = fs::remove_dir_all(directory);
        let _ = fs::remove_dir_all(outside);
    }

    #[test]
    fn bounded_record_count_refuses_a_new_preview_without_evicting_saved_work() {
        let directory = temporary_storage();
        let mut ids = Vec::new();
        for _ in 0..MAX_PREVIEW_COUNT {
            let value = preview();
            ids.push(value.generation_id.clone());
            save(&directory, &value).unwrap();
        }
        let extra = preview();
        assert!(
            save(&directory, &extra)
                .unwrap_err()
                .contains("Too many generated previews")
        );
        assert_eq!(list(&directory).unwrap().len(), MAX_PREVIEW_COUNT);
        assert!(ids.iter().all(|id| load(&directory, id).is_ok()));
        let _ = fs::remove_dir_all(directory);
    }
}
