# LLM agents

Agent assistance is optional. The [first decision](dadrock.md#first-decision) needs only Dadrock. This page is for configuring an agent or integrating Dadrock into other tools.

## Configure one local agent

Dadrock connects to a locally installed agent through the Agent Client Protocol (ACP). The agent may use a remote model. You need an ACP-compatible provider or bridge and its authentication. Built-in tasks include their own instructions; no separately installed skill is required. An ordinary `claude` or `codex` executable alone is not necessarily an ACP provider.

Use your provider's own installation and authentication instructions. Choose a bridge explicitly supported by your agent provider. Dadrock does not install providers or manage their credentials.

For optional guidance outside built-in tasks, copy `skills/dadr/` from this checkout to your agent's supported skill directory, such as `.agents/skills/dadr` or `~/.codex/skills/dadr`. The agent's own discovery rules determine which location works.

Check the connection:

```sh
dadrock agent check
dadrock agent skills
```

When more than one provider is available, choose one explicitly with `--agent`. Provider-specific arguments can be passed with `--agent-arg`. `agent skills` lists optional local instructions. Built-in task runs do not require a skill-loading preflight.

## Draft for review

```sh
dadrock task run code-to-record --path analysis/ --limit 5
dadrock task run context-to-record --context "Explain our choice of sample restrictions"
```

The limit asks the agent to prioritize a small batch, rather than treating every operation as a decision. It is a drafting request, not a hard guarantee on model output. Outputs remain proposals. Do not infer motivation from implementation alone; supply missing rationale before accepting.

Tool permission requests default to `deny`. When your selected workflow needs tools, explicitly choose the appropriate permission mode with `--permission allow-once` or `--permission allow-always`. This does not authorize an agent to accept decisions on your behalf.

`dadrock agent draft` remains available for compatibility. New workflows should use `task run context-to-record`.

## Custom workflows and runs

```sh
dadrock task list
dadrock task show code-to-record
dadrock task runs
dadrock task result RUN_ID
```

Built-in workflows include drafting, editing candidates, drafting successors for accepted decisions, merging drafts, finding conflicts, and synthesizing accepted decisions. Project overrides live in `dadr/tasks/<task-id>/`, with a `task.toml` manifest and a prompt file. Inspect an existing task before customizing it. Advanced and third-party inputs can use `--input-json`; do not combine it with friendly input flags such as `--path` or `--context`.

Runs are persisted under `dadr/runs/`, which is ignored by default and may contain source excerpts or model transcripts. Output kinds determine how results are rendered. Protocol and run metadata stay outside record front matter.

## Integrate another agent client

Dadrock exposes a Model Context Protocol (MCP) server:

```sh
dadrock mcp serve
dadrock mcp serve --read-only
```

The normal server exposes inspection and mutation tools through the same command layer as the CLI. Use read-only mode for clients that should only inspect records. Human approval is still required for acceptance or rejection.

Dadrock attaches MCP to its agent sessions by default. `--no-mcp` disables that attachment for clients where it is unsuitable. These integration settings are not required to write or review records manually.

Future automated faithfulness and coverage checks are described on the [roadmap](roadmap.md).
