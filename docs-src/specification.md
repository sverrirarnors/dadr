---
title: DADR specification
---

# DADR specification

For a short introduction, see [the format in one page](core.md). This page is the full normative reference for format implementers. Suggested tag vocabularies are [documented separately](tags.md).

**DADR** (Data-Analysis Decision Records) defines a file-first format for recording consequential decisions in empirical and data-analysis projects. A DADR record is a Markdown file that captures one analytic decision: the context that motivated it, the decision that was made, alternatives considered, and consequences for the analysis. Records live alongside the analysis code, manuscript, data references, and project documentation.

This document is the canonical specification. It defines:

- the current specification version and stability policy;
- the record data model;
- required and optional front-matter fields;
- required body sections for accepted decisions and conventions for additional sections;
- tag syntax;
- inter-record links, source-reference links, and citation syntax;
- identifier and filename rules;
- project layout expectations;
- the status vocabulary, lifecycle transitions, and immutability rules;
- conformance requirements for records, projects, and tools.

Tooling, inference, version-control hooks, preview sites, reports, and agent workflows are outside this normative core unless explicitly stated.

---

## Version { #version }

This document defines DADR specification version **0.1.0-alpha.1**.

DADR uses Semantic Versioning. Versions below `1.0.0` are alpha-stage specifications: breaking changes MAY appear in minor or patch releases while the model is still being validated. Starting with `1.0.0`, breaking changes MUST increment the major version.

---

## 1. Terms { #terms }

The key words **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are to be interpreted as normative requirements when written in uppercase. Lowercase words such as "must" and "should" are used in their ordinary English sense and are not normative. Examples are non-normative unless explicitly labelled as requirements.

**DADR** — Data-Analysis Decision Records, the standard defined here.

**Record** — a single Markdown file with DADR front matter and body content.

**Record set** — the Markdown record files that make up a DADR project. The DADR specification does not mandate where these files live on disk.

**Candidate** — a proposed decision captured in a record file. A candidate has `status: candidate` while under review. It becomes an accepted decision by conversion of the same record, or becomes a rejected record when closed without approval.

**Accepted decision** — a human-approved decision record. An accepted decision normally begins as a candidate and keeps the same UUIDv7 when accepted.

**Superseded decision** — an accepted decision whose `successors` array points to a later accepted decision. "Superseded" is a derived predicate on the record graph, not a `status` value.

**Rejected record** — a candidate closed without becoming an accepted decision.

**Predecessor** — an accepted record whose `successors` array points to another accepted record.

**Successor** — an accepted record listed in another accepted record's `successors` array.

**DADR project** — a collection of DADR records within a project directory.

---

## 2. One file per decision { #one-file-per-decision }

A DADR record MUST describe exactly one decision or one candidate decision, and MUST be stored as a single Markdown file. Closely related choices SHOULD be split into separately reviewable records when they can be accepted, rejected, or superseded independently.

The durable source of truth for a record is the Markdown file. Derived indexes, databases, graphs, reports, badges, and static sites are not canonical. Records MUST be stored as readable, editable Markdown. Specialized DADR tooling MUST NOT be required to inspect or modify record text, though tools MAY be used to allocate identifiers, normalize manually drafted records, maintain canonical filenames, and validate lifecycle relationships.

Record files MUST be UTF-8 encoded and MUST NOT include a byte-order mark (BOM). Tools MUST treat invalid UTF-8 as non-conformant input. Tools MAY accept and remove a UTF-8 BOM during import or normalization.

Version control is recommended but not required by the file format.

---

## 3. Preamble { #preamble }

A DADR record MUST begin with a YAML preamble delimited by `---`. The preamble carries the record's required title and lifecycle status, the conformant record's stable `id`, and any optional structural metadata.

### 3.1 Required entries { #required-entries }

| Field | Type | Requirement |
|---|---|---|
| `title` | string | MUST be a short human-readable title (a noun phrase). It MUST be non-empty after trimming, MUST be a single line, and MUST NOT have leading or trailing whitespace. Titles SHOULD be brief enough for filenames, lists, and tables; 120 characters is the recommended maximum. |
| `status` | enum | MUST be one of `candidate`, `accepted`, or `rejected`. |
| `id` | string | MUST be a stable UUIDv7 (RFC 9562) on conformant records, written in lowercase hyphenated canonical text form (`8-4-4-4-12`, exactly 36 characters). A manually drafted record MAY temporarily omit `id`; tools allocate one during normalization (see [Manual records](#manual-records)). A candidate converted into an accepted decision keeps the same `id`. |

Authors rarely type the `id` by hand. Tooling allocates a fresh UUIDv7 whenever a record is created by a tool or fully normalized from a manual draft.

### 3.2 Optional entries { #optional-entries }

| Field | Type | Requirement |
|---|---|---|
| `successors` | array of UUIDv7 strings | MAY list accepted decisions that supersede this accepted decision. Candidate and rejected records MUST NOT have non-empty `successors`. Missing `successors` and `successors: []` are semantically equivalent. See [Supersession](#supersession). |
| `tags` | array of strings | MAY classify the whole record. See [Tags](#tags). |
| `bibliography` | string or array | MAY override the project bibliography for this record. See [Citations](#citations). |

Fields that classify a record by **kind**, **confidence**, **person**,
**robustness role**, **origin**, **topic**, or **workflow state** SHOULD be
expressed as tags rather than as dedicated front-matter entries. Decision
ownership and review attribution SHOULD use `#people/decider/<handle>` and
`#people/approver/<handle>` tags rather than dedicated `deciders` or `reviewers`
fields.

Lifecycle timestamp fields such as `date`, `created`, `accepted_at`, or `superseded_at` MUST NOT be used. DADR derives record-creation timestamps from UUIDv7 identifiers (see [Identifiers](#identifiers)).

For optional entries, an empty `tags: []` value is equivalent to omitting `tags`. An empty `bibliography: ""` string and an empty `bibliography: []` array are equivalent to omitting `bibliography`; they do not suppress a project-level bibliography. Empty `successors: []` is defined above and is equivalent to omitting `successors`.

### 3.3 Examples { #preamble-examples }

A minimal preamble for an accepted record (the body must still contain the four standard sections; see [Body](#body)):

```yaml
---
title: Drop observations with missing treatment assignment
status: accepted
id: 01938c20-a3b4-7000-8000-111122223333
---
```

A candidate record with optional structural entries:

```yaml
---
title: Drop observations with missing treatment assignment
status: candidate
id: 01938b8f-9d71-7456-8000-fedcba987654
tags:
  - kind/sample
  - writer/agent
  - confidence/high
---
```

---

## 4. Body { #body }
### 4.1 Required sections { #required-sections }
An accepted DADR record MUST contain the four standard sections as level-2 Markdown headings (`## Context`, `## Decision`, `## Alternatives`, and `## Consequences`). Heading matching is case-insensitive. Each standard section MUST appear exactly once. The sections MAY appear in any order, and optional sections MAY be interleaved with them, but conventionally they appear as below:

```markdown
## Context

What problem, dataset, or constraint motivated this decision? Briefly describe
the analytical situation and any relevant prior choices.

## Decision

What was decided? State the choice in one or two sentences, in the active voice.

## Alternatives

What other options were considered, and why were they not chosen? List the
defensible alternatives and the reason each was set aside.

## Consequences

What follows from this decision? Note expected effects on results, downstream
analyses, robustness obligations, or future work.
```

The body template is modelled on **Architecture Decision Records (ADRs)**, an extremely common practice in software engineering for capturing the context, choice, and consequences of a design decision in a short, reviewable Markdown file. DADR adopts the same four sections and applies them to data analysis instead of architecture.

The four standard sections MAY be empty, but empty sections are discouraged because they reduce the value of the record as a decision artifact. If no genuine alternatives are known or applicable, the `Alternatives` section SHOULD say so explicitly (for example, "Not applicable" or "Unknown") rather than leaving it empty.

Candidate records MAY contain any subset of the four standard sections. The four-section requirement applies only to accepted records.

### 4.2 Optional sections { #optional-sections }
A record MAY contain additional body sections. Their contents are unconstrained: authors are free to add whatever helps the analysis. Examples include:

```markdown
## Robustness

## Notes

## Plan deviation

## Open questions
```

Additional sections are project-specific unless another DADR specification profile defines them. Tooling MAY support project-specific templates so common extra sections can be scaffolded into new candidates automatically.

---

## 5. Tags { #tags }
DADR records MAY use tags for classification, retrieval, reporting, and inline annotation.

### 5.1 Format rules { #format-rules }
DADR adopts the **Obsidian tag format** with no extensions:

- a tag begins with `#` when used inline in Markdown body text;
- a tag in the YAML `tags` list MUST omit the leading `#`;
- tag characters MAY be Unicode letters, Unicode numbers, emoji, underscore (`_`), hyphen (`-`), and forward slash (`/`);
- forward slash (`/`) denotes hierarchy: `#kind/sample` is a child of `#kind`;
- slash-delimited tags MUST NOT contain empty path segments, so `#bad//tag`, `#bad/`, and `#/bad` are invalid;
- tags are case-insensitive (`#Kind` and `#kind` are the same tag); the first casing observed in a deterministic project scan SHOULD be treated as the canonical display form. If a tool needs a tie-breaker, it SHOULD scan records by lexicographic record path and then scan front matter before body text in document order;
- a tag MUST contain at least one non-numeric character in the tag as a whole (`#1984` is not valid; `#y1984` and `#dadr/0123456789ab` are valid). Numeric-only slash-delimited segments are allowed when another segment contains a non-numeric character;
- a tag MUST NOT contain whitespace; combine words with `camelCase`, `PascalCase`, `snake_case`, or `kebab-case`;

These are the only format rules. **DADR does not define a required tag namespace or controlled vocabulary.** Projects MAY define their own conventions.

### 5.2 Front-matter tags { #front-matter-tags }
Front-matter tags appear in the YAML `tags:` list, without the `#` prefix. They classify the whole record:

```yaml
---
title: Drop observations with missing treatment assignment
status: candidate
id: 01938b8e-4d70-7abc-8def-0123456789ab
tags:
  - kind/sample
  - confidence/high
---
```

### 5.3 Inline tags { #inline-tags }
Inline tags appear in the Markdown body prefixed with `#`. They attach to a specific sentence, paragraph, or subsection rather than the whole record. A common pattern is to put them on the first line of a Markdown blockquote so the tagged passage is visually grouped:

```markdown
> #rejected/no-longer-relevant
> The pre-analysis plan called for a complete-case rule, but the registered
> sample is small enough that we now prefer multiple imputation. This deviation
> was approved by the steward on 2026-05-07.
```

Tools SHOULD treat front-matter tags and inline tags as part of the same tag set when building indexes, reports, or coverage views.

### 5.4 Code tags { #code-tags }
Use code tags in source comments to link code to DADR records or to annotate
decision-relevant code with DADR-style tags.

A **code tag** is a tag with a leading `#dadr/` prefix:

```r
# #dadr/01938b8e-4d70-7abc-8def-0123456789ab
df <- df[!is.na(df$treatment), ]
```

It follows the same character rules as any other DADR tag (see [Format
rules](#format-rules)). The `#dadr/` prefix distinguishes DADR tags from
arbitrary comment text and from non-DADR tag systems in shared source files.

```python
# #dadr/kind/measurement #dadr/topic/student-survey
survey["anx_mean"] = survey[["anx1", "anx2", "anx3"]].mean(axis=1)
```

```sql
-- #dadr/kind/sample #dadr/topic/missingness
where treatment is not null
```

The `#dadr/` prefix is recommended **only in code comments**. Tags inside DADR record files (YAML or body) do not need the prefix because the surrounding context already identifies them as DADR metadata. If a DADR record nevertheless contains a tag such as `#dadr/kind/sample`, it is a normal tag whose top-level segment is `dadr`; it is valid but discouraged unless the project deliberately uses that namespace in records.

The same tag namespaces used in records also apply in code:

| Pattern | Meaning |
|---|---|
| `#dadr/<uuid7>` or `#dadr/<uuid7-prefix>` | Link the surrounding code to a specific record. A 12-character UUIDv7 prefix SHOULD be used when unambiguous, matching the wikilink recommendation. |
| `#dadr/kind/<value>` | Coarse classification of the decision kind. |
| `#dadr/topic/<value>` | Free-form topic. |
| `#dadr/plan/<id>` | Plan reference. |
| `#dadr/axis/<name>/<value>` | Robustness or sensitivity axis and the value implemented here. |
| `#dadr/confidence/<low\|medium\|high>` | Author confidence in the annotation. |
| `#dadr/ignore` | Suppress candidate generation for the surrounding code. |

Multiple tags MAY appear in one comment.

A code tag applies to the next logical statement or nearby code block. Parsers MAY use language-aware structure (AST, notebook cell boundaries, SQL statements, workflow nodes) when available. When language-aware structure is unavailable, the default fallback scope is the next non-blank, non-comment line after the tag comment. If the tag appears on the same line as executable code, the fallback scope is that line. For longer blocks, tag the first line; the parser uses the surrounding context to infer the range.

---

## 6. Links and citations { #links-and-citations }
DADR uses two link syntaxes, split by what they point at:

- **Wikilinks** `[[<uuid7-prefix>]]` for inter-DADR record references.
- **Standard Markdown** `[text](path-or-url)` for files, code references, and external URLs.

The split keeps each syntax to one job: Markdown handles paths (so Pandoc, Quarto, GitHub, and every other renderer can resolve them out of the box), and wikilinks handle stable UUID references (so the author types only the UUID and the link survives slug renames). The wikilink syntax `[[Page]]` originated with MediaWiki and is supported by most personal-knowledge-management tools (Obsidian, Logseq, Foam, Dendron, Roam, TiddlyWiki) and several wiki-style renderers.

### 6.1 Wikilinks { #wikilinks }
References between DADR records SHOULD use wikilinks containing a UUIDv7 prefix:

```markdown
See [[01938c20a3b4|the sampling rule]].
Compare with [[01938b8e4d70|the original preprocessing decision]].
```

The prefix MUST resolve unambiguously within the project. A 12-character UUIDv7 prefix SHOULD be used when unambiguous.

If a DADR wikilink prefix resolves to no record or to multiple records, conforming tools MUST report a validation error and MUST NOT silently select a target. The form `[[<uuid7-prefix>|display text]]` MAY be used to provide display text. Wikilinks that do not begin with a UUIDv7 prefix are outside the DADR inter-record link syntax; tools SHOULD pass them through unchanged.

**Structural supersession relationships MUST be represented in the `successors` front-matter array, not only in prose.** Wikilinks are for "see also" and reading-flow references.

Tools MAY resolve the prefix to the canonical filename and convert wikilinks to standard Markdown links during a static-site build, so renderers without wikilink support display them as ordinary clickable links.

### 6.2 Markdown { #markdown-links }
References to code, data, notebooks, manuscripts, plans, outputs, external registries, and web pages MUST use standard Markdown links or bare autolinks. Repository-relative links are allowed. Full URLs are allowed.

Repository-relative path with a line-range anchor:

```markdown
The exclusion is enforced in
[`analysis/build_sample.py` lines 42-58](analysis/build_sample.py#L42-L58).
```

Full source-host URL (clickable from any rendered location, including a published static site):

```markdown
The exclusion is enforced in
[`analysis/build_sample.py:42-58`](https://github.com/acme/study/blob/main/analysis/build_sample.py#L42-L58).
```

Notebook cell or section:

```markdown
See [cell `prep-sample`](analysis/eda.qmd#prep-sample) for the diagnostics
this rule is based on.
```

Data file:

```markdown
The cleaned dataset is [`data/cleaned/survey.csv`](data/cleaned/survey.csv).
```

External registry, dashboard, or web article:

```markdown
The pre-registration is at <https://osf.io/abcde>.
```

### 6.3 Citations { #citations }
DADR records MAY use standard Pandoc or Quarto citation syntax such as `[@citation-key]`.

If a record declares `bibliography`, citation-aware tools and renderers MUST resolve that bibliography for the record. A string value is one bibliography path. An array value is an ordered list of bibliography paths. Relative bibliography paths MUST be interpreted relative to the project root for portability across renderers and record locations.

Record-level `bibliography` replaces, rather than extends, any project-level default bibliography. If no record-level bibliography is declared, citation-aware tools and renderers MAY use a project-level default. Project-level defaults are defined by the tooling, not by this specification.

Most projects do not need to set `bibliography` on every record. The default is set once project-wide; the per-record field exists only as an override.

---

## 7. Identifiers { #identifiers }
Every DADR record MUST carry a UUIDv7 identifier (RFC 9562) in the front-matter `id` field. The identifier MUST be allocated when the record is created and MUST be written in lowercase hyphenated canonical text form (`8-4-4-4-12`, exactly 36 characters).

UUIDv7 is chosen because it is collision-resistant across concurrent branches and agents (no shared counter), encodes a Unix-millisecond timestamp, and sorts lexicographically by time:

```text
01938b8e-4d70-7abc-8def-0123456789ab
01938b8f-9d71-7456-8000-fedcba987654
01938b91-bcd0-7e00-8b00-cafebabe0002
```

The first 12 hex characters are the millisecond timestamp; the remaining digits are a version marker (`7`) and randomness.

The `id` field is the **stable machine key**. Filenames, titles, slugs, and directory locations are not stable machine keys.

A candidate converted into an accepted decision keeps the same UUIDv7 identifier. Directly authored accepted records are also allowed when the project has already completed the human approval step; the `id` still identifies the record, not a separate lifecycle event.

DADR derives record-creation timestamps from UUIDv7 identifiers:

- the candidate draft timestamp is the timestamp encoded in the record's UUIDv7;
- for records created directly as accepted decisions, the record creation timestamp is the timestamp encoded in the UUIDv7.

Acceptance, rejection, and supersession event times are not encoded as DADR lifecycle timestamps, because they mutate an existing record's `status` or `successors` rather than allocating an event identifier. The timestamp embedded in a converted candidate's UUIDv7 is the draft-creation time, not the acceptance time.

Projects that need exact acceptance, rejection, supersession, actor, reviewer, or review-latency data SHOULD use version-control history or tool-maintained event logs. A tool-maintained lifecycle event log SHOULD be append-only and SHOULD record, at minimum, the record `id`, the lifecycle action (`accepted`, `rejected`, or `superseded`), the event time, and the actor when known. Event logs are audit metadata: they MUST NOT override the lifecycle state expressed in record front matter.

DADR records MUST NOT store lifecycle timestamp overrides in front matter. Renderers MAY display UUIDv7-derived timestamps.

---

## 8. Filenames { #filenames }
### 8.1 Canonical form { #canonical-form }
A canonical DADR filename MUST begin with the **first 12 hex characters of the front-matter `id`**, with UUID hyphens removed, followed by `-`, a short kebab-case slug, and the `.md` extension:

```text
<uuid7[0:12]>-short-kebab-title.md
```

Example:

```text
01938b8e4d70-income-preprocessing.md
```

If the preferred canonical filename already exists, tools MUST choose a unique slug suffix while preserving the same 12-character prefix.

A conformant record whose filename does not match this form is not canonical.

The stable machine key is always the front-matter `id` (full UUIDv7); the filename is a review-friendly locator.

Tools MUST use the front-matter `id` as the stable identifier even when a filename is missing, non-canonical, renamed, or manually written.

### 8.2 Manual records { #manual-records }
Authors MAY draft records manually without canonical filenames or complete tool-generated metadata, provided the file contains enough information for safe normalization.

A manually drafted record MUST declare `title` and `status`, because title is required record metadata and lifecycle status cannot be inferred reliably from filename or directory location. The `id` field MAY be omitted; tools allocate a UUIDv7 during normalization:

```yaml
---
title: Income preprocessing idea
status: candidate
---
```

A full normalization pass MUST produce a canonical record by:

- allocating a fresh UUIDv7 when `id` is absent;
- writing the allocated UUIDv7 to the `id` field;
- filling safe structural defaults;
- renaming the file to the canonical form described above.

Tools MAY provide partial normalization modes that allocate or fill metadata without renaming, but such records are conformant rather than canonical.

Tools MUST NOT infer acceptance from directory location alone.

---

## 9. Project layout { #project-layout }
DADR does not mandate a specific directory structure. A conforming project MAY keep all records in one directory, split records by lifecycle state, organize records by topic, or use another layout that fits the project.

Specific tools MAY mandate a directory structure for discovery, lifecycle operations, local state, or user-interface consistency. A tool-mandated layout is a tool profile, not a requirement of the DADR record format.

A record's lifecycle stage is determined by the front-matter `status` field. Tools MUST NOT infer status from directory path alone; front matter remains authoritative.

Dadrock mandates this project layout:

```text
dadr/
  accepted/
    01938b8e4d70-income-preprocessing.md
    01938b8f9d71-drop-missing-treatment.md
  candidates/
    01938ba1c4f3-country-exclusions.md
  rejected/
    01938b8d9c10-deprecated-cohort-filter.md        # rejected
    01938b91bcd0-imputation-rule.md                 # rejected (duplicate)
  templates/
    candidate.md                                    # optional
  tasks/
  runs/
    .gitignore
```

In Dadrock's layout:

- accepted decisions, whether current or superseded, live under `dadr/accepted/`;
- active candidates live under `dadr/candidates/`;
- rejected records live under `dadr/rejected/`;
- project-local task customizations live under `dadr/tasks/` as task packages such as `dadr/tasks/<task-id>/task.toml` plus the prompt file named by that manifest;
- task run state lives under `dadr/runs/` and is ignored by default.

When a candidate is accepted, the same record is converted into an accepted decision: its `status` changes from `candidate` to `accepted`, its UUIDv7 stays unchanged, and under Dadrock's layout the file moves from `dadr/candidates/` to `dadr/accepted/`. Acceptance does not add a `successors` edge. When a candidate is rejected, the same record changes to `status: rejected` and moves to `dadr/rejected/`.

Tools that support non-Dadrock layouts SHOULD document how they discover the project record set. Conforming tools MUST use the front-matter `status` to identify lifecycle stage, not the directory.

---

## 10. Lifecycle { #lifecycle }
### 10.1 Status vocabulary { #status-vocabulary }
The `status` field is mandatory and MUST be one of:

| Status | Meaning |
|---|---|
| `candidate` | A proposed decision: untriaged, in active review, or being revised. |
| `accepted` | A human-approved decision record. May be current or superseded; supersession is derived from `successors`, not from `status`. |
| `rejected` | Candidate closed without becoming a decision (rejected outright, withdrawn, duplicate, merged, or no longer relevant). |

Status values outside this vocabulary are invalid.

For `rejected` records, the closure subtype MAY be captured in the body or with tags such as `#rejected/duplicate`, `#rejected/merged`, `#rejected/withdrawn`, or `#rejected/no-longer-relevant`. A rejection without a subtype tag is treated as a plain rejection.

An accepted record whose `successors` array points to one or more accepted records is **superseded**. This is a derived predicate on the record graph and not a `status` value.

### 10.2 Legal transitions { #legal-transitions }

The only status transitions are:

```text
candidate -> accepted
candidate -> rejected
```

Acceptance changes a candidate's `status` to `accepted` on the same record. Supersession does **not** change a record's `status`; it appends to an accepted predecessor's `successors` array (see [Acceptance](#acceptance) and [Supersession](#supersession)).

`rejected` is terminal: a rejected record's `status` MUST NOT change. Accepted records are also write-once for `status` (it stays `accepted` whether or not the record is later superseded). See [Immutability](#immutability).

If a rejected idea becomes relevant again, a new candidate with a fresh UUIDv7 MUST be created. If a superseded decision becomes relevant again, a new candidate MUST be created and converted through the normal acceptance process.

### 10.3 Acceptance { #acceptance }
Acceptance is a human approval boundary. Tools and agents MUST NOT auto-accept records without human approval.

When a candidate is accepted:

1. the candidate record MUST persist;
2. its `status` MUST become `accepted`;
3. its UUIDv7 `id` MUST remain unchanged;
4. its body MUST satisfy the requirements for an accepted record;
5. acceptance MUST NOT create a `successors` edge from the candidate to another record.

Tools MAY move the converted record to the accepted-record location in the project's layout. Tools MAY require the candidate to be edited before acceptance so the accepted record satisfies the body-section and project-policy requirements.

Acceptance audit metadata, such as who accepted the candidate and when, MUST NOT be written as front-matter lifecycle fields. Tools that expose this metadata SHOULD derive it from version-control history or from a tool-maintained lifecycle event log.

### 10.4 Rejection { #rejection }
When a candidate is rejected:

1. the candidate record MUST persist;
2. its `status` MUST become `rejected`;
3. no accepted-decision record is created;
4. its `successors` array MUST be absent or empty.

### 10.5 Supersession { #supersession }

When an accepted decision is superseded:

1. a successor accepted decision MUST exist, either as an existing accepted record or as a candidate converted to `status: accepted` as part of the same human-approved operation;
2. the prior accepted decision MUST persist with `status: accepted`;
3. the prior accepted decision's `successors` array MUST include the successor accepted decision's UUIDv7.

The `successors` array on the prior accepted record is the canonical machine-readable record of the supersession relationship. Acceptance itself is represented only by the converted record's `status: accepted`.

```yaml
successors:
  - 01938c20-a3b4-7000-8000-111122223333
```

Cardinality is many-to-many on both sides:

- **Split.** One accepted predecessor MAY have multiple accepted successors when a single decision is replaced by two or more granular decisions. Each successor's UUIDv7 is appended to the predecessor's `successors` array.
- **Consolidation.** Multiple accepted predecessors MAY share a single accepted successor when one new decision supersedes several older ones at once. The same successor UUIDv7 appears in each predecessor's `successors` array.

The successor graph MUST contain accepted records only and MUST be acyclic. A record MUST NOT list its own `id` in `successors`. Each `successors` entry MUST resolve to an accepted record within the project; unresolved or non-accepted entries are validation errors. Candidate and rejected records MUST NOT have non-empty `successors`.

**Only the forward direction is stored on disk.** The reverse direction (predecessors of a given record) MUST be derived by tools that scan all records for matching `successors` entries. Because `successors` is an array on every record, the full directed acyclic graph can always be reconstructed.

## 11. Immutability { #immutability }
Candidate records MAY be revised freely while under review.

Accepted decision records are **immutable for substantive content**. The body, including the context, decision, alternatives, and consequences, MUST NOT be edited in place after acceptance. Acceptance is the boundary where the draft becomes durable. To revise a substantive accepted decision, a project MUST create a new candidate, accept it, and supersede the previous accepted decision (see [Supersession](#supersession)).

Non-substantive edits (spelling, formatting, citation repairs) MAY be made according to project policy. Tools SHOULD make such edits visible and reviewable.

Users who want to track in-place evolution of a record (typo fixes, formatting tweaks, contraventions of the immutability imperative) SHOULD rely on a version-control system. DADR's lifecycle model captures intentional decision changes; version control captures everything else.

Tools MAY enforce immutability by recording accepted-decision content baselines and warning when subsequent edits change those baselines. The baseline format, hash algorithm, and normalization policy are tooling concerns unless a DADR profile defines them.

---

## 12. Example record { #non-normative-complete-example }

This is a non-normative complete example:

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
introducing an additional imputation model.

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

The reference implementation is **dadrock**, a Rust library with CLI, Web UI, and IDE integrations. See the [**dadrock**](dadrock/dadrock.md) section for the command reference and the surrounding tooling (CLI, web, IDE). See the [Tags](tags.md) page for the recommended tag vocabulary.
