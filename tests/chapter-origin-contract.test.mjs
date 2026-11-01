import test from 'node:test';
import assert from 'node:assert/strict';
import { isCurrentChapterGenerationOrigin } from '../src/lib/chapterOrigin.ts';
import { readChapterStudioDraft } from '../src/lib/chapterStudioDraft.ts';

const digest = 'a'.repeat(64);
const currentOrigin = () => ({
  schemaVersion: 6,
  projectId: 'project',
  snapshotId: 'snapshot',
  targetPath: 'src/font.rs',
  purpose: 'fileChapter',
  requestId: 'request',
  requestFingerprint: digest,
  requestBodySha256: digest,
  contractVersion: 'contract-v1',
  settings: { backend: 'codex', modelId: 'gpt-6-luna', effort: 'max', approvalPolicy: 'never' },
  sourceFiles: [{ path: 'src/font.rs', sha256: digest, sizeBytes: 1 }],
  sectionUpdate: null,
  plannedChapter: null,
  contentSha256: digest,
});

test('chapter lineage accepts schema 6 and rejects the superseded schema 5 shape', () => {
  const current = currentOrigin();
  assert.equal(isCurrentChapterGenerationOrigin(current), true);

  const superseded = {
    ...current,
    schemaVersion: 5,
    retrieval: {
      policy: 'captured-ranges-v1',
      ledgerSha256: digest,
      ranges: [{
        snapshotId: 'snapshot', path: 'src/font.rs', sourceSha256: digest,
        startByte: 0, endByte: 1, startLine: 1, endLine: 1, chunkSha256: digest,
        enclosing: null, parent: null, parser: null,
      }],
    },
  };
  assert.equal(isCurrentChapterGenerationOrigin(superseded), false);
});

test('unsupported lineage in a saved editor draft is preserved as unsupported', () => {
  const serialized = JSON.stringify({
    version: 1,
    candidate: { schemaVersion: 5, generationOrigin: { schemaVersion: 5 } },
  });
  assert.deepEqual(
    readChapterStudioDraft(serialized, 'project', 'snapshot', null, null),
    { status: 'preserved', reason: 'unsupported' },
  );
});
