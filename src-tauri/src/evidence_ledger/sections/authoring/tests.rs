use super::*;

pub(crate) fn section(reference: &str) -> Value {
    json!({"title":"Formatting a greeting","claims":[{
        "category":"codeBehavior","aspect":"inputOutput",
        "prose":"The greeting function formats the supplied name into a message. The returned greeting includes that name.",
        "inferenceBasis":"","identifiers":["greeting"],"evidence":[{"referenceId":reference}],"relationshipIds":[],
        "scene":{"title":"From name to message","narration":"Start with a person's name. The function places it inside a greeting, so that name appears in the returned message.",
        "mode":"composition","nodes":[
            {"key":"input","label":"Name","detail":"","references":[reference]},
            {"key":"result","label":"Message","detail":"","references":[reference]}],
        "edges":[{"fromKey":"input","toKey":"result","label":"format","references":[reference]}],
        "elements":[
            {"key":"layout","kind":"row","parentKey":null,"nodeKey":null,"tone":"neutral","value":"","valueOrigin":"none"},
            {"key":"name","kind":"code","parentKey":"layout","nodeKey":"input","tone":"accent","value":"name","valueOrigin":"source"},
            {"key":"direction","kind":"arrow","parentKey":"layout","nodeKey":null,"tone":"muted","value":"","valueOrigin":"none"},
            {"key":"message","kind":"token","parentKey":"layout","nodeKey":"result","tone":"positive","value":"","valueOrigin":"none"}]}
    }],"visual":{"kind":"dataFlow","title":"Format the greeting","direction":"leftToRight"}})
}

pub(crate) fn proposal(reference: &str, key: &str, order: usize) -> Value {
    json!({"proposalVersion":super::super::SCENE_PROPOSAL_VERSION,"sectionKey":key,"order":order,
        "previousGenerationId":null,"section":section(reference)})
}

#[test]
fn local_keys_compile_to_exact_canonical_references_without_rewriting_content() {
    let raw = section("code:1");
    let result = compile(&raw).unwrap_or_else(|f| panic!("{f:?}"));
    let section = &result.section;
    let mut claim = raw["claims"][0].clone();
    claim.as_object_mut().unwrap().remove("scene");
    assert_eq!(section["claims"][0], claim);
    assert_eq!(
        section["visual"]["nodes"][0]["evidence"],
        json!([{"claimIndex":0,"evidenceIndex":0}])
    );
    assert_eq!(section["visual"]["edges"][0]["fromNodeIndex"], 0);
    assert_eq!(section["visual"]["edges"][0]["toNodeIndex"], 1);
    let scene = &section["visual"]["presentation"]["scenes"][0];
    assert_eq!(scene["claimIndex"], 0);
    assert_eq!(scene["narration"], raw["claims"][0]["scene"]["narration"]);
    assert_eq!(scene["elements"][1]["parentIndex"], 0);
    assert_eq!(scene["elements"][1]["nodeIndex"], 0);
    assert_eq!(scene["elements"][3]["nodeIndex"], 1);
    assert_eq!(scene["elements"][1]["value"], "name");
}

#[test]
fn identical_keys_in_different_scenes_cannot_cross_claim_or_element_scope() {
    let mut raw = section("code:1");
    let mut second = raw["claims"][0].clone();
    second["evidence"][0]["referenceId"] = json!("code:2");
    for node in second["scene"]["nodes"].as_array_mut().unwrap() {
        node["references"] = json!(["code:2"]);
    }
    second["scene"]["edges"][0]["references"] = json!(["code:2"]);
    raw["claims"].as_array_mut().unwrap().push(second);
    let compiled = compile(&raw).unwrap_or_else(|f| panic!("{f:?}"));
    assert_eq!(
        compiled.section["visual"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        compiled.section["visual"]["nodes"][2]["evidence"],
        json!([{"claimIndex":1,"evidenceIndex":0}])
    );
    assert_eq!(
        compiled.section["visual"]["presentation"]["scenes"][1]["elements"][3]["nodeIndex"],
        3
    );
    assert_eq!(
        compiled.section["visual"]["presentation"]["scenes"][1]["elements"][3]["parentIndex"],
        0
    );
    raw["claims"][1]["scene"]["nodes"][0]["references"] = json!(["code:1"]);
    assert_eq!(
        compile(&raw).err().unwrap()[0],
        json!({"location":"/section/claims/1/scene/nodes/0/references","code":"sceneAuthoringReference"})
    );
}

#[test]
fn duplicate_unknown_forward_and_wrong_claim_identities_are_rejected_without_guessing() {
    for case in [
        "node",
        "element",
        "parent",
        "forward",
        "reference",
        "duplicateReference",
        "ambiguousEvidence",
        "missingNullable",
        "extraField",
    ] {
        let mut raw = section("code:1");
        let claim = &mut raw["claims"][0];
        match case {
            "node" => claim["scene"]["nodes"][1]["key"] = json!("input"),
            "element" => claim["scene"]["elements"][1]["key"] = json!("layout"),
            "parent" => claim["scene"]["elements"][1]["parentKey"] = json!("not-there"),
            "forward" => claim["scene"]["elements"][0]["parentKey"] = json!("message"),
            "reference" => claim["scene"]["nodes"][0]["references"] = json!(["code:2"]),
            "duplicateReference" => {
                claim["scene"]["nodes"][0]["references"] = json!(["code:1", "code:1"])
            }
            "ambiguousEvidence" => claim["evidence"]
                .as_array_mut()
                .unwrap()
                .push(json!({"referenceId":"code:1"})),
            "missingNullable" => {
                claim["scene"]["elements"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("nodeKey");
            }
            "extraField" => {
                claim["scene"]["elements"][0]["html"] =
                    json!("ignored content must not be discarded")
            }
            _ => unreachable!(),
        }
        assert!(compile(&raw).is_err(), "{case}");
    }
}

#[test]
fn canonical_positions_remap_to_authored_positions_and_never_copy_values_into_findings() {
    let compiled = compile(&section("code:1")).unwrap_or_else(|f| panic!("{f:?}"));
    let mut findings = vec![
        json!({"location":"/section/visual/nodes/1/evidence/0","code":"citation"}),
        json!({"location":"/section/visual/presentation/scenes/0/elements/1/value","code":"sceneSourceValue"}),
        json!({"location":"/section/visual/edges/0/toNodeIndex","code":"sectionValidation"}),
        json!({"location":"/section/visual/presentation/scenes","code":"sceneNarration"}),
    ];
    compiled.remap(&mut findings);
    assert_eq!(
        findings[0]["location"],
        "/section/claims/0/scene/nodes/1/references/0"
    );
    assert_eq!(
        findings[1]["location"],
        "/section/claims/0/scene/elements/1/value"
    );
    assert_eq!(
        findings[2]["location"],
        "/section/claims/0/scene/edges/0/toKey"
    );
    assert_eq!(findings[3]["location"], "/section/claims");
    assert!(!serde_json::to_string(&findings).unwrap().contains("Name"));
}

#[test]
fn compiler_preserves_section_wide_limits_and_explicit_no_visual_fallback() {
    let mut raw = section("code:1");
    let claim = raw["claims"][0].clone();
    raw["claims"] = json!(vec![claim; 7]);
    assert_eq!(
        compile(&raw).err().unwrap()[0]["code"],
        "sceneAuthoringLimit"
    );
    let mut raw = section("code:1");
    raw["visual"] = json!({"kind":"none","title":"","direction":"auto"});
    assert!(compile(&raw).is_err());
    raw["claims"][0]["scene"] = Value::Null;
    let compiled = compile(&raw).unwrap_or_else(|f| panic!("{f:?}"));
    assert_eq!(
        compiled.section["visual"],
        json!({"kind":"none","title":"","direction":"auto","nodes":[],"edges":[]})
    );
    assert!(compiled.section["claims"][0].get("scene").is_none());
}

#[test]
fn claim_evidence_bounds_report_the_exact_field_without_removing_references() {
    for count in [0, 9] {
        for no_visual in [false, true] {
            let mut raw = section("code:1");
            raw["claims"][0]["evidence"] = json!(
                (1..=count)
                    .map(|i| json!({"referenceId":format!("code:{i}")}))
                    .collect::<Vec<_>>()
            );
            if no_visual {
                raw["visual"] = json!({"kind":"none","title":"","direction":"auto"});
                raw["claims"][0]["scene"] = Value::Null;
            }
            let original = raw.clone();
            assert_eq!(
                compile(&raw).err().unwrap(),
                vec![json!({"location":"/section/claims/0/evidence","code":"sceneAuthoringLimit"})]
            );
            assert_eq!(raw, original);
        }
    }
}

#[test]
fn advertised_schema_matches_authored_objects_and_forbids_shared_numeric_pointers() {
    let schema = section_schema();
    let validator = jsonschema::validator_for(&schema).unwrap();
    let original = section("code:1");
    assert!(validator.is_valid(&original));
    for case in [
        "globalNodes",
        "claimIndex",
        "nodeIndex",
        "missingScene",
        "missingParentKey",
        "unknownField",
    ] {
        let mut raw = original.clone();
        match case {
            "globalNodes" => raw["visual"]["nodes"] = json!([]),
            "claimIndex" => raw["claims"][0]["scene"]["claimIndex"] = json!(0),
            "nodeIndex" => raw["claims"][0]["scene"]["elements"][1]["nodeIndex"] = json!(0),
            "missingScene" => {
                raw["claims"][0].as_object_mut().unwrap().remove("scene");
            }
            "missingParentKey" => {
                raw["claims"][0]["scene"]["elements"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("parentKey");
            }
            "unknownField" => {
                raw["claims"][0]["scene"]["nodes"][0]["url"] = json!("https://example.com")
            }
            _ => unreachable!(),
        }
        assert!(!validator.is_valid(&raw), "{case}");
        assert!(compile(&raw).is_err(), "{case}");
    }
    let mut fallback = original;
    fallback["claims"][0]["scene"] = Value::Null;
    fallback["visual"] = json!({"kind":"none","title":"","direction":"auto"});
    assert!(validator.is_valid(&fallback));
}

#[test]
fn compilation_is_deterministic_and_preserves_unicode_and_authored_examples_exactly() {
    let mut raw = section("code:1");
    raw["title"] = json!("名前をメッセージへ");
    raw["claims"][0]["scene"]["narration"] = json!(
        "For example, the name José remains in the greeting. An empty name still keeps the greeting prefix."
    );
    raw["claims"][0]["scene"]["elements"][3]["value"] = json!("Hello, José 👋");
    raw["claims"][0]["scene"]["elements"][3]["valueOrigin"] = json!("illustration");
    let first = compile(&raw).unwrap_or_else(|f| panic!("{f:?}"));
    for _ in 0..16 {
        assert_eq!(
            compile(&raw).unwrap_or_else(|f| panic!("{f:?}")).section,
            first.section
        );
    }
    assert_eq!(first.section["title"], raw["title"]);
    assert_eq!(
        first.section["visual"]["presentation"]["scenes"][0]["elements"][3]["value"],
        "Hello, José 👋"
    );
    assert_eq!(
        first.section["visual"]["presentation"]["scenes"][0]["narration"],
        raw["claims"][0]["scene"]["narration"]
    );
}
