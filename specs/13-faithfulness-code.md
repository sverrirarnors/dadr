# Tool: Faithfulness — Code

## Status

Future RFC. Dadrock does not currently expose a top-level
`dadrock faithfulness` command. Code-faithfulness should be implemented first as
a task/report workflow that reuses existing link validation, task context, and
agent infrastructure.

## Purpose

Define how Dadrock checks the records against the **code, configuration, notebooks, and workflow files** that implement the analysis. This is the second of three faithfulness anchors — the others are plan ([12-faithfulness-plan.md](12-faithfulness-plan.md)) and manuscript ([14-faithfulness-manuscript.md](14-faithfulness-manuscript.md)).

The intent is to surface drift: code that no longer matches an accepted decision, decisions whose implementation is missing, and consequential code that has no DADR at all.

## What the Check Answers

A future code-faithfulness workflow answers:

- whether the implemented code still appears to match accepted decisions;
- whether accepted decisions lack current implementation references;
- whether code implements choices that contradict accepted DADRs;
- whether code implements consequential choices with no accepted DADR;
- whether the links cited in records still resolve to existing code, data, or plans;
- whether superseded decisions appear to be implemented accidentally.

## Possible Surface

The preferred first implementation is a task package that emits a
`diagnosis_report.v1` or future faithfulness report kind. A dedicated future CLI
could look like:

```bash
dadrock faithfulness --code
dadrock faithfulness --code 01938b8e-4d70-7abc-8def-0123456789ab
dadrock faithfulness --code --code-path analysis/ --records dadr/
dadrock faithfulness --code --report markdown
dadrock faithfulness --code --report json
dadrock faithfulness --code --write-report
```

Common options:

```bash
dadrock faithfulness --code --include-candidates
dadrock faithfulness --code --include-superseded
dadrock faithfulness --code --fail-on contradiction
dadrock faithfulness --code --local-only
dadrock faithfulness --code --allow-remote-model
dadrock faithfulness --code --max-records 25
dadrock faithfulness --code --max-context-lines 80
dadrock faithfulness --code --json
```

## Default Behavior

- compare accepted records only;
- resolve the links cited in each record (Markdown links and `#dadr/...` code-comment tags) before model-assisted comparison;
- inspect linked code ranges and nearby context;
- inspect requested code paths when supplied;
- use local parsing and lexical checks before model-assisted comparison;
- never rewrite accepted records;
- never mark a decision compliant without human review when source context is ambiguous.

## Report Statuses

| Status | Meaning |
|---|---|
| `faithful` | Code context appears to implement the accepted decision. |
| `partial` | Some source context matches, but implementation appears incomplete. |
| `unverified` | Source context is missing, ambiguous, or outside the scanned source tree. |
| `contradiction` | Current code appears to conflict with an accepted decision. |
| `undocumented` | Code appears to implement a consequential decision with no accepted DADR. |
| `broken_link` | The decision references a code path or symbol that no longer resolves. |

## Generated Report

```markdown
# DADR Code-Faithfulness Report

## Summary

## Contradictions

## Partial Implementations

## Unverified Decisions

## Undocumented Consequential Choices

## Broken Source Links

## Record-by-Record Results
```

Machine-readable reports should include record IDs, source locators (file path and line range), status labels, confidence signals, and recommended follow-up actions.

## Follow-Up Actions

The report's machine-readable form names a follow-up action per finding:

- re-resolve links and re-run the check;
- create a candidate DADR for undocumented code;
- create a superseding DADR for changed implementation;
- update the code to match the accepted DADR;
- mark the report item as a false positive with reviewer rationale.

## Limits

Code-faithfulness is a review aid. It must not edit accepted DADRs, approve deviations, or mutate locked plans. Ambiguous findings stay as `unverified`; the tool surfaces them for a human to triage rather than guessing.

The check operates on the current working tree. It does not analyze historical commits except as needed to recover renamed files. Comparing accepted records with an earlier revision is a separate, implemented check, `dadrock validate changes`.

## Relationship to Other Faithfulness Checks

- The **plan** check ([12-faithfulness-plan.md](12-faithfulness-plan.md)) verifies coverage and review workflow against the governing plan. It does not look at code.
- The **code** check (this spec) verifies that the code matches the records. It does not look at the plan or the manuscript.
- The **manuscript** check ([14-faithfulness-manuscript.md](14-faithfulness-manuscript.md)) verifies that what the paper or rendered output reports is consistent with the records. It does not look at code.

The three checks are independent and could be run separately or together in a
future combined workflow.
