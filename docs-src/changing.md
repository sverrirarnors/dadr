# Changing decisions

Once approved, a decision's substantive content is preserved. Corrections that leave the meaning intact, such as fixed spelling, formatting, links, or citations, are made in the record itself. Changing the decision, however slightly, means accepting a successor that supersedes the original.

The original remains accepted, and its `successors` field names the records that supersede it. “Superseded” describes an accepted record with successors, not a fourth stored status. DADRock keeps superseded records in its [archive](layout.md), next to rejected ones, and offers a superseded filter for convenience. The web interface's Ledger lists the decisions in force; **Show archived** adds the rejected and superseded records.

## Correcting a record

To correct a decided record, edit it. In the [web interface](dadrock/web.md) or the [standalone app](dadrock/app.md), **Edit** opens the record's Markdown in place, on every record; on a decided record, a note above the editor says the edit is for corrections. You can also open the file in your usual [editor](dadrock/editors.md). The record keeps its ID and stays in its folder. When you save from the web interface, DADRock refuses to change the `id`, `status`, or `successors` of an accepted, superseded, or rejected record, and an accepted record must keep its four required sections, each once and with text; the title, body, tags, and front-matter keys outside DADR can change.

Whether an edit corrects a record or changes the decision is a judgment no rule can make ([specification §6.3](specification.md#immutability)), so DADRock makes every edit visible instead of forbidding it. The record's [history](#history) lists the edits Git recorded to its title and body since it was accepted or rejected, including changes not committed yet, and [`dadrock validate changes`](#checking-changes) reports them with their diffs.

Agents never edit decided text. The `edit-record` agent task revises candidates only; for a decision in force, **Revise with agent** and **Fix with agent** draft a successor instead, and archived records offer no agent fix.

## Drafting a successor

To change what was decided, draft a successor: a new candidate that supersedes the old record, its predecessor. The predecessor stays in force until you accept the successor.

- **Supersede** on a record in the Ledger opens a full-page successor draft on the Draft page. It starts from the record's exact title, tags, and body and asks "Why does this decision change?"; the reason is optional. **Add a decision to merge** attaches more decisions in force to supersede them together with one successor (a merge); the draft then starts untitled, from every predecessor's sections and tags. **Create candidate** saves the successor candidate and opens it in the Inbox. **Supersede with existing decisions instead** links an accepted record to successors that already exist.
- **Revise with agent** on the record opens the Draft page, which asks "What changed, and why?" and sends your answer to the `supersede-record` [task](llms.md#custom-workflows-and-runs); the task asks a local agent to draft the successor. The task takes one or more accepted records and the change you want. The agent starts from the predecessor's exact text and changes only what the request requires, so the successor reads as a small diff; for several predecessors it combines them into one decision. It returns the reason separately, and DADRock writes it into `## Supersedes` when you save the proposal as a candidate.
- You can also write the candidate yourself.

The successor names its predecessors in a `## Supersedes` section, optionally followed by the reason for the change:

```markdown
## Supersedes

[[01938c20-a3b4-7000-8000-111122223333|Winsorize income at p99]]

Winsorizing at p99 still left implausible incomes in two waves.
```

The section starts with one or more lines, each exactly one record link by full UUID, `[[uuid]]` or `[[uuid|label]]`. The text after the links, if any, is the reason, in ordinary Markdown. A record has at most one `## Supersedes` section, and the section must start with a link; acceptance refuses a candidate that breaks either rule, although `dadrock validate` does not report it. When DADRock writes the section, from the web interface, a `supersede-record` result, or [MCP](llms.md#mcp), it is the first section of the body, each link is labeled with the predecessor's title, and the reason follows the links.

## Accepting a successor

In the Inbox, a successor candidate opens on a **Changes** tab: the reason from its `## Supersedes` section, then the difference from its predecessor, with a changed title, the tags added and removed, and a line diff of the body without `## Supersedes`. The **Document** tab shows the record itself. A successor of several decisions opens on a section view, Decision first, with each predecessor's text followed by the new text; **Compare with** switches to a line diff against one predecessor.

Accepting a successor adds its `id` to each predecessor's `successors` and moves each predecessor to `dadr/archive/` with its body unchanged. The `## Supersedes` section stays in the successor's body as the record of why the decision changed. It is a DADRock convention, not an additional required section of the DADR format. A superseded record says where the decision went ("Superseded by …", linking the decisions now in force); it keeps **Edit** but has no **Supersede** or **Revise with agent**.

- **One-to-one:** the successor names one predecessor.
- **Merge:** one successor names several predecessors, and each gains the successor's `id`.
- **Split:** several successors name the same predecessor. Accept them together so that the predecessor gains all of their ids at once.

The Inbox shows candidates that supersede the same decision as one item, "Split “X” into 2 decisions", with each candidate's reason and diff. Select the ones that belong together and choose **Accept selected**: all of them to split the decision, or one of them to choose between competing alternatives. Reject the others one by one. From a terminal, accept them together with `--with`:

```sh
dadrock accept "Impute income" --with "Complete cases for the outcome"
```

`--with` repeats, and the positional words still join into one query. In general, accepting a set of candidates adds, for each predecessor named in the set, every candidate in the set that names it. Candidates without `## Supersedes` are simply accepted.

DADRock checks the whole set before writing anything: every candidate must meet the acceptance requirements, and every named predecessor must exist, be accepted, not already be superseded, and not be in the set. A predecessor that is already superseded is refused with the names of its current successors. If a write fails partway, every record is restored.

For two records that are already accepted, `dadrock supersede --from OLD --to NEW` links them explicitly and moves `OLD` to `dadr/archive/`. Records linked this way have no `## Supersedes` section, so no reason is recorded.

## Stale successors

Two candidates that name the same predecessor may be the halves of a split or competing alternatives; you decide by what you accept together. Once one of them is accepted alone, the other is stale: its predecessor was superseded after it was drafted, like a branch whose base has moved. `dadrock validate` warns about a stale candidate (`stale_supersedes`) and names the decisions now in force, and acceptance refuses it. Point its `## Supersedes` link at the current decision, or reject it.

DADRock never retargets a link on its own. In the Inbox, a stale successor shows a warning that names the decisions now in force, and **Accept** waits until you choose **Retarget**, which points the link in `## Supersedes` at the current decision (you pick one when there are several), saves the candidate, and shows the diff against it.

Validation also warns when an accepted record's `## Supersedes` section names a predecessor that does not list it in `successors` (`unlinked_supersedes`), for example after a record was accepted by hand.

## History

A decision's history is its chain of versions, linked by supersession, plus the corrections made to each version. DADRock derives the versions from `successors` alone, so they need no extra front matter and no Git. Corrections stay in the record itself, and DADRock reads them from Git.

In the web interface, a record's **History** tab shows the decision's versions newest first, the opened one highlighted and the current ones labeled. Lanes join where a decision was split and fork where several were merged. Each change shows its reason and, on request, its diff. Records at the other end of a split or merge appear on one line ("Also became …", "Merged with …") and link to the record. Under each version, "Edited 2 times since accepted" lists its [text edits](#text-edits) newest first: the date, the author, and the commit subject, each with its diff. An edit that is not committed yet comes first, marked "Not committed yet". A record whose text changed after acceptance shows "Edited since acceptance" ("Edited since rejection" for a rejected record) next to its file name, which opens the History tab. Outside a Git repository, History shows the versions without edits.

From a terminal:

```sh
dadrock history QUERY            # versions as a graph, newest first
dadrock history QUERY --patch    # with the diff of each change and each text edit
dadrock history QUERY --json
dadrock diff OLD NEW             # compare any two records
```

`QUERY` is any version: a full id, an id prefix, a filename, or a title. The history of a record contains:

- **versions**: the record, its predecessors back to the first version, and its successors forward to the current ones;
- **current versions**: the versions without successors;
- **links**: each supersession between two records, with its reason, the text after the links in the successor's `## Supersedes` section. Records linked with `dadrock supersede` alone have no reason;
- **neighbors**: records at the other end of a link that are not versions, listed on one line without their own history;
- **text edits**: for each version, the edits Git recorded to its title or body since the version was accepted or rejected, newest first.

Versions are listed newest first, and a record always appears above its predecessors, even when it was created earlier. `◉` marks the record you asked for. A decision superseded by several records is a split; several decisions superseded by one record are a merge:

```text
●   0193b2a1d117  Multiple imputation for income  current
│ ● 0193b2a1c0f4  Complete cases for the outcome  current
├─╯ Outcome and covariates need different treatment.
●   01938c20a3b4  Listwise deletion
│   Complete cases lose a third of the sample.
◉   01938a77e310  Pairwise deletion
```

```text
◉   0193c4e1a2b0  One outlier rule for all covariates  current
├─╮ The two rules disagreed at the 99th percentile.
● │ 0193a0b7e911  Winsorize income at p99
│ ● 01939f2c4d07  Drop implausible ages
●   01938f11aa02  Trim income at p95
```

The history from one half of a split shows the other half as a neighbor ("also became …"), and the history from one input of a merge shows the other input as a neighbor ("merged with …").

Diffs compare the title, the tags, and the body without `## Supersedes`, after normalizing line endings; other front matter is not compared. `--patch` prints each link's diff under it: the title change, the tag changes (`+topic/missing-data −topic/attrition`), and the changed lines with a few lines of context. `dadrock diff OLD NEW` prints the same comparison for any two records, linked or not. Diffs are computed when shown and never stored.

The history ignores successor ids that match no record and stops at cycles, with a warning; `dadrock validate` reports both. The local web server serves the same JSON at `/api/records/{id}/history` (`?patch=1` for diffs) and `/api/records/{id}/diff?against={other}`, which compares `{other}` to `{id}`.

### Text edits

A correction that leaves the meaning intact, such as a fixed typo, link, or citation, is made in the record itself and keeps its `id` ([specification §6.3](specification.md#immutability)). DADRock never copies these edits into records. It reads them from Git with `git log --follow` on the version's file, which follows the moves between `inbox/`, `ledger/`, and `archive/`:

- The baseline is the earliest commit in which the record is accepted or rejected. Edits made while it was a candidate are not listed.
- Every later commit whose title or body (without `## Supersedes`, line endings aside) differs from the previous commit's is an edit. A commit that changes only tags, `successors`, or the folder is not.
- A change not committed yet, the working tree against `HEAD`, is listed first.

A version with edits gets one line under it in the graph, and `--patch` adds each edit's diff under a header with the commit's short hash and subject:

```text
◉   01938c20a3b4  Winsorize income at p99  current
    edited 2 times since accepted (latest 2026-10-01, Alice)
```

In JSON, each version has `edits`, newest first. Each edit is `{commit, short_commit, author, date, subject, title_changed, body_changed}`, plus `diff` with `--patch`; `date` is the author date in ISO 8601, and the commit fields are `null` for an edit not committed yet. Candidates have no edits (`[]`). Outside a Git repository, or without `git`, `edits` is `null` and the rest of the history is unchanged.

Listing the edits does not judge them. Whether an edit is a correction or a change of decision is the reviewer's call; to change the decision, supersede the record instead.

## Checking changes

Once a record is accepted or rejected, its `id` and `status` do not change, and entries are not removed from its `successors`. Its title and body take corrections that leave the meaning intact; changing the decision means superseding the record ([specification §6.3](specification.md#immutability)). `dadrock validate changes` checks the first rule against Git and shows every title and body edit, so a reviewer can judge the second:

```sh
dadrock validate changes                      # working tree against HEAD
dadrock validate changes --staged             # staged changes against HEAD
dadrock validate changes --base origin/main   # everything since another revision
dadrock validate changes --strict             # title and body edits are errors too
```

For each record file in `dadr/inbox/`, `dadr/ledger/`, or `dadr/archive/` that changed since the base revision, DADRock reads the file's version at the base. If that version was accepted or rejected, DADRock compares the current version with it. A record that moved between folders, by being accepted, rejected, or superseded, is followed through Git's rename detection or by its `id`.

- **Errors:** a changed `id` (`record_id_changed`) or `status` (`record_status_changed`), a removed `successors` entry (`successor_removed`), or a file that no longer parses (`invalid_record`). The message names the record; for a changed `id` or `status`, it suggests [drafting a successor](#drafting-a-successor) instead, or a new candidate for a rejected record.
- **Warnings, with the diff:** an edited title (`record_title_edited`) or body (`record_body_edited`; line endings aside). The message names the record and says what to do instead if the edit changes the decision: supersede it, or draft a new candidate for a rejected record. The text output prints the diff under the warning: the old and new title, or the changed body lines with a few lines of context. In JSON, the issue's `diff` carries the same change: `title` with `old` and `new`, or the body's `lines`, each `{kind, text}` with `kind` one of `context`, `added`, or `removed`.
- Deleting an accepted or rejected record is a warning (`record_deleted`).

```text
dadrock validate changes: 0 error(s), 1 warning(s) (working tree against HEAD (5309d425ba44))
[warning] record_body_edited /home/me/analysis/dadr/ledger/winsorize-income-at-p99.md: the body of accepted record `Winsorize income at p99` (01938c20a3b4) was edited since HEAD. If this changes the decision, supersede the record instead.

      ## Consequences

    - Estimates are less sensitive to outliers.
    + Estimates are much less sensitive to outliers.
```

`tags`, keys outside DADR, and new `successors` entries change freely, and so do candidates. New files have no earlier version to compare, and a repository without commits passes. Warnings alone exit with status zero. With `--strict`, title and body edits are errors too, for projects that want accepted text frozen; there is no project setting, so pass the flag wherever the check runs. Outside a Git repository, or when the base revision does not resolve, the command fails. Ordinary `dadrock validate` does not use Git.

### In continuous integration

To review what a pull request changes in accepted and rejected records, compare it with its target branch. The checkout needs that branch's history:

```sh
git fetch origin main
dadrock validate
dadrock validate changes --base origin/main
```

A changed `id` or `status`, or a removed successor, fails the check. Title and body edits appear in the CI log as warnings with their diffs, so a reviewer can tell a correction from a change of decision. A project that wants accepted text frozen adds `--strict`, which fails the check on those edits too:

```sh
dadrock validate changes --base origin/main --strict
```

`--base origin/main` compares with the tip of `main`. That is right when the checkout already includes `main`, such as the merge commit many CI services check out for a pull request. For a branch that `main` has moved past, compare with the point where the branch started: `--base "$(git merge-base origin/main HEAD)"`. A CLI-only CI build can use the [`no-js-build` feature](dadrock/index.md#without-the-web-interface).

### In a pre-commit hook

To catch changes before they are committed, run the check on the staged changes in a pre-commit hook, `.git/hooks/pre-commit`, made executable:

```sh
#!/bin/sh
exec dadrock validate changes --staged
```

The hook blocks a commit only on errors; title and body edits print as warnings. Add `--strict` to block those edits too. Git runs hooks from the repository root. If the project lives in a subdirectory, such as `analysis/dadr/`, change into that subdirectory first: `cd analysis && exec dadrock validate changes --staged`.
