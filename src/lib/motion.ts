/** Use non-animated navigation when the user has requested reduced motion. */
export function scrollBehaviorForMotionPreference(reducedMotion: boolean): ScrollBehavior {
  return reducedMotion ? 'auto' : 'smooth';
}
