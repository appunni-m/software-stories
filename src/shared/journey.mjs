// Keep explored chapters beside their parent rather than at the bottom of a
// growing list. Old or malformed saved journeys must still remain navigable.
export function journeyEntries(chapters) {
  const ids = new Set(chapters.map(chapter => chapter.id)), seen = new Set(), entries = [];
  function visit(chapter, depth) {
    if (seen.has(chapter.id)) return;
    seen.add(chapter.id);
    entries.push({ chapter, index: chapters.indexOf(chapter), depth });
    for (const child of chapters.filter(item => item.parentId === chapter.id)) visit(child, Math.min(depth + 1, 12));
  }
  for (const chapter of chapters.filter(item => !item.parentId || !ids.has(item.parentId))) visit(chapter, 0);
  for (const chapter of chapters) visit(chapter, 0);
  return entries;
}

export function chapterTrail(chapters, id) {
  const trail = [], seen = new Set();
  let current = chapters.find(chapter => chapter.id === id);
  while (current && !seen.has(current.id)) {
    seen.add(current.id); trail.unshift(current);
    current = chapters.find(chapter => chapter.id === current.parentId);
  }
  return trail;
}

// The viewed path is a stack of visits, not the chapter's persisted parent tree.
// An epoch lets Back detach an outstanding request without a late answer
// stealing focus. The server can finish and cache that answer independently.
export class Exploration {
  constructor(id) {
    this.current = { id, progress: 0 };
    this.stack = [];
    this.phase = 'idle';
    this.pending = false;
    this.epoch = 0;
    this.label = '';
  }
  get locked() { return this.pending || this.phase === 'playing' || this.phase === 'loading'; }
  begin(label = '') {
    if (this.locked) return null;
    this.stack.push({ ...this.current });
    this.pending = true; this.phase = 'loading'; this.label = label;
    return ++this.epoch;
  }
  accepts(token) { return token === this.epoch; }
  arrive(token, id) {
    if (!this.accepts(token)) return false;
    this.current = { id, progress: 0 }; this.phase = 'ready';
    return true;
  }
  start() { this.phase = 'playing'; }
  pause() { this.phase = 'paused'; }
  complete() { this.current.progress = 1; this.pending = false; this.phase = 'completed'; this.label = ''; }
  back() {
    if (!this.stack.length) return null;
    ++this.epoch;
    this.current = this.stack.pop();
    this.pending = false; this.phase = 'paused'; this.label = '';
    return { ...this.current };
  }
  fail(token) { if (this.accepts(token)) return this.back(); return null; }
}
