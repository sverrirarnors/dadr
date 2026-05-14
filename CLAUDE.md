# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository purpose

DADR ("Data Analysis Decision Records") is a file-first Markdown format for capturing the choices behind an empirical analysis. This repo contains:

- The **`dadrock`** Rust binary (CLI + web UI) that implements the DADR tooling — workspace under `crates/`.
- The **specification** (`docs-src/specification.md`) and supporting docs site, built with [Zensical](https://zensical.org) (`zensical.toml`, output to `docs/`).
- **Working specs** in `specs/` covering material more detailed than the website summaries (architecture, inference, faithfulness checks, roadmap). The website is authoritative for the format itself; `specs/` is authoritative for implementation choices.
- `implementation/` (backlog/plan) and `reference/` (notes on related tools) are non-normative working notes.
- DADR's own decisions are drafted in `draft/decisions/`.

The user-facing standard artifacts keep the `dadr` name (`dadr.toml`, `.dadr-dir`, `decisions/`, `#dadr/...` tags) even though the binary is `dadrock`.

## Common commands

### Rust (the binary)

```bash
cargo build                                   # debug build of the workspace
cargo build --release
cargo test                                    # run all unit + integration tests
cargo test -p dadrock-core                    # one crate
cargo test -p dadrock-core project_commands   # one integration-test file
cargo test -p dadrock-core -- new_record_creates_candidate   # one test by name (substring)
cargo clippy --workspace --all-targets        # lints (workspace forbids unsafe_code)
cargo fmt --all
cargo install --path crates/dadrock --bin dadrock       # install via `make install`
```

The web UI server and CLI live in `crates/dadrock`; the core crate has integration tests under `crates/dadrock-core/tests/`. Tests build temporary projects in `std::env::temp_dir()` and assert on returned envelopes.

### Docs site

The Makefile wraps the Python doc-site toolchain:

```bash
make serve     # uv run zensical serve  (live reload)
make build     # uv run zensical build  (writes to docs/)
make clean     # rm -rf docs
make install   # cargo install the dadrock binary
```

`make web-deps` (== `uv sync`) installs Python deps into `.venv/`.

## Architecture

### Workspace layout

Two crates, both under `crates/`:

- **`dadrock-core`** — pure library: parsing, the project model, command implementations, agent (ACP) plumbing. No terminal code, no `anyhow`. All errors flow through `DadrError` (`error.rs`) plus per-domain `Diagnostic` lists.
- **`dadrock`** — the `dadrock` binary. `main.rs` is a hand-written argv router; with no args it starts the local web preview server with the same defaults as `dadrock preview`.

`unsafe_code = "forbid"` is set as a workspace lint.

### Command surface (the heart of `dadrock-core`)

Every CLI verb in `crates/dadrock/src/main.rs` delegates to a function in `dadrock_core::commands` that returns a `CommandEnvelope<T>`:

```
CommandEnvelope { command, ok, data, diagnostics, changed_paths }
```

This same envelope is what `--json` prints and what the web API returns after mutations. **Do not bypass `commands::*` from the binary or web handlers** — every mutation should go through a command function so JSON output, diagnostics, and `changed_paths` stay consistent across surfaces. `CommandContext::current_dir()` → `discover_project()` is the standard entry; `commands::COMMANDS` is the single source of truth for the command catalog (`dadrock commands`, completions, help).

Command modules in `dadrock-core` and what they own:

| Module | Owns |
|---|---|
| `project` | Project discovery (`.dadr-dir` marker → `decisions/` dir → fallback), `dadr.toml` parsing, `init` |
| `records` | Record file I/O, candidate creation, lifecycle transitions (accept/reject/supersede), find/list/scan |
| `model` | `Record`, `RecordFrontMatter`, `RecordStatus`, `Tag`, `BibliographyRef` (the on-disk schema) |
| `ids` | `RecordId` — UUIDv7 only; non-v7 UUIDs are a parse error |
| `markdown::frontmatter` | Splitting `---` front matter from the body |
| `tags` | `#dadr/...` tag normalization, listing, validation |
| `links` | Wikilink discovery and validation (consumed by `validate links`) |
| `templates` | Project templates with built-in fallbacks (`TemplateSource::{Project,BuiltIn}`) |
| `validation` | Project validation engine behind `validate` |
| `demo` | Seeded showcase project |
| `agent` | ACP (Agent Client Protocol) integration — drafting and revising candidates via locally installed ACP agents |

### Records and lifecycle

A record is a Markdown file with YAML front matter (`title`, `status`, `id`, `successors`, `tags`, optional `bibliography`) and four prose sections (`Context`, `Decision`, `Alternatives`, `Consequences`). `RecordStatus` is `candidate | accepted | rejected | superseded`. Candidates live in `<decisions>/candidates/`; accepted/superseded records live at the project root. Acceptance and supersession **never edit in place** — they create a new record (fresh UUIDv7) and update the predecessor's `status` and `successors`.

### Agent (ACP) integration

`crates/dadrock-core/src/agent.rs` (~2.4k lines, the heaviest module) wraps `agent-client-protocol` + `agent-client-protocol-tokio` to drive locally installed ACP providers (`claude-agent-acp`, `claude-code-acp`, `gemini-acp`, etc.). Provider resolution: `--agent` flag → `DADROCK_ACP_AGENT` env → `KNOWN_ACP_PROVIDERS` auto-detect on `PATH`. The protocol identifier `dadrock-agent-draft` v1 (`DADROCK_DRAFT_PROTOCOL`/`DADROCK_DRAFT_VERSION`) defines the structured envelope agents must return; `validate_draft_response` / `validate_revision_response` enforce shape before anything is written to disk.

### Web UI

`crates/dadrock/src/web.rs` owns the local preview server and API routes. Static assets live under `crates/dadrock/src/web/static/`. The web handlers should stay thin: parse HTTP requests, call `dadrock_core::commands` or read-only project/index helpers, and return JSON snapshots or mutation envelopes.

## Conventions worth knowing

- **Paths** use `camino::Utf8PathBuf` end-to-end. Treat non-UTF8 paths as a hard error (`path_to_utf8` in `project/mod.rs`).
- **IDs** are UUIDv7 only. `RecordId::parse` rejects v4. Generate via `RecordId::new_v7()`.
- **Errors**: `dadrock-core` uses `thiserror`/`DadrError`; `dadrock` uses `anyhow` at the binary boundary (`main.rs`, `web.rs`). Don't pull `anyhow` into the core crate.
- **Diagnostics vs. errors**: parse-time issues become `Diagnostic` entries on `ParsedRecord` so a single bad file doesn't fail a whole command. Reserve `DadrError` for genuinely fatal conditions.
- **Idempotency**: `dadrock init` must never overwrite an existing config or record. `dadrock demo` is intentionally *not* idempotent (it requires an empty `decisions/`).
- **JSON output**: most commands accept `--json`; the web API uses the same `CommandEnvelope`. New commands should preserve this dual-output contract.

## Spec ↔ implementation mapping

When changing behavior, check the relevant spec first. The website (`docs-src/`) is authoritative for the format and the user-facing CLI; `specs/` is authoritative for implementation choices not yet on the website. `specs/SPEC_COVERAGE_MATRIX.md` is the ledger that tracks which spec clauses have implementation coverage.
