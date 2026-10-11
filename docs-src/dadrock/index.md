<img src="../assets/dadrock_logo.svg" alt="dadrock" style="display:block; margin: 0 auto 2rem; max-width: 22rem;">

**DADR** is the standard, the record format, and the project vocabulary. 

**dadrock** is one Rust tool that implements DADR. It offers convenience functions to manage decision records, validate them, direct LLM agents, etc.

## At a glance

```bash
dadrock init                                # scaffold dadr/
dadrock new "Decision title"                # draft a candidate
dadrock list --status candidate             # browse the queue
dadrock                                     # local web UI on :4004
dadrock status accept <id>                  # candidate → accepted
dadrock status supersede --from OLD --to NEW # link a new accepted decision to the one it supersedes
dadrock task list                           # list built-in and project tasks
dadrock task run code-to-record --input-json '{"path":"analysis/"}'
dadrock validate                            # full project validation
dadrock preview --port 4005                 # local web UI with explicit options
```

The full surface is documented in the pages above.

**dadrock** is the Rust binary that implements DADR. It is repository-local by default. Records are the canonical files; the binary orchestrates lifecycle transitions, validation, tag inspection, local preview, task-based agent workflows, and optional MCP integration.

## Install

Prebuilt binaries are published with each [release](https://github.com/vincentarelbundock/dadr/releases). A single-line installer script is available for macOS, Linux, and Windows. From source:

```bash
cargo install dadrock
```

Confirm the binary is on `PATH`:

```bash
dadrock --version
```

## Project layout

```text
dadr/
  accepted/
    01938c20-a3b4-7000-...md
  candidates/
    01938b8e-4d70-7abc-...md
  rejected/
  templates/
    candidate.md
  tasks/
  runs/
```

- `dadr/accepted/` holds accepted records, current or superseded.
- `dadr/candidates/` holds active proposals.
- `dadr/rejected/` holds rejected records.
- `dadr/templates/` holds optional project template overrides.
- `dadr/tasks/` holds project-local task packages such as `dadr/tasks/<task-id>/task.toml` plus the prompt file named by that manifest.
- `dadr/runs/` holds ignored task run state: inputs, metadata, logs, results, and other execution artifacts.

## Validation

Validation is concerned with the **internal coherence** of the records: front matter parses, required fields are present, status values are valid, IDs are well-formed and unique, links resolve, the supersession DAG is acyclic, and accepted records have not silently mutated since acceptance. Checks against external anchors (plan, code, manuscript) live under [Existing projects](../getting-started.md#existing-projects).

| Layer | What it checks | Command |
|---|---|---|
| Schema | Per-record front matter, required fields, well-typed values, unique UUIDv7. | `dadrock validate` |
| Structural | Per-record body sections, plan-deviation requirements. | `dadrock validate` |
| Project-wide | DAG integrity, short-id collisions, accepted-record immutability, link resolution across the project. | `dadrock validate` |

`dadrock validate` runs the full project suite. Use `dadrock validate records` to target record front matter and structure, `dadrock validate links` to target links, and `dadrock validate tags` to target tag syntax.

### Schema

- front matter exists and parses as YAML;
- required fields (`title`, `status`, `id`) are present and well-typed;
- `id` is a well-formed UUIDv7, and both the `id` and its 12-character short id are unique within the project;
- `status` is one of `candidate`, `accepted`, or `rejected`;
- front matter contains no `date` key and no ISO 8601 date or date-time value: a record's time comes only from its UUIDv7 `id`;
- an accepted record with a non-empty `successors` array is **superseded** — derived from the graph rather than expressed as a status value;
- `successors` entries, when supplied, are well-formed UUIDv7s on accepted records;
- `tags` entries follow the format rules in the [DADR specification](../specification.md#tags);
- `#plan/<id>` tags resolve against local plans or registry metadata when supplied.

### Structural

- accepted records contain `Context`, `Decision`, `Alternatives`, and `Consequences` sections, each exactly once and each with text;
- accepted records contain no placeholders such as `{...}` or `TBD`;
- records that document plan deviations include a `## Plan deviation` section
  with reason, impact assessment, and reviewer;
- records that amend a plan link to the approved amendment or review event.

### Project validation

```bash
dadrock validate
dadrock validate --json
dadrock validate records path/to/record.md
dadrock validate links
dadrock validate tags
```

Checks include:

- **Preambles.** Schema and structural validation for every record.
- **DAG integrity.** `successors` entries resolve to accepted records; no cycles; candidates and rejected records do not carry supersession edges.
- **Short-id collisions.** No two records share the same filename or the same 12-character short id.
- **Accepted-record immutability.** Validation finds the commit at which each record's `status` first became `accepted` and treats the file's blob hash from that commit as the immutability baseline. Subsequent edits that change the hash are flagged.
- **Link resolution.** Re-resolves every Markdown link and wikilink: referenced local paths exist; record links (`[[<short-id>]]` or `[[<full-id>]]`) resolve to a record.

`--json` emits a machine-readable version for CI consumption. Validation never edits accepted-record body content or changes a record's UUIDv7.

## Link checking

The format spec defines two link syntaxes: standard Markdown links for files, code, and external URLs, and wikilinks `[[<short-id>]]` for references between records. Wikilinks are supported by most PKM tools but not by base Markdown renderers (Pandoc, Quarto, GitHub, GitLab), which display them as literal `[[...]]`.

```bash
dadrock validate links
```

`dadrock validate links` verifies that every record link (`[[<short-id>]]` or `[[<full-id>]]`) resolves to a record and that Markdown links to local files exist. Wikilinks with other targets are left alone.

The preview UI (`dadrock preview`) runs a wikilink → Markdown pass before rendering. Wikilinks remain the source of truth on disk; resolving them at render time means a record file moving (e.g., on lifecycle transition) does not silently break links.

## CI integration

A minimal GitHub Actions workflow:

```yaml
name: dadrock
on:
  pull_request:
  push:
    branches: [main]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install dadrock
        run: cargo install dadrock
      - name: Validate records
        run: dadrock validate
      - name: Run full project validation
        run: dadrock validate --json
```

Faithfulness checks against plan, code, and manuscript live in separate jobs documented under [Existing projects](../getting-started.md#existing-projects).

## Metrics

A handful of decision-focused metrics, derived from records plus version-control history or tool-maintained lifecycle events where event timing is needed:

| Metric | Definition | Purpose |
|---|---|---|
| Candidate-to-accepted ratio | accepted / total candidates | Detector precision |
| Median review latency | record UUIDv7 creation time to acceptance event time | Process health |
| Broken-link rate | broken / total resolved Markdown + wikilinks | Reference health |
| Robustness follow-through | completed sensitivity records / records that flagged follow-up | Robustness discipline |
| Supersession lag | new decision UUIDv7 timestamp − superseded decision UUIDv7 timestamp | Responsiveness |
| Coverage estimate | accepted records against detected hotspots | Documentation completeness |
