// Candidate orchestration for @anthropic-ai/claude-agent-sdk 0.3.300.
// This library does nothing on import. The native-owned worker must load
// the integrity-verified SDK, clear its environment before import, own/reap its
// process group and bind executable/account observations around this call.
// No renderer command currently invokes this helper. Metadata is not entitlement.
import { isAbsolute } from 'node:path';

export const candidateSdkVersion = '0.3.300';
const candidateRuntimeVersion = '2.1.300';
const catalogLimit = 128 * 1024;
const discoveryTimeoutMs = 30_000;
const cleanupTimeoutMs = 5_000;

const messages = {
  incompatible: 'The Claude discovery adapter or metadata is incompatible. No settings were accepted.',
  cancelled: 'Claude model discovery was cancelled. No model prompt was sent.',
  timedOut: 'Claude model discovery exceeded its local deadline. No settings were accepted.',
  unavailable: 'Claude model metadata could not be obtained. Check the qualified runtime and account route.',
  cleanupFailed: 'Claude SDK cleanup could not be confirmed. The native owner must stop and reap its process group.',
};

export class ClaudeDiscoveryFailure extends Error {
  constructor(kind) {
    if (!Object.prototype.hasOwnProperty.call(messages, kind)) kind = 'unavailable';
    super(messages[kind]);
    this.name = 'ClaudeDiscoveryFailure';
    this.kind = kind;
  }
}

function rejectMetadata() { throw new ClaudeDiscoveryFailure('incompatible'); }
function boundedText(value, limit) {
  return typeof value === 'string' && value.length > 0
    && Buffer.byteLength(value, 'utf8') <= limit && !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
}
function absolutePath(value) {
  return boundedText(value, 4096) && isAbsolute(value) && !value.split('/').includes('..');
}

function projectModel(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)
    || !boundedText(value.value, 200) || !boundedText(value.displayName, 160)
    || (value.resolvedModel !== undefined && !boundedText(value.resolvedModel, 200))
    || (value.supportsEffort !== undefined && typeof value.supportsEffort !== 'boolean')
    || (value.supportedEffortLevels !== undefined && (!Array.isArray(value.supportedEffortLevels)
      || value.supportedEffortLevels.length > 5
      || value.supportedEffortLevels.some((effort) => !boundedText(effort, 16))))) rejectMetadata();
  // Project known model fields only. Never return raw initialization, account,
  // credentials, diagnostics, descriptions or future SDK metadata fields.
  return {
    value: value.value,
    resolvedModel: value.resolvedModel,
    displayName: value.displayName,
    supportsEffort: value.supportsEffort,
    supportedEffortLevels: value.supportedEffortLevels,
  };
}

// Sending an empty string would be a user turn. This iterable yields zero
// messages; the maintained SDK handles initialization and then closes input.
async function* noUserMessages() {}

async function closeQuery(request) {
  let timer;
  try {
    request.close();
    await Promise.race([
      request.return(undefined),
      new Promise((_, reject) => {
        timer = setTimeout(() => reject(new ClaudeDiscoveryFailure('cleanupFailed')), cleanupTimeoutMs);
      }),
    ]);
  } catch {
    throw new ClaudeDiscoveryFailure('cleanupFailed');
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Obtain candidate catalog bytes through the maintained SDK's public methods.
 * `sdk` must be loaded by a trusted native worker from the exact verified
 * package, not selected by the WebView. SDK version strings alone are not
 * provenance. `paths` and the child lifecycle also belong to that native owner.
 * Returned bytes still require native parse_model_catalog and fresh account,
 * executable, isolation and product-route qualification before any generation.
 */
export async function discoverClaudeModels(sdk, paths, signal) {
  if (sdk?.version !== candidateSdkVersion || typeof sdk.query !== 'function'
    || !paths || !['executable', 'stage', 'temporary', 'accountHome'].every((field) => absolutePath(paths[field]))
    || (signal != null && (typeof signal.aborted !== 'boolean'
      || typeof signal.addEventListener !== 'function' || typeof signal.removeEventListener !== 'function'))) {
    rejectMetadata();
  }
  if (signal?.aborted) throw new ClaudeDiscoveryFailure('cancelled');
  const controller = new AbortController();
  let stopReason;
  let rejectStopped;
  const stopped = new Promise((_, reject) => { rejectStopped = reject; });
  // The SDK can throw synchronously before the race is attached. Retain a
  // handler then too; final cancellation checks still reject the observation.
  void stopped.catch(() => {});
  const stop = (kind) => {
    if (stopReason) return;
    stopReason = kind;
    controller.abort();
    rejectStopped(new ClaudeDiscoveryFailure(kind));
  };
  const cancel = () => stop('cancelled');
  signal?.addEventListener('abort', cancel, { once: true });
  const timer = setTimeout(() => stop('timedOut'), discoveryTimeoutMs);
  let request;
  let catalog;
  let error;
  try {
    request = sdk.query({
      prompt: noUserMessages(),
      options: {
        abortController: controller,
        pathToClaudeCodeExecutable: paths.executable,
        cwd: paths.stage,
        env: {
          HOME: paths.accountHome,
          TMPDIR: paths.temporary,
          PATH: '/usr/bin:/bin:/usr/sbin:/sbin',
          LANG: 'en_US.UTF-8',
          DISABLE_AUTOUPDATER: '1',
        },
        settingSources: [],
        tools: [],
        disallowedTools: ['mcp__*'],
        mcpServers: {},
        strictMcpConfig: true,
        permissionMode: 'dontAsk',
        permissionPrompts: 'none',
        settings: { disableAllHooks: true, fallbackModel: [] },
        persistSession: false,
        maxTurns: 1,
        extraArgs: { safe: null, restricted: null, 'disable-slash-commands': null, 'no-chrome': null },
        stderr: () => {},
      },
    });
    const [initialization, models] = await Promise.race([
      Promise.all([request.initializationResult(), request.supportedModels()]),
      stopped,
    ]);
    if (initialization?.claude_code_version !== candidateRuntimeVersion
      || !Array.isArray(models) || models.length === 0 || models.length > 256) rejectMetadata();
    const text = JSON.stringify({
      protocolVersion: 1,
      sdkVersion: candidateSdkVersion,
      runtimeVersion: initialization.claude_code_version,
      models: models.map(projectModel),
    });
    if (Buffer.byteLength(text, 'utf8') > catalogLimit) rejectMetadata();
    catalog = Buffer.from(text, 'utf8');
  } catch (failure) {
    error = failure instanceof ClaudeDiscoveryFailure ? failure : new ClaudeDiscoveryFailure('unavailable');
  } finally {
    try {
      if (request) await closeQuery(request);
    } catch {
      // Cleanup uncertainty takes priority over cancellation or valid metadata.
      error = new ClaudeDiscoveryFailure('cleanupFailed');
    }
    clearTimeout(timer);
    signal?.removeEventListener('abort', cancel);
  }
  if (error?.kind === 'cleanupFailed') throw error;
  if (stopReason || signal?.aborted) throw new ClaudeDiscoveryFailure(stopReason ?? 'cancelled');
  if (error) throw error;
  return catalog;
}
