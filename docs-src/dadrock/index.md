---
title: Installation
---

!!! warning
    **DADRock** is under heavy development — pre-alpha release. Expect breaking changes.

<img src="../assets/dadrock_logo.svg" alt="DADRock" style="display:block; margin: 0 auto 2rem; max-width: 22rem;">

**DADR** is the standard: the decision record format and the project vocabulary.

**DADRock** is a Rust tool that implements DADR. It previews, creates, edits, and validates decision records, and it can direct local LLM agents through tasks on the decision ledger. Everything it writes is a Markdown file you could have written yourself.

```sh
dadrock init
dadrock new "Decision title"
dadrock preview
dadrock accept ID
dadrock validate
```

Running `dadrock` without a subcommand prints help. `preview` starts the local [web interface](web.md), normally at port 4004.

DADRock is a single binary. The web interface is embedded in it, so a complete build needs [Rust](https://rustup.rs) and [Bun](https://bun.sh); a CLI-only build needs Rust alone.

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

## Optional: a local agent

Drafting and conflict checks use a local agent speaking the Agent Client Protocol, such as `claude-agent-acp`. DADRock detects one on `PATH` automatically; see [LLM agents](llms.md#acp) to configure a specific provider. Nothing else in DADRock depends on it.

## Next

[Record your first decision](web.md#first-decision) in the web interface, set up your [editor](editors.md), or read what a [Data Analysis Decision Record](../core.md) is.
