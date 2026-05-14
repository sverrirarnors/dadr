# Task Context And Result Actions Design

## Purpose

Dadrock tasks should support richer user context before a run and clearer
actions after a run. The current task system already has task-specific form
inputs, typed output kinds, persisted runs, and result renderers. This design
adds a common context layer and a typed action model so users can steer tasks
with files, records, notes, and follow-up instructions, then act on results
without guessing what to do next.

## Existing Task Families

Dadrock's built-in tasks fall into four interaction families:

| Family | Built-in tasks | Primary user need |
|---|---|---|
| Authoring | `context-to-record`, `code-to-record` | Turn evidence into one or more candidate DADRs. |
| Revision | `edit-record` | Transform an existing candidate while preserving intent. |
| Triage | `find-conflicts` | Identify conflicts and follow-up work. Deterministic ledger diagnostics belong in validation, not agent tasks. |
| Lifecycle proposal | `supersede-record` | Propose a candidate that changes record lifecycle direction. |

These families should not become separate UI modes. They should share one task
runner, one history model, and one result surface, with output-kind-specific
actions.

## Context Model

Every task run should accept a common optional context envelope in addition to
the task's declared manifest inputs.

```ts
interface TaskContext {
  instructions?: string;
  paths?: string[];
  records?: string[];
  text?: string;
}
```

`instructions` is the user's extra prompt for this run, such as "focus on MCP
setup and ignore docs." `paths` names code files or directories to inspect.
`records` names relevant DADR records. `text` stores pasted context such as PR
descriptions, meeting notes, error logs, or excerpts.

The task runner sends this context to the agent as structured data, not as
ad-hoc prose concatenation. The UI should show the context tray below the
task-specific fields so the primary task inputs stay prominent.

## Input Types

The existing manifest input types remain valid. Add richer input types only
where they improve validation or UI affordances:

| Input type | Use |
|---|---|
| `record` | Select or resolve one DADR record. |
| `record_list` | Select multiple DADR records for comparison or scope. |
| `path_list` | Provide multiple code paths or directories. |
| `instructions` | A task-visible free-text instruction field. |

The context tray can be implemented before these manifest input types. Manifest
types are useful once task authors need task-specific record or path fields.

## Result Action Model

Result actions are keyed by output kind and, when applicable, by result item.
The UI should show curated buttons first and an optional follow-up instruction
field for agent-guided actions.

```ts
interface TaskResultAction {
  id: string;
  label: string;
  target: "command" | "task" | "navigation";
  requiresInstructions?: boolean;
}
```

Actions should not mutate files blindly. Destructive or content-changing actions
should either preview first or launch a follow-up task with the selected result
item as context.

## Actions By Output Kind

| Output kind | Actions |
|---|---|
| `candidate_draft.v1` | Edit, write candidate, revise draft with note. |
| `candidate_batch.v1` | Write one, write selected, write all, revise one proposal with note. |
| `record_revision.v1` | Preview diff, apply revision, revise again with note. |
| `supersession_proposal.v1` | Write superseding candidate, open superseded record. |
| `conflict_set.v1` | Open records side-by-side, start revision, start supersession, dismiss finding. |
| `diagnosis_report.v1` | Open affected record, fix with note, create candidate, dismiss issue. |
| `markdown_report.v1` and vendor kinds | Rerun with note, turn into candidate, copy raw result. |

The generic "fix" affordance should be labeled by context. For example, a
diagnosis issue can show "Fix..." and a conflict finding can show
"Resolve...". Both open the same follow-up instruction affordance.

## Follow-Up Tasks

Follow-up actions launch tasks with prefilled context. Examples:

- Diagnosis issue with `record_id` -> `edit-record` with the issue copied
  into `requested_changes`.
- Diagnosis issue without `record_id` -> `context-to-record` with the issue as
  context.
- Conflict finding -> `edit-record` or `supersede-record`, chosen by
  explicit user action.
- Candidate draft -> rerun authoring with the draft and user note as context.
- Candidate batch item -> revise only that proposed candidate before writing.

The system should preserve the source run id and item id in the follow-up input
so users can trace why an action exists.

## UI Layout

The Tasks page keeps its current three-tab shape:

- `Tasks`: task selection, task-specific inputs, context tray, run controls.
- `History`: persisted runs and status.
- `Results`: typed renderer, action panel, agent log access.

The context tray should be collapsible and default to closed unless the selected
task has no required inputs or a previous run supplied context. The action panel
belongs near the result item it affects, not only at the top of the page.

## Error Handling

Invalid paths or records should be reported before the agent run starts when
Dadrock can validate them. Agent-generated results still require output-kind
validation before actions appear. If a follow-up action cannot determine a safe
target task, the UI should offer "Rerun with note" rather than attempting a
mutation.

Dismissed findings are local run annotations, not edits to DADR records. They
can be stored under the run directory and should not affect the canonical ledger.

## Implementation Slices

1. Add the common context envelope to task run requests, persistence, prompt
   rendering, and the web API.
2. Add context tray UI for instructions, paths, records, and pasted text.
3. Add a result action registry for existing output kinds.
4. Implement low-risk actions first: navigation, write candidate, rerun with
   note.
5. Implement preview-gated mutations: revision apply and supersession-related
   actions.
6. Add local dismissal annotations for triage findings.

## Open Decisions

- Whether follow-up actions should create a new run every time or support an
  inline continuation of an existing run. Version 1 should create a new run for
  traceability.
- Whether record and path selectors should be manifest input types immediately
  or initially exist only inside the shared context tray. Version 1 should start
  with the context tray and add manifest types when task authors need them.
- Whether dismissals should be visible only in the current result or across
  future runs. Version 1 should keep dismissals local to the run.
