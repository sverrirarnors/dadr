# Tags reference

Tag format and the recommended vocabulary, mirroring the dadr Tags page and specification §7.

## Where tags live

| Location | Prefix | Scope |
|---|---|---|
| Front-matter `tags:` list | none | Whole record |
| Inline body text (often first line of a blockquote) | `#` | One paragraph or sentence |
| Source code comments | `#dadr/` | Surrounding code |

```yaml
---
tags:
  - kind/sample
  - writer/agent
---
```

```markdown
> #multiverse/sample/complete-case
> Drop the 47 respondents without recorded treatment assignment.
```

```r
# #dadr/01938b8e4d70
df <- df[!is.na(df$treatment), ]
```

## Format rules

- Unicode letters, digits, emoji, underscore, hyphen, forward slash.
- `/` denotes hierarchy: `kind/sample` is a child of `kind`. No empty levels (`a//b`, `a/`, `/a`).
- Case-insensitive (`Kind` ≡ `kind`); use the project's existing casing.
- Must contain at least one character other than a digit, `_`, `-`, or `/`.
- No whitespace; combine words with `kebab-case`, `snake_case`, `camelCase`, or `PascalCase`.

## Suggested tags for agent-drafted candidates

Tags are optional. Respect existing project vocabularies and user preferences. When useful, give an agent-drafted candidate `writer/agent` (provenance: drafted by an agent, not the analyst) and one `kind/` value.

### Kinds

`kind/` is the main classification: which part of the analysis a decision is about. It drives downstream filtering and grouping.

| Tag | Decisions about |
|---|---|
| `kind/setup` | The computing environment: software and package versions, random seeds, configuration |
| `kind/input` | Data sources: which datasets, versions, and extracts are used, and how they are obtained |
| `kind/sample` | Which units and observations enter the analysis: inclusion and exclusion rules, filters, joins, time windows |
| `kind/measurement` | How constructs become variables: outcome, treatment, and covariate definitions, coding, scales, derived scores |
| `kind/transformation` | How values are processed: cleaning, transforms, winsorizing, standardizing, imputation |
| `kind/exploration` | Exploratory analysis: descriptive checks and diagnostics that inform later choices |
| `kind/modeling` | Model specification: estimand, estimator, formula, fixed or random effects, priors |
| `kind/inference` | Uncertainty and testing: standard errors, intervals, tests, multiple-comparison adjustments |
| `kind/robustness` | Sensitivity analysis: alternative specifications, placebo and falsification checks |
| `kind/visualization` | Figures: plot types, encodings, scales |
| `kind/communication` | Reporting to readers: which results appear, tables, captions, labels, rounding, wording |
| `kind/output` | Produced artifacts: which result files and datasets are written, in what format, and where |
| `kind/governance` | Process: preregistration, plan deviations, ethics and data-access constraints, sign-off |
| `kind/quality` | Checks: data validation rules, assertions, tests, reproducibility checks |
| `kind/orchestration` | Pipeline execution: step order, dependencies, caching, parallelization |

Use one `kind/` value per candidate unless the project already has a clear
multi-kind convention. Prefer the closest analytic activity over a broad label:
`kind/communication` for table/caption/manuscript wording, `kind/visualization`
for plot encoding, and `kind/modeling` for model formulas or estimators.

## Optional namespaces

Add when applicable. Do not invent namespaces beyond these unless the project already uses them.

| Namespace | Use it for | Examples |
|---|---|---|
| `writer/` | Who drafted the record | `writer/agent`, `writer/analyst` |
| `confidence/` | How settled the choice or its supporting evidence is | `confidence/low`, `confidence/medium`, `confidence/high` |
| `topic/` | A free-form subject that cuts across kinds | `topic/missingness`, `topic/winsorization`, `topic/multiple-testing` |
| `plan/` | The plan a decision implements or deviates from | `plan/sap-2026`, `plan/pap-main` |
| `axis/<axis>/<value>` | A choice along a robustness or specification axis | `axis/sample/complete-case`, `axis/model/ols` |
| `multiverse/<axis>/<value>` | The same idea, for projects that call their axes a multiverse | `multiverse/sample/complete-case`, `multiverse/se-type/cluster-robust` |
| `robustness/` | The specific robustness check a record covers | `robustness/placebo`, `robustness/leave-one-out` |
| `people/decider/` | Person responsible for the choice | `people/decider/alice` |
| `people/approver/` | Person who reviewed and approved it | `people/approver/bob` |
| `people/reviewer/` | Optional project convention for a reviewer | `people/reviewer/carol` |
| `rejected/` | Why a candidate was closed; rejected records only | `rejected/duplicate`, `rejected/merged`, `rejected/withdrawn`, `rejected/no-longer-relevant` |

Pick one of `axis/` and `multiverse/` per project, following the one it already uses. Add `people/` tags only when the evidence names the people.

## Tidy tags

- Prefer one spelling per concept. Do not mix `kind/sample` and `kind/sampling`.
- Use singular forms consistently.
- Avoid near-duplicates. Before adding `topic/missing-data`, check whether the project already uses `topic/missingness`.

Check which tags the project already uses (for example with `dadrock tags list`) before inventing new ones.

## Code tags (`#dadr/`)

When annotating source code (not record files), use the `#dadr/` prefix:

| Pattern | Meaning |
|---|---|
| `#dadr/<short-id>` | Link surrounding code to a specific record, e.g. `#dadr/01938b8e4d70`. A full UUID also works. |
| `#dadr/kind/<value>` | Inline classification (no record yet). |
| `#dadr/topic/<value>` | Free-form topic. |
| `#dadr/multiverse/<axis>/<value>` | Robustness/specification axis and the value implemented here. |
| `#dadr/ignore` | Dadrock convention: do not draft candidates from the surrounding code. |

The prefix belongs **only in code comments** — tags inside DADR record files do not use it. Dadrock does not yet scan or validate code tags; agents reading code can still honor them.
