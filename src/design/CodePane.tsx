import { useEffect, useRef, useState } from 'react';
import { Button, IconButton, Tooltip } from '@radix-ui/themes';
import { Code2, MessageSquarePlus, X } from 'lucide-react';
import { EditorState } from '@codemirror/state';
import { EditorView, drawSelection, highlightActiveLine, lineNumbers, keymap } from '@codemirror/view';
import { defaultKeymap } from '@codemirror/commands';
import { javascript } from '@codemirror/lang-javascript';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import type { Context as CodeContext } from '../workspace/model';
import { codeSelectionContext } from '../lib/codeSelection';
import { languageForPath } from '../lib/inventory';

const sourceLanguage = (path: string) => {
  const language = languageForPath(path);
  return language.startsWith('.') || language === 'No extension' ? 'Plain text' : language;
};

const colors = HighlightStyle.define([
  { tag: tags.keyword, color: 'var(--code-keyword)' },
  { tag: [tags.function(tags.variableName), tags.function(tags.propertyName)], color: 'var(--code-function)' },
  { tag: tags.string, color: 'var(--code-string)' },
  { tag: [tags.typeName, tags.className], color: 'var(--code-type)' },
  { tag: tags.number, color: 'var(--code-number)' },
]);

export function CodePane({ path, contents, sha256, theme, reveal, canExplain = true, onExplain, onAttach, onClose }: {
  path: string; contents: string; sha256?: string; theme: 'dark' | 'light'; reveal: CodeContext | null;
  canExplain?: boolean;
  onExplain: (context: CodeContext) => void; onAttach: (context: CodeContext) => void; onClose: () => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<EditorView | null>(null);
  const [selection, setSelection] = useState<CodeContext | null>(null);
  const callback = useRef(onExplain); callback.current = onExplain;
  const canExplainNow = useRef(canExplain); canExplainNow.current = canExplain;
  useEffect(() => {
    if (!host.current) return;
    const view = new EditorView({ parent: host.current, state: EditorState.create({
      doc: contents, extensions: [
        EditorState.readOnly.of(true), lineNumbers(), drawSelection(), highlightActiveLine(),
        ...(/\.(?:[cm]?[jt]s|[jt]sx)$/i.test(path) ? [javascript({ typescript: /\.(?:[cm]?ts|tsx)$/i.test(path), jsx: /\.[jt]sx$/i.test(path) })] : []), syntaxHighlighting(colors),
        EditorView.theme({ '&': { height: '100%', backgroundColor: 'var(--editor-bg)', color: 'var(--text)' }, '.cm-scroller': { fontFamily: 'var(--mono)', fontSize: '13px', lineHeight: '1.9' }, '.cm-content': { padding: '16px 0' }, '.cm-gutters': { backgroundColor: 'var(--editor-bg)', color: 'var(--dim)', borderRight: 'none', paddingRight: '8px' }, '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'var(--active-line)' }, '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': { backgroundColor: 'var(--selection)' }, '&.cm-focused': { outline: 'none' } }, { dark: theme === 'dark' }),
        EditorView.contentAttributes.of({ 'aria-label': `Source code, ${path}` }),
        keymap.of([{ key: 'Mod-i', run: (v) => { const r = v.state.selection.main; if (!canExplainNow.current || r.empty) return false; callback.current(codeSelectionContext(path, sha256, v.state.doc.lineAt(r.from).number, v.state.doc.lineAt(Math.max(r.from,r.to-1)).number)); return true; } }, ...defaultKeymap]),
        EditorView.updateListener.of((update) => {
          if (!update.selectionSet) return;
          const range = update.state.selection.main;
          setSelection(range.empty ? null : codeSelectionContext(path, sha256, update.state.doc.lineAt(range.from).number, update.state.doc.lineAt(Math.max(range.from,range.to-1)).number));
        }),
      ],
    }) });
    editor.current = view; setSelection(null);
    return () => { editor.current = null; view.destroy(); };
  }, [path, contents, sha256, theme]);
  useEffect(() => {
    const view = editor.current;
    if (!view || !reveal || reveal.path !== path) return;
    if (reveal.from !== undefined && (!Number.isInteger(reveal.from) || reveal.from < 1 || (reveal.to !== undefined && (!Number.isInteger(reveal.to) || reveal.to < reveal.from)))) return;
    if (!reveal.from) { view.dispatch({ selection: { anchor: 0 } }); return; }
    const from = view.state.doc.line(Math.min(reveal.from ?? 1, view.state.doc.lines)).from;
    const to = view.state.doc.line(Math.min(reveal.to ?? reveal.from ?? 1, view.state.doc.lines)).to;
    view.dispatch({ selection: { anchor: from, head: to }, effects: EditorView.scrollIntoView(from, { y: 'start', yMargin: 24 }) });
  }, [reveal, path, contents, theme]);
  useEffect(() => {
    const view = editor.current;
    if (!view || !reveal || reveal.path !== path || !reveal.from) return;
    const line = reveal.from;
    let frame = 0;
    const restoreCitation = () => {
      cancelAnimationFrame(frame);
      const position = view.state.doc.line(Math.min(line, view.state.doc.lines)).from;
      frame = requestAnimationFrame(() => {
        if (view.dom.isConnected) view.dispatch({ effects: EditorView.scrollIntoView(position, { y: 'start', yMargin: 24 }) });
      });
    };
    const observer = new MutationObserver(restoreCitation);
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-app-zoom'] });
    return () => { observer.disconnect(); cancelAnimationFrame(frame); };
  }, [reveal, path, contents, theme]);
  return <section className="preview-code-pane" aria-label="Code" data-focus-key="workspace-source" tabIndex={-1}>
    <div className="preview-editor-tab"><Code2 size={14} /><span>{path.split('/').at(-1)}</span><Tooltip content="Close code pane"><IconButton size="1" variant="ghost" aria-label="Close code pane" onClick={onClose}><X size={14} /></IconButton></Tooltip></div>
    <div className="preview-code-path">{path.split('/').join('  /  ')}</div>
    <div ref={host} className="preview-editor" />
    {selection && <div className="preview-selection-tools" onMouseDown={(event) => event.preventDefault()}>
      <Button size="1" disabled={!canExplain} title={canExplain ? undefined : 'The saved conversation is unavailable.'} onClick={() => onExplain(selection)}><MessageSquarePlus size={13} />Explain this code</Button>
      <Tooltip content="Add selected lines to your question"><IconButton size="1" variant="soft" disabled={!canExplain} aria-label="Add selection to chat" onClick={() => onAttach(selection)}>+</IconButton></Tooltip>
    </div>}
    <footer className="preview-editor-footer"><span>{sourceLanguage(path)}</span><span>{selection ? `Lines ${selection.from}–${selection.to}` : 'Select code to ask about it'}</span></footer>
  </section>;
}
