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
* [`dadrock status`↴](#dadrock-status)
* [`dadrock status accept`↴](#dadrock-status-accept)
* [`dadrock status reject`↴](#dadrock-status-reject)
* [`dadrock status supersede`↴](#dadrock-status-supersede)
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
* [`dadrock preview`↴](#dadrock-preview)

## `dadrock`

DADR decision records

**Usage:** `dadrock [COMMAND]`

###### **Subcommands:**

* `help` — Show top-level help or detailed metadata for one command.
* `commands` — List command metadata for automation and command discovery.
* `completions` — Generate a shell completion script.
* `dir` — Print the discovered decisions directory.
* `config` — Print the resolved project configuration.
* `init` — Create a DADR decisions directory, marker, and default config.
* `demo` — Seed a new project with DADR records covering the core specification.
* `new` — Create a candidate decision record from the active candidate template.
* `list` — List records, optionally filtered by lifecycle status, tag, or query.
* `status` — Inspect one record's lifecycle status or run lifecycle transitions.
* `validate` — Run project validation, or target records, links, or tags.
* `tags` — Inspect DADR tag usage.
* `mcp` — Serve Dadrock MCP tools over stdio.
* `agent` — Call a configured local ACP agent for candidate drafting.
* `preview` — Serve the local web review UI.



## `dadrock help`

Show top-level help or detailed metadata for one command.

**Usage:** `dadrock help [TOPIC]...`

###### **Arguments:**

* `<TOPIC>`



## `dadrock commands`

List command metadata for automation and command discovery.

**Usage:** `dadrock commands [OPTIONS]`

###### **Options:**

* `--json`



## `dadrock completions`

Generate a shell completion script.

**Usage:** `dadrock completions <SHELL>`

###### **Arguments:**

* `<SHELL>`

  Possible values: `bash`, `zsh`, `fish`




## `dadrock dir`

Print the discovered decisions directory.

**Usage:** `dadrock dir`



## `dadrock config`

Print the resolved project configuration.

**Usage:** `dadrock config [OPTIONS]`

###### **Options:**

* `--json`



## `dadrock init`

Create a DADR decisions directory, marker, and default config.

**Usage:** `dadrock init [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>`



## `dadrock demo`

Seed a new project with DADR records covering the core specification.

**Usage:** `dadrock demo [OPTIONS] <DIRECTORY>`

###### **Arguments:**

* `<DIRECTORY>`

###### **Options:**

* `--json`



## `dadrock new`

Create a candidate decision record from the active candidate template.

**Usage:** `dadrock new [OPTIONS] <TITLE>...`

###### **Arguments:**

* `<TITLE>`

###### **Options:**

* `--tag <TAGS>`
* `--quiet`



## `dadrock list`

List records, optionally filtered by lifecycle status, tag, or query.

**Usage:** `dadrock list [OPTIONS]`

###### **Options:**

* `--status <STATUS>`

  Possible values: `candidate`, `accepted`, `rejected`, `superseded`

* `--tag <TAG>`
* `--query <QUERY>`
* `--unique`



## `dadrock status`

Inspect one record's lifecycle status or run lifecycle transitions.

**Usage:** `dadrock status [QUERY]... [COMMAND]`

###### **Subcommands:**

* `accept` — Accept a candidate by converting it into an accepted decision.
* `reject` — Reject a candidate and move it into the candidate archive.
* `supersede` — Link accepted successors to the records they supersede.

###### **Arguments:**

* `<QUERY>`



## `dadrock status accept`

Accept a candidate by converting it into an accepted decision.

**Usage:** `dadrock status accept <QUERY>...`

###### **Arguments:**

* `<QUERY>`



## `dadrock status reject`

Reject a candidate and move it into the candidate archive.

**Usage:** `dadrock status reject <QUERY>...`

###### **Arguments:**

* `<QUERY>`



## `dadrock status supersede`

Link accepted successors to the records they supersede.

**Usage:** `dadrock status supersede [OPTIONS]`

###### **Options:**

* `--from <FROM>`
* `--to <TO>`



## `dadrock validate`

Run project validation, or target records, links, or tags.

**Usage:** `dadrock validate [OPTIONS] [COMMAND]`

###### **Subcommands:**

* `records` — Validate record front matter and structure.
* `links` — Report broken or ambiguous wikilinks and local Markdown links.
* `tags` — Report malformed YAML and inline tags.

###### **Options:**

* `--json`



## `dadrock validate records`

Validate record front matter and structure.

**Usage:** `dadrock validate records [OPTIONS] [PATHS]...`

###### **Arguments:**

* `<PATHS>`

###### **Options:**

* `--json`



## `dadrock validate links`

Report broken or ambiguous wikilinks and local Markdown links.

**Usage:** `dadrock validate links [OPTIONS]`

###### **Options:**

* `--json`



## `dadrock validate tags`

Report malformed YAML and inline tags.

**Usage:** `dadrock validate tags [OPTIONS]`

###### **Options:**

* `--json`



## `dadrock tags`

Inspect DADR tag usage.

**Usage:** `dadrock tags <COMMAND>`

###### **Subcommands:**

* `list` — List normalized YAML and inline tags.



## `dadrock tags list`

List normalized YAML and inline tags.

**Usage:** `dadrock tags list [OPTIONS]`

###### **Options:**

* `--json`



## `dadrock mcp`

Serve Dadrock MCP tools over stdio.

**Usage:** `dadrock mcp <COMMAND>`

###### **Subcommands:**

* `serve` — Serve read-only Dadrock MCP tools over stdio.



## `dadrock mcp serve`

Serve read-only Dadrock MCP tools over stdio.

**Usage:** `dadrock mcp serve [DIRECTORY]`

###### **Arguments:**

* `<DIRECTORY>`



## `dadrock agent`

Call a configured local ACP agent for candidate drafting.

**Usage:** `dadrock agent <COMMAND>`

###### **Subcommands:**

* `check` — Check that a local ACP agent can start a Dadrock session.
* `draft` — Ask a local ACP agent to draft a candidate decision.
* `skills` — List local Dadrock agent skills and prompt packs.



## `dadrock agent check`

Check that a local ACP agent can start a Dadrock session.

**Usage:** `dadrock agent check [OPTIONS]`

###### **Options:**

* `--agent <AGENT>`
* `--agent-arg <AGENT_ARGS>`
* `--no-mcp`



## `dadrock agent draft`

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
* `--skill <SKILLS>` [alias: `prompt-pack`]
* `--no-mcp`



## `dadrock agent skills`

List local Dadrock agent skills and prompt packs.

**Usage:** `dadrock agent skills [OPTIONS]`

###### **Options:**

* `--json`



## `dadrock preview`

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
