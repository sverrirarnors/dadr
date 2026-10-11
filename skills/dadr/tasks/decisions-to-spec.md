# Decisions -> spec

Read the accepted DADRs in a project's `dadr/` directory and synthesize a written implementation or analysis specification from them. The decisions remain the source of truth, and the spec summarizes what downstream code, analysis, or writing should follow.

## Preconditions

The shared preconditions in [../SKILL.md](../SKILL.md) apply. In addition:

1. `dadr/` must contain at least one accepted record:
   ```bash
   dadrock list --status accepted --json
   ```
   If empty, stop and ask the user what decision set or draft material should drive the spec.

2. The user should identify the intended spec surface when it is not obvious: analysis plan, implementation plan, methods spec, reporting spec, robustness spec, or a project-specific document.

## Workflow

### Step 1 - Enumerate accepted decisions

```bash
dadrock list --status accepted --json
```

The JSON envelope's `data` array gives `{id, title, path, tags, ...}` for each record. Read the file at `path` to get the four sections.

To narrow the spec, filter by tag:

```bash
dadrock list --status accepted --tag kind/sample --json
dadrock list --status accepted --tag kind/modeling --json
dadrock list --status accepted --tag plan/sap-2026 --json
```

### Step 2 - Parse each decision

For each accepted record, extract:

- **Title** - the noun phrase summarizing the choice.
- **Context** - source links, constraints, datasets, plans, or prior records that motivate the choice.
- **Decision** - the binding requirement the spec must preserve.
- **Alternatives** - rejected options that the spec must not silently reintroduce.
- **Consequences** - downstream obligations, checks, or reporting caveats.
- **Tags** - grouping signals such as `kind/`, `topic/`, and `plan/`.

If a decision is too vague to specify an action, include it in an "Open questions" section rather than inventing missing requirements.

### Step 3 - Organize the spec

Group decisions by the surface the user needs:

| Tags or content | Spec section |
|---|---|
| `kind/sample`, cohort filters, joins, exclusions | Sample and data construction |
| `kind/measurement`, recodes, scale construction | Variables and measurement |
| `kind/transformation`, imputation, winsorization | Transformations |
| `kind/modeling`, estimands, fixed effects, priors | Models and estimands |
| `kind/inference`, standard errors, intervals, multiplicity | Inference |
| `kind/robustness`, `axis/`, sensitivity choices | Robustness and sensitivity |
| `kind/communication`, tables, figures, claims | Reporting |
| plan deviation sections, amendments | Deviations from prior plans |

Within each section, write requirements in active voice and cite the DADR id or file path for traceability.

### Step 4 - Draft the spec

Use this structure unless the user requests a different format:

1. **Purpose and scope** - what decisions this spec covers and which tags or statuses were included.
2. **Decision-derived requirements** - grouped by section, with each requirement tied to one or more accepted DADRs.
3. **Constraints and non-goals** - alternatives that must not be implemented or reported as current choices.
4. **Downstream checks** - validation, reruns, robustness checks, descriptive-stat updates, or manuscript updates implied by consequences.
5. **Open questions** - accepted records that lack enough detail for a precise requirement, conflicts, or missing source links.

Do not rewrite the accepted DADRs. The spec is derived documentation, not a lifecycle transition.

### Step 5 - Verify

Before reporting:

1. Re-scan the accepted decision list and confirm every included record appears in the spec or in "Open questions."
2. Check that no rejected `Alternative` is phrased as an active requirement.
3. Run `dadrock validate` if the spec links to DADR files or source anchors.
4. If the spec is written to a file, report the path and summarize the decision count covered.

## Constraints

- **Accepted decisions are binding.** Do not soften or reinterpret the `Decision` section to make the spec easier to write.
- **Alternatives stay alternatives.** Mention them only as non-goals or rejected options.
- **Stop on conflict.** If two accepted records prescribe incompatible requirements, do not choose one silently. Put the conflict in "Open questions" and ask the user to resolve it through supersession.
- **No lifecycle changes.** This task does not accept, reject, supersede, or edit records.
- **Traceability matters.** Each substantive requirement should point back to a DADR id, path, or title.

## Quick reference

| Goal | Command |
|---|---|
| List accepted decisions | `dadrock list --status accepted` |
| Same, JSON for scripting | `dadrock list --status accepted --json` |
| Filter by tag | `dadrock list --status accepted --tag kind/sample` |
| Validate links | `dadrock validate` |
| Inspect one record | `dadrock status <query>` |

## Example end-to-end

User: "turn the accepted decisions into an analysis spec"

1. `dadrock list --status accepted --json` -> 12 accepted records.
2. Read each record and group by `kind/`, `topic/`, and `plan/` when those tags
   exist.
3. Draft sections for sample construction, variables, models, inference, robustness, and reporting.
4. For the accepted record "Cluster standard errors at the school level", write: "All primary models use school-clustered standard errors. Source: dadr/ledger/<id>-cluster-standard-errors.md."
5. Put an accepted but vague "Use conservative robustness checks" record under "Open questions" because it does not specify the robustness axes.
6. Report that the spec covers 11 records and leaves 1 open question for supersession or clarification.
