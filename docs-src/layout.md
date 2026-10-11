# Project layout

The DADR format does not mandate a directory structure. **DADRock** uses this one, created by `dadrock init`:

```text
dadr/
  inbox/
    country-exclusions.md
  ledger/
    income-preprocessing.md
    drop-missing-treatment.md
  archive/
    deprecated-cohort-filter.md
  templates/
    candidate.md                # optional
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
| `dadr/tasks/` | Optional project-specific agent workflows and [overrides of built-in tasks](llms.md#custom-workflows-and-runs) (`dadr/tasks/<task-id>.md`: YAML front matter for the manifest, Markdown body for the prompt) |
| `dadr/runs/` | Agent-run inputs, results, and logs; ignored by version control |

`dadrock accept` changes a candidate's `status` to `accepted` and moves the file from `dadr/inbox/` to `dadr/ledger/`. `dadrock reject` changes it to `rejected` and moves it to `dadr/archive/`. The `id` never changes. Superseding a decision, by [accepting its successor](changing.md#accepting-a-successor) or with `dadrock supersede`, appends the successor's `id` to the predecessor's `successors` and moves the predecessor to `dadr/archive/`; its `status` stays `accepted`.

Filenames carry no meaning: DADRock identifies a record by its `id`, never by its filename ([specification §5](specification.md#identifiers)). It names a new record after its title, `income-preprocessing.md`, and adds the short id, `income-preprocessing-01938b8e4d70.md`, only when the inbox already has that name. Accepting, rejecting, and superseding keep the filename and change only the folder; the short id is added only when the target folder already has that name. A name that another record leaves in the same operation counts as free, so a successor with its predecessor's title takes the predecessor's filename in the ledger. Renaming a record file does not change the record.

The directory is a convenience: the front matter is authoritative. `dadrock validate` warns when a record sits in a directory that does not match its `status` and `successors`. Moving a file breaks relative Markdown links to its path; record wikilinks use ids and are unaffected. Use `dadrock config` to inspect the resolved layout. Keep the record directories in version control; run state may contain pasted context and source excerpts and is ignored by default. **Clear agent runs**, in the project menu of the [web interface](dadrock/web.md), deletes `dadr/runs/`.

Earlier versions of DADRock used `candidates/`, `accepted/`, and `rejected/`. DADRock no longer reads those directories: move candidates to `inbox/`, current decisions to `ledger/`, and rejected and superseded records to `archive/`.
