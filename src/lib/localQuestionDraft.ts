export type LocalQuestionDraftIssue = 'unreadable' | 'unsupported' | 'invalid' | null;
export interface LocalQuestionDraft {
  question: string;
  issue: LocalQuestionDraftIssue;
}

const MAX_DRAFT_BYTES = 2_048;
const MAX_QUESTION_CHARACTERS = 300;
const CONTROL_CHARACTERS = /[\u0000-\u001f\u007f-\u009f]/u;

function validQuestion(value: unknown): value is string {
  return typeof value === 'string'
    && value.length <= MAX_QUESTION_CHARACTERS * 2
    && [...value].length <= MAX_QUESTION_CHARACTERS
    && !CONTROL_CHARACTERS.test(value);
}

export function parseLocalQuestionDraft(serialized: string | null): LocalQuestionDraft {
  if (serialized === null) return { question: '', issue: null };
  if (serialized.length > MAX_DRAFT_BYTES) return { question: '', issue: 'invalid' };
  if (new TextEncoder().encode(serialized).byteLength > MAX_DRAFT_BYTES) return { question: '', issue: 'invalid' };
  let value: unknown;
  try { value = JSON.parse(serialized); }
  catch { return { question: '', issue: 'unreadable' }; }
  if (!value || typeof value !== 'object' || Array.isArray(value)) return { question: '', issue: 'invalid' };
  const record = value as Record<string, unknown>;
  if (!Object.hasOwn(record, 'schemaVersion') || record.schemaVersion !== 1) return { question: '', issue: 'unsupported' };
  if (Object.keys(record).length !== 2 || !Object.hasOwn(record, 'question') || !validQuestion(record.question)) {
    return { question: '', issue: 'invalid' };
  }
  return { question: record.question, issue: null };
}

export function serializeLocalQuestionDraft(question: string): string | null {
  if (!validQuestion(question)) return null;
  const serialized = JSON.stringify({ schemaVersion: 1, question });
  return new TextEncoder().encode(serialized).byteLength <= MAX_DRAFT_BYTES ? serialized : null;
}
