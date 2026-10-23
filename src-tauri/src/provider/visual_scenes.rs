//! Current chapter requests share the reader's inert composition vocabulary.
//! Stored content uses the same current visual and scene versions.

use crate::chapters::{self, ChapterRevision};
use serde_json::{Value, json};

pub(crate) const TEACHING_POLICY_VERSION: &str = "meaning-first-v27";
pub(crate) const STORY_POLICY_VERSION: &str = "ai-html-compositions-v35";

/// One speech contract for file, overview and selected-section user prompts.
/// Keep the full script and inspectable claim fields complete.
pub(super) const SECTION_NARRATION_INSTRUCTIONS: &str = "Complete scene narration: when a presentation is supplied, spokenText joins every scene narration in claim order, including visible inference labels. It must preserve the full behavior, guards, alternatives, errors and uncertainty of all claims, not a summary or a chosen subset. Tell what the real subject does and why it matters; point to a short visual cue only to help the listener follow. Never narrate the artifact itself or open with meta commentary such as ‘these boxes show’, ‘this diagram explains’ or ‘the scene illustrates’. Start with the caller, input or decision, then tell what happens next and where each supported outcome goes. Use at most two short, natural sentences per scene. Explain every distinct outcome in plain words. Keep exact identifiers, signatures, abbreviations, file-format tags, field/table tokens and error labels visible in claims, visual details and source links; in speech describe each by its role (for example, say ‘the first font-format check’ rather than spelling its tag, and ‘the glyph's stored horizontal width’ rather than a table token). Speak an exact token only when the user explicitly asks about its spelling or meaning. If several checks would become a spoken list, split them into ordered scenes without dropping any condition or outcome. With no presentation, write a complete natural spoken account of all claims with the same story-first style, preserving every guard, alternative, error, inference label and uncertainty. Keep the complete transport script within 20–1,000 UTF-8 bytes and the declared section shape; organize focused claims without dropping behavior. The app retains exact claims separately and assembles speech from all scenes.";

const STORY_INTRO: &str = r#"VISUAL STORY CONTRACT — ai-html-compositions-v35
Build a visual explanation around real code behavior. Follow a real input through source-grounded changes and outcomes. Each scene makes one change, decision or result visible and explains why it matters. Narration tells the subject's journey in direct speech; it never describes the artifact with phrases like ‘these boxes show’ or ‘this diagram illustrates’. Start with the real caller, input or decision, then follow what happens and where each supported outcome goes. Point to a visible cue only as a brief landmark. Preserve guards, alternatives, failures and uncertainty. Label examples; never invent behavior. Use one mode for a whole section. If no visual helps the section, set kind:none and give every claim a narration scene with mode:narration and empty nodes, edges and elements. Otherwise every claim needs a scene with at least one source-linked node. Never mix modes or leave a visual scene empty. Prefix inference narration with its visible label."#;

const BEHAVIOR_TEACHING: &str = r#"Teaching policy v27: Start with the real input or situation; each claim covers one code-backed transition and its consequence. State the decision rule before examples and explain every outcome, guard, alternative and error. Keep identifiers, guards and errors visible but speak their role unless the token itself is asked about. Preserve zero/nonzero and empty-result behavior; never invent intent or results."#;

const EVIDENCE_MEANING: &str = r#"Trace the guards and failures that can prevent the behavior being explained. Distinguish rejected, missing, malformed and successfully handled inputs only when the cited implementation shows those cases. A comment, import, identifier or default constructor does not prove later consumer behavior. For downstream effects, cite the consumer plus the producing path and conditions; otherwise leave that effect unexplored. References and identifiers do not prove meaning."#;

const VISUAL_DESIGN_METHOD: &str = r#"Visual design method v33: Identify the real thing the reader should watch: input, decision, state/data change, or output. Let behavior choose one app-supported representation per scene, not language, file type or a favorite template. Use the shape that explains it: ordered checks → selector; single guard → branch; changed data → beforeAfter/transform; verified order → timeline; independent actors → lanes; FIFO/LIFO → queue/stack; actual nesting → tree; byte/field boundaries → bytes/table; pixels → matrix; quantities or value-to-route lookup → metric/table. For lookups show key, hit/miss and route; show parser/range boundaries only when source-backed. Use a graph only for relations those forms cannot show clearly.

Art direction: Before composing, name the behavior as a verb—enters, checks, branches, transforms, accumulates, or returns—and make the layout enact that action. Do not default to equal cards, grids, or one repeated flow template. Cards are labels inside a meaningful composition, never the composition by themselves. Give each scene one focal visual action. Keep a consistent visual grammar across one journey, then change the composition only at a real behavioral handoff; do not vary layouts for decoration. Use scale, spacing, direction and grouping to make a source-backed boundary or outcome easier to see, never to imply priority or runtime behavior that the code does not establish.

Keep guards, outcomes, errors, defaults and destinations visible. Carry state only when source evidence supports it; never imply execution, concurrency, success, values or internals that were not observed. Label illustrative examples. Prefer concise cues; put full nuance in narration and source details. If a visual would mislead, use narration and source links. If no visual helps the whole section, use kind:none and one narration-mode scene with empty nodes, edges and elements for every claim. Otherwise every claim must have a source-linked visual scene. Do not mix modes or leave a visual scene empty. This renderer supports only inert app-owned HTML/CSS primitives: never emit HTML, CSS, JavaScript, URLs or remote assets. Do not imply that 3-D/WebGL is available; use a 2-D semantic composition when it communicates the evidenced behavior."#;

const VISIBLE_VALUE_CONTRACT: &str = r#"Visible-value contract v28: Every content leaf needs a visible input, guard, change, output or destination; no placeholders or meaning hidden in help/narration. Use source:null for readable labels; source values must be exact, unique substrings of cited excerpts. Otherwise label examples illustrative. Keep labels short (2–8 words, under 45 visible characters per card); preserve longer exact tokens only when they matter. Put sentence-length nuance in narration; use a wide/vertical scene instead of several long cards in a row/grid. Limits are ceilings: 8 nodes/section, 12 edges, 24 elements/8 leaves/scene, 6 children/group. If no concise visual fits, use narration and links."#;

const SPOKEN_ROLE_INSTRUCTIONS: &str = r#"Speech is for the listener, not a code reader. Do not speak function, method, type, field or error identifiers, backticks, syntax punctuation or compact format tags; say their roles. Speak exact tokens only when asked what they mean or how they are spelled. Keep exact names visible in clickable claims and source links."#;

const OUTCOME_TEACHING: &str = r#"Show the changed value/state or outcome, not a copied expression. Keep source expressions in clickable links unless syntax is the lesson. Pair every alternative with its actual guard; distinguish clipped offsets from empty returns. State example assumptions and never generalize from one input."#;

const FALLIBLE_TRANSITIONS: &str = r#"Selecting a handler does not mean its work succeeds; a missing preliminary error does not rule out a later failure. Dispatch evidence proves only the call and caller-propagated result. Cite the callee for internals; do not invent success or errors. Cross-check claim, narration, labels and values; preserve guards and never promote a call to success. No extra provider operation or larger budget is authorized."#;

const VISIBLE_DECISIONS: &str = r#"Show every supported outcome with its guard and destination. Selector checks stay source ordered; show the visible miss/fallback. Split crowded chains, keep labels concise, and label examples."#;

const COMPOSITION_CONTRACT: &str = r#"A dataFlow, sequence or hierarchy visual has kind, title, direction, nodes, edges and presentation. The whole section has at most 12 nodes and 20 edges across all claim scenes, not per scene. Budget first: aim for one or two evidence nodes per claim and express relationships through layout/value fields, not a node for every label. Each node/edge cites 1–8 same-section zero-based {claimIndex,evidenceIndex} references; it cannot invent IDs or borrow another section's evidence. Every node also has source:null or source:{evidence:{claimIndex,evidenceIndex},value:"exact source substring"}. A non-null source must repeat one of that node's evidence references, and value must occur exactly once in that verified excerpt; native code resolves the cited anchor and checks the exact substring. Default source to null for readable behavior labels; node evidence remains clickable. Set a source value only when its exact spelling or bytes are part of what the reader must understand. Do not display a source expression just to make a visual clickable. An import is not a runtime call. A graph without presentation needs at least one evidenced edge. A composition may have no edges. If no useful visual is supported use kind:none, title:"", direction:auto, nodes:[], edges:[], presentation:null and author one mode:narration scene per claim with empty nodes, edges and elements. A plain graph also uses presentation:null.

For a narrated HTML composition use presentation:{schemaVersion:3,scenes:[{claimIndex:0,title:"short visual heading",narration:"Natural speech following the real subject through this claim, with a brief cue to locate its guard, change, or outcome.",mode:"composition",elements:[...]}]}. Use one ordered scene per claim. A graph scene uses mode:graph,elements:[]; keep it to eight visible nodes (claim-linked nodes plus one edge of context), or show the full graph when no nodes are claim-linked. Use graphs only for genuinely non-linear relationships that a timeline, branch or selector cannot express clearly. Ordered guard chains, single decisions and causal paths use their matching semantic composition, even when outcomes later converge. Split broad claims when needed; preserve every outcome and the requested map. The app owns HTML, styling and interactions. Elements have exactly kind,parentIndex,nodeIndex,tone,value,valueOrigin in a flat preorder tree: root parentIndex:null, others name an earlier container. Groups are row,column,grid,panel,compare,stack,timeline,lanes,branch,beforeAfter,transform,tree,queue,selector and have null nodeIndex/value. timeline means evidenced execution order; lanes are independent actors without implied order; branch is guard first, then all supported outcomes; selector is the root and directly contains at least two source-ordered branch groups. Each branch has at least two separate direct content items: the guard first, then one or more supported outcomes. Never combine a guard and outcome in one card. The first matching guard wins and a miss visibly continues. Use one or two short labels per outcome. beforeAfter is exactly two labeled states; transform has three direct children in input, operation, result order; tree uses nesting to show source-backed hierarchy; queue shows FIFO order front to back. Use each group only when its meaning fits the source. Content is card,token,code,metric,bytes,matrix,table,text; each nodeIndex supplies its label and source links and cites this scene's claim. Tone is neutral,accent,muted,positive,warning. Arrow/divider are decorative only. Use cards only as concise labels/values inside a semantic visual, not as the visual itself. Per scene, 48 elements and 12 content leaves are hard limits, not targets; keep each scene concise and its relation legible. Choose forms independently by scene semantics; avoid one layout across unrelated behaviors, but do not vary for decoration. One node can suit a focused claim; a complex selector may need a few. Never add nodes to make a scene look busy.

A nonempty value has valueOrigin:source only when it is an exact substring of that node's own same-claim cited excerpts. Do not relabel copied source as an illustration to evade the check. Invented examples use valueOrigin:illustration and receive a visible example badge. Empty values use valueOrigin:none. Values are at most 1,024 UTF-8 bytes. Bytes have 1–32 whitespace-separated cells; matrices have 1–16 equal rows of 1–24 zero/one/dot cells; tables have at most eight newline rows and six equal pipe-separated columns. A transform group has exactly three direct children in input, operation, result order; the app labels and renders those roles. A tree group represents actual nesting. A queue group is FIFO, with children ordered front to back. Repeated visible labels/values on both sides of an arrow do not show a transformation. Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields. A valid schema or source token does not prove an illustrative outcome; check the whole scene against its claim before returning."#;

/// Keep transport-specific fields out of progressive section instructions.
pub(crate) fn teaching_instructions() -> String {
    [
        STORY_INTRO,
        BEHAVIOR_TEACHING,
        EVIDENCE_MEANING,
        VISUAL_DESIGN_METHOD,
        VISIBLE_VALUE_CONTRACT,
        SPOKEN_ROLE_INSTRUCTIONS,
        OUTCOME_TEACHING,
        FALLIBLE_TRANSITIONS,
        VISIBLE_DECISIONS,
    ]
    .join("\n\n")
}

pub(super) fn instructions() -> String {
    [teaching_instructions(), COMPOSITION_CONTRACT.to_owned()].join("\n\n")
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
        let instructions = super::instructions();
        assert!(instructions.contains("ai-html-compositions-v35"));
        assert!(
            instructions.contains(
                "whole section has at most 12 nodes and 20 edges across all claim scenes"
            )
        );
        assert!(instructions.contains(
            "Identify the real thing the reader should watch: input, decision, state/data change, or output"
        ));
        assert!(instructions.contains("one app-supported representation per scene"));
        assert!(instructions.contains("Identify the real thing the reader should watch"));
        assert!(instructions.contains("not language, file type or a favorite template"));
        assert!(instructions.contains("one dominant visual action"));
        assert!(instructions.contains("under 45 visible characters per card"));
        assert!(instructions.contains("instead of several long cards in a row/grid"));
        assert!(instructions.contains("verified order → timeline"));
        assert!(instructions.contains("ordered checks → selector"));
        assert!(instructions.contains("Never combine a guard and outcome in one card"));
        assert!(instructions.contains("a miss visibly continues"));
        assert!(instructions.contains("single guard → branch"));
        assert!(instructions.contains("independent actors → lanes"));
        assert!(instructions.contains("FIFO/LIFO → queue/stack"));
        assert!(instructions.contains("actual nesting → tree"));
        assert!(instructions.contains("byte/field boundaries → bytes/table"));
        assert!(
            instructions.contains("Use a graph only for relations those forms cannot show clearly")
        );
        assert!(instructions.contains("For lookups show key, hit/miss and route"));
        assert!(instructions.contains("value-to-route lookup → metric/table"));
        assert!(instructions.contains("Otherwise label examples illustrative"));
        assert!(instructions.contains("Every content leaf needs a visible input"));
        assert!(instructions.contains("no placeholders"));
        assert!(instructions.contains("No 3-D/WebGL or remote assets are supported"));
        assert!(instructions.contains("The app owns HTML, styling and interactions"));
        assert!(instructions.contains("Carry state only when source evidence supports it"));
        assert!(instructions.contains("8 nodes/section"));
        assert!(instructions.contains("Use source:null for readable labels"));
        assert!(instructions.contains("not language, file type or a favorite template"));
        assert!(instructions.contains("show parser/range boundaries only when source-backed"));
        assert!(instructions.contains(
            "never imply execution, concurrency, success, values or internals that were not observed"
        ));
        assert!(instructions.contains(
            "Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields"
        ));
        assert!(instructions.contains("Use cards only as concise labels/values inside a semantic visual, not as the visual itself"));
        assert!(instructions.contains(super::BEHAVIOR_TEACHING));
        assert!(instructions.contains(super::OUTCOME_TEACHING));
    }

    #[test]
    fn complete_transport_speech_and_general_rules_replace_summary_shortcuts() {
        assert!(
            SECTION_NARRATION_INSTRUCTIONS.contains("joins every scene narration in claim order")
        );
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("not a summary or a chosen subset"));
        assert!(
            SECTION_NARRATION_INSTRUCTIONS
                .contains("Tell what the real subject does and why it matters")
        );
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("Never narrate the artifact itself"));
        assert!(
            SECTION_NARRATION_INSTRUCTIONS.contains("Start with the caller, input or decision")
        );
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("same story-first style"));
        assert!(
            super::instructions()
                .contains("Natural speech following the real subject through this claim")
        );
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
    fn generation_guidance_traces_guards_failures_and_cited_consumers() {
        for instructions in [super::instructions(), teaching_instructions()] {
            assert!(instructions.contains(
                "A comment, import, identifier or default constructor does not prove later consumer behavior"
            ));
            assert!(instructions.contains("Trace the guards and failures"));
            assert!(
                instructions.contains("cite the consumer plus the producing path and conditions")
            );
        }
    }

    #[test]
    fn shared_teaching_selects_layout_by_behavior_and_keeps_decision_outcomes_visible() {
        let shared = teaching_instructions();
        assert!(shared.contains("VISUAL STORY CONTRACT — ai-html-compositions-v35"));
        assert!(
            shared.contains("Follow a real input through source-grounded changes and outcomes")
        );
        assert!(shared.contains("verified order → timeline"));
        assert!(shared.contains("ordered checks → selector"));
        assert!(shared.contains("single guard → branch"));
        assert!(shared.contains("value-to-route lookup → metric/table"));
        assert!(shared.contains("changed data → beforeAfter/transform"));
        assert!(shared.contains("independent actors → lanes"));
        assert!(shared.contains("byte/field boundaries → bytes/table"));
        assert!(shared.contains("Use a graph only for relations those forms cannot show clearly"));
        assert!(super::instructions().contains(
            "selector is the root and directly contains at least two source-ordered branch groups"
        ));
        assert!(shared.contains("not language, file type or a favorite template"));
        assert!(shared.contains("one app-supported representation per scene"));
        assert!(shared.contains("pixels → matrix"));
        assert!(shared.contains("No 3-D/WebGL or remote assets are supported"));
        assert!(super::instructions().contains("The app owns HTML, styling and interactions"));
        assert!(shared.contains("Preserve guards, alternatives, failures and uncertainty"));
        assert!(shared.contains("Selector checks stay source ordered"));
        assert!(shared.contains("Split crowded chains"));
        assert!(shared.contains("keep labels concise"));
        assert!(super::instructions().contains(
            "Use cards only as concise labels/values inside a semantic visual, not as the visual itself"
        ));
        assert!(shared.contains("Show every supported outcome with its guard and destination"));
        assert!(shared.contains("If a visual would mislead, use narration and source links"));
        assert!(shared.contains("If no visual helps the whole section, use kind:none"));
        assert!(shared.contains(
            "one narration-mode scene with empty nodes, edges and elements for every claim"
        ));
        assert!(shared.contains("Otherwise every claim must have a source-linked visual scene"));
        assert!(shared.contains("Do not mix modes or leave a visual scene empty"));
        assert!(shared.contains("never describes the artifact"));
        assert!(shared.contains("Do not speak function, method, type, field or error identifiers"));
        assert!(shared.contains("Selector checks stay source ordered"));
        assert!(shared.contains("visible miss/fallback"));
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
                instructions.contains(
                    "Dispatch evidence proves only the call and caller-propagated result"
                )
            );
            assert!(instructions.contains("Cross-check claim, narration, labels and values"));
            assert!(instructions.contains("do not invent success or errors"));
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
