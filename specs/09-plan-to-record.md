# Plan to Record

## Status

Future RFC. This spec describes a future task package for drafting candidate
DADRs from SAPs, PAPs, preregistrations, study protocols, data-cleaning plans,
or other governing plan artifacts. It is an improvement path, not a description
of existing Dadrock behavior.

## Purpose

`plan-to-record` reads plan text and drafts candidate DADRs for consequential
planned choices that are not yet represented in the decision ledger. It is the
authoring counterpart to [12-faithfulness-plan.md](12-faithfulness-plan.md):
faithfulness reports plan gaps; `plan-to-record` drafts records for review.

## Core Principle

The task is assistive. It may detect, rank, draft, link, and explain candidate
decisions, but a human must accept, reject, split, merge, or supersede before a
candidate becomes an accepted DADR.

The accepted Markdown record is the durable rationale. Plan excerpts,
fingerprints, and source references are evidence for drafting, not a replacement
for record prose.

## Inputs

A future task should accept:

- one or more plan files or directories;
- optional plan identifiers, such as `sap-2026` or `pap-main`;
- optional path allowlists and denylists;
- optional maximum excerpt length;
- optional remote-model policy, defaulting to local-only or explicit opt-in.

## Detection Strategies

The task should combine multiple signals:

- parse headings, numbered items, tables, appendices, and amendment sections;
- detect planned sample rules, outcomes, treatments, covariates, models,
  inference choices, robustness checks, reporting commitments, and governance
  requirements;
- match each plan item against existing accepted records via `plan/<id>` tags,
  body links to plan sections, and semantic similarity;
- identify plan items with no implementing record;
- identify plan text that names explicit alternatives, such as complete-case
  analysis versus multiple imputation;
- flag ambiguous plan language as low-confidence rather than inventing
  rationale.

## Candidate Evidence

Future plan-derived candidates should add:

- `writer/agent`, the recommended provenance tag
  ([`docs-src/tags.md`](../docs-src/tags.md#other-namespaces));
- `plan/<id>` when the plan id is known;
- a source link to the relevant plan section;
- a bounded excerpt or fingerprint of the plan text;
- `confidence/low`, `confidence/medium`, or `confidence/high`;
- warnings for ambiguous wording, missing plan identifiers, or likely overlap
  with existing records.

The candidate itself still uses the ordinary DADR candidate format. Long plan
excerpts should not be embedded in the body.

## Deduplication

Before proposing a candidate, the task should compare the plan item against:

- accepted records;
- live candidates;
- other plan-derived proposals in the same run;
- future manuscript- or code-derived proposals when combined runs exist.

Potential duplicates should be grouped for review rather than written as
separate candidates by default.

## Review Rules

Plan-derived candidates need extra care around deviations and amendments:

- a candidate that implements a plan item follows normal DADR review;
- a candidate that clarifies ambiguous plan text should show the governing plan
  section beside the draft;
- a candidate that amends a plan should link to the amendment artifact;
- a candidate that deviates from a plan must include planned choice, actual
  choice, reason, expected impact, and reviewer approval before acceptance.

The task must never mark a deviation as approved or mutate locked plan text.

## Privacy and Minimization

The task should store paths, plan IDs, hashes, summaries, and bounded excerpts
by default. Sensitive plan material should not be sent to a remote model unless
the project explicitly enables that behavior.

## Failure Modes

- treating every plan sentence as a separate decision;
- missing decisions embedded in tables, footnotes, appendices, or amendment
  history;
- duplicating records that already implement a plan section;
- presenting ambiguous plan language as high-confidence;
- drafting deviations without the required human approval boundary;
- storing more plan text than needed for review.
