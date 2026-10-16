import { useEffect, useRef, useState } from 'react';
import { Button, IconButton, Tooltip } from '@radix-ui/themes';
import { Code2, MessageSquarePlus, X } from 'lucide-react';
import { EditorState } from '@codemirror/state';
import { EditorView, drawSelection, highlightActiveLine, lineNumbers, keymap } from '@codemirror/view';
import { defaultKeymap } from '@codemirror/commands';
import { javascript } from '@codemirror/lang-javascript';
import { HighlightStyle, syntaxHighlighting } from '@codemirror/language';
import { tags } from '@lezer/highlight';
import { exampleFiles, type CodeContext } from './example';

const colors = HighlightStyle.define([
  { tag: tags.keyword, color: 'var(--code-keyword)' },
  { tag: [tags.function(tags.variableName), tags.function(tags.propertyName)], color: 'var(--code-function)' },
  { tag: tags.string, color: 'var(--code-string)' },
  { tag: [tags.typeName, tags.className], color: 'var(--code-type)' },
  { tag: tags.number, color: 'var(--code-number)' },
]);

export function CodePane({ path, theme, reveal, onExplain, onAttach, onClose }: {
  path: string; theme: 'dark' | 'light'; reveal: CodeContext | null;
  onExplain: (context: CodeContext) => void; onAttach: (context: CodeContext) => void; onClose: () => void;
}) {
  const host = useRef<HTMLDivElement>(null);
  const editor = useRef<EditorView | null>(null);
  const [selection, setSelection] = useState<CodeContext | null>(null);
  const callback = useRef(onExplain); callback.current = onExplain;
  useEffect(() => {
    if (!host.current) return;
    const view = new EditorView({ parent: host.current, state: EditorState.create({
      doc: exampleFiles[path], extensions: [
        EditorState.readOnly.of(true), lineNumbers(), drawSelection(), highlightActiveLine(), EditorView.lineWrapping,
        javascript({ typescript: path.endsWith('.ts') }), syntaxHighlighting(colors),
        EditorView.theme({ '&': { height: '100%', backgroundColor: 'var(--editor-bg)', color: 'var(--text)' }, '.cm-scroller': { fontFamily: 'var(--mono)', fontSize: '13px', lineHeight: '1.9' }, '.cm-content': { padding: '16px 0' }, '.cm-gutters': { backgroundColor: 'var(--editor-bg)', color: 'var(--dim)', borderRight: 'none', paddingRight: '8px' }, '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'var(--active-line)' }, '.cm-selectionBackground, &.cm-focused .cm-selectionBackground': { backgroundColor: 'var(--selection)' }, '&.cm-focused': { outline: 'none' } }, { dark: theme === 'dark' }),
        EditorView.contentAttributes.of({ 'aria-label': `Source code, ${path}` }),
        keymap.of([{ key: 'Mod-i', run: (v) => { const r = v.state.selection.main; if (r.empty) return false; callback.current({ path, from: v.state.doc.lineAt(r.from).number, to: v.state.doc.lineAt(r.to).number }); return true; } }, ...defaultKeymap]),
        EditorView.updateListener.of((update) => {
          if (!update.selectionSet) return;
          const range = update.state.selection.main;
          setSelection(range.empty ? null : { path, from: update.state.doc.lineAt(range.from).number, to: update.state.doc.lineAt(range.to).number });
        }),
      ],
    }) });
    editor.current = view; setSelection(null);
    return () => { editor.current = null; view.destroy(); };
  }, [path, theme]);
  useEffect(() => {
    const view = editor.current;
    if (!view || !reveal || reveal.path !== path) return;
    const from = view.state.doc.line(Math.min(reveal.from ?? 1, view.state.doc.lines)).from;
    const to = view.state.doc.line(Math.min(reveal.to ?? reveal.from ?? 1, view.state.doc.lines)).to;
    view.dispatch({ selection: { anchor: from, head: to }, effects: EditorView.scrollIntoView(from, { y: 'center' }) });
  }, [reveal, path, theme]);
  return <section className="preview-code-pane" aria-label="Code">
    <div className="preview-editor-tab"><Code2 size={14} /><span>{path.split('/').at(-1)}</span><Tooltip content="Close code pane"><IconButton size="1" variant="ghost" aria-label="Close code pane" onClick={onClose}><X size={14} /></IconButton></Tooltip></div>
    <div className="preview-code-path">{path.split('/').join('  /  ')}</div>
    <div ref={host} className="preview-editor" />
    {selection && <div className="preview-selection-tools" onMouseDown={(event) => event.preventDefault()}>
      <Button size="1" onClick={() => onExplain(selection)}><MessageSquarePlus size={13} />Explain this code</Button>
      <Tooltip content="Add selected lines to your question"><IconButton size="1" variant="soft" aria-label="Add selection to chat" onClick={() => onAttach(selection)}>+</IconButton></Tooltip>
    </div>}
    <footer className="preview-editor-footer"><span>TypeScript</span><span>{selection ? `Lines ${selection.from}–${selection.to}` : 'Select code to ask about it'}</span></footer>
  </section>;
}
