---
id: find-conflicts
title: "Find conflicting decisions"
description: "Audit selected DADRs for contradictions, duplicates, or unresolved overlap that need follow-up."
version: 1
inputs:
  - name: status_scope
    label: "Records to scan"
    type: select
    default: "live"
    options: ["live", "candidates", "accepted", "all"]
requirements:
  records: none
  record_status: all
  paths: none
  notes: optional
  scope: live
output:
  kind: conflict_set.v1
tools:
  - name: dadrock.list_records
    mode: read
---

# Find conflicts

Audit DADR records for contradictions, duplicates, and unresolved overlap, and report each finding with what a reviewer needs to resolve it. A ledger drafted over time, especially by agents, can accumulate a history ("do X", later "don't do X") instead of one current state; supersession and rejection are how DADR settles that. This task finds and explains the problems. Resolving them is the reviewer's job.

## Inputs

- `status_scope`: which records to scan.
  - `live` (default): candidates, plus accepted records that have not been superseded.
  - `candidates`: candidates only.
  - `accepted`: accepted records that have not been superseded.
  - `all`: every record, including rejected and superseded ones, which also reveals re-drafts of closed ideas.

Interactively, this comes from the user's request; default to `live`.

## What counts as a conflict

A conflict is two records whose Decisions cannot both hold on the same analytic surface at the same time, or that record the same decision twice.

Strong signals:

- the same `kind/` tag and incompatible Decision text;
- Context links to the same file with overlapping line ranges;
- the same `plan/` tag with incompatible plan-related prose.

Weaker signals, worth checking but not conclusive:

- the same `topic/` tag or title noun phrase under different `kind/` tags;
- near-duplicate titles.

Not a conflict:

- one record is the other's successor: already resolved;
- both are rejected;
- one is a robustness or alternative-specification record that explicitly varies the other, primary choice: they coexist by design;
- they describe different steps of the same pipeline, such as a row filter and a later imputation. Read the Decision text, not just the Context paths.

## Work cheaply first

Reading bodies is the expensive part. Narrow the corpus with metadata before reading any: drop records outside the scope and superseded records, group the rest by `kind/` tag, then by `topic/` or `plan/` tags and title keywords (missing, winsorize, cluster, treatment, …). Order each group by creation time, which the UUIDv7 id encodes, so the newer record reads as the likely revision. Read Decisions and Context links only for groups with more than one record. With only a handful of records, read them all and say there is little to find.

## Relationships and resolutions

| Relationship | What happened | Typical resolution |
|---|---|---|
| candidate ↔ candidate | Two drafts of the same choice, neither accepted | Keep one and reject the other (`rejected/duplicate`), or merge them into a new candidate ([merge-records](merge-records.md)). |
| candidate ↔ accepted | A newer candidate revises an accepted decision | Turn the candidate into a replacement of the accepted record ([supersede-record](supersede-record.md)) and accept it, or reject the candidate if the accepted decision stands. |
| accepted ↔ accepted | Two decisions in force contradict each other | Draft one consolidated replacement that supersedes both once accepted. |
| candidate ↔ rejected | A rejected idea has been drafted again | Possibly intentional (new evidence), possibly an oversight; the reviewer decides. |

A cycle in `successors` (A lists B and B lists A) is a ledger error; report it as a finding.

## Constraints

- Report only conflicts grounded in record text. If you cannot point at the incompatible sentence in each Decision, it is not a conflict.
- Do not mistake sequence for contradiction.
- This task reports; it does not resolve. Never accept, reject, or supersede records, and never edit an accepted record to make two records agree: supersession is the only way to revise an accepted decision.

## Result

A conflict report:

- **Summary**: which records were scanned, how they were grouped, how many findings there are, and anything skipped. Zero findings is a valid result.
- **Findings**, one per conflicting pair; report a group of three or more as pairs with a shared title. Each finding has:
  - a short **title** naming the contested choice;
  - the **two records** by id, the older one first;
  - the **relationship**: the pattern above (for example, candidate ↔ accepted) and its nature (contradiction, duplicate, or overlap);
  - a **conflict explanation**: the incompatible statements, quoted or closely paraphrased from each Decision, in a few sentences;
  - **why it might not be a conflict**: the most plausible innocent reading, such as sequential steps or a deliberate robustness variant. Always include one; it gives the reviewer a quick way to decline;
  - **suggested actions**: concrete, ordered steps for the reviewer that name the records involved.

Keep findings terse and uniform, so a reviewer can skim twenty of them in a few minutes.
