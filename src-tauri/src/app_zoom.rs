//! One observable page-zoom owner for native menus, shortcuts and restoration.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

const IN: &str = "code-atlas:zoom-in";
const OUT: &str = "code-atlas:zoom-out";
const RESET: &str = "code-atlas:zoom-reset";
const LEVEL: &str = "code-atlas:zoom-level";
const UNAVAILABLE: &str = "Text size could not be changed. Try again from the View menu.";

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ZoomAction {
    In,
    Out,
    Reset,
}

#[derive(Clone, Copy)]
enum Change {
    Action(ZoomAction),
    Restore(u16),
}

fn valid(percent: u16) -> bool {
    (20..=1000).contains(&percent) && percent.is_multiple_of(20)
}

fn persist_preference(change: Change) -> bool {
    matches!(change, Change::Action(_))
}

fn next(current: u16, change: Change) -> Result<u16, String> {
    if !valid(current) {
        return Err(UNAVAILABLE.into());
    }
    Ok(match change {
        Change::Action(ZoomAction::In) => (current + 20).min(1000),
        Change::Action(ZoomAction::Out) => current.saturating_sub(20).max(20),
        Change::Action(ZoomAction::Reset) => 100,
        Change::Restore(percent) if valid(percent) => percent,
        Change::Restore(_) => return Err(UNAVAILABLE.into()),
    })
}

pub(crate) struct AppZoom(Mutex<u16>);
impl Default for AppZoom {
    fn default() -> Self {
        Self(Mutex::new(100))
    }
}

#[derive(Clone, Serialize)]
struct ZoomChanged {
    percent: u16,
    persist: bool,
}

pub(crate) fn extend_menu(
    app: &AppHandle,
    menu: &tauri::menu::Menu<tauri::Wry>,
) -> tauri::Result<()> {
    use tauri::menu::{MenuItem, PredefinedMenuItem, Submenu};
    let mut view = None;
    for item in menu.items()? {
        if let Some(submenu) = item.as_submenu() {
            if submenu.text()? == "View" {
                view = Some(submenu.clone());
                break;
            }
        }
    }
    let view = match view {
        Some(view) => view,
        None => {
            let view = Submenu::new(app, "View", true)?;
            menu.append(&view)?;
            view
        }
    };
    view.append_items(&[
        &PredefinedMenuItem::separator(app)?,
        &MenuItem::with_id(app, LEVEL, "Zoom: 100%", false, None::<&str>)?,
        &MenuItem::with_id(app, IN, "Zoom In", true, Some("CmdOrCtrl+Equal"))?,
        &MenuItem::with_id(app, OUT, "Zoom Out", true, Some("CmdOrCtrl+Minus"))?,
        &MenuItem::with_id(app, RESET, "Actual Size", true, Some("CmdOrCtrl+0"))?,
    ])
}

fn zoom_label(app: &AppHandle) -> Option<tauri::menu::MenuItem<tauri::Wry>> {
    // Menu::get searches immediate children. The label belongs to View.
    for item in app.menu()?.items().ok()? {
        if let Some(item) = item.as_submenu().and_then(|view| view.get(LEVEL)) {
            if let Some(label) = item.as_menuitem() {
                return Some(label.clone());
            }
        }
    }
    None
}

/// Called only on the event-loop thread. Do not hold this lock across dispatch
/// from a worker: menu callbacks must never wait on a worker awaiting that thread.
fn apply(app: &AppHandle, window: &WebviewWindow, change: Change) -> Result<u16, String> {
    let state: State<'_, AppZoom> = app.state();
    let mut current = state.0.lock().map_err(|_| UNAVAILABLE)?;
    let percent = next(*current, change)?;
    window
        .set_zoom(f64::from(percent) / 100.)
        .map_err(|_| UNAVAILABLE)?;
    *current = percent;
    // A missing menu display must not report that an already applied zoom failed.
    if let Some(item) = zoom_label(app) {
        let _ = item.set_text(format!("Zoom: {percent}%"));
    }
    let _ = window.emit(
        "code-atlas:zoom-changed",
        ZoomChanged {
            percent,
            persist: persist_preference(change),
        },
    );
    Ok(percent)
}

async fn dispatch(app: AppHandle, window: WebviewWindow, change: Change) -> Result<u16, String> {
    if window.label() != "main" {
        return Err(UNAVAILABLE.into());
    }
    let (send, receive) = tokio::sync::oneshot::channel();
    let owner = app.clone();
    app.run_on_main_thread(move || {
        let _ = send.send(apply(&owner, &window, change));
    })
    .map_err(|_| UNAVAILABLE)?;
    receive.await.map_err(|_| UNAVAILABLE)?
}

#[tauri::command]
pub(crate) async fn restore_app_zoom(
    app: AppHandle,
    window: WebviewWindow,
    percent: u16,
) -> Result<u16, String> {
    if !valid(percent) {
        return Err(UNAVAILABLE.into());
    }
    dispatch(app, window, Change::Restore(percent)).await
}

#[tauri::command]
pub(crate) async fn change_app_zoom(
    app: AppHandle,
    window: WebviewWindow,
    action: ZoomAction,
) -> Result<u16, String> {
    dispatch(app, window, Change::Action(action)).await
}

pub(crate) fn handle_menu_event(app: &AppHandle, event: &tauri::menu::MenuEvent) {
    let action = match event.id().as_ref() {
        IN => ZoomAction::In,
        OUT => ZoomAction::Out,
        RESET => ZoomAction::Reset,
        _ => return,
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = apply(app, &window, Change::Action(action));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_steps_reach_exactly_two_hundred_percent_and_reset_is_exact() {
        let mut percent = 100;
        for _ in 0..5 {
            percent = next(percent, Change::Action(ZoomAction::In)).unwrap();
        }
        assert_eq!(percent, 200);
        assert_eq!(
            next(percent, Change::Action(ZoomAction::Reset)).unwrap(),
            100
        );
    }

    #[test]
    fn page_zoom_retains_the_existing_hotkey_bounds_without_overflow() {
        assert_eq!(next(1000, Change::Action(ZoomAction::In)).unwrap(), 1000);
        assert_eq!(next(20, Change::Action(ZoomAction::Out)).unwrap(), 20);
        for percent in (20..=1000).step_by(20) {
            assert_eq!(next(100, Change::Restore(percent)).unwrap(), percent);
        }
    }

    #[test]
    fn malformed_or_non_step_restoration_never_changes_the_level() {
        for percent in [0, 1, 19, 21, 99, 101, 1001, u16::MAX] {
            assert!(next(100, Change::Restore(percent)).is_err());
            assert!(next(percent, Change::Action(ZoomAction::In)).is_err());
        }
        assert!(serde_json::from_str::<ZoomAction>("\"other\"").is_err());
    }

    #[test]
    fn restoring_a_default_does_not_mark_it_as_a_user_preference() {
        assert!(!persist_preference(Change::Restore(100)));
        assert!(persist_preference(Change::Action(ZoomAction::Reset)));
    }
}
