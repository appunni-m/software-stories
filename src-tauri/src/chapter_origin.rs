//! Local dispatch lineage, not an account attestation or proof of answer quality.
//! Only native agent result paths create it; edits inherit the stored record.

use crate::{
    chapters::{ChapterRevision, ChapterTargetScope},
    planned_chapter::PlannedChapterScope,
    provider,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use uuid::Uuid;

pub(crate) const CHAPTER_ORIGIN_SCHEMA_VERSION: u32 = 6;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "backend", deny_unknown_fields)]
pub enum RecordedAgentSettings {
    #[serde(rename = "codex", rename_all = "camelCase")]
    Codex {
        model_id: String,
        effort: String,
        approval_policy: String,
    },
    #[serde(rename = "claude-agent", rename_all = "camelCase")]
    Claude {
        model_id: String,
        effort: Option<String>,
        approval_policy: String,
        max_turns: u32,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordedSourceDigest {
    path: String,
    sha256: String,
    size_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecordedRetrievalEvidence {
    policy: String,
    ledger_sha256: String,
    ranges: Vec<crate::repository_context::CodeRange>,
    index_identity: RecordedIndexIdentity,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "camelCase", deny_unknown_fields)]
pub enum RecordedIndexIdentity {
    Available {
        schema_version: u32,
        indexer_version: String,
    },
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChapterGenerationOrigin {
    pub schema_version: u32,
    pub project_id: String,
    pub snapshot_id: String,
    pub target_path: Option<String>,
    pub purpose: String,
    pub request_id: String,
    pub request_fingerprint: String,
    pub request_body_sha256: String,
    pub contract_version: String,
    pub settings: RecordedAgentSettings,
    pub source_files: Vec<RecordedSourceDigest>,
    pub section_update: Option<provider::SectionUpdateReviewTarget>,
    pub(crate) planned_chapter: Option<PlannedChapterScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) retrieval: Option<RecordedRetrievalEvidence>,
    /// Digest of original project/snapshot/target/title/sections. Lifecycle,
    /// review IDs and this lineage record are deliberately excluded.
    pub content_sha256: String,
}

impl ChapterGenerationOrigin {
    pub(crate) fn retrieval_ranges(
        &self,
    ) -> Option<&[crate::repository_context::CodeRange]> {
        self.retrieval
            .as_ref()
            .map(|retrieval| retrieval.ranges.as_slice())
    }

    pub(crate) fn retrieval_index_identity(&self) -> Option<String> {
        match &self.retrieval.as_ref()?.index_identity {
            RecordedIndexIdentity::Available {
                schema_version,
                indexer_version,
            } => Some(format!("{schema_version}:{indexer_version}")),
            RecordedIndexIdentity::Unavailable => None,
        }
    }

    pub(crate) fn from_agent_review(
        review: &provider::ProviderRequestReview,
        revision: &ChapterRevision,
    ) -> Result<Self, String> {
        let config = &review.generation_config;
        let string = |key: &str| {
            config[key]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "The recorded agent settings are incomplete.".to_owned())
        };
        let settings = match (review.provider_id.as_str(), review.endpoint_url.as_str()) {
            ("codex", "codex-app-server://stdio") => RecordedAgentSettings::Codex {
                model_id: review.model_id.clone(),
                effort: string("effort")?,
                approval_policy: string("approvalPolicy")?,
            },
            ("claude-agent", "claude-agent-cli://stdio") => {
                let settings =
                    provider::claude_settings_from_review(review).map_err(|error| error.message)?;
                RecordedAgentSettings::Claude {
                    model_id: settings.model_id,
                    effort: settings.effort,
                    approval_policy: settings.approval_policy,
                    max_turns: settings.max_turns,
                }
            }
            _ => {
                return Err(
                    "Only native Codex or Claude dispatch can record agent generation lineage."
                        .to_owned(),
                );
            }
        };
        let planned_chapter = config
            .get("plannedChapter")
            .map(|_| provider::planned_chapter_review_scope(review).map_err(|error| error.message))
            .transpose()?;
        let origin = Self {
            schema_version: CHAPTER_ORIGIN_SCHEMA_VERSION,
            project_id: review.project_id.clone(),
            snapshot_id: review.snapshot_id.clone(),
            target_path: review.target_path.clone(),
            purpose: review.purpose.clone(),
            request_id: review.review_id.clone(),
            request_fingerprint: review.fingerprint.clone(),
            request_body_sha256: review.request_body_sha256.clone(),
            contract_version: string("contractVersion")?,
            settings,
            source_files: review
                .selected_files
                .iter()
                .map(|file| RecordedSourceDigest {
                    path: file.path.clone(),
                    sha256: file.sha256.clone(),
                    size_bytes: file.size_bytes,
                })
                .collect(),
            section_update: review.section_update_context.as_ref().map(|context| {
                provider::SectionUpdateReviewTarget {
                    chapter_revision_id: context.chapter_revision_id.clone(),
                    section_id: context.section_id.clone(),
                    section_revision_id: context.section_revision_id.clone(),
                    repair_session_id: context.repair_session_id.clone(),
                }
            }),
            planned_chapter,
            retrieval: None,
            content_sha256: content_digest(revision)?,
        };
        origin.validate(revision)?;
        if !origin.matches_content(revision) {
            return Err(
                "The agent generation record does not match its original chapter content."
                    .to_owned(),
            );
        }
        Ok(origin)
    }

    pub(crate) fn from_answer_checkpoint(
        operation_id: &str,
        request_sha256: &str,
        ledger_sha256: &str,
        model_id: &str,
        effort: &str,
        ranges: Vec<crate::repository_context::CodeRange>,
        index: Option<&crate::index::SnapshotIndex>,
        sources: &[crate::snapshot::SnapshotSource],
        revision: &ChapterRevision,
    ) -> Result<Self, String> {
        let settings = RecordedAgentSettings::Codex {
            model_id: model_id.into(),
            effort: effort.into(),
            approval_policy: "never".into(),
        };
        let index_identity = index.map_or(RecordedIndexIdentity::Unavailable, |index| {
            RecordedIndexIdentity::Available {
                schema_version: index.schema_version,
                indexer_version: index.indexer_version.clone(),
            }
        });
        let fingerprint = crate::repository_context::digest(
            &serde_json::to_vec(&(request_sha256, ledger_sha256, &index_identity, &settings))
                .map_err(|_| "The retrieval origin could not be encoded.")?,
        );
        let origin = Self {
            schema_version: CHAPTER_ORIGIN_SCHEMA_VERSION,
            project_id: revision.project_id.clone(),
            snapshot_id: revision.snapshot_id.clone(),
            target_path: None,
            purpose: "repositoryOverview".into(),
            request_id: operation_id.into(),
            request_fingerprint: fingerprint,
            request_body_sha256: request_sha256.into(),
            contract_version: crate::evidence_ledger::PROGRESSIVE_CONTRACT.into(),
            settings,
            source_files: sources
                .iter()
                .map(|s| RecordedSourceDigest {
                    path: s.path.clone(),
                    sha256: s.sha256.clone(),
                    size_bytes: s.size_bytes,
                })
                .collect(),
            section_update: None,
            planned_chapter: None,
            retrieval: Some(RecordedRetrievalEvidence {
                policy: crate::evidence_ledger::POLICY.into(),
                ledger_sha256: ledger_sha256.into(),
                ranges,
                index_identity,
            }),
            content_sha256: content_digest(revision)?,
        };
        origin.validate(revision)?;
        Ok(origin)
    }

    pub(crate) fn validate(&self, revision: &ChapterRevision) -> Result<(), String> {
        let invalid = || "The chapter's recorded agent generation metadata is invalid.".to_owned();
        let paths = self
            .source_files
            .iter()
            .map(|file| &file.path)
            .collect::<BTreeSet<_>>();
        let model_ok = |model: &str| text(model, 512);
        let (backend, settings_ok) = match &self.settings {
            RecordedAgentSettings::Codex {
                model_id,
                effort,
                approval_policy,
            } => (
                "codex",
                model_ok(model_id) && text(effort, 32) && approval_policy == "never",
            ),
            RecordedAgentSettings::Claude {
                model_id,
                effort,
                approval_policy,
                max_turns,
            } => (
                "claude",
                model_ok(model_id)
                    && effort.as_deref().is_none_or(|value| text(value, 32))
                    && approval_policy == "dontAsk"
                    && (1..=8).contains(max_turns),
            ),
        };
        let section_update_ok = || {
            self.section_update.as_ref().is_some_and(|target| {
                text(&target.chapter_revision_id, 256)
                    && text(&target.section_id, 256)
                    && text(&target.section_revision_id, 256)
            })
        };
        let purpose_ok = match (self.purpose.as_str(), &revision.target.scope) {
            ("repositoryOverview", ChapterTargetScope::Overview) => {
                self.target_path.is_none() && self.section_update.is_none()
            }
            ("fileChapter", ChapterTargetScope::File { path }) => {
                self.target_path.as_ref() == Some(path) && self.section_update.is_none()
            }
            ("sectionUpdate", ChapterTargetScope::Overview) => {
                self.target_path.is_none() && section_update_ok()
            }
            ("sectionUpdate", ChapterTargetScope::File { path }) => {
                self.target_path.as_ref() == Some(path) && section_update_ok()
            }
            ("sectionUpdate", ChapterTargetScope::Group { .. }) => {
                self.target_path.is_none() && section_update_ok()
            }
            _ => false,
        };
        let retrieval_ok = if let Some(retrieval) = &self.retrieval {
            let mut ordered = retrieval.ranges.iter().collect::<Vec<_>>();
            ordered.sort_by_key(|r| (&r.path, r.start_byte));
            let current_retrieval = self.schema_version == CHAPTER_ORIGIN_SCHEMA_VERSION
                && matches!(
                    &retrieval.index_identity,
                    RecordedIndexIdentity::Available { .. }
                        | RecordedIndexIdentity::Unavailable
                );
            let fingerprint_valid = current_retrieval
                && serde_json::to_vec(&(
                    &self.request_body_sha256,
                    &retrieval.ledger_sha256,
                    &retrieval.index_identity,
                    &self.settings,
                ))
                .is_ok_and(|bytes| {
                    crate::repository_context::digest(&bytes) == self.request_fingerprint
                });
            current_retrieval
                && backend == "codex"
                && self.planned_chapter.is_none()
                && self.section_update.is_none()
                && self.purpose == "repositoryOverview"
                && self.target_path.is_none()
                && retrieval.policy == crate::evidence_ledger::POLICY
                && match &retrieval.index_identity {
                    RecordedIndexIdentity::Available {
                        schema_version,
                        indexer_version,
                    } => current_retrieval && *schema_version > 0 && text(indexer_version, 128),
                    RecordedIndexIdentity::Unavailable => current_retrieval,
                }
                && digest(&retrieval.ledger_sha256)
                && fingerprint_valid
                && !retrieval.ranges.is_empty()
                && retrieval.ranges.len() <= 512
                // One checkpoint may support an overview with multiple sections.
                && retrieval.ranges.iter().all(|r| {
                    crate::repository_context::safe_path(&r.path)
                        && r.snapshot_id == self.snapshot_id
                        && digest(&r.chunk_sha256)
                        && r.start_line > 0
                        && r.end_line >= r.start_line
                        && r.start_byte < r.end_byte
                        && r.end_byte - r.start_byte <= 32768
                        && r.end_byte - r.start_byte <= crate::native_citations::MAX_SPAN
                        && self.source_files.iter().any(|s| {
                            s.path == r.path
                                && s.sha256 == r.source_sha256
                                && r.end_byte as u64 <= s.size_bytes
                        })
                })
                && ordered.windows(2).all(|pair| {
                    pair[0].path != pair[1].path || pair[0].end_byte <= pair[1].start_byte
                })
                && retrieval
                    .ranges
                    .iter()
                    .map(|r| (r.end_byte.saturating_sub(r.start_byte)) as u128)
                    .sum::<u128>()
                    <= 250000
                && self
                    .source_files
                    .iter()
                    .all(|s| retrieval.ranges.iter().any(|r| r.path == s.path))
        } else {
            self.schema_version == CHAPTER_ORIGIN_SCHEMA_VERSION
        };
        let expected_contract = if self.retrieval.is_some() {
            crate::evidence_ledger::PROGRESSIVE_CONTRACT.to_owned()
        } else if self.planned_chapter.is_some() {
            format!("{backend}-planned-chapter-v1")
        } else if backend == "codex" {
            "codex-chapter-v2".to_owned()
        } else {
            "claude-chapter-v1".to_owned()
        };
        if self.schema_version != CHAPTER_ORIGIN_SCHEMA_VERSION
            || !retrieval_ok
            || revision.schema_version != crate::chapters::CHAPTER_SCHEMA_VERSION
            || self.project_id != revision.project_id
            || !text(&self.project_id, 4096)
            || !text(&self.snapshot_id, 256)
            || !settings_ok
            || !purpose_ok
            || Uuid::parse_str(&self.request_id)
                .ok()
                .is_none_or(|id| id.to_string() != self.request_id)
            || !digest(&self.request_fingerprint)
            || !digest(&self.request_body_sha256)
            || !digest(&self.content_sha256)
            || self.contract_version != expected_contract
            || paths.is_empty()
            || paths.len() > 12
            || paths.len() != self.source_files.len()
            || self
                .source_files
                .iter()
                .any(|file| !text(&file.path, 4096) || !digest(&file.sha256))
            || self
                .source_files
                .iter()
                .try_fold(0_u64, |total, file| total.checked_add(file.size_bytes))
                .is_none_or(|total| {
                    total
                        > if self.retrieval.is_some() {
                            100 * 1024 * 1024
                        } else {
                            250_000
                        }
                })
            || self
                .target_path
                .as_ref()
                .is_some_and(|path| !paths.contains(path))
        {
            return Err(invalid());
        }
        if let Some(scope) = &self.planned_chapter {
            if self.purpose != "fileChapter"
                || !scope.valid_recorded_shape()
                || scope.snapshot_id != self.snapshot_id
                || self.target_path.as_deref() != Some(scope.primary_path.as_str())
                || scope.selected_paths.iter().collect::<BTreeSet<_>>() != paths
            {
                return Err(invalid());
            }
        }
        // Descendant edits/restores may carry historical lineage. An original
        // generated draft may never claim a different original content digest.
        if revision.based_on_revision_id.is_none() && !self.matches_content(revision) {
            return Err(invalid());
        }
        Ok(())
    }

    pub(crate) fn matches_content(&self, revision: &ChapterRevision) -> bool {
        content_digest(revision).is_ok_and(|hash| hash == self.content_sha256)
    }

    pub(crate) fn model_id(&self) -> &str {
        match &self.settings {
            RecordedAgentSettings::Codex { model_id, .. }
            | RecordedAgentSettings::Claude { model_id, .. } => model_id,
        }
    }
}

pub(crate) fn validate_revision_origin(revision: &ChapterRevision) -> Result<(), String> {
    if let Some(origin) = &revision.generation_origin {
        origin.validate(revision)?;
    }
    Ok(())
}

/// Direct API and manual drafts may omit metadata. Present null is invalid rather
/// than silently losing the generation identity during recovery/import.
pub(crate) fn deserialize_origin<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<ChapterGenerationOrigin>, D::Error> {
    ChapterGenerationOrigin::deserialize(deserializer).map(Some)
}

fn content_digest(revision: &ChapterRevision) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Content<'a> {
        project_id: &'a str,
        snapshot_id: &'a str,
        target: &'a crate::chapters::ChapterTarget,
        title: &'a str,
        sections: &'a [crate::chapters::ChapterSection],
    }
    digest_content(&Content {
        project_id: &revision.project_id,
        snapshot_id: &revision.snapshot_id,
        target: &revision.target,
        title: &revision.title,
        sections: &revision.sections,
    })
}

fn digest_content(content: &impl Serialize) -> Result<String, String> {
    let mut writer = ContentDigestWriter {
        hasher: Sha256::new(),
        bytes: 0,
    };
    serde_json::to_writer(&mut writer, content).map_err(|_| {
        "The generated chapter content could not be fingerprinted within its 4 MB limit.".to_owned()
    })?;
    Ok(format!("{:x}", writer.hasher.finalize()))
}

struct ContentDigestWriter {
    hasher: Sha256,
    bytes: usize,
}
impl std::io::Write for ContentDigestWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let total = self
            .bytes
            .checked_add(bytes.len())
            .filter(|total| *total <= 4 * 1024 * 1024)
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "chapter content exceeds digest limit",
                )
            })?;
        self.hasher.update(bytes);
        self.bytes = total;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn text(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(char::is_control)
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE, simple_proposal_text};

    fn chapter(f: &Fixture) -> ChapterRevision {
        crate::chapter_generation::canonicalize_file_chapter(
            &f.manifest.summary.project_id,
            &f.manifest,
            Some(&f.index),
            SIMPLE_PATH,
            &f.sources(&[SIMPLE_PATH]),
            &std::collections::HashSet::new(),
            &simple_proposal_text(),
        )
        .unwrap()
    }

    #[test]
    fn current_agent_origin_has_one_wire_version_and_detects_content_changes() {
        let f = Fixture::new();
        let revision = chapter(&f);
        let runtime = crate::codex_identity::CodexReviewRuntime {
            connection_id: Uuid::new_v4().to_string(),
            executable_path: "/usr/local/bin/codex".into(),
            executable_sha256: "a".repeat(64),
            account_identity_sha256: "b".repeat(64),
            account_revision: 1,
        };
        let review = provider::build_codex_chapter_review(
            &f.storage,
            &provider::BuildCodexReviewInput {
                project_id: revision.project_id.clone(),
                snapshot_id: revision.snapshot_id.clone(),
                target_path: Some(SIMPLE_PATH.into()),
                selected_paths: vec![SIMPLE_PATH.into()],
                model_id: "gpt-6-luna".into(),
                effort: "max".into(),
                approval_policy: "never".into(),
                section_update: None,
                question: None,
                focus: vec![],
                workspace_grant: None,
            },
            &runtime,
        )
        .unwrap()
        .review;
        let origin = ChapterGenerationOrigin::from_agent_review(&review, &revision).unwrap();
        assert_eq!(origin.schema_version, 6);
        assert_eq!(
            serde_json::from_slice::<ChapterGenerationOrigin>(
                &serde_json::to_vec(&origin).unwrap()
            )
            .unwrap(),
            origin
        );
        for version in [0, 1, 2, 3, 4, 5] {
            let mut unsupported = origin.clone();
            unsupported.schema_version = version;
            assert!(unsupported.validate(&revision).is_err());
        }
        let mut changed = revision.clone();
        changed.title.push_str(" changed");
        assert!(!origin.matches_content(&changed));
        assert!(origin.validate(&changed).is_err());
        changed.based_on_revision_id = Some(revision.revision_id.clone());
        assert!(origin.validate(&changed).is_ok());
    }

    #[test]
    fn current_checkpoint_origin_limits_each_delivered_fragment_and_rejects_tampering() {
        let f = Fixture::new();
        let mut revision = chapter(&f);
        revision.chapter_id = "chapter:overview".into();
        revision.target.target_id = "target:overview".into();
        revision.target.scope = ChapterTargetScope::Overview;
        revision.sections.truncate(1);
        let sources = f.sources(&[SIMPLE_PATH]);
        let range =
            crate::repository_context::range_from_source(&sources[0], 0, SIMPLE_SOURCE.len())
                .unwrap()
                .range;
        let origin = ChapterGenerationOrigin::from_answer_checkpoint(
            &Uuid::new_v4().to_string(),
            &"a".repeat(64),
            &"b".repeat(64),
            "gpt-6-luna",
            "max",
            vec![range],
            None,
            &sources,
            &revision,
        )
        .unwrap();
        assert_eq!(origin.schema_version, 6);
        assert_eq!(
            origin.contract_version,
            crate::evidence_ledger::PROGRESSIVE_CONTRACT
        );
        assert!(origin.validate(&revision).is_ok());
        let mut changed = origin.clone();
        changed.retrieval.as_mut().unwrap().ranges[0].end_byte = 1001;
        assert!(changed.validate(&revision).is_err());
        let mut changed = origin.clone();
        changed.retrieval.as_mut().unwrap().ledger_sha256 = "c".repeat(64);
        assert!(changed.validate(&revision).is_err());
    }

    #[test]
    fn schema_five_repository_origin_is_rejected_without_implicit_index_identity() {
        let f = Fixture::new();
        let mut revision = chapter(&f);
        revision.chapter_id = "chapter:overview".into();
        revision.target.target_id = "target:overview".into();
        revision.target.scope = ChapterTargetScope::Overview;
        revision.sections.truncate(1);
        let sources = f.sources(&[SIMPLE_PATH]);
        let range =
            crate::repository_context::range_from_source(&sources[0], 0, SIMPLE_SOURCE.len())
                .unwrap()
                .range;
        let mut origin = ChapterGenerationOrigin::from_answer_checkpoint(
            &Uuid::new_v4().to_string(),
            &"a".repeat(64),
            &"b".repeat(64),
            "gpt-6-luna",
            "max",
            vec![range],
            None,
            &sources,
            &revision,
        )
        .unwrap();
        origin.schema_version = 5;
        assert!(origin.validate(&revision).is_err());

        let mut old_wire = serde_json::to_value(&origin).unwrap();
        old_wire["retrieval"]
            .as_object_mut()
            .unwrap()
            .remove("indexIdentity");
        assert!(
            serde_json::from_value::<ChapterGenerationOrigin>(old_wire).is_err(),
            "a missing index identity is not treated as an implicit supported value"
        );
    }
}
