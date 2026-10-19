//! Current chapter requests share the reader's inert composition vocabulary.
//! Stored content uses the same current visual and scene versions.

use crate::chapters::{self, ChapterRevision};
use serde_json::{json, Value};

pub(crate) const TEACHING_POLICY_VERSION: &str = "meaning-first-v9";

/// One speech contract for file, overview and selected-section user prompts.
/// Keep the full script and inspectable claim fields complete.
pub(super) const SECTION_NARRATION_INSTRUCTIONS: &str = "Complete scene narration: when a presentation is supplied, spokenText joins every scene narration in claim order, including visible inference labels. It must preserve the full behavior, guards, alternatives, errors and uncertainty of all claims, not a summary or a chosen subset. Narration is a spoken guide to the visible scene, not a compressed list of code details: point to a short label or position, then explain what changes and why it matters. Use at most two short, natural sentences per scene. Explain every distinct outcome in plain words, but do not recite identifiers, signatures, format names or error labels unless the user asks for them or the name itself is the behavior. Keep exact names and conditions inspectable in claims, visual labels/details and source links. If several checks would become a spoken list, split them into ordered scenes without dropping any condition or outcome. With no presentation, write a complete natural spoken account of all claims with the same style, preserving every guard, alternative, error, inference label and uncertainty. Keep the complete transport script within 20–1,000 UTF-8 bytes and the declared section shape; organize focused claims without dropping behavior. The app retains exact claims separately and assembles speech from all scenes.";

const STORY_INTRO: &str = r#"VISUAL STORY CONTRACT — ai-html-compositions-v4
Author a causal illustrated answer. Keep exact implementation claims as inspectable source-grounded details; every declared identifier still appears literally in its claim. Write separate natural narration on each scene to explain that claim's meaning, not its spelling. Usually use 2–5 short claims/scenes per section. Speech follows the input, why a choice or guard matters, and the consequence, including all alternative/error conditions. Refer to the short visual labels, introduce useful invented inputs as examples, and connect scenes causally. Do not read declarations, helper names, type constructors, paths, call syntax or punctuation by default. A user explicitly asking about a name or syntax may need that name spoken. Do not change or omit a claim's conditions, exceptions, inference or uncertainty when paraphrasing. The app prefixes inference narration with its visible label. Exact source links remain available. Narration is AI-authored, not automatic code-name removal or proof of semantic equivalence.
The app assembles full details from all validated claims and speech from all ordered scene narrations; it never selects a convenient subset. Keep each complete section's details and speech within 4,000 UTF-8 bytes or split it without dropping behavior. Separate prose/spokenText transport fields remain required under their documented bounds: prose copies complete claim sentences, spokenText copies the complete scene narration. With no visual presentation, prose still copies all complete claim sentences; spokenText is the complete natural account of those claims, with inference labels preserved."#;

const BEHAVIOR_TEACHING: &str = r#"Teaching policy v9: For behavior, open with the problem and a useful input-to-result example. Explain work and consequences, not signatures or field lists. Narration guides the reader through the visible cue: name its short label or position, then explain what changes and why it matters. Use at most two short, natural sentences per scene. Speak for the ear, not as a compressed list of identifiers, signatures, format names, guards or error labels. Explain each distinct outcome in plain words; exact names and conditions remain inspectable in claims, visual labels/details and source links. Say an identifier aloud only when the user asks or the name itself is the behavior. If a scene would need a long spoken list, split the behavior into ordered, supported scenes without dropping any guard, alternative, error or uncertainty. For conditional claims, state the general decision rule first; an example alone cannot replace the rule. Explain actual selectors, including zero versus any nonzero value, and empty results retaining or resetting location. Teach prediction of unshown inputs; never invent a guard."#;

const EVIDENCE_MEANING: &str = r#"Trace every earlier guard and failure: lookup and byte-range extraction can fail before a parser runs. Parser fallback starts after those checks. Distinguish missing entries, inaccessible bytes, malformed contents and parsing in claims, visuals and narration. A comment or default constructor does not prove later consumer behavior. For downstream effects, cite that consumer's implementation plus producer and conditions; otherwise leave them unexplored. References and identifiers do not prove meaning."#;

const REPRESENTATION_CHOICES: &str = r#"Choose the representation that teaches the behavior: bytes for signatures, before/after transformations, comparisons/tables for alternatives, state stacks, raster matrices, graphs for real paths. A single caption or text card is not a visual composition: show a source-supported relationship such as input-to-result, guarded alternatives, or state change, or choose none. A single matrix, byte strip, table or metric may stand alone when that primitive itself explains the claim. Avoid code-card templates: different source tokens alone do not establish a transformation. Independent resources belong in comparisons, not conversion arrows. Show guards; never make a branch-specific helper unconditional. Label defensible invented examples; never claim observed execution."#;

const OUTCOME_TEACHING: &str = r#"Show decisions or consequences as values, quantities, changed state or alternatives; a copied expression is not its effect. Unless syntax is requested, use at most one code element in a scene. For alternatives, give each outcome its own accurately named node or table row and guard. Distinguish clipped offsets from empty returns. Narrate example assumptions; no unconditional conclusion from one input."#;

const FALLIBLE_TRANSITIONS: &str = r#"Selecting a handler does not mean its work succeeds; a missing preliminary error does not rule out a later failure. With dispatch-only evidence, say that the selected handler is called and its result propagates if the caller does that. Cite callee and conditions for internals or successful construction; do not invent an error or a successful result. Before publishing, compare the claim with its narration, visible labels, node details and values: preserve guards and actual error handling, never promote a call/result to success. No extra provider operation or larger budget is authorized."#;

const VISIBLE_DECISIONS: &str = r#"Show choices and fall-through. Narration and collapsed details must not carry a missing branch by themselves. Unlabeled arrows between predicates are not decisions. Split dense scenes without dropping outcomes. Source-backed meaning can be expressed in concise node labels and details with value:"",valueOrigin:"none". Quotes are optional; illustrative values are invented examples, not source facts."#;

const COMPOSITION_CONTRACT: &str = r#"Every visual has kind, title, direction, nodes, edges and presentation. Use kind dataFlow, sequence or hierarchy for the underlying structure, with 2–12 short evidence-linked nodes and at most 20 real edges. Every node/edge cites 1–8 same-section zero-based {claimIndex,evidenceIndex} references; it cannot invent IDs or borrow another section's evidence. An import is not a runtime call. A graph without presentation needs at least one evidenced edge. A composition may have no edges. If no useful visual is supported use kind:none, title:"", direction:auto, nodes:[], edges:[], presentation:null. A plain graph also uses presentation:null.

For a narrated HTML composition use presentation:{schemaVersion:3,scenes:[{claimIndex:0,title:"short visual heading",narration:"Natural speech explaining this claim and visual, including its guards and alternatives.",mode:"composition",elements:[...]}]}. Supply exactly one scene per claim in claim order; a justified graph scene also has narration and uses mode:graph,elements:[]. Keep each focused graph scene to at most eight visible nodes: nodes citing its claim plus one edge of context. A scene with no directly cited nodes shows the full graph. Split an over-broad claim into narrower grounded claims and scenes, or use a meaningful composition for that scene; preserve every outcome and the complete user-requested full map. The app owns all HTML, styling and interactions. Each element has exactly kind,parentIndex,nodeIndex,tone,value,valueOrigin. Elements form one flat preorder tree: root parentIndex:null, others an earlier container's index. Groups are row,column,grid,panel,compare,stack, with nodeIndex:null,value:"",valueOrigin:none. Content is card,token,code,metric,bytes,matrix,table,text; nodeIndex names the node that supplies its label and captured-source links. That node must cite this scene's own claim, not merely another claim in the section. Tone is neutral,accent,muted,positive,warning. Arrow/divider are decoration with null nodeIndex and no value. Favor 2–4 meaningful large elements, at most 48 elements, six nesting levels, twelve children per group and twelve content elements.

A nonempty value has valueOrigin:source only when it is an exact substring of that node's own same-claim cited excerpts. Do not relabel copied source as an illustration to evade the check. Invented examples use valueOrigin:illustration and receive a visible example badge. Empty values use valueOrigin:none. Values are at most 1,024 UTF-8 bytes. Bytes have 1–32 whitespace-separated cells; matrices have 1–16 equal rows of 1–24 zero/one/dot cells; tables have at most eight newline rows and six equal pipe-separated columns. Repeated visible labels/values on both sides of an arrow do not show a transformation. Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields. A valid schema or source token does not prove an illustrative outcome; check the whole scene against its claim before returning."#;

/// Keep transport-specific fields out of progressive section instructions.
pub(crate) fn teaching_instructions() -> String {
    [
        BEHAVIOR_TEACHING,
        EVIDENCE_MEANING,
        REPRESENTATION_CHOICES,
        OUTCOME_TEACHING,
        FALLIBLE_TRANSITIONS,
        VISIBLE_DECISIONS,
    ].join("\n\n")
}

pub(super) fn instructions() -> String {
    [
        STORY_INTRO.to_owned(),
        teaching_instructions(),
        COMPOSITION_CONTRACT.to_owned(),
    ]
    .join("\n\n")
}

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
            .is_some_and(|p| p.schema_version != 3)
        {
            return Err(
                "New chapter scenes require presentation version 3. No draft was saved.".into(),
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
        let narration = chapters::authored_section_narration(section)?.unwrap_or_else(|| section.spoken_text.trim().to_owned());
        if !(20..=4_000).contains(&narration.len()) {
            return Err("The complete authored narration must be 20–4,000 UTF-8 bytes. Split the section without omitting behavior. No draft was saved.".into());
        }
        if section.claims.iter().any(|claim| match &claim.category {
            chapters::ClaimCategory::Inference { visible_label, .. } => !narration.contains(&format!("{}:", visible_label.trim())),
            _ => false,
        }) {
            return Err("Authored narration must retain every visible inference label. No draft was saved.".into());
        }
        scripts.push((index, complete, narration));
    }
    // Do not expose even a locally half-prepared revision on a later failure.
    for (index, details, script) in scripts {
        revision.sections[index].prose = details;
        revision.sections[index].spoken_text = script;
    }
    revision.schema_version = chapters::CHAPTER_SCHEMA_VERSION;
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
        if version == 3 {
            proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["narration"] = json!("For example, a name of Mira becomes Hello, Mira; an empty name still gets the same greeting prefix.");
        }
        proposal
    }

    #[test]
    fn ordinary_prompt_binds_the_versioned_scene_narration_policy() {
        assert_eq!(
            crate::repository_context::digest(super::instructions().as_bytes()),
            "9768b33c423cf2a4073d198f82bcfec2e9034154642eb7bb1a535c6d1772f5fb"
        );
        assert!(super::instructions().contains(super::BEHAVIOR_TEACHING));
        assert!(super::instructions().contains(super::OUTCOME_TEACHING));
    }

    #[test]
    fn complete_transport_speech_and_general_rules_replace_summary_shortcuts() {
        assert!(
            SECTION_NARRATION_INSTRUCTIONS.contains("joins every scene narration in claim order")
        );
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("not a summary or a chosen subset"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("spoken guide to the visible scene"));
        assert!(
            SECTION_NARRATION_INSTRUCTIONS
                .contains("at most two short, natural sentences per scene")
        );
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("split them into ordered scenes"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("20–1,000 UTF-8 bytes"));
        assert!(BEHAVIOR_TEACHING.contains("state the general decision rule"));
        assert!(BEHAVIOR_TEACHING.contains("an example alone cannot replace the rule"));
        assert!(teaching_instructions().contains(BEHAVIOR_TEACHING));
    }

    #[test]
    fn all_generation_routes_distinguish_preconditions_parse_fallbacks_and_consumer_facts() {
        for instructions in [super::instructions(), teaching_instructions()] {
            assert!(instructions.contains("A comment or default constructor does not prove later consumer behavior"));
            assert!(instructions.contains("lookup and byte-range extraction can fail before a parser runs"));
            assert!(instructions.contains("cite that consumer's implementation"));
            assert!(instructions.contains("every earlier guard and failure"));
        }
    }

    #[test]
    fn shared_teaching_keeps_representation_choices_without_ordinary_transport_fields() {
        let shared = teaching_instructions();
        assert!(shared.contains(REPRESENTATION_CHOICES));
        assert!(shared.contains("different source tokens alone do not establish a transformation"));
        assert!(shared.contains("Independent resources"));
        assert!(shared.contains("A single caption or text card is not a visual composition"));
        assert!(shared.contains("A single matrix, byte strip, table or metric may stand alone"));
        assert!(shared.contains("Narration and collapsed details must not carry a missing branch by themselves"));
        assert!(shared.contains("Source-backed meaning can be expressed in concise node labels and details"));
        assert!(!shared.contains("parentIndex"));
        assert!(!shared.contains("spokenText copies"));
        assert!(!shared.contains("20–1,000"));
    }

    #[test]
    fn fresh_teaching_preserves_fallible_calls_in_narration_and_visible_outcomes() {
        for instructions in [super::instructions(), teaching_instructions()] {
            assert!(instructions.contains("Selecting a handler does not mean its work succeeds"));
            assert!(instructions.contains("a missing preliminary error does not rule out a later failure"));
            assert!(instructions.contains("say that the selected handler is called and its result propagates"));
            assert!(instructions.contains("compare the claim with its narration, visible labels, node details and values"));
            assert!(instructions.contains("do not invent an error or a successful result"));
            assert!(instructions.contains("No extra provider operation or larger budget"));
        }
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
        let authored = "A name becomes a friendly greeting. Inference: that message could be used for display; the captured function alone does not show where it is used.";
        draft.sections[0].spoken_text = authored.into();
        prepare_generated_sections(&mut draft, None).unwrap();
        assert_eq!(draft.sections[0].spoken_text, authored);
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
    fn invalid_standalone_narration_never_leaves_a_half_prepared_revision() {
        let fixture = Fixture::new();
        for speech in ["too short".to_owned(), "é".repeat(2_001)] {
            let mut draft = revision(&fixture, serde_json::to_value(simple_proposal()).unwrap());
            draft.sections[0].visual = None;
            draft.sections[0].spoken_text = speech;
            let original = draft.clone();
            assert!(prepare_generated_sections(&mut draft, None).is_err());
            assert_eq!(draft, original);
        }
    }

    #[test]
    fn current_compositions_use_v3_and_preserve_authored_narration() {
        let fixture = Fixture::new();
        let mut current = revision(&fixture, composed_proposal(3));
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
        assert!(current.sections[0].prose.contains("greeting"));
        assert!(!current.sections[0].spoken_text.contains("greeting("));
        assert!(current.sections[0].spoken_text.starts_with("For example"));
        assert_ne!(current.sections[0].prose, current.sections[0].spoken_text);
        let before = current.clone();
        let mut value = serde_json::to_value(&current).unwrap();
        value["sections"][0]["visual"]["presentation"]["scenes"][0]["narration"] = json!("");
        let mut invalid: ChapterRevision = serde_json::from_value(value).unwrap();
        let untouched = invalid.clone();
        assert!(prepare_generated_sections(&mut invalid, None).is_err());
        assert_eq!(invalid, untouched);
        assert_ne!(invalid, before);
    }

    #[test]
    fn selective_preparation_preserves_every_carried_section_byte() {
        let fixture = Fixture::new();
        let mut draft = revision(&fixture, composed_proposal(3));
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
