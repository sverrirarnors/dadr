!!! warning
    **dadrock** is under heavy development — pre-alpha release. Expect breaking changes.

<img src="../assets/dadrock_logo.svg" alt="dadrock" style="display:block; margin: 0 auto 2rem; max-width: 22rem;">

**DADR** is the standard, the decision record format, and the project vocabulary. 

**dadrock** is a Rust tool that implements DADR. It offers convenience functions to preview, create, edit, manage, and validate decision records. It can also direct LLM agents to execute a variety of tasks related to DADR.

## Install

Prebuilt binaries are published with each [release](https://github.com/vincentarelbundock/dadr/releases). A single-line installer script is available for macOS, Linux, and Windows. From source:

```bash
cargo install dadrock
```

Confirm the binary is on `PATH`:

```bash
dadrock --version
```

See the [current capabilities, validation, and CI overview](index.md), or [create your first decision](../getting-started.md).

## Project layout

The DADR format does not mandate a directory structure. **dadrock** uses this
one, created by `dadrock init`:

```text
dadr/
  accepted/
    01938b8e4d70-income-preprocessing.md
    01938b8f9d71-drop-missing-treatment.md
  candidates/
    01938ba1c4f3-country-exclusions.md
  rejected/
    01938b8d9c10-deprecated-cohort-filter.md
  templates/
    candidate.md                                    # optional
  tasks/
  runs/
    .gitignore
```

- `dadr/accepted/` holds accepted decisions, current or superseded.
- `dadr/candidates/` holds active candidates.
- `dadr/rejected/` holds rejected records.
- `dadr/templates/` holds optional template overrides for new candidates.
- `dadr/tasks/` holds project-local task packages, such as
  `dadr/tasks/<task-id>/task.toml` plus the prompt file it names.
- `dadr/runs/` holds task run state and is ignored by version control.

`dadrock accept` changes a candidate's `status` to `accepted` and moves
the file to `dadr/accepted/`. `dadrock reject` changes it to `rejected`
and moves it to `dadr/rejected/`. The `id` never changes. `dadrock
supersede` appends the new record's `id` to the old record's `successors` and
leaves both files in `dadr/accepted/`.

The directory is a convenience: `status` in the front matter is authoritative.
`dadrock validate` warns when a record sits in a directory that does not match
its status.
