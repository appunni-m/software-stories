use super::*;
use std::fs::OpenOptions;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt};
use std::path::PathBuf;
use std::process::Command;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("code-atlas-git-cancel-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        let fixture = Self(path);
        assert!(fixture.git(&["init", "--quiet"]).status.success());
        fixture
    }

    fn git(&self, args: &[&str]) -> Output {
        git_command()
            .current_dir(&self.0)
            .args(args)
            .output()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn stopped_comparison_has_no_refs_evidence_or_invalid_ref_recovery() {
    let cancel = AtomicBool::new(true);
    let result = compare_local_revisions_cancellable(
        Path::new("/code-atlas-no-such-repository"),
        "bad..ref",
        "HEAD",
        &cancel,
    );
    assert_eq!(result.status, GitComparisonStatus::Cancelled);
    assert!(result.base.is_none() && result.head.is_none() && result.ref_problem.is_none());
    assert!(result.changes.is_empty() && result.comparison_id.is_none());
    assert_eq!(result.message.as_deref(), Some("Comparison stopped."));
    assert_eq!(
        serde_json::to_value(&result).unwrap()["status"],
        "cancelled"
    );
}

#[test]
fn cancelled_control_reaches_git_ref_validation_blob_reads_and_syntax_work() {
    let cancel = AtomicBool::new(true);
    let control = Control {
        end: Instant::now() + MAX_COMPARISON_TIME,
        cancelled: &cancel,
    };
    assert!(!valid_git_ref("refs/heads/main", control));
    assert!(matches!(
        git_output(
            Path::new("/code-atlas-no-such-repository"),
            &["status"],
            128,
            control
        ),
        Err(GitEvidenceError::Cancelled)
    ));
    assert!(parse_git_symbols(ParseLanguage::Rust, b"pub fn message() {}", control).is_none());
}

#[test]
fn cancellation_interrupts_a_real_git_child_blocked_on_config_io() {
    let fixture = Fixture::new();
    fs::write(
        fixture.0.join("input.txt"),
        "synthetic cancellation fixture\n",
    )
    .unwrap();
    let pipe = fixture.0.join("held-config");
    assert!(
        Command::new("mkfifo")
            .args(["-m", "600"])
            .arg(&pipe)
            .status()
            .unwrap()
            .success()
    );
    let head = fs::read(fixture.0.join(".git/HEAD")).unwrap();
    let config = fs::read(fixture.0.join(".git/config")).unwrap();
    let cancel = AtomicBool::new(false);
    let include = format!("include.path={}", pipe.to_str().unwrap());
    thread::scope(|scope| {
        let worker = scope.spawn(|| {
            let control = Control {
                end: Instant::now() + Duration::from_secs(5),
                cancelled: &cancel,
            };
            git_output(
                &fixture.0,
                &[
                    "-c",
                    &include,
                    "config",
                    "--includes",
                    "--get",
                    "codeatlas.fixture",
                ],
                128,
                control,
            )
        });
        // A nonblocking writer opens only once Git reaches the fixture FIFO.
        // Config reading, unlike loose-object loading, accepts FIFO streams.
        // Keep the writer open without bytes/EOF and observe that Git remains
        // blocked before requesting cancellation.
        let observe_until = Instant::now() + Duration::from_secs(2);
        let mut writer = None;
        while Instant::now() < observe_until && !worker.is_finished() {
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
            let held_until = Instant::now() + Duration::from_millis(40);
            while Instant::now() < held_until {
                assert!(
                    !worker.is_finished(),
                    "the fixture must stay blocked without EOF"
                );
                thread::sleep(Duration::from_millis(2));
            }
        }
        let stopped_at = Instant::now();
        cancel.store(true, Ordering::Release);
        let result = worker.join().unwrap();
        assert!(
            writer.is_some(),
            "Git must first be observed waiting on the owned config"
        );
        assert!(
            matches!(result, Err(GitEvidenceError::Cancelled)),
            "{result:?}"
        );
        assert!(stopped_at.elapsed() < Duration::from_secs(2));
        drop(writer);
    });
    assert!(fs::symlink_metadata(&pipe).unwrap().file_type().is_fifo());
    assert_eq!(fs::read(fixture.0.join(".git/HEAD")).unwrap(), head);
    assert_eq!(fs::read(fixture.0.join(".git/config")).unwrap(), config);
    assert_eq!(
        fs::read_to_string(fixture.0.join("input.txt")).unwrap(),
        "synthetic cancellation fixture\n"
    );
}
