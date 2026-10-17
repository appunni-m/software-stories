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
            schema_version: 2,
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

    pub(crate) fn from_retrieval(
        operation_id: &str,
        request_sha256: &str,
        ledger_sha256: &str,
        ranges: Vec<crate::repository_context::CodeRange>,
        sources: &[crate::snapshot::SnapshotSource],
        revision: &ChapterRevision,
    ) -> Result<Self, String> {
        let fingerprint = crate::repository_context::digest(
            &serde_json::to_vec(&(request_sha256, ledger_sha256))
                .map_err(|_| "The retrieval origin could not be encoded.")?,
        );
        let origin = Self {
            schema_version: 4,
            project_id: revision.project_id.clone(),
            snapshot_id: revision.snapshot_id.clone(),
            target_path: None,
            purpose: "repositoryOverview".into(),
            request_id: operation_id.into(),
            request_fingerprint: fingerprint,
            request_body_sha256: request_sha256.into(),
            contract_version: crate::evidence_ledger::RETRIEVAL_CONTRACT.into(),
            settings: RecordedAgentSettings::Codex {
                model_id: "gpt-6-luna".into(),
                effort: "max".into(),
                approval_policy: "never".into(),
            },
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
            }),
            content_sha256: content_digest(revision)?,
        };
        origin.validate(revision)?;
        Ok(origin)
    }

    /// A progressive receipt names an immutable delivered prefix, independently
    /// of the mutable operation ledger or its later terminal state.
    pub(crate) fn from_answer_checkpoint(
        operation_id: &str, request_sha256: &str, checkpoint_sha256: &str,
        ranges: Vec<crate::repository_context::CodeRange>,
        sources: &[crate::snapshot::SnapshotSource], revision: &ChapterRevision,
    ) -> Result<Self, String> {
        let mut origin = Self::from_retrieval(operation_id, request_sha256, checkpoint_sha256, ranges, sources, revision)?;
        origin.schema_version = 5;
        origin.contract_version = crate::evidence_ledger::PROGRESSIVE_CONTRACT.into();
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
        let purpose_ok = match (self.purpose.as_str(), &revision.target.scope) {
            ("repositoryOverview", ChapterTargetScope::Overview) => {
                self.target_path.is_none() && self.section_update.is_none()
            }
            ("fileChapter", ChapterTargetScope::File { path }) => {
                self.target_path.as_ref() == Some(path) && self.section_update.is_none()
            }
            ("sectionUpdate", ChapterTargetScope::File { path }) => {
                self.target_path.as_ref() == Some(path)
                    && self.section_update.as_ref().is_some_and(|target| {
                        text(&target.chapter_revision_id, 256)
                            && text(&target.section_id, 256)
                            && text(&target.section_revision_id, 256)
                    })
            }
            _ => false,
        };
        let retrieval_ok = if let Some(retrieval) = &self.retrieval {
            let mut ordered = retrieval.ranges.iter().collect::<Vec<_>>();
            ordered.sort_by_key(|r| (&r.path, r.start_byte));
            matches!(self.schema_version, 3 | 4 | 5)
                && backend == "codex"
                && self.planned_chapter.is_none()
                && self.section_update.is_none()
                && self.purpose == "repositoryOverview"
                && self.target_path.is_none()
                && self.settings
                    == RecordedAgentSettings::Codex {
                        model_id: "gpt-6-luna".into(),
                        effort: "max".into(),
                        approval_policy: "never".into(),
                    }
                && retrieval.policy == crate::evidence_ledger::POLICY
                && digest(&retrieval.ledger_sha256)
                && serde_json::to_vec(&(&self.request_body_sha256, &retrieval.ledger_sha256))
                    .is_ok_and(|bytes| {
                        crate::repository_context::digest(&bytes) == self.request_fingerprint
                    })
                && !retrieval.ranges.is_empty()
                && retrieval.ranges.len() <= 512
                && retrieval.ranges.iter().all(|r| {
                    crate::repository_context::safe_path(&r.path)
                        && r.snapshot_id == self.snapshot_id
                        && digest(&r.chunk_sha256)
                        && r.start_line > 0
                        && r.end_line >= r.start_line
                        && r.start_byte < r.end_byte
                        && r.end_byte - r.start_byte <= 32768
                        && (self.schema_version == 3 || r.end_byte - r.start_byte <= crate::native_citations::MAX_SPAN)
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
            !matches!(self.schema_version, 3 | 4 | 5)
        };
        let expected_contract = if self.retrieval.is_some() {
            if self.schema_version == 3 { "codex-retrieval-v1".to_owned() }
            else if self.schema_version == 4 { crate::evidence_ledger::RETRIEVAL_CONTRACT.to_owned() }
            else { crate::evidence_ledger::PROGRESSIVE_CONTRACT.to_owned() }
        } else if self.planned_chapter.is_some() {
            format!("{backend}-planned-chapter-v1")
        } else if backend == "codex" {
            "codex-chapter-v2".to_owned()
        } else {
            "claude-chapter-v1".to_owned()
        };
        if !matches!(self.schema_version, 1 | 2 | 3 | 4 | 5)
            || !retrieval_ok
            || revision.schema_version < 4
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
            || (self.schema_version == 1
                && legacy_content_digest(revision).is_ok_and(|hash| hash == self.content_sha256))
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

/// Omitted metadata supports legacy records. Present null is invalid rather
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

/// Origin v1 predates camelCase enum fields. Reconstruct that exact bounded
/// serialization for digest verification only; never rewrite stored bytes or
/// treat a mismatched digest as valid. New origins use v2 and the current wire.
fn legacy_content_digest(revision: &ChapterRevision) -> Result<String, String> {
    use crate::chapters::*;
    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    enum Category<'a> {
        CodeBehavior,
        RepositoryContext,
        Inference {
            visible_label: &'a str,
            basis: &'a str,
        },
    }
    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    enum Dependency<'a> {
        SourceFile {
            path: &'a str,
            source_sha256: &'a str,
        },
        Symbol {
            path: &'a str,
            source_sha256: &'a str,
            symbol_id: &'a str,
        },
        Relationship {
            path: &'a str,
            source_sha256: &'a str,
            relationship_id: &'a str,
        },
    }
    #[derive(Serialize)]
    #[serde(tag = "kind", rename_all = "camelCase")]
    enum Destination<'a> {
        Section { section_id: &'a str },
        CodeLens { anchor_id: &'a str },
        AtlasFile { path: &'a str },
        AtlasSymbol { path: &'a str, symbol_id: &'a str },
        Relationship { relationship_id: &'a str },
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Claim<'a> {
        claim_id: &'a str,
        category: Category<'a>,
        prose: &'a str,
        anchors: &'a [ChapterSourceAnchor],
        dependencies: Vec<Dependency<'a>>,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Hotspot<'a> {
        hotspot_id: &'a str,
        label: &'a str,
        destination: Destination<'a>,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Section<'a> {
        section_id: &'a str,
        section_revision_id: &'a str,
        order: u32,
        title: &'a str,
        prose: &'a str,
        spoken_text: &'a str,
        claims: Vec<Claim<'a>>,
        dependencies: Vec<Dependency<'a>>,
        visual: &'a Option<ChapterVisualPayload>,
        hotspots: Vec<Hotspot<'a>>,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Content<'a> {
        project_id: &'a str,
        snapshot_id: &'a str,
        target: &'a ChapterTarget,
        title: &'a str,
        sections: Vec<Section<'a>>,
    }
    fn dependencies(values: &[SectionDependency]) -> Vec<Dependency<'_>> {
        values
            .iter()
            .map(|value| match value {
                SectionDependency::SourceFile {
                    path,
                    source_sha256,
                } => Dependency::SourceFile {
                    path,
                    source_sha256,
                },
                SectionDependency::Symbol {
                    path,
                    source_sha256,
                    symbol_id,
                } => Dependency::Symbol {
                    path,
                    source_sha256,
                    symbol_id,
                },
                SectionDependency::Relationship {
                    path,
                    source_sha256,
                    relationship_id,
                } => Dependency::Relationship {
                    path,
                    source_sha256,
                    relationship_id,
                },
            })
            .collect()
    }
    let sections = revision
        .sections
        .iter()
        .map(|section| Section {
            section_id: &section.section_id,
            section_revision_id: &section.section_revision_id,
            order: section.order,
            title: &section.title,
            prose: &section.prose,
            spoken_text: &section.spoken_text,
            claims: section
                .claims
                .iter()
                .map(|claim| Claim {
                    claim_id: &claim.claim_id,
                    category: match &claim.category {
                        ClaimCategory::CodeBehavior => Category::CodeBehavior,
                        ClaimCategory::RepositoryContext => Category::RepositoryContext,
                        ClaimCategory::Inference {
                            visible_label,
                            basis,
                        } => Category::Inference {
                            visible_label,
                            basis,
                        },
                    },
                    prose: &claim.prose,
                    anchors: &claim.anchors,
                    dependencies: dependencies(&claim.dependencies),
                })
                .collect(),
            dependencies: dependencies(&section.dependencies),
            visual: &section.visual,
            hotspots: section
                .hotspots
                .iter()
                .map(|hotspot| Hotspot {
                    hotspot_id: &hotspot.hotspot_id,
                    label: &hotspot.label,
                    destination: match &hotspot.destination {
                        HotspotDestination::Section { section_id } => {
                            Destination::Section { section_id }
                        }
                        HotspotDestination::CodeLens { anchor_id } => {
                            Destination::CodeLens { anchor_id }
                        }
                        HotspotDestination::AtlasFile { path } => Destination::AtlasFile { path },
                        HotspotDestination::AtlasSymbol { path, symbol_id } => {
                            Destination::AtlasSymbol { path, symbol_id }
                        }
                        HotspotDestination::Relationship { relationship_id } => {
                            Destination::Relationship { relationship_id }
                        }
                    },
                })
                .collect(),
        })
        .collect();
    digest_content(&Content {
        project_id: &revision.project_id,
        snapshot_id: &revision.snapshot_id,
        target: &revision.target,
        title: &revision.title,
        sections,
    })
}

/// Stream bounded serialization into the digest without cloning the chapter or
/// retaining another serialized source/visual payload in memory.
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

    #[test]
    fn legacy_native_origin_remains_readable_without_rehashing_saved_content() {
        // A real generated, synthetic-only v1 record from before the wire fix.
        let revision: ChapterRevision =
            serde_json::from_str(include_str!("../tests/fixtures/legacy-agent-chapter.json"))
                .unwrap();
        let origin = revision.generation_origin.as_ref().unwrap();
        assert_eq!(origin.schema_version, 1);
        assert_ne!(content_digest(&revision).unwrap(), origin.content_sha256);
        assert_eq!(
            legacy_content_digest(&revision).unwrap(),
            origin.content_sha256
        );
        validate_revision_origin(&revision).unwrap();
        let serialized = serde_json::to_string(&revision).unwrap();
        assert!(serialized.contains("anchorId"));
        assert!(!serialized.contains("anchor_id"));
        let roundtrip: ChapterRevision = serde_json::from_str(&serialized).unwrap();
        validate_revision_origin(&roundtrip).unwrap();
        let mut changed = revision.clone();
        changed.sections[0].prose.push_str(" An unsupported edit.");
        assert!(validate_revision_origin(&changed).is_err());
    }

    #[test]
    fn v2_origin_requires_current_content_digest() {
        let mut revision: ChapterRevision =
            serde_json::from_str(include_str!("../tests/fixtures/legacy-agent-chapter.json"))
                .unwrap();
        let current = content_digest(&revision).unwrap();
        revision.generation_origin.as_mut().unwrap().schema_version = 2;
        assert!(validate_revision_origin(&revision).is_err());
        revision.generation_origin.as_mut().unwrap().content_sha256 = current;
        validate_revision_origin(&revision).unwrap();
        revision.sections[0]
            .spoken_text
            .push_str(" Changed narration.");
        assert!(validate_revision_origin(&revision).is_err());
    }

    #[test]
    fn retrieval_origin_records_ranges_of_large_local_sources_and_rejects_tampering() {
        use crate::{
            chapter_generation::tests::{Fixture, SIMPLE_PATH, SIMPLE_SOURCE},
            repository_context as context,
        };
        let mut fixture = Fixture::new();
        fixture.recapture_with_source(
            SIMPLE_PATH,
            &format!(
                "{SIMPLE_SOURCE}{}",
                "// large captured body\n".repeat(30000)
            ),
        );
        let sources = fixture.sources(&[SIMPLE_PATH]);
        assert!(sources[0].size_bytes > 512000);
        let read = context::read_range(
            &fixture.storage,
            &fixture.manifest,
            SIMPLE_PATH,
            &sources[0].sha256,
            0,
            SIMPLE_SOURCE.len(),
        )
        .unwrap();
        let mut revision: ChapterRevision =
            serde_json::from_str(include_str!("../tests/fixtures/legacy-agent-chapter.json"))
                .unwrap();
        revision.project_id = fixture.manifest.summary.project_id.clone();
        revision.snapshot_id = fixture.manifest.summary.snapshot_id.clone();
        revision.target.scope = ChapterTargetScope::Overview;
        revision.generation_origin = None;
        revision.generation_origin = Some(
            ChapterGenerationOrigin::from_retrieval(
                &Uuid::new_v4().to_string(),
                &"c".repeat(64),
                &"d".repeat(64),
                vec![read.range],
                &sources,
                &revision,
            )
            .unwrap(),
        );
        let encoded = serde_json::to_vec(&revision).unwrap();
        let roundtrip: ChapterRevision = serde_json::from_slice(&encoded).unwrap();
        validate_revision_origin(&roundtrip).unwrap();
        assert_eq!(roundtrip.generation_origin.as_ref().unwrap().schema_version,4);
        // Historical quote-based retrieval receipts keep their contract and
        // bytes, and remain independently readable without any inference.
        let mut historical=roundtrip.clone();
        let origin=historical.generation_origin.as_mut().unwrap();
        origin.schema_version=3;
        origin.contract_version="codex-retrieval-v1".into();
        let historical_bytes=serde_json::to_vec(&historical).unwrap();
        let historical_roundtrip:ChapterRevision=serde_json::from_slice(&historical_bytes).unwrap();
        validate_revision_origin(&historical_roundtrip).unwrap();
        assert_eq!(serde_json::to_vec(&historical_roundtrip).unwrap(),historical_bytes);
        for mode in 0..4 {
            let mut changed = roundtrip.clone();
            let receipt = changed
                .generation_origin
                .as_mut()
                .unwrap()
                .retrieval
                .as_mut()
                .unwrap();
            match mode {
                0 => receipt.ledger_sha256 = "e".repeat(64),
                1 => receipt.ranges.push(receipt.ranges[0].clone()),
                2 => receipt.ranges[0].start_line = 0,
                _ => receipt.ranges[0].end_byte = sources[0].size_bytes as usize + 1,
            }
            assert!(validate_revision_origin(&changed).is_err());
        }
    }
}
