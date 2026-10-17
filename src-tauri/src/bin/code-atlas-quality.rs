//! Separate offline checker entry point. Invalid invocation can never open a UI.
fn main() {
    if !code_atlas_desktop_lib::run_quality_helper() {
        eprintln!("Use --chapter-quality-helper with one protocol JSON request on stdin.");
        std::process::exit(2);
    }
}
