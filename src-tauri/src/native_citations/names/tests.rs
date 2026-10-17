use super::*;

fn compiled(prose: &str, fragments: &[&str]) -> Result<Resolved, Vec<Value>> {
    compiled_with_policy(prose, fragments, Policy::ExactV1)
}

fn compiled_with_policy(
    prose: &str,
    fragments: &[&str],
    policy: Policy,
) -> Result<Resolved, Vec<Value>> {
    let mut citations = std::collections::HashMap::new();
    for (index, fragment) in fragments.iter().enumerate() {
        let source = SnapshotSource {
            snapshot_id: "snapshot".into(),
            path: format!("src/{index}.rs"),
            sha256: context::digest(fragment.as_bytes()),
            size_bytes: fragment.len() as u64,
            contents: (*fragment).into(),
        };
        let range = context::range_from_source(&source, 0, fragment.len())
            .unwrap()
            .range;
        citations.insert(
            short_reference(index),
            Citation::checked(&range, &source).unwrap(),
        );
    }
    let raw = json!({"citationVersion":SHORT_VERSION,"title":"Keep all fields","sections":[{"claims":[{
        "prose":prose,"inferenceBasis":"unchanged","scene":{"narration":"Complete natural speech."},
        "evidence":(0..fragments.len()).map(|i|json!({"referenceId":short_reference(i)})).collect::<Vec<_>>()
    }]}]}).to_string();
    match resolve(&raw, |id| {
        citations.get(id).cloned().ok_or("referenceUnavailable")
    }) {
        Resolution::Ready(resolved) => compile_with_policy(resolved, policy),
        Resolution::Invalid(issues) => panic!("{issues:?}"),
    }
}

#[test]
fn grounded_suffixes_preserve_prose_and_prefer_the_longest_supported_literal_name() {
    for (prose_name, source, expected) in [
        ("Library::greeting", "fn greeting() { return; }", "greeting"),
        ("crate::font::描画", "font::描画(data);", "font::描画"),
        (
            "module.renderer.render",
            "renderer.render(data);",
            "renderer.render",
        ),
        ("font-size", "set(\"font-size\", 12);", "font-size"),
    ] {
        let prose = format!("The `{prose_name}` operation returns `Result`.");
        let resolved = compiled_with_policy(&prose, &[source], Policy::GroundedSuffixV2).unwrap();
        let value: Value = serde_json::from_str(resolved.text()).unwrap();
        let claim = &value["sections"][0]["claims"][0];
        assert_eq!(claim["identifiers"], json!([expected]));
        assert_eq!(claim["prose"], prose);
        assert_eq!(claim["inferenceBasis"], "unchanged");
        assert_eq!(claim["scene"]["narration"], "Complete natural speech.");
        assert_eq!(claim["evidence"][0]["quote"], source);
        assert_eq!(resolved.claim_spans(0, 0).unwrap()[0].excerpt, source);
        // A literal suffix is metadata alignment, not proof of the surrounding
        // qualification, return type or any other semantic statement in prose.
        assert!(compiled(&prose, &[source]).is_err());
    }
}

#[test]
fn grounded_suffixes_do_not_split_keys_match_partial_names_or_cover_unrelated_fragments() {
    for (prose, fragments) in [
        (
            "The `font-size` key controls this operation.",
            vec!["let size = 12;"],
        ),
        (
            "The `Library::greeting` helper returns.",
            vec!["fn greeting_extra() {}"],
        ),
        (
            "The `Library::greeting` helper returns.",
            vec!["fn greeting() {}", "fn other() {}"],
        ),
        (
            "The `crate::return` operation returns.",
            vec!["return value;"],
        ),
        ("The greeting helper returns.", vec!["fn greeting() {}"]),
    ] {
        assert!(
            compiled_with_policy(prose, &fragments, Policy::GroundedSuffixV2).is_err(),
            "{prose}"
        );
    }
}

#[test]
fn only_explicit_names_from_each_own_fragment_form_bounded_deterministic_metadata() {
    assert_eq!(POLICY, "explicit-prose-names-own-fragments-v1");
    let prose = "The `greeting` helper calls `format_name` to create the message.";
    let fragments = [
        "fn greeting() { format_name(); }",
        "fn format_name() { return; }",
    ];
    for _ in 0..2 {
        let resolved = compiled(prose, &fragments).unwrap_or_else(|f| panic!("{f:?}"));
        let value: Value = serde_json::from_str(resolved.text()).unwrap();
        let claim = &value["sections"][0]["claims"][0];
        assert_eq!(claim["identifiers"], json!(["greeting", "format_name"]));
        assert_eq!(claim["prose"], prose);
        assert_eq!(claim["scene"]["narration"], "Complete natural speech.");
        assert_eq!(claim["inferenceBasis"], "unchanged");
        for (i, span) in resolved.claim_spans(0, 0).unwrap().iter().enumerate() {
            assert_eq!(span.excerpt, fragments[i]);
            assert_eq!(claim["evidence"][i]["quote"], fragments[i]);
        }
    }
    let fragments = (0..8)
        .map(|i| format!("fn helper_{i}() {{ return; }}"))
        .collect::<Vec<_>>();
    let prose = (0..8)
        .map(|i| format!("`helper_{i}`"))
        .collect::<Vec<_>>()
        .join(" calls ");
    let value: Value = serde_json::from_str(
        compiled(
            &prose,
            &fragments.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .unwrap_or_else(|f| panic!("{f:?}"))
        .text(),
    )
    .unwrap();
    assert_eq!(
        value["sections"][0]["claims"][0]["identifiers"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
}

#[test]
fn natural_words_keywords_expressions_fences_and_partial_identifiers_do_not_ground_claims() {
    for prose in [
        "The value returns the result of greeting.",
        "The `return` keyword stops here.",
        "The `x + y` expression adds.",
        "The ```greeting``` is fenced.",
        "The `greeting is unclosed.",
    ] {
        let findings = compiled(prose, &["fn greeting() { return value; }"])
            .err()
            .unwrap();
        assert_eq!(findings[0]["code"], "namedIdentifier", "{prose}");
        assert_eq!(findings[0]["location"], "/sections/0/claims/0/prose");
    }
    let findings = compiled(
        "The `greeting` helper returns a message.",
        &["fn greeting_extra() { return; }"],
    )
    .err()
    .unwrap();
    assert_eq!(
        findings[0],
        json!({"location":"/sections/0/claims/0/evidence/0","code":"namedIdentifierEvidence"})
    );
    let findings = compiled(
        "The `greeting` helper returns a message.",
        &["fn greeting() {}", "fn other() {} "],
    )
    .err()
    .unwrap();
    assert_eq!(findings[0]["location"], "/sections/0/claims/0/evidence/1");
    assert!(!serde_json::to_string(&findings).unwrap().contains("other"));
    let findings = compiled(
        "The `greeting` helper uses `unread_helper` to create the message.",
        &["fn greeting() { return; }"],
    )
    .err()
    .unwrap();
    assert_eq!(
        findings[0],
        json!({"location":"/sections/0/claims/0/prose","code":"namedIdentifierEvidence"})
    );
}

#[test]
fn unicode_qualified_names_and_configuration_keys_use_existing_whole_name_matching() {
    for (name, source) in [
        ("描画", "fn 描画() { return; }"),
        (
            "crate::font::memory_face",
            "crate::font::memory_face(data)?;",
        ),
        ("font-size", "set(\"font-size\", 12);"),
        ("$glyph", "const $glyph = render();"),
    ] {
        let resolved = compiled(
            &format!("The `{name}` name determines this operation."),
            &[source],
        )
        .unwrap_or_else(|f| panic!("{name}: {f:?}"));
        let value: Value = serde_json::from_str(resolved.text()).unwrap();
        assert_eq!(
            value["sections"][0]["claims"][0]["identifiers"],
            json!([name])
        );
    }
    for invalid in [
        "src/file.rs",
        "font:::size",
        "2glyph",
        "foo..bar",
        "-key",
        "fn()",
    ] {
        assert!(!name(invalid), "{invalid}");
    }
}

#[test]
fn metadata_is_the_only_changed_field_and_explicit_old_metadata_is_not_silently_removed() {
    let contents = "fn greeting() { return; }";
    let source = SnapshotSource {
        snapshot_id: "snapshot".into(),
        path: "src/code.rs".into(),
        sha256: context::digest(contents.as_bytes()),
        size_bytes: contents.len() as u64,
        contents: contents.into(),
    };
    let range = context::range_from_source(&source, 0, source.contents.len())
        .unwrap()
        .range;
    let citation = Citation::checked(&range, &source).unwrap();
    let raw = json!({"citationVersion":SHORT_VERSION,"sections":[{"claims":[{
        "prose":"The `greeting` helper returns here.","evidence":[{"referenceId":"code:1"}],"scene":null
    }]}]}).to_string();
    let Resolution::Ready(resolved) = resolve(&raw, |_| Ok(citation.clone())) else {
        panic!("checked source must resolve")
    };
    let original: Value = serde_json::from_str(resolved.text()).unwrap();
    let compiled = resolved
        .with_named_identifiers()
        .unwrap_or_else(|f| panic!("{f:?}"));
    let mut value: Value = serde_json::from_str(compiled.text()).unwrap();
    value["sections"][0]["claims"][0]
        .as_object_mut()
        .unwrap()
        .remove("identifiers");
    assert_eq!(value, original);
    let mut raw: Value = serde_json::from_str(&raw).unwrap();
    raw["sections"][0]["claims"][0]["identifiers"] = json!(["unrelated"]);
    let Resolution::Ready(resolved) = resolve(&raw.to_string(), |_| Ok(citation.clone())) else {
        panic!("checked source must resolve")
    };
    assert_eq!(
        resolved.with_named_identifiers().err().unwrap()[0]["location"],
        "/sections/0/claims/0/identifiers"
    );
}
