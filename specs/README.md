# DADR Specs

This directory contains future Dadrock design notes and improvement paths that
are not yet fully covered by the implementation or public documentation. The
normative record format, lifecycle rules, links, tags, and conformance language
live in [`docs-src/specification.md`](../docs-src/specification.md). Dadrock's
project layout is documented in
[`docs-src/dadrock/dadrock.md`](../docs-src/dadrock/dadrock.md). Existing Dadrock
behavior belongs in `docs-src/` or generated CLI docs, not in this directory.

## Spec Index

| Spec | Status | Purpose |
|---|---|---|
| [06-code-tags.md](06-code-tags.md) | Future RFC | Inline `#dadr/...` code-comment extraction, source-range binding, and code-driven tag checks. |
| [07-agent-and-app-architecture.md](07-agent-and-app-architecture.md) | Future RFC | Future app, agent, team-review, permissions, notification, and API constraints. |
| [08-decision-history.md](08-decision-history.md) | In progress | Every change to a decision as a new record, version history with diffs, splits and merges, and a Git-diff immutability check. |
| [09-plan-to-record.md](09-plan-to-record.md) | Future RFC | Draft candidate DADRs from SAPs, PAPs, preregistrations, protocols, and other plan artifacts. |
| [10-git-hooks.md](10-git-hooks.md) | Future RFC | Advisory Git hook workflow for detecting consequential analysis changes. |
| [11-manuscript-to-record.md](11-manuscript-to-record.md) | Future RFC | Draft candidate DADRs from manuscript prose, captions, tables, figures, and supplements. |
| [12-faithfulness-plan.md](12-faithfulness-plan.md) | Future RFC | Plan-to-DADR coverage and deviation review checks. |
| [13-faithfulness-code.md](13-faithfulness-code.md) | Future RFC | Checks for drift between records and code/config/notebooks/workflows. |
| [14-faithfulness-manuscript.md](14-faithfulness-manuscript.md) | Future RFC | Checks for drift between records and manuscript or report claims. |
| [19-roadmap-and-open-questions.md](19-roadmap-and-open-questions.md) | Living roadmap | Cross-cutting future improvements and open product questions. |

`library.bib` is supporting reference material for DADR rationale and examples.

## Maintenance Rules

- Do not duplicate normative record-format language here. Link to
  `docs-src/specification.md` instead.
- Mark future work as future work. Avoid documenting unimplemented commands as
  if they already exist.
- Do not keep implementation notes for features that already exist. Update
  public docs or generated references instead.
- Prefer task-system extensions for new agent-assisted workflows unless a new
  top-level CLI command has a clear reason to exist.
