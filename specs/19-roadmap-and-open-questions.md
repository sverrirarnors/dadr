# Roadmap and Open Questions

## Status

Living roadmap for future product work. It is not a promise that every future
RFC becomes a top-level CLI command. Implemented behavior should be documented
in `docs-src/` or generated references, not repeated here.

## Future Roadmap

| Area | Status | Notes |
|---|---|---|
| Code tags | Future RFC | Implement extraction of `#dadr/...` source comments, source-range binding, and optional validation. See [06-code-tags.md](06-code-tags.md). |
| Git hooks | Future RFC | Advisory staged-diff detection for consequential analysis changes. Should reuse inference/task detectors. See [10-git-hooks.md](10-git-hooks.md). |
| Plan/manuscript drafting | Future RFC | Add `plan-to-record` and `manuscript-to-record` task packages for drafting candidates from governing plans and reporting artifacts. See [09-plan-to-record.md](09-plan-to-record.md) and [11-manuscript-to-record.md](11-manuscript-to-record.md). |
| Code-derived drafting | Future improvements | Extend the existing `code-to-record` task with richer detectors, source-reference scoring, and combined deduplication. |
| Faithfulness | Future RFC | Plan/code/manuscript drift checks should start as task/report workflows. See specs 12-14. |
| Conflict resolution | Future improvements | Project-level persistent dismissals and richer cross-run triage for the existing `find-conflicts` task remain future work. |
| Reports and publishing | Future product work | Static publishing/report generation needs a refreshed product spec before implementation. |
| Pipeline integrations | Future product work | Link DADRs to `targets`, DVC, Snakemake, and similar workflow metadata without copying full workflow definitions. |
| Team features | Future product work | Notifications, RBAC, scheduled checks, metrics, and shared review surfaces. |
| Evidence/provenance layer | Future product work | Structured evidence objects and provenance graphs should not change the canonical Markdown record format. |
| Lifecycle event log | Future product work | An append-only log of accept, reject, and supersede events recording at least the record `id`, the action, the event time, and the actor when known. The log is audit metadata and never overrides `status` or `successors` in front matter. |

## Supersession Suggestions

Candidate creation should eventually include an advisory supersession pass,
regardless of whether the candidate comes from manual drafting, an agent draft,
a task result, future hooks, or future drafting workflows. The pass should
compare the new candidate against existing records and recent candidates with
overlapping normalized tags, especially `#kind/...`, `#topic/...`,
`#multiverse/...`, `#plan/...`, and future code-tag-derived classifications.

The classifier should label close matches as:

- `possible-supersession`: the new candidate appears to supersede an accepted
  decision by amending, narrowing, broadening, or reversing it;
- `possible-duplicate`: the new candidate appears already covered by an
  existing decision or candidate;
- `conflict`: both decisions could apply but prescribe incompatible actions;
- `contradiction`: the new candidate asserts facts, plan references, or analytic
  choices inconsistent with an accepted record;
- `no-relation`: tag overlap is incidental.

The pass is advisory. It must not mark a record as `superseded`, rewrite
successors, reject a candidate, or block candidate creation on its own.

## Conflict Detection Helper

The `find-conflicts` task already scans accepted records and live candidates,
reports each conflicting pair with its relationship and suggested follow-up
actions, and changes nothing; resolution goes through normal lifecycle actions.
The Inbox remembers a dismissed finding only in the reviewer's browser.

Future conflict triage should be durable across runs:

- classify findings with the labels above (`conflict`, `contradiction`,
  `possible-supersession`);
- rank findings by tag overlap, recency, and strength of disagreement;
- persist dismissals in the project so a reviewer can mark a finding as "not a
  conflict" and avoid seeing the same pair every run.

The helper must remain read-only until a human invokes a lifecycle action.

## Migration from ADRs

Migration from existing ADRs should be mostly mechanical:

- preserve filenames and Git history where possible;
- add minimal DADR front matter;
- map `Title`, `Status`, `Context`, `Decision`, and `Consequences` directly;
- map MADR `Context and Problem Statement` to `Context`;
- keep MADR `Decision Outcome` text inside `Decision`;
- preserve `Pros and Cons` as optional narrative;
- translate supersession chains directly;
- add classification tags such as `#kind/...` and `#topic/...` where useful.

## Pipeline Integrations

Pipeline integrations should link to existing workflow metadata rather than copy
workflow definitions into DADRs.

Recommended mappings:

- `targets`: target names and DAG nodes;
- DVC: `dvc.lock` stage definitions, params, deps, and outs;
- Snakemake: rule names, config keys, and rule outputs.

The goal is navigation between the decision record and the execution graph.

## Open Questions

- Should future plan deviations require a distinct reviewer role, or is normal
  record review enough when the deviation section includes reason, expected
  impact, and approval?
- Should inference and faithfulness remain task-first workflows, or should some
  modes graduate into dedicated top-level commands?
- What project-level configuration surface is needed for future hooks,
  inference, path allowlists/denylists, and remote-model policy?
- How much persistent evidence/provenance should Dadrock keep without weakening
  the invariant that Markdown records are the canonical durable artifact?
- Should the `## Supersedes` section become part of the DADR format, so that
  the reason a decision changed is defined independently of Dadrock? It is a
  Dadrock convention today.
- Should agents be able to draft a split? The `supersede-record` task returns
  one successor; a split would need several candidates from one run.
- Should DADRock show a record's text edits when the project is not in a Git
  repository? Text history comes from Git only.

The remaining choices do not change the core design: Markdown files on disk are
canonical, durable reasoning is captured in those files, and tooling resolves
links to code, data, plans, and outputs on demand.
