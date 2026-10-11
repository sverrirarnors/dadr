---
title: DADR specification
---

# DADR specification

For a short introduction, see [Data Analysis Decision Records](core.md). Suggested tag vocabularies are [documented separately](tags.md).

**DADR** (Data-Analysis Decision Records) is a file-first format for recording consequential decisions in empirical and data-analysis projects. A DADR record is a Markdown file that captures one analytic decision: the context that motivated it, the decision that was made, the alternatives considered, and the consequences for the analysis. Records live alongside the analysis code, manuscript, data references, and project documentation.

This page is the canonical specification of the record format. Directory layouts, inference, version-control hooks, previews, and agent workflows are tooling concerns; the reference implementation documents its own choices in the [**dadrock**](dadrock.md) section.

---

## Version { #version }

This document defines DADR specification version **0.1.0-alpha.2**.

DADR uses Semantic Versioning. Versions below `1.0.0` are alpha-stage specifications: breaking changes MAY appear in minor or patch releases while the model is still being validated. Starting with `1.0.0`, breaking changes MUST increment the major version.

---

## 1. Conventions { #terms }

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative when written in uppercase. Examples are non-normative.

**Record** — one Markdown file with DADR front matter and a body.

**Record set** — the records a tool is pointed at, such as all records in a project. Every rule that relates records to one another (unique identifiers, resolving links, supersession) is evaluated over the record set. This specification does not mandate where record files live.

---

## 2. Records { #records }

A record MUST describe exactly one decision or one proposed decision. Closely related choices SHOULD be split into separate records when they could be accepted, rejected, or superseded independently.

A record MUST be a single UTF-8 Markdown file without a byte-order mark. It consists of YAML [front matter](#front-matter) followed by a Markdown [body](#body).

The Markdown file is the source of truth. Indexes, databases, graphs, reports, and static sites built from records are derived and not canonical. Records MUST remain readable and editable with an ordinary text editor; tools are a convenience, not a requirement.

---

## 3. Front matter { #front-matter }

A record MUST begin with YAML front matter delimited by `---` lines. The front matter MUST be a YAML mapping with these keys:

| Key | Required | Value |
|---|---|---|
| `title` | yes | A non-empty, single-line string: a short noun phrase. 120 characters or fewer is recommended. |
| `status` | yes | `candidate`, `accepted`, or `rejected`. See [Lifecycle](#lifecycle). |
| `id` | yes | A UUIDv7 in lowercase hyphenated form. See [Identifiers](#identifiers). |
| `successors` | no | An array of `id`s of the accepted records that supersede this one. See [Supersession](#supersession). |
| `tags` | no | An array of tags, written without the leading `#`. See [Tags](#tags). |

An omitted array and an empty array (`[]`) are equivalent.

Front matter MUST NOT record dates or times. A record's time is always derived from its `id` ([Identifiers](#identifiers)). In particular, front matter MUST NOT contain a `date` key, nor, under any key, a value written as an ISO 8601 date or date-time, such as `2026-05-07` or `2026-05-07T14:30:00Z`.

Other keys MAY appear, for example `author`, `aliases`, or Pandoc's `bibliography`. They carry no DADR meaning. Tools MUST NOT reject a record because of them unless they break the date rule above, and tools that rewrite front matter MUST preserve them.

DADR expresses classifications such as kind, confidence, provenance, topic, or the people involved as tags rather than as dedicated keys. The [Tags](tags.md) page suggests a vocabulary.

```yaml
---
title: Drop observations with missing treatment assignment
status: candidate
id: 01938b8f-9d71-7456-8000-fedcba987654
tags:
  - kind/sample
  - writer/agent
---
```

---

## 4. Body { #body }

An accepted record MUST contain exactly one `##` heading for each of the four standard sections. Headings match case-insensitively. A section's content runs from its heading to the next `#` or `##` heading.

```markdown
## Context

What problem, dataset, or constraint motivated this decision?

## Decision

What was decided? State the choice in one or two sentences, in the active voice.

## Alternatives

What other options were considered, and why were they set aside?

## Consequences

What follows for results, downstream analyses, robustness checks, or future work?
```

In an accepted record, each standard section MUST contain text other than whitespace and HTML comments. When nothing applies, say so in words; for example, "No alternatives were considered."

The sections MAY appear in any order. Records MAY add other sections, such as `## Robustness`, `## Plan deviation`, or `## Open questions`. A candidate MAY omit or leave empty any section; the requirements above apply once it is accepted.

The four sections follow **Architecture Decision Records (ADRs)**, a common software-engineering practice for capturing the context, choice, and consequences of a design decision in a short Markdown file. DADR applies the same structure to data analysis.

---

## 5. Identifiers and filenames { #identifiers }

A record's `id` is a UUIDv7 ([RFC 9562](https://www.rfc-editor.org/rfc/rfc9562)) written in lowercase canonical form (`8-4-4-4-12`, 36 characters). It is assigned when the record is created and never changes, including when the record is accepted or rejected. The `id` is the record's only stable key: titles, filenames, and locations may change.

UUIDv7 needs no shared counter, so concurrent branches and agents can create records safely. Its first 48 bits are a Unix-millisecond timestamp, so identifiers sort by creation time. That timestamp is the record's only date: records never store another one ([§3](#front-matter)). When a record was accepted, rejected, or superseded, and by whom, belongs to version control or tool logs, not to the record.

The **short id** is the first 12 hexadecimal digits of the `id`, with no hyphen:

```text
id:        01938b8e-4d70-7abc-8def-0123456789ab
short id:  01938b8e4d70
```

Short ids MUST be unique within a record set. Tools that create records MUST ensure this, for example by generating a fresh `id` when the short id is already taken.

A record's filename SHOULD be its short id, a hyphen, a short kebab-case slug of its title, and `.md`:

```text
01938b8e4d70-income-preprocessing.md
```

Authors drafting by hand can take an `id` from the [UUID v7 generator](uuidv7.md). A file without an `id` is not a valid record; tools MAY repair it by assigning a fresh `id`.

---

## 6. Lifecycle { #lifecycle }

### 6.1 Status { #status }

| Status | Meaning |
|---|---|
| `candidate` | A proposed decision under review. |
| `accepted` | A human-approved decision, whether or not it has since been superseded. |
| `rejected` | A candidate closed without approval: rejected, withdrawn, duplicated, merged into another record, or no longer relevant. |

The only transitions are `candidate → accepted` and `candidate → rejected`. Each changes the `status` of the same record and keeps its `id`. `accepted` and `rejected` are final. To revisit a rejected or superseded decision, create a new candidate.

A record's lifecycle stage is given by its `status` alone. Tools MUST NOT infer it from the file's name or location.

Acceptance is a human decision. Tools and agents MUST NOT accept a candidate without human approval, and a candidate MUST meet the [body requirements](#body) to be accepted.

### 6.2 Supersession { #supersession }

To supersede an accepted decision, accept a new record and add its `id` to the old record's `successors`. An accepted record with a non-empty `successors` array is **superseded**. This is derived from the record set; it is not a status, and the old record's `status` stays `accepted`.

```yaml
successors:
  - 01938c20-a3b4-7000-8000-111122223333
```

Within a record set:

- only accepted records MAY have a non-empty `successors` array;
- each entry MUST be the `id` of an accepted record other than the record itself;
- the graph formed by `successors` MUST be acyclic.

`successors` is an array, so one decision can be split into several successors, and several decisions can share one successor. Only the forward edge is stored; tools derive predecessors by scanning the record set. Supersession exists only in `successors`: a mention in prose or a wikilink does not supersede a record.

### 6.3 Immutability { #immutability }

Candidates MAY be edited freely. Once a record is accepted or rejected:

- its `id`, `status`, `title`, and body MUST NOT change;
- entries MUST NOT be removed from `successors`.

`tags` and keys outside DADR MAY change, and an accepted record gains `successors` entries when it is superseded. To change a decision, however slightly, create a new candidate, accept it, and supersede the old record.

These rules can be checked mechanically, for example against version-control history.

---

## 7. Tags { #tags }

DADR uses the Obsidian tag syntax:

- in body text, a tag begins with `#`; in the front-matter `tags` array, it is written without `#`;
- a tag consists of Unicode letters, numbers, emoji, `_`, `-`, and `/`, and contains no whitespace;
- `/` separates levels of a hierarchy, so `kind/sample` is a child of `kind`; empty levels (`a//b`, `a/`, `/a`) are invalid;
- a tag MUST contain at least one character other than a digit, `_`, `-`, or `/`, so `#1984` is invalid and `#y1984` is valid;
- tags are case-insensitive: `#Kind` and `#kind` are the same tag.

DADR defines no required tag vocabulary. Projects choose their own; the [Tags](tags.md) page suggests one.

Front-matter tags classify the whole record. Inline tags in the body attach to the passage they appear in; a common pattern is the first line of a blockquote. Tools SHOULD treat both as one tag set when indexing or reporting.

```markdown
> #confidence/low
> The pre-analysis plan called for a complete-case rule. We now prefer multiple
> imputation, pending a check of missingness by site.
```

### 7.1 Code tags { #code-tags }

In source-code comments, DADR tags carry a `#dadr/` prefix that separates them from other comment text and other tag systems:

```r
# #dadr/01938b8e4d70
df <- df[!is.na(df$treatment), ]
```

```python
# #dadr/kind/measurement #dadr/topic/student-survey
survey["anx_mean"] = survey[["anx1", "anx2", "anx3"]].mean(axis=1)
```

- `#dadr/` followed by a short id or full `id` links the nearby code to that record. A link that does not resolve to a record in the record set is broken, and tools that check code tags SHOULD report it.
- `#dadr/` followed by any other tag annotates the nearby code with that tag; for example, `#dadr/kind/measurement` carries the tag `kind/measurement`.

Which lines of code a tag covers is defined by the tools that read it. Tags inside record files do not use the `#dadr/` prefix.

---

## 8. Links and citations { #links-and-citations }

### 8.1 Links between records { #wikilinks }

A wikilink whose target is a short id or a full `id` is a **record link**:

```markdown
See [[01938c20a3b4]] for the sampling rule.
Compare with [[01938b8e4d70|the original preprocessing decision]].
```

The short id form is recommended. Hexadecimal digits match case-insensitively. A record link MUST resolve to a record in the record set, and tools MUST report links that do not resolve rather than guess a target. The form `[[target|display text]]` sets the displayed text.

Wikilinks with any other target are not DADR links, and tools MUST leave them unchanged. Tools MAY convert record links to ordinary Markdown links when rendering, for renderers that do not support wikilinks.

### 8.2 Other links { #markdown-links }

References to code, data, notebooks, manuscripts, plans, outputs, registries, and web pages SHOULD be standard Markdown links or autolinks, which every renderer supports. Relative paths resolve from the directory that contains the record, as in any Markdown renderer.

```markdown
The exclusion is enforced in
[`analysis/build_sample.py` lines 42–58](../../analysis/build_sample.py#L42-L58),
following the pre-registration at <https://osf.io/abcde>.
```

### 8.3 Citations { #citations }

Records MAY cite sources with Pandoc citation syntax, such as `[@rubin1976]`. Resolving citation keys is up to the renderer. A Pandoc `bibliography` key in front matter is treated like any other key outside DADR.

---

## 9. Conformance { #conformance }

A **record** conforms if:

1. it is a UTF-8 Markdown file without a byte-order mark, beginning with YAML front matter that is a mapping;
2. `title`, `status`, and `id` are present and valid ([§3](#front-matter), [§5](#identifiers));
3. the front matter contains no `date` key and no ISO 8601 date or date-time value ([§3](#front-matter));
4. `successors`, if present, is an array of UUIDv7 `id`s, and is empty unless `status` is `accepted`;
5. `tags`, if present, is an array of valid tags ([§7](#tags));
6. if `status` is `accepted`, the body meets [§4](#body).

A **record set** conforms if every record in it conforms and:

1. no two records share an `id` or a short id;
2. every `successors` entry names another accepted record, and the `successors` graph is acyclic ([§6.2](#supersession));
3. every record link resolves ([§8.1](#wikilinks)).

A **tool** conforms if it:

1. identifies records by `id` and lifecycle stage by `status`, never by filename or location;
2. reports violations of these rules instead of silently guessing or repairing them;
3. preserves front-matter keys it does not use when it rewrites a record;
4. creates records with unique short ids;
5. never accepts a candidate without human approval, and changes accepted records only as [§6.3](#immutability) allows.

A filename that does not follow [§5](#identifiers) does not make a record non-conforming.

---

## 10. Example record { #example }

This example is non-normative.

```markdown
---
title: Drop observations with missing treatment assignment
status: accepted
id: 01938c20-a3b4-7000-8000-111122223333
tags:
  - kind/sample
---

# Drop observations with missing treatment assignment

## Context

The treatment variable is required for the primary estimand. Observations with
missing treatment assignment cannot contribute to the main comparison without
introducing an additional imputation model. See [[01938b8e4d70]] for how the
analysis sample was first defined.

## Decision

Drop observations with missing treatment assignment from the main analysis
sample.

## Alternatives

One alternative was to impute treatment assignment. This was not chosen because
the assignment mechanism is part of the study design rather than a downstream
measurement, and imputing it would make the estimand harder to interpret.

Another alternative was to keep the observations for descriptive tables only.
This can still be done in a supplemental descriptive analysis, but those rows
are excluded from the primary model.

## Consequences

The main sample size decreases. The manuscript should report the number of
excluded observations, and robustness checks should verify that treatment
missingness is not concentrated in one site.
```

---

## Reference implementation { #reference-implementation }

The reference implementation is **dadrock**, a Rust tool with a CLI, a web UI, and IDE integrations. See the [**dadrock**](dadrock.md) section for its project layout, commands, and validation, and the [Tags](tags.md) page for the recommended tag vocabulary.
