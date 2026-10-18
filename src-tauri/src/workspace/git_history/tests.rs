use super::*;
use crate::{
    chapter_generation::tests::Fixture, repository_context_commands::RepositoryContextJobs,
};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn files(root: &Path) -> BTreeMap<PathBuf, String> {
    fn visit(root: &Path, path: &Path, out: &mut BTreeMap<PathBuf, String>) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), out);
            } else if entry.file_type().unwrap().is_file() {
                out.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    crate::repository_context::digest(&fs::read(entry.path()).unwrap()),
                );
            }
        }
    }
    let mut out = BTreeMap::new();
    visit(root, root, &mut out);
    out
}

#[test]
fn cancelled_and_expired_history_never_launch_git() {
    let missing = Path::new("/code-atlas-missing-history-repository");
    assert_eq!(
        list(missing, &AtomicBool::new(true)).unwrap_err(),
        "Git history stopped."
    );
    assert_eq!(
        list_until(
            missing,
            &AtomicBool::new(false),
            Instant::now() - Duration::from_millis(1)
        )
        .unwrap_err(),
        "Git history took too long. Try again."
    );
}

#[test]
fn real_history_preserves_captured_source_storage_and_git_metadata() {
    let fixture = Fixture::new();
    let root = Path::new(&fixture.manifest.summary.project_id);
    let before = (files(root), files(&fixture.storage));
    let commits = list(root, &AtomicBool::new(false)).unwrap();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].subject, "independent chapter fixtures");
    assert!(commits[0].parents.is_empty());
    assert!(matches!(commits[0].oid.len(), 40 | 64));
    assert_eq!((files(root), files(&fixture.storage)), before);
}

#[test]
fn oversized_history_returns_fixed_error_without_commit_text_or_mutation() {
    let fixture = Fixture::new();
    let root = Path::new(&fixture.manifest.summary.project_id);
    let message = root.join("synthetic-oversized-message.txt");
    fs::write(
        &message,
        format!("private-synthetic-fixture-{}\n", "x".repeat(600_000)),
    )
    .unwrap();
    assert!(
        project::git_command()
            .current_dir(root)
            .args(["commit", "--allow-empty", "--quiet", "-F"])
            .arg(&message)
            .status()
            .unwrap()
            .success()
    );
    let before = (files(root), files(&fixture.storage));
    assert_eq!(
        list(root, &AtomicBool::new(false)).unwrap_err(),
        "Git history exceeds its local size limit."
    );
    assert_eq!((files(root), files(&fixture.storage)), before);
}

#[cfg(unix)]
#[test]
fn owned_stop_interrupts_a_real_history_child_held_on_git_config() {
    use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt, process::Command, thread};
    let fixture = Fixture::new();
    let root = Path::new(&fixture.manifest.summary.project_id);
    let pipe = root.join("held-history-config");
    assert!(
        Command::new("mkfifo")
            .args(["-m", "600"])
            .arg(&pipe)
            .status()
            .unwrap()
            .success()
    );
    let config_path = root.join(".git/config");
    let mut config = fs::read(&config_path).unwrap();
    config.extend_from_slice(format!("\n[include]\n\tpath = {}\n", pipe.display()).as_bytes());
    fs::write(&config_path, config).unwrap();
    let before = (files(root), files(&fixture.storage));
    let jobs = RepositoryContextJobs::default();
    let id = uuid::Uuid::new_v4().to_string();
    let request = jobs.begin(&id).unwrap();
    thread::scope(|scope| {
        let worker = scope.spawn(|| list(root, request.cancellation()));
        let until = Instant::now() + Duration::from_secs(2);
        let mut writer = None;
        while Instant::now() < until && !worker.is_finished() {
            if let Ok(file) = OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&pipe)
            {
                writer = Some(file);
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        if writer.is_some() {
            let held = Instant::now() + Duration::from_millis(40);
            while Instant::now() < held {
                assert!(!worker.is_finished());
                thread::sleep(Duration::from_millis(2));
            }
        }
        let stopped = Instant::now();
        jobs.cancel(&id).unwrap();
        let result = worker.join().unwrap();
        assert!(
            writer.is_some(),
            "history must be independently observed waiting on the config stream"
        );
        assert_eq!(result.unwrap_err(), "Git history stopped.");
        assert!(stopped.elapsed() < Duration::from_secs(2));
        drop(writer);
    });
    drop(request);
    assert!(jobs.begin(&id).is_ok());
    assert_eq!((files(root), files(&fixture.storage)), before);
}
