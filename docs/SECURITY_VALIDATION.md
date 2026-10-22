# Security validation boundaries

This document maps the current implementation to its adversarial checks. It is
an evidence index, not a threat-model review or security certification. The
code and tests are authoritative if this summary drifts.

## Local repository input

Repository selection is canonicalized in Rust before a project is recorded.
Snapshot capture reads repository files into immutable, content-addressed
objects. It does not run repository scripts or build tools. Symlinks and
nested repositories are excluded; paths and captured bytes have explicit
limits.

| Boundary | Enforced behavior | Source and evidence |
| --- | --- | --- |
| Selected folder | Resolves the selected directory to its canonical Git root; opening a project does not read source files. | [project.rs](../src-tauri/src/project.rs); project::tests::nested_folder_resolves_root_and_reads_no_repository_bytes |
| Capture paths | Skips symlinks, nested repositories, ignored untracked paths, and policy-excluded generated directories; tracked ignored files remain eligible. Recovery cleanup rejects traversal and symlinked staging paths. | [snapshot.rs](../src-tauri/src/snapshot.rs); snapshot::tests::interrupted_staging_cleanup_rejects_traversal_and_symlinked_paths; snapshot::tests::policy_skips_ignored_generated_symlink_and_nested_repository_but_keeps_tracked_ignored_file |
| Capture size | Defaults to 2 MiB per file, 100 MiB total, 25,000 paths, 1,024 UTF-8 bytes per path, and 1,000 entries per directory. Excluded or over-limit files remain visible as skipped inventory records. | [snapshot.rs](../src-tauri/src/snapshot.rs); snapshot::tests::all_capture_bounds_record_the_path_that_was_not_captured |
| Cancellation and consistency | Checks cancellation while scanning and reading; a cancelled or inconsistent capture does not publish a partial snapshot. | [snapshot.rs](../src-tauri/src/snapshot.rs); snapshot::tests::cancellation_keeps_the_previous_manifest_and_history_entry; snapshot::tests::change_during_chunked_read_is_rejected_without_a_manifest |

## Git subprocess bounds

Repository metadata, snapshot capture and local revision comparison use one
bounded child runner. The Unix source now supervises nonblocking stdin, stdout
and stderr in one loop, keeps the original deadline through child exit and both
output EOFs, and stores no output beyond the command cap. It owns child cleanup
and releases parent pipe handles on errors; there are no pipe worker threads.
Non-Unix hosts fail before launch. Capture passes its cancellation flag into
every Git phase. The runner's focused tests cover bounded streams, timeout,
cancellation, inherited pipes and child cleanup; project, snapshot and
comparison tests cover their callers and preservation behavior. These are local
source/test checks, not packaged-app qualification or proof of behavior on every
Git installation and filesystem. The limits fail the operation with a visible
error; they do not silently truncate evidence.

| Operation | Per-command / total deadline | stdout | stderr | stdin |
| --- | --- | ---: | ---: | ---: |
| Open repository and read branch/HEAD | 5 seconds / 20 seconds | 16 KiB | 16 KiB | none |
| Capture status, ignore rules and tracked-path checks | 15 seconds / 5 minutes for the complete capture | 32 MiB | 64 KiB | 2 MiB |
| Read local commit history | 8 seconds / 10 seconds for the complete history request | 512 KiB | 8 KiB | none |
| Compare local revisions | Remaining time up to 30 seconds for the comparison | Caller-defined evidence cap (from 128 bytes to the bounded relationship-tree read) | 16 KiB | 1 MiB |

Project inspection and snapshot capture treat exit code 1 from their optional
branch/HEAD probes as the expected detached-HEAD or unborn-repository case;
other nonzero exits fail with a bounded error instead of silently erasing
metadata. The comparison view exposes **Stop comparison** and sends a
request-scoped cancellation to the native runner. Snapshot capture is also
cancellable while Git is running. Packaged cancellation, status propagation,
and no-orphan behavior remain unqualified.

## Persisted metadata and source objects

Persisted metadata is bounded before JSON decoding. A size/count overflow or
damaged/unsupported record returns an error and preserves the existing file;
the app does not replace it with an empty catalog or Recents list.

| Data | Read/write bounds | Enforcement |
| --- | --- | --- |
| Recent-project records | 16 MiB and 10,000 entries | `project.rs` uses the shared bounded reader and checks the count before returning or writing. |
| Snapshot catalog | 16 MiB and 20,000 snapshots per project | `snapshot.rs` checks byte size before decode and count before returning or publishing. |
| Snapshot manifest | 128 MiB | `snapshot.rs` checks size before decode and before atomic publication. The limit is above the current capture path and byte ceilings, leaving room for inventory metadata. |
| Existing content-addressed source object | 2 MiB | `snapshot.rs` hashes in fixed-size chunks and stops at the first byte beyond the captured-file limit; mismatches are not overwritten. |

The shared bounded file reader is covered at the exact byte boundary and one
byte above it. `bounded_file::tests`, `project::tests::recent_project_count_overflow_is_explicit_and_preserves_saved_bytes`,
`project::tests::oversized_recent_project_metadata_is_rejected_without_truncating_the_file`,
and the `snapshot::tests::snapshot_catalog_*`, `snapshot::tests::snapshot_manifest_*`,
and `snapshot::tests::recapture_does_not_replace_a_corrupt_object_or_previous_snapshot_history`
cases cover supported limits, malformed/unsupported metadata, identity mismatch,
corrupt objects, and preservation. These local fixtures do not exercise
recovery in a packaged macOS app or data stores from every historical release.

## Static indexing and saved chapters

The indexer parses captured snapshot blobs, not the current checkout. It caps
work at 5,000 files, 1 MiB per source file, 50 MiB of source, 250,000 syntax
nodes per file, 1.5 seconds per file, and bounded symbol, relationship, and
serialized-index counts. Indexing is cancellable; cancellation cannot replace
a previously saved complete index.

Chapter revision JSON is capped at 4 MiB and decoded with duplicate-key
rejection. Rust validates cited paths, hashes, ranges, excerpts, typed visuals,
and app-owned destinations against the selected snapshot. The chapter schema
does not define HTML, scripts, arbitrary URLs, or CSS fields. Before typed
decoding, Rust rejects case-insensitive `html`, `script`, or `javascript` keys
at any depth inside a visual payload. Unsupported non-executable visual shapes
remain unavailable to the renderer. Prose and labels are strings and must be
rendered as text.

Relevant evidence:

- [index.rs](../src-tauri/src/index.rs), test
  index::tests::captured_snapshot_index_is_deterministic_anchored_and_conservative
  includes source that would throw if executed, then checks the repository was
  unchanged after indexing.
- [chapters.rs](../src-tauri/src/chapters.rs), test
  chapters::tests::chapter_decoder_rejects_duplicate_keys_at_every_depth_and_v1_review_keys
  checks duplicate JSON keys and version-specific field rejection. Test
  chapters::tests::chapter_revision_json_enforces_its_four_megabyte_boundary
  accepts a valid revision at the exact byte cap and rejects oversized decode
  and encode inputs.
- [chapters.rs](../src-tauri/src/chapters.rs), test
  chapters::tests::visual_validation_reports_item_and_anchor_for_malformed_typed_inputs
  and chapters::tests::invalid_revision_reports_snapshot_path_range_hash_excerpt_claim_and_destination_codes
  exercise malformed visuals and invalid evidence.
- [chapters.rs](../src-tauri/src/chapters.rs), test
  chapters::tests::chapter_decoder_rejects_html_and_script_fields_in_visual_payloads
  rejects executable-shaped fields at the Rust revision boundary while
  preserving unsupported non-executable visual data as unavailable.

## Provider responses and offline export

The Rust provider reader caps an HTTP response at 1,000,000 bytes and generated
chapter text at 300 KiB. Blocked, incomplete, invalid, or oversized output
does not create a result. The provider request review is one-shot and bound to
the exact reviewed source and request; the API key is not part of the request
body.

[provider.rs](../src-tauri/src/provider.rs), tests
provider::tests::blocked_incomplete_and_oversized_provider_responses_create_no_result
and provider::tests::request_body_contains_the_exact_source_and_no_key_field
cover those cases.

The production Rig transport uses HTTPS-only Reqwest and replaces its internal
credential placeholder only after the endpoint and exact reviewed body match.
Localhost-only tests in [provider_api.rs](../src-tauri/src/provider_api.rs)
send through that transport for Gemini, OpenAI, and Anthropic. They inspect the
raw HTTP request to verify that the reviewed body is unchanged and the key
appears only in the provider's authentication header. A separate pending
request test cancels through the HTTP layer and verifies that the loopback
server observes the connection close. The test-only client is restricted to
127.0.0.1 and disables environment proxies. These fixtures do not verify vendor
eligibility, production TLS/proxy behavior, or a live provider endpoint. A
separate regression test sends an HTTP URL to the production client and checks
that it rejects the request before the loopback listener accepts a connection.

The local PR Guide path separately rebuilds a ready comparison from local Git
objects, restricts the request to up to twelve selected eligible text diffs,
and refuses secret-like paths or excerpts before request review. The user sees
the exact diff, prompt, settings, endpoint, and provider disclosure before
consent. Immediately before transport, the backend rebuilds the comparison and
rejects a moved ref or changed fingerprint. Returned claims are checked against
the exact selected hunk, base/head side, commit/blob anchor, and quoted diff
line. The local store bounds history size, rejects symlinks and malformed
revision chains, and appends accept/reject decisions without touching chapter
history or Git refs.

[pr_guide.rs](../src-tauri/src/pr_guide.rs),
[pr_guide_store.rs](../src-tauri/src/pr_guide_store.rs), and
[provider.rs](../src-tauri/src/provider.rs) contain this boundary. Tests cover
selected-hunk request scope, no transport before consent, ref movement, claim
citation validation, separate immutable history, claim selection, one decision
per draft, and recovery from a well-formed interrupted staging file.

Offline export is generated by the TypeScript UI layer. It has no importer for
a general archive or project package. Its tests in
[offlineExport.test.mjs](../tests/offlineExport.test.mjs) confirm that chapter text and
typed visual labels are escaped, hidden references and common credentials are
omitted or redacted, and unsafe target paths are not exported:
tests/offlineExport.test.mjs.

When the user opts in, export reads narration only through a local, reviewed
cache lookup; it never starts TTS or sends a provider request. Each selected
track must match the section's current narration fingerprint and the supported
24 kHz mono PCM WAV format. The package hashes the decoded audio bytes, embeds
them as `data:audio/wav`, and permits only that media source in its content
security policy. Each track is capped at 12 MiB and the complete HTML package
remains capped at 25 MiB.

## Package-format scope

There is no general-purpose package/archive import endpoint in the current
application. The nearest persisted payload is a chapter-revision JSON record,
which the Rust decoder bounds and validates. These checks do not establish a
contract for importing arbitrary offline HTML, ZIP files, or other packages.

## Reproducing the checks

Run the Rust boundary suite with:

cargo test --manifest-path src-tauri/Cargo.toml --lib

Run the offline-export checks with:

node --experimental-strip-types --test tests/offlineExport.test.mjs

Passing tests show only that the named cases pass for the tested revision and
environment. They do not prove absence of vulnerabilities or replace a
security review.
