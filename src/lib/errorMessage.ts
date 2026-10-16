/** Read a useful message from native command rejections and ordinary JS errors. */
export function errorMessageOr(error: unknown, fallback: string): string {
  if (typeof error === 'string' && error.trim()) return error;
  if (error instanceof Error && error.message.trim()) return error.message;
  if (error && typeof error === 'object' && 'message' in error) {
    const message = error.message;
    if (typeof message === 'string' && message.trim()) return message;
  }
  return fallback;
}

/** Keep persisted legacy validation IDs out of the reading workspace. */
export function explanationErrorMessage(error: string): string {
  if (/does not name (?:its declared identifier|its declared source identifier)|does not connect (?:identifier|source identifier)|AI draft still did not connect/.test(error)) {
    return 'The AI draft did not connect its explanation to the cited code, so it was not saved. Try this question again.';
  }
  return error;
}
