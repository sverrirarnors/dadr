---
id: decisions-to-spec
title: "Decisions to spec"
description: "Synthesize accepted DADRs into an implementation, analysis, or reporting spec for downstream work."
version: 1
inputs:
  - name: focus
    label: "Spec focus"
    type: textarea
    required: false
requirements:
  records: none
  record_status: accepted
  paths: none
  notes: optional
  scope: accepted
output:
  kind: markdown_report.v1
tools:
  - name: dadrock.list_records
    mode: read
---

# Decisions to spec

Synthesize the project's accepted decisions into a written specification (an analysis plan, an implementation plan, a methods or reporting spec) for downstream code, analysis, or writing to follow. The decisions remain the source of truth; the spec is derived documentation.

## Inputs

- `focus`: optional; the kind of spec wanted and any subset of decisions to cover, such as "reporting spec", "sample construction only", or "decisions tagged `plan/sap-2026`". Without it, write a general analysis spec covering all current decisions.

Interactively, this comes from the user's request.

## Which decisions

Use accepted records that have not been superseded; a superseded decision no longer binds; its successors do. Ignore candidates and rejected records. If there are no accepted records, say so and stop rather than drafting from candidates.

From each decision, take:

- the **Decision**: the binding requirement the spec must preserve;
- the **Context**: the sources, datasets, plans, or prior records behind it;
- the **Alternatives**: rejected options the spec must not reintroduce;
- the **Consequences**: downstream obligations, checks, and reporting caveats;
- the **tags**: grouping signals such as `kind/`, `topic/`, and `plan/`.

## Organize

Group requirements by the surface the spec needs. A typical analysis spec has:

| Section | Decisions about |
|---|---|
| Sample and data construction | inclusion rules, cohort filters, joins, exclusions (`kind/sample`) |
| Variables and measurement | outcome and treatment definitions, recodes, scales |
| Transformations | imputation, winsorization, rescaling |
| Models and estimands | estimands, formulas, fixed effects, priors (`kind/modeling`) |
| Inference | standard errors, intervals, multiplicity (`kind/inference`) |
| Robustness and sensitivity | alternative specifications, `axis/` and `robustness/` choices |
| Reporting | tables, figures, claims |
| Deviations from prior plans | plan deviations and amendments (`plan/`) |

Follow the project's own tags where they exist, and adapt the sections to `focus`.

## Draft

Unless `focus` asks for another format, use this structure:

1. **Purpose and scope**: what the spec covers and which decisions it includes.
2. **Decision-derived requirements**, grouped as above. Write each requirement in active voice and cite the records behind it with a record link (`[[<short-id>]]`) and title.
3. **Constraints and non-goals**: rejected alternatives that must not be implemented or reported as current choices.
4. **Downstream checks**: validation, reruns, robustness checks, and manuscript updates implied by the Consequences.
5. **Open questions**: decisions too vague to turn into a precise requirement, conflicts between decisions, and missing source links.

Before finishing, check that every included decision appears in a requirement or under Open questions, and that no rejected alternative reads as an active requirement.

## Constraints

- Accepted decisions are binding. Do not soften or reinterpret a Decision to make the spec easier to write, and do not add requirements no decision states.
- Alternatives stay alternatives: mention them only as non-goals.
- If two accepted decisions prescribe incompatible requirements, do not choose between them. List the conflict under Open questions; resolving it takes a successor that supersedes them.
- The spec does not change any record.

## Result

A Markdown report:

- **Title**: the spec's name.
- **Summary**: one or two sentences giving the number of decisions covered and the number of open questions.
- **Body**: the spec itself.
