#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFile, realpath } from "node:fs/promises";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const defaultManifest = "benchmarks/chapter-quality/corpus/manifest.json";
// Bump this when the provider-facing file-chapter response contract changes.
const responseSchemaVersion = 1;
const requiredRoleFamilies = [
  { label: "route/entrypoint", aliases: ["route", "entrypoint", "route-entrypoint", "route/entrypoint"] },
  { label: "service/core", aliases: ["service", "core", "service-core", "service/core"] },
  { label: "data model", aliases: ["model", "data-model", "data model"] },
  { label: "test", aliases: ["test", "tests"] },
  { label: "configuration", aliases: ["config", "configuration"] },
  { label: "unsupported language", aliases: ["unsupported", "unsupported-language", "unsupported language"] },
];
const rubricKeys = [
  "responsibility",
  "declarations",
  "executionPath",
  "applicableBehavior",
  "relatedContext",
  "citationPrecision",
  "uncertainty",
  "unsupportedClaims",
];
const expectedFactDimensions = new Set([
  "responsibility",
  "declarations",
  "executionPath",
  "inputsOutputs",
  "stateEffects",
  "failurePaths",
  "relatedCallersAndTests",
  "uncertainty",
]);
const applicabilityStatuses = new Set(["required", "notApplicable", "notProvided", "sourceOnly", "notAssessable"]);
const allowedFailureCategories = new Set([
  "generic",
  "missing_responsibility",
  "missing_declaration",
  "missing_flow",
  "missing_applicable_behavior",
  "missing_related_context",
  "weak_citation",
  "unsupported_claim",
  "uncertainty_missing",
  "other",
]);

function fail(message) {
  console.error(`Chapter quality gate: ${message}`);
  process.exitCode = 1;
}

function sha256(value) {
  return createHash("sha256").update(value).digest("hex");
}

function parseJsonWithoutDuplicateKeys(source, label) {
  let cursor = 0;
  const skipWhitespace = () => {
    while (/[\t\n\r ]/.test(source[cursor] ?? "")) cursor += 1;
  };
  const parseString = () => {
    const start = cursor;
    if (source[cursor] !== '"') throw new Error(`${label} contains invalid JSON`);
    cursor += 1;
    while (cursor < source.length) {
      if (source[cursor] === "\\") {
        cursor += 2;
      } else if (source[cursor] === '"') {
        cursor += 1;
        return JSON.parse(source.slice(start, cursor));
      } else {
        cursor += 1;
      }
    }
    throw new Error(`${label} contains an unterminated JSON string`);
  };
  const parseValue = (depth = 0) => {
    if (depth > 128) throw new Error(`${label} exceeds the JSON nesting limit`);
    skipWhitespace();
    if (source[cursor] === "{") {
      cursor += 1;
      skipWhitespace();
      const keys = new Set();
      if (source[cursor] === "}") {
        cursor += 1;
        return;
      }
      while (cursor < source.length) {
        skipWhitespace();
        const key = parseString();
        if (keys.has(key)) throw new Error(`${label} contains duplicate JSON object key ${JSON.stringify(key)}`);
        keys.add(key);
        skipWhitespace();
        if (source[cursor] !== ":") throw new Error(`${label} contains invalid JSON`);
        cursor += 1;
        parseValue(depth + 1);
        skipWhitespace();
        if (source[cursor] === "}") {
          cursor += 1;
          return;
        }
        if (source[cursor] !== ",") throw new Error(`${label} contains invalid JSON`);
        cursor += 1;
      }
      throw new Error(`${label} contains an unterminated JSON object`);
    }
    if (source[cursor] === "[") {
      cursor += 1;
      skipWhitespace();
      if (source[cursor] === "]") {
        cursor += 1;
        return;
      }
      while (cursor < source.length) {
        parseValue(depth + 1);
        skipWhitespace();
        if (source[cursor] === "]") {
          cursor += 1;
          return;
        }
        if (source[cursor] !== ",") throw new Error(`${label} contains invalid JSON`);
        cursor += 1;
      }
      throw new Error(`${label} contains an unterminated JSON array`);
    }
    if (source[cursor] === '"') {
      parseString();
      return;
    }
    const rest = source.slice(cursor);
    const literal = /^(?:true|false|null)(?![A-Za-z0-9_$])/.exec(rest);
    const number = /^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/.exec(rest);
    if (literal) cursor += literal[0].length;
    else if (number) cursor += number[0].length;
    else throw new Error(`${label} contains invalid JSON`);
  };
  parseValue();
  skipWhitespace();
  if (cursor !== source.length) throw new Error(`${label} contains trailing data after JSON`);
  return JSON.parse(source);
}

function isRepoRelativePath(value) {
  return typeof value === "string" && value.length > 0 && value === value.trim()
    && !value.startsWith("/") && !value.includes("\\")
    && value.split("/").every((part) => part.length > 0 && part !== "." && part !== "..");
}

function usage() {
  console.log(`Usage:
  npm run quality:corpus
  npm run quality:metadata
  npm run quality:gate -- --report <completed-report.json> [--manifest <manifest.json>]

The corpus check is local and never calls a model. The release gate requires a
completed, human-reviewed report tied to the current corpus and generation versions.`);
}

function readOption(args, option, fallback = undefined) {
  const index = args.indexOf(option);
  if (index < 0) return fallback;
  const value = args[index + 1];
  if (!value || value.startsWith("--")) {
    if (fallback !== undefined) return fallback;
    throw new Error(`${option} requires a path`);
  }
  return value;
}

function normalizeRole(role) {
  return String(role ?? "").trim().toLowerCase().replaceAll("_", "-");
}

async function currentParserLanguages() {
  const inventorySource = await readFile(resolve(root, "src/lib/inventory.ts"), "utf8");
  const declaration = inventorySource.match(/const parserLanguages = new Set\(\[([\s\S]*?)\]\);/)?.[1];
  if (!declaration) throw new Error("Could not read the current supported-language set from src/lib/inventory.ts");
  const languages = [...declaration.matchAll(/'((?:\\.|[^'\\])*)'/g)].map((match) => match[1]);
  if (languages.length === 0 || new Set(languages).size !== languages.length) {
    throw new Error("The current supported-language set is empty or contains duplicate entries");
  }
  return new Set(languages);
}

function extractRustBlock(source, signature) {
  const signatureIndex = source.indexOf(signature);
  if (signatureIndex < 0) throw new Error(`Could not find Rust item ${signature}`);
  const open = source.indexOf("{", signatureIndex + signature.length);
  if (open < 0) throw new Error(`Could not find body for Rust item ${signature}`);

  let depth = 0;
  let inString = false;
  let escaped = false;
  for (let index = open; index < source.length; index += 1) {
    const char = source[index];
    if (inString) {
      if (escaped) escaped = false;
      else if (char === "\\") escaped = true;
      else if (char === '"') inString = false;
      continue;
    }
    if (char === '"') {
      inString = true;
      continue;
    }
    if (char === "{") depth += 1;
    else if (char === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(signatureIndex, index + 1);
    }
  }
  throw new Error(`Unclosed body for Rust item ${signature}`);
}

async function loadManifest(manifestArg) {
  const manifestPathInput = resolve(root, manifestArg ?? defaultManifest);
  const workspaceReal = await realpath(root);
  const manifestPath = await realpath(manifestPathInput);
  const manifestRelative = relative(workspaceReal, manifestPath);
  if (manifestRelative.startsWith("..") || isAbsolute(manifestRelative)) {
    throw new Error("Manifest must resolve inside this workspace");
  }
  const bytes = await readFile(manifestPath);
  let manifest;
  try {
    const manifestText = bytes.toString("utf8");
    if (!Buffer.from(manifestText, "utf8").equals(bytes)) throw new Error("Manifest is not valid UTF-8");
    manifest = parseJsonWithoutDuplicateKeys(manifestText, "Manifest");
  } catch (error) {
    throw new Error(`Manifest is not valid JSON: ${error.message}`);
  }
  if (manifest.version !== 1 || typeof manifest.corpusId !== "string" || !manifest.corpusId.trim() || !Array.isArray(manifest.cases)) {
    throw new Error("Manifest must have version 1, a corpusId, and a cases array");
  }

  const ids = new Set();
  const repositories = new Set();
  const roles = new Set();
  const supportedLanguages = await currentParserLanguages();
  const corpusRoot = dirname(manifestPath);
  const corpusRootReal = await realpath(corpusRoot);
  const corpusRelative = relative(workspaceReal, corpusRootReal);
  if (corpusRelative.startsWith("..") || isAbsolute(corpusRelative)) throw new Error("Corpus directory must resolve inside this workspace");
  const loadSource = async (relativePath, caseId) => {
    if (!isRepoRelativePath(relativePath)) throw new Error(`${caseId}: sourceFile must be a canonical repository-relative path`);
    const sourcePath = resolve(corpusRoot, relativePath ?? "");
    const sourceRelative = relative(corpusRoot, sourcePath);
    if (!relativePath || sourceRelative.startsWith("..") || isAbsolute(sourceRelative)) {
      throw new Error(`${caseId}: source files must stay inside the corpus directory`);
    }
    const sourcePathReal = await realpath(sourcePath);
    const sourceRealRelative = relative(corpusRootReal, sourcePathReal);
    if (sourceRealRelative.startsWith("..") || isAbsolute(sourceRealRelative)) {
      throw new Error(`${caseId}: source file resolves outside the corpus directory`);
    }
    const sourceBytes = await readFile(sourcePathReal);
    const source = sourceBytes.toString("utf8");
    if (!Buffer.from(source, "utf8").equals(sourceBytes)) throw new Error(`${caseId}: source file is not valid UTF-8`);
    return { bytes: sourceBytes, source };
  };
  for (const item of manifest.cases) {
    if (!item || typeof item.id !== "string" || !/^[a-z0-9][a-z0-9._-]*$/.test(item.id)) throw new Error("Every case needs a canonical lowercase id");
    if (ids.has(item.id)) throw new Error(`Duplicate case id: ${item.id}`);
    ids.add(item.id);
    if (typeof item.repository !== "string" || !/^[a-z0-9][a-z0-9._-]*$/.test(item.repository)) {
      throw new Error(`${item.id}: repository must be a canonical lowercase repository identifier`);
    }
    if (typeof item.language !== "string" || !item.language.trim()
      || !["supported", "unsupportedLanguage", "not-evaluated-config"].includes(item.parserStatus)) {
      throw new Error(`${item.id}: language and parserStatus must describe the current source-indexing coverage`);
    }
    const isUnsupportedLanguageRole = requiredRoleFamilies
      .find((family) => family.label === "unsupported language")
      .aliases.some((alias) => normalizeRole(alias) === normalizeRole(item.role));
    if (isUnsupportedLanguageRole !== (item.parserStatus === "unsupportedLanguage")) {
      throw new Error(`${item.id}: the unsupported-language corpus role must match parserStatus unsupportedLanguage`);
    }
    if (item.parserStatus === "supported" && !supportedLanguages.has(item.language)) {
      throw new Error(`${item.id}: parserStatus is supported but ${item.language} is not in the current parser language set`);
    }
    if (item.parserStatus === "unsupportedLanguage" && supportedLanguages.has(item.language)) {
      throw new Error(`${item.id}: ${item.language} is currently parser-supported and cannot represent the unsupported-language role`);
    }
    repositories.add(item.repository);
    roles.add(normalizeRole(item.role));
    exactKeys(item.applicability, [...expectedFactDimensions], `${item.id} applicability`);
    for (const [dimension, applicability] of Object.entries(item.applicability)) {
      exactKeys(applicability, ["status", "reason"], `${item.id} applicability.${dimension}`);
      if (!applicabilityStatuses.has(applicability.status)
        || typeof applicability.reason !== "string" || !applicability.reason.trim()) {
        throw new Error(`${item.id}: applicability.${dimension} needs a recognized status and a non-empty reason`);
      }
    }
    if (!Array.isArray(item.mustCover) || item.mustCover.length === 0) {
      throw new Error(`${item.id}: mustCover needs at least one applicable review question`);
    }
    if (!isRepoRelativePath(item.path) || !isRepoRelativePath(item.sourceFile)) {
      throw new Error(`${item.id}: target path and sourceFile must be canonical repository-relative paths`);
    }
    if (!Array.isArray(item.files) || item.files.length === 0) {
      throw new Error(`${item.id}: files must list target source evidence`);
    }
    if (item.supportingFiles !== undefined && (!Array.isArray(item.supportingFiles)
      || item.supportingFiles.some((file) => !file || !isRepoRelativePath(file.path) || !isRepoRelativePath(file.sourceFile)))) {
      throw new Error(`${item.id}: supportingFiles must contain canonical path/sourceFile pairs`);
    }
    const sources = new Map();
    const logicalPaths = new Set();
    const sourceFiles = new Set();
    for (const file of item.files) {
      if (!isRepoRelativePath(file.path) || !isRepoRelativePath(file.sourceFile) || !/^[a-f0-9]{64}$/.test(file.sha256)
        || !Number.isInteger(file.sizeBytes) || file.sizeBytes < 0 || !["target", "support"].includes(file.role)) {
        throw new Error(`${item.id}: every file needs a path, sourceFile, SHA-256, and byte size`);
      }
      if (logicalPaths.has(file.path)) throw new Error(`${item.id}: duplicate logical file path ${file.path}`);
      if (sourceFiles.has(file.sourceFile)) throw new Error(`${item.id}: duplicate sourceFile ${file.sourceFile}`);
      logicalPaths.add(file.path);
      sourceFiles.add(file.sourceFile);
      const loaded = await loadSource(file.sourceFile, item.id);
      if (sha256(loaded.bytes) !== file.sha256) throw new Error(`${item.id}: source hash changed for ${file.path}`);
      if (loaded.bytes.byteLength !== file.sizeBytes) throw new Error(`${item.id}: source size changed for ${file.path}`);
      sources.set(file.sourceFile, loaded);
    }
    const targetFile = item.files.find((file) => file.role === "target");
    if (item.files.filter((file) => file.role === "target").length !== 1 || !sources.has(item.sourceFile)
      || !targetFile || targetFile.path !== item.path || targetFile.sourceFile !== item.sourceFile) {
      throw new Error(`${item.id}: target path/sourceFile must match its target evidence file`);
    }
    if (item.supportingFiles !== undefined) {
      const declaredSupport = item.supportingFiles.map((file) => {
        exactKeys(file, ["path", "sourceFile"], `${item.id} supporting file`);
        return { path: file.path, sourceFile: file.sourceFile };
      }).sort((a, b) => a.path.localeCompare(b.path));
      const selectedSupport = item.files.filter((file) => file.role === "support")
        .map((file) => ({ path: file.path, sourceFile: file.sourceFile })).sort((a, b) => a.path.localeCompare(b.path));
      if (stableJson(declaredSupport) !== stableJson(selectedSupport)) {
        throw new Error(`${item.id}: supportingFiles must exactly match files with role support`);
      }
    }
    if (!Array.isArray(item.mustCover) || item.mustCover.some((factId) => typeof factId !== "string" || !factId.trim())
      || new Set(item.mustCover).size !== item.mustCover.length || item.mustCover.length === 0) {
      throw new Error(`${item.id}: mustCover needs distinct, non-empty expected-fact IDs`);
    }
    if (!Array.isArray(item.expectedFacts) || item.expectedFacts.length === 0) {
      throw new Error(`${item.id}: expectedFacts must include source-grounded facts`);
    }
    const factIds = new Set();
    for (const fact of item.expectedFacts) {
      exactKeys(fact, ["id", "dimension", "statement", "path", "sourceFile", "quote", "lineStart", "lineEnd", "byteStart", "byteEndExclusive", "sourceSha256"], `${item.id} expected fact`);
      if (typeof fact.id !== "string" || !/^[a-z0-9][a-z0-9._-]*$/.test(fact.id) || factIds.has(fact.id)) {
        throw new Error(`${item.id}: expected fact IDs must be distinct canonical lowercase identifiers`);
      }
      if (!expectedFactDimensions.has(fact.dimension)
        || typeof fact.statement !== "string" || byteLength(fact.statement.trim()) < 12 || byteLength(fact.statement.trim()) > 1_000) {
        throw new Error(`${item.id}: expected fact ${fact.id} needs a supported dimension and a concrete 12-1,000 byte statement`);
      }
      if (item.applicability[fact.dimension].status === "notApplicable") {
        throw new Error(`${item.id}: expected fact ${fact.id} contradicts notApplicable dimension ${fact.dimension}`);
      }
      factIds.add(fact.id);
    }
    if (item.mustCover.some((factId) => !factIds.has(factId))) {
      throw new Error(`${item.id}: every mustCover entry must name an expectedFacts id`);
    }
    const fileBySource = new Map(item.files.map((file) => [file.sourceFile, file]));
    for (const fact of item.expectedFacts) {
      if (typeof fact.quote !== "string" || !fact.quote.trim()) {
        throw new Error(`${item.id}: every expected fact needs an exact source quote`);
      }
      const factSourcePath = fact.sourceFile ?? item.sourceFile;
      const loaded = sources.get(factSourcePath);
      if (!loaded) throw new Error(`${item.id}: expected fact ${fact.id ?? "(unnamed)"} cites an unlisted source file`);
      const factFile = fileBySource.get(factSourcePath);
      if (!factFile || fact.path !== factFile.path) throw new Error(`${item.id}: expected fact ${fact.id} path/sourceFile do not identify the same selected file`);
      const anchor = exactQuoteAnchor(loaded.source, fact.quote, `${item.id} expected fact ${fact.id}`);
      const lineStart = fact.lineStart ?? fact.startLine;
      const lineEnd = fact.lineEnd ?? fact.endLine;
      if (lineStart !== anchor.startLine || lineEnd !== anchor.endLine) {
        throw new Error(`${item.id}: expected fact ${fact.id} must use the exact quote line range ${anchor.startLine}-${anchor.endLine}`);
      }
      if (!Number.isInteger(fact.byteStart) || !Number.isInteger(fact.byteEndExclusive) || fact.byteStart < 0
        || fact.byteEndExclusive > loaded.bytes.length || fact.byteStart !== anchor.byteStart || fact.byteEndExclusive !== anchor.byteEndExclusive) {
        throw new Error(`${item.id}: expected fact ${fact.id ?? "(unnamed)"} needs a UTF-8 byte range`);
      }
      if (!loaded.bytes.subarray(fact.byteStart, fact.byteEndExclusive).equals(Buffer.from(fact.quote, "utf8"))) {
        throw new Error(`${item.id}: expected quote bytes do not match the declared range for ${fact.id ?? "(unnamed)"}`);
      }
      if (fact.sourceSha256 !== sha256(loaded.bytes)) {
        throw new Error(`${item.id}: expected fact ${fact.id ?? "(unnamed)"} has a stale source hash`);
      }
    }

    if (item.allowedRelationshipIds !== undefined && (!Array.isArray(item.allowedRelationshipIds)
      || item.allowedRelationshipIds.some((relationshipId) => typeof relationshipId !== "string" || !relationshipId.trim())
      || new Set(item.allowedRelationshipIds).size !== item.allowedRelationshipIds.length)) {
      throw new Error(`${item.id}: allowedRelationshipIds must be distinct, non-empty strings`);
    }
  }

  if (!Array.isArray(manifest.knownFailureExamples) || manifest.knownFailureExamples.length < 1) {
    throw new Error("Corpus needs at least one schema-shaped, source-anchored human-review failure example");
  }
  const failureExampleIds = new Set();
  for (const example of manifest.knownFailureExamples) {
    const item = manifest.cases.find((candidate) => candidate.id === example.caseId);
    const claim = example.schemaShapedClaim;
    if (typeof example.id !== "string" || !example.id.trim() || failureExampleIds.has(example.id)) {
      throw new Error("Known failure examples need distinct, non-empty IDs");
    }
    failureExampleIds.add(example.id);
    if (!item || example.expectedDisposition !== "fail" || typeof example.why !== "string" || !example.why.trim()
      || !Array.isArray(example.failureCodes) || example.failureCodes.length === 0
      || !claim || !claim.prose || !Array.isArray(claim.identifiers) || !Array.isArray(claim.evidence) || claim.evidence.length === 0) {
      throw new Error(`Known failure example ${example.id ?? "(unnamed)"} needs a case, shaped claim, and failure rationale`);
    }
    if (new Set(example.failureCodes).size !== example.failureCodes.length
      || example.failureCodes.some((code) => typeof code !== "string" || !code.trim())) {
      throw new Error(`${example.id}: failureCodes must be distinct, non-empty strings`);
    }
    exactKeys(claim, ["category", "aspect", "prose", "inferenceBasis", "identifiers", "evidence", "relationshipIds"], `${example.id} schemaShapedClaim`);
    if (!new Set(["codeBehavior", "repositoryContext", "inference"]).has(claim.category)
      || !new Set(["responsibility", "execution", "inputOutput", "stateEffect", "failurePath", "dependency", "relationship"]).has(claim.aspect)
      || typeof claim.prose !== "string" || byteLength(claim.prose.trim()) < 24 || byteLength(claim.prose.trim()) > 2_000
      || typeof claim.inferenceBasis !== "string") {
      throw new Error(`${example.id}: failure example claim does not match the generated claim contract`);
    }
    if (claim.category === "inference" && !claim.inferenceBasis.trim()) {
      throw new Error(`${example.id}: inference failure example needs a non-empty inference basis`);
    }
    validateIdentifierList(claim.identifiers, `${example.id} identifiers`);
    const exampleSources = await loadManifestSources(manifestPath, item);
    const relationshipIds = new Set(item.allowedRelationshipIds ?? []);
    if (!Array.isArray(claim.relationshipIds) || claim.relationshipIds.length > 8
      || claim.relationshipIds.some((relationshipId) => typeof relationshipId !== "string" || !relationshipIds.has(relationshipId))
      || new Set(claim.relationshipIds).size !== claim.relationshipIds.length
      || (claim.aspect === "relationship" && claim.relationshipIds.length === 0)) {
      throw new Error(`${example.id}: failure example has unreviewed relationship IDs`);
    }
    const anchors = validateEvidenceList(claim.evidence, claim.prose, claim.identifiers, example.id, exampleSources, item.path);
    if (claim.aspect === "responsibility" && !anchors.targetAnchored) {
      throw new Error(`${example.id}: responsibility failure example must cite its target file`);
    }
  }

  if (manifest.cases.length < requiredRoleFamilies.length) {
    throw new Error(`Corpus needs at least ${requiredRoleFamilies.length} cases to cover the required file roles`);
  }
  const configuredRepositoryCount = Number(manifest.minimumRepositoryCount ?? 2);
  if (!Number.isInteger(configuredRepositoryCount) || configuredRepositoryCount < 1) {
    throw new Error("minimumRepositoryCount must be a positive integer");
  }
  const minimumRepositoryCount = Math.max(2, configuredRepositoryCount);
  if (!Number.isInteger(minimumRepositoryCount) || repositories.size < minimumRepositoryCount) {
    throw new Error(`Corpus must cover at least ${minimumRepositoryCount} representative synthetic repositories`);
  }
  if (manifest.requiredRoles !== undefined && (!Array.isArray(manifest.requiredRoles)
    || manifest.requiredRoles.some((role) => typeof role !== "string" || !role.trim()))) {
    throw new Error("requiredRoles must be an array of non-empty role names");
  }
  const additionalRoleFamilies = (manifest.requiredRoles ?? [])
    .filter((role) => !requiredRoleFamilies.some((family) => family.aliases.some((alias) => normalizeRole(alias) === normalizeRole(role))))
    .map((role) => ({ label: role, aliases: [normalizeRole(role)] }));
  const requiredFamilies = [...requiredRoleFamilies, ...additionalRoleFamilies];
  for (const family of requiredFamilies) {
    if (!family.aliases.some((alias) => roles.has(normalizeRole(alias)))) {
      throw new Error(`Corpus is missing the required ${family.label} role`);
    }
  }

  return {
    manifest,
    manifestPath,
    manifestSha256: sha256(bytes),
    caseIds: [...ids].sort(),
    requiredRoles: requiredFamilies.map((family) => family.label),
  };
}

async function validateCorpus(manifestArg) {
  const loaded = await loadManifest(manifestArg);
  const reportTemplate = await loadReportTemplate();
  if (reportTemplate.status !== "pending") throw new Error("The checked-in report template must remain pending until a real local review batch is recorded");
  await validateQualityGateExample(reportTemplate.qualityGateExample, reportTemplate.qualityGateExample);
  validateFactCoverageContractExample();
  console.log(`Corpus ${loaded.manifest.corpusId}: ${loaded.caseIds.length} cases across ${new Set(loaded.manifest.cases.map((item) => item.repository)).size} synthetic repositories`);
  console.log(`Required roles covered: ${loaded.requiredRoles.join(", ")}`);
  console.log("Fact-coverage self-check rejects unrelated evidence spans and accepts containing anchors.");
  console.log("Review template is pending; shallow schema-shaped example and repair are source-linked and digest-checked.");
  console.log(`Manifest SHA-256: ${loaded.manifestSha256}`);
}

function validateFactCoverageContractExample() {
  const path = "src/example.ts";
  const source = "// λ 🐇\nexport const answer = 42;\nexport const other = 7;\n";
  const expectedQuote = "export const answer = 42;";
  const expectedAnchor = exactQuoteAnchor(source, expectedQuote, "fact coverage self-check expected fact");
  const sources = new Map([[path, { text: source }]]);
  const manifestCase = {
    mustCover: ["required-fact"],
    expectedFacts: [{ id: "required-fact", path, quote: expectedQuote, ...expectedAnchor }],
  };
  const sectionClaimId = "sections[0].claims[0]";
  const evidenceLimitId = "evidenceLimits[0]";
  const artifactFor = (evidencePath, quote) => ({
    sectionClaimIds: new Set([sectionClaimId]),
    claimEvidenceById: collectSectionClaimEvidenceById({
      sections: [{ claims: [{ evidence: [{ path: evidencePath, quote }] }] }],
      evidenceLimits: [{ evidence: [{ path, quote: expectedQuote }] }],
    }, sources),
  });
  const matchingArtifact = artifactFor(path, expectedQuote);
  validateFactCoverage({ "required-fact": [sectionClaimId] }, manifestCase, matchingArtifact, "fact coverage self-check", "pass");
  validateFactCoverage({ "required-fact": [] }, manifestCase, matchingArtifact, "fact coverage self-check", "fail");
  validateFactCoverage({ "required-fact": [sectionClaimId] }, manifestCase, artifactFor(path, source.trimEnd()), "fact coverage self-check", "pass");
  for (const [coverage, outcome, artifact, expected] of [
    [{ "required-fact": [] }, "pass", matchingArtifact, "passing empty coverage"],
    [{ "required-fact": [evidenceLimitId] }, "fail", matchingArtifact, "evidence-limit coverage"],
    [{ "required-fact": [sectionClaimId] }, "fail", artifactFor(path, "export const other = 7;"), "unrelated same-file quote"],
    [{ "required-fact": [sectionClaimId] }, "fail", artifactFor("src/other.ts", expectedQuote), "quote on another path"],
  ]) {
    let rejected = false;
    try {
      validateFactCoverage(coverage, manifestCase, artifact, "fact coverage self-check", outcome);
    } catch {
      rejected = true;
    }
    if (!rejected) throw new Error(`Fact-coverage self-check accepted ${expected}`);
  }
}

async function printCurrentMetadata(manifestArg) {
  await loadReportTemplate();
  const loaded = await loadManifest(manifestArg);
  const current = await currentGenerationMetadata();
  console.log(JSON.stringify({
    corpusId: loaded.manifest.corpusId,
    corpusManifestSha256: loaded.manifestSha256,
    modelIds: current.modelIds,
    modelAllowlistSha256: current.modelAllowlistSha256,
    promptVersion: current.promptVersion,
    promptSha256: current.promptSha256,
    requestAssemblySha256: current.requestAssemblySha256,
    generationConfigSha256: current.generationConfigSha256,
    chapterSchemaVersion: current.chapterSchemaVersion,
    responseSchemaVersion: current.responseSchemaVersion,
    responseSchemaSha256: current.responseSchemaSha256,
  }, null, 2));
}

async function currentGenerationMetadata() {
  const provider = await readFile(resolve(root, "src-tauri/src/provider.rs"), "utf8");
  const indexSource = await readFile(resolve(root, "src-tauri/src/index.rs"), "utf8");
  const chapterTypes = await readFile(resolve(root, "src-tauri/src/chapters.rs"), "utf8");
  const chapterGeneration = await readFile(resolve(root, "src-tauri/src/chapter_generation.rs"), "utf8");
  const chapterValidation = chapterTypes;
  const commandBridge = await readFile(resolve(root, "src-tauri/src/lib.rs"), "utf8");
  const providerApiSource = await readFile(resolve(root, "src-tauri/src/provider_api.rs"), "utf8");
  const promptVersion = provider.match(/const PROMPT_VERSION: &str = "([^"]+)";/)?.[1];
  const chapterSchemaVersion = Number(chapterTypes.match(/pub const CHAPTER_SCHEMA_VERSION: u32 = (\d+);/)?.[1]);
  if (!promptVersion || !Number.isInteger(chapterSchemaVersion)) {
    throw new Error("Could not read current prompt and chapter schema versions from Rust sources");
  }
  const systemPrompt = provider.match(/let system_instruction = ("(?:[^"\\]|\\.)*")\.to_owned\(\);/)?.[1];
  const userPromptTemplate = provider.match(/let user_prompt = format!\(\s*r#"([\s\S]*?)"#,/)?.[1];
  if (!systemPrompt || !userPromptTemplate) throw new Error("Could not fingerprint the current chapter prompt text");

  const disclosure = extractRustBlock(provider, "fn gemini_disclosure()");
  const modelIds = [...disclosure.matchAll(/model_id:\s*"([^"]+)"/g)].map((match) => match[1]).sort();
  if (modelIds.length === 0 || new Set(modelIds).size !== modelIds.length) {
    throw new Error("Could not derive a unique Gemini chapter-model allowlist from provider::gemini_disclosure");
  }
  const responseSchema = extractRustBlock(provider, "fn file_chapter_response_schema()");
  const generationConfig = extractRustBlock(provider, "fn file_chapter_generation_config()");
  const requestAssembly = [
    ...["GEMINI_API_ROOT", "OPENAI_API_ROOT", "ANTHROPIC_API_ROOT", "GEMINI_DISCLOSURE_VERSION", "OPENAI_DISCLOSURE_VERSION", "ANTHROPIC_DISCLOSURE_VERSION", "DISCLOSURE_VERSION", "PROMPT_VERSION", "MAX_CONTEXT_BYTES", "MAX_REQUEST_BODY_BYTES", "MAX_SELECTED_FILES", "MAX_REVIEW_SYMBOLS", "MAX_REVIEW_RELATIONSHIPS", "MAX_PROVIDER_RESPONSE_BYTES", "MAX_PROVIDER_ERROR_BYTES"]
      .map((name) => provider.match(new RegExp(`const ${name}: [^=]+ = [^;]+;`))?.[0] ?? ""),
    ...[
      "pub struct ProviderRequestFile",
      "pub struct ProviderSymbolEvidence",
      "pub struct ProviderRelationshipEvidence",
      "struct FingerprintFile",
      "struct FingerprintInputs",
      "pub struct ProviderRequestReview",
      "pub struct ProviderSendResult",
      "pub struct ProviderHttpResult",
      "pub struct BuildProviderReviewInput",
      "pub struct PendingProviderRequest",
      "pub struct ProviderModel",
      "pub struct ProviderDisclosure",
      "fn selected_index_evidence(",
      "fn checked_index_excerpt<",
      "pub fn provider_catalog()",
      "pub fn disclosure_for(",
      "fn model_for(",
      "pub fn build_request_review(",
      "pub fn build_request_review_for_provider(",
      "pub fn build_pr_guide_request_review_for_provider(",
      "fn request_fingerprint(",
      "fn request_fingerprint_for_provider(",
      "pub fn revalidate_request_review(",
      "pub async fn dispatch_reviewed_request<",
      "pub async fn dispatch_reviewed_request_via_rig(",
      "fn rig_request_failure(",
      "fn check_generated_text_size(",
      "fn generated_text_too_large_failure(",
    ].map((signature) => extractRustBlock(provider, signature)),
    chapterTypes.match(/pub const MAX_GENERATED_RESPONSE_BYTES: [^=]+ = [^;]+;/)?.[0] ?? "",
    indexSource,
    chapterGeneration,
    chapterValidation,
    providerApiSource,
    extractRustBlock(commandBridge, "fn prepare_chapter_draft_preview("),
    extractRustBlock(commandBridge, "fn build_provider_request_review("),
    extractRustBlock(commandBridge, "fn build_provider_pr_guide_request_review("),
    extractRustBlock(commandBridge, "async fn send_provider_request("),
    extractRustBlock(commandBridge, "async fn send_provider_pr_guide_request("),
    await readFile(resolve(root, "src-tauri/src/snapshot.rs"), "utf8"),
    await readFile(resolve(root, "src-tauri/Cargo.toml"), "utf8"),
    await readFile(resolve(root, "src-tauri/Cargo.lock"), "utf8"),
  ].join("\n");
  return {
    promptVersion,
    promptSha256: sha256(JSON.stringify([JSON.parse(systemPrompt), userPromptTemplate])),
    requestAssemblySha256: sha256(requestAssembly),
    generationConfigSha256: sha256(generationConfig),
    chapterSchemaVersion,
    responseSchemaVersion,
    modelIds,
    modelAllowlistSha256: sha256(JSON.stringify(modelIds)),
    responseSchemaSha256: sha256(responseSchema),
  };
}

async function loadReportTemplate() {
  const templatePath = resolve(root, "benchmarks/chapter-quality/review/REPORT_TEMPLATE.json");
  try {
    const bytes = await readFile(templatePath);
    const text = bytes.toString("utf8");
    if (!Buffer.from(text, "utf8").equals(bytes)) throw new Error("Template is not valid UTF-8");
    const template = parseJsonWithoutDuplicateKeys(text, "Checked-in report template");
    validateReportTemplateShape(template);
    return template;
  } catch (error) {
    throw new Error(`Could not read the checked-in report template: ${error.message}`);
  }
}

function validateReportTemplateShape(template) {
  exactKeys(template, ["reportSchemaVersion", "kind", "status", "corpusId", "corpusManifestSha256", "modelId", "modelVersion", "modelAllowlistSha256", "promptVersion", "promptSha256", "requestAssemblySha256", "generationConfigSha256", "chapterSchemaVersion", "responseSchemaVersion", "responseSchemaSha256", "reviewer", "reviewedAt", "qualityGateExample", "cases"], "Report template");
  if (template.reportSchemaVersion !== 3 || template.kind !== "chapter-quality-review-report" || template.status !== "pending") {
    throw new Error("Report template must use schema version 3 and remain pending");
  }
  for (const key of ["corpusId", "corpusManifestSha256", "modelId", "modelVersion", "modelAllowlistSha256", "promptVersion", "promptSha256", "requestAssemblySha256", "generationConfigSha256", "chapterSchemaVersion", "responseSchemaVersion", "responseSchemaSha256", "reviewer", "reviewedAt"]) {
    if (template[key] !== null) throw new Error(`Report template ${key} must remain empty until a local review batch is recorded`);
  }
  if (!Array.isArray(template.cases) || template.cases.length !== 1) throw new Error("Report template must contain one unrun case example");
  const item = template.cases[0];
  exactKeys(item, ["caseId", "repositoryId", "targetPath", "sourceHashesByPath", "requestFingerprint", "chapterArtifactPath", "chapterArtifactSha256", "schemaValidation", "outcome", "failureCategories", "findings", "repairSummary", "factCoverage", "rubric", "attempts"], "Report template case");
  if (item.caseId !== null || item.repositoryId !== null || item.targetPath !== null || item.chapterArtifactPath !== null
    || item.requestFingerprint !== null || item.chapterArtifactSha256 !== null || item.schemaValidation !== "pending" || item.outcome !== "pending"
    || item.repairSummary !== null || stableJson(item.factCoverage) !== "{}"
    || !Array.isArray(item.failureCategories) || item.failureCategories.length !== 0
    || !Array.isArray(item.findings) || item.findings.length !== 0 || !Array.isArray(item.attempts) || item.attempts.length !== 0
    || stableJson(item.sourceHashesByPath) !== "{}") {
    throw new Error("Report template case must remain unrun with empty artifact and assessment fields");
  }
  exactKeys(item.rubric, rubricKeys, "Report template rubric");
  for (const key of rubricKeys) {
    const criterion = item.rubric[key];
    exactKeys(criterion, ["outcome", "reason", "claimIds", "sourceRefs"], `Report template rubric.${key}`);
    if (criterion.outcome !== "pending" || criterion.reason !== null
      || !Array.isArray(criterion.claimIds) || criterion.claimIds.length !== 0
      || !Array.isArray(criterion.sourceRefs) || criterion.sourceRefs.length !== 0) {
      throw new Error(`Report template rubric.${key} must remain pending and empty`);
    }
  }
}

function stableJson(value) {
  if (Array.isArray(value)) return `[${value.map(stableJson).join(",")}]`;
  if (value && typeof value === "object") {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableJson(value[key])}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function requireHash(value, label) {
  if (typeof value !== "string" || !/^[a-f0-9]{64}$/.test(value)) {
    throw new Error(`${label} must be a lowercase SHA-256 digest`);
  }
}

function normalizeCriterionOutcome(value) {
  return String(value ?? "")
    .trim()
    .replace(/[A-Z]/g, (letter) => `_${letter.toLowerCase()}`)
    .toLowerCase()
    .replaceAll("-", "_");
}

const criterionApplicability = {
  responsibility: ["responsibility"],
  declarations: ["declarations"],
  executionPath: ["executionPath"],
  applicableBehavior: ["inputsOutputs", "stateEffects", "failurePaths"],
  relatedContext: ["relatedCallersAndTests"],
  citationPrecision: [],
  uncertainty: ["uncertainty"],
  unsupportedClaims: [],
};

function validateCriterion(caseId, key, criterion, manifestCase, schemaValidation = "pass") {
  if (!criterion || typeof criterion !== "object") throw new Error(`${caseId}: rubric.${key} is missing`);
  exactKeys(criterion, ["outcome", "reason", "claimIds", "sourceRefs"], `${caseId} rubric.${key}`);
  const outcome = normalizeCriterionOutcome(criterion.outcome ?? criterion.status);
  if (!["pass", "fail", "not_applicable"].includes(outcome)) {
    throw new Error(`${caseId}: rubric.${key} must be pass, fail, or not_applicable`);
  }
  if (!Array.isArray(criterion.sourceRefs) || !Array.isArray(criterion.claimIds)) {
    throw new Error(`${caseId}: rubric.${key} needs claimIds and sourceRefs arrays`);
  }
  if (criterion.claimIds.some((claimId) => typeof claimId !== "string" || !claimId.trim())
    || new Set(criterion.claimIds).size !== criterion.claimIds.length) {
    throw new Error(`${caseId}: rubric.${key} claimIds must be distinct non-empty strings`);
  }
  if (outcome !== "not_applicable" && (criterion.sourceRefs.length === 0
    || (criterion.claimIds.length === 0 && !(schemaValidation === "fail" && outcome === "fail")))) {
    throw new Error(`${caseId}: rubric.${key} needs source references and reviewed claim IDs`);
  }
  if (outcome === "not_applicable") {
    const dimensions = criterionApplicability[key] ?? [];
    if (dimensions.length === 0 || dimensions.some((dimension) => manifestCase.applicability?.[dimension]?.status !== "notApplicable")) {
      throw new Error(`${caseId}: rubric.${key} is required for this case and cannot be marked notApplicable`);
    }
    if (typeof criterion.reason !== "string" || !criterion.reason.trim()) {
      throw new Error(`${caseId}: rubric.${key} needs a reason when marked notApplicable`);
    }
    if (criterion.claimIds.length > 0 || criterion.sourceRefs.length > 0) {
      throw new Error(`${caseId}: notApplicable rubric.${key} must not cite an unreviewed chapter claim`);
    }
  } else if (criterion.reason !== null) {
    throw new Error(`${caseId}: rubric.${key} reason must be null unless marked notApplicable`);
  }
  return outcome;
}

function validateFactCoverage(coverage, manifestCase, artifact, label, outcome) {
  exactKeys(coverage, manifestCase.mustCover, `${label} factCoverage`);
  for (const factId of manifestCase.mustCover) {
    const claimIds = coverage[factId];
    if (!Array.isArray(claimIds) || new Set(claimIds).size !== claimIds.length
      || claimIds.some((claimId) => typeof claimId !== "string" || !artifact.sectionClaimIds.has(claimId))) {
      throw new Error(`${label} factCoverage.${factId} must list distinct positional section-claim IDs from this attempt's artifact`);
    }
    if (outcome === "pass" && claimIds.length === 0) {
      throw new Error(`${label} passes while mustCover fact ${factId} has no reviewed claim`);
    }
    const fact = manifestCase.expectedFacts.find((candidate) => candidate.id === factId);
    if (claimIds.some((claimId) => !artifact.claimEvidenceById?.get(claimId)?.some((evidence) => (
      evidence.path === fact.path
      && evidence.byteStart <= fact.byteStart
      && evidence.byteEndExclusive >= fact.byteEndExclusive
    )))) {
      throw new Error(`${label} factCoverage.${factId} must map to a section claim whose exact evidence span contains the expected fact anchor in ${fact.path}`);
    }
  }
}

async function validateReportRubric(caseId, rubric, manifestCase, schemaValidation, artifact, manifestPath, label) {
  if (!rubric || typeof rubric !== "object") throw new Error(`${label}: missing binary rubric results`);
  exactKeys(rubric, rubricKeys, `${label} rubric`);
  const outcomes = [];
  for (const key of rubricKeys) {
    const outcome = validateCriterion(caseId, key, rubric[key], manifestCase, schemaValidation);
    outcomes.push(outcome);
    for (const claimId of rubric[key].claimIds) {
      if (!artifact.claimIds.has(claimId)) throw new Error(`${label} rubric.${key} references unknown claim ID ${claimId}`);
    }
    for (const reference of rubric[key].sourceRefs ?? []) {
      await validateSourceRef(reference, manifestPath, manifestCase, `${label} rubric.${key}`);
    }
  }
  return outcomes;
}

function lineNumberAtOffset(source, offset) {
  let line = 1;
  for (let index = 0; index < offset; index += 1) {
    if (source[index] === "\n") line += 1;
  }
  return line;
}

function exactQuoteAnchor(source, quote, label) {
  if (typeof quote !== "string" || !quote.trim() || byteLength(quote) < 8 || byteLength(quote) > 1_000) {
    throw new Error(`${label} needs an exact source quote containing 8-1,000 UTF-8 bytes`);
  }
  const start = source.indexOf(quote);
  const firstCodePoint = String.fromCodePoint(quote.codePointAt(0));
  if (start < 0 || source.indexOf(quote, start + firstCodePoint.length) >= 0) {
    throw new Error(`${label} quote must occur exactly once in its captured file`);
  }
  const startLine = lineNumberAtOffset(source, start);
  let lastContent = start + quote.length - 1;
  while (lastContent >= start && (source[lastContent] === "\r" || source[lastContent] === "\n")) lastContent -= 1;
  if (lastContent < start) throw new Error(`${label} quote cannot consist only of line endings`);
  return {
    startLine,
    endLine: lineNumberAtOffset(source, lastContent),
    byteStart: Buffer.byteLength(source.slice(0, start), "utf8"),
    byteEndExclusive: Buffer.byteLength(source.slice(0, start + quote.length), "utf8"),
  };
}

function exactKeys(value, expectedKeys, label) {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(`${label} must be an object`);
  const actual = Object.keys(value).sort();
  const expected = [...expectedKeys].sort();
  if (stableJson(actual) !== stableJson(expected)) throw new Error(`${label} has missing or unsupported properties`);
}

function byteLength(value) {
  return Buffer.byteLength(value, "utf8");
}

function validateIdentifierList(identifiers, label) {
  if (!Array.isArray(identifiers) || identifiers.length < 1 || identifiers.length > 8
    || identifiers.some((identifier) => typeof identifier !== "string" || !identifier.trim() || identifier !== identifier.trim())
    || new Set(identifiers).size !== identifiers.length) {
    throw new Error(`${label} must contain one to eight distinct, non-empty identifiers`);
  }
}

function validateEvidenceList(evidence, prose, identifiers, label, sources, targetPath = undefined) {
  if (!Array.isArray(evidence) || evidence.length < 1 || evidence.length > 8) {
    throw new Error(`${label} must contain one to eight exact source quotes`);
  }
  const verifiedIdentifiers = new Set();
  let targetAnchored = false;
  for (const [index, item] of evidence.entries()) {
    exactKeys(item, ["path", "quote"], `${label} evidence ${index + 1}`);
    if (typeof item.path !== "string" || typeof item.quote !== "string") {
      throw new Error(`${label} evidence ${index + 1} needs a path and quote`);
    }
    const source = sources.get(item.path);
    if (!source) throw new Error(`${label} evidence ${index + 1} cites a file outside the selected corpus files`);
    if (byteLength(item.quote) < 8 || byteLength(item.quote) > 1_000 || !item.quote.trim()) {
      throw new Error(`${label} evidence ${index + 1} quote must contain 8-1,000 non-whitespace UTF-8 bytes`);
    }
    exactQuoteAnchor(source.text, item.quote, `${label} evidence ${index + 1}`);
    const matched = identifiers.filter((identifier) => identifierAppears(item.quote, identifier));
    if (matched.length === 0) throw new Error(`${label} evidence ${index + 1} does not name a declared identifier`);
    for (const identifier of matched) verifiedIdentifiers.add(identifier);
    if (targetPath !== undefined && item.path === targetPath) targetAnchored = true;
  }
  for (const identifier of identifiers) {
    if (!identifierAppears(prose, identifier)) throw new Error(`${label} prose does not contain identifier ${identifier}`);
    if (!verifiedIdentifiers.has(identifier)) throw new Error(`${label} evidence does not substantiate identifier ${identifier}`);
  }
  return { targetAnchored };
}

function validateProposalArtifact(proposal, label, sources, targetPath, allowedRelationshipIds = new Set()) {
  const claimIds = new Set();
  const textField = (value, minimum, maximum, fieldLabel) => {
    if (typeof value !== "string" || byteLength(value.trim()) < minimum || byteLength(value.trim()) > maximum) {
      throw new Error(`${fieldLabel} must contain ${minimum}-${maximum} non-whitespace UTF-8 bytes`);
    }
  };
  exactKeys(proposal, ["title", "sections", "evidenceLimits"], label);
  if (typeof proposal.title !== "string" || Array.from(proposal.title.trim()).length < 8 || Array.from(proposal.title.trim()).length > 120
    || ["overview", "file explanation", "code overview", "understanding this file"].some((generic) => generic.toLowerCase() === proposal.title.trim().toLowerCase())) {
    throw new Error(`${label} title must be specific and contain 8-120 characters`);
  }
  if (!Array.isArray(proposal.sections) || proposal.sections.length < 1 || proposal.sections.length > 12) {
    throw new Error(`${label} must contain 1-12 sections`);
  }
  if (!Array.isArray(proposal.evidenceLimits) || proposal.evidenceLimits.length < 1 || proposal.evidenceLimits.length > 5) {
    throw new Error(`${label} must contain 1-5 evidence limits`);
  }
  let hasResponsibility = false;
  for (const [sectionIndex, section] of proposal.sections.entries()) {
    const sectionLabel = `${label} section ${sectionIndex + 1}`;
    exactKeys(section, ["title", "prose", "spokenText", "claims"], sectionLabel);
    if (typeof section.title !== "string" || !section.title.trim() || byteLength(section.title) > 120
      || typeof section.prose !== "string" || byteLength(section.prose.trim()) < 20 || byteLength(section.prose.trim()) > 4_000
      || typeof section.spokenText !== "string" || byteLength(section.spokenText) > 1_000 || section.spokenText.includes("```")) {
      throw new Error(`${sectionLabel} has invalid title, prose, or narration lengths`);
    }
    if (!Array.isArray(section.claims) || section.claims.length < 1 || section.claims.length > 20) {
      throw new Error(`${sectionLabel} must contain 1-20 claims`);
    }
    let sectionTargetAnchored = false;
    for (const [claimIndex, claim] of section.claims.entries()) {
      const claimLabel = `${sectionLabel} claim ${claimIndex + 1}`;
      exactKeys(claim, ["category", "aspect", "prose", "inferenceBasis", "identifiers", "evidence", "relationshipIds"], claimLabel);
      if (!new Set(["codeBehavior", "repositoryContext", "inference"]).has(claim.category)
        || !new Set(["responsibility", "execution", "inputOutput", "stateEffect", "failurePath", "dependency", "relationship"]).has(claim.aspect)
        || typeof claim.prose !== "string" || byteLength(claim.prose.trim()) < 24 || byteLength(claim.prose.trim()) > 2_000
        || ["this file plays an important role", "it is important to note", "this file contains code"].some((filler) => claim.prose.toLowerCase().includes(filler))
        || typeof claim.inferenceBasis !== "string") {
        throw new Error(`${claimLabel} does not match the generated claim contract`);
      }
      if (claim.category === "inference" && !claim.inferenceBasis.trim()) throw new Error(`${claimLabel} inference needs a basis`);
      validateIdentifierList(claim.identifiers, `${claimLabel} identifiers`);
      if (!Array.isArray(claim.relationshipIds) || claim.relationshipIds.length > 8
        || claim.relationshipIds.some((id) => typeof id !== "string" || !id.trim())
        || new Set(claim.relationshipIds).size !== claim.relationshipIds.length) {
        throw new Error(`${claimLabel} relationshipIds must contain at most eight distinct, non-empty strings`);
      }
      if (claim.relationshipIds.some((id) => !allowedRelationshipIds.has(id))) {
        throw new Error(`${claimLabel} cites a relationship that is absent from this corpus case's reviewed selected-index context`);
      }
      if (claim.aspect === "relationship" && claim.relationshipIds.length === 0) {
        throw new Error(`${claimLabel} relationship aspect needs an indexed relationship ID`);
      }
      const anchors = validateEvidenceList(claim.evidence, claim.prose.trim(), claim.identifiers, claimLabel, sources, targetPath);
      sectionTargetAnchored ||= anchors.targetAnchored;
      if (claim.aspect === "responsibility") {
        if (!anchors.targetAnchored) throw new Error(`${claimLabel} responsibility must cite the target file`);
        hasResponsibility = true;
      }
      claimIds.add(`sections[${sectionIndex}].claims[${claimIndex}]`);
    }
    if (!sectionTargetAnchored) throw new Error(`${sectionLabel} has no evidence from the target file`);
  }
  if (!hasResponsibility) throw new Error(`${label} has no target-anchored responsibility claim`);
  for (const [index, limit] of proposal.evidenceLimits.entries()) {
    const limitLabel = `${label} evidence limit ${index + 1}`;
    exactKeys(limit, ["prose", "basis", "identifiers", "evidence"], limitLabel);
    textField(limit.prose, 24, 800, `${limitLabel} prose`);
    textField(limit.basis, 12, 500, `${limitLabel} basis`);
    validateIdentifierList(limit.identifiers, `${limitLabel} identifiers`);
    const anchors = validateEvidenceList(limit.evidence, limit.prose.trim(), limit.identifiers, limitLabel, sources, targetPath);
    if (!anchors.targetAnchored) throw new Error(`${limitLabel} must cite the target file`);
    claimIds.add(`evidenceLimits[${index}]`);
  }
  return claimIds;
}

function collectProposalClaimIds(proposal) {
  const claimIds = new Set();
  if (Array.isArray(proposal?.sections)) {
    proposal.sections.forEach((section, sectionIndex) => {
      if (Array.isArray(section?.claims)) {
        section.claims.forEach((_, claimIndex) => claimIds.add(`sections[${sectionIndex}].claims[${claimIndex}]`));
      }
    });
  }
  if (Array.isArray(proposal?.evidenceLimits)) {
    proposal.evidenceLimits.forEach((_, index) => claimIds.add(`evidenceLimits[${index}]`));
  }
  return claimIds;
}

function collectSectionClaimIds(proposal) {
  const claimIds = new Set();
  if (Array.isArray(proposal?.sections)) {
    proposal.sections.forEach((section, sectionIndex) => {
      if (!Array.isArray(section?.claims)) return;
      section.claims.forEach((_, claimIndex) => claimIds.add(`sections[${sectionIndex}].claims[${claimIndex}]`));
    });
  }
  return claimIds;
}

function collectSectionClaimEvidenceById(proposal, sources) {
  const evidenceById = new Map();
  if (Array.isArray(proposal?.sections)) {
    proposal.sections.forEach((section, sectionIndex) => {
      if (!Array.isArray(section?.claims)) return;
      section.claims.forEach((claim, claimIndex) => {
        const claimId = `sections[${sectionIndex}].claims[${claimIndex}]`;
        const anchors = [];
        if (Array.isArray(claim?.evidence)) {
          for (const evidence of claim.evidence) {
            if (typeof evidence?.path !== "string" || typeof evidence?.quote !== "string") continue;
            const source = sources.get(evidence.path);
            if (!source) continue;
            try {
              const anchor = exactQuoteAnchor(source.text, evidence.quote, `${claimId} fact-coverage evidence`);
              anchors.push({ path: evidence.path, byteStart: anchor.byteStart, byteEndExclusive: anchor.byteEndExclusive });
            } catch {
              // Invalid or ambiguous evidence cannot establish fact coverage.
            }
          }
        }
        evidenceById.set(claimId, anchors);
      });
    });
  }
  return evidenceById;
}

async function loadManifestSources(manifestPath, manifestCase) {
  const corpusRoot = dirname(manifestPath);
  const corpusRootReal = await realpath(corpusRoot);
  const sources = new Map();
  for (const file of manifestCase.files) {
    const sourceAbsolute = resolve(corpusRoot, file.sourceFile);
    const sourceReal = await realpath(sourceAbsolute);
    const sourceRelative = relative(corpusRootReal, sourceReal);
    if (sourceRelative.startsWith("..") || isAbsolute(sourceRelative)) throw new Error(`${manifestCase.id}: source resolves outside the corpus`);
    const bytes = await readFile(sourceReal);
    if (sha256(bytes) !== file.sha256) throw new Error(`${manifestCase.id}: source hash changed for ${file.path}`);
    const text = bytes.toString("utf8");
    if (!Buffer.from(text, "utf8").equals(bytes)) throw new Error(`${manifestCase.id}: source file is not valid UTF-8`);
    sources.set(file.path, { text, sha256: file.sha256 });
  }
  return sources;
}

async function readChapterArtifact(path, digest, label, reportsDirectory, manifestPath, manifestCase, schemaValidation) {
  const bytes = await readVerifiedArtifact(path, digest, label, reportsDirectory);
  if (bytes.byteLength > 300 * 1024) throw new Error(`${label} exceeds the 300 KiB provider response limit`);
  let proposal;
  let parseError;
  try {
    const jsonText = bytes.toString("utf8");
    if (!Buffer.from(jsonText, "utf8").equals(bytes)) throw new Error("artifact is not valid UTF-8");
    proposal = parseJsonWithoutDuplicateKeys(jsonText, label);
  } catch (error) {
    parseError = error;
  }
  if (schemaValidation === "pass" && parseError) {
    throw new Error(`${label} is marked schema-valid but cannot be parsed: ${parseError.message}`);
  }
  if (schemaValidation === "fail" && parseError) {
    return { bytes, proposal: null, claimIds: new Set(), sectionClaimIds: new Set(), claimEvidenceById: new Map(), contractValid: false, parseError };
  }
  if (schemaValidation !== "pass" && schemaValidation !== "fail") {
    throw new Error(`${label} schemaValidation must be pass or fail`);
  }
  const claimIds = collectProposalClaimIds(proposal);
  const sectionClaimIds = collectSectionClaimIds(proposal);
  const sources = await loadManifestSources(manifestPath, manifestCase);
  const claimEvidenceById = collectSectionClaimEvidenceById(proposal, sources);
  const allowedRelationshipIds = new Set(manifestCase.allowedRelationshipIds ?? []);
  let contractError;
  try {
    validateProposalArtifact(proposal, label, sources, manifestCase.path, allowedRelationshipIds);
  } catch (error) {
    contractError = error;
  }
  if (schemaValidation === "pass" && contractError) {
    throw new Error(`${label} is marked schema-valid but fails local proposal/evidence validation: ${contractError.message}`);
  }
  if (schemaValidation === "fail" && !contractError) {
    throw new Error(`${label} passes local proposal/evidence validation but is recorded with schemaValidation fail`);
  }
  return { bytes, proposal, claimIds, sectionClaimIds, claimEvidenceById, contractValid: !contractError, contractError };
}

async function validateSourceRef(reference, manifestPath, manifestCase, label) {
  if (!reference || typeof reference.path !== "string" || typeof reference.quote !== "string") {
    throw new Error(`${label} needs a source path and exact quote`);
  }
  exactKeys(reference, ["path", "sourceSha256", "startLine", "endLine", "quote"], label);
  const file = manifestCase.files.find((item) => item.path === reference.path);
  if (!file || reference.sourceSha256 !== file.sha256) throw new Error(`${label} does not point to a captured file hash for this case`);
  const corpusRoot = dirname(manifestPath);
  const sourceAbsolute = resolve(corpusRoot, file.sourceFile);
  const sourceReal = await realpath(sourceAbsolute);
  const corpusReal = await realpath(corpusRoot);
  const sourceRelative = relative(corpusReal, sourceReal);
  if (sourceRelative.startsWith("..") || isAbsolute(sourceRelative)) throw new Error(`${label} resolves outside the corpus`);
  const sourceBytes = await readFile(sourceReal);
  if (sha256(sourceBytes) !== file.sha256) throw new Error(`${label} source file changed during report validation`);
  const source = sourceBytes.toString("utf8");
  if (!Buffer.from(source, "utf8").equals(sourceBytes)) throw new Error(`${label} source file is not valid UTF-8`);
  const anchor = exactQuoteAnchor(source, reference.quote, label);
  if (reference.startLine !== anchor.startLine || reference.endLine !== anchor.endLine) {
    throw new Error(`${label} line range must exactly match its unique quote (${anchor.startLine}-${anchor.endLine})`);
  }
}

async function validateFindings(findings, manifestPath, manifestCase, label, claimIds) {
  if (!Array.isArray(findings)) throw new Error(`${label} must be an array`);
  for (const [index, finding] of findings.entries()) {
    exactKeys(finding, ["category", "claimIds", "sourceRefs", "description"], `${label}[${index}]`);
    if (!allowedFailureCategories.has(finding.category) || typeof finding.description !== "string" || !finding.description.trim()
      || !Array.isArray(finding.claimIds) || finding.claimIds.some((claimId) => typeof claimId !== "string" || !claimId.trim())
      || new Set(finding.claimIds).size !== finding.claimIds.length
      || !Array.isArray(finding.sourceRefs) || finding.sourceRefs.length === 0) {
      throw new Error(`${label}[${index}] needs an allowed category, description, claim IDs, and source references`);
    }
    for (const claimId of finding.claimIds) {
      if (!claimIds.has(claimId)) throw new Error(`${label}[${index}] references unknown claim ID ${claimId}`);
    }
    for (const reference of finding.sourceRefs) await validateSourceRef(reference, manifestPath, manifestCase, `${label}[${index}]`);
  }
}

function identifierAppears(text, identifier) {
  const escaped = identifier.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const isIdentifierCharacter = (character) => character !== undefined && /^[\p{L}\p{N}_$]$/u.test(character);
  const startsAsIdentifier = isIdentifierCharacter(Array.from(identifier)[0]);
  const endsAsIdentifier = isIdentifierCharacter(Array.from(identifier).at(-1));
  for (const match of text.matchAll(new RegExp(escaped, "gu"))) {
    const before = Array.from(text.slice(0, match.index)).at(-1);
    const after = Array.from(text.slice(match.index + match[0].length))[0];
    if ((!startsAsIdentifier || !isIdentifierCharacter(before)) && (!endsAsIdentifier || !isIdentifierCharacter(after))) return true;
  }
  return false;
}

async function readVerifiedArtifact(artifactPath, artifactSha256, label, allowedDirectory) {
  requireHash(artifactSha256, `${label} artifactSha256`);
  if (typeof artifactPath !== "string" || !artifactPath.trim()) throw new Error(`${label} needs a local artifact path`);
  const artifactAbsolute = resolve(root, artifactPath);
  const artifactRelative = relative(root, artifactAbsolute);
  if (artifactRelative.startsWith("..") || isAbsolute(artifactRelative)) throw new Error(`${label} artifact path must stay inside the workspace`);
  const workspaceReal = await realpath(root);
  const artifactReal = await realpath(artifactAbsolute);
  const artifactRealRelative = relative(workspaceReal, artifactReal);
  if (artifactRealRelative.startsWith("..") || isAbsolute(artifactRealRelative)) throw new Error(`${label} artifact resolves outside the workspace`);
  if (allowedDirectory) {
    const allowedReal = await realpath(resolve(root, allowedDirectory));
    const allowedRelative = relative(allowedReal, artifactReal);
    if (allowedRelative.startsWith("..") || isAbsolute(allowedRelative)) {
      throw new Error(`${label} artifact must stay in ${allowedDirectory}`);
    }
  }
  const bytes = await readFile(artifactReal);
  if (sha256(bytes) !== artifactSha256) throw new Error(`${label} artifact digest does not match its file`);
  return bytes;
}

async function validateQualityGateExample(example, pinnedExample) {
  if (!example || typeof example !== "object") throw new Error("qualityGateExample is required");
  exactKeys(example, ["kind", "artifactPath", "artifactSha256", "authoring", "providerRun", "schemaValid", "shallowOutcome", "failureCategories", "findings", "repairedOutcome", "humanAssessment"], "qualityGateExample report fixture");
  if (!pinnedExample || stableJson(example) !== stableJson(pinnedExample)) {
    throw new Error("qualityGateExample must exactly match the checked-in template's pinned fixture path, digest, assessments, and findings");
  }
  if (example.kind !== "fixture/rubric-example" || typeof example.authoring !== "string" || !example.authoring.startsWith("human-authored")
    || example.schemaValid !== true || example.providerRun !== false) {
    throw new Error("qualityGateExample must be an explicitly local, schema-valid rubric fixture, not a provider run");
  }
  exactKeys(example.humanAssessment, ["kind", "reviewer", "reviewedAt", "shallowOutcome", "failureCategories", "repairedOutcome"], "qualityGateExample humanAssessment");
  if (example.shallowOutcome !== "fail" || example.repairedOutcome !== "pass") {
    throw new Error("qualityGateExample must show the shallow candidate failing and its repair passing");
  }
  if (!Array.isArray(example.findings) || example.findings.length === 0 || !Array.isArray(example.failureCategories) || !example.failureCategories.some((category) => allowedFailureCategories.has(category))) {
    throw new Error("qualityGateExample needs a recognized failure category");
  }
  const examplePath = "benchmarks/chapter-quality/review/QUALITY_GATE_EXAMPLE.json";
  if (example.artifactPath !== examplePath) throw new Error("qualityGateExample must reference the checked-in shallow-versus-repaired example");
  const bytes = await readVerifiedArtifact(example.artifactPath, example.artifactSha256, "qualityGateExample");
  const artifact = parseJsonWithoutDuplicateKeys(bytes.toString("utf8"), "qualityGateExample artifact");
  exactKeys(artifact, ["kind", "authoring", "providerRun", "schemaValid", "source", "shallowCandidate", "shallowAssessment", "repairedCandidate", "repairedAssessment"], "qualityGateExample artifact");
  if (artifact.kind !== example.kind || typeof artifact.authoring !== "string" || !artifact.authoring.startsWith("human-authored")
    || artifact.providerRun !== false || artifact.schemaValid !== true) {
    throw new Error("qualityGateExample artifact is not the declared local rubric fixture");
  }
  exactKeys(artifact.source, ["path", "sourceExcerptSha256", "sourceExcerpt"], "qualityGateExample source");
  exactKeys(artifact.shallowAssessment, ["outcome", "failureCategories", "findings"], "qualityGateExample shallowAssessment");
  exactKeys(artifact.repairedAssessment, ["outcome", "failureCategories", "findings", "repairSummary"], "qualityGateExample repairedAssessment");
  if (example.shallowOutcome !== artifact.shallowAssessment?.outcome
    || example.repairedOutcome !== artifact.repairedAssessment?.outcome
    || JSON.stringify([...example.failureCategories].sort()) !== JSON.stringify([...artifact.shallowAssessment.failureCategories].sort())) {
    throw new Error("qualityGateExample report fields disagree with its checked-in artifact");
  }
  if (example.humanAssessment.kind !== "authored-rubric-demonstration" || typeof example.humanAssessment.reviewer !== "string"
    || !example.humanAssessment.reviewer.trim() || example.humanAssessment.reviewedAt !== null
    || example.humanAssessment.shallowOutcome !== artifact.shallowAssessment.outcome
    || example.humanAssessment.repairedOutcome !== artifact.repairedAssessment.outcome
    || !Array.isArray(example.humanAssessment.failureCategories)
    || stableJson([...example.humanAssessment.failureCategories].sort()) !== stableJson([...artifact.shallowAssessment.failureCategories].sort())) {
    throw new Error("qualityGateExample humanAssessment must match the checked-in authored shallow and repaired results");
  }
  if (stableJson(example.findings) !== stableJson(artifact.shallowAssessment?.findings)) {
    throw new Error("qualityGateExample findings must exactly match the checked-in shallow assessment");
  }
  if (!Array.isArray(artifact.repairedAssessment?.findings) || artifact.repairedAssessment.findings.length !== 0
    || !Array.isArray(artifact.repairedAssessment.failureCategories) || artifact.repairedAssessment.failureCategories.length !== 0
    || typeof artifact.repairedAssessment.repairSummary !== "string" || !artifact.repairedAssessment.repairSummary.trim()) {
    throw new Error("qualityGateExample repair must record a passing assessment, no findings, and a repair summary");
  }
  const exampleFindingCategories = [...new Set(example.findings.map((finding) => finding.category))].sort();
  const artifactFindingCategories = [...new Set(artifact.shallowAssessment.findings.map((finding) => finding.category))].sort();
  if (JSON.stringify(exampleFindingCategories) !== JSON.stringify([...example.failureCategories].sort())
    || JSON.stringify(artifactFindingCategories) !== JSON.stringify([...example.failureCategories].sort())) {
    throw new Error("qualityGateExample must preserve a finding for every failure category");
  }
  const source = artifact.source;
  if (!source || typeof source.sourceExcerpt !== "string" || sha256(Buffer.from(source.sourceExcerpt, "utf8")) !== source.sourceExcerptSha256) {
    throw new Error("qualityGateExample source excerpt hash is invalid");
  }
  const chapterGeneration = await readFile(resolve(root, "src-tauri/src/chapter_generation.rs"), "utf8");
  const simpleSourceLiteral = chapterGeneration.match(/const SIMPLE_SOURCE: &str =\s*("(?:[^"\\]|\\.)*")\s*;/)?.[1];
  if (source.path !== "src/greeting.ts" || !simpleSourceLiteral
    || !JSON.parse(simpleSourceLiteral).includes(source.sourceExcerpt)) {
    throw new Error("qualityGateExample source excerpt must come from the checked-in SIMPLE_SOURCE Rust fixture");
  }
  exactQuoteAnchor(JSON.parse(simpleSourceLiteral), source.sourceExcerpt, "qualityGateExample source excerpt");
  const exampleSources = new Map([[source.path, { text: source.sourceExcerpt, sha256: source.sourceExcerptSha256 }]]);
  const shallowClaimIds = new Set([...validateProposalArtifact(artifact.shallowCandidate, "qualityGateExample shallowCandidate", exampleSources, source.path)]
    .map((claimId) => `shallowCandidate.${claimId}`));
  validateProposalArtifact(artifact.repairedCandidate, "qualityGateExample repairedCandidate", exampleSources, source.path);
  for (const [index, finding] of example.findings.entries()) {
    exactKeys(finding, ["category", "claimIds", "sourceRefs", "description"], `qualityGateExample finding ${index + 1}`);
    if (!allowedFailureCategories.has(finding.category) || !Array.isArray(finding.claimIds) || finding.claimIds.length === 0
      || !Array.isArray(finding.sourceRefs) || finding.sourceRefs.length === 0 || typeof finding.description !== "string") {
      throw new Error(`qualityGateExample finding ${index + 1} is incomplete`);
    }
    if (finding.claimIds.some((claimId) => !shallowClaimIds.has(claimId))) throw new Error(`qualityGateExample finding ${index + 1} references an unknown shallow claim`);
    for (const reference of finding.sourceRefs) {
      exactKeys(reference, ["path", "sourceSha256", "startLine", "endLine", "quote"], `qualityGateExample finding ${index + 1} sourceRef`);
      const anchor = exactQuoteAnchor(source.sourceExcerpt, reference.quote, `qualityGateExample finding ${index + 1}`);
      if (reference.path !== source.path || reference.sourceSha256 !== source.sourceExcerptSha256
        || reference.startLine !== anchor.startLine || reference.endLine !== anchor.endLine) {
        throw new Error(`qualityGateExample finding ${index + 1} is not anchored to its exact source excerpt`);
      }
    }
  }
  if (!Array.isArray(artifact.shallowAssessment.findings) || artifact.shallowAssessment.findings.length === 0) {
    throw new Error("qualityGateExample shallow assessment needs concrete human findings");
  }
}

async function validateReport(reportArg, manifestArg) {
  if (!reportArg) throw new Error("--report needs a completed report path; the pending template is not a release result");
  const loaded = await loadManifest(manifestArg);
  const reportsDirectory = "benchmarks/chapter-quality/review/reports";
  const reportsAbsolute = resolve(root, reportsDirectory);
  const workspaceReal = await realpath(root);
  let reportsReal;
  try {
    reportsReal = await realpath(reportsAbsolute);
  } catch (error) {
    if (error.code === "ENOENT") {
      throw new Error(`The local reports directory is missing. Create it and copy the template first: mkdir -p ${reportsDirectory} && cp benchmarks/chapter-quality/review/REPORT_TEMPLATE.json ${reportsDirectory}/run.json`);
    }
    throw error;
  }
  const reportsWorkspaceRelative = relative(workspaceReal, reportsReal);
  if (reportsWorkspaceRelative.startsWith("..") || isAbsolute(reportsWorkspaceRelative)) {
    throw new Error(`${reportsDirectory} must resolve inside this workspace`);
  }
  const reportPath = resolve(root, reportArg);
  let reportReal;
  try {
    reportReal = await realpath(reportPath);
  } catch (error) {
    if (error.code === "ENOENT") throw new Error(`Report file is missing. Copy REPORT_TEMPLATE.json into ${reportsDirectory}/, fill it, then rerun the gate.`);
    throw error;
  }
  const workspaceRelative = relative(workspaceReal, reportReal);
  if (workspaceRelative.startsWith("..") || isAbsolute(workspaceRelative)) {
    throw new Error("Report path resolves outside the workspace");
  }
  let report;
  try {
    const reportBytes = await readFile(reportReal);
    const reportText = reportBytes.toString("utf8");
    if (!Buffer.from(reportText, "utf8").equals(reportBytes)) throw new Error("Report is not valid UTF-8");
    report = parseJsonWithoutDuplicateKeys(reportText, "Report JSON");
  } catch (error) {
    throw new Error(`Could not read report JSON: ${error.message}`);
  }
  exactKeys(report, ["reportSchemaVersion", "kind", "status", "corpusId", "corpusManifestSha256", "modelId", "modelVersion", "modelAllowlistSha256", "promptVersion", "promptSha256", "requestAssemblySha256", "generationConfigSha256", "chapterSchemaVersion", "responseSchemaVersion", "responseSchemaSha256", "reviewer", "reviewedAt", "qualityGateExample", "cases"], "Report");
  if (report.reportSchemaVersion !== 3 || report.kind !== "chapter-quality-review-report" || !["pass", "fail"].includes(report.status)) {
    throw new Error("Report must use schema version 3 and final status pass or fail; pending or partial reports cannot gate release");
  }
  const reportRealRelative = relative(reportsReal, reportReal);
  if (reportRealRelative.startsWith("..") || isAbsolute(reportRealRelative)) {
    throw new Error(`Completed reports must be stored under ${reportsDirectory}/`);
  }
  if (report.corpusId !== loaded.manifest.corpusId) throw new Error("Report corpusId does not match the current manifest");
  requireHash(report.corpusManifestSha256, "corpusManifestSha256");
  if (report.corpusManifestSha256 !== loaded.manifestSha256) throw new Error("Report was reviewed against a different corpus manifest");

  const current = await currentGenerationMetadata();
  if (report.promptVersion !== current.promptVersion) throw new Error(`Report promptVersion is stale; current version is ${current.promptVersion}`);
  requireHash(report.promptSha256, "promptSha256");
  if (report.promptSha256 !== current.promptSha256) throw new Error("Report promptSha256 is stale; rerun quality review against the current prompt text");
  if (Number(report.chapterSchemaVersion) !== current.chapterSchemaVersion) throw new Error(`Report chapterSchemaVersion is stale; current version is ${current.chapterSchemaVersion}`);
  if (report.responseSchemaVersion !== current.responseSchemaVersion) throw new Error(`Report responseSchemaVersion is stale; current version is ${current.responseSchemaVersion}`);
  for (const key of ["modelAllowlistSha256", "responseSchemaSha256"]) {
    requireHash(report[key], key);
    if (report[key] !== current[key]) throw new Error(`Report ${key} is stale; rerun quality review against the current generation contract`);
  }
  for (const key of ["requestAssemblySha256", "generationConfigSha256"]) {
    requireHash(report[key], key);
    if (report[key] !== current[key]) throw new Error(`Report ${key} is stale; rerun quality review against the current exact request generation contract`);
  }
  if (!current.modelIds.includes(report.modelId)) throw new Error("Report modelId is not in the current provider allowlist");
  if (typeof report.modelVersion !== "string" || !report.modelVersion.trim()) {
    throw new Error('Report needs the provider-returned modelVersion, or the explicit value "not-returned"');
  }
  if (typeof report.reviewer !== "string" || !report.reviewer.trim()) throw new Error("Report needs a local reviewer name or identifier");
  if (typeof report.reviewedAt !== "string"
    || !/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})$/.test(report.reviewedAt)
    || Number.isNaN(Date.parse(report.reviewedAt))) throw new Error("Report needs an ISO-8601 reviewedAt timestamp with a timezone");
  const reportTemplate = await loadReportTemplate();
  if (reportTemplate.status !== "pending") throw new Error("The checked-in report template must remain pending and act as the local review baseline");
  await validateQualityGateExample(reportTemplate.qualityGateExample, reportTemplate.qualityGateExample);
  await validateQualityGateExample(report.qualityGateExample, reportTemplate.qualityGateExample);

  if (!Array.isArray(report.cases)) throw new Error("Report needs a cases array");
  if (report.cases.some((item) => !item || typeof item.caseId !== "string")) throw new Error("Every report case needs a caseId");
  const byId = new Map(report.cases.map((item) => [item.caseId, item]));
  if (byId.size !== loaded.caseIds.length || report.cases.length !== byId.size || loaded.caseIds.some((id) => !byId.has(id))) {
    throw new Error("Report case IDs must match every current corpus case exactly once");
  }

  const failures = [];
  for (const caseId of loaded.caseIds) {
    const item = byId.get(caseId);
    const manifestCase = loaded.manifest.cases.find((candidate) => candidate.id === caseId);
    exactKeys(item, ["caseId", "repositoryId", "targetPath", "sourceHashesByPath", "requestFingerprint", "chapterArtifactPath", "chapterArtifactSha256", "schemaValidation", "outcome", "failureCategories", "findings", "repairSummary", "factCoverage", "rubric", "attempts"], `${caseId} report case`);
    if (!["pass", "fail"].includes(item.outcome)) throw new Error(`${caseId}: final outcome must be pass or fail`);
    if (!Array.isArray(item.failureCategories) || new Set(item.failureCategories).size !== item.failureCategories.length
      || !Array.isArray(item.findings)) {
      throw new Error(`${caseId}: failureCategories and findings must be arrays`);
    }
    if (item.repositoryId !== manifestCase.repository || item.targetPath !== manifestCase.path) {
      throw new Error(`${caseId}: repositoryId or targetPath does not match the corpus manifest`);
    }
    const expectedSourceHashes = Object.fromEntries(manifestCase.files.map((file) => [file.path, file.sha256]));
    const actualSourceHashes = Object.fromEntries(Object.entries(item.sourceHashesByPath ?? {}).sort(([a], [b]) => a.localeCompare(b)));
    const sortedExpectedSourceHashes = Object.fromEntries(Object.entries(expectedSourceHashes).sort(([a], [b]) => a.localeCompare(b)));
    if (JSON.stringify(actualSourceHashes) !== JSON.stringify(sortedExpectedSourceHashes)) {
      throw new Error(`${caseId}: sourceHashesByPath must match every target and selected support file in the corpus`);
    }
    requireHash(item.requestFingerprint, `${caseId} requestFingerprint`);
    if (item.schemaValidation !== "pass" && item.schemaValidation !== "fail") {
      throw new Error(`${caseId}: schemaValidation must be pass or fail`);
    }
    const finalArtifact = await readChapterArtifact(item.chapterArtifactPath, item.chapterArtifactSha256, `${caseId} chapter`, reportsDirectory, loaded.manifestPath, manifestCase, item.schemaValidation);
    validateFactCoverage(item.factCoverage, manifestCase, finalArtifact, caseId, item.outcome);
    for (const category of item.failureCategories) {
      if (!allowedFailureCategories.has(category)) throw new Error(`${caseId}: unrecognized failure category ${category}`);
    }
    await validateFindings(item.findings, loaded.manifestPath, manifestCase, `${caseId} findings`, finalArtifact.claimIds);
    const finalFindingCategories = [...new Set(item.findings.map((finding) => finding.category))].sort();
    if (JSON.stringify(finalFindingCategories) !== JSON.stringify([...item.failureCategories].sort())) {
      throw new Error(`${caseId}: final failureCategories must match its concrete findings`);
    }
    const rubricOutcomes = await validateReportRubric(caseId, item.rubric, manifestCase, item.schemaValidation,
      finalArtifact, loaded.manifestPath, `${caseId} final`);
    const hasFailedCriterion = rubricOutcomes.includes("fail");
    if (item.outcome === "pass" && (hasFailedCriterion || item.schemaValidation !== "pass")) {
      throw new Error(`${caseId}: final pass conflicts with a failed schema or rubric criterion`);
    }
    if (item.outcome === "fail" && !hasFailedCriterion && item.schemaValidation !== "fail") {
      throw new Error(`${caseId}: fail must identify a failed schema or rubric criterion`);
    }

    if (!Array.isArray(item.attempts) || item.attempts.length === 0) throw new Error(`${caseId}: completed reviews must retain at least the initial attempt`);
    const attemptArtifactPaths = new Set();
    const attemptArtifactDigests = new Set();
    for (const [attemptIndex, attempt] of item.attempts.entries()) {
      exactKeys(attempt, ["label", "outcome", "schemaValidation", "modelId", "modelVersion", "requestFingerprint", "chapterArtifactPath", "chapterArtifactSha256", "failureCategories", "findings", "factCoverage", "rubric", "repairSummary"], `${caseId} attempt ${attemptIndex + 1}`);
      if (attempt.modelId !== report.modelId || attempt.modelVersion !== report.modelVersion) {
        throw new Error(`${caseId}: attempt ${attemptIndex + 1} model ID/version must match this benchmark batch`);
      }
      requireHash(attempt.requestFingerprint, `${caseId} attempt ${attemptIndex + 1} requestFingerprint`);
      if (attempt.label !== (attemptIndex === 0 ? "initial" : "repaired") || !["pass", "fail"].includes(attempt.outcome)) {
        throw new Error(`${caseId}: attempts must start with one initial result followed by repaired results`);
      }
      if (attempt.schemaValidation !== "pass" && attempt.schemaValidation !== "fail") {
        throw new Error(`${caseId}: attempt ${attemptIndex + 1} schemaValidation must be pass or fail`);
      }
      const attemptArtifact = await readChapterArtifact(attempt.chapterArtifactPath, attempt.chapterArtifactSha256, `${caseId} attempt ${attemptIndex + 1}`, reportsDirectory, loaded.manifestPath, manifestCase, attempt.schemaValidation);
      validateFactCoverage(attempt.factCoverage, manifestCase, attemptArtifact, `${caseId} attempt ${attemptIndex + 1}`, attempt.outcome);
      if (attemptArtifactPaths.has(attempt.chapterArtifactPath) || attemptArtifactDigests.has(attempt.chapterArtifactSha256)) {
        throw new Error(`${caseId}: each attempt must preserve a new artifact path and distinct content digest`);
      }
      attemptArtifactPaths.add(attempt.chapterArtifactPath);
      attemptArtifactDigests.add(attempt.chapterArtifactSha256);
      if (!Array.isArray(attempt.failureCategories) || new Set(attempt.failureCategories).size !== attempt.failureCategories.length
        || !Array.isArray(attempt.findings)) {
        throw new Error(`${caseId}: attempt ${attemptIndex + 1} needs failureCategories and findings arrays`);
      }
      for (const category of attempt.failureCategories) {
        if (!allowedFailureCategories.has(category)) throw new Error(`${caseId}: attempt has unrecognized failure category ${category}`);
      }
      await validateFindings(attempt.findings, loaded.manifestPath, manifestCase, `${caseId} attempt ${attemptIndex + 1} findings`, attemptArtifact.claimIds);
      const attemptRubricOutcomes = await validateReportRubric(caseId, attempt.rubric, manifestCase, attempt.schemaValidation,
        attemptArtifact, loaded.manifestPath, `${caseId} attempt ${attemptIndex + 1}`);
      const attemptFindingCategories = [...new Set(attempt.findings.map((finding) => finding.category))].sort();
      if (JSON.stringify(attemptFindingCategories) !== JSON.stringify([...attempt.failureCategories].sort())) {
        throw new Error(`${caseId}: attempt failureCategories must match its concrete findings`);
      }
      if (attempt.outcome === "fail" && (attempt.failureCategories.length === 0 || attempt.findings.length === 0)) {
        throw new Error(`${caseId}: failed attempt needs a failure category and concrete finding`);
      }
      if (attempt.outcome === "pass" && (attempt.failureCategories.length > 0 || attempt.findings.length > 0)) {
        throw new Error(`${caseId}: passing attempt cannot retain unresolved failure categories or findings`);
      }
      if (attempt.outcome === "pass" && attempt.schemaValidation !== "pass") {
        throw new Error(`${caseId}: a passing attempt must have schemaValidation pass`);
      }
      if (attempt.outcome === "pass" && attemptRubricOutcomes.includes("fail")) {
        throw new Error(`${caseId}: a passing attempt cannot retain a failed rubric criterion`);
      }
      if (attempt.outcome === "fail" && !attemptRubricOutcomes.includes("fail") && attempt.schemaValidation !== "fail") {
        throw new Error(`${caseId}: failed attempt must identify a failed rubric criterion or schema failure`);
      }
      if (attemptIndex === 0 && attempt.repairSummary !== null) {
        throw new Error(`${caseId}: the initial attempt repairSummary must be null`);
      }
      if (attemptIndex > 0 && (typeof attempt.repairSummary !== "string" || !attempt.repairSummary.trim())) {
        throw new Error(`${caseId}: every repaired attempt needs its own repairSummary`);
      }
      if (attemptIndex > 0 && item.attempts[attemptIndex - 1].outcome !== "fail") {
        throw new Error(`${caseId}: a repaired attempt must follow a failed attempt`);
      }
      if (attempt.outcome === "fail" && (attempt.failureCategories.length === 0 || attempt.findings.length === 0)) {
        throw new Error(`${caseId}: failed attempt needs a failure category and concrete finding`);
      }
    }
    const lastAttempt = item.attempts[item.attempts.length - 1];
    if (lastAttempt.outcome !== item.outcome || lastAttempt.chapterArtifactPath !== item.chapterArtifactPath
      || lastAttempt.chapterArtifactSha256 !== item.chapterArtifactSha256 || lastAttempt.schemaValidation !== item.schemaValidation
      || lastAttempt.requestFingerprint !== item.requestFingerprint) {
      throw new Error(`${caseId}: final case outcome, schema result, and artifact must match its last reviewed attempt`);
    }
    if (stableJson(item.findings) !== stableJson(lastAttempt.findings)
      || stableJson([...item.failureCategories].sort()) !== stableJson([...lastAttempt.failureCategories].sort())) {
      throw new Error(`${caseId}: final findings and failureCategories must exactly match the last reviewed attempt`);
    }
    if (item.repairSummary !== lastAttempt.repairSummary) {
      throw new Error(`${caseId}: final repairSummary must match the last reviewed attempt`);
    }
    if (stableJson(item.factCoverage) !== stableJson(lastAttempt.factCoverage)
      || stableJson(item.rubric) !== stableJson(lastAttempt.rubric)) {
      throw new Error(`${caseId}: final factCoverage and rubric must exactly match the last reviewed attempt`);
    }
    if (item.outcome === "pass" && item.attempts.some((attempt) => attempt.outcome === "fail")
      && (typeof item.repairSummary !== "string" || !item.repairSummary.trim())) {
      throw new Error(`${caseId}: a repaired final pass needs a repairSummary`);
    }
    if (item.outcome === "fail") {
      if (item.failureCategories.length === 0 || item.findings.length === 0) throw new Error(`${caseId}: failed result needs a failure category and concrete finding`);
      failures.push({ caseId, categories: item.failureCategories });
    } else if (item.failureCategories.length > 0 || item.findings.length > 0) {
      throw new Error(`${caseId}: a passing final result cannot retain unresolved failure categories or findings`);
    }
  }

  if ((report.status === "pass") !== (failures.length === 0)) {
    throw new Error(`Report status ${report.status} does not match the final case outcomes`);
  }
  if (failures.length > 0) {
    console.error(`Chapter quality review found ${failures.length} unresolved case(s):`);
    for (const failure of failures) console.error(`- ${failure.caseId}: ${failure.categories.join(", ")}`);
    process.exitCode = 1;
    return;
  }
  console.log(`Chapter quality gate passed: ${loaded.caseIds.length} reviewed cases; model ${report.modelId}; prompt ${report.promptVersion}; chapter schema ${report.chapterSchemaVersion}.`);
}

try {
  const args = process.argv.slice(2);
  if (args.includes("--help") || args.includes("-h")) {
    usage();
  } else if (args.includes("--report")) {
    await validateReport(readOption(args, "--report"), readOption(args, "--manifest", defaultManifest));
  } else if (args.includes("--corpus")) {
    await validateCorpus(readOption(args, "--corpus", defaultManifest));
  } else if (args.includes("--metadata")) {
    await printCurrentMetadata(readOption(args, "--metadata", defaultManifest));
  } else {
    usage();
    process.exitCode = 2;
  }
} catch (error) {
  fail(error.message);
}
