export type WorkspaceViewShortcut = 'guide' | 'flow' | 'history';

/** The reader tabs are one workspace; these shortcuts must never route to a different screen. */
export function workspaceViewForShortcut(key: string): WorkspaceViewShortcut | null {
  switch (key.toLowerCase()) {
    case '1': return 'guide';
    case '2': return 'flow';
    case '3': return 'history';
    default: return null;
  }
}
