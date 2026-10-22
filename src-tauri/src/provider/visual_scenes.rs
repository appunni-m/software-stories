//! Current chapter requests share the reader's inert composition vocabulary.
//! Stored content uses the same current visual and scene versions.

use crate::chapters::{self, ChapterRevision};
use serde_json::{Value, json};

pub(crate) const TEACHING_POLICY_VERSION: &str = "meaning-first-v22";

/// One speech contract for file, overview and selected-section user prompts.
/// Keep the full script and inspectable claim fields complete.
pub(super) const SECTION_NARRATION_INSTRUCTIONS: &str = "Complete scene narration: when a presentation is supplied, spokenText joins every scene narration in claim order, including visible inference labels. It must preserve the full behavior, guards, alternatives, errors and uncertainty of all claims, not a summary or a chosen subset. Narration is a spoken guide to the visible scene, not a compressed list of code details: point to a short label or position, then explain what changes and why it matters. Use at most two short, natural sentences per scene. Explain every distinct outcome in plain words. Keep exact identifiers, signatures, abbreviations, file-format tags, field/table tokens and error labels visible in claims, visual details and source links; in speech describe each by its role (for example, say ‘the first font-format check’ rather than spelling its tag, and ‘the glyph's stored horizontal width’ rather than a table token). Speak an exact token only when the user explicitly asks about its spelling or meaning. If several checks would become a spoken list, split them into ordered scenes without dropping any condition or outcome. With no presentation, write a complete natural spoken account of all claims with the same style, preserving every guard, alternative, error, inference label and uncertainty. Keep the complete transport script within 20–1,000 UTF-8 bytes and the declared section shape; organize focused claims without dropping behavior. The app retains exact claims separately and assembles speech from all scenes.";

const STORY_INTRO: &str = r#"VISUAL STORY CONTRACT — ai-html-compositions-v14
Teach with a causal visual story, not a glossary. Each claim/scene shows one consequential, source-grounded behavior; connect scenes when evidence connects them. Preserve conditions, alternatives, failures and uncertainty. Split crowded behavior into ordered scenes; usually use 2–5 when that covers the requested path. Claims retain exact identifiers and complete inspectable details. Narration points to the visual cue, explains why the choice matters and what its result changes. Keep speech natural; exact tokens stay visible and clickable, and are spoken only when asked. Distinguish source values from labeled examples. Use narration with source links when a visual adds no clarity. The app prefixes inference narration with its visible label."#;

const BEHAVIOR_TEACHING: &str = r#"Teaching policy v22: Start with the problem and a useful supported example. Each claim covers one transition: input, decision/change, result and downstream meaning. Narration follows a visible cue in at most two short, natural sentences per scene; sound like a guide, not a glossary or list. Keep identifiers, format/field tokens, guards and errors in claims, labels and source links; in speech describe their role (say “the glyph's stored width,” not a field token). Speak a token only when asked. State the general decision rule before an example; an example alone cannot replace the rule. Explain every outcome. Split crowded checks into ordered claims/scenes without dropping guards, alternatives, errors or uncertainty. Preserve zero/nonzero and empty-result location behavior. Do not invent guards."#;

const EVIDENCE_MEANING: &str = r#"Trace every earlier guard and failure: lookup and byte-range extraction can fail before a parser runs. Parser fallback starts after those checks. Distinguish missing entries, inaccessible bytes, malformed contents and parsing in claims, visuals and narration. A comment or default constructor does not prove later consumer behavior. For downstream effects, cite that consumer's implementation plus producer and conditions; otherwise leave them unexplored. References and identifiers do not prove meaning."#;

const VISUAL_DESIGN_METHOD: &str = r#"Visual design method: identify one focal subject and visual question (route, choice, change, state, shape, parallel work or relationship). Make its source-supported input, decision/change and outcome visible in a sparse composition using its natural data form. Track the same subject across scenes, changing form when behavior changes. Use the app's typed renderer only. Avoid generic card grids and box rows; cards only label. Never output HTML, JavaScript, CSS, imports or external resources. Use 2-D by default; 3-D only for source-grounded spatial behavior with an existing tested app-owned renderer. Never invent execution or measurements."#;

const REPRESENTATION_CHOICES: &str = r#"Choose per behavior: actual order/calls/loops → timeline; prioritized probes/fallbacks → selector; one guarded choice → branch; discrete value-to-route mapping → table; real value change → transform; changed state → beforeAfter; independent actors → lanes; nesting → tree; LIFO/FIFO → stack/queue; byte structure → bytes; pixel/grid data → matrix; meaningful count → metric. Use graph only for source-backed cycles or genuinely non-linear links; chains, fan-out and ordered guards are not enough. Split crowded outcomes into ordered scenes, preserving each result and failure."#;

const OUTCOME_TEACHING: &str = r#"Show the changed value/state or outcome, not a copied expression. Unless syntax is the lesson, keep source expressions in clickable links and use short behavior labels. Pair every alternative with its actual guard in a branch, selector step or table row. Distinguish clipped offsets from empty returns. State example assumptions; never generalize from one input."#;

const FALLIBLE_TRANSITIONS: &str = r#"Selecting a handler does not mean its work succeeds; a missing preliminary error does not rule out a later failure. With dispatch-only evidence, say that the selected handler is called and its result propagates if the caller does that. Cite callee and conditions for internals or successful construction; do not invent an error or a successful result. Before publishing, compare the claim with its narration, visible labels, node details and values: preserve guards and actual error handling, never promote a call/result to success. No extra provider operation or larger budget is authorized."#;

const VISIBLE_DECISIONS: &str = r#"Show every supported outcome with its guard and destination; keep alternatives in the visual, not speech. Selector checks stay source ordered with explicit misses and fallback; split crowded chains into ordered scenes. Use concise source-backed labels; examples are illustrative."#;

const COMPOSITION_CONTRACT: &str = r#"Every visual has kind, title, direction, nodes, edges and presentation. The whole section has at most 12 nodes and 20 edges across all claim scenes, not per scene. Budget first: aim for one or two evidence nodes per claim and express relationships through layout/value fields, not a node for every label. Each node/edge cites 1–8 same-section zero-based {claimIndex,evidenceIndex} references; it cannot invent IDs or borrow another section's evidence. Every node also has source:null or source:{evidence:{claimIndex,evidenceIndex},value:"exact source substring"}. A non-null source must repeat one of that node's evidence references, and value must occur exactly once in that verified excerpt; native code resolves the cited anchor and checks the exact substring. Use this for a concise behavior label whose click opens one precise code expression. Leave it null when no unique exact substring supports that step. An import is not a runtime call. A graph without presentation needs at least one evidenced edge. A composition may have no edges. If no useful visual is supported use kind:none, title:"", direction:auto, nodes:[], edges:[], presentation:null. A plain graph also uses presentation:null.

For a narrated HTML composition use presentation:{schemaVersion:3,scenes:[{claimIndex:0,title:"short visual heading",narration:"Natural speech explaining this claim and visual, including guards and alternatives.",mode:"composition",elements:[...]}]}. Use one ordered scene per claim. A graph scene uses mode:graph,elements:[]; keep it to eight visible nodes (claim-linked nodes plus one edge of context), or show the full graph when no nodes are claim-linked. Use graphs only for genuinely non-linear relationships that a timeline, branch or selector cannot express clearly. Ordered guard chains, single decisions and causal paths use their matching semantic composition, even when outcomes later converge. Split broad claims when needed; preserve every outcome and the requested map. The app owns HTML, styling and interactions. Elements have exactly kind,parentIndex,nodeIndex,tone,value,valueOrigin in a flat preorder tree: root parentIndex:null, others name an earlier container. Groups are row,column,grid,panel,compare,stack,timeline,lanes,branch,beforeAfter,transform,tree,queue,selector and have null nodeIndex/value. timeline means evidenced chronological order; lanes are independent actors without implied order; branch is guard first, then all supported outcomes; selector contains at least two branch groups in source order, each with a guard followed by its supported outcome(s), so the first match wins and a miss visibly continues; beforeAfter is exactly two labeled states; transform has three direct children in input, operation, result order; tree uses nesting to show source-backed hierarchy; queue shows FIFO order front to back. Use each group only when its meaning fits the source. Content is card,token,code,metric,bytes,matrix,table,text; each nodeIndex supplies its label and source links and cites this scene's claim. Tone is neutral,accent,muted,positive,warning. Arrow/divider are decorative only. Use cards only as concise labels/values inside a semantic visual, not as the visual itself. Per scene, 48 elements and 12 content leaves are hard limits, not targets; keep each scene concise and its relation legible. Choose forms independently by scene semantics; avoid one layout across unrelated behaviors, but do not vary for decoration. One node can suit a focused claim; a complex selector may need a few. Never add nodes to make a scene look busy.

A nonempty value has valueOrigin:source only when it is an exact substring of that node's own same-claim cited excerpts. Do not relabel copied source as an illustration to evade the check. Invented examples use valueOrigin:illustration and receive a visible example badge. Empty values use valueOrigin:none. Values are at most 1,024 UTF-8 bytes. Bytes have 1–32 whitespace-separated cells; matrices have 1–16 equal rows of 1–24 zero/one/dot cells; tables have at most eight newline rows and six equal pipe-separated columns. A transform group has exactly three direct children in input, operation, result order; the app labels and renders those roles. A tree group represents actual nesting. A queue group is FIFO, with children ordered front to back. Repeated visible labels/values on both sides of an arrow do not show a transformation. Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields. A valid schema or source token does not prove an illustrative outcome; check the whole scene against its claim before returning."#;

/// Keep transport-specific fields out of progressive section instructions.
pub(crate) fn teaching_instructions() -> String {
    [
        BEHAVIOR_TEACHING,
        EVIDENCE_MEANING,
        VISUAL_DESIGN_METHOD,
        REPRESENTATION_CHOICES,
        OUTCOME_TEACHING,
        FALLIBLE_TRANSITIONS,
        VISIBLE_DECISIONS,
    ]
    .join("\n\n")
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
        let narration = chapters::authored_section_narration(section)?
            .unwrap_or_else(|| section.spoken_text.trim().to_owned());
        if !(20..=4_000).contains(&narration.len()) {
            return Err("The complete authored narration must be 20–4,000 UTF-8 bytes. Split the section without omitting behavior. No draft was saved.".into());
        }
        if section.claims.iter().any(|claim| match &claim.category {
            chapters::ClaimCategory::Inference { visible_label, .. } => {
                !narration.contains(&format!("{}:", visible_label.trim()))
            }
            _ => false,
        }) {
            return Err(
                "Authored narration must retain every visible inference label. No draft was saved."
                    .into(),
            );
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
        tests::{Fixture, SIMPLE_PATH, simple_proposal},
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
                {"label":"Greeting template","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null},
                {"label":"Example reply","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null}
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
            proposal["sections"][0]["visual"]["presentation"]["scenes"][0]["narration"] = json!(
                "For example, a name of Mira becomes Hello, Mira; an empty name still gets the same greeting prefix."
            );
        }
        proposal
    }

    #[test]
    fn ordinary_prompt_binds_the_versioned_scene_narration_policy() {
        assert!(super::instructions().contains("ai-html-compositions-v14"));
        assert!(
            super::instructions().contains(
                "whole section has at most 12 nodes and 20 edges across all claim scenes"
            )
        );
        assert!(super::instructions().contains("not per scene"));
        assert!(super::instructions().contains("Choose per behavior:"));
        assert!(super::instructions().contains("Use cards only as concise labels/values inside a semantic visual, not as the visual itself"));
        assert!(super::instructions().contains("prioritized probes/fallbacks → selector"));
        assert!(
            super::instructions().contains("chains, fan-out and ordered guards are not enough")
        );
        assert!(super::instructions().contains("Use 2-D by default"));
        assert!(super::instructions().contains("Visual design method:"));
        assert!(super::instructions().contains("one focal subject"));
        assert!(super::instructions().contains("Avoid generic card grids and box rows"));
        assert!(
            super::instructions()
                .contains("Use narration with source links when a visual adds no clarity")
        );
        assert!(super::instructions().contains("an existing tested app-owned renderer"));
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
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("file-format tags, field/table tokens"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("the first font-format check"));
        assert!(
            SECTION_NARRATION_INSTRUCTIONS
                .contains("explicitly asks about its spelling or meaning")
        );
        assert!(
            SECTION_NARRATION_INSTRUCTIONS
                .contains("at most two short, natural sentences per scene")
        );
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("split them into ordered scenes"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("20–1,000 UTF-8 bytes"));
        assert!(BEHAVIOR_TEACHING.contains("State the general decision rule before an example"));
        assert!(BEHAVIOR_TEACHING.contains("an example alone cannot replace the rule"));
        assert!(BEHAVIOR_TEACHING.contains("the glyph's stored width"));
        assert!(teaching_instructions().contains(BEHAVIOR_TEACHING));
    }

    #[test]
    fn all_generation_routes_distinguish_preconditions_parse_fallbacks_and_consumer_facts() {
        for instructions in [super::instructions(), teaching_instructions()] {
            assert!(instructions.contains(
                "A comment or default constructor does not prove later consumer behavior"
            ));
            assert!(
                instructions
                    .contains("lookup and byte-range extraction can fail before a parser runs")
            );
            assert!(instructions.contains("cite that consumer's implementation"));
            assert!(instructions.contains("every earlier guard and failure"));
        }
    }

    #[test]
    fn shared_teaching_selects_layout_by_behavior_and_keeps_decision_outcomes_visible() {
        let shared = teaching_instructions();
        assert!(shared.contains(REPRESENTATION_CHOICES));
        assert!(shared.contains("Choose per behavior:"));
        assert!(shared.contains("actual order/calls/loops → timeline"));
        assert!(shared.contains("independent actors → lanes"));
        assert!(shared.contains("one guarded choice → branch"));
        assert!(shared.contains("changed state → beforeAfter"));
        assert!(shared.contains("real value change → transform"));
        assert!(shared.contains("LIFO/FIFO → stack/queue"));
        assert!(shared.contains("prioritized probes/fallbacks → selector"));
        assert!(shared.contains("discrete value-to-route mapping → table"));
        assert!(shared.contains("Use graph only for source-backed cycles"));
        assert!(shared.contains("chains, fan-out and ordered guards are not enough"));
        assert!(
            super::instructions()
                .contains("selector contains at least two branch groups in source order")
        );
        assert!(shared.contains("cards only label"));
        assert!(shared.contains("Use 2-D by default"));
        assert!(shared.contains("Use the app's typed renderer only"));
        assert!(shared.contains("keep alternatives in the visual, not speech"));
        assert!(
            shared
                .contains("Selector checks stay source ordered with explicit misses and fallback")
        );
        assert!(shared.contains("split crowded chains into ordered scenes"));
        assert!(shared.contains("Use concise source-backed labels"));
        assert!(!shared.contains("parentIndex"));
        assert!(!shared.contains("spokenText copies"));
        assert!(!shared.contains("20–1,000"));
    }

    #[test]
    fn fresh_teaching_preserves_fallible_calls_in_narration_and_visible_outcomes() {
        for instructions in [super::instructions(), teaching_instructions()] {
            assert!(instructions.contains("Selecting a handler does not mean its work succeeds"));
            assert!(
                instructions
                    .contains("a missing preliminary error does not rule out a later failure")
            );
            assert!(
                instructions
                    .contains("say that the selected handler is called and its result propagates")
            );
            assert!(instructions.contains(
                "compare the claim with its narration, visible labels, node details and values"
            ));
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
        assert!(
            prepare_generated_sections(&mut draft, None)
                .unwrap_err()
                .contains("4,000 UTF-8 bytes")
        );
        assert_eq!(draft, before);
    }
}
