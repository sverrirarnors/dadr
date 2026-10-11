# Optional tags

You can use DADR without tags. Add them when they help your team find related decisions; there is no mandatory vocabulary.

## Start small

These are suggested conventions, not reserved fields or required categories:

| Tag | Meaning |
|---|---|
| `kind/sample` | Sample definition or inclusion rules |
| `kind/modeling` | Model choices |
| `kind/inference` | Uncertainty and tests |
| `writer/agent` | An agent helped draft the record |
| `people/decider/alice` | Person responsible for the choice |
| `people/approver/bob` | Person who reviewed and approved it |
| `people/reviewer/bob` | Optional project convention for a reviewer |
| `axis/sample/complete-case` | A choice along a robustness or specification axis |
| `multiverse/model/ols` | Another project convention for an alternative specification |
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

## Inspect and validate

```sh
dadrock tags list
dadrock validate tags
```

The specification also describes `#dadr/` annotations in code comments. Dadrock does not currently scan or validate those source annotations; that is [roadmap work](roadmap.md).

## Optional code annotations

These conventions describe future tooling; Dadrock does not yet scan source comments.


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

