//! Current chapter requests share the reader's inert composition vocabulary.
//! Stored content uses the same current visual and scene versions.

use crate::chapters::{self, ChapterRevision};
use serde_json::{Value, json};

pub(crate) const TEACHING_POLICY_VERSION: &str = "meaning-first-v26";
pub(crate) const STORY_POLICY_VERSION: &str = "ai-html-compositions-v22";

/// One speech contract for file, overview and selected-section user prompts.
/// Keep the full script and inspectable claim fields complete.
pub(super) const SECTION_NARRATION_INSTRUCTIONS: &str = "Complete scene narration: when a presentation is supplied, spokenText joins every scene narration in claim order, including visible inference labels. It must preserve the full behavior, guards, alternatives, errors and uncertainty of all claims, not a summary or a chosen subset. Narration is a spoken guide to the visible scene, not a compressed list of code details: point to a short label or position, then explain what changes and why it matters. Use at most two short, natural sentences per scene. Explain every distinct outcome in plain words. Keep exact identifiers, signatures, abbreviations, file-format tags, field/table tokens and error labels visible in claims, visual details and source links; in speech describe each by its role (for example, say ‘the first font-format check’ rather than spelling its tag, and ‘the glyph's stored horizontal width’ rather than a table token). Speak an exact token only when the user explicitly asks about its spelling or meaning. If several checks would become a spoken list, split them into ordered scenes without dropping any condition or outcome. With no presentation, write a complete natural spoken account of all claims with the same style, preserving every guard, alternative, error, inference label and uncertainty. Keep the complete transport script within 20–1,000 UTF-8 bytes and the declared section shape; organize focused claims without dropping behavior. The app retains exact claims separately and assembles speech from all scenes.";

const STORY_INTRO: &str = r#"VISUAL STORY CONTRACT — ai-html-compositions-v22
Follow the real input through one consequential, source-grounded transition per scene; connect scenes when evidence tracks the same value or state. Preserve guards, alternatives, failures and uncertainty. Split crowded paths into 2–5 ordered scenes. Narration points to a visible cue, says why the choice matters and what changes next. Begin with the incoming value or situation, not “the function accepts” or “X is Y.” Keep exact tokens visible and clickable, but speak their role unless asked to explain the token. Label examples. Use narration with source links when a visual adds no clarity. Prefix inference narration with its visible label."#;

const BEHAVIOR_TEACHING: &str = r#"Teaching policy v26: Start with the real input or situation; follow its code-backed change, decision and outcome. Each claim covers one transition and its downstream meaning. Narration follows a visible cue in at most two short, natural sentences; sound like a patient guide, not documentation or a list. Avoid “the function accepts/returns” openings. Use grounded consequences or contrasts, such as match versus miss. Keep identifiers, guards and errors visible; speak their role unless the token itself is asked about. State the decision rule before examples and explain every outcome. Split crowded checks without dropping conditions, alternatives or failures. Preserve zero/nonzero and empty-result behavior. Invent no guards, intent or results."#;

const EVIDENCE_MEANING: &str = r#"Trace every earlier guard and failure: lookup and byte-range extraction can fail before a parser runs. Parser fallback starts after those checks. Distinguish missing entries, inaccessible bytes, malformed contents and parsing in claims, visuals and narration. A comment or default constructor does not prove later consumer behavior. For downstream effects, cite that consumer's implementation plus producer and conditions; otherwise leave them unexplored. References and identifiers do not prove meaning."#;

const VISUAL_DESIGN_METHOD: &str = r#"Visual design method v22: choose one subject and question per scene. Follow the real input/state across scenes and change form when behavior changes. Identify the specific relationship proved by the cited code, then choose a source-appropriate shape (route, guarded branch, transform, table, bytes or matrix) that shows it before labels. Do not start from boxes/cards or a graph. Use a project-specific proof point such as the actual guard, changed value, selected destination, fallback, error path or data shape; if the whole scene would still tell the same story after swapping in another project's labels, redesign it. When an operation has distinct success and failure outcomes, draw a visible fork at its result; never place mutually exclusive outcomes as consecutive steps. Give each guard and destination its own short, distinct behavior label; do not repeat the scene title or a generic group label on every item. If a source-ordered selector has more than three checks, split it into consecutive scenes of at most three checks each; name the handoff so the reader knows the next scene continues after a miss, and keep the final fallback visible. Keep labels short and plain; show exact source values only when spelling or bytes matter, using clickable evidence for source navigation. Keep each input, guard, outcome and error visible; pair defaults/fallbacks with outputs; split dense claims. Metrics need actual values or a clear output shape. Group multiple fields in one table, not repeated cards. For bitmap/mask output, show actual pixels as a matrix; if absent, use an explicitly illustrative grid and state real pixels depend on input. Show formats without runtime values as one source-backed output template. If no truthful visual fits, use narration and source links. Use only the typed app renderer; never emit executable markup or resources. Use 2-D unless source behavior is spatial and a tested renderer exists. Invent no execution or measurements."#;

const VISUAL_SPECIFICITY_CHECK: &str = r#"Visual specificity check v22: Before arranging elements, identify what the cited code makes observable: byte positions, a chosen route, a transformed value, a state change, ownership/order, repeated work, parallel work, pixels/geometry, or an output shape. Make the most specific available semantic form the focal point. Do not fill generic panels, cards, rows or grids with labels; use them only when that layout itself explains the behavior. If a visual could be reused in another project by swapping names alone, add one source-backed detail that distinguishes this implementation, or use narration and source links when evidence supports no stronger visual. Prefer one concrete guard, transition or result over a catalog of components; give a new behavior its own scene instead of stacking unrelated diagrams. Keep every supported alternative and failure visible. Represent mutually exclusive success/failure results as a visible fork from the operation result, with each supported destination named; never draw them as consecutive events. Give each guard and destination a distinct short label instead of repeating the scene title. Split ordered selectors longer than three checks into sequential scenes of at most three checks, showing the miss handoff and preserving the final fallback. Do not invent motion, runtime values or state. Reserve 3-D for spatial behavior that needs depth and an app-owned tested renderer; ordinary code and data flow stays 2-D. HTML, styles, interaction and accessibility remain app-owned; output typed scene data, never executable markup or JavaScript."#;

const REPRESENTATION_CHOICES: &str = r#"Choose independently per behavior: execution order/hand-offs → linked path with role labels; ordered probes/fallbacks → numbered selector gates with match destinations and a visible miss hand-off, capped at three checks per scene; one choice → guarded branch with a plain-language condition and named outcomes; mutually exclusive success/failure results → a visible fork at the operation result, not consecutive steps; value-to-route mapping → table; input/result change → transform; changed state → before/after; parallel actors → lanes; nesting → tree; LIFO/FIFO → stack/queue; real bytes, pixels or counts → byte strip, matrix or metric. Graph only source-backed cycles or non-linear links; chains, fan-out and ordered guards do not qualify. Prefer short behavior labels over source expressions. Split dense outcomes without dropping results or failures."#;

const SPOKEN_ROLE_INSTRUCTIONS: &str = r#"Speech is for a listener, not a code reader. Do not speak function, method, type, field or error identifiers, backticks, syntax punctuation, or compact format tags. Say what each does in this scene (for example, “the face's glyph-loading call,” “the missing-bitmap result,” or “the invalid pixel-size error”). Speak an exact token only when the question asks what that token means or how it is spelled. Before returning, replace code names in the spoken sentences with their role; keep exact names visible in source links and collapsed details."#;

const OUTCOME_TEACHING: &str = r#"Show the changed value/state or outcome, not a copied expression. Unless syntax is the lesson, keep source expressions in clickable links and use short behavior labels. Pair every alternative with its actual guard in a branch, selector step or table row. Distinguish clipped offsets from empty returns. State example assumptions; never generalize from one input."#;

const FALLIBLE_TRANSITIONS: &str = r#"Selecting a handler does not mean its work succeeds; a missing preliminary error does not rule out a later failure. With dispatch-only evidence, say that the selected handler is called and its result propagates if the caller does that. Cite callee and conditions for internals or successful construction; do not invent an error or a successful result. Before publishing, compare the claim with its narration, visible labels, node details and values: preserve guards and actual error handling, never promote a call/result to success. No extra provider operation or larger budget is authorized."#;

const VISIBLE_DECISIONS: &str = r#"Show every supported outcome with its guard and destination; keep alternatives visible. Selector checks stay source ordered; its direct children are branch groups, with visible miss/fallback. Split crowded chains into ordered scenes. Use concise behavior labels; examples are illustrative."#;

const COMPOSITION_CONTRACT: &str = r#"Every visual has kind, title, direction, nodes, edges and presentation. The whole section has at most 12 nodes and 20 edges across all claim scenes, not per scene. Budget first: aim for one or two evidence nodes per claim and express relationships through layout/value fields, not a node for every label. Each node/edge cites 1–8 same-section zero-based {claimIndex,evidenceIndex} references; it cannot invent IDs or borrow another section's evidence. Every node also has source:null or source:{evidence:{claimIndex,evidenceIndex},value:"exact source substring"}. A non-null source must repeat one of that node's evidence references, and value must occur exactly once in that verified excerpt; native code resolves the cited anchor and checks the exact substring. Default source to null for readable behavior labels; node evidence remains clickable. Set a source value only when its exact spelling or bytes are part of what the reader must understand. Do not display a source expression just to make a visual clickable. An import is not a runtime call. A graph without presentation needs at least one evidenced edge. A composition may have no edges. If no useful visual is supported use kind:none, title:"", direction:auto, nodes:[], edges:[], presentation:null. A plain graph also uses presentation:null.

For a narrated HTML composition use presentation:{schemaVersion:3,scenes:[{claimIndex:0,title:"short visual heading",narration:"Natural speech explaining this claim and visual, including guards and alternatives.",mode:"composition",elements:[...]}]}. Use one ordered scene per claim. A graph scene uses mode:graph,elements:[]; keep it to eight visible nodes (claim-linked nodes plus one edge of context), or show the full graph when no nodes are claim-linked. Use graphs only for genuinely non-linear relationships that a timeline, branch or selector cannot express clearly. Ordered guard chains, single decisions and causal paths use their matching semantic composition, even when outcomes later converge. Split broad claims when needed; preserve every outcome and the requested map. The app owns HTML, styling and interactions. Elements have exactly kind,parentIndex,nodeIndex,tone,value,valueOrigin in a flat preorder tree: root parentIndex:null, others name an earlier container. Groups are row,column,grid,panel,compare,stack,timeline,lanes,branch,beforeAfter,transform,tree,queue,selector and have null nodeIndex/value. timeline means evidenced execution order; lanes are independent actors without implied order; branch is guard first, then all supported outcomes; selector root directly contains at least two source-ordered branch groups, each with a concise guard followed by supported outcome(s), so the first match wins and a miss visibly continues. Use one or two short labels per outcome. beforeAfter is exactly two labeled states; transform has three direct children in input, operation, result order; tree uses nesting to show source-backed hierarchy; queue shows FIFO order front to back. Use each group only when its meaning fits the source. Content is card,token,code,metric,bytes,matrix,table,text; each nodeIndex supplies its label and source links and cites this scene's claim. Tone is neutral,accent,muted,positive,warning. Arrow/divider are decorative only. Use cards only as concise labels/values inside a semantic visual, not as the visual itself. Per scene, 48 elements and 12 content leaves are hard limits, not targets; keep each scene concise and its relation legible. Choose forms independently by scene semantics; avoid one layout across unrelated behaviors, but do not vary for decoration. One node can suit a focused claim; a complex selector may need a few. Never add nodes to make a scene look busy.

A nonempty value has valueOrigin:source only when it is an exact substring of that node's own same-claim cited excerpts. Do not relabel copied source as an illustration to evade the check. Invented examples use valueOrigin:illustration and receive a visible example badge. Empty values use valueOrigin:none. Values are at most 1,024 UTF-8 bytes. Bytes have 1–32 whitespace-separated cells; matrices have 1–16 equal rows of 1–24 zero/one/dot cells; tables have at most eight newline rows and six equal pipe-separated columns. A transform group has exactly three direct children in input, operation, result order; the app labels and renders those roles. A tree group represents actual nesting. A queue group is FIFO, with children ordered front to back. Repeated visible labels/values on both sides of an arrow do not show a transformation. Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields. A valid schema or source token does not prove an illustrative outcome; check the whole scene against its claim before returning."#;

/// Keep transport-specific fields out of progressive section instructions.
pub(crate) fn teaching_instructions() -> String {
    [
        BEHAVIOR_TEACHING,
        EVIDENCE_MEANING,
        VISUAL_DESIGN_METHOD,
        VISUAL_SPECIFICITY_CHECK,
        REPRESENTATION_CHOICES,
        SPOKEN_ROLE_INSTRUCTIONS,
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
        assert!(super::instructions().contains("ai-html-compositions-v22"));
        assert!(
            super::instructions().contains(
                "whole section has at most 12 nodes and 20 edges across all claim scenes"
            )
        );
        assert!(super::instructions().contains("not per scene"));
        assert!(super::instructions().contains("Choose independently per behavior"));
        assert!(super::instructions().contains("Use cards only as concise labels/values inside a semantic visual, not as the visual itself"));
        assert!(super::instructions().contains("pair defaults/fallbacks with outputs"));
        assert!(
            super::instructions().contains("Metrics need actual values or a clear output shape")
        );
        assert!(
            super::instructions()
                .contains("Group multiple fields in one table, not repeated cards")
        );
        assert!(super::instructions().contains("use an explicitly illustrative grid"));
        assert!(super::instructions().contains("one source-backed output template"));
        assert!(super::instructions().contains("shows it before labels"));
        assert!(
            super::instructions().contains("ordered probes/fallbacks → numbered selector gates")
        );
        assert!(
            super::instructions().contains("chains, fan-out and ordered guards do not qualify")
        );
        assert!(super::instructions().contains("Use 2-D unless source behavior is spatial"));
        assert!(super::instructions().contains("Visual design method v22:"));
        assert!(super::instructions().contains("Visual specificity check v22:"));
        assert!(super::instructions().contains("swapping in another project's labels"));
        assert!(super::instructions().contains("mutually exclusive outcomes as consecutive steps"));
        assert!(super::instructions().contains("at most three checks each"));
        assert!(super::instructions().contains("own short, distinct behavior label"));
        assert!(super::instructions().contains("choose one subject and question per scene"));
        assert!(super::instructions().contains("Do not start from boxes/cards or a graph"));
        assert!(super::instructions().contains("using clickable evidence for source navigation"));
        assert!(
            super::instructions()
                .contains("Use narration with source links when a visual adds no clarity")
        );
        assert!(super::instructions().contains("and a tested renderer exists"));
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
        assert!(BEHAVIOR_TEACHING.contains("State the decision rule before examples"));
        assert!(BEHAVIOR_TEACHING.contains("explain every outcome"));
        assert!(
            BEHAVIOR_TEACHING.contains("speak their role unless the token itself is asked about")
        );
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
        assert!(shared.contains("Choose independently per behavior"));
        assert!(shared.contains("execution order/hand-offs → linked path"));
        assert!(shared.contains("parallel actors → lanes"));
        assert!(shared.contains("one choice → guarded branch"));
        assert!(shared.contains("changed state → before/after"));
        assert!(shared.contains("input/result change → transform"));
        assert!(shared.contains("LIFO/FIFO → stack/queue"));
        assert!(shared.contains("mutually exclusive success/failure results → a visible fork"));
        assert!(shared.contains("capped at three checks per scene"));
        assert!(shared.contains("ordered probes/fallbacks → numbered selector gates"));
        assert!(shared.contains("value-to-route mapping → table"));
        assert!(shared.contains("Graph only source-backed cycles or non-linear links"));
        assert!(shared.contains("chains, fan-out and ordered guards do not qualify"));
        assert!(
            super::instructions().contains(
                "selector root directly contains at least two source-ordered branch groups"
            )
        );
        assert!(
            shared
                .contains("If a visual could be reused in another project by swapping names alone")
        );
        assert!(shared.contains("Use 2-D unless source behavior is spatial"));
        assert!(shared.contains("Use only the typed app renderer; never emit executable markup"));
        assert!(shared.contains("keep alternatives visible"));
        assert!(shared.contains("Selector checks stay source ordered"));
        assert!(shared.contains("Split crowded chains into ordered scenes"));
        assert!(shared.contains("Use concise behavior labels"));
        assert!(super::instructions().contains(
            "Use cards only as concise labels/values inside a semantic visual, not as the visual itself"
        ));
        assert!(shared.contains("Show every supported outcome with its guard and destination"));
        assert!(shared.contains("using clickable evidence for source navigation"));
        assert!(shared.contains("Do not speak function, method, type, field or error identifiers"));
        assert!(shared.contains("its direct children are branch groups"));
        assert!(shared.contains("with visible miss/fallback"));
        assert!(
            super::instructions().contains("Default source to null for readable behavior labels")
        );
        assert!(
            super::instructions()
                .contains("Do not display a source expression just to make a visual clickable")
        );
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
