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
