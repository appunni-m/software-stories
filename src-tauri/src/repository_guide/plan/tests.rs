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
