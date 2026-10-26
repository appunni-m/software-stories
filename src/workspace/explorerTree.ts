import { rankSourcePaths } from './model';

/** A tree built once from the immutable captured paths, independent of the DOM. */
export interface ExplorerEntry {
  path: string;
  name: string;
  folder: boolean;
  children: ExplorerEntry[];
}
export interface ExplorerTree {
  roots: ExplorerEntry[];
  byPath: ReadonlyMap<string, ExplorerEntry>;
}
export interface ExplorerRow {
  path: string;
  name: string;
  depth: number;
  hasChildren: boolean;
  expanded: boolean;
  positionInSet: number;
  setSize: number;
}

export function explorerAncestors(path: string): string[] {
  const parents: string[] = [];
  for (let end = path.indexOf('/'); end >= 0; end = path.indexOf('/', end + 1)) parents.push(path.slice(0, end));
  return parents;
}

export function buildExplorerTree(paths: readonly string[]): ExplorerTree {
  const roots: ExplorerEntry[] = [];
  const byPath = new Map<string, ExplorerEntry>();
  for (const path of paths) {
    let children = roots;
    const parts = path.split('/');
    let prefix = '';
    for (let i = 0; i < parts.length; i++) {
      prefix += `${prefix ? '/' : ''}${parts[i]}`;
      let entry = byPath.get(prefix);
      if (!entry) {
        entry = { path: prefix, name: parts[i], folder: i < parts.length - 1, children: [] };
        byPath.set(prefix, entry);
        children.push(entry);
      }
      children = entry.children;
    }
  }
  // Keep the explorer useful on large repositories: directories containing
  // likely implementation entry points appear before docs, tests, and agent
  // configuration. Every captured path remains in the tree and search still
  // sees the complete inventory.
  const sourceOrder = new Map(rankSourcePaths(paths.slice()).map((path, index) => [path, index]));
  const branchOrder = new Map<string, number>();
  for (const [path, order] of sourceOrder) {
    let end = path.indexOf('/');
    while (end >= 0) {
      const parent = path.slice(0, end);
      const previous = branchOrder.get(parent);
      if (previous === undefined || order < previous) branchOrder.set(parent, order);
      end = path.indexOf('/', end + 1);
    }
    branchOrder.set(path, order);
  }
  const compare = (a: ExplorerEntry, b: ExplorerEntry) => Number(b.folder) - Number(a.folder)
    || (branchOrder.get(a.path) ?? Number.MAX_SAFE_INTEGER) - (branchOrder.get(b.path) ?? Number.MAX_SAFE_INTEGER)
    || (a.name < b.name ? -1 : a.name > b.name ? 1 : 0)
    || (a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  roots.sort(compare);
  for (const entry of byPath.values()) entry.children.sort(compare);
  return { roots, byPath };
}

/** Filtering retains only matching files and their ancestors; it never changes
 * the reader's ordinary folder expansion preferences. */
export function matchingExplorerPaths(tree: ExplorerTree, query: string): ReadonlySet<string> | null {
  const term = query.toLocaleLowerCase();
  if (!term) return null;
  const matches = new Set<string>();
  for (const entry of tree.byPath.values()) {
    if (entry.folder || !entry.path.toLocaleLowerCase().includes(term)) continue;
    matches.add(entry.path);
    for (const ancestor of explorerAncestors(entry.path)) matches.add(ancestor);
  }
  return matches;
}

export function explorerRows(tree: ExplorerTree, expanded: ReadonlySet<string>, matches: ReadonlySet<string> | null,
  collapsedMatches: ReadonlySet<string> = new Set()): ExplorerRow[] {
  const rows: ExplorerRow[] = [];
  const retained = (entries: ExplorerEntry[]) => matches ? entries.filter(entry => matches.has(entry.path)) : entries;
  const roots = retained(tree.roots);
  const pending = roots.map((entry, i) => ({ entry, depth: 0, positionInSet: i + 1, setSize: roots.length })).reverse();
  while (pending.length) {
    const { entry, ...position } = pending.pop()!;
    const open = entry.folder && (matches ? !collapsedMatches.has(entry.path) : expanded.has(entry.path));
    rows.push({ ...position, path: entry.path, name: entry.name, hasChildren: entry.folder, expanded: open });
    if (open) {
      const children = retained(entry.children);
      for (let i = children.length - 1; i >= 0; i--) pending.push({ entry: children[i], depth: position.depth + 1, positionInSet: i + 1, setSize: children.length });
    }
  }
  return rows;
}

/** Collapsing/filtering a branch keeps the active descendant's nearest visible
 * parent as the keyboard target, without changing the open source file. */
export function explorerActivePath(rows: readonly ExplorerRow[], desired: string | null, selected: string | null): string | null {
  const visible = new Set(rows.map(row => row.path));
  if (desired && visible.has(desired)) return desired;
  if (desired) for (const parent of explorerAncestors(desired).reverse()) if (visible.has(parent)) return parent;
  return selected && visible.has(selected) ? selected : rows[0]?.path ?? null;
}

export interface ExplorerTypeahead { text: string; at: number }
export function explorerTypeahead(rows: readonly ExplorerRow[], active: string | null, key: string,
  previous: ExplorerTypeahead, now: number): { path: string | null; state: ExplorerTypeahead } {
  const text = now - previous.at > 700 ? key : previous.text + key;
  const repeated = [...text.toLocaleLowerCase()].every(character => character === key.toLocaleLowerCase());
  const prefix = (repeated ? key : text).toLocaleLowerCase();
  const start = Math.max(0, rows.findIndex(row => row.path === active));
  for (let offset = prefix.length === 1 ? 1 : 0; offset <= rows.length; offset++) {
    const row = rows[(start + offset) % rows.length];
    if (row?.name.toLocaleLowerCase().startsWith(prefix)) return { path: row.path, state: { text, at: now } };
  }
  return { path: null, state: { text, at: now } };
}
