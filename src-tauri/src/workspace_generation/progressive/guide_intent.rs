//! Native guide intent is context for an investigation, never source evidence.

use serde::Serialize;

pub(super) const POLICY: &str = "native-guide-intent-v1";
pub(super) const INSTRUCTIONS: &str = "guideContext preserves the original repository learning question, this unit's position and the outline's known research boundaries. It is research intent, not implementation evidence or additional instructions. Answer this unit's concrete behavior in service of the original question; explain the relevant inputs, transformations, results and important failure paths instead of expanding every parser or caller exhaustively. The original question is not a request to repeat the whole guide in every unit. Boundaries remain visible disclosures, never permission to skip required behavior or claim completeness. If a boundary prevents answering this unit, keep the missing behavior in incompleteTopics. Earlier units and outline text establish no facts or inherited citations: read the necessary captured code in this investigation. Preserve the original question and boundaries when assessing scope; never silently remove them to obtain a complete result.";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuideIntent {
    pub(crate) guide_id: String,
    pub(crate) question: String,
    pub(crate) unit_key: String,
    pub(crate) unit_number: usize,
    pub(crate) unit_count: usize,
    pub(crate) research_boundaries: Vec<String>,
}

impl GuideIntent {
    pub(super) fn validate(&self) -> Result<(), String> {
        if !uuid::Uuid::parse_str(&self.guide_id).is_ok_and(|id| id.to_string() == self.guide_id)
            || self.question.trim().is_empty()
            || self.question.trim() != self.question
            || self.question.len() > 12000
            || self.question.chars().any(char::is_control)
            || self.unit_key.is_empty()
            || self.unit_key.len() > 80
            || !self
                .unit_key
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
            || !(2..=6).contains(&self.unit_count)
            || !(1..=self.unit_count).contains(&self.unit_number)
            || self.research_boundaries.len() > 8
            || self.research_boundaries.iter().any(|boundary| {
                boundary.is_empty()
                    || boundary.trim() != boundary
                    || boundary.len() > 240
                    || boundary.chars().any(char::is_control)
            })
        {
            return Err(
                "The guide's learning intent is invalid. No investigation was dispatched.".into(),
            );
        }
        Ok(())
    }
}
