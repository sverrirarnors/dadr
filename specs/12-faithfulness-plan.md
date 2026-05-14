# Tool: Faithfulness — Plan

## Status

Future RFC. Dadrock does not currently expose a top-level
`dadrock faithfulness` command. Plan-faithfulness should be implemented first as
a task/report workflow unless a dedicated command becomes necessary.

## Purpose

Define how Dadrock checks records against the **governing plan**. This is the
first of three faithfulness anchors — the others are code
([13-faithfulness-code.md](13-faithfulness-code.md)) and manuscript
([14-faithfulness-manuscript.md](14-faithfulness-manuscript.md)).

## Plans, Briefly

A "plan" is any external artifact that fixes analytic choices in advance:

- **SAP** (Statistical Analysis Plan) — specifies statistical analysis before
  analysis lock or final analysis.
- **PAP** (Pre-Analysis Plan) — specifies hypotheses, outcomes, models, and
  analyses before outcome data access.
- **Pre-registration** — public or private timestamped registration of
  hypotheses and plans.
- **Study protocol** — study-design document approved before execution.
- **Data cleaning plan** — fixes data-preparation choices before cleaning.

DADR does not define a format for plan documents. They live wherever the
project keeps them. A DADR record references a plan with a `plan/<id>` tag and
links to the relevant plan section in the body. The relationship to the plan is
explained in prose, not by a separate relationship tag.

Records that deviate from a plan should include a `## Plan deviation` body
section:

```markdown
## Plan deviation

Planned choice:

Actual choice:

Reason:

Expected impact:

Reviewer approval:
```

Records that amend a plan should link to the approved amendment or review event.

## What the Check Answers

A future plan-faithfulness workflow answers:

- which plan sections are implemented by accepted DADRs;
- which plan sections lack any implementing DADR;
- which records document plan deviations and whether each has reviewer
  approval;
- which records document plan amendments and whether the amendment artifact
  exists and is approved;
- whether locked plan text has been edited outside the amendment workflow.

## Possible Surface

The preferred first implementation is a task package that emits a
`diagnosis_report.v1` or future faithfulness report kind. A dedicated future CLI
could look like:

```bash
dadrock faithfulness --plan
dadrock faithfulness --plan PAP-2026-001
dadrock faithfulness --plan --report markdown
dadrock faithfulness --plan --report json
dadrock faithfulness --plan --fail-on unresolved-gap
```

## Plan-To-DADR Trace

The trace report shows:

- plan sections;
- DADRs that implement each section;
- DADRs that clarify, amend, or deviate from each section;
- deviation approval status;
- unresolved plan gaps;
- robustness checks required by the plan.

The report is generated from `plan/...` tags, body links to plan sections, plan
headings, and validated local paths. It does not require manual duplication of
the plan.

## Review Workflow

Plan-linked DADRs follow the normal candidate lifecycle. The tool should flag
additional review requirements when body prose indicates a deviation or
amendment:

| Case | Additional review requirement |
|---|---|
| Implements plan section | Normal DADR review. |
| Clarifies plan section | Normal review, with the governing plan visible. |
| Amends plan section | Requires amendment approval before acceptance, or as part of the same acceptance action. |
| Deviates from plan section | Requires explicit reviewer approval and impact assessment. |

The web UI (see [07-agent-and-app-architecture.md](07-agent-and-app-architecture.md))
should show the governing plan beside the candidate DADR when a `plan/...` tag
or plan link is present.

## Validation Rules

Plan-faithfulness validation checks:

- referenced plan IDs in `plan/...` tags exist in local plans or registry
  metadata;
- records that include a `Plan deviation` section include planned choice,
  actual choice, reason, expected impact, and reviewer approval;
- amendment records link to an amendment artifact;
- locked plans have not been edited outside the amendment workflow;
- plan trace reports have no unresolved required sections before final release,
  when project policy requires that.

## Agent Responsibilities

The agent may:

- detect candidate DADRs from SAP/PAP/pre-registration text;
- suggest links from a candidate DADR to relevant plan sections;
- flag possible deviations from plan text;
- draft deviation explanations;
- summarize unresolved plan gaps.

The agent must not:

- mark a deviation as approved;
- mutate locked plan text silently;
- infer compliance when the plan is ambiguous;
- replace human review of amendments or deviations.

## Possible Reports

Future report outputs could include:

- `plan-trace.qmd`: governing plans mapped to DADRs.
- `deviations.qmd`: all deviations, reasons, impact assessments, and approvals.
- `amendments.qmd`: amendments and supersession chains.
- `unresolved-plan-gaps.qmd`: planned analyses without linked implementation
  DADRs.

## Limits

The plan-faithfulness check is about coverage and approval workflow, not
semantic validation of analytic content. It should not second-guess whether a
record that says it implements a plan item really does so in code; that belongs
to code-faithfulness or manuscript-faithfulness, depending on the implementation
surface.
