import type { ExplanationFailureCode } from '../workspace/model.ts';

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
  if (error === 'The answer did not publish any sections.') return explanationFailureMessage('noPublishedSections')!;
  if (
    /^(?:The private Codex file could not be (?:created|saved)\. )?Local storage is full\. Free disk space, then try again\.$/u.test(error)
  ) {
    return 'Code Atlas could not prepare the selected code because local storage is full. Free some space, then try again.';
  }
  if (/^The AI explanation does not (?:name its declared source identifier|connect source identifier) '[^']+'(?: to its cited code)?\. No draft was saved\.$/u.test(error)) {
    return 'The AI draft did not connect its explanation to the cited code, so it was not saved. Try this question again.';
  }
  return error;
}

/** Explain a finite native failure class without exposing internal diagnostics. */
export function explanationFailureMessage(code: ExplanationFailureCode | undefined): string | undefined {
  switch (code) {
    case 'noPublishedSections':
      return 'I couldn’t verify an explanation from the selected code, so nothing was saved. Try asking about one behavior at a time, or ask how a specific caller or helper changes the result.';
    case 'contextTimeout':
      return 'The selected code took too long to gather. Try narrowing the selection to the key lines.';
    case 'runTimeout':
      return 'This explanation took too long to finish. Try a narrower question or select the key function.';
    case 'toolEvent':
    case 'invalidToolArguments':
      return 'The code reading step could not complete safely. Try the question again with a smaller code selection.';
    case 'runtimeIdentity':
      return 'The connected Codex session changed during this explanation. Reconnect and try again.';
    case 'contextPeerClosed':
    case 'contextTransport':
    case 'contextDelivery':
      return 'The code context could not be delivered to the explanation. Check the connection and try again.';
    case 'finalManifestTooLarge':
    case 'invalidFinalManifest':
    case 'unsupportedManifestVersion':
    case 'invalidIncompleteTopics':
      return 'The explanation could not be saved in a usable form. Try a more focused question.';
    case 'stopped':
      return 'Stopped before an explanation was saved.';
    case 'unknown':
      return 'This explanation could not be completed. Try again.';
    default:
      return undefined;
  }
}
