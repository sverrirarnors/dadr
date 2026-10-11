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
tools:
  - name: dadrock.project_snapshot
    mode: read
---

# Context to record

Draft one candidate DADR from context the user supplies: a plan paragraph, a code excerpt, a manuscript passage, a meeting summary, or a plain description of the choice. Unlike [code-to-record](code-to-record.md), this task does not survey the project; the user has already pointed at what matters.

## Inputs

- `context`: the description of the decision to capture.
- Optionally, files or folders the context refers to.

Interactively, these come from the user's request and any files they point at.

## Check that it is a decision

A DADR records one **consequential analytic choice**. Before drafting, check that the context describes one:

- **Could it have gone another way?** A tooling default or a formatting preference is not a decision.
- **Does it affect a reported result?** If nothing downstream depends on it, say so.
- **Is it independently reviewable?** A reviewer should be able to accept or reject this one choice without deciding adjacent ones.

If the context is too thin to support a real decision, say so in the warnings rather than filling gaps with invented facts. If it bundles two independent choices (for example, "drop missing treatment and impute baseline covariates"), draft the principal one and say in the warnings that the other needs its own candidate.

## Drafting

Write the candidate to the [record reference](../record.md): a noun-phrase title and the four sections. In addition:

- **Anchor it.** Context cites a specific source: a file with a line range, a plan section, or a document anchor. Look for it in the supplied context and the files it refers to. Do not invent paths or line numbers; if no anchor can be found, say so in the warnings.
- **Stay within the evidence.** When the context says what was done but not why, write the exact phrase "Rationale not documented" in Context, repeat it in the warnings, and ask the reviewer to supply the reason. Never infer motivation from code alone.
- **Real alternatives only.** Use alternatives the context or its sources name. If none is identifiable, write "Not applicable" rather than inventing one.
- **Tags.** Give the candidate `writer/agent` and one `kind/` tag from the project's vocabulary or the [tag reference](../tags.md). Add `topic/`, `plan/`, or `confidence/` tags when they apply. Suggest `people/decider/` or `people/approver/` tags only when the context names those people.

## Constraints

- Draft exactly one candidate, and do not change existing records.
- Never accept the candidate. Acceptance is the analyst's decision.

## Result

One candidate:

- **Title**: a noun phrase describing the choice, not a question.
- **Tags**, including `writer/agent` and a `kind/` tag.
- **Body** with Context, Decision, Alternatives, and Consequences.
- **Source references**: the files, line ranges, plan sections, or document anchors behind each claim.
- **Warnings**: thin evidence, missing rationale, a missing anchor, ambiguity, or a second choice that needs its own candidate.
