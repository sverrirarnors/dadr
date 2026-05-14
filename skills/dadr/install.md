# Install dadrock and an ACP provider

Set up the local machine to use `dadrock` and (optionally) one local LLM agent for candidate drafting via the Agent Client Protocol (ACP).

Trigger phrases:

- "install dadrock", "set up dadrock", "set up dadr"
- "install an ACP provider", "configure dadrock agent"
- "what do I need to use dadrock?"

## What you are installing

| Component | Required? | Purpose |
|---|---|---|
| `dadrock` Rust binary | Required for any DADR workflow. | Project init, validation, lifecycle commands, MCP server. |
| One ACP provider CLI (Claude Agent ACP, Codex, Gemini CLI, Cursor Agent, Copilot CLI, Auggie, Mistral Vibe, Cline, Goose, OpenCode) | Optional — required for `dadrock agent draft`, TUI candidate revision, and Task view agent runs. | Local LLM agent that drafts/revises candidates. dadrock talks to it via ACP. |
| `dadr` skill installed for the local ACP agent | Required for Task view agent runs. | The agent-side guidance for Dadrock tasks. Dadrock preflights this by asking the ACP agent to load the installed skill. |
| Provider authentication | Required iff a provider is installed. | Configured inside the provider's own CLI. dadrock never handles credentials. |

dadrock does **not** bundle LLM agents or ACP providers — they have their own auth and update lifecycles. If a provider's CLI is already on `PATH`, you can skip its install step.

## Preconditions

This task changes global state. Before running install commands, confirm with the user that they want a global install. Pick the install method that matches a toolchain they already have rather than asking them to install Rust, Node, or Homebrew first.

## Workflow

### Step 1 — Inventory the local machine

Run a single sweep to determine platform, current dadrock state, available toolchains, and already-installed providers. The unknown commands print "not found" and continue:

```bash
uname -s 2>/dev/null || ver  # Darwin | Linux | Windows version
command -v dadrock
command -v cargo
command -v brew
command -v pnpm
command -v npm
command -v uv
command -v pipx
command -v claude
command -v codex
command -v gemini
command -v cursor-agent
command -v copilot
command -v auggie
command -v opencode
command -v goose
command -v mistral-vibe
command -v cline
```

The output of this sweep determines:

- **Whether dadrock needs installing.** If `dadrock --version` succeeds, skip Step 2.
- **Which install path to recommend for dadrock.** Prefer `cargo install` if Rust is on `PATH`; otherwise the prebuilt binary or one-line installer.
- **Which ACP providers are already usable.** If exactly one is on `PATH`, dadrock auto-detects it — skip Step 3 unless the user wants a different one.

On Windows (PowerShell), substitute `Get-Command <name>` for `command -v <name>`.

### Step 2 — Install dadrock

Skip if `dadrock --version` already works. Otherwise pick by what is available:

| Available toolchain | Recommended command |
|---|---|
| Rust / `cargo` on `PATH` | `cargo install dadrock` |
| Nothing (any platform) | Download the prebuilt binary from <https://github.com/vincentarelbundock/dadr/releases> or use the platform-specific installer linked from that page. |

Confirm afterwards:

```bash
dadrock --version
```

Surface the chosen command to the user and **wait for confirmation** before running. Global installs change `PATH` and may collide with the user's other tooling.

### Step 3 — Pick an ACP provider

Optional but required for `dadrock agent draft`. Skip entirely if exactly one provider CLI is already on `PATH` — `dadrock` will auto-detect.

If the user has no preference, recommend by what is already in play:

| Local context | Suggested provider |
|---|---|
| Already using Claude Code on macOS / Linux | Claude Agent ACP (same Anthropic auth carries over). |
| Already using Codex | Codex ACP. |
| Already using Cursor | Cursor Agent. |
| Already authenticated with Gemini | Gemini CLI. |
| macOS, no prior preference | Claude Agent ACP via `pnpm`/`npm`, OR OpenCode via `brew install opencode`. |
| Linux, no prior preference | Claude Agent ACP via `pnpm`/`npm`, OR the OpenCode installer script. |
| Windows, no prior preference | Cursor Agent (`irm 'https://cursor.com/install?win32=true' \| iex`) OR Claude Agent ACP via `npm`. |

Provider install table (mirror of dadrock's upstream — pick whichever matches the user's auth and toolchain):

| Provider | Install (pick one) |
|---|---|
| Claude Agent ACP | `pnpm add -g @agentclientprotocol/claude-agent-acp` · `npm i -g @agentclientprotocol/claude-agent-acp` · prebuilt binary from the releases page |
| Gemini CLI | `pnpm add -g @google/gemini-cli` · `npm i -g @google/gemini-cli` · `brew install --cask gemini` |
| Codex ACP | `pnpm add -g @zed-industries/codex-acp` · `npm i -g @zed-industries/codex-acp` · prebuilt binary |
| OpenCode | `pnpm add -g opencode-ai` · `npm i -g opencode-ai` · `brew install opencode` · `curl -fsSL https://opencode.ai/install \| bash` |
| Cursor Agent | macOS/Linux: `curl https://cursor.com/install -fsS \| bash` · Windows: `irm 'https://cursor.com/install?win32=true' \| iex` |
| Copilot CLI | `pnpm add -g @github/copilot` · `npm i -g @github/copilot` · `brew install copilot-cli` · `curl -fsSL https://gh.io/copilot-install \| bash` |
| Auggie | `pnpm add -g @augmentcode/auggie` · `npm i -g @augmentcode/auggie` |
| Mistral Vibe | `curl -LsSf https://mistral.ai/vibe/install.sh \| bash` · `uv tool install mistral-vibe` · `pip install mistral-vibe` |
| Cline | `pnpm add -g cline` · `npm i -g cline` |
| Goose | `brew install block-goose-cli` |

These are convenience shortcuts — refer to each provider's own installation guide for the canonical instructions.

Surface the chosen command and **wait for confirmation** before running.

### Step 4 — Authenticate the provider

Authentication is done inside the provider's own CLI; dadrock and this task do not handle credentials. Common patterns:

- `claude login`
- `gemini auth login`
- `codex login`
- `cursor-agent login`
- `gh copilot auth`
- `goose configure`

If the provider was previously authenticated for non-ACP use, the same auth carries over — skip this step.

### Step 5 — Install the `dadr` skill for the agent

Task view runs assume the local ACP agent can load an installed skill named `dadr`. Install this repository's `skills/dadr/` directory into the skill location used by that agent. Common locations Dadrock can list are:

```bash
.agents/skills/dadr
~/.agents/skills/dadr
~/.codex/skills/dadr
~/.claude/skills/dadr
$CODEX_HOME/skills/dadr
```

The ACP agent's own skill discovery is authoritative for Task view runs. Dadrock does not inline the skill; it asks the agent to prove access before starting the task.

### Step 6 — Verify

```bash
dadrock --version
dadrock agent check
dadrock agent skills
```

`dadrock agent check` starts a Dadrock ACP session against the configured (or auto-detected) provider and tears it down. A clean exit means the chain is wired correctly: provider on `PATH`, authenticated, MCP attaches, prompt envelope round-trips.

`dadrock agent skills` lists skills and prompt packs visible to Dadrock's local filesystem search. The definitive Task view check happens at run time: Dadrock asks the ACP agent to load `dadr` and reports a clear failure if the agent cannot access it.

If `dadrock agent check` fails, in order:

1. Confirm the provider's own CLI runs: `claude --version`, `gemini --version`, etc.
2. Confirm authentication: most providers print auth status with `--version` or `whoami`.
3. Force a specific provider to bypass auto-detection: `dadrock agent check --agent <name>`.
4. Isolate MCP from ACP: `dadrock agent draft --no-mcp "Test"`. If that works but `agent check` doesn't, the MCP attach is the problem; investigate `dadrock mcp serve` separately.

## Constraints

- **Never auto-install global packages.** Always surface the proposed command and wait for user confirmation. Global installs change `PATH` and may conflict with the user's tooling.
- **Never modify auth state.** Authentication runs in the provider's own CLI. This task does not handle credentials and never writes to `~/.config` or provider credential files.
- **Prefer the toolchain the user already has.** If `cargo` is on `PATH`, suggest `cargo install dadrock` rather than installing Rust first. Same for `brew` vs `npm` vs `pnpm`.
- **One ACP provider is enough.** Do not install several "to be safe". Adding more is a runtime choice via `--agent <name>`.
- **Document, don't hide, the install method.** Tell the user exactly what command was chosen and why, so they can audit or undo.

## Quick reference

| Goal | Command |
|---|---|
| Check dadrock is installed | `dadrock --version` |
| Install dadrock from source | `cargo install dadrock` |
| Verify ACP wiring end-to-end | `dadrock agent check` |
| Force a specific provider | `dadrock agent check --agent <name>` |
| Smoke-test a draft | `dadrock agent draft --permission allow-once "Test"` |
| List local skills/prompt packs visible to Dadrock | `dadrock agent skills` |
| Run the MCP server directly | `dadrock mcp serve` |

## Example end-to-end

User: "set up dadrock with an agent on this Mac"

1. Inventory: `uname -s` → `Darwin`. `command -v dadrock` → not found. `command -v cargo` → found. `command -v claude` → found. `command -v codex`, `gemini`, `cursor-agent`, `copilot` → not found.
2. Recommend: install dadrock via `cargo install dadrock` (Rust available). For ACP, the existing Claude Code CLI is **not** the same binary as Claude Agent ACP — verify after dadrock is in.
3. Confirm with user, then run `cargo install dadrock`.
4. `dadrock --version` → prints version.
5. `dadrock agent check` → fails with "no ACP provider on PATH". Confirms Claude Code ≠ Claude Agent ACP.
6. Recommend: `pnpm add -g @agentclientprotocol/claude-agent-acp` (or `npm i -g …`). Confirm with user, run.
7. `dadrock agent check` → ok.
8. Tell the user: drafting with `dadrock agent draft "..."`; default permission is `deny`, use `--permission allow-once` per session, or `--permission allow-always` once trust is established.
