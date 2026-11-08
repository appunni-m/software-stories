/** Return a safe current time to save when the loaded audio player is being
 * hidden or disposed. A pending restore must keep its saved time instead. */
export function playbackPositionToPersist(
  player: { readyState: number; currentTime: number } | null,
  restorePending: boolean,
): number | undefined {
  if (!player || player.readyState < 1 || restorePending
    || !Number.isFinite(player.currentTime) || player.currentTime < 0) return undefined;
  return Math.min(player.currentTime, 3600);
}

/** Consume a queued autoplay request once the media element can play. */
export function takePendingAutoplay(intent: { current: boolean }): boolean {
  const shouldAutoplay = intent.current;
  intent.current = false;
  return shouldAutoplay;
}

/** Cancel autoplay that is waiting for a media readiness event. */
export function cancelPendingAutoplay(intent: { current: boolean }): void {
  intent.current = false;
}
