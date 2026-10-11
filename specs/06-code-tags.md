# Code Tags

## Status

Future RFC. The code-tag syntax (`#dadr/<short-id>` links and `#dadr/<tag>`
annotations) is documented in
[`docs-src/specification.md`](../docs-src/specification.md#code-tags), and the
suggested namespaces in [`docs-src/tags.md`](../docs-src/tags.md#code-tags).
The public spec deliberately leaves the covered code range to tools. This spec
defines that range, the `#dadr/ignore` directive, and future extraction and
code-tag-driven inference or faithfulness checks.

## Purpose

Define future tooling behavior for source-code comments that use the code-tag
syntax documented in the public specification. Code tags should help tools link
source ranges to records, seed candidate drafts, and support faithfulness
checks.

Code tags are an authoring aid and retrieval mechanism. They do not replace accepted DADR records.

## Future Extraction Semantics

The extractor should:

- scan source comments for `#dadr/...` tags;
- classify `#dadr/<short-id>` and `#dadr/<full-id>` tags as record references;
- treat classification tags such as `#dadr/kind/...`, `#dadr/topic/...`,
  `#dadr/plan/...`, `#dadr/multiverse/...`, and `#dadr/confidence/...` as
  evidence on the surrounding source range;
- treat `#dadr/ignore` (the tag `ignore`) as a Dadrock-specific signal that
  suppresses candidate generation for the bound source range;
- preserve raw tag text, normalized tag text, file path, line number, and bound
  source range in the extraction result;
- report record references that do not resolve rather than guessing.

## Source-Range Binding

A tag applies to the next logical statement or nearby code block. Parsers may
use language-aware structure (AST, notebook cell boundaries, SQL statements,
workflow nodes) when available. When no language-aware structure exists, the
fallback binds a tag to the next non-blank, non-comment line after the tag
comment; if the tag shares a line with executable code, it binds to that line.
For longer blocks, authors tag the first line and the parser uses surrounding
structure to infer the range when it can.

## How Tooling Uses Code Tags

- **Code-derived drafting improvements** may treat `#dadr/<uuid>` as a source
  reference linking the surrounding code to that record, harvest classification
  tags onto generated candidates, and skip ranges marked `#dadr/ignore`.
- **Faithfulness — Code** ([13-faithfulness-code.md](13-faithfulness-code.md))
  may use `#dadr/<uuid>` to find the code that backs a record's claim and flag
  records whose linked code no longer matches.
- **Link validation** may eventually resolve `#dadr/<uuid>` code-comment
  references against records.
- **Tag hygiene** may eventually normalize near-duplicate `#dadr/...` tags
  across code and records.

## Failure Modes

- stale tags that no longer describe the code;
- duplicate tags for the same decision;
- comments that expose sensitive data, identifiers, or rationale that should not be sent to a remote model;
- relying on tags as a substitute for accepted DADRs;
- detector overreach that treats every classification tag as a new decision rather than as a signal on an existing one.

The review UI should show the tag text, the annotated code range, and any candidate derived from the tag.
