# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository purpose

DADR ("Data Analysis Decision Records") is a file-first Markdown format for capturing the choices behind an empirical analysis. This repo contains:

- The **`dadrock`** Rust binary (CLI, web UI, MCP server, agent tasks) that implements the DADR tooling — workspace under `crates/`.
- The **specification** (`docs-src/specification.md`) and supporting docs site, built with [Zensical](https://zensical.org) (`zensical.toml`, output to `docs/`, which is committed).
- **Future-work RFCs** in `specs/` (code tags, git hooks, faithfulness checks, roadmap). Implemented behavior belongs in `docs-src/`, not `specs/`.
- `skills/dadr/` — the agent skill shipped for DADR workflows (record anatomy, tags, task guides).
- `editors/vscode/` — the VS Code extension.
- `desktop/` — the Tauri desktop app (`dadrock-desktop`), a window around `dadrock preview`; its own Cargo workspace (see `desktop/README.md`).
- `packaging/linux/` + `scripts/` + `.github/workflows/` — release distribution: cargo-dist, then `release-desktop.yml`, which calls the Linux package, Homebrew and Scoop workflows.
- `reference/` — non-normative notes on related tools.

The user-facing artifacts keep the `dadr` name (`dadr/` project directory, `#dadr/...` code tags) even though the binary is `dadrock`.

## Common commands

### Rust

```bash
cargo build                                   # also builds the Svelte web UI via build.rs if `bun` is on PATH
cargo test --workspace --no-fail-fast
cargo test -p dadr                            # core record crate
cargo test -p dadrock --test project_commands # one integration-test file
cargo test -p dadrock -- accept_requires      # tests by name substring
cargo clippy --workspace --all-targets        # workspace forbids unsafe_code
cargo fmt --all
make install                                  # cargo install the dadrock binary
```

`agent::tests::fake_acp_conversation_streams_chat_response` is occasionally flaky; rerun before investigating.

### Web UI (`crates/dadrock/web`, SvelteKit + Bun)

```bash
bun run test:unit
bun run lint
```

`web/build/` is gitignored and embedded into the binary with `rust-embed`; building without Bun (or with `--features no-js-build`) falls back to a no-assets page.

The UI has three routes, `/inbox` (everything that needs a human decision), `/ledger` (what was decided), and `/draft` (the composer page), plus a project popover; Markdown bodies are edited with CodeMirror (`src/lib/components/editor/MarkdownEditor.svelte`); legacy routes (`/records`, `/tasks`, `/validation`, `/tags`, `/settings`, `/candidates/*`) redirect through `src/lib/navigation/legacy.ts`. The design vocabulary lives in `crates/dadrock/web/docs/web-ui-vocabulary.md`.

### Desktop app (`desktop/`, Tauri 2)

```bash
make desktop-dev PROJECT=path   # debug dadrock + app; on NixOS runs inside desktop/shell.nix
make desktop                    # release build (macOS .dmg / Linux binary)
cd desktop && cargo clippy --all-targets && cargo test
```

`desktop/` is deliberately outside the root workspace so `cargo test --workspace` never needs WebKitGTK; `make bump` keeps its version in step. The app finds `dadrock` via `$DADROCK_BIN`, a sibling executable, or `PATH`, and relies on the `dadrock-preview: listening on <url>` stderr line and `DADROCK_PREVIEW_EXIT_ON_STDIN_EOF` (both in `web.rs`) — keep them stable.

### Docs site

```bash
make serve     # uv run zensical serve  (live reload)
make build     # uv run zensical build  (writes to docs/)
```

## Architecture

### Workspace layout

- **`crates/dadr`** — the record format: front-matter parsing (`record.rs`), `RecordId`/`RecordRef` (`ids.rs`), the model (`model.rs`), and per-record `Diagnostic`s. No I/O beyond strings.
- **`crates/dadrock`** — library + binary. The library holds project discovery, scanning, lifecycle commands, validation, links, tags, tasks, agents, and MCP. `main.rs` (clap, `cli.rs`) and `web.rs` are the binary; with no arguments `dadrock` prints help, and `dadrock preview` starts the local web UI.

### Command surface

CLI verbs delegate to functions in `dadrock::commands` that return a `CommandEnvelope<T>`:

```
CommandEnvelope { command, ok, data, diagnostics, changed_paths }
```

This is what `--json` prints and what the web API and MCP return after mutations. **Do not bypass `commands::*` from the binary or web handlers** — mutations go through command functions so JSON output, diagnostics, and `changed_paths` stay consistent. `commands::COMMANDS` (`commands/catalog.rs`) is the single source of truth for the command catalog (`dadrock commands`, help, completions).

Key `dadrock` modules:

| Module | Owns |
|---|---|
| `project` | Project discovery (nearest ancestor `dadr/`, searching up to the git root), `init` |
| `records` | Record scanning, candidate creation (unique short ids), accept/reject/supersede |
| `index` | `ProjectIndex`: scanned records, CLI queries, record-link resolution |
| `validation` | `dadrock validate`: sections, ids, short-id collisions, successor graph, links, tags |
| `links` | Wikilink and Markdown-link checks |
| `tags` | Tag normalization, listing, validation |
| `templates` | Candidate template with built-in fallback |
| `tasks` | Task packages (`tasks/builtin/*/prompt.md`), runs under `dadr/runs/` |
| `agent` | ACP integration for drafting and revising candidates via local agents |
| `mcp` | MCP server |
| `demo` | Seeded example projects (`dadrock demo --example ...`) |

### Records and lifecycle

The spec (`docs-src/specification.md`) is authoritative. In short:

- Front matter: required `title`, `status`, `id`; optional `successors`, `tags`. Other keys are allowed, have no DADR meaning, and must be preserved on rewrite.
- Records never store dates: no `date` key and no ISO 8601 date/date-time value anywhere in front matter (`DiagnosticCode::DateInFrontMatter`). A record's time is always derived from its UUIDv7 id; lifecycle event times come from version control.
- `status` is `candidate | accepted | rejected`. "Superseded" is derived: an accepted record with non-empty `successors`.
- Accepting or rejecting changes `status` on the **same** record and keeps its UUIDv7 `id`. Dadrock moves the file between `dadr/candidates/`, `dadr/accepted/`, and `dadr/rejected/`.
- Accepted records need `## Context`, `## Decision`, `## Alternatives`, `## Consequences`, each exactly once and non-empty (`dadr::check_required_sections`).
- The short id (first 12 hex digits of the id) must be unique per project; it prefixes filenames and is the recommended wikilink target. Record links are exactly `[[<short-id>]]` or `[[<full-id>]]`; other wikilinks are ignored.

## Conventions

- **Paths** use `camino::Utf8PathBuf` end-to-end; non-UTF8 paths are a hard error.
- **IDs** are UUIDv7 only (`RecordId::parse` rejects other versions). Allocate new record ids through `records` so short ids stay unique.
- **Errors**: library code uses `thiserror`-based `DadrError`; `anyhow` only at the binary boundary (`main.rs`, `web.rs`).
- **Diagnostics vs. errors**: per-file parse problems are `Diagnostic`s on the scanned record, so one bad file doesn't fail a whole command. Reserve `DadrError` for fatal conditions.
- **Idempotency**: `dadrock init` never overwrites existing files. `dadrock demo` requires an empty target.
- **Tag vocabulary** lives in `docs-src/tags.md` (`kind/`, `confidence/`, `writer/`, `people/`, `topic/`, `plan/`, `axis/`, `multiverse/`, `robustness/`, `rejected/`). Keep prompts, skills, and demos consistent with it.

## Onboarding and agent boundaries

Agent tasks return proposals for human review. Do not invent an analyst's rationale from code alone or automatically accept a candidate. Advanced ACP/MCP integration details belong in `docs-src/llms.md`, not onboarding.

Use `dadrock accept`, `dadrock reject`, and `dadrock supersede` for everyday lifecycle actions; the `status` subcommands remain compatible. A replacement candidate can include an optional `## Replaces` section with a labeled full-UUID link. Acceptance validates and links that replacement while preserving the predecessor's body.

Document only behavior implemented in source. Keep planned features in `docs-src/roadmap.md`, with detailed future-work RFCs under `specs/`. Current validation does not establish code faithfulness or compare accepted files against Git history. Do not promise release binaries, registry packages, or editor integrations without checking their availability.
