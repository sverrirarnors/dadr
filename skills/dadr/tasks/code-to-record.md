---
id: code-to-record
title: "Code to records"
description: "Scan selected project files and draft candidate DADRs for consequential choices to review."
version: 1
inputs:
  - name: path
    label: "Path"
    type: path
    default: "."
  - name: focus
    label: "Focus"
    type: textarea
    required: false
  - name: limit
    label: "Maximum decisions"
    type: number
    default: "5"
    required: false
requirements:
  records: none
  record_status: all
  paths: required
  notes: optional
  scope: none
output:
  kind: candidate_batch.v1
mcp_access: full
tools:
  - name: dadrock.project_snapshot
    mode: read
---

# Code -> record

Crawl an existing data-analysis project, identify the consequential choices that warrant a Data Analysis Decision Record, and draft one candidate per decision with all four standard sections filled in.

This is for **onboarding existing projects**. For projects that already use DADR, prefer `dadrock new` directly or the `dadrock agent draft` ACP flow.

## Preconditions

The shared preconditions in [../SKILL.md](../SKILL.md) apply. In addition:

- If `dadrock dir` errors or no project is configured, run `dadrock init`. It creates the Dadrock project directory at `dadr/`, including `dadr/accepted/`, `dadr/candidates/`, `dadr/rejected/`, `dadr/templates/`, `dadr/tasks/`, and `dadr/runs/`.
- If `dadrock dir` prints an existing path, **skip init** — it is idempotent but noisy.

## Workflow

### Step 1 — Inventory the project

Build a focused picture of what kind of analysis this is. Read, in this order:

1. `README.md`, `README.qmd`, `index.qmd`, or top-level prose — what is the study about?
2. Any plan-like artefact: `sap.md`, `pap.md`, `preregistration.md`, `analysis-plan.*`, `protocol.*`, `data-cleaning-plan.*`, files in `plan/`, `docs/plan/`, `preregistration/`.
3. The analysis tree: `R/`, `analysis/`, `src/`, `code/`, `notebooks/`, `scripts/`, `python/`, `stata/`, plus any `.R`, `.r`, `.py`, `.ipynb`, `.qmd`, `.Rmd`, `.do`, `.sql`, `.smk`, `Snakefile`, `Makefile`, `targets.R`, `_targets.R`.
4. The manuscript if present: `paper.qmd`, `manuscript.*`, `ms.*`, `*.tex`.
5. Existing `dadr/` records (skip records that already exist — do not duplicate).

Use `Glob` and `Grep` for breadth, then `Read` for the files that look load-bearing. Spawn an `Explore` subagent for repos larger than ~50 source files so the main context stays clean.

### Step 2 — Identify consequential decisions

A decision is **consequential** when it plausibly changes results, interpretation, or scope. Canonical surfaces:

- **Sample construction**: inclusion/exclusion rules, dropping missing rows, deduplication, joins/linkage rules, time windows, geographic or cohort filters.
- **Variable definitions**: outcome, treatment/exposure, covariates, constructs, derived scores, recoding, scale composition.
- **Transformations**: log/sqrt/winsorize/standardize, imputation rule, top-coding, categorization, threshold/cut points.
- **Model & estimator**: formula, family/link, fixed vs. random effects, prior, hyperparameter, resampling design.
- **Inference**: standard errors (HC*, clustered, bootstrap), tests, intervals, multiple-testing adjustments.
- **Robustness**: alternative specifications, placebo/falsification checks, multiverse axes.
- **Reporting**: table selection, rounding, plot encoding choices, exported artefact format.
- **Plan references**: implementation, clarification, deviation, or amendment
  described in relation to a governing plan.

**Skip routine code**: I/O glue, formatting, test fixtures, dependency management, plotting cosmetics, file moves, lint-only edits.

#### What counts as one decision

A DADR records one **consequential analytic choice** — something a thoughtful peer reviewer or replicator would want explained. The bar is *reviewability*: if a defensible analyst could have chosen otherwise, and that other choice would plausibly change a number, an interpretation, or the scope of a claim, it's a decision.

Use this test:

- **Could it have gone another way?** If the only "alternative" is a typo or a tooling default with no semantic content, skip it.
- **Does it affect a result the project reports?** Trace from the choice to the numbers, figures, or claims in the manuscript or output. If nothing downstream depends on it, skip it.
- **Is it independently reviewable?** A reviewer should be able to accept, reject, or supersede this one choice without forcing decisions on adjacent choices. If two choices always travel together, they're one decision.

Examples that **are** decisions: "drop rows with missing treatment", "winsorize income at 1st/99th percentile", "cluster SEs at the school level", "use complete-case rather than multiple imputation", "primary outcome is the change-score", "report the bootstrap CI rather than the asymptotic one".

Examples that **are not** decisions: "use `dplyr` rather than `data.table`", "reorder columns before saving", "set `theme_minimal()` on plots", "name the dataframe `df`".

#### Granularity

One choice → one record. Start with at most five consequential choices unless the user requests another scope. A large project may contain many more choices; defer lower-priority ones to later review batches.

Do not split a choice to hit a count or merge distinct choices to fit a batch. Prioritize the choices a collaborator most needs explained, and report useful follow-up areas without drafting them all at once.

### Step 3 — Draft each candidate

For each consequential decision, use Dadrock's candidate-writing surface. **Do not hand-write the file** — Dadrock allocates the UUIDv7 and canonical filename.

#### Step 3a — Write via Dadrock

When running inside a Dadrock ACP task and the MCP tool `dadrock.create_candidate` is available, call it once for each candidate with `title`, `tags`, and `body_markdown`. This writes the file under `dadr/candidates/`. Do not merely return proposed candidates for the UI to write later.

When working outside MCP, use the equivalent CLI flow:

```bash
dadrock new "Drop observations with missing treatment assignment" \
  --tag kind/sample \
  --tag writer/agent \
  --quiet
```

- Title is a **noun phrase** describing the choice, not a question.
- Pass tags via repeated `--tag` flags (no `#` prefix in YAML/CLI).
- Use `--quiet` to keep output terse.
- Capture the printed file path — you will edit it next.

If you need the path programmatically, use `--json` instead of `--quiet` and parse the envelope:

```bash
dadrock new "..." --tag kind/sample --json
```

The path is in `data.path` (or `changed_paths[0]`).

#### Step 3b — Fill the four sections via Edit

The scaffold has empty `## Context`, `## Decision`, `## Alternatives`, `## Consequences`. Use the `Edit` tool to replace each section with substantive prose, grounded in evidence from the repo.

See [../record.md](../record.md) for the full anatomy: per-section length envelopes, the source-link convention, the active-voice rule for `Decision`, the "Not applicable" rule for `Alternatives`, the inline-tag pattern, and a worked example. The minimum viable record is ~200 words of body prose with all four sections; a record that grows past ~600 words is usually two decisions and should be split.

### Step 4 — Tags

Tags are optional. Follow the project's vocabulary and user preferences; inspect existing tags with `dadrock tags list`. When helpful, suggested defaults are `kind/sample`, `kind/modeling`, `kind/measurement`, `kind/transformation`, `kind/inference`, `kind/communication`, or `kind/robustness`, plus `writer/agent` for drafting provenance.

See [../tags.md](../tags.md) for syntax and additional optional namespaces. Do not require a new tag scheme to draft a useful decision.

### Step 5 — Validate and report

After drafting, run:

```bash
dadrock validate
```

Fix any reported errors (malformed tags, broken wikilinks, structural issues). Then summarize for the user:

- how many candidates were drafted, with one-line titles
- any consequential surfaces you deliberately skipped (and why)
- the next step: the analyst reviews each candidate and runs `dadrock status accept <query>` or `dadrock status reject <query>`

Mention that **acceptance is a human approval boundary** — the skill must never auto-accept.

## Constraints

- **Never overwrite existing records.** Before drafting, list current candidates with `dadrock list --status candidate` and skip any decision that is already covered.
- **No fabricated alternatives.** If you cannot identify a real alternative the analyst plausibly considered, say "Not applicable" or describe a generic methodological alternative honestly tagged `confidence/low`.
- **Do not edit accepted records.** This skill writes only to `dadr/candidates/`.

## Example end-to-end

User: "bootstrap DADR for this project"

1. `command -v dadrock` → ok.
2. `dadrock dir` → errors (no project). Run `dadrock init`.
3. Read `README.md`, glob `**/*.{R,py,qmd,Rmd}`, identify a SAP at `docs/sap.md`.
4. From `analysis/build_sample.R`: identify `df %>% filter(!is.na(treatment))` → candidate "Drop observations with missing treatment assignment", `kind/sample, writer/agent`.
5. `dadrock new "Drop observations with missing treatment assignment" --tag kind/sample --tag writer/agent --quiet` → writes `dadr/candidates/<uuid>-drop-observations-....md`.
6. Edit that file's four sections with grounded prose and a link back to `analysis/build_sample.R#L12-L18`.
7. Repeat for each consequential choice identified in the inventory.
8. `dadrock validate` → clean.
9. Report the count, the titles, and the next step: `dadrock list --status candidate`, then `dadrock status accept <query>` or `dadrock status reject <query>`.
