---
name: dadr-supersede-record
description: Propose a candidate that supersedes an existing accepted DADR. Use when an accepted decision needs to change because of new evidence, plan amendment, or a downstream conflict — without editing the original record in place.
---

# Supersede record

Draft a successor: a new candidate that, once accepted, will supersede an existing accepted record, its predecessor. The predecessor's body stays untouched; when the successor is accepted, Dadrock links the two via the `successors` graph and moves the predecessor to `dadr/archive/`. This skill produces the successor candidate and the supersession plan, not the lifecycle transition itself.

Use this when the user wants to change an accepted decision. If both records are still candidates, use [merge-records.md](merge-records.md) instead. If you are detecting conflicts across the ledger, start at [find-conflicts.md](find-conflicts.md) and use this skill on each confirmed pair.

Trigger phrases:

- "supersede decision 0193…"
- "change the accepted choice"
- "draft a successor for this DADR"
- "the SAP just changed — update the corresponding decision"

## Preconditions

Shared preconditions in [../SKILL.md](../SKILL.md) apply. In addition:

- The predecessor must be `accepted` (`dadrock status <query>` to confirm). You cannot supersede a candidate, rejected, or already-superseded record. (An already-superseded record is closed; instead, draft a successor to its current successor.)
- The user must articulate *why* supersession is warranted. New evidence, plan amendment, conflict with another decision, scope clarification, etc. "I changed my mind" is acceptable but should be captured concretely in the new Context.

## Workflow

### Step 1 — Read the predecessor

```bash
dadrock status <query>
```

Capture: title, four sections (Context, Decision, Alternatives, Consequences), tags, source links. Note which alternatives the predecessor named — they often become the predecessor itself as the new candidate's `Alternatives` entry.

### Step 2 — Frame the supersession

In one or two sentences, state what is changing and why. This becomes the `rationale` field in your proposal and informs the new Context section. Be concrete: "New SAP amendment §4.3 requires multiple imputation rather than complete-case", not "we want to do it differently".

### Step 3 — Draft the new candidate

The successor is a full candidate with all four canonical sections. See [../record.md](../record.md) for the anatomy. Adjustments specific to supersession:

- **Context** — name the predecessor (id or filename), the supersession trigger (new SAP version, conflict, new evidence), and the current source surface (file:line). Preserve the original Context anchor if the new decision still applies to the same code.
- **Decision** — active voice describing the new choice. Make the *difference* from the predecessor explicit when it is non-obvious ("Use multiple imputation via `mice` instead of complete-case dropping…").
- **Alternatives** — usually include the predecessor itself as a rejected alternative, with a one-line reason that mirrors the supersession trigger. Add other defensible options not chosen. If genuinely no alternative applies, write "Not applicable".
- **Consequences** — note downstream effects, including any robustness obligation the supersession introduces, and any prior result that may need to be re-run.

Name the predecessor in a `## Supersedes` section, placed first in the body: a link to the predecessor by its full UUID, labeled with its title, then the reason for the change (the rationale from Step 2):

```markdown
## Supersedes

[[<predecessor-uuid>|<predecessor title>]]

<Why the decision changed, in one or two sentences.>
```

The section starts with the link line; any text after it is the reason, in ordinary Markdown. A record has at most one `## Supersedes` section, and acceptance supports exactly one predecessor for now. Dadrock validates the predecessor and links it when the candidate is accepted, including through the CLI. This is an optional Dadrock convention, not a required core-format section. Context and Alternatives may also use labeled links to discuss the predecessor.

Total body 200–400 words.

### Step 4 — What carries over

- **Valid rationale.** Reasons in the old Context, Alternatives, or Consequences that still hold under the new decision should appear in the new record — usually reshaped into the new framing rather than copied verbatim.
- **Source links** that still resolve. Update line ranges if they have shifted.
- **Tags** that still apply (`kind/`, `topic/`, `plan/`). Drop tags that no
  longer fit.

### Step 5 — Scaffold via `dadrock new`

```bash
dadrock new "Impute missing treatment via multiple imputation (mice)" \
  --tag kind/sample \
  --tag writer/agent \
  --quiet
```

Capture the printed path. Edit the four sections into the scaffold.

### Step 6 — Validate and report

```bash
dadrock validate
```

Then ask the human to review the candidate. With the `## Supersedes` section present, acceptance also links the predecessor and moves it to `dadr/archive/`:

```bash
dadrock accept <new-id>
```

Include the predecessor id, the new candidate's path and id, the one-sentence rationale, and the suggested CLI steps. **Do not run `accept` or `supersede` yourself.**

Do not run a second `supersede` command after acceptance has already linked the predecessor. If two records were accepted separately without a `## Supersedes` section, explicit `dadrock supersede --from <old-id> --to <new-id>` remains available.

## Constraints

- **Never edit the predecessor's body.** Supersession leaves it intact. Dadrock writes the relation when the successor is accepted or through an explicit supersede action.
- **Never modify `successors` by hand, or move records between folders.** Let Dadrock do both when the successor is accepted or through an explicit supersede action.
- **One predecessor per proposal.** If consolidation requires several predecessors, state that scope explicitly and use the supported consolidation workflow rather than silently extending a one-predecessor proposal.
- **No lost evidence.** Do not silently discard sources or alternatives from the old record. Either carry them over or explain in `rationale` why they no longer apply.
- **Acceptance is a human boundary.** This skill writes only to `dadr/inbox/`. Lifecycle transitions are user actions.

## Example

User: "supersede 01938fc1 — the SAP was amended last week to require multiple imputation instead of complete-case."

1. `dadrock status 01938fc1` — predecessor "Drop observations with missing treatment". Decision: drop the 47 respondents. Alternatives: included "Impute treatment assignment via logistic model — rejected because…". Tags: `kind/sample`, `plan/sap-2026`.
2. Frame: SAP amendment §4.3 (dated 2026-05-05) changes the missing-treatment rule.
3. Draft new candidate "Impute missing treatment via multiple imputation (mice)". Context cites 01938fc1, the SAP §4.3 anchor, and `analysis/build_sample.R:12-44`. Decision describes the imputation. Alternatives include 01938fc1 (complete-case dropping, now rejected) and "logistic-model imputation" (still rejected as before). Consequences quantify the sample-size restoration and note new diagnostic obligations.
4. `dadrock new "Impute missing treatment via multiple imputation (mice)" --tag kind/sample --tag writer/agent --tag plan/sap-2026 --quiet` → writes `dadr/inbox/<new-uuid>-impute-missing-treatment-....md`.
5. Add `## Supersedes` as the first section, linking the predecessor by full UUID with its title as the label and the SAP amendment as the reason; then edit the four sections.
6. `dadrock validate` → clean.
7. Report: path of new candidate, predecessor id, rationale, suggested action `dadrock accept <new>`, which also links the predecessor and moves it to the archive.
