import type { ChapterGenerationOrigin } from './lib/chapterOrigin';

export function GenerationRecord({ origin }: { origin?: ChapterGenerationOrigin | null }) {
  if (!origin) return null;
  return <details className="provider-small-note generation-record">
    <summary>Original explanation</summary>
    {origin.plannedChapter && <p><strong>Question:</strong> {origin.plannedChapter.question}</p>}
    {origin.sourceFiles.length > 0 && <p>{origin.sourceFiles.length} source files were reviewed for this explanation.</p>}
    <p>This explanation stays tied to its original source snapshot. Later edits do not rewrite it.</p>
  </details>;
}
