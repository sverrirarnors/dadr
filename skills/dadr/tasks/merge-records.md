---
id: merge-records
title: "Merge records"
description: "Combine overlapping candidate DADRs into one clearer candidate while leaving source records unchanged."
version: 1
inputs:
  - name: record_ids
    label: "Candidate records"
    type: textarea
    required: true
  - name: merge_goal
    label: "Merge goal"
    type: textarea
    required: false
requirements:
  records: many
  record_status: candidate
  paths: none
  notes: optional
  scope: none
output:
  kind: candidate_draft.v1
mcp_access: full
tools:
  - name: dadrock.get_record
    mode: read
---

# Merge records

Consolidate two or more overlapping candidate DADRs into one new candidate that states the shared choice once, clearly. Merging produces a new candidate; the source candidates stay as they are until the reviewer decides what to do with them. If one of the overlapping records is already accepted, this is the wrong task: draft a successor with [supersede-record](supersede-record.md), which can supersede several accepted records at once.

## Inputs

- `record_ids`: the candidates to merge.
- `merge_goal`: optional; what the merged record should emphasize, or how to settle differences between the sources.

Interactively, these come from the user's request. Do not guess the sources.

## Confirm it is one decision

Read every source's Decision before consolidating. Candidates that share a `kind/` tag or a file can still be distinct choices on adjacent steps of a pipeline. If the sources describe different choices, the merge is wrong: say so in the warnings, name the distinct sources, and limit the draft to the sources that do share one choice.

## What to keep

- **The strongest Context.** Often one source gives the motivation (a plan section, a theoretical reason) and another the concrete site (a file and its lines). Combine them, keeping every distinct source link.
- **The clearest Decision wording**: active voice, present tense, quantitatively specific.
- **Every distinct alternative**, deduplicated, keeping any reason that makes a real distinction.
- **The most quantified Consequences.**
- **The union of tags**, with one spelling per concept (not both `kind/sample` and `kind/sampling`). Keep `writer/agent` if any source has it.

Merged prose should read as one record, not several pasted together. Do not add alternatives or rationale that no source contains. The merged candidate is a new record, so it does not reuse a source's id.

## Disagreements

If the sources disagree on substance, such as a winsorization threshold, an imputation rule, or a sample-size figure, do not pick silently and do not invent a compromise. Follow `merge_goal` when it settles the question. Otherwise, state both values in the Decision as an open choice for the reviewer and flag the disagreement in the warnings.

## Constraints

- All sources must be candidates; do not merge across the lifecycle boundary.
- Leave the sources unchanged. Rejecting them once the merged record is accepted (for example as `rejected/duplicate`) is the reviewer's decision.
- One merged candidate per merge. A primary choice and its robustness variant are two records.
- Never accept or reject anything.

## Result

One candidate:

- **Title**: a noun phrase describing the consolidated choice.
- **Tags**: the normalized union of the sources' tags.
- **Body**: the four sections, around 200–400 words. Past ~600 words, the sources were probably distinct decisions; say so.
- **Source references**: every distinct file, line range, or anchor from the sources.
- **Warnings**: disagreements between sources and how they were handled, sources that turned out to be distinct, and the ids of the source candidates the reviewer may close once the merge is accepted.
