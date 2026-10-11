# Install

Dadrock is a single binary. The web interface is embedded in it, so a complete build needs [Rust](https://rustup.rs) and [Bun](https://bun.sh); a CLI-only build needs Rust alone.

## From source

Clone the repository and build with Cargo:

```sh
git clone https://github.com/vincentarelbundock/dadr.git
cd dadr
DADROCK_REQUIRE_JS_BUILD=1 cargo install --path crates/dadrock --bin dadrock
dadrock --version
```

The first build downloads dependencies and may take several minutes. `DADROCK_REQUIRE_JS_BUILD=1` makes the build fail rather than silently skip the web interface when Bun is missing.

## Without the web interface

For servers and continuous integration, build the command line only:

```sh
cargo install --path crates/dadrock --bin dadrock --features no-js-build
```

`dadrock preview` then shows a notice instead of the interface; every other command works. Install from a pinned revision for reproducibility.

## Editor integration

The repository ships a VS Code extension under `editors/vscode`. From a checkout, `make vscode` builds the binary and the extension and installs it with the VS Code CLI; `make positron` targets Positron. These targets need the editor CLI, Rust, Bun, and Node. You can also edit DADR files in any editor and run `dadrock preview` alongside it.

## Optional: a local agent

Drafting and conflict checks use a local agent speaking the Agent Client Protocol, such as `claude-agent-acp`. Dadrock detects one on `PATH` automatically; see [LLM agents](llms.md) to configure a specific provider. Nothing else in Dadrock depends on it.

## Next

[Record your first decision](dadrock.md#first-decision) with the software, or read what a [Data Analysis Decision Record](core.md) is.
