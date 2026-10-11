# Get started

This page walks through installing **dadrock**, starting a new DADR project, and onboarding an analysis that already has substantial code, plans, or prose. DADR can be added to any project under version control without reorganizing it.

## Install

Prebuilt binaries (when available) install with a single command. From source:

```bash
cargo install dadrock
```

Confirm the binary is on `PATH`:

```bash
dadrock --version
```

## New projects { #new-projects }

### Initialize

From your project root:

```bash
dadrock init
```

This creates:

- `dadr/accepted/` — where accepted Markdown decision records will live;
- `dadr/candidates/` — where active candidate records will live;
- `dadr/rejected/` — where rejected records will live;
- `dadr/templates/`, `dadr/tasks/`, and `dadr/runs/` — project-local templates, task customizations, and ignored task run state.

`dadrock init` is idempotent: re-running never overwrites an existing record, template, or task file.

### Demo

`dadrock demo PATH` is a sibling command that seeds a named example project.
Unlike `init`, demo is intentionally not idempotent: it requires a new or empty
target directory.

```bash
dadrock demo --example lifecycle PATH   # default: DADR lifecycle showcase
dadrock demo --example candidate-review PATH # focused candidate review queue
dadrock demo --example penguins PATH    # realistic Python analysis for code-to-record
dadrock demo --example broken PATH      # validation repair practice
dadrock demo --example conflicts PATH   # conflict and merge practice
dadrock demo --example task-history PATH # persisted task result showcase
```

The `lifecycle` example is the compact spec showcase with candidate, accepted,
rejected, superseded, split, and consolidation examples. The `candidate-review`
example keeps the ledger small and fills the candidate queue with accept,
reject, edit, and low-confidence review cases. The `penguins` example writes a
self-contained `uv` Python script that downloads Palmer Penguins data from
Rdatasets when run separately, subsets species, creates a dummy variable, and
fits a statsmodels OLS model with a formula. The `task-history` example seeds
successful persisted task runs for the important typed result renderers. Dadrock
does not run Python or download data while seeding the demo.

### Version-control hooks

Git hooks are future work. The intended hook workflow is an assistive guardrail:
inspect staged diffs, flag consequential analysis changes, and optionally draft
candidate DADRs for human review. It must not accept records, block ordinary
commits by default, or silently rewrite accepted DADR prose.

#### What counts as consequential

A change is consequential when it plausibly affects:

- the analysis sample;
- outcome / treatment / exposure / covariate / construct definitions;
- transformations, imputations, coding, derived scores;
- joins, deduplication, linkage rules;
- model formula, estimator, prior, hyperparameter, resampling design;
- standard errors, tests, intervals, validation metrics, multiple-testing adjustments;
- thresholds, cut points, inclusion rules, missingness handling;
- robustness axes or specification families;
- reporting, rounding, table selection, plot encoding, exported artefacts;
- implementation, clarification, amendment, or deviation from a governing plan.

Routine changes should not create candidates by default: whitespace, formatting,
lint-only edits, variable renames with no semantic change, dependency import
cleanup, file moves recovered by rename detection, test fixture churn,
generated-artefact timestamps, and refactors that preserve parsed operations and
outputs. Routine changes may still become candidates when a source annotation,
commit message, plan reference, or output difference indicates decision
relevance.

## Existing projects { #existing-projects }

DADR can be bolted onto a project that already has substantial code, plans, and
prose. Current Dadrock does this through task packages and local ACP agents
rather than a dedicated `infer` command.

### Task-Based Drafting

Use the local web UI or CLI task surface to draft candidate records from
existing material:

```bash
dadrock task list
dadrock task show code-to-record
dadrock task run code-to-record --input-json '{"path":"analysis/"}'
```

Useful built-in tasks include:

| Task | Use it for |
|---|---|
| `context-to-record` | Turn supplied context into one candidate DADR. |
| `code-to-record` | Scan selected source paths and propose candidate records. |
| `find-conflicts` | Look for duplicate, contradictory, or overlapping records. |
| `merge-records` | Consolidate overlapping candidates into one candidate. |
| `supersede-record` | Draft a replacement candidate for an accepted record. |

Task outputs are proposals. A human still accepts, rejects, merges, or
supersedes records through normal lifecycle commands.

### Code-tag scan

Authors can annotate analysis code directly with `#dadr/...` tags in source
comments. The convention is documented in the [format spec](specification.md#code-tags),
but Dadrock does not yet scan source comments or validate code-tag references.

`#dadr/<short-id>` tags link surrounding code to a specific record.
`#dadr/kind/...`, `#dadr/topic/...`, `#dadr/multiverse/...`, etc. classify the
surrounding code without naming a specific record. Future inference,
faithfulness, and tag-hygiene tools may use these annotations as signals.

### Faithfulness

Faithfulness checks are future work. Once a stable trace of accepted records
exists, these checks will compare records to plans, code, and manuscripts and
report discrepancies for human triage.

| Check | What it compares |
|---|---|
| Plan faithfulness | Records vs. SAP / PAP / preregistration / cleaning plan |
| Code faithfulness | Records vs. code, configuration, notebooks, workflow files |
| Manuscript faithfulness | Records vs. manuscript or rendered output |

Each check should be read-only. Each should produce a structured discrepancy
report and list candidate fixes. None should mutate records.

#### Plan faithfulness

Compares accepted DADRs against a governing-plan artefact (Statistical Analysis Plan, Pre-Analysis Plan, preregistration, study protocol, data cleaning plan).

- parses plan headings and numbered items;
- maps each item to zero or more accepted records via `#plan/<id>` tags and
  plan links in record bodies;
- flags **plan items with no implementing record** (gaps);
- flags **records that reference plan items that do not exist** (orphans);
- flags **records that contradict a plan item** (drift);
- separates documented **deviations** from gaps and orphans, since they are an
  expected part of analysis.

Each deviation must include a `## Plan deviation` section with reason, impact assessment, and reviewer (validation enforces this; see [**dadrock**](dadrock/dadrock.md)).

#### Code faithfulness

Compares accepted DADRs against the code, configuration, notebooks, and workflow files that implement the analysis.

- builds the set of accepted records;
- builds the set of analytic activities present in the source tree (using the project's tag vocabulary and conventions described in [Tags](tags.md));
- maps each activity to zero or more records via `#dadr/<uuid>` code tags, structural matches, and tag overlap;
- flags **activities with no governing record** (gaps);
- flags **records with no implementing activity** (orphans);
- flags **records that contradict the activity** (drift) — for example, a record specifying a complete-case rule when the code path implements multiple imputation;
- flags **stale `#dadr/<uuid>` code tags** that point at non-existent records.

Code faithfulness is expected to be the largest mode in practice. It will be
most useful immediately after major refactors, before submission, and on every
PR via CI.

#### Manuscript faithfulness

Compares accepted DADRs against the paper, supplementary materials, dashboard, or other rendered output.

- parses methods/results section headings and per-paragraph claims;
- detects numerical claims and identifies the analytic choices they imply;
- detects "we did X" sentences (excluded, winsorized, imputed, fitted, clustered) and checks whether each has a matching DADR;
- detects robustness sentences and checks whether each robustness check has a matching record;
- flags **manuscript claims with no governing record** (gaps);
- flags **records that contradict a manuscript claim** (drift);
- flags **boilerplate prose** ("we used standard methods") as a `#confidence/low` candidate signal rather than a hard gap.

#### Discrepancy report

A faithfulness report has a consistent shape across the three modes:

```text
Faithfulness: code (analysis/)

  ─ gaps
    [analysis/build_sample.py:42-58] sample-filter activity has no record
    [analysis/build_sample.py:71]    deduplication activity has no record

  ─ orphans
    [01938c20a3b4] references analysis/build_sample.py:88 (file deleted)

  ─ drift
    [01938c1f9d12] specifies complete-case; code path implements multiple imputation
                   (analysis/impute.py:14)

  ─ deviations
    [01938c30b1f0] #plan/SAP-2.1
                   reviewer: jdoe; impact: documented in body

7 issues found (2 gaps, 1 orphan, 1 drift, 1 deviation, 2 informational)
```

The report is grouped by category, severity-labelled, and link-rich.

Faithfulness checks are diagnostic, not authoritative. The records remain the
canonical statement of what the analysis decides; the artefacts are checked
*against* them. When a check reports drift, the right response is usually to
draft a new candidate, accept it, and link it as superseding the old accepted
decision, not to silently rewrite the prior record. See the immutability rules
in the [DADR specification](specification.md#immutability).

## What to do next

- Read the [DADR specification](specification.md) so the four sections and the candidate/accepted/superseded lifecycle feel natural.
- Browse the [**dadrock**](dadrock/dadrock.md) section for the binary, CLI, web UI, and IDE integrations.
- Skim the [Tags](tags.md) page for the recommended vocabulary (`#kind/`, `#confidence/`, `#multiverse/`, `#people/`).
