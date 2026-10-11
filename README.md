# DADR: Data Analysis Decision Records

Keep a record of the important choices behind your analysis, so you and your collaborators can remember why you made them.

Each decision is a Markdown file with four sections: **Context, Decision, Alternatives, Consequences**. Draft it, review it, and accept or reject it. When an accepted decision changes, accept a successor that supersedes it; the original moves to the archive unchanged.

**DADR** is the format. **Dadrock** is the CLI and local web application for writing, reviewing, and finding those records. Agent-assisted drafting is optional.

- [Install](docs-src/install.md)
- [Data Analysis Decision Records](docs-src/core.md)
- [Full specification](docs-src/specification.md)
- [Documentation website](https://vincentarelbundock.github.io/dadr/)

## Install from this checkout

With Rust/Cargo and Bun installed:

```sh
DADROCK_REQUIRE_JS_BUILD=1 cargo install --path crates/dadrock --bin dadrock
```

Bun builds the embedded web interface. The first build downloads dependencies and can take several minutes. Then, in your analysis project:

```sh
dadrock init
dadrock new "Handling missing income"
dadrock preview
```

Open the printed local URL, fill in the four sections, and review the decision before accepting it. No agent account is needed.

## Development

The Rust workspace contains `crates/dadr` (format library) and `crates/dadrock` (CLI, application logic, local server). The frontend lives in `crates/dadrock/web`.

```sh
cargo test --workspace
make web-test
make web-lint
make build
```

Website sources are in `docs-src/`; `docs/` is generated. See [CLAUDE.md](CLAUDE.md) for contributor guidance.
