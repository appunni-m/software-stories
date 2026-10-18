import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { describe, it } from 'node:test';
import ts from 'typescript';
import { initialNavigationState, navigateBack, navigateForward, navigateToWorkspace } from '../src/lib/navigation.ts';
import { loadShellStartupData, PRIMARY_WORKSPACES, WORKSPACE_COPY, workspaceHeading } from '../src/lib/shell.ts';

const allWorkspaces = ['atlas', 'code-lens', 'walkthrough', 'guide-plan', 'chapter-studio', 'changes'];

describe('desktop shell acceptance contract', () => {
  it('names the captured conversation workspace consistently without relabeling tools or empty states', () => {
    for (const workspace of allWorkspaces) {
      assert.equal(workspaceHeading(workspace, false), WORKSPACE_COPY[workspace]);
      const copy = workspaceHeading(workspace, true);
      if (['atlas', 'code-lens', 'guide-plan'].includes(workspace)) {
        assert.equal(copy.title, 'Explore this codebase.');
        assert.match(copy.subtitle, /question.*guide.*flow.*source/);
      } else {
        assert.equal(copy, WORKSPACE_COPY[workspace]);
      }
    }
  });

  it('opens local file browsing with no project and exposes the simple primary destinations', () => {
    assert.deepEqual(initialNavigationState.current, {
      workspace: 'atlas', projectId: null, snapshotId: null, focusTarget: 'atlas:open-repository',
    });
    assert.deepEqual(PRIMARY_WORKSPACES.map(({ id, label }) => [id, label]), [
      ['guide-plan', 'Guide'],
      ['atlas', 'Files'],
      ['changes', 'Changes'],
    ]);
    assert.deepEqual(Object.keys(WORKSPACE_COPY).sort(), [...allWorkspaces].sort());
    for (const workspace of allWorkspaces) {
      const copy = WORKSPACE_COPY[workspace];
      assert.ok(copy.eyebrow.length > 0, `${workspace} has an eyebrow`);
      assert.ok(copy.title.length > 0, `${workspace} has a title`);
      assert.ok(copy.subtitle.length > 0, `${workspace} has explanatory empty-state copy`);
    }
  });

  it('reaches every workspace while preserving one project and snapshot selection', () => {
    let state = {
      ...initialNavigationState,
      current: { ...initialNavigationState.current, projectId: 'project-1', snapshotId: 'snapshot-1' },
    };
    for (const workspace of allWorkspaces.slice(1)) {
      state = navigateToWorkspace(state, workspace);
      assert.equal(state.current.workspace, workspace);
      assert.equal(state.current.projectId, 'project-1');
      assert.equal(state.current.snapshotId, 'snapshot-1');
    }
    for (let index = 0; index < allWorkspaces.length - 1; index += 1) state = navigateBack(state);
    assert.equal(state.current.workspace, 'atlas');
    assert.equal(state.current.projectId, 'project-1');
    assert.equal(state.current.snapshotId, 'snapshot-1');
    while (state.forward.length > 0) state = navigateForward(state);
    assert.equal(state.current.workspace, allWorkspaces.at(-1));
  });

  it('loads startup data through only the safe shell command boundary and tolerates independent failures', async () => {
    const calls = [];
    const forbiddenCalls = [];
    const recent = [{ projectId: 'p', displayName: 'Repo' }];
    const result = await loadShellStartupData({
      getApplicationStatus: async () => { calls.push('getApplicationStatus'); return { productName: 'Code Atlas', version: '0.1.0', native: true }; },
      getCaptureLimits: async () => { calls.push('getCaptureLimits'); return { maxFileBytes: 1 }; },
      listRecentProjects: async () => { calls.push('listRecentProjects'); return recent; },
      listInterruptedOperations: async () => { calls.push('listInterruptedOperations'); return []; },
      listPendingGeneratedChapterPreviews: async () => { calls.push('listPendingGeneratedChapterPreviews'); return []; },
      inspectRepository: async () => { forbiddenCalls.push('inspectRepository'); },
      getProviderCredentialStatus: async () => { forbiddenCalls.push('getProviderCredentialStatus'); },
      sendGeminiRequest: async () => { forbiddenCalls.push('sendGeminiRequest'); },
      synthesizeReviewedNarrationAudio: async () => { forbiddenCalls.push('synthesizeReviewedNarrationAudio'); },
    });
    assert.deepEqual(calls.sort(), ['getApplicationStatus', 'getCaptureLimits', 'listInterruptedOperations', 'listPendingGeneratedChapterPreviews', 'listRecentProjects']);
    assert.deepEqual(forbiddenCalls, []);
    assert.equal(result.applicationStatus?.native, true);
    assert.deepEqual(result.recentProjects, recent);
    assert.equal(result.applicationStatusUnavailable, false);
    assert.equal(result.recentProjectsUnavailable, false);
    assert.deepEqual(result.interruptedOperations, []);
    assert.equal(result.interruptedOperationsUnavailable, false);
    assert.deepEqual(result.pendingChapterPreviews, []);
    assert.equal(result.pendingChapterPreviewsUnavailable, false);

    const degraded = await loadShellStartupData({
      getApplicationStatus: async () => { throw new Error('bridge down'); },
      getCaptureLimits: async () => ({ maxFileBytes: 1 }),
      listRecentProjects: async () => { throw new Error('storage unavailable'); },
      listInterruptedOperations: async () => { throw new Error('recovery data unavailable'); },
      listPendingGeneratedChapterPreviews: async () => { throw new Error('saved previews unavailable'); },
    });
    assert.equal(degraded.applicationStatusUnavailable, true);
    assert.equal(degraded.applicationStatus, null);
    assert.equal(degraded.captureLimits.maxFileBytes, 1);
    assert.equal(degraded.recentProjectsUnavailable, true);
    assert.equal(degraded.recentProjects, null);
    assert.equal(degraded.interruptedOperationsUnavailable, true);
    assert.equal(degraded.interruptedOperations, null);
    assert.equal(degraded.pendingChapterPreviewsUnavailable, true);
    assert.equal(degraded.pendingChapterPreviews, null);
  });

  it('keeps recovery actions in the notification center without occupying the workspace', async () => {
    const source = await readFile(new URL('../src/App.tsx', import.meta.url), 'utf8');
    assert.match(source, /title: 'Recent projects could not be loaded'[\s\S]*?target: \{kind:'retryRecents'\}/);
    assert.match(source, /target\?\.kind==='retryRecents'\)\{void retryRecentProjectsLoad\(\)/);
    assert.match(source, /<NotificationCenter items=\{workspaceNotifications\}/);
    assert.match(source, /Try update again/);
    assert.match(source, /retryInterruptedCapture\(operation\) : reviewInterruptedGeneration\(operation\)/);
    assert.match(source, /run: \(\) => \{ void discardRecovery\(operation\); \}/);
    assert.doesNotMatch(source, /className="notice-banner notice-warning interrupted-work"/);
  });

  it('makes local Recents available before slower recovery lookups finish', async () => {
    let releaseRecovery;
    const recovery = new Promise((resolve) => { releaseRecovery = resolve; });
    let received;
    const recentsReady = new Promise((resolve) => { received = resolve; });
    let finished = false;
    const pending = loadShellStartupData({
      getApplicationStatus: async () => ({ native: true }),
      getCaptureLimits: async () => ({}),
      listRecentProjects: async () => [{ projectId: 'saved' }],
      listInterruptedOperations: () => recovery,
      listPendingGeneratedChapterPreviews: async () => [],
    }, received).then((value) => { finished = true; return value; });
    assert.deepEqual(await recentsReady, [{ projectId: 'saved' }]);
    assert.equal(finished, false);
    releaseRecovery([]);
    assert.equal((await pending).recentProjectsUnavailable, false);
  });

  it('releases the initial view when Recents fails even while recovery is pending', async () => {
    let releaseRecovery;
    const recovery = new Promise((resolve) => { releaseRecovery = resolve; });
    let received;
    const recentsReady = new Promise((resolve) => { received = resolve; });
    const pending = loadShellStartupData({
      getApplicationStatus: async () => ({ native: true }),
      getCaptureLimits: async () => ({}),
      listRecentProjects: async () => { throw new Error('local storage unavailable'); },
      listInterruptedOperations: () => recovery,
      listPendingGeneratedChapterPreviews: async () => [],
    }, received);
    assert.equal(await recentsReady, null);
    releaseRecovery([]);
    assert.equal((await pending).recentProjectsUnavailable, true);
  });

  it('restores persisted previews for local review with only explicit save or discard actions', async () => {
    const source = await readFile(new URL('../src/App.tsx', import.meta.url), 'utf8');
    const commands = await readFile(new URL('../src/lib/commands.ts', import.meta.url), 'utf8');
    assert.match(source, /listPendingGeneratedChapterPreviews/);
    assert.match(source, /label: 'Open explanation'[\s\S]*?setRecoveredPreviewId\(preview.generationId\)/);
    assert.match(source, /Closing keeps this preview in local recovery/);
    assert.match(source, /Save unaccepted draft/);
    assert.match(source, /No provider request will be sent when you review or save it/);
    assert.match(commands, /loadPendingGeneratedChapterPreview[\s\S]*?load_pending_generated_chapter_preview/);
    const restoredDialog = source.slice(source.indexOf('function RecoveredGeneratedPreviewDialog'), source.indexOf('function GeneratedChapterPreview'));
    assert.doesNotMatch(restoredDialog, /sendGeminiRequest|buildGeminiRequestReview|acceptChapterDraft/);
    assert.match(restoredDialog, /discardGeneratedChapterDraft/);
    assert.match(restoredDialog, /saveGeneratedChapterDraft/);
  });

  it('offers provider setup only from an explicit generation or narration flow', async () => {
    const source = await readFile(new URL('../src/App.tsx', import.meta.url), 'utf8');
    const menuStart = source.indexOf('<div className="tools-menu"');
    const menuEnd = source.indexOf('<button className="menu-scrim"', menuStart);
    assert.ok(menuStart >= 0 && menuEnd > menuStart, 'project tools menu is present');
    assert.doesNotMatch(source.slice(menuStart, menuEnd), /AI and narration setup/);
    const narration = await readFile(new URL('../src/NarrationPlayer.tsx', import.meta.url), 'utf8');
    assert.match(narration, /Set up Gemini for narration/);
    assert.match(source.slice(source.indexOf('function ProviderDialog')), /provider-key-card/);
  });

  it('keeps App startup behind the shell bootstrap rather than invoking repo or provider commands directly', async () => {
    const source = await readFile(new URL('../src/App.tsx', import.meta.url), 'utf8');
    const file = ts.createSourceFile('App.tsx', source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
    const forbidden = new Set([
      'pickProjectFolder', 'openRecentProject', 'startSnapshot', 'startSnapshotIndexing', 'readSnapshotSource',
      'getProviderCredentialStatus', 'getProviderDisclosure', 'buildGeminiRequestReview', 'sendGeminiRequest',
      'getNarrationDisclosure', 'buildNarrationRequestReview', 'synthesizeReviewedNarrationAudio',
    ]);
    let bootstrapEffectCount = 0;
    const visit = (node) => {
      if (ts.isCallExpression(node) && node.expression.getText(file) === 'useEffect'
        && node.arguments[0] && ts.isArrowFunction(node.arguments[0])
        && node.arguments[0].getText(file).includes('loadShellStartupData')) {
        bootstrapEffectCount += 1;
        const effectCalls = [];
        const visitEffect = (child) => {
          if (ts.isCallExpression(child) && ts.isIdentifier(child.expression)) effectCalls.push(child.expression.text);
          ts.forEachChild(child, visitEffect);
        };
        visitEffect(node.arguments[0]);
        assert.deepEqual(effectCalls.filter((name) => forbidden.has(name)), []);
        assert.ok(effectCalls.includes('loadShellStartupData'));
      }
      ts.forEachChild(node, visit);
    };
    visit(file);
    assert.equal(bootstrapEffectCount, 1);
  });

  it('configures one resizable zoom-capable desktop window without broad filesystem or shell access', async () => {
    const config = JSON.parse(await readFile(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'));
    const capability = JSON.parse(await readFile(new URL('../src-tauri/capabilities/default.json', import.meta.url), 'utf8'));
    assert.equal(config.app.windows.length, 1);
    assert.equal(config.app.windows[0].label, 'main');
    assert.equal(config.app.windows[0].resizable, true);
    assert.ok(config.app.windows[0].minWidth >= 1024);
    assert.ok(config.app.windows[0].minHeight >= 680);
    // Menus, keyboard and restoration share the native zoom owner. Disable the
    // injected polyfill and its renderer permission to avoid competing levels.
    assert.equal(config.app.windows[0].zoomHotkeysEnabled, false);
    assert.deepEqual(capability.permissions, ['core:default']);
    const bootstrap=await readFile(new URL('../src/main.tsx',import.meta.url),'utf8');
    assert.match(bootstrap,/await installAppZoom\(\);/);

    const source = await readFile(new URL('../src/KeyboardShortcuts.tsx', import.meta.url), 'utf8');
    assert.match(source, /Zoom in \/ out \(20% steps\)/);
    assert.match(source, /⌘= \/ ⌘−/);
  });

  it('opens standard keyboard help explicitly without adding a saved notification', async () => {
    const [app,help]=await Promise.all(['../src/App.tsx','../src/KeyboardShortcuts.tsx'].map(path=>readFile(new URL(path,import.meta.url),'utf8')));
    assert.doesNotMatch(app,/showComingSoon/);
    assert.match(app,/<KeyboardShortcuts open=\{shortcutsOpen\} onOpenChange=\{setShortcutsOpen\}/);
    assert.match(app,/if \(shortcutsOpen\) return;/);
    assert.match(help,/<Dialog.Root open=\{open\} onOpenChange=\{onOpenChange\}/);
    assert.match(help,/<Dialog.Title>Keyboard shortcuts/);
    assert.match(help,/data-focus-key="tools-button"/);
    assert.doesNotMatch(help,/noticeHistory|invoke|fetch/);
  });
});
