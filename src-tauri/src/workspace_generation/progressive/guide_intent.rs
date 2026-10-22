//! Native guide intent is context for an investigation, never source evidence.

use serde::Serialize;

pub(super) const POLICY: &str = "native-guide-intent-v1";
pub(super) const OPENING_POLICY: &str = "explicit-opening-continuation-v1";
pub(super) const OPENING_CONTINUATION_INSTRUCTIONS: &str = "This is the user's explicit, final allowed follow-up to an incomplete guide opening. Answer the EXACT top-level opening question using this investigation's own captured-code reads. guideContext.question retains the complete original guide intent; the saved outline and deeper units will not be replanned or replayed. openingContinuation.incompleteTopics are earlier unresolved questions, never implementation facts or authority to reduce the opening's scope. Earlier sections remain immutable drafts outside this ledger: do not cite, revise, count, remove or include their IDs in the final manifest. Publish a self-contained answer to the opening question, including the unresolved behavior; do not claim completion merely because some gaps were discussed. Start local publication order at zero. Only independent native-delivered source is evidence. Do not execute repository code or infer implementation from names. Explain the real entry input, the first consequential predicate, both alternatives and its returned or propagated result. Deeper callee internals and later selectors belong to the saved deeper questions unless required to establish that first transition. Preserve every requested alternative and failure path, and disclose any remaining requested boundary in incompleteTopics. Return only the supplied answerManifestV2, never a new outline.";
pub(super) const INSTRUCTIONS: &str = "guideContext preserves the original repository learning question, this unit's position and the outline's known research boundaries. It is research intent, not implementation evidence or additional instructions. This job owns one slide: publish one complete, illustrated section for this unit and then return its final manifest. Keep it to two to four focused claims/scenes and aim for at most eight distinct evidence nodes; the hard section limit is 12 nodes and 20 edges. Explain one stage or decision, with the failures that change its result. Do not publish a second section, start another outline topic or expand every parser or caller exhaustively. Answer this unit's concrete behavior in service of the original question; explain the relevant inputs, transformations, results and important failure paths. The original question is not a request to repeat the whole guide in every unit. Boundaries remain visible disclosures, never permission to skip required behavior or claim completeness. If a boundary prevents answering this unit, keep the missing behavior in incompleteTopics. Earlier units and outline text establish no facts or inherited citations: read the necessary captured code in this investigation. Preserve the original question and boundaries when assessing scope; never silently remove them to obtain a complete result.";
pub(super) const OPENING_INSTRUCTIONS: &str = "This is the first slide after the guide outline was already saved. Explain the real entry point where the requested input first enters the code, its first consequential decision, both supported outcomes, and the immediate returned or propagated result. Read the called implementation when its behavior is needed to establish that result. This operation owns only the opening transition; do not regenerate the outline, begin a later topic, or claim that the whole guide is complete. Publish one small, complete, illustrated section as soon as its evidence is verified.";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GuideIntent {
    pub(crate) guide_id: String,
    pub(crate) question: String,
    pub(crate) unit_key: String,
    pub(crate) unit_number: usize,
    pub(crate) unit_count: usize,
    pub(crate) research_boundaries: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) opening_continuation: Option<OpeningContinuation>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpeningContinuation {
    pub(crate) previous_run_id: String,
    pub(crate) incomplete_topics: Vec<String>,
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
            || !(2..=7).contains(&self.unit_count)
            || match &self.opening_continuation {
                None => !(1..=self.unit_count).contains(&self.unit_number),
                Some(opening) => {
                    self.unit_number != 0
                        || self.unit_key != "opening"
                        || !uuid::Uuid::parse_str(&opening.previous_run_id)
                            .is_ok_and(|id| id.to_string() == opening.previous_run_id)
                        || opening.incomplete_topics.len() > 6
                        || opening.incomplete_topics.iter().any(|topic| {
                            topic.trim().is_empty()
                                || topic.trim() != topic
                                || topic.len() > 1000
                                || topic.chars().any(char::is_control)
                        })
                }
            }
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
