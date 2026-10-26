import type { SnapshotManifest } from './commands';

export type CaptureDisposition = 'captured' | 'skipped' | 'excluded';
export type ParserReadiness = 'notReady' | 'unsupported' | 'notApplicable';
export type LanguageSupportTier = 'partialStructural' | 'textOnly' | 'unavailable';
export type InventoryCategory = 'config' | 'test' | 'generated' | 'secret';

export interface InventoryRow {
  path: string;
  kind: 'file' | 'directory' | 'symlink' | 'other' | 'unknown';
  sizeBytes: number | null;
  disposition: CaptureDisposition;
  reason: string;
  details: string;
  language: string;
  contentKind: 'text' | 'binary' | 'invalidEncoding' | 'unknown' | 'notApplicable';
  parserReadiness: ParserReadiness;
  indexEligibility: 'pending' | 'ineligible';
  categories: InventoryCategory[];
  sha256: string | null;
}

export interface InventoryFilters {
  query: string;
  language: string;
  disposition: 'all' | CaptureDisposition;
  parser: 'all' | ParserReadiness;
  content: 'all' | InventoryRow['contentKind'];
  category: 'all' | InventoryCategory;
}

export interface InventoryPage {
  rows: InventoryRow[];
  total: number;
  page: number;
  pageSize: number;
  pageCount: number;
  firstVisible: number;
  lastVisible: number;
}

// Parser availability is only partial structural capability, not end-to-end
// semantic or generated-narrative qualification.
const partialStructuralParserLanguages = new Set([
  'TypeScript', 'JavaScript', 'Rust', 'Python', 'Go',
]);

const languageByExtension: Record<string, string> = {
  ts: 'TypeScript', tsx: 'TypeScript', mts: 'TypeScript', cts: 'TypeScript',
  js: 'JavaScript', jsx: 'JavaScript', mjs: 'JavaScript', cjs: 'JavaScript',
  rs: 'Rust', py: 'Python', pyw: 'Python', pyi: 'Python', go: 'Go', java: 'Java',
  c: 'C', h: 'C/C++/Objective-C header', cc: 'C++', cpp: 'C++', cxx: 'C++', 'c++': 'C++',
  hpp: 'C++', 'h++': 'C++', ipp: 'C++', inl: 'C++', ixx: 'C++', cppm: 'C++', ccm: 'C++', cxxm: 'C++',
  cs: 'C#', csx: 'C#', swift: 'Swift', kt: 'Kotlin', kts: 'Kotlin',
  d: 'D',
  rb: 'Ruby', php: 'PHP', scala: 'Scala', sc: 'Scala',
  ex: 'Elixir', exs: 'Elixir', erl: 'Erlang', hrl: 'Erlang',
  hs: 'Haskell', lhs: 'Haskell', ml: 'OCaml', mli: 'OCaml',
  fs: 'F#', fsx: 'F#', clj: 'Clojure', cljs: 'Clojure', cljc: 'Clojure',
  lua: 'Lua', zig: 'Zig', dart: 'Dart', vue: 'Vue', svelte: 'Svelte',
  jl: 'Julia', pl: 'Perl / Prolog', pm: 'Perl', r: 'R',
  asm: 'Assembly', s: 'Assembly', v: 'Verilog / V', sv: 'SystemVerilog',
  vhd: 'VHDL', vhdl: 'VHDL', sol: 'Solidity', proto: 'Protocol Buffers',
  purs: 'PureScript', res: 'ReScript', resi: 'ReScript', coffee: 'CoffeeScript',
  odin: 'Odin', gleam: 'Gleam', elm: 'Elm',
  f: 'Fortran', f90: 'Fortran', f95: 'Fortran', f03: 'Fortran',
  cob: 'COBOL', cbl: 'COBOL', nix: 'Nix', tf: 'Terraform',
  cmake: 'CMake', prisma: 'Prisma', gradle: 'Gradle',
  groovy: 'Groovy', gvy: 'Groovy', gsh: 'Groovy',
  ps1: 'PowerShell', psm1: 'PowerShell', psd1: 'PowerShell', bat: 'Batch', cmd: 'Batch',
  m: 'Objective-C / MATLAB', mm: 'Objective-C++',
  json: 'JSON', jsonc: 'JSON with comments', yaml: 'YAML', yml: 'YAML', toml: 'TOML',
  md: 'Markdown', mdx: 'MDX', rst: 'reStructuredText', txt: 'Text',
  html: 'HTML', htm: 'HTML', css: 'CSS', scss: 'SCSS', less: 'Less',
  sql: 'SQL', sh: 'Shell', bash: 'Shell', zsh: 'Shell', fish: 'Shell',
  properties: 'Java properties', diff: 'Diff', patch: 'Diff',
  ini: 'INI',
  xml: 'XML', svg: 'SVG', graphql: 'GraphQL', gql: 'GraphQL',
  dockerfile: 'Dockerfile', makefile: 'Makefile', lock: 'Lockfile',
};

const languageByFilename: Record<string, string> = {
  '.bashrc': 'Shell', '.bash_profile': 'Shell', '.bash_login': 'Shell', '.bash_aliases': 'Shell',
  '.profile': 'Shell', '.zshrc': 'Shell', '.zprofile': 'Shell', '.zshenv': 'Shell', '.envrc': 'Shell',
  '.editorconfig': 'EditorConfig', '.npmrc': 'npm config', '.gitconfig': 'Git config', '.gitmodules': 'Git config',
};

const intentionallyExcludedReasons = new Set([
  'directoryContainer', 'ignoredByGit', 'excludedByPolicy', 'gitMetadata', 'nestedRepository',
  'symlink', 'nonRegularFile',
]);

function stableCompare(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

export function languageForPath(path: string): string {
  const name = path.split('/').at(-1)?.toLowerCase() ?? '';
  if (languageByFilename[name]) return languageByFilename[name]!;
  if (isDockerfileName(name)) return 'Dockerfile';
  if (name === 'makefile' || name === 'gnumakefile') return 'Makefile';
  if (name === 'cmakelists.txt') return 'CMake';
  if (name === 'nginx.conf') return 'Nginx config';
  const extension = name.includes('.') ? name.split('.').at(-1) ?? '' : '';
  return languageByExtension[extension] ?? (extension ? `.${extension}` : 'No extension');
}

function isDockerfileName(name: string): boolean {
  return name === 'dockerfile' || name === 'containerfile'
    || name.startsWith('dockerfile.') || name.startsWith('containerfile.');
}

function categoriesForPath(path: string, secretLike: boolean): InventoryCategory[] {
  const lower = path.toLowerCase();
  const name = lower.split('/').at(-1) ?? lower;
  const segments = lower.split('/');
  const categories: InventoryCategory[] = [];
  if (
    ['.github', '.vscode', 'config', 'configs'].some((part) => segments.includes(part))
    || name.startsWith('.')
    || /(^|[-.])(config|configuration)([-.]|$)/.test(name)
    || ['makefile', 'dockerfile', 'package.json', 'cargo.toml', 'go.mod', 'pyproject.toml'].includes(name)
    || ['json', 'jsonc', 'yaml', 'yml', 'toml', 'ini', 'conf'].includes(name.split('.').at(-1) ?? '')
  ) categories.push('config');
  if (
    ['test', 'tests', '__tests__', 'spec', 'specs'].some((part) => segments.includes(part))
    || /\.(test|spec)\.[^.]+$/.test(name)
  ) categories.push('test');
  if (
    /(^|\/)(generated|dist|build|target|coverage|out|vendor|node_modules)(\/|$)/.test(lower)
    || /\.generated\.[^.]+$/.test(name)
    || /\.min\.(js|css)$/.test(name)
  ) categories.push('generated');
  if (secretLike || /(^|[._/-])(secret|credentials?|token|\.env|id_rsa|id_ed25519)([._/-]|$)/.test(lower) || /\.(pem|p12|pfx|key)$/.test(name)) {
    categories.push('secret');
  }
  return categories;
}

export function inventoryReasonLabel(reason: string): string {
  const labels: Record<string, string> = {
    captured: 'Captured in snapshot',
    directoryContainer: 'Folder contents listed separately',
    ignoredByGit: 'Ignored by Git',
    excludedByPolicy: 'Excluded by capture policy',
    gitMetadata: 'Git metadata',
    nestedRepository: 'Nested repository',
    symlink: 'Symbolic link',
    nonRegularFile: 'Not a regular file',
    fileByteLimit: 'Over per-file byte limit',
    totalByteLimit: 'Over total snapshot byte limit',
    pathLimit: 'Path scan limit reached',
    pathByteLimit: 'Path exceeds length limit',
    directoryEntryLimit: 'Folder entry limit reached',
    unreadableFile: 'File could not be read',
    unreadableDirectory: 'Folder could not be read',
    unreadablePath: 'Path metadata could not be read',
    unreadableDirectoryEntry: 'Folder entry could not be read',
    uninspectedDirectory: 'Folder safety could not be verified',
    nonUtf8Path: 'Path name is not valid Unicode',
  };
  return labels[reason] ?? reason;
}

function capturedReadiness(contentKind: InventoryRow['contentKind'], language: string): ParserReadiness {
  if (contentKind !== 'text') return 'notApplicable';
  return partialStructuralParserLanguages.has(language) ? 'notReady' : 'unsupported';
}

export function languageSupportTier(
  row: Pick<InventoryRow, 'disposition' | 'contentKind' | 'parserReadiness'>,
): LanguageSupportTier {
  if (row.disposition !== 'captured' || row.contentKind !== 'text') return 'unavailable';
  switch (row.parserReadiness) {
    case 'notReady': return 'partialStructural';
    case 'unsupported': return 'textOnly';
    case 'notApplicable': return 'unavailable';
  }
}

export function languageSupportLabel(tier: LanguageSupportTier): string {
  switch (tier) {
    case 'partialStructural': return 'Partial structural';
    case 'textOnly': return 'Text only';
    case 'unavailable': return 'Not analyzed';
  }
}

export function languageSupportNote(tier: LanguageSupportTier, language: string): string {
  switch (tier) {
    case 'partialStructural':
      if (language === 'Python') {
        return 'Python declarations and selected same-file calls are indexed. Module imports, framework behavior, and generated explanation quality are not qualified.';
      }
      if (language === 'Go') {
        return 'Go declarations and selected same-file calls are indexed. Package imports, framework behavior, and generated explanation quality are not qualified.';
      }
      return `${language} has a local syntax parser. Relationships are conservative, and framework behavior and generated explanation quality are not language-qualified.`;
    case 'textOnly':
      return `Captured ${language} text and exact citations remain available; the local indexer does not extract symbols or relationships for this language.`;
    case 'unavailable':
      return 'This entry was not structurally analyzed. Check its capture disposition and content type before assuming source text is available.';
  }
}

export function buildInventory(manifest: SnapshotManifest): InventoryRow[] {
  const rows: InventoryRow[] = [];
  for (const directory of manifest.directories ?? []) {
    rows.push({
      path: directory.path,
      kind: 'directory',
      sizeBytes: directory.sizeBytes,
      disposition: 'excluded',
      reason: 'directoryContainer',
      details: 'Folder contents are represented by their own rows; folders have no captured file bytes.',
      language: 'Folder',
      contentKind: 'notApplicable',
      parserReadiness: 'notApplicable',
      indexEligibility: 'ineligible',
      categories: categoriesForPath(directory.path, false),
      sha256: null,
    });
  }
  for (const file of manifest.files) {
    const language = languageForPath(file.path);
    const contentKind = file.contentKind ?? 'unknown';
    const categories = categoriesForPath(file.path, file.secretLike ?? false);
    const parserReadiness = capturedReadiness(contentKind, language);
    rows.push({
      path: file.path,
      kind: file.kind ?? 'file',
      sizeBytes: file.sizeBytes,
      disposition: 'captured',
      reason: 'captured',
      details: 'Immutable source bytes are stored in the selected snapshot.',
      language,
      contentKind,
      parserReadiness,
      indexEligibility: parserReadiness === 'notReady' ? 'pending' : 'ineligible',
      categories,
      sha256: file.sha256,
    });
  }
  for (const skipped of manifest.skipped) {
    const disposition: CaptureDisposition = intentionallyExcludedReasons.has(skipped.reason) ? 'excluded' : 'skipped';
    const language = skipped.kind === 'directory' ? 'Folder' : languageForPath(skipped.path);
    const categories = categoriesForPath(skipped.path, false);
    rows.push({
      path: skipped.path,
      kind: skipped.kind ?? 'unknown',
      sizeBytes: skipped.sizeBytes ?? null,
      disposition,
      reason: skipped.reason,
      details: skipped.details,
      language,
      contentKind: skipped.kind === 'file' ? 'unknown' : 'notApplicable',
      parserReadiness: 'notApplicable',
      indexEligibility: 'ineligible',
      categories,
      sha256: null,
    });
  }
  return rows.sort((left, right) => stableCompare(left.path, right.path) || stableCompare(left.reason, right.reason));
}

export const emptyInventoryFilters: InventoryFilters = {
  query: '', language: 'all', disposition: 'all', parser: 'all', content: 'all', category: 'all',
};

export function filterInventory(rows: InventoryRow[], filters: InventoryFilters): InventoryRow[] {
  const query = filters.query.trim().toLowerCase();
  return rows.filter((row) => {
    if (query && !`${row.path}\n${row.reason}\n${row.details}`.toLowerCase().includes(query)) return false;
    if (filters.language !== 'all' && row.language !== filters.language) return false;
    if (filters.disposition !== 'all' && row.disposition !== filters.disposition) return false;
    if (filters.parser !== 'all' && row.parserReadiness !== filters.parser) return false;
    if (filters.content !== 'all' && row.contentKind !== filters.content) return false;
    if (filters.category !== 'all' && !row.categories.includes(filters.category)) return false;
    return true;
  });
}

export function paginateInventory(rows: InventoryRow[], requestedPage: number, pageSize = 100): InventoryPage {
  const pageCount = Math.max(1, Math.ceil(rows.length / pageSize));
  const page = Math.min(Math.max(0, requestedPage), pageCount - 1);
  const offset = page * pageSize;
  const visible = rows.slice(offset, offset + pageSize);
  return {
    rows: visible,
    total: rows.length,
    page,
    pageSize,
    pageCount,
    firstVisible: rows.length === 0 ? 0 : offset + 1,
    lastVisible: offset + visible.length,
  };
}
