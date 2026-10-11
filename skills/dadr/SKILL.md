---
name: dadr
description: Manage Data Analysis Decision Records (DADRs) in an analysis project using the `dadrock` CLI. Workflows — install dadrock and an ACP provider on the local machine, draft candidate DADRs from existing code/notebooks/plans/manuscripts or from a described decision, revise, merge, or supersede records, detect conflicting decisions, OR synthesize specs from accepted decisions. Use when the user asks to "install dadrock", "set up dadrock", "configure dadrock agent", "bootstrap DADR", "draft DADR candidates", "find decisions in this project", "seed dadr/", "scaffold DADRs", "capture this decision", "revise this candidate", "supersede decision X", "merge these candidates", "find conflicting decisions", "resolve duplicate decisions", "clean up the dadr/ backlog", or "turn accepted decisions into a spec".
---

# DADR

The umbrella router for managing Data Analysis Decision Records with `dadrock`. Each task has a guide under [tasks/](tasks/) that says what to produce and the domain rules for producing it; shared reference documents live at this level.

Dadrock runs these same guides as its built-in tasks (from the web UI or `dadrock task run <id>`) and adds its own delivery rules, so the guides never say how to deliver a result. When you work interactively, deliver through [Writing records](#writing-records-interactive-use) below.

## Task guides

| Task | Guide | Result | What it does |
|---|---|---|---|
| `code-to-record` | [tasks/code-to-record.md](tasks/code-to-record.md) | New candidates | Survey an existing analysis project and draft candidates for its consequential choices. |
| `context-to-record` | [tasks/context-to-record.md](tasks/context-to-record.md) | One new candidate | Draft one candidate from supplied context (plan paragraph, code excerpt, paper passage). |
| `edit-record` | [tasks/edit-record.md](tasks/edit-record.md) | Revised record | Apply reviewer-requested changes to a record while preserving its choice and evidence. |
| `supersede-record` | [tasks/supersede-record.md](tasks/supersede-record.md) | Replacement candidate | Draft a replacement for an accepted decision that has changed. |
| `merge-records` | [tasks/merge-records.md](tasks/merge-records.md) | One new candidate | Consolidate overlapping candidates into one. |
| `find-conflicts` | [tasks/find-conflicts.md](tasks/find-conflicts.md) | Report | Find contradictions, duplicates, and overlap, with suggested resolutions. |
| `decisions-to-spec` | [tasks/decisions-to-spec.md](tasks/decisions-to-spec.md) | Report | Synthesize accepted decisions into an implementation, analysis, or reporting spec. |
| `custom` | [tasks/custom.md](tasks/custom.md) | Report | Escape hatch for ad-hoc DADR instructions that no dedicated guide covers. |

## References

- [record.md](record.md) — anatomy of a single DADR record (front matter, four sections, length envelope, optional tags, lifecycle, common pitfalls).
- [tags.md](tags.md) — tag format, optional tags, and recommended namespaces.
- [cli.md](cli.md) — `dadrock` command reference (mirrors `dadrock help`).
- [install.md](install.md) — installing dadrock and an optional ACP provider.

## Routing

Pick the workflow that matches the request:

| User says | Where to look |
|---|---|
| "install dadrock", "set up dadr", "configure ACP", "what do I need to use dadrock?" | [install.md](install.md) |
| "bootstrap DADR", "seed dadr/", "find analytic choices in this project", "scaffold candidates", "onboard this analysis to DADR" | [tasks/code-to-record.md](tasks/code-to-record.md) |
| "draft a DADR for this", "capture this decision", "make a candidate from this plan paragraph", "log this choice" | [tasks/context-to-record.md](tasks/context-to-record.md) |
| "revise this candidate", "rework the alternatives section", "tighten the consequences", "the reviewer wants changes" | [tasks/edit-record.md](tasks/edit-record.md) |
| "supersede decision X", "replace the accepted choice", "draft a successor", "the SAP changed, update the decision" | [tasks/supersede-record.md](tasks/supersede-record.md) |
| "merge these candidates", "consolidate the duplicate drafts", "these two are the same decision" | [tasks/merge-records.md](tasks/merge-records.md) |
| "find conflicting decisions", "resolve duplicate DADRs", "any decisions contradict each other?", "clean up dadr/" | [tasks/find-conflicts.md](tasks/find-conflicts.md) |
| "diagnose dadr/", "health-check the DADR ledger", "audit the ledger", "what's broken in dadr/?" | Use the validation page or `dadrock validate`; deterministic diagnostics do not go through agent tasks. |
| "turn accepted decisions into a spec", "write a spec from dadr/", "decisions to spec" | [tasks/decisions-to-spec.md](tasks/decisions-to-spec.md) |
| Ad-hoc DADR request that doesn't match the rows above | [tasks/custom.md](tasks/custom.md) |
| "what does a record look like?", "what does a candidate look like?", "how long should a DADR be?" | [record.md](record.md) |
| "which tags?", "is `kind/foo` valid?", tag taxonomy questions | [tags.md](tags.md) |
| "what does `dadrock <cmd>` do?", flag/option lookups | [cli.md](cli.md) |

Do **not** trigger this skill for a bare "create a record titled X", spec questions, or validation/lint requests — call `dadrock` directly.

## Writing records (interactive use)

The task guides describe the result. When you do the work yourself in the user's project, this section says how to deliver it.

### Before you start

1. `dadrock` must be on `PATH`:
   ```bash
   command -v dadrock && dadrock --version
   ```
   If it is absent, follow [install.md](install.md), which handles platform detection and the install method.
2. Confirm the current directory is the project the user means (`pwd`, `ls` if unclear), then run `dadrock dir`. If no project is found, guides that draft records need `dadrock init`, which creates `dadr/` and never overwrites files; confirm with the user before running it.
3. Guides that read accepted records (`supersede-record`, `decisions-to-spec`) need at least one. If `dadr/accepted/` is empty, stop and ask the user.
4. Take each guide's inputs from the user's request, and ask for required inputs that are missing (the record to revise, the reason for a replacement).

Read the ledger with `dadrock list` (JSON summaries; filter with `--status`, `--tag`, `--query`), `dadrock status <query>` for one record and its path, and `dadrock tags list`. When Dadrock's MCP tools are available, `dadrock.list_records`, `dadrock.get_record`, and `dadrock.list_tags` do the same.

### Delivering each result

- **New candidates** (`code-to-record`, `context-to-record`, `merge-records`): create one record per candidate with
  ```bash
  dadrock new "Drop observations with missing treatment assignment" \
    --tag kind/sample \
    --tag writer/agent
  ```
  The title is the guide's noun-phrase title. Pass tags without `#`, one `--tag` each. The command prints a JSON envelope whose `data.path` is the new file; with `--quiet` it prints only the new id. When the MCP tool `dadrock.create_candidate` is available, call it with `title`, `tags`, and `body_markdown` instead. Never hand-write an `id` or a record file.
- **Filling the body**: edit the created file, replacing the empty `## Context`, `## Decision`, `## Alternatives`, and `## Consequences` sections with the drafted text. Leave `id` and `status` as created. Source references become Markdown links in Context, relative to the record's directory (`[analysis/clean.R:42-57](../../analysis/clean.R#L42-L57)`). A confidence level can become a `confidence/` tag. The summary, deferred areas, and warnings go in your report to the user.
- **Revisions** (`edit-record`): edit the record file in place; `dadrock status <query>` gives its path. Candidates may be edited freely. Edit an accepted record only with the user's explicit permission, and only with corrections that keep its meaning. Never touch `id`, `status`, or `successors`.
- **Replacements** (`supersede-record`): create a new candidate as above, whose body ends with the `## Replaces` section. Never edit the predecessor. When the user accepts the replacement (`dadrock accept <new-id>`), dadrock links the predecessor. `dadrock supersede --from <old-id> --to <new-id>` is only for linking two records that are both already accepted.
- **Reports** (`find-conflicts`, `decisions-to-spec`, `custom`): give the report to the user, or write it to a file they name. The user decides what to accept, reject, or supersede. Run `dadrock accept`, `dadrock reject`, or `dadrock supersede` only when the user explicitly asks for that specific action.
- **Warnings**: where a guide says to flag something in the warnings (missing rationale, an unanchored claim, a disputed value), you may ask the user directly when the answer is cheap to get, and use their answer.

### Finish

1. Run `dadrock validate` and fix errors in the files you wrote (malformed tags, broken links, structural issues).
2. Report what you drafted or changed (titles and paths), what you skipped or deferred and why, and the open warnings.
3. Remind the user that accepting is their step: `dadrock accept <id>` or `dadrock reject <id>`, or review in the web UI (`dadrock preview`).

## Shared constraints

These hold in every guide, whether you deliver the result yourself or dadrock runs the task:

- **Acceptance is a human approval boundary.** Never accept, reject, or supersede records on your own initiative.
- **Ids come from Dadrock.** Never write an `id`, a `successors` entry, or a date into front matter by hand.
- **Accepted decisions keep their meaning.** Corrections are fine; a changed decision is a new candidate that supersedes the old record.
- **One record per decision.** A single piece of code may embed two distinct choices; they get two records.
- **No invented rationale or alternatives.** Write "Rationale not documented" or "Not applicable" instead.

## Reflect and improve

After completing the task, briefly reflect and update *this* SKILL.md if a generalizable lesson came up.

1. **Reflect**:
   - Did any instruction here mislead, confuse, or get ignored?
   - Did the user correct your approach, supply a preference, or surface an edge case not yet covered?
   - Was a recurring failure mode or trigger phrase missing from the examples?
   - Was a step unnecessary, out of order, or missing?

2. **Decide**:
   - Generalizable lesson about *how this skill should work* → update this SKILL.md.
   - One-off or user-specific lesson (not about the skill itself) → skip; consider auto memory instead.
   - Nothing notable → say so explicitly and skip the edit.

3. **Edit**:
   - Make small, surgical changes — refine a sentence, add a bullet, add an example. Do not rewrite sections wholesale.
   - Keep delivery mechanics in this file; task guides under `tasks/` stay mode-neutral because dadrock sends them to agents as task prompts.
   - Never remove this reflection step itself.

4. **Report**:
   - Tell the user in one or two sentences what changed (or that nothing changed) and why.
