use super::*;

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
