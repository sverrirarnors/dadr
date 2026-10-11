# Command line

`dadrock` is one command with subcommands. Running it without a subcommand prints help; `dadrock help COMMAND` describes one command, and `dadrock completions bash|zsh|fish` prints a shell completion script.

## Project layout

The DADR format does not mandate a directory structure. **DADRock** uses this one, created by `dadrock init`:

```text
dadr/
  accepted/
    01938b8e4d70-income-preprocessing.md
    01938b8f9d71-drop-missing-treatment.md
  candidates/
    01938ba1c4f3-country-exclusions.md
  rejected/
    01938b8d9c10-deprecated-cohort-filter.md
  templates/
    candidate.md                                    # optional
  tasks/
  runs/
    .gitignore
```

| Directory | Contents |
|---|---|
| `dadr/candidates/` | Proposals awaiting review |
| `dadr/accepted/` | Accepted decisions, including those later replaced |
| `dadr/rejected/` | Rejected proposals |
| `dadr/templates/` | Optional template overrides for new candidates |
| `dadr/tasks/` | Optional project-specific agent workflows (`dadr/tasks/<task-id>/task.toml` plus its prompt file) |
| `dadr/runs/` | Agent-run inputs, results, and logs; ignored by version control |

`dadrock accept` changes a candidate's `status` to `accepted` and moves the file to `dadr/accepted/`. `dadrock reject` changes it to `rejected` and moves it to `dadr/rejected/`. The `id` never changes. `dadrock supersede` appends the new record's `id` to the old record's `successors` and leaves both files in `dadr/accepted/`.

The directory is a convenience: `status` in the front matter is authoritative. `dadrock validate` warns when a record sits in a directory that does not match its status. Use `dadrock config` to inspect the resolved layout. Keep the record directories in version control; run state may contain pasted context and source excerpts and is ignored by default.

## Validation

```sh
dadrock validate
dadrock validate --json
```

Current validation checks:

- no `date` key or ISO 8601 date/time values in front matter;
- YAML fields, UUIDv7 IDs, four sections each appearing exactly once with text, and status/location consistency;
- duplicate full IDs and short IDs, and filename collisions;
- missing or invalid successors and cycles in replacement relationships;
- broken or ambiguous record wikilinks and missing local Markdown-link targets;
- malformed tags and unreadable candidate templates.

Focused commands are `dadrock validate records`, `dadrock validate links`, and `dadrock validate tags`. Use labeled links such as `[[01938c20a3b4|sampling rule]]` to make raw Markdown easier to read.

Validation checks the records' structure and internal references. It does not compare accepted records with Git history, resolve bibliography citations, detect `TBD` prose, enforce plan-deviation sections, or determine whether code follows a decision. Those checks are [roadmap work](../roadmap.md). Human review remains necessary.

## Continuous integration

Once DADRock is installed in your CI environment, run `dadrock validate` from the analysis project root. A nonzero exit status signals an error; `--json` provides structured diagnostics. Install from a pinned source revision for reproducibility; a CLI-only CI build can use the [`no-js-build` feature](index.md#without-the-web-interface).

## Reference

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
* `dir` — Print the discovered accepted records directory.
* `config` — Print the resolved project layout.
* `init` — Create a Dadrock dadr/ directory.
* `demo` — Seed a new project with a named DADR example.
* `new` — Create a candidate decision record from the active candidate template.
* `list` — List records, optionally filtered by lifecycle status, tag, or query.
* `status` — Inspect one record's lifecycle status or run lifecycle transitions.
* `accept` — Accept a candidate decision.
* `reject` — Reject a candidate decision.
* `supersede` — Link replacement decisions to the decisions they replace.
* `validate` — Run project validation, or target records, links, or tags.
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

Print the discovered accepted records directory.

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

* `accept` — Accept a candidate by converting it into an accepted decision.
* `reject` — Reject a candidate and move it into the rejected records directory.
* `supersede` — Link accepted successors to the records they supersede.

###### **Arguments:**

* `<QUERY>`

### `dadrock status accept`

Accept a candidate by converting it into an accepted decision.

**Usage:** `dadrock status accept <QUERY>...`

###### **Arguments:**

* `<QUERY>`

### `dadrock status reject`

Reject a candidate and move it into the rejected records directory.

**Usage:** `dadrock status reject <QUERY>...`

###### **Arguments:**

* `<QUERY>`

### `dadrock status supersede`

Link accepted successors to the records they supersede.

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

Link replacement decisions to the decisions they replace.

**Usage:** `dadrock supersede [OPTIONS]`

###### **Options:**

* `--from <FROM>`
* `--to <TO>`

### `dadrock validate`

Run project validation, or target records, links, or tags.

**Usage:** `dadrock validate [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `records` — Validate record front matter and structure.
* `links` — Report broken or ambiguous wikilinks and local Markdown links.
* `tags` — Report malformed YAML and inline tags.

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
