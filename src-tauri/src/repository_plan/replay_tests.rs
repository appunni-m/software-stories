//! Frozen, previously saved account-generated synthetic route. No account or
//! runtime is available to these tests: reopening is strictly local.

use super::*;
use crate::{
    chapter_store, chapters, planned_chapter, repository_plan_history, repository_plan_store,
};
use std::{collections::BTreeMap, fs, path::PathBuf};

use super::contracts as historical_contracts;

const PROJECT: &str = "/private/tmp/code-atlas-codex-synthetic";
const SNAPSHOT: &str = "snap-1792133675714600000-43528-0";
const KEY: &str = "7fb1e48e15c0bbeb341ef7176851a9f21c3e92ff374c80a93432f0a33baa496b";
const DRAFT: &str = "plan:b5742581-61d9-4eb2-bc6c-a89a75ef8431";
const REVISION: &str = "plan-revision:3be55a8a-1d3e-446f-958d-19766a2784f6";

struct SavedRoute {
    storage: PathBuf,
    record: repository_plan_store::SavedPlanRecord,
}

impl SavedRoute {
    fn new() -> Self {
        let storage =
            std::env::temp_dir().join(format!("code-atlas-route-replay-{}", Uuid::new_v4()));
        let inputs: Vec<(String, &[u8])> = vec![
            (
                format!("snapshots/{KEY}/catalog.json"),
                include_bytes!("../../../benchmarks/saved-route-replay/catalog.json"),
            ),
            (
                format!("snapshots/{KEY}/manifests/{SNAPSHOT}.json"),
                include_bytes!("../../../benchmarks/saved-route-replay/manifest.json"),
            ),
            (
                format!("indexes/{KEY}/atlas-symbol-index-v2/{SNAPSHOT}.json"),
                include_bytes!("../../../benchmarks/saved-route-replay/index.json"),
            ),
            (
                format!("repository-plans/{KEY}/b5742581-61d9-4eb2-bc6c-a89a75ef8431.json"),
                include_bytes!("../../../benchmarks/saved-route-replay/draft.json"),
            ),
            (
                format!(
                    "repository-plan-history/{KEY}/cffcfde1-c9fe-4883-aae5-a3ede8c1b823.0000000001.json"
                ),
                include_bytes!("../../../benchmarks/saved-route-replay/history.json"),
            ),
            (
                format!(
                    "chapters/{KEY}/targets/target:file:724c4c8cc0d65b446d1a137dc2b0aaf253b2fdf99860d3251c42d0b1e33dfaae/revision:145ce15d-5bac-4a10-a949-6238cac05025.json"
                ),
                include_bytes!("../../../benchmarks/saved-route-replay/chapter.json"),
            ),
            (
                "objects/sha256/bd/1859f143bc0f6eb5858ea58da490c7844b91602b5ca19e614e1d46c8f2680a"
                    .to_owned(),
                include_bytes!("../../../benchmarks/saved-route-replay/cart.ts"),
            ),
        ];
        for (relative, bytes) in inputs {
            let path = storage.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        let record = repository_plan_store::read_draft_record(&storage, PROJECT, DRAFT).unwrap();
        Self { storage, record }
    }

    fn input(&self) -> PrepareRepositoryPlanInput {
        PrepareRepositoryPlanInput {
            project_id: PROJECT.to_owned(),
            snapshot_id: SNAPSHOT.to_owned(),
            question: self.record.scope.question.clone(),
            selected_paths: self.record.selected_paths.clone(),
        }
    }

    fn assert_reopens(&self) {
        let before = files(&self.storage);
        repository_plan_store::load(&self.storage, PROJECT, DRAFT).unwrap();
        repository_plan_history::load(&self.storage, PROJECT, REVISION).unwrap();
        let digest = repository_plan_store::record_digest(&self.record).unwrap();
        for step in &self.record.preview.plan.steps {
            let target = planned_chapter::PlannedChapterTarget {
                revision_id: REVISION.to_owned(),
                record_sha256: digest.clone(),
                step_id: step.step_id.clone(),
            };
            let scope = planned_chapter::prepare(&self.storage, PROJECT, &target).unwrap();
            assert_eq!(scope.target, target);
            assert_eq!(scope.question, step.question);
            assert_eq!(scope.plan_context, self.record.scope);
        }
        let saved = chapter_store::list_for_snapshot(&self.storage, PROJECT, SNAPSHOT).unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(
            saved[0].revision_id,
            "revision:145ce15d-5bac-4a10-a949-6238cac05025"
        );
        let origin = saved[0].generation_origin.as_ref().unwrap();
        let planned = origin.planned_chapter.as_ref().unwrap();
        assert_eq!(planned.target.revision_id, REVISION);
        assert_eq!(planned.target.record_sha256, digest);
        assert_eq!(planned.plan_context, self.record.scope);
        let manifest = snapshot::load_snapshot_manifest(&self.storage, PROJECT, SNAPSHOT).unwrap();
        let index = serde_json::from_slice(include_bytes!(
            "../../../benchmarks/saved-route-replay/index.json"
        ))
        .unwrap();
        let report =
            chapters::validate_chapter_revision(&self.storage, &manifest, Some(&index), &saved[0]);
        assert!(report.issues.is_empty(), "{:?}", report.issues);
        assert_eq!(
            files(&self.storage),
            before,
            "Reopening must not rewrite history, grants or source"
        );
    }

    // A synthetic older-contract variant, not another historical account run.
    // Rebuild consistent record/accepted-step links in isolated storage only.
    fn simulate_historical_contract(&mut self, contract: RequestContract) {
        let current = prepare(&self.storage, self.input()).unwrap();
        let data: Value = serde_json::from_str(
            current
                .user_prompt
                .rsplit_once("\n\nSOURCE AND INDEX DATA:\n")
                .unwrap()
                .1,
        )
        .unwrap();
        let request = json!({"systemInstruction": contract.system_instruction(),
            "userPrompt": contract.user_prompt(&current.context_sha256, &data),
            "outputSchema": contract.output_schema()});
        self.record.scope.planning_contract_sha256 = digest_json(&request).unwrap();
        assert_ne!(
            self.record.scope.planning_contract_sha256,
            current.request_body_sha256
        );
        fs::write(
            self.storage.join(format!(
                "repository-plans/{KEY}/b5742581-61d9-4eb2-bc6c-a89a75ef8431.json"
            )),
            serde_json::to_vec(&self.record).unwrap(),
        )
        .unwrap();
        let digest = repository_plan_store::record_digest(&self.record).unwrap();
        let history_path = self.storage.join(format!(
            "repository-plan-history/{KEY}/cffcfde1-c9fe-4883-aae5-a3ede8c1b823.0000000001.json"
        ));
        let mut history: Value = serde_json::from_slice(&fs::read(&history_path).unwrap()).unwrap();
        history["record"] = serde_json::to_value(&self.record).unwrap();
        history["revision"]["recordSha256"] = json!(digest);
        fs::write(history_path, serde_json::to_vec(&history).unwrap()).unwrap();
        let chapter_path = self.storage.join(format!("chapters/{KEY}/targets/target:file:724c4c8cc0d65b446d1a137dc2b0aaf253b2fdf99860d3251c42d0b1e33dfaae/revision:145ce15d-5bac-4a10-a949-6238cac05025.json"));
        let mut chapter: Value = serde_json::from_slice(&fs::read(&chapter_path).unwrap()).unwrap();
        chapter["generationOrigin"]["plannedChapter"]["target"]["recordSha256"] = json!(digest);
        chapter["generationOrigin"]["plannedChapter"]["planContext"] =
            serde_json::to_value(&self.record.scope).unwrap();
        fs::write(chapter_path, serde_json::to_vec(&chapter).unwrap()).unwrap();
    }
}

fn files(root: &Path) -> BTreeMap<PathBuf, String> {
    fn visit(root: &Path, path: &Path, result: &mut BTreeMap<PathBuf, String>) {
        for entry in fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    format!("{:x}", Sha256::digest(fs::read(path).unwrap())),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

impl Drop for SavedRoute {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.storage);
    }
}

#[test]
fn reopens_actual_saved_route_and_its_independent_chapter() {
    let fixture = SavedRoute::new();
    let current = prepare(&fixture.storage, fixture.input()).unwrap();
    // This real saved record was created using today's explicit evidence rules.
    assert_eq!(current.context_sha256, fixture.record.scope.context_sha256);
    assert_eq!(
        current.request_body_sha256,
        fixture.record.scope.planning_contract_sha256
    );
    fixture.assert_reopens();
}

#[test]
fn historical_record_digest_survives_typed_round_trip() {
    let fixture = SavedRoute::new();
    let encoded = serde_json::to_vec(&fixture.record).unwrap();
    let original: Value = serde_json::from_slice(include_bytes!(
        "../../../benchmarks/saved-route-replay/draft.json"
    ))
    .unwrap();
    let roundtrip: Value = serde_json::from_slice(&encoded).unwrap();
    fn changes(before: &Value, after: &Value, path: &str, found: &mut Vec<String>) {
        match (before, after) {
            (Value::Object(a), Value::Object(b)) => {
                for key in a.keys().chain(b.keys()).collect::<BTreeSet<_>>() {
                    changes(
                        &a.get(key).unwrap_or(&Value::Null),
                        &b.get(key).unwrap_or(&Value::Null),
                        &format!("{path}/{key}"),
                        found,
                    );
                }
            }
            (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
                for (index, (a, b)) in a.iter().zip(b).enumerate() {
                    changes(a, b, &format!("{path}/{index}"), found);
                }
            }
            _ if before != after => found.push(path.to_owned()),
            _ => {}
        }
    }
    let mut differences = Vec::new();
    changes(&original, &roundtrip, "", &mut differences);
    assert!(differences.is_empty(), "changed fields: {differences:?}");
    assert_eq!(
        repository_plan_store::record_digest(&fixture.record).unwrap(),
        "665b5886ff8e246abd6a499735bdd18559b70c2e5aa7f2b398a7d968973ae11a"
    );
}

#[test]
fn saved_route_replays_known_historical_contract_after_prompt_upgrade() {
    for contract in historical_contracts::RequestContract::HISTORICAL {
        let mut fixture = SavedRoute::new();
        fixture.simulate_historical_contract(contract);
        fixture.assert_reopens();
    }
}

#[test]
fn historical_contract_cannot_create_a_new_live_plan() {
    let fixture = SavedRoute::new();
    for contract in RequestContract::HISTORICAL {
        let context = prepare_with_contract(&fixture.storage, fixture.input(), contract).unwrap();
        assert!(
            canonicalize(
                &fixture.storage,
                &context,
                fixture.record.response_text.as_bytes()
            )
            .unwrap_err()
            .contains("only reopen saved routes")
        );
    }
}

#[test]
fn request_contract_bytes_match_independent_git_baselines() {
    // A separate Rust probe extracted the original literal instructions,
    // prompt builder and schema from each named Git commit and compared exact
    // JSON bytes. These pins catch edits/order/bound changes to frozen policies.
    let context = "0".repeat(64);
    let data = json!({});
    for (contract, expected) in [
        (
            RequestContract::Initial,
            "d567453fef95580b4f272508b3fe294f520785831ba79172b56298b017d92756",
        ),
        (
            RequestContract::CodeFirst,
            "61bd7a116e25b2eb2004222cf4f1ea1900a51ab5d02fbfb4c9c15268b26b42cd",
        ),
        (
            RequestContract::ExplicitEvidence,
            "eac43d71fc6576a7ebd096dbd9aebeb277bbe939a07c653694d52fca5e2391d6",
        ),
    ] {
        let request = json!({"systemInstruction": contract.system_instruction(),
            "userPrompt": contract.user_prompt(&context, &data), "outputSchema": contract.output_schema()});
        assert_eq!(digest_json(&request).unwrap(), expected, "{contract:?}");
    }
}

#[test]
fn unknown_contract_is_distinguished_from_changed_source_and_preserved() {
    let fixture = SavedRoute::new();
    let before = files(&fixture.storage);
    let mut scope = fixture.record.scope.clone();
    scope.planning_contract_sha256 = "f".repeat(64);
    let error = revalidate_saved(
        &fixture.storage,
        fixture.input(),
        &scope,
        fixture.record.response_text.as_bytes(),
        &fixture.record.preview.plan,
    )
    .unwrap_err();
    assert!(error.contains("unsupported planning contract"), "{error}");
    assert!(!error.contains("source or index changed"));
    assert_eq!(files(&fixture.storage), before);
}

#[test]
fn historical_contract_never_waives_source_context_or_proposal_checks() {
    let mut fixture = SavedRoute::new();
    fixture.simulate_historical_contract(RequestContract::Initial);
    let original = fixture.record.scope.clone();
    let before = files(&fixture.storage);
    for field in ["context", "inventory", "question", "coverage", "index"] {
        let mut scope = original.clone();
        match field {
            "context" => scope.context_sha256 = "f".repeat(64),
            "inventory" => scope.inventory_sha256 = "f".repeat(64),
            "question" => scope.question = "What else does the cart calculate?".into(),
            "coverage" => scope.coverage.skipped_path_count += 1,
            _ => scope.index_status = "missing".into(),
        }
        let error = revalidate_saved(
            &fixture.storage,
            fixture.input(),
            &scope,
            fixture.record.response_text.as_bytes(),
            &fixture.record.preview.plan,
        )
        .unwrap_err();
        assert!(
            error.contains("source or index changed"),
            "{field}: {error}"
        );
    }
    for field in ["quote", "prerequisite", "identifier", "destination"] {
        let mut response: Value = serde_json::from_str(&fixture.record.response_text).unwrap();
        match field {
            "quote" => {
                response["steps"][0]["claim"]["evidence"][0]["quote"] =
                    json!("inventedCodeDoesNotExist")
            }
            "prerequisite" => {
                response["steps"][0]["prerequisiteIds"] = json!([response["steps"][1]["localId"]])
            }
            "identifier" => {
                response["steps"][0]["claim"]["identifiers"] = json!(["inventedCodeDoesNotExist"])
            }
            _ => response["steps"][0]["primaryPath"] = json!("absent.ts"),
        }
        assert!(
            revalidate_saved(
                &fixture.storage,
                fixture.input(),
                &original,
                response.to_string().as_bytes(),
                &fixture.record.preview.plan
            )
            .is_err(),
            "{field}"
        );
    }
    let mut changed = fixture.record.preview.plan.clone();
    changed.steps[0]
        .claim
        .prose
        .push_str(" Invented extra behavior.");
    assert!(
        revalidate_saved(
            &fixture.storage,
            fixture.input(),
            &original,
            fixture.record.response_text.as_bytes(),
            &changed
        )
        .is_err()
    );
    assert_eq!(files(&fixture.storage), before);
}

#[test]
fn changed_source_or_index_cannot_reopen_a_historical_route() {
    for change in ["source", "index"] {
        let mut fixture = SavedRoute::new();
        fixture.simulate_historical_contract(RequestContract::CodeFirst);
        let path = if change == "source" {
            fixture.storage.join(
                "objects/sha256/bd/1859f143bc0f6eb5858ea58da490c7844b91602b5ca19e614e1d46c8f2680a",
            )
        } else {
            fixture.storage.join(format!(
                "indexes/{KEY}/atlas-symbol-index-v2/{SNAPSHOT}.json"
            ))
        };
        fs::write(path, b"corrupt immutable evidence").unwrap();
        let before = files(&fixture.storage);
        assert!(
            repository_plan_store::load(&fixture.storage, PROJECT, DRAFT).is_err(),
            "{change}"
        );
        assert!(
            repository_plan_history::load(&fixture.storage, PROJECT, REVISION).is_err(),
            "{change}"
        );
        assert_eq!(files(&fixture.storage), before);
    }
}

#[test]
fn both_v1_encodings_and_new_v2_records_keep_their_own_digest() {
    let fixture = SavedRoute::new();
    let old_digest = repository_plan_store::record_digest(&fixture.record).unwrap();
    for version in [1, 2] {
        let mut data = serde_json::to_value(&fixture.record).unwrap();
        data["schemaVersion"] = json!(version);
        data["preview"] = serde_json::to_value(&fixture.record.preview).unwrap();
        let bytes = serde_json::to_vec(&data).unwrap();
        let record: repository_plan_store::SavedPlanRecord =
            serde_json::from_slice(&bytes).unwrap();
        assert_eq!(serde_json::to_vec(&record).unwrap(), bytes);
        assert_ne!(
            repository_plan_store::record_digest(&record).unwrap(),
            old_digest
        );
        repository_plan_store::revalidate(&fixture.storage, &record).unwrap();
        let mut changed = record.clone();
        changed.preview.plan.title.push_str(" changed");
        assert_ne!(
            repository_plan_store::record_digest(&changed).unwrap(),
            repository_plan_store::record_digest(&record).unwrap()
        );
    }
}

#[test]
fn new_v2_record_can_be_explicitly_accepted_without_changing_older_history() {
    let fixture = SavedRoute::new();
    let before = files(&fixture.storage);
    let new_id = Uuid::new_v4().to_string();
    let draft_id = format!("plan:{new_id}");
    let mut data = serde_json::to_value(&fixture.record).unwrap();
    data["schemaVersion"] = json!(2);
    data["preview"] = serde_json::to_value(&fixture.record.preview).unwrap();
    data["preview"]["plan"]["draftId"] = json!(draft_id);
    for step in data["preview"]["plan"]["steps"].as_array_mut().unwrap() {
        let step_id = step["stepId"].as_str().unwrap().replace(DRAFT, &draft_id);
        step["stepId"] = json!(step_id);
        for prerequisite in step["prerequisiteStepIds"].as_array_mut().unwrap() {
            *prerequisite = json!(prerequisite.as_str().unwrap().replace(DRAFT, &draft_id));
        }
    }
    let bytes = serde_json::to_vec(&data).unwrap();
    let record: repository_plan_store::SavedPlanRecord = serde_json::from_slice(&bytes).unwrap();
    repository_plan_store::revalidate(&fixture.storage, &record).unwrap();
    fs::write(
        fixture
            .storage
            .join(format!("repository-plans/{KEY}/{new_id}.json")),
        &bytes,
    )
    .unwrap();
    let digest = repository_plan_store::record_digest(&record).unwrap();
    let accepted = repository_plan_history::accept(
        &fixture.storage,
        repository_plan_history::AcceptRepositoryPlanInput {
            project_id: PROJECT.into(),
            draft_id,
            expected_record_sha256: digest.clone(),
            based_on_revision_id: None,
        },
    )
    .unwrap();
    let accepted = serde_json::to_value(accepted).unwrap();
    let revision_id = accepted["revision"]["revisionId"].as_str().unwrap();
    assert_eq!(accepted["revision"]["recordSha256"], digest);
    repository_plan_history::load(&fixture.storage, PROJECT, revision_id).unwrap();
    for step in &record.preview.plan.steps {
        planned_chapter::prepare(
            &fixture.storage,
            PROJECT,
            &planned_chapter::PlannedChapterTarget {
                revision_id: revision_id.into(),
                record_sha256: digest.clone(),
                step_id: step.step_id.clone(),
            },
        )
        .unwrap();
    }
    let after = files(&fixture.storage);
    for (path, hash) in before {
        assert_eq!(after.get(&path), Some(&hash), "{}", path.display());
    }
    fixture.assert_reopens();
}

#[test]
fn future_mixed_unknown_or_duplicate_record_fields_are_rejected() {
    let fixture = SavedRoute::new();
    for case in [
        "future",
        "v2legacy",
        "mixed",
        "unknown",
        "duplicate",
        "aliasDuplicate",
    ] {
        let mut data = serde_json::to_value(&fixture.record).unwrap();
        match case {
            "future" => data["schemaVersion"] = json!(3),
            "v2legacy" => data["schemaVersion"] = json!(2),
            "mixed" => {
                let category = data["preview"]["plan"]["unknowns"][0]["category"]
                    .as_object_mut()
                    .unwrap();
                let label = category.remove("visible_label").unwrap();
                category.insert("visibleLabel".into(), label);
            }
            "unknown" => data["preview"]["plan"]["extra"] = json!(true),
            "aliasDuplicate" => {
                data["preview"]["plan"]["unknowns"][0]["category"]["visibleLabel"] =
                    json!("Inference")
            }
            _ => {}
        }
        let mut encoded = data.to_string();
        if case == "duplicate" {
            encoded = encoded.replacen(
                "\"reviewId\":",
                "\"reviewId\":\"duplicate\",\"reviewId\":",
                1,
            );
        }
        assert!(
            serde_json::from_str::<repository_plan_store::SavedPlanRecord>(&encoded).is_err(),
            "{case}"
        );
    }
}
