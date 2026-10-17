//! Explicit read-only local diagnosis of preserved proposal rejections.
//! No inference, account connection, receipt rewrite or acceptance occurs.

use super::*;
use std::io::Write;

#[test]
#[ignore = "Explicit local saved-section inspection; requires private data/run/output paths. Uses no account or model."]
fn inspect_saved_section_rejections() {
    let storage = PathBuf::from(
        std::env::var_os("CODE_ATLAS_SECTION_INSPECT_DATA_DIR")
            .expect("set CODE_ATLAS_SECTION_INSPECT_DATA_DIR"),
    );
    let run = std::env::var("CODE_ATLAS_SECTION_INSPECT_RUN_ID")
        .expect("set CODE_ATLAS_SECTION_INSPECT_RUN_ID");
    let output = PathBuf::from(
        std::env::var_os("CODE_ATLAS_SECTION_INSPECT_OUTPUT")
            .expect("set a fresh CODE_ATLAS_SECTION_INSPECT_OUTPUT"),
    );
    assert!(storage.is_absolute() && output.is_absolute());
    let record = super::super::inspect(&storage, &run).unwrap();
    let answer = inspect(&storage, &run).unwrap();
    assert!(
        matches!(answer.schema_version, 2 | 3 | 4 | 5 | 6),
        "inspection requires short-reference answers"
    );
    assert_eq!(answer.binding.operation_id, record.binding.operation_id);
    assert_eq!(answer.binding.project_id, record.binding.project_id);
    assert_eq!(answer.binding.snapshot_id, record.binding.snapshot_id);
    let manifest = snapshot::load_snapshot_manifest(
        &storage,
        &record.binding.project_id,
        &record.binding.snapshot_id,
    )
    .unwrap();
    assert_eq!(
        record.binding.manifest_sha256,
        context::digest(&serde_json::to_vec(&manifest).unwrap())
    );
    let syntax = crate::index::load_snapshot_index(
        &storage,
        &record.binding.project_id,
        &record.binding.snapshot_id,
    )
    .unwrap();
    let mut reports = Vec::new();
    for (index, attempt) in answer
        .attempts
        .iter()
        .enumerate()
        .filter(|(_, a)| !a.findings.is_empty())
    {
        let mut proposal: Proposal = serde_json::from_str(&attempt.arguments).unwrap();
        let position = record
            .calls
            .iter()
            .position(|call| {
                call.tool == Tool::ProposeSection
                    && call.arguments_sha256 == attempt.arguments_sha256
            })
            .expect("the rejected proposal must have an owned tool call");
        // Later reads must not make an earlier rejection accidentally valid.
        let delivered = &record.calls[..position];
        let mut sources = HashMap::new();
        let mut citations = HashMap::new();
        for (ordinal, evidence) in record.evidence.iter().enumerate() {
            if !delivered.iter().any(|call| {
                call.state == CallState::Delivered && call.evidence_ids.contains(&evidence.id)
            }) {
                continue;
            }
            if !sources.contains_key(&evidence.range.path) {
                sources.insert(
                    evidence.range.path.clone(),
                    context::source(&storage, &manifest, &evidence.range.path).unwrap(),
                );
            }
            let source = &sources[&evidence.range.path];
            assert_eq!(
                context::range_from_source(
                    source,
                    evidence.range.start_byte,
                    evidence.range.end_byte
                )
                .unwrap()
                .range,
                evidence.range
            );
            let fragment = &source.contents[evidence.range.start_byte..evidence.range.end_byte];
            if native_citations::citeable(fragment) {
                let citation =
                    native_citations::Citation::checked(&evidence.range, source).unwrap();
                citations.insert(evidence.id.clone(), citation.clone());
                citations.insert(native_citations::short_reference(ordinal), citation);
            }
        }
        let source_list = sources.into_values().collect::<Vec<_>>();
        // A separate, explicitly hypothetical fresh-policy comparison. Never
        // change the saved proposal, checkpoint, outcome or consumed attempts.
        // Resolve only source delivered before this exact rejected call.
        let grounded_policy = (|| -> Result<Value, Vec<Value>> {
            let mut fresh: Proposal = serde_json::from_str(&attempt.arguments).unwrap();
            fresh.proposal_version = GROUNDED_NAMES_PROPOSAL_VERSION.to_owned();
            let (response, _) = wrapped_authored(&mut fresh)?;
            let resolved = resolved_authored(&fresh, native_citations::resolve(&response, |id| {
                citations.get(id).cloned().ok_or("referenceUnavailable")
            }))?;
            let mut findings = alignment_findings(&fresh.section, resolved.text());
            findings.extend(visual_shape_findings(&fresh.section));
            findings.extend(scene_version_findings(&fresh.section, GROUNDED_NAMES_CHECKPOINT_VERSION));
            findings.extend(scene_narration_findings(&fresh.section));
            if !findings.is_empty() {
                return Err(findings);
            }
            let (error, validation) = match chapter_generation::canonicalize_narrated_answer_section(
                &record.binding.project_id, &manifest, syntax.as_ref(), &source_list, &resolved,
            ) {
                Err(error) => (Some(error), Value::Null),
                Ok(revision) => (None, serde_json::to_value(chapters::validate_chapter_revision(
                    &storage, &manifest, syntax.as_ref(), &revision,
                )).unwrap()),
            };
            Ok(json!({"findings":[],"canonicalizationError":error,"validation":validation}))
        })().unwrap_or_else(|findings| json!({"findings":findings,"canonicalizationError":null,"validation":null}));
        let response = match wrapped_authored(&mut proposal) {
            Ok((response, _)) => response,
            Err(findings) => {
                reports.push(json!({"attempt":index+1,"sectionKey":proposal.section_key,"originalFindings":attempt.findings,"currentAuthoringFindings":findings,"canonicalizationError":null,"validation":null,"hypotheticalFreshGroundedPolicy":grounded_policy}));
                continue;
            }
        };
        let resolved = resolved_authored(&proposal, native_citations::resolve(&response, |id| {
            citations.get(id).cloned().ok_or("referenceUnavailable")
        }));
        let (error, validation) = match resolved {
            Err(_) => {
                (Some("referenceUnavailable".to_owned()), Value::Null)
            }
            Ok(resolved) => {
                match chapter_generation::canonicalize_narrated_answer_section(
                    &record.binding.project_id,
                    &manifest,
                    syntax.as_ref(),
                    &source_list,
                    &resolved,
                ) {
                    Err(error) => (Some(error), Value::Null),
                    Ok(revision) => (
                        None,
                        serde_json::to_value(chapters::validate_chapter_revision(
                            &storage,
                            &manifest,
                            syntax.as_ref(),
                            &revision,
                        ))
                        .unwrap(),
                    ),
                }
            }
        };
        reports.push(json!({"attempt":index+1,"sectionKey":proposal.section_key,"originalFindings":attempt.findings,"canonicalizationError":error,"validation":validation,"hypotheticalFreshGroundedPolicy":grounded_policy}));
    }
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut artifact = options.open(output).unwrap();
    let final_manifests = answer.final_responses.iter().map(|response| json!({
        "sha256":response.sha256,"bytes":response.text.len(),
        "admissibleUnderRecordedSchema":answer.manifest_topics(&response.text).is_ok()
    })).collect::<Vec<_>>();
    artifact.write_all(&serde_json::to_vec_pretty(&json!({
        "kind":"read-only-saved-section-rejection-inspection","runId":run,
        "scope":"Replay only this recorded operation's pre-proposal delivered source through current narrated-section validators. Original receipts are untouched. No account, inference, publication, acceptance or semantic review.",
        "hypotheticalFreshGroundedPolicyScope":"Independently compare unchanged authored content and the same pre-proposal delivered sources under the current grounded-name policy for a hypothetical fresh proposal. Does not change historical rejection, outcome, attempts, final manifests, ownership or any receipt; does not qualify real generation or semantic meaning.",
        "accepted":false,"humanReview":false,"voiceReview":false,"reports":reports,
        "answerSchemaVersion":answer.schema_version,"finalManifests":final_manifests
    })).unwrap()).unwrap();
    artifact.sync_all().unwrap();
}
