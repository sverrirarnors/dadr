---
id: find-conflicts
title: "Find conflicting decisions"
description: "Audit selected DADRs for contradictions, duplicates, or unresolved overlap that need follow-up."
version: 1
inputs:
  - name: status_scope
    label: "Records to scan"
    type: select
    default: "live"
    options: ["live", "candidates", "accepted", "all"]
requirements:
  records: none
  record_status: all
  paths: none
  notes: optional
  scope: live
output:
  kind: conflict_set.v1
mcp_access: full
tools:
  - name: dadrock.list_records
    mode: read
---

# Find conflicts

Detect and surface pairs (or groups) of DADR records whose `Decision` sections express incompatible choices, then scaffold a resolution via supersession or rejection. Complements [code-to-record.md](code-to-record.md).

This task exists because of a failure mode that's common when candidates are drafted by an LLM on every git commit: the project accumulates a *history* of decisions ("do X" then "actually don't do X") instead of a single current state. Supersession is how DADR resolves this; this task automates the detection and proposes the resolution.

Trigger phrases:

- "find conflicting DADRs", "any decisions that contradict each other?"
- "resolve duplicate decisions"
- "clean up the dadr/ backlog"
- "check dadr/ for inconsistencies"

## Preconditions

Shared preconditions in [../SKILL.md](../SKILL.md) apply. In addition:

- `dadr/` should have enough records that conflicts are plausible. With fewer than ~5 records, tell the user there is little to find and offer a manual read instead.
- The repo should be clean. Resolution runs `dadrock status` commands that move files; tangling them with unstaged work makes review hard.

## What counts as a conflict

A conflict is a pair (or group) of records whose `Decision` sections cannot both be true at the same time on the same analytic surface.

**Strong signals:**

- Same `kind/<surface>` and incompatible `Decision` text.
- Both records' `Context` links overlap on the same file/line range.
- Both records are referenced by a `#dadr/<uuid>` code tag at the same source location.
- Same plan reference (`plan/<id>`) with incompatible plan-related prose.

**Weaker signals — surface for human review:**

- Same `topic/`, `plan/`, or title noun phrase with different `kind/` values.
- Title near-duplicates (overlapping noun phrases).

**NOT a conflict:**

- One record's UUID appears in the other's `successors` array — already resolved.
- Both are `rejected`.
- One is `kind/robustness` and the other is the primary analysis choice it
  explicitly varies — these can coexist by design.
- They describe different steps of the same pipeline (e.g., one is the row filter, the other is the imputation order). Read the `Decision` text, not just the `Context` paths.

## Cheap-first protocol

Body reads are the expensive part of this task — a project with 50 records and 5 conflict clusters should not require reading 50 bodies. Use deterministic CLI filters to narrow the corpus first, then read bodies only for the small clusters that survive.

What dadrock can do for free (no body reads):

- **Skip `rejected`** via `--status` filter.
- **Skip already-resolved chains** via the `superseded: true` field on each summary (returned by `dadrock list --json`).
- **Cluster by `kind/`** via `dadrock list --tag kind/X --json`.
- **Sub-cluster by `topic/` or `plan/`** when those tags exist.
- **Keyword pre-cluster** via `dadrock list --query <substring> --json` — matches case-insensitively across filename and title (NOT body). Use for noun-phrase neighbors like `winsorize`, `missing`, `cluster`, `treatment`.
- **Discover the tag axes in use** via `dadrock tags list --json`.
- **Order newest-revises-oldest** via the `timestamp_ms` field on each summary (derived from the UUIDv7).

What still requires body reads:

- The `Decision` text itself (to judge incompatibility).
- `Context` source links (to confirm file/line overlap).
- Successor target IDs (`superseded: true` is a boolean flag — it tells you a record HAS successors, not who they are; for skipping superseded records, the boolean is enough).

Order of operations: tag/keyword filter → drop `superseded` → sort by `timestamp_ms` → read bodies for the residual.

## Workflow

### Step 1 — Enumerate the live corpus

Live records are non-rejected, non-superseded:

```bash
dadrock list --status candidate --json
dadrock list --status accepted --json
```

Each summary returns `{path, id, title, status, superseded, timestamp_ms}`. Drop entries where `superseded: true`. Sort the remaining records by `timestamp_ms` ascending — when classifying a candidate↔accepted conflict in Step 3, the newer record is the likely revising one.

### Step 2 — Cluster cheaply, before any body read

First, see what tag axes are in play:

```bash
dadrock tags list --json
```

For each `kind/<surface>` value with more than one live record, fetch the cluster:

```bash
dadrock list --tag kind/sample --json
```

Within a kind cluster, sub-cluster by topic or plan tags when they exist:

```bash
dadrock list --tag topic/missingness --json
dadrock list --tag plan/sap-2026 --json
```

For projects with many `writer/agent` candidates, run a keyword pass to catch noun-phrase neighbors that share a topic but not a tag. Pick keywords from the kind cluster's titles:

```bash
dadrock list --tag kind/sample --query missing --json
dadrock list --tag kind/sample --query treatment --json
```

Only after a cluster is narrowed to ~2–N records should you read bodies. Read
the `Decision`, `Context` source links, and any plan-related sections for those
records — and only those.

For projects with many `writer/agent` candidates, also cluster by overlapping `Context` source-link path prefixes once bodies are read — duplicate drafts often share `analysis/build_sample.R` even when their tags differ. This step is body-read-dependent and should follow the cheap pre-clustering above.

### Step 3 — Classify each conflict

| Pattern | What happened | Resolution direction |
|---|---|---|
| candidate ↔ candidate | Two drafts for the same surface, neither accepted | Keep one and reject the other; OR merge into a new candidate. No supersession (both are still candidates). |
| candidate ↔ accepted | A newer candidate revises a prior accepted record | Edit candidate to be the current state, `dadrock status accept <new>`, then `dadrock supersede --from <old> --to <new>`. |
| accepted ↔ accepted | Two accepted records contradict each other | Should not happen, but does. Draft a third candidate as the consolidated current state; on acceptance, supersede both. |
| candidate ↔ rejected | Re-draft of a rejected idea | Surface to user; may be intentional (new evidence) or an oversight. Never auto-act. |

### Step 4 — Surface, do NOT dump

A flat list of paths with one-line rationales is not useful. For each conflict, emit a fixed-format block with everything the user needs to decide in ~30 seconds:

```
Conflict 1 — Treatment missingness handling
  kind/sample · topic/missingness

  A: 01938b8e  candidate   drafted 2026-04-12  writer/agent
     "Drop observations with missing treatment assignment"
     Decision: Drop the 47 respondents with missing `treatment`.
     Context:  analysis/build_sample.R:12-18

  B: 01938fc1  accepted    drafted 2026-03-02  writer/analyst
     "Impute missing treatment with logistic model"
     Decision: Impute treatment via logistic regression on baseline covariates.
     Context:  analysis/build_sample.R:20-44

  Pattern: candidate ↔ accepted (newer candidate appears to revise earlier accepted)

  Point of conflict:
    A drops rows with missing treatment; B keeps them and imputes. They cannot
    both be true on the same sample.

  Suggested resolution:
    1. Confirm A is the desired current state.
    2. Verify A's body is complete (4 sections, source link).
    3. dadrock status accept 01938b8e
    4. dadrock supersede --from 01938fc1 --to 01938b8e

  Reason this might NOT be a conflict:
    If B describes the imputation step *after* A's row-drop, they are sequential.
    Re-read both Decision sections to confirm before resolving.
```

Block-per-conflict, fixed fields, no creative prose. The structure lets the user skim 20 conflicts in a few minutes. The "Reason this might NOT be a conflict" line is mandatory — it forces a sanity check and gives the user a quick decline path.

### Step 5 — Per-conflict prompt

For each surfaced conflict, ask:

1. **Confirm or decline.** Is this actually a conflict?
2. **If confirmed, which record is current?** A, B, or "draft a new candidate that supersedes both".
3. **What happens to the loser?** Reject it, or leave it as a candidate for the analyst to revise.

Collect all answers before executing anything. Batch by default — running prompts one-by-one through 20 conflicts is the "annoying default behavior" we are trying to avoid.

### Step 6 — Execute confirmed resolutions

Run only what the user approved:

```bash
# Accept the surviving candidate
dadrock status accept <id>

# Supersede a prior accepted record
dadrock supersede --from <old-id> --to <new-id>

# Reject a losing candidate
dadrock status reject <id>
```

For "draft a new candidate that supersedes both", scaffold via `dadrock new` (see [code-to-record.md](code-to-record.md) Step 3), fill the four sections grounded in both originals, then let the user accept it before running `supersede` against each predecessor.

### Step 7 — Validate and report

```bash
dadrock validate
```

Confirm the successor graph is acyclic and wikilinks resolve. Report:

- Conflicts found (count + titles).
- Conflicts resolved (with the supersede/reject chain).
- Conflicts deferred (with the user's stated reason).
- New consolidator candidates drafted (still awaiting acceptance).

## Constraints

- **Never auto-resolve.** Acceptance, supersession, and rejection all require human confirmation.
- **Never edit accepted records' bodies** to make them agree. The supersession graph is the only legitimate way to revise an accepted decision.
- **Never modify `successors` by hand.** Always go through `dadrock supersede --from <id> --to <id>`.
- **Don't manufacture conflicts.** If you cannot point at the specific incompatible sentence in each `Decision` section, it is not a conflict.
- **Don't conflate sequence with contradiction.** Two decisions about the same file can both hold if they describe different pipeline steps. Read the `Decision` text, not just the `Context` paths.
- **Stop on cycles.** If A's `successors` lists B and B's `successors` lists A, surface as a project-management error and do not proceed.
- **Don't include `rejected` records in conflict analysis.** They are closed; a re-draft of a rejected idea is a separate signal (Step 3, candidate ↔ rejected pattern).

## Quick reference

| Goal | Command |
|---|---|
| Live candidates (skip rejected) | `dadrock list --status candidate --json` |
| Live accepted (filter `superseded: true` after) | `dadrock list --status accepted --json` |
| Filter by surface | `dadrock list --tag kind/sample --json` |
| Sub-cluster by topic | `dadrock list --tag topic/missingness --json` |
| Keyword pre-cluster (filename + title only, not body) | `dadrock list --query winsorize --json` |
| Combine tag + keyword | `dadrock list --tag kind/sample --query missing --json` |
| All tag axes in use | `dadrock tags list --json` |
| Inspect one record | `dadrock status <query>` |
| Accept a candidate | `dadrock status accept <id>` |
| Supersede an accepted record | `dadrock supersede --from <old> --to <new>` |
| Reject a candidate | `dadrock status reject <id>` |
| Validate the graph | `dadrock validate` |

## Example end-to-end

User: "find any conflicting decisions in dadr/"

1. `dadrock list --status candidate --json` + `dadrock list --status accepted --json` → 23 live records (15 `writer/agent` candidates, 8 accepted; 2 of the accepted have `superseded: true`, drop them). 21 in scope. **No body reads yet.**
2. `dadrock tags list --json` → `kind/{sample,inference,model,communication}`, `topic/{missingness,treatment,table-1}`. Plan the cluster sweep.
3. Per-kind queries: `dadrock list --tag kind/sample --json` (5 records), `--tag kind/inference --json` (3), `--tag kind/communication --json` (2), `--tag kind/modeling --json` (1, skip). **Still no body reads.**
4. NOW read bodies for the 5+3+2 = 10 records in multi-record clusters. Skip the 11 records in singleton or rejected clusters.
5. Cluster `kind/sample, topic/missingness` (5 records): two pairs share `Context` files. One pair is sequential (drop then impute), the other contradicts on the imputation rule. **1 conflict.**
6. Cluster `kind/inference, topic/treatment` (3 records): three different SE choices. **2 pairwise conflicts.**
7. Cluster `kind/communication, topic/table-1` (2 records): both decide table 1's rounding. **1 conflict.**
8. Emit 4 structured blocks. User confirms 3, declines 1 (the SE pair turns out to be main + robustness — should re-tag, not supersede).
9. Execute: 1 `accept`, 1 `supersede`, 2 `reject`, 1 new consolidator candidate drafted via `dadrock new`.
10. `dadrock validate` → clean.
11. Report: 4 conflicts found, 3 resolved, 1 declined with re-tag note for the analyst, 1 consolidator awaiting acceptance.
