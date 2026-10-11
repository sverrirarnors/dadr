---
id: supersede-record
title: "Supersede record"
description: "Draft a replacement candidate for an accepted DADR when the project decision has changed."
version: 1
inputs:
  - name: record_id
    label: "Record to supersede"
    type: text
    required: true
  - name: reason
    label: "Reason"
    type: textarea
    required: true
requirements:
  records: one
  record_status: accepted
  paths: none
  notes: required
  scope: none
output:
  kind: supersession_proposal.v1
mcp_access: full
tools:
  - name: dadrock.get_record
    mode: read
---

# Supersede record

Draft a replacement for an accepted DADR whose decision has changed. DADR never rewrites an accepted decision: the predecessor stays as history, and the replacement is a new candidate. Once a human accepts the replacement, dadrock adds it to the predecessor's `successors`.

## Inputs

- `record_id`: the accepted record to replace.
- `reason`: why the decision changed, such as new evidence, a plan amendment, a conflict with another decision, or a clarified scope.

Interactively, these come from the user's request. "I changed my mind" is a valid reason, but state it concretely.

The predecessor must be accepted and not yet superseded. If it has already been replaced, its current successor is the decision to replace; say so rather than drafting.

## What carries over

- **Valid rationale.** Reasons in the old Context, Alternatives, or Consequences that still hold, reshaped for the new decision rather than copied.
- **Source links** that still resolve, with updated line ranges.
- **Tags** that still apply (`kind/`, `topic/`, `plan/`), plus `writer/agent`. Drop tags that no longer fit.

Do not silently discard evidence or alternatives from the old record: carry them over, or explain in the rationale why they no longer apply.

## What must change

Write the replacement to the [record reference](../record.md), with a fresh noun-phrase title, and:

- **Context** names the predecessor with a record link (`[[<short-id>]]`), states what triggered the change, and cites the source that motivates it (a file with a line range, a plan section).
- **Decision** states the new choice in active voice and makes the difference from the predecessor explicit.
- **Alternatives** usually include the predecessor's choice as a rejected alternative, linked again, with a one-line reason. Add other real alternatives, or write "Not applicable".
- **Consequences** cover downstream effects, any new robustness obligation, and prior results that must be re-run.
- A final **`## Replaces`** section holds exactly one line: a link to the predecessor's full UUID, `[[<full-predecessor-uuid>|Previous decision]]`. It records the intended replacement while the record is a candidate; dadrock validates it and links the two records only when a human accepts the replacement.

Keep the body around 200–400 words. Do not invent rationale beyond the stated reason and the sources.

## Constraints

- One predecessor, one replacement. Two decisions that both need replacing get two replacements.
- Never propose edits to the predecessor's body, and never write `successors` by hand.
- Never accept the replacement or supersede the predecessor. Those are the reviewer's actions.

## Result

A supersession proposal:

- **Summary**: one sentence framing the proposal.
- **Superseded record id**: the predecessor's full id.
- **Title** of the replacement: a noun phrase, not a question.
- **Tags** of the replacement: the predecessor's tags that still apply, plus `writer/agent`.
- **Body** of the replacement: the four sections plus `## Replaces`.
- **Rationale**: one to three sentences, for the reviewer, on why the predecessor needs replacing. This is separate from the body.
- **Suggested actions**: ordered next steps for the reviewer, such as reviewing and accepting the replacement and re-running the analyses its Consequences name. Accepting the replacement links the predecessor, so do not add a separate supersede step.
