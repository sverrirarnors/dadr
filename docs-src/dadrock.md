---
title: Dadrock
---

!!! warning
    **dadrock** is under heavy development — pre-alpha release. Expect breaking changes.

<img src="../assets/dadrock_logo.svg" alt="dadrock" style="display:block; margin: 0 auto 2rem; max-width: 22rem;">

**DADR** is the standard: the decision record format and the project vocabulary.

**dadrock** is a Rust tool that implements DADR. It previews, creates, edits, and validates decision records, and it can direct local LLM agents through tasks on the decision ledger. Everything it writes is a Markdown file you could have written yourself.

```sh
dadrock init
dadrock new "Decision title"
dadrock preview
dadrock accept ID
dadrock validate
```

Running `dadrock` without a subcommand prints help. `preview` starts the local web server, normally at port 4004. See [Install](install.md) for building the binary.

## Project layout

The DADR format does not mandate a directory structure. **dadrock** uses this one, created by `dadrock init`:

```text
dadr/
  inbox/
    01938ba1c4f3-country-exclusions.md
  ledger/
    01938b8e4d70-income-preprocessing.md
    01938b8f9d71-drop-missing-treatment.md
  archive/
    01938b8d9c10-deprecated-cohort-filter.md
  templates/
    candidate.md                                    # optional
  tasks/
  runs/
    .gitignore
```

| Directory | Contents |
|---|---|
| `dadr/inbox/` | Candidates: proposals awaiting review |
| `dadr/ledger/` | Decisions in force: accepted records without successors |
| `dadr/archive/` | Records kept but no longer in force: rejected records and superseded records |
| `dadr/templates/` | Optional template overrides for new candidates |
| `dadr/tasks/` | Optional project-specific agent workflows (`dadr/tasks/<task-id>/task.toml` plus its prompt file) |
| `dadr/runs/` | Agent-run inputs, results, and logs; ignored by version control |

`dadrock accept` changes a candidate's `status` to `accepted` and moves the file from `dadr/inbox/` to `dadr/ledger/`. `dadrock reject` changes it to `rejected` and moves it to `dadr/archive/`. The `id` never changes. Superseding a decision, by accepting its successor or with `dadrock supersede`, appends the successor's `id` to the predecessor's `successors` and moves the predecessor to `dadr/archive/`; its `status` stays `accepted`.

The directory is a convenience: the front matter is authoritative. `dadrock validate` warns when a record sits in a directory that does not match its `status` and `successors`. Moving a file breaks relative Markdown links to its path; record wikilinks use ids and are unaffected. Use `dadrock config` to inspect the resolved layout. Keep the record directories in version control; run state may contain pasted context and source excerpts and is ignored by default.

Earlier versions of Dadrock used `candidates/`, `accepted/`, and `rejected/`. Dadrock no longer reads those directories: move candidates to `inbox/`, current decisions to `ledger/`, and rejected and superseded records to `archive/`.

## First decision

Five minutes from an empty project to one accepted decision. You do not need an agent or tags.

Change to the project where you keep your analysis, then run:

```sh
dadrock init
dadrock new "Exclude observations without treatment assignment"
```

`init` creates a `dadr/` directory without overwriting existing records. `new` prints the location of a candidate Markdown file and generates its ID.

Start the web interface and open the local URL printed in your terminal:

```sh
dadrock preview
```

The candidate is waiting in the **Inbox**. Open it, choose **Edit**, and fill in its four sections (or edit the Markdown file in your usual editor):

- **Context:** Some observations lack treatment assignment.
- **Decision:** Exclude those observations from this analysis.
- **Alternatives:** Recover assignment from the original log; explain whether this was possible.
- **Consequences:** Report how many observations were removed and consider whether missingness affects interpretation.

Replace these illustrative statements with what actually happened in your project. In particular, explain *why* you chose this approach.

Check that the record describes your analysis accurately and that you endorse the rationale. Save your edits, then choose **Accept**. The record moves to `dadr/ledger/`, keeping its ID, and appears in the **Ledger**. Commit it with your analysis code. You can also accept from a terminal with `dadrock accept ID`, replacing `ID` with the candidate's ID or an unambiguous prefix.

Find it again later:

```sh
dadrock list --status accepted
dadrock validate
```

For a disposable example instead of your own analysis, run `dadrock demo --example candidate-review /tmp/dadr-example`, then `dadrock preview /tmp/dadr-example`. The demo requires a new or empty target directory.

## Changing a decision

An accepted decision is not rewritten. When an accepted choice changes, draft a successor: a new candidate that supersedes the old record, its predecessor. Use **Supersede** on the record in the web interface, or write the candidate yourself. The successor names its predecessor in a `## Supersedes` section:

```markdown
## Supersedes

[[01938c20-a3b4-7000-8000-111122223333|Winsorize income at p99]]

Winsorizing at p99 still left implausible incomes in two waves.
```

The section starts with a record link to the predecessor by its full UUID, `[[uuid]]` or `[[uuid|label]]`. Any text after the link is the reason for the change, in ordinary Markdown. A record has at most one `## Supersedes` section. When Dadrock writes the section, it is the first section of the body, and the link is labeled with the predecessor's title. For now, a successor supersedes exactly one predecessor.

On acceptance, Dadrock checks that the predecessor is accepted and not already superseded, adds the successor's `id` to the predecessor's `successors`, and moves the predecessor to `dadr/archive/` with its body unchanged. The section stays in the successor's body as the record of why the decision changed. It is a Dadrock convention, not an additional required section of the DADR format.

For two records that are already accepted, `dadrock supersede --from OLD --to NEW` links them explicitly and moves `OLD` to `dadr/archive/`. Records linked this way have no `## Supersedes` section.

## Validation

```sh
dadrock validate
dadrock validate --json
```

Current validation checks:

- no `date` key or ISO 8601 date/time values in front matter;
- YAML fields, UUIDv7 IDs, four sections each appearing exactly once with text, and status/location consistency;
- duplicate full IDs and short IDs, and filename collisions;
- missing or invalid successors and cycles in supersession relationships;
- broken or ambiguous record wikilinks and missing local Markdown-link targets;
- malformed tags and unreadable candidate templates.

Focused commands are `dadrock validate records`, `dadrock validate links`, and `dadrock validate tags`. Use labeled links such as `[[01938c20a3b4|sampling rule]]` to make raw Markdown easier to read.

### Changes since a Git revision

Once a record is accepted or rejected, its `id`, `status`, `title`, and body do not change, and entries are not removed from its `successors`. `dadrock validate changes` checks this against Git history:

```sh
dadrock validate changes                      # working tree against HEAD
dadrock validate changes --staged             # staged changes against HEAD
dadrock validate changes --base origin/main   # everything since another revision
```

For each record file in `dadr/inbox/`, `dadr/ledger/`, or `dadr/archive/` that changed since the base revision, Dadrock reads the file's version at the base. If that version was accepted or rejected, the current version must have the same `id`, `status`, `title`, and body (line endings aside), and its `successors` must still list every earlier entry. `tags`, keys outside DADR, and new `successors` entries may change. A record that moved between folders, by being accepted, rejected, or superseded, is followed through Git's rename detection or by its `id`, and passes when these rules hold.

A violation is an error that names the record and suggests drafting a successor instead (`record_id_changed`, `record_status_changed`, `record_title_changed`, `record_body_changed`, `successor_removed`, or `invalid_record` when the new version no longer parses). Deleting an accepted or rejected record is a warning (`record_deleted`). Candidates change freely, new files have no earlier version to compare, and a repository without commits passes. Outside a Git repository, or when the base revision does not resolve, the command fails. Ordinary `dadrock validate` does not use Git.

Validation checks the records' structure, their internal references, and, with `validate changes`, their history in Git. It does not resolve bibliography citations, detect `TBD` prose, enforce plan-deviation sections, or determine whether code follows a decision. Those checks are [roadmap work](roadmap.md). Human review remains necessary.

## Continuous integration

Once Dadrock is installed in your CI environment, run `dadrock validate` from the analysis project root. A nonzero exit status signals an error; `--json` provides structured diagnostics. Install from a pinned source revision for reproducibility; a CLI-only CI build can use the [`no-js-build` feature](install.md#without-the-web-interface).

To check that a pull request leaves accepted and rejected records unchanged, compare it with its target branch. The checkout needs that branch's history:

```sh
git fetch origin main
dadrock validate
dadrock validate changes --base origin/main
```

`--base origin/main` compares with the tip of `main`. That is right when the checkout already includes `main`, such as the merge commit many CI services check out for a pull request. For a branch that `main` has moved past, compare with the point where the branch started: `--base "$(git merge-base origin/main HEAD)"`.

To catch changes before they are committed, run the check on the staged changes in a pre-commit hook, `.git/hooks/pre-commit`, made executable:

```sh
#!/bin/sh
exec dadrock validate changes --staged
```

Git runs hooks from the repository root. If the project lives in a subdirectory, such as `analysis/dadr/`, change into that subdirectory first: `cd analysis && exec dadrock validate changes --staged`.

## Web interface

```sh
dadrock preview
dadrock preview --port 4005 path/to/project
```

Open the local URL printed in the terminal. The interface has three places, side by side in the header: Inbox, Ledger, and Draft.

### Inbox

The Inbox is the home screen and the work queue. Everything that needs a human lands here, grouped by kind:

- **Candidates** wait for a decision. Read the record, fix its explanation if needed, then **Accept** or **Reject**. **Revise with agent** sends it back to a local agent with your instructions.
- **Agent proposals** are decisions a local agent drafted from your notes or your code. **Save as candidate** writes the file so you can review it like any other candidate; **Discard** drops it.
- **Proposed revisions** show an agent's edit to an existing record as a diff. **Apply** writes it; **Discard** drops it.
- **Conflicts** come from **Check for conflicts**, which asks the agent to look for overlapping or contradictory decisions. From a conflict you can **Merge** two candidates, **Supersede** an accepted decision, or **Dismiss** the finding.
- **Issues** are validation findings: structure, links, tags, and project layout. Validation runs on every load, so there is nothing to trigger. **Open record** jumps to the editor; **Fix with agent** asks for a revision; warnings and notes can be dismissed.

Review both the observed choice and its rationale. An agent-generated description of code may be accurate while the reason for that choice remains unknown. Supply the missing rationale before accepting.

An empty Inbox is the goal. The search box narrows the queue to matching titles, tags, paths, and messages. Use `j` and `k` to move through items, `a` and `r` for the primary and secondary action, `e` to edit, and `o` to open the record in the Ledger.

Record text is edited in a CodeMirror editor with Markdown and front-matter highlighting, search (`Ctrl+F`), and `Ctrl+S` to save.

### Ledger

The Ledger is what has been decided. It lists the decisions in force, newest first, with one search box and a tag facet. **Show archived** adds the archived records: rejected and superseded ones. Each record shows its supersession chain, an inline editor, and two verbs: **Supersede** drafts a successor, and **Revise with agent** asks a local agent to rewrite the record from your instructions. The result appears in the Inbox as a proposed revision.

### Draft

**Draft** is the third place: a full page reached from the header or from any record. Describe the decision in the text box, or attach files and folders to have a [configured agent](llms.md) look for decisions in your code. Attach two candidates to merge them. Opened from a record, the same page revises that record or drafts its successor. Results land in the Inbox as proposals.

Without an agent, **Write it by hand** gives you a plain form with the candidate template. **Supersede with existing decisions instead** links an accepted record to successors that already exist.

### Project

The project name in the header opens a small panel with the project root, the detected agent, and three actions: initialize a new project, run validation now, and clear the agent runs. Local run state is stored under `dadr/runs/` and ignored by default because it may contain pasted context, source excerpts, and model transcripts.

Old links still work: `/records`, `/candidates/new`, `/tasks`, `/validation`, `/tags`, and `/settings` redirect to the Inbox, the Ledger, or the Draft page.

## Editor integration

The repository contains one extension implementation in `editors/vscode`. It provides Dadrock integration for VS Code, with a build/install target for Positron using the same extension source; see [Install](install.md#editor-integration). This does not imply published marketplace packages or separate official extensions for Cursor, RStudio, or Neovim. You can edit DADR Markdown files in any editor and run `dadrock preview` alongside it.

## LLM agents

Agents are optional. Once a local agent is [configured](llms.md), start with a small batch of consequential choices:

```sh
dadrock task run code-to-record --path analysis/ --limit 5
dadrock task run context-to-record --context "Document why we exclude rows without treatment assignment"
```

Agent outputs are proposals for review. Code can show what happened without establishing why; correct unsupported claims and supply missing rationale before acceptance. The web interface offers the same drafting actions. See [LLM agents](llms.md) for provider configuration, custom tasks, and MCP integration.

## Command-line reference

Generated from the CLI definition by `make cli-docs`; it mirrors `dadrock help`.

<!-- cli-docs:start -->

**Command Overview:**

* [`dadrock`↴](#dadrock)
* [`dadrock help`↴](#dadrock-help)
* [`dadrock commands`↴](#dadrock-commands)
* [`dadrock completions`↴](#dadrock-completions)
* [`dadrock dir`↴](#dadrock-dir)
* [`dadrock config`↴](#dadrock-config)
* [`dadrock init`↴](#dadrock-init)
* [`dadrock demo`↴](#dadrock-demo)
* [`dadrock new`↴](#dadrock-new)
* [`dadrock list`↴](#dadrock-list)
* [`dadrock status`↴](#dadrock-status)
* [`dadrock status accept`↴](#dadrock-status-accept)
* [`dadrock status reject`↴](#dadrock-status-reject)
* [`dadrock status supersede`↴](#dadrock-status-supersede)
* [`dadrock accept`↴](#dadrock-accept)
* [`dadrock reject`↴](#dadrock-reject)
* [`dadrock supersede`↴](#dadrock-supersede)
* [`dadrock validate`↴](#dadrock-validate)
* [`dadrock validate records`↴](#dadrock-validate-records)
* [`dadrock validate links`↴](#dadrock-validate-links)
* [`dadrock validate tags`↴](#dadrock-validate-tags)
* [`dadrock validate changes`↴](#dadrock-validate-changes)
* [`dadrock tags`↴](#dadrock-tags)
* [`dadrock tags list`↴](#dadrock-tags-list)
* [`dadrock mcp`↴](#dadrock-mcp)
* [`dadrock mcp serve`↴](#dadrock-mcp-serve)
* [`dadrock agent`↴](#dadrock-agent)
* [`dadrock agent check`↴](#dadrock-agent-check)
* [`dadrock agent draft`↴](#dadrock-agent-draft)
* [`dadrock agent skills`↴](#dadrock-agent-skills)
* [`dadrock task`↴](#dadrock-task)
* [`dadrock task list`↴](#dadrock-task-list)
* [`dadrock task show`↴](#dadrock-task-show)
* [`dadrock task run`↴](#dadrock-task-run)
* [`dadrock task runs`↴](#dadrock-task-runs)
* [`dadrock task result`↴](#dadrock-task-result)
* [`dadrock preview`↴](#dadrock-preview)

### `dadrock`

DADR decision records and web review UI.

With no command, prints this help menu.

**Usage:** `dadrock [COMMAND]`

###### **Subcommands:**

* `help` — Show top-level help or detailed metadata for one command.
* `commands` — List command metadata for automation and command discovery.
* `completions` — Generate a shell completion script.
* `dir` — Print the discovered Dadrock project directory (dadr/).
* `config` — Print the resolved project layout.
* `init` — Create a Dadrock dadr/ directory.
* `demo` — Seed a new project with a named DADR example.
* `new` — Create a candidate decision record from the active candidate template.
* `list` — List records, optionally filtered by lifecycle status, tag, or query.
* `status` — Inspect one record's lifecycle status or run lifecycle transitions.
* `accept` — Accept a candidate decision.
* `reject` — Reject a candidate decision.
* `supersede` — Link accepted successors to the records they supersede and move those records to the archive.
* `validate` — Run project validation, or target records, links, tags, or changes.
* `tags` — Inspect DADR tag usage.
* `mcp` — Serve Dadrock MCP tools over stdio.
* `agent` — Call a configured local ACP agent for candidate drafting.
* `task` — Discover, inspect, and run Dadrock task packages.
* `preview` — Serve the local web review UI.

### `dadrock help`

Show top-level help or detailed metadata for one command.

**Usage:** `dadrock help [TOPIC]...`

###### **Arguments:**

* `<TOPIC>`

### `dadrock commands`

List command metadata for automation and command discovery.

**Usage:** `dadrock commands [OPTIONS]`

###### **Options:**

* `--json`

### `dadrock completions`

Generate a shell completion script.

**Usage:** `dadrock completions <SHELL>`

###### **Arguments:**

* `<SHELL>`

  Possible values: `bash`, `zsh`, `fish`

### `dadrock dir`

Print the discovered Dadrock project directory (dadr/).

**Usage:** `dadrock dir`

### `dadrock config`

Print the resolved project layout.

**Usage:** `dadrock config [OPTIONS]`

###### **Options:**

* `--json`

### `dadrock init`

Create a Dadrock dadr/ directory.

**Usage:** `dadrock init [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>`

### `dadrock demo`

Seed a new project with a named DADR example.

**Usage:** `dadrock demo [OPTIONS] [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>`

###### **Options:**

* `--json`
* `--example <EXAMPLE>`

  Default value: `lifecycle`

  Possible values: `lifecycle`, `candidate-review`, `penguins`, `broken`, `conflicts`, `task-history`

### `dadrock new`

Create a candidate decision record from the active candidate template.

**Usage:** `dadrock new [OPTIONS] <TITLE>...`

###### **Arguments:**

* `<TITLE>`

###### **Options:**

* `--tag <TAGS>`
* `--quiet`

### `dadrock list`

List records, optionally filtered by lifecycle status, tag, or query.

**Usage:** `dadrock list [OPTIONS]`

###### **Options:**

* `--status <STATUS>`

  Possible values: `candidate`, `accepted`, `rejected`, `superseded`

* `--tag <TAG>`
* `--query <QUERY>`
* `--unique`

### `dadrock status`

Inspect one record's lifecycle status or run lifecycle transitions.

**Usage:** `dadrock status [QUERY]... [COMMAND]`

###### **Subcommands:**

* `accept` — Accept a candidate and move it to the ledger.
* `reject` — Reject a candidate and move it to the archive.
* `supersede` — Link accepted successors to the records they supersede and move those records to the archive.

###### **Arguments:**

* `<QUERY>`

### `dadrock status accept`

Accept a candidate and move it to the ledger.

**Usage:** `dadrock status accept <QUERY>...`

###### **Arguments:**

* `<QUERY>`

### `dadrock status reject`

Reject a candidate and move it to the archive.

**Usage:** `dadrock status reject <QUERY>...`

###### **Arguments:**

* `<QUERY>`

### `dadrock status supersede`

Link accepted successors to the records they supersede and move those records to the archive.

**Usage:** `dadrock status supersede [OPTIONS]`

###### **Options:**

* `--from <FROM>`
* `--to <TO>`

### `dadrock accept`

Accept a candidate decision.

**Usage:** `dadrock accept <QUERY>...`

###### **Arguments:**

* `<QUERY>`

### `dadrock reject`

Reject a candidate decision.

**Usage:** `dadrock reject <QUERY>...`

###### **Arguments:**

* `<QUERY>`

### `dadrock supersede`

Link accepted successors to the records they supersede and move those records to the archive.

**Usage:** `dadrock supersede [OPTIONS]`

###### **Options:**

* `--from <FROM>`
* `--to <TO>`

### `dadrock validate`

Run project validation, or target records, links, tags, or changes.

**Usage:** `dadrock validate [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `records` — Validate record front matter and structure.
* `links` — Report broken or ambiguous wikilinks and local Markdown links.
* `tags` — Report malformed YAML and inline tags.
* `changes` — Check that accepted and rejected records changed since a Git revision only where allowed.

###### **Options:**

* `--json`

### `dadrock validate records`

Validate record front matter and structure.

**Usage:** `dadrock validate records [OPTIONS] [PATHS]...`

###### **Arguments:**

* `<PATHS>`

###### **Options:**

* `--json`

### `dadrock validate links`

Report broken or ambiguous wikilinks and local Markdown links.

**Usage:** `dadrock validate links [OPTIONS]`

###### **Options:**

* `--json`

### `dadrock validate tags`

Report malformed YAML and inline tags.

**Usage:** `dadrock validate tags [OPTIONS]`

###### **Options:**

* `--json`

### `dadrock validate changes`

Check that accepted and rejected records changed since a Git revision only where allowed.

**Usage:** `dadrock validate changes [OPTIONS]`

###### **Options:**

* `--base <BASE>` — Git revision to compare with

  Default value: `HEAD`
* `--staged` — Compare the index (staged changes) instead of the working tree
* `--json`

### `dadrock tags`

Inspect DADR tag usage.

**Usage:** `dadrock tags <COMMAND>`

###### **Subcommands:**

* `list` — List normalized YAML and inline tags.

### `dadrock tags list`

List normalized YAML and inline tags.

**Usage:** `dadrock tags list [OPTIONS]`

###### **Options:**

* `--json`

### `dadrock mcp`

Serve Dadrock MCP tools over stdio.

**Usage:** `dadrock mcp <COMMAND>`

###### **Subcommands:**

* `serve` — Serve Dadrock MCP tools over stdio.

### `dadrock mcp serve`

Serve Dadrock MCP tools over stdio.

**Usage:** `dadrock mcp serve [OPTIONS] [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>`

###### **Options:**

* `--read-only`

### `dadrock agent`

Call a configured local ACP agent for candidate drafting.

**Usage:** `dadrock agent <COMMAND>`

###### **Subcommands:**

* `check` — Check that a local ACP agent can start a Dadrock session.
* `draft` — Ask a local ACP agent to draft a candidate decision.
* `skills` — List available Dadrock agent skills and prompt packs.

### `dadrock agent check`

Check that a local ACP agent can start a Dadrock session.

**Usage:** `dadrock agent check [OPTIONS]`

###### **Options:**

* `--agent <AGENT>`
* `--agent-arg <AGENT_ARGS>`
* `--no-mcp`

### `dadrock agent draft`

Ask a local ACP agent to draft a candidate decision.

**Usage:** `dadrock agent draft [OPTIONS] <PROMPT>...`

###### **Arguments:**

* `<PROMPT>`

###### **Options:**

* `--write`
* `--json`
* `--permission <PERMISSION>`

  Default value: `deny`

  Possible values: `deny`, `allow-once`, `allow-always`

* `--agent <AGENT>`
* `--agent-arg <AGENT_ARGS>`
* `--skill <SKILLS>` \[alias: `prompt-pack`\]
* `--no-mcp`

### `dadrock agent skills`

List available Dadrock agent skills and prompt packs.

**Usage:** `dadrock agent skills [OPTIONS]`

###### **Options:**

* `--json`

### `dadrock task`

Discover, inspect, and run Dadrock task packages.

**Usage:** `dadrock task <COMMAND>`

###### **Subcommands:**

* `list` — List built-in, user, and project task packages.
* `show` — Show one resolved task package.
* `run` — Run one resolved task package with a local ACP agent.
* `runs` — List persisted local task runs.
* `result` — Read a persisted task run result.

### `dadrock task list`

List built-in, user, and project task packages.

**Usage:** `dadrock task list [OPTIONS]`

###### **Options:**

* `--json`

### `dadrock task show`

Show one resolved task package.

**Usage:** `dadrock task show [OPTIONS] <TASK_ID>`

###### **Arguments:**

* `<TASK_ID>`

###### **Options:**

* `--json`

### `dadrock task run`

Run one resolved task package with a local ACP agent.

**Usage:** `dadrock task run [OPTIONS] <TASK_ID>`

###### **Arguments:**

* `<TASK_ID>`

###### **Options:**

* `--input-json <INPUT_JSON>`
* `--path <PATH>` — File or directory to inspect
* `--context <CONTEXT>` — Describe the decision to capture
* `--focus <FOCUS>` — Focus the code review on a topic
* `--limit <LIMIT>` — Maximum initial candidates (code-to-record defaults to 5)
* `--json`
* `--permission <PERMISSION>`

  Default value: `deny`

  Possible values: `deny`, `allow-once`, `allow-always`

* `--agent <AGENT>`
* `--agent-arg <AGENT_ARGS>`
* `--no-mcp`

### `dadrock task runs`

List persisted local task runs.

**Usage:** `dadrock task runs [OPTIONS]`

###### **Options:**

* `--json`

### `dadrock task result`

Read a persisted task run result.

**Usage:** `dadrock task result [OPTIONS] <RUN_ID>`

###### **Arguments:**

* `<RUN_ID>`

###### **Options:**

* `--json`

### `dadrock preview`

Serve the local web review UI.

**Usage:** `dadrock preview [OPTIONS] [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>`

###### **Options:**

* `--port <PORT>`

  Default value: `4004`
* `--host <HOST>`

  Default value: `127.0.0.1`

<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>

<!-- cli-docs:end -->
