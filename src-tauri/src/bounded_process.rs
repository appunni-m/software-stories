use std::io::{self, Read, Write};
use std::process::{Child, Command, Output, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const DRAIN_BUFFER_BYTES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug)]
pub(crate) struct Limits {
    pub command_timeout: Duration,
    pub operation_deadline: Instant,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
    pub stdin_bytes: usize,
}

#[derive(Debug)]
pub(crate) enum RunError {
    Start(io::Error),
    Wait(io::Error),
    Read(io::Error),
    Write(io::Error),
    ReaderPanicked,
    WriterPanicked,
    Cancelled,
    TimedOut,
    OutputLimit,
    InputLimit,
}

impl RunError {
    pub(crate) fn start_error(&self) -> Option<&io::Error> {
        match self {
            Self::Start(error) => Some(error),
            _ => None,
        }
    }
}

impl std::fmt::Display for RunError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start(error) => write!(formatter, "Git could not start: {error}"),
            Self::Wait(error) => write!(formatter, "Git process state could not be read: {error}"),
            Self::Read(error) => write!(formatter, "Git output could not be read: {error}"),
            Self::Write(error) => write!(formatter, "Git input could not be written: {error}"),
            Self::ReaderPanicked => formatter.write_str("Git output reader stopped unexpectedly."),
            Self::WriterPanicked => formatter.write_str("Git input writer stopped unexpectedly."),
            Self::Cancelled => formatter.write_str("Git operation was cancelled."),
            Self::TimedOut => formatter.write_str("Git operation reached its time limit."),
            Self::OutputLimit => formatter.write_str("Git output exceeded its size limit."),
            Self::InputLimit => formatter.write_str("Git input exceeded its size limit."),
        }
    }
}

struct DrainedOutput {
    bytes: Vec<u8>,
    exceeded_limit: bool,
}

/// Runs one child with bounded time, pipe storage, and optional cancellation.
/// Reader threads keep draining after a limit is crossed so a full pipe cannot
/// deadlock the child; the supervising thread then kills and reaps it.
pub(crate) fn run(
    command: &mut Command,
    input: Option<Vec<u8>>,
    limits: Limits,
    cancel: Option<&AtomicBool>,
) -> Result<Output, RunError> {
    if input
        .as_ref()
        .is_some_and(|bytes| bytes.len() > limits.stdin_bytes)
    {
        return Err(RunError::InputLimit);
    }
    if cancel.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        return Err(RunError::Cancelled);
    }

    let now = Instant::now();
    let command_deadline = now
        .checked_add(limits.command_timeout)
        .unwrap_or(limits.operation_deadline)
        .min(limits.operation_deadline);
    if command_deadline <= now {
        return Err(RunError::TimedOut);
    }

    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(RunError::Start)?;
    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            terminate_and_reap(&mut child);
            return Err(RunError::Read(io::Error::other(
                "stdout pipe was unavailable",
            )));
        }
    };
    let stderr = match child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            terminate_and_reap(&mut child);
            return Err(RunError::Read(io::Error::other(
                "stderr pipe was unavailable",
            )));
        }
    };
    let stdin = if input.is_some() {
        match child.stdin.take() {
            Some(stdin) => Some(stdin),
            None => {
                terminate_and_reap(&mut child);
                return Err(RunError::Write(io::Error::other(
                    "stdin pipe was unavailable",
                )));
            }
        }
    } else {
        None
    };

    let output_limit_hit = Arc::new(AtomicBool::new(false));
    let stdout_hit = Arc::clone(&output_limit_hit);
    let stdout_limit = limits.stdout_bytes;
    let stdout_reader = thread::spawn(move || drain_output(stdout, stdout_limit, stdout_hit));
    let stderr_hit = Arc::clone(&output_limit_hit);
    let stderr_limit = limits.stderr_bytes;
    let stderr_reader = thread::spawn(move || drain_output(stderr, stderr_limit, stderr_hit));
    let input_writer = match (input, stdin) {
        (Some(bytes), Some(stdin)) => Some(thread::spawn(move || {
            let mut stdin = stdin;
            stdin.write_all(&bytes)
        })),
        (None, None) => None,
        _ => unreachable!("stdin is piped exactly when input is present"),
    };

    let mut failure = None;
    loop {
        if cancel.is_some_and(|flag| flag.load(Ordering::Acquire)) {
            failure = Some(RunError::Cancelled);
            break;
        }
        if output_limit_hit.load(Ordering::Acquire) {
            failure = Some(RunError::OutputLimit);
            break;
        }
        let remaining = command_deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            failure = Some(RunError::TimedOut);
            break;
        }
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => thread::sleep(POLL_INTERVAL.min(remaining)),
            Err(error) => {
                failure = Some(RunError::Wait(error));
                break;
            }
        }
    }

    let exit_status = if failure.is_some() {
        terminate_and_reap(&mut child);
        None
    } else {
        match child.wait() {
            Ok(status) => Some(status),
            Err(error) => {
                failure = Some(RunError::Wait(error));
                terminate_and_reap(&mut child);
                None
            }
        }
    };

    let stdout = join_reader(stdout_reader)?;
    let stderr = join_reader(stderr_reader)?;
    let input_error = match input_writer {
        Some(writer) => writer.join().map_err(|_| RunError::WriterPanicked)?.err(),
        None => None,
    };

    if let Some(error) = failure {
        return Err(error);
    }
    if stdout.exceeded_limit || stderr.exceeded_limit {
        return Err(RunError::OutputLimit);
    }
    if let Some(error) = input_error {
        return Err(RunError::Write(error));
    }

    let status = exit_status
        .ok_or_else(|| RunError::Wait(io::Error::other("Git exited without an exit status")))?;
    Ok(Output {
        status,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
    })
}

fn drain_output<R: Read>(
    mut reader: R,
    limit: usize,
    limit_hit: Arc<AtomicBool>,
) -> io::Result<DrainedOutput> {
    let mut bytes = Vec::with_capacity(limit.min(DRAIN_BUFFER_BYTES));
    let mut buffer = [0_u8; DRAIN_BUFFER_BYTES];
    let mut exceeded_limit = false;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        let remaining = limit.saturating_sub(bytes.len());
        let retained = remaining.min(read);
        bytes.extend_from_slice(&buffer[..retained]);
        if retained < read {
            exceeded_limit = true;
            limit_hit.store(true, Ordering::Release);
        }
    }
    Ok(DrainedOutput {
        bytes,
        exceeded_limit,
    })
}

fn join_reader(
    reader: thread::JoinHandle<io::Result<DrainedOutput>>,
) -> Result<DrainedOutput, RunError> {
    reader
        .join()
        .map_err(|_| RunError::ReaderPanicked)?
        .map_err(RunError::Read)
}

fn terminate_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn limits(timeout: Duration) -> Limits {
        Limits {
            command_timeout: timeout,
            operation_deadline: Instant::now() + timeout + Duration::from_secs(1),
            stdout_bytes: 128,
            stderr_bytes: 128,
            stdin_bytes: 128,
        }
    }

    #[cfg(unix)]
    fn shell(script: &str) -> Command {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", script]);
        command
    }

    #[cfg(unix)]
    fn sleeper() -> Command {
        let mut command = Command::new("/bin/sleep");
        command.arg("10");
        command
    }

    #[cfg(unix)]
    #[test]
    fn output_overflow_is_bounded_and_reported() {
        let error = run(
            &mut shell("printf '%0200d' 1"),
            None,
            limits(Duration::from_secs(1)),
            None,
        )
        .expect_err("output above the configured cap should be rejected");
        assert!(matches!(error, RunError::OutputLimit));
    }

    #[cfg(unix)]
    #[test]
    fn timeout_terminates_the_child() {
        let started = Instant::now();
        let error = run(
            &mut sleeper(),
            None,
            limits(Duration::from_millis(40)),
            None,
        )
        .expect_err("a sleeping child should hit the deadline");
        assert!(matches!(error, RunError::TimedOut));
        assert!(started.elapsed() < Duration::from_secs(2));
    }

    #[cfg(unix)]
    #[test]
    fn blocked_stdin_write_obeys_the_command_deadline() {
        let mut run_limits = limits(Duration::from_millis(40));
        run_limits.stdin_bytes = 512 * 1024;
        let error = run(
            &mut sleeper(),
            Some(vec![b'x'; run_limits.stdin_bytes]),
            run_limits,
            None,
        )
        .expect_err("a child that does not read stdin should be stopped at the deadline");
        assert!(matches!(error, RunError::TimedOut));
    }

    #[cfg(unix)]
    #[test]
    fn cancellation_terminates_a_running_child() {
        let cancel = Arc::new(AtomicBool::new(false));
        let trigger = Arc::clone(&cancel);
        let cancellation = thread::spawn(move || {
            thread::sleep(Duration::from_millis(40));
            trigger.store(true, Ordering::Release);
        });
        let error = run(
            &mut sleeper(),
            None,
            limits(Duration::from_secs(2)),
            Some(&cancel),
        )
        .expect_err("a cancelled child should be stopped");
        cancellation
            .join()
            .expect("cancellation thread should finish");
        assert!(matches!(error, RunError::Cancelled));
    }

    #[cfg(unix)]
    #[test]
    fn oversized_input_is_rejected_before_spawn() {
        let error = run(
            &mut shell("cat"),
            Some(vec![b'x'; 129]),
            limits(Duration::from_secs(1)),
            None,
        )
        .expect_err("input above the configured cap should be rejected");
        assert!(matches!(error, RunError::InputLimit));
    }
}
