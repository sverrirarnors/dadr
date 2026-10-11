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
| Code-derived drafting | Future improvements | Extend the existing code-to-record/code-to-decisions task skill with richer detectors, source-reference scoring, and combined deduplication. |
| Faithfulness | Future RFC | Plan/code/manuscript drift checks should start as task/report workflows. See specs 12-14. |
| Conflict resolution | Future improvements | Persistent dismissals and richer cross-run triage remain future work. |
| Reports and publishing | Future product work | Static publishing/report generation needs a refreshed product spec before implementation. |
| Pipeline integrations | Future product work | Link DADRs to `targets`, DVC, Snakemake, and similar workflow metadata without copying full workflow definitions. |
| Team features | Future product work | Notifications, RBAC, scheduled checks, metrics, and shared review surfaces. |
| Evidence/provenance layer | Future product work | Structured evidence objects and provenance graphs should not change the canonical Markdown record format. |
| Lifecycle event log | Future product work | An append-only log of accept, reject, and supersede events recording at least the record `id`, the action, the event time, and the actor when known. The log is audit metadata and never overrides `status` or `successors` in front matter. |
| Decision history | In progress (dadr#zfgm) | Every change to an accepted decision becomes a new record; a decision's history and diffs are derived from `successors`, including splits and merges; a Git-diff check enforces that accepted and rejected records keep their title and body. Replaces the earlier "immutability baselines" idea. See [08-decision-history.md](08-decision-history.md). |

## Supersession Suggestions

Candidate creation should eventually include an advisory supersession pass,
regardless of whether the candidate comes from manual drafting, an agent draft,
a task result, future hooks, or future drafting workflows. The pass should
compare the new candidate against existing records and recent candidates with
overlapping normalized tags, especially `#kind/...`, `#topic/...`,
`#multiverse/...`, `#plan/...`, and future code-tag-derived classifications.

The classifier should label close matches as:

- `possible-supersession`: the new candidate appears to replace, amend, narrow,
  broaden, or reverse an accepted decision;
- `possible-duplicate`: the new candidate appears already covered by an
  existing decision or candidate;
- `conflict`: both decisions could apply but prescribe incompatible actions;
- `contradiction`: the new candidate asserts facts, plan references, or analytic
  choices inconsistent with an accepted record;
- `no-relation`: tag overlap is incidental.

The pass is advisory. It must not mark a record as `superseded`, rewrite
successors, reject a candidate, or block candidate creation on its own.

## Conflict Detection Helper

Future conflict triage should be durable across runs:

- scan accepted records and live candidates for pairs and clusters classified
  as `conflict`, `contradiction`, or `possible-supersession`;
- rank findings by tag overlap, recency, and strength of disagreement;
- present each finding with the records, proposed relationship, and explicit
  follow-up actions;
- route real resolution through normal lifecycle transitions rather than
  applying graph changes automatically;
- persist dismissals so a reviewer can mark a finding as "not a conflict" and
  avoid seeing the same pair every run.

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

The remaining choices do not change the core design: Markdown files on disk are
canonical, durable reasoning is captured in those files, and tooling resolves
links to code, data, plans, and outputs on demand.
