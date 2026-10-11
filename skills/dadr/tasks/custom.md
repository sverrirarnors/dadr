---
name: dadr-custom
description: Run a custom DADR-aware instruction against the project, using selected records, paths, and notes as context. Use as an escape hatch when none of the other dadr-* task skills fit — for ad-hoc questions, one-off transformations, or experimental workflows.
---

# Custom DADR task

The escape hatch. Use when the user's request is DADR-related but does not match any of the dedicated task skills (`dadr-code-to-record`, `dadr-find-conflicts`, `dadr-context-to-record`, `dadr-edit-record`, `dadr-supersede-record`, `dadr-merge-records`).

The user supplies instructions explicitly; you act on them with the project snapshot, selected records, and supplied paths as the source of truth.

Trigger phrases:

- ad-hoc requests that mention dadrock or dadr/ but don't match a specific workflow
- "do this on these records…"
- "experimental — try X on the ledger…"

## Preconditions

Shared preconditions in [../SKILL.md](../SKILL.md) apply. In addition:

- The user must supply *instructions*. "Do something useful" is not a valid invocation; ask what they want before acting.
- The user should identify *what to act on*: specific records (by id, query, or filter), paths, or pasted text.

## When NOT to use this skill

If any of these match, prefer the dedicated skill:

| Request | Use instead |
|---|---|
| Bootstrap a new project's candidates | [code-to-record.md](code-to-record.md) |
| Draft a single candidate from a plan/code/paper snippet | [context-to-record.md](context-to-record.md) |
| Revise an existing candidate per reviewer feedback | [edit-record.md](edit-record.md) |
| Supersede an accepted decision with a successor | [supersede-record.md](supersede-record.md) |
| Consolidate overlapping candidates | [merge-records.md](merge-records.md) |
| Find or resolve conflicts | [find-conflicts.md](find-conflicts.md) |
| Audit the ledger for issues | Use the validation page or `dadrock validate`; deterministic diagnostics do not go through agent tasks. |
| Set up dadrock or an ACP provider | [../install.md](../install.md) |

If the request fits one of those rows, route there.

## Workflow

### Step 1 — Restate the instruction

Echo back, in one sentence, what you understood the user wants to do. Confirm before acting if the request is ambiguous or would create, delete, or transition records.

### Step 2 — Gather context

The user typically supplies:

- **Records** — by id, by query, or by filter (`dadrock list --status accepted --tag kind/inference`).
- **Paths** — files or directories relevant to the instruction.
- **Pasted text** — a plan section, paper passage, meeting note, or freeform description.
- **Instructions** — the imperative the agent should follow.

Read each piece before acting. For records: `dadrock status <query>` to fetch the body. For paths: `Read` or `Glob` as appropriate.

### Step 3 — Act

Follow the user's instruction. Common patterns:

- Produce a written report on a subset of records.
- Cross-reference records against an external document (paper, plan, manuscript).
- Compute a summary statistic over the ledger (counts by tag, age distribution, sample-size impact).
- Suggest a re-tagging plan.
- Draft custom prose (e.g., a methods-section paragraph based on accepted decisions).

When the instruction would create, modify, or transition records, **stop and route to the dedicated skill** rather than acting from here. This skill is for reporting and analysis; lifecycle changes go through the dedicated workflows.

### Step 4 — Output

Return a single markdown report covering:

- **Summary** — one or two sentences on what was done.
- **Body** — the answer to the user's instruction, with anchors to the records and paths consulted.
- **Caveats** — anything the user should know about the limits of the result (records skipped, body reads deferred, ambiguous instructions).

The output is a `markdown_report.v1` shape: title, summary, `body_markdown`.

## Constraints

- **Never run lifecycle transitions.** No `dadrock status accept|reject|supersede` from here.
- **Never edit accepted records.** If the instruction would require it, push back and propose a supersession workflow.
- **Read-only by default.** This skill reports on the ledger; it does not modify it unless the user explicitly authorizes specific edits and the edits are confined to candidate records (in which case prefer [edit-record.md](edit-record.md)).
- **Don't invent context.** If the user references a record or file you cannot find, ask before guessing.

## Example

User: "How many of our accepted decisions are tagged `kind/sample` versus `kind/modeling`, and which sample decisions have no source link in Context?"

1. Restate: "Count accepted records by `kind/`, and within `kind/sample`, list those whose Context section lacks a source link."
2. `dadrock list --status accepted --tag kind/sample --json` and `dadrock list --status accepted --tag kind/modeling --json`.
3. For each main-analysis record, read the body and check for a markdown link in `Context`.
4. Report: "32 accepted records — 24 `kind/sample`, 8 `kind/modeling`. Of the 24 sample records, 3 lack source links: 01938b8e, 01938c4a, 01938d12."
5. Suggest next step: [edit-record.md](edit-record.md) — except these are accepted, so the path is supersession. Offer [supersede-record.md](supersede-record.md) per record if the user wants links added.
