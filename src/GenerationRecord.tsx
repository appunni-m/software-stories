import type { ChapterGenerationOrigin } from './lib/chapterOrigin';

export function GenerationRecord({ origin }: { origin?: ChapterGenerationOrigin | null }) {
  if (!origin) return null;
  const settings = origin.settings;
  return <details className="provider-small-note generation-record">
    <summary>Original generation · {settings.backend === 'codex' ? 'Codex' : 'Claude Agent'}</summary>
    {origin.plannedChapter && <><p><strong>Learning question:</strong> {origin.plannedChapter.question}</p>
      <p>Recorded plan revision <code>{origin.plannedChapter.target.revisionId}</code> · step <code>{origin.plannedChapter.target.stepId}</code></p></>}
    <p>{settings.modelId} · reasoning: {settings.effort ?? 'runtime default'} · approval: {settings.approvalPolicy}
      {settings.backend === 'claude-agent' ? ` · up to ${settings.maxTurns} turns` : ''}</p>
    <p>Request <code>{origin.requestId}</code> · {origin.sourceFiles.length} reviewed files · original snapshot <code>{origin.snapshotId}</code></p>
    <p>This records the original request. Later edits or source changes may follow. Local metadata is not account attestation or proof that the question was answered.</p>
  </details>;
}
