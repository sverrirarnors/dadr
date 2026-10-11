# Command line

<!-- Generated from the CLI definition by `make cli-docs`; it mirrors `dadrock <command> --help`. -->

<!-- cli-docs:start -->

## Commands

**Projects**

| Command | Description |
|---|---|
| [`init`](#dadrock-init) | Create a Dadrock dadr/ directory. |
| [`demo`](#dadrock-demo) | Seed a new project with a named DADR example. |
| [`preview`](#dadrock-preview) | Serve the local web review UI. |
| [`config`](#dadrock-config) | Print the resolved project layout. |
| [`dir`](#dadrock-dir) | Print the discovered Dadrock project directory (dadr/). |

**Records**

| Command | Description |
|---|---|
| [`new`](#dadrock-new) | Create a candidate decision record from the active candidate template. |
| [`list`](#dadrock-list) | List records, optionally filtered by lifecycle status, tag, or query. |
| [`history`](#dadrock-history) | Show a decision's versions as a graph, newest first, with the reason for each change and each version's text edits from Git. |
| [`diff`](#dadrock-diff) | Compare two records: title, tags, and body without `## Supersedes`. |
| [`accept`](#dadrock-accept) | Accept a candidate decision, or several together with --with. |
| [`reject`](#dadrock-reject) | Reject a candidate decision. |
| [`supersede`](#dadrock-supersede) | Link accepted successors to the records they supersede and move those records to the archive. |
| [`status`](#dadrock-status) | Inspect one record's lifecycle status or run lifecycle transitions. |

**Validation**

| Command | Description |
|---|---|
| [`validate`](#dadrock-validate) | Run project validation, or target records, links, tags, or changes. |
| [`tags`](#dadrock-tags) | Inspect DADR tag usage. |

**Agents**

| Command | Description |
|---|---|
| [`agent`](#dadrock-agent) | Call a configured local ACP agent for candidate drafting. |
| [`task`](#dadrock-task) | Discover, inspect, and run Dadrock task packages. |
| [`mcp`](#dadrock-mcp) | Serve Dadrock MCP tools over stdio. |

**Help**

| Command | Description |
|---|---|
| [`help`](#dadrock-help) | Show top-level help or detailed metadata for one command. |
| [`commands`](#dadrock-commands) | List command metadata for automation and command discovery. |
| [`completions`](#dadrock-completions) | Generate a shell completion script. |

## `dadrock init`

Create a Dadrock dadr/ directory.

**Usage:** `dadrock init [DIRECTORY]`

**Arguments**

* `<DIRECTORY>`

## `dadrock demo`

Seed a new project with a named DADR example.

**Usage:** `dadrock demo [OPTIONS] [DIRECTORY]`

**Arguments**

* `<DIRECTORY>`

**Options**

* `--json`
* `--example <EXAMPLE>`

  Default value: `lifecycle`

  Possible values: `lifecycle`, `candidate-review`, `penguins`, `broken`, `conflicts`, `task-history`

## `dadrock preview`

Serve the local web review UI.

**Usage:** `dadrock preview [OPTIONS] [DIRECTORY]`

**Arguments**

* `<DIRECTORY>`

**Options**

* `--port <PORT>`

  Default value: `4004`
* `--host <HOST>`

  Default value: `127.0.0.1`

## `dadrock config`

Print the resolved project layout.

**Usage:** `dadrock config [OPTIONS]`

**Options**

* `--json`

## `dadrock dir`

Print the discovered Dadrock project directory (dadr/).

**Usage:** `dadrock dir`

## `dadrock new`

Create a candidate decision record from the active candidate template.

**Usage:** `dadrock new [OPTIONS] <TITLE>...`

**Arguments**

* `<TITLE>`

**Options**

* `--tag <TAGS>`
* `--quiet`

## `dadrock list`

List records, optionally filtered by lifecycle status, tag, or query.

**Usage:** `dadrock list [OPTIONS]`

**Options**

* `--status <STATUS>`

  Possible values: `candidate`, `accepted`, `rejected`, `superseded`

* `--tag <TAG>`
* `--query <QUERY>`
* `--unique`

## `dadrock history`

Show a decision's versions as a graph, newest first, with the reason for each change and each version's text edits from Git.

**Usage:** `dadrock history [OPTIONS] <QUERY>...`

**Arguments**

* `<QUERY>` — Any version of the decision: full id, id prefix, filename, or title

**Options**

* `--patch` — Print each change's diff under its link, and each text edit's diff under its version
* `--json`

## `dadrock diff`

Compare two records: title, tags, and body without `## Supersedes`.

**Usage:** `dadrock diff [OPTIONS] <OLD> <NEW>`

**Arguments**

* `<OLD>` — The record to compare from: full id, id prefix, filename, or title
* `<NEW>` — The record to compare to

**Options**

* `--json`

## `dadrock accept`

Accept a candidate decision, or several together with --with.

**Usage:** `dadrock accept [OPTIONS] <QUERY>...`

**Arguments**

* `<QUERY>` — The candidate to accept; the words join into one query

**Options**

* `--with <QUERY>` — Another candidate to accept in the same operation, such as the other half of a split. Repeat for more

## `dadrock reject`

Reject a candidate decision.

**Usage:** `dadrock reject <QUERY>...`

**Arguments**

* `<QUERY>`

## `dadrock supersede`

Link accepted successors to the records they supersede and move those records to the archive.

**Usage:** `dadrock supersede [OPTIONS]`

**Options**

* `--from <FROM>`
* `--to <TO>`

## `dadrock status`

Inspect one record's lifecycle status or run lifecycle transitions.

**Usage:** `dadrock status [QUERY]... [COMMAND]`

**Subcommands**

* [`accept`](#dadrock-status-accept) — Accept a candidate and move it to the ledger, or several together with --with.
* [`reject`](#dadrock-status-reject) — Reject a candidate and move it to the archive.
* [`supersede`](#dadrock-status-supersede) — Link accepted successors to the records they supersede and move those records to the archive.

**Arguments**

* `<QUERY>`

### `dadrock status accept`

Accept a candidate and move it to the ledger, or several together with --with.

**Usage:** `dadrock status accept [OPTIONS] <QUERY>...`

**Arguments**

* `<QUERY>` — The candidate to accept; the words join into one query

**Options**

* `--with <QUERY>` — Another candidate to accept in the same operation, such as the other half of a split. Repeat for more

### `dadrock status reject`

Reject a candidate and move it to the archive.

**Usage:** `dadrock status reject <QUERY>...`

**Arguments**

* `<QUERY>`

### `dadrock status supersede`

Link accepted successors to the records they supersede and move those records to the archive.

**Usage:** `dadrock status supersede [OPTIONS]`

**Options**

* `--from <FROM>`
* `--to <TO>`

## `dadrock validate`

Run project validation, or target records, links, tags, or changes.

**Usage:** `dadrock validate [OPTIONS] [COMMAND]`

**Subcommands**

* [`records`](#dadrock-validate-records) — Validate record front matter and structure.
* [`links`](#dadrock-validate-links) — Report broken or ambiguous wikilinks and local Markdown links.
* [`tags`](#dadrock-validate-tags) — Report malformed YAML and inline tags.
* [`changes`](#dadrock-validate-changes) — Check accepted and rejected records against a Git revision: a changed id or status or a removed successor is an error; a title or body edit is a warning with the diff.

**Options**

* `--json`

### `dadrock validate records`

Validate record front matter and structure.

**Usage:** `dadrock validate records [OPTIONS] [PATHS]...`

**Arguments**

* `<PATHS>`

**Options**

* `--json`

### `dadrock validate links`

Report broken or ambiguous wikilinks and local Markdown links.

**Usage:** `dadrock validate links [OPTIONS]`

**Options**

* `--json`

### `dadrock validate tags`

Report malformed YAML and inline tags.

**Usage:** `dadrock validate tags [OPTIONS]`

**Options**

* `--json`

### `dadrock validate changes`

Check accepted and rejected records against a Git revision: a changed id or status or a removed successor is an error; a title or body edit is a warning with the diff.

**Usage:** `dadrock validate changes [OPTIONS]`

**Options**

* `--base <BASE>` — Git revision to compare with

  Default value: `HEAD`
* `--staged` — Compare the index (staged changes) instead of the working tree
* `--strict` — Report title and body edits to accepted and rejected records as errors
* `--json`

## `dadrock tags`

Inspect DADR tag usage.

**Usage:** `dadrock tags <COMMAND>`

**Subcommands**

* [`list`](#dadrock-tags-list) — List normalized YAML and inline tags.

### `dadrock tags list`

List normalized YAML and inline tags.

**Usage:** `dadrock tags list [OPTIONS]`

**Options**

* `--json`

## `dadrock agent`

Call a configured local ACP agent for candidate drafting.

**Usage:** `dadrock agent <COMMAND>`

**Subcommands**

* [`check`](#dadrock-agent-check) — Check that a local ACP agent can start a Dadrock session.
* [`draft`](#dadrock-agent-draft) — Ask a local ACP agent to draft a candidate decision.
* [`skills`](#dadrock-agent-skills) — List available Dadrock agent skills and prompt packs.

### `dadrock agent check`

Check that a local ACP agent can start a Dadrock session.

**Usage:** `dadrock agent check [OPTIONS]`

**Options**

* `--agent <AGENT>`
* `--agent-arg <AGENT_ARGS>`
* `--no-mcp`

### `dadrock agent draft`

Ask a local ACP agent to draft a candidate decision.

**Usage:** `dadrock agent draft [OPTIONS] <PROMPT>...`

**Arguments**

* `<PROMPT>`

**Options**

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

**Options**

* `--json`

## `dadrock task`

Discover, inspect, and run Dadrock task packages.

**Usage:** `dadrock task <COMMAND>`

**Subcommands**

* [`list`](#dadrock-task-list) — List built-in, user, and project task packages.
* [`show`](#dadrock-task-show) — Show one resolved task package.
* [`run`](#dadrock-task-run) — Run one resolved task package with a local ACP agent.
* [`runs`](#dadrock-task-runs) — List persisted local task runs.
* [`result`](#dadrock-task-result) — Read a persisted task run result.

### `dadrock task list`

List built-in, user, and project task packages.

**Usage:** `dadrock task list [OPTIONS]`

**Options**

* `--json`

### `dadrock task show`

Show one resolved task package.

**Usage:** `dadrock task show [OPTIONS] <TASK_ID>`

**Arguments**

* `<TASK_ID>`

**Options**

* `--json`
* `--raw` — Print the task file exactly as written, front matter included

### `dadrock task run`

Run one resolved task package with a local ACP agent.

**Usage:** `dadrock task run [OPTIONS] <TASK_ID>`

**Arguments**

* `<TASK_ID>`

**Options**

* `--input-json <INPUT_JSON>`
* `--path <PATH>` — File or directory to inspect
* `--context <CONTEXT>` — Describe the decision to capture
* `--focus <FOCUS>` — Topic to concentrate on (code-to-record, decisions-to-spec)
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

**Options**

* `--json`

### `dadrock task result`

Read a persisted task run result.

**Usage:** `dadrock task result [OPTIONS] <RUN_ID>`

**Arguments**

* `<RUN_ID>`

**Options**

* `--json`

## `dadrock mcp`

Serve Dadrock MCP tools over stdio.

**Usage:** `dadrock mcp <COMMAND>`

**Subcommands**

* [`serve`](#dadrock-mcp-serve) — Serve Dadrock MCP tools over stdio.

### `dadrock mcp serve`

Serve Dadrock MCP tools over stdio.

**Usage:** `dadrock mcp serve [OPTIONS] [DIRECTORY]`

**Arguments**

* `<DIRECTORY>`

**Options**

* `--read-only`

## `dadrock help`

Show top-level help or detailed metadata for one command.

**Usage:** `dadrock help [TOPIC]...`

**Arguments**

* `<TOPIC>`

## `dadrock commands`

List command metadata for automation and command discovery.

**Usage:** `dadrock commands [OPTIONS]`

**Options**

* `--json`

## `dadrock completions`

Generate a shell completion script.

**Usage:** `dadrock completions <SHELL>`

**Arguments**

* `<SHELL>`

  Possible values: `bash`, `zsh`, `fish`

<!-- cli-docs:end -->
