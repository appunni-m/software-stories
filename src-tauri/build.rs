use sha2::{Digest, Sha256};
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let mut inputs = ["build.rs", "Cargo.toml", "Cargo.lock", "tauri.conf.json"]
        .into_iter()
        .map(|path| root.join(path))
        .collect::<Vec<_>>();
    for directory in ["src", "resources", "capabilities"] {
        let directory = root.join(directory);
        println!("cargo:rerun-if-changed={}", directory.display());
        collect_inputs(&directory, &mut inputs)?;
    }
    inputs.sort();
    let mut hash = Sha256::new();
    for path in inputs {
        let relative = path.strip_prefix(&root)?;
        // Evidence is added after qualification. Hashing its own receipt would
        // make every profile invalidate itself when inserted into the release.
        if relative == Path::new("resources/claude_generation_qualifications.json") {
            continue;
        }
        let name = relative
            .to_str()
            .ok_or("Native build input path is not UTF-8")?;
        if !fs::symlink_metadata(&path)?.file_type().is_file() {
            return Err("Native qualification inputs must be regular files".into());
        }
        let bytes = fs::read(&path)?;
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(&bytes);
        println!("cargo:rerun-if-changed={}", path.display());
    }
    println!(
        "cargo:rustc-env=CODE_ATLAS_CLAUDE_ADAPTER_SHA256={:x}",
        hash.finalize()
    );
    tauri_build::build();
    Ok(())
}

fn collect_inputs(directory: &Path, inputs: &mut Vec<PathBuf>) -> Result<(), Box<dyn Error>> {
    if !fs::symlink_metadata(directory)?.file_type().is_dir() {
        return Err("Native qualification input directories cannot be symbolic links".into());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_dir() {
            collect_inputs(&entry.path(), inputs)?;
        } else if kind.is_file() {
            inputs.push(entry.path());
        } else {
            return Err("Native qualification inputs must be regular files or directories".into());
        }
    }
    Ok(())
}
