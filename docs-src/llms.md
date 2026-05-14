# LLMs and agents

Using DADR should be [frictionless.](/dadr/#frictionless) The **dadrock** software can help with tedious file management and validation. But the key to making this system work is to use LLM agents to draft and review decision candidates.

LLMs work especially well with DADR, because they are plain text (markdown) and atomic (one decision per file). This makes them easy for agents to process.

As noted in the [workflow diagram,](/dadr/#workflow) all acceptance or rejection decisions are *human-gated*. LLM agents

* MAY draft or edit candidate decisions.
* MUST NOT accept or reject decisions.

## Implemented

These features are already implemented in **dadrock**:

* Draft a decision candidate based on user input. 
* Chat interface for iterative improvement of decision candidates.
* MCP tools for structured project inspection and DADR lifecycle mutations during agent sessions.
* Optional local skill/prompt-pack instructions for agent drafts.

## Dadrock Tasks

Dadrock tasks are editable prompt workflows that use the same Dadrock MCP and command layers as the CLI and web UI. Built-in tasks are examples, not special cases: project tasks live under `dadr/tasks/`, user-installed tasks can also be discovered, and task runs are persisted under `dadr/runs/`.

Agents running tasks must return the declared JSON output kind exactly. They must not accept, reject, supersede, or rewrite records except through explicit user-requested Dadrock lifecycle actions.

## Roadmap

These features will be implemented in **dadrock** in the short term:

* Draft decision candidates automatically based on 
    - Git commits 
    - Working tree changes to the code base.
    - Manuscript 
    - Pre-analysis plan.
* Faithfulness analysis to compare
    - Manuscript vs. Decisions
    - Pre-analysis plan vs. Decisions
    - Code vs. Decisions
* Consistency and completeness
    - Do some decisions contradict each other?
    - Are the unresolved ambiguities?
    - Are all the decisions made in the code recorded in DADR?
* Decision review
    - Comment on the decisions and their unforseen consequences.
* Draft reports
    - Write a draft methods section based on the narrative DADR.


## LLM agent setup

**dadrock** uses local agents through the [**Agent Client Protocol (ACP)**](https://agentclientprotocol.com/).

### Requirements

To use agent drafting or web candidate revision, the local machine needs:

- the `dadrock` binary installed;
- an installed LLM agent like Claude Code, Codex, or Open Code;
- an ACP-compatible bridge;
- an authenticated account with a LLM provider.

### Install a provider CLI

**dadrock** does not bundle LLM agents or ACP providers. Install one supported agent and ACP locally, and configure authentication in that provider's own tool.[^acp-install-security]^,^[^acp-install-local]

Install the CLI for any ACP-compatible provider of your choice, using your preferred method. For examples, see the table below.[^agentic-nvim]


[^acp-install-security]:
    For security reasons, **dadrock** does not install or manage provider binaries for you. You must install them manually.

[^acp-install-local]:
    Note: The provider may still call a remote model; "local" here means **dadrock** talks to a local ACP process.

[^agentic-nvim]:
    This install table is adapted from the ACP provider installation guidance in the [`agentic.nvim` README](https://github.com/agentic-labs/agentic.nvim).

!!! warning
    These install commands are included for convenience. Always refer to the official installation instructions from the respective ACP provider.

| Provider | Install |
|---|---|
| [Claude Agent ACP](https://github.com/agentclientprotocol/claude-agent-acp) | `pnpm add -g @agentclientprotocol/claude-agent-acp`<br/>**OR** `npm i -g @agentclientprotocol/claude-agent-acp`<br/>**OR** [Download binary](https://github.com/agentclientprotocol/claude-agent-acp/releases) |
| [Gemini CLI](https://github.com/google-gemini/gemini-cli) | `pnpm add -g @google/gemini-cli`<br/>**OR** `npm i -g @google/gemini-cli`<br/>**OR** `brew install --cask gemini` |
| [Codex ACP](https://github.com/zed-industries/codex) | `pnpm add -g @zed-industries/codex-acp`<br/>**OR** `npm i -g @zed-industries/codex-acp`<br/>**OR** [Download binary](https://github.com/zed-industries/codex/releases) |
| [OpenCode](https://opencode.ai/) | `pnpm add -g opencode-ai`<br/>**OR** `npm i -g opencode-ai`<br/>**OR** `brew install opencode`<br/>**OR** `curl -fsSL https://opencode.ai/install \| bash` |
| [Cursor Agent](https://cursor.com/docs/cli/agent) | `curl https://cursor.com/install -fsS \| bash`<br/>**OR** Windows: `irm 'https://cursor.com/install?win32=true' \| iex`<br/>**OR** See [Cursor docs](https://cursor.com/docs/cli/agent) |
| [Copilot CLI](https://github.com/github/copilot-cli) | `pnpm add -g @github/copilot`<br/>**OR** `npm i -g @github/copilot`<br/>**OR** `brew install copilot-cli`<br/>**OR** `curl -fsSL https://gh.io/copilot-install \| bash` |
| [Auggie](https://www.augmentcode.com/) | `pnpm add -g @augmentcode/auggie`<br/>**OR** `npm i -g @augmentcode/auggie`<br/>**OR** See [Auggie docs](https://docs.augmentcode.com/) |
| [Mistral Vibe](https://mistral.ai/news/mistral-code) | `curl -LsSf https://mistral.ai/vibe/install.sh \| bash`<br/>**OR** `uv tool install mistral-vibe`<br/>**OR** `pip install mistral-vibe`<br/>**OR** [Download binary](https://github.com/mistralai/vibe/releases) |
| [Cline](https://cline.bot/) | `pnpm add -g cline`<br/>**OR** `npm i -g cline`<br/>**OR** See [Cline docs](https://docs.cline.bot/) |
| [Goose](https://block.github.io/goose/) | `brew install block-goose-cli`<br/>**OR** See [Goose docs](https://block.github.io/goose/docs/getting-started/installation) |



### Use a LLM agent to draft decision candidates

If exactly one supported provider is installed, automatic detection is enough:

```bash
dadrock agent draft "Document the missing sample restriction"
```

To choose a provider explicitly:

```bash
dadrock agent draft --agent gemini --agent-arg --acp "Draft a DADR for the new model specification"
```

ACP tool permission requests are denied by default. To allow them for a session:

```bash
dadrock agent draft --permission allow-once "Draft a DADR for the sample restriction"
```

By default, **dadrock** attaches an MCP server to ACP sessions so the agent can inspect records, tags, validation reports, and the bounded project snapshot with structured tools instead of guessing. The same MCP surface also exposes DADR lifecycle mutations through the `dadrock::commands` layer used by the CLI, so writes return command envelopes with changed paths and diagnostics. To run the MCP server directly for another client:

```bash
dadrock mcp serve
```

To check that a local ACP provider accepts a Dadrock session with MCP attached:

```bash
dadrock agent check
```

Disable automatic MCP attachment for one draft with `--no-mcp`:

```bash
dadrock agent draft --no-mcp "Draft a DADR without project tools"
```

Dadrock ships with a built-in `dadr` skill for DADR workflow guidance:

```bash
dadrock agent draft --skill dadr "Draft candidate DADRs from this project"
```

To add local guidance, put instructions in a `SKILL.md` file under `.agents/skills/<name>/`, `.github/skills/<name>/`, `.claude/skills/<name>/`, `~/.agents/skills/<name>/`, `~/.codex/skills/<name>/`, `~/.codex/skills/.system/<name>/`, or `~/.claude/skills/<name>/`, then pass it by name:

```bash
dadrock agent draft --skill dadr-style "Draft a DADR for the new API boundary"
```

You can also pass a direct file or directory path with `--skill`; `--prompt-pack` is an alias. **dadrock** includes the selected instructions in the prompt and records a combined SHA-256 hash in the agent response provenance.

To inspect built-in, installed, and project-local skills and prompt packs:

```bash
dadrock agent skills
```

Prompt-pack provenance stays in the agent response envelope rather than the DADR record front matter. This keeps record files focused on the decision itself and avoids expanding the DADR schema with tool-run metadata.

Even when permissions are allowed, the agent still produces candidates or revisions for review. It does not accept decisions.
