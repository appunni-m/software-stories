//! Read-only history under an owned caller cancellation token.

use super::GitCommit;
use crate::{bounded_process, project};
use std::{
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

pub(super) fn list(root: &Path, cancelled: &AtomicBool) -> Result<Vec<GitCommit>, String> {
    list_until(root, cancelled, Instant::now() + Duration::from_secs(10))
}

fn list_until(
    root: &Path,
    cancelled: &AtomicBool,
    deadline: Instant,
) -> Result<Vec<GitCommit>, String> {
    let mut command = project::git_command();
    command
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .args([
            "--no-pager",
            "log",
            "--all",
            "--date-order",
            "--max-count=80",
            "--format=%H%x00%P%x00%s%x00%an%x00%aI%x00",
            "--",
        ]);
    let output = bounded_process::run(
        &mut command,
        None,
        bounded_process::Limits {
            command_timeout: Duration::from_secs(8),
            operation_deadline: deadline,
            stdout_bytes: 512_000,
            stderr_bytes: 8192,
            stdin_bytes: 0,
        },
        Some(cancelled),
    )
    .map_err(|error| {
        match error {
            bounded_process::RunError::Cancelled => "Git history stopped.",
            bounded_process::RunError::TimedOut => "Git history took too long. Try again.",
            bounded_process::RunError::OutputLimit => "Git history exceeds its local size limit.",
            _ => "Git history could not be read within its local limits.",
        }
        .to_owned()
    })?;
    check_control(cancelled, deadline)?;
    if !output.status.success() {
        return Err(
            "Git history is unavailable. This repository may have no commits yet.".to_owned(),
        );
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let fields: Vec<&str> = text.split('\0').collect();
    let mut commits = Vec::new();
    for row in fields.chunks_exact(5) {
        check_control(cancelled, deadline)?;
        let oid = row[0].trim();
        if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        commits.push(GitCommit {
            oid: oid.to_owned(),
            parents: row[1].split_whitespace().map(str::to_owned).collect(),
            subject: row[2].chars().take(300).collect(),
            author: row[3].chars().take(120).collect(),
            timestamp: row[4].to_owned(),
            is_head: false,
        });
    }
    if let Some(mut head) = current_head(root, cancelled, deadline)? {
        if let Some(commit) = commits.iter_mut().find(|commit| commit.oid == head.oid) {
            commit.is_head = true;
        } else {
            head.is_head = true;
            commits.insert(0, head);
            commits.truncate(80);
        }
    }
    check_control(cancelled, deadline)?;
    Ok(commits)
}

fn current_head(
    root: &Path,
    cancelled: &AtomicBool,
    deadline: Instant,
) -> Result<Option<GitCommit>, String> {
    check_control(cancelled, deadline)?;
    let mut command = project::git_command();
    command
        .current_dir(root)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .args([
            "--no-pager",
            "show",
            "-s",
            "--format=%H%x00%P%x00%<(300,trunc)%s%x00%<(120,trunc)%an%x00%aI",
            "HEAD",
        ]);
    let output = bounded_process::run(
        &mut command,
        None,
        bounded_process::Limits {
            command_timeout: Duration::from_secs(2),
            operation_deadline: deadline,
            stdout_bytes: 2048,
            stderr_bytes: 8192,
            stdin_bytes: 0,
        },
        Some(cancelled),
    )
    .map_err(|error| {
        match error {
            bounded_process::RunError::Cancelled => "Git history stopped.",
            bounded_process::RunError::TimedOut => "Git history took too long. Try again.",
            bounded_process::RunError::OutputLimit => "Git history exceeds its local size limit.",
            _ => "Git history could not be read within its local limits.",
        }
        .to_owned()
    })?;
    check_control(cancelled, deadline)?;
    if !output.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let fields: Vec<&str> = stdout.split('\0').collect();
    if fields.len() < 5 {
        return Ok(None);
    }
    let oid = fields[0].trim();
    if !matches!(oid.len(), 40 | 64) || !oid.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Ok(None);
    }
    Ok(Some(GitCommit {
        oid: oid.to_owned(),
        parents: fields[1].split_whitespace().map(str::to_owned).collect(),
        subject: fields[2].trim().chars().take(300).collect(),
        author: fields[3].trim().chars().take(120).collect(),
        timestamp: fields[4].trim().to_owned(),
        is_head: true,
    }))
}

fn check_control(cancelled: &AtomicBool, deadline: Instant) -> Result<(), String> {
    if cancelled.load(Ordering::Acquire) {
        Err("Git history stopped.".into())
    } else if Instant::now() >= deadline {
        Err("Git history took too long. Try again.".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
