---
id: edit-record
title: "Edit record"
description: "Revise a candidate DADR before it is accepted or rejected."
version: 2
inputs:
  - name: record_id
    label: "Candidate"
    type: text
    required: true
  - name: requested_changes
    label: "Requested changes"
    type: textarea
    required: true
requirements:
  records: one
  record_status: candidate
  paths: none
  notes: required
  scope: none
output:
  kind: record_revision.v1
tools:
  - name: dadrock.get_record
    mode: read
---

# Edit record

Revise a candidate DADR as a reviewer requests while preserving its choice and its evidence. The candidate stays a candidate; only its title, tags, or body change. A revision sharpens or corrects how the decision is described; it does not switch to a different decision.

## Inputs

- `record_id`: the candidate to revise.
- `requested_changes`: what the reviewer wants changed.

Interactively, these come from the user's request. "Make it better" is not specific enough: find out which sections should change, and how.

## Candidates only

Only candidates are revised. Agents never edit accepted or rejected records, superseded or not, and the `edit-record` task refuses them before it runs:

- For an **accepted** record, a person corrects its wording in place when the meaning stays the same (spelling, formatting, links, citations). To change what it decides, draft a successor with [supersede-record](supersede-record.md).
- For a **rejected** record, revisiting the idea means drafting a new candidate, for example with [context-to-record](context-to-record.md).

Never change `id`, `status`, or `successors`.

## What to preserve

- **The choice itself.** If the request amounts to a different decision, apply it faithfully and warn that the result reads as a different decision.
- **Valid evidence.** Keep source links that still resolve. Update line ranges that have shifted; remove links that no longer exist and say so.
- **Alternatives.** Do not silently drop an alternative the original named. If you remove one, explain why.
- **Tags already in use.** Add tags when they apply, but keep `kind/` and `writer/` tags unless the request explicitly changes them.
- **Other sections**, such as `## Supersedes` or `## Robustness`, unless the request targets them.

## Section conventions

All four sections stay present and follow the [record reference](../record.md): a Context with at least one source link, an active-voice Decision, Alternatives (or "Not applicable", never empty), and Consequences quantified where possible. Keep the body around 200–400 words; past ~600 words, warn that the record may now bundle two decisions.

Avoid the usual pitfalls: tense drift in the Decision ("we will", "we plan to"), question-form titles, emptied Alternatives, source links you cannot verify, and new rationale the evidence does not support. If the request would stretch the record beyond one choice, apply the narrow change and flag the scope concern.

## Constraints

- Revise only the named candidate.
- Never accept, reject, or supersede it. Those are the reviewer's next steps.

## Result

A revision of the candidate:

- **Summary**: one sentence on what changed.
- **Record id** of the revised candidate.
- **Title**, only if it changed.
- **Tags**, only if they changed, as the complete new list.
- **Body**: the complete revised body, with all four sections and any other sections it had.
- **Change explanation**: section by section, which sentences, links, or alternatives were added, changed, or removed, and why.
- **Warnings**: unresolved ambiguity, requested changes that would alter the decision, removed evidence, or length concerns.
