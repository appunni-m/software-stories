//! Current chapter requests share the reader's inert composition vocabulary.
//! Stored content uses the same current visual and scene versions.

use crate::chapters::{self, ChapterRevision};
use serde_json::{Value, json};

pub(crate) const TEACHING_POLICY_VERSION: &str = "meaning-first-v38";
pub(crate) const STORY_POLICY_VERSION: &str = "ai-html-compositions-v83";
pub(crate) const READER_LANGUAGE_POLICY_VERSION: &str = "reader-language-v1";

/// One speech contract for file, overview and selected-section user prompts.
/// Keep the full script and inspectable claim fields complete.
pub(super) const SECTION_NARRATION_INSTRUCTIONS: &str = "Complete scene narration: when a section has a visual presentation, spokenText joins every scene narration in claim order, including visible inference labels. It must preserve the full behavior, guards, alternatives, errors and uncertainty of all claims, not a summary or a chosen subset. Tell what the real subject does and why it matters; point to a short visual cue only to help the listener follow. Never narrate the artifact itself or open with meta commentary such as ‘these boxes show’, ‘this diagram explains’ or ‘the scene illustrates’. Avoid definition-first openings such as ‘X is…’ or ‘this function handles…’; do not catalogue responsibilities. Start at the caller, input or decision, use present-tense cause and effect to tell what changes next, then explain what the result means for the caller or following step when the code supports that consequence. Keep simple scenes to one or two short sentences; when the behavior needs room, use up to three natural sentences and 420 UTF-8 bytes to connect the input, actual decision or change, a brief cue visible in this scene and the caller-visible consequence. Aim for 20–35 words per scene. Use short clauses with measured pacing. Every spoken cue must appear in this scene and be supported by its claim. Explain every outcome plainly. At first spoken use, briefly unpack a specialist term in the role the cited implementation gives it; do not leave jargon or an error type as the whole explanation. For a failure, say what stops or continues and what the caller receives, keeping exact error labels in the visible claim and source. Suggest recovery only when cited behavior supports it. If the cited source does not establish a term's meaning or caller consequence, state that limit instead of guessing. Keep exact identifiers, signatures, abbreviations, file-format tags, field/table tokens and error labels visible in claims, visual details and source links; in speech describe each by its role (for example, say ‘the first font-format check’ rather than spelling its tag, and ‘the glyph's stored horizontal width’ rather than a table token). Do not pronounce all-capital abbreviations or format tags letter by letter; replace them with their job in the behavior. Speak an exact token only when the user explicitly asks about its spelling or meaning. If several checks would become a spoken list, split them into ordered scenes without dropping any condition or outcome. When the section has no visual, write a complete natural spoken account of all claims with the same story-first style, preserving every guard, alternative, error, inference label and uncertainty. Keep the complete transport script within 20–4,000 UTF-8 bytes and the declared section shape; organize focused claims without dropping behavior. The app retains exact claims separately and assembles speech from all scenes.";

const STORY_INTRO: &str = r#"VISUAL STORY CONTRACT — ai-html-compositions-v83
Give every section and scene a reader-facing title that names its behavior or question in plain language. Avoid bare file names, format tags, or terse labels such as “PFR0: direct handoff.” Each scene makes one change, decision or result visible and explains why it matters.

Start from a real caller, input, event, or decision shown by the cited implementation. Follow a real input through source-grounded changes and outcomes. Do not invent a user motive, interface event, runtime result, or example value. Use a visual for concrete behavior and use a text-only section only for context or an evidence boundary with no honest visual form. When every claim is context or an evidence boundary without an honest visual form, use narration only. Choose the visual mode independently for each claim scene; a visual section may mix composition and graph scenes. Use an example only when its values are supported or visibly labeled hypothetical.

Tell the behavior in cause-and-effect order. Explain what the result means to the caller or next step when the code proves it. Avoid dictionary definitions and responsibility lists. Narration never describes the artifact. Preserve guards, alternatives, failures and uncertainty, including inference. Name the specific limit when evidence stops the story instead of guessing.

Every generated visual scene must have at least one node linked to that scene's own cited claim. Do not leave a scene empty or substitute an unrelated full map for its focused explanation. Use natural-language behavior titles and labels; keep exact identifiers in inspectable claims and clickable source. Prefix inference narration with its visible label."#;

const BEHAVIOR_TEACHING: &str = r#"State the decision rule before examples; explain every source-supported outcome, including zero/nonzero and empty-result behavior."#;

const EVIDENCE_MEANING: &str = r#"Check guards, failures, sibling branches and later conditions. Distinguish rejected, missing, malformed and successful inputs only when code shows them. Claims saying only, always or never require every relevant branch and updated field or dimension; otherwise mark partial. Comments, imports, identifiers and defaults do not prove consumer behavior. Cite downstream consumers and producing conditions or omit effects. Names and references do not prove meaning."#;

pub(crate) const READER_LANGUAGE_POLICY: &str = r#"Reader language policy (reader-language-v1): Write all reader-facing text, including titles, claims, summaries, caveats, visual labels and narration, in the language explicitly requested by the user; otherwise use the natural language of the user's question. If the request contains no clear language signal, use English. Keep source identifiers, file paths, code, literals, exact citations and required schema keys unchanged. Explain code tokens in the requested language rather than translating or reading the tokens aloud."#;

const VISUAL_DESIGN_METHOD: &str = r#"Visual design method v73: Let cited behavior choose the smallest app-supported form for each claim. Do not choose by file type or favorite template. Before composing, name the behavior and its best teaching form.

Use transform when the focal claim maps a named source-backed input or state to a distinct result; show different input and result around the operation. Point/vector/outline mapping MUST use root pointFlow, not transform; show symbolic before/after; invent no coordinates. Aggregate with bounds/metric. Use branch only when mutually exclusive, source-backed choices are the behavior being taught. An if, match, early return or other control-flow syntax alone does not justify a branch. If a guard gates a source-backed data/state change, use transform, include the guard in its operation label, and show the actual input and result; do not leave a supported change to narration alone. Give a failure its own scene unless comparing outcomes is the focal question. When matched cases are central, an enum, match, switch or pattern match uses distinct matched cases and results; use a compact table for many cases. Never reuse the guard label as an outcome label. Independent validation guards in one function are separate claim-scenes; preserve their path condition or split into a follow-up scene. Use selector only for competing ordered candidates. In prose, use it only for source-ordered candidates that continue after a miss. Each selector branch has exactly one guard and its matching result; the claim or narration must describe that handoff. Verified call order uses timeline; independent actors use lanes; actual nesting uses tree; queue shows FIFO order front to back; stack entries are authored oldest/bottom first and displayed newest/top first. Use beforeAfter only to compare states, not explain behavior. Byte/field boundaries use bytes/table, pixels use matrix, lookup uses key, hit/miss and route, and quantities use metric. Use a graph only when those forms cannot show a genuinely non-linear relationship. Every graph scene must show a labeled edge joining a node focused on that claim, and both node and edge must cite that claim's source evidence. Every other edge touching a focused node must cite the same claim; if no such evidence-backed relationships exist, choose a composition. The full map is separate.

Branch only for focal choices. Table ordered guards only if early exit is clear; otherwise split by path. Use timeline for proven sequences, transform for gated changes. Never imply later checks run after failure.

Rows, columns, grids, and panels arrange content; those containers are layout, not the visual idea. Use the actual source variable or field as a symbolic input/result when it clarifies static code; do not replace it with generic stand-ins such as “Loaded input” or “Processed result.” Never infer a transformation, runtime value, execution, priority, or relationship from nearby names. A code arrow asserts a source-proven relation.

The visual must remain understandable with narration collapsed. Give each visual node a distinct, behavior-specific label; keep a shared mechanism label in the scene title. Use short natural-language actions or outcomes rather than raw identifiers or snake_case. Exact names stay in claims and clickable source unless spelling itself is the lesson. A value must add visible meaning beyond its label. Keep the visual focused rather than busy. Across adjacent scenes, repeat a root form only when it still fits each focal behavior; change form when the next behavior calls for a clearer supported representation. Do not force variety by adding decoration or inventing content. The app owns HTML, styling and interactions; this version uses inert app-owned HTML/CSS primitives. Raw JavaScript and 3-D/WebGL are unsupported."#;

const VISUAL_COMPOSITION_REVIEW: &str = r#"Visual composition review v73: With narration collapsed, can a reader see the source-backed input, action/guard and result? Does the root form teach the claim's focal behavior, or does an incidental guard dominate it? A control-flow construct alone is not a branch reason. If data or state changes, show before/operation/result, with any gating condition in the operation. Labels and arrows must mean what the cited code proves. Repeated branches across a Guide should prompt a form check, not forced variety. Symbolic or hypothetical values must not look observed.

Keep one focal point and a source-appropriate semantic form: branch, selector, timeline, lanes, transform, before/after, tree, queue, stack, point-flow, comparison, or data-specific bytes, matrix, table or metric. Rows, columns, grids and panels arrange content; cards, tokens, code and prose alone are not the visual idea. If the scene can be reduced to generic boxes with different labels, choose a more informative form. Do not force a visual when no honest one helps. Cards may label a semantic form but cannot replace it. Do not repeat the same generic row/cards/arrow skeleton with only labels changed. Before proposing another section for the same answer, compare its behavior with sections already published. A new title or a second rendering of the same decision is not new coverage. If evidence or limits block a request, name the concrete uncovered behavior instead of repeating a neighboring section."#;

const VISIBLE_VALUE_CONTRACT: &str = r#"Visible-value contract v33: Every source-backed behavior needs a visual scene with a visible input, guard, change, output or destination; use text-only only for context or a genuine evidence boundary. Every content leaf needs a visible input, guard, change, output or destination; no placeholders or meaning hidden in help/narration. Default every node's source to null. A nonempty value uses valueOrigin:source for its exact cited substring, valueOrigin:explanation for grounded visual copy, or valueOrigin:illustration for a labeled hypothetical; empty values use valueOrigin:none. Use an exact source value only when that literal itself matters to understanding and occurs exactly once in its cited excerpt; never use source for identifiers, labels or code snippets. Node evidence keeps code clickable. Use explanation for short visual copy grounded in the claim, such as a guard, role, or outcome label; it is not a measured runtime value. Use illustration only for an invented hypothetical input or output, and label it illustrative. Do not badge grounded explanation as an example. Keep labels short (2–8 words, under 45 visible characters per card); preserve longer exact tokens only when they matter. Put sentence-length nuance in narration; use a wide/vertical scene instead of several long cards in a row/grid. Treat all limits as ceilings, never targets: the whole section allows at most 12 nodes and 20 edges across its claim scenes; every graph scene needs a node linked to its own claim and a labeled edge joining that focus with evidence from the same claim; every edge touching a focused node must cite that same claim; a focused graph scene shows at most eight visible nodes including one edge of context. Full-map inspection remains a separate reader view. Each scene allows at most 48 elements and 12 content leaves, with at most six nesting levels and 12 direct children per group. These bounds match the native admission schema. A transform’s input and result slots must show different visible content; repeated labels and values are not a transformation even though the renderer provides an arrow between slots. A pointFlow has exactly three direct content leaves in input, operation, result order; use it only for a source-proved point/vector mapping and never imply coordinates absent from source. If a specific behavior cannot be represented safely, use narration and links and state the source boundary plainly."#;

const SPOKEN_ROLE_INSTRUCTIONS: &str = r#"Speech is for the listener, not a code reader. Do not speak function, method, type, field or error identifiers, snake_case names, member chains, compact calls such as load_glyph(), namespace syntax such as Font::memory_face, primitive type spellings such as u32, generic syntax, file paths, backticks, operators, or compact format tags; say their roles in ordinary language. Never spell an all-capital abbreviation or file-format tag aloud letter by letter; replace it with its role. Speak exact tokens only when asked what they mean or how they are spelled. Keep exact names visible in clickable claims and source links."#;

const OUTCOME_TEACHING: &str = r#"Show the changed value/state or outcome, not a copied expression. Keep source expressions in clickable links unless syntax is the lesson. Pair every alternative with its actual guard; distinguish clipped offsets from empty returns. State example assumptions and never generalize from one input."#;

const FALLIBLE_TRANSITIONS: &str = r#"Selecting a handler does not mean its work succeeds; a missing preliminary error does not rule out a later failure. Dispatch evidence proves only the call and caller-propagated result. Cite the callee for internals; do not invent success or errors. Cross-check claim, narration, labels and values; preserve guards and never promote a call to success. In a graph, label an edge with the operation that actually produces its result: a caller lookup followed by a load is a call or sequence, not a status edge from the lookup. Start success/failure branches at the exact operation or status check that distinguishes them; never connect those outcomes to an earlier node merely because the caller later reaches them. Each edge needs cited code that directly establishes the relationship. If the source only establishes that two operations occur in order, show order without claiming the later operation's status. The full map is the union of source-backed edges, never an inferred link added to complete a story. No extra provider operation or larger budget is authorized."#;

const COMPOSITION_CONTRACT: &str = r#"A dataFlow, sequence or hierarchy visual has kind, title, direction, nodes, edges and a schema-version-3 presentation. Every non-none visual requires this presentation; graph-only diagrams use graph-mode scenes. Budget first: aim for one or two evidence nodes per claim and express relationships through layout/value fields, not a node for every label. Each node and edge cites 1–8 same-section, zero-based {claimIndex,evidenceIndex} references; never invent IDs or borrow another section's evidence. Each node has source:null or source:{evidence:{claimIndex,evidenceIndex},value:"exact source substring"}. A non-null source repeats one of that node's evidence references, and the value occurs exactly once in that verified excerpt; native code resolves the cited anchor and checks the substring. Do not display a source expression just to make a visual clickable. An import is not a runtime call. A composition may have no edges. Use kind:none only when no claim benefits from a safe visual; set title:"", direction:auto, nodes:[], edges:[], presentation:null, and put the complete natural script in section.spokenText. Do not create presentation scenes.

For a narrated composition use presentation:{schemaVersion:3,scenes:[{claimIndex:0,title:"short visual heading",narration:"Natural speech following the real subject through this claim, with a brief cue to locate its guard, change, or outcome.",mode:"composition",elements:[...]}]}. Use one ordered scene per claim and choose composition or graph independently for each; both may appear in one section. A graph scene has mode:graph,elements:[], a node citing its own claim, and a labeled edge connecting it to another node. Every edge touching a claim-focused node cites that same claim's evidence. The shared teaching rules set the focused-view limit.

Elements have exactly kind,parentIndex,nodeIndex,tone,value,valueOrigin in a flat preorder tree: root parentIndex:null, others name an earlier container. Groups are row,column,grid,panel,compare,stack,timeline,lanes,branch,beforeAfter,transform,pointFlow,tree,queue,selector; groups have null nodeIndex/value. Content is card,token,code,metric,bytes,matrix,table,text; each content nodeIndex supplies its label and source links and cites this scene's claim. Each content leaf sets valueOrigin to source, explanation, illustration, or none, following the shared value rules. Tone is neutral,accent,muted,positive,warning. Arrow/divider are decorative only. A selector is a root group whose direct children are at least two source-ordered candidate branches. Each branch directly inside a selector has exactly two direct content leaves: one guard and its matching result. The reader renders the miss-and-continue path, so do not add a miss leaf. An ordinary branch keeps its guard and one separate leaf for every supported outcome. A transform group has exactly three direct children in input, operation, result order; the app labels and renders those roles. Bytes have 1–32 whitespace-separated cells; matrices have 1–16 equal rows of 1–24 zero/one/dot cells; tables have at most eight newline rows and six equal pipe-separated columns. Values are at most 1,024 UTF-8 bytes.

Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields. A valid schema or source token does not prove an illustrative outcome; check the whole scene against its claim before returning."#;

/// Keep transport-specific fields out of progressive section instructions.
pub(crate) fn teaching_instructions() -> String {
    [
        STORY_INTRO,
        BEHAVIOR_TEACHING,
        READER_LANGUAGE_POLICY,
        EVIDENCE_MEANING,
        VISUAL_DESIGN_METHOD,
        VISUAL_COMPOSITION_REVIEW,
        VISIBLE_VALUE_CONTRACT,
        SPOKEN_ROLE_INSTRUCTIONS,
        OUTCOME_TEACHING,
        FALLIBLE_TRANSITIONS,
    ]
    .join("\n\n")
}

pub(crate) fn progressive_teaching_instructions() -> String {
    [
        STORY_INTRO,
        BEHAVIOR_TEACHING,
        READER_LANGUAGE_POLICY,
        EVIDENCE_MEANING,
        VISUAL_DESIGN_METHOD,
        VISUAL_COMPOSITION_REVIEW,
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
            .map(|v| &v.presentation)
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
            return Err("Narration contains code-shaped syntax or a compact format token. Keep exact names in the visible claim and source link; speak what the code does instead. No draft was saved.".into());
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
        tests::{Fixture, SIMPLE_PATH, simple_proposal_value},
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
        let mut proposal = simple_proposal_value();
        proposal["sections"][0]["visual"] = json!({
            "kind":"dataFlow", "title":"Put the name into the reply", "direction":"auto",
            "nodes":[
                {"label":"Greeting template","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null},
                {"label":"Example reply","detail":"","evidence":[{"claimIndex":0,"evidenceIndex":0}],"source":null}
            ], "edges":[], "presentation":{"schemaVersion":version,"scenes":[{
                "claimIndex":0,"title":"The name becomes part of the greeting","mode":"composition",
                "elements":[
                    {"kind":"transform","parentIndex":null,"nodeIndex":null,"tone":"neutral","value":"","valueOrigin":"none"},
                    {"kind":"code","parentIndex":0,"nodeIndex":0,"tone":"accent","value":"Hello, ${name}","valueOrigin":"source"},
                    {"kind":"text","parentIndex":0,"nodeIndex":0,"tone":"neutral","value":"Insert the supplied name","valueOrigin":"explanation"},
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
        for requirement in [
            "ai-html-compositions-v83",
            "Visual design method v73:",
            "Visual composition review v73:",
            "Branch only for focal choices",
            "Table ordered guards only if early exit is clear",
            "Never imply later checks run after failure",
            "Follow a real input through source-grounded changes and outcomes",
            "a visual section may mix composition and graph scenes",
            "Use transform when the focal claim maps a named source-backed input or state",
            "An if, match, early return or other control-flow syntax alone does not justify a branch",
            "do not leave a supported change to narration alone",
            "Each selector branch has exactly one guard",
            "Use a graph only when those forms cannot show a genuinely non-linear relationship",
            "Every other edge touching a focused node must cite the same claim",
            "visual must remain understandable with narration collapsed",
            "Every generated visual scene must have at least one node linked to that scene's own cited claim",
            "Raw JavaScript and 3-D/WebGL are unsupported",
            "Keep one focal point and a source-appropriate semantic form",
            "If the scene can be reduced to generic boxes with different labels",
            "Every non-none visual requires this presentation",
            "whole section allows at most 12 nodes and 20 edges across its claim scenes",
            "Each scene allows at most 48 elements and 12 content leaves",
            "Default every node's source to null",
            "Never supply HTML strings, CSS, URLs, JavaScript, event handlers or extra fields",
        ] {
            assert!(
                instructions.contains(requirement),
                "missing prompt requirement: {requirement}"
            );
        }
        assert!(!instructions.contains("mode:narration"));
        assert!(!instructions.contains("Visual design method v33:"));
        assert!(instructions.contains("The app owns HTML, styling and interactions"));
    }

    #[test]
    fn authoring_prompt_stays_bounded_after_shared_rules_are_composed() {
        let shared = teaching_instructions();
        let complete = instructions();
        assert!(
            shared.len() <= 13_000,
            "shared prompt grew to {} bytes",
            shared.len()
        );
        assert!(
            complete.len() <= 16_000,
            "complete prompt grew to {} bytes",
            complete.len()
        );
        assert!(complete.len() > shared.len());
    }

    #[test]
    fn visual_scene_document_tracks_stable_policy_versions_and_limits() {
        let reference = include_str!("../../../docs/VISUAL_SCENES.md");
        let normalized_reference = reference.split_whitespace().collect::<Vec<_>>().join(" ");
        for version in [
            STORY_POLICY_VERSION,
            TEACHING_POLICY_VERSION,
            READER_LANGUAGE_POLICY_VERSION,
        ] {
            assert!(
                normalized_reference.contains(version),
                "reference is missing {version}"
            );
        }
        let sentence_limit = crate::scene_presentation::MAX_SCENE_NARRATION_SENTENCES;
        let byte_limit = crate::scene_presentation::MAX_SCENE_NARRATION_BYTES;
        assert!(normalized_reference.contains(&format!(
            "up to {sentence_limit} short sentences and {byte_limit} UTF-8 bytes"
        )));
        assert!(normalized_reference.contains("Visual design method and composition review `v73`"));
        assert!(normalized_reference.contains(
            "Every narrated graph scene must include a node citing its own claim and at least one labeled edge connecting that focused node to another node."
        ));
        assert!(normalized_reference.contains(
            "Every edge touching a claim-focused node must cite the same claim's source evidence."
        ));
        assert!(normalized_reference.contains(
            "the separate Full map view does not substitute for a scene-grounded visual"
        ));
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
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("Start at the caller, input or decision"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("same story-first style"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("briefly unpack a specialist term"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("what the caller receives"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("state that limit instead of guessing"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("Aim for 20–35 words per scene"));
        let progressive = super::progressive_teaching_instructions();
        assert!(progressive.contains("say their roles in ordinary language"));
        assert!(
            SECTION_NARRATION_INSTRUCTIONS.contains(
                "For a failure, say what stops or continues and what the caller receives"
            )
        );
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
                .contains("up to three natural sentences and 420 UTF-8 bytes to connect the input")
        );
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("split them into ordered scenes"));
        assert!(SECTION_NARRATION_INSTRUCTIONS.contains("20–4,000 UTF-8 bytes"));
        assert!(BEHAVIOR_TEACHING.contains("State the decision rule before examples"));
        assert!(BEHAVIOR_TEACHING.contains("explain every source-supported outcome"));
        assert!(SPOKEN_ROLE_INSTRUCTIONS.contains("say their roles in ordinary language"));
        assert!(teaching_instructions().contains(BEHAVIOR_TEACHING));
        assert!(instructions().contains("valueOrigin:explanation"));
        assert!(instructions().contains("invented hypothetical input or output"));
    }

    #[test]
    fn generation_guidance_traces_guards_failures_and_cited_consumers() {
        for instructions in [super::instructions(), teaching_instructions()] {
            assert!(instructions.contains(
                "Comments, imports, identifiers and defaults do not prove consumer behavior"
            ));
            assert!(instructions.contains(
                "Check guards, failures, sibling branches and later conditions"
            ));
            assert!(
                instructions.contains("Cite downstream consumers and producing conditions")
            );
            assert!(
                instructions.contains("every relevant branch and updated field or dimension")
            );
            assert!(instructions.contains("otherwise mark partial"));
        }
    }

    #[test]
    fn shared_teaching_selects_layout_by_behavior_and_keeps_decision_outcomes_visible() {
        let shared = teaching_instructions();
        let instructions = super::instructions();
        for requirement in [
            "Visual design method v73:",
            "Visual composition review v73:",
            "Branch only for focal choices",
            "Table ordered guards only if early exit is clear",
            "Never imply later checks run after failure",
            "Do not choose by file type or favorite template",
            "Use branch only when mutually exclusive, source-backed choices are the behavior being taught",
            "An if, match, early return or other control-flow syntax alone does not justify a branch",
            "If a guard gates a source-backed data/state change, use transform, include the guard in its operation label, and show the actual input and result",
            "do not leave a supported change to narration alone",
            "Repeated branches across a Guide should prompt a form check, not forced variety",
            "When matched cases are central",
            "Does the root form teach the claim's focal behavior",
            "When matched cases are central, an enum, match, switch or pattern match uses distinct matched cases and results",
            "Each selector branch has exactly one guard",
            "source-ordered candidates that continue after a miss",
            "Independent validation guards in one function are separate claim-scenes",
            "Point/vector/outline mapping MUST use root pointFlow, not transform",
            "Aggregate with bounds/metric",
            "queue shows FIFO order front to back",
            "call order uses timeline",
            "field boundaries use bytes/table",
            "lookup uses key, hit/miss and route",
            "with narration collapsed",
            "Cards may label a semantic form but cannot replace it",
            "Do not repeat the same generic row/cards/arrow skeleton with only labels changed",
            "Before proposing another section for the same answer",
            "A new title or a second rendering of the same decision is not new coverage",
            "name the concrete uncovered behavior instead of repeating a neighboring section",
            "Use illustration only for an invented hypothetical input or output",
            "this version uses inert app-owned HTML/CSS primitives",
            "Preserve guards, alternatives, failures and uncertainty",
            "Claims saying only, always or never require every relevant branch and updated field or dimension",
        ] {
            assert!(
                shared.contains(requirement),
                "missing shared teaching rule: {requirement}"
            );
        }
        assert!(instructions.contains(
            "Each branch directly inside a selector has exactly two direct content leaves"
        ));
        assert!(instructions.contains("The app owns HTML, styling and interactions"));
        assert!(instructions.contains("The full map is the union of source-backed edges"));
        assert!(!shared.contains("mode:narration"));
        assert!(!shared.contains("parentIndex"));
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
        let mut draft = revision(&fixture, simple_proposal_value());
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
            let mut draft = revision(&fixture, simple_proposal_value());
            draft.sections[0].visual = None;
            draft.sections[0].spoken_text = speech;
            let original = draft.clone();
            let error = prepare_generated_sections(&mut draft, None).unwrap_err();
            if original.sections[0].spoken_text.contains("PFR0") {
                assert!(error.contains("code-shaped syntax or a compact format token"));
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
        let mut draft = revision(&fixture, simple_proposal_value());
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
