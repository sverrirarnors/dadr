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
<p class="hero-tagline"><b>Record human decisions</b> in a simple file format. <br><b>Implement the data analysis</b> using any tools you like. <br><b>Assess conformity</b> of the code with decisions automatically.</p>

<div class="hero-diagram">
<svg class="wf" viewBox="0 0 1080 620" role="img" aria-labelledby="wf-title-id">
<title id="wf-title-id">DADR workflow: draft, review, accept or reject, then audit, report, or supersede</title>
<defs>
<marker id="wf-arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse">
<path d="M0,0 L10,5 L0,10 z" class="wf-arrowhead"/>
</marker>
</defs>
<path d="M530.0,160 V210" class="wf-edge" marker-end="url(#wf-arrow)"/>
<path d="M530.0,310 V360" class="wf-edge" marker-end="url(#wf-arrow)"/>
<path d="M640.0,310 C640.0,340 860.0,330 860.0,360" class="wf-edge" marker-end="url(#wf-arrow)"/>
<path d="M530.0,410 V460" class="wf-edge" marker-end="url(#wf-arrow)"/>
<path d="M500.0,410 C500.0,450 180.0,420 180.0,460" class="wf-edge" marker-end="url(#wf-arrow)"/>
<path d="M560.0,410 C560.0,450 880.0,420 880.0,460" class="wf-edge" marker-end="url(#wf-arrow)"/>
<path d="M1040,530.0 H1065 V90.0 H690.0" class="wf-edge wf-loop" marker-end="url(#wf-arrow)"/>
<rect x="370.0" y="20" width="320" height="140" rx="2" class="wf-box"/>
<text x="530.0" y="50" text-anchor="middle" class="wf-title">Draft</text>
<text x="386.0" y="74" class="wf-text"><tspan x="386.0" dy="0">• Write a data analysis decision</tspan><tspan x="398.0" dy="22">from scratch</tspan><tspan x="386.0" dy="22">• Ask an LLM to read an existing</tspan><tspan x="398.0" dy="22">code base and identify key decisions</tspan></text>
<rect x="370.0" y="210" width="320" height="100" rx="2" class="wf-box"/>
<text x="530.0" y="240" text-anchor="middle" class="wf-title">Review</text>
<text x="386.0" y="264" class="wf-text"><tspan x="386.0" dy="0">Only a human analyst is allowed to</tspan><tspan x="386.0" dy="22">approve or reject decision records</tspan></text>
<rect x="470.0" y="360" width="120" height="50" rx="2" class="wf-box wf-accept"/>
<text x="530.0" y="390.0" text-anchor="middle" class="wf-title">Accept</text>
<rect x="800.0" y="360" width="120" height="50" rx="2" class="wf-box wf-reject"/>
<text x="860.0" y="390.0" text-anchor="middle" class="wf-title">Reject</text>
<rect x="20" y="460" width="320" height="140" rx="2" class="wf-box"/>
<text x="180.0" y="490" text-anchor="middle" class="wf-title">Audit</text>
<text x="36" y="514" class="wf-text"><tspan x="36" dy="0">• Substantive review: humans must agree</tspan><tspan x="48" dy="22">with all decisions</tspan><tspan x="36" dy="22">• Code review: AI ensures that the code</tspan><tspan x="48" dy="22">faithfully implements decisions</tspan></text>
<rect x="370" y="460" width="320" height="140" rx="2" class="wf-box"/>
<text x="530.0" y="490" text-anchor="middle" class="wf-title">Report</text>
<text x="386" y="514" class="wf-text"><tspan x="386" dy="0">• Methods section</tspan><tspan x="386" dy="22">• Decision tables</tspan><tspan x="386" dy="22">• Onboarding documents</tspan></text>
<rect x="720" y="460" width="320" height="140" rx="2" class="wf-box"/>
<text x="880.0" y="490" text-anchor="middle" class="wf-title">Supersede</text>
<text x="736" y="514" class="wf-text"><tspan x="736" dy="0">• Revert a past decision</tspan><tspan x="736" dy="22">• Tweak a previous choice</tspan></text>
</svg>
</div>
</div>

<div class="problem" markdown>

## Problem 1: Opacity

<div class="problem-body" markdown>

<div class="problem-prose" markdown>
Every data analysis rests on dozens of choices: which observations to include, how to code key variables, which model to fit, how to quantify uncertainty. These choices matter: *Many Analysts* studies show that independent teams given the same data and hypotheses routinely reach different results.[^many-analysts]

[^many-analysts]:
    *Many Analysts* studies:

    - Silberzahn et al. (2018), "Many Analysts, One Data Set: Making Transparent How Variations in Analytic Choices Affect Results," *Advances in Methods and Practices in Psychological Science* 1(3): 337–356, [doi:10.1177/2515245917747646](https://doi.org/10.1177/2515245917747646).
    - Botvinik-Nezer et al. (2020), "Variability in the Analysis of a Single Neuroimaging Dataset by Many Teams," *Nature* 582: 84–88, [doi:10.1038/s41586-020-2314-9](https://doi.org/10.1038/s41586-020-2314-9).
    - Huntington-Klein et al. (2021), "The Influence of Hidden Researcher Decisions in Applied Microeconomics," *Economic Inquiry* 59(3): 944–960, [doi:10.1111/ecin.12992](https://doi.org/10.1111/ecin.12992).
    - Breznau et al. (2022), "Observing Many Researchers Using the Same Data and Hypothesis Reveals a Hidden Universe of Uncertainty," *PNAS* 119(44): e2203150119, [doi:10.1073/pnas.2203150119](https://doi.org/10.1073/pnas.2203150119).

Most of these choices never appear in a methods section, and reconstructing them from code and data after the fact is an archeology project.
</div>

<div class="pull pull--aside" markdown>
A simple way to record what was decided, why, what alternatives were considered, and what the consequences are.
</div>

</div>

</div>

<div class="problem" markdown>

## Problem 2: Drift

<div class="problem-body" markdown>

<div class="problem-prose" markdown>
On long projects, analysts lose track of key decisions and the reasons behind them. The code drifts from the original intent, and minor-looking choices can change how results are interpreted. The risk grows when analysis is delegated to coding agents, which produce a lot of hard-to-review code.
</div>

<div class="pull pull--aside" markdown>
A human-approved, auditable, and authoritative record of the key decisions made in a project.
</div>

</div>

</div>


## Solution: Data Analysis Decision Records

Data Analysis Decision Records (DADR) are designed to make our key analytical choices explicit. Each decision becomes a short Markdown record, stored alongside the code, data, and prose of a project. The DADR markdown files are human-editable, LLM-readable, and version-controllable. They include meta-data fields and tags that enable external tools to track decision flows, audit the faithfulness of a codebase to recorded decisions, and generate reports and narratives directly from the decision ledger.

DADR is not meant to be a comprehensive record of every thought, but rather a focused ledger of the key decisions that shape an analysis. It is a simple specification inspired by the [Architecture Decision Record](https://learn.microsoft.com/en-us/azure/well-architected/architect-role/architecture-decision-record), a proven and popular development practice in the software engineering community.

<div class="principle" markdown>
Every candidate decision <b>must</b> be approved by the human analyst before it enters the permanent record.
</div>

<figure class="screenshot" markdown>
![The DADRock web UI Inbox: a list of five candidate decisions, with one open showing its Context, Decision, Alternatives, and Consequences and Accept and Reject buttons](assets/dadrock-review-light.webp){ loading=lazy }
<figcaption>Reviewing a candidate in the <a href="dadrock/web/">DADRock web UI</a> Inbox.</figcaption>
</figure>

## New possibilities

A complete, structured trace of analytic decisions is more than documentation. It is an asset that downstream tools and humans can act on.

<div class="feature-grid" markdown>

<div class="feature" markdown>
#### Faithfulness
Compare code, plan, and manuscript against recorded decisions to flag drift, undocumented choices, or contradictions. An LLM walks the relevant code paths and produces a discrepancy report for human triage.
</div>

<div class="feature" markdown>
#### Compliance
Cross-check decisions against a Statistical Analysis Plan (SAP), a Pre-Analysis Plan (PAP), Pre-registration, or data cleaning plan to surface deviations and unexecuted commitments.
</div>

<div class="feature" markdown>
#### Multiverse and robustness
Automatically surface the choices that need to be subjected to sensitivity, robustness, or multiverse analysis.
</div>

<div class="feature" markdown>
#### Decision flow
Follow how analytic choices evolved: which decisions replaced which, when, and why.
</div>

<div class="feature" markdown>
#### Drafting
Generate methods sections, appendices, and decision tables directly from the trace, with citations back to the records.
</div>

<div class="feature" markdown>
#### Onboarding
Hand a collaborator or new contributor a readable narrative instead of a code-archaeology assignment.
</div>

<div class="feature" markdown>
#### Replication and reuse
Give independent analysts a self-contained rationale for every consequential choice.
</div>

<div class="feature" markdown>
#### Coverage and audit
Report which analysis activities are backed by decisions, which are unreviewed, and which have gone stale.
</div>

<div class="feature" markdown>
#### Teaching
Use real annotated analyses to show how defensible alternatives were considered and chosen.
</div>

</div>

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

<div class="hero-buttons" style="margin-top: 2.5rem">
  <a class="md-button" href="core/">Documentation</a>
  <a class="md-button" href="dadrock/">Software</a>
</div>
