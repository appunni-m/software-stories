import type { AriaLabelConfig } from '@xyflow/react';

/** Read-only evidence diagrams leave navigation and source inspection to their labeled controls. */
export const readOnlyDiagramAriaLabelConfig = {
  'node.a11yDescription.default': 'This diagram node is read-only and cannot be selected or moved. Use its source button when available.',
  'node.a11yDescription.keyboardDisabled': 'This diagram node is read-only and cannot be selected or moved. Use its source button when available.',
  'edge.a11yDescription.default': 'This diagram relationship is read-only and cannot be selected or edited.',
} satisfies Partial<AriaLabelConfig>;
