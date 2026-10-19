//! Bounded Git child IO on Unix, including pipe draining after child exit.
//!
//! One supervisor owns all three nonblocking pipes. No pipe worker can remain
//! blocked or detached when cancellation, a deadline or an IO error wins.
//! The runner also owns a fresh process group and kills same-group descendants
//! when the child exits or an operation aborts. A descendant that deliberately
//! changes process group, session or credentials can escape. Child startup and
//! OS kill/reap are not hard real-time guarantees.

use std::io::{self, Read, Write};
#[cfg(unix)]
use std::os::fd::{AsFd, AsRawFd};
#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);
const PIPE_CHUNK_BYTES: usize = 16 * 1024;

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
    Cancelled,
    TimedOut,
    Cleanup(io::Error),
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
            Self::Cancelled => formatter.write_str("Git operation was cancelled."),
            Self::TimedOut => formatter.write_str("Git operation reached its time limit."),
            Self::Cleanup(error) => {
                write!(
                    formatter,
                    "Git child cleanup could not be confirmed: {error}"
                )
            }
            Self::OutputLimit => formatter.write_str("Git output exceeded its size limit."),
            Self::InputLimit => formatter.write_str("Git input exceeded its size limit."),
        }
    }
}

/// Run one child until its exit and both output EOFs, under the same deadline.
/// A finite read/write per pipe keeps a noisy stream from starving the other
/// pipes, cancellation or time checks. Complete stdin is closed immediately.
#[cfg(unix)]
pub(crate) fn run(
    command: &mut Command,
    input: Option<Vec<u8>>,
    limits: Limits,
    cancel: Option<&AtomicBool>,
) -> Result<Output, RunError> {
    run_with_pipe_setup(command, input, limits, cancel, nonblocking)
}

/// Keep pipe-setup errors inside the same child-ownership scope as the IO loop.
/// The injected setup operation also lets tests force a post-spawn OS failure.
#[cfg(unix)]
fn run_with_pipe_setup(
    command: &mut Command,
    input: Option<Vec<u8>>,
    limits: Limits,
    cancel: Option<&AtomicBool>,
    mut set_nonblocking: impl FnMut(&dyn AsFd) -> io::Result<()>,
) -> Result<Output, RunError> {
    if input
        .as_ref()
        .is_some_and(|bytes| bytes.len() > limits.stdin_bytes)
    {
        return Err(RunError::InputLimit);
    }
    let now = Instant::now();
    let deadline = now
        .checked_add(limits.command_timeout)
        .unwrap_or(limits.operation_deadline)
        .min(limits.operation_deadline);
    check_control(cancel, deadline)?;

    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Give this invocation a process group so cancellation also closes pipes
    // held by ordinary descendants. A child that deliberately starts a new
    // group/session is outside this ownership boundary.
    command.process_group(0);
    let spawned = command.spawn().map_err(RunError::Start)?;
    let mut child = RunningChild {
        process_group: Some(spawned.id()),
        child: spawned,
        reaped: false,
    };
    let mut stdout = child
        .child
        .stdout
        .take()
        .ok_or_else(|| RunError::Read(io::Error::other("stdout pipe was unavailable")))?;
    let mut stderr = child
        .child
        .stderr
        .take()
        .ok_or_else(|| RunError::Read(io::Error::other("stderr pipe was unavailable")))?;
    let mut stdin =
        if input.is_some() {
            Some(
                child.child.stdin.take().ok_or_else(|| {
                    RunError::Write(io::Error::other("stdin pipe was unavailable"))
                })?,
            )
        } else {
            None
        };
    set_nonblocking(&stdout).map_err(RunError::Read)?;
    set_nonblocking(&stderr).map_err(RunError::Read)?;
    if let Some(pipe) = &stdin {
        set_nonblocking(pipe).map_err(RunError::Write)?;
    }
    let input = input.unwrap_or_default();
    if input.is_empty() {
        stdin = None;
    }
    let mut written = 0_usize;
    let mut stdout_bytes = Vec::with_capacity(limits.stdout_bytes.min(PIPE_CHUNK_BYTES));
    let mut stderr_bytes = Vec::with_capacity(limits.stderr_bytes.min(PIPE_CHUNK_BYTES));
    let mut stdout_eof = false;
    let mut stderr_eof = false;
    let mut status = None;

    loop {
        check_control(cancel, deadline)?;
        let mut progressed = false;
        if status.is_none() {
            if let Some(exit) = child.poll()? {
                status = Some(exit);
                progressed = true;
            }
        }
        progressed |= read_chunk(
            &mut stdout,
            &mut stdout_bytes,
            &mut stdout_eof,
            limits.stdout_bytes,
        )?;
        progressed |= read_chunk(
            &mut stderr,
            &mut stderr_bytes,
            &mut stderr_eof,
            limits.stderr_bytes,
        )?;
        if let Some(pipe) = stdin.as_mut() {
            progressed |= write_chunk(pipe, &input, &mut written)?;
            if written == input.len() {
                stdin = None;
            }
        }
        // Reaping the child is not enough: an inherited pipe can remain open.
        // Never leave the bounded loop to join a reader or writer after exit.
        check_control(cancel, deadline)?;
        if stdout_eof && stderr_eof && stdin.is_none() {
            if let Some(status) = status {
                return Ok(Output {
                    status,
                    stdout: stdout_bytes,
                    stderr: stderr_bytes,
                });
            }
        }
        if !progressed {
            thread::sleep(POLL_INTERVAL.min(deadline.saturating_duration_since(Instant::now())));
        }
    }
}

/// Unsupported pipe APIs must fail before launch instead of substituting a
/// blocking implementation that cannot enforce this runner's IO deadline.
#[cfg(not(unix))]
pub(crate) fn run(
    _command: &mut Command,
    _input: Option<Vec<u8>>,
    _limits: Limits,
    _cancel: Option<&AtomicBool>,
) -> Result<Output, RunError> {
    Err(RunError::Start(io::Error::new(
        io::ErrorKind::Unsupported,
        "Bounded Git subprocess IO requires a supported Unix host.",
    )))
}

#[cfg(unix)]
fn check_control(cancel: Option<&AtomicBool>, deadline: Instant) -> Result<(), RunError> {
    if cancel.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        return Err(RunError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(RunError::TimedOut);
    }
    Ok(())
}

#[cfg(unix)]
fn write_chunk(
    writer: &mut impl Write,
    input: &[u8],
    written: &mut usize,
) -> Result<bool, RunError> {
    if *written == input.len() {
        return Ok(false);
    }
    let end = written.saturating_add(PIPE_CHUNK_BYTES).min(input.len());
    match writer.write(&input[*written..end]) {
        Ok(0) => Err(RunError::Write(io::Error::new(
            io::ErrorKind::WriteZero,
            "Git stdin stopped accepting input",
        ))),
        Ok(count) => {
            *written += count;
            Ok(true)
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(RunError::Write(error)),
    }
}

#[cfg(unix)]
fn read_chunk(
    reader: &mut impl Read,
    retained: &mut Vec<u8>,
    eof: &mut bool,
    limit: usize,
) -> Result<bool, RunError> {
    if *eof {
        return Ok(false);
    }
    let mut buffer = [0_u8; PIPE_CHUNK_BYTES];
    match reader.read(&mut buffer) {
        Ok(0) => {
            *eof = true;
            Ok(true)
        }
        Ok(count) => {
            if count > limit.saturating_sub(retained.len()) {
                return Err(RunError::OutputLimit);
            }
            retained.extend_from_slice(&buffer[..count]);
            Ok(true)
        }
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(RunError::Read(error)),
    }
}

#[cfg(unix)]
fn nonblocking(pipe: &dyn AsFd) -> io::Result<()> {
    let fd = pipe.as_fd().as_raw_fd();
    // SAFETY: the descriptor is borrowed from a live owned child pipe for both
    // calls. F_GETFL takes no variadic argument or pointer; failure is checked.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the same borrowed descriptor remains live. F_SETFL takes the
    // integer flags returned above plus O_NONBLOCK, preserving existing flags.
    if unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
struct RunningChild {
    child: Child,
    reaped: bool,
    process_group: Option<u32>,
}

#[cfg(unix)]
impl RunningChild {
    fn poll(&mut self) -> Result<Option<std::process::ExitStatus>, RunError> {
        if self.reaped {
            return Ok(None);
        }
        // Keep the leader unreaped while its isolated process group is
        // terminated; otherwise the OS could reuse its PID before the group
        // cleanup signal is delivered.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        // SAFETY: info is a uniquely borrowed output record. P_PID selects
        // this owned child and WNOWAIT preserves its identity for cleanup.
        let observed = unsafe {
            libc::waitid(
                libc::P_PID,
                self.child.id() as libc::id_t,
                &mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        if observed < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                return Ok(None);
            }
            // ECHILD means ownership is gone; never signal an identity another
            // waiter may already have released and the OS may have reused.
            if error.raw_os_error() == Some(libc::ECHILD) {
                self.reaped = true;
            }
            return Err(RunError::Wait(error));
        }
        if info.si_signo == 0 {
            return Ok(None);
        }
        if info.si_signo != libc::SIGCHLD {
            return Err(RunError::Wait(io::Error::new(
                io::ErrorKind::InvalidData,
                "Git returned an unexpected child status record",
            )));
        }
        self.kill_group(true).map_err(RunError::Cleanup)?;
        match self.child.wait() {
            Ok(status) => {
                self.reaped = true;
                Ok(Some(status))
            }
            Err(error) => {
                if error.raw_os_error() == Some(libc::ECHILD) {
                    self.reaped = true;
                }
                Err(RunError::Wait(error))
            }
        }
    }

    fn kill_group(&self, exited: bool) -> io::Result<()> {
        let Some(process_group) = self.process_group else {
            return Ok(());
        };
        let process_group = i32::try_from(process_group)
            .map_err(|_| io::Error::other("invalid owned process group"))?;
        // SAFETY: this is called while the Child identity remains unreaped;
        // process_group(0) created the isolated group, so a negative ID cannot
        // refer to the application process or be reused before this signal.
        if unsafe { libc::kill(-process_group, libc::SIGKILL) } < 0 {
            let error = io::Error::last_os_error();
            // On macOS, a group containing only its exited zombie leader can
            // report EPERM because there is no signalable member. When the
            // leader is still live, EPERM remains a cleanup failure.
            if error.raw_os_error() != Some(libc::ESRCH)
                && !(exited && error.raw_os_error() == Some(libc::EPERM))
            {
                return Err(error);
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for RunningChild {
    fn drop(&mut self) {
        if !self.reaped {
            if self.poll().is_ok_and(|status| status.is_some()) || self.reaped {
                return;
            }
            // poll may still report a live child; cancel its group first so
            // inherited pipes close, then retain direct-child cleanup as a
            // fallback if group signalling failed.
            let _ = self.kill_group(false);
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(all(test, unix))]
mod edge_tests;

#[cfg(all(test, unix))]
mod lifecycle_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
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
