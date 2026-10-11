# Command-Line Help for `dadrock`

This document contains the help content for the `dadrock` command-line program.

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
* [`dadrock history`↴](#dadrock-history)
* [`dadrock diff`↴](#dadrock-diff)
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

## `dadrock`

DADR decision records and web review UI.

With no command, prints this help menu.

**Usage:** `dadrock [COMMAND]`

###### **Subcommands:**

* `help` — Show top-level help or detailed metadata for one command.
* `commands` — List command metadata for automation and command discovery.
* `completions` — Generate a shell completion script.
* `dir` — Print the project's dadr/ directory, even before it exists.
* `config` — Print the resolved project layout.
* `init` — Create a Dadrock dadr/ directory.
* `demo` — Seed a new project with a named DADR example.
* `new` — Create a candidate decision record from the active candidate template.
* `list` — List records, optionally filtered by lifecycle status, tag, or query.
* `history` — Show a decision's versions as a graph, newest first, with the reason for each change and each version's text edits from Git.
* `diff` — Compare two records: title, tags, and body without `## Supersedes`.
* `status` — Inspect one record's lifecycle status or run lifecycle transitions.
* `accept` — Accept a candidate decision, or several together with --with.
* `reject` — Reject a candidate decision.
* `supersede` — Link accepted successors to the records they supersede and move those records to the archive.
* `validate` — Run project validation, or target records, links, tags, or changes.
* `tags` — Inspect DADR tag usage.
* `mcp` — Serve Dadrock MCP tools over stdio.
* `agent` — Call a configured local ACP agent for candidate drafting.
* `task` — Discover, inspect, and run Dadrock task packages.
* `preview` — Serve the local web review UI.



## `dadrock help`

Show top-level help or detailed metadata for one command.

**Usage:** `dadrock help [TOPIC]...`

###### **Arguments:**

* `<TOPIC>` — The command to describe, such as `validate changes`



## `dadrock commands`

List command metadata for automation and command discovery.

**Usage:** `dadrock commands [OPTIONS]`

###### **Options:**

* `--json` — Print the JSON envelope instead of text



## `dadrock completions`

Generate a shell completion script.

**Usage:** `dadrock completions <SHELL>`

###### **Arguments:**

* `<SHELL>` — The shell to generate the script for

  Possible values: `bash`, `zsh`, `fish`




## `dadrock dir`

Print the project's dadr/ directory, even before it exists.

**Usage:** `dadrock dir`



## `dadrock config`

Print the resolved project layout.

**Usage:** `dadrock config [OPTIONS]`

###### **Options:**

* `--json` — Print the JSON envelope instead of text



## `dadrock init`

Create a Dadrock dadr/ directory.

**Usage:** `dadrock init [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>` — Where to create `dadr/`: its parent directory, or a path ending in `dadr` (defaults to the current directory)



## `dadrock demo`

Seed a new project with a named DADR example.

**Usage:** `dadrock demo [OPTIONS] [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>` — The directory to seed, which must be new or empty (defaults to the current directory)

###### **Options:**

* `--json` — Print the JSON envelope instead of text
* `--example <EXAMPLE>` — The example project to seed

  Default value: `lifecycle`

  Possible values: `lifecycle`, `candidate-review`, `penguins`, `broken`, `conflicts`, `task-history`




## `dadrock new`

Create a candidate decision record from the active candidate template.

**Usage:** `dadrock new [OPTIONS] <TITLE>...`

###### **Arguments:**

* `<TITLE>` — The record's title; the words join into one title. Options can come before or after it; put `--` before a title that starts with `-`

###### **Options:**

* `--tag <TAG>` — A tag for the record, without `#`; repeat for more
* `--quiet` — Print only the new record's id instead of the JSON envelope



## `dadrock list`

List records, optionally filtered by lifecycle status, tag, or query.

**Usage:** `dadrock list [OPTIONS]`

###### **Options:**

* `--status <STATUS>` — Only records with this status; `accepted` leaves out superseded records, and `superseded` lists only them

  Possible values: `candidate`, `accepted`, `rejected`, `superseded`

* `--tag <TAG>` — Only records with this tag, in front matter or inline (the `#` is optional)
* `--query <QUERY>` — Only records matching a full id, an id prefix, or part of a filename or title
* `--unique` — Exit non-zero unless exactly one record matches



## `dadrock history`

Show a decision's versions as a graph, newest first, with the reason for each change and each version's text edits from Git.

**Usage:** `dadrock history [OPTIONS] <QUERY>...`

###### **Arguments:**

* `<QUERY>` — Any version of the decision: full id, id prefix, filename, or title

###### **Options:**

* `--patch` — Print each change's diff under its link, and each text edit's diff under its version
* `--json` — Print the JSON envelope instead of text



## `dadrock diff`

Compare two records: title, tags, and body without `## Supersedes`.

**Usage:** `dadrock diff [OPTIONS] <OLD> <NEW>`

###### **Arguments:**

* `<OLD>` — The record to compare from: full id, id prefix, filename, or title
* `<NEW>` — The record to compare to

###### **Options:**

* `--json` — Print the JSON envelope instead of text



## `dadrock status`

Inspect one record's lifecycle status or run lifecycle transitions.

**Usage:** `dadrock status [QUERY]... [COMMAND]`

###### **Subcommands:**

* `accept` — Accept a candidate and move it to the ledger, or several together with --with.
* `reject` — Reject a candidate and move it to the archive.
* `supersede` — Link accepted successors to the records they supersede and move those records to the archive.

###### **Arguments:**

* `<QUERY>` — The record to inspect: full id, id prefix, filename, or title; the words join into one query



## `dadrock status accept`

Accept a candidate and move it to the ledger, or several together with --with.

**Usage:** `dadrock status accept [OPTIONS] <QUERY>...`

###### **Arguments:**

* `<QUERY>` — The candidate to accept; the words join into one query

###### **Options:**

* `--with <QUERY>` — Another candidate to accept in the same operation, such as the other half of a split. Repeat for more



## `dadrock status reject`

Reject a candidate and move it to the archive.

**Usage:** `dadrock status reject <QUERY>...`

###### **Arguments:**

* `<QUERY>` — The candidate: full id, id prefix, filename, or title; the words join into one query



## `dadrock status supersede`

Link accepted successors to the records they supersede and move those records to the archive.

**Usage:** `dadrock status supersede [OPTIONS]`

###### **Options:**

* `--from <FROM>` — An accepted record to supersede: full id, id prefix, filename, or title. Repeat for more
* `--to <TO>` — An accepted record that replaces it. Repeat for more; every `--from` record gets every `--to` record as a successor



## `dadrock accept`

Accept a candidate decision, or several together with --with.

**Usage:** `dadrock accept [OPTIONS] <QUERY>...`

###### **Arguments:**

* `<QUERY>` — The candidate to accept; the words join into one query

###### **Options:**

* `--with <QUERY>` — Another candidate to accept in the same operation, such as the other half of a split. Repeat for more



## `dadrock reject`

Reject a candidate decision.

**Usage:** `dadrock reject <QUERY>...`

###### **Arguments:**

* `<QUERY>` — The candidate: full id, id prefix, filename, or title; the words join into one query



## `dadrock supersede`

Link accepted successors to the records they supersede and move those records to the archive.

**Usage:** `dadrock supersede [OPTIONS]`

###### **Options:**

* `--from <FROM>` — An accepted record to supersede: full id, id prefix, filename, or title. Repeat for more
* `--to <TO>` — An accepted record that replaces it. Repeat for more; every `--from` record gets every `--to` record as a successor



## `dadrock validate`

Run project validation, or target records, links, tags, or changes.

**Usage:** `dadrock validate [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `records` — Validate record front matter and structure.
* `links` — Report broken or ambiguous wikilinks and local Markdown links.
* `tags` — Report malformed YAML and inline tags.
* `changes` — Check accepted and rejected records against a Git revision: a changed id or status or a removed successor is an error; a title or body edit is a warning with the diff.

###### **Options:**

* `--json` — Print the project validation's JSON envelope instead of text



## `dadrock validate records`

Validate record front matter and structure.

**Usage:** `dadrock validate records [OPTIONS] [PATHS]...`

###### **Arguments:**

* `<PATHS>` — Record files or directories to validate (defaults to every record in the project)

###### **Options:**

* `--json` — No effect: this command always prints the JSON envelope



## `dadrock validate links`

Report broken or ambiguous wikilinks and local Markdown links.

**Usage:** `dadrock validate links [OPTIONS]`

###### **Options:**

* `--json` — Print the JSON envelope instead of text



## `dadrock validate tags`

Report malformed YAML and inline tags.

**Usage:** `dadrock validate tags [OPTIONS]`

###### **Options:**

* `--json` — Print the JSON envelope instead of text



## `dadrock validate changes`

Check accepted and rejected records against a Git revision: a changed id or status or a removed successor is an error; a title or body edit is a warning with the diff.

**Usage:** `dadrock validate changes [OPTIONS]`

###### **Options:**

* `--base <BASE>` — Git revision to compare with

  Default value: `HEAD`
* `--staged` — Compare the index (staged changes) instead of the working tree
* `--strict` — Report title and body edits to accepted and rejected records as errors
* `--json` — Print the JSON envelope instead of text



## `dadrock tags`

Inspect DADR tag usage.

**Usage:** `dadrock tags <COMMAND>`

###### **Subcommands:**

* `list` — List normalized YAML and inline tags.



## `dadrock tags list`

List normalized YAML and inline tags.

**Usage:** `dadrock tags list [OPTIONS]`

###### **Options:**

* `--json` — Print the JSON envelope instead of text



## `dadrock mcp`

Serve Dadrock MCP tools over stdio.

**Usage:** `dadrock mcp <COMMAND>`

###### **Subcommands:**

* `serve` — Serve Dadrock MCP tools over stdio.



## `dadrock mcp serve`

Serve Dadrock MCP tools over stdio.

**Usage:** `dadrock mcp serve [OPTIONS] [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>` — The directory to find the project from (defaults to the current directory)

###### **Options:**

* `--read-only` — Serve only the read tools, so no tool can create or change a record



## `dadrock agent`

Call a configured local ACP agent for candidate drafting.

**Usage:** `dadrock agent <COMMAND>`

###### **Subcommands:**

* `check` — Check that a local ACP agent can start a Dadrock session.
* `draft` — Ask a local ACP agent to draft a candidate decision.
* `skills` — List available Dadrock agent skills and prompt packs.



## `dadrock agent check`

Check that a local ACP agent can start a Dadrock session.

**Usage:** `dadrock agent check [OPTIONS]`

###### **Options:**

* `--agent <AGENT>` — The ACP agent program to start (defaults to `DADROCK_ACP_AGENT`, then an installed provider)
* `--agent-arg <AGENT_ARGS>` — An extra argument for the agent program; repeat for more
* `--no-mcp` — Start the agent without Dadrock's MCP tools



## `dadrock agent draft`

Ask a local ACP agent to draft a candidate decision.

**Usage:** `dadrock agent draft [OPTIONS] <PROMPT>...`

###### **Arguments:**

* `<PROMPT>` — The decision to draft; the words join into one prompt

###### **Options:**

* `--write` — Save the draft as a new candidate record; without it, the draft is only printed
* `--json` — Print the JSON envelope instead of text
* `--permission <PERMISSION>` — How to answer the agent's tool permission requests

  Default value: `deny`

  Possible values:
  - `deny`:
    Deny every tool permission request
  - `allow-reads`:
    Approve read, search, and think tool requests once; deny the rest
  - `allow-once`:
    Approve every tool permission request once
  - `allow-always`:
    Approve every tool permission request for the session

* `--agent <AGENT>` — The ACP agent program to start (defaults to `DADROCK_ACP_AGENT`, then an installed provider)
* `--agent-arg <AGENT_ARGS>` — An extra argument for the agent program; repeat for more
* `--skill <SKILLS>` \[alias: `prompt-pack`\] — An agent skill or prompt pack to include, by name or path; repeat for more
* `--no-mcp` — Start the agent without Dadrock's MCP tools



## `dadrock agent skills`

List available Dadrock agent skills and prompt packs.

**Usage:** `dadrock agent skills [OPTIONS]`

###### **Options:**

* `--json` — Print the JSON envelope instead of text



## `dadrock task`

Discover, inspect, and run Dadrock task packages.

**Usage:** `dadrock task <COMMAND>`

###### **Subcommands:**

* `list` — List built-in, user, and project task packages.
* `show` — Show one resolved task package.
* `run` — Run one resolved task package with a local ACP agent.
* `runs` — List persisted local task runs.
* `result` — Read a persisted task run result.



## `dadrock task list`

List built-in, user, and project task packages.

**Usage:** `dadrock task list [OPTIONS]`

###### **Options:**

* `--json` — Print the JSON envelope instead of text



## `dadrock task show`

Show one resolved task package.

**Usage:** `dadrock task show [OPTIONS] <TASK_ID>`

###### **Arguments:**

* `<TASK_ID>` — The task id, as `dadrock task list` prints it

###### **Options:**

* `--json` — Print the JSON envelope instead of text
* `--raw` — Print the task file exactly as written, front matter included



## `dadrock task run`

Run one resolved task package with a local ACP agent.

**Usage:** `dadrock task run [OPTIONS] <TASK_ID>`

###### **Arguments:**

* `<TASK_ID>` — Task to run, by id (see `dadrock task list`)

###### **Options:**

* `--input-json <JSON>` — Task inputs as a JSON object, for tasks without friendly input flags
* `--path <PATH>` — File or directory to inspect (code-to-record)
* `--context <CONTEXT>` — Describe the decision to capture (context-to-record)
* `--focus <FOCUS>` — Topic to concentrate on (code-to-record, decisions-to-spec)
* `--limit <LIMIT>` — Maximum initial candidates (code-to-record defaults to 5)
* `--instructions <INSTRUCTIONS>` — What the agent should do, as task context (required by custom)
* `--record <QUERY>` — Record for the agent to work on, by id, short id, or title; repeatable
* `--attach <PATH>` — File or folder for the agent to read, as task context; repeatable
* `--json` — Print the run as a JSON command envelope
* `--permission <PERMISSION>` — How to answer the agent's tool permission requests

  Default value: `deny`

  Possible values:
  - `deny`:
    Deny every tool permission request
  - `allow-reads`:
    Approve read, search, and think tool requests once; deny the rest
  - `allow-once`:
    Approve every tool permission request once
  - `allow-always`:
    Approve every tool permission request for the session

* `--agent <PROGRAM>` — ACP agent executable to run instead of the configured or detected one
* `--agent-arg <ARG>` — Extra argument for the ACP agent; repeatable
* `--no-mcp` — Do not attach Dadrock's MCP server to the agent session



## `dadrock task runs`

List persisted local task runs.

**Usage:** `dadrock task runs [OPTIONS]`

###### **Options:**

* `--json` — Print the JSON envelope instead of text



## `dadrock task result`

Read a persisted task run result.

**Usage:** `dadrock task result [OPTIONS] <RUN_ID>`

###### **Arguments:**

* `<RUN_ID>` — The run id, as `dadrock task runs` prints it

###### **Options:**

* `--json` — Print the whole JSON envelope instead of the result alone



## `dadrock preview`

Serve the local web review UI.

**Usage:** `dadrock preview [OPTIONS] [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>` — The directory to find the project from (defaults to the current directory)

###### **Options:**

* `--port <PORT>` — The port to listen on; 0 picks a free one

  Default value: `4004`
* `--host <HOST>` — The address to listen on: only `localhost`, `127.0.0.1`, and `::1` are accepted

  Default value: `127.0.0.1`



<hr/>

<small><i>
    This document was generated automatically by
    <a href="https://crates.io/crates/clap-markdown"><code>clap-markdown</code></a>.
</i></small>
