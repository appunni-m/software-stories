use super::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

const ROLE: &str = "CODE_ATLAS_TEST_PIPE_ROLE";
const SOCKET: &str = "CODE_ATLAS_TEST_PIPE_SOCKET";
const STREAMS: &str = "CODE_ATLAS_TEST_PIPE_STREAMS";
const READY: &str = "CODE_ATLAS_TEST_PIPE_READY";
const HOLD_PARENT: &str = "CODE_ATLAS_TEST_PIPE_HOLD_PARENT";
const FIXTURE_TEST: &str = "bounded_process::lifecycle_tests::retained_pipe_fixture";

fn fixture_command(
    role: &str,
    socket: &std::path::Path,
    streams: &str,
    ready: &std::path::Path,
    hold_parent: bool,
) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            FIXTURE_TEST,
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(ROLE, role)
        .env(SOCKET, socket)
        .env(STREAMS, streams)
        .env(READY, ready)
        .env(HOLD_PARENT, if hold_parent { "1" } else { "0" });
    command
}

fn hello(stream: &mut UnixStream, role: u8) {
    let mut message = [0; 5];
    message[0] = role;
    message[1..].copy_from_slice(&std::process::id().to_be_bytes());
    write_control(stream, &message, Duration::from_secs(1)).unwrap();
}

fn write_control(stream: &mut UnixStream, mut bytes: &[u8], timeout: Duration) -> io::Result<()> {
    stream.set_nonblocking(true)?;
    let end = Instant::now() + timeout;
    while !bytes.is_empty() {
        if Instant::now() >= end {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match stream.write(bytes) {
            Ok(0) => return Err(io::ErrorKind::WriteZero.into()),
            Ok(count) => bytes = &bytes[count..],
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

fn read_control(
    stream: &mut UnixStream,
    mut bytes: &mut [u8],
    timeout: Duration,
) -> io::Result<()> {
    stream.set_nonblocking(true)?;
    let end = Instant::now() + timeout;
    while !bytes.is_empty() {
        if Instant::now() >= end {
            return Err(io::ErrorKind::TimedOut.into());
        }
        match stream.read(bytes) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) => bytes = &mut bytes[count..],
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted
                ) =>
            {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

// This entry point only runs as an explicitly selected subprocess. The
// descendant exits on a socket release/EOF or is killed with its same-group
// parent. Tests never signal a PID after the runner has reaped its leader.
#[test]
#[ignore = "Owned subprocess fixture, invoked explicitly by the lifecycle tests."]
fn retained_pipe_fixture() {
    let role = std::env::var(ROLE).expect("fixture role is required");
    let socket = PathBuf::from(std::env::var_os(SOCKET).expect("fixture socket is required"));
    let ready = PathBuf::from(std::env::var_os(READY).expect("fixture readiness path is required"));
    let streams = std::env::var(STREAMS).unwrap();
    let mut connection = UnixStream::connect(&socket).unwrap();
    if role == "parent" {
        hello(&mut connection, b'p');
        drop(connection);
        let mut descendant = fixture_command("descendant", &socket, &streams, &ready, false);
        descendant
            .stdin(if streams == "stdin" {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stdout(if matches!(streams.as_str(), "stdout" | "both") {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stderr(if matches!(streams.as_str(), "stderr" | "both") {
                Stdio::inherit()
            } else {
                Stdio::null()
            });
        drop(descendant.spawn().unwrap());
        let ready_deadline = Instant::now() + Duration::from_secs(2);
        while !ready.exists() {
            assert!(
                Instant::now() < ready_deadline,
                "descendant did not become ready"
            );
            thread::sleep(Duration::from_millis(2));
        }
        if std::env::var(HOLD_PARENT).as_deref() == Ok("1") {
            let hold_deadline = Instant::now() + Duration::from_secs(6);
            let release = ready.with_extension("release");
            while !release.exists() {
                assert!(Instant::now() < hold_deadline, "parent release timed out");
                thread::sleep(Duration::from_millis(2));
            }
        }
        std::process::exit(7);
    }
    assert_eq!(role, "descendant");
    if matches!(streams.as_str(), "stdout" | "both") {
        std::io::stdout().write_all(b"held stdout\n").unwrap();
        std::io::stdout().flush().unwrap();
    }
    if matches!(streams.as_str(), "stderr" | "both") {
        std::io::stderr().write_all(b"held stderr\n").unwrap();
        std::io::stderr().flush().unwrap();
    }
    hello(&mut connection, b'd');
    fs::write(&ready, b"ready").unwrap();
    let mut release = [0];
    let released = read_control(&mut connection, &mut release, Duration::from_secs(5)).is_ok();
    if released {
        write_control(&mut connection, b"released", Duration::from_secs(1)).unwrap();
    }
    drop(connection);
    std::process::exit(if released { 0 } else { 8 });
}

struct Fixture {
    directory: PathBuf,
    listener: UnixListener,
}

impl Fixture {
    fn new() -> Self {
        // A short path stays below Unix sockaddr_un's platform-specific cap.
        let directory = PathBuf::from("/tmp").join(format!("ca-pipes-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&directory).unwrap();
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        let listener = UnixListener::bind(directory.join("control")).unwrap();
        listener.set_nonblocking(true).unwrap();
        Self {
            directory,
            listener,
        }
    }

    fn command(&self, streams: &str, hold_parent: bool) -> Command {
        fixture_command(
            "parent",
            &self.directory.join("control"),
            streams,
            &self.directory.join("ready"),
            hold_parent,
        )
    }

    fn observe(&self) -> (u32, HeldDescendant) {
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut parent = None;
        let mut descendant = None;
        while Instant::now() < deadline && (parent.is_none() || descendant.is_none()) {
            match self.listener.accept() {
                Ok((mut stream, _)) => {
                    let mut message = [0; 5];
                    read_control(&mut stream, &mut message, Duration::from_secs(1)).unwrap();
                    let pid = u32::from_be_bytes(message[1..].try_into().unwrap());
                    match message[0] {
                        b'p' => {
                            assert!(parent.replace(pid).is_none());
                        }
                        b'd' => {
                            assert!(descendant.is_none());
                            descendant = Some(HeldDescendant(Some(stream)));
                        }
                        _ => panic!("unexpected fixture role"),
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("fixture accept failed: {error}"),
            }
        }
        (
            parent.expect("observe the direct child"),
            descendant.expect("observe the pipe holder"),
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

struct HeldDescendant(Option<UnixStream>);

impl HeldDescendant {
    fn release(&mut self) -> io::Result<()> {
        if let Some(mut stream) = self.0.take() {
            write_control(&mut stream, b"x", Duration::from_secs(1))?;
            let mut acknowledgement = [0; 8];
            read_control(&mut stream, &mut acknowledgement, Duration::from_secs(1))?;
            assert_eq!(&acknowledgement, b"released");
            let eof = read_control(&mut stream, &mut [0], Duration::from_secs(1)).unwrap_err();
            assert_eq!(eof.kind(), io::ErrorKind::UnexpectedEof);
        }
        Ok(())
    }

    fn expect_closed(&mut self) {
        if let Some(mut stream) = self.0.take() {
            let error = read_control(&mut stream, &mut [0], Duration::from_secs(2))
                .expect_err("the owned process group should close the descendant socket");
            assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof);
        }
    }
}

impl Drop for HeldDescendant {
    fn drop(&mut self) {
        let _ = self.release();
    }
}

fn observe_reaped_parent(pid: u32) {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        // Observation only: a zombie still appears here until its owner reaps
        // it. No PID/group kill or child identity escapes the runner.
        let output = Command::new("/bin/ps")
            .args(["-p", &pid.to_string(), "-o", "pid="])
            .output()
            .unwrap();
        if output.status.code() == Some(1) && output.stdout.is_empty() {
            return;
        }
        assert!(output.status.success(), "process observation failed");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            pid.to_string()
        );
        thread::sleep(Duration::from_millis(2));
    }
    panic!("the runner must reap its direct child");
}

fn observe_live_parent(pid: u32) {
    let output = Command::new("/bin/ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output()
        .unwrap();
    assert!(output.status.success(), "live process observation failed");
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        pid.to_string()
    );
}

fn limits(timeout: Duration) -> Limits {
    Limits {
        command_timeout: Duration::from_secs(4),
        operation_deadline: Instant::now() + timeout,
        stdout_bytes: 64 * 1024,
        stderr_bytes: 64 * 1024,
        stdin_bytes: 4 * 1024 * 1024,
    }
}

fn input(streams: &str) -> Option<Vec<u8>> {
    (streams == "stdin").then(|| vec![b'x'; 4 * 1024 * 1024])
}

#[test]
fn child_exit_terminates_same_group_pipe_holders_before_deadline() {
    for streams in ["stdout", "stderr", "stdin"] {
        let fixture = Fixture::new();
        let mut command = fixture.command(streams, false);
        let limits = limits(Duration::from_secs(3));
        let started = Instant::now();
        thread::scope(|scope| {
            let worker = scope.spawn(|| run(&mut command, input(streams), limits, None));
            let (parent, mut descendant) = fixture.observe();
            observe_reaped_parent(parent);
            let result = worker.join().unwrap();
            if streams == "stdin" {
                assert!(
                    matches!(result, Err(RunError::Write(error)) if error.kind() == io::ErrorKind::BrokenPipe)
                );
            } else {
                let output = result.unwrap_or_else(|error| panic!("{streams}: {error}"));
                assert_eq!(output.status.code(), Some(7));
            }
            assert!(started.elapsed() < Duration::from_secs(2));
            descendant.expect_closed();
        });
    }
}

#[test]
fn cancellation_terminates_live_child_and_same_group_pipe_holder() {
    for streams in ["stdout", "stderr", "stdin"] {
        let fixture = Fixture::new();
        let mut command = fixture.command(streams, true);
        let limits = limits(Duration::from_secs(4));
        let cancel = AtomicBool::new(false);
        thread::scope(|scope| {
            let worker = scope.spawn(|| run(&mut command, input(streams), limits, Some(&cancel)));
            let (parent, mut descendant) = fixture.observe();
            observe_live_parent(parent);
            let held_until = Instant::now() + Duration::from_millis(40);
            while Instant::now() < held_until {
                assert!(
                    !worker.is_finished(),
                    "observe retained {streams} before Stop"
                );
                thread::sleep(Duration::from_millis(2));
            }
            let stopped_at = Instant::now();
            cancel.store(true, Ordering::Release);
            let result = worker.join().unwrap();
            assert!(
                matches!(result, Err(RunError::Cancelled)),
                "{streams}: {result:?}"
            );
            assert!(stopped_at.elapsed() < Duration::from_secs(2));
            observe_reaped_parent(parent);
            descendant.expect_closed();
        });
    }
}

#[test]
fn a_reaped_nonzero_parent_keeps_captured_output_and_closes_pipe_holders() {
    let fixture = Fixture::new();
    let mut command = fixture.command("both", false);
    let limits = limits(Duration::from_secs(4));
    thread::scope(|scope| {
        let worker = scope.spawn(|| run(&mut command, None, limits, None));
        let (parent, mut descendant) = fixture.observe();
        observe_reaped_parent(parent);
        let output = worker.join().unwrap().unwrap();
        assert_eq!(output.status.code(), Some(7));
        assert!(
            output
                .stdout
                .windows(12)
                .any(|bytes| bytes == b"held stdout\n")
        );
        assert_eq!(output.stderr, b"held stderr\n");
        descendant.expect_closed();
    });
}

#[test]
fn simultaneous_full_stdin_stdout_and_stderr_make_progress() {
    let mut command = Command::new("/bin/sh");
    command.args(["-c", "dd if=/dev/zero bs=16384 count=32 2>/dev/null & dd if=/dev/zero bs=16384 count=32 1>&2 2>/dev/null & cat >/dev/null; wait"]);
    let size = 512 * 1024;
    let mut limits = limits(Duration::from_secs(4));
    limits.stdout_bytes = size;
    limits.stderr_bytes = size;
    limits.stdin_bytes = size;
    let output = run(&mut command, Some(vec![b'x'; size]), limits, None).unwrap();
    assert!(output.status.success());
    assert_eq!(output.stdout, vec![0; size]);
    assert_eq!(output.stderr, vec![0; size]);
}
