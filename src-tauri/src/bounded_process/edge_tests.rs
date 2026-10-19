use super::*;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

fn limits() -> Limits {
    Limits {
        command_timeout: Duration::from_secs(3),
        operation_deadline: Instant::now() + Duration::from_secs(4),
        stdout_bytes: 128,
        stderr_bytes: 128,
        stdin_bytes: 128,
    }
}

fn shell(script: &str) -> Command {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", script]);
    command
}

#[test]
fn exact_independent_output_caps_preserve_nonzero_exit_and_both_streams() {
    let output = run(
        &mut shell("printf '%0128d' 1; printf '%0128d' 2 >&2; exit 7"),
        None,
        limits(),
        None,
    )
    .unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, format!("{:0128}", 1).as_bytes());
    assert_eq!(output.stderr, format!("{:0128}", 2).as_bytes());
}

#[test]
fn stderr_overflow_has_the_same_bound_as_stdout() {
    let error = run(&mut shell("printf '%0200d' 1 >&2"), None, limits(), None).unwrap_err();
    assert!(matches!(error, RunError::OutputLimit));
}

#[test]
fn zero_output_caps_accept_empty_streams_and_reject_one_byte() {
    let mut limits = limits();
    limits.stdout_bytes = 0;
    limits.stderr_bytes = 0;
    let output = run(&mut shell("exit 0"), None, limits, None).unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    for script in ["printf x", "printf x >&2"] {
        assert!(matches!(
            run(&mut shell(script), None, limits, None),
            Err(RunError::OutputLimit)
        ));
    }
}

#[test]
fn large_binary_stdin_and_stdout_progress_without_pipe_workers() {
    let input = (0..512 * 1024).map(|i| (i % 251) as u8).collect::<Vec<_>>();
    let mut limits = limits();
    limits.stdin_bytes = input.len();
    limits.stdout_bytes = input.len();
    limits.stderr_bytes = 0;
    let output = run(
        &mut Command::new("/bin/cat"),
        Some(input.clone()),
        limits,
        None,
    )
    .unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, input);
    assert!(output.stderr.is_empty());
}

#[test]
fn original_operation_deadline_wins_over_a_longer_command_timeout() {
    let started = Instant::now();
    let mut limits = limits();
    limits.operation_deadline = started + Duration::from_millis(40);
    let error = run(&mut shell("exec /bin/sleep 3"), None, limits, None).unwrap_err();
    assert!(matches!(error, RunError::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn cancelled_or_expired_requests_fail_before_a_missing_executable_can_start() {
    let cancel = AtomicBool::new(true);
    let mut command = Command::new("/code-atlas-nonexistent-test-executable");
    assert!(matches!(
        run(&mut command, None, limits(), Some(&cancel)),
        Err(RunError::Cancelled)
    ));
    let mut limits = limits();
    limits.operation_deadline = Instant::now();
    assert!(matches!(
        run(&mut command, None, limits, None),
        Err(RunError::TimedOut)
    ));
}

struct TransientReader(u8);

impl Read for TransientReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.0 += 1;
        match self.0 {
            1 => Err(io::ErrorKind::WouldBlock.into()),
            2 => Err(io::ErrorKind::Interrupted.into()),
            3 => {
                buffer[..2].copy_from_slice(b"ok");
                Ok(2)
            }
            4 => Ok(0),
            _ => panic!("an EOF pipe must not be read again"),
        }
    }
}

#[test]
fn transient_reads_preserve_bytes_and_eof_then_allow_progress() {
    let mut reader = TransientReader(0);
    let mut bytes = Vec::new();
    let mut eof = false;
    for _ in 0..2 {
        assert!(!read_chunk(&mut reader, &mut bytes, &mut eof, 2).unwrap());
        assert!(bytes.is_empty() && !eof);
    }
    assert!(read_chunk(&mut reader, &mut bytes, &mut eof, 2).unwrap());
    assert_eq!(bytes, b"ok");
    assert!(!eof);
    assert!(read_chunk(&mut reader, &mut bytes, &mut eof, 2).unwrap());
    assert!(eof);
    assert!(!read_chunk(&mut reader, &mut bytes, &mut eof, 2).unwrap());
}

#[test]
fn overflowing_reads_never_append_bytes_past_the_retained_cap() {
    let mut reader = io::Cursor::new(b"abcd");
    let mut bytes = b"xx".to_vec();
    let mut eof = false;
    assert!(matches!(
        read_chunk(&mut reader, &mut bytes, &mut eof, 5),
        Err(RunError::OutputLimit)
    ));
    assert_eq!(bytes, b"xx");
    assert!(!eof);
}

struct FailedReader(io::ErrorKind);

impl Read for FailedReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::from(self.0))
    }
}

struct TestDirectory(PathBuf);

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn read_errors_are_typed_without_changing_retained_output() {
    let mut reader = FailedReader(io::ErrorKind::PermissionDenied);
    let mut bytes = b"previous output".to_vec();
    let mut eof = false;
    let error = read_chunk(&mut reader, &mut bytes, &mut eof, 128).unwrap_err();
    assert!(
        matches!(error, RunError::Read(error) if error.kind() == io::ErrorKind::PermissionDenied)
    );
    assert_eq!(bytes, b"previous output");
    assert!(!eof);
}

struct ZeroWriter;

impl Write for ZeroWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct FailedWriter(io::ErrorKind);

impl Write for FailedWriter {
    fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(self.0))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

struct TransientWriter {
    calls: usize,
    bytes: Vec<u8>,
}

impl Write for TransientWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        match self.calls {
            1 => Err(io::ErrorKind::WouldBlock.into()),
            2 => Err(io::ErrorKind::Interrupted.into()),
            _ => {
                let count = buffer.len().min(3);
                self.bytes.extend_from_slice(&buffer[..count]);
                Ok(count)
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn write_zero_and_os_errors_are_typed_without_advancing_input() {
    let input = b"pending input";
    let mut written = 0;
    assert!(matches!(
        write_chunk(&mut ZeroWriter, input, &mut written),
        Err(RunError::Write(error)) if error.kind() == io::ErrorKind::WriteZero
    ));
    assert_eq!(written, 0);

    assert!(matches!(
        write_chunk(
            &mut FailedWriter(io::ErrorKind::PermissionDenied),
            input,
            &mut written
        ),
        Err(RunError::Write(error)) if error.kind() == io::ErrorKind::PermissionDenied
    ));
    assert_eq!(written, 0);
}

#[test]
fn transient_and_partial_writes_preserve_the_input_cursor() {
    let input = b"partial input progresses";
    let mut writer = TransientWriter {
        calls: 0,
        bytes: Vec::new(),
    };
    let mut written = 0;

    assert!(!write_chunk(&mut writer, input, &mut written).unwrap());
    assert!(!write_chunk(&mut writer, input, &mut written).unwrap());
    assert_eq!(written, 0);

    while written < input.len() {
        assert!(write_chunk(&mut writer, input, &mut written).unwrap());
    }
    assert_eq!(writer.bytes, input);
    assert_eq!(written, input.len());
}

#[test]
fn externally_reaped_child_is_marked_unowned_before_drop() {
    let child = shell("exit 0").spawn().unwrap();
    let pid = child.id() as libc::pid_t;
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let mut status = 0;
        // SAFETY: pid is the live child just spawned above; this test models a
        // competing waiter that reaps it before RunningChild polls the handle.
        let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        if waited == pid {
            break;
        }
        assert!(
            waited == 0,
            "waitpid failed: {}",
            io::Error::last_os_error()
        );
        assert!(Instant::now() < deadline, "the test child did not exit");
        thread::sleep(Duration::from_millis(1));
    }

    let mut running = RunningChild {
        child,
        reaped: false,
    };
    let error = running.poll().unwrap_err();
    assert!(matches!(
        error,
        RunError::Wait(error) if error.raw_os_error() == Some(libc::ECHILD)
    ));
    assert!(running.reaped);
}

#[test]
fn broken_stdin_pipe_returns_a_write_error_after_reaping_the_child() {
    let directory = TestDirectory(
        PathBuf::from("/tmp").join(format!("ca-write-error-{}", uuid::Uuid::new_v4())),
    );
    fs::create_dir(&directory.0).unwrap();
    let pid_path = directory.0.join("child-pid");
    let mut command = shell(r#"printf '%s' "$$" > "$1"; exit 0"#);
    command.arg("write-error-fixture").arg(&pid_path);

    let input = vec![b'x'; 4 * 1024 * 1024];
    let mut run_limits = limits();
    run_limits.stdin_bytes = input.len();
    run_limits.stdout_bytes = 0;
    run_limits.stderr_bytes = 0;
    run_limits.command_timeout = Duration::from_secs(2);
    run_limits.operation_deadline = Instant::now() + Duration::from_secs(3);

    let result = run(&mut command, Some(input), run_limits, None);
    let pid = fs::read_to_string(&pid_path)
        .expect("the child records its PID before closing stdin")
        .parse::<libc::pid_t>()
        .unwrap();

    assert!(matches!(
        result,
        Err(RunError::Write(error)) if error.kind() == io::ErrorKind::BrokenPipe
    ));
    let mut status = 0;
    // SAFETY: pid was written by the child process just spawned by this test.
    // run() owns and must reap it before returning the broken-pipe error.
    let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
    assert_eq!(waited, -1);
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

#[test]
fn each_pipe_setup_failure_returns_only_after_reaping_the_spawned_child() {
    for failed_setup in 1..=3 {
        let directory = TestDirectory(
            PathBuf::from("/tmp").join(format!("ca-pipe-setup-{}", uuid::Uuid::new_v4())),
        );
        fs::create_dir(&directory.0).unwrap();
        let pid_path = directory.0.join("child-pid");
        let mut command = shell(r#"printf '%s' "$$" > "$1"; exec /bin/sleep 10"#);
        command.arg("pipe-setup-fixture").arg(&pid_path);

        let mut setup_count = 0;
        let mut run_limits = limits();
        run_limits.command_timeout = Duration::from_secs(3);
        run_limits.operation_deadline = Instant::now() + Duration::from_secs(4);
        let result =
            run_with_pipe_setup(&mut command, Some(Vec::new()), run_limits, None, |pipe| {
                setup_count += 1;
                if setup_count != failed_setup {
                    return nonblocking(pipe);
                }
                let deadline = Instant::now() + Duration::from_secs(2);
                while !pid_path.exists() && Instant::now() < deadline {
                    thread::sleep(Duration::from_millis(2));
                }
                if !pid_path.exists() {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "child did not report readiness",
                    ));
                }
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            });
        let pid = fs::read_to_string(&pid_path)
            .expect("the child records its PID before injected pipe setup fails")
            .parse::<libc::pid_t>()
            .unwrap();

        assert_eq!(setup_count, failed_setup);
        if failed_setup == 3 {
            assert!(matches!(
                result,
                Err(RunError::Write(error)) if error.kind() == io::ErrorKind::PermissionDenied
            ));
        } else {
            assert!(matches!(
                result,
                Err(RunError::Read(error)) if error.kind() == io::ErrorKind::PermissionDenied
            ));
        }
        let mut status = 0;
        // SAFETY: pid came from the child created by this runner. A setup error
        // after spawn must not return until the owned child has been reaped.
        let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
        assert_eq!(waited, -1);
        assert_eq!(
            io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }
}

#[test]
fn cancellation_returns_only_after_the_owned_child_is_reaped() {
    let directory = TestDirectory(
        PathBuf::from("/tmp").join(format!("ca-cancel-child-{}", uuid::Uuid::new_v4())),
    );
    fs::create_dir(&directory.0).unwrap();
    let pid_path = directory.0.join("child-pid");
    let mut command = shell(r#"printf '%s' "$$" > "$1"; exec /bin/sleep 10"#);
    command.arg("cancel-fixture").arg(&pid_path);

    let cancel = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&cancel);
    let ready_path = pid_path.clone();
    let cancellation = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !ready_path.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(2));
        }
        assert!(ready_path.exists(), "the child did not report readiness");
        thread::sleep(Duration::from_millis(20));
        signal.store(true, Ordering::Release);
    });

    let mut run_limits = limits();
    run_limits.command_timeout = Duration::from_secs(5);
    run_limits.operation_deadline = Instant::now() + Duration::from_secs(6);
    let result = run(&mut command, None, run_limits, Some(&cancel));
    cancellation.join().unwrap();
    let pid = fs::read_to_string(&pid_path)
        .expect("the child records its PID before sleeping")
        .parse::<libc::pid_t>()
        .unwrap();

    assert!(matches!(result, Err(RunError::Cancelled)));
    let mut status = 0;
    // SAFETY: pid was written by the child process just spawned by this test.
    // run() must terminate and reap it before returning cancellation.
    let waited = unsafe { libc::waitpid(pid, &mut status, libc::WNOHANG) };
    assert_eq!(waited, -1);
    assert_eq!(
        io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}
