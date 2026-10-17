# Chapter quality benchmark and release gate

Code Atlas validation proves that a chapter has the expected shape and that its citations point into the selected snapshot. It cannot prove that the explanation says what the cited code actually does. This benchmark adds a separate human review gate for that semantic question.

This gate currently covers the synthetic **file-chapter proposal** workflow. It does not qualify repository overviews, TTS output, provider/agent backends, accessibility, native packaging, or a release. A passing report is one piece of release evidence, not release approval by itself.

## Corpus

The local synthetic corpus in [`benchmarks/chapter-quality/corpus/`](../benchmarks/chapter-quality/corpus/) covers a route/entrypoint, service/core module, data model, test, configuration, and an unsupported language across at least two small repositories. Each case records the file's role, applicable questions, expected source-grounded facts, and exact source excerpts. The fixtures contain no customer or repository code.

Run the structural corpus check with:

```sh
npm run quality:corpus
```

The check is deterministic and local. It enforces all six role families and at least two distinct synthetic repositories, checks that fixture parser-dispatch tags match the current Atlas parser-language set, checks canonical fixture paths and hashes, and requires each expected quote to occupy its exact declared line and UTF-8 byte range. The parser-dispatch check does not qualify language semantics or generated explanation quality; see the [language capability matrix](LANGUAGE_CAPABILITIES.md). It never contacts a model.

Build the offline native helper, then get the current corpus hash, prompt/schema fingerprints and backend profile:

```sh
npm run quality:build
```

The helper is a separate `code-atlas-quality` executable, so a missing or old
helper cannot fall back to opening the desktop UI. The Node adapter compares its
compiled build identity with the current native source tree and refuses stale
helpers. Rebuild after changing native code or locked dependencies.

The default profile is the existing Codex route, GPT-6 Luna/max with approval
`never`. Metadata generation does not connect to an account, require an API key
or send source to a model. Its fixed public source and synthetic runtime identity
fingerprint the local request contract; they are not an account or model run.

Get metadata with:

```sh
npm run quality:metadata
```

Direct API contract metadata is also available with an explicit backend and a
model from the app's current catalog, for example `--backend gemini --model
<catalog-model-id>` appended after `npm run quality:metadata --`. Direct API
profiles record null agent settings. This prepares metadata only; it does not
qualify provider availability or authorize generation.

Start a local report from the pending template. Save the raw `GeneratedChapterProposal` JSON returned for each case and every repair attempt in this ignored directory too; do not wrap it in provider metadata or a saved chapter revision. The gate verifies each digest and sends current candidates to the native request schema, chapter canonicalizer, fresh narration preparation and complete chapter validator. That includes version-2 compositions, tree/value/evidence rules, unique source quotes, identifiers and target-file coverage. Selected corpus bytes are hash-checked and staged in private temporary storage, then removed. Node resolves review claim IDs against stable paths such as `sections[0].claims[0]` and `evidenceLimits[0]`.

```sh
mkdir -p benchmarks/chapter-quality/review/reports
cp benchmarks/chapter-quality/review/REPORT_TEMPLATE.json benchmarks/chapter-quality/review/reports/run.json
```

## Human review

Use [`benchmarks/chapter-quality/review/RUBRIC.md`](../benchmarks/chapter-quality/review/RUBRIC.md) for the binary, source-linked criteria and [`REPORT_TEMPLATE.json`](../benchmarks/chapter-quality/review/REPORT_TEMPLATE.json) as the local batch format. Review each final chapter against the case's `mustCover` list and the exact cited source. For every criterion, record pass/fail or not applicable with a reason. Use not applicable only when the manifest explicitly says `notApplicable`; `notProvided` and `notAssessable` require a bounded explanation, not a waiver. A citation hash or a model's own self-review is not a semantic proof.

Report schema v5 records `backendId`, `modelId`, returned model version,
`reasoningEffort` and `approvalPolicy` at batch and attempt level. Codex currently
requires GPT-6 Luna/max and `never`; direct APIs require null agent settings.
It also records prompt version, exact compiled prompt/config/schema hashes,
chapter schema version, response schema version 2, backend model-list hash,
corpus-manifest hash and each attempt's request-review fingerprint.

The metadata uses the actual native file-chapter review builder on a fixed
public fixture. It includes the appended visual/meaning-first instructions and
actual structured response schema, rather than extracting prompt fragments with
regular expressions. The request-assembly hash covers the native input set hashed by `build.rs`
plus both Node checker files: native sources, shared policy,
canonicalizers, scene/source validators, build configuration, resources,
capabilities and the locked dependency graph. The sole excluded native input is
the Claude qualification receipt, whose own receipt must not invalidate itself.
Any other relevant edit makes earlier report metadata stale. This is local
change detection, not executable or account attestation.

Saved review fingerprints and backend/settings fields are still reviewer-supplied
provenance. The checker does not yet bind them to a captured real request-review
artifact or prove that a model executed. Indexed relationship IDs also require a
saved native review/index binding; arbitrary manifest IDs are refused. These
remaining provenance checks and the complete generated/human-reviewed batch keep
[Kata t14](kata-tasks/14-chapter-quality.md) open.

The report also records reviewer, review date, outcomes, failure categories, findings, and repairs. Every attempt records its complete rubric and a `factCoverage` map from each `mustCover` fact to positional section-claim IDs in that attempt's artifact. The gate rejects evidence-limit IDs and requires each mapped claim to have an exact, unique evidence quote on the expected path whose UTF-8 byte span contains the fact's exact manifest anchor. Failed attempts may leave a fact's list empty to record a missed fact; passing attempts require a mapped candidate for every fact. This span check only proves that the claim cites the expected source bytes: the human reviewer still decides whether the claim accurately explains them. Each repaired attempt must have its own artifact path, content digest, and repair summary. The final case result, request fingerprint, fact coverage, rubric, findings, categories, and schema status must match the last attempt. Source-reference line ranges must match the exact unique quote; UTF-8 byte offsets are derived from that quote. Completed reports and chapter artifacts live under the ignored local `benchmarks/chapter-quality/review/reports/` directory; do not include API keys or copied repository source. A report tied to old inputs is stale.

The rubric deliberately has no minimum word count. A small file can pass with a short explanation when the applicable behavior is specific and fully cited. Generic praise, restating a filename, missing applicable inputs/outputs or state/error behavior, unsupported execution claims, and vague citations fail even when the chapter is schema-valid.

## Release check

After generating a benchmark batch through the product's exact-context request review and consent flow, complete the report and run:

```sh
npm run quality:gate -- --report benchmarks/chapter-quality/review/reports/run.json
```

The gate compares the report with the current corpus manifest, compiled prompt version/text, complete native request assembly, generation settings, chapter schema version, response schema version/hash and the selected backend model list. Bump the response schema version when its provider-facing contract changes; the compiled schema hash also detects contract changes. It rejects a pending report, missing or duplicate cases, stale metadata, unresolved failed cases, missing binary findings, invalid source references, changed artifact digests, a chapter proposal that fails local shape/source validation while marked valid, inconsistent repair history, or a changed shallow-output fixture. The release gate is separate from ordinary `npm run check` because it requires a completed human-reviewed batch.

Run and review a fresh batch before every release and after any prompt, response schema, chapter schema, or model-allowlist change. The command verifies that the review is complete and current; a human remains responsible for whether the prose truly matches the cited source.

## Evidence boundaries

The unchanged `fixture/rubric-example` uses the historical pre-visual contract
solely to demonstrate that source-shaped prose can still fail human teaching
review. Its private legacy JavaScript validator is not used for current model
output. A passing corpus check, replayed Fontdone artifact or compiled helper is
not a passing human-reviewed batch, listening test or comprehension study.
