import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { spawnSync } from 'node:child_process';

const workspace = process.cwd();
const project = 'code-atlas-rebuild';
const idempotencyPrefix = 'code-atlas-rebuild-product-plan-v1';
const taskDirectory = resolve(workspace, 'docs/kata-tasks');

function parseTask(file) {
  const source = readFileSync(resolve(taskDirectory, file), 'utf8');
  const match = source.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  if (!match) throw new Error(`Missing task frontmatter: ${file}`);
  const meta = {};
  for (const line of match[1].split('\n')) {
    const separator = line.indexOf(':');
    if (separator < 0) continue;
    meta[line.slice(0, separator).trim()] = line.slice(separator + 1).trim().replace(/^"(.*)"$/, '$1');
  }
  meta.file = file;
  meta.body = `${match[2].trim()}\n`;
  meta.priority = Number(meta.priority ?? 2);
  meta.labels = (meta.labels || '').split(',').map((value) => value.trim()).filter(Boolean);
  meta.blocked_by = (meta.blocked_by || '').split(',').map((value) => value.trim()).filter(Boolean);
  if (Object.hasOwn(meta, 'related')) {
    meta.related = (meta.related || '').split(',').map((value) => value.trim()).filter(Boolean);
  }
  return meta;
}

function run(args, input) {
  const result = spawnSync('kata', args, { cwd: workspace, input, encoding: 'utf8' });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(`kata ${args.join(' ')} failed:\n${result.stderr || ''}${result.stdout || ''}`);
  }
  return result.stdout.trim();
}

function listIssues() {
  // Closed tasks remain part of the canonical plan; include them so idempotent
  // sync finds existing issues instead of trying to recreate their keys.
  const result = JSON.parse(run(['list', '--project', project, '--limit', '0', '--status', 'all', '--json']));
  return result.issues || [];
}

const tasks = readdirSync(taskDirectory)
  .filter((file) => file.endsWith('.md'))
  .sort()
  .map(parseTask);
const contracts = tasks.filter((task) => task.kind === 'contract');
const milestones = tasks.filter((task) => task.kind === 'milestone');
const implementation = tasks.filter((task) => task.kind === 'task');
if (contracts.length !== 1) throw new Error(`Expected exactly one product contract; found ${contracts.length}.`);

const byKey = new Map(tasks.map((task) => [task.key, task]));
if (byKey.size !== tasks.length) throw new Error('Task keys must be unique.');
for (const task of tasks) {
  if (!task.key || !task.title) throw new Error(`Missing key/title in ${task.file}.`);
  for (const dependency of task.blocked_by) {
    if (!byKey.has(dependency)) throw new Error(`${task.key} references unknown blocker ${dependency}.`);
  }
  for (const related of task.related || []) {
    if (!byKey.has(related)) throw new Error(`${task.key} references unknown related issue ${related}.`);
  }
  if (task.kind === 'task' && task.parent && !byKey.has(task.parent)) throw new Error(`${task.key} references unknown parent ${task.parent}.`);
}

const kataConfig = resolve(workspace, '.kata.toml');
const boundProject = existsSync(kataConfig)
  ? readFileSync(kataConfig, 'utf8').match(/^name\s*=\s*"([^"]+)"/m)?.[1]
  : undefined;
if (boundProject !== project) run(['init', '--project', project, '--replace', '--reassign']);

const refs = new Map();
function issueRef(title) {
  const issue = listIssues().find((item) => item.title === title);
  if (!issue?.short_id) throw new Error(`Could not resolve Kata issue: ${title}`);
  return issue.short_id;
}

function create(task) {
  const existing = listIssues().find((item) => item.title === task.title);
  if (existing?.short_id) {
    refs.set(task.key, existing.short_id);
    process.stdout.write(`${task.key}\t${existing.short_id}\t${task.title} (existing)\n`);
    return;
  }
  const args = [
    'create', '--project', project, task.title, '--body-stdin',
    '--priority', String(task.priority),
    '--idempotency-key', `${idempotencyPrefix}-${task.key}`,
    '--json',
  ];
  if (task.kind === 'milestone') args.push('--parent', refs.get('contract'));
  if (task.kind === 'task' && task.parent) args.push('--parent', refs.get(task.parent));
  for (const label of task.labels) args.push('--label', label);
  run(args, task.body);
  const ref = issueRef(task.title);
  refs.set(task.key, ref);
  process.stdout.write(`${task.key}\t${ref}\t${task.title}\n`);
}

for (const task of contracts) create(task);
for (const task of milestones) create(task);
for (const task of implementation) create(task);

// Add dependency edges after all issues exist so the catalog may express any
// acyclic dependency order without relying on filename order.
let currentIssues = listIssues();
for (const task of tasks) {
  let issue = currentIssues.find((item) => item.title === task.title);
  if (!issue) throw new Error(`Issue disappeared after creation: ${task.title}`);
  if (issue.priority !== task.priority) run(['edit', issue.short_id, '--priority', String(task.priority)]);
  if ((issue.body || '').trim() !== task.body.trim()) run(['edit', issue.short_id, '--body', task.body]);
  const currentLabels = new Set(issue.labels || []);
  for (const label of task.labels) {
    if (!currentLabels.has(label)) run(['label', 'add', issue.short_id, label]);
  }
  currentIssues = listIssues();
  issue = currentIssues.find((item) => item.short_id === issue.short_id);
  const existing = new Set((issue.blocked_by || []).map((item) => item.short_id));
  const desired = new Set(task.blocked_by.map((dependency) => refs.get(dependency)));
  for (const blockerRef of existing) {
    if (!desired.has(blockerRef)) run(['edit', issue.short_id, '--remove-blocked-by', blockerRef]);
  }
  for (const dependency of task.blocked_by) {
    const blockerRef = refs.get(dependency);
    if (!blockerRef) throw new Error(`Missing created blocker ${dependency} for ${task.key}.`);
    if (!existing.has(blockerRef)) run(['edit', issue.short_id, '--blocked-by', blockerRef]);
  }
  if (task.parent) {
    const parentRef = task.kind === 'milestone' ? refs.get('contract') : refs.get(task.parent);
    if (issue.parent?.short_id !== parentRef) run(['edit', issue.short_id, '--parent', parentRef]);
  }
  if (Object.hasOwn(task, 'related')) {
    currentIssues = listIssues();
    issue = currentIssues.find((item) => item.short_id === issue.short_id);
    const existing = new Set((issue.related || []).map((item) => item.short_id));
    const desired = new Set(task.related.map((key) => refs.get(key)));
    for (const relatedRef of existing) {
      if (!desired.has(relatedRef)) run(['edit', issue.short_id, '--remove-related', relatedRef]);
    }
    for (const key of task.related) {
      const relatedRef = refs.get(key);
      if (!relatedRef) throw new Error(`Missing related issue ${key} for ${task.key}.`);
      if (!existing.has(relatedRef)) run(['edit', issue.short_id, '--related', relatedRef]);
    }
  }
  currentIssues = listIssues();
}

currentIssues = listIssues();
const refByTitle = new Map(currentIssues.map((issue) => [issue.title, issue.short_id]));
const lines = [
  '# Kata backlog — Code Atlas rebuild',
  '',
  `Workspace project: **${project}**. Product plan: [PRODUCT_BLUEPRINT.md](PRODUCT_BLUEPRINT.md).`,
  `Technical plan: [TECHNICAL_PLAN.md](TECHNICAL_PLAN.md). Generated on ${new Date().toISOString().slice(0, 10)}.`,
  'Kata issue bodies are synchronized from `docs/kata-tasks/*.md`; each issue states its user outcome, interaction contract, acceptance criteria, exclusions, and closure evidence.',
  '',
  '## Product contract',
  '',
  '| Ref | Issue | Definition |',
  '|---|---|---|',
];
for (const task of contracts) lines.push(`| ${refByTitle.get(task.title)} | ${task.title} | [${task.file}](kata-tasks/${task.file}) |`);
lines.push('', '## Milestones', '', '| Key | Ref | Milestone | Gate definition |', '|---|---|---|---|');
for (const task of milestones) lines.push(`| ${task.key.toUpperCase()} | ${refByTitle.get(task.title)} | ${task.title} | [${task.file}](kata-tasks/${task.file}) |`);
lines.push('', '## Implementation tasks', '', '| Phase | Ref | Task | Blocked by | Definition |', '|---|---|---|---|---|');
for (const task of implementation) {
  const blockedBy = task.blocked_by.map((key) => {
    const blocker = byKey.get(key);
    return `${refByTitle.get(blocker.title)} ${blocker.title}`;
  }).join('<br>') || '—';
  const phase = task.phase || task.parent?.replace(/^m/, 'M') || 'Cross-cutting';
  lines.push(`| ${phase} | ${refByTitle.get(task.title)} | ${task.title} | ${blockedBy} | [${task.file}](kata-tasks/${task.file}) |`);
}
lines.push(
  '',
  '## Build order',
  '',
  '1. Design the screen/state/navigation contract, then build the empty app shell.',
  '2. Capture local snapshots, account for every path, index supported relationships, and connect Atlas to exact Code Lens source.',
  '3. Make file-by-file coverage visible; add the exact-context Gemini boundary, structured chapters, generation, human review, and quality gate.',
  '4. Add manual section playback, typed linked visuals, and optional natural-speech narration.',
  '5. Add section-level freshness, selective revision, history, and offline export.',
  '6. Release the dependable Mac app. Build local PR explanation later; it does not block the first release.',
  '7. Add selectable direct model APIs and coding-agent runtimes through maintained libraries, ACP where supported, and each agent’s documented integration surface.',
  '',
);
writeFileSync(resolve(workspace, 'docs/KATA_BACKLOG.md'), lines.join('\n'));
writeFileSync(resolve(taskDirectory, 'issue-refs.json'), `${JSON.stringify(Object.fromEntries(refs), null, 2)}\n`);
process.stdout.write(`\nSynchronized ${refs.size} Kata issues in ${project}.\n`);
