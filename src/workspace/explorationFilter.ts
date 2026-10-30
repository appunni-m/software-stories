import type { Exploration } from './model.ts';

export interface ExplorationFilterResult {
  matchedIds: ReadonlySet<string>;
  visibleIds: ReadonlySet<string>;
  expandedIds: ReadonlySet<string>;
}

/** Keep matching explorations and their ancestors visible in the history tree. */
export function filterExplorations(
  explorations: readonly Pick<Exploration, 'id' | 'parentId' | 'title' | 'file'>[],
  query: string,
  rootTitle = '',
): ExplorationFilterResult {
  const term = query.trim().toLocaleLowerCase();
  const byId = new Map(explorations.map((exploration) => [exploration.id, exploration]));
  const matchedIds = new Set<string>();
  const visibleIds = new Set<string>();
  const expandedIds = new Set<string>();

  if (!term) {
    for (const exploration of explorations) visibleIds.add(exploration.id);
    return { matchedIds, visibleIds, expandedIds };
  }

  for (const exploration of explorations) {
    const title = exploration.id === 'root' && rootTitle ? rootTitle : exploration.title;
    const searchableText = `${title} ${exploration.file ?? ''}`.toLocaleLowerCase();
    if (!searchableText.includes(term)) continue;

    matchedIds.add(exploration.id);
    let current: typeof exploration | undefined = exploration;
    const visited = new Set<string>();
    while (current && !visited.has(current.id)) {
      visited.add(current.id);
      visibleIds.add(current.id);
      if (current.id !== exploration.id) expandedIds.add(current.id);
      current = current.parentId ? byId.get(current.parentId) : undefined;
    }
  }

  return { matchedIds, visibleIds, expandedIds };
}
