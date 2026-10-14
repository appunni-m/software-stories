import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { scrollBehaviorForMotionPreference } from '../src/lib/motion.ts';

const css = await readFile(new URL('../src/App.css', import.meta.url), 'utf8');
const surfaces = ['#0b0e12', '#10141a', '#141a21', '#1a222b'];

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

test('Reduce Motion selects instant navigation and disables app animations and transitions', () => {
  assert.equal(scrollBehaviorForMotionPreference(true), 'auto');
  assert.equal(scrollBehaviorForMotionPreference(false), 'smooth');

  const reducedMotionBlock = css.match(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{\s*\*,\s*\*::before,\s*\*::after\s*\{([\s\S]*?)\n\s*\}/)?.[1];
  assert.ok(reducedMotionBlock, 'a global prefers-reduced-motion override is present');
  assert.match(reducedMotionBlock, /scroll-behavior:\s*auto\s*!important;/);
  assert.match(reducedMotionBlock, /animation:\s*none\s*!important;/);
  assert.match(reducedMotionBlock, /transition:\s*none\s*!important;/);
});
