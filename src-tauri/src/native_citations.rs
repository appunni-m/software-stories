//! Citation transport for an owned investigation. The agent selects opaque
//! delivered IDs; native code supplies bytes and coordinates. This module does
//! not fetch code, grant access, or certify the meaning of a claim.

use crate::{
    chapters::MAX_GENERATED_RESPONSE_BYTES, repository_context as context, snapshot::SnapshotSource,
};
use serde_json::{Value, json};

pub(crate) const VERSION: &str = "nativeEvidenceV1";
pub(crate) const SHORT_VERSION: &str = "nativeEvidenceV2";
pub(crate) const MAX_REFERENCES: usize = 512;
pub(crate) const MAX_SPAN: usize = 1_000;
const MAX_ISSUES: usize = 64;

/// One-based ordinal in an immutable operation/checkpoint, never a source hash.
pub(crate) fn short_reference(index: usize) -> String {
    format!("code:{}", index + 1)
}

fn reference_shape(id: &str, short: bool) -> bool {
    if short {
        let Some(ordinal) = id.strip_prefix("code:") else {
            return false;
        };
        return !ordinal.starts_with('0')
            && ordinal.len() <= 3
            && ordinal.bytes().all(|b| b.is_ascii_digit())
            && ordinal
                .parse::<usize>()
                .is_ok_and(|n| (1..=MAX_REFERENCES).contains(&n));
    }
    id.len() == 73
        && id.starts_with("evidence:")
        && id[9..]
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Clone, Debug)]
pub(crate) struct Citation {
    range: context::CodeRange,
    excerpt: String,
}

impl Citation {
    /// Retain a native-delivered starting range for a research outline.
    pub(crate) fn range(&self) -> &context::CodeRange {
        &self.range
    }

    /// Only the ledger calls this after proving current ownership and delivery.
    /// Recheck against the full immutable source without substring matching.
    pub(crate) fn checked(
        range: &context::CodeRange,
        source: &SnapshotSource,
    ) -> Result<Self, String> {
        let read = context::range_from_source(source, range.start_byte, range.end_byte)?;
        if &read.range != range || !citeable(&read.contents) {
            return Err(
                "The native citation does not match its captured bytes and coordinates.".into(),
            );
        }
        Ok(Self {
            range: range.clone(),
            excerpt: read.contents,
        })
    }

    pub(crate) fn checked_start(
        &self,
        path: &str,
        quote: &str,
        source: &SnapshotSource,
    ) -> Result<usize, String> {
        if path != self.range.path || quote != self.excerpt {
            return Err("A resolved native citation was changed before admission.".into());
        }
        Self::checked(&self.range, source)?;
        Ok(self.range.start_byte)
    }
}

pub(crate) fn citeable(contents: &str) -> bool {
    (8..=MAX_SPAN).contains(&contents.len()) && !contents.trim().is_empty()
}

/// Partition only the already requested gap. Prefer line ends; preserve UTF-8
/// and CRLF. Short/blank fragments are delivered but explicitly not citeable.
pub(crate) fn spans(source: &str, start: usize, end: usize) -> Result<Vec<(usize, usize)>, String> {
    source
        .get(start..end)
        .ok_or("The citation range is not valid UTF-8.")?;
    let mut result = vec![];
    let mut from = start;
    while from < end {
        let mut to = end.min(from + MAX_SPAN);
        while !source.is_char_boundary(to) {
            to -= 1;
        }
        if to < end {
            if source.as_bytes()[to - 1] == b'\r' && source.as_bytes()[to] == b'\n' {
                to -= 1;
            }
            if let Some(newline) = source.as_bytes()[from..to]
                .iter()
                .rposition(|b| *b == b'\n')
            {
                if newline + 1 >= MAX_SPAN / 2 {
                    to = from + newline + 1;
                }
            }
        }
        result.push((from, to));
        from = to;
    }
    Ok(result)
}

/// No Deserialize implementation: provider JSON cannot create trusted spans.
pub(crate) struct Resolved {
    text: String,
    spans: Vec<Vec<Vec<Citation>>>,
}
impl Resolved {
    pub(crate) fn text(&self) -> &str {
        &self.text
    }
    pub(crate) fn claim_spans(&self, section: usize, claim: usize) -> Option<&[Citation]> {
        self.spans.get(section)?.get(claim).map(Vec::as_slice)
    }
}

pub(crate) enum Resolution {
    Ready(Resolved),
    Invalid(Vec<Value>),
}

fn issue(location: &str, code: &'static str) -> Value {
    json!({"location":location,"citationProblem":code})
}
fn invalid(location: &str, code: &'static str) -> Resolution {
    Resolution::Invalid(vec![issue(location, code)])
}

/// Fixed positions/codes for private recovery. Do not persist resolved quotes
/// or additional source in diagnostic fields.
pub(crate) fn fixed_findings(issues: &[Value]) -> Vec<Value> {
    issues.iter().take(MAX_ISSUES).map(|finding| json!({
        "location":finding["location"],
        "citationProblem":finding.get("citationProblem").and_then(Value::as_str).unwrap_or("identifierAlignment"),
    })).collect()
}

pub(crate) fn valid_findings(findings: &[Value]) -> bool {
    findings.len() <= MAX_ISSUES
        && findings.iter().all(|finding| {
            finding.as_object().is_some_and(|o| o.len() == 2)
                && finding["location"].as_str().is_some_and(|s| {
                    s.len() <= 160 && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'/')
                })
                && finding["citationProblem"].as_str().is_some_and(|s| {
                    matches!(
                        s,
                        "citationResponseLimit"
                            | "citationShape"
                            | "citationVersion"
                            | "referenceShape"
                            | "referenceUnavailable"
                            | "citationExpansionLimit"
                            | "identifierAlignment"
                    )
                })
        })
}

/// Resolve only references in bounded claim evidence arrays. Keep the raw
/// model response elsewhere; this expanded JSON is local validation input.
pub(crate) fn resolve(
    response: &str,
    mut lookup: impl FnMut(&str) -> Result<Citation, &'static str>,
) -> Resolution {
    if response.len() > MAX_GENERATED_RESPONSE_BYTES {
        return invalid("", "citationResponseLimit");
    }
    let Ok(mut value) = crate::strict_json::from_slice(response.as_bytes()) else {
        return invalid("", "citationShape");
    };
    let short = match value.get("citationVersion").and_then(Value::as_str) {
        Some(VERSION) => false,
        Some(SHORT_VERSION) => true,
        _ => return invalid("/citationVersion", "citationVersion"),
    };
    value.as_object_mut().unwrap().remove("citationVersion");
    let Some(sections) = value.get_mut("sections").and_then(Value::as_array_mut) else {
        return invalid("/sections", "citationShape");
    };
    if sections.is_empty() || sections.len() > 12 {
        return invalid("/sections", "citationShape");
    }
    let mut spans = Vec::with_capacity(sections.len());
    let mut issues = vec![];
    for (si, section) in sections.iter_mut().enumerate() {
        let location = format!("/sections/{si}/claims");
        let Some(claims) = section.get_mut("claims").and_then(Value::as_array_mut) else {
            return invalid(&location, "citationShape");
        };
        if claims.is_empty() || claims.len() > 20 {
            return invalid(&location, "citationShape");
        }
        let mut section_spans = Vec::with_capacity(claims.len());
        for (ci, claim) in claims.iter_mut().enumerate() {
            let location = format!("{location}/{ci}/evidence");
            let Some(evidence) = claim.get_mut("evidence").and_then(Value::as_array_mut) else {
                return invalid(&location, "citationShape");
            };
            if evidence.is_empty() || evidence.len() > 8 {
                return invalid(&location, "citationShape");
            }
            let mut claim_spans = Vec::with_capacity(evidence.len());
            for (ei, item) in evidence.iter_mut().enumerate() {
                let location = format!("{location}/{ei}");
                let reference = item
                    .as_object()
                    .filter(|o| o.len() == 1)
                    .and_then(|o| o.get("referenceId"))
                    .and_then(Value::as_str)
                    .filter(|id| reference_shape(id, short));
                let found = reference.ok_or("referenceShape").and_then(&mut lookup);
                match found {
                    Ok(citation) => {
                        *item = json!({"path":citation.range.path,"quote":citation.excerpt});
                        claim_spans.push(citation);
                    }
                    Err(code) if issues.len() < MAX_ISSUES => issues.push(issue(&location, code)),
                    Err(_) => {}
                }
            }
            section_spans.push(claim_spans);
        }
        spans.push(section_spans);
    }
    if !issues.is_empty() {
        return Resolution::Invalid(issues);
    }
    let Ok(text) = serde_json::to_string(&value) else {
        return invalid("", "citationShape");
    };
    if text.len() > MAX_GENERATED_RESPONSE_BYTES {
        return invalid("", "citationExpansionLimit");
    }
    Resolution::Ready(Resolved { text, spans })
}

#[cfg(test)]
mod tests;
