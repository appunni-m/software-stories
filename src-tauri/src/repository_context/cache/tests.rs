use super::*;
use crate::{chapter_generation::tests::Fixture, repository_context as context};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

fn open(storage: &Path, snapshot: &str, budget: &Budget<'_>, limits: Limits) -> Cache {
    Cache::with_limits(storage, "retention-project", snapshot, budget, limits).unwrap()
}

fn age(path: &Path, seconds: u64) {
    lock_file(path)
        .unwrap()
        .set_times(
            fs::FileTimes::new()
                .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)),
        )
        .unwrap();
}

#[test]
fn chunk_lru_enforces_both_bytes_and_entries_across_reopening() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let limits = Limits {
        chunk_bytes: 8,
        chunks: 2,
        ..Limits::default()
    };
    let mut cache = open(&fixture.storage, "chunks", &budget, limits);
    let a = context::digest(b"a");
    let b = context::digest(b"b");
    let c = context::digest(b"c");
    cache.save_chunk(&a, b"aaaa", &budget).unwrap();
    cache.save_chunk(&b, b"bbbb", &budget).unwrap();
    // Explicit times avoid relying on filesystem timestamp precision or sleeps.
    age(&cache.chunk_path(&a), 1);
    age(&cache.chunk_path(&b), 2);
    drop(cache);
    let mut cache = open(&fixture.storage, "chunks", &budget, limits);
    cache.chunk_used(&a);
    cache.save_chunk(&c, b"cccc", &budget).unwrap();
    assert!(cache.chunk_path(&a).exists());
    assert!(!cache.chunk_path(&b).exists());
    assert!(cache.chunk_path(&c).exists());
    assert_eq!(cache.chunks.values().map(|e| e.bytes).sum::<u64>(), 8);
    drop(cache);
    let cache = open(&fixture.storage, "chunks", &budget, limits);
    assert_eq!(cache.chunks.len(), 2);
    drop(cache);
    let cache = open(
        &fixture.storage,
        "chunks",
        &budget,
        Limits {
            chunk_bytes: 4,
            chunks: 1,
            ..limits
        },
    );
    assert_eq!(cache.chunks.len(), 1);
    assert_eq!(cache.chunks.values().map(|e| e.bytes).sum::<u64>(), 4);
}

#[test]
fn scope_lru_skips_live_readers_and_refuses_a_new_scope_when_all_are_pinned() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let limits = Limits {
        scopes: 2,
        ..Limits::default()
    };
    let first = open(&fixture.storage, "first", &budget, limits);
    let first_path = first.directory.clone();
    let first_generation = first_path.join(Uuid::new_v4().to_string());
    directory(&first_generation).unwrap();
    let first_lease = first.lease(&first_generation).unwrap();
    age(&first_path.join("last-used"), 1);
    drop(first);
    let second = open(&fixture.storage, "second", &budget, limits);
    let second_path = second.directory.clone();
    age(&second_path.join("last-used"), 2);
    drop(second);
    let third = open(&fixture.storage, "third", &budget, limits);
    let third_path = third.directory.clone();
    let third_generation = third_path.join(Uuid::new_v4().to_string());
    directory(&third_generation).unwrap();
    let third_lease = third.lease(&third_generation).unwrap();
    assert!(first_path.exists());
    assert!(!second_path.exists());
    assert_eq!(third.scopes(&budget).unwrap().len(), 2);
    drop(third);
    let error = Cache::with_limits(
        &fixture.storage,
        "retention-project",
        "fourth",
        &budget,
        limits,
    )
    .err()
    .unwrap();
    assert_eq!(error, FULL);
    assert!(first_path.exists());
    assert!(third_path.exists());
    drop(first_lease);
    let fourth = open(&fixture.storage, "fourth", &budget, limits);
    assert!(!first_path.exists());
    assert!(third_path.exists());
    assert_eq!(fourth.scopes(&budget).unwrap().len(), 2);
    drop(third_lease);
}

#[test]
fn global_byte_quota_evicts_old_scopes_and_counts_unknown_files_without_deleting_them() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let first = open(&fixture.storage, "old", &budget, Limits::default());
    let first_path = first.directory.clone();
    fs::write(first_path.join("derived-payload"), vec![0; 1024]).unwrap();
    let unrelated = first.root.join("unknown-kept");
    fs::write(&unrelated, b"keep this").unwrap();
    drop(first);
    let current = open(
        &fixture.storage,
        "new",
        &budget,
        Limits {
            bytes: 128,
            ..Limits::default()
        },
    );
    assert!(!first_path.exists());
    assert_eq!(fs::read(&unrelated).unwrap(), b"keep this");
    drop(current);
    fs::write(&unrelated, vec![0; 256]).unwrap();
    assert_eq!(
        Cache::with_limits(
            &fixture.storage,
            "retention-project",
            "new",
            &budget,
            Limits {
                bytes: 128,
                ..Limits::default()
            }
        )
        .err()
        .unwrap(),
        FULL
    );
    assert_eq!(fs::metadata(unrelated).unwrap().len(), 256);
}

#[test]
fn byte_pressure_preserves_pinned_scopes_then_recovers_after_reader_drop() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let cache = open(&fixture.storage, "pinned", &budget, Limits::default());
    let scope = cache.directory.clone();
    let generation = scope.join(Uuid::new_v4().to_string());
    directory(&generation).unwrap();
    fs::write(generation.join("payload"), vec![0; 1024]).unwrap();
    let lease = cache.lease(&generation).unwrap();
    drop(cache);
    let limits = Limits {
        bytes: 128,
        ..Limits::default()
    };
    assert_eq!(
        Cache::with_limits(
            &fixture.storage,
            "retention-project",
            "new",
            &budget,
            limits
        )
        .err()
        .unwrap(),
        FULL
    );
    assert!(generation.join("payload").exists());
    drop(lease);
    let new = open(&fixture.storage, "new", &budget, limits);
    assert!(!scope.exists());
    assert!(new.directory.exists());
}

#[test]
fn publication_reserves_pointer_growth_and_keeps_chunk_and_index_quotas_independent() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let mut cache = open(
        &fixture.storage,
        "pointer",
        &budget,
        Limits {
            bytes: 8,
            chunk_bytes: 16,
            ..Limits::default()
        },
    );
    cache
        .save_chunk(&context::digest(b"layout"), &[0; 16], &budget)
        .unwrap();
    let stage = cache.directory.join(format!("staging-{}", Uuid::new_v4()));
    directory(&stage).unwrap();
    fs::write(stage.join("payload"), [0; 4]).unwrap();
    cache.reserve_publication(&stage, 4, &budget).unwrap();
    assert_eq!(
        cache.reserve_publication(&stage, 5, &budget).err().unwrap(),
        FULL
    );
    assert!(!cache.directory.join("current.json").exists());
    assert_eq!(cache.chunk_bytes, 16);
}

#[test]
fn pressure_reclaims_released_generations_in_the_current_scope_without_replacing_its_pointer() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let cache = open(&fixture.storage, "current", &budget, Limits::default());
    let scope = cache.directory.clone();
    let old = scope.join(Uuid::new_v4().to_string());
    let current_id = Uuid::new_v4().to_string();
    let current = scope.join(&current_id);
    for generation in [&old, &current] {
        directory(generation).unwrap();
        fs::write(generation.join("payload"), vec![0; 1024]).unwrap();
    }
    let pointer = serde_json::to_vec(&context::Pointer {
        generation: current_id,
        fingerprint: "f".repeat(64),
        metadata_sha256: "a".repeat(64),
    })
    .unwrap();
    fs::write(scope.join("current.json"), &pointer).unwrap();
    let lease = cache.lease(&old).unwrap();
    drop(cache);
    let limits = Limits {
        bytes: 2048,
        ..Limits::default()
    };
    assert_eq!(
        Cache::with_limits(
            &fixture.storage,
            "retention-project",
            "current",
            &budget,
            limits
        )
        .err()
        .unwrap(),
        FULL
    );
    assert!(old.exists());
    drop(lease);
    let reopened = open(&fixture.storage, "current", &budget, limits);
    assert!(!old.exists());
    assert!(current.exists());
    assert_eq!(fs::read(scope.join("current.json")).unwrap(), pointer);
    assert!(reopened.scopes(&budget).unwrap().contains(&scope));
}

#[test]
fn maintenance_owner_drop_releases_the_lock_while_an_inherited_handle_remains() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let owner = open(&fixture.storage, "owner", &budget, Limits::default());
    let inherited = owner._maintenance.try_clone().unwrap();
    let lock_path = owner.root.join("maintenance.lock");
    let contender = lock_file(&lock_path).unwrap();
    assert!(matches!(
        contender.try_lock(),
        Err(TryLockError::WouldBlock)
    ));
    drop(owner);
    assert!(
        contender.try_lock().is_ok(),
        "an inherited handle must not extend maintenance ownership"
    );
    let observer = lock_file(&lock_path).unwrap();
    drop(inherited);
    assert!(matches!(observer.try_lock(), Err(TryLockError::WouldBlock)));
    contender.unlock().unwrap();
    assert!(observer.try_lock().is_ok());
}

#[test]
fn maintenance_wait_obeys_deadline_and_cancellation_without_removing_the_owner() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let owner = open(&fixture.storage, "owner", &budget, Limits::default());
    let contender = lock_file(&owner.root.join("maintenance.lock")).unwrap();
    assert!(matches!(
        contender.try_lock(),
        Err(TryLockError::WouldBlock)
    ));
    let timed = Budget {
        cancelled: &cancelled,
        deadline: std::time::Instant::now() + Duration::from_millis(30),
    };
    assert!(
        Cache::open(&fixture.storage, "other", "waiting", &timed)
            .err()
            .unwrap()
            .contains("time limit")
    );
    let stop = Arc::new(AtomicBool::new(false));
    let stop_thread = Arc::clone(&stop);
    let storage = fixture.storage.clone();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let handle = thread::spawn(move || {
        started_tx.send(()).unwrap();
        Cache::open(&storage, "other", "cancelled", &Budget::new(&stop_thread))
            .err()
            .unwrap()
    });
    started_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    stop.store(true, Ordering::Release);
    assert_eq!(handle.join().unwrap(), "Code search stopped.");
    assert!(owner.directory.exists());
    assert!(matches!(
        contender.try_lock(),
        Err(TryLockError::WouldBlock)
    ));
    drop(owner);
    assert!(contender.try_lock().is_ok());
}

#[test]
fn interrupted_temporary_files_are_removed_without_touching_complete_pointer_or_unknown_data() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let cache = open(&fixture.storage, "interrupted", &budget, Limits::default());
    let scope = cache.directory.clone();
    let stage = scope.join(format!("staging-{}", Uuid::new_v4()));
    directory(&stage).unwrap();
    fs::write(stage.join("partial"), b"partial").unwrap();
    let pointer_tmp = scope.join(format!("pointer-{}.tmp", Uuid::new_v4()));
    fs::write(&pointer_tmp, b"partial pointer").unwrap();
    let chunk_tmp =
        cache
            .root
            .join("chunk-cache")
            .join(format!("{}-{}.tmp", "a".repeat(64), Uuid::new_v4()));
    fs::write(&chunk_tmp, b"partial chunk").unwrap();
    fs::write(scope.join("current.json"), b"complete pointer").unwrap();
    fs::write(scope.join("staging-unknown"), b"keep").unwrap();
    drop(cache);
    let reopened = open(&fixture.storage, "interrupted", &budget, Limits::default());
    assert!(!stage.exists());
    assert!(!pointer_tmp.exists());
    assert!(!chunk_tmp.exists());
    assert_eq!(
        fs::read(scope.join("current.json")).unwrap(),
        b"complete pointer"
    );
    assert_eq!(fs::read(scope.join("staging-unknown")).unwrap(), b"keep");
    assert_eq!(reopened.directory, scope);
}

#[test]
fn refused_publication_preserves_the_previous_pointer_and_searchable_reader() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let first = context::prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    let cache = Cache::with_limits(
        &fixture.storage,
        &fixture.manifest.summary.project_id,
        &fixture.manifest.summary.snapshot_id,
        &budget,
        Limits {
            generation_bytes: 1,
            ..Limits::default()
        },
    )
    .unwrap();
    let scope = cache.directory.clone();
    let pointer = fs::read(scope.join("current.json")).unwrap();
    // A different syntax fingerprint forces a new build of the same immutable source.
    let error = context::prepare_cached(&fixture.storage, &fixture.manifest, None, &budget, cache)
        .err()
        .unwrap();
    assert_eq!(error, FULL);
    assert_eq!(fs::read(scope.join("current.json")).unwrap(), pointer);
    assert!(!fs::read_dir(&scope).unwrap().any(|e| {
        e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("staging-")
    }));
    let result = context::search(
        &fixture.storage,
        &fixture.manifest,
        &first,
        "greeting",
        None,
        8,
        &budget,
    )
    .unwrap();
    assert!(!result.matches.is_empty());
}

#[test]
fn old_generation_lease_survives_multiple_rebuilds_until_reader_drop() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let first = context::prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    let scope = context::root(
        &fixture.storage,
        &fixture.manifest.summary.project_id,
        &fixture.manifest.summary.snapshot_id,
    );
    let first_pointer: context::Pointer =
        serde_json::from_slice(&fs::read(scope.join("current.json")).unwrap()).unwrap();
    let first_path = scope.join(&first_pointer.generation);
    fs::write(first_path.join("ranges.json"), b"bad metadata").unwrap();
    let second = context::prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    let third = context::prepare(&fixture.storage, &fixture.manifest, None, &budget).unwrap();
    assert!(first_path.exists());
    assert!(
        !context::search(
            &fixture.storage,
            &fixture.manifest,
            &first,
            "greeting",
            None,
            8,
            &budget
        )
        .unwrap()
        .matches
        .is_empty()
    );
    drop(first);
    let fourth = context::prepare(
        &fixture.storage,
        &fixture.manifest,
        Some(&fixture.index),
        &budget,
    )
    .unwrap();
    assert!(!first_path.exists());
    assert!(!fourth.reused_generation);
    drop((second, third));
}

#[test]
fn bounded_scan_fails_instead_of_undercounting_and_cancelled_scan_keeps_data() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let cache = open(&fixture.storage, "bounded", &budget, Limits::default());
    let keep = cache.directory.join("keep");
    fs::write(&keep, b"kept").unwrap();
    assert!(size(&cache.directory, 0, &mut 1, &budget).is_err());
    cancelled.store(true, Ordering::Release);
    let mut remaining = SCAN_ENTRIES;
    assert_eq!(
        size(&cache.directory, 0, &mut remaining, &budget)
            .err()
            .unwrap(),
        "Code search stopped."
    );
    assert_eq!(fs::read(keep).unwrap(), b"kept");
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_redirect_cache_reads_locks_or_eviction_outside_derived_storage() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let outside = fixture.storage.join("outside-kept");
    directory(&outside).unwrap();
    fs::write(outside.join("keep"), b"keep").unwrap();
    let root = fixture.storage.join("repository-context");
    symlink(&outside, &root).unwrap();
    assert!(Cache::open(&fixture.storage, "project", "snapshot", &budget).is_err());
    fs::remove_file(&root).unwrap();
    let cache = open(&fixture.storage, "links", &budget, Limits::default());
    let chunk = cache.chunk_path(&"a".repeat(64));
    let scope = cache.directory.clone();
    drop(cache);
    symlink(outside.join("keep"), &chunk).unwrap();
    assert!(Cache::open(&fixture.storage, "project", "snapshot", &budget).is_err());
    fs::remove_file(chunk).unwrap();
    let lock_path = root.join("maintenance.lock");
    fs::remove_file(&lock_path).unwrap();
    symlink(outside.join("keep"), &lock_path).unwrap();
    assert!(Cache::open(&fixture.storage, "project", "snapshot", &budget).is_err());
    fs::remove_file(lock_path).unwrap();
    let generation_link = scope.join(Uuid::new_v4().to_string());
    symlink(&outside, generation_link).unwrap();
    assert!(Cache::open(&fixture.storage, "project", "snapshot", &budget).is_err());
    assert_eq!(fs::read(outside.join("keep")).unwrap(), b"keep");
}

#[test]
fn reader_leases_are_enforced_by_an_independent_process() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let cache = open(&fixture.storage, "process", &budget, Limits::default());
    let generation = cache.directory.join(Uuid::new_v4().to_string());
    directory(&generation).unwrap();
    let lease = cache.lease(&generation).unwrap();
    let locks = [
        generation.join("readers.lock"),
        cache.directory.join("readers.lock"),
        cache.root.join("maintenance.lock"),
    ];
    let probe = |locked: bool| {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "repository_context::cache::tests::cross_process_lease_probe",
                "--nocapture",
            ])
            .env(
                "CODE_ATLAS_CACHE_TEST_LOCKS",
                serde_json::to_string(&locks).unwrap(),
            )
            .env("CODE_ATLAS_CACHE_TEST_LOCKED", locked.to_string())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    };
    probe(true);
    drop(lease);
    drop(cache);
    probe(false);
}

#[test]
#[ignore = "subprocess helper; invoked only by the disposable reader lease test"]
fn cross_process_lease_probe() {
    let paths: Vec<PathBuf> =
        serde_json::from_str(&std::env::var("CODE_ATLAS_CACHE_TEST_LOCKS").unwrap()).unwrap();
    let locked = std::env::var("CODE_ATLAS_CACHE_TEST_LOCKED").unwrap() == "true";
    for path in paths {
        let file = lock_file(&path).unwrap();
        if locked {
            assert!(matches!(file.try_lock(), Err(TryLockError::WouldBlock)));
        } else {
            file.try_lock().unwrap();
        }
    }
}

#[test]
fn a_base_lease_pins_its_original_scope_instead_of_the_new_snapshot_scope() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let original = open(&fixture.storage, "original", &budget, Limits::default());
    let original_scope = original.directory.clone();
    let generation = original_scope.join(Uuid::new_v4().to_string());
    directory(&generation).unwrap();
    drop(original);
    let next = open(&fixture.storage, "next", &budget, Limits::default());
    let lease = next.lease(&generation).unwrap();
    assert!(
        exclusive(&original_scope.join("readers.lock"))
            .unwrap()
            .is_none()
    );
    assert!(
        exclusive(&next.directory.join("readers.lock"))
            .unwrap()
            .is_some()
    );
    drop(lease);
    assert!(
        exclusive(&original_scope.join("readers.lock"))
            .unwrap()
            .is_some()
    );
    let other_project = next
        .root
        .join("b".repeat(64))
        .join("c".repeat(64))
        .join(Uuid::new_v4().to_string());
    assert!(next.lease(&other_project).is_err());
}

#[test]
fn reader_owner_drop_unlocks_both_leases_even_with_inherited_file_descriptions() {
    let fixture = Fixture::new();
    let cancelled = AtomicBool::new(false);
    let budget = Budget::new(&cancelled);
    let cache = open(
        &fixture.storage,
        "inherited-reader",
        &budget,
        Limits::default(),
    );
    let generation = cache.directory.join(Uuid::new_v4().to_string());
    directory(&generation).unwrap();
    let reader = cache.lease(&generation).unwrap();
    let inherited_generation = reader._generation.try_clone().unwrap();
    let inherited_scope = reader._scope.try_clone().unwrap();
    assert!(
        exclusive(&generation.join("readers.lock"))
            .unwrap()
            .is_none()
    );
    assert!(
        exclusive(&cache.directory.join("readers.lock"))
            .unwrap()
            .is_none()
    );
    drop(reader);
    assert!(
        exclusive(&generation.join("readers.lock"))
            .unwrap()
            .is_some()
    );
    assert!(
        exclusive(&cache.directory.join("readers.lock"))
            .unwrap()
            .is_some()
    );
    drop((inherited_generation, inherited_scope));
}
