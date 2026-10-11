---
title: DADR
hide:
  - navigation
  - toc
  - title
  - footer
---

<div class="hero" markdown>
<img src="assets/dadr_logo.svg" alt="DADR — Data Analysis Decision Records" class="hero-logo">
<br>
<p class="hero-tagline">Keep a record of the <b>important choices behind your analysis</b>, so you and your collaborators can remember why you made them.</p>
</div>

## Why did we exclude these observations?

Months later, the code may still tell you which rows were dropped. A decision record tells you why. Each important choice gets one short Markdown file:

```markdown
---
title: Exclude observations without treatment assignment
status: accepted
id: 01938c20-a3b4-7000-8000-111122223333
---

## Context

The assignment log is missing treatment assignment for 47 of 8,213
respondents. We could not recover it from the original records.

## Decision

Exclude those 47 respondents from the treatment-effect analysis.

## Alternatives

Recover assignment from the original records: attempted without success.
Treat missing assignments as control: rejected because the assignment
is unknown, not evidence of assignment to control.

## Consequences

The sample falls to 8,166. Report the exclusions and investigate whether
missing assignment is associated with observed respondent characteristics.
```

This is an illustrative record, not a recommendation for every missing-data problem.

## Write, review, remember

**DADR** is the file format: four sections, a title, a status, and an ID. **Dadrock** is the tool that helps you write, review, and find these records.

Draft a candidate yourself or ask an agent for help. Review what it says and why, then accept or reject it. Once approved, the decision's substantive content is preserved. Changing your mind creates a linked replacement; minor corrections remain possible. The old record remains accepted, with successors identifying its replacements.

Start with a few consequential choices. Tags, agents, and integrations are optional. You can keep the files alongside your analysis and read them in any text editor.

<figure class="screenshot" markdown>
![Reviewing a candidate decision in the dadrock web UI, with Accept and Reject buttons](assets/dadrock-review-light.webp){ loading=lazy }
<figcaption>Reviewing a candidate in the <a href="dadrock/web/">dadrock web UI</a>.</figcaption>
</figure>

## What Dadrock does today

Dadrock creates and organizes records, provides a local web review interface, links replacement decisions, and checks record structure and references. With a configured local agent, it can propose drafts from your context or code and help review conflicting records.

An agent can identify a sample restriction without knowing your reason for choosing it. Supply that reason during review; accepting a plausible description is not a substitute for endorsing the decision.

Automated comparisons of code, plans, and manuscripts against decisions are [roadmap ideas](roadmap.md), not guarantees provided by current validation.

<div class="hero-buttons" style="margin-top: 2.5rem">
  <a class="md-button" href="getting-started/">Create your first decision</a>
  <a class="md-button" href="core/">The format in one page</a>
</div>
