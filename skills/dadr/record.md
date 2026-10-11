# Record reference

What a single DADR record should look like, end-to-end. Use this whenever you draft, revise, supersede, or audit a record. For the tag vocabulary, see [tags.md](tags.md).

A **record** is a Markdown file with YAML front matter and four prose sections. Every record in `dadr/` (whether a candidate, accepted, superseded, or rejected) follows the same anatomy; only the `status` field and a few status-specific rules differ.

## Lifecycle

The `status` field takes one of three values: `candidate`, `accepted`, or `rejected`. "Superseded" is not a status: it is an accepted record whose `successors` array is non-empty.

| Record | Editable? | Notes |
|---|---|---|
| `candidate` | Yes — freely revisable | A proposed decision under review. Lives in `dadr/candidates/`. Each of the four sections must have text before acceptance; thin candidates create work for the reviewer. |
| `accepted` | No changes in meaning | The decision is in force. Lives under `dadr/accepted/`. Typo and link fixes are fine; anything else goes through a new candidate plus supersession. Use `dadrock supersede` to link accepted replacements, or accept a replacement candidate with a `## Replaces` section. |
| `accepted`, superseded | No changes in meaning | An accepted record whose `successors` array points at its replacement. Stays in `dadr/accepted/` with `status: accepted`. |
| `rejected` | No edits | A closed candidate. Usually tagged with `rejected/<reason>` (e.g., `rejected/duplicate`, `rejected/withdrawn`). |

Acceptance, rejection, and supersession are all **human approval boundaries**. No skill auto-transitions.

## Anatomy

```markdown
---
title: Drop observations with missing treatment assignment
status: candidate
id: 01938b8f-9d71-7456-8000-fedcba987654
tags:
  - kind/sample
  - writer/agent
---

## Context

The treatment variable is required for the primary estimand defined in
[docs/sap.md §3.2](../../docs/sap.md#3-2-primary-estimand). The cleaned panel at
[analysis/build_sample.R:12-18](../../analysis/build_sample.R#L12-L18) currently
keeps the 47 respondents whose `treatment` is `NA`, which would force an
imputation model upstream of the main analysis.

## Decision

Drop the 47 respondents with missing `treatment` from the main analysis
sample before any downstream modeling.

## Alternatives

Impute treatment assignment using a logistic model on baseline covariates.
Rejected because the assignment mechanism is part of the study design, not a
downstream measurement, and imputing it would obscure the estimand.

Keep the 47 respondents for descriptive tables only. This is consistent with
the dropping rule and could be done as a supplemental descriptive table, but
it is not the main-analysis decision being recorded here.

## Consequences

The main sample drops from 1,247 to 1,200. Robustness obligation: verify that
treatment missingness is not concentrated in one site. The manuscript should
report the exclusion count and the missingness pattern.
```

## Required structure

### Front matter

| Field | Notes |
|---|---|
| `title` | Noun phrase describing the choice. **Not a question.** |
| `status` | One of `candidate \| accepted \| rejected`. Set on creation; changed only via `dadrock status` lifecycle commands. |
| `id` | UUIDv7 allocated by `dadrock new`. **Never write by hand.** Its first 12 hex digits are the record's short id. |
| `tags` | Optional; follow project conventions (see [tags.md](tags.md)). |
| `successors` | Optional list of UUIDv7 ids. Names accepted replacements; a non-empty list makes an accepted record superseded. Written by Dadrock during replacement acceptance or an explicit `dadrock supersede` action. |

Other keys (for example Pandoc's `bibliography`) are allowed and passed through unchanged; DADR gives them no meaning. **Never add dates**: no `date` key and no `2026-05-07`-style value under any key. A record's time comes only from its `id`.

### Body — four sections

Every accepted record has all four `##` headings, each appearing exactly once and each followed by text. They MAY appear in any order; the conventional order is:

1. `## Context`
2. `## Decision`
3. `## Alternatives`
4. `## Consequences`

#### Context — 2–4 sentences

What problem, dataset, or constraint motivates this choice?

- **Always** include a Markdown link to the source surface. Relative paths resolve from the record's own directory; records live in `dadr/<status>/`, so a repository file is `../../path`: `[analysis/build_sample.py:42-58](../../analysis/build_sample.py#L42-L58)`.
- Link other records with a short-id wikilink: `[[01938b8e4d70]]`.
- Mention the dataset, prior decision, or plan section that frames the choice.
- For records produced by supersession, also name the predecessor and the supersession trigger.

#### Decision — 1–3 sentences

What was actually done?

- Active voice: "Drop the 47 respondents…", not "It was decided that…".
- Present tense for `accepted` records ("Drops the 47 respondents…"); present-tense or imperative for `candidate` records.
- If you are inferring from code rather than transcribing from a plan, say so: "The code at L12-L18 implies…".
- An inline tag MAY appear on the first line of a blockquote here for
  `#multiverse/...`, `#confidence/...`, or similar paragraph-level metadata.

#### Alternatives — 1–3 alternatives, each a sentence or two

The defensible options not chosen, with a one-line reason each.

- Real alternatives only. **Do not fabricate.**
- If genuinely no alternative applies, write **"Not applicable"** — never leave empty.
- For records produced by supersession, the predecessor itself usually appears here as a rejected alternative.
- Bullets are acceptable here; prose is fine too.

#### Consequences — 2–4 sentences

Downstream effects on sample size, estimand, robustness obligations, or reporting.

- Quantify when possible (sample-size delta, expected direction).
- Note follow-ups the analyst should consider.
- For records produced by supersession, note any prior result that may need to be re-run.

## Suggested tags

Tags are optional. When useful and consistent with the project vocabulary, a record may carry:

```yaml
tags:
  - kind/<surface>      # setup | input | sample | measurement |
                        # transformation | modeling | inference |
                        # communication | robustness | ...
```

Agent-drafted records may additionally carry:

```yaml
  - writer/agent        # provenance: drafted by an agent, not the analyst
```

See [tags.md](tags.md) for optional namespaces (`confidence/`, `topic/`,
`plan/`, `axis/`, `people/decider/`, `rejected/`, ...).

## Length envelope

A well-formed record is **roughly 200–400 words of body prose** — long enough that a reviewer can accept or reject it without reopening every linked source file, short enough that a reviewer reads it in under a minute.

- Below ~100 words usually means at least one section is too thin.
- Above ~600 words usually means two decisions are bundled — split into two records.

## Common pitfalls

- **Empty `Alternatives`.** Write "Not applicable" or describe a generic methodological alternative honestly tagged `confidence/low`.
- **Bullet dumps.** Use prose. Bullets are fine in `Alternatives`; the other three sections should be sentences.
- **No source link in `Context`.** Every record is grounded in evidence. If you cannot link to a file/line, you cannot draft the record yet — surface the gap to the user.
- **Two decisions in one file.** If `Decision` describes both an exclusion and a recoding, split into two records.
- **Hand-written `id`.** Always go through `dadrock new`.
- **Hand-written `successors`.** Always go through `dadrock supersede`.
- **Question-form titles.** "Should we drop missing treatment?" → "Drop observations with missing treatment".
- **Tense drift in `Decision`.** Active voice describing what was done (or what the code does); not "we plan to" or "we should".
- **Editing accepted bodies.** Preserve substantive content; changed decisions require a new candidate plus supersession. Minor corrections are allowed.

## Minimum viable record

When a decision is genuinely terse — e.g., a reporting choice with no real alternatives — the floor still has all four sections, a source link, and optional tags:

```markdown
---
title: Round reported percentages to one decimal place
status: candidate
id: 01938c4a-1234-7000-8000-aabbccddeeff
tags:
  - kind/communication
  - writer/agent
---

## Context

Tables 1 and 3 of [paper.qmd](../../paper.qmd#tables) report percentages. The
current draft mixes one- and two-decimal-place rounding.

## Decision

Round all reported percentages to one decimal place.

## Alternatives

Two decimal places. Rejected because point estimates are rarely meaningful at
that precision given the sample size of n=1,200.

## Consequences

Pure reporting choice; does not affect estimates or inference. Update the
manuscript's table-formatting helper.
```

This is the bar a record must clear before acceptance, not a candidate-only convenience.
