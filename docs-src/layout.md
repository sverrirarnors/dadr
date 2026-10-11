# Project layout

The DADR format does not mandate a directory structure. **DADRock** uses this one, created by `dadrock init`:

```text
dadr/
  inbox/
    01938ba1c4f3-country-exclusions.md
  ledger/
    01938b8e4d70-income-preprocessing.md
    01938b8f9d71-drop-missing-treatment.md
  archive/
    01938b8d9c10-deprecated-cohort-filter.md
  templates/
    candidate.md                                    # optional
  tasks/
  runs/
    .gitignore
```

| Directory | Contents |
|---|---|
| `dadr/inbox/` | Candidates: proposals awaiting review |
| `dadr/ledger/` | Decisions in force: accepted records without successors |
| `dadr/archive/` | Records kept but no longer in force: rejected records and superseded records |
| `dadr/templates/` | Optional template overrides for new candidates |
| `dadr/tasks/` | Optional project-specific agent workflows (`dadr/tasks/<task-id>/task.toml` plus its prompt file) |
| `dadr/runs/` | Agent-run inputs, results, and logs; ignored by version control |

`dadrock accept` changes a candidate's `status` to `accepted` and moves the file from `dadr/inbox/` to `dadr/ledger/`. `dadrock reject` changes it to `rejected` and moves it to `dadr/archive/`. The `id` never changes. Superseding a decision, by accepting its successor or with `dadrock supersede`, appends the successor's `id` to the predecessor's `successors` and moves the predecessor to `dadr/archive/`; its `status` stays `accepted`.

The directory is a convenience: the front matter is authoritative. `dadrock validate` warns when a record sits in a directory that does not match its `status` and `successors`. Moving a file breaks relative Markdown links to its path; record wikilinks use ids and are unaffected. Use `dadrock config` to inspect the resolved layout. Keep the record directories in version control; run state may contain pasted context and source excerpts and is ignored by default.

Earlier versions of DADRock used `candidates/`, `accepted/`, and `rejected/`. DADRock no longer reads those directories: move candidates to `inbox/`, current decisions to `ledger/`, and rejected and superseded records to `archive/`.
