import { readFile, realpath } from 'node:fs/promises';
import { resolve, relative, extname, basename, posix } from 'node:path';
import { CACHE, hash, run, saveJSON, loadJSON } from './util.mjs';

const extensions = new Set(['.mjs', '.js', '.ts', '.tsx', '.jsx', '.py', '.rs', '.go', '.c', '.cpp', '.h', '.hpp', '.java', '.kt', '.swift', '.rb', '.php', '.vue', '.svelte', '.html', '.css', '.scss', '.sql', '.sh', '.md', '.json', '.toml', '.yaml', '.yml']);
const ignore = ['.git', '.cache', '.venv*', 'venv', 'node_modules', 'target', 'dist', 'build', '.aws', '.codex', '.agents', '.coverage-mcp', '__pycache__', 'experiments/voice/output', '*.lock*', 'package-lock.json', '*.min.js'];
export function sourceKind(path) {
  if (/\.(md|rst|txt)$/.test(path)) return 'documentation';
  if (/(^|\/)(tests?|__tests__)\/|\.(test|spec)\./.test(path)) return 'test';
  if (/\.(json|toml|ya?ml)$/.test(path)) return 'configuration';
  return 'implementation';
}
export async function captureRepository(directory, { baseRef } = {}) {
  const root = await realpath(resolve(directory));
  const { stdout } = await run('rg', ['--files', '--hidden', ...ignore.flatMap(pattern => ['-g', `!${pattern}`]), root]);
  const paths = stdout.trim().split('\n').filter(Boolean).map(path => relative(root, path)).filter(path => extensions.has(extname(path))).sort();
  const files = [];
  let bytes = 0, omitted = 0;
  for (const path of paths) {
    if (files.length >= 2000 || bytes > 12_000_000) { omitted++; continue; }
    const absolute = await realpath(resolve(root, path));
    if (relative(root, absolute).startsWith('..')) { omitted++; continue; }
    const buffer = await readFile(absolute);
    if (buffer.length > 250_000 || buffer.includes(0)) { omitted++; continue; }
    const text = buffer.toString('utf8');
    bytes += buffer.length;
    files.push({ path, text, sha256: hash(buffer), kind: sourceKind(path), revision: 'workspace snapshot' });
  }
  let commit = null;
  try { commit = (await run('git', ['rev-parse', '--verify', 'HEAD'], { cwd: root })).stdout.trim(); } catch {}
  let diff = null, baseCommit = null;
  if (baseRef) {
    if (!commit) throw new Error('PR comparisons require a Git repository with a commit.');
    if (typeof baseRef !== 'string' || !/^[a-zA-Z0-9_./~-]{1,200}$/.test(baseRef) || baseRef.startsWith('-')) throw new Error('Invalid Git base reference.');
    baseCommit = (await run('git', ['rev-parse', '--verify', `${baseRef}^{commit}`], { cwd: root })).stdout.trim();
    diff = (await run('git', ['diff', '--no-ext-diff', '--no-textconv', baseCommit, '--'], { cwd: root })).stdout.slice(0, 50_000);
    const changed = (await run('git', ['diff', '--name-only', baseCommit, '--'], { cwd: root })).stdout.trim().split('\n').filter(Boolean);
    for (const path of changed.slice(0, 12)) {
      try {
        const text = (await run('git', ['show', `${baseCommit}:${path}`], { cwd: root })).stdout;
        if (text.length <= 250_000) files.push({ path: `before:${path}`, text, sha256: hash(text), kind: sourceKind(path), revision: baseCommit });
      } catch {}
    }
  }
  if (!files.length) throw new Error('No readable source files were found in this directory.');
  const id = hash({ root, files: files.map(({ path, sha256 }) => ({ path, sha256 })), commit, baseCommit }).slice(0, 24);
  const snapshot = { id, root, name: basename(root), commit, baseCommit, diff, capturedAt: new Date().toISOString(), omitted, files };
  await saveJSON(resolve(CACHE, 'snapshots', `${id}.json`), snapshot);
  return snapshot;
}
export const loadSnapshot = id => loadJSON(resolve(CACHE, 'snapshots', `${id}.json`));
export const snapshotInfo = snapshot => ({ id: snapshot.id, root: snapshot.root, name: snapshot.name, commit: snapshot.commit, baseCommit: snapshot.baseCommit, capturedAt: snapshot.capturedAt, fileCount: snapshot.files.length, omitted: snapshot.omitted });
export function searchSources(snapshot, query, limit = 8, { focusLabel = '', focusPaths = [], citedSymbols = {} } = {}) {
  const terms = [...new Set(query.toLowerCase().match(/[a-z_][a-z_0-9]{2,}/g) ?? [])].filter(term => !['the', 'how', 'does', 'this', 'that', 'what', 'why', 'and', 'with', 'from', 'explain', 'understand', 'project'].includes(term));
  const focus = [...new Set(focusLabel.toLowerCase().match(/[a-z_][a-z_0-9]{2,}/g) ?? [])].flatMap(term => term.endsWith('renderer') ? [term, 'render'] : [term]);
  const ranked = snapshot.files.map(file => {
    const lower = file.text.toLowerCase();
    let score = terms.reduce((sum, term) => sum + (file.path.toLowerCase().includes(term) ? 12 : 0) + Math.min(lower.split(term).length - 1, 8), 0);
    score += focus.reduce((sum, term) => sum + (file.path.toLowerCase().includes(term) ? 65 : 0) + (lower.includes(term) ? 8 : 0), 0);
    if (focusPaths.includes(file.path)) score += 35;
    if (/src\/server|src\/shared|public\/app/.test(file.path)) score += 2;
    if (file.kind === 'documentation') score *= 0.7;
    return { ...file, score };
  }).sort((a, b) => b.score - a.score);
  // Static import hints provide nearby implementation context. They are not
  // evidence of runtime call order or a replacement for a language server.
  const related = new Set();
  for (const file of ranked.slice(0, 2)) for (const match of file.text.matchAll(/(?:from\s+|import\s*|require\s*\()\s*['"](\.[^'"]+)['"]/g)) {
    const path = posix.normalize(posix.join(posix.dirname(file.path), match[1]));
    const dependency = snapshot.files.find(item => [path, path + '.mjs', path + '.js', path + '.ts', path + '/index.js', path + '/index.ts'].includes(item.path));
    if (dependency) related.add(dependency.path);
  }
  const chosen = [...ranked.slice(0, Math.max(1, limit - 2)), ...ranked.filter(file => related.has(file.path)).slice(0, 2), ...ranked];
  return [...new Map(chosen.map(file => [file.path, file])).values()].slice(0, limit).map(file => {
    const lines = file.text.split('\n');
    const selected = new Set();
    for (let i = 0; i < Math.min(25, lines.length); i++) selected.add(i);
    // Get the selected responsibility's definitions before spending the
    // excerpt budget on frequent generic terms near the top of a large file.
    for (let i = 0; i < lines.length && selected.size < 190; i++) {
      const symbol = lines[i].match(/\b(?:function|class|def|fn|func)\s+([A-Za-z_]\w*)/)?.[1];
      if (symbol && (focus.some(term => symbol.toLowerCase().includes(term)) || citedSymbols[file.path]?.includes(symbol))) {
        for (let j = Math.max(0, i - 3); j < Math.min(lines.length, i + 55); j++) selected.add(j);
      }
    }
    for (let i = 0; i < lines.length && selected.size < 190; i++) {
      if (terms.some(term => lines[i].toLowerCase().includes(term))) for (let j = Math.max(0, i - 6); j < Math.min(lines.length, i + 14); j++) selected.add(j);
    }
    if (selected.size < 60) for (let i = 0; i < Math.min(150, lines.length); i++) selected.add(i);
    let previous = -1;
    const excerpt = [...selected].sort((a, b) => a - b).map(i => { const gap = previous >= 0 && i > previous + 1 ? '[intervening lines omitted]\n' : ''; previous = i; return `${gap}${i + 1}: ${lines[i]}`; }).join('\n');
    return { path: file.path, kind: file.kind, revision: file.revision, sha256: file.sha256, excerpt };
  });
}
export function citation(snapshot, path, anchor, endAnchor = anchor) {
  const file = snapshot.files.find(file => file.path === path);
  if (!file) throw new Error(`Missing source ${path}`);
  const lines = file.text.split('\n');
  const start = lines.findIndex(line => line.includes(anchor));
  const end = lines.findIndex((line, index) => index >= start && line.includes(endAnchor));
  if (start < 0 || end < start) throw new Error(`Missing source anchor ${path}: ${anchor}`);
  return { path, startLine: start + 1, endLine: end + 1, quote: lines.slice(start, end + 1).join('\n') };
}
