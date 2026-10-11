# Web interface

```sh
dadrock preview
dadrock preview --port 4005 path/to/project
```

Open the local URL printed in the terminal. The interface has three places, side by side in the header: [Inbox](#inbox), [Ledger](#ledger), and [Draft](#draft).

## First decision

Five minutes from an empty project to one accepted decision. You do not need an agent or tags.

Change to the project where you keep your analysis, then run:

```sh
dadrock init
dadrock new "Exclude observations without treatment assignment"
```

`init` creates a `dadr/` directory without overwriting existing records. `new` prints the location of a candidate Markdown file and generates its ID.

Start the web interface and open the local URL printed in your terminal:

```sh
dadrock preview
```

The candidate is waiting in the **Inbox**. Open it, choose **Edit**, and fill in its four sections (or edit the Markdown file in your usual [editor](editors.md)):

- **Context:** Some observations lack treatment assignment.
- **Decision:** Exclude those observations from this analysis.
- **Alternatives:** Recover assignment from the original log; explain whether this was possible.
- **Consequences:** Report how many observations were removed and consider whether missingness affects interpretation.

Replace these illustrative statements with what actually happened in your project. In particular, explain *why* you chose this approach.

Check that the record describes your analysis accurately and that you endorse the rationale. Save your edits, then choose **Accept**. The record moves to `dadr/ledger/`, keeping its ID, and appears in the **Ledger**. Commit it with your analysis code. You can also accept from a terminal with `dadrock accept ID`, replacing `ID` with the candidate's ID or an unambiguous prefix.

Find it again later:

```sh
dadrock list --status accepted
dadrock validate
```

For a disposable example instead of your own analysis, run `dadrock demo --example candidate-review /tmp/dadr-example`, then `dadrock preview /tmp/dadr-example`. The demo requires a new or empty target directory.

## Inbox

The Inbox is the home screen and the work queue. Everything that needs a human lands here, grouped by kind:

- **Candidates** wait for a decision. Read the record, fix its explanation if needed, then **Accept** or **Reject**. **Edit** opens the Markdown; **Revise with agent** sends it back to a local agent with your instructions.
- **Successor candidates** open on a **Changes** tab: the reason from their [`## Supersedes`](#changing-a-decision) section, then the difference from the predecessor, with a changed title, the tags added and removed, and a line diff of the body without `## Supersedes`. The **Document** tab shows the record itself. A successor of several decisions (a merge) opens on a section view, Decision first, with each predecessor's text followed by the new text; **Compare with** switches to a line diff against one predecessor.
- **Splits** are candidates that supersede the same decision, shown as one item: "Split “X” into 2 decisions". Each candidate shows its reason and its diff. Select the ones that belong together and choose **Accept selected**: all of them to split the decision, or one of them to choose between competing alternatives. Reject the others one by one.
- **Stale successors** name a decision that was superseded after they were drafted. A warning names the decisions now in force, and **Accept** waits until you choose **Retarget**, which points the link in `## Supersedes` at the current decision (you pick one when there are several), saves the candidate, and shows the diff against it.
- **Agent proposals** are decisions a local agent drafted from your notes or your code. **Save as candidate** writes the file so you can review it like any other candidate; **Discard** drops it.
- **Proposed revisions** show an agent's edit to a candidate as a diff. **Apply** writes it; **Discard** drops it.
- **Conflicts** come from **Check for conflicts**, which asks the agent to look for overlapping or contradictory decisions. From a conflict you can **Merge** two candidates, **Supersede** an accepted decision, or **Dismiss** the finding.
- **Issues** are validation findings: structure, links, tags, and project layout. Validation runs on every load, so there is nothing to trigger. **Open record** jumps to the record; **Fix with agent** asks the agent to revise a candidate or to draft a successor of a decision in force; warnings and notes can be dismissed. Archived records never change, so their issues offer no fix.

Review both the observed choice and its rationale. An agent-generated description of code may be accurate while the reason for that choice remains unknown. Supply the missing rationale before accepting.

An empty Inbox is the goal. The search box narrows the queue to matching titles, tags, paths, and messages. Use `j` and `k` to move through items, `a` and `r` for the primary and secondary action, `e` to edit, and `o` to open the record in the Ledger.

Record text is edited in a CodeMirror editor with Markdown and front-matter highlighting, search (`Ctrl+F`), and `Ctrl+S` to save.

## Ledger

The Ledger is what has been decided. It lists the decisions in force, newest first, with one search box and a tag facet. **Show archived** adds the archived records: rejected and superseded ones.

A decision in force never changes in place. **Edit** opens a full-page successor draft on the Draft page, starting from the record's exact title, tags, and body, with a required "Why does this decision change?" field. Saving creates a successor candidate and opens it in the Inbox on its **Changes** tab; the decision stays in force until you accept the successor. **Revise with agent** asks a local agent to draft the successor (`supersede-record`), and **Supersede** opens the Draft page to draft it with the agent or by hand, or to link decisions that already exist. Tags stay editable in place on accepted and rejected records: the tag button next to them saves only `tags`.

Archived records have no **Edit** or **Revise with agent**. A superseded record says where the decision went ("Superseded by …", linking the decisions now in force).

The **History** tab shows the decision's versions newest first, the opened one highlighted and the current ones labeled. Lanes join where a decision was split and fork where several were merged. Each change shows its reason and, on request, its diff. Records at the other end of a split or merge appear on one line ("Also became …", "Merged with …") and link to their own history. From a terminal, [`dadrock history`](cli.md#history) shows the same versions.

## Draft

**Draft** is the third place: a full page reached from the header or from any record. Describe the decision in the text box, or attach files and folders to have a [configured agent](llms.md#acp) look for decisions in your code. Attach two candidates to merge them. Opened from a record, the same page revises a candidate or drafts a successor of a decision in force; attach more decisions in force to supersede them together with one successor (a merge). Results land in the Inbox as proposals.

**Write it by hand** (the default without an agent) gives you a plain form with the candidate template. For a successor, the form starts from the predecessor's exact text, or from every predecessor's sections for a merge, and asks why the decision changes; DADRock writes the `## Supersedes` section from it. **Supersede with existing decisions instead** links an accepted record to successors that already exist.

## Project

The project name in the header opens a small panel with the project root, the detected agent, and three actions: initialize a new project, run validation now, and clear the agent runs. Local run state is stored under `dadr/runs/` and ignored by default because it may contain pasted context, source excerpts, and model transcripts.

Old links still work: `/records`, `/candidates/new`, `/tasks`, `/validation`, `/tags`, and `/settings` redirect to the Inbox, the Ledger, or the Draft page.

## Changing a decision

An accepted decision is never rewritten, not even to fix a typo. Its `id`, `status`, `title`, body, and `successors` are frozen, and so are those of a rejected record; only tags and front-matter keys outside DADR can change. DADRock refuses any other change to an accepted or rejected record, from the web editor or an agent revision, and suggests drafting a successor instead. Bodies are compared with normalized line endings. The `edit-record` agent task revises candidates only.

To change a decision, however slightly, draft a successor: a new candidate that supersedes the old record, its predecessor. Use **Edit**, **Revise with agent**, or **Supersede** on the record in the [Ledger](#ledger), run the `supersede-record` [task](llms.md#custom-workflows-and-runs), or write the candidate yourself. The successor names its predecessors in a `## Supersedes` section, followed by the reason for the change:

```markdown
## Supersedes

[[01938c20-a3b4-7000-8000-111122223333|Winsorize income at p99]]

Winsorizing at p99 still left implausible incomes in two waves.
```

The section starts with one or more lines, each exactly one record link by full UUID, `[[uuid]]` or `[[uuid|label]]`. The text after the links is the reason, in ordinary Markdown. A record has at most one `## Supersedes` section, and a section without a link is an error. When DADRock writes the section, from the web interface, a `supersede-record` result, or [MCP](llms.md#mcp), it is the first section of the body, each link is labeled with the predecessor's title, and the reason follows the links. The section stays in the successor's body after acceptance as the record of why the decision changed. It is a DADRock convention, not an additional required section of the DADR format.

The `supersede-record` task takes one or more accepted records and the change you want. The agent starts from the predecessor's exact text and changes only what the request requires, so the successor reads as a small diff; for several predecessors it combines them into one decision. It returns the reason separately, and DADRock writes it into `## Supersedes` when you save the proposal as a candidate.

### Accepting successors

Accepting a successor adds its `id` to each predecessor's `successors` and moves each predecessor to `dadr/archive/` with its body unchanged.

- **One-to-one:** the successor names one predecessor.
- **Merge:** one successor names several predecessors, and each gains the successor's `id`.
- **Split:** several successors name the same predecessor. Accept them together so that the predecessor gains all of their ids at once.

```sh
dadrock accept "Impute income" --with "Complete cases for the outcome"
```

`--with` repeats, and the positional words still join into one query. In general, accepting a set of candidates adds, for each predecessor named in the set, every candidate in the set that names it. Candidates without `## Supersedes` are simply accepted.

DADRock checks the whole set before writing anything: every candidate must meet the acceptance requirements, and every named predecessor must exist, be accepted, not already be superseded, and not be in the set. A predecessor that is already superseded is refused with the names of its current successors. If a write fails partway, every record is restored.

### Stale successors

Two candidates that name the same predecessor may be the halves of a split or competing alternatives; you decide by what you accept together. Once one of them is accepted alone, the other is stale: its predecessor was superseded after it was drafted, like a branch whose base has moved. `dadrock validate` warns about a stale candidate (`stale_supersedes`) and names the decisions now in force, and acceptance refuses it. Point its `## Supersedes` link at the current decision, or reject it. DADRock never retargets a link on its own; in the [Inbox](#inbox), **Retarget** on the candidate does it when you ask.

Validation also warns when an accepted record's `## Supersedes` section names a predecessor that does not list it in `successors` (`unlinked_supersedes`), for example after a record was accepted by hand.

For two records that are already accepted, `dadrock supersede --from OLD --to NEW` links them explicitly and moves `OLD` to `dadr/archive/`. Records linked this way have no `## Supersedes` section, so no reason is recorded.
