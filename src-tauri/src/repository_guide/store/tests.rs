use super::*;
use crate::{
    chapter_generation::tests::Fixture,
    repository_guide::tests::{attempt, record},
};
#[test]
fn private_records_reopen_and_conflicting_creates_preserve_original_bytes() {
    let f = Fixture::new();
    let record = record(&f);
    save(&f.storage, &record, true).unwrap();
    let before = std::fs::read(path(&f.storage, &record.id).unwrap()).unwrap();
    assert!(save(&f.storage, &record, true).is_err());
    assert_eq!(
        std::fs::read(path(&f.storage, &record.id).unwrap()).unwrap(),
        before
    );
    assert_eq!(read(&f.storage, &record.id).unwrap().id, record.id);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(path(&f.storage, &record.id).unwrap())
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
#[test]
fn worker_lease_excludes_a_second_worker_until_the_original_is_joined() {
    let f = Fixture::new();
    let id = Uuid::new_v4().to_string();
    let worker = lease(&f.storage, &id).unwrap().unwrap();
    assert!(lease(&f.storage, &id).unwrap().is_none());
    drop(worker);
    assert!(lease(&f.storage, &id).unwrap().is_some());
}
#[test]
fn damaged_contract_duplicate_runs_and_excess_attempts_fail_without_rewriting() {
    let f = Fixture::new();
    let mut r = record(&f);
    let next = attempt(&r, Outcome::Complete);
    r.units[0].attempts.push(next);
    assert!(validate(&r).is_ok());
    let a = r.units[0].attempts[0].clone();
    r.units[1].attempts.push(a);
    assert!(validate(&r).is_err());
    r.units[1].attempts.clear();
    r.contract = "unknown".into();
    assert!(validate(&r).is_err());
    r.contract = CONTRACT.into();
    for _ in 0..2 {
        let next = attempt(&r, Outcome::Partial);
        r.units[0].attempts.push(next);
    }
    assert!(validate(&r).is_err());
    assert!(read(&f.storage, "../../outside").is_err());
}
#[test]
fn nonregular_records_and_oversized_data_are_preserved() {
    let f = Fixture::new();
    let r = record(&f);
    save(&f.storage, &r, true).unwrap();
    let location = path(&f.storage, &r.id).unwrap();
    std::fs::write(&location, vec![b'x'; MAX_RECORD + 1]).unwrap();
    assert!(read(&f.storage, &r.id).is_err());
    #[cfg(unix)]
    {
        std::fs::remove_file(&location).unwrap();
        std::os::unix::fs::symlink(f.storage.join("missing"), &location).unwrap();
        assert!(read(&f.storage, &r.id).is_err());
        assert!(list(&f.storage, &r.project_id, &r.snapshot_id).is_err());
    }
}
#[test]
fn transactions_exclude_competing_writes_and_revisions_cannot_go_backwards() {
    let f = Fixture::new();
    let r = record(&f);
    create(&f.storage, &r).unwrap();
    let transaction = lock_named(&f.storage, ".transactions.lock")
        .unwrap()
        .unwrap();
    assert!(
        update(&f.storage, &r.id, |r| {
            r.status = Status::Stopped;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(read(&f.storage, &r.id).unwrap().revision, 0);
    drop(transaction);
    let first = update(&f.storage, &r.id, |r| {
        r.status = Status::Stopped;
        Ok(())
    })
    .unwrap();
    assert_eq!(first.revision, 1);
    let second = update(&f.storage, &r.id, |r| {
        r.revision = 0;
        Ok(())
    })
    .unwrap();
    assert_eq!(second.revision, 2);
}
#[test]
fn complete_without_sections_and_foreign_cached_answer_lineage_are_rejected() {
    let f = Fixture::new();
    let mut r = record(&f);
    let mut a = attempt(&r, Outcome::Complete);
    a.summary.as_mut().unwrap().sections.clear();
    r.units[0].attempts.push(a);
    assert!(validate(&r).is_err());
    r.units[0].attempts.clear();
    let mut a = attempt(&r, Outcome::Partial);
    a.summary.as_mut().unwrap().project_id = "another repository".into();
    r.units[0].attempts.push(a);
    assert!(validate(&r).is_err());
}
