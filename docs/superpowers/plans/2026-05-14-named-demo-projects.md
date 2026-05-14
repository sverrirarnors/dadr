# Named Demo Projects Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add named Dadrock demo projects for lifecycle review, realistic code-to-record analysis, validation repair, and conflict cleanup, plus a screencast specification.

**Architecture:** Keep the existing lifecycle demo as the default. Add a core `DemoExample` enum and dispatch seeding through focused helper functions in `crates/dadrock/src/demo.rs`; expose the selection through `dadrock demo --example <name>`. Demo projects write static files only; the Penguins script may download data when users run it separately, but Dadrock does not run Python or fetch network data while seeding.

**Tech Stack:** Rust, clap, Dadrock project seeding helpers, Markdown docs, generated static Python script with `uv` inline metadata.

---

### Task 1: CLI And Command Plumbing

**Files:**
- Modify: `crates/dadrock/src/cli.rs`
- Modify: `crates/dadrock/src/commands/project.rs`
- Modify: `crates/dadrock/src/main.rs`
- Modify: `crates/dadrock/src/commands/catalog.rs`
- Test: `crates/dadrock/tests/project_commands.rs`

- [ ] Add failing tests that `demo_project_with_example` can seed a non-default example and that CLI parses `demo --example penguins`.
- [ ] Add a `DemoExample` core enum and a `DemoExampleArg` CLI enum.
- [ ] Pass the selected example from CLI to command layer to demo seeding.
- [ ] Preserve current `dadrock demo PATH` default behavior as lifecycle.

### Task 2: Demo Project Seeds

**Files:**
- Modify: `crates/dadrock/src/demo.rs`
- Test: `crates/dadrock/tests/project_commands.rs`

- [ ] Add tests for generated Penguins, broken, and conflicts examples.
- [ ] Keep existing lifecycle records unchanged except for routing through `seed_lifecycle_demo_project`.
- [ ] Add `penguins` project files: `README.md`, `analysis/penguins_ols.py`, `analysis/notes.md`, and initialized `dadr/`.
- [ ] Add `broken` project by seeding lifecycle plus intentional validation defects.
- [ ] Add `conflicts` project by seeding lifecycle plus overlapping/conflicting candidates.
- [ ] Ensure all demo examples refuse non-empty targets using the existing guard.

### Task 3: Docs And Screencast Spec

**Files:**
- Modify: `docs-src/getting-started.md`
- Modify: `docs-src/dadrock/cli.md`
- Modify: `docs/superpowers/specs/2026-05-14-ui-screencast-demo-plan.md`
- Create: `screencasts.md`

- [ ] Document the named demo examples.
- [ ] Add a root screencast spec with episode order, demo example/state requirements, and recording constraints.
- [ ] Update the earlier planning spec to reference named examples instead of one monolithic demo project.

### Task 4: Verification

**Files:**
- All changed files.

- [ ] Run focused Rust tests for project demo commands.
- [ ] Run `cargo test -p dadrock --features no-js-build`.
- [ ] Run formatting with `cargo fmt`.
- [ ] Review `git diff` for accidental changes or unrelated files.
