import type { ExplanationFailureCode } from '../workspace/model.ts';
import type { ProviderFailure } from './commands';

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

/** Keep provider request failures actionable and consistent across workflows. */
export function providerFailureMessage(failure: ProviderFailure): string {
  // Only native-authored rejection instructions are safe to display verbatim.
  if (failure.kind === 'providerRejectedRequest' && failure.message.trim()) return failure.message;
  const messages: Partial<Record<ProviderFailure['kind'], string>> = {
    keychainUnavailable: 'Secure credential storage is unavailable on this Mac.',
    keychainOperationFailed: 'The saved provider credential could not be accessed. Check Connections and try again.',
    missingApiKey: 'Add the selected provider key in Connections before generating.',
    invalidApiKey: 'The selected provider key was rejected. Check it in Connections.',
    consentRequired: 'Review and confirm this request before generating.',
    reviewExpired: 'This review expired. Build a fresh request and confirm it again.',
    reviewChanged: 'The request changed after review. Build a fresh review before sending.',
    reviewUnavailable: 'This review is no longer available. Build a fresh request.',
    contextTooLarge: 'There is too much code to review in one request, so nothing was sent. Ask Guide about one behavior so Code Atlas can focus the source selection, or remove supporting files.',
    secretLikeContext: 'The selected context contains a blocked secret-like path. Remove it before reviewing again.',
    unavailableModel: 'The selected model or reasoning setting is unavailable. Refresh the model list and choose again.',
    billingRequired: 'The provider requires a billing-enabled account for this request. No other provider was selected.',
    quotaExceeded: 'The connected account quota is unavailable or exhausted. Check the account before trying again.',
    regionUnavailable: 'This provider is unavailable in the current region. Choose another connection explicitly.',
    requestCancelled: 'Generation was cancelled. No response was accepted.',
    requestTimedOut: 'The provider did not finish this request. No draft was accepted; build a fresh review before retrying.',
    networkUnavailable: 'The provider could not complete this request. Check the connection, then build a fresh review.',
    malformedProviderResponse: 'The response did not pass format or source checks. No response was accepted.',
    providerResponseTooLarge: 'The response was too large to save. Try a more focused question.',
    localStorageUnavailable: 'Local storage is unavailable. Existing saved work was preserved.',
    invalidReview: 'The local request could not be validated. Check the selected snapshot and connection.',
  };
  return messages[failure.kind] ?? 'The selected AI connection could not complete this request. Check its setup and try again.';
}

/** Keep current source-validation identifiers out of the reading workspace. */
export function explanationErrorMessage(error: string): string {
  if (error === 'The answer did not publish any sections.') return explanationFailureMessage('noPublishedSections')!;
  if (/^This conversation changed in another window\./u.test(error)) {
    return 'Your question is still here, but it was not sent because this conversation changed in another window. Choose “Keep this window” or “Keep saved value” above, then try again.';
  }
  if (
    /^(?:The private Codex file could not be (?:created|saved)\. )?Local storage is full\. Free disk space, then try again\.$/u.test(error)
  ) {
    return 'Code Atlas could not prepare the selected code because local storage is full. Free some space, then try again.';
  }
  if (
    /^The AI explanation does not (?:name its declared source identifier|connect source identifier) '[^']+'(?: to its cited code)?\. No draft was saved\.$/u.test(error)
    || /^Claim(?: '[^']+')? in section '[^']+' does not name its declared identifier '[^']+' in the explanation\.$/u.test(error)
  ) {
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
