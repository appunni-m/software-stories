#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if code_atlas_desktop_lib::run_quality_helper() { return; }
    if code_atlas_desktop_lib::run_context_helper() { return; }
    code_atlas_desktop_lib::run()
}
