---
id: custom
title: "Custom"
description: "Use a freeform prompt with selected records, files, folders, or notes when no dedicated DADR workflow fits."
version: 1
requirements:
  records: many
  record_status: all
  paths: optional
  notes: required
  scope: none
output:
  kind: markdown_report.v1
mcp_access: full
tools:
  - name: dadrock.project_snapshot
    mode: read
---

# Custom DADR task

Carry out a DADR-related instruction that no dedicated task covers, using the selected records, files, pasted text, and the project itself as the source of truth.

## Inputs

- The user's instructions: what to do. Required.
- Selected records of any status and, optionally, files, folders, or pasted text to act on.

Interactively, these come from the user's request. "Do something useful" is not an instruction; find out what the user wants.

## Prefer a dedicated task

| Request | Use instead |
|---|---|
| Find decisions in existing code or documents | [code-to-record](code-to-record.md) |
| Capture one known decision | [context-to-record](context-to-record.md) |
| Revise an existing candidate | [edit-record](edit-record.md) |
| Supersede an accepted decision with a successor | [supersede-record](supersede-record.md) |
| Consolidate overlapping candidates | [merge-records](merge-records.md) |
| Find contradictions or duplicates | [find-conflicts](find-conflicts.md) |
| Turn accepted decisions into a spec | [decisions-to-spec](decisions-to-spec.md) |
| Check records, links, and tags for format errors | Dadrock's validation, which is deterministic and not an agent task |

## Typical work

- A written report on a subset of records.
- Cross-referencing records against an external document such as a plan or manuscript.
- Summaries over the ledger: counts by tag, decisions per analysis stage, sample-size impacts.
- A re-tagging plan.
- Prose built from accepted decisions, such as a methods paragraph.

## Constraints

- Ground every statement in the records, files, and text supplied or found in the project. If a referenced record or file cannot be found, say so rather than guessing.
- This task reports. If the instruction calls for creating, revising, or superseding records, describe the proposed changes in the report and name the dedicated task that would carry them out.
- Never accept, reject, or supersede records, and never propose changing the meaning of an accepted record: a changed decision needs a successor that supersedes it.

## Result

A Markdown report:

- **Title**, when helpful.
- **Summary**: one or two sentences restating the instruction and what was done.
- **Body**: the answer, citing records with record links (`[[<short-id>]]`) and files with Markdown links, and ending with caveats such as records skipped, ambiguous instructions, or limits of the evidence.
