# Tool: Git Hooks

## Status

Future RFC. Dadrock does not currently expose `dadrock hooks` commands. Hook
logic should share detectors and candidate-writing behavior with task-based
inference workflows rather than becoming an independent lifecycle path.

Checking changes to accepted and rejected records, in CI or a pre-commit hook,
is already implemented as `dadrock validate changes`; see
[`docs-src/changing.md`](../docs-src/changing.md). This RFC covers a different
check: detecting consequential changes to analysis code that no record
documents yet.

## Purpose

Define how DADR can use Git commit hooks to detect consequential analysis
changes and create candidate DADRs automatically, while ignoring routine,
mechanical, or low-signal changes.

The hook is an assistive guardrail. It may create candidate files and update
project state. It must not accept records, block ordinary commits by default, or
silently rewrite accepted DADR prose.

## Core Principle

The hook should run on the staged diff, classify likely analysis activities,
score whether each change is consequential, and create candidates only when the
signal is strong enough. Routine changes should pass without a candidate.

The hook optimizes for low friction. False positives should be reviewable and
easy to suppress; false negatives should be addressed through periodic
whole-repo bootstrap and review reports.

## Hook Surfaces

Recommended hook surfaces:

| Hook | Role | Default |
|---|---|---|
| `pre-commit` | Inspect staged diff and optionally create candidate files. | Enabled in advisory mode. |
| `prepare-commit-msg` | Add candidate references to the commit message when candidates were created. | Optional. |
| `commit-msg` | Validate manually supplied DADR trailers or candidate references. | Optional. |
| `post-commit` | Record hook telemetry or refresh derived state after commit. | Optional. |

The v1 default should be `pre-commit` in advisory mode. Teams may opt into
auto-staging candidates after trust in the detector improves.

## Possible Surface

A future hook surface could be:

```bash
dadrock hooks install
dadrock hooks install --mode advisory
dadrock hooks install --mode create
dadrock hooks install --mode stage
dadrock hooks uninstall
dadrock hooks run pre-commit
dadrock hooks check --range origin/main..HEAD
```

`dadrock hooks check --range` would run the same detection logic against an
arbitrary commit range without installing anything; it would be the
CI-equivalent of the local pre-commit hook.

Hook scripts should be small shims that call the Dadrock binary. The logic belongs
in the DADR implementation, not in generated shell scripts.

Example `.git/hooks/pre-commit` shim:

```sh
#!/bin/sh
exec dadrock hooks run pre-commit
```

Hooks are local Git configuration. CI should run equivalent checks explicitly
rather than relying on a developer's local hook installation.

## Modes

| Mode | Behavior | Recommended use |
|---|---|---|
| `off` | Hook does nothing. | Projects that do not want local hooks. |
| `advisory` | Report consequential changes and suggested candidates, but do not write files. | Default v1 mode. |
| `create` | Create candidate files but leave them unstaged. | Low-risk drafting. |
| `stage` | Create candidate files and `git add` them into the current commit. | Mature teams with trusted rules. |
| `block` | Fail the commit when high-confidence consequential changes lack candidate coverage. | Strict governance projects only. |

`block` mode should be opt-in and should support a clear bypass with an audit
note.

## Consequential Change Definition

A change is consequential when it plausibly affects:

- the analysis sample;
- outcome, treatment, exposure, covariate, or construct definitions;
- transformations, imputations, coding, or derived scores;
- joins, deduplication, or linkage rules;
- model formula, estimator, prior, hyperparameter, or resampling design;
- standard errors, tests, intervals, validation metrics, or multiple-testing adjustments;
- thresholds, cut points, inclusion rules, or missingness handling;
- robustness axes or specification families;
- reporting, rounding, table selection, plot encoding, or exported artifacts;
- implementation of, clarification of, amendment to, or deviation from a governing plan.

Routine changes should not create candidates by default:

- whitespace, formatting, comments without `#dadr/...` tags, and lint-only edits;
- renaming local variables with no semantic change;
- reordering columns solely for display unless the output contract changes;
- dependency import cleanup with no behavioral impact;
- file moves or path changes recovered by Git rename detection;
- test fixture churn unrelated to analysis decisions;
- generated artifact timestamp changes;
- refactors that preserve parsed operations and outputs.

Routine changes may still become candidates when a `#dadr/...` tag, commit
message, plan reference, or output difference indicates decision relevance.

## Detection Pipeline

The hook should use a staged-diff pipeline:

```text
read staged diff
  -> exclude ignored paths and generated files
  -> classify changed files by source kind
  -> extract `#dadr/...` code tags
  -> classify activities using the recommended `kind/...` vocabulary
  -> detect consequential decision surfaces
  -> compare with existing DADRs and candidates
  -> score candidates
  -> create, stage, report, or block according to mode
```

The hook should analyze only staged content. It should not inspect unstaged
changes unless the user explicitly requests it.

## Signal Sources

The hook may use:

- staged diff hunks;
- nearby unchanged context;
- `#dadr/...` code tags ([06-code-tags.md](06-code-tags.md));
- language parsers and ASTs;
- SQL parsers;
- notebook cell metadata;
- workflow definitions and lockfiles;
- configuration parameter changes;
- governing plan references;
- commit message text, when available through `prepare-commit-msg` or `commit-msg`;
- existing DADR records and candidates.

The hook should avoid remote model calls by default. Remote model use inside a
commit hook is too slow and privacy-sensitive for the default path.

## Scoring

The hook should assign each possible candidate a score and confidence label.

Recommended score components:

| Signal | Effect |
|---|---|
| Code tag such as `#dadr/<uuid>`, `#dadr/plan/...`, or `#dadr/axis/...` | Strong positive. |
| Change touches sample, measurement, transformation, modeling, inference, robustness, governance, or output activity | Positive. |
| Change includes thresholds, formulas, joins, imputations, missingness handling, or estimator arguments | Strong positive. |
| Change is linked to a governing plan or plan deviation | Strong positive. |
| Nearby human rationale in comments or commit text | Positive. |
| Output artifact changed materially with linked code/config change | Positive. |
| Pure formatting, linting, generated timestamp, or path-only rename | Strong negative. |
| Existing accepted DADR or candidate already covers the same changed range | Negative for new candidate; positive for link refresh. |
| `#dadr/ignore` tag or configured ignore rule covers the range | Strong negative. |

Candidate creation thresholds should be configurable:

```toml
[hooks]
mode = "advisory"
min_confidence = "high"
create_on_code_tag = true
stage_created_candidates = false
fail_on_high_confidence_without_candidate = false
```

The default should create or report only `high` confidence candidates.

## Candidate Creation

When the hook creates a candidate, it should:

- allocate a collision-resistant candidate ID, preferably `<uuid7>`;
- write the candidate under `dadr/inbox/`;
- set `status: candidate`;
- add tags such as `#origin/hook`, `#origin/hook/pre-commit`, and `#confidence/<level>` so the candidate's provenance is visible without a separate sidecar;
- include the Git diff range and staged blob IDs in the candidate body or in a tool-managed cache (not in the canonical front matter);
- draft short `Context`, `Decision`, and `Consequences` sections;
- attach source references as standard Markdown links in the body;
- report the created file path to the user.

Example front matter:

```yaml
---
id: 01938b8e-4d70-7abc-8def-0123456789ab
title: Drop missing treatment assignment
status: candidate
tags:
  - kind/sample
  - confidence/high
  - origin/hook/pre-commit
---
```

The hook must not change the candidate's `status` to `accepted`. Acceptance is a human action.

The hook should not create multiple candidates for the same semantic change in
one commit. It should cluster related hunks before drafting.

## Existing Coverage

Before creating a new candidate, the hook should check whether the change is
already covered by:

- an accepted DADR with fresh links to the changed code;
- an existing candidate covering the same decision surface;
- an inline `#dadr/<uuid>` code tag;
- a commit message trailer referencing a DADR;
- a configured suppression rule.

When coverage exists, the hook should refresh state links rather than creating a
duplicate candidate.

## Commit Message Trailers

Optional trailers:

```text
DADR: 01938b8e-4d70-7abc-8def-0123456789ab
DADR-Candidate: 01938b8e-4d70-7abc-8def-0123456789ab
DADR-Ignore: formatting-only
```

Semantics:

- `DADR` links the commit to accepted records.
- `DADR-Candidate` links the commit to candidates created or updated by the hook.
- `DADR-Ignore` explains why no candidate is needed for a detected change.

The hook should not require trailers in advisory mode. Strict projects may
require either candidate coverage or an ignore trailer for high-confidence
consequential changes.

## Suppression

Supported suppression mechanisms:

- inline `#dadr/ignore` tags ([06-code-tags.md](06-code-tags.md));
- future path ignore rules in project configuration;
- commit trailers such as `DADR-Ignore: formatting-only`;
- one-shot bypass with `DADR_SKIP_HOOK=1`;
- Git's standard `--no-verify` bypass.

Suppressions should be visible in hook reports and, when state is updated,
recorded as hook events. Suppression should not delete existing candidates.

Example config:

```toml
[hooks.ignore]
paths = [
  "renv.lock",
  "package-lock.json",
  "outputs/generated/**"
]
activities = ["setup"]
```

## Output

The hook should print a concise report:

```text
DADR hook: 2 consequential changes detected
  created dadr/inbox/drop-missing-treatment.md
  covered by 01938b89-1234-7000-8000-aaaaaaaaaaaa: analysis/models.py:88
Routine changes ignored: 5
Bypass: git commit --no-verify or DADR_SKIP_HOOK=1 git commit ...
```

In advisory mode, the report should show what would be created without writing
files.

## Failure Behavior

The hook should fail closed only for implementation errors that make repository
state unsafe, such as invalid generated IDs or invalid generated
front matter. Detector uncertainty should not fail the commit in advisory,
create, or stage mode.

When candidate creation fails, the hook should leave the user's staged files
unchanged and print a recovery command.

## Privacy and Performance

Commit hooks must be fast and local by default:

- no network calls unless explicitly enabled;
- no remote model calls by default;
- no raw data scanning unless configured;
- skip large generated files and binary files;
- cap context window size around changed hunks;
- cache parser results where safe;
- finish within a small configured timeout.

Recommended default timeout: 5 seconds. If the timeout is exceeded, advisory
mode should warn and allow the commit.

## CI Relationship

Local hooks improve authoring discipline but are not enforceable by themselves.
CI should run equivalent checks with explicit commands:

Future CI could run `dadrock hooks check --range origin/main..HEAD` alongside
ordinary record and link validation and the existing
`dadrock validate changes --base origin/main` check on accepted records.

CI may use stricter policy than local hooks, such as failing when
high-confidence consequential changes lack DADR coverage.

## Validation Rules

Validation should check:

- hook-created candidates carry an `#origin/hook` (or `#origin/hook/<event>`) tag and a `#confidence/<level>` tag;
- hook-created candidates have valid front matter and required body sections;
- ignored high-confidence changes include a suppression reason when policy requires one;
- `stage` mode never stages accepted-record prose changes unless the user explicitly edited them.

## Failure Modes

Important failure modes:

- too many false positives, causing developers to bypass the hook;
- missed consequential changes because the detector only sees code and not plans or outputs;
- duplicate candidates for the same semantic decision;
- hook-created candidates staged into commits unexpectedly;
- slow hooks interrupting normal Git workflow;
- remote model calls leaking sensitive context;
- treating hook-created candidates as accepted governance records.

The default configuration should prioritize trust and reviewability over
maximal automation.
