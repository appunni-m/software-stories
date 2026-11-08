import test from 'node:test';
import assert from 'node:assert/strict';
import { CHAPTER_SCHEMA_VERSION } from '../src/lib/chapterSchema.ts';
import { CHAPTER_ORIGIN_SCHEMA_VERSION, isCurrentChapterGenerationOrigin } from '../src/lib/chapterOrigin.ts';
import { readChapterStudioDraft } from '../src/lib/chapterStudioDraft.ts';

const digest = 'a'.repeat(64);
const currentOrigin = () => ({
  schemaVersion: CHAPTER_ORIGIN_SCHEMA_VERSION,
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

const currentOverviewOrigin = () => ({
  ...currentOrigin(),
  targetPath: null,
  purpose: 'repositoryOverview',
  retrieval: {
    policy: 'progressive-v1',
    ledgerSha256: digest,
    indexIdentity: { status: 'unavailable' },
    ranges: [{
      snapshotId: 'snapshot', path: 'src/font.rs', sourceSha256: digest,
      startByte: 0, endByte: 1, startLine: 1, endLine: 1, chunkSha256: digest,
      enclosing: null, parent: null, parser: null,
    }],
    searches: [{
      querySha256: digest,
      query: 'font parser',
      scope: null,
      resultKind: 'files',
      limit: 32,
      offset: 0,
      candidates: [{ path: 'src/font.rs', sourceSha256: digest }],
      candidatesComplete: true,
      limited: false,
      nextOffset: null,
    }],
  },
});

test('chapter lineage accepts only the current schema 8', () => {
  const current = currentOrigin();
  assert.equal(isCurrentChapterGenerationOrigin(current), true);
  assert.equal(isCurrentChapterGenerationOrigin({ ...current, schemaVersion: 7 }), false);
});

test('schema 8 retrieval lineage includes validated recorded search scopes', () => {
  const origin = currentOverviewOrigin();
  assert.equal(isCurrentChapterGenerationOrigin(origin), true);
  assert.equal(isCurrentChapterGenerationOrigin({
    ...origin,
    retrieval: { ...origin.retrieval, searches: undefined },
  }), false);
  assert.equal(isCurrentChapterGenerationOrigin({
    ...origin,
    retrieval: {
      ...origin.retrieval,
      searches: [{ ...origin.retrieval.searches[0], query: '  font parser  ' }],
    },
  }), false);
});

test('chapter studio restores a schema 8 retrieval draft instead of rejecting it as old', () => {
  const origin = currentOverviewOrigin();
  const candidate = {
    schemaVersion: CHAPTER_SCHEMA_VERSION,
    chapterId: 'chapter',
    revisionId: 'revision',
    revisionNumber: 1,
    projectId: 'project',
    snapshotId: 'snapshot',
    target: { targetId: 'overview', scope: { kind: 'overview' } },
    title: 'Repository guide',
    lifecycle: 'draft',
    validationStatus: 'valid',
    createdAtUnixMs: 1,
    generationOrigin: origin,
    sections: [{
      sectionId: 'section',
      sectionRevisionId: 'section-revision',
      order: 0,
      title: 'Opening',
      prose: '',
      spokenText: '',
      claims: [],
      dependencies: [],
      visual: null,
      hotspots: [],
    }],
  };
  const serialized = JSON.stringify({
    version: 1,
    projectId: 'project',
    snapshotId: 'snapshot',
    targetKey: 'overview',
    baseRevisionId: 'revision',
    candidate,
  });
  assert.equal(readChapterStudioDraft(serialized, 'project', 'snapshot', null, null).status, 'restored');
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
