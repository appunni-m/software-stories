//! User-selected native exports bypass WebKit's download destination handler.
//! No renderer-supplied filesystem path is accepted by an IPC command.

use std::path::Path;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

const MAX_GUIDE_BYTES: usize = 25 * 1024 * 1024;
const SAVE_ERROR: &str = "The export could not be confirmed. Your explanations are preserved; check the selected destination before trying again.";

fn validate(filename: &str, extension: &str, bytes: &[u8], limit: usize) -> Result<(), String> {
    if filename.is_empty()
        || filename.len() > 120
        || !filename
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || !filename.ends_with(&format!(".{extension}"))
        || bytes.is_empty()
        || bytes.len() > limit
    {
        return Err("The export name or size is invalid. No file was written.".into());
    }
    Ok(())
}

fn write_selected(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if !path.is_absolute() || path.file_name().is_none() {
        return Err("The selected export destination is invalid.".into());
    }
    crate::codex_agent::atomic_private_write(path, bytes).map_err(|_| SAVE_ERROR.into())
}

pub(crate) async fn save(
    app: AppHandle,
    filename: String,
    extension: &'static str,
    bytes: Vec<u8>,
    limit: usize,
) -> Result<bool, String> {
    validate(&filename, extension, &bytes, limit)?;
    let (send, receive) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title("Save export")
        .set_file_name(filename)
        .add_filter("Export", &[extension])
        .save_file(move |selected| {
            let _ = send.send(selected);
        });
    let Some(selected) = receive
        .await
        .map_err(|_| "The save dialog did not finish.")?
    else {
        return Ok(false);
    };
    let path = selected
        .into_path()
        .map_err(|_| "Choose a local export destination.")?;
    // No generation/storage ownership lock is held while choosing a file or
    // writing it, and neither operation blocks the main application event loop.
    tauri::async_runtime::spawn_blocking(move || write_selected(&path, &bytes))
        .await
        .map_err(|_| SAVE_ERROR)??;
    Ok(true)
}

#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn save_offline_guide(
    app: AppHandle,
    filename: String,
    html: String,
) -> Result<bool, String> {
    // The existing accepted-guide builder owns privacy selection and HTML
    // sanitization. Saving never loads the file, executes it or publishes it.
    save(app, filename, "html", html.into_bytes(), MAX_GUIDE_BYTES).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn export_names_and_sizes_cannot_supply_paths_or_unbounded_payloads() {
        for name in [
            "",
            "../guide.html",
            "/tmp/guide.html",
            "guide\\file.html",
            "guide.json",
            "guide html",
            ".html/",
        ] {
            assert!(validate(name, "html", b"html", 10).is_err());
        }
        assert!(validate("guide.html", "html", b"", 10).is_err());
        assert!(validate("guide.html", "html", b"too large", 3).is_err());
        validate("code-atlas-guide.html", "html", b"html", 10).unwrap();
        validate("code-atlas-timing.json", "json", b"{}", 10).unwrap();
    }

    #[test]
    fn native_export_writes_exact_private_bytes_and_preserves_a_failed_destination() {
        let root =
            std::env::temp_dir().join(format!("atlas-native-export-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let path = root.join("report.json");
        write_selected(&path, b"first").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first");
        write_selected(&path, b"replacement").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"replacement");
        let directory = root.join("directory.json");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("keep"), b"untouched").unwrap();
        assert!(write_selected(&directory, b"replacement").is_err());
        assert_eq!(fs::read(directory.join("keep")).unwrap(), b"untouched");
        assert!(write_selected(Path::new("relative.json"), b"bytes").is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::{PermissionsExt, symlink};
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            let link = root.join("link.json");
            symlink(&path, &link).unwrap();
            assert!(write_selected(&link, b"unexpected").is_err());
            assert_eq!(fs::read(&path).unwrap(), b"replacement");
        }
        assert_eq!(
            fs::read_dir(&root).unwrap().count(),
            if cfg!(unix) { 3 } else { 2 }
        );
        fs::remove_dir_all(root).unwrap();
    }
}
