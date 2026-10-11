---
id: edit-record
title: "Edit record"
description: "Revise any existing DADR while preserving its lifecycle status until you choose what to do next."
version: 1
inputs:
  - name: record_id
    label: "Record"
    type: text
    required: true
  - name: requested_changes
    label: "Requested changes"
    type: textarea
    required: true
requirements:
  records: one
  record_status: all
  paths: none
  notes: required
  scope: none
output:
  kind: record_revision.v1
mcp_access: full
tools:
  - name: dadrock.get_record
    mode: read
---

# Edit record

Revise an existing DADR as a reviewer requests while preserving its choice, its evidence, and its lifecycle status. A revision sharpens or corrects how the decision is described; it does not replace the decision.

## Inputs

- `record_id`: the record to revise.
- `requested_changes`: what the reviewer wants changed.

Interactively, these come from the user's request. "Make it better" is not specific enough: find out which sections should change, and how.

## What may change

- **Candidates** may be revised freely, as long as they still record the same choice.
- **Accepted records**, superseded or not, allow only changes that keep their meaning: spelling, formatting, links, citations, and tags. If the request would change what an accepted record decides or why, do not apply that part; say in the warnings that it needs a replacement record ([supersede-record](supersede-record.md)).
- **Rejected records** are closed. Limit changes to the same meaning-preserving corrections; revisiting a rejected idea means drafting a new candidate.

Never change `id`, `status`, or `successors`.

## What to preserve

- **The choice itself.** If a request for a candidate amounts to a different decision, apply it faithfully and warn that the result reads as a different decision.
- **Valid evidence.** Keep source links that still resolve. Update line ranges that have shifted; remove links that no longer exist and say so.
- **Alternatives.** Do not silently drop an alternative the original named. If you remove one, explain why.
- **Tags already in use.** Add tags when they apply, but keep `kind/` and `writer/` tags unless the request explicitly changes them.
- **Other sections**, such as `## Replaces` or `## Robustness`, unless the request targets them.

## Section conventions

All four sections stay present and follow the [record reference](../record.md): a Context with at least one source link, an active-voice Decision, Alternatives (or "Not applicable", never empty), and Consequences quantified where possible. Keep the body around 200–400 words; past ~600 words, warn that the record may now bundle two decisions.

Avoid the usual pitfalls: tense drift in the Decision ("we will", "we plan to"), question-form titles, emptied Alternatives, source links you cannot verify, and new rationale the evidence does not support. If the request would stretch the record beyond one choice, apply the narrow change and flag the scope concern.

## Constraints

- Revise only the named record.
- Never accept, reject, or supersede it. Those are the reviewer's next steps.

## Result

A revision of the record:

- **Summary**: one sentence on what changed.
- **Record id** of the revised record.
- **Title**, only if it changed.
- **Tags**, only if they changed, as the complete new list.
- **Body**: the complete revised body, with all four sections and any other sections it had.
- **Change explanation**: section by section, which sentences, links, or alternatives were added, changed, or removed, and why.
- **Warnings**: unresolved ambiguity, requested changes that would alter the decision, removed evidence, or length concerns.
