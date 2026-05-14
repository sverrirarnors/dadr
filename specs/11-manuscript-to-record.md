# Manuscript to Record

## Status

Future RFC. This spec describes a future task package for drafting candidate
DADRs from manuscripts, supplementary materials, slides, dashboards, or other
rendered reporting artifacts. It is an improvement path, not a description of
existing Dadrock behavior.

## Purpose

`manuscript-to-record` reads reporting prose and drafts candidate DADRs for
analytic claims whose underlying decisions are not yet represented in the
ledger. It is the authoring counterpart to
[14-faithfulness-manuscript.md](14-faithfulness-manuscript.md):
faithfulness reports unbacked claims; `manuscript-to-record` drafts records for
review.

## Core Principle

The task is assistive. It may detect, rank, draft, link, and explain candidate
decisions, but a human must accept, reject, split, merge, or supersede before a
candidate becomes an accepted DADR.

The manuscript is evidence about reported analytic choices. It is not itself a
durable rationale unless the resulting DADR body explains the decision.

## Inputs

A future task should accept:

- one or more manuscript source files, rendered text files, or directories;
- optional section filters, such as methods, results, appendix, or captions;
- optional source maps from rendered output back to source files;
- optional path allowlists and denylists;
- optional maximum excerpt length;
- optional remote-model policy, defaulting to local-only or explicit opt-in.

PDF-only inputs should require a separate text-extraction step before this task
runs.

## Detection Strategies

The task should combine multiple signals:

- parse methods and results headings;
- inspect table notes, figure captions, appendix sections, and robustness
  paragraphs;
- detect "we did X" claims, such as excluded, winsorized, imputed, fitted,
  clustered, bootstrapped, adjusted, or transformed;
- detect numerical claims, such as sample sizes, coefficients, standard errors,
  p-values, confidence intervals, and model summaries;
- detect robustness and sensitivity claims;
- detect plan-deviation or amendment disclosures;
- match claims against accepted records and live candidates by tags, body text,
  source links, and semantic similarity.

## Candidate Evidence

Future manuscript-derived candidates should add:

- `origin/manuscript-to-record` or another project-approved origin tag;
- source file, line, section, caption, or paragraph reference;
- the surrounding sentence or a bounded excerpt when useful;
- `confidence/low`, `confidence/medium`, or `confidence/high`;
- warnings for boilerplate prose, weak evidence, ambiguous claims, or likely
  overlap with existing records.

The candidate itself still uses the ordinary DADR candidate format. Long
manuscript excerpts should not be embedded in the body.

## Deduplication

Before proposing a candidate, the task should compare the manuscript claim
against:

- accepted records;
- live candidates;
- other manuscript-derived proposals in the same run;
- future plan- or code-derived proposals when combined runs exist.

Potential duplicates should be grouped for review rather than written as
separate candidates by default.

## Review Rules

Manuscript-derived candidates need extra care because reporting prose often
omits rationale:

- boilerplate methods claims should usually produce low-confidence candidates;
- numerical claims should link to the source sentence and, when available, the
  table or figure where the claim appears;
- claims that appear to disclose a plan deviation should require the same
  deviation fields as plan-linked DADRs before acceptance;
- unbacked robustness claims should be drafted as candidates only when the
  claimed analytic choice is clear enough to review.

The task must never edit the manuscript or mark a claim as faithful without
human review.

## Privacy and Minimization

The task should store paths, section identifiers, hashes, summaries, and bounded
excerpts by default. Drafting from unpublished manuscripts should avoid remote
model calls unless the project explicitly enables that behavior.

## Failure Modes

- generating candidates from generic boilerplate, such as "standard methods";
- treating reported results as rationale rather than evidence;
- missing decisions hidden in table notes, captions, supplements, or footnotes;
- duplicating records that already support the manuscript claim;
- overfitting to one manuscript format or heading convention;
- storing more manuscript text than needed for review.
