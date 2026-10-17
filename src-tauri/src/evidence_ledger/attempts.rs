//! Private rejected candidates, separate from source-free evidence diagnostics.
//! These records are never chapters, trusted previews or inference replay input.

use super::*;
use serde_json::Value;
use std::sync::Mutex;

const MAX_RECORD: usize = 2 * 1024 * 1024;
const MAX_OPERATIONS: usize = 8;
const MAX_ENTRIES: usize = 32;
static STORE: Mutex<()> = Mutex::new(());

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Candidate {
    pub(crate) ordinal: u8,
    pub(crate) thread_id: String,
    pub(crate) turn_id: String,
    pub(crate) response_text: String,
    response_sha256: String,
    pub(crate) alignment_issues: Vec<Value>,
    pub(crate) rejection: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Candidates {
    schema_version: u32,
    binding: Binding,
    request_fingerprint: String,
    evidence_sha256: String,
    pub(crate) candidates: Vec<Candidate>,
}

fn directory(storage: &Path) -> Result<PathBuf, String> {
    let path = storage.join("rejected-explanations");
    codex_agent::ensure_private_directory(&path)?;
    Ok(path)
}

fn identity(value: &str) -> bool {
    !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control)
}

fn read(path: &Path) -> Result<Candidates, String> {
    let meta =
        fs::symlink_metadata(path).map_err(|_| "Saved rejected explanation is unavailable.")?;
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err("Saved rejected explanation is not a regular file.".into());
    }
    let bytes = bounded_file::read(path, MAX_RECORD)
        .map_err(|_| "Saved rejected explanation exceeds its bound or is unreadable.")?;
    let record: Candidates = serde_json::from_slice(&bytes)
        .map_err(|_| "Saved rejected explanation is damaged; it was preserved.")?;
    if record.schema_version != 1
        || !canonical_uuid(&record.binding.operation_id)
        || !hash(&record.request_fingerprint)
        || !hash(&record.evidence_sha256)
        || record.candidates.is_empty()
        || record.candidates.len() > 2
        || record.candidates.iter().enumerate().any(|(i, c)| {
            c.ordinal as usize != i + 1
                || !identity(&c.thread_id)
                || !identity(&c.turn_id)
                || c.response_text.len() > chapters::MAX_GENERATED_RESPONSE_BYTES
                || c.response_sha256 != context::digest(c.response_text.as_bytes())
                || c.alignment_issues
                    != crate::claim_alignment::diagnostics(&c.response_text).unwrap_or_default()
                || c.rejection
                    .as_ref()
                    .is_some_and(|r| r.is_empty() || r.len() > 2000)
        })
        || (record.candidates.len() == 2
            && (record.candidates[0].thread_id != record.candidates[1].thread_id
                || record.candidates[0].turn_id == record.candidates[1].turn_id))
    {
        return Err(
            "Saved rejected explanation has inconsistent identities or bounds; it was preserved."
                .into(),
        );
    }
    Ok(record)
}

impl Ledger {
    /// Persist only rejected/corrected candidates, with the original operation
    /// binding and delivered evidence digest. A successful initial answer does
    /// not consume a recovery slot. No existing failed operation is evicted.
    pub(crate) fn save_candidate(
        &self,
        fingerprint: &str,
        response: &codex_agent::CodexRunResult,
        ordinal: u8,
        rejection: Option<&str>,
    ) -> Result<(), String> {
        let _guard = STORE
            .lock()
            .map_err(|_| "Rejected explanation recovery is busy.")?;
        if !hash(fingerprint)
            || response.model != self.record.binding.model
            || response.effort != self.record.binding.effort
            || !(1..=2).contains(&ordinal)
            || !identity(&response.thread_id)
            || !identity(&response.turn_id)
            || response.response_text.len() > chapters::MAX_GENERATED_RESPONSE_BYTES
            || rejection.is_some_and(|r| r.is_empty() || r.len() > 2000)
        {
            return Err(
                "Rejected explanation does not belong to this operation or exceeds its bounds."
                    .into(),
            );
        }
        let directory = directory(&self.storage)?;
        let path = directory.join(format!("{}.json", self.record.binding.operation_id));
        let mut visited = 0;
        let mut count = 0;
        let mut current = None;
        for entry in
            fs::read_dir(&directory).map_err(|_| "Rejected explanation recovery is unreadable.")?
        {
            visited += 1;
            if visited > MAX_ENTRIES {
                return Err("Rejected explanation recovery has too many entries.".into());
            }
            let entry = entry.map_err(|_| "Rejected explanation recovery is unreadable.")?;
            // Interrupted staging files remain untouched, but count toward the
            // directory bound. Never follow a link or silently erase damage.
            let p = entry.path();
            if p.file_name()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.starts_with(".settings-"))
            {
                continue;
            }
            let saved = read(&p)?;
            if p.file_stem().and_then(|s| s.to_str()) != Some(saved.binding.operation_id.as_str()) {
                return Err("Rejected explanation filename and operation disagree.".into());
            }
            count += 1;
            if p == path {
                current = Some(saved);
            }
        }
        // Reserving MAX_RECORD per operation keeps the complete two-attempt
        // history within 16 MiB, even when other operations arrive meanwhile.
        if current.is_none() && count >= MAX_OPERATIONS {
            return Err("Rejected explanation recovery is full. Remove an older saved rejection before another correction.".into());
        }
        let evidence_sha256 = context::digest(
            &serde_json::to_vec(&self.admitted_ranges())
                .map_err(|_| "Captured evidence could not be identified.")?,
        );
        let mut saved = current.unwrap_or_else(|| Candidates {
            schema_version: 1,
            binding: self.record.binding.clone(),
            request_fingerprint: fingerprint.into(),
            evidence_sha256: evidence_sha256.clone(),
            candidates: vec![],
        });
        if saved.binding != self.record.binding
            || saved.request_fingerprint != fingerprint
            || saved.evidence_sha256 != evidence_sha256
        {
            return Err(
                "Saved rejected explanation belongs to different evidence or request settings."
                    .into(),
            );
        }
        let candidate = Candidate {
            ordinal,
            thread_id: response.thread_id.clone(),
            turn_id: response.turn_id.clone(),
            response_text: response.response_text.clone(),
            response_sha256: context::digest(response.response_text.as_bytes()),
            alignment_issues: crate::claim_alignment::diagnostics(&response.response_text)
                .unwrap_or_default(),
            rejection: rejection.map(str::to_owned),
        };
        if let Some(prior) = saved.candidates.get_mut(ordinal as usize - 1) {
            if prior.response_sha256 != candidate.response_sha256
                || prior.thread_id != candidate.thread_id
                || prior.turn_id != candidate.turn_id
                || (prior.rejection.is_some() && prior.rejection != candidate.rejection)
            {
                return Err("An existing rejected candidate cannot be replaced.".into());
            }
            if candidate.rejection.is_some() {
                prior.rejection = candidate.rejection;
            }
        } else {
            if ordinal as usize != saved.candidates.len() + 1
                || (ordinal == 2
                    && (saved.candidates[0].thread_id != candidate.thread_id
                        || saved.candidates[0].turn_id == candidate.turn_id))
            {
                return Err("Rejected candidate order or thread changed.".into());
            }
            saved.candidates.push(candidate);
        }
        let bytes =
            serde_json::to_vec(&saved).map_err(|_| "Rejected explanation could not be encoded.")?;
        if bytes.len() > MAX_RECORD {
            return Err("Rejected explanation recovery exceeds its size bound.".into());
        }
        codex_agent::atomic_private_write(&path, &bytes)
    }
}

/// Local inspection only: validates identity against the source-free ledger.
#[cfg(test)]
pub(crate) fn inspect(storage: &Path, operation: &str) -> Result<Candidates, String> {
    let _guard = STORE
        .lock()
        .map_err(|_| "Rejected explanation recovery is busy.")?;
    if !canonical_uuid(operation) {
        return Err("Invalid rejected explanation identity.".into());
    }
    let saved = read(&directory(storage)?.join(format!("{operation}.json")))?;
    let ledger = super::inspect(storage, operation)?;
    let admitted = ledger
        .evidence
        .iter()
        .filter(|e| {
            ledger
                .calls
                .iter()
                .any(|c| c.state == CallState::Delivered && c.evidence_ids.contains(&e.id))
        })
        .map(|e| e.range.clone())
        .collect::<Vec<_>>();
    let evidence = context::digest(
        &serde_json::to_vec(&admitted).map_err(|_| "Saved evidence identity is unavailable.")?,
    );
    if saved.binding != ledger.binding || saved.evidence_sha256 != evidence {
        return Err("Rejected explanation no longer matches its evidence operation.".into());
    }
    Ok(saved)
}

#[cfg(test)]
mod tests;
