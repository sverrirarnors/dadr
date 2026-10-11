# Tags

You can use DADR without tags. Add them when they help your team find related decisions; there is no mandatory vocabulary.

## Start small

These are suggested conventions, not reserved fields or required categories:

| Tag | Meaning |
|---|---|
| `kind/sample` | Which observations enter the analysis |
| `kind/modeling` | Model specification and estimator |
| `kind/inference` | Uncertainty, tests, and intervals |
| `writer/agent` | An agent drafted the record |
| `confidence/low` | The choice is tentative or weakly supported |
| `people/decider/alice` | Person responsible for the choice |
| `axis/sample/complete-case` | A choice along a robustness or specification axis |
| `rejected/duplicate` | Why a candidate was closed |

Use project-specific tags when they are useful. Existing vocabularies remain valid; there is no need to migrate records just to follow these examples.

## Where to put them

In YAML front matter, omit the leading `#`:

```yaml
tags:
  - kind/sample
  - people/decider/alice
```

In prose, include it:

```markdown
The exclusion rule is one sensitivity-analysis option. #axis/sample/complete-case
```

Use one spelling per concept and avoid near-duplicates. See the [specification](specification.md#tags) for exact syntax and normalization rules.

## Recommended vocabulary

When a few tags are not enough, use this vocabulary. The DADR agent skill and DADRock's task prompts follow it. It is still a suggestion: DADRock does not enforce it, and a project's own conventions take precedence.

### Kinds

`kind/` is the main classification: which part of the analysis a decision is about. Most records need one `kind/` value: pick the closest match.

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

### Other namespaces

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

Pick one of `axis/` and `multiverse/` per project.

## Inspect and validate

```sh
dadrock tags list
dadrock validate tags
```

The specification also describes `#dadr/` annotations in code comments. DADRock does not currently scan or validate those source annotations; that is [roadmap work](roadmap.md).

## Optional code annotations

These conventions describe future tooling; DADRock does not yet scan source comments.

In source-code comments, prefix tags with `#dadr/`. The same namespaces apply:

| Pattern | Meaning |
|---|---|
| `#dadr/<short-id>` | Link the surrounding code to a record, e.g. `#dadr/01938c20a3b4`. A full UUID also works. |
| `#dadr/kind/<value>` | The kind of decision the code implements. |
| `#dadr/topic/<value>` | Free-form topic. |
| `#dadr/plan/<id>` | The plan item the code implements. |
| `#dadr/multiverse/<axis>/<value>` | The value of a robustness or specification axis implemented here. |
| `#dadr/confidence/<low\|medium\|high>` | Author confidence in the annotation. |

Several tags can share one comment:

```sql
-- #dadr/kind/sample #dadr/topic/missingness
where treatment is not null
```

