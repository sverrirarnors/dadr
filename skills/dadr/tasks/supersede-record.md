---
id: supersede-record
title: "Supersede record"
description: "Draft a successor candidate that supersedes one or more accepted DADRs when the decision has changed."
version: 2
inputs:
  - name: record_ids
    label: "Records to supersede"
    type: textarea
    required: true
  - name: requested_changes
    label: "Requested changes"
    type: textarea
    required: true
requirements:
  records: one_or_more
  record_status: accepted
  paths: none
  notes: required
  scope: none
output:
  kind: supersession_proposal.v1
tools:
  - name: dadrock.get_record
    mode: read
---

# Supersede record

Draft a successor: a new candidate that, once a human accepts it, supersedes one or more accepted records, its predecessors. Agents never edit an accepted record. A person may correct its wording in place when the meaning stays the same (spelling, formatting, links, citations); every change to what was decided goes through a successor. When the successor is accepted, dadrock adds its id to each predecessor's `successors` and moves the predecessors to `dadr/archive/`.

## Inputs

- `record_ids`: the accepted records to supersede, one per line. Several records make a merge: one successor takes the place of all of them.
- `requested_changes`: what the analyst wants changed, and usually why: new evidence, a plan amendment, a conflict with another decision, a correction.

Interactively, these come from the user's request. "I changed my mind" is a valid reason, but state it concretely.

Each predecessor must be accepted and current. A superseded record is closed: its current successors hold the decision now in force. Say so and name them rather than drafting; the change belongs in a successor of those decisions. If the records are still candidates, revise them with [edit-record](edit-record.md) or consolidate them with [merge-records](merge-records.md).

Read each predecessor in full before writing: title, tags, and the four sections, with their exact wording.

## Start from the predecessor's exact text

The reviewer reads the successor as a diff against its predecessor, so every unnecessary edit is noise.

- **One predecessor:** copy its title, tags, and four sections exactly, then change only what the request requires. Leave every other sentence, link, citation, and line break as it is. A different estimator may change one phrase; a new cutoff changes the sentences that state the cutoff and the consequences that follow from it.
- **Several predecessors (a merge):** combine their sections into one decision. Keep each predecessor's wording where it still holds, remove repetition, and change only what the merged decision requires. Keep the tags that still apply.
- Keep the title unless the request changes what the decision is about. A title is a noun phrase, not a question.
- Keep source links that still resolve, and update line ranges that have shifted. Do not invent paths or line numbers.
- When the decision itself changed, the predecessor's choice often becomes an alternative, with the reason it no longer holds.

The body has exactly the four sections of the [record reference](../record.md), each once and with text. If a predecessor lacks one, write it from the evidence you have; if genuinely no alternative applies, write "Not applicable".

## What not to write

- **Do not narrate the supersession.** Do not rewrite Context to say that a predecessor existed, why it is being superseded, or that this record supersedes it. The successor reads as a complete record of the decision now in force.
- **Do not put a `## Supersedes` section in the body**, or add links to the predecessors for the sake of the supersession. The successor's `## Supersedes` section is built from the superseded record ids and the rationale: one link per predecessor, then the rationale as the reason for the change.
- **Do not edit the predecessors**, list successors, or suggest lifecycle commands. Accepting the successor links the records; no separate supersede step follows.

## Rationale

The rationale explains why the decision changed, in one to three sentences, for a reviewer deciding whether to accept the successor. Be concrete: "SAP amendment §4.3 requires multiple imputation rather than complete cases", not "this is better". It becomes the reason in the successor's `## Supersedes` section, the permanent record of why the decision changed, so write it as plain prose that stands on its own, without headings.

## Constraints

- **No lost evidence.** Do not silently drop sources or alternatives from a predecessor. Keep them, or explain in the rationale why they no longer apply.
- **One decision per successor.** If the request splits a decision in two, draft the successor for the part the request names and mention the other part in the summary.
- Never edit a predecessor, and never write `successors` by hand.
- Never accept the successor or supersede the predecessors. Those are the reviewer's actions.

## Result

A supersession proposal:

- **Summary**: one sentence describing the change.
- **Superseded record ids**: the full id of every predecessor, one per record in `record_ids`.
- **Title** of the successor: the predecessor's title unless the request changes what the decision is about.
- **Tags** of the successor, starting from the predecessors' tags.
- **Body** of the successor: the four sections, and nothing else.
- **Rationale**: why the decision changed.
