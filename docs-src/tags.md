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
| `axis/sample/complete-case` | A choice along a robustness or specification axis |

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
