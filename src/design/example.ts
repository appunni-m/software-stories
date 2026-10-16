/** Public, illustrative data. This preview never reads a repository or calls AI. */
export const exampleFiles: Record<string, string> = {
  'src/cart.ts': `export function subtotal(prices: number[]): number {
  if (prices.some((price) => !Number.isFinite(price) || price < 0)) {
    throw new RangeError("Prices must be finite and non-negative");
  }
  return prices.reduce((total, price) => total + price, 0);
}

export function totalWithTax(prices: number[], taxRate: number): number {
  if (!Number.isFinite(taxRate) || taxRate < 0) {
    throw new RangeError("Tax rate must be finite and non-negative");
  }
  return subtotal(prices) * (1 + taxRate);
}
`,
  'src/checkout.ts': `import { totalWithTax } from './cart';

export function checkout(prices: number[]) {
  const amount = totalWithTax(prices, 0.1);
  return { amount, currency: 'USD' };
}
`,
  'tests/cart.test.ts': `import { expect, test } from 'vitest';
import { subtotal, totalWithTax } from '../src/cart';

test('adds prices and applies tax', () => {
  expect(subtotal([12, 8])).toBe(20);
  expect(totalWithTax([12, 8], 0.1)).toBe(22);
});

test('rejects negative prices', () => {
  expect(() => subtotal([-1])).toThrow(RangeError);
});
`,
  'package.json': '{\n  "name": "checkout-demo",\n  "type": "module",\n  "scripts": { "test": "vitest" }\n}\n',
  'README.md': '# Checkout demo\n\nA small example used to explore the Code Atlas interface.\n',
};

export type ExplorationView = 'guide' | 'flow' | 'history';
export interface CodeContext { path: string; from?: number; to?: number }
export interface Exploration {
  id: string;
  parentId: string | null;
  question: string;
  view: ExplorationView;
  context: CodeContext[];
  kind: 'overview' | 'validation' | 'tax' | 'limits';
  turns?: { id: string; question: string; context: CodeContext[] }[];
}
export interface PreviewState { explorations: Exploration[]; currentId: string; file: string; theme: 'dark' | 'light' }
export const initialPreview: PreviewState = {
  explorations: [{ id: 'root', parentId: null, question: 'How does checkout calculate the total?', view: 'guide', context: [], kind: 'overview' }],
  currentId: 'root', file: 'src/cart.ts', theme: 'dark',
};

/** Context and identity stay separate from the presentation selected for it. */
export function explorationPath(items: Exploration[], id: string): Exploration[] {
  const path: Exploration[] = [];
  const seen = new Set<string>();
  let node = items.find((item) => item.id === id);
  while (node && !seen.has(node.id)) {
    seen.add(node.id); path.unshift(node);
    node = node.parentId ? items.find((item) => item.id === node!.parentId) : undefined;
  }
  return path;
}

export function findExampleCode(query: string): { path: string; line?: number; text: string }[] {
  const q = query.trim().toLowerCase();
  if (!q) return [];
  return Object.entries(exampleFiles).flatMap(([path, contents]) => {
    if (path.toLowerCase().includes(q)) return [{ path, text: path }];
    return contents.split('\n').flatMap((text, index) => text.toLowerCase().includes(q) ? [{ path, line: index + 1, text: text.trim() }] : []);
  }).slice(0, 6);
}

export function readPreview(): PreviewState {
  try {
    const saved = JSON.parse(localStorage.getItem('code-atlas:workspace-design:v1') ?? 'null') as PreviewState | null;
    if (saved && saved.explorations?.length && saved.explorations.every((e) => typeof e.id === 'string' && typeof e.question === 'string' && ['guide', 'flow', 'history'].includes(e.view) && Array.isArray(e.context))
      && saved.explorations.some((e) => e.id === saved.currentId) && exampleFiles[saved.file] && ['dark', 'light'].includes(saved.theme)) return saved;
  } catch { /* A preview preference must never block opening the design. */ }
  return initialPreview;
}
