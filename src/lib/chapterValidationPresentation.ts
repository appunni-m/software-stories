import type { ChapterIssueCode, ChapterRevision, ChapterSourceAnchor, ChapterValidationIssue } from './commands';

const issueLabels: Record<ChapterIssueCode, string> = {
  unsupportedSchemaVersion: 'Unsupported chapter format', invalidId: 'Invalid reference',
  invalidRevision: 'Revision needs review', invalidTarget: 'Chapter scope needs review',
  invalidSectionOrder: 'Section order needs review', duplicateId: 'Repeated reference',
  invalidClaim: 'Claim needs review', wrongSnapshot: 'Different source version',
  missingPath: 'File unavailable', rangeMismatch: 'Source lines need review',
  hashMismatch: 'Source fingerprint differs', excerptMismatch: 'Quoted code differs',
  ambiguousRemap: 'Source match is ambiguous', missingSymbol: 'Code symbol unavailable',
  invalidDependency: 'Supporting code needs review', invalidDestination: 'Link needs review',
  invalidVisual: 'Visual needs review', indexUnavailable: 'Code index unavailable',
  sourceUnavailable: 'Captured code unavailable',
};

type EntityKind = 'section' | 'claim' | 'anchor' | 'hotspot' | 'visual';
type Entity = { kind: EntityKind; sectionId: string; sectionLabel: string; label: string; claimId?: string; anchor?: ChapterSourceAnchor };
export type PresentedChapterIssue = {
  issue: ChapterValidationIssue;
  title: string;
  message: string;
  location: string;
  source: string | null;
  target: { sectionId: string; claimId: string | null } | null;
  repairAnchor: ChapterSourceAnchor | null;
};

function shortLabel(value: string): string {
  const text = value.replace(/\s+/g, ' ').trim();
  return text.length > 120 ? `${text.slice(0, 119)}…` : text;
}

export function chapterIssueFeedback(count: number): string {
  return count === 1 ? '1 issue needs review before acceptance.' : `${count} issues need review before acceptance.`;
}

/** Resolve against the displayed candidate, never a different saved revision.
 * Ambiguous, missing or contradictory identities get no navigation/repair action.
 * Native messages and identities remain available unchanged in technical details.
 */
export function presentChapterIssues(revision: ChapterRevision, issues: ChapterValidationIssue[]): PresentedChapterIssue[] {
  const entities = new Map<string, Entity[]>();
  const add = (id: string, entity: Entity) => entities.set(id, [...(entities.get(id) ?? []), entity]);
  for (const [index, section] of revision.sections.entries()) {
    const sectionLabel = `Section ${index + 1} · ${shortLabel(section.title) || 'Untitled section'}`;
    const owner = { sectionId: section.sectionId, sectionLabel };
    add(section.sectionId, { ...owner, kind: 'section', label: sectionLabel });
    for (const [claimIndex, claim] of section.claims.entries()) {
      const label = `Claim ${claimIndex + 1} · ${shortLabel(claim.prose) || 'Empty claim'}`;
      add(claim.claimId, { ...owner, kind: 'claim', claimId: claim.claimId, label });
      for (const anchor of claim.anchors) add(anchor.anchorId, { ...owner, kind: 'anchor', claimId: claim.claimId, label: 'Source citation', anchor });
    }
    for (const hotspot of section.hotspots) add(hotspot.hotspotId, { ...owner, kind: 'hotspot', label: `Link · ${shortLabel(hotspot.label) || 'Untitled link'}` });
    for (const node of section.visual?.nodes ?? []) add(node.nodeId, { ...owner, kind: 'visual', label: `Visual · ${shortLabel(node.label) || 'Untitled node'}` });
    for (const edge of section.visual?.edges ?? []) add(edge.edgeId, { ...owner, kind: 'visual', label: `Visual link · ${shortLabel(edge.label) || 'Untitled link'}` });
  }
  const resolve = (id: string, kind: EntityKind) => {
    const matches = entities.get(id);
    return matches?.length === 1 && matches[0].kind === kind ? matches[0] : null;
  };
  return issues.map(issue => {
    const references: Array<[string | null, EntityKind]> = [[issue.sectionId, 'section'], [issue.claimId, 'claim'], [issue.anchorId, 'anchor'], [issue.hotspotId, 'hotspot'], [issue.visualItemId, 'visual']];
    const named = references.filter((entry): entry is [string, EntityKind] => entry[0] !== null);
    const resolved = named.map(([id, kind]) => resolve(id, kind));
    const owner = resolved.find((entity): entity is Entity => entity !== null);
    const claims = new Set(resolved.flatMap(entity => entity?.claimId ? [entity.claimId] : []));
    const anchor = resolved.find(entity => entity?.kind === 'anchor')?.anchor ?? null;
    const consistent = Boolean(owner) && Boolean(owner && resolve(owner.sectionId, 'section'))
      && resolved.every(entity => entity !== null && entity.sectionId === owner!.sectionId)
      && claims.size <= 1 && [...claims].every(id => resolve(id, 'claim')?.sectionId === owner!.sectionId)
      && (!issue.path || !anchor || issue.path === anchor.path);
    const claim = resolved.find(entity => entity?.kind === 'claim') ?? (claims.size === 1 ? resolve([...claims][0], 'claim') : null);
    const extra = claim ?? resolved.find(entity => entity?.kind === 'visual' || entity?.kind === 'hotspot');
    const location = consistent ? `${owner!.sectionLabel}${extra ? ` / ${extra.label}` : ''}` : named.length ? 'Chapter · location needs review' : 'Chapter';
    let message = issue.message;
    // Replace whole identity tokens only; never rewrite a matching substring in a
    // source path, identifier or a longer ID. Raw native text stays in the disclosure.
    for (const [id, kind] of [...named].sort((a, b) => b[0].length - a[0].length)) {
      if (!id) continue;
      const escaped = id.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      message = message.replace(new RegExp(`(^|[^\\w:/.-])${escaped}(?=$|[^\\w:/.-])`, 'g'), (_match, prefix: string) => `${prefix}the affected ${kind === 'anchor' ? 'citation' : kind === 'visual' ? 'visual item' : kind}`);
    }
    message = message.replace(/(^|[^\w:/.-])(section|claim|anchor|hotspot|visualnode|visualedge|node|edge|scene|revision|chapter):[A-Za-z0-9:_-]+(?=$|[^\w:/.-])/g, (_match, prefix: string, kind: string) => `${prefix}the affected ${kind === 'anchor' ? 'citation' : kind}`);
    const path = issue.path ?? (consistent ? anchor?.path : null) ?? null;
    const hasRange = consistent && anchor && Number.isSafeInteger(anchor.startLine) && Number.isSafeInteger(anchor.endLine) && anchor.startLine > 0 && anchor.endLine >= anchor.startLine && anchor.endLine <= 0xffffffff;
    return {
      issue, title: issueLabels[issue.code], message, location,
      source: path ? `${path}${hasRange ? `:${anchor.startLine}–${anchor.endLine}` : ''}` : null,
      target: consistent ? { sectionId: owner!.sectionId, claimId: [...claims][0] ?? null } : null,
      repairAnchor: consistent ? anchor : null,
    };
  });
}
