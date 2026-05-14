# Tool: Faithfulness — Manuscript

## Status

Future RFC. Dadrock does not currently expose a top-level
`dadrock faithfulness` command. Manuscript-faithfulness should be implemented
first as a task/report workflow unless a dedicated command becomes necessary.

## Purpose

Define how Dadrock checks the records against the **manuscript** (the paper, supplementary materials, slides, dashboard, or any other rendered output that reports the analysis to readers). This is the third of three faithfulness anchors — the others are plan ([12-faithfulness-plan.md](12-faithfulness-plan.md)) and code ([13-faithfulness-code.md](13-faithfulness-code.md)).

The intent is to surface mismatches between the analytic record and what the paper actually claims: methods that are described but not in the records, decisions that are in the records but absent from the methods section, and numerical claims whose underlying decisions are unrecorded.

## What the Check Answers

A future manuscript-faithfulness workflow answers:

- whether every analytic claim in the manuscript is backed by an accepted DADR (or is explicitly out of scope);
- whether every accepted DADR that should be visible to readers (sample definition, primary outcome, model, robustness checks) is mentioned in the manuscript;
- whether numerical claims (sample sizes, effect estimates, standard errors, p-values, model summaries) match what the latest accepted decisions imply;
- whether deviations from the plan are disclosed in the manuscript;
- whether superseded decisions are still cited in the manuscript by mistake.

## Possible Surface

The preferred first implementation is a task package that emits a
`diagnosis_report.v1` or future faithfulness report kind. A dedicated future CLI
could look like:

```bash
dadrock faithfulness --manuscript path/to/paper.qmd
dadrock faithfulness --manuscript path/to/paper.qmd --records dadr/
dadrock faithfulness --manuscript path/to/paper.qmd --report markdown
dadrock faithfulness --manuscript path/to/paper.qmd --report json
dadrock faithfulness --manuscript path/to/paper.qmd --fail-on undisclosed-deviation
```

The manuscript path can point to a single Quarto/Markdown file, a directory of source files, or a glob.

## Default Behavior

- compare the manuscript text against accepted records only;
- resolve any explicit references in the manuscript to DADR records (e.g., `[[<uuid7>]]` wikilinks in supplementary material, or footnotes citing record IDs);
- use local parsing and lexical checks for headings, captions, tables, and section structure before model-assisted comparison;
- use model-assisted comparison for prose claims (with the same `--local-only` / `--allow-remote-model` policy as the code check);
- never rewrite accepted records or the manuscript;
- never mark a claim faithful without human review when the underlying data are not unambiguously cited.

## Report Statuses

| Status | Meaning |
|---|---|
| `faithful` | Manuscript claim is consistent with the accepted records. |
| `unmentioned` | Accepted decision is consequential for the reader but not mentioned in the manuscript. |
| `undisclosed-deviation` | Manuscript reports an analysis that deviates from a plan-linked DADR without disclosing the deviation. |
| `contradiction` | Manuscript claim conflicts with an accepted decision (different sample, model, or metric). |
| `superseded-citation` | Manuscript cites a record that has been superseded; the current decision should be cited instead. |
| `unbacked-claim` | Manuscript reports an analytic choice (e.g., a winsorization threshold, an exclusion rule) that has no accepted DADR. |
| `unverified` | The check cannot determine consistency from the available records and source references. |

## Common Patterns

A few patterns that the check should recognize:

- **Methods-section paragraphs.** Each methodological statement in a `## Methods` section should map to one or more accepted DADRs.
- **Robustness-section claims.** Claims like "results are robust to..." should be backed by `#robustness/...`-tagged records.
- **Tables and figures.** Captions or footnotes that describe a model specification should cite the record that defines that specification.
- **Numerical sentences.** Sentences containing sample sizes, coefficients, or standard errors should be checked against the most recent accepted record for that estimand.

## Generated Report

```markdown
# DADR Manuscript-Faithfulness Report

## Summary

## Contradictions

## Undisclosed Deviations

## Unbacked Claims

## Unmentioned Decisions

## Superseded Citations

## Section-by-Section Results
```

Machine-readable reports should include the manuscript file path, the source line and surrounding sentence for each finding, the related record IDs, the status label, and a recommended follow-up action.

## Follow-Up Actions

The report's machine-readable form names a follow-up action per finding:

- add a methods-section sentence citing the relevant DADR;
- update a citation to point at the current (non-superseded) record;
- create a candidate DADR for an unbacked claim;
- disclose the plan deviation in the manuscript;
- mark the finding as a false positive with reviewer rationale.

## Limits

Manuscript-faithfulness depends on (a) the manuscript being available as parseable text and (b) accepted DADRs that name the analytic choices clearly enough to compare. For early-stage projects without a manuscript, the check is a no-op. For finished papers in PDF-only form, the check requires a separate text-extraction step.

The check is a review aid. It must not edit the manuscript, edit accepted records, or auto-resolve a finding without human approval.

## Relationship to Other Faithfulness Checks

- The **plan** check ([12-faithfulness-plan.md](12-faithfulness-plan.md)) verifies coverage and approval workflow against the governing plan.
- The **code** check ([13-faithfulness-code.md](13-faithfulness-code.md)) verifies that the code matches the records.
- The **manuscript** check (this spec) verifies that the rendered output matches the records.

The three checks are independent and could be run separately or together in a
future combined workflow. Manuscript-faithfulness usually runs late in a
project, after the manuscript text exists; code-faithfulness runs continuously;
plan-faithfulness runs from the start.
