//! Nonblocking native leases whose lifetime belongs to one Rust owner, even
//! when a spawned process temporarily retains a duplicated file descriptor.

use std::{fs::File, io};

pub(crate) struct Lease {
    file: File,
}

#[cfg(unix)]
pub(crate) fn try_exclusive(file: File) -> io::Result<Option<Lease>> {
    use std::os::fd::AsRawFd;
    if !file.metadata()?.is_file() {
        return Err(io::Error::from(io::ErrorKind::InvalidInput));
    }
    // SAFETY: File owns this valid descriptor throughout the call. flock
    // borrows it, takes no pointers and cannot unwind. LOCK_NB cannot wait.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(Some(Lease { file }));
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock {
        Ok(None)
    } else {
        Err(error)
    }
}

#[cfg(not(unix))]
pub(crate) fn try_exclusive(_file: File) -> io::Result<Option<Lease>> {
    Err(io::Error::from(io::ErrorKind::Unsupported))
}

#[cfg(unix)]
impl Drop for Lease {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        // SAFETY: our private File remains alive until after Drop returns.
        // Unlock only our open file description; never a different writer's
        // lease. Explicit unlock ends this owner's lease even if dup/fork has
        // retained the description. Closing File alone waits for those copies.
        // LOCK_UN releases ownership without waiting; Drop must not panic.
        let _ = unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
    }
}

#[cfg(all(test, unix))]
impl Lease {
    pub(crate) fn try_clone(&self) -> io::Result<File> {
        self.file.try_clone()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf, sync::mpsc, time::Duration};

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir()
                .join(format!("code-atlas-file-lease-{}", uuid::Uuid::new_v4()));
            crate::codex_agent::ensure_private_directory(&root).unwrap();
            fs::write(root.join("lease"), b"unchanged local lock bytes").unwrap();
            Self(root)
        }
        fn open(&self) -> File {
            File::open(self.0.join("lease")).unwrap()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn competing_acquisitions_are_nonblocking_and_do_not_change_file_bytes() {
        let f = Fixture::new();
        let owner = try_exclusive(f.open()).unwrap().unwrap();
        assert!(try_exclusive(f.open()).unwrap().is_none());
        drop(owner);
        assert!(try_exclusive(f.open()).unwrap().is_some());
        assert_eq!(
            fs::read(f.0.join("lease")).unwrap(),
            b"unchanged local lock bytes"
        );
    }

    #[test]
    fn panic_releases_ownership_while_an_inherited_descriptor_remains() {
        let f = Fixture::new();
        let owner = try_exclusive(f.open()).unwrap().unwrap();
        let inherited = owner.try_clone().unwrap();
        let result = std::panic::catch_unwind(move || {
            let _owner = owner;
            panic!("controlled native lease unwind");
        });
        assert!(result.is_err());
        let next = try_exclusive(f.open()).unwrap().unwrap();
        drop(inherited);
        assert!(try_exclusive(f.open()).unwrap().is_none());
        drop(next);
        assert!(try_exclusive(f.open()).unwrap().is_some());
    }

    #[test]
    fn moving_to_a_worker_keeps_ownership_until_the_worker_is_joined() {
        let f = Fixture::new();
        let owner = try_exclusive(f.open()).unwrap().unwrap();
        let (release, wait) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            let _owner = owner;
            wait.recv_timeout(Duration::from_secs(3)).unwrap();
        });
        let excluded = try_exclusive(f.open()).unwrap().is_none();
        release.send(()).unwrap();
        worker.join().unwrap();
        assert!(excluded);
        assert!(try_exclusive(f.open()).unwrap().is_some());
    }

    #[test]
    fn a_directory_descriptor_is_rejected_without_creating_a_lease() {
        let f = Fixture::new();
        assert_eq!(
            try_exclusive(File::open(&f.0).unwrap())
                .err()
                .unwrap()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(f.0.is_dir());
    }
}
