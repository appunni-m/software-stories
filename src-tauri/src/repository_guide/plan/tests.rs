use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};

const GUIDE_QUESTION: &str = "How does a greeting turn an incoming name into output?";

fn proposal() -> Value {
    let mut proposal = json!({"citationVersion":"nativeEvidenceV2","planVersion":"repositoryGuidePlanV1","title":"Follow the greeting journey",
    "sections":[
        {"key":"opening","question":"Opening question supplied by the app?","prerequisites":[],"claims":[{"evidence":[{"referenceId":"code:1"}]}]},
        {"key":"input","question":"How does the incoming name enter the greeting?","prerequisites":[],"claims":[{"evidence":[{"referenceId":"code:2"}]}]},
        {"key":"output","question":"How does the greeting turn into the returned output?","prerequisites":["input"],"claims":[{"evidence":[{"referenceId":"code:3"}]}]}
    ],"omissions":["Runtime execution was not observed."]});
    proposal["sections"][0]["question"] = json!(GUIDE_QUESTION);
    proposal
}

fn canonicalize_proposal(value: Value) -> Result<Plan, String> {
    canonicalize(&resolve(value), GUIDE_QUESTION)
}

fn resolve(value: Value) -> native_citations::Resolved {
    let f = Fixture::new();
    let source = context::source(&f.storage, &f.manifest, SIMPLE_PATH).unwrap();
    let first_line_end = SIMPLE_SOURCE.find('\n').unwrap() + 1;
    let second_line_end = first_line_end + SIMPLE_SOURCE[first_line_end..].find('\n').unwrap() + 1;
    let third_line_end = second_line_end + SIMPLE_SOURCE[second_line_end..].find('\n').unwrap() + 1;
    let ranges = [
        context::range_from_source(&source, 0, first_line_end)
            .unwrap()
            .range,
        context::range_from_source(&source, first_line_end, second_line_end)
            .unwrap()
            .range,
        context::range_from_source(&source, second_line_end, third_line_end)
            .unwrap()
            .range,
    ];
    match native_citations::resolve(&value.to_string(), |id| {
        let range = match id {
            "code:1" => &ranges[0],
            "code:2" => &ranges[1],
            "code:3" => &ranges[2],
            _ => return Err("referenceUnavailable"),
        };
        Ok(native_citations::Citation::checked(range, &source).unwrap())
    }) {
        native_citations::Resolution::Ready(r) => r,
        native_citations::Resolution::Invalid(issues) => {
            panic!("fixture references must resolve: {issues:?}")
        }
    }
}
#[test]
fn outline_keeps_research_questions_and_native_ranges_not_unverified_explanations() {
    let result = canonicalize_proposal(proposal()).unwrap();
    assert_eq!(result.units.len(), 2);
    assert_eq!(result.opening_ranges[0].start_line, 1);
    assert_eq!(result.units[0].starting_ranges[0].path, SIMPLE_PATH);
    assert_eq!(result.units[0].starting_ranges[0].start_line, 2);
    assert_ne!(result.opening_ranges, result.units[0].starting_ranges);
    assert_eq!(result.units[1].prerequisites, vec!["input"]);
    let encoded = serde_json::to_string(&result).unwrap();
    assert!(!encoded.contains("quote"));
    assert!(!encoded.contains(SIMPLE_SOURCE));
    let schema = schema();
    assert_eq!(
        schema["properties"]["sections"]["maxItems"],
        MAX_INITIAL_UNITS + 1
    );
    assert_eq!(schema["properties"]["sections"]["minItems"], 1);
    assert_eq!(
        schema["properties"]["sections"]["prefixItems"][0]["properties"]["question"]["minLength"],
        1
    );
    assert_eq!(
        schema["properties"]["sections"]["prefixItems"][0]["properties"]["question"]["maxLength"],
        12000
    );
    assert_eq!(
        schema["properties"]["sections"]["items"]["properties"]["question"]["minLength"],
        16
    );
    assert_eq!(
        schema["properties"]["sections"]["items"]["properties"]["question"]["maxLength"],
        800
    );
    assert!(
        schema["properties"]["sections"]["description"]
            .as_str()
            .unwrap()
            .contains("one question-first opening at index zero")
    );
}

#[test]
fn narrow_question_can_be_complete_with_only_the_question_first_opening() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    let mut candidate = proposal();
    candidate["sections"].as_array_mut().unwrap().truncate(1);
    candidate["omissions"] = json!([]);

    assert!(validator.is_valid(&candidate));
    let plan = canonicalize_proposal(candidate).unwrap();
    assert!(plan.units.is_empty());
    assert!(plan.omissions.is_empty());
}

#[test]
fn opening_question_can_match_short_or_maximum_length_user_input() {
    let validator = jsonschema::validator_for(&schema()).unwrap();
    for opening_question in ["What is X?".to_owned(), "x".repeat(12000)] {
        let mut candidate = proposal();
        candidate["sections"][0]["question"] = json!(opening_question.clone());
        assert!(validator.is_valid(&candidate));
        assert!(canonicalize(&resolve(candidate), &opening_question).is_ok());
    }
}

#[test]
fn initial_outline_accepts_five_detailed_units_and_rejects_overflow() {
    let mut full = proposal();
    for index in 2..MAX_INITIAL_UNITS {
        full["sections"].as_array_mut().unwrap().push(json!({
            "key": format!("surface-{index}"),
            "question": format!("How does caller surface {index} transform its input?"),
            "prerequisites": [],
            "claims": [{"evidence":[{"referenceId":"code:2"}]}]
        }));
    }
    assert_eq!(
        full["sections"].as_array().unwrap().len(),
        MAX_INITIAL_UNITS + 1
    );
    assert_eq!(
        canonicalize_proposal(full.clone()).unwrap().units.len(),
        MAX_INITIAL_UNITS
    );

    full["sections"].as_array_mut().unwrap().push(json!({
        "key": "surface-overflow",
        "question": "How does the unsupported extra input surface behave?",
        "prerequisites": [],
        "claims": [{"evidence":[{"referenceId":"code:2"}]}]
    }));
    assert!(canonicalize_proposal(full).is_err());
}

#[test]
fn saved_outlines_missing_required_opening_ranges_are_rejected() {
    let plan = canonicalize_proposal(proposal()).unwrap();
    let mut stored = serde_json::to_value(plan).unwrap();
    stored.as_object_mut().unwrap().remove("openingRanges");
    assert!(serde_json::from_value::<Plan>(stored).is_err());
}
#[test]
fn outline_rejects_cycles_duplicate_questions_extra_prose_and_unknown_fields() {
    for kind in [
        "cycle",
        "duplicateKey",
        "duplicateQuestion",
        "extraProse",
        "forward",
        "notQuestion",
        "extraRoot",
        "extraClaim",
    ] {
        let mut p = proposal();
        match kind {
            "cycle" => p["sections"][1]["prerequisites"] = json!(["output"]),
            "duplicateKey" => p["sections"][2]["key"] = json!("input"),
            "duplicateQuestion" => {
                p["sections"][2]["question"] = p["sections"][1]["question"].clone()
            }
            "extraProse" => p["sections"][1]["claims"][0]["prose"] = json!("An unsupported fact"),
            "forward" => p["sections"][1]["prerequisites"] = json!(["output"]),
            "notQuestion" => {
                p["sections"][1]["question"] = json!("This is not a research question.")
            }
            "extraRoot" => p["accepted"] = json!(true),
            "extraClaim" => {
                let c = p["sections"][1]["claims"][0].clone();
                p["sections"][1]["claims"].as_array_mut().unwrap().push(c);
            }
            _ => unreachable!(),
        }
        assert!(canonicalize_proposal(p).is_err(), "{kind}");
    }
}

#[test]
fn opening_lead_must_match_app_intent_and_cannot_be_a_follow_on_dependency() {
    let mutations: [fn(&mut Value); 4] = [
        |p: &mut Value| p["sections"][0]["key"] = json!("first-slide"),
        |p: &mut Value| p["sections"][0]["question"] = json!("How does another path start?"),
        |p: &mut Value| p["sections"][0]["prerequisites"] = json!(["input"]),
        |p: &mut Value| p["sections"][1]["prerequisites"] = json!(["opening"]),
    ];
    for mutate in mutations {
        let mut invalid = proposal();
        mutate(&mut invalid);
        assert!(canonicalize(&resolve(invalid), GUIDE_QUESTION).is_err());
    }
}

#[test]
fn outline_rejections_report_fixed_structural_findings() {
    let cases = [
        (
            "invalidKey",
            "Guide section keys must use only lowercase letters, digits, and hyphens.",
        ),
        ("duplicateKey", "Guide section keys must be unique."),
        (
            "notQuestion",
            "Each guide section question must end with a question mark.",
        ),
        (
            "duplicateQuestion",
            "Guide section questions must be distinct.",
        ),
        (
            "selfPrerequisite",
            "The outline refers to a slide before it appears; list only earlier follow-on topics as prerequisites.",
        ),
        (
            "unknownPrerequisite",
            "The outline refers to a slide before it appears; list only earlier follow-on topics as prerequisites.",
        ),
    ];

    for (kind, expected) in cases {
        let mut p = proposal();
        match kind {
            "invalidKey" => p["sections"][1]["key"] = json!("bad_key"),
            "duplicateKey" => p["sections"][2]["key"] = json!("input"),
            "notQuestion" => {
                p["sections"][1]["question"] =
                    json!("How does the incoming name enter the greeting.")
            }
            "duplicateQuestion" => {
                p["sections"][2]["question"] = p["sections"][1]["question"].clone()
            }
            "selfPrerequisite" => p["sections"][1]["prerequisites"] = json!(["input"]),
            "unknownPrerequisite" => p["sections"][2]["prerequisites"] = json!(["missing"]),
            _ => unreachable!(),
        }
        assert_eq!(canonicalize_proposal(p).unwrap_err(), expected, "{kind}");
    }

    let mut repeated_prerequisite = proposal();
    let third = json!({"key":"finish","question":"How does the final value return to its caller?",
        "prerequisites":["input","input"],"claims":[{"evidence":[{"referenceId":"code:1"}]}]});
    repeated_prerequisite["sections"]
        .as_array_mut()
        .unwrap()
        .push(third);
    assert_eq!(
        canonicalize_proposal(repeated_prerequisite).unwrap_err(),
        "Guide section prerequisites must be unique."
    );
}
#[test]
fn unread_references_never_become_starting_paths() {
    assert!(matches!(
        native_citations::resolve(&proposal().to_string(), |_| Err("referenceUnavailable")),
        native_citations::Resolution::Invalid(_)
    ));
}

#[test]
fn plan_only_request_binds_the_whole_question_and_excludes_answer_generation() {
    let f = Fixture::new();
    let mut r = crate::repository_guide::tests::fresh_record(&f);
    r.question =
        "How do café 🦀 bytes reach output, including every requested alternate and failure?"
            .into();
    r.opening.as_mut().unwrap().question = r.question.clone();
    r.opening_context_sha256 = Some(opening::context_sha256(&r).unwrap());
    let (prompt, output, fingerprint, instructions) = request(&r).unwrap();
    let intent: Value = serde_json::from_str(&prompt).unwrap();
    assert_eq!(intent["question"], r.question);
    assert!(intent["guideContext"].get("question").is_none());
    assert!(intent["guideContext"].get("openingQuestion").is_none());
    assert_eq!(intent["guideContext"]["guideId"], r.id);
    assert_eq!(output, schema());
    assert!(output["properties"].get("openingAnswer").is_none());
    assert!(instructions.contains("Create the initial research outline only"));
    assert!(instructions.contains("Do not write narration, explanations, HTML, or slide content"));
    assert!(instructions.contains("the exact question supplied by the app"));
    assert!(
        instructions
            .contains("continue the opening's caller-to-engine path at its next real handoff")
    );
    assert!(
        instructions
            .contains("finish that central path, then cover alternate callers or languages")
    );
    assert!(
        agent_context::RESEARCH_BASE_INSTRUCTIONS.contains("Do not call propose_answer_section")
    );
    assert!(instructions.contains(RESEARCH_INSTRUCTIONS));
    let mut separate_opening = r.clone();
    separate_opening.opening.as_mut().unwrap().question =
        "An older separate opening intent.".into();
    separate_opening.opening_context_sha256 =
        Some(opening::context_sha256(&separate_opening).unwrap());
    assert!(request(&separate_opening).is_err());
    let contract = serde_json::to_value(context_mcp::research_tools()).unwrap();
    assert!(!contract.to_string().contains("propose_answer_section"));
    r.question.push_str(" Preserve downstream ownership.");
    assert!(
        request(&r).is_err(),
        "an unchanged binding cannot admit a changed guide question"
    );
    r.opening.as_mut().unwrap().question = r.question.clone();
    r.opening_context_sha256 = Some(opening::context_sha256(&r).unwrap());
    assert_ne!(request(&r).unwrap().2, fingerprint);
    let mut wrong_opening = r.clone();
    wrong_opening
        .opening
        .as_mut()
        .unwrap()
        .question
        .push_str(" unrelated");
    wrong_opening.opening_context_sha256 = Some(opening::context_sha256(&wrong_opening).unwrap());
    assert!(request(&wrong_opening).is_err());
    r.question = "\"".repeat(12000);
    r.opening.as_mut().unwrap().question = r.question.clone();
    r.opening_context_sha256 = Some(opening::context_sha256(&r).unwrap());
    let (prompt, _, _, _) = request(&r).unwrap();
    let intent: Value = serde_json::from_str(&prompt).unwrap();
    assert_eq!(intent["question"], r.question);
    assert!(intent["guideContext"].get("openingQuestion").is_none());
    assert!(prompt.len() < 128 * 1024);
    assert_eq!(MAX_INITIAL_UNITS, 5);
    assert_eq!(MAX_TOTAL_UNITS, 100);
    assert_eq!(GUIDE_INITIAL_BUFFER_SLIDES, 5);
    assert_eq!(MAX_ATTEMPTS, 3);
}

#[test]
fn plan_instructions_keep_bounded_outline_and_incremental_publication() {
    let f = Fixture::new();
    let r = crate::repository_guide::tests::fresh_record(&f);
    let (_, schema, _, instructions) = request(&r).unwrap();
    assert!(instructions.contains("Create the initial research outline only"));
    assert!(instructions.contains("sections array contains 1–6 items total"));
    assert!(instructions.contains("at most five are follow-on topics"));
    assert!(instructions.contains("Omissions contain at most 64 unique"));
    assert!(instructions.contains("Do not repeat labels after case/spacing normalization"));
    assert!(instructions.contains(
        "Follow-on prerequisites are optional and may name only keys of earlier follow-on items"
    ));
    assert!(instructions.contains("never reference `opening`, the current item, or a later item"));
    assert!(instructions.contains(
        "Each follow-on also has exactly one claim with one to four delivered code references"
    ));
    assert!(
        instructions.contains("Keep every other requested or discovered route explicitly pending")
    );
    assert!(instructions.contains("Code proves behavior"));
    assert!(instructions.contains("Any internal look-ahead must be small and must never gate the outline, first slide, or playback"));
    assert!(!instructions.contains("five-slide ready buffer"));
    assert!(!instructions.contains("keeps up to five ready slides buffered"));
    assert_eq!(
        schema["properties"]["sections"]["maxItems"],
        MAX_INITIAL_UNITS + 1
    );
    assert!(
        schema["properties"]["sections"]["description"]
            .as_str()
            .unwrap()
            .contains("six sections total")
    );
    assert_eq!(
        schema["properties"]["sections"]["prefixItems"][0]["properties"]["key"]["enum"],
        json!(["opening"])
    );
    assert!(
        schema["properties"]["sections"]["items"]["properties"]["prerequisites"]["description"]
            .as_str()
            .unwrap()
            .contains("earlier follow-on keys only")
    );
    assert!(
        agent_context::RESEARCH_BASE_INSTRUCTIONS
            .contains("begin with one focused search_code call using resultKind:files")
    );
    assert!(agent_context::RESEARCH_BASE_INSTRUCTIONS
        .contains("Every discovered route must appear as an initial detailed slide or an explicit pending topic"));
    assert!(instructions.contains(RESEARCH_INSTRUCTIONS));
}

#[test]
fn planning_retry_prompt_explains_the_outline_bounds() {
    let f = Fixture::new();
    let mut r = crate::repository_guide::tests::fresh_record(&f);
    r.planning_retry_count = 1;
    let (_, _, _, instructions) = request(&r).unwrap();
    assert!(instructions.contains("automatic retry after the previous outline was rejected"));
    assert!(instructions.contains("one opening plus at most five follow-ons"));
    assert!(instructions.contains("prerequisites may name only earlier follow-on keys"));
    assert!(instructions.contains("64 unique omission labels"));
    assert!(instructions.contains("Prefer fewer follow-ons over risking another invalid outline"));
}

#[test]
fn invalid_outline_limits_report_the_bound_that_failed() {
    let mut too_many_sections = proposal();
    for index in 0..=MAX_INITIAL_UNITS {
        too_many_sections["sections"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "key": format!("extra-{index}"),
                "question": format!("How does the extra route {index} behave?"),
                "prerequisites": [],
                "claims": [{"evidence":[{"referenceId":"code:2"}]}]
            }));
    }
    assert!(
        canonicalize_proposal(too_many_sections)
            .unwrap_err()
            .contains("one opening and no more than five follow-ons")
    );

    let mut duplicate_omissions = proposal();
    duplicate_omissions["omissions"] = json!(["Pending route A", " pending   ROUTE a "]);
    assert!(
        canonicalize_proposal(duplicate_omissions)
            .unwrap_err()
            .contains("too many or repeated pending topics")
    );
}

#[test]
fn repository_guide_outline_has_no_wall_clock_deadline() {
    let f = Fixture::new();
    let r = crate::repository_guide::tests::fresh_record(&f);
    let (_, _, _, instructions) = request(&r).unwrap();
    let started = std::time::Instant::now() - std::time::Duration::from_secs(11 * 60);

    assert!(instructions.contains("no wall-clock deadline"));
    assert!(!instructions.contains("ten-minute"));
    assert_eq!(
        crate::codex_agent::RunDeadline::UserControlled.duration(),
        None
    );
    assert!(!crate::codex_agent::RunDeadline::UserControlled.exceeded(started));
}

#[test]
fn outline_plan_is_persistable_without_waiting_for_opening_content() {
    let f = Fixture::new();
    let r = crate::repository_guide::tests::fresh_record(&f);
    let before = serde_json::to_vec(&r).unwrap();
    let binding = opening::context_sha256(&r).unwrap();
    let (prompt, schema, fingerprint, instructions) = request(&r).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&prompt).unwrap()["question"],
        r.question
    );
    assert_eq!(schema, super::schema());
    assert!(schema["properties"].get("openingAnswer").is_none());
    assert!(instructions.contains("Return only the supplied schema"));
    assert_eq!(request(&r).unwrap().2, fingerprint);
    assert_eq!(opening::context_sha256(&r).unwrap(), binding);
    assert_eq!(serde_json::to_vec(&r).unwrap(), before);
    assert_eq!(MAX_INITIAL_UNITS, 5);
    assert_eq!(MAX_TOTAL_UNITS, 100);
    assert_eq!(GUIDE_INITIAL_BUFFER_SLIDES, 5);
    assert_eq!(MAX_ATTEMPTS, 3);
}

#[test]
fn production_plan_session_has_no_answer_receipt_or_publication_tool() {
    let f = Fixture::new();
    let mut r = crate::repository_guide::tests::fresh_record(&f);
    let runtime = crate::codex_identity::CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(),
        executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64),
        runtime_version: Some("fixture-runtime-1".to_owned()),
        runtime_version_source: Some(crate::codex_identity::CodexRuntimeVersionSource::UserAgent),
        account_identity_sha256: "b".repeat(64),
        account_revision: 1,
    };
    r.grant_id =
        workspace::save_retrieval_grant(&f.storage, &r.project_id, &r.snapshot_id, &runtime)
            .unwrap();
    let (_, _, fingerprint, _) = request(&r).unwrap();
    let _session = new_session(
        &f.storage,
        &r,
        f.manifest.clone(),
        &runtime,
        RepositoryContextJobs::default(),
        &fingerprint,
        Arc::new(|_| {}),
    )
    .unwrap();
    assert!(!reading::exists(&f.storage, &r.planning_run_id).unwrap());
    assert!(
        !_session.configuration().unwrap()["enabled_tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool == context_mcp::PROPOSAL_TOOL)
    );
    assert_eq!(unit_intent(&r, 0).unwrap().question, r.question);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}
