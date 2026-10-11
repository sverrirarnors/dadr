---
id: context-to-record
title: "Context to record"
description: "Turn supplied context into one candidate DADR when you already know the decision to capture."
version: 1
inputs:
  - name: context
    label: "Context"
    type: textarea
    required: true
requirements:
  records: none
  record_status: all
  paths: optional
  notes: required
  scope: none
output:
  kind: candidate_draft.v1
mcp_access: read_only
tools:
  - name: dadrock.project_snapshot
    mode: read
---

# Context -> record

Capture a single consequential analytic choice as a DADR candidate, drawing on the context the user has supplied directly (a plan paragraph, code excerpt, manuscript passage, meeting summary, or freeform description). Unlike [code-to-record.md](code-to-record.md), this skill does not crawl the project — the user has already pointed at what matters.

Trigger phrases:

- "draft a DADR for this"
- "capture this decision"
- "make a candidate from this plan paragraph"
- "log this choice as a candidate"

## Preconditions

Shared preconditions in [../SKILL.md](../SKILL.md) apply. In addition:

- `dadrock dir` must succeed. If the project is not initialized, run `dadrock init` after confirming with the user.
- You need enough context to fill all four canonical sections. If the supplied context covers only `Decision` (the what), ask for at least one anchor for `Context` (where in the code, plan, or paper this lives) before drafting.

## What to capture

A DADR records one **consequential analytic choice**. Before drafting, sanity-check that the user's request actually describes a decision:

- **Could it have gone another way?** A tooling default or formatting choice is not a decision.
- **Does it affect a reported result?** If nothing downstream depends on it, decline politely and explain.
- **Is it independently reviewable?** A reviewer should be able to accept or reject this one choice without forcing decisions on adjacent choices.

If the request bundles two choices (e.g., "drop missing treatment AND impute baseline covariates"), draft them as two separate candidates and tell the user why you split.

## Workflow

### Step 1 — Confirm the choice and anchor

Restate the choice in one sentence and identify the anchor — the file:line, plan section, or paper passage that grounds it. If the anchor is ambiguous, ask before drafting; a candidate without a source link is incomplete.

### Step 2 — Draft the four sections

See [../record.md](../record.md) for the full anatomy. Briefly:

- **Context** (2–4 sentences) — what motivates the choice. Always include a markdown link to the source surface.
- **Decision** (1–3 sentences) — what is done, active voice ("Drop the 47 respondents…"), present tense.
- **Alternatives** (1–3 items) — defensible options not chosen, with a one-line reason each. Write "Not applicable" if no real alternative exists. **Do not fabricate.**
- **Consequences** (2–4 sentences) — downstream effects, quantified when possible.

The total body should land in the 200–400 word window. Below ~100 means a section is too thin; above ~600 usually means two decisions bundled.

### Step 3 — Scaffold via `dadrock new`

Never hand-write the record file — `dadrock new` allocates the UUIDv7 and canonical filename:

```bash
dadrock new "Drop observations with missing treatment assignment" \
  --tag kind/sample \
  --tag writer/agent \
  --quiet
```

- Title is a noun phrase, not a question.
- Pass tags via repeated `--tag` (no `#` prefix on the CLI).
- Capture the printed path; you will edit the file's four sections next.

### Step 4 — Fill the sections via Edit

Replace the empty `## Context`, `## Decision`, `## Alternatives`, `## Consequences` with substantive prose grounded in the supplied context. Preserve the front-matter as `dadrock new` generated it.

### Step 5 — Tag and validate

Every candidate carries `kind/<surface>` and `writer/agent` at minimum. See
[../tags.md](../tags.md) for optional namespaces (`confidence/`, `topic/`,
`plan/`).

```bash
dadrock validate
```

Fix any reported issues. Then surface the new path to the user and remind them the next step is review (`dadrock status accept <id>` or `dadrock status reject <id>`).

## Constraints

- **One choice per candidate.** If the supplied context describes two decisions, draft two candidates and explain the split.
- **No fabricated alternatives.** "Not applicable" is a valid answer.
- **No source links you cannot verify.** Every Context link should resolve to a real file, plan section, or paper anchor.
- **Acceptance is a human boundary.** Never auto-accept; report the path and the review command.

## Example

User: "log this — we decided to winsorize income at the 1st and 99th percentiles before fitting the wage model. The reasoning is in `docs/sap.md` §4.1, implemented in `analysis/clean.R:88-104`."

1. Confirm: one decision, anchored at `analysis/clean.R:88-104` and `docs/sap.md§4.1`.
2. Draft Context, Decision, Alternatives (e.g., "trim instead of winsorize", "no transformation"), Consequences.
3. `dadrock new "Winsorize income at 1st and 99th percentiles" --tag kind/transformation --tag writer/agent --quiet`.
4. Edit the four sections of the generated file.
5. `dadrock validate` → clean.
6. Report path + `dadrock status accept <id>`.
