// Launched only by the native manager in a private stage. No repository source
// or renderer-selected options enter this protocol. The native caller owns the
// host process group; this wrapper separately observes the SDK child's close.
import { spawn } from 'node:child_process';
import { PassThrough } from 'node:stream';
import { TextDecoder } from 'node:util';
import { isAbsolute } from 'node:path';
import { ClaudeDiscoveryFailure, discoverClaudeModels } from './discovery.mjs';

const exits = { incompatible: 20, cancelled: 21, timedOut: 22, unavailable: 23, cleanupFailed: 24, outputLimit: 25 };
const controller = new AbortController();
let externalCancelled = false;
const cancel = () => {
  externalCancelled = true;
  controller.abort();
  process.stdin.destroy();
};
process.on('SIGINT', cancel);
process.on('SIGTERM', cancel);

let child;
let childClosed = false;
let childClose;
let streamFailure;
let spawnAttempted = false;
let sdkOutput;
let stdoutEof = false;
let stderrEof = false;

function failStream(kind) {
  streamFailure ??= kind;
  controller.abort();
}

async function readConfiguration() {
  const chunks = [];
  let size = 0;
  for await (const chunk of process.stdin) {
    size += chunk.length;
    if (size > 32 * 1024) throw new ClaudeDiscoveryFailure('incompatible');
    chunks.push(chunk);
  }
  const value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(chunks)));
  const fields = ['protocolVersion', 'executable', 'stage', 'temporary', 'accountHome'];
  if (!value || typeof value !== 'object' || Array.isArray(value)
    || Object.keys(value).length !== fields.length || Object.keys(value).some((field) => !fields.includes(field))
    || fields.slice(1).some((field) => typeof value[field] !== 'string' || !isAbsolute(value[field])
      || Buffer.byteLength(value[field]) > 4096 || /[\u0000-\u001f\u007f-\u009f]/u.test(value[field])
      || value[field].split('/').includes('..'))
    || value.protocolVersion !== 1 || process.cwd() !== value.stage
    || process.env.HOME !== value.accountHome || process.env.TMPDIR !== value.temporary
    || value.temporary !== `${value.stage}/tmp` || process.env.PATH !== '/usr/bin:/bin:/usr/sbin:/sbin'
    || process.env.LANG !== 'en_US.UTF-8'
    || process.env.DISABLE_AUTOUPDATER !== '1') throw new ClaudeDiscoveryFailure('incompatible');
  // SDK env options merge with process.env. Remove any unexpected host inputs
  // before importing vendor code. Native env_clear is still mandatory before
  // Node starts: this cannot undo a startup loader from inherited NODE_OPTIONS.
  const allowed = new Set(['HOME', 'TMPDIR', 'PATH', 'LANG', 'DISABLE_AUTOUPDATER']);
  for (const key of Object.keys(process.env)) if (!allowed.has(key)) delete process.env[key];
  return value;
}

// This is the fixed launch contract of the inspected SDK, not a general CLI
// parser. Accept the SDK's split or --name=value encoding and reject drift
// before starting the CLI. In particular, --bare would change authentication.
function hasDiscoveryArguments(args) {
  const switches = new Set([
    '--verbose', '--strict-mcp-config', '--no-session-persistence',
    '--safe-mode', '--restricted', '--disable-slash-commands', '--no-chrome',
  ]);
  const values = new Map([
    ['--output-format', 'stream-json'], ['--input-format', 'stream-json'],
    ['--max-turns', '1'], ['--permission-prompts', 'none'],
    ['--disallowedTools', 'mcp__*'], ['--tools', ''], ['--setting-sources', ''],
    ['--permission-mode', 'dontAsk'], ['--mcp-config', '{"mcpServers":{}}'],
    ['--settings', '{"disableAllHooks":true,"fallbackModel":[]}'],
  ]);
  const seen = new Set();
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index];
    const separator = argument.indexOf('=');
    const name = separator < 0 ? argument : argument.slice(0, separator);
    if (seen.has(name)) return false;
    seen.add(name);
    if (switches.has(name)) {
      if (separator >= 0) return false;
    } else if (values.has(name)) {
      const value = separator < 0 ? args[++index] : argument.slice(separator + 1);
      if (value !== values.get(name)) return false;
    } else return false;
  }
  return seen.size === switches.size + values.size;
}

function ownedSpawn(configuration, options) {
  if (spawnAttempted || options.command !== configuration.executable || options.cwd !== configuration.stage
    || !Array.isArray(options.args) || options.args.length > 128
    || options.args.some((value) => typeof value !== 'string' || Buffer.byteLength(value) > 8192)
    || options.args.reduce((total, value) => total + Buffer.byteLength(value), 0) > 64 * 1024
    || !hasDiscoveryArguments(options.args)) {
    throw new ClaudeDiscoveryFailure('incompatible');
  }
  spawnAttempted = true;
  // Native ownership uses one process group. Do not detach the SDK child or
  // inherit shell/credential/endpoint/provider/loader environment from the SDK.
  const env = {
    HOME: configuration.accountHome, TMPDIR: configuration.temporary,
    PATH: '/usr/bin:/bin:/usr/sbin:/sbin', LANG: 'en_US.UTF-8', DISABLE_AUTOUPDATER: '1',
    CLAUDE_AGENT_SDK_VERSION: '0.3.300',
  };
  if (typeof options.env?.CLAUDE_CODE_ENTRYPOINT === 'string'
    && /^[A-Za-z0-9_-]{1,80}$/.test(options.env.CLAUDE_CODE_ENTRYPOINT)) {
    env.CLAUDE_CODE_ENTRYPOINT = options.env.CLAUDE_CODE_ENTRYPOINT;
  }
  child = spawn(configuration.executable, options.args, {
    cwd: configuration.stage, env, stdio: ['pipe', 'pipe', 'pipe'],
    detached: false, shell: false, windowsHide: true, signal: options.signal,
  });
  childClose = new Promise((resolve) => child.once('close', () => { childClosed = true; resolve(); }));
  child.on('error', (error) => { if (error.code !== 'ABORT_ERR') failStream('unavailable'); });
  if (!child.stdin || !child.stdout || !child.stderr) throw new ClaudeDiscoveryFailure('unavailable');
  let total = 0;
  let frame = 0;
  let diagnostics = 0;
  const decoder = new TextDecoder('utf-8', { fatal: true });
  const bounded = new PassThrough();
  sdkOutput = bounded;
  const rejectOutput = (kind) => { failStream(kind); bounded.destroy(new Error(kind)); };
  // Validate upstream of the SDK-owned readable. SDK cleanup can destroy that
  // readable; remaining raw bytes must still pass the independent limits.
  child.stdout.on('data', (chunk) => {
    if (streamFailure) return;
    total += chunk.length;
    if (total > 32 * 1024 * 1024) { rejectOutput('outputLimit'); return; }
    for (const byte of chunk) {
      if (byte === 10) frame = 0;
      else if (++frame > 2 * 1024 * 1024) { rejectOutput('outputLimit'); return; }
    }
    try { decoder.decode(chunk, { stream: true }); }
    catch { rejectOutput('incompatible'); }
  });
  child.stdout.on('end', () => {
    stdoutEof = true;
    try { decoder.decode(); }
    catch { rejectOutput('incompatible'); }
  });
  bounded.on('error', () => {});
  child.stdout.on('error', () => { failStream('unavailable'); bounded.destroy(new Error('unavailable')); });
  child.stdout.pipe(bounded);
  child.stderr.on('data', (chunk) => {
    diagnostics += chunk.length;
    if (diagnostics > 256 * 1024) { failStream('outputLimit'); child.stderr.destroy(); }
  });
  child.stderr.on('error', () => failStream('unavailable'));
  child.stderr.on('end', () => { stderrEof = true; });
  return {
    stdin: child.stdin, stdout: bounded,
    get killed() { return child.killed; },
    get exitCode() { return child.exitCode; },
    get signalCode() { return child.signalCode; },
    kill: child.kill.bind(child), on: child.on.bind(child),
    once: child.once.bind(child), off: child.off.bind(child),
  };
}

async function waitForChild(milliseconds) {
  if (childClosed) return true;
  let timer;
  try {
    await Promise.race([childClose, new Promise((resolve) => { timer = setTimeout(resolve, milliseconds); })]);
    return childClosed;
  } finally { clearTimeout(timer); }
}

async function releaseChild() {
  // The SDK has finished its cleanup. Drain the raw pipe through its validation
  // tap even if it stopped consuming/destroyed the readable we gave it.
  if (child?.stdout) { child.stdout.unpipe(sdkOutput); child.stdout.resume(); }
  sdkOutput?.resume();
  if (!child || childClosed) return true;
  // ChildProcess owns its live process handle; do not signal a published raw
  // PID after exit. The native owner also kills/reaps its retained host group.
  if (child.exitCode === null && child.signalCode === null) child.kill('SIGTERM');
  if (await waitForChild(1000)) return true;
  if (child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
  return waitForChild(1000);
}

let catalog;
let failure;
try {
  const configuration = await readConfiguration();
  if (controller.signal.aborted) throw new Error('cancelled');
  const sdk = await import('./sdk.mjs');
  if (controller.signal.aborted) throw new Error('cancelled');
  catalog = await discoverClaudeModels({
    version: '0.3.300',
    query: (request) => sdk.query({
      ...request,
      options: { ...request.options, spawnClaudeCodeProcess: (options) => ownedSpawn(configuration, options) },
    }),
  }, configuration, controller.signal);
} catch (error) {
  failure = Object.prototype.hasOwnProperty.call(exits, error?.kind) ? error.kind : 'unavailable';
} finally {
  try { if (!(await releaseChild())) failure = 'cleanupFailed'; }
  catch { failure = 'cleanupFailed'; }
  process.removeListener('SIGINT', cancel);
  process.removeListener('SIGTERM', cancel);
}
if (failure !== 'cleanupFailed') {
  failure = streamFailure ?? (externalCancelled ? 'cancelled' : failure);
  if (!failure && (!child || child.exitCode !== 0 || child.signalCode !== null || !stdoutEof || !stderrEof)) failure = 'unavailable';
}
if (failure) process.exitCode = exits[failure];
else {
  // No output is emitted until SDK cleanup and independent child completion.
  // The Rust owner still requires host EOF/success/group cleanup before use.
  await new Promise((resolve, reject) => process.stdout.write(catalog, (error) => error ? reject(error) : resolve()));
}
