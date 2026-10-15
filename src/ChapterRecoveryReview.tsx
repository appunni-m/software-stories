import { useEffect, useRef, useState } from 'react';
import {
  listChapterRevisions, listPendingGeneratedChapterPreviews,
  type ChapterRevision, type PendingChapterPreviewSummary,
} from './lib/commands';

/** Local history inspection before another interrupted chapter request. The
 * legacy preview format has no request provenance, so matches are candidates. */
export function ChapterRecoveryReview({ native, projectId, snapshotId, targetPath, locked,
  onReady, onOpenPreview,
}: {
  native: boolean; projectId: string; snapshotId: string; targetPath: string | null;
  locked: boolean; onReady: (ready: boolean) => void;
  onOpenPreview?: (generationId: string) => void;
}) {
  const [pending, setPending] = useState<PendingChapterPreviewSummary[]>([]);
  const [saved, setSaved] = useState<ChapterRevision[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState(false);
  const [checked, setChecked] = useState(false);
  const [reload, setReload] = useState(0);
  const callback = useRef(onReady);
  callback.current = onReady;

  useEffect(() => {
    let active = true;
    let request = 0;
    const refresh = () => {
      const version = ++request;
      callback.current(false);
      setLoading(true); setChecked(false); setError(false);
      if (!native) { setLoading(false); setError(true); return; }
      void Promise.all([listPendingGeneratedChapterPreviews(), listChapterRevisions(projectId, snapshotId)])
        .then(([previews, revisions]) => {
          if (!active || version !== request) return;
          // Unreadable previews cannot be safely attributed to another scope.
          setPending(previews.filter((preview) => preview.unavailableReason || preview.projectId === null
            || preview.projectId === projectId && (preview.snapshotId === null
              || preview.snapshotId === snapshotId && preview.targetPath === targetPath)));
          setSaved(revisions.filter((revision) => revision.projectId === projectId && revision.snapshotId === snapshotId
            && (targetPath === null ? revision.target.scope.kind === 'overview'
              : revision.target.scope.kind === 'file' && revision.target.scope.path === targetPath)));
        })
        .catch(() => { if (active && version === request) setError(true); })
        .finally(() => { if (active && version === request) setLoading(false); });
    };
    refresh();
    window.addEventListener('code-atlas:pending-chapter-previews-changed', refresh);
    window.addEventListener('code-atlas:chapters-changed', refresh);
    return () => {
      active = false; ++request;
      window.removeEventListener('code-atlas:pending-chapter-previews-changed', refresh);
      window.removeEventListener('code-atlas:chapters-changed', refresh);
      callback.current(false);
    };
  }, [native, projectId, snapshotId, targetPath, reload]);

  return <section className="provider-review-card" aria-labelledby="recovered-chapter-title">
    <h3 id="recovered-chapter-title">Check earlier results first</h3>
    <p>A lost reply may leave a preview or saved draft. These possible matches use the project, snapshot and chapter target; older records do not prove which agent or request produced them.</p>
    {loading && <p role="status">Reading local previews and chapter history…</p>}
    {error && <p role="alert">Local result history could not be read. Reload it before preparing another request.</p>}
    {!loading && !error && <>
      {pending.length ? <ul>{pending.map((preview) => <li key={preview.generationId}>
        <strong>{preview.title || 'Saved preview'}</strong>{preview.unavailableReason && <p>{preview.unavailableReason}</p>}
        <button className="text-button" disabled={locked || !!preview.unavailableReason || !onOpenPreview}
          onClick={() => onOpenPreview?.(preview.generationId)}>Review saved preview</button>
      </li>)}</ul> : <p>No matching pending chapter previews were found.</p>}
      {saved.length ? <details open><summary>{saved.length} saved chapter revisions for this target</summary>
        <ul>{saved.slice(0, 20).map((revision) => <li key={revision.revisionId}>{revision.title} · {revision.lifecycle} · revision {revision.revisionNumber}
          <small> · {new Date(revision.createdAtUnixMs).toLocaleString()}</small></li>)}</ul>
        {saved.length > 20 && <p>Showing the first 20. All revisions remain in Edit.</p>}
        <p>Close this request to inspect saved chapters in Edit before deciding whether another generation is needed.</p>
      </details> : <p>No saved chapter revisions for this snapshot and target were found.</p>}
      <label className="provider-checkbox"><input type="checkbox" checked={checked} disabled={locked}
        onChange={(event) => { setChecked(event.target.checked); callback.current(event.target.checked); }} />
        I checked earlier results and want a fresh request review.</label>
    </>}
    <button className="secondary-button" disabled={locked || loading} onClick={() => setReload((value) => value + 1)}>Reload local results</button>
  </section>;
}
