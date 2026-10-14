import assert from 'node:assert/strict';
import { readdir, readFile } from 'node:fs/promises';
import path from 'node:path';
import test from 'node:test';
import ts from 'typescript';
import { scrollBehaviorForMotionPreference } from '../src/lib/motion.ts';

const css = await readFile(new URL('../src/App.css', import.meta.url), 'utf8');
const sourceRoot = new URL('../src/', import.meta.url);
const surfaces = ['#0b0e12', '#10141a', '#141a21', '#1a222b'];
const nameableRoles = new Set([
  'alert', 'alertdialog', 'application', 'article', 'button', 'complementary', 'dialog',
  'figure', 'form', 'grid', 'group', 'list', 'listbox', 'main', 'menu', 'navigation',
  'region', 'row', 'status', 'table', 'tablist', 'tabpanel', 'toolbar', 'tree', 'treegrid',
  'treeitem',
]);

async function collectTsxFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async (entry) => {
    const file = new URL(`${entry.name}${entry.isDirectory() ? '/' : ''}`, directory);
    if (entry.isDirectory()) return collectTsxFiles(file);
    return entry.isFile() && entry.name.endsWith('.tsx') ? [file] : [];
  }));
  return nested.flat();
}

function jsxAttribute(element, name) {
  return element.attributes.properties.find((property) =>
    ts.isJsxAttribute(property) && property.name.getText() === name);
}

function roleIsNameable(roleAttribute, sourceFile) {
  if (!roleAttribute?.initializer) return false;
  if (ts.isStringLiteral(roleAttribute.initializer)) return nameableRoles.has(roleAttribute.initializer.text);

  const possibleRoles = [];
  const collect = (node) => {
    if (ts.isStringLiteral(node)) possibleRoles.push(node.text);
    ts.forEachChild(node, collect);
  };
  collect(roleAttribute.initializer);
  return possibleRoles.length > 0 && possibleRoles.every((role) => nameableRoles.has(role))
    && roleAttribute.initializer.getText(sourceFile).includes('undefined');
}

function readTokens(block) {
  return Object.fromEntries(
    [...block.matchAll(/--([\w-]+):\s*(#[\da-f]{6})\s*;/gi)]
      .map(([, name, value]) => [name, value.toLowerCase()]),
  );
}

function relativeLuminance(hex) {
  const channels = [1, 3, 5].map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16) / 255);
  const linear = channels.map((value) => value <= 0.04045
    ? value / 12.92
    : ((value + 0.055) / 1.055) ** 2.4);
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
}

function contrastRatio(foreground, background) {
  const luminances = [relativeLuminance(foreground), relativeLuminance(background)]
    .sort((left, right) => right - left);
  return (luminances[0] + 0.05) / (luminances[1] + 0.05);
}

function resolveSolidColor(value, tokens) {
  const normalized = value.trim().toLowerCase();
  const token = /^var\(--([\w-]+)\)$/.exec(normalized);
  const color = token ? tokens[token[1]] : normalized;
  return color && /^#[\da-f]{6}$/.test(color) ? color : undefined;
}

const rootBlock = css.match(/:root\s*\{([\s\S]*?)\n\}/)?.[1];
const highContrastBlock = css.match(/@media\s*\(prefers-contrast:\s*more\)\s*\{\s*:root\s*\{([\s\S]*?)\n\s{2}\}/)?.[1];

test('default dim token remains readable against every application surface', () => {
  assert.ok(rootBlock, 'root design tokens are present');
  const tokens = readTokens(rootBlock);
  for (const surface of surfaces) {
    assert.ok(
      contrastRatio(tokens.dim, surface) >= 4.5,
      `--dim ${tokens.dim} must have 4.5:1 contrast against ${surface}`,
    );
  }
});

test('Increase Contrast preference strengthens text, boundaries, and focus indicators', () => {
  assert.ok(highContrastBlock, 'a prefers-contrast: more token override is present');
  const tokens = readTokens(highContrastBlock);
  for (const token of ['muted', 'dim', 'border', 'border-strong']) {
    assert.ok(tokens[token], `high contrast defines --${token}`);
  }
  for (const surface of surfaces) {
    assert.ok(contrastRatio(tokens.muted, surface) >= 4.5, `--muted is readable on ${surface}`);
    assert.ok(contrastRatio(tokens.dim, surface) >= 4.5, `--dim is readable on ${surface}`);
    assert.ok(contrastRatio(tokens.border, surface) >= 3, `--border distinguishes components on ${surface}`);
    assert.ok(contrastRatio(tokens['border-strong'], surface) >= 3, `--border-strong distinguishes components on ${surface}`);
  }
  assert.match(css, /@media\s*\(prefers-contrast:\s*more\)[\s\S]*?:where\(\*\)\s*\{\s*border-color:\s*var\(--border-strong\)\s*!important;/);
  assert.match(css, /@media\s*\(prefers-contrast:\s*more\)[\s\S]*?outline:\s*3px solid #ffffff\s*!important;/);
});

test('solid text and surface colors in the same CSS rule meet normal-text contrast', () => {
  assert.ok(rootBlock, 'root design tokens are present');
  const defaultTokens = readTokens(rootBlock);
  const contrastTokens = { ...defaultTokens, ...readTokens(highContrastBlock ?? '') };
  const failingPairs = [];

  for (const [, selector, declarations] of css.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
    const foregroundValues = [...declarations.matchAll(/(?<![\w-])color:\s*([^;]+)/g)]
      .map(([, value]) => value);
    const backgroundValues = [...declarations.matchAll(/(?<![\w-])background(?:-color)?:\s*([^;]+)/g)]
      .map(([, value]) => value);
    const pairs = foregroundValues.flatMap((foreground) => backgroundValues.map((background) => [foreground, background]));

    for (const [foregroundValue, backgroundValue] of pairs) {
      for (const [mode, tokens] of [['default', defaultTokens], ['Increase Contrast', contrastTokens]]) {
        const foreground = resolveSolidColor(foregroundValue, tokens);
        const background = resolveSolidColor(backgroundValue, tokens);
        if (foreground && background && contrastRatio(foreground, background) < 4.5) {
          failingPairs.push(`${selector.trim()} (${mode}): ${foregroundValue.trim()} on ${backgroundValue.trim()}`);
        }
      }
    }
  }

  assert.deepEqual(failingPairs, [], `same-rule solid text/surface color pairs need at least 4.5:1 contrast:\n${failingPairs.join('\n')}`);
});

test('Reduce Motion selects instant navigation and disables app animations and transitions', () => {
  assert.equal(scrollBehaviorForMotionPreference(true), 'auto');
  assert.equal(scrollBehaviorForMotionPreference(false), 'smooth');

  const reducedMotionBlock = css.match(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{\s*\*,\s*\*::before,\s*\*::after\s*\{([\s\S]*?)\n\s*\}/)?.[1];
  assert.ok(reducedMotionBlock, 'a global prefers-reduced-motion override is present');
  assert.match(reducedMotionBlock, /scroll-behavior:\s*auto\s*!important;/);
  assert.match(reducedMotionBlock, /animation:\s*none\s*!important;/);
  assert.match(reducedMotionBlock, /transition:\s*none\s*!important;/);
});

test('ARIA labels on generic containers have an explicit nameable role', async () => {
  const genericTags = new Set(['div', 'label', 'pre', 'span']);
  const files = await collectTsxFiles(sourceRoot);
  const unlabeledRoles = [];

  for (const file of files) {
    const source = await readFile(file, 'utf8');
    const sourceFile = ts.createSourceFile(file.pathname, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
    const inspect = (node) => {
      if (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) {
        const tag = node.tagName.getText(sourceFile);
        const hasAccessibleName = jsxAttribute(node, 'aria-label') || jsxAttribute(node, 'aria-labelledby');
        if (genericTags.has(tag) && hasAccessibleName) {
          const roleAttribute = jsxAttribute(node, 'role');
          const role = roleAttribute && ts.isStringLiteral(roleAttribute.initializer)
            ? roleAttribute.initializer.text
            : undefined;
          if (role ? !nameableRoles.has(role) : !roleIsNameable(roleAttribute, sourceFile)) {
            const line = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile)).line + 1;
            unlabeledRoles.push(`${path.relative(sourceRoot.pathname, file.pathname)}:${line} <${tag}>`);
          }
        }
      }
      ts.forEachChild(node, inspect);
    };
    inspect(sourceFile);
  }

  assert.deepEqual(unlabeledRoles, [], `generic containers cannot be named by ARIA without a nameable role:\n${unlabeledRoles.join('\n')}`);
});
