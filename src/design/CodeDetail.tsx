import { Code2, MessageSquare } from 'lucide-react';
import { InlineCodeText } from '../InlineCodeText';
import type { ChapterSourceAnchor } from '../lib/commands';
import './code-detail.css';

interface Props {
  title: string;
  claims: readonly string[];
  anchors: readonly ChapterSourceAnchor[];
  onOpen: (anchor: ChapterSourceAnchor) => void;
  onExplore?: (title: string, anchors: readonly ChapterSourceAnchor[]) => void;
}

/** A small, source-backed detail view shared by Guide scenes and Flow nodes. */
export function CodeDetail({ title, claims, anchors, onOpen, onExplore }: Props) {
  return <section className="code-detail" aria-label={`Code detail: ${title}`} aria-live="polite">
    <div className="code-detail-heading"><Code2 size={14} aria-hidden="true"/><h3>What this code does</h3></div>
    {claims.length > 0
      ? <div className="code-detail-claims">{claims.map((claim, index) => <p key={`${index}:${claim}`}><InlineCodeText text={claim}/></p>)}</div>
      : <p className="code-detail-empty">No validated explanation claim is linked to this visual point. Open its cited source to inspect the implementation.</p>}
    <div className="code-detail-sources" aria-label="Cited source lines">
      {anchors.map((anchor) => <button key={anchor.anchorId} type="button" onClick={() => onOpen(anchor)}>
        <Code2 size={12} aria-hidden="true"/><span>{anchor.path}:{anchor.startLine}–{anchor.endLine}</span>
      </button>)}
    </div>
    {anchors.length > 0 && onExplore && <button className="code-detail-explore" type="button" onClick={() => onExplore(title, anchors)}>
      <MessageSquare size={13} aria-hidden="true"/>Explore this code
    </button>}
  </section>;
}
