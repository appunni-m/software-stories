import { useEffect, useRef, useState } from 'react';
import { Badge, Button, Dialog, Flex, IconButton, Tabs, TextField, Theme, Tooltip } from '@radix-ui/themes';
import { Group, Panel, Separator } from 'react-resizable-panels';
import { Background, Controls, MarkerType, ReactFlow, type Node } from '@xyflow/react';
import { ArrowDown, ArrowLeft, ArrowRight, ArrowUp, BookOpen, Check, ChevronDown, ChevronRight, Code2, FileCode2, Files, GitBranch, History, MessageSquare, Moon, Network, Paperclip, Pause, Play, Search, Sun, X } from 'lucide-react';
import { NotificationCenter, type WorkspaceNotification } from '../NotificationCenter';
import { CodePane } from './CodePane';
import { exampleFiles, explorationPath, findExampleCode, readPreview, type CodeContext, type Exploration, type ExplorationView } from './example';

const nodeLabels = ['Receive the prices', 'Validate rate and apply tax', 'Validate prices and sum'];
const nodes: Node[] = nodeLabels.map((label, i) => ({
  id: String(i), position: { x: 0, y: i * 112 },
  data: { label: <><small>{['checkout', 'totalWithTax', 'subtotal'][i]}</small><strong>{label}</strong><span>{['Passes a 0.10 rate', 'Calls subtotal, then multiplies', 'Checks each price before adding'][i]}</span></> },
  style: { width: 290 },
  ariaLabel: label,
}));
const edges = nodes.slice(1).map((node, i) => ({ id: `e${i}`, source: String(i), target: node.id, markerEnd: { type: MarkerType.ArrowClosed }, label: i === 1 ? 'calls subtotal(prices)' : undefined }));
const subtotalNodes: Node[] = [
  { id: '0', position: { x: 108, y: 0 }, data: { label: <><small>subtotal · lines 1–4</small><strong>Check every price</strong><span>Finite and non-negative?</span></> }, style: { width: 205 } },
  { id: '1', position: { x: 0, y: 140 }, data: { label: <><small>line 3</small><strong>Reject bad input</strong><span>Throw a RangeError</span></> }, style: { width: 205 } },
  { id: '2', position: { x: 216, y: 140 }, data: { label: <><small>line 5</small><strong>Add from zero</strong><span>Return the subtotal</span></> }, style: { width: 205 } },
];
const subtotalEdges = [
  { id: 'bad', source: '0', target: '1', label: 'Invalid', markerEnd: { type: MarkerType.ArrowClosed } },
  { id: 'good', source: '0', target: '2', label: 'Valid', markerEnd: { type: MarkerType.ArrowClosed } },
];

function contextLabel(context: CodeContext) { return `${context.path.split('/').at(-1)}${context.from ? `:${context.from}${context.to !== context.from ? `–${context.to}` : ''}` : ''}`; }

export function WorkspacePreview() {
  const [state, setState] = useState(readPreview);
  const current = state.explorations.find((e) => e.id === state.currentId)!;
  const ancestors = explorationPath(state.explorations, current.id);
  const [query, setQuery] = useState('');
  const [attachments, setAttachments] = useState<CodeContext[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [searchOpen, setSearchOpen] = useState(false);
  const [composerFocused, setComposerFocused] = useState(false);
  const [composerResults, setComposerResults] = useState<ReturnType<typeof findExampleCode>>([]);
  const [fileFilter, setFileFilter] = useState('');
  const [closedFolders, setClosedFolders] = useState<string[]>([]);
  const [activity, setActivity] = useState<'files'|'changes'|'history'>('files');
  const [narrowPane, setNarrowPane] = useState<'explanation'|'source'|'explorer'>('explanation');
  const [codeOpen, setCodeOpen] = useState(true);
  const [reveal, setReveal] = useState<CodeContext | null>(null);
  const [slide, setSlide] = useState(0);
  const [playing, setPlaying] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const [notifications, setNotifications] = useState(['guide-ready']);
  const composer = useRef<HTMLTextAreaElement>(null);
  const scroll = useRef<HTMLDivElement>(null);
  const resultList = useRef<HTMLDivElement>(null);

  useEffect(() => { try { localStorage.setItem('code-atlas:workspace-design:v2', JSON.stringify(state)); } catch { /* Preview only. */ } }, [state]);
  useEffect(() => {
    const timer = window.setTimeout(() => setComposerResults(findExampleCode(query)), 150);
    return () => window.clearTimeout(timer);
  }, [query]);
  useEffect(() => { if (!playing) return; const timer = window.setTimeout(() => { setSlide((n) => Math.min(n + 1, 2)); setPlaying(false); }, 2500); return () => window.clearTimeout(timer); }, [playing, slide]);
  useEffect(() => { setPlaying(false); setSlide(0); scroll.current?.scrollTo({ top: 0 }); }, [current.id]);

  function openFile(context: CodeContext) {
    setState((s) => ({ ...s, file: context.path })); setCodeOpen(true); setNarrowPane('source'); setReveal(context); setSearchOpen(false); setComposerFocused(false);
  }
  function attach(context: CodeContext) {
    setAttachments((items) => items.some((i) => contextLabel(i) === contextLabel(context)) ? items : [...items, context]);
    composer.current?.focus();
  }
  function setView(view: ExplorationView) {
    scroll.current?.scrollTo({ top: 0 });
    setState((s) => ({ ...s, explorations: s.explorations.map((e) => e.id === s.currentId ? { ...e, view } : e) }));
  }
  function explore(question: string, view: 'guide' | 'flow' = current.view === 'flow' ? 'flow' : 'guide', context = attachments, originTurnId = 'main') {
    if (!question.trim()) { setView(view); composer.current?.focus(); return; }
    const kind = /negative|invalid|validat|reject|subtotal/i.test(question) ? 'validation' : /overflow|break|limit|safe/i.test(question) ? 'limits' : /tax|rate/i.test(question) ? 'tax' : 'overview';
    const existing = state.explorations.find((e) => e.parentId === current.id && (e.originTurnId ?? 'main') === originTurnId && e.question === question && JSON.stringify(e.context) === JSON.stringify(context));
    const next: Exploration = existing ?? { id: crypto.randomUUID(), parentId: current.id, originTurnId, question, view, context, kind };
    setState((s) => ({ ...s, currentId: next.id, explorations: existing ? s.explorations : [...s.explorations, next] }));
    setQuery(''); setAttachments([]); setComposerFocused(false); setSearchOpen(false);setNarrowPane('explanation');
  }
  function ask(question: string, view: 'guide' | 'flow' = current.view === 'flow' ? 'flow' : 'guide', context = attachments) {
    if (!question.trim()) { setView(view); composer.current?.focus(); return; }
    const turn = { id: crypto.randomUUID(), question, context };
    setState((s) => ({ ...s, explorations: s.explorations.map((e) => e.id === s.currentId ? { ...e, view, turns: [...(e.turns ?? []), turn] } : e) }));
    setQuery(''); setAttachments([]); setComposerFocused(false); setSearchOpen(false);setNarrowPane('explanation');
    window.requestAnimationFrame(() => scroll.current?.scrollTo({ top: scroll.current.scrollHeight }));
  }
  function explain(context: CodeContext) { ask(`Explain ${contextLabel(context)}`, current.view === 'flow' ? 'flow' : 'guide', [context]); }
  useEffect(() => {
    const listener = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'l') { event.preventDefault(); composer.current?.focus(); }
      if ((event.metaKey || event.ctrlKey) && (event.key.toLowerCase() === 'p' || (event.shiftKey && event.key.toLowerCase() === 'f'))) { event.preventDefault(); setSearchOpen(true); }
      if ((event.metaKey || event.ctrlKey) && ['1', '2', '3'].includes(event.key)) { event.preventDefault(); setView((['guide', 'flow', 'history'] as const)[Number(event.key) - 1]); }
      if (event.altKey && event.key === 'ArrowLeft' && current.parentId) { event.preventDefault(); setState((s) => ({ ...s, currentId: current.parentId! })); }
    };
    window.addEventListener('keydown', listener); return () => window.removeEventListener('keydown', listener);
  }, [current.id, current.parentId]);

  const updates: WorkspaceNotification[] = notifications.map((id) => ({ id, title: 'Your checkout guide is ready', message: 'Continue where you left off.', actions: [
    { label: 'Open guide', run: () => { setState((s) => ({ ...s, currentId: 'root' })); setView('guide'); setNotifications([]); } },
    { label: 'Dismiss', run: () => setNotifications([]) },
  ] }));
  const children = state.explorations.filter((e) => e.parentId === current.id);
  const mainChildren = children.filter((e) => !e.originTurnId || e.originTurnId === 'main');
  const renderTree = (parent: string | null, depth = 0): React.ReactNode => state.explorations.filter((e) => e.parentId === parent).map((e) => <li key={e.id}><button className={e.id === current.id ? 'active' : ''} style={{ paddingLeft: 7 + depth * 8 }} onClick={() => (setNarrowPane('explanation'),setState((s) => ({ ...s, currentId: e.id })))}><MessageSquare size={12} /><span>{e.id === 'root' ? 'Checkout' : e.question}</span><small>{1 + (e.turns?.length ?? 0)}</small></button><ul>{renderTree(e.id, depth + 1)}</ul></li>);
  const childLinks = (items: Exploration[]) => <div className="preview-inline-branches">{items.map((e) => <button key={e.id} onClick={() => setState((s) => ({ ...s, currentId:e.id }))}><span>↳</span><span>{e.question}</span><small>{1 + (e.turns?.length ?? 0)} answers</small><ChevronRight size={12}/></button>)}</div>;
  const results = findExampleCode(searchQuery);
  const nodeSource = (index: number): CodeContext => index === 0 ? { path: 'src/checkout.ts', from: 3, to: 5 } : index === 2 ? { path: 'src/cart.ts', from: 1, to: 6 } : { path: 'src/cart.ts', from: 8, to: 13 };
  const detailFlow = /subtotal|negative|invalid/i.test(current.question);

  return <Theme appearance={state.theme} accentColor="blue" grayColor="slate" radius="medium" scaling="100%" className={`workspace-preview ${state.theme}`}>
    <header className="preview-titlebar">
      <span className="preview-brand"><Code2 size={18} />Code Atlas</span><span className="preview-title-project">checkout-demo</span>
      <Button variant="ghost" color="gray" size="1" onClick={() => setSearchOpen(true)} className="preview-quick-open"><Search size={14} />Search code<kbd>⌘ P</kbd></Button>
      <div className="preview-title-actions"><Tooltip content="Connected to Codex"><span className="preview-connected"><span />Connected</span></Tooltip>
        <Tooltip content={`Use ${state.theme === 'dark' ? 'light' : 'dark'} appearance`}><IconButton variant="ghost" color="gray" size="1" aria-label="Change appearance" onClick={() => setState((s) => ({ ...s, theme: s.theme === 'dark' ? 'light' : 'dark' }))}>{state.theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}</IconButton></Tooltip>
        <NotificationCenter items={updates} />
      </div>
    </header>
    <div className={`preview-workspace-body preview-narrow-${narrowPane}`}><nav className="preview-activity" aria-label="Repository tools"><button aria-label="Explorer" aria-pressed={activity==='files'} onClick={()=>{setActivity('files');setNarrowPane('explorer');}}><Files size={21}/></button><button aria-label="Search" onClick={()=>setSearchOpen(true)}><Search size={21}/></button><button aria-label="Changes" aria-pressed={activity==='changes'} onClick={()=>{setActivity('changes');setNarrowPane('explorer');}}><GitBranch size={21}/></button><button aria-label="Explorations" aria-pressed={activity==='history'} onClick={()=>{setActivity('history');setNarrowPane('explorer');}}><History size={21}/></button></nav><Group orientation="horizontal" className="preview-body">
      <Panel id="explorer" defaultSize="205px" minSize="155px" maxSize="320px">
        <aside className="preview-explorer" aria-label="File explorer">
          <div className="preview-explorer-heading"><Files size={14} /><strong>{activity==='changes'?'Source control':activity==='history'?'Explorations':'Explorer'}</strong></div>{activity==='changes'?<div className="preview-sample-commits"><strong>Commit history</strong><p>Illustrative checkout history</p>{['Validate prices before calculating','Apply the tax rate at checkout','Add the subtotal function'].map((title,i)=><div key={title}><GitBranch size={14}/><span>{title}<small>{['a94f5c2','4d92b11','19bc7aa'][i]}</small></span></div>)}</div>:activity==='history'?<ul className="preview-exploration-tree">{renderTree(null)}</ul>:<>
          <TextField.Root size="1" value={fileFilter} onChange={(e) => setFileFilter(e.target.value)} placeholder="Find a file…" aria-label="Filter files"><TextField.Slot><Search size={13} /></TextField.Slot></TextField.Root>
          <div className="preview-repo-label"><ChevronDown size={13} />CHECKOUT-DEMO</div>
          <nav aria-label="Repository files" className="preview-file-list">{['src', 'tests', ''].map((folder) => <div key={folder}>
            {folder && <button className="preview-folder" aria-expanded={!closedFolders.includes(folder)} onClick={() => setClosedFolders((all) => all.includes(folder) ? all.filter((f) => f !== folder) : [...all, folder])}>{closedFolders.includes(folder) ? <ChevronRight size={13} /> : <ChevronDown size={13} />}{folder}</button>}
            {!closedFolders.includes(folder) && Object.keys(exampleFiles).filter((p) => (folder ? p.startsWith(`${folder}/`) : !p.includes('/')) && p.toLowerCase().includes(fileFilter.toLowerCase())).map((path) => <button key={path} className={`preview-file ${state.file === path && codeOpen ? 'active' : ''}`} style={{ paddingLeft: folder ? 29 : 14 }} draggable
              onDragStart={(e) => { e.dataTransfer.setData('application/x-code-atlas-context', JSON.stringify({ path })); e.dataTransfer.effectAllowed = 'copy'; }}
              onClick={() => openFile({ path })} title={`${path} · drag into chat`}><FileCode2 size={14} /><span>{path.split('/').at(-1)}</span></button>)}
          </div>)}</nav>
          <div className="preview-explorer-bottom"><History size={13} /><strong>Explorations</strong></div>
          <nav className="preview-recent" aria-label="Explorations"><ul className="preview-exploration-tree">{renderTree(null)}</ul></nav></>}
        </aside>
      </Panel>
      <Separator className="preview-separator" aria-label="Resize explorer" />
      <Panel id="workspace" minSize="320px">
        <main className="preview-main">
          <nav className="preview-breadcrumbs" aria-label="Exploration path">
            <Tooltip content="Back one exploration · ⌥ ←"><IconButton size="1" color="gray" variant="ghost" disabled={!current.parentId} aria-label="Back one exploration" onClick={() => setState((s) => ({ ...s, currentId: current.parentId! }))}><ArrowLeft size={15} /></IconButton></Tooltip>
            <button onClick={() => setState((s) => ({ ...s, currentId: 'root' }))}>Checkout</button>
            {ancestors.slice(1).map((e) => <span key={e.id}><ChevronRight size={12} /><button aria-current={e.id === current.id ? 'page' : undefined} onClick={() => (setNarrowPane('explanation'),setState((s) => ({ ...s, currentId: e.id })))}>{e.question}</button></span>)}
          </nav>
          <div className={`preview-composer ${dragOver ? 'is-drag-over' : ''}`} onDragOver={(e) => { if (e.dataTransfer.types.includes('application/x-code-atlas-context')) { e.preventDefault(); setDragOver(true); } }} onDragLeave={(e) => { if (!e.currentTarget.contains(e.relatedTarget as globalThis.Node | null)) setDragOver(false); }} onDrop={(e) => {
            e.preventDefault(); setDragOver(false); try { const context = JSON.parse(e.dataTransfer.getData('application/x-code-atlas-context')) as CodeContext; if (exampleFiles[context.path]) attach(context); } catch { /* Ignore unrelated drops. */ }
          }}>
            <Flex gap="2" mb="2"><Button variant="soft" size="1" onClick={() => ask(query, 'guide')}><BookOpen size={14} />Guide me</Button><Button variant="soft" size="1" color="gray" onClick={() => ask(query, 'flow')}><Network size={14} />Show the flow</Button><IconButton className="preview-narrow-toggle" size="1" variant="ghost" aria-label={narrowPane==='source'?'Show explanation':'Show source'} onClick={()=>setNarrowPane(narrowPane==='source'?'explanation':'source')}><Code2 size={15}/></IconButton></Flex>
            <div className="preview-chat-input">
              {attachments.length > 0 && <div className="preview-context-chips">{attachments.map((c) => <Badge variant="soft" color="gray" key={contextLabel(c)}><FileCode2 size={12} />{contextLabel(c)}<button aria-label={`Remove ${contextLabel(c)}`} onClick={() => setAttachments((all) => all.filter((i) => i !== c))}><X size={11} /></button></Badge>)}</div>}
              <textarea ref={composer} rows={1} value={query} aria-label="Ask about this codebase" placeholder={current.parentId ? 'Ask a follow-up about this…' : 'What would you like to understand?'} onFocus={() => setComposerFocused(true)} onChange={(e) => setQuery(e.target.value)} onKeyDown={(e) => { if (e.key === 'Escape') setComposerFocused(false); if (e.key === 'Enter' && !e.shiftKey && !e.nativeEvent.isComposing) { e.preventDefault(); ask(query); } }} />
              <div className="preview-input-end"><Tooltip content="Add the open file to your question"><IconButton variant="ghost" size="1" color="gray" aria-label="Attach open file" onClick={() => attach({ path: state.file })}><Paperclip size={15} /></IconButton></Tooltip><Tooltip content="Ask · Enter"><IconButton size="1" aria-label="Send question" disabled={!query.trim()} onClick={() => ask(query)}><ArrowUp size={15} /></IconButton></Tooltip></div>
            </div>
            {composerFocused && query.trim().length > 1 && <div className="preview-search-suggestions">
              <div className="preview-result-heading">{composerResults.length ? 'In your code' : 'No matching code'}</div>
              {composerResults.map((r, i) => <button key={i} onMouseDown={(e) => e.preventDefault()} onClick={() => openFile({ path: r.path, from: r.line, to: r.line })}><FileCode2 size={14} /><span><strong>{r.path}{r.line ? `:${r.line}` : ''}</strong><small>{r.text}</small></span><ChevronRight size={13} /></button>)}
              <button className="preview-explore-ai" onMouseDown={(e) => e.preventDefault()} onClick={() => ask(query)}><MessageSquare size={14} /><strong>Explore with AI</strong><ArrowUp size={14} /></button>
            </div>}
          </div>
          <Group orientation="horizontal" className="preview-content">
            {codeOpen && <><Panel id="code" defaultSize="42%" minSize="220px"><CodePane path={state.file} contents={exampleFiles[state.file]} theme={state.theme} reveal={reveal} onClose={() => {setCodeOpen(false);setNarrowPane('explanation');}} onAttach={attach} onExplain={explain} /></Panel><Separator className="preview-separator" aria-label="Resize code and explanation" /></>}
            <Panel id="explanation" minSize="280px">
              <Tabs.Root value={current.view} onValueChange={(value) => setView(value as ExplorationView)} className="preview-explanation">
                <Tabs.List aria-label="Exploration views"><Tabs.Trigger value="guide"><BookOpen size={13} />Guide</Tabs.Trigger><Tabs.Trigger value="flow"><Network size={13} />Flow</Tabs.Trigger><Tabs.Trigger value="history"><MessageSquare size={13} />Chat history</Tabs.Trigger></Tabs.List>
                <div className="preview-answer-scroll" ref={scroll}>
                  <div className="preview-thread-summary"><span>{1 + (current.turns?.length ?? 0)} answers</span><span>{children.length} branch{children.length === 1 ? '' : 'es'}</span>{current.parentId && <span>Level {ancestors.length}</span>}</div><div className="preview-question"><span>EXPLORING</span><h1>{current.question}</h1>{current.context.length > 0 && <div className="preview-question-context">{current.context.map((c) => <button key={contextLabel(c)} onClick={() => openFile(c)}><Code2 size={12} />{contextLabel(c)}</button>)}</div>}</div>
                  <Tabs.Content value="guide">
                    <article className="preview-answer-card" id="preview-answer-main">
                      <div className="preview-answer-number">01<span>{current.kind === 'validation' ? 'Input validation' : current.kind === 'limits' ? 'Edge cases' : 'Follow the calculation'}</span></div>
                      <h2>{current.kind === 'validation' ? 'Bad prices stop the calculation.' : current.kind === 'limits' ? 'Valid inputs can still produce an invalid total.' : 'Validate first. Sum next. Apply tax last.'}</h2>
                      <p>{current.kind === 'validation' ? 'subtotal checks every price before reducing the array. A negative value, NaN, or Infinity throws a RangeError. No partial total is returned.' : current.kind === 'limits' ? 'Checking each input is not enough to guarantee a finite result. Adding very large finite prices can overflow, and the final multiplication has no extra check.' : 'checkout calls totalWithTax with a 0.10 rate. The rate is checked first. subtotal then validates and adds the prices; the result is multiplied by 1 + taxRate.'}</p>
                      <button className="preview-source-link" onClick={() => openFile({ path: 'src/cart.ts', from: current.kind === 'validation' ? 1 : 8, to: current.kind === 'validation' ? 6 : 13 })}><Code2 size={13} />cart.ts:{current.kind === 'validation' ? '1–6' : '8–13'}<ArrowRight size={13} /></button>
                      <details className="preview-visual-disclosure"><summary><Play size={12}/>Walk through the calculation</summary>{current.kind === 'validation' ? <div className="preview-visual preview-validation-visual"><div className="preview-visual-label">BEFORE ADDING ANY PRICES</div><div><code>[-1, 8]</code><ArrowRight size={16} /><span>RangeError</span></div><div><code>[12, 8]</code><ArrowRight size={16} /><span>20</span></div><p>One invalid price stops the calculation. An empty array returns zero.</p></div> : <div className="preview-visual">
                        <div className="preview-visual-label">{['CHECK THE INPUTS', 'ADD THE PRICES', 'APPLY TAX'][slide]}</div>
                        <div className="preview-calculation"><div><small>{slide === 0 ? 'Prices' : slide === 1 ? 'First price' : 'Subtotal'}</small><strong>{slide === 0 ? '[12, 8]' : slide === 1 ? '$12' : '$20'}</strong></div><span>{slide === 0 ? '+' : slide === 1 ? '+' : '×'}</span><div><small>{slide === 0 ? 'Tax rate' : slide === 1 ? 'Second price' : 'Multiplier'}</small><strong>{slide === 0 ? '0.10' : slide === 1 ? '$8' : '1.10'}</strong></div><ArrowRight size={18} /><div className="preview-calculation-result"><small>{slide === 0 ? 'Both finite ≥ 0' : slide === 1 ? 'Subtotal' : 'Total'}</small><strong>{slide === 0 ? <Check size={23} /> : slide === 1 ? '$20' : '$22'}</strong></div></div>
                        <div className="preview-visual-caption">{['Invalid inputs throw a RangeError before a result is returned.', 'reduce starts at zero and adds each validated price.', 'A rate of 0.10 adds 10% to the subtotal.'][slide]}</div>
                        <div className="preview-player"><Tooltip content="Play this visual step"><IconButton size="1" variant="ghost" color="gray" aria-label={playing ? 'Pause visual preview' : 'Play visual preview'} onClick={() => { if (slide === 2) setSlide(0); setPlaying((p) => !p); }}>{playing ? <Pause size={14} /> : <Play size={14} />}</IconButton></Tooltip><span>{slide + 1} of 3</span><div className="preview-player-dots">{[0, 1, 2].map((n) => <button key={n} aria-label={`Visual step ${n + 1}`} aria-current={slide === n ? 'step' : undefined} onClick={() => { setSlide(n); setPlaying(false); }} />)}</div><Button size="1" variant="ghost" color="gray" disabled={slide === 2} onClick={() => { setSlide((n) => n + 1); setPlaying(false); }}>Next<ArrowRight size={13} /></Button></div>
                      </div>}</details>
                      <details className="preview-extra-detail"><summary>The detail that matters</summary><h3>{current.kind === 'limits' ? 'What to watch for' : 'The detail that matters'}</h3>
                      <p>{current.kind === 'limits' ? 'There is no rounding or currency type here. This example uses ordinary JavaScript numbers; callers must decide how monetary precision and overflow should be handled.' : 'The rate check runs first. If the rate is invalid, subtotal never runs. If a price is invalid, subtotal throws and the tax multiplication never happens.'}</p>
                      </details><footer className="preview-card-footer"><button onClick={() => explore('Why reject negative prices?', 'guide')}><ArrowDown size={13} />Why reject negative prices?</button><span>{mainChildren.length ? `${mainChildren.length} exploration${mainChildren.length === 1 ? '' : 's'}` : 'Go deeper'}</span></footer>
                    {childLinks(mainChildren)}</article>


                  </Tabs.Content>
                  <Tabs.Content value="flow">
                    <p className="preview-flow-intro">Follow the calculation. Select a step to see its code.</p>
                    <div className="preview-flow"><ReactFlow key={detailFlow ? 'subtotal' : 'project'} nodes={detailFlow ? subtotalNodes : nodes} edges={detailFlow ? subtotalEdges : edges} fitView fitViewOptions={{ padding: .06 }} nodesDraggable={false} nodesConnectable={false} colorMode={state.theme} onNodeClick={(_, node) => openFile(detailFlow ? { path: 'src/cart.ts', from: node.id === '0' ? 1 : node.id === '1' ? 3 : 5, to: node.id === '0' ? 4 : node.id === '1' ? 3 : 5 } : nodeSource(Number(node.id)))} minZoom={.4} maxZoom={1.5} aria-label={detailFlow ? 'Subtotal validation flow' : 'Checkout calculation flow'}><Background gap={20} size={1} /><Controls showInteractive={false} /></ReactFlow></div>
                    <div className="preview-flow-caption"><Code2 size={13} />{detailFlow ? 'Validation and calculation inside subtotal.' : 'Function calls in this example.'}</div>
                    <Button variant="soft" size="2" onClick={() => { openFile({ path: 'src/cart.ts', from: 1, to: 6 }); explore(detailFlow ? 'Why reject negative prices?' : 'How does subtotal work?', 'flow'); }}><Network size={14} />{detailFlow ? 'Explore input validation' : 'Explore subtotal'}<ChevronRight size={14} /></Button>
                  </Tabs.Content>
                  <Tabs.Content value="history"><div className="preview-chat-history">{ancestors.flatMap((e) => [{ id: e.id, question: e.question, owner: e.id }, ...(e.turns ?? []).map((t) => ({ ...t, owner: e.id }))]).map((turn) => <div key={turn.id}><div className="preview-history-question"><MessageSquare size={15} /><strong>{turn.question}</strong></div><p>The calculation validates its inputs, sums prices, and applies the tax multiplier.</p><button onClick={() => { setState((s) => ({ ...s, currentId: turn.owner })); setView('guide'); }}><BookOpen size={13} />Open explanation<ArrowRight size={13} /></button></div>)}</div></Tabs.Content>
                  {current.view !== 'history' && (current.turns ?? []).map((turn, index) => {
                    const context = turn.context[0] ?? { path: 'src/cart.ts', from: 1, to: 13 };
                    const isSubtotal = context.to !== undefined && context.to <= 6;
                    const limits = /overflow|break|limit/i.test(turn.question);
                    const turnChildren = children.filter((e) => e.originTurnId === turn.id);
                    const response = /empty/i.test(turn.question) ? 'An empty list returns zero. No element fails the validation check, and reduce starts with an initial total of zero. Tax is then applied to zero, so checkout also returns zero.' : /where.*tax|10%/i.test(turn.question) ? 'checkout supplies the literal 0.1 on line 4. totalWithTax does not choose a default rate; it receives the rate from its caller. The example returns USD, but there is no region or tax policy lookup.' : null;
                    return <article key={turn.id} className="preview-answer-card preview-follow-up"><div className="preview-answer-number">{String(index + 3).padStart(2, '0')}<span>{turn.context.length ? 'Selected code' : 'Follow-up'}</span></div><h2>{turn.question}</h2><p>{response ?? (limits ? 'Each input is checked for finiteness, but the result is not. Large valid prices can overflow during addition or tax multiplication. The code also does not round monetary values.' : isSubtotal ? 'subtotal first checks whether any price is non-finite or negative. It throws a RangeError if so. Otherwise, reduce adds all prices starting at zero; an empty array returns zero.' : 'totalWithTax rejects an invalid tax rate before calling subtotal. subtotal checks the prices and sums them. Only if both checks succeed does the result get multiplied by 1 + taxRate.')}</p><button className="preview-source-link" onClick={() => openFile(context)}><Code2 size={13} />{contextLabel(context)}<ArrowRight size={13} /></button><footer className="preview-card-footer"><button onClick={() => explore(turn.question, current.view === 'flow' ? 'flow' : 'guide', turn.context, turn.id)}><ArrowDown size={13} />Explore this further</button><span>{turnChildren.length ? `${turnChildren.length} exploration${turnChildren.length===1?'':'s'}` : 'Go deeper'}</span></footer>{childLinks(turnChildren)}</article>;
                  })}
                </div>
              </Tabs.Root>
            </Panel>
          </Group>
        </main>
      </Panel>
    </Group></div>
    <footer className="preview-statusbar"><span><GitBranch size={12} />main</span><span>Design preview · sample answers · no AI calls</span><span>⌘ L Ask<span className="preview-status-gap" />⌘ P Search</span></footer>
    <Dialog.Root open={searchOpen} onOpenChange={setSearchOpen}><Dialog.Content maxWidth="620px" className="preview-search-dialog"><Dialog.Title size="3">Search code</Dialog.Title><Dialog.Description size="2" color="gray" mb="3">Find a file, function, or phrase.</Dialog.Description><TextField.Root autoFocus placeholder="Search files and code…" aria-label="Search files and code" value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} onKeyDown={(e) => { if (e.key === 'ArrowDown') { e.preventDefault(); resultList.current?.querySelector<HTMLButtonElement>('button')?.focus(); } }}><TextField.Slot><Search size={15} /></TextField.Slot></TextField.Root>
      <div className="preview-command-results" ref={resultList} onKeyDown={(e) => {
        if (!['ArrowDown', 'ArrowUp'].includes(e.key)) return;
        const buttons = Array.from(resultList.current?.querySelectorAll<HTMLButtonElement>('button') ?? []); const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
        e.preventDefault(); buttons[(index + (e.key === 'ArrowDown' ? 1 : buttons.length - 1)) % buttons.length]?.focus();
      }}>{results.map((r, i) => <button key={i} onClick={() => openFile({ path: r.path, from: r.line, to: r.line })}><FileCode2 size={15} /><span><strong>{r.path}{r.line ? `:${r.line}` : ''}</strong><small>{r.text}</small></span><ArrowRight size={14} /></button>)}{searchQuery && !results.length && <p>No matching files or code.</p>}{searchQuery && <Button variant="soft" onClick={() => ask(searchQuery)}><MessageSquare size={14} />Explore with AI<ArrowRight size={14} /></Button>}</div>
      <Dialog.Close><IconButton className="preview-search-close" size="1" variant="ghost" color="gray" aria-label="Close search"><X size={16} /></IconButton></Dialog.Close>
    </Dialog.Content></Dialog.Root>
  </Theme>;
}
