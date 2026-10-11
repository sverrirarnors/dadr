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
tools:
  - name: dadrock.project_snapshot
    mode: read
---

# Code to records

Find the consequential analytic choices in an existing analysis project and draft one candidate DADR for each, so the analyst can review, complete, and accept or reject them. This task is mainly for bringing an existing project under DADR. When the user already knows which decision to capture, [context-to-record](context-to-record.md) fits better.

## Inputs

- `path`: the file or directory to inspect. Defaults to `.`, the project root.
- `focus`: optional topic to concentrate on, such as "missing data" or "standard errors".
- `limit`: the maximum number of candidates. Defaults to 5.

Interactively, these come from the user's request; use the defaults for anything not given.

## Survey the scope

Understand the analysis before judging its choices. Read the README and any plan-like documents (statistical or pre-analysis plans, preregistrations, protocols, data-cleaning plans) for what the study is about, even when they lie outside `path`. Then read the analysis code within `path` (scripts, notebooks, Quarto or R Markdown files, SQL, pipeline definitions), the manuscript if there is one, and the existing records in `dadr/`, so you do not propose choices that are already recorded. Search broadly first, then read the files that look load-bearing. Skip code marked `#dadr/ignore`.

## What is decision-worthy

A DADR records one **consequential analytic choice**: something a thoughtful peer reviewer or replicator would want explained. Apply this three-part test:

- **Could it have gone another way?** If the only "alternative" is a typo or a tooling default with no semantic content, skip it.
- **Does it affect a reported result?** Trace the choice to the numbers, figures, or claims the project produces. If nothing downstream depends on it, skip it.
- **Is it independently reviewable?** A reviewer should be able to accept, reject, or supersede this one choice without forcing decisions on adjacent choices. If two choices always travel together, they are one decision.

Look for choices on these surfaces:

- **Sample construction** (`kind/sample`): inclusion and exclusion rules, dropping missing rows, deduplication, joins and linkage, time windows, geographic or cohort filters.
- **Variable definitions** (`kind/measurement`): outcome, treatment or exposure, covariates, constructs, derived scores, recoding, scale composition.
- **Transformations** (`kind/transformation`): log or square-root transforms, winsorizing, standardizing, imputation rules, top-coding, categorization, thresholds and cut points.
- **Model and estimator** (`kind/modeling`): formula, family and link, fixed or random effects, priors, hyperparameters, resampling design.
- **Inference** (`kind/inference`): standard errors (HC*, clustered, bootstrap), tests, intervals, multiple-testing adjustments.
- **Robustness** (`kind/robustness`): alternative specifications, placebo or falsification checks, multiverse axes.
- **Reporting** (`kind/communication`, `kind/visualization`, `kind/output`): table selection, rounding, plot encodings, exported artefact formats.
- **Plan relation**: implementation, clarification, deviation, or amendment relative to a governing plan.

Skip routine code: I/O glue, formatting, test fixtures, dependency management, plotting cosmetics, file moves, lint-only edits, variable naming, and package choices without methodological consequence.

These are decisions: "drop rows with missing treatment", "winsorize income at the 1st/99th percentile", "cluster standard errors at the school level", "use complete cases rather than multiple imputation", "the primary outcome is the change score", "report the bootstrap interval rather than the asymptotic one".

These are not: "use `dplyr` rather than `data.table`", "reorder columns before saving", "apply `theme_minimal()` to plots", "name the data frame `df`".

## Granularity

One choice, one candidate. Propose at most `limit` candidates (five by default), ordered by likely value to a collaborator. Do not split choices to fill the limit, and do not merge distinct choices to fit it. When unsure whether two choices are independently useful, prefer one coherent decision. Skip choices that existing records already cover. Name the areas you deferred so the user can ask for a larger limit or a focused follow-up.

Prefer fewer high-signal candidates over broad speculation. Returning zero candidates is correct when the scope contains no decisions worth recording.

## Drafting each candidate

Write each candidate to the [record reference](../record.md): a noun-phrase title and the four sections. In addition:

- **Anchor every claim to the code.** Context links the specific lines (`analysis/clean.R:42-57`), not whole files; a choice that spans call sites gets several links.
- **Never infer motivation from code alone.** When the source does not explain why a choice was made, write the exact phrase "Rationale not documented" in Context, repeat it in the warnings, and ask the reviewer to supply the reason.
- **Say when the Decision is read from code** rather than transcribed from a plan ("The code at L12-L18 implies…").
- **Real alternatives only.** Use alternatives named in plans, comments, or the code itself. If none is identifiable, write "Not applicable" rather than inventing one.
- **Tags.** Give every candidate `writer/agent` and one `kind/` tag: the closest value in the project's existing vocabulary or in the [tag reference](../tags.md), such as `kind/sample`, `kind/modeling`, or `kind/inference`. Add `topic/`, `plan/`, `confidence/`, `axis/`, `multiverse/`, or `robustness/` tags when they apply. Suggest `people/decider/` or `people/approver/` tags only when the evidence names those people.

## Constraints

- Do not change existing records; this task proposes new candidates only.
- Never accept, reject, or supersede anything. Acceptance is the analyst's decision.

## Result

A batch of candidates with a summary:

- **Summary**: what was inspected (paths and focus), what was skipped, and which areas were deferred and why.
- **Candidates**: at most `limit`, ordered by value. Each has a title, tags, a body with the four sections, source references, a confidence level, and warnings.
  - **Source references**: specific files with line ranges, matching the links in Context.
  - **Confidence**: `high` only when the choice is unambiguous in the code and clearly consequential; `medium` or `low` when the code permits several readings or the downstream impact is uncertain.
  - **Warnings**: thin evidence, "Rationale not documented", doubt about whether the choice was deliberate, or likely overlap with an existing record.
