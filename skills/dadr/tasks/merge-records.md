---
id: merge-records
title: "Merge records"
description: "Combine overlapping candidate DADRs into one clearer candidate while leaving source records unchanged."
version: 1
inputs:
  - name: record_ids
    label: "Candidate records"
    type: textarea
    required: true
  - name: merge_goal
    label: "Merge goal"
    type: textarea
    required: false
requirements:
  records: many
  record_status: candidate
  paths: none
  notes: optional
  scope: none
output:
  kind: candidate_draft.v1
mcp_access: full
tools:
  - name: dadrock.get_record
    mode: read
---

# Merge records

Collapse overlapping candidate records into one consolidated candidate that captures the choice cleanly. The sources stay on disk as candidates until the user explicitly rejects them — this skill produces a new candidate, not a lifecycle transition.

Use this when bootstrapping ([code-to-record.md](code-to-record.md)) or conflict detection ([find-conflicts.md](find-conflicts.md)) has surfaced two or more candidate drafts that describe the same decision from slightly different angles. If the duplicates are an accepted record + a newer candidate, this is the wrong skill — use [supersede-record.md](supersede-record.md) on the accepted predecessor.

Trigger phrases:

- "merge these candidates"
- "consolidate the duplicate drafts"
- "these two are the same decision — combine them"

## Preconditions

Shared preconditions in [../SKILL.md](../SKILL.md) apply. In addition:

- All source records must be `candidate` status. The skill refuses to merge accepted, rejected, or superseded records.
- The user must identify the sources by id or query — pass them in explicitly rather than guessing.
- Before merging, confirm the sources actually describe the same choice. Two
  candidates that share `kind/` can still be distinct decisions on adjacent
  pipeline steps. Read each `Decision` section before consolidating.

## What to keep from each source

- **The strongest `Context` framing.** Often one source describes the motivation (plan section, theoretical reason) and another anchors the concrete site (file:line). Combine both.
- **The clearest `Decision` wording.** Pick the formulation that is most active-voice, present-tense, and quantitatively specific. If sources disagree on a parameter (e.g., one says 1st/99th percentile, the other says 5th/95th), surface the disagreement to the user — do not silently pick.
- **All distinct alternatives.** Deduplicate; merge near-duplicates; preserve any reason that makes a real distinction.
- **The most quantified `Consequences`.** Where sources differ, surface the difference to the user before consolidating.
- **The union of tags** — but normalize spelling (`kind/sample` vs `kind/sampling`). Drop `writer/` tags that conflict; keep `writer/agent` if any source carries it.
- **All distinct source links.** Multiple links across files are fine; the consolidated record may legitimately point at several call sites.

## What NOT to keep

- Redundant phrasings — merged prose should read as one document, not three concatenated.
- Conflicting parameter choices — those are surfaced as warnings, not silently chosen.
- Records' UUIDs in the merged front-matter — the new candidate gets a fresh UUIDv7 from `dadrock new`.

## Workflow

### Step 1 — Read all sources

```bash
dadrock status <id-1>
dadrock status <id-2>
# ...for each source
```

Capture each record's title, four sections, tags, and source links. Make a mental (or written) table of what each source contributes.

### Step 2 — Confirm same decision

Restate the consolidated choice in one sentence. If the sources actually describe different choices, **stop**: the merge is wrong. Tell the user which sources are distinct and ask whether to merge a subset or abandon.

### Step 3 — Draft the consolidated body

See [../record.md](../record.md) for the anatomy. The merged body has the four canonical sections:

- **Context** — combines the motivation and the site, with all distinct source links.
- **Decision** — one active-voice sentence (or up to three) capturing the consolidated choice. If sources disagreed on a parameter, pick the user's preferred value and explain in the consolidation summary; do not invent a compromise.
- **Alternatives** — union of distinct alternatives from sources, deduplicated.
- **Consequences** — combined downstream effects. Where sources offered different sample-size or estimand impacts, pick the most defensible and surface the discrepancy.

Total body 200–400 words. If the merge pushes past ~600 words, the sources may have been distinct decisions after all — surface that as a warning.

### Step 4 — Scaffold via `dadrock new`

```bash
dadrock new "Drop observations with missing treatment assignment" \
  --tag kind/sample \
  --tag writer/agent \
  --quiet
```

Capture the printed path. Edit the four sections into the scaffold.

### Step 5 — Validate and report

```bash
dadrock validate
```

Surface for the user:

- The new consolidated candidate's path and id.
- The source ids that fed into it, each with a one-line summary of what it contributed.
- Any disagreements surfaced (parameter values, sample-size estimates) and how you resolved them.
- The suggested next steps as two user actions:
  1. Review and accept the new candidate (`dadrock status accept <new-id>`).
  2. Reject each source (`dadrock status reject <source-id>`), once the user is satisfied the new candidate captures everything.

**Do not run `accept` or `reject` yourself.**

## Common pitfalls

- **Silent parameter picking.** If sources disagree on a winsorization threshold or imputation rule, the user picks. The skill flags, it does not arbitrate.
- **Fabricated synthesis.** If no source named a particular alternative, do not add one to round out the section.
- **Bundling decisions.** If the merge pushes past ~600 words or covers two distinct choices, the sources weren't duplicates. Stop and surface.
- **Auto-rejecting sources.** Acceptance and rejection are human actions. The merge produces a new candidate; the sources stay until the user rejects them.

## Constraints

- **All sources must be candidates.** No merging across the lifecycle boundary.
- **No edits to source records.** The merge writes a new candidate via `dadrock new`. Source candidates stay on disk until the user rejects them.
- **One consolidated candidate per merge.** Do not split into "main" and "robustness" variants — that is two separate candidates the user can draft afterwards.
- **Acceptance is a human boundary.** Report paths and commands; never run lifecycle transitions.

## Example

User: "merge 01938b8e and 01938b9a — they're both about dropping missing treatment."

1. `dadrock status 01938b8e` and `dadrock status 01938b9a` — both candidates, both tagged `kind/sample`.
2. Confirm: both describe dropping rows with missing `treatment`. One anchors at `analysis/build_sample.R:12-18` with motivation from `docs/sap.md §3.2`. The other anchors at the same lines but quantifies the sample-size impact (47 rows). Same decision.
3. Draft consolidated body: Context cites both SAP §3.2 and `build_sample.R:12-18`. Decision: "Drop the 47 respondents with missing `treatment`." Alternatives unions the two sources' lists. Consequences quantifies "n drops from 1,247 to 1,200 (3.8%)" from the second source and notes the robustness obligation from the first.
4. `dadrock new "Drop observations with missing treatment assignment" --tag kind/sample --tag writer/agent --quiet`.
5. Edit the four sections.
6. `dadrock validate` → clean.
7. Report: new path + id, source ids and contributions, suggested next steps `dadrock status accept <new>` then `dadrock status reject 01938b8e` and `dadrock status reject 01938b9a` once the user confirms the merge captured everything.
