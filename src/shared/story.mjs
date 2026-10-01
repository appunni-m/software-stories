import { narrativeVersion, maxNarrationWords, narratesTheScreen } from './narrative.mjs';
export const sceneTypes = ['flow', 'sequence', 'branch', 'state', 'memory', 'diff', 'code'];
const string = { type: 'string' };
const object = properties => ({ type: 'object', properties, required: Object.keys(properties), additionalProperties: false });
const array = items => ({ type: 'array', items });
const evidenceSchema = object({ path: string, startLine: { type: 'integer' }, endLine: { type: 'integer' }, quote: string });
const sceneSchema = object({
  type: { type: 'string', enum: sceneTypes },
  rationale: string,
  nodes: array(object({ id: string, label: string, detail: string })),
  edges: array(object({ from: string, to: string, label: string })),
  steps: array(object({ label: string, detail: string, nodeId: string, fromNodeId: { type: ['string', 'null'] }, cue: string, claimIndex: { type: 'integer' } })),
});
export const storySchema = object({ title: string, chapters: array(object({
  title: string, summary: string, takeaway: string, narration: string,
  narrativeVersion: { type: 'integer', enum: [narrativeVersion] },
  claims: array(object({ text: string, basis: { type: 'string', enum: ['implementation', 'test', 'documentation', 'configuration', 'inference'] }, evidence: array(evidenceSchema) })),
  scene: sceneSchema, follows: array(object({ label: string, prompt: string })),
})) });
function assert(condition, message) { if (!condition) throw new Error(`Story validation: ${message}`); }
function text(value, label, max = 2000) { assert(typeof value === 'string' && value.trim().length > 0 && value.length <= max, `${label} must be nonempty text (max ${max} characters).`); }
export function validateStory(story, snapshot) {
  text(story.title, 'title', 160);
  assert(Array.isArray(story.chapters) && story.chapters.length >= 1 && story.chapters.length <= 6, 'Expected 1–6 chapters.');
  const files = new Map(snapshot.files.map(file => [file.path, file]));
  for (const chapter of story.chapters) {
    text(chapter.title, 'chapter title', 160); text(chapter.summary, 'summary', 600); text(chapter.narration, 'narration', 2000);
    if (chapter.takeaway !== undefined) {
      text(chapter.takeaway, 'recap takeaway', 180);
      assert(chapter.narration.trim().split(/\s+/).length <= maxNarrationWords, `An explanation must stay within ${maxNarrationWords} spoken words.`);
    }
    if (chapter.narrativeVersion !== undefined) {
      assert(chapter.narrativeVersion === narrativeVersion, 'Unknown narrative version.');
      assert(!narratesTheScreen(chapter.narration), 'Explain the software rather than directing attention to animation.');
    }
    assert(Array.isArray(chapter.claims) && chapter.claims.length >= 1 && chapter.claims.length <= 8, 'Claims are required.');
    for (const claim of chapter.claims) {
      text(claim.text, 'claim', 1500);
      assert(['implementation', 'test', 'documentation', 'configuration', 'inference'].includes(claim.basis), 'Unknown claim basis.');
      assert(Array.isArray(claim.evidence) && claim.evidence.length >= 1 && claim.evidence.length <= 6, 'Each claim needs evidence.');
      for (const citation of claim.evidence) {
        const file = files.get(citation.path);
        assert(file, `Unknown evidence path ${citation.path}.`);
        const lines = file.text.split('\n');
        assert(Number.isInteger(citation.startLine) && Number.isInteger(citation.endLine) && citation.startLine >= 1 && citation.endLine >= citation.startLine && citation.endLine <= lines.length, 'Evidence line range is invalid.');
        text(citation.quote, 'quote', 8000);
        assert(lines.slice(citation.startLine - 1, citation.endLine).join('\n').includes(citation.quote), `Quote does not match ${citation.path}:${citation.startLine}.`);
        assert(claim.basis === 'inference' || claim.basis === file.kind, `A ${file.kind} source cannot establish an ${claim.basis} claim.`);
      }
    }
    const scene = chapter.scene;
    assert(scene && sceneTypes.includes(scene.type), 'Unknown scene type.');
    if (scene.rationale !== undefined) text(scene.rationale, 'visual rationale', 300);
    if (scene.rationale !== undefined && scene.type === 'diff') {
      const evidence = chapter.claims.flatMap(claim => claim.evidence);
      assert(evidence.some(before => before.path.startsWith('before:') && evidence.some(after => after.path === before.path.slice(7))), 'A comparison needs cited earlier and current source for the same file.');
    }
    assert(Array.isArray(scene.nodes) && scene.nodes.length >= 1 && scene.nodes.length <= 8, 'Scenes need 1–8 nodes.');
    const ids = new Set();
    for (const node of scene.nodes) { text(node.id, 'node ID', 80); text(node.label, 'node label', 80); text(node.detail, 'node detail', 240); assert(!ids.has(node.id), 'Duplicate scene node.'); ids.add(node.id); }
    assert(Array.isArray(scene.edges) && scene.edges.length <= 12, 'Invalid scene edges.');
    for (const edge of scene.edges) { assert(ids.has(edge.from) && ids.has(edge.to), 'Edge references an unknown node.'); assert(typeof edge.label === 'string' && edge.label.length <= 100, 'Invalid edge label.'); }
    assert(Array.isArray(scene.steps) && scene.steps.length >= 1 && scene.steps.length <= 10, 'Scenes need 1–10 steps.');
    let previousCue = -1;
    for (const step of scene.steps) {
      text(step.label, 'step label', 120); text(step.detail, 'step detail', 300); assert(ids.has(step.nodeId), 'Step references an unknown node.');
      if (step.fromNodeId !== undefined && step.fromNodeId !== null) {
        assert(ids.has(step.fromNodeId), 'Handoff starts at an unknown node.');
        if (['flow', 'branch'].includes(scene.type)) assert(scene.edges.some(edge => edge.from === step.fromNodeId && edge.to === step.nodeId), 'A visual handoff needs a scene relationship.');
      }
      if (step.cue !== undefined) {
        text(step.cue, 'narration cue', 180);
        const cue = chapter.narration.indexOf(step.cue);
        assert(cue >= 0 && cue > previousCue, 'Narration cues must be exact phrases in spoken order.'); previousCue = cue;
      }
      if (step.claimIndex !== undefined) assert(Number.isInteger(step.claimIndex) && step.claimIndex >= 0 && step.claimIndex < chapter.claims.length, 'Step references an unknown claim.');
    }
    assert(Array.isArray(chapter.follows) && chapter.follows.length <= 4, 'Invalid follow-up choices.');
    for (const follow of chapter.follows) { text(follow.label, 'follow label', 100); text(follow.prompt, 'follow prompt', 1000); }
  }
  return story;
}
export function activeStep(scene, progress) { return Math.min(scene.steps.length - 1, Math.floor(Math.max(0, Math.min(0.9999, progress)) * scene.steps.length)); }
export function durationOf(chapter) { return chapter.audio?.duration ?? Math.max(8, chapter.narration.split(/\s+/).length / 2.5); }
