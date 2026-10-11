# Tags reference

Tag format and the namespaces this skill uses. Distilled from the dadr `tags.md` page and specification §7.

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
- `/` denotes hierarchy: `kind/sample` is a child of `kind`.
- Case-insensitive (`Kind` ≡ `kind`); use the project's existing casing.
- Must contain at least one character other than a digit, `_`, `-`, or `/`.
- No whitespace; combine words with `kebab-case`, `snake_case`, `camelCase`, or `PascalCase`.

## Mandatory tags for agent-drafted candidates

Every candidate this skill creates **must** include:

| Tag | Values | Why |
|---|---|---|
| `kind/<surface>` | `setup`, `input`, `sample`, `measurement`, `transformation`, `exploration`, `modeling`, `inference`, `robustness`, `visualization`, `communication`, `output`, `governance`, `quality`, `orchestration` | Main classification axis. Drives downstream filtering and review routing. |
| `writer/agent` | (no value) | Provenance: drafted by an agent, not the analyst. |

Pass them via repeated `--tag` flags (no `#` prefix on the CLI):

```bash
dadrock new "Drop observations with missing treatment" \
  --tag kind/sample \
  --tag writer/agent \
  --quiet
```

Use one `kind/` value per candidate unless the project already has a clear
multi-kind convention. Prefer the closest analytic activity over a broad label:
`kind/communication` for table/caption/manuscript wording, `kind/visualization`
for plot encoding, and `kind/modeling` for model formulas or estimators.

## Optional namespaces

Add when applicable. Do not invent namespaces beyond these unless the project already uses them.

| Namespace | Use it for | Example values |
|---|---|---|
| `confidence/` | How settled the choice or its inference is | `confidence/low`, `confidence/medium`, `confidence/high` |
| `topic/` | Free-form subject | `topic/missingness`, `topic/winsorization`, `topic/multiple-testing` |
| `plan/` | Plan reference | `plan/sap-2026`, `plan/pap-main` |
| `multiverse/<axis>/` or `robustness/` | Alternative specifications and robustness work | `multiverse/model/ols`, `robustness/placebo` |
| `people/decider/` and `people/reviewer/` | Decision ownership and review attribution | `people/decider/alice`, `people/reviewer/bob` |
| `rejected/` | Rejection subtype (on rejected records only) | `rejected/duplicate`, `rejected/withdrawn`, `rejected/no-longer-relevant` |

## Tidy tags

- Prefer one spelling per concept. Do not mix `kind/sample` and `kind/sampling`.
- Use singular forms consistently.
- Avoid near-duplicates. Before adding `topic/missing-data`, check whether the project already uses `topic/missingness`.

Inspect what's already in use before inventing:

```bash
dadrock tags list
```

## Code tags (`#dadr/`)

When annotating source code (not record files), use the `#dadr/` prefix:

| Pattern | Meaning |
|---|---|
| `#dadr/<short-id>` | Link surrounding code to a specific record, e.g. `#dadr/01938b8e4d70`. A full UUID also works. |
| `#dadr/kind/<value>` | Inline classification (no record yet). |
| `#dadr/topic/<value>` | Free-form topic. |
| `#dadr/multiverse/<axis>/<value>` | Robustness/specification axis and the value implemented here. |
| `#dadr/ignore` | Dadrock convention: suppress candidate generation for the surrounding code. |

The prefix is recommended **only in code comments** — tags inside DADR record files do not need it.
