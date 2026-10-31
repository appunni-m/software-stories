use super::*;
use serde_json::json;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(format!("code-atlas-conversation-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
    fn path(&self) -> PathBuf {
        location(&self.0, "project", "snapshot").unwrap()
    }
    fn put(&self, bytes: &[u8]) {
        fs::create_dir_all(self.path().parent().unwrap()).unwrap();
        fs::write(self.path(), bytes).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn state() -> Value {
    json!({"version":1,"projectId":"project","snapshotId":"snapshot","currentId":"root","file":"src/font.rs","explorations":[{"id":"root","parentId":null,"originTurnId":null,"title":"Repository","view":"guide","turns":[{"id":"turn-root","question":"What happens to the font bytes?\nShow the failure path.","context":[{"path":"src/font.rs","from":3,"to":7,"sha256":"a".repeat(64)}],"status":"stopped","reading":{"guide":{"revisionId":"revision-one","sectionId":"section:two"}},"outcomeAt":123,"outcomeSequence":1,"outcomeNotice":{"dismissed":"123456789abcdef0"}}],"focusedTurnId":"turn-root","scrollPositions":{"guide":123.5,"flow":400}}]})
}

#[test]
fn storage_write_failures_give_specific_safe_recovery_guidance() {
    let full = conversation_storage_error(&std::io::Error::from(std::io::ErrorKind::StorageFull));
    assert_eq!(full, SAVE_STORAGE_FULL);
    assert!(full.contains("available disk space"));

    let quota =
        conversation_storage_error(&std::io::Error::from(std::io::ErrorKind::QuotaExceeded));
    assert_eq!(quota, SAVE_STORAGE_QUOTA);
    assert_ne!(quota, full);
    assert!(quota.contains("quota"));

    let permissions =
        conversation_storage_error(&std::io::Error::from(std::io::ErrorKind::PermissionDenied));
    assert_eq!(permissions, SAVE_STORAGE_ACCESS);
    assert!(permissions.contains("Check local storage permissions"));

    let other = conversation_storage_error(&std::io::Error::from(std::io::ErrorKind::Other));
    assert_eq!(other, SAVE_STORAGE);
    for message in [full, quota, permissions, other] {
        assert!(!message.contains('/'));
        assert!(!message.contains("ErrorKind"));
    }
}

#[test]
fn finite_answer_failure_codes_are_preserved_but_unknown_values_are_rejected() {
    let f = Fixture::new();
    let mut current = state();
    current["explorations"][0]["turns"][0]["failureCode"] = json!("noPublishedSections");
    let saved = save(&f.0, "project", "snapshot", None, current.clone()).unwrap();
    assert_eq!(saved.state, current);

    let mut invalid = current;
    invalid["explorations"][0]["turns"][0]["failureCode"] = json!("raw provider failure");
    assert!(save(&f.0, "project", "snapshot", Some(saved.revision), invalid).is_err());
}

#[test]
fn absent_read_creates_nothing_and_scope_keys_are_unambiguous() {
    let f = Fixture::new();
    assert_eq!(load(&f.0, "project", "snapshot").unwrap(), None);
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
    assert_ne!(
        location(&f.0, "a:b", "c").unwrap(),
        location(&f.0, "a", "b:c").unwrap()
    );
    assert!(location(&f.0, "project", "../escape").is_err());
}

#[test]
fn recovery_question_labels_require_exact_unique_run_and_snapshot_ownership() {
    let f = Fixture::new();
    let run = uuid::Uuid::new_v4().to_string();
    let other = uuid::Uuid::new_v4().to_string();
    let mut s = state();
    s["explorations"][0]["turns"][0]["runIds"] = json!([run]);
    save(&f.0, "project", "snapshot", None, s.clone()).unwrap();
    let before = fs::read(f.path()).unwrap();
    let found = questions_for_runs(&f.0, "project", "snapshot", &[&run, &other]);
    assert_eq!(found.len(), 1);
    assert_eq!(
        found[&run],
        s["explorations"][0]["turns"][0]["question"]
            .as_str()
            .unwrap()
    );
    assert!(questions_for_runs(&f.0, "other-project", "snapshot", &[&run]).is_empty());
    assert!(questions_for_runs(&f.0, "project", "other-snapshot", &[&run]).is_empty());
    assert_eq!(fs::read(f.path()).unwrap(), before);
    let mut second = s["explorations"][0]["turns"][0].clone();
    second["id"] = json!("second-turn");
    second["question"] = json!("A different question");
    s["explorations"][0]["turns"]
        .as_array_mut()
        .unwrap()
        .push(second);
    save(&f.0, "project", "snapshot", Some(1), s).unwrap();
    assert!(questions_for_runs(&f.0, "project", "snapshot", &[&run]).is_empty());
}

#[test]
fn missing_or_damaged_question_history_stays_unknown_without_writes() {
    let f = Fixture::new();
    let run = uuid::Uuid::new_v4().to_string();
    assert!(questions_for_runs(&f.0, "project", "snapshot", &[&run]).is_empty());
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
    f.put(b"{damaged conversation");
    assert!(questions_for_runs(&f.0, "project", "snapshot", &[&run]).is_empty());
    assert_eq!(fs::read(f.path()).unwrap(), b"{damaged conversation");
}

fn presentation_state() -> Value {
    let mut s = state();
    s["explorations"][0]["codeOpen"] = json!(false);
    s["explorations"][0]["fileFlow"] = json!({"path":"src/font.rs","showAll":true});
    s["explorations"][0]["turns"][0]["revisionId"] = json!("revision-one");
    s["explorations"][0]["turns"][0]["presentation"] = json!({
        "stories":[{"revisionId":"revision-one","sectionId":"section:two","sectionRevisionId":"section-revision:two","sceneId":"claim:second","positions":{"claim:first":12.4,"claim:second":3.2},"fullMap":true,"viewport":{"x":-34,"y":91,"zoom":0.6},"focusedNodeId":"claim:second","focusedEdgeId":"edge:one","focusLayoutVersion":6}],
        "diagrams":[{"revisionId":"revision-one","sectionId":"section:two","sectionRevisionId":"section-revision:two","viewport":{"x":250,"y":-91,"zoom":1.7},"focusedEdgeId":"edge:subtotal","layoutVersion":FLOW_DIAGRAM_LAYOUT_VERSION}]
    });
    s
}

#[test]
fn exact_scene_audio_map_pane_and_flow_positions_round_trip_as_metadata() {
    let f = Fixture::new();
    let s = presentation_state();
    let record = save(&f.0, "project", "snapshot", None, s.clone()).unwrap();
    assert_eq!(record.state, s);
    assert_eq!(load(&f.0, "project", "snapshot").unwrap(), Some(record));
    assert_eq!(fs::read_dir(f.path().parent().unwrap()).unwrap().count(), 2);
    let bytes = fs::read_to_string(f.path()).unwrap();
    assert!(!bytes.contains("autoPlay"));
    assert!(!bytes.contains("spokenText"));
}

#[test]
fn a_guide_waiting_between_playback_batches_can_be_saved() {
    let f = Fixture::new();
    let mut s = state();
    let guide_id = "79461b1a-e33d-40d2-be00-0ad86abec96a";
    s["explorations"][0]["turns"][0]["status"] = json!("waiting");
    s["explorations"][0]["turns"][0]["guideId"] = json!(guide_id);
    s["explorations"][0]["turns"][0]["guide"] = json!({
        "revision": 15,
        "id": guide_id,
        "projectId": "/Users/example/fontdone",
        "snapshotId": "snap-fontdone-main",
        "question": "How does this project work?",
        "title": "A representative code path",
        "status": "waitingForPlayback",
        "openingRunId": null,
        "openingRunIds": ["96c38bee-29db-4147-8a42-0f240daa9292"],
        "canContinue": true,
        "canExtend": false,
        "continuationOperations": 2,
        "units": [{
            "key": "render-path",
            "question": "How does rendering use a loaded slot?",
            "prerequisites": [],
            "status": "pending",
            "runIds": []
        }],
        "omissions": [],
        "capturedFiles": 12,
        "startingFiles": 4
    });

    let record = save(&f.0, "project", "snapshot", None, s.clone()).unwrap();

    assert_eq!(record.state, s);
    assert_eq!(load(&f.0, "project", "snapshot").unwrap(), Some(record));
}

#[test]
fn focused_story_layout_requires_the_current_version() {
    let f = Fixture::new();
    let s = presentation_state();
    let record = save(&f.0, "project", "snapshot", None, s.clone()).unwrap();
    assert_eq!(record.state, s);
    let before = fs::read(f.path()).unwrap();

    for version in [json!(5), json!(4), json!(3), json!(2)] {
        let mut unsupported = s.clone();
        unsupported["explorations"][0]["turns"][0]["presentation"]["stories"][0]["focusLayoutVersion"] =
            version;
        assert!(
            save(
                &f.0,
                "project",
                "snapshot",
                Some(record.revision),
                unsupported
            )
            .is_err()
        );
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }

    let mut unversioned = s;
    unversioned["explorations"][0]["turns"][0]["presentation"]["stories"][0]
        .as_object_mut()
        .unwrap()
        .remove("focusLayoutVersion");
    assert!(
        save(
            &f.0,
            "project",
            "snapshot",
            Some(record.revision),
            unversioned
        )
        .is_err()
    );
    assert_eq!(fs::read(f.path()).unwrap(), before);
}

#[test]
fn flow_diagram_layout_requires_the_current_layout_version() {
    let f = Fixture::new();
    let s = presentation_state();
    let record = save(&f.0, "project", "snapshot", None, s.clone()).unwrap();
    let before = fs::read(f.path()).unwrap();

    for version in [json!(1), json!(2), json!(3), json!(4), Value::Null] {
        let mut unsupported = s.clone();
        if version.is_null() {
            unsupported["explorations"][0]["turns"][0]["presentation"]["diagrams"][0]
                .as_object_mut()
                .unwrap()
                .remove("layoutVersion");
        } else {
            unsupported["explorations"][0]["turns"][0]["presentation"]["diagrams"][0]["layoutVersion"] =
                version;
        }
        assert!(
            save(
                &f.0,
                "project",
                "snapshot",
                Some(record.revision),
                unsupported
            )
            .is_err()
        );
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }
}

#[test]
fn focused_flow_edge_round_trips_without_a_saved_viewport() {
    let f = Fixture::new();
    let mut s = presentation_state();
    s["explorations"][0]["turns"][0]["presentation"]["diagrams"][0]
        .as_object_mut()
        .unwrap()
        .remove("viewport");
    let record = save(&f.0, "project", "snapshot", None, s).unwrap();
    assert_eq!(
        record.state["explorations"][0]["turns"][0]["presentation"]["diagrams"][0]["focusedEdgeId"],
        "edge:subtotal"
    );
    let reopened = load(&f.0, "project", "snapshot").unwrap().unwrap();
    let diagram = &reopened.state["explorations"][0]["turns"][0]["presentation"]["diagrams"][0];
    assert_eq!(diagram["focusedEdgeId"], "edge:subtotal");
    assert_eq!(diagram["viewport"], json!({"x":0,"y":0,"zoom":1}));
}

#[test]
fn opening_stale_reader_layouts_preserves_answers_and_discards_only_positions() {
    let f = Fixture::new();
    let state = presentation_state();
    let record = save(&f.0, "project", "snapshot", None, state).unwrap();
    let mut stored = serde_json::to_value(record).unwrap();
    stored["state"]["explorations"][0]["turns"][0]["presentation"]["stories"][0]["focusLayoutVersion"] =
        json!(2);
    stored["state"]["explorations"][0]["turns"][0]["presentation"]["diagrams"][0]["layoutVersion"] =
        json!(3);
    fs::write(&f.path(), serde_json::to_vec(&stored).unwrap()).unwrap();
    let original_bytes = fs::read(f.path()).unwrap();

    let restored = load(&f.0, "project", "snapshot")
        .unwrap()
        .expect("stale optional reader layouts must not hide saved answers");
    let turn = &restored.state["explorations"][0]["turns"][0];
    assert_eq!(
        turn["question"],
        "What happens to the font bytes?\nShow the failure path."
    );
    assert_eq!(turn["status"], "stopped");
    assert_eq!(turn["presentation"]["stories"], json!([]));
    assert_eq!(turn["presentation"]["diagrams"], json!([]));
    assert_eq!(
        fs::read(f.path()).unwrap(),
        original_bytes,
        "reader-position recovery must not rewrite the durable conversation"
    );
}

#[test]
fn opening_malformed_bookmark_and_scroll_positions_preserves_the_answer_and_record_bytes() {
    let f = Fixture::new();
    let mut original = presentation_state();
    original["explorations"][0]["turns"][0]["answerVersions"] = json!([
        {"originalRevisionId":"revision-one","revisionId":"revision:accepted","selected":true,"reading":{"guide":"section:two","flow":"section:three"}}
    ]);
    save(&f.0, "project", "snapshot", None, original).unwrap();

    let mut stored: Value = serde_json::from_slice(&fs::read(f.path()).unwrap()).unwrap();
    let exploration = &mut stored["state"]["explorations"][0];
    exploration["scrollPositions"] =
        json!({"guide":"not-a-number","flow":240,"history":-1,"unknown":50});
    let turn = &mut exploration["turns"][0];
    turn["reading"] = json!({
        "guide":{"revisionId":"revision-one","sectionId":"bad\nid"},
        "flow":{"revisionId":"revision-one","sectionId":"section:two"},
        "history":{"revisionId":"revision-one","sectionId":"section:two"}
    });
    turn["answerVersions"][0]["reading"] = json!({
        "guide":"section:two",
        "flow":{"unexpected":"shape"},
        "history":"section:three"
    });
    fs::write(f.path(), serde_json::to_vec(&stored).unwrap()).unwrap();
    let original_bytes = fs::read(f.path()).unwrap();

    let restored = load(&f.0, "project", "snapshot")
        .unwrap()
        .expect("damaged reader progress must not hide the saved answer");
    let restored_exploration = &restored.state["explorations"][0];
    let restored_turn = &restored_exploration["turns"][0];

    assert_eq!(restored_exploration["scrollPositions"], json!({"flow":240}));
    assert_eq!(
        restored_turn["reading"],
        json!({"flow":{"revisionId":"revision-one","sectionId":"section:two"}})
    );
    assert_eq!(
        restored_turn["answerVersions"][0]["reading"],
        json!({"guide":"section:two"})
    );
    assert_eq!(
        restored_turn["question"],
        "What happens to the font bytes?\nShow the failure path."
    );
    assert_eq!(
        fs::read(f.path()).unwrap(),
        original_bytes,
        "reader recovery must not rewrite the durable conversation"
    );
}

#[test]
fn opening_malformed_visual_positions_preserves_the_answer_and_valid_progress() {
    let f = Fixture::new();
    let original = presentation_state();
    save(&f.0, "project", "snapshot", None, original).unwrap();

    let mut stored: Value = serde_json::from_slice(&fs::read(f.path()).unwrap()).unwrap();
    let presentation = &mut stored["state"]["explorations"][0]["turns"][0]["presentation"];
    let story = &mut presentation["stories"][0];
    story["positions"]["claim:first"] = json!("not-a-position");
    story["viewport"]["zoom"] = json!(10.0);
    story["fullMap"] = json!("true");
    story["focusedNodeId"] = json!("bad\nnode");
    presentation["diagrams"][0]["viewport"]["x"] = json!(10_000_001);
    presentation["diagrams"][0]["focusedEdgeId"] = json!("bad\nedge");
    fs::write(f.path(), serde_json::to_vec(&stored).unwrap()).unwrap();
    let original_bytes = fs::read(f.path()).unwrap();

    let restored = load(&f.0, "project", "snapshot")
        .unwrap()
        .expect("damaged visual progress must not hide the saved answer");
    let turn = &restored.state["explorations"][0]["turns"][0];
    let story = &turn["presentation"]["stories"][0];

    assert_eq!(
        story["positions"],
        json!({"claim:second":3.2}),
        "keep valid per-claim progress while dropping only the damaged entry"
    );
    assert!(
        story.get("viewport").is_none(),
        "invalid viewport is discarded"
    );
    assert_eq!(
        story["fullMap"], false,
        "invalid layout preference gets a safe default"
    );
    assert!(story.get("focusedNodeId").is_none());
    assert_eq!(story["focusedEdgeId"], "edge:one");
    assert_eq!(
        turn["presentation"]["diagrams"][0]["viewport"],
        json!({"x":0,"y":0,"zoom":1}),
        "reset only the malformed Flow viewport"
    );
    assert!(
        turn["presentation"]["diagrams"][0]
            .get("focusedEdgeId")
            .is_none()
    );
    assert_eq!(
        turn["question"],
        "What happens to the font bytes?\nShow the failure path."
    );
    assert_eq!(fs::read(f.path()).unwrap(), original_bytes);
}

#[test]
fn opening_visual_position_with_unowned_revision_still_rejects_the_record() {
    let f = Fixture::new();
    save(&f.0, "project", "snapshot", None, presentation_state()).unwrap();

    let mut stored: Value = serde_json::from_slice(&fs::read(f.path()).unwrap()).unwrap();
    stored["state"]["explorations"][0]["turns"][0]["presentation"]["stories"][0]["revisionId"] =
        json!("revision:foreign");
    fs::write(f.path(), serde_json::to_vec(&stored).unwrap()).unwrap();
    let original_bytes = fs::read(f.path()).unwrap();

    assert!(load(&f.0, "project", "snapshot").is_err());
    assert_eq!(fs::read(f.path()).unwrap(), original_bytes);
}

#[test]
fn selected_answer_versions_and_independent_reading_round_trip_without_rewriting_receipts() {
    let f = Fixture::new();
    let mut s = presentation_state();
    let t = &mut s["explorations"][0]["turns"][0];
    t["answerVersions"] = json!([
        {"originalRevisionId":"revision-one","revisionId":"revision-one","selected":false,"reading":{"guide":"section:two","flow":"section:one"}},
        {"originalRevisionId":"revision-one","revisionId":"revision:accepted","selected":true,"reading":{"guide":"section:three"}}
    ]);
    t["presentation"]["stories"][0]["revisionId"] = json!("revision:accepted");
    t["reading"]["guide"] = json!({"revisionId":"revision:accepted","sectionId":"section:three"});
    let record = save(&f.0, "project", "snapshot", None, s.clone()).unwrap();
    let loaded = load(&f.0, "project", "snapshot").unwrap().unwrap();
    assert_eq!(loaded, record);
    assert_eq!(loaded.state, s);
    assert_eq!(
        loaded.state["explorations"][0]["turns"][0]["revisionId"],
        "revision-one"
    );
}

#[test]
fn invalid_answer_version_preferences_cannot_replace_a_valid_journal() {
    let f = Fixture::new();
    let mut good = presentation_state();
    let entry = json!({"originalRevisionId":"revision-one","revisionId":"revision:accepted","selected":true,"reading":{"guide":"section:one"}});
    good["explorations"][0]["turns"][0]["answerVersions"] = json!([entry.clone()]);
    save(&f.0, "project", "snapshot", None, good.clone()).unwrap();
    let before = fs::read(f.path()).unwrap();
    let mut variants = vec![
        Value::Null,
        json!({}),
        json!(vec![entry.clone(); 257]),
        json!([entry.clone(), entry.clone()]),
    ];
    for (field, value) in [
        ("originalRevisionId", json!("revision:unrelated")),
        ("revisionId", json!("bad\nkey")),
        ("selected", json!("yes")),
        ("reading", json!({"guide":"bad\nsection"})),
        ("reading", json!({"history":"section:one"})),
        ("html", json!("<script>untrusted</script>")),
    ] {
        let mut bad = entry.clone();
        bad[field] = value;
        variants.push(json!([bad]));
    }
    let mut other = entry.clone();
    other["revisionId"] = json!("revision:other");
    variants.push(json!([entry, other])); // Two selected versions of one original.
    for entries in variants {
        let mut bad = good.clone();
        bad["explorations"][0]["turns"][0]["answerVersions"] = entries;
        assert!(save(&f.0, "project", "snapshot", Some(1), bad).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }
}

#[test]
fn invalid_unowned_or_oversized_reader_positions_preserve_the_valid_journal() {
    let f = Fixture::new();
    let good = presentation_state();
    save(&f.0, "project", "snapshot", None, good.clone()).unwrap();
    let before = fs::read(f.path()).unwrap();
    let mut cases = vec![];
    for (pointer, value) in [
        ("/explorations/0/codeOpen", json!("false")),
        ("/explorations/0/fileFlow/path", json!("../outside")),
        ("/explorations/0/fileFlow/showAll", Value::Null),
        (
            "/explorations/0/turns/0/presentation/stories/0/revisionId",
            json!("revision:foreign"),
        ),
        (
            "/explorations/0/turns/0/presentation/stories/0/sceneId",
            json!("bad\nscene"),
        ),
        (
            "/explorations/0/turns/0/presentation/stories/0/fullMap",
            json!(1),
        ),
        (
            "/explorations/0/turns/0/presentation/stories/0/positions/claim:first",
            json!(-1),
        ),
        (
            "/explorations/0/turns/0/presentation/stories/0/positions/claim:first",
            json!(3601),
        ),
        (
            "/explorations/0/turns/0/presentation/stories/0/viewport/zoom",
            json!(0.09),
        ),
        (
            "/explorations/0/turns/0/presentation/diagrams/0/viewport/x",
            json!(10000001),
        ),
        (
            "/explorations/0/turns/0/presentation/diagrams/0/viewport/zoom",
            json!(4.01),
        ),
    ] {
        let mut s = good.clone();
        *s.pointer_mut(pointer).unwrap() = value;
        cases.push(s);
    }
    let mut duplicate = good.clone();
    let entries = duplicate["explorations"][0]["turns"][0]["presentation"]["stories"]
        .as_array_mut()
        .unwrap();
    entries.push(entries[0].clone());
    cases.push(duplicate);
    let mut copied = good.clone();
    copied["explorations"][0]["turns"][0]["presentation"]["stories"][0]["autoPlay"] = json!(true);
    cases.push(copied);
    let mut oversized = good.clone();
    oversized["explorations"][0]["turns"][0]["presentation"]["stories"][0]["positions"] =
        Value::Object((0..129).map(|i| (format!("claim:{i}"), json!(0))).collect());
    cases.push(oversized);
    for s in cases {
        assert!(save(&f.0, "project", "snapshot", Some(1), s).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }
}

#[test]
fn source_metadata_preserves_unicode_spaces_and_colons_without_opening_live_paths() {
    let f = Fixture::new();
    let mut s = state();
    s["file"] = json!("字型:demo/font file.rs");
    s["explorations"][0]["turns"][0]["context"][0]["path"] = json!("字型:demo/font file.rs");
    let stored = save(&f.0, "project", "snapshot", None, s.clone()).unwrap();
    assert_eq!(stored.state, s);
    assert!(
        fs::read_dir(&f.0)
            .unwrap()
            .all(|e| e.unwrap().file_name() == "workspace-conversations")
    );
}

#[test]
fn five_levels_siblings_views_sections_and_source_survive_without_copying_answers() {
    let f = Fixture::new();
    let mut value = state();
    for n in 1..=5 {
        let parent = if n == 1 {
            "root".into()
        } else {
            format!("child-{}", n - 1)
        };
        let origin = if n == 1 {
            "turn-root".into()
        } else {
            format!("turn-{}", n - 1)
        };
        value["explorations"].as_array_mut().unwrap().push(json!({"id":format!("child-{n}"),"parentId":parent,"originTurnId":origin,"title":format!("Depth {n}"),"view":(["guide","flow","history"][n%3]),"turns":[{"id":format!("turn-{n}"),"question":"Follow this decision","context":[],"status":"interrupted"}],"file":"src/font.rs","scrollPositions":{"history":100*n}}));
    }
    value["explorations"].as_array_mut().unwrap().push(json!({"id":"sibling","parentId":"root","originTurnId":"turn-root","title":"Sibling","view":"flow","turns":[]}));
    value["currentId"] = json!("child-5");
    let record = save(&f.0, "project", "snapshot", None, value.clone()).unwrap();
    assert_eq!(record.revision, 1);
    assert_eq!(record.state, value);
    assert_eq!(load(&f.0, "project", "snapshot").unwrap(), Some(record));
    assert!(
        !fs::read_to_string(f.path())
            .unwrap()
            .contains("chapterContent")
    );
}

#[test]
fn serial_revisions_exact_replay_and_stale_writers_preserve_newer_state() {
    let f = Fixture::new();
    let one = save(&f.0, "project", "snapshot", None, state()).unwrap();
    let before = fs::read(f.path()).unwrap();
    assert_eq!(
        save(&f.0, "project", "snapshot", None, state()).unwrap(),
        one
    );
    assert_eq!(fs::read(f.path()).unwrap(), before);
    let mut newer = state();
    newer["explorations"][0]["view"] = json!("flow");
    let two = save(&f.0, "project", "snapshot", Some(1), newer).unwrap();
    assert_eq!(two.revision, 2);
    let before = fs::read(f.path()).unwrap();
    assert_eq!(
        save(&f.0, "project", "snapshot", Some(1), state()).unwrap_err(),
        CONFLICT
    );
    assert_eq!(fs::read(f.path()).unwrap(), before);
    assert!(save(&f.0, "project", "snapshot", None, state()).is_err());
}

#[test]
fn invalid_trees_contexts_scopes_and_unknown_content_cannot_replace_a_valid_file() {
    let f = Fixture::new();
    save(&f.0, "project", "snapshot", None, state()).unwrap();
    let before = fs::read(f.path()).unwrap();
    let mut cases = vec![];
    for (field, value) in [
        ("version", json!(2)),
        ("projectId", json!("another")),
        ("snapshotId", json!("another")),
        ("currentId", json!("missing")),
        ("file", json!("../outside")),
        ("content", json!("must not be copied")),
    ] {
        let mut s = state();
        s[field] = value;
        cases.push(s);
    }
    for (field, value) in [
        ("id", json!("other")),
        ("parentId", json!("root")),
        ("originTurnId", json!("turn-root")),
        ("view", json!("settings")),
        ("focusedTurnId", json!("foreign")),
        ("scrollPositions", json!({"guide":10000001})),
        ("reveal", json!({"path":"src/font.rs","from":8,"to":3})),
    ] {
        let mut s = state();
        s["explorations"][0][field] = value;
        cases.push(s);
    }
    for (field, value) in [
        ("id", json!("")),
        ("status", json!("complete")),
        ("question", json!("\u{0000}")),
        ("guide", json!({"duplicated":"job state"})),
        ("runIds", json!(["not-a-run"])),
        (
            "reading",
            json!({"guide":{"revisionId":"ok","sectionId":""}}),
        ),
        ("outcomeNotice", json!({"read":"not-a-hash"})),
        ("failureCode", json!("raw provider failure")),
    ] {
        let mut s = state();
        s["explorations"][0]["turns"][0][field] = value;
        cases.push(s);
    }
    let mut duplicate = state();
    let root = duplicate["explorations"][0].clone();
    duplicate["explorations"].as_array_mut().unwrap().push(root);
    cases.push(duplicate);
    let mut duplicate_turn = state();
    let t = duplicate_turn["explorations"][0]["turns"][0].clone();
    duplicate_turn["explorations"][0]["turns"]
        .as_array_mut()
        .unwrap()
        .push(t);
    cases.push(duplicate_turn);
    for s in cases {
        assert!(save(&f.0, "project", "snapshot", Some(1), s).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }
}

#[test]
fn cycles_foreign_origins_and_duplicate_turn_ownership_are_rejected() {
    let mut s = state();
    s["explorations"].as_array_mut().unwrap().push(json!({"id":"child","parentId":"root","originTurnId":"unknown","title":"Child","view":"guide","turns":[]}));
    assert!(validate(&s, "project", "snapshot").is_err());
    s["explorations"][1]["originTurnId"] = json!("turn-root");
    assert!(validate(&s, "project", "snapshot").is_ok());
    s["explorations"][1]["parentId"] = json!("child");
    assert!(validate(&s, "project", "snapshot").is_err());
    s["explorations"][1]["parentId"] = json!("root");
    let t = s["explorations"][0]["turns"][0].clone();
    s["explorations"][1]["turns"]
        .as_array_mut()
        .unwrap()
        .push(t);
    assert!(validate(&s, "project", "snapshot").is_err());
}

#[test]
fn reference_bounds_are_checked_without_requiring_live_source_or_authorizing_it() {
    let run = uuid::Uuid::new_v4().to_string();
    let generation = format!("generation:{}", uuid::Uuid::new_v4());
    let mut s = state();
    let t = &mut s["explorations"][0]["turns"][0];
    t["runIds"] = json!([run]);
    t["publishedSections"] = json!([{"runId":run,"key":"opening","order":0,"generationId":generation,"revisionId":"revision:one","supersedes":null,"contextElapsedMs":2,"runElapsedMs":null}]);
    assert!(validate(&s, "project", "snapshot").is_ok());
    s["explorations"][0]["turns"][0]["publishedSections"][0]["runId"] =
        json!(uuid::Uuid::new_v4().to_string());
    assert!(validate(&s, "project", "snapshot").is_err());
}

#[test]
fn native_admission_rejects_metadata_the_legacy_reader_would_drop() {
    let f = Fixture::new();
    save(&f.0, "project", "snapshot", None, state()).unwrap();
    let before = fs::read(f.path()).unwrap();
    let run = "abcdefab-1234-5678-9abc-123456789abc";
    let mut good = state();
    good["explorations"][0]["turns"][0]["runIds"] = json!([run]);
    good["explorations"][0]["turns"][0]["publishedSections"] = json!([{"runId":run,"key":"opening","order":0,"generationId":format!("generation:{run}"),"revisionId":"revision:one","supersedes":null,"contextElapsedMs":2,"runElapsedMs":null}]);
    assert!(validate(&good, "project", "snapshot").is_ok());
    let mut cases = vec![];
    for key in ["supersedes", "runElapsedMs"] {
        let mut s = good.clone();
        s["explorations"][0]["turns"][0]["publishedSections"][0]
            .as_object_mut()
            .unwrap()
            .remove(key);
        cases.push(s);
    }
    let mut upper = state();
    upper["explorations"][0]["turns"][0]["guideId"] = json!(run.to_uppercase());
    cases.push(upper);
    let mut note = state();
    note["explorations"][0]["turns"][0]["notes"] = json!(["line one\nline two"]);
    cases.push(note);
    let mut unicode_note = state();
    unicode_note["explorations"][0]["turns"][0]["notes"] = json!(["🧭".repeat(501)]);
    cases.push(unicode_note);
    for s in cases {
        assert!(save(&f.0, "project", "snapshot", Some(1), s).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }
}

#[test]
fn corrupt_future_duplicate_scope_and_oversized_storage_are_preserved() {
    let f = Fixture::new();
    let good = save(&f.0, "project", "snapshot", None, state()).unwrap();
    let mut future = serde_json::to_value(&good).unwrap();
    future["schemaVersion"] = json!(2);
    let mut foreign = serde_json::to_value(&good).unwrap();
    foreign["state"]["snapshotId"] = json!("foreign");
    let mut cases = vec![
        b"{broken".to_vec(),
        serde_json::to_vec(&future).unwrap(),
        serde_json::to_vec(&foreign).unwrap(),
        br#"{"schemaVersion":1,"schemaVersion":1}"#.to_vec(),
    ];
    cases.push(vec![b' '; MAX_BYTES + 1]);
    for bytes in cases {
        f.put(&bytes);
        assert!(load(&f.0, "project", "snapshot").is_err());
        assert!(save(&f.0, "project", "snapshot", Some(1), state()).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), bytes);
    }
}

#[cfg(unix)]
#[test]
fn private_files_held_lease_and_symlinks_cannot_damage_other_data() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let f = Fixture::new();
    save(&f.0, "project", "snapshot", None, state()).unwrap();
    assert_eq!(
        fs::metadata(f.path()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(f.path().parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.path().with_extension("lock"))
        .unwrap();
    let _lease = crate::file_lease::try_exclusive(lock).unwrap().unwrap();
    let before = fs::read(f.path()).unwrap();
    assert_eq!(
        save(&f.0, "project", "snapshot", Some(1), state()).unwrap_err(),
        SAVE_BUSY
    );
    assert_eq!(fs::read(f.path()).unwrap(), before);
    drop(_lease);
    let outside = f.0.join("outside");
    fs::write(&outside, b"preserved answer").unwrap();
    fs::remove_file(f.path()).unwrap();
    symlink(&outside, f.path()).unwrap();
    assert!(load(&f.0, "project", "snapshot").is_err());
    assert!(save(&f.0, "project", "snapshot", Some(1), state()).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"preserved answer");
}
