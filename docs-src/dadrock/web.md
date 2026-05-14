# Web

**dadrock** can launch a localhost web server for interactive browsing, review, validation, and lifecycle management of decision records:

```bash
dadrock preview
dadrock preview --port 4005 path/to/project
```

The preview UI is the interactive workbench: a split record browser with lifecycle filters, candidate accept/reject actions, supersession controls, project and record validation queues, link health, settings, and local ACP agent drafting when an agent provider is configured.

## Tasks

The preview UI includes a Tasks tab for agent-assisted workflows over the decision ledger. Tasks can be built in, project-local under `dadr/tasks/`, or user-installed. Runs are saved under `dadr/runs/` and ignored by default because they may contain pasted context, source excerpts, and model transcripts.

Typed task results drive the matching UI: candidate drafts open in a one-panel editor, conflict findings use side-by-side review, diagnosis reports render as issue lists, and unknown third-party output kinds fall back to a report view with raw JSON.
