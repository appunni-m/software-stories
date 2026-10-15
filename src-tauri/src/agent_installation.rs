//! User-selected installation paths for the existing native agent adapters.
//!
//! Selection is a path hint only. It neither executes the file nor qualifies
//! the runtime, and adapters must revalidate it at their existing boundaries.

use serde::Deserialize;
use std::{fs, sync::Mutex};
use tauri::{AppHandle, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

static PICKER: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AgentInstallationFile {
    CodexExecutable,
    ClaudeExecutable,
    NodeExecutable,
    ClaudeSdkModule,
}

impl AgentInstallationFile {
    fn title(self) -> &'static str {
        match self {
            Self::CodexExecutable => "Choose your installed Codex executable",
            Self::ClaudeExecutable => "Choose your installed Claude executable",
            Self::NodeExecutable => "Choose your installed Node executable",
            Self::ClaudeSdkModule => "Choose the installed Claude Agent SDK sdk.mjs file",
        }
    }
}

/// Open the native picker without starting or changing an agent connection.
#[tauri::command(rename_all = "camelCase")]
pub(crate) async fn pick_agent_installation_file(
    app: AppHandle,
    window: WebviewWindow,
    kind: AgentInstallationFile,
) -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _picker = PICKER
            .try_lock()
            .map_err(|_| "Finish choosing the current installation file first.".to_owned())?;
        let Some(selected) = app
            .dialog()
            .file()
            .set_title(kind.title())
            .set_parent(&window)
            .blocking_pick_file()
        else {
            return Ok(None);
        };
        let path = selected
            .into_path()
            .map_err(|_| "Choose a local installation file.".to_owned())?;
        // Keep the selected spelling (including an installation symlink).
        // Canonical identity belongs to the explicit adapter check/save.
        let text = path
            .to_str()
            .filter(|value| value.len() <= 4096 && !value.chars().any(char::is_control))
            .ok_or_else(|| "The selected path cannot be used. Choose another file.".to_owned())?;
        if !path.is_absolute() || !fs::metadata(&path).is_ok_and(|metadata| metadata.is_file()) {
            return Err("Choose an available local installation file, not a folder.".to_owned());
        }
        if matches!(kind, AgentInstallationFile::ClaudeSdkModule)
            && path.file_name().is_none_or(|name| name != "sdk.mjs")
        {
            return Err("Choose the installed Claude Agent SDK file named sdk.mjs.".to_owned());
        }
        Ok(Some(text.to_owned()))
    })
    .await
    .map_err(|_| {
        "The installation file picker could not finish. You can enter its path instead.".to_owned()
    })?
}
