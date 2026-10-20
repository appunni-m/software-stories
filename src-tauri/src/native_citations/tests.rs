use super::*;

fn source(text: &str) -> SnapshotSource {
    SnapshotSource {
        snapshot_id: "captured".into(),
        path: "src/code.rs".into(),
        sha256: context::digest(text.as_bytes()),
        size_bytes: text.len() as u64,
        contents: text.into(),
    }
}
fn draft(id: &str) -> String {
    json!({"citationVersion":SHORT_VERSION,"sections":[{"claims":[{"evidence":[{"referenceId":id}]}]}]})
        .to_string()
}
fn ready(resolution: Resolution) -> Resolved {
    match resolution {
        Resolution::Ready(resolved) => resolved,
        Resolution::Invalid(issues) => panic!("{issues:?}"),
    }
}

#[test]
fn short_reference_transport_keeps_exact_bytes_and_does_not_guess_unknown_ids() {
    let source = source("fn greeting() { return; }\n");
    let range = context::range_from_source(&source, 0, source.contents.len())
        .unwrap()
        .range;
    let citation = Citation::checked(&range, &source).unwrap();
    let raw = json!({"citationVersion":"nativeEvidenceV2","sections":[{"claims":[{"evidence":[{"referenceId":"code:1"}]}]}]}).to_string();
    let resolved = ready(resolve(&raw, |id| {
        assert_eq!(id, "code:1");
        Ok(citation.clone())
    }));
    assert_eq!(
        resolved.claim_spans(0, 0).unwrap()[0].excerpt,
        source.contents
    );
    for id in [
        "code:0",
        "code:01",
        "code:-1",
        "code:513",
        "code:1suffix",
        "code: 1",
    ] {
        let invalid = raw.replace("code:1", id);
        assert_eq!(
            issues(resolve(&invalid, |_| panic!(
                "Malformed IDs cannot reach lookup"
            )))[0]["citationProblem"],
            "referenceShape"
        );
    }
    assert_eq!(
        issues(resolve(&raw.replace("code:1", "code:2"), |_| Err(
            "referenceUnavailable"
        )))[0]["citationProblem"],
        "referenceUnavailable"
    );
    assert_eq!(
        issues(resolve(
            &raw.replace("nativeEvidenceV2", "nativeEvidenceV1"),
            |_| panic!("Unsupported protocol cannot reach lookup")
        ))[0]["citationProblem"],
        "citationVersion"
    );
    let long_id = format!("evidence:{}", "a".repeat(64));
    assert_eq!(
        issues(resolve(&draft(&long_id), |_| {
            panic!("Long evidence IDs cannot reach lookup")
        }))[0]["citationProblem"],
        "referenceShape"
    );
}
fn issues(resolution: Resolution) -> Vec<Value> {
    match resolution {
        Resolution::Ready(_) => panic!("Unexpected valid references"),
        Resolution::Invalid(issues) => issues,
    }
}

#[test]
fn exact_coordinates_resolve_the_second_repeated_occurrence_without_substring_guessing() {
    let text = "fn repeated() {}\nfn repeated() {}\n";
    let source = source(text);
    let from = "fn repeated() {}\n".len();
    let range = context::range_from_source(&source, from, text.len())
        .unwrap()
        .range;
    let citation = Citation::checked(&range, &source).unwrap();
    assert_eq!(
        crate::chapter_generation::exact_quote_start(text, "fn repeated() {}\n"),
        Err(crate::chapter_generation::QuoteProblem::Ambiguous)
    );
    let resolved = ready(resolve(&draft(&"code:1".to_owned()), |_| {
        Ok(citation.clone())
    }));
    let span = &resolved.claim_spans(0, 0).unwrap()[0];
    assert_eq!(
        span.checked_start(&source.path, "fn repeated() {}\n", &source)
            .unwrap(),
        from
    );
    assert!(
        span.checked_start("src/foreign.rs", "fn repeated() {}\n", &source)
            .is_err()
    );
    assert!(
        span.checked_start(&source.path, "fn repeated(){}\n", &source)
            .is_err()
    );
    let mut damaged = source.clone();
    damaged.contents.replace_range(from..from + 2, "xx");
    assert!(
        span.checked_start(&source.path, "fn repeated() {}\n", &damaged)
            .is_err()
    );
}

#[test]
fn comment_bytes_cannot_be_omitted_or_reflowed_by_a_native_reference() {
    let text =
        "probe();\n// Its unknown-format result is\n// the final face-open error.\nreturn error;\n";
    let source = source(text);
    let range = context::range_from_source(&source, 0, text.len())
        .unwrap()
        .range;
    let citation = Citation::checked(&range, &source).unwrap();
    let id = "code:2".to_owned();
    let resolved = ready(resolve(&draft(&id), |_| Ok(citation.clone())));
    let json: Value = serde_json::from_str(resolved.text()).unwrap();
    assert_eq!(
        json["sections"][0]["claims"][0]["evidence"][0]["quote"],
        text
    );
    for changed in [
        text.replace("result is\n// the", "result\n// is the"),
        text.replace(
            "// Its unknown-format result is\n// the final face-open error.\n",
            "",
        ),
    ] {
        assert!(
            citation
                .checked_start(&source.path, &changed, &source)
                .is_err()
        );
    }
    let forged = draft(&id).replace(
        &format!("\"referenceId\":\"{id}\""),
        "\"path\":\"src/code.rs\",\"quote\":\"probe();\"",
    );
    assert_eq!(
        issues(resolve(&forged, |_| panic!("Quotes must not reach lookup")))[0]["citationProblem"],
        "referenceShape"
    );
}

#[test]
fn partitions_preserve_large_unicode_crlf_and_long_line_source_exactly() {
    let text = format!(
        "{}\r\n{}\r\n{}",
        "a".repeat(999),
        "α".repeat(16_000),
        "// same line\r\n".repeat(500)
    );
    let pieces = spans(&text, 0, text.len()).unwrap();
    assert!(pieces.len() > 32);
    let mut bytes = String::new();
    let mut expected = 0;
    for (from, to) in pieces {
        assert_eq!(from, expected);
        assert!(to > from && to - from <= MAX_SPAN);
        if to < text.len() {
            assert_ne!(
                (&text.as_bytes()[to - 1], &text.as_bytes()[to]),
                (&b'\r', &b'\n')
            );
        }
        bytes.push_str(&text[from..to]);
        expected = to;
    }
    assert_eq!(bytes, text);
    assert!(spans("αβγδ", 1, 8).is_err());
    assert!(!citeable("tiny"));
    assert!(!citeable("          "));
    assert!(citeable("fn ok() {}"));
}

#[test]
fn malformed_foreign_and_oversized_references_have_bounded_source_free_findings() {
    let long_id = format!("evidence:{}", "c".repeat(64));
    for input in [
        "{}".to_owned(),
        draft("../secret"),
        draft(&long_id).replace(SHORT_VERSION, "future"),
        draft(&long_id).replace("\"referenceId\":", "\"quote\":"),
        "{\"citationVersion\":\"nativeEvidenceV1\",\"citationVersion\":\"nativeEvidenceV1\"}"
            .to_owned(),
    ] {
        let found = issues(resolve(&input, |_| Err("referenceUnavailable")));
        assert!(found.len() <= MAX_ISSUES);
        assert!(!serde_json::to_string(&found).unwrap().contains("secret"));
    }
    let id = "code:1";
    let claim = json!({"evidence":vec![json!({"referenceId":id});8]});
    let input =
        json!({"citationVersion":SHORT_VERSION,"sections":vec![json!({"claims":vec![claim;20]});12]})
            .to_string();
    let found = issues(resolve(&input, |_| Err("referenceUnavailable")));
    assert_eq!(found.len(), MAX_ISSUES);
    assert!(serde_json::to_vec(&found).unwrap().len() < 12000);
    let source = source(&"x".repeat(MAX_SPAN));
    let range = context::range_from_source(&source, 0, MAX_SPAN)
        .unwrap()
        .range;
    let citation = Citation::checked(&range, &source).unwrap();
    assert_eq!(
        issues(resolve(&input, |_| Ok(citation.clone())))[0]["citationProblem"],
        "citationExpansionLimit"
    );
}
