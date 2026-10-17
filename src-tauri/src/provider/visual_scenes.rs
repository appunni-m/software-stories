//! Current chapter requests share the reader's inert composition vocabulary.
//! Saved guides and historical retrieval contracts keep their original policy.

use crate::chapters::{self, ChapterRevision};
use serde_json::{json, Value};

pub(super) const INSTRUCTIONS: &str = r#"VISUAL STORY CONTRACT — ai-html-compositions-v2
Author a causal illustrated answer, with the claims themselves as its complete narration. Usually use 2–5 short claims/scenes per section. Explain what happens to the input, why a guard or choice matters, and the consequence, including failure exceptions. Use ordinary spoken sentences and only the minimum exact identifiers needed for grounding; put call syntax and implementation detail in visual code/source links. Preserve every supported condition and qualifier. The app assembles full prose and speech from all validated claim sentences with inference labels; it never selects a more convenient subset. Keep that complete section within 4,000 UTF-8 bytes or split it. The separate prose/spokenText fields remain required transport fields under their documented bounds; fill them with complete sentences from these same claims.

Teaching policy v2: For a behavior question, open with the problem and a useful input-to-result example, then explain the decisions that make that outcome possible. Do not open with a function signature, argument-type list, or definition of result fields. A responsibility claim should explain the work accomplished, not recite its declaration. Narrate meaning: say that an unavailable record gives no location, or an invalid bound gives an empty range, rather than reading type constructors, angle brackets, field initializer syntax, question marks or call expressions. Keep only necessary bare source identifiers for grounding and explain them in ordinary words. Source links retain the precise implementation. Do not shorten the answer by omitting a guard or alternative.

Choose the representation that teaches the behavior: byte strips for format signatures, before/after values for a transformation, comparisons or tables for alternatives, stacks for state, matrices for raster concepts, and graphs for genuine paths. Do not turn every scene into source-code cards. Show a meaningful example or change when the source supports it. Arrows must reflect the relationship actually described by the claim; different source tokens alone do not establish a transformation. Independent resources, such as two copied tables, belong beside each other in a comparison/grid, not across a conversion arrow. Show a conditional path with its guard; do not present a branch-specific helper as the unconditional result. Invented values must be defensible illustrative examples, introduced as examples in narration, never captured runtime observations.

Make the primary visual show the decision or consequence in readable values, quantities, grouped alternatives or changed state. A copied expression alone is not an explanation of its effect. Unless the reader explicitly asks for syntax, use at most one code element in a scene and put additional implementation detail behind source links. For alternatives, give each outcome its own accurately named node or table row and show the guard. Never caption both a clipped offset and an empty return with the same final-slot label. Introduce illustrative numbers in narration and make their input assumptions clear; never infer an unconditional result from one worked example.

Every visual has kind, title, direction, nodes, edges and presentation. Use kind dataFlow, sequence or hierarchy for the underlying structure, with 2–12 short evidence-linked nodes and at most 20 real edges. Every node/edge cites 1–8 same-section zero-based {claimIndex,evidenceIndex} references; it cannot invent IDs or borrow another section's evidence. An import is not a runtime call. A graph without presentation needs at least one evidenced edge. A composition may have no edges. If no useful visual is supported use kind:none, title:"", direction:auto, nodes:[], edges:[], presentation:null. A plain graph also uses presentation:null.

For a narrated HTML composition use presentation:{schemaVersion:2,scenes:[{claimIndex:0,title:"short visual heading",mode:"composition",elements:[...]}]}. Supply exactly one scene per claim in claim order; a justified graph scene uses mode:graph,elements:[]. The app owns all HTML, styling and interactions. Each element has exactly kind,parentIndex,nodeIndex,tone,value,valueOrigin. Elements form one flat preorder tree: root parentIndex:null, others an earlier container's index. Groups are row,column,grid,panel,compare,stack, with nodeIndex:null,value:"",valueOrigin:none. Content is card,token,code,metric,bytes,matrix,table,text; nodeIndex names the node that supplies its label and captured-source links. That node must cite this scene's own claim, not merely another claim in the section. Tone is neutral,accent,muted,positive,warning. Arrow/divider are decoration with null nodeIndex and no value. Favor 2–4 meaningful large elements, at most 48 elements, six nesting levels, twelve children per group and twelve content elements.

A nonempty value has valueOrigin:source only when it is an exact substring of that node's own same-claim cited excerpts. Do not relabel copied source as an illustration to evade the check. Invented examples use valueOrigin:illustration and receive a visible example badge. Empty values use valueOrigin:none. Values are at most 1,024 UTF-8 bytes. Bytes have 1–32 whitespace-separated cells; matrices have 1–16 equal rows of 1–24 zero/one/dot cells; tables have at most eight newline rows and six equal pipe-separated columns. Repeated visible labels/values on both sides of an arrow do not show a transformation. Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields. A valid schema or source token does not prove an illustrative outcome; check the whole scene against its claim before returning."#;

pub(super) fn nullable_schema() -> Value {
    json!({"anyOf":[crate::scene_presentation::proposal_schema(),{"type":"null"}]})
}

/// Apply only to fresh results, never stored revisions. For a selective repair,
/// the caller supplies its exact section ID so carried sections remain intact.
pub(crate) fn prepare_generated_sections(
    revision: &mut ChapterRevision,
    selected_section: Option<&str>,
) -> Result<(), String> {
    if selected_section.is_some_and(|id| !revision.sections.iter().any(|s| s.section_id == id)) {
        return Err("The selected section is missing. No draft was saved.".into());
    }
    let mut scripts = Vec::new();
    for (index, section) in revision.sections.iter().enumerate() {
        if selected_section.is_some_and(|id| section.section_id != id) {
            continue;
        }
        if section
            .visual
            .as_ref()
            .and_then(|v| v.typed_visual())
            .and_then(|v| v.presentation.as_ref())
            .is_some_and(|p| p.schema_version != 2)
        {
            return Err(
                "New chapter scenes require presentation version 2. No draft was saved.".into(),
            );
        }
        // The app's evidence-limits section is intentionally silent. Generated
        // behavior sections already require nonempty speech before this point.
        if section.spoken_text.trim().is_empty() {
            continue;
        }
        let complete = chapters::overview_claim_sentences(&section.claims).join(" ");
        if !(20..=4_000).contains(&complete.len()) {
            return Err("The complete grounded narration must be 20–4,000 UTF-8 bytes. Split the section without omitting behavior. No draft was saved.".into());
        }
        scripts.push((index, complete));
    }
    // Do not expose even a locally half-prepared revision on a later failure.
    for (index, script) in scripts {
        revision.sections[index].prose = script.clone();
        revision.sections[index].spoken_text = script;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::chapter_generation::{
        self,
        tests::{simple_proposal, Fixture, SIMPLE_PATH},
    };
    use crate::chapters::{ClaimCategory, ValidationReportStatus};
    use std::collections::HashSet;

    fn revision(fixture: &Fixture, proposal: Value) -> ChapterRevision {
        chapter_generation::canonicalize_file_chapter(
            &fixture.manifest.summary.project_id,
            &fixture.manifest,
            Some(&fixture.index),
            SIMPLE_PATH,
            &fixture.sources(&[SIMPLE_PATH]),
            &HashSet::new(),
            &proposal.to_string(),
        )
        .unwrap()
    }

    pub(crate) fn composed_proposal(version: u32) -> Value {
        let mut proposal = serde_json::to_value(simple_proposal()).unwrap();
        proposal["sections"][0]["visual"] = json!({
            "kind":"dataFlow", "title":"Put the name into the reply", "direction":"auto",
            "nodes":[
                {"label":"Greeting template","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}]},
                {"label":"Example reply","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}]}
            ], "edges":[], "presentation":{"schemaVersion":version,"scenes":[{
                "claimIndex":0,"title":"The name becomes part of the greeting","mode":"composition",
                "elements":[
                    {"kind":"row","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
                    {"kind":"code","parentIndex":0,"nodeIndex":0,"tone":"accent","value":"Hello, ${name}","valueOrigin":"source"},
                    {"kind":"arrow","parentIndex":0,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
                    {"kind":"token","parentIndex":0,"nodeIndex":1,"tone":"positive","value":"Hello, Mira","valueOrigin":"illustration"}
                ]
            }]}
        });
        proposal
    }

    #[test]
    fn complete_narration_keeps_all_claims_and_inference_but_silent_limits_stay_silent() {
        let fixture = Fixture::new();
        let mut draft = revision(&fixture, serde_json::to_value(simple_proposal()).unwrap());
        let silent = draft.sections[1].clone();
        let mut inference = draft.sections[0].claims[0].clone();
        inference.claim_id.push_str("-inference");
        for anchor in &mut inference.anchors {
            anchor.anchor_id.push_str("-inference");
        }
        inference.category = ClaimCategory::Inference {
            visible_label: "Inference".into(),
            basis: "The captured function returns only its formatted string.".into(),
        };
        inference.prose =
            "greeting can provide a small display message without changing other state.".into();
        draft.sections[0].claims.push(inference);
        let expected = chapters::overview_claim_sentences(&draft.sections[0].claims).join(" ");
        prepare_generated_sections(&mut draft, None).unwrap();
        assert_eq!(draft.sections[0].spoken_text, expected);
        assert_eq!(draft.sections[0].prose, expected);
        assert!(expected.contains("Inference: greeting"));
        assert_eq!(draft.sections[1], silent);
        let validation = chapters::validate_chapter_revision(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            &draft,
        );
        assert_eq!(
            validation.status,
            ValidationReportStatus::Valid,
            "{validation:?}"
        );
    }

    #[test]
    fn fresh_compositions_use_v2_without_changing_legacy_read_validation() {
        let fixture = Fixture::new();
        let legacy = revision(&fixture, composed_proposal(1));
        let validation = chapters::validate_chapter_revision(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            &legacy,
        );
        assert_eq!(validation.status, ValidationReportStatus::Valid);
        let mut fresh = legacy.clone();
        assert!(prepare_generated_sections(&mut fresh, None)
            .unwrap_err()
            .contains("version 2"));
        assert_eq!(fresh, legacy);
        let mut current = revision(&fixture, composed_proposal(2));
        prepare_generated_sections(&mut current, None).unwrap();
        let validation = chapters::validate_chapter_revision(
            &fixture.storage,
            &fixture.manifest,
            Some(&fixture.index),
            &current,
        );
        assert_eq!(
            validation.status,
            ValidationReportStatus::Valid,
            "{validation:?}"
        );
    }

    #[test]
    fn selective_preparation_preserves_every_carried_legacy_byte() {
        let fixture = Fixture::new();
        let mut draft = revision(&fixture, composed_proposal(1));
        let carried = chapters::encode_chapter_revision(&draft).unwrap();
        let mut selected = draft.sections[0].clone();
        selected.section_id.push_str("-selected");
        selected.section_revision_id.push_str("-selected");
        selected.visual = None;
        selected.prose = "A shorter draft summary before narration assembly.".into();
        let selected_id = selected.section_id.clone();
        let original_sections = draft.sections.clone();
        draft.sections.push(selected);
        prepare_generated_sections(&mut draft, Some(&selected_id)).unwrap();
        assert_eq!(
            &draft.sections[..original_sections.len()],
            &original_sections
        );
        assert_ne!(
            draft.sections.last().unwrap().prose,
            "A shorter draft summary before narration assembly."
        );
        draft.sections.pop();
        assert_eq!(chapters::encode_chapter_revision(&draft).unwrap(), carried);
        assert!(prepare_generated_sections(&mut draft, Some("section:missing")).is_err());
        assert_eq!(chapters::encode_chapter_revision(&draft).unwrap(), carried);
    }

    #[test]
    fn oversized_complete_story_rejects_atomically_in_utf8_bytes_without_omitting_claims() {
        let fixture = Fixture::new();
        let mut draft = revision(&fixture, serde_json::to_value(simple_proposal()).unwrap());
        let mut late = draft.sections[0].clone();
        late.section_id.push_str("-late");
        late.claims[0].prose = format!("greeting {}.", "é".repeat(2_000));
        draft.sections.push(late);
        draft.sections[0].prose = "Keep this original summary until all sections succeed.".into();
        let before = draft.clone();
        assert!(prepare_generated_sections(&mut draft, None)
            .unwrap_err()
            .contains("4,000 UTF-8 bytes"));
        assert_eq!(draft, before);
    }
}
