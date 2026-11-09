export interface InlineSectionDraft {
  key: string;
  baseRevisionId: string;
  selectedRevisionId: string;
  title: string;
  prose: string;
  spokenText: string;
}

export interface InlineSectionEditFeedback {
  key: string;
  text: string;
  tone: 'success' | 'error';
}

export interface InlineSectionEditIssues {
  key: string;
  items: string[];
}

export interface InlineSectionEditEntry {
  draft: InlineSectionDraft | null;
  reviseQuestion: string;
  reviseOpen: boolean;
  busy: boolean;
  feedback: InlineSectionEditFeedback | null;
  issues: InlineSectionEditIssues | null;
}

export type InlineSectionEditEntries = Record<string, InlineSectionEditEntry>;

export const EMPTY_INLINE_SECTION_EDIT_ENTRY: InlineSectionEditEntry = Object.freeze({
  draft: null,
  reviseQuestion: '',
  reviseOpen: false,
  busy: false,
  feedback: null,
  issues: null,
});

/** Update one immutable section's editor state without discarding other open drafts. */
export function updateInlineSectionEditEntries(
  entries: InlineSectionEditEntries,
  key: string,
  update: (previous: InlineSectionEditEntry) => InlineSectionEditEntry,
): InlineSectionEditEntries {
  if (!key) return entries;
  const previous = entries[key] ?? EMPTY_INLINE_SECTION_EDIT_ENTRY;
  const next = update(previous);
  if (next === previous) return entries;

  const empty = !next.draft && !next.reviseQuestion && !next.reviseOpen && !next.busy && !next.feedback && !next.issues;
  if (empty) {
    if (!(key in entries)) return entries;
    const remaining = { ...entries };
    delete remaining[key];
    return remaining;
  }
  return { ...entries, [key]: next };
}
