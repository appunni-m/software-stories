import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';

const app = readFileSync(new URL('../src/App.tsx', import.meta.url), 'utf8');

test('Keychain mutations block sending and keep both close controls visibly unavailable', () => {
  const sendStart = app.indexOf('const handleSend = async () => {');
  const sendEnd = app.indexOf('\n  const handleConfirmCancel', sendStart);
  const send = app.slice(sendStart, sendEnd);
  assert.match(send, /if\s*\(!providerId \|\| !modelId \|\| !requestReview \|\| keyBusy \|\|/);
  assert.match(app, /const requestReady = Boolean\(\s*!keyBusy &&/);
  assert.match(app, /onClick=\{handleClose\} disabled=\{keyBusy \|\| sending \|\| saveBusy\}/);
  assert.match(app, /const handleClose = \(\) => \{\s*if \(keyBusy \|\| sending \|\| saveBusy\) return;/);
});

test('Keychain save and remove always release the busy state after success or failure', () => {
  const saveStart = app.indexOf('const handleSaveKey = async');
  const removeStart = app.indexOf('const handleRemoveKey = async');
  const modelStart = app.indexOf('const handleModelChange =', removeStart);
  const save = app.slice(saveStart, removeStart);
  const remove = app.slice(removeStart, modelStart);
  for (const operation of [save, remove]) {
    assert.match(operation, /if \(!providerId \|\| keyBusy\) return;/);
    assert.match(operation, /setKeyBusy\(true\)/);
    assert.match(operation, /finally \{\s*setKeyBusy\(false\);\s*\}/);
  }
});

test('a credential lookup cannot publish after the provider changes or Keychain work starts', () => {
  const lookupStart = app.indexOf('getProviderCredentialStatus(providerId)');
  const effectStart = app.lastIndexOf('useEffect(() => {', lookupStart);
  const effectEnd = app.indexOf('}, [keyBusy, native, providerCatalog, providerId]);', lookupStart);
  const lookup = app.slice(effectStart, effectEnd);
  assert.match(lookup, /if \(keyBusy\) return;/);
  assert.match(lookup, /let active = true;[\s\S]*?getProviderCredentialStatus\(providerId\)[\s\S]*?if \(active\) setCredential\(status\)/);
  assert.match(lookup, /return \(\) => \{ active = false; \};/);

  const providerStart = app.indexOf('const handleProviderChange =');
  const providerEnd = app.indexOf('const handleContextFileChange =', providerStart);
  const providerChange = app.slice(providerStart, providerEnd);
  assert.match(providerChange, /if \(keyBusy \|\| sending/);
  assert.match(providerChange, /setCredential\(null\)/);
  assert.match(app, /<AppSelect ariaLabel="Direct API provider"/);
  assert.match(app, /onValueChange=\{handleProviderChange\}/);
  assert.match(app, /disabled=\{keyBusy \|\| sending \|\| saveBusy \|\| Boolean\(sendResult \|\| codeLensResult\)\}/);
});
