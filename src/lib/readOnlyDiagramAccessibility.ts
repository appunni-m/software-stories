import type { AriaLabelConfig } from '@xyflow/react';

/** Read-only evidence diagrams leave navigation and source inspection to their labeled controls. */
export const readOnlyDiagramAriaLabelConfig = {
  'node.a11yDescription.default': 'This read-only point has a labeled control for inspecting its code.',
  'node.a11yDescription.keyboardDisabled': 'Use this point’s labeled control to inspect its code; the diagram itself cannot be edited.',
  'edge.a11yDescription.default': 'This read-only relationship cannot be edited. Use a connected point’s control to inspect code.',
} satisfies Partial<AriaLabelConfig>;
