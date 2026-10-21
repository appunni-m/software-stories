use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};

mod narration;

fn setup(f: &Fixture) -> (Ledger, CodexReviewRuntime, AtomicBool) {
    let runtime = CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        account_identity_sha256: "b".repeat(64),
        account_revision: 1,
    };
    let grant = workspace::save_retrieval_grant(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
        &runtime,
    )
    .unwrap();
    let ledger = Ledger::new_progressive(
        &f.storage,
        &Uuid::new_v4().to_string(),
        &f.manifest,
        &grant,
        &runtime,
        "gpt-6-luna",
        "max",
    )
    .unwrap();
    (ledger, runtime, AtomicBool::new(false))
}

fn read(
    ledger: &mut Ledger,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) -> String {
    let call = ledger
        .begin_call(Tool::Read, &"c".repeat(64), &f.manifest, runtime, cancel)
        .unwrap();
    let source = f
        .manifest
        .files
        .iter()
        .find(|file| file.path == SIMPLE_PATH)
        .unwrap();
    let mut result = Value::Null;
    ledger
        .deliver(
            call,
            Context::Read {},
            &[RangeRequest {
                path: SIMPLE_PATH.into(),
                source_sha256: source.sha256.clone(),
                start_byte: 0,
                end_byte: SIMPLE_SOURCE.len(),
            }],
            &f.manifest,
            runtime,
            cancel,
            |bytes| {
                result = serde_json::from_slice(bytes).unwrap();
                Ok(())
            },
        )
        .unwrap();
    result["added"][0]["referenceId"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn start(
    ledger: &Ledger,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) -> ActiveAnswer {
    ledger
        .start_narration_answer_sections(
            "How is the greeting created?",
            &"d".repeat(64),
            &f.manifest,
            runtime,
            cancel,
        )
        .unwrap()
}

fn publish(
    answer: &mut ActiveAnswer,
    ledger: &Ledger,
    raw: &Value,
    f: &Fixture,
    runtime: &CodexReviewRuntime,
    cancel: &AtomicBool,
) {
    let prepared = answer
        .prepare(ledger, &raw.to_string(), &f.manifest, runtime, cancel)
        .unwrap();
    assert!(prepared.findings.is_empty(), "{:?}", prepared.findings);
    answer
        .commit(ledger, prepared, &f.manifest, runtime, cancel)
        .unwrap();
}

fn validation_issue(code: chapters::ChapterIssueCode) -> chapters::ChapterValidationIssue {
    chapters::ChapterValidationIssue {
        code,
        message: "PRIVATE_SOURCE_DETAILS".into(),
        path: Some("private/source/path.rs".into()),
        section_id: Some("section:private".into()),
        claim_id: Some("claim:private".into()),
        anchor_id: Some("anchor:private".into()),
        hotspot_id: Some("hotspot:private".into()),
        visual_item_id: Some("visual:private".into()),
    }
}

fn validation_report(
    issues: Vec<chapters::ChapterValidationIssue>,
) -> chapters::ChapterValidationReport {
    chapters::ChapterValidationReport {
        schema_version: chapters::CHAPTER_SCHEMA_VERSION,
        chapter_id: "chapter:test".into(),
        revision_id: "revision:test".into(),
        snapshot_id: "snapshot:test".into(),
        status: chapters::ValidationReportStatus::Invalid,
        issues,
    }
}

#[test]
fn chapter_validation_feedback_is_specific_deduplicated_and_source_free() {
    let report = validation_report(vec![
        validation_issue(chapters::ChapterIssueCode::InvalidVisual),
        validation_issue(chapters::ChapterIssueCode::SourceUnavailable),
        validation_issue(chapters::ChapterIssueCode::InvalidVisual),
    ]);

    let findings = chapter_validation_findings(&report);
    assert_eq!(
        findings,
        vec![
            json!({"location":"/section","code":"chapterInvalidVisual"}),
            json!({"location":"/section","code":"chapterSourceUnavailable"}),
        ]
    );
    assert!(valid_findings(&findings));
    let serialized = serde_json::to_string(&findings).unwrap();
    assert!(!serialized.contains("PRIVATE_SOURCE_DETAILS"));
    assert!(!serialized.contains("private/source/path.rs"));
    assert!(!serialized.contains("section:private"));
}

#[test]
fn chapter_validation_feedback_is_bounded_and_rejects_unknown_codes() {
    let codes = [
        chapters::ChapterIssueCode::UnsupportedSchemaVersion,
        chapters::ChapterIssueCode::InvalidId,
        chapters::ChapterIssueCode::InvalidRevision,
        chapters::ChapterIssueCode::InvalidTarget,
        chapters::ChapterIssueCode::InvalidSectionOrder,
        chapters::ChapterIssueCode::DuplicateId,
        chapters::ChapterIssueCode::InvalidClaim,
        chapters::ChapterIssueCode::WrongSnapshot,
        chapters::ChapterIssueCode::MissingPath,
        chapters::ChapterIssueCode::RangeMismatch,
        chapters::ChapterIssueCode::HashMismatch,
        chapters::ChapterIssueCode::ExcerptMismatch,
        chapters::ChapterIssueCode::AmbiguousRemap,
        chapters::ChapterIssueCode::MissingSymbol,
        chapters::ChapterIssueCode::InvalidDependency,
        chapters::ChapterIssueCode::InvalidDestination,
        chapters::ChapterIssueCode::InvalidVisual,
        chapters::ChapterIssueCode::IndexUnavailable,
        chapters::ChapterIssueCode::SourceUnavailable,
    ];
    let findings = chapter_validation_findings(&validation_report(
        codes.into_iter().map(validation_issue).collect(),
    ));
    assert_eq!(findings.len(), MAX_CHAPTER_VALIDATION_FEEDBACK);
    assert!(valid_findings(&findings));
    assert!(!valid_findings(&[json!({
        "location":"/section",
        "code":"chapterArbitrary"
    })]));
    assert_eq!(
        chapter_validation_findings(&validation_report(vec![])),
        finding("sectionValidation")
    );
}

#[test]
fn current_answer_publication_replays_schema_5_with_authored_narration() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start(&ledger, &f, &runtime, &cancel);
    let proposal = authoring::tests::narrated_proposal(&reference, "entry", 0);
    publish(&mut answer, &ledger, &proposal, &f, &runtime, &cancel);

    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.schema_version, NARRATION_ANSWER_VERSION);
    assert_eq!(saved.sections.len(), 1);
    assert_eq!(
        saved.sections[0].checkpoint_version,
        NARRATION_CHECKPOINT_VERSION
    );
    assert_eq!(
        saved.sections[0].checkpoint.schema_version,
        LEDGER_SCHEMA_VERSION
    );
    let revision = &saved.sections[0].preview.revision;
    assert_eq!(
        revision.schema_version,
        crate::chapters::CHAPTER_SCHEMA_VERSION
    );
    assert!(revision.sections[0].visual.is_none());
    assert_eq!(
        revision.sections[0].spoken_text,
        proposal["section"]["claims"][0]["scene"]["narration"]
    );
    assert_eq!(saved.attempts[0].arguments, proposal.to_string());
}

#[test]
fn guide_slide_jobs_can_publish_one_section_and_safely_reject_a_second() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start(&ledger, &f, &runtime, &cancel);
    let first = authoring::tests::narrated_proposal(&reference, "entry", 0);
    let prepared = answer
        .prepare_with_section_limit(
            &ledger,
            &first.to_string(),
            &f.manifest,
            &runtime,
            &cancel,
            1,
        )
        .unwrap();
    assert!(prepared.findings.is_empty());
    answer
        .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
        .unwrap();
    assert_eq!(answer.next_new_section_order_with_limit(1), None);

    let second = authoring::tests::narrated_proposal(&reference, "next-slide", 1);
    let prepared = answer
        .prepare_with_section_limit(
            &ledger,
            &second.to_string(),
            &f.manifest,
            &runtime,
            &cancel,
            1,
        )
        .unwrap();
    assert_eq!(prepared.findings, finding("guideSlideComplete"));
    assert!(prepared.generation_id.is_none());
    answer
        .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
        .unwrap();

    let saved = inspect(&f.storage, &ledger.record.binding.operation_id).unwrap();
    assert_eq!(saved.sections.len(), 1);
    assert_eq!(saved.sections[0].key, "entry");
    assert_eq!(saved.attempts.len(), 2);
    assert_eq!(saved.attempts[1].findings, finding("guideSlideComplete"));
}

#[test]
fn old_proposal_answer_checkpoint_and_ledger_versions_are_rejected() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start(&ledger, &f, &runtime, &cancel);

    for version in [
        "answerSectionV1",
        "answerSectionV2",
        "answerSectionV3",
        "answerSectionV4",
        "answerSectionV5",
    ] {
        let mut proposal = authoring::tests::narrated_proposal(&reference, "entry", 0);
        proposal["proposalVersion"] = json!(version);
        let prepared = answer
            .prepare(
                &ledger,
                &proposal.to_string(),
                &f.manifest,
                &runtime,
                &cancel,
            )
            .unwrap();
        assert_eq!(
            prepared.findings,
            vec![json!({"location":"/proposalVersion","code":"unsupportedProposalVersion"})]
        );
        answer
            .commit(&ledger, prepared, &f.manifest, &runtime, &cancel)
            .unwrap();
    }

    assert!(
        (1..NARRATION_CHECKPOINT_VERSION)
            .all(|version| { !answer.value.supports_checkpoint(version) })
    );
    assert!(
        answer
            .value
            .supports_checkpoint(NARRATION_CHECKPOINT_VERSION)
    );
    assert!(answer.value.sections.is_empty());

    for version in [1, 2] {
        let mut record = ledger.record.clone();
        record.schema_version = version;
        assert!(validate_record(&record, &record.binding.operation_id).is_err());
    }
}

#[test]
fn only_answer_manifest_v2_can_complete_current_publications() {
    let f = Fixture::new();
    let (mut ledger, runtime, cancel) = setup(&f);
    let reference = read(&mut ledger, &f, &runtime, &cancel);
    let mut answer = start(&ledger, &f, &runtime, &cancel);
    publish(
        &mut answer,
        &ledger,
        &authoring::tests::narrated_proposal(&reference, "entry", 0),
        &f,
        &runtime,
        &cancel,
    );

    assert!(
        answer
            .finish_manifest(
                &json!({"manifestVersion":"answerManifestV1","sections":[],"incompleteTopics":[]})
                    .to_string(),
                false,
            )
            .is_err()
    );
    answer
        .finish_manifest(
            &json!({"manifestVersion":NATIVE_MANIFEST_VERSION,"incompleteTopics":[]}).to_string(),
            false,
        )
        .unwrap();
    assert_eq!(
        inspect(&f.storage, &ledger.record.binding.operation_id)
            .unwrap()
            .outcome,
        Outcome::Complete
    );
}

#[test]
fn saved_answer_schema_version_6_is_not_replayed() {
    let f = Fixture::new();
    let (ledger, runtime, cancel) = setup(&f);
    let answer = start(&ledger, &f, &runtime, &cancel);
    let mut value: Value = serde_json::from_slice(&fs::read(&answer.path).unwrap()).unwrap();
    value["schemaVersion"] = json!(6);
    fs::write(&answer.path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(inspect(&f.storage, &ledger.record.binding.operation_id).is_err());
}

#[test]
fn current_answer_listing_skips_unsupported_old_records_without_modifying_them() {
    let f = Fixture::new();
    let (ledger, runtime, cancel) = setup(&f);
    let current = start(&ledger, &f, &runtime, &cancel);
    let old_run = Uuid::new_v4().to_string();
    let old_path = f
        .storage
        .join("answer-sections")
        .join(format!("{old_run}.json"));
    let old_bytes = br#"{"schemaVersion":1,"unsupported":"preserve verbatim"}"#;
    fs::write(&old_path, old_bytes).unwrap();

    let listed = reading::list(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
    )
    .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].run_id, ledger.record.binding.operation_id);
    assert_eq!(listed[0].run_id, current.value.binding.operation_id);
    assert_eq!(fs::read(&old_path).unwrap(), old_bytes);
    assert!(inspect(&f.storage, &old_run).is_err());
}

#[test]
fn legacy_answers_count_toward_the_total_limit_and_remain_unchanged_on_check() {
    let f = Fixture::new();
    let (ledger, runtime, cancel) = setup(&f);
    let current = start(&ledger, &f, &runtime, &cancel);
    let mut legacy = current.value.clone();
    legacy.schema_version = 1;

    let directory = f.storage.join("answer-sections");
    let mut originals = Vec::new();
    for _ in 0..MAX_ANSWER_RECORDS - 1 {
        let run = Uuid::new_v4().to_string();
        legacy.binding.operation_id = run.clone();
        let path = directory.join(format!("{run}.json"));
        let bytes = encode_answer(&legacy).unwrap();
        fs::write(&path, &bytes).unwrap();
        originals.push((path, bytes));
    }

    capacity::check(&f.storage, None, None).unwrap();
    let mut next = current.value.binding.clone();
    next.operation_id = Uuid::new_v4().to_string();
    let error = capacity::check(&f.storage, None, Some(&next)).unwrap_err();
    assert!(error.contains("1,000 answer records"), "{error}");
    for (path, bytes) in originals {
        assert_eq!(fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn newer_answer_schema_blocks_current_capacity_without_rewriting_it() {
    let f = Fixture::new();
    let (ledger, runtime, cancel) = setup(&f);
    let current = start(&ledger, &f, &runtime, &cancel);
    let mut future = current.value.clone();
    future.schema_version = NARRATION_ANSWER_VERSION + 1;
    let bytes = encode_answer(&future).unwrap();
    fs::write(&current.path, &bytes).unwrap();

    let error = capacity::check(&f.storage, None, None).unwrap_err();
    assert!(error.contains("newer saved answer"), "{error}");
    assert_eq!(fs::read(current.path).unwrap(), bytes);
}

#[test]
fn answer_manager_lists_verified_records_when_another_current_record_is_unavailable() {
    let f = Fixture::new();
    let (ledger, runtime, cancel) = setup(&f);
    let current = start(&ledger, &f, &runtime, &cancel);
    let invalid_run = Uuid::new_v4().to_string();
    let invalid_path = f
        .storage
        .join("answer-sections")
        .join(format!("{invalid_run}.json"));
    let invalid_bytes = serde_json::to_vec(&json!({
        "schemaVersion": NARRATION_ANSWER_VERSION,
        "unsupported": "preserve verbatim"
    }))
    .unwrap();
    fs::write(&invalid_path, &invalid_bytes).unwrap();

    let listing = reading::list_for_management(&f.storage).unwrap();
    assert_eq!(listing.records.len(), 1);
    assert_eq!(
        listing.records[0].run_id,
        current.value.binding.operation_id
    );
    assert_eq!(listing.unavailable_count, 1);
    let workspace_answers = reading::list(
        &f.storage,
        &f.manifest.summary.project_id,
        &f.manifest.summary.snapshot_id,
    )
    .unwrap();
    assert_eq!(workspace_answers.len(), 1);
    assert_eq!(
        workspace_answers[0].run_id, current.value.binding.operation_id,
        "an unavailable receipt must not hide another independently verified answer"
    );
    assert_eq!(fs::read(invalid_path).unwrap(), invalid_bytes);
}
