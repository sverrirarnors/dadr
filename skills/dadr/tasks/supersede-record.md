---
name: dadr-supersede-record
description: Propose a candidate that supersedes one or more accepted DADRs. Use when an accepted decision needs to change, however slightly, because of new evidence, a plan amendment, a correction, or a downstream conflict, or when several accepted decisions should be merged into one, without editing the originals.
---

# Supersede record

Draft a successor: a new candidate that, once accepted, supersedes one or more accepted records, its predecessors. Accepted records never change, not even for a typo, so every change to a decision goes through a successor. When the successor is accepted, Dadrock adds its id to each predecessor's `successors` and moves the predecessors to `dadr/archive/`. This skill produces the successor candidate, not the lifecycle transition itself.

Use this when the user wants to change an accepted decision or merge several accepted decisions into one. If the records are still candidates, use [edit-record.md](edit-record.md) or [merge-records.md](merge-records.md) instead. If you are detecting conflicts across the ledger, start at [find-conflicts.md](find-conflicts.md) and use this skill on each confirmed pair.

Trigger phrases:

- "supersede decision 0193…"
- "change the accepted choice"
- "fix a typo in an accepted record"
- "merge these two accepted decisions"
- "draft a successor for this DADR"
- "the SAP just changed — update the corresponding decision"

## Preconditions

Shared preconditions in [../SKILL.md](../SKILL.md) apply. In addition:

- Each predecessor must be accepted and current (`dadrock status <query>` to confirm). You cannot supersede a candidate or a rejected record. A record that is already superseded is closed: draft a successor to its current successor instead.
- The user must say what should change and, ideally, why. Capture the why concretely; it becomes the reason in `## Supersedes`.

## Workflow

### Step 1 — Read the predecessors

```bash
dadrock status <query>
```

Read each predecessor in full: title, tags, and the four sections, with their exact wording.

### Step 2 — State the reason

In one to three sentences, say why the decision changed: new evidence, a plan amendment, a conflict with another decision, a correction. Be concrete: "SAP amendment §4.3 requires multiple imputation rather than complete cases", not "we want to do it differently". This reason goes into `## Supersedes`, not into Context.

### Step 3 — Start from the predecessor's exact text

The reviewer reads the successor as a diff against its predecessor, so every unnecessary edit is noise.

- **One predecessor:** copy its title, tags, and four sections exactly, then change only what the request requires. A typo fix changes one word; a new cutoff changes the sentences that state the cutoff and the consequences that follow from it.
- **Several predecessors (a merge):** combine their sections into one decision. Keep each predecessor's wording where it still holds, remove repetition, and change only what the merged decision requires.
- **Do not narrate the supersession.** Do not rewrite Context to say that a predecessor existed or why it is being replaced. The successor reads as a complete record of the decision now in force; the reason lives in `## Supersedes`.
- Keep source links that still resolve, updating shifted line ranges. Keep the tags that still apply.

See [../record.md](../record.md) for the anatomy of the four sections.

### Step 4 — Write the candidate with `## Supersedes` first

Scaffold the candidate, then write its body:

```bash
dadrock new "Impute missing treatment via multiple imputation (mice)" \
  --tag kind/sample \
  --tag writer/agent \
  --quiet
```

The body starts with a `## Supersedes` section: one line per predecessor, each a link by full UUID labeled with the predecessor's title, then the reason from Step 2. The four sections follow.

```markdown
## Supersedes

[[<predecessor-uuid>|<predecessor title>]]
[[<second-predecessor-uuid>|<second predecessor title>]]

<Why the decision changed, in one to three sentences.>

## Context
...
```

A record has at most one `## Supersedes` section, and a section without a link is an error. The section stays in the body after acceptance as the permanent record of why the decision changed. It is a Dadrock convention, not a required core-format section.

When the `supersede-record` agent task runs instead (`dadrock task run supersede-record` or the web interface), the agent returns `superseded_record_ids`, the title, the four sections, and a `rationale`; Dadrock writes the `## Supersedes` section from them when the proposal is saved as a candidate.

### Step 5 — Validate and report

```bash
dadrock validate
```

Then ask the human to review the candidate. Accepting it links every predecessor and moves them to `dadr/archive/`:

```bash
dadrock accept <new-id>
```

For a **split**, where several successors replace one predecessor, the candidates each name the same predecessor and are accepted together, so the predecessor gains all of them at once:

```bash
dadrock accept <first-new-id> --with <second-new-id>
```

Include the predecessor ids, the new candidate's path and id, the reason, and the suggested accept command. **Do not run `accept` or `supersede` yourself.** Do not run `dadrock supersede` after acceptance: acceptance already links the records.

## Stale successors

If a predecessor is superseded while your candidate waits (for example, a competing candidate was accepted first), your candidate is stale. `dadrock validate` warns with `stale_supersedes` and names the decisions now in force, and acceptance is refused. Point the `## Supersedes` link at the current decision, adjusting the text if needed, or ask the user to reject the stale candidate.

## Constraints

- **Never edit a predecessor.** Accepted and rejected records are frozen; Dadrock refuses changes to their `id`, `status`, `title`, body, and `successors`. Only tags and keys outside DADR can change.
- **Never modify `successors` by hand, or move records between folders.** Dadrock does both when the successor is accepted.
- **No lost evidence.** Do not silently discard sources or alternatives from a predecessor. Keep them, or explain in the reason why they no longer apply.
- **Acceptance is a human boundary.** This skill writes only to `dadr/inbox/`. Lifecycle transitions are user actions.

## Example

User: "supersede 01938fc1 — the SAP was amended last week to require multiple imputation instead of complete cases."

1. `dadrock status 01938fc1` — predecessor "Drop observations with missing treatment", tags `kind/sample`, `plan/sap-2026`.
2. Reason: SAP amendment §4.3 requires multiple imputation for missing treatment.
3. Copy the predecessor's sections. Change the Decision to multiple imputation via `mice`, move complete-case dropping into Alternatives with the reason it no longer applies, and update the Consequences that follow from the sample size. Leave the rest of Context as it was.
4. `dadrock new "Impute missing treatment via multiple imputation (mice)" --tag kind/sample --tag writer/agent --tag plan/sap-2026 --quiet`, then write `## Supersedes` first, linking 01938fc1 by full UUID with its title and the SAP amendment as the reason, followed by the four sections.
5. `dadrock validate` → clean.
6. Report the new candidate's path, the predecessor id, the reason, and the suggested `dadrock accept <new-id>`.
