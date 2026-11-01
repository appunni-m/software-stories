import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import test from 'node:test';

const workspace=readFileSync(new URL('../src/workspace/Workspace.tsx',import.meta.url),'utf8');
const app=readFileSync(new URL('../src/App.tsx',import.meta.url),'utf8');

function functionBody(signature,nextSignature){
  const start=workspace.indexOf(signature);
  assert.notEqual(start,-1,'missing '+signature);
  const end=workspace.indexOf(nextSignature,start+signature.length);
  assert.notEqual(end,-1,'missing boundary '+nextSignature);
  return workspace.slice(start,end);
}

test('Explain selected code submits directly at the current exploration level',()=>{
  const explain=functionBody('function explain(context:Context){','  function explainSafely(context:Context){');
  assert.match(explain,/verifiedCodeContext\(context,snapshotId\)/);
  assert.match(explain,/ask\(question,view,\[selected\]\)/);
  assert.doesNotMatch(explain,/setPendingCodeExplanation\(pending\)/);
  assert.doesNotMatch(workspace,/Review this code question|Nothing is sent until you choose Ask about this code\./);
  assert.ok(!workspace.includes('codeQuestionReview'));
});

test('selected code stays bound to the current snapshot and sharing consent',()=>{
  const ask=functionBody('function ask(question:string','function verifiedCodeContext(context:Context,expectedSnapshotId:string)');
  assert.ok(ask.indexOf('requireCodexConnection()')<ask.indexOf('appendQuestion('));
  assert.match(ask,/if\(state\.grantId\).*runQuestion/s);
  assert.match(ask,/setConsentTurn\(id\)/);

  const validate=functionBody('function verifiedCodeContext(context:Context,expectedSnapshotId:string)','  function explain(context:Context)');
  assert.match(validate,/expectedSnapshotId!==snapshotId/);
  assert.match(validate,/source\.snapshotId!==snapshotId/);
  assert.match(validate,/source\.path!==selected\.path/);
  assert.match(validate,/source\.sha256/);
  assert.match(validate,/selected\.sha256===source\.sha256/);
});

test('the captured-source action resumes in the same conversation and asks only after exact-source replay',()=>{
  const handlerStart=app.indexOf('const explainCodeLensRangeWithProvider =');
  const handlerEnd=app.indexOf('  const closeProviderDialog =',handlerStart);
  assert.ok(handlerStart>=0&&handlerEnd>handlerStart,'missing captured-source Code Lens action');
  const handler=app.slice(handlerStart,handlerEnd);
  assert.match(handler,/resolveCodeLensCitation\(/);
  assert.match(handler,/setPendingCodeLensQuestion\(/);
  assert.match(handler,/workspace: 'guide-plan'/);
  assert.match(handler,/relativePath: targetPath/);
  assert.doesNotMatch(handler,/setProviderDialog\(/,'the action must not open a second provider review');

  const replayStart=workspace.indexOf('const handledCodeLensQuestion=useRef');
  const replayEnd=workspace.indexOf("  useEffect(()=>{\n    const pending=props.pendingLargeFileExplanation;",replayStart);
  assert.ok(replayStart>=0&&replayEnd>replayStart,'missing pending Code Lens replay');
  const replay=workspace.slice(replayStart,replayEnd);
  assert.match(replay,/source\.path!==pending\.path/);
  assert.match(replay,/source\.sha256!==pending\.sourceSha256/);
  assert.match(replay,/explainSafely\(\{path:pending\.path,sha256:pending\.sourceSha256,from:pending\.startLine,to:pending\.endLine\}\)/);
  assert.match(replay,/onPendingCodeLensQuestionConsumed\?\.\(pending\.id\)/);
});
