import { useId, useLayoutEffect, useMemo, useRef, useState, type RefObject } from 'react';
import { ChevronDown, ChevronRight, FileCode2, Folder } from 'lucide-react';
import { atlasTreeKeyboardAction, virtualRowWindow } from '../lib/atlas';
import { buildExplorerTree, explorerActivePath, explorerAncestors, explorerRows, explorerTypeahead, matchingExplorerPaths, type ExplorerRow } from './explorerTree';

const ROW_HEIGHT = 28;
const OVERSCAN = 6;

/** One keyboard target and a bounded DOM, even when a captured folder is large.
 * Arrow/typeahead navigation changes focus; Enter/Space opens captured source. */
export function WorkspaceExplorer({ paths, query, selectedPath, expanded, onExpandedChange, onOpen, treeRef }: {
  paths: readonly string[];
  query: string;
  selectedPath: string | null;
  expanded: ReadonlySet<string>;
  onExpandedChange: (next: Set<string>) => void;
  onOpen: (path: string) => void;
  treeRef: RefObject<HTMLDivElement | null>;
}) {
  const id = useId();
  const tree = useMemo(() => buildExplorerTree(paths), [paths]);
  const matches = useMemo(() => matchingExplorerPaths(tree, query), [tree, query]);
  const [collapsedMatches, setCollapsedMatches] = useState<Set<string>>(() => new Set());
  const rows = useMemo(() => explorerRows(tree, expanded, matches, collapsedMatches), [tree, expanded, matches, collapsedMatches]);
  const [desiredPath, setDesiredPath] = useState<string | null>(selectedPath);
  const activePath = explorerActivePath(rows, desiredPath, selectedPath);
  const activeIndex = rows.findIndex(row => row.path === activePath);
  const [viewport, setViewport] = useState({ top: 0, height: 400 });
  const reveal = useRef(true);
  const typeahead = useRef({ text: '', at: -Infinity });

  const moveTo = (index: number) => {
    const row = rows[index];
    const element = treeRef.current;
    if (!row || !element) return;
    setDesiredPath(row.path);
    const top = index * ROW_HEIGHT;
    if (top < element.scrollTop) element.scrollTop = top;
    else if (top + ROW_HEIGHT > element.scrollTop + element.clientHeight) element.scrollTop = top + ROW_HEIGHT - element.clientHeight;
    setViewport({ top: element.scrollTop, height: element.clientHeight });
  };
  useLayoutEffect(() => {
    const element = treeRef.current;
    if (!element) return;
    const update = () => setViewport({ top: element.scrollTop, height: element.clientHeight });
    update();
    const observer = new ResizeObserver(update);
    observer.observe(element);
    return () => observer.disconnect();
  }, [treeRef]);
  useLayoutEffect(() => {
    setDesiredPath(selectedPath);
    reveal.current = true;
    if (!selectedPath) return;
    const parents = explorerAncestors(selectedPath);
    if (parents.some(path => !expanded.has(path))) onExpandedChange(new Set([...expanded, ...parents]));
  }, [selectedPath, tree]);
  useLayoutEffect(() => {
    setCollapsedMatches(new Set());
    setDesiredPath(selectedPath);
    reveal.current = true;
    typeahead.current = { text: '', at: -Infinity };
  }, [query]);
  useLayoutEffect(() => {
    if (!reveal.current || activeIndex < 0) return;
    // Wait one render for ancestors of a newly selected file to expand.
    if (!matches && selectedPath && activePath !== selectedPath && tree.byPath.has(selectedPath)) return;
    moveTo(activeIndex);
    reveal.current = false;
  }, [rows, activeIndex, selectedPath, matches]);

  const toggle = (row: ExplorerRow) => {
    if (matches) {
      const next = new Set(collapsedMatches);
      if (row.expanded) next.add(row.path); else next.delete(row.path);
      setCollapsedMatches(next);
    } else {
      const next = new Set(expanded);
      if (row.expanded) next.delete(row.path); else next.add(row.path);
      onExpandedChange(next);
    }
    setDesiredPath(row.path);
  };
  const activate = (row: ExplorerRow) => row.hasChildren ? toggle(row) : onOpen(row.path);
  const window = virtualRowWindow(rows.length, viewport.top, viewport.height, ROW_HEIGHT, OVERSCAN);
  const indexes = Array.from({ length: window.endIndex - window.startIndex }, (_, i) => i + window.startIndex);
  // The active descendant must exist for accessibility even after pointer scrolling.
  if (activeIndex >= 0 && !indexes.includes(activeIndex)) indexes.push(activeIndex);
  const rowId = (index: number) => `${id}-file-${index}`;

  return <div ref={treeRef} className="dw-file-list" role="tree" aria-label="Captured files" tabIndex={rows.length ? 0 : -1}
    aria-activedescendant={activeIndex >= 0 ? rowId(activeIndex) : undefined}
    onScroll={event => setViewport({ top: event.currentTarget.scrollTop, height: event.currentTarget.clientHeight })}
    onKeyDown={event => {
      if (event.metaKey || event.ctrlKey || event.altKey || event.nativeEvent.isComposing) return;
      const action = atlasTreeKeyboardAction(rows, activeIndex, event.key);
      if (action.kind !== 'none') {
        event.preventDefault();
        typeahead.current = { text: '', at: -Infinity };
        if (action.kind === 'focus') moveTo(action.index);
        else if (action.kind === 'expand' || action.kind === 'collapse') toggle(rows[activeIndex]);
        else if (action.kind === 'activate' || action.kind === 'select') activate(rows[activeIndex]);
      } else if ([...event.key].length === 1) {
        const found = explorerTypeahead(rows, activePath, event.key, typeahead.current, performance.now());
        typeahead.current = found.state;
        if (found.path) { event.preventDefault(); moveTo(rows.findIndex(row => row.path === found.path)); }
      }
    }}>
    {rows.length === 0 ? <p className="dw-file-empty" role="status">{query ? 'No matching files.' : 'No captured text files.'}</p>
      : <div className="dw-file-rows" style={{ height: rows.length * ROW_HEIGHT }}>{indexes.map(index => {
        const row = rows[index];
        return <div key={row.path} id={rowId(index)} role="treeitem" aria-label={`${row.path}, ${row.hasChildren ? 'folder' : 'file'}`}
          aria-level={row.depth + 1} aria-posinset={row.positionInSet} aria-setsize={row.setSize}
          aria-selected={!row.hasChildren && row.path === selectedPath} aria-expanded={row.hasChildren ? row.expanded : undefined}
          className={`dw-file${row.hasChildren ? ' dw-folder' : ''}${!row.hasChildren && row.path === selectedPath ? ' active' : ''}${index === activeIndex ? ' dw-file-focused' : ''}`}
          style={{ top: index * ROW_HEIGHT, paddingLeft: 8 + row.depth * 12 }} title={row.path} draggable={!row.hasChildren}
          onDragStart={event => { if (row.hasChildren) { event.preventDefault(); return; } event.dataTransfer.setData('application/x-code-atlas-context', JSON.stringify({ path: row.path })); event.dataTransfer.effectAllowed = 'copy'; }}
          onClick={() => { setDesiredPath(row.path); treeRef.current?.focus({ preventScroll: true }); activate(row); }}>
          {row.hasChildren ? <>{row.expanded ? <ChevronDown size={12} aria-hidden="true" /> : <ChevronRight size={12} aria-hidden="true" />}<Folder size={14} aria-hidden="true" /></> : <><span className="dw-file-indent" aria-hidden="true" /><FileCode2 size={14} aria-hidden="true" /></>}
          <span>{row.name}</span>
        </div>;
      })}</div>}
  </div>;
}
