# Making decisions

A Data Analysis Decision Record answers one question: **what did we decide, and why?** Keep one Markdown file per consequential choice, alongside your analysis.

## Four sections

- **Context:** the question and relevant evidence.
- **Decision:** what you chose, stated directly.
- **Alternatives:** other approaches considered and why you set them aside.
- **Consequences:** implications, tradeoffs, and follow-up work.

A file also has three required YAML fields: `title`, `status`, and `id`. DADRock generates the ID for you. Tags and links are optional; you do not need a classification system to get started.

## Example

```markdown
---
title: Drop observations with missing treatment assignment
status: accepted
id: 01938c20-a3b4-7000-8000-111122223333
tags:
  - kind/sample
  - confidence/high
---

## Context

The randomization log is missing the treatment field for 47 of 8,213
respondents. Without an assignment we cannot estimate either
intent-to-treat or per-protocol effects for those rows. This extends
the sampling rule established in [[01938b8e4d70]].

## Decision

> #axis/sample/complete-case
> Drop the 47 respondents without recorded treatment assignment from
> the analysis sample.

## Alternatives

- Multiple imputation of the assignment indicator (rejected: assignment
  is a treatment, not a covariate; imputing it would invent
  counterfactual exposures).
- Coding the missing rows as control (rejected: silently biases the ITT
  estimate toward the null).

## Consequences

The analysis sample drops from 8,213 to 8,166. Power calculations need
to be re-run; see [[01938c30b1e2]].
```

## Draft, review, preserve

A new record is a **candidate**. A human reviews it and marks it **accepted** or **rejected**. Acceptance keeps the same ID. An agent may help draft a candidate, but cannot decide whether you endorse it.

Once approved, a decision's substantive content is preserved. Minor corrections remain possible; changing the decision means accepting a successor that supersedes it. See [Changing decisions](changing.md).

## What deserves a record?

Start with choices a collaborator would ask you to explain: who enters the sample, how a variable is defined, why a model was chosen, or how missing values are handled. Skip routine formatting and mechanical refactoring. A few useful explanations are better than an unmanageable review queue.

Code can reveal what an analysis does without revealing why. When a draft says the rationale is unknown, supply it or leave the record as a candidate until you can review it.

[Install DADRock](dadrock/index.md) and open the [web interface](dadrock/web.md) or the [standalone app](dadrock/app.md), or consult the [full specification](specification.md) for exact syntax and implementation rules.
