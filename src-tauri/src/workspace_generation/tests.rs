use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH};

// libtest cannot serve the application's MCP subcommand. Fail before opening
// an account/session or reserving a measurement artifact when a live probe
// omitted its built application helper.
pub(super) fn require_live_context_helper() {
    let path = std::path::PathBuf::from(std::env::var_os("CODE_ATLAS_CONTEXT_HELPER_BIN")
        .expect("set CODE_ATLAS_CONTEXT_HELPER_BIN to the built Code Atlas executable"));
    assert!(path.is_absolute() && path.is_file(), "the live probe requires an absolute built application executable path");
    assert_ne!(std::fs::canonicalize(&path).unwrap(), std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap(), "the Rust test executable cannot serve the MCP helper");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_ne!(path.metadata().unwrap().permissions().mode() & 0o111, 0, "the live context helper is not executable");
    }
}

fn input(f: &Fixture) -> Input {
    Input {
        run_id: Uuid::new_v4().to_string(),
        project_id: f.manifest.summary.project_id.clone(),
        snapshot_id: f.manifest.summary.snapshot_id.clone(),
        grant_id: Uuid::new_v4().to_string(),
        question: "How does greeting format its result?".into(),
        references: vec![],
        model_id: "gpt-6-luna".into(),
        effort: "max".into(),
    }
}
#[test]
fn initial_request_contains_intent_and_no_whole_source_or_index() {
    let f = Fixture::new();
    let input = input(&f);
    let prompt = question_prompt(&input, &f.manifest).unwrap();
    assert!(prompt.contains(&input.question));
    assert!(!prompt.contains("export function greeting"));
    assert!(!prompt.contains("selectedSymbols"));
    assert!(prompt.len() < 1000);

}
#[test]
fn attached_large_files_are_starting_points_and_stale_or_forged_references_fail() {
    let mut f = Fixture::new();
    let text = format!(
        "export function giant() {{\n{}\n}}\n",
        "// static filler\n".repeat(40000)
    );
    f.recapture_with_source(SIMPLE_PATH, &text);
    assert!(text.len() > 512000);
    let mut input = input(&f);
    let source = &f
        .manifest
        .files
        .iter()
        .find(|f| f.path == SIMPLE_PATH)
        .unwrap();
    input.references.push(Reference {
        path: SIMPLE_PATH.into(),
        sha256: Some(source.sha256.clone()),
        from: None,
        to: None,
    });
    let prompt = question_prompt(&input, &f.manifest).unwrap();
    assert!(prompt.len() < 1200);
    assert!(!prompt.contains("static filler"));
    input.references[0].sha256 = Some("f".repeat(64));
    assert!(question_prompt(&input, &f.manifest).is_err());
    input.references[0].sha256 = None;
    input.references[0].from = Some(2);
    assert!(question_prompt(&input, &f.manifest).is_err());
    input.references[0].from = None;
    input.references[0].path = "../live.ts".into();
    assert!(question_prompt(&input, &f.manifest).is_err());
}
#[test]
fn question_settings_and_bounds_are_part_of_the_contract() {
    let f = Fixture::new();
    let mut input = input(&f);
    let first = question_prompt(&input, &f.manifest).unwrap();
    input.question = "Which input becomes the greeting?".into();
    assert_ne!(question_prompt(&input, &f.manifest).unwrap(), first);
    input.model_id = "gpt-5.6-luna".into();
    input.effort = "medium".into();
    assert!(question_prompt(&input, &f.manifest).is_ok());
    input.model_id.clear();
    assert!(question_prompt(&input, &f.manifest).is_err());
    input.model_id = "gpt-6-luna".into();
    input.effort.clear();
    assert!(question_prompt(&input, &f.manifest).is_err());
    input.effort = "max".into();
    input.model_id = "m".repeat(201);
    assert!(question_prompt(&input, &f.manifest).is_err());
    input.model_id = "gpt-6-luna".into();
    input.effort = "e".repeat(33);
    assert!(question_prompt(&input, &f.manifest).is_err());
    input.effort = "max".into();
    input.question = "q".repeat(12001);
    assert!(question_prompt(&input, &f.manifest).is_err());
}
