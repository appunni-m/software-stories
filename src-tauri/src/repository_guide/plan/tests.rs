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
    assert!(INSTRUCTIONS.contains("Read large files by ranges"));
    assert!(INSTRUCTIONS.contains("not an answer or file inventory"));
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
fn opening_request_retains_full_intent_and_binds_its_versioned_publication_contract() {
    let f = Fixture::new();
    let mut r = crate::repository_guide::tests::record(&f);
    let (_, legacy_schema, legacy_digest, legacy_instructions) = request(&r).unwrap();
    assert_eq!(legacy_schema, schema());
    assert_eq!(legacy_instructions, INSTRUCTIONS);
    r.schema_version = 3;
    r.contract = CONTRACT.into();
    r.question = "How do café 🦀 bytes reach output, including rejected inputs?".into();
    r.opening = Some(Attempt {
        run_id: r.planning_run_id.clone(),
        question: r.question.clone(),
        summary: None,
    });
    let (prompt, output, digest, instructions) = request(&r).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&prompt).unwrap()["question"],
        r.question
    );
    assert_eq!(output, opening::schema(schema()));
    assert_ne!(digest, legacy_digest);
    assert!(instructions.contains("sceneAuthoringLimit"));
    assert_eq!(request(&r).unwrap().2, digest);
    r.question
        .push_str(" Preserve every requested alternative.");
    assert_ne!(request(&r).unwrap().2, digest);
    r.question = "\"".repeat(12000);
    r.opening.as_mut().unwrap().question = r.question.clone();
    let (prompt, _, _, _) = request(&r).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&prompt).unwrap()["question"],
        r.question
    );
    assert_eq!(MAX_UNITS, 6);
    assert_eq!(MAX_ATTEMPTS, 2);
}

#[test]
fn fresh_research_policy_is_fingerprinted_without_rewriting_historical_requests() {
    let f = Fixture::new();
    let mut r = crate::repository_guide::tests::fresh_record(&f);
    let (prompt, output, fresh_digest, fresh_instructions) = request(&r).unwrap();
    assert!(fresh_instructions.ends_with(RESEARCH_INSTRUCTIONS));
    r.schema_version = 2;
    r.contract = OPENING_CONTRACT.into();
    let (old_prompt, old_output, old_digest, old_instructions) = request(&r).unwrap();
    assert_eq!(prompt, old_prompt);
    assert_eq!(output, old_output);
    assert_eq!(old_instructions, opening::instructions());
    assert_ne!(fresh_digest, old_digest);
    assert_eq!(request(&r).unwrap().2, old_digest);
    assert_eq!(RESEARCH_POLICY, "ordered-independent-research-v1");
}
