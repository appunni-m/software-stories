import { useEffect, useState } from 'react';
import { getDocumentationNotices, type DocumentationNotice } from './lib/sourceDiscovery';
import { isNativeRuntime } from './lib/commands';

export function DocumentationNotices({ projectId, snapshotId, onOpenFile }: {
  projectId: string; snapshotId: string; onOpenFile: (path: string) => void;
}) {
  const [notices, setNotices] = useState<DocumentationNotice[]>([]);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let current = true;
    setNotices([]); setFailed(false);
    if (isNativeRuntime()) void getDocumentationNotices(projectId, snapshotId).then((value) => { if (current) setNotices(value); })
      .catch(() => { if (current) setFailed(true); });
    return () => { current = false; };
  }, [projectId, snapshotId]);
  const deprecated = notices.filter((doc) => doc.status === 'declaredDeprecated');
  return <details className="documentation-notices"><summary>Code is the source of truth{deprecated.length ? ` · ${deprecated.length} deprecated / archived docs found` : ' · docs are hints'}</summary>
    <p>Documentation can guide discovery. Behavior must be supported by implementation citations. A document is called outdated only when a specific code contradiction is demonstrated; timestamps alone do not establish that.</p>
    {failed ? <p>Documentation labels could not be read. Their status is unknown.</p> : <>
      <ul>{notices.map((doc) => <li key={doc.path}><button className="text-button" onClick={() => onOpenFile(doc.path)}>{doc.path}</button> · {doc.status === 'declaredDeprecated' ? 'Declared deprecated / archived' : 'Not verified against code'}<p>{doc.reason}</p></li>)}</ul>
      <p className="provider-small-note">Local scan of up to 20 captured text documents and their first 40 lines. Unlisted documents have not been checked.</p>
    </>}
  </details>;
}
