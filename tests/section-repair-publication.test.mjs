import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { it } from 'node:test';

it('a selected section repair builds and sends its request without another review or consent action', async () => {
  const source = await readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8');
  const automaticReviewRequest = source.slice(
    source.indexOf('  const automaticReviewRequest ='),
    source.indexOf('  const [recoveryReady'),
  );
  const buildReviewEffect = source.slice(
    source.indexOf('  useEffect(() => {\n    if (!automaticReviewRequest'),
    source.indexOf('  useEffect(() => {\n    if (!automaticSnapshotRequest'),
  );
  const sendEffect = source.slice(
    source.indexOf('  useEffect(() => {\n    if (!automaticSectionRepair || !review'),
    source.indexOf('  return (\n'),
  );
  const generateGuard = source.slice(
    source.indexOf('  const generate = async () => {'),
    source.indexOf('    if (Date.now() >= review.expiresAtUnixMs)', source.indexOf('  const generate = async () => {')),
  );

  assert.match(source, /mode\.kind === 'section-update' && !mode\.recoveryOperationId/u);
  assert.match(automaticReviewRequest, /\|\| automaticSectionRepair/u);
  assert.match(buildReviewEffect, /void buildReview\(\)/u);
  assert.match(sendEffect, /void generate\(\)/u);
  assert.match(generateGuard, /!review \|\| \(requiresFreshConsent && !consent\)/u);
  assert.match(source, /const requiresFreshConsent = !automaticSectionRepair && !retrievalLimits/u);
  assert.match(generateGuard, /!status\?\.authenticated/u);
});

it('a validated selected-section result is saved, published, and checked against the original section scope', async () => {
  const source = await readFile(new URL('../src/CodexChapterDialog.tsx', import.meta.url), 'utf8');
  const publishRepair = source.slice(
    source.indexOf('      if (sectionUpdate) {\n        const candidate = result.data;'),
    source.indexOf('      } else {\n        resources.current.draft = result.data.generationId;', source.indexOf('      if (sectionUpdate) {\n        const candidate = result.data;')),
  );

  assert.match(publishRepair, /saveGeneratedChapterDraft\(candidate\.generationId\)/u);
  assert.match(publishRepair, /acceptChapterDraft\(draft\.projectId, draft\.revisionId\)/u);
  assert.match(publishRepair, /result\.revision\.lifecycle !== 'accepted'/u);
  assert.match(publishRepair, /result\.validation\.status !== 'valid'/u);
  assert.match(publishRepair, /result\.revision\.basedOnRevisionId !== draft\.revisionId/u);
  assert.match(publishRepair, /accepted\.sections\.some\(\(section, index\) => section\.sectionId !== draft\?\.sections\[index\]\?\.sectionId\s*\|\| section\.sectionRevisionId !== draft\?\.sections\[index\]\?\.sectionRevisionId\)/u);
  assert.match(publishRepair, /repairedSection\.sectionRevisionId === sectionUpdate\.sectionRevisionId/u);
  assert.match(publishRepair, /setNote\(`Updated \$\{sectionTitle \?\? 'the selected section'\} in a new immutable revision\./u);
  assert.match(publishRepair, /onSaved\?\.\(accepted\)/u);
});
