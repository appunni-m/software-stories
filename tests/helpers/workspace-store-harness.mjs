import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import vm from 'node:vm';
import ts from 'typescript';

/** Execute the real store/model code with only its native and React boundaries
 * replaced. Unexpected commands fail, so restoration cannot dispatch inference. */
export function workspaceStoreHarness(invoke,options={}) {
  const root = fileURLToPath(new URL('../../src/', import.meta.url));
  const listeners = new Map();
  const calls = [];
  const storage = options.storage??new Map();
  const unexpected = () => { throw new Error('Unexpected model or indexing operation during local restoration'); };
  const context = vm.createContext({
    crypto, CustomEvent, Error, console, setTimeout, clearTimeout, queueMicrotask,
    window: new EventTarget(),
    localStorage: { getItem: (key) => storage.get(key) ?? null, setItem: (key, value) => storage.set(key, value) },
  });
  const stubs = {
    react: { useRef: unexpected, useSyncExternalStore: unexpected },
    '@tauri-apps/api/core': { invoke: (name, args) => { calls.push({ name, args }); return invoke(name, args); } },
    '@tauri-apps/api/event': { listen: async (name, callback) => {
      if (!listeners.has(name)) listeners.set(name, new Set());
      listeners.get(name).add(callback);
      return () => listeners.get(name).delete(callback);
    } },
    '../lib/commands': {
      isNativeRuntime:()=>options.native??false,
      listChapterRevisions: async () => [], cancelCodexChapterRequest: unexpected,
      getSnapshotIndexProgress: unexpected, loadSnapshotIndex: unexpected, startSnapshotIndexing: unexpected,
    },
    '../lib/connectionStore': { ensureCodexConnection: options.ensureCodexConnection??unexpected, readCodexSelection: options.readCodexSelection??unexpected },
  };
  const allowed = new Set(['workspace/store.ts', 'workspace/model.ts', 'workspace/conversationPersistence.ts', 'workspace/outcomes.ts', 'workspace/queue.ts', 'workspace/refreshQueue.ts', 'lib/errorMessage.ts'].map((file) => resolve(root, file)));
  const modules = new Map();
  function load(path) {
    if (!allowed.has(path)) throw new Error(`Unexpected store dependency: ${path}`);
    if (modules.has(path)) return modules.get(path).exports;
    const module = { exports: {} };
    modules.set(path, module);
    const { outputText } = ts.transpileModule(readFileSync(path, 'utf8'), {
      compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 }, fileName: path,
    });
    const require = (name) => {
      if (Object.hasOwn(stubs, name)) return stubs[name];
      if (!name.startsWith('.')) throw new Error(`Unexpected package: ${name}`);
      return load(resolve(dirname(path), name.endsWith('.ts') ? name : `${name}.ts`));
    };
    vm.runInContext(`(function(require,module,exports){${outputText}\n})`, context, { filename: path })(require, module, module.exports);
    return module.exports;
  }
  return {
    store: load(resolve(root, 'workspace/store.ts')), calls, storage,
    emit(name, payload) { for (const callback of listeners.get(name) ?? []) callback({ payload }); },
    listenerCount() { return [...listeners.values()].reduce((sum, items) => sum + items.size, 0); },
    onWindowEvent(name, callback) {
      context.window.addEventListener(name, callback);
      return () => context.window.removeEventListener(name, callback);
    },
  };
}

export function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
export const flushLocalReads = () => new Promise((resolve) => setImmediate(resolve));
