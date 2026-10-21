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
    guide_intent: Option<&GuideIntent>,
) -> Result<(String, Value, String, String), String> {
    request_for(
        storage,
        input,
        manifest,
        ranges,
        guide_intent,
        RunDeadline::TenMinutes,
    )
}

pub(super) fn request_for(
    storage: &Path,
    input: &Input,
    manifest: &snapshot::SnapshotManifest,
    ranges: &[context::CodeRange],
    guide_intent: Option<&GuideIntent>,
    run_deadline: RunDeadline,
) -> Result<(String, Value, String, String), String> {
    if let Some(intent) = guide_intent {
        intent.validate()?;
        if intent
            .opening_continuation
            .as_ref()
            .is_some_and(|opening| opening.previous_run_id == input.run_id)
        {
            return Err("The opening continuation must own a new investigation.".into());
        }
    }
    if ranges.is_empty() && guide_intent.is_none() {
        // Keep an unseeded question on the same current request contract.
        let (prompt, schema, fingerprint) = super::request_for(input, manifest, run_deadline)?;
        return Ok((
            prompt,
            schema,
            fingerprint,
            system_instructions_for(run_deadline),
        ));
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
    if !ranges.is_empty() {
        prompt["startingRanges"] = json!(hints);
        prompt["startingRangePolicy"] = json!(POLICY);
    }
    if let Some(intent) = guide_intent {
        prompt["guideContext"] = json!(intent);
        prompt["guideContextPolicy"] = json!(guide_intent::POLICY);
        if intent.opening_continuation.is_some() {
            prompt["openingContinuationPolicy"] = json!(guide_intent::OPENING_POLICY);
        }
    }
    let prompt =
        serde_json::to_string(&prompt).map_err(|_| "The question could not be encoded.")?;
    let schema = schema();
    let mut instructions = system_instructions_for(run_deadline);
    if !ranges.is_empty() {
        instructions.push_str(&format!("\n\n{INSTRUCTIONS}"));
    }
    if guide_intent.is_some() {
        instructions.push_str(&format!("\n\n{}", guide_intent::INSTRUCTIONS));
    }
    if guide_intent.is_some_and(|intent| intent.unit_key == "opening") {
        instructions.push_str(&format!("\n\n{}", guide_intent::OPENING_INSTRUCTIONS));
    }
    if guide_intent.is_some_and(|intent| intent.opening_continuation.is_some()) {
        instructions.push_str(&format!(
            "\n\n{}",
            guide_intent::OPENING_CONTINUATION_INSTRUCTIONS
        ));
    }
    let mut contract = generation_contract_for(input, &prompt, &schema, run_deadline)?;
    contract["systemInstructions"] = json!(instructions);
    if !ranges.is_empty() {
        contract["startingRangePolicy"] = json!(POLICY);
    }
    if guide_intent.is_some() {
        contract["guideContextPolicy"] = json!(guide_intent::POLICY);
    }
    if guide_intent.is_some_and(|intent| intent.unit_key == "opening") {
        contract["openingSlidePolicy"] = json!("independent-outline-first-opening-v1");
    }
    if guide_intent.is_some_and(|intent| intent.opening_continuation.is_some()) {
        contract["openingContinuationPolicy"] = json!(guide_intent::OPENING_POLICY);
    }
    let bytes = serde_json::to_vec(&contract)
        .map_err(|_| "The progressive question contract could not be encoded.")?;
    // A guide carries its original question separately from the independent
    // unit/continuation question. Keep both bounded and exact. This changes no
    // source/tool/proposal/run policy and sends no additional source bytes.
    // The fingerprint also encodes the already-serialized prompt as a JSON
    // string, so quotes are escaped twice inside this bounded local contract.
    let limit = if guide_intent.is_some() {
        128 * 1024
    } else {
        super::INITIAL_REQUEST_LIMIT_BYTES
    };
    if bytes.len() > limit {
        return Err(format!(
            "The question and references use {} bytes, exceeding the {limit}-byte initial request limit.",
            bytes.len()
        ));
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
            request(&f.storage, &input, &f.manifest, &[], None).unwrap();
        assert_eq!((prompt, schema, fingerprint), previous);
        assert_eq!(instructions, system_instructions());
    }

    fn guide_intent() -> GuideIntent {
        GuideIntent {
            guide_id: Uuid::new_v4().to_string(),
            question: "How does a requested name become output, including important failure paths?"
                .into(),
            unit_key: "greeting-input".into(),
            unit_number: 1,
            unit_count: 2,
            research_boundaries: vec!["Runtime execution was not observed.".into()],
            opening_continuation: None,
        }
    }

    #[test]
    fn opening_follow_up_binds_both_questions_gaps_owner_and_original_budgets() {
        let f = Fixture::new();
        let (input, _) = prepared(&f);
        let mut intent = guide_intent();
        intent.unit_key = "opening".into();
        intent.unit_number = 0;
        intent.opening_continuation = Some(OpeningContinuation {
            previous_run_id: Uuid::new_v4().to_string(),
            incomplete_topics: vec![
                "The first guard's false alternative remains unexplored.".into(),
            ],
        });
        let (prompt, schema, fingerprint, instructions) =
            request(&f.storage, &input, &f.manifest, &[], Some(&intent)).unwrap();
        let value: Value = serde_json::from_str(&prompt).unwrap();
        assert_eq!(value["question"], input.question);
        assert_eq!(value["guideContext"], json!(intent));
        assert_eq!(
            value["openingContinuationPolicy"],
            guide_intent::OPENING_POLICY
        );
        assert!(instructions.contains(guide_intent::OPENING_CONTINUATION_INSTRUCTIONS));
        assert!(!prompt.contains(SIMPLE_SOURCE));
        assert!(value.get("startingRanges").is_none());
        assert!(schema["properties"].get("outline").is_none());
        let mut contract = generation_contract(&input, &prompt, &schema).unwrap();
        contract["systemInstructions"] = json!(instructions);
        contract["guideContextPolicy"] = json!(guide_intent::POLICY);
        contract["openingSlidePolicy"] = json!("independent-outline-first-opening-v1");
        contract["openingContinuationPolicy"] = json!(guide_intent::OPENING_POLICY);
        assert_eq!(
            fingerprint,
            context::digest(&serde_json::to_vec(&contract).unwrap())
        );
        assert_eq!(contract["sectionAttempts"], 8);
        assert_eq!(contract["uniqueSections"], 6);
        assert_eq!(
            contract["tools"],
            serde_json::to_value(context_mcp::tools()).unwrap()
        );
        for field in ["owner", "gaps"] {
            let mut changed = intent.clone();
            let opening = changed.opening_continuation.as_mut().unwrap();
            match field {
                "owner" => opening.previous_run_id = Uuid::new_v4().to_string(),
                "gaps" => opening
                    .incomplete_topics
                    .push("The returned error is still unknown.".into()),
                _ => unreachable!(),
            }
            assert_ne!(
                request(&f.storage, &input, &f.manifest, &[], Some(&changed))
                    .unwrap()
                    .2,
                fingerprint
            );
        }
        assert!(!f.storage.join("answer-sections").exists());
    }

    #[test]
    fn invalid_opening_follow_up_cannot_borrow_an_owner_or_expand_context_bounds() {
        let f = Fixture::new();
        let (input, _) = prepared(&f);
        let mut original = guide_intent();
        original.unit_key = "opening".into();
        original.unit_number = 0;
        original.opening_continuation = Some(OpeningContinuation {
            previous_run_id: Uuid::new_v4().to_string(),
            incomplete_topics: vec![],
        });
        for field in [
            "owner", "number", "key", "missing", "count", "control", "long",
        ] {
            let mut changed = original.clone();
            match field {
                "owner" => {
                    changed
                        .opening_continuation
                        .as_mut()
                        .unwrap()
                        .previous_run_id = input.run_id.clone()
                }
                "number" => changed.unit_number = 1,
                "key" => changed.unit_key = "unit".into(),
                "missing" => changed.opening_continuation = None,
                "count" => {
                    changed
                        .opening_continuation
                        .as_mut()
                        .unwrap()
                        .incomplete_topics = vec!["gap".into(); 7]
                }
                "control" => {
                    changed
                        .opening_continuation
                        .as_mut()
                        .unwrap()
                        .incomplete_topics = vec!["gap\nother".into()]
                }
                "long" => {
                    changed
                        .opening_continuation
                        .as_mut()
                        .unwrap()
                        .incomplete_topics = vec!["é".repeat(501)]
                }
                _ => unreachable!(),
            }
            assert!(
                request(&f.storage, &input, &f.manifest, &[], Some(&changed)).is_err(),
                "{field}"
            );
        }
        assert!(!f.storage.join("answer-sections").exists());
        assert!(!f.storage.join("evidence-ledgers").exists());
    }

    #[test]
    fn guide_request_preserves_original_intent_and_boundaries_as_context_not_evidence() {
        let f = Fixture::new();
        let (input, range) = prepared(&f);
        let intent = guide_intent();
        let (prompt, schema, fingerprint, instructions) = request(
            &f.storage,
            &input,
            &f.manifest,
            &[range.clone()],
            Some(&intent),
        )
        .unwrap();
        let value: Value = serde_json::from_str(&prompt).unwrap();
        assert_eq!(value["question"], input.question);
        assert_eq!(value["guideContext"], json!(intent));
        assert_ne!(value["question"], value["guideContext"]["question"]);
        let ordinary = request(&f.storage, &input, &f.manifest, &[range], None).unwrap();
        assert_ne!(fingerprint, ordinary.2);
        assert!(instructions.contains("guideContext"));
        assert!(!prompt.contains(SIMPLE_SOURCE));
        for forbidden in ["referenceId", "code:1", "quote", "contents"] {
            assert!(!value["guideContext"].to_string().contains(forbidden));
        }
        let mut contract = generation_contract(&input, &prompt, &schema).unwrap();
        contract["systemInstructions"] = json!(instructions);
        contract["startingRangePolicy"] = json!(POLICY);
        contract["guideContextPolicy"] = json!(guide_intent::POLICY);
        assert_eq!(
            fingerprint,
            context::digest(&serde_json::to_vec(&contract).unwrap())
        );
        assert_eq!(value["guideContextPolicy"], guide_intent::POLICY);
        assert_eq!(contract["sectionAttempts"], 8);
        assert_eq!(contract["uniqueSections"], 6);
        assert_eq!(
            contract["tools"],
            serde_json::to_value(context_mcp::tools()).unwrap()
        );
        assert!(!f.storage.join("answer-sections").exists());
        assert!(!f.storage.join("evidence-ledgers").exists());
    }

    #[test]
    fn every_guide_intent_field_is_bound_without_rewriting_unit_or_original_questions() {
        let f = Fixture::new();
        let (input, range) = prepared(&f);
        let original = guide_intent();
        let ranges = [range];
        let previous = request(&f.storage, &input, &f.manifest, &ranges, Some(&original)).unwrap();
        for field in ["guide", "question", "key", "number", "count", "boundaries"] {
            let mut changed = original.clone();
            match field {
                "guide" => changed.guide_id = Uuid::new_v4().to_string(),
                "question" => changed
                    .question
                    .push_str(" What happens on malformed input?"),
                "key" => changed.unit_key.push_str("-revised"),
                "number" => changed.unit_number = 2,
                "count" => changed.unit_count = 3,
                "boundaries" => changed
                    .research_boundaries
                    .push("No downstream caller was executed.".into()),
                _ => unreachable!(),
            }
            let next = request(&f.storage, &input, &f.manifest, &ranges, Some(&changed)).unwrap();
            assert_ne!(previous.2, next.2, "{field}");
            let prompt: Value = serde_json::from_str(&next.0).unwrap();
            assert_eq!(prompt["question"], input.question);
            assert_eq!(prompt["guideContext"]["question"], changed.question);
        }
    }

    #[test]
    fn invalid_guide_intent_is_rejected_before_source_retrieval_or_inference() {
        let f = Fixture::new();
        let (input, _) = prepared(&f);
        let original = guide_intent();
        for field in [
            "guide",
            "empty",
            "long",
            "control",
            "key",
            "number",
            "count",
            "boundaries",
            "boundary",
        ] {
            let mut changed = original.clone();
            match field {
                "guide" => changed.guide_id = "not-a-guide".into(),
                "empty" => changed.question = " ".into(),
                "long" => changed.question = "é".repeat(6001),
                "control" => changed.question.push('\n'),
                "key" => changed.unit_key = "bad_key".into(),
                "number" => changed.unit_number = 0,
                "count" => changed.unit_count = 8,
                "boundaries" => changed.research_boundaries = vec!["boundary".into(); 9],
                "boundary" => changed.research_boundaries = vec!["é".repeat(121)],
                _ => unreachable!(),
            }
            let error = request(&f.storage, &input, &f.manifest, &[], Some(&changed)).unwrap_err();
            assert!(
                error.contains("learning intent is invalid"),
                "{field}: {error}"
            );
        }
        assert!(!f.storage.join("answer-sections").exists());
        assert!(!f.storage.join("evidence-ledgers").exists());
    }

    #[test]
    fn a_full_size_original_question_survives_separately_from_a_full_size_unit() {
        let f = Fixture::new();
        let (mut input, range) = prepared(&f);
        let mut intent = guide_intent();
        intent.question = "\"é".repeat(4000);
        input.question = "\"x".repeat(6000);
        assert_eq!(intent.question.len(), 12000);
        assert_eq!(input.question.len(), 12000);
        let (prompt, _, _, _) =
            request(&f.storage, &input, &f.manifest, &[range], Some(&intent)).unwrap();
        let value: Value = serde_json::from_str(&prompt).unwrap();
        assert_eq!(value["question"], input.question);
        assert_eq!(value["guideContext"]["question"], intent.question);
        let without_ranges = request(&f.storage, &input, &f.manifest, &[], Some(&intent)).unwrap();
        let value: Value = serde_json::from_str(&without_ranges.0).unwrap();
        assert_eq!(value["guideContext"], json!(intent));
        assert!(value.get("startingRanges").is_none());
    }

    #[test]
    fn seeded_request_contains_only_checked_read_arguments_and_binds_the_policy() {
        let f = Fixture::new();
        let (input, range) = prepared(&f);
        let (prompt, schema, fingerprint, instructions) =
            request(&f.storage, &input, &f.manifest, &[range.clone()], None).unwrap();
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
            serde_json::to_value(context_mcp::tools()).unwrap()
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
                request(&f.storage, &input, &f.manifest, &[changed], None).is_err(),
                "{field}"
            );
        }
        assert!(
            request(
                &f.storage,
                &input,
                &f.manifest,
                &vec![range.clone(); 5],
                None
            )
            .is_err()
        );
        let mut missing = input;
        missing.references.clear();
        assert!(request(&f.storage, &missing, &f.manifest, &[range], None).is_err());
    }

    #[test]
    fn unicode_boundaries_and_changed_captured_bytes_are_rechecked() {
        let mut f = Fixture::new();
        let text = format!("{SIMPLE_SOURCE}\r\n// A greeting for café 🦀\r\n");
        f.recapture_with_source(SIMPLE_PATH, &text);
        let (input, range) = prepared(&f);
        assert!(request(&f.storage, &input, &f.manifest, &[range.clone()], None).is_ok());
        let mut invalid = range.clone();
        invalid.end_byte = text.find('🦀').unwrap() + 1;
        assert!(request(&f.storage, &input, &f.manifest, &[invalid], None).is_err());
        f.recapture_with_source(SIMPLE_PATH, &text.replace("greeting", "salutation"));
        assert!(request(&f.storage, &input, &f.manifest, &[range], None).is_err());
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
        let (prompt, _, _, _) = request(
            &f.storage,
            &input,
            &f.manifest,
            &[range],
            Some(&guide_intent()),
        )
        .unwrap();
        let value: Value = serde_json::from_str(&prompt).unwrap();
        let ranges: Vec<RangeRequest> =
            serde_json::from_value(value["startingRanges"].clone()).unwrap();
        let mut ledger = Ledger::new_progressive(
            &f.storage,
            &input.run_id,
            &f.manifest,
            &input.grant_id,
            &runtime,
            &input.model_id,
            &input.effort,
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
