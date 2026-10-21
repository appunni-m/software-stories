//! Native close/quit handshakes. A renderer acknowledgment may close only the
//! currently issued request, after its local save barrier has completed.

use serde::Serialize;
use std::{sync::Mutex, time::Duration};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

const MAIN: &str = "main";
const DEADLINE: Duration = Duration::from_secs(20);
const UNAVAILABLE: &str = "Code Atlas stayed open to preserve unsaved changes.";
const QUIT_MENU: &str = "code-atlas:quit-after-saving";

/// Muda's predefined macOS Quit sends `terminate:` directly to NSApplication,
/// bypassing Tauri's ExitRequested event. Keep the standard menu, but replace
/// that item so both its click and Cmd+Q pass through the local save handshake.
pub(crate) fn menu(app: &AppHandle) -> tauri::Result<tauri::menu::Menu<tauri::Wry>> {
    let menu = tauri::menu::Menu::default(app)?;
    #[cfg(target_os = "macos")]
    {
        use tauri::menu::{MenuItem, PredefinedMenuItem};
        let unavailable =
            || std::io::Error::other("The guarded application menu could not be installed.");
        let items = menu.items()?;
        let application = items
            .first()
            .and_then(|item| item.as_submenu())
            .ok_or_else(unavailable)?;
        let items = application.items()?;
        let position = items.len().checked_sub(1).ok_or_else(unavailable)?;
        let original = items[position]
            .as_predefined_menuitem()
            .ok_or_else(unavailable)?;
        let title = original.text()?;
        if title != PredefinedMenuItem::quit(app, None)?.text()? {
            return Err(unavailable().into());
        }
        application.remove_at(position)?;
        application.insert(
            &MenuItem::with_id(app, QUIT_MENU, title, true, Some("CmdOrCtrl+Q"))?,
            position,
        )?;
    }
    crate::app_zoom::extend_menu(app, &menu)?;
    Ok(menu)
}

pub(crate) fn handle_menu_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    if event.id().as_ref() == QUIT_MENU {
        // Start the save handshake at the menu boundary instead of relying on
        // this menu callback to be translated into ExitRequested by the
        // platform event loop. After the renderer acknowledges, complete_app_close
        // calls AppHandle::exit; the approved intent lets that event pass once.
        if request(app, Intent::Quit) == Decision::Allow {
            app.exit(0);
        }
    } else {
        crate::app_zoom::handle_menu_event(app, &event);
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Intent {
    Quit,
    Window(String),
}

#[derive(Default)]
struct Lifecycle {
    armed: bool,
    pending: Option<(String, Intent)>,
    approved: Option<Intent>,
}

#[derive(Debug, PartialEq)]
enum Decision {
    Allow,
    Hold,
    Prepare(String),
}

impl Lifecycle {
    fn arm(&mut self) {
        self.armed = true;
        self.pending = None;
        self.approved = None;
    }
    fn request(&mut self, intent: Intent, id: String) -> Decision {
        if !self.armed {
            return Decision::Allow;
        }
        if self.approved.is_some() {
            self.approved = None;
            if matches!(intent, Intent::Window(_)) {
                // The only interactive renderer is being destroyed. A possible
                // last-window ExitRequested is safe without a second handshake.
                self.armed = false;
            }
            return Decision::Allow;
        }
        if let Some((_, pending)) = &mut self.pending {
            if matches!(intent, Intent::Quit) {
                *pending = Intent::Quit;
            }
            return Decision::Hold;
        }
        self.pending = Some((id.clone(), intent));
        Decision::Prepare(id)
    }

    fn complete(&mut self, id: &str, saved: bool) -> Result<Option<Intent>, String> {
        if self.pending.as_ref().map(|(key, _)| key.as_str()) != Some(id) {
            return Err(UNAVAILABLE.into());
        }
        let (_, intent) = self.pending.take().ok_or(UNAVAILABLE)?;
        if saved {
            self.approved = Some(intent.clone());
            Ok(Some(intent))
        } else {
            Ok(None)
        }
    }

    fn expire(&mut self, id: &str) -> bool {
        if self.pending.as_ref().is_some_and(|(key, _)| key == id) {
            self.pending = None;
            true
        } else {
            false
        }
    }
}

#[derive(Default)]
pub(crate) struct AppCloseGuard(Mutex<Lifecycle>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseRequest {
    request_id: String,
}

#[tauri::command]
pub(crate) fn register_app_close_guard(
    window: WebviewWindow,
    guard: State<'_, AppCloseGuard>,
) -> Result<(), String> {
    if window.label() != MAIN {
        return Err(UNAVAILABLE.into());
    }
    guard.0.lock().map_err(|_| UNAVAILABLE)?.arm();
    Ok(())
}

#[tauri::command]
pub(crate) fn complete_app_close(
    app: AppHandle,
    window: WebviewWindow,
    guard: State<'_, AppCloseGuard>,
    request_id: String,
    saved: bool,
) -> Result<(), String> {
    if window.label() != MAIN {
        return Err(UNAVAILABLE.into());
    }
    let intent = guard
        .0
        .lock()
        .map_err(|_| UNAVAILABLE)?
        .complete(&request_id, saved)?;
    match intent {
        Some(Intent::Quit) => app.exit(0),
        Some(Intent::Window(label)) => {
            let result = app.get_webview_window(&label).map(|window| window.close());
            if !matches!(result, Some(Ok(()))) {
                guard.0.lock().map_err(|_| UNAVAILABLE)?.approved = None;
                return Err(UNAVAILABLE.into());
            }
        }
        None => {}
    }
    Ok(())
}

fn request(app: &AppHandle, intent: Intent) -> Decision {
    let decision = match app.state::<AppCloseGuard>().0.lock() {
        Ok(mut lifecycle) => lifecycle.request(intent, uuid::Uuid::new_v4().to_string()),
        Err(_) => return Decision::Hold,
    };
    if let Decision::Prepare(id) = &decision {
        let payload = CloseRequest {
            request_id: id.clone(),
        };
        let delivered = app
            .get_webview_window(MAIN)
            .is_some_and(|window| window.emit("code-atlas:prepare-close", &payload).is_ok());
        if !delivered {
            if let Ok(mut lifecycle) = app.state::<AppCloseGuard>().0.lock() {
                lifecycle.expire(id);
            }
            return Decision::Hold;
        }
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(DEADLINE).await;
            let expired = app
                .state::<AppCloseGuard>()
                .0
                .lock()
                .is_ok_and(|mut lifecycle| lifecycle.expire(&payload.request_id));
            if expired {
                if let Some(window) = app.get_webview_window(MAIN) {
                    let _ = window.emit("code-atlas:cancel-close", payload);
                }
            }
        });
    }
    decision
}

pub(crate) fn handle_event(app: &AppHandle, event: &tauri::RunEvent) {
    match event {
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen {
            has_visible_windows: false,
            ..
        } => {
            if let Some(window) = app.get_webview_window(MAIN) {
                let _ = window.show();
                let _ = window.set_focus();
            } else if let Some(config) = app
                .config()
                .app
                .windows
                .iter()
                .find(|window| window.label == MAIN)
            {
                if let Ok(builder) = tauri::WebviewWindowBuilder::from_config(app, config) {
                    let _ = builder.build();
                }
            }
        }
        tauri::RunEvent::ExitRequested { api, .. } => {
            if request(app, Intent::Quit) != Decision::Allow {
                api.prevent_exit();
            }
        }
        tauri::RunEvent::WindowEvent {
            label,
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } if label == MAIN => {
            if request(app, Intent::Window(label.clone())) != Decision::Allow {
                api.prevent_close();
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn armed() -> Lifecycle {
        Lifecycle {
            armed: true,
            ..Lifecycle::default()
        }
    }

    #[test]
    fn startup_without_an_interactive_renderer_can_close() {
        assert_eq!(
            Lifecycle::default().request(Intent::Quit, "one".into()),
            Decision::Allow
        );
    }

    #[test]
    fn close_and_quit_wait_for_the_exact_saved_acknowledgment() {
        for intent in [Intent::Quit, Intent::Window(MAIN.into())] {
            let mut state = armed();
            assert_eq!(
                state.request(intent.clone(), "one".into()),
                Decision::Prepare("one".into())
            );
            assert!(state.complete("forged", true).is_err());
            assert!(state.pending.is_some());
            assert_eq!(state.complete("one", true).unwrap(), Some(intent.clone()));
            assert_eq!(state.request(intent, "two".into()), Decision::Allow);
            assert!(state.pending.is_none());
        }
    }

    #[test]
    fn failed_save_keeps_the_renderer_and_requires_a_new_request() {
        let mut state = armed();
        state.request(Intent::Quit, "one".into());
        assert_eq!(state.complete("one", false).unwrap(), None);
        assert!(state.armed && state.approved.is_none());
        assert_eq!(
            state.request(Intent::Quit, "two".into()),
            Decision::Prepare("two".into())
        );
        assert!(state.complete("one", true).is_err());
    }

    #[test]
    fn duplicate_requests_coalesce_and_quit_upgrades_a_window_close() {
        let mut state = armed();
        state.request(Intent::Window(MAIN.into()), "one".into());
        assert_eq!(
            state.request(Intent::Window(MAIN.into()), "two".into()),
            Decision::Hold
        );
        assert_eq!(state.request(Intent::Quit, "three".into()), Decision::Hold);
        assert_eq!(state.complete("one", true).unwrap(), Some(Intent::Quit));
        assert_eq!(state.request(Intent::Quit, "four".into()), Decision::Allow);
    }

    #[test]
    fn expired_or_previous_acknowledgments_can_never_close_a_new_session() {
        let mut state = armed();
        state.request(Intent::Quit, "one".into());
        assert!(!state.expire("wrong"));
        assert!(state.expire("one"));
        assert!(!state.expire("one"));
        assert!(state.complete("one", true).is_err());
        state.request(Intent::Quit, "two".into());
        assert!(state.complete("one", true).is_err());
        assert_eq!(state.complete("two", true).unwrap(), Some(Intent::Quit));
    }

    #[test]
    fn approved_last_window_close_does_not_request_a_destroyed_renderer_again() {
        let mut state = armed();
        state.request(Intent::Window(MAIN.into()), "one".into());
        state.complete("one", true).unwrap();
        assert_eq!(
            state.request(Intent::Window(MAIN.into()), "two".into()),
            Decision::Allow
        );
        assert_eq!(state.request(Intent::Quit, "three".into()), Decision::Allow);
    }

    #[test]
    fn a_new_renderer_cannot_inherit_an_old_close_approval_or_request() {
        let mut state = armed();
        state.request(Intent::Quit, "one".into());
        state.arm();
        assert!(state.complete("one", true).is_err());
        state.request(Intent::Quit, "two".into());
        state.complete("two", true).unwrap();
        state.arm();
        assert_eq!(
            state.request(Intent::Quit, "three".into()),
            Decision::Prepare("three".into())
        );
    }
}
