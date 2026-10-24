//! Current chapter requests share the reader's inert composition vocabulary.
//! Stored content uses the same current visual and scene versions.

use crate::chapters::{self, ChapterRevision};
use serde_json::{Value, json};

pub(crate) const TEACHING_POLICY_VERSION: &str = "meaning-first-v28";
pub(crate) const STORY_POLICY_VERSION: &str = "ai-html-compositions-v44";

/// One speech contract for file, overview and selected-section user prompts.
/// Keep the full script and inspectable claim fields complete.
pub(super) const SECTION_NARRATION_INSTRUCTIONS: &str = "Complete scene narration: when a presentation is supplied, spokenText joins every scene narration in claim order, including visible inference labels. It must preserve the full behavior, guards, alternatives, errors and uncertainty of all claims, not a summary or a chosen subset. Tell what the real subject does and why it matters; point to a short visual cue only to help the listener follow. Never narrate the artifact itself or open with meta commentary such as ‘these boxes show’, ‘this diagram explains’ or ‘the scene illustrates’. Start with the caller, input or decision, then tell what happens next and where each supported outcome goes. Use at most two short, natural sentences per scene. Explain every distinct outcome in plain words. Keep exact identifiers, signatures, abbreviations, file-format tags, field/table tokens and error labels visible in claims, visual details and source links; in speech describe each by its role (for example, say ‘the first font-format check’ rather than spelling its tag, and ‘the glyph's stored horizontal width’ rather than a table token). Do not pronounce all-capital abbreviations or format tags letter by letter; replace them with their job in the behavior. Speak an exact token only when the user explicitly asks about its spelling or meaning. If several checks would become a spoken list, split them into ordered scenes without dropping any condition or outcome. With no presentation, write a complete natural spoken account of all claims with the same story-first style, preserving every guard, alternative, error, inference label and uncertainty. Keep the complete transport script within 20–1,000 UTF-8 bytes and the declared section shape; organize focused claims without dropping behavior. The app retains exact claims separately and assembles speech from all scenes.";

const STORY_INTRO: &str = r#"VISUAL STORY CONTRACT — ai-html-compositions-v44
Build a visual explanation around real code behavior. Follow a real input through source-grounded changes and outcomes. Each scene makes one change, decision or result visible and explains why it matters. Narration tells the subject's journey in direct speech; it never describes the artifact with phrases like ‘these boxes show’ or ‘this diagram illustrates’. Start with the real caller, input or decision, then follow what happens and where each supported outcome goes. Point to a visible cue only as a brief landmark. Preserve guards, alternatives, failures and uncertainty. Label examples; never invent behavior. Use one mode for a whole section. If no visual helps the section, set kind:none and give every claim a narration scene with mode:narration and empty nodes, edges and elements. Otherwise every claim needs a scene with at least one source-linked node. Never mix modes or leave a visual scene empty. Prefix inference narration with its visible label."#;

const BEHAVIOR_TEACHING: &str = r#"Teaching policy v28: Start with the real input or situation; each claim covers one code-backed transition and its consequence. State the decision rule before examples and explain every outcome, guard, alternative and error. Keep identifiers, guards and errors visible but speak their role unless the token itself is asked about. Preserve zero/nonzero and empty-result behavior; never invent intent or results."#;

const EVIDENCE_MEANING: &str = r#"Trace the guards and failures that can prevent the behavior being explained. Distinguish rejected, missing, malformed and successfully handled inputs only when the cited implementation shows those cases. A comment, import, identifier or default constructor does not prove later consumer behavior. For downstream effects, cite the consumer plus the producing path and conditions; otherwise leave that effect unexplored. References and identifiers do not prove meaning."#;

const VISUAL_DESIGN_METHOD: &str = r#"Visual design method v44: Let observed behavior choose one app-supported composition per scene, not file type or a favorite template. A single guard with its success and failure outcomes uses branch. Independent validation guards in one function are separate claim-scenes; each scene shows that guard's matching failure and explains that a miss lets the caller continue. Use selector only for competing ordered candidates (format signatures, handlers or priority routes) where the first match wins and a miss tries the next candidate. The claim or narration must explicitly describe that candidate handoff; otherwise the selector visually invents fall-through. Each selector branch has exactly one guard and the result for a match; the app already renders the miss-and-continue path, so never duplicate it as a branch outcome. A source-backed operation that changes data or state uses transform, with the actual operation in the middle; beforeAfter is only for comparing two supported states when this claim does not explain the operation between them. Verified call order uses timeline; independent actors use lanes; FIFO/LIFO uses queue/stack; actual nesting uses tree; byte/field boundaries use bytes/table; pixels use matrix; lookup uses key, hit/miss and route; quantities use metric. Use a graph only when those forms cannot show a genuinely non-linear relationship; focused graphs show at most eight nodes and one edge of context. Choose among these visual grammars by meaning. Across a guide, let code handoffs change the form when the behavior changes. Do not fall back to generic cards or a graph to fill space. Name each scene with the action. Give each visual node a distinct, behavior-specific label; keep a shared mechanism label in the scene title and use unique guard/result names on each branch. Make the layout enact the action around one focal point. Preserve source-backed guards, misses, outcomes, errors and uncertainty; never imply unobserved execution, priority or values. For static code without a concrete runtime sample, use the actual source variable or field as a symbolic input/result where it clarifies the change; do not replace it with generic stand-ins such as “Loaded input” or “Processed result”. Label symbolic values as symbolic when a reader could mistake them for observed runtime data. Never invent coordinates, payload contents or measured outputs. Default every authored node's source to null; use an exact source value only when that literal itself matters and occurs once in the node's cited excerpt. Node evidence keeps code clickable. The app renders safe structured elements; never emit markup, scripts, URLs or handlers. This version uses inert app-owned HTML/CSS primitives; do not imply 3-D/WebGL is available. In narrated scenes, keep compact all-caps abbreviations and format tags in visible claims/source links; speak the role they play in the behavior. New scenes with a compact all-caps token in speech are rejected for correction."#;

const VISIBLE_VALUE_CONTRACT: &str = r#"Visible-value contract v30: Every content leaf needs a visible input, guard, change, output or destination; no placeholders or meaning hidden in help/narration. Default every visual node source to null. Use an exact source value only when that literal itself matters to understanding and occurs exactly once in its cited excerpt; never use source for identifiers, labels or code snippets. Node evidence keeps code clickable. Use explanation for short visual copy grounded in the claim, such as a guard, role, or outcome label; it is not a measured runtime value. Use illustration only for an invented hypothetical input or output, and label it illustrative. Do not badge grounded explanation as an example. Keep labels short (2–8 words, under 45 visible characters per card); preserve longer exact tokens only when they matter. Put sentence-length nuance in narration; use a wide/vertical scene instead of several long cards in a row/grid. Limits are ceilings: 8 nodes/section, 12 edges, 24 elements/8 leaves/scene, 6 children/group. A pointFlow has exactly three direct content leaves in input, operation, result order; use it only for a source-proved point/vector mapping and never imply coordinates absent from source. If no concise visual fits, use narration and links."#;

const SPOKEN_ROLE_INSTRUCTIONS: &str = r#"Speech is for the listener, not a code reader. Do not speak function, method, type, field or error identifiers, backticks, syntax punctuation or compact format tags; say their roles. Never spell an all-capital abbreviation or file-format tag aloud letter by letter; replace it with its role. Speak exact tokens only when asked what they mean or how they are spelled. Keep exact names visible in clickable claims and source links."#;

const OUTCOME_TEACHING: &str = r#"Show the changed value/state or outcome, not a copied expression. Keep source expressions in clickable links unless syntax is the lesson. Pair every alternative with its actual guard; distinguish clipped offsets from empty returns. State example assumptions and never generalize from one input."#;

const FALLIBLE_TRANSITIONS: &str = r#"Selecting a handler does not mean its work succeeds; a missing preliminary error does not rule out a later failure. Dispatch evidence proves only the call and caller-propagated result. Cite the callee for internals; do not invent success or errors. Cross-check claim, narration, labels and values; preserve guards and never promote a call to success. In a graph, label an edge with the operation that actually produces its result: a caller lookup followed by a load is a call or sequence, not a status edge from the lookup. Start success/failure branches at the exact operation or status check that distinguishes them; never connect those outcomes to an earlier node merely because the caller later reaches them. Each edge needs cited code that directly establishes the relationship. If the source only establishes that two operations occur in order, show order without claiming the later operation's status. The full map is the union of source-backed edges, never an inferred link added to complete a story. No extra provider operation or larger budget is authorized."#;

const VISIBLE_DECISIONS: &str = r#"Show every supported outcome with its guard and destination. Selector checks stay source ordered; show the visible miss/fallback. Split crowded chains, keep labels concise, and label examples."#;

const COMPOSITION_CONTRACT: &str = r#"A dataFlow, sequence or hierarchy visual has kind, title, direction, nodes, edges and presentation. The whole section has at most 12 nodes and 20 edges across all claim scenes, not per scene. Budget first: aim for one or two evidence nodes per claim and express relationships through layout/value fields, not a node for every label. Each node/edge cites 1–8 same-section zero-based {claimIndex,evidenceIndex} references; it cannot invent IDs or borrow another section's evidence. Every node also has source:null or source:{evidence:{claimIndex,evidenceIndex},value:"exact source substring"}. A non-null source must repeat one of that node's evidence references, and value must occur exactly once in that verified excerpt; native code resolves the cited anchor and checks the exact substring. Default every node's source to null; node evidence keeps code clickable. Set a source value only when its exact literal spelling or bytes are necessary to understand the behavior, never for identifiers, labels or snippets. Do not display a source expression just to make a visual clickable. An import is not a runtime call. A graph without presentation needs at least one evidenced edge. A composition may have no edges. If no useful visual is supported use kind:none, title:"", direction:auto, nodes:[], edges:[], presentation:null and author one mode:narration scene per claim with empty nodes, edges and elements. A plain graph also uses presentation:null.

For a narrated HTML composition use presentation:{schemaVersion:3,scenes:[{claimIndex:0,title:"short visual heading",narration:"Natural speech following the real subject through this claim, with a brief cue to locate its guard, change, or outcome.",mode:"composition",elements:[...]}]}. Use one ordered scene per claim. A graph scene uses mode:graph,elements:[]; keep it to eight visible nodes (claim-linked nodes plus one edge of context), or show the full graph when no nodes are claim-linked. Use graphs only for genuinely non-linear relationships that a timeline, branch or selector cannot express clearly. A single if/else uses branch, with its guard and each supported outcome as separate content leaves; never combine a guard and outcome in one card. Separate independent validation guards into separate claims/scenes. Use selector only for competing ordered candidates where first match wins and a miss tries the next candidate; the claim or narration must say this handoff explicitly. Each direct branch has exactly two direct content leaves, one guard and the one result for a match. The app renders the miss-and-continue path between candidates; do not add the miss as another outcome. Split broad claims when needed; preserve every outcome and the requested map. The app owns HTML, styling and interactions. Elements have exactly kind,parentIndex,nodeIndex,tone,value,valueOrigin in a flat preorder tree: root parentIndex:null, others name an earlier container. Groups are row,column,grid,panel,compare,stack,timeline,lanes,branch,beforeAfter,transform,pointFlow,tree,queue,selector and have null nodeIndex/value. timeline means evidenced execution order; lanes are independent actors without implied order; branch is one guard followed by its complete set of outcomes; selector is the root and directly contains at least two source-ordered candidate branches. beforeAfter compares two supported states only when the operation between them is outside this claim. Use transform when the cited operation changes its input; it has three direct children in input, operation, result order. The middle slot names that source-backed operation, so the visual does not reduce a meaningful change to "A became B". Use pointFlow only when cited code proves a point or vector enters an operation and emerges as a result; it uses symbolic point markers and a function cue, and must never imply coordinates or geometry the source does not show. Tree uses nesting to show source-backed hierarchy; queue shows FIFO order front to back. Use each group only when its meaning fits the source. Content is card,token,code,metric,bytes,matrix,table,text; each nodeIndex supplies its label and source links and cites this scene's claim. Tone is neutral,accent,muted,positive,warning. Arrow/divider are decorative only. Use cards only as concise labels/values inside a semantic visual, not as the visual itself. Per scene, 48 elements and 12 content leaves are hard limits, not targets; keep each scene concise and its relation legible. Choose forms independently by scene semantics; across a guide, let actual behavior handoffs change the form. Do not repeat one layout across unrelated topics or vary a form just for decoration. One node can suit a focused claim; a complex selector may need a few. Never add nodes to make a scene look busy.

A nonempty value has valueOrigin:source only when it is an exact substring of that node's own same-claim cited excerpts. Do not relabel copied source as an illustration to evade the check. Use valueOrigin:explanation for concise visual copy grounded in the claim, such as a guard, role or outcome label; it is not a runtime observation. Use valueOrigin:illustration only for invented hypothetical inputs or outputs, and receive a visible example badge. Empty values use valueOrigin:none. Values are at most 1,024 UTF-8 bytes. Bytes have 1–32 whitespace-separated cells; matrices have 1–16 equal rows of 1–24 zero/one/dot cells; tables have at most eight newline rows and six equal pipe-separated columns. A transform group has exactly three direct children in input, operation, result order; the app labels and renders those roles. A tree group represents actual nesting. A queue group is FIFO, with children ordered front to back. Repeated visible labels/values on both sides of an arrow do not show a transformation. Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields. A valid schema or source token does not prove an illustrative outcome; check the whole scene against its claim before returning."#;

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

pub(crate) fn progressive_teaching_instructions() -> String {
    [
        STORY_INTRO,
        BEHAVIOR_TEACHING,
        EVIDENCE_MEANING,
        VISUAL_DESIGN_METHOD,
        VISIBLE_VALUE_CONTRACT,
        SPOKEN_ROLE_INSTRUCTIONS,
        OUTCOME_TEACHING,
        FALLIBLE_TRANSITIONS,
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
        let authored_narration = chapters::authored_section_narration(section)?;
        let narration = authored_narration
            .as_deref()
            .unwrap_or_else(|| section.spoken_text.trim())
            .to_owned();
        if !(20..=4_000).contains(&narration.len()) {
            return Err("The complete authored narration must be 20–4,000 UTF-8 bytes. Split the section without omitting behavior. No draft was saved.".into());
        }
        if authored_narration.is_none()
            && crate::scene_presentation::has_compact_spoken_code_token(&narration)
        {
            return Err("Narration contains a compact code abbreviation or format token. Keep exact names in the visible claim and source link; speak what the token does instead. No draft was saved.".into());
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
        assert!(instructions.contains("ai-html-compositions-v44"));
        assert!(
            instructions.contains(
                "whole section has at most 12 nodes and 20 edges across all claim scenes"
            )
        );
        assert!(instructions.contains(
            "Each scene makes one change, decision or result visible and explains why it matters"
        ));
        assert!(instructions.contains("one app-supported composition per scene"));
        assert!(instructions.contains("not file type or a favorite template"));
        assert!(instructions.contains("one focal point"));
        assert!(instructions.contains("Give each visual node a distinct, behavior-specific label"));
        assert!(instructions.contains("keep a shared mechanism label in the scene title"));
        assert!(
            instructions
                .contains("use the actual source variable or field as a symbolic input/result")
        );
        assert!(instructions.contains("do not replace it with generic stand-ins such as"));
        assert!(instructions.contains("under 45 visible characters per card"));
        assert!(instructions.contains("instead of several long cards in a row/grid"));
        assert!(instructions.contains("Verified call order uses timeline"));
        assert!(instructions.contains("Use selector only for competing ordered candidates"));
        assert!(
            instructions.contains(
                "Independent validation guards in one function are separate claim-scenes"
            )
        );
        assert!(instructions.contains("Each selector branch has exactly one guard"));
        assert!(
            instructions
                .contains("claim or narration must explicitly describe that candidate handoff")
        );
        assert!(instructions.contains("uses transform, with the actual operation in the middle"));
        assert!(instructions.contains("never combine a guard and outcome in one card"));
        assert!(instructions.contains("app renders the miss-and-continue path between candidates"));
        assert!(
            instructions
                .contains("A single guard with its success and failure outcomes uses branch")
        );
        assert!(instructions.contains("independent actors use lanes"));
        assert!(instructions.contains("FIFO/LIFO uses queue/stack"));
        assert!(instructions.contains("actual nesting uses tree"));
        assert!(instructions.contains("byte/field boundaries use bytes/table"));
        assert!(instructions.contains(
            "Use a graph only when those forms cannot show a genuinely non-linear relationship"
        ));
        assert!(instructions.contains("lookup uses key, hit/miss and route"));
        assert!(
            instructions.contains("Use explanation for short visual copy grounded in the claim")
        );
        assert!(
            instructions
                .contains("Use illustration only for an invented hypothetical input or output")
        );
        assert!(instructions.contains("Every content leaf needs a visible input"));
        assert!(instructions.contains("no placeholders"));
        assert!(instructions.contains("This version uses inert app-owned HTML/CSS primitives"));
        assert!(instructions.contains("do not imply 3-D/WebGL is available"));
        assert!(instructions.contains("The app owns HTML, styling and interactions"));
        assert!(instructions.contains("8 nodes/section"));
        assert!(instructions.contains("Default every visual node source to null"));
        assert!(instructions.contains("never imply unobserved execution, priority or values"));
        assert!(instructions.contains(
            "Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields"
        ));
        assert!(instructions.contains("Use cards only as concise labels/values inside a semantic visual, not as the visual itself"));
        assert!(instructions.contains(super::BEHAVIOR_TEACHING));
        assert!(instructions.contains(super::OUTCOME_TEACHING));
        assert!(instructions.contains("a caller lookup followed by a load is a call or sequence, not a status edge from the lookup"));
        assert!(instructions.contains("The full map is the union of source-backed edges"));
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
            SECTION_NARRATION_INSTRUCTIONS.contains("Do not pronounce all-capital abbreviations")
        );
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
        assert!(instructions().contains("valueOrigin:explanation"));
        assert!(instructions().contains("invented hypothetical input or output"));
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
        assert!(super::instructions().contains("Visual design method v44:"));
        assert!(super::progressive_teaching_instructions().contains("Visual design method v44:"));
        assert!(!super::instructions().contains("Visual design method v33:"));
        assert!(shared.contains("VISUAL STORY CONTRACT — ai-html-compositions-v44"));
        assert!(shared.contains("speak the role they play in the behavior"));
        assert!(shared.contains("rejected for correction"));
        assert!(
            shared.contains("Follow a real input through source-grounded changes and outcomes")
        );
        assert!(shared.contains("Verified call order uses timeline"));
        assert!(shared.contains("Use selector only for competing ordered candidates"));
        assert!(
            shared.contains("A single guard with its success and failure outcomes uses branch")
        );
        assert!(shared.contains("lookup uses key, hit/miss and route"));
        assert!(shared.contains("uses transform, with the actual operation in the middle"));
        assert!(shared.contains("independent actors use lanes"));
        assert!(shared.contains("byte/field boundaries use bytes/table"));
        assert!(shared.contains(
            "Use a graph only when those forms cannot show a genuinely non-linear relationship"
        ));
        assert!(
            super::instructions()
                .contains("Each direct branch has exactly two direct content leaves")
        );
        assert!(shared.contains("not file type or a favorite template"));
        assert!(shared.contains("one app-supported composition per scene"));
        assert!(shared.contains("pixels use matrix"));
        assert!(shared.contains("This version uses inert app-owned HTML/CSS primitives"));
        assert!(shared.contains("do not imply 3-D/WebGL is available"));
        assert!(super::instructions().contains("The app owns HTML, styling and interactions"));
        assert!(shared.contains("Preserve guards, alternatives, failures and uncertainty"));
        assert!(shared.contains("Selector checks stay source ordered"));
        assert!(shared.contains("Split crowded chains"));
        assert!(shared.contains("keep labels concise"));
        assert!(super::instructions().contains(
            "Use cards only as concise labels/values inside a semantic visual, not as the visual itself"
        ));
        assert!(shared.contains("Show every supported outcome with its guard and destination"));
        assert!(shared.contains("If no visual helps the section, set kind:none"));
        assert!(shared.contains(
            "give every claim a narration scene with mode:narration and empty nodes, edges and elements"
        ));
        assert!(
            shared.contains(
                "Otherwise every claim needs a scene with at least one source-linked node"
            )
        );
        assert!(shared.contains("Never mix modes or leave a visual scene empty"));
        assert!(shared.contains("never describes the artifact"));
        assert!(shared.contains("Do not speak function, method, type, field or error identifiers"));
        assert!(shared.contains("Selector checks stay source ordered"));
        assert!(shared.contains("visible miss/fallback"));
        assert!(super::instructions().contains("Default every node's source to null"));
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
        for speech in [
            "too short".to_owned(),
            "é".repeat(2_001),
            "The PFR0 tag routes bytes to its dedicated font-face loader.".to_owned(),
        ] {
            let mut draft = revision(&fixture, serde_json::to_value(simple_proposal()).unwrap());
            draft.sections[0].visual = None;
            draft.sections[0].spoken_text = speech;
            let original = draft.clone();
            let error = prepare_generated_sections(&mut draft, None).unwrap_err();
            if original.sections[0].spoken_text.contains("PFR0") {
                assert!(error.contains("compact code abbreviation or format token"));
            }
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
