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

## Suggested tags for agent-drafted candidates

Tags are optional. Respect existing project vocabularies and user preferences. When useful, these defaults help classify agent-drafted candidates:

| Tag | Values | Why |
|---|---|---|
| `kind/<surface>` | `setup`, `input`, `sample`, `measurement`, `transformation`, `exploration`, `modeling`, `inference`, `robustness`, `visualization`, `communication`, `output`, `governance`, `quality`, `orchestration` | Main classification axis. Drives downstream filtering and review routing. |
| `writer/agent` | (no value) | Provenance: drafted by an agent, not the analyst. |

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
| `axis/<axis>/`, `multiverse/<axis>/`, or `robustness/` | Alternative specifications and robustness work | `axis/model/ols`, `multiverse/sample/complete-case`, `robustness/placebo` |
| `people/decider/` and `people/approver/` | Decision ownership and review attribution, only when the evidence names the people | `people/decider/alice`, `people/approver/bob` |
| `rejected/` | Rejection subtype (on rejected records only) | `rejected/duplicate`, `rejected/withdrawn`, `rejected/no-longer-relevant` |

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

The prefix is recommended **only in code comments** — tags inside DADR record files do not need it. Dadrock does not yet scan or validate code tags; agents reading code can still honor them.
