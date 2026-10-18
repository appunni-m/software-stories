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
fn source_metadata_preserves_unicode_spaces_and_colons_without_opening_live_paths() {
    let f = Fixture::new();
    let mut s = state();
    s["file"] = json!("字型:demo/font file.rs");
    s["explorations"][0]["turns"][0]["context"][0]["path"] = json!("字型:demo/font file.rs");
    let stored = save(&f.0, "project", "snapshot", None, s.clone()).unwrap();
    assert_eq!(stored.state, s);
    assert!(fs::read_dir(&f.0)
        .unwrap()
        .all(|e| e.unwrap().file_name() == "workspace-conversations"));
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
    assert!(!fs::read_to_string(f.path())
        .unwrap()
        .contains("chapterContent"));
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
    use std::os::unix::fs::{symlink, PermissionsExt};
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
    assert!(save(&f.0, "project", "snapshot", Some(1), state()).is_err());
    drop(_lease);
    let outside = f.0.join("outside");
    fs::write(&outside, b"preserved answer").unwrap();
    fs::remove_file(f.path()).unwrap();
    symlink(&outside, f.path()).unwrap();
    assert!(load(&f.0, "project", "snapshot").is_err());
    assert!(save(&f.0, "project", "snapshot", Some(1), state()).is_err());
    assert_eq!(fs::read(&outside).unwrap(), b"preserved answer");
}
