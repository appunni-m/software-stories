# Chapter quality corpus v1

This corpus is a small, checked-in set of invented repository files for reviewing file chapters. It is designed to test source-grounded explanation quality across six roles: route/entrypoint, service/core, data model, test, config, and unsupported language. It is not a set of golden prose answers. Reviewers compare chapter claims with the expected facts and their exact source anchors.

All fixture repository names and code are synthetic. A corpus runner must be offline and must never call a model or upload source. Evaluate a real repository only through Code Atlas' ordinary exact-context preview and explicit consent flow; do not add real repository content to this checked-in corpus.

## Manifest format

[`manifest.json`](manifest.json) is the source of truth. Each case identifies a logical `repository`, repository-relative target `path`, role and language, parser status, target `sourceFile`, optional `supportingFiles`, and `applicability` for each review dimension. When present, `supportingFiles` must exactly match the support-role entries in `files`. The `files` array lists the exact target and support inputs with their corpus-relative file path, byte size, and SHA-256.

Each `expectedFacts` entry gives a canonical ID, a supported review dimension, and a concrete statement of a fact that a reviewer may expect the chapter to explain. The dimension must exist in the case applicability map and cannot be marked `notApplicable`. Its `quote` must match the source exactly and uniquely. `lineStart` and `lineEnd` are one-based inclusive lines and must be the exact lines occupied by the quote; `byteStart` and `byteEndExclusive` are zero-based UTF-8 byte offsets and must be nonnegative, in-bounds, and exact; `sourceSha256` hashes the complete source file. Logical file paths must be unique and the expected fact's path must identify the same source file as its quote. The manifest was prepared from these fixture bytes. `mustCover` lists distinct facts required for a passing case; expected facts do not dictate wording.

For report coverage, each mapped positional section claim must cite an exact, unique quote on the expected file whose byte span contains the expected fact's manifest span. A same-file quote elsewhere does not satisfy the mapping. This mechanical match establishes byte-level linkage only; a human reviewer still judges whether the claim explains the quoted behavior.

The runner enforces all six listed role families and a floor of two distinct synthetic repository identifiers. It reads the current parser-language set from `src/lib/inventory.ts` and checks each case's `parserStatus` against that set; the required unsupported-language case must use a language the current indexer does not support. A manifest may add role or repository requirements, but cannot lower those floors.

Applicability status meanings:

- `required`: review this dimension and cover the listed facts.
- `notApplicable`: the fixture does not contain this kind of behavior; do not invent it.
- `notProvided`: supporting source is absent; do not claim facts that require it.
- `sourceOnly`: report only directly visible syntax or values.
- `notAssessable`: the relevant semantics cannot be established from this corpus.

The global `reviewContract` makes the case gate binary. Exact citation matching is only a mechanical precondition: the reviewer must verify that the cited range supports the full claim. A case fails if a required fact is missing or any behavioral assertion is unsupported or contradicted. Short explanations can pass when they cover the applicable facts.

`knownFailureExamples` are intentionally plausible, schema-shaped claims that must fail human review. They demonstrate why valid JSON, exact quote matching, and identifier overlap cannot establish semantic support.
