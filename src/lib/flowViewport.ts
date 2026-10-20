import type { FlowViewport } from './readerProgress';

export interface FlowViewportInstance {
  viewportInitialized: boolean;
  getNodesBounds: (nodes: string[]) => { x: number; y: number; width: number; height: number };
  getViewport: () => FlowViewport;
  setViewport: (viewport: FlowViewport, options?: { duration?: number }) => Promise<boolean>;
  fitView: (options: { nodes: { id: string }[]; padding: number; minZoom: number; maxZoom: number }) => Promise<boolean>;
}

/** Saved viewports belong to a particular canvas size. Preserve them while
 * their visible nodes fit, then reframe the scene after a pane resize.
 */
export function viewportClipsNodeBounds(
  bounds: { x: number; y: number; width: number; height: number },
  viewport: FlowViewport,
  width: number,
  height: number,
  padding = 24,
): boolean {
  if (![bounds.x, bounds.y, bounds.width, bounds.height, viewport.x, viewport.y, viewport.zoom, width, height, padding].every(Number.isFinite)
    || bounds.width <= 0 || bounds.height <= 0 || viewport.zoom <= 0 || width <= 0 || height <= 0 || padding < 0) return false;

  const left = bounds.x * viewport.zoom + viewport.x;
  const top = bounds.y * viewport.zoom + viewport.y;
  const right = (bounds.x + bounds.width) * viewport.zoom + viewport.x;
  const bottom = (bounds.y + bounds.height) * viewport.zoom + viewport.y;
  return left < padding || top < padding || right > width - padding || bottom > height - padding;
}
