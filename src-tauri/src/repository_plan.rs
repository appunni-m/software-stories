//! Transport-independent preparation and validation for an AI repository plan.
//!
//! This is a bounded planning unit, not an overview chapter or a complete guide.
//! Preparation is local. It does not connect an agent, authorize a request,
//! persist a plan, generate chapters or mark any file explained. Automatic
//! Codex/Claude dispatch lives in the existing native operation modules. The
//! app's planning workflow and durable plan store live separately.

use crate::{
    chapter_generation,
    chapters::{ChapterClaim, GeneratedChapterClaim},
    provider::{self, ProviderRelationshipEvidence, ProviderRequestFile, ProviderSymbolEvidence},
    snapshot::{
        self, CaptureStatus, CapturedContentKind, InventoryPathKind, SnapshotManifest,
        SnapshotSource,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

mod contracts;
use contracts::RequestContract;

// Immutable evidence/context serialization for the current planning request.
pub(crate) const PROMPT_VERSION: &str = RequestContract::CURRENT.prompt_version();
const MAX_STEPS: usize = 24;
const MAX_RESPONSE_BYTES: usize = 1_000_000;
const INVALID: &str = "The repository plan does not match the reviewed source, evidence or planning contract. No plan or chapter was saved.";

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PrepareRepositoryPlanInput {
    pub project_id: String,
    pub snapshot_id: String,
    pub question: String,
    pub selected_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PlanSourceCoverage {
    selected_file_count: usize,
    eligible_captured_file_count: usize,
    unselected_eligible_file_count: usize,
    excluded_captured_path_count: usize,
    skipped_path_count: usize,
}

/// Native preparation identity included in an agent's stored review. It is
/// descriptive context, never account/runtime qualification or source consent.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepositoryPlanReviewScope {
    pub schema_version: u32,
    pub question: String,
    pub inventory_sha256: String,
    pub context_sha256: String,
    pub planning_contract_sha256: String,
    pub capture_status: CaptureStatus,
    pub coverage: PlanSourceCoverage,
    pub index_status: String,
}

impl RepositoryPlanReviewScope {
    pub(crate) fn valid_recorded_shape(&self) -> bool {
        self.valid_recovery_shape(self.coverage.selected_file_count)
    }

    /// A journal retains descriptive planning context only. Reopening it must
    /// still reproduce this entire scope from native immutable evidence.
    pub(crate) fn valid_recovery_shape(&self, selected_count: usize) -> bool {
        let valid_digest = |value: &str| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        };
        self.schema_version == 1
            && valid_learning_question(&self.question)
            && valid_digest(&self.inventory_sha256)
            && valid_digest(&self.context_sha256)
            && valid_digest(&self.planning_contract_sha256)
            && (1..=12).contains(&selected_count)
            && self.coverage.selected_file_count == selected_count
            && self.coverage.eligible_captured_file_count >= selected_count
            && self.coverage.unselected_eligible_file_count
                == self.coverage.eligible_captured_file_count - selected_count
            && matches!(
                self.index_status.as_str(),
                "complete" | "partial" | "cancelled" | "missing" | "unreadable"
            )
    }

    /// A coarse filter only; exact evidence and request digests are compared
    /// after rebuilding each supported historical contract.
    fn same_replay_envelope(&self, other: &Self) -> bool {
        self.schema_version == other.schema_version
            && self.question == other.question
            && self.inventory_sha256 == other.inventory_sha256
            && self.capture_status == other.capture_status
            && self.coverage == other.coverage
            && self.index_status == other.index_status
    }
}

/// Owned native inputs for the existing transport-independent agent review
/// builder. No renderer deserializer can supply prompt/schema/source parts.
pub(crate) struct RepositoryPlanRequestParts {
    pub project_id: String,
    pub snapshot_id: String,
    pub scope: RepositoryPlanReviewScope,
    pub selected_files: Vec<ProviderRequestFile>,
    pub selected_symbols: Vec<ProviderSymbolEvidence>,
    pub selected_relationships: Vec<ProviderRelationshipEvidence>,
    pub index_evidence_truncated: bool,
    pub source_bytes: usize,
    pub system_instruction: String,
    pub user_prompt: String,
    pub output_schema: Value,
}

/// Exact local context for a native agent-specific request review. Neither
/// digest is consent, a connection ID, or proof of whole-repository coverage.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparedRepositoryPlanContext {
    project_id: String,
    snapshot_id: String,
    question: String,
    prompt_version: &'static str,
    capture_status: CaptureStatus,
    inventory_sha256: String,
    context_sha256: String,
    coverage: PlanSourceCoverage,
    selected_files: Vec<ProviderRequestFile>,
    selected_symbols: Vec<ProviderSymbolEvidence>,
    selected_relationships: Vec<ProviderRelationshipEvidence>,
    index_status: &'static str,
    index_evidence_truncated: bool,
    source_bytes: usize,
    system_instruction: String,
    user_prompt: String,
    output_schema: Value,
    request_body_bytes: usize,
    request_body_sha256: String,
    estimated_input_tokens: usize,
    #[serde(skip)]
    request_contract: RequestContract,
    #[serde(skip)]
    input: PrepareRepositoryPlanInput,
}

impl PreparedRepositoryPlanContext {
    pub(crate) fn review_scope(&self) -> RepositoryPlanReviewScope {
        RepositoryPlanReviewScope {
            schema_version: 1,
            question: self.question.clone(),
            inventory_sha256: self.inventory_sha256.clone(),
            context_sha256: self.context_sha256.clone(),
            planning_contract_sha256: self.request_body_sha256.clone(),
            capture_status: self.capture_status.clone(),
            coverage: self.coverage.clone(),
            index_status: self.index_status.to_owned(),
        }
    }

    fn with_contract(&self, contract: RequestContract) -> Result<Self, String> {
        let mut prepared = self.clone();
        prepared.apply_contract(contract)?;
        Ok(prepared)
    }

    fn apply_contract(&mut self, contract: RequestContract) -> Result<(), String> {
        let data = json!({
            "promptVersion": contract.prompt_version(),
            "projectId": self.project_id,
            "snapshotId": self.snapshot_id,
            "captureStatus": self.capture_status,
            "question": self.question,
            "inventorySha256": self.inventory_sha256,
            "coverage": self.coverage,
            "selectedFiles": self.selected_files,
            "index": {
                "status": self.index_status,
                "truncated": self.index_evidence_truncated,
                "symbols": self.selected_symbols,
                "relationships": self.selected_relationships,
            },
        });
        let context_sha256 = digest_json(&data)?;
        let prompt_data = json!({
            "promptVersion": contract.prompt_version(),
            "projectId": self.project_id,
            "snapshotId": self.snapshot_id,
            "captureStatus": self.capture_status,
            "question": self.question,
            "inventorySha256": self.inventory_sha256,
            "coverage": self.coverage,
            "selectedFiles": self.selected_files.iter().map(|file| &file.path).collect::<Vec<_>>(),
            "index": {
                "status": self.index_status,
                "truncated": self.index_evidence_truncated,
                "symbols": self.selected_symbols,
                "relationships": self.selected_relationships,
            },
        });
        let source_blocks = provider::format_selected_source_blocks(&self.selected_files)
            .map_err(|_| INVALID.to_owned())?;
        let system_instruction = contract.system_instruction();
        let user_prompt = contract.user_prompt(&context_sha256, &prompt_data, &source_blocks);
        let output_schema = contract.output_schema();
        let encoded = serde_json::to_vec(&json!({
            "systemInstruction": system_instruction,
            "userPrompt": user_prompt,
            "outputSchema": output_schema,
        }))
        .map_err(|_| INVALID.to_owned())?;

        self.prompt_version = contract.prompt_version();
        self.context_sha256 = context_sha256;
        self.system_instruction = system_instruction;
        self.user_prompt = user_prompt;
        self.output_schema = output_schema;
        self.request_body_bytes = encoded.len();
        self.request_body_sha256 = format!("{:x}", Sha256::digest(&encoded));
        self.estimated_input_tokens = encoded.len().div_ceil(4);
        self.request_contract = contract;
        Ok(())
    }

    pub(crate) fn into_request_parts(self) -> RepositoryPlanRequestParts {
        let scope = self.review_scope();
        RepositoryPlanRequestParts {
            project_id: self.project_id,
            snapshot_id: self.snapshot_id,
            scope,
            selected_files: self.selected_files,
            selected_symbols: self.selected_symbols,
            selected_relationships: self.selected_relationships,
            index_evidence_truncated: self.index_evidence_truncated,
            source_bytes: self.source_bytes,
            system_instruction: self.system_instruction,
            user_prompt: self.user_prompt,
            output_schema: self.output_schema,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneratedRepositoryPlan {
    context_sha256: String,
    title: String,
    steps: Vec<GeneratedPlanStep>,
    unknowns: Vec<GeneratedChapterClaim>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneratedPlanStep {
    local_id: String,
    question: String,
    primary_path: String,
    prerequisite_ids: Vec<String>,
    claim: GeneratedChapterClaim,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepositoryPlanStep {
    pub step_id: String,
    pub question: String,
    pub primary_path: String,
    pub prerequisite_step_ids: Vec<String>,
    pub claim: ChapterClaim,
}

/// Validated, unaccepted plan data only. No chapter coverage or accepted
/// revision can be inferred from this DTO, and this module does not save it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RepositoryPlanDraft {
    pub schema_version: u32,
    pub draft_id: String,
    pub project_id: String,
    pub snapshot_id: String,
    pub question: String,
    pub title: String,
    capture_status: CaptureStatus,
    inventory_sha256: String,
    context_sha256: String,
    pub created_at_unix_ms: u64,
    coverage: PlanSourceCoverage,
    planned_selected_paths: Vec<String>,
    cited_selected_paths: Vec<String>,
    omitted_selected_paths: Vec<String>,
    pub steps: Vec<RepositoryPlanStep>,
    unknowns: Vec<ChapterClaim>,
}

pub(crate) fn prepare(
    storage: &Path,
    input: PrepareRepositoryPlanInput,
) -> Result<PreparedRepositoryPlanContext, String> {
    let context = prepare_with_contract(storage, input, RequestContract::CURRENT)?;
    if context.request_body_bytes > provider::MAX_REQUEST_BODY_BYTES {
        return Err("The selected source and code map exceed the single-request limit. Narrow the question or its source scope, then prepare a fresh review.".to_owned());
    }
    Ok(context)
}

fn prepare_for_replay(
    storage: &Path,
    input: PrepareRepositoryPlanInput,
    contract: RequestContract,
) -> Result<PreparedRepositoryPlanContext, String> {
    prepare_with_contract(storage, input, contract)
}

fn prepare_with_contract(
    storage: &Path,
    input: PrepareRepositoryPlanInput,
    contract: RequestContract,
) -> Result<PreparedRepositoryPlanContext, String> {
    if !valid_learning_question(&input.question) {
        return Err("Describe the learning question in 8–300 characters without leading/trailing whitespace or control characters.".to_owned());
    }
    let mut evidence = load_evidence(storage, &input)?;
    provider::bound_review_index_evidence(
        evidence.index_status,
        evidence.snapshot_index.is_some(),
        &mut evidence.selected_symbols,
        &mut evidence.selected_relationships,
        &mut evidence.index_evidence_truncated,
    );
    let inventory_sha256 = inventory_digest(&evidence.manifest)?;
    let eligible = evidence
        .manifest
        .files
        .iter()
        .filter(|file| eligible_file(file))
        .count();
    let selected = evidence.selected_files.len();
    if selected > eligible
        || evidence.selected_files.iter().any(|source| {
            !evidence
                .manifest
                .files
                .iter()
                .any(|file| file.path == source.path && eligible_file(file))
                || snapshot::looks_secret_like(&source.path, source.contents.as_bytes())
        })
    {
        return Err("Plan only from captured, eligible text files without secret-like content. Existing source stays local.".to_owned());
    }
    let coverage = PlanSourceCoverage {
        selected_file_count: selected,
        eligible_captured_file_count: eligible,
        unselected_eligible_file_count: eligible - selected,
        excluded_captured_path_count: evidence.manifest.files.len() - eligible,
        skipped_path_count: evidence
            .manifest
            .skipped
            .iter()
            .map(|item| &item.path)
            .collect::<BTreeSet<_>>()
            .len(),
    };
    let mut context = PreparedRepositoryPlanContext {
        project_id: input.project_id.clone(),
        snapshot_id: input.snapshot_id.clone(),
        question: input.question.clone(),
        prompt_version: contract.prompt_version(),
        capture_status: evidence.manifest.summary.status.clone(),
        inventory_sha256,
        context_sha256: String::new(),
        coverage,
        selected_files: evidence.selected_files,
        selected_symbols: evidence.selected_symbols,
        selected_relationships: evidence.selected_relationships,
        index_status: evidence.index_status,
        index_evidence_truncated: evidence.index_evidence_truncated,
        source_bytes: evidence.source_bytes,
        system_instruction: String::new(),
        user_prompt: String::new(),
        output_schema: Value::Null,
        request_body_bytes: 0,
        request_body_sha256: String::new(),
        estimated_input_tokens: 0,
        request_contract: contract,
        input,
    };
    context.apply_contract(contract)?;
    Ok(context)
}

fn valid_learning_question(question: &str) -> bool {
    bounded_text(question, 8, 300) && question == question.trim()
}

#[cfg(test)]
mod live_tests {
    use super::*;
    use crate::codex_agent::{CodexAgentManager, CodexRunInput, CodexSourceFile};

    /// Explicitly opted-in account smoke check, using synthetic code only.
    /// No plan is persisted or accepted. Diagnostic output stays at the path
    /// explicitly supplied by the person running this ignored test.
    #[test]
    #[ignore = "uses an installed Codex account and its quota; synthetic evidence only"]
    fn installed_codex_synthetic_plan() {
        let executable = std::env::var("CODE_ATLAS_LIVE_CODEX").unwrap();
        let output = std::env::var("CODE_ATLAS_LIVE_PLAN_OUTPUT").unwrap();
        let mut fixture = chapter_generation::tests::Fixture::new();
        fixture.recapture_with_source(
            "cart.ts",
            include_str!("../../benchmarks/codex-account-smoke/cart.ts"),
        );
        let context = prepare(
            &fixture.storage,
            PrepareRepositoryPlanInput {
                project_id: fixture.manifest.summary.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                question:
                    "How do subtotal and totalWithTax validate inputs and calculate a taxed total?"
                        .to_owned(),
                selected_paths: vec!["cart.ts".to_owned()],
            },
        )
        .unwrap();
        let manager = CodexAgentManager::default();
        manager.configure(&fixture.storage, &executable).unwrap();
        let result = (|| -> Result<(), String> {
            if !manager.resume_connection(&fixture.storage)?.authenticated {
                return Err("An existing Codex account is required.".to_owned());
            }
            let response = manager.run(
                &fixture.storage,
                CodexRunInput {
                    run_id: format!("synthetic-plan-{}", Uuid::new_v4()),
                    expected_runtime: manager.review_runtime(&fixture.storage)?,
                    model: "gpt-6-luna".to_owned(),
                    effort: "max".to_owned(),
                    run_deadline: crate::codex_agent::RunDeadline::TenMinutes,
                    approval_policy: "never".to_owned(),
                    system_instruction: context.system_instruction.to_owned(),
                    user_prompt: context.user_prompt.clone(),
                    output_schema: context.output_schema.clone(),
                    correction_policy: crate::claim_alignment::CorrectionPolicy::Disabled,
                    retrieval: None,
                    sources: context
                        .selected_files
                        .iter()
                        .map(|file| CodexSourceFile {
                            path: file.path.clone(),
                            sha256: file.sha256.clone(),
                            contents: file.contents.clone(),
                        })
                        .collect(),
                },
                |_| {},
            )?;
            std::fs::write(&output, &response.response_text)
                .map_err(|_| "Cannot save synthetic diagnostic.".to_owned())?;
            let plan = canonicalize(
                &fixture.storage,
                &context,
                response.response_text.as_bytes(),
            )?;
            assert!(!plan.steps.is_empty());
            Ok(())
        })();
        manager.shutdown();
        result.unwrap();
    }

    #[test]
    fn a_named_function_needs_its_name_in_the_cited_code() {
        let mut fixture = chapter_generation::tests::Fixture::new();
        let source = include_str!("../../benchmarks/codex-account-smoke/cart.ts");
        fixture.recapture_with_source("cart.ts", source);
        let context = prepare(
            &fixture.storage,
            PrepareRepositoryPlanInput {
                project_id: fixture.manifest.summary.project_id.clone(),
                snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
                question:
                    "How do subtotal and totalWithTax validate inputs and calculate a taxed total?"
                        .to_owned(),
                selected_paths: vec!["cart.ts".to_owned()],
            },
        )
        .unwrap();
        let mut proposal: Value = serde_json::from_str(include_str!(
            "../../benchmarks/codex-account-smoke/2026-10-16-rejected-plan.json"
        ))
        .unwrap();
        // Fixture identities are fresh. Isolate quote admission from index IDs.
        proposal["contextSha256"] = json!(context.context_sha256);
        for step in proposal["steps"].as_array_mut().unwrap() {
            step["claim"]["relationshipIds"] = json!([]);
        }
        let error = canonicalize(&fixture.storage, &context, proposal.to_string().as_bytes())
            .err()
            .unwrap();
        assert!(
            error.contains("does not connect source identifier 'subtotal' to its cited code"),
            "{error}"
        );
        // The same claims become reviewable with exact, complete evidence.
        let valid_response = valid_cart_plan_response(&context, source);
        canonicalize(&fixture.storage, &context, &valid_response).unwrap();
        assert!(
            context
                .user_prompt
                .contains("EVERY listed identifier must occur in at least one quote")
        );
        assert!(
            context
                .user_prompt
                .contains("inferenceBasis to an empty string")
        );
    }

    #[test]
    fn saved_plan_replays_after_a_prompt_only_upgrade() {
        let mut fixture = chapter_generation::tests::Fixture::new();
        let source = include_str!("../../benchmarks/codex-account-smoke/cart.ts");
        fixture.recapture_with_source("cart.ts", source);
        let input = PrepareRepositoryPlanInput {
            project_id: fixture.manifest.summary.project_id.clone(),
            snapshot_id: fixture.manifest.summary.snapshot_id.clone(),
            question:
                "How do subtotal and totalWithTax validate inputs and calculate a taxed total?"
                    .to_owned(),
            selected_paths: vec!["cart.ts".to_owned()],
        };
        let saved_context = prepare_with_contract(
            &fixture.storage,
            input.clone(),
            RequestContract::ExplicitEvidenceV1,
        )
        .unwrap();
        let response = valid_cart_plan_response(&saved_context, source);
        let saved = canonicalize(&fixture.storage, &saved_context, &response).unwrap();

        let current_context = prepare(&fixture.storage, input.clone()).unwrap();
        assert_eq!(saved_context.prompt_version, "grounded-repository-plan-v1");
        assert_eq!(
            current_context.prompt_version,
            "grounded-repository-plan-v2"
        );
        assert!(
            saved_context
                .review_scope()
                .same_replay_envelope(&current_context.review_scope())
        );
        assert_ne!(
            saved_context.review_scope().planning_contract_sha256,
            current_context.review_scope().planning_contract_sha256
        );

        revalidate_saved(
            &fixture.storage,
            input.clone(),
            &saved_context.review_scope(),
            &response,
            &saved,
        )
        .unwrap();

        let mut unknown_contract = saved_context.review_scope();
        unknown_contract.planning_contract_sha256 = "0".repeat(64);
        let error = revalidate_saved(
            &fixture.storage,
            input,
            &unknown_contract,
            &response,
            &saved,
        )
        .unwrap_err();
        assert!(error.contains("planning contract"), "{error}");
    }

    fn valid_cart_plan_response(context: &PreparedRepositoryPlanContext, source: &str) -> Vec<u8> {
        let mut proposal: Value = serde_json::from_str(include_str!(
            "../../benchmarks/codex-account-smoke/2026-10-16-rejected-plan.json"
        ))
        .unwrap();
        proposal["contextSha256"] = json!(context.context_sha256);
        for step in proposal["steps"].as_array_mut().unwrap() {
            step["claim"]["relationshipIds"] = json!([]);
            step["claim"]["evidence"] = json!([{"path":"cart.ts","quote":source}]);
            step["claim"]["identifiers"] = if step["localId"].as_str().unwrap().starts_with("tax") {
                json!(["totalWithTax"])
            } else {
                json!(["subtotal"])
            };
        }
        for claim in proposal["unknowns"].as_array_mut().unwrap() {
            claim["evidence"] = json!([{"path":"cart.ts","quote":source}]);
            claim["identifiers"] = json!(["RangeError"]);
        }
        serde_json::to_vec(&proposal).unwrap()
    }
}

/// Accept only a response to the same re-opened immutable planning context.
/// Agent/account/model provenance belongs to the native caller's dispatch
/// contract; this local validator cannot establish who produced the response.
pub(crate) fn canonicalize(
    storage: &Path,
    context: &PreparedRepositoryPlanContext,
    response: &[u8],
) -> Result<RepositoryPlanDraft, String> {
    canonicalize_with_identity(storage, context, response, None)
}

fn canonicalize_with_identity(
    storage: &Path,
    context: &PreparedRepositoryPlanContext,
    response: &[u8],
    identity: Option<(&str, u64)>,
) -> Result<RepositoryPlanDraft, String> {
    if response.is_empty() || response.len() > MAX_RESPONSE_BYTES {
        return Err(INVALID.to_owned());
    }
    let fresh = prepare_for_replay(storage, context.input.clone(), context.request_contract)?;
    if fresh.context_sha256 != context.context_sha256
        || fresh.request_body_sha256 != context.request_body_sha256
        || fresh.inventory_sha256 != context.inventory_sha256
    {
        return Err("The planning evidence changed. Prepare a fresh review before generating again; no plan was saved.".to_owned());
    }
    let proposal: GeneratedRepositoryPlan =
        serde_json::from_slice(response).map_err(|_| INVALID.to_owned())?;
    if proposal.context_sha256 != fresh.context_sha256
        || !bounded_text(&proposal.title, 8, 120)
        || ["overview", "repository overview", "reading plan"]
            .iter()
            .any(|value| proposal.title.eq_ignore_ascii_case(value))
        || proposal.steps.is_empty()
        || proposal.steps.len() > MAX_STEPS
        || proposal.unknowns.is_empty()
        || proposal.unknowns.len() > 8
    {
        return Err(INVALID.to_owned());
    }
    let mut evidence = load_evidence(storage, &context.input)?;
    provider::bound_review_index_evidence(
        evidence.index_status,
        evidence.snapshot_index.is_some(),
        &mut evidence.selected_symbols,
        &mut evidence.selected_relationships,
        &mut evidence.index_evidence_truncated,
    );
    // Retain the same index observation used to validate relationship IDs.
    if inventory_digest(&evidence.manifest)? != fresh.inventory_sha256
        || serde_json::to_value(&evidence.selected_relationships).map_err(|_| INVALID.to_owned())?
            != serde_json::to_value(&fresh.selected_relationships)
                .map_err(|_| INVALID.to_owned())?
        || serde_json::to_value(&evidence.selected_symbols).map_err(|_| INVALID.to_owned())?
            != serde_json::to_value(&fresh.selected_symbols).map_err(|_| INVALID.to_owned())?
        || evidence.index_status != fresh.index_status
        || evidence.index_evidence_truncated != fresh.index_evidence_truncated
    {
        return Err(INVALID.to_owned());
    }
    let sources = evidence
        .selected_files
        .iter()
        .map(|file| SnapshotSource {
            snapshot_id: context.input.snapshot_id.clone(),
            path: file.path.clone(),
            sha256: file.sha256.clone(),
            size_bytes: file.size_bytes,
            contents: file.contents.clone(),
        })
        .collect::<Vec<_>>();
    let allowed = evidence
        .selected_relationships
        .iter()
        .map(|item| item.relationship_id.clone())
        .collect::<HashSet<_>>();
    let draft_id = identity
        .map(|(id, _)| id.to_owned())
        .unwrap_or_else(|| format!("plan:{}", Uuid::new_v4()));
    let mut ids = HashMap::<String, String>::new();
    let mut planned = BTreeSet::new();
    let mut cited = BTreeSet::new();
    let mut steps = Vec::with_capacity(proposal.steps.len());
    for step in proposal.steps {
        if !local_id(&step.local_id)
            || ids.contains_key(&step.local_id)
            || !bounded_text(&step.question, 12, 180)
            || !step.question.ends_with('?')
            || step.prerequisite_ids.len() > 8
            || step.prerequisite_ids.iter().collect::<HashSet<_>>().len()
                != step.prerequisite_ids.len()
        {
            return Err(INVALID.to_owned());
        }
        let prerequisites = step
            .prerequisite_ids
            .iter()
            .map(|id| ids.get(id).cloned().ok_or_else(|| INVALID.to_owned()))
            .collect::<Result<Vec<_>, _>>()?;
        let step_id = format!("{draft_id}:{}", step.local_id);
        let claim = canonicalize_claim(&step.claim, &step_id, &evidence, &sources, &allowed)?;
        if !claim
            .anchors
            .iter()
            .any(|anchor| anchor.path == step.primary_path)
        {
            return Err(INVALID.to_owned());
        }
        planned.insert(step.primary_path.clone());
        cited.extend(claim.anchors.iter().map(|anchor| anchor.path.clone()));
        ids.insert(step.local_id, step_id.clone());
        steps.push(RepositoryPlanStep {
            step_id,
            question: step.question,
            primary_path: step.primary_path,
            prerequisite_step_ids: prerequisites,
            claim,
        });
    }
    let unknowns = proposal
        .unknowns
        .iter()
        .map(|claim| {
            if claim.category != "inference" {
                return Err(INVALID.to_owned());
            }
            canonicalize_claim(
                claim,
                &format!("{draft_id}:unknowns"),
                &evidence,
                &sources,
                &allowed,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    cited.extend(
        unknowns
            .iter()
            .flat_map(|claim| claim.anchors.iter().map(|anchor| anchor.path.clone())),
    );
    let omitted = fresh
        .selected_files
        .iter()
        .filter(|file| !planned.contains(&file.path))
        .map(|file| file.path.clone())
        .collect();
    Ok(RepositoryPlanDraft {
        schema_version: 1,
        draft_id,
        project_id: fresh.project_id,
        snapshot_id: fresh.snapshot_id,
        question: fresh.question,
        title: proposal.title,
        capture_status: fresh.capture_status,
        inventory_sha256: fresh.inventory_sha256,
        context_sha256: fresh.context_sha256,
        created_at_unix_ms: match identity {
            Some((_, time)) => time,
            None => SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|_| INVALID.to_owned())?
                .as_millis()
                .try_into()
                .map_err(|_| INVALID.to_owned())?,
        },
        coverage: fresh.coverage,
        planned_selected_paths: planned.into_iter().collect(),
        cited_selected_paths: cited.into_iter().collect(),
        omitted_selected_paths: omitted,
        steps,
        unknowns,
    })
}

/// Replay the original strict proposal against current immutable evidence.
/// Only native IDs and creation time are retained; every semantic field must
/// equal the independently rebuilt plan. This grants no new agent admission.
pub(crate) fn revalidate_saved(
    storage: &Path,
    input: PrepareRepositoryPlanInput,
    scope: &RepositoryPlanReviewScope,
    response: &[u8],
    saved: &RepositoryPlanDraft,
) -> Result<(), String> {
    if !canonical_uuid_id(&saved.draft_id, "plan")
        || saved.schema_version != 1
        || saved.created_at_unix_ms == 0
        || saved.created_at_unix_ms > (1_u64 << 53) - 1
    {
        return Err("The saved plan has an invalid native identity or creation time. Its local copy was preserved.".to_owned());
    }
    let context = prepare_for_replay(storage, input.clone(), RequestContract::CURRENT).map_err(|_| {
        "The saved plan's original source or index is unavailable. Its local copy was preserved.".to_owned()
    })?;
    let current_scope = context.review_scope();
    let replay_context = if current_scope == *scope {
        context
    } else if !scope.same_replay_envelope(&current_scope) {
        return Err("The saved plan's source or index context changed. Its local copy was preserved; regenerate the route.".to_owned());
    } else {
        RequestContract::KNOWN
            .into_iter()
            .find_map(|contract| {
                context
                    .with_contract(contract)
                    .ok()
                    .filter(|candidate| candidate.review_scope() == *scope)
            })
            .ok_or_else(|| {
                "The saved plan's source/index context or planning contract could not be reproduced. Its local copy was preserved; regenerate the route."
                    .to_owned()
            })?
    };
    let mut checked = canonicalize_with_identity(
        storage,
        &replay_context,
        response,
        Some((&saved.draft_id, saved.created_at_unix_ms)),
    ).map_err(|_| "The saved plan's original proposal could not be validated against its source. Its local copy was preserved.".to_owned())?;
    if checked.steps.len() != saved.steps.len() || checked.unknowns.len() != saved.unknowns.len() {
        return Err(
            "The saved plan's claim collection changed. Its local copy was preserved.".to_owned(),
        );
    }
    let mut ids = HashSet::new();
    for (current, original) in checked.steps.iter_mut().zip(&saved.steps) {
        retain_claim_ids(&mut current.claim, &original.claim, &mut ids)?;
    }
    for (current, original) in checked.unknowns.iter_mut().zip(&saved.unknowns) {
        retain_claim_ids(current, original, &mut ids)?;
    }
    if checked != *saved {
        return Err("The saved plan differs from its independently validated proposal and source. Its local copy was preserved.".to_owned());
    }
    Ok(())
}

fn retain_claim_ids(
    checked: &mut ChapterClaim,
    original: &ChapterClaim,
    ids: &mut HashSet<String>,
) -> Result<(), String> {
    if !canonical_uuid_id(&original.claim_id, "claim")
        || !ids.insert(original.claim_id.clone())
        || checked.anchors.len() != original.anchors.len()
    {
        return Err(
            "The saved plan has an invalid claim identity. Its local copy was preserved."
                .to_owned(),
        );
    }
    checked.claim_id = original.claim_id.clone();
    for (current, saved) in checked.anchors.iter_mut().zip(&original.anchors) {
        if !canonical_uuid_id(&saved.anchor_id, "anchor") || !ids.insert(saved.anchor_id.clone()) {
            return Err(
                "The saved plan has an invalid citation identity. Its local copy was preserved."
                    .to_owned(),
            );
        }
        current.anchor_id = saved.anchor_id.clone();
    }
    Ok(())
}

pub(crate) fn canonical_uuid_id(value: &str, prefix: &str) -> bool {
    value
        .strip_prefix(prefix)
        .and_then(|tail| tail.strip_prefix(':'))
        .and_then(|tail| {
            Uuid::parse_str(tail)
                .ok()
                .map(|uuid| uuid.to_string() == tail)
        })
        .unwrap_or(false)
}

fn load_evidence(
    storage: &Path,
    input: &PrepareRepositoryPlanInput,
) -> Result<provider::SelectedSnapshotEvidence, String> {
    provider::load_selected_snapshot_evidence(
        storage,
        &provider::BuildProviderReviewInput {
            project_id: input.project_id.clone(),
            snapshot_id: input.snapshot_id.clone(),
            target_path: None,
            selected_paths: input.selected_paths.clone(),
            model_id: String::new(),
            section_update: None,
        },
    )
    .map_err(|error| error.message)
}

fn canonicalize_claim(
    claim: &GeneratedChapterClaim,
    id: &str,
    evidence: &provider::SelectedSnapshotEvidence,
    sources: &[SnapshotSource],
    allowed: &HashSet<String>,
) -> Result<ChapterClaim, String> {
    if !bounded_text(&claim.prose, 24, 2_000)
        || (claim.category == "inference" && !bounded_text(&claim.inference_basis, 12, 500))
        || (claim.category != "inference" && !claim.inference_basis.is_empty())
        || claim.identifiers.is_empty()
        || claim.identifiers.len() > 8
        || claim.identifiers.iter().any(|id| !bounded_text(id, 1, 200))
        || claim.relationship_ids.len() > 8
        || claim.relationship_ids.iter().collect::<HashSet<_>>().len()
            != claim.relationship_ids.len()
        || claim.evidence.is_empty()
        || claim.evidence.len() > 8
    {
        return Err(INVALID.to_owned());
    }
    chapter_generation::canonicalize_repository_plan_claim(
        claim,
        id,
        &evidence.manifest,
        sources,
        evidence.snapshot_index.as_ref(),
        allowed,
    )
}

fn eligible_file(file: &snapshot::CapturedFile) -> bool {
    file.kind == InventoryPathKind::File
        && !file.secret_like
        && matches!(
            file.content_kind,
            CapturedContentKind::Text | CapturedContentKind::Unknown
        )
}

fn bounded_text(value: &str, min: usize, max: usize) -> bool {
    value == value.trim()
        && (min..=max).contains(&value.chars().count())
        && !value.chars().any(char::is_control)
}

fn local_id(value: &str) -> bool {
    (1..=40).contains(&value.len())
        && value.as_bytes()[0].is_ascii_lowercase()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn digest_json(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|_| INVALID.to_owned())
}

fn inventory_digest(manifest: &SnapshotManifest) -> Result<String, String> {
    fn frame(hash: &mut Sha256, value: &impl Serialize) -> Result<(), String> {
        let bytes = serde_json::to_vec(value).map_err(|_| INVALID.to_owned())?;
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
        Ok(())
    }
    let mut hash = Sha256::new();
    frame(
        &mut hash,
        &(
            PROMPT_VERSION,
            manifest.schema_version,
            &manifest.summary.project_id,
            &manifest.summary.snapshot_id,
            &manifest.summary.status,
            &manifest.summary.policy_version,
        ),
    )?;
    let mut files = manifest.files.iter().collect::<Vec<_>>();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    if files.windows(2).any(|pair| pair[0].path == pair[1].path) {
        return Err(INVALID.to_owned());
    }
    frame(&mut hash, &("files", files.len()))?;
    for file in files {
        frame(&mut hash, file)?;
    }
    let mut skipped = manifest.skipped.iter().collect::<Vec<_>>();
    skipped.sort_by(|left, right| {
        (&left.path, &left.reason, &left.details).cmp(&(&right.path, &right.reason, &right.details))
    });
    frame(&mut hash, &("skipped", skipped.len()))?;
    for item in skipped {
        frame(&mut hash, item)?;
    }
    let mut directories = manifest.directories.iter().collect::<Vec<_>>();
    directories.sort_by(|left, right| left.path.cmp(&right.path));
    frame(&mut hash, &("directories", directories.len()))?;
    for item in directories {
        frame(&mut hash, item)?;
    }
    Ok(format!("{:x}", hash.finalize()))
}
