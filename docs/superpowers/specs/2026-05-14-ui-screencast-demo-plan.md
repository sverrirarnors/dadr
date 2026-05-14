# Dadrock UI Screencast Demo Plan

## Purpose

Create a compact set of prepared UI-only demo states for Dadrock screencasts.
The recordings should illustrate the important or complicated parts of the
workflow without ever showing command-line usage. Any server startup, fixture
reset, or file preparation happens before recording.

The screencasts should use a small family of named demo examples rather than a
large matrix of unrelated fixture directories. The lifecycle demo covers DADR
concepts; the Penguins demo covers realistic code-to-record detection; the
broken and conflicts demos cover advanced repair workflows.

## Principles

- The UI is the product surface. Screencasts start with Dadrock already open.
- The human approval boundary is always visible: tasks propose, users accept,
  reject, save, merge, or supersede.
- Use a small number of resettable project states instead of many independent
  demo projects.
- Each state should have enough data to make the UI interesting, but not so much
  that the viewer loses the thread.
- Agent task demos should be deterministic for recording. Prefer prepared
  task-run fixtures or a stable local/mock provider over live nondeterministic
  output.
- Do not record future-work surfaces as if they are shipped. Hooks, code-tag
  scanning, and faithfulness checks can be mentioned only in roadmap material.

## Screencast Set

| Screencast | Primary point | Demo state |
|---|---|---|
| Initialize a project | A folder becomes a DADR project from the UI | S0, then S1 |
| Create the first candidate | Manual candidate authoring in the UI | S1 or S2 |
| Review a candidate | Edit, validate, accept, and reject candidates | S3 |
| Workbench tour | Records, filters, detail pane, tags, links, settings | S4 |
| Context to record task | Turn supplied context into one candidate | S3 or S4 |
| Code to records task | Onboard existing analysis code into candidate records | S5 |
| Supersede a decision | Change direction without rewriting history | S4 |
| Validation and repair | Diagnose front matter, links, tags, and structure | S6 |
| Find conflicts | Detect overlap and contradictions in the ledger | S7 |
| Merge records | Consolidate overlapping candidate drafts | S7 |
| Edit record task | Apply reviewer feedback to a candidate | S3 or S7 |
| Tags, search, and navigation | Keep a larger ledger usable | S4 |
| Task history and results | Inspect persisted runs and typed outputs | S8 |

## Minimal Project Strategy

Use `dadrock demo --example <name>` to prepare the smallest project needed for
each recording:

| Example | Primary use |
|---|---|
| `lifecycle` | Initialization follow-up, workbench tour, tags, links, supersession |
| `candidate-review` | Focused candidate queue for accept, reject, revise, and low-confidence triage |
| `penguins` | Realistic code-to-record detection from Python analysis code |
| `broken` | Validation queue and repair |
| `conflicts` | Find-conflicts, merge-records, and supersession follow-up |
| `task-history` | Persisted successful task runs for typed result rendering |

The `penguins` example is the realistic analysis story: a `uv` Python script
downloads Palmer Penguins data from Rdatasets when run separately, subsets
species, creates a species dummy, drops missing model variables, and fits a
statsmodels OLS model with a formula.

The only separate fixture should be an uninitialized empty folder if the UI
cannot initialize from an already-staged project root. Even then, it should use
the same project name and narrative.

## Required Stages

### S0: Uninitialized Folder

Purpose: initialization screencast.

Contents:

- A project folder with ordinary analysis files but no `dadr/` directory.
- A simple README or project title visible in the UI if Dadrock exposes one.
- Optional analysis folders such as `analysis/`, `data-notes/`, and `paper/`.

Recording starts with Dadrock showing the uninitialized state. The user clicks
the initialize action and lands in an empty initialized workbench.

### S1: Empty Initialized Project

Purpose: first-candidate screencast and clean workbench orientation.

Contents:

- Initialized `dadr/` project layout.
- Candidate template present.
- No accepted records.
- No active candidates, or one unsaved draft created during the recording.

This state should make the first manual candidate feel obvious: the ledger is
empty, so the user records the first consequential choice.

### S2: First Candidate Draft

Purpose: show save, edit, validation, and the candidate queue without needing a
large ledger.

Contents:

- One candidate about the initial analysis sample or treatment assignment.
- Good title and tags, but body text with one or two intentionally incomplete
  sections so validation/editing has a reason to exist.

This can be created live from S1 or kept as a reset state for retakes.

### S3: Candidate Review Queue

Purpose: core candidate review, context-to-record, and edit-record demos.

Contents:

- A project seeded with `dadrock demo --example candidate-review`.
- Three to five candidates:
  - one strong candidate ready to accept;
  - one weak or duplicate candidate to reject;
  - one candidate needing reviewer edits;
  - one agent-origin candidate with `origin/agent` and `confidence/high`;
  - optional low-confidence candidate with an open question.
- At least one candidate with a broken or incomplete section for validation.
- Enough tags to show filtering: `kind/sample`, `kind/measurement`,
  `kind/inference`, `origin/human`, `origin/agent`, `confidence/high`.

This state is the most reusable early-project state and is now represented by
the `candidate-review` named example.

### S4: Populated Ledger

Purpose: workbench tour, tags/search, links, supersession, and mature ledger
navigation.

Contents:

- Six to ten accepted records.
- Two rejected records.
- At least one active candidate.
- One superseded accepted record with successors.
- Inter-record wikilinks that demonstrate graph navigation.
- Front-matter and inline tags across several namespaces:
  `kind/`, `topic/`, `axis/`, `confidence/`, `origin/`, `workflow/`.
- Optional bibliography examples if citation rendering is visible.

This should closely resemble the built-in demo project, with any labels tuned
for screencast clarity.

### S5: Penguins Analysis-Onboarding Project

Purpose: code-to-record task screencast.

Contents:

- A project seeded with `dadrock demo --example penguins`.
- Realistic analysis files under paths visible in the UI path picker.
- Analysis files should contain distinct consequential choices:
  - sample exclusions;
  - species subset;
  - outcome selection;
  - dummy-variable construction;
  - complete-case handling;
  - model formula;
  - statsmodels OLS estimator.
- Prepared deterministic task output for `code-to-record` with a candidate
  batch, including one obviously good proposal, one duplicate, and one
  low-confidence proposal.

### S6: Broken Ledger

Purpose: validation queue and repair screencast.

Contents:

- A project seeded with `dadrock demo --example broken`, based on the lifecycle
  example plus intentional issues:
  - malformed or missing required front-matter field;
  - accepted record missing one required body section;
  - malformed tag;
  - broken wikilink;
  - supersession edge pointing to an invalid or non-accepted record, if the UI
    surfaces that diagnostic clearly.

Keep this as a separate reset state, not something created live, so the
validation screencast opens with a useful diagnostic queue immediately.

### S7: Conflict And Merge Queue

Purpose: find-conflicts and merge-records screencasts.

Contents:

- A project seeded with `dadrock demo --example conflicts`, based on the
  lifecycle example plus overlapping or contradictory candidates:
  - two candidates describing the same sample exclusion;
  - one candidate that conflicts with an accepted imputation or complete-case
    decision;
  - two candidates that should be merged into one clearer candidate;
  - one accepted record that should be superseded rather than edited.
- Prepared deterministic `find-conflicts` output.
- Prepared deterministic `merge-records` output for the duplicate candidates.

This state can fork from S4 with additional candidate clutter. It should not be
the default workbench-tour state because duplicate records are distracting.

### S8: Task History Showcase

Purpose: task history and typed-result rendering screencast.

Contents:

- A project seeded with `dadrock demo --example task-history`.
- Persisted task runs under `dadr/runs/` for the important output kinds:
  - `candidate_draft.v1` from `context-to-record`;
  - `candidate_batch.v1` from `code-to-record`;
  - `supersession_proposal.v1` from `supersede-record`;
  - `conflict_set.v1` from `find-conflicts`;
  - `candidate_draft.v1` from `merge-records`;
  - `record_revision.v1` from `edit-record`;
  - `markdown_report.v1` from `custom`, if recording the custom-task episode.

This state is useful for retakes and for a focused task-history episode.

## Collapsed Fixture Map

The stages can be implemented as reset points for one project:

| Reset state | Covers | Can derive from |
|---|---|---|
| S0 uninitialized | initialization | clean folder |
| S1 initialized-empty | first candidate | S0 after initialize |
| S3 candidate-review | manual review, context-to-record, edit-record | `dadrock demo --example candidate-review` |
| S4 populated-ledger | workbench, tags, search, supersession | `dadrock demo --example lifecycle` |
| S5 analysis-onboarding | code-to-record | `dadrock demo --example penguins` |
| S6 broken-ledger | validation repair | `dadrock demo --example broken` |
| S7 conflict-merge | find-conflicts, merge-records | `dadrock demo --example conflicts` |
| S8 task-history | task result tour | `dadrock demo --example task-history` |

S2 is optional because it can be created during the first-candidate recording or
kept as a convenience snapshot for retakes.

## Preparation Needed

1. Keep `lifecycle` as the default named demo example.
2. Use the `penguins` named example for code-to-record and path-picker demos.
3. Use `broken` and `conflicts` named examples for advanced repair workflows.
4. Create resettable snapshots for S0, S1, S3, S4, S5, S6, S7, and optionally
   S8.
5. Create deterministic task outputs or a stable local/mock task runner for:
   `context-to-record`, `code-to-record`, `supersede-record`,
   `find-conflicts`, `merge-records`, and `edit-record`.
6. Verify every screencast can start from a prepared URL and needs no visible
   terminal interaction.
7. Write a short shot list for each screencast, including the exact starting
   state, start URL, primary clicks, and ending state.

## Open Decisions

- Whether to implement snapshots as copied fixture directories, generated
  fixtures, or named reset commands used off-camera.
- Whether to add more named examples later for manuscript-to-record or
  plan-to-record once those task packages exist.
- Whether task demos should use real ACP output during recording or seeded task
  results. Seeded results are recommended for reliability.
- Whether to include the custom task in the first recording batch. It is useful
  for power users but less important than the six built-in workflows above.

## First Recording Batch

The smallest high-impact batch is:

1. Initialize a project.
2. Create and review the first candidate.
3. Run context-to-record.
4. Run code-to-record.
5. Supersede an accepted decision.
6. Repair validation issues.
7. Find conflicts and merge duplicate candidates.

These seven recordings exercise every major UI surface and every important task
without requiring a large fixture matrix.
