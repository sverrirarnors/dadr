# Dadrock

Dadrock is the CLI and local web application for DADR files. Start with [your first decision](../getting-started.md), or consult the [command reference](cli.md).

```sh
dadrock init
dadrock new "Decision title"
dadrock preview
dadrock accept ID
dadrock validate
```

Running `dadrock` without a subcommand prints help. `preview` starts the local web server, normally at port 4004.

## Where files live

| Directory | Contents |
|---|---|
| `dadr/candidates/` | Proposals awaiting review |
| `dadr/accepted/` | Accepted decisions, including those later replaced |
| `dadr/rejected/` | Rejected proposals |
| `dadr/templates/` | Optional record templates |
| `dadr/tasks/` | Optional project-specific agent workflows |
| `dadr/runs/` | Ignored local agent-run inputs, results and logs |

Use `dadrock config` to inspect the resolved layout. Keep the record directories in version control. Run state may include pasted context and source excerpts and is ignored by default.

## Changing a decision

Use **Change this decision** in the web interface to create a replacement candidate. Its optional `## Replaces` section contains a labeled link to the original full UUID. Keep that section to preserve replacement intent. Acceptance validates the predecessor and links the accepted replacement while preserving the original body, including when accepting through the CLI. See [the web workflow](web.md) for an example.

For two records that are already accepted, `dadrock supersede --from OLD --to NEW` links the replacement explicitly.

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

Validation does not compare accepted records with Git history, resolve bibliography citations, detect `TBD` prose, enforce plan-deviation sections, or determine whether code follows a decision. Those checks are [roadmap work](../roadmap.md). Human review remains necessary.

## Continuous integration

Once Dadrock is installed in your CI environment, run `dadrock validate` from the analysis project root. A nonzero exit status signals an error; `--json` provides structured diagnostics. Install from a pinned source revision for reproducibility, using the [source installation instructions](../getting-started.md#1-install-from-source). A CLI-only CI build can use `cargo install --path crates/dadrock --bin dadrock --features no-js-build` from the checked-out Dadrock source.

## Agents and integrations

Agents are optional. See [advanced agent setup](../llms.md) for provider configuration, custom tasks, and MCP integration. The [editor integration page](ide.md) describes the extension source available in this repository.
