# Contributor guidance

DADR is the Markdown record format. Dadrock is its CLI and local web application. Read `docs-src/specification.md` for normative format rules; implementation plans in `specs/` may describe future work and must be checked against source before documenting features as available.

## Layout

- `crates/dadr`: format library (parsing, IDs, model and format rules).
- `crates/dadrock`: application logic, commands, CLI, project discovery, validation, agents, tasks and local web server.
- `crates/dadrock/web`: Svelte frontend; built with Bun and embedded by the Rust build script.
- `editors/vscode`: editor extension source.
- `docs-src`: documentation sources; `zensical.toml` controls the site. `docs/` is generated output.
- `skills/dadr`: agent-facing workflow guidance.

## Commands

```sh
cargo build
cargo test --workspace
cargo test -p dadr
cargo test -p dadrock
cargo clippy --workspace --all-targets
cargo fmt --all
make web-test
make web-lint
make install
make build
make serve
```

Rust and Bun are required for a build with the embedded web UI. `no-js-build` permits a CLI-only build; preview then shows a no-assets notice. `make build` regenerates CLI documentation and the website. `make web-deps` installs the Python documentation dependencies, not frontend dependencies.

## Records and lifecycle

`dadrock init` creates `dadr/accepted/`, `dadr/candidates/`, `dadr/rejected/`, `dadr/templates/`, `dadr/tasks/`, and `dadr/runs/`. Use project discovery helpers rather than assuming paths in application code. Init is idempotent; demos require a new or empty target directory.

A record has three required fields (`title`, `status`, `id`) and four sections (`Context`, `Decision`, `Alternatives`, `Consequences`). `successors`, `tags`, and bibliography metadata are optional. Stored statuses are `candidate`, `accepted`, and `rejected`. Superseded is a derived state: an accepted record with a non-empty successors list.

Acceptance preserves the ID and moves a candidate to the accepted directory. Rejection also preserves identity. A changed decision gets a separate record; supersession links accepted successors to their predecessors. Preserve accepted substantive content; the specification permits minor non-substantive corrections. Never infer human approval from agent output.

## Application boundaries

Route CLI and web mutations through `dadrock::commands` so `CommandEnvelope` diagnostics and changed paths stay consistent. Keep web handlers thin. Command metadata and generated CLI documentation must agree with the CLI definition. Running `dadrock` without a subcommand prints help; `dadrock preview` starts the web interface.

IDs must be UUIDv7. Use existing ID generation and parsing helpers. Preserve the distinction between recoverable record diagnostics and fatal application errors. Respect the workspace's unsafe-code prohibition.

Agent tasks return proposals for human review. Do not invent an analyst's rationale from code alone or automatically accept a candidate. Advanced ACP/MCP integration details belong in `docs-src/llms.md`, not onboarding.

## Documentation checks

Document only behavior implemented in source. Keep planned features in `docs-src/roadmap.md`. In particular, current validation does not establish code faithfulness or compare accepted files against Git history. Do not promise release binaries, registry packages, or editor integrations without checking their availability.
