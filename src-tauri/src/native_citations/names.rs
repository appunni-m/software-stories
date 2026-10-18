//! Deterministic metadata from explicitly code-named prose and owned citations.
//! This neither discovers source nor verifies a claim's semantic meaning.

use super::*;

pub(crate) const GROUNDED_POLICY: &str = "explicit-prose-grounded-suffixes-v2";

fn issue(location: String, code: &str) -> Vec<Value> {
    vec![json!({"location":location,"code":code})]
}

/// A concrete name or configuration key, including Unicode and qualified names.
/// Expressions, paths, whitespace, literals and reserved keywords are not names.
fn name(value: &str) -> bool {
    if value.is_empty() || value.len() > 160 || value.contains(':') && !value.contains("::") {
        return false;
    }
    if matches!(
        value,
        "if" | "else"
            | "return"
            | "fn"
            | "let"
            | "pub"
            | "const"
            | "class"
            | "def"
            | "function"
            | "true"
            | "false"
            | "null"
    ) {
        return false;
    }
    value
        .split("::")
        .flat_map(|s| s.split(['.', '-']))
        .all(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .is_some_and(|c| c.is_alphabetic() || matches!(c, '_' | '$'))
                && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '$'))
        })
}

fn candidates(prose: &str) -> Option<Vec<&str>> {
    // Only single inline backticks are name markers. Never mine natural words
    // or interpret fenced code, operators or unmatched delimiters as metadata.
    if prose.len() > 2000 || prose.contains("``") || prose.matches('`').count() % 2 != 0 {
        return None;
    }
    let mut result = Vec::new();
    for item in prose.split('`').skip(1).step_by(2).filter(|s| name(s)) {
        if !result.contains(&item) {
            result.push(item);
        }
    }
    (!result.is_empty()).then_some(result)
}

fn variants(value: &str) -> Vec<&str> {
    let mut result = vec![value];
    for separator in ["::", "."] {
        for (index, _) in value.match_indices(separator) {
            let suffix = &value[index + separator.len()..];
            if name(suffix) && !result.contains(&suffix) {
                result.push(suffix);
            }
        }
    }
    // Prefer the longest literal qualification supported by the fragment.
    result.sort_by_key(|value| std::cmp::Reverse(value.len()));
    result
}

pub(super) fn compile_grounded(resolved: Resolved) -> Result<Resolved, Vec<Value>> {
    let mut value: Value = serde_json::from_str(&resolved.text)
        .map_err(|_| issue(String::new(), "namedIdentifier"))?;
    let sections = value["sections"]
        .as_array_mut()
        .ok_or_else(|| issue("/sections".into(), "namedIdentifier"))?;
    for (si, section) in sections.iter_mut().enumerate() {
        let base = format!("/sections/{si}/claims");
        let claims = section["claims"]
            .as_array_mut()
            .ok_or_else(|| issue(base.clone(), "namedIdentifier"))?;
        for (ci, claim) in claims.iter_mut().enumerate() {
            let base = format!("{base}/{ci}");
            let object = claim
                .as_object_mut()
                .ok_or_else(|| issue(base.clone(), "namedIdentifier"))?;
            if object.contains_key("identifiers") {
                return Err(issue(format!("{base}/identifiers"), "namedIdentifier"));
            }
            let prose = object
                .get("prose")
                .and_then(Value::as_str)
                .ok_or_else(|| issue(format!("{base}/prose"), "namedIdentifier"))?;
            let names = candidates(prose)
                .ok_or_else(|| issue(format!("{base}/prose"), "namedIdentifier"))?;
            let grounded: Vec<_> = names
                .iter()
                .flat_map(|name| variants(name))
                .collect();
            let spans = resolved
                .claim_spans(si, ci)
                .filter(|spans| (1..=8).contains(&spans.len()))
                .ok_or_else(|| issue(format!("{base}/evidence"), "namedIdentifierEvidence"))?;
            let mut selected = Vec::new();
            // One first prose-named identifier per fragment is sufficient for
            // the existing canonical invariant and yields at most eight names.
            for (ei, span) in spans.iter().enumerate() {
                let chosen = grounded
                    .iter()
                    .find(|id| {
                        crate::chapter_generation::contains_identifier(prose, id)
                            && crate::chapter_generation::contains_identifier(&span.excerpt, id)
                    })
                    .ok_or_else(|| {
                        issue(format!("{base}/evidence/{ei}"), "namedIdentifierEvidence")
                    })?;
                if !selected.contains(chosen) {
                    selected.push(*chosen);
                }
            }
            object.insert("identifiers".into(), json!(selected));
        }
    }
    let text =
        serde_json::to_string(&value).map_err(|_| issue(String::new(), "namedIdentifier"))?;
    if text.len() > MAX_GENERATED_RESPONSE_BYTES {
        return Err(issue(String::new(), "namedIdentifier"));
    }
    Ok(Resolved {
        text,
        spans: resolved.spans,
    })
}

#[cfg(test)]
mod tests;
