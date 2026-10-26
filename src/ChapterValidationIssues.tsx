import type { ChapterRevision, ChapterSourceAnchor, ChapterValidationIssue } from './lib/commands';
import { presentChapterIssues } from './lib/chapterValidationPresentation';
import { InlineCodeText } from './InlineCodeText';

export function ChapterValidationIssues({ revision, issues, busy, canRepair, onReview, onRepair }: {
  revision: ChapterRevision;
  issues: ChapterValidationIssue[];
  busy: boolean;
  canRepair: boolean;
  onReview: (sectionId: string, claimId: string | null) => void;
  onRepair: (anchor: ChapterSourceAnchor) => void;
}) {
  if (!issues.length) return null;
  return <section className="chapter-review-issues" aria-label="Chapter validation issues">
    <strong>Review before accepting</strong>
    <ul>{presentChapterIssues(revision, issues).map((finding, index) => <li key={index}>
      <strong className="chapter-issue-location">{finding.location}</strong>
      <p><strong><InlineCodeText text={finding.title}/>.</strong> <InlineCodeText text={finding.message}/></p>
      {finding.source && <code>{finding.source}</code>}
      <div className="chapter-issue-actions">
        {finding.target && <button className="text-button" disabled={busy} onClick={() => onReview(finding.target!.sectionId, finding.target!.claimId)}>{finding.target.claimId ? 'Review claim' : 'Review section'}</button>}
        {finding.repairAnchor && canRepair && <button className="text-button" disabled={busy} onClick={() => onRepair(finding.repairAnchor!)}>Repair citation</button>}
      </div>
      <details><summary>Technical details</summary>
        <p><InlineCodeText text={finding.issue.message}/></p>
        <dl>{Object.entries(finding.issue).filter(([key, value]) => key !== 'message' && value !== null).map(([key, value]) => <div key={key}><dt>{key}</dt><dd><code>{value}</code></dd></div>)}</dl>
      </details>
    </li>)}</ul>
  </section>;
}
