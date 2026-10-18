use super::*;
use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE};

fn proposal() -> Value {
    json!({"citationVersion":"nativeEvidenceV2","planVersion":"repositoryGuidePlanV1","title":"Follow the greeting journey",
    "sections":[{"key":"input","question":"How does the incoming name enter the greeting?","prerequisites":[],"claims":[{"evidence":[{"referenceId":"code:1"}]}]},
    {"key":"output","question":"How does the greeting turn into the returned output?","prerequisites":["input"],"claims":[{"evidence":[{"referenceId":"code:1"}]}]}],"omissions":["Runtime execution was not observed."]})
}
fn resolve(value: Value) -> native_citations::Resolved {
    let f = Fixture::new();
    let source = context::source(&f.storage, &f.manifest, SIMPLE_PATH).unwrap();
    let range = context::range_from_source(&source, 0, SIMPLE_SOURCE.len())
        .unwrap()
        .range;
    match native_citations::resolve(&value.to_string(), |_| {
        Ok(native_citations::Citation::checked(&range, &source).unwrap())
    }) {
        native_citations::Resolution::Ready(r) => r,
        _ => panic!("fixture references must resolve"),
    }
}
#[test]
fn outline_keeps_research_questions_and_native_ranges_not_unverified_explanations() {
    let result = canonicalize(&resolve(proposal())).unwrap();
    assert_eq!(result.units.len(), 2);
    assert_eq!(result.units[0].starting_ranges[0].path, SIMPLE_PATH);
    assert_eq!(result.units[1].prerequisites, vec!["input"]);
    let encoded = serde_json::to_string(&result).unwrap();
    assert!(!encoded.contains("quote"));
    assert!(!encoded.contains(SIMPLE_SOURCE));
    let schema = schema();
    assert_eq!(schema["properties"]["sections"]["maxItems"], 6);
    assert!(opening::instructions().contains("outline"));
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
            "cycle" => p["sections"][0]["prerequisites"] = json!(["input"]),
            "duplicateKey" => p["sections"][1]["key"] = json!("input"),
            "duplicateQuestion" => {
                p["sections"][1]["question"] = p["sections"][0]["question"].clone()
            }
            "extraProse" => p["sections"][0]["claims"][0]["prose"] = json!("An unsupported fact"),
            "forward" => p["sections"][0]["prerequisites"] = json!(["output"]),
            "notQuestion" => {
                p["sections"][0]["question"] = json!("This is not a research question.")
            }
            "extraRoot" => p["accepted"] = json!(true),
            "extraClaim" => {
                let c = p["sections"][0]["claims"][0].clone();
                p["sections"][0]["claims"].as_array_mut().unwrap().push(c);
            }
            _ => unreachable!(),
        }
        assert!(canonicalize(&resolve(p)).is_err(), "{kind}");
    }
}
#[test]
fn unread_references_never_become_starting_paths() {
    assert!(matches!(
        native_citations::resolve(&proposal().to_string(), |_| Err("referenceUnavailable")),
        native_citations::Resolution::Invalid(_)
    ));
}



#[test]
fn separate_opening_intent_binds_both_questions_and_preserves_the_complete_outline_scope() {
    let f = Fixture::new();
    let mut r = crate::repository_guide::tests::fresh_record(&f);
    r.question = "How do café 🦀 bytes reach output, including every requested alternate and failure?".into();
    r.opening_context_sha256 = Some(opening::context_sha256(&r).unwrap());
    let (prompt, output, fingerprint, instructions) = request(&r).unwrap();
    let intent: Value = serde_json::from_str(&prompt).unwrap();
    assert_eq!(intent["question"], opening::LEARNING_QUESTION);
    assert_eq!(intent["guideContext"]["question"], r.question);
    assert_eq!(intent["guideContext"]["guideId"], r.id);
    assert_eq!(output, opening::schema_for(&r, schema()));
    assert!(output["properties"]["openingAnswer"]["description"].as_str().unwrap().contains("not a completion declaration"));
    assert!(instructions.contains("two distinct questions"));
    assert!(instructions.contains("Missing conditions, fall-through or returned/propagated outcomes"));
    assert!(instructions.contains("never completes the whole guide"));
    assert!(instructions.contains(RESEARCH_INSTRUCTIONS));
    assert_eq!(opening::policy(&r), "first-transition-with-separate-learning-intent-v3");
    r.question.push_str(" Preserve downstream ownership.");
    assert!(request(&r).is_err(), "an unchanged binding cannot admit a changed guide question");
    r.opening_context_sha256 = Some(opening::context_sha256(&r).unwrap());
    assert_ne!(request(&r).unwrap().2, fingerprint);
    let mut wrong_opening = r.clone();
    wrong_opening.opening.as_mut().unwrap().question = wrong_opening.question.clone();
    assert!(request(&wrong_opening).is_err());
    r.question = "\"".repeat(12000);
    r.opening_context_sha256 = Some(opening::context_sha256(&r).unwrap());
    let (prompt, _, _, _) = request(&r).unwrap();
    assert_eq!(serde_json::from_str::<Value>(&prompt).unwrap()["guideContext"]["question"], r.question);
    assert!(prompt.len() < 128 * 1024);
    assert_eq!(MAX_UNITS + 1, 7);
    assert_eq!(MAX_ATTEMPTS, 2);
}

#[test]
fn opening_instructions_keep_the_question_scoped() {
    let f = Fixture::new();
    let r = crate::repository_guide::tests::fresh_record(&f);
    let (_, _, _, instructions) = request(&r).unwrap();
    assert!(!instructions.contains("The original learning question remains the guide's full scope."),
        "the top-level opening question cannot also be declared the full-guide scope");
    assert!(instructions.contains("guideContext.question"));
    assert!(instructions.contains("Unclaimed callee internals and later selectors belong in the deeper outline"));
    assert!(instructions.contains("If a published claim depends on an unverified helper"));
    assert!(instructions.contains("never completes the whole guide"));
    assert!(instructions.contains(RESEARCH_INSTRUCTIONS));

    assert_eq!(MAX_UNITS + 1, 7);
    assert_eq!(MAX_ATTEMPTS, 2);
}

#[test]
fn scoped_opening_keeps_one_transition_and_defers_later_entries_without_dropping_scope() {
    let f = Fixture::new();
    let r = crate::repository_guide::tests::fresh_record(&f);
    let before = serde_json::to_vec(&r).unwrap();
    let binding = opening::context_sha256(&r).unwrap();
    let (prompt, schema, fingerprint, instructions) = request(&r).unwrap();
    assert!(instructions.contains("Choose exactly one entry point"));
    assert!(instructions.contains("wrappers and predicate helpers"));
    assert!(instructions.contains("Normally publish one focused opening section"));
    assert!(instructions.contains("later API entry points, selectors and flag normalization"));
    assert!(instructions.contains("revisit source for genuinely new behavior"));
    assert!(instructions.contains("Never discard a publication or hide an unresolved condition"));
    assert!(instructions.contains("Native publication, evidence and lifetime budgets are unchanged"));
    assert_eq!(serde_json::from_str::<Value>(&prompt).unwrap()["guideContext"]["question"], r.question);
    assert_eq!(schema, opening::schema_for(&r, super::schema()));
    assert_eq!(request(&r).unwrap().2, fingerprint);
    assert_eq!(opening::context_sha256(&r).unwrap(), binding);
    assert_eq!(serde_json::to_vec(&r).unwrap(), before);
    assert_eq!(MAX_UNITS + 1, 7);
    assert_eq!(MAX_ATTEMPTS, 2);


}

#[test]
fn production_opening_session_stores_the_separate_question_without_connecting_an_account() {
    let f = Fixture::new();
    let mut r = crate::repository_guide::tests::fresh_record(&f);
    let runtime = crate::codex_identity::CodexReviewRuntime {
        connection_id: Uuid::new_v4().to_string(), executable_path: "/usr/local/bin/codex".into(),
        executable_sha256: "a".repeat(64), account_identity_sha256: "b".repeat(64), account_revision: 1,
    };
    r.grant_id = workspace::save_retrieval_grant(&f.storage, &r.project_id, &r.snapshot_id, &runtime).unwrap();
    let (_, _, fingerprint, _) = request(&r).unwrap();
    let _session = new_session(&f.storage, &r, f.manifest.clone(), &runtime,
        RepositoryContextJobs::default(), &fingerprint, Arc::new(|_| {})).unwrap();
    let answer = reading::scoped(&f.storage, &r.planning_run_id, &r.project_id, &r.snapshot_id).unwrap();
    assert_eq!(answer.question, opening::LEARNING_QUESTION);
    assert_ne!(answer.question, r.question);
    assert_eq!(answer.outcome, Outcome::Working);
    assert!(answer.sections.is_empty());
    assert_eq!(unit_intent(&r, 0).unwrap().question, r.question);
    assert!(!f.storage.join("codex-agent/runtime.json").exists());
}
