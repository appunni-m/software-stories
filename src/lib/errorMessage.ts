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

/** Keep current source-validation identifiers out of the reading workspace. */
export function explanationErrorMessage(error: string): string {
  if (/^The AI explanation does not (?:name its declared source identifier|connect source identifier) '[^']+'(?: to its cited code)?\. No draft was saved\.$/u.test(error)) {
    return 'The AI draft did not connect its explanation to the cited code, so it was not saved. Try this question again.';
  }
  return error;
}
