---
name: dadr
description: Manage Data Analysis Decision Records (DADRs) in an analysis project using the `dadrock` CLI. Workflows — install dadrock and an ACP provider on the local machine, bootstrap candidate DADRs from existing code/notebooks/plans/manuscripts, synthesize specs from accepted decisions, OR detect and resolve conflicting decisions via supersession. Use when the user asks to "install dadrock", "set up dadrock", "configure dadrock agent", "bootstrap DADR", "draft DADR candidates", "find decisions in this project", "seed dadr/", "scaffold DADRs", "turn accepted decisions into a spec", "find conflicting decisions", "resolve duplicate decisions", or "clean up the dadr/ backlog".
---

# DADR

The umbrella router for managing Data Analysis Decision Records with `dadrock`. Task guidance lives as flat Markdown files under [tasks/](tasks/); shared reference documents live at this level.

## Task Guidance

One guide per `dadrock` task or task-like workflow:

| Task | Skill | What it does |
|---|---|---|
| `code-to-record` | [tasks/code-to-record.md](tasks/code-to-record.md) | Crawl an existing analysis project and draft candidate DADRs for the consequential choices. |
| `context-to-record` | [tasks/context-to-record.md](tasks/context-to-record.md) | Draft one candidate from supplied context (plan paragraph, code excerpt, paper passage). |
| `edit-record` | [tasks/edit-record.md](tasks/edit-record.md) | Apply reviewer-requested changes to a candidate while preserving intent and evidence. |
| `supersede-record` | [tasks/supersede-record.md](tasks/supersede-record.md) | Propose a candidate that supersedes an accepted record when circumstances change. |
| `merge-records` | [tasks/merge-records.md](tasks/merge-records.md) | Consolidate overlapping candidate drafts into one. |
| `find-conflicts` | [tasks/find-conflicts.md](tasks/find-conflicts.md) | Detect and scaffold resolution of contradicting decisions via supersession or rejection. |
| `custom` | [tasks/custom.md](tasks/custom.md) | Escape hatch for ad-hoc DADR-related instructions that don't match a dedicated skill. |
| `decisions-to-spec` | [tasks/decisions-to-spec.md](tasks/decisions-to-spec.md) | Synthesize accepted decisions into an implementation, analysis, or reporting spec. |

## References

- [record.md](record.md) — anatomy of a single DADR record (front matter, four sections, length envelope, optional tags, lifecycle, common pitfalls).
- [tags.md](tags.md) — tag format, optional tags, and recommended namespaces.
- [cli.md](cli.md) — `dadrock` command reference (mirrors `dadrock help`).

## Routing

Pick the workflow that matches the request:

| User says | Where to look |
|---|---|
| "install dadrock", "set up dadr", "configure ACP", "what do I need to use dadrock?" | [install.md](install.md) |
| "bootstrap DADR", "seed dadr/", "find analytic choices in this project", "scaffold candidates", "onboard this analysis to DADR" | [tasks/code-to-record.md](tasks/code-to-record.md) |
| "draft a DADR for this", "capture this decision", "log this choice" | [tasks/context-to-record.md](tasks/context-to-record.md) |
| "revise this candidate", "rework the alternatives section", "the reviewer wants changes" | [tasks/edit-record.md](tasks/edit-record.md) |
| "supersede decision X", "change the accepted choice", "draft a successor" | [tasks/supersede-record.md](tasks/supersede-record.md) |
| "merge these candidates", "consolidate the duplicate drafts" | [tasks/merge-records.md](tasks/merge-records.md) |
| "find conflicting decisions", "resolve duplicate DADRs", "any decisions contradict each other?", "clean up dadr/" | [tasks/find-conflicts.md](tasks/find-conflicts.md) |
| "diagnose dadr/", "health-check the DADR ledger", "audit the ledger", "what's broken in dadr/?" | Use the validation page or `dadrock validate`; deterministic diagnostics do not go through agent tasks. `dadrock validate changes` reports edits to accepted or rejected records since a Git revision. |
| "turn accepted decisions into a spec", "write a spec from dadr/", "decisions to spec" | [tasks/decisions-to-spec.md](tasks/decisions-to-spec.md) |
| Ad-hoc DADR request that doesn't match the rows above | [tasks/custom.md](tasks/custom.md) |
| "what does a record look like?", "what does a candidate look like?", "how long should a DADR be?" | [record.md](record.md) |
| "which tags?", "is `kind/foo` valid?", tag taxonomy questions | [tags.md](tags.md) |
| "what does `dadrock <cmd>` do?", flag/option lookups | [cli.md](cli.md) |

Do **not** trigger this skill for routine `dadrock new` requests (single-decision drafting), spec questions, or validation/lint requests — call `dadrock` directly.

## Shared preconditions

The decision-handling workflows require:

1. `dadrock` on `PATH`:
   ```bash
   command -v dadrock && dadrock --version
   ```
   If absent, route to [install.md](install.md) — that guide handles platform detection and picks the right install method.

2. The current directory is the project root the user means. Run `pwd` and `ls` to confirm if unclear.

3. Inspect the project's Dadrock directory:
   ```bash
   dadrock dir
   ```
   The two main workflow families diverge from there:
   - **code → record** ([tasks/code-to-record.md](tasks/code-to-record.md)) may need `dadrock init` if no project is configured.
   - workflows that read accepted records require an existing populated `dadr/ledger/` with at least one decision in force. Stop and ask the user if it is empty.

## Shared constraints

- **Acceptance is a human approval boundary** — never auto-accept candidates.
- **UUIDv7 only.** Always go through Dadrock's candidate-writing surface (`dadrock.create_candidate` in MCP or `dadrock new` in the CLI); never hand-write the `id` field.
- **Never overwrite or edit accepted records** without explicit user permission. New choices that contradict an accepted record are drafted as successor candidates with a `## Supersedes` section (or linked later with `dadrock supersede`).
- **One file per decision.** A single piece of code may embed two distinct choices; draft (or implement) them as two records.

## Reflect and improve

After completing the task, briefly reflect and update *this* SKILL.md if a generalizable lesson came up.

1. **Reflect**:
   - Did any instruction here mislead, confuse, or get ignored?
   - Did the user correct your approach, supply a preference, or surface an edge case not yet covered?
   - Was a recurring failure mode or trigger phrase missing from the examples?
   - Was a step unnecessary, out of order, or missing?

2. **Decide**:
   - Generalizable lesson about *how this skill should work* → update this SKILL.md with the Edit tool.
   - One-off or user-specific lesson (not about the skill itself) → skip; consider auto memory instead.
   - Nothing notable → say so explicitly and skip the edit.

3. **Edit**:
   - Make small, surgical changes — refine a sentence, add a bullet, add an example. Do not rewrite sections wholesale.
   - Never remove this reflection step itself.

4. **Report**:
   - Tell the user in one or two sentences what changed (or that nothing changed) and why.
