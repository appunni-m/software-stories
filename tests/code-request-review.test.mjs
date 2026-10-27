import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';

const workspace=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');

function functionBody(signature,nextSignature){
  const start=workspace.indexOf(signature);
  assert.notEqual(start,-1,`missing ${signature}`);
  const end=workspace.indexOf(nextSignature,start+signature.length);
  assert.notEqual(end,-1,`missing boundary ${nextSignature}`);
  return workspace.slice(start,end);
}

test('Explain this code opens a review without dispatching a provider request',()=>{
  const explain=functionBody('function explain(context:Context){','  function acceptCodeRequestReview()');
  assert.match(explain,/setCodeRequestReview\(/);
  assert.doesNotMatch(explain,/\b(?:ask|runQuestion|appendQuestion|runRepositoryGuide|sendCodexCodeLensRequest)\s*\(/);
  assert.match(workspace,/Nothing is sent until you choose Ask about this code\./);
  assert.match(workspace,/onClick=\{acceptCodeRequestReview\}/);
});

test('the explicit review action revalidates the selected snapshot before dispatch',()=>{
  const freshness=functionBody('function isCurrentCodeRequestReview(review:CodeRequestReview){','  function explain(context:Context)');
  assert.match(freshness,/review\.snapshotId!==snapshotId/);
  assert.match(freshness,/source\.snapshotId!==snapshotId/);
  assert.match(freshness,/source\.path!==review\.context\.path/);
  assert.match(freshness,/source\.sha256!==review\.sourceSha256/);
  assert.match(freshness,/!rows\.some\(\(row\)=>row\.path===review\.context\.path\)/);
  assert.match(freshness,/review\.context\.sha256===review\.sourceSha256/);

  const accept=functionBody('function acceptCodeRequestReview(){','  useEffect(()=>{');
  assert.ok(accept.indexOf('isCurrentCodeRequestReview(review)')<accept.indexOf('requireCodexConnection()'));
  assert.ok(accept.indexOf('requireCodexConnection()')<accept.indexOf('ask(review.question'));
  assert.match(accept,/ask\(review\.question,current\.view==='flow'\?'flow':'guide',\[review\.context\]\)/);
});
