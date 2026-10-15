use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

#[derive(Debug)]
pub(crate) enum ReadError {
    Io(io::Error),
    TooLarge,
}

/// Reads no more than `limit + 1` bytes so an oversized file is detected
/// without first allocating its full contents.
pub(crate) fn read(path: &Path, limit: usize) -> Result<Vec<u8>, ReadError> {
    let file = File::open(path).map_err(ReadError::Io)?;
    let read_limit = limit.saturating_add(1) as u64;
    let mut bytes = Vec::with_capacity(limit.min(64 * 1024));
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(ReadError::Io)?;
    if bytes.len() > limit {
        return Err(ReadError::TooLarge);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestFile(PathBuf);

    impl TestFile {
        fn new(contents: &[u8]) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after UNIX epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "code-atlas-bounded-file-{}-{nonce}",
                std::process::id()
            ));
            fs::write(&path, contents).expect("fixture bytes should be written");
            Self(path)
        }
    }

    impl Drop for TestFile {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    #[test]
    fn accepts_an_input_at_the_exact_limit() {
        let file = TestFile::new(b"abc");
        assert_eq!(read(&file.0, 3).unwrap(), b"abc");
    }

    #[test]
    fn rejects_the_first_byte_over_the_limit() {
        let file = TestFile::new(b"abcd");
        assert!(matches!(read(&file.0, 3), Err(ReadError::TooLarge)));
    }

    #[test]
    fn zero_limit_accepts_only_an_empty_file() {
        let empty = TestFile::new(b"");
        assert_eq!(read(&empty.0, 0).unwrap(), b"");

        let nonempty = TestFile::new(b"x");
        assert!(matches!(read(&nonempty.0, 0), Err(ReadError::TooLarge)));
    }
}
