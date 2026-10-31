use super::*;
use std::path::PathBuf;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("code-atlas-notices-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        Self(root)
    }
    fn path(&self) -> PathBuf {
        self.0.join("app-notices/history.json")
    }
    fn save(&self, value: &serde_json::Value) {
        fs::create_dir_all(self.0.join("app-notices")).unwrap();
        fs::write(self.path(), serde_json::to_vec(value).unwrap()).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn input(message: &str) -> Input {
    Input {
        title: "Saved update".into(),
        message: message.into(),
        tone: Tone::Info,
        project_id: Some("project-one".into()),
        snapshot_id: Some("snap-one".into()),
        occurrence_id: None,
        target: Some(Target::OpenExploration {
            exploration_id: "root".into(),
        }),
    }
}

#[test]
fn conversation_retry_requires_scope_and_cannot_store_a_command() {
    let mut value = input("Try saving locally");
    value.target = Some(Target::RetryConversation {});
    assert!(validate(&value).is_ok());
    value.snapshot_id = None;
    assert!(validate(&value).is_err());
    assert!(
        serde_json::from_value::<Target>(
            serde_json::json!({"kind":"retryConversation","command":"run an agent"})
        )
        .is_err()
    );
}
fn publish(f: &Fixture, value: Input) -> History {
    apply(&f.0, Operation::Publish { input: value }).unwrap()
}

#[test]
fn empty_read_is_local_and_creates_no_storage() {
    let f = Fixture::new();
    assert_eq!(list(&f.0).unwrap(), History::default());
    assert_eq!(fs::read_dir(&f.0).unwrap().count(), 0);
}

#[test]
fn canonical_key_matches_the_renderer_unicode_and_null_contract() {
    assert_eq!(
        key(&input("Font bytes → pixels")).unwrap(),
        "d18911fbf06bfa438a39042ba1c956a52025741c0dd8cfab89be057b27879517"
    );
    let mut value = input("A \"quoted\" path <file> \\ 🧭");
    value.project_id = None;
    value.snapshot_id = None;
    value.target = None;
    let encoded = serde_json::to_string(&value).unwrap();
    assert!(encoded.contains("\"projectId\":null"));
    assert!(encoded.contains("🧭"));
}

#[test]
fn independent_notices_survive_reload_and_repeat_preserves_read_state() {
    let f = Fixture::new();
    let first = publish(&f, input("One"));
    let id = first.items[0].id.clone();
    apply(&f.0, Operation::Read { id: id.clone() }).unwrap();
    let before = fs::read(f.path()).unwrap();
    let same = publish(&f, input("One"));
    assert!(same.items[0].read);
    assert_eq!(fs::read(f.path()).unwrap(), before);
    let second = publish(&f, input("Two"));
    assert_eq!(list(&f.0).unwrap(), second);
    assert_eq!(second.items.len(), 2);
    assert_eq!(second.items[0].id, id);
}

#[test]
fn dismissal_survives_republication_without_erasing_unrelated_work() {
    let f = Fixture::new();
    let one = publish(&f, input("One"));
    let id = one.items[0].id.clone();
    publish(&f, input("Two"));
    fs::write(f.0.join("saved-answer"), b"original answer bytes").unwrap();
    let dismissed = apply(&f.0, Operation::Dismiss { id: id.clone() }).unwrap();
    assert_eq!(dismissed.items.len(), 1);
    assert_eq!(dismissed.dismissed, vec![id.clone()]);
    let before = fs::read(f.path()).unwrap();
    assert_eq!(publish(&f, input("One")), dismissed);
    assert_eq!(fs::read(f.path()).unwrap(), before);
    assert_eq!(
        fs::read(f.0.join("saved-answer")).unwrap(),
        b"original answer bytes"
    );
    assert_eq!(apply(&f.0, Operation::Dismiss { id }).unwrap(), dismissed);
}

#[test]
fn group_dismissal_is_atomic_idempotent_and_preserves_unrelated_notices() {
    let f = Fixture::new();
    let one = publish(&f, input("One"));
    let first = one.items[0].id.clone();
    let two = publish(&f, input("Two"));
    let second = two.items[1].id.clone();
    publish(&f, input("Three"));
    let before = list(&f.0).unwrap();

    let operation: Operation = serde_json::from_value(serde_json::json!({
        "kind": "dismissMany",
        "ids": [first.clone(), second.clone()]
    }))
    .unwrap();
    let dismissed = apply(&f.0, operation).unwrap();
    assert_eq!(dismissed.items.len(), 1);
    assert_eq!(dismissed.items[0].input.message, "Three");
    assert_eq!(dismissed.dismissed, vec![first.clone(), second.clone()]);
    let bytes = fs::read(f.path()).unwrap();
    assert_eq!(
        apply(
            &f.0,
            Operation::DismissMany {
                ids: vec![first, second],
            }
        )
        .unwrap(),
        dismissed
    );
    assert_eq!(fs::read(f.path()).unwrap(), bytes);
    assert_eq!(before.items.len(), 3);
}

#[test]
fn group_dismissal_rejects_invalid_batches_and_evicts_oldest_acknowledgments() {
    let f = Fixture::new();
    let notice = publish(&f, input("Keep me"));
    let id = notice.items[0].id.clone();
    let before = fs::read(f.path()).unwrap();
    for ids in [
        vec![],
        vec!["bad".to_owned()],
        vec![id.clone(), id.clone()],
        vec!["a".repeat(64); MAX_DISMISSED + 1],
    ] {
        assert!(apply(&f.0, Operation::DismissMany { ids }).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }
    let mut full = notice;
    full.dismissed = (0..=MAX_DISMISSED)
        .map(|n| format!("{n:064x}"))
        .filter(|candidate| candidate != &id)
        .take(MAX_DISMISSED)
        .collect();
    let oldest = full.dismissed[0].clone();
    f.save(&serde_json::to_value(&full).unwrap());
    let dismissed = apply(
        &f.0,
        Operation::DismissMany {
            ids: vec![id.clone()],
        },
    )
    .unwrap();
    assert!(dismissed.items.is_empty());
    assert_eq!(dismissed.dismissed.len(), MAX_DISMISSED);
    assert!(!dismissed.dismissed.contains(&oldest));
    assert_eq!(dismissed.dismissed.last(), Some(&id));
}

#[test]
fn group_read_is_atomic_idempotent_and_preserves_other_notices_and_dismissals() {
    let f = Fixture::new();
    let one = publish(&f, input("One"));
    let first = one.items[0].id.clone();
    let two = publish(&f, input("Two"));
    let second = two.items[1].id.clone();
    publish(&f, input("Three"));
    let before = list(&f.0).unwrap();

    let read = apply(
        &f.0,
        Operation::ReadMany {
            ids: vec![first.clone(), second.clone()],
        },
    )
    .unwrap();
    assert_eq!(read.items.len(), 3);
    assert!(read.items[0].read);
    assert!(read.items[1].read);
    assert!(!read.items[2].read);
    assert!(read.dismissed.is_empty());
    let bytes = fs::read(f.path()).unwrap();
    assert_eq!(
        apply(
            &f.0,
            Operation::ReadMany {
                ids: vec![first, second],
            }
        )
        .unwrap(),
        read
    );
    assert_eq!(fs::read(f.path()).unwrap(), bytes);
    assert_eq!(before.items.iter().filter(|item| item.read).count(), 0);
}

#[test]
fn group_read_rejects_invalid_batches_without_mutation() {
    let f = Fixture::new();
    let notice = publish(&f, input("Keep me"));
    let id = notice.items[0].id.clone();
    let before = fs::read(f.path()).unwrap();
    for ids in [
        vec![],
        vec!["bad".to_owned()],
        vec![id.clone(), id.clone()],
        vec!["a".repeat(64); MAX_DISMISSED + 1],
    ] {
        assert!(apply(&f.0, Operation::ReadMany { ids }).is_err());
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }
}

#[test]
fn scope_and_action_changes_are_distinct_without_confusing_repository_paths() {
    let f = Fixture::new();
    let mut first = input("Same message");
    first.project_id = Some("/Users/dev/work/Font App/字型".into());
    publish(&f, first.clone());
    first.project_id = Some("/Users/dev/work/another".into());
    publish(&f, first.clone());
    first.target = Some(Target::RetryRecents {});
    let final_state = publish(&f, first);
    assert_eq!(final_state.items.len(), 3);
}

#[test]
fn invalid_scope_text_and_action_metadata_are_rejected() {
    let mut cases = Vec::new();
    let mut v = input("Valid");
    v.project_id = None;
    cases.push(v);
    let mut v = input("Valid");
    v.snapshot_id = Some("../snapshot".into());
    cases.push(v);
    let mut v = input("Valid");
    v.message = "unsafe\0text".into();
    cases.push(v);
    let mut v = input("Valid");
    v.title = "🧭".repeat(101);
    cases.push(v);
    let mut v = input("Valid");
    v.project_id = Some("x".repeat(4097));
    cases.push(v);
    for value in cases {
        assert!(key(&value).is_err());
    }
    let mut value = input("🧭".repeat(700).as_str());
    value.title = "🧭".repeat(100);
    assert!(key(&value).is_ok());
    let invalid = serde_json::json!({"kind":"openRepository","command":"send model request"});
    assert!(serde_json::from_value::<Target>(invalid).is_err());
}

#[test]
fn corruption_future_schemas_duplicate_keys_and_changed_identity_preserve_original_bytes() {
    let f = Fixture::new();
    let history = publish(&f, input("Original"));
    let good = serde_json::to_value(history).unwrap();
    let mut future = good.clone();
    future["schemaVersion"] = 2.into();
    let mut identity = good.clone();
    identity["items"][0]["input"]["message"] = "changed".into();
    let mut duplicate = good.clone();
    duplicate["items"]
        .as_array_mut()
        .unwrap()
        .push(good["items"][0].clone());
    let mut extra = good.clone();
    extra["consent"] = true.into();
    for value in [future, identity, duplicate, extra] {
        f.save(&value);
        let before = fs::read(f.path()).unwrap();
        assert!(list(&f.0).is_err());
        assert!(
            apply(
                &f.0,
                Operation::Publish {
                    input: input("New")
                }
            )
            .is_err()
        );
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }
    fs::write(
        f.path(),
        b"{\"schemaVersion\":1,\"schemaVersion\":1,\"items\":[],\"dismissed\":[]}",
    )
    .unwrap();
    assert!(list(&f.0).is_err());
    fs::write(f.path(), vec![b' '; MAX_BYTES + 1]).unwrap();
    assert!(list(&f.0).is_err());
    assert!(
        apply(
            &f.0,
            Operation::Publish {
                input: input("New")
            }
        )
        .is_err()
    );
    assert_eq!(
        fs::metadata(f.path()).unwrap().len(),
        (MAX_BYTES + 1) as u64
    );
}

#[test]
fn maximum_sized_records_at_the_thousand_item_cap_fit_the_bounded_history() {
    let f = Fixture::new();
    let mut history = History::default();
    for n in 0..MAX_ITEMS {
        let mut notice = input(&"🧭".repeat(700));
        notice.title = "🧭".repeat(100);
        notice.project_id = Some("p".repeat(4096));
        notice.snapshot_id = Some("s".repeat(256));
        notice.occurrence_id = Some(format!("{n:0256}"));
        notice.target = Some(Target::OpenExploration {
            exploration_id: "x".repeat(256),
        });
        history.items.push(Item {
            id: key(&notice).unwrap(),
            input: notice,
            created_at_unix_ms: n as u64 + 1,
            read: false,
        });
    }
    let value = serde_json::to_value(&history).unwrap();
    let bytes = serde_json::to_vec(&value).unwrap();
    assert!(bytes.len() <= MAX_BYTES);
    f.save(&value);
    assert_eq!(list(&f.0).unwrap(), history);
}

#[test]
fn capacity_never_evicts_unread_work_and_dismissal_frees_one_visible_slot() {
    let f = Fixture::new();
    for n in 0..MAX_ITEMS {
        publish(&f, input(&format!("Update {n}")));
    }
    let before = fs::read(f.path()).unwrap();
    assert!(
        apply(
            &f.0,
            Operation::Publish {
                input: input("Overflow")
            }
        )
        .is_err()
    );
    assert_eq!(fs::read(f.path()).unwrap(), before);
    let oldest = list(&f.0).unwrap().items[0].id.clone();
    apply(&f.0, Operation::Dismiss { id: oldest }).unwrap();
    assert_eq!(publish(&f, input("Overflow")).items.len(), MAX_ITEMS);
    assert_eq!(publish(&f, input("Update 0")).items.len(), MAX_ITEMS);
}

#[test]
fn acknowledgment_retention_keeps_visible_updates_dismissible() {
    let f = Fixture::new();
    let mut history = publish(&f, input("Keep me"));
    let id = history.items[0].id.clone();
    history.dismissed = (0..=MAX_DISMISSED)
        .map(|n| format!("{n:064x}"))
        .filter(|candidate| candidate != &id)
        .take(MAX_DISMISSED)
        .collect();
    let oldest = history.dismissed[0].clone();
    f.save(&serde_json::to_value(&history).unwrap());
    let dismissed = apply(&f.0, Operation::Dismiss { id: id.clone() }).unwrap();
    assert!(dismissed.items.is_empty());
    assert_eq!(dismissed.dismissed.len(), MAX_DISMISSED);
    assert!(!dismissed.dismissed.contains(&oldest));
    assert_eq!(dismissed.dismissed.last(), Some(&id));
}

#[test]
fn unknown_acknowledgments_do_not_create_tombstones_or_rewrite_history() {
    let f = Fixture::new();
    publish(&f, input("Original"));
    let before = fs::read(f.path()).unwrap();
    for op in [
        Operation::Read { id: "0".repeat(64) },
        Operation::Dismiss { id: "0".repeat(64) },
    ] {
        apply(&f.0, op).unwrap();
        assert_eq!(fs::read(f.path()).unwrap(), before);
    }
}

#[cfg(unix)]
#[test]
fn held_lease_and_symlinked_paths_cannot_replace_another_owners_bytes() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let f = Fixture::new();
    publish(&f, input("Original"));
    let before = fs::read(f.path()).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(f.0.join("app-notices/history.lock"))
        .unwrap();
    let lease = crate::file_lease::try_exclusive(lock).unwrap().unwrap();
    assert!(
        apply(
            &f.0,
            Operation::Publish {
                input: input("Other")
            }
        )
        .is_err()
    );
    assert_eq!(fs::read(f.path()).unwrap(), before);
    drop(lease);
    assert_eq!(
        fs::metadata(f.path()).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(f.0.join("app-notices"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let outside = f.0.join("outside");
    fs::write(&outside, b"unchanged external bytes").unwrap();
    fs::remove_file(f.path()).unwrap();
    symlink(&outside, f.path()).unwrap();
    assert!(list(&f.0).is_err());
    assert!(
        apply(
            &f.0,
            Operation::Publish {
                input: input("Other")
            }
        )
        .is_err()
    );
    assert_eq!(fs::read(&outside).unwrap(), b"unchanged external bytes");
    fs::remove_file(f.path()).unwrap();
    fs::remove_file(f.0.join("app-notices/history.lock")).unwrap();
    fs::remove_dir(f.0.join("app-notices")).unwrap();
    symlink(&outside, f.0.join("app-notices")).unwrap();
    assert!(list(&f.0).is_err());
    assert!(
        apply(
            &f.0,
            Operation::Publish {
                input: input("Other")
            }
        )
        .is_err()
    );
}
