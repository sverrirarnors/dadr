# LLM agents

Agent assistance is optional: you can record decisions with DADRock alone. Agents can work with DADRock in three ways:

- **[MCP](#mcp):** an agent client calls DADRock's tools through its Model Context Protocol server.
- **[ACP](#acp):** DADRock directs a local agent through the Agent Client Protocol to draft and revise candidates.
- **[CLI](#cli):** an agent that runs shell commands uses `dadrock` directly.

Whichever route you use, agent outputs are proposals for review. Code can show what happened without establishing why; correct unsupported claims and supply missing rationale before accepting. Acceptance and rejection remain human decisions.

## MCP

DADRock exposes a [Model Context Protocol](https://modelcontextprotocol.io) server over stdio:

```sh
dadrock mcp serve
dadrock mcp serve --read-only
dadrock mcp serve path/to/project
```

The normal server exposes inspection and mutation tools through the same command layer as the CLI: listing, searching, and reading records, lifecycle status, decision history and diffs, tags, tasks and task runs, validation, and a project snapshot; plus creating candidates, accepting, rejecting, superseding, and submitting task results. Use read-only mode for clients that should only inspect records. Human approval is still required for acceptance or rejection.

Register the server with your client as a stdio command. The configuration file and its location depend on the client; many accept an entry like this:

```json
{
  "mcpServers": {
    "dadrock": {
      "command": "dadrock",
      "args": ["mcp", "serve", "--read-only", "/path/to/project"]
    }
  }
}
```

## ACP

DADRock connects to a locally installed agent through the [Agent Client Protocol](https://agentclientprotocol.com) (ACP). The agent may use a remote model. You need an ACP-compatible provider or bridge and its authentication. An ordinary `claude` or `codex` executable alone is not necessarily an ACP provider.

Use your provider's own installation and authentication instructions. Choose a bridge explicitly supported by your agent provider. DADRock does not install providers or manage their credentials.

### Configure one local agent

DADRock looks for known provider commands on `PATH`, such as `claude-agent-acp`, `codex-acp`, or `gemini --acp`. To choose one, set `DADROCK_ACP_AGENT` to the executable and put any arguments in `DADROCK_ACP_AGENT_ARGS`, or pass `--agent` and `--agent-arg` to a single command. Check the connection:

```sh
dadrock agent check
dadrock agent skills
```

Every task carries its own instructions; no separately installed skill is required. `agent skills` lists optional local instructions.

DADRock attaches its own [MCP server](#mcp) to its agent sessions by default. `--no-mcp` disables that attachment for providers where it is unsuitable.

### Draft for review

```sh
dadrock task run code-to-record --path analysis/ --limit 5
dadrock task run context-to-record --context "Explain our choice of sample restrictions"
```

The limit asks the agent to prioritize a small batch, rather than treating every operation as a decision. It is a drafting request, not a hard guarantee on model output. The [web interface](dadrock/web.md) offers the same drafting actions, and results land in its Inbox.

Tool permission requests default to `deny`. When your selected workflow needs tools, explicitly choose the appropriate permission mode with `--permission allow-once` or `--permission allow-always`. This does not authorize an agent to accept decisions on your behalf.

`dadrock agent draft` remains available for compatibility. New workflows should use `task run context-to-record`.

### Custom workflows and runs

```sh
dadrock task list
dadrock task show code-to-record
dadrock task runs
dadrock task result RUN_ID
```

Built-in workflows include drafting, editing candidates, [drafting successors](changing.md#drafting-a-successor) for accepted decisions, merging drafts, finding conflicts, and synthesizing accepted decisions into a spec. The built-in tasks are the task guides of the `dadr` skill, the files in `skills/dadr/tasks/` of this repository. Each is one Markdown file: its YAML front matter is the task manifest (id, inputs, requirements, output kind, and `mcp_access`, which is `full` or `read_only`) and its body is the prompt. `dadrock task show <task-id>` prints a task's prompt, and `--json` adds its manifest.

To customize a task, copy its guide to `dadr/tasks/<task-id>.md` and adapt it. A project task overrides a user task in `~/.dadrock/tasks/<task-id>.md` (or `$DADROCK_TASKS_DIR`), which overrides the built-in task with the same id. The file name must match the `id` in the front matter; `dadrock task list` reports files that do not load, including task directories in the old `task.toml` layout. Advanced and third-party inputs can use `--input-json`; do not combine it with friendly input flags such as `--path` or `--context`.

Runs are persisted under `dadr/runs/`, which is ignored by default and may contain source excerpts or model transcripts. Output kinds determine how results are rendered. Protocol and run metadata stay outside record front matter.

## CLI

Coding agents that run shell commands can drive `dadrock` like any other tool. Its machine-readable output is a JSON command envelope with `command`, `ok`, and `data`, plus `diagnostics` and `changed_paths` when there are any. Commands such as `list` and `new` always print it; commands with a human-readable summary, such as `validate`, print it with `--json`:

```sh
dadrock commands --json
dadrock list --status candidate
dadrock new "Decision title"
dadrock validate --json
```

`dadrock commands --json` describes every command: its usage, whether it changes the project, whether it supports JSON, and when it exits non-zero. `dadrock help COMMAND` prints the same detail for one command. The full list is in the [command-line reference](dadrock/cli.md).

For guidance beyond the built-in tasks, copy `skills/dadr/` from this checkout to your agent's supported skill directory, such as `.agents/skills/dadr` or `~/.codex/skills/dadr`. The agent's own discovery rules determine which location works. The skill routes requests such as bootstrapping candidates from existing code, finding conflicting decisions, or synthesizing a spec to the matching `dadrock` commands.

Future automated faithfulness and coverage checks are described on the [roadmap](roadmap.md).
