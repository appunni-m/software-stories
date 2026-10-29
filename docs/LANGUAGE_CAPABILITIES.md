# Language capabilities

**Status reviewed: 2026-10-27.** A parser being installed does not mean the language's behavior or generated explanations are qualified.

## Tiers

| Tier | What the app can claim | What it cannot claim |
| --- | --- | --- |
| **Partial structural** | A bundled syntax parser may produce declarations and conservative import, call, or test-reference candidates. Resolved relationships retain exact source anchors; ambiguous and dynamic relationships are withheld or labeled unresolved. | Complete language semantics, framework entry points, runtime call graphs, every import/call, or language-specific narrative quality. |
| **Text only** | Eligible captured text remains locally readable and can support exact source citations in a reviewed explanation. | Local declarations, imports, calls, or test-reference relationships. |
| **Not analyzed** | The inventory retains the path, capture disposition, and reason when available. | Structural evidence or source citations when suitable text bytes were not captured. |
| **Qualified structural** | No language currently has this tier. | Do not interpret parser availability or chapter-schema validity as qualification. |

## Current operation matrix

| Language / files | Capture and exact citations | Local structure | Framework and mixed-language behavior | Generated explanation quality |
| --- | --- | --- | --- | --- |
| Rust (`.rs`) | Available for eligible captured text | **Partial structural**: the Rust grammar is bundled; indexed relationships are conservative and source-linked. | Not qualified. | Not qualified by a Rust-specific human review. |
| TypeScript (`.ts`, `.mts`, `.cts`) | Available for eligible captured text | **Partial structural**: TypeScript grammar is bundled; indexed relationships are conservative and source-linked. | Not qualified. | Not qualified by a TypeScript-specific human review. |
| TSX (`.tsx`) | Available for eligible captured text | **Partial structural**: TSX grammar is bundled; indexed relationships are conservative and source-linked. | Not qualified. | Not qualified by a TSX-specific human review. |
| JavaScript (`.js`, `.jsx`, `.mjs`, `.cjs`) | Available for eligible captured text | **Partial structural**: JavaScript grammar is bundled; indexed relationships are conservative and source-linked. | Not qualified. | Not qualified by a JavaScript-specific human review. |
| Python (`.py`, `.pyi`, `.pyw`) | Available for eligible captured text | **Partial structural**: the Python grammar indexes functions, classes, methods, same-file direct calls, and conservative import/call candidates. A named callable import can resolve when an explicit relative import targets a fully parsed captured module in a proven regular package and exactly one direct module-level function matches. Absolute imports and namespace-package paths remain unresolved. | Not qualified; absolute module resolution, namespace-package resolution, and framework conventions are not implemented. | Not qualified by a Python-specific human review. |
| Go (`.go`) | Available for eligible captured text | **Partial structural**: the Go grammar indexes functions, methods, structs, interfaces, same-file direct calls, and unambiguous bare-function calls across captured files in the same directory with the same parsed package clause. Imports and calls through imported packages remain unresolved. | Not qualified; module/import resolution and framework conventions are not implemented. | Not qualified by a Go-specific human review. |
| Other captured text | Available for eligible captured text | **Text only**: no local structural parser is shipped for these files. | Not qualified. | The provider may explain selected text with exact local citations, but language-specific explanation quality is not qualified. |

The current parser versions are Tree-sitter runtime `0.27.1`, Rust grammar `0.24.2`, TypeScript/TSX grammar `0.23.2`, JavaScript grammar `0.25.0`, and Python/Go grammars `0.25.0`, as pinned in `src-tauri/Cargo.lock`. The index records the runtime and grammar versions/ABIs so parser-produced evidence is tied to its implementation.

The local index recognizes a bounded syntax subset. It attempts declarations and selected import/call/test-reference relationships for its six parser languages. Python explicit relative imports resolve only when every package level needed by the import has a captured `__init__.py` and exactly one `.py` module or package initializer matches. For a named imported call, the imported module must also be fully parsed and contain exactly one direct module-level function with the imported name. Absolute imports and namespace-package paths remain unresolved. Go bare-function calls may resolve across captured `.go` files only when the files share a directory and the same parsed package clause; Go module/import resolution and calls through imported packages remain unresolved. A high-confidence edge means only that the local static resolver found a supported, source-backed relationship. It does not prove the code executes. Dynamic dispatch, reflection, macros, framework routing, generated registrations, and cross-language protocols may not resolve. Ambiguous cases retain an unresolved reason instead of being presented as proven.

## UI wording

- **Partial structural** means a local syntax parser is available, while relationship and narrative coverage remain incomplete.
- **Text only** means captured text and citations are available, but local symbols and relationships are not produced.
- **Not analyzed** means the entry has no suitable captured text for analysis, such as a binary file or a path excluded from capture. Captured text without a bundled parser remains **Text only**, including text with an unrecognized extension.
- **Qualified structural** is not currently advertised for any language.

The current source maps parser-backed text to **Partial structural**, captured text without a supported parser to **Text only**, and non-text or uncaptured entries to **Not analyzed**. Capture disposition and its reason remain separate from the capability label. The packaged workspace was reopened on 2026-10-24, but the language-capability labels themselves were not inspected in that packaged run; this is not native UI acceptance. See [Getting started](GETTING_STARTED.md) for the source-review workflow and [Troubleshooting](TROUBLESHOOTING.md) for capture/index limitations.

The source editor also lazy-loads display-only syntax highlighting for additional captured languages, including C#, Kotlin, Scala, Ruby, R, Lua, Haskell, OCaml, F#, Clojure, Fortran, assembly, Dart, Swift, Erlang, Julia, Perl, PowerShell, COBOL, Elm, Groovy, CoffeeScript, Verilog, VHDL, D, Protocol Buffers, Java properties, INI, unified diffs, and Nginx configuration. Markdown-family files, SCSS, shell variants, and Gradle scripts use their closest available display mode. This is separate from the structural parser matrix above: highlighting does not add symbols, relationships, runtime behavior, or language-specific explanation qualification. Files without a matching display mode remain readable as plain text.

## Qualification still required

The parser tests cover selected syntax and resolution cases, including Python/Go structural fixtures and the lexical-shadowing matrix recorded in [the parser task](kata-tasks/31-parser-binding-resolution.md). They do not qualify every language feature, framework convention, mixed-language boundary, or generated narrative. Promotion to **qualified structural** requires source-linked fixtures and human-reviewed file explanations and behavior journeys for that language, with unresolved paths and gaps recorded.
