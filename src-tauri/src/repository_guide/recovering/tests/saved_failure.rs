use super::*;

#[test]
#[ignore = "Explicit offline review of an owned terminal guide copy; no account connection, inference or continuation."]
fn saved_failed_guide_cleanup_probe() {
    use std::io::Write;
    let root = std::path::PathBuf::from(
        std::env::var_os("CODE_ATLAS_FAILED_GUIDE_FIXTURE")
            .expect("provide an isolated owned fixture copy"),
    );
    let marker: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("owned-fixture.json")).unwrap()).unwrap();
    assert_eq!(marker["kind"], "code-atlas-owned-failed-guide-fixture-v1");
    let storage = root.join("storage");
    let source = std::path::PathBuf::from(marker["originalStorage"].as_str().unwrap());
    assert_ne!(storage.canonicalize().unwrap(), source.canonicalize().unwrap());
    let id = marker["guideId"].as_str().unwrap();
    let guide_path = storage.join("repository-guides").join(format!("{id}.json"));
    let raw = std::fs::read(&guide_path).unwrap();
    assert_eq!(context::digest(&raw), marker["originalGuideSha256"]);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut report = options.open(root.join("native-review.json")).unwrap();
    let _lease = store::lease(&storage, id).unwrap().unwrap();
    let original = store::read(&storage, id).unwrap();
    assert_eq!(original.status, Status::Building);
    assert!(original.units.iter().flat_map(|u| &u.attempts).any(|a| {
        a.summary.as_ref().is_some_and(|s| s.outcome == Outcome::Failed)
    }));
    let before = summary(&storage, &original).unwrap();
    let manager = codex_agent::CodexAgentManager::default();
    let settled = settle_failed_build(&storage, id, &manager).unwrap();
    assert_eq!(settled.status, Status::Partial);
    assert!(settled.active_run_id.is_none());
    assert_eq!(settled.revision, original.revision + 1);
    let mut old = serde_json::to_value(&original).unwrap();
    let mut new = serde_json::to_value(&settled).unwrap();
    for field in ["revision", "status", "activeRunId"] {
        old.as_object_mut().unwrap().remove(field);
        new.as_object_mut().unwrap().remove(field);
    }
    assert_eq!(old, new, "saved intent, outline, all attempts and receipts must survive");
    let after = summary(&storage, &settled).unwrap();
    assert_eq!(serde_json::to_value(&before.answers).unwrap(), serde_json::to_value(&after.answers).unwrap());
    assert_eq!(before.continuation_operations, after.continuation_operations);
    let bytes = std::fs::read(&guide_path).unwrap();
    assert_eq!(settle_failed_build(&storage, id, &manager).unwrap().revision, settled.revision);
    assert_eq!(reconcile(&storage, &settled, &manager).unwrap().revision, settled.revision);
    assert_eq!(std::fs::read(&guide_path).unwrap(), bytes);
    let result = serde_json::json!({
        "guideId":id, "originalGuideSha256":context::digest(&raw),
        "settledGuideSha256":context::digest(&bytes),
        "originalRevision":original.revision, "settledRevision":settled.revision,
        "originalStatus":"building", "settledStatus":"partial",
        "questionsOutlineAttemptsReceiptsUnchanged":true,
        "priorPublications":before.answers.iter().map(|a| a.sections.len()).sum::<usize>(),
        "remainingExplicitOperations":after.continuation_operations,
        "reopeningIsBytePreserving":true,
        "accountConnected":false, "inferenceDispatched":false, "continuationDispatched":false,
        "accepted":false, "semanticAcceptance":false, "visualAcceptance":false, "voiceAcceptance":false
    });
    report.write_all(&serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    report.sync_all().unwrap();
}
