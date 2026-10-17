//! Carry native planner coordinates into a fresh investigation without copying
//! source, trusting planner claims, or pre-admitting evidence from another run.

use super::*;
use crate::evidence_ledger::RangeRequest;
use std::collections::HashMap;

const POLICY: &str = "native-starting-range-hints-v1";
const INSTRUCTIONS: &str = "The prompt's startingRanges are native-checked captured byte coordinates from an earlier planning operation. They are hints, not evidence or a source selection. You may copy their path, sourceSha256, startByte and endByte directly into read_code without repeating discovery. Read promising starting ranges early, then choose further searches and reads as the question requires. Only citeable fragments delivered by this investigation's own read_code are evidence; never cite a starting range or an earlier operation's reference ID.";

pub(super) fn request(
    storage: &Path,
    input: &Input,
    manifest: &snapshot::SnapshotManifest,
    ranges: &[context::CodeRange],
) -> Result<(String, Value, String, String), String> {
    if ranges.is_empty() {
        // Preserve the exact ordinary-question contract and historical prompt.
        let (prompt, schema, fingerprint) = super::request(input, manifest)?;
        return Ok((prompt, schema, fingerprint, system_instructions()));
    }
    if ranges.len() > 4 {
        return Err("A guide unit may have at most four starting ranges.".into());
    }
    let mut prompt: Value = serde_json::from_str(&question_prompt(input, manifest)?)
        .map_err(|_| "The question could not be encoded.")?;
    let mut sources = HashMap::new();
    let mut hints = Vec::with_capacity(ranges.len());
    for range in ranges {
        if !sources.contains_key(&range.path) {
            sources.insert(
                range.path.clone(),
                context::source(storage, manifest, &range.path)?,
            );
        }
        let source = &sources[&range.path];
        if context::range_from_source(source, range.start_byte, range.end_byte)?.range != *range
            || !input.references.iter().any(|reference| {
                reference.path == range.path
                    && reference.sha256.as_ref() == Some(&range.source_sha256)
                    && reference.from.map(u64::from) == Some(range.start_line as u64)
                    && reference.to.map(u64::from) == Some(range.end_line as u64)
            })
        {
            return Err("A guide starting range changed. No investigation was dispatched.".into());
        }
        hints.push(RangeRequest {
            path: range.path.clone(),
            source_sha256: range.source_sha256.clone(),
            start_byte: range.start_byte,
            end_byte: range.end_byte,
        });
    }
    prompt["startingRanges"] = json!(hints);
    prompt["startingRangePolicy"] = json!(POLICY);
    let prompt =
        serde_json::to_string(&prompt).map_err(|_| "The question could not be encoded.")?;
    let schema = schema();
    let instructions = format!("{}\n\n{INSTRUCTIONS}", system_instructions());
    let mut contract = generation_contract(input, &prompt, &schema)?;
    contract["systemInstructions"] = json!(instructions);
    contract["startingRangePolicy"] = json!(POLICY);
    let bytes = serde_json::to_vec(&contract)
        .map_err(|_| "The progressive question contract could not be encoded.")?;
    if bytes.len() > 32000 {
        return Err("The question and references exceed the initial request limit.".into());
    }
    Ok((prompt, schema, context::digest(&bytes), instructions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};
    use std::sync::atomic::AtomicBool;

    fn prepared(f: &Fixture) -> (Input, context::CodeRange) {
        let source = context::source(&f.storage, &f.manifest, SIMPLE_PATH).unwrap();
        let range = context::range_from_source(&source, 0, source.contents.len())
            .unwrap()
            .range;
        let input = Input {
            run_id: Uuid::new_v4().to_string(),
            project_id: f.manifest.summary.project_id.clone(),
            snapshot_id: f.manifest.summary.snapshot_id.clone(),
            grant_id: Uuid::new_v4().to_string(),
            question: "How does greeting create its result?".into(),
            references: vec![Reference {
                path: range.path.clone(),
                sha256: Some(range.source_sha256.clone()),
                from: Some(range.start_line as u32),
                to: Some(range.end_line as u32),
            }],
            model_id: "gpt-6-luna".into(),
            effort: "max".into(),
        };
        (input, range)
    }

    #[test]
    fn unseeded_request_preserves_the_exact_existing_contract() {
        let f = Fixture::new();
        let (input, _) = prepared(&f);
        let previous = super::super::request(&input, &f.manifest).unwrap();
        let (prompt, schema, fingerprint, instructions) =
            request(&f.storage, &input, &f.manifest, &[]).unwrap();
        assert_eq!((prompt, schema, fingerprint), previous);
        assert_eq!(instructions, system_instructions());
    }

    #[test]
    fn seeded_request_contains_only_checked_read_arguments_and_binds_the_policy() {
        let f = Fixture::new();
        let (input, range) = prepared(&f);
        let (prompt, schema, fingerprint, instructions) =
            request(&f.storage, &input, &f.manifest, &[range.clone()]).unwrap();
        let value: Value = serde_json::from_str(&prompt).unwrap();
        assert_eq!(value["startingRangePolicy"], POLICY);
        assert_eq!(
            value["startingRanges"],
            json!([{
                "path":range.path,"sourceSha256":range.source_sha256,
                "startByte":range.start_byte,"endByte":range.end_byte
            }])
        );
        assert!(prompt.len() < 1600);
        for forbidden in [
            SIMPLE_SOURCE,
            "contents",
            "chunkSha256",
            "referenceId",
            "code:1",
        ] {
            assert!(!prompt.contains(forbidden));
        }
        assert!(instructions.contains("without repeating discovery"));
        assert!(instructions.contains("not evidence or a source selection"));
        assert!(instructions.contains("this investigation's own read_code"));
        let mut contract = generation_contract(&input, &prompt, &schema).unwrap();
        contract["systemInstructions"] = json!(instructions);
        contract["startingRangePolicy"] = json!(POLICY);
        assert_eq!(
            fingerprint,
            context::digest(&serde_json::to_vec(&contract).unwrap())
        );
        assert_eq!(contract["policy"], crate::evidence_ledger::POLICY);
        assert_eq!(contract["sectionAttempts"], 8);
        assert_eq!(contract["uniqueSections"], 6);
        assert_eq!(
            contract["tools"],
            serde_json::to_value(context_mcp::tools_for(true)).unwrap()
        );
        assert_ne!(
            fingerprint,
            super::super::request(&input, &f.manifest).unwrap().2
        );
    }

    #[test]
    fn seeded_requests_reject_unmatched_forged_foreign_or_oversized_ranges() {
        let f = Fixture::new();
        let (input, range) = prepared(&f);
        for field in [
            "path", "source", "snapshot", "chunk", "start", "end", "line",
        ] {
            let mut changed = range.clone();
            match field {
                "path" => changed.path = "../live.ts".into(),
                "source" => changed.source_sha256 = "f".repeat(64),
                "snapshot" => changed.snapshot_id = "another-snapshot".into(),
                "chunk" => changed.chunk_sha256 = "e".repeat(64),
                "start" => changed.start_byte = changed.end_byte,
                "end" => changed.end_byte += 1,
                "line" => changed.start_line += 1,
                _ => unreachable!(),
            }
            assert!(
                request(&f.storage, &input, &f.manifest, &[changed]).is_err(),
                "{field}"
            );
        }
        assert!(request(&f.storage, &input, &f.manifest, &vec![range.clone(); 5]).is_err());
        let mut missing = input;
        missing.references.clear();
        assert!(request(&f.storage, &missing, &f.manifest, &[range]).is_err());
    }

    #[test]
    fn unicode_boundaries_and_changed_captured_bytes_are_rechecked() {
        let mut f = Fixture::new();
        let text = format!("{SIMPLE_SOURCE}\r\n// A greeting for café 🦀\r\n");
        f.recapture_with_source(SIMPLE_PATH, &text);
        let (input, range) = prepared(&f);
        assert!(request(&f.storage, &input, &f.manifest, &[range.clone()]).is_ok());
        let mut invalid = range.clone();
        invalid.end_byte = text.find('🦀').unwrap() + 1;
        assert!(request(&f.storage, &input, &f.manifest, &[invalid]).is_err());
        f.recapture_with_source(SIMPLE_PATH, &text.replace("greeting", "salutation"));
        assert!(request(&f.storage, &input, &f.manifest, &[range]).is_err());
    }

    #[test]
    fn starting_hints_are_not_evidence_until_an_owned_range_read_is_delivered() {
        use crate::{
            codex_identity::CodexReviewRuntime,
            evidence_ledger::{Context, Ledger, Tool},
        };
        let f = Fixture::new();
        let (mut input, range) = prepared(&f);
        let runtime = CodexReviewRuntime {
            connection_id: Uuid::new_v4().to_string(),
            executable_path: "/usr/local/bin/codex".into(),
            executable_sha256: "a".repeat(64),
            account_identity_sha256: "b".repeat(64),
            account_revision: 1,
        };
        input.grant_id = workspace::save_retrieval_grant(
            &f.storage,
            &input.project_id,
            &input.snapshot_id,
            &runtime,
        )
        .unwrap();
        let (prompt, _, _, _) = request(&f.storage, &input, &f.manifest, &[range]).unwrap();
        let value: Value = serde_json::from_str(&prompt).unwrap();
        let ranges: Vec<RangeRequest> =
            serde_json::from_value(value["startingRanges"].clone()).unwrap();
        let mut ledger = Ledger::new_progressive(
            &f.storage,
            &input.run_id,
            &f.manifest,
            &input.grant_id,
            &runtime,
        )
        .unwrap();
        assert!(ledger.admitted_ranges().is_empty());
        let cancelled = AtomicBool::new(false);
        let call = ledger
            .begin_call(
                Tool::Read,
                &"c".repeat(64),
                &f.manifest,
                &runtime,
                &cancelled,
            )
            .unwrap();
        let mut delivered = Value::Null;
        ledger
            .deliver(
                call,
                Context::Read {},
                &ranges,
                &f.manifest,
                &runtime,
                &cancelled,
                |bytes| {
                    delivered = serde_json::from_slice(bytes).unwrap();
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(delivered["added"][0]["referenceId"], "code:1");
        assert_eq!(delivered["added"][0]["citeable"], true);
        assert!(!ledger.admitted_ranges().is_empty());
    }
}
