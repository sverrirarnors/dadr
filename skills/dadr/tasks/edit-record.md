---
name: dadr-edit-record
description: Revise an existing DADR candidate to apply reviewer-requested changes while preserving its intent and valid evidence. Use when a reviewer has feedback on a candidate before accepting, or when the candidate's body needs to be sharpened in response to new information.
---

# Edit record

Apply requested changes to an existing candidate DADR. The candidate stays a candidate; only its body, title, or tags change. If the request would change the underlying choice to a different one, this is the wrong skill — use [supersede-record.md](supersede-record.md) (if the predecessor is already accepted) or draft a fresh candidate via [context-to-record.md](context-to-record.md).

Trigger phrases:

- "revise this candidate"
- "rework the alternatives section"
- "tighten the consequences"
- "the reviewer wants a different framing"

## Preconditions

Shared preconditions in [../SKILL.md](../SKILL.md) apply. In addition:

- The target must be a candidate (`dadrock list --status candidate` to confirm). This skill does not edit accepted, rejected, or superseded records.
- The user's revision request must be specific. "Make it better" is not actionable; ask for the section(s) and the change desired before editing.

## What to preserve

- **The choice itself.** A revision sharpens or corrects how the choice is described; it does not switch to a different choice. If the requested changes would invalidate the decision, push back and propose drafting a new candidate or a supersession instead.
- **Valid evidence.** Source links (file:line, plan section, document anchor) in the original Context that still resolve must remain. Update line ranges if they have shifted; remove links that no longer exist and note the removal explicitly.
- **Alternatives grounded in the original.** Do not silently drop an alternative the original named. If you remove one, explain why.
- **Tag axes already in use.** Add new tags when applicable, but do not strip
  `kind/` or `writer/` unless the changes explicitly require it.

## Workflow

### Step 1 — Locate and read

```bash
dadrock status <query>
```

Read the candidate's body. Identify which of the four sections (`Context`, `Decision`, `Alternatives`, `Consequences`) the request touches. Note any source links and the current tag set.

### Step 2 — Apply changes section-by-section

See [../record.md](../record.md) for the full anatomy and length envelopes. Briefly:

- **Context** (2–4 sentences) — always include at least one source link.
- **Decision** (1–3 sentences) — active voice, present tense ("Drop the 47…", not "we will drop").
- **Alternatives** (1–3 items) — real options only; "Not applicable" beats fabrication.
- **Consequences** (2–4 sentences) — quantify when possible.

Total body remains in the 200–400 word window. If a revision pushes past ~600 words, flag that the candidate may now bundle two decisions and propose a split.

Use `Edit` for surgical changes. Preserve the front-matter exactly unless the request changes title or tags. **Never edit the `id` or `status` fields.**

### Step 3 — Update tags if needed

If the revision adds a new `kind/` or `topic/` axis, update the front-matter
`tags:` list. See [../tags.md](../tags.md) for the vocabulary. Do not invent
namespaces beyond those listed unless the project already uses them —
`dadrock tags list` shows what is in play.

### Step 4 — Validate and report

```bash
dadrock validate
```

Then summarize:

- Section-by-section what changed.
- Any source links updated or removed (with reason).
- Any alternatives added or dropped (with reason).
- Whether the revision pushes past the length envelope.
- Next step for the user: re-review and accept (`dadrock status accept <id>`) or request further changes.

## Common pitfalls

- **Tense drift.** Keep `Decision` active and present-tense. Do not introduce "we will" or "we should".
- **Question-form titles.** Convert "Should we drop missing treatment?" → "Drop observations with missing treatment".
- **Empty Alternatives.** Never delete the Alternatives section. "Not applicable" is the floor.
- **Fabricated source links.** Do not add file paths or line ranges you cannot verify against the project.
- **Scope creep.** If a request would expand the candidate beyond one consequential choice, apply the narrow change and flag the scope concern.
- **Auto-accept.** Acceptance is a human boundary. Report the revised path; do not run `dadrock status accept` without explicit user instruction.

## Constraints

- **This skill does not run lifecycle transitions.** No `accept`, `reject`, or `supersede` from here — those are separate user actions.
- **No edits to accepted, rejected, or superseded records.** Use supersession instead.
- **One candidate per invocation.** If two candidates need revising, run the skill twice.
- **Preserve `id` and `status`.** Both are dadrock-owned front-matter fields.

## Example

User: "revise candidate 01938b8e — the reviewer wants the Alternatives section to explicitly list multiple imputation as a rejected option, and Consequences should quantify the row drop."

1. `dadrock status 01938b8e` — read the current body.
2. Edit Alternatives: add "Multiple imputation via `mice` or `mi` on baseline covariates. Rejected because treatment assignment is part of the design, not a downstream measurement; imputing it would obscure the estimand."
3. Edit Consequences: change "the sample shrinks" → "The main sample drops from 1,247 to 1,200 (3.8% reduction). Verify treatment missingness is not concentrated in one site before reporting."
4. Word count: 312, within envelope.
5. `dadrock validate` → clean.
6. Report the two section changes and the new word count; next step is reviewer re-read and `dadrock status accept 01938b8e`.
