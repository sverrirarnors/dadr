# Install Dadrock and optional agent support

First inspect the user's existing Dadrock installation and toolchain. An explicit request to install authorizes installation; honor any environment permission requirements.

## Dadrock from source

Use a checkout of this repository. Rust/Cargo is required; Bun is required for the embedded web interface. From its root:

```sh
DADROCK_REQUIRE_JS_BUILD=1 cargo install --path crates/dadrock --bin dadrock
dadrock --version
```

Do not assume registry packages, prebuilt releases, or installer scripts are published. A CLI-only build may use `--features no-js-build`, but cannot provide the full web review interface. Explain that limitation if choosing it.

Initialize the user's analysis project with `dadrock init`. Manual writing and review require no model provider.

## Optional agent support

For agent workflows, install one ACP-compatible provider or bridge using its official instructions and the user's chosen account. A regular Claude Code or Codex executable is not necessarily an ACP bridge. Authentication belongs to the provider; never print credentials or assume existing authentication carries over.

Install this repository's `skills/dadr/` in the agent's supported skill directory. Candidate locations include `.agents/skills/dadr`, `~/.agents/skills/dadr`, `~/.codex/skills/dadr`, and `~/.claude/skills/dadr`. The agent's own discovery rules are authoritative.

```sh
dadrock agent check
dadrock agent skills
```

Task execution separately checks that the agent can load the installed `dadr` skill. If setup fails, inspect the provider executable, its authentication, and explicit `--agent` selection before changing configuration.

## First draft

```sh
dadrock task run code-to-record --path analysis/ --limit 5
```

Review proposed candidates and supply missing rationale; never automatically accept them. Permission requests default to deny. Select an appropriate permission mode explicitly only when the requested workflow needs it.

The website's `docs-src/llms.md` documents advanced configuration and MCP integration. Prefer a single working provider over installing several alternatives.
