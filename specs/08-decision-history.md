# Tool: Decision History

## Status

In progress (Kata epic `dadr#zfgm`). When it lands, the Dadrock behavior is
documented in [`docs-src/dadrock/`](../docs-src/dadrock/) and this file is
removed.

Revised 2026-10-11 (`dadr#gk69`): an earlier draft froze accepted records, so
that every edit, even a spelling fix, became a new version. That was dropped.
Records are plain files that people edit outside any tool, nobody writes notes
or commit messages by hand, and a freeze pushes corrections out of sight.
Decision history is supersession; text history is Git.

Before this work, the Ledger let a reviewer edit an accepted record in place
(**Edit** and **Revise with agent**, both saved through `save_record_source`),
a candidate's `## Replaces` section could name exactly one predecessor, records
lived in `dadr/candidates/`, `dadr/accepted/`, and `dadr/rejected/`, and the
record detail showed only the immediate predecessors and successors.

## Purpose

Make every change of decision a new record, keep corrections in place, and show
a decision's history as its chain of versions with the differences between
them, plus the text edits Git recorded for each version, the way `git log -p`
shows the history of a file. Splits (one decision superseded by several) and
merges (several superseded by one) are part of the same model.

## Principles

- **Every change of decision is a new version.** Changing what was decided
  means drafting a successor candidate, which a human accepts or rejects like
  any other candidate.
- **Corrections stay in place.** Fixing spelling, formatting, links, or
  citations edits the record itself: same id, nothing to write, as §6.3 of the
  specification already allows. The line between a correction and a change of
  decision is a judgment that no rule can make, so the tools make every edit
  visible instead of forbidding it.
- **Versions, not patches.** Each record stays a complete record that reads on
  its own. Diffs are computed when shown and never stored.
- **Decision history lives in the record set; text history lives in Git.**
  Supersession is derived from `successors`, needs no new front-matter key, and
  can be read without Git. Edits to a record's text are read from Git and shown,
  never copied into records.
- **Agents never edit accepted text.** They only propose successors.
- **The reason travels with the change.** The successor's `## Supersedes`
  section says why the decision changed, like a commit message.
- **Folders follow the workflow.** What needs a decision, what is in force, and
  what is kept but rarely read each have one folder.

## Vocabulary

| Term | Meaning |
|---|---|
| supersede | The only verb for one record taking the place of others. "Replace" and "replacement" are not used. |
| successor / predecessor | The newer and older record of a supersession. Matches the `successors` key. |
| superseded | An accepted record with a non-empty `successors` array (derived, not a status). |
| Inbox / `dadr/inbox/` | Records waiting for a human decision: candidates. |
| Ledger / `dadr/ledger/` | Decisions in force: accepted records without successors. |
| archive / `dadr/archive/` | Records kept but not in force: rejected records and superseded records. |
| history | One decision's chain of versions, with the text edits Git recorded for each. |

In the web UI the Ledger's toggle reads **Show archived**, and the project
popover's **Clear agent history** reads **Clear agent runs** (task runs live
in `dadr/runs/`).

## The format

§6.3 Immutability keeps its wording: `id` and `status` never change, entries
are never removed from `successors`, and the meaning of the title and body
never changes; corrections that leave the meaning intact are allowed, and
changing a decision means superseding it. One sentence is added after "The
second requires a reviewer's judgment.":

> Tools can support that judgment by listing edits to accepted records, for
> example from version-control history.

§6.2 needs no change: `successors` is already many-to-many and acyclic. The
format names no directories, so the layout below is Dadrock's alone.

## Layout

| Directory | Holds | Front matter |
|---|---|---|
| `dadr/inbox/` | candidates | `status: candidate` |
| `dadr/ledger/` | decisions in force | `status: accepted`, no `successors` |
| `dadr/archive/` | rejected and superseded records | `status: rejected`, or `status: accepted` with `successors` |

`dadr/templates/`, `dadr/tasks/`, and `dadr/runs/` are unchanged.

- `dadrock init` creates `inbox/`, `ledger/`, and `archive/`. Project
  discovery looks for those three. The old layout is not read and not migrated.
- Accepting moves a record from `inbox/` to `ledger/`; rejecting moves it from
  `inbox/` to `archive/`; superseding moves each predecessor from `ledger/` to
  `archive/`, whether through acceptance of a successor or `dadrock supersede`.
- The location check warns when a record's folder does not match the table.
- Moving a file breaks relative Markdown links to its path. Wikilinks and
  `#dadr/` code tags use ids and are unaffected.

## Editing records

- **Edit** works in place on every record: candidates freely; accepted,
  rejected, and superseded records for corrections. The save path refuses to
  change `id`, `status`, or `successors` (supersession adds successors), and an
  accepted record keeps its four required sections. Title, body, tags, and keys
  outside DADR may change. Line endings are normalized when comparing.
- **Supersede** on a decision in force opens a full-page successor draft
  pre-filled with the record's exact title, body, and tags, plus a
  `## Supersedes` section linking the record and an optional reason. Because
  the starting text is identical, the diff shows only what changed.
- **Revise with agent** on a decision in force runs `supersede-record`; on a
  candidate it runs `edit-record`. Archived records have no **Supersede** or
  **Revise**. Agents never edit accepted text: `edit-record` only targets
  candidates, and no MCP tool rewrites a record's text.
- `supersede-record` takes one or more predecessors (several for a merge). Its
  prompt starts from the predecessor's text and changes only what the request
  requires; it does not rewrite Context to narrate the supersession. Its output
  lists `superseded_record_ids`, the successor's title and four-section body,
  and a `rationale`, which becomes the reason in `## Supersedes`.

## The `## Supersedes` section

```markdown
## Supersedes

[[01938c20-a3b4-7000-8000-111122223333|Winsorize income at p99]]
[[0193a0b7-e911-7000-8000-444455556666|Drop implausible ages]]

The two rules disagreed at the 99th percentile; one rule now covers every
continuous covariate.
```

- At most one `## Supersedes` section per record. When Dadrock writes it, it
  is the first section of the body.
- It starts with one or more lines, each exactly one record link by full UUID,
  with an optional label. Any following non-empty lines are the reason,
  ordinary Markdown. A section with no link is an error.
- The section stays in the body after acceptance and is the permanent record of
  why the decision changed. It is excluded from diffs.
- Records linked with `dadrock supersede` alone have no `## Supersedes`
  section, and their history shows no reason.
- `## Supersedes` is a Dadrock convention, not a section the format requires.

## Accepting successors

**One-to-one.** Accepting the candidate adds its `id` to the predecessor's
`successors` and moves the predecessor to `archive/`.

**Merge.** One candidate names several predecessors. Accepting it adds its
`id` to every predecessor's `successors`.

**Split.** Several candidates name the same predecessor and are accepted
together, so the predecessor gains all of their `id`s at once:
`dadrock accept B --with C` (`--with` repeats). `dadrock accept` still joins
its positional words into one query. In general, accepting a set of candidates
adds, for each predecessor named in the set, every candidate in the set that
names it.

**Checks before any write.** Every candidate in the set meets the acceptance
requirements, and every named predecessor is accepted, not superseded, and not
in the set. A superseded predecessor is refused with a message naming its
current successors. If a later write fails, earlier writes are restored, as
`accept_candidate` already does for its move.

**Stale successor candidates.** A candidate whose `## Supersedes` names a
record that is no longer current (superseded since the candidate was drafted)
is stale, like a branch whose base has moved. Validation warns about it, and
the record lifecycle data reports it, so the Inbox can offer to retarget the
link at the current successor. Dadrock never retargets on its own. Two
candidates naming the same predecessor may be the halves of a split or
competing alternatives; the reviewer decides by what they accept together.

**Consistency.** Validation warns when an accepted record's `## Supersedes`
names a predecessor that does not list it in `successors`.

## History

### What a history contains

For a record R:

- **versions**: R, all of its predecessors back to the first version, and all
  of its successors forward to the current ones;
- **current versions**: the versions with no successors;
- **links**: every supersession edge that touches a version, with the reason
  from the successor's `## Supersedes` section;
- **neighbors**: records at the other end of a link that are not versions (the
  other half of a split, the other input of a merge), shown as one-line entries
  without their own history;
- **text edits**: for each version, the edits Git recorded to its title or body
  since it was accepted or rejected, newest first.

### Text edits from Git

- Read with `git log --follow` on the version's file, which follows the moves
  between `inbox/`, `ledger/`, and `archive/`. The baseline is the earliest
  commit in which the file's front matter says `accepted` or `rejected`; every
  later commit whose title or body (without `## Supersedes`, line endings
  normalized) differs from the previous commit's is an edit. Changes to tags,
  `successors`, or the folder alone are not edits.
- An uncommitted change in the working tree (title or body differing from
  `HEAD`) is listed first, without a commit.
- Each edit has the commit (full and short hash), author, author date (ISO 8601,
  from Git), subject, which of title and body changed, and with `--patch` the
  diff. Candidates have no edits. Outside a Git repository, or without `git`,
  `edits` is null and everything else works.

The history is marked at the opened version. Along a chain, opening any
version shows the same versions; across a split or a merge, the branches that
do not lead to or from the opened version appear as neighbors.
Versions are listed newest first, and a record always appears above its
predecessors; ties are broken by id timestamp. Id order alone is not enough,
because `dadrock supersede` can link a successor created before its
predecessor.

### Diffs

- The compared text is the title and the body without `## Supersedes`, after
  normalizing line endings. Tag changes are summarized
  (`+topic/missing-data −topic/attrition`). Other front matter is not compared.
- **A record with one predecessor** (a one-to-one supersession, or each record
  of a split) gets a line diff against it.
- **A record with several predecessors** (a merge) opens on a section view: for
  each of the four required sections, starting with Decision, each
  predecessor's text followed by the new text. A line diff against each
  predecessor is available on request.

### Command line

```bash
dadrock history QUERY            # versions as a graph, newest first
dadrock history QUERY --patch    # with a diff for each link
dadrock history QUERY --json
dadrock diff OLD NEW             # compare any two records
```

Both are read-only `dadrock::commands` functions returning a `CommandEnvelope`,
listed in `commands::COMMANDS`, and available through the web API
(`/api/records/{id}/history`) and MCP. The JSON form lists `versions` (id,
short id, title, status, superseded, current, path, tags, body without
`## Supersedes`, and `edits`), `links` (predecessor, successor, reason, and with
`--patch` the diff lines), and `neighbors` (summaries). Each edit is
`{commit, short_commit, author, date, subject, title_changed, body_changed}`
plus `diff` with `--patch`; `commit`, `short_commit`, `author`, `date`, and
`subject` are null for the uncommitted working-tree edit. In text output, a
version with edits gets a line such as `edited 2 times since accepted (latest
2026-10-01, Alice)`, and `--patch` prints each edit's diff.

A split whose predecessor had an earlier version:

```text
●   0193b2a1c0f4  Multiple imputation for income     current
│ ● 0193b2a1d117  Complete cases for the outcome     current
├─╯ Outcome and covariates need different treatment.
●   01938c20a3b4  Listwise deletion
│   Complete cases lose a third of the sample.
●   01938a77e310  Pairwise deletion
```

A merge:

```text
●   0193c4e1a2b0  One outlier rule for all covariates  current
├─╮ The two rules disagreed at the 99th percentile.
● │ 0193a0b7e911  Winsorize income at p99
│ ● 01939f2c4d07  Drop implausible ages
●   01938f11aa02  Trim income at p95
```

### Web UI

- **Inbox.** A successor candidate opens on a **Changes** tab with the diff
  against its predecessor. A merge has a "Compare with" selector and the
  section view. Candidates that name the same predecessor appear as one item,
  where the reviewer selects which to accept together. Stale successor
  candidates are flagged with a **Retarget** action.
- **Ledger.** Every record has a **History** view, replacing the one-step
  predecessor and successor links, with each version's text edits (date,
  author, subject, expandable diff; "not committed yet" for working-tree
  edits). A record edited since acceptance is marked as such. The **Show
  archived** toggle lists archived records of every decision; the History view
  shows the versions of one.
- **Record actions.** **Edit** in place on every record, with a note on
  accepted and archived records that corrections stay in the record and show
  in History, and that changing the decision means **Supersede**. **Supersede**
  (decisions in force only) opens the pre-filled successor draft.

## Git check: `dadrock validate changes`

```bash
dadrock validate changes                       # working tree against HEAD
dadrock validate changes --staged              # index against HEAD (pre-commit)
dadrock validate changes --base origin/main    # CI
dadrock validate changes --strict              # text edits are errors too
```

For each record file under `dadr/` that changed between the base revision and
the working tree (or the index with `--staged`), with rename detection, and
that was accepted or rejected at the base:

- **Errors:** a changed `id` or `status`, a removed `successors` entry, or a
  file that no longer parses.
- **Warnings, with the diff:** a changed title (`record_title_edited`) or body
  (`record_body_edited`). They appear in CI logs and pull-request checks so a
  reviewer can tell a correction from a change of decision, without blocking.
  The text output prints the diff under the warning; the JSON issue carries it.
  `--strict` makes them errors, for projects that want accepted text frozen.
  There is no project configuration file; CI and hooks pass the flag.
- Deleting an accepted or rejected record is a warning.
- Outside a Git repository the command fails with a clear message; ordinary
  validation is unchanged.

Considered and set aside: freezing accepted text by default (see Status), a
`## Corrections` section written by hand or generated (people do not write
notes; generated entries duplicate Git), classifying an edit by the section it
touches (surprising), and a digest in front matter (says that the text changed
but not how, flags every correction).

## Open questions

- Should `## Supersedes` become part of the format, so that the reason for a
  change is defined independently of Dadrock?
- Agent support for drafting a split: the task output would need to return
  several candidates.
