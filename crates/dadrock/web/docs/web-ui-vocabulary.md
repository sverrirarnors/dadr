# Web UI Vocabulary

Dadrock's web UI should feel dense, quiet, and utilitarian: closer to an editor or knowledge-base workbench than a marketing app. Prefer a small set of shared primitives before adding page-local visual patterns.

## Places

There are three places, all in the header with an icon before the word. Everything else is a popover or a redirect.

- **Inbox** (`/inbox`, the home route): everything that needs a human decision. Candidates, agent proposals, proposed revisions, conflicts, reports, and validation issues are one grouped list; the selected item's detail sits beside it. Empty is the success state ("Inbox zero").
  - A successor candidate (its `## Supersedes` names predecessors) opens on a **Changes** tab: the reason, then the diff against the predecessor (`RecordDiffView`: title change, tag summary, `LineDiff` of the body without `## Supersedes`). A merge opens on the section view (`SectionCompare`, Decision first) with a "Compare with" selector. The other tab is **Document**.
  - Candidates that supersede the same current decision are one **split** item ("Split “X” into N decisions", `SplitDetail`), with a checkbox, reason, and diff per candidate and one **Accept selected** (`acceptCandidate(id, withIds)`). `inboxUrl(candidateItemId(id))` still reaches a candidate inside a split. Grouping lives in `$lib/inbox/items.ts`.
  - A stale successor (its predecessor was superseded since) shows a warning with **Retarget**, which rewrites the link in `## Supersedes` (`$lib/records/retarget.ts`) and saves the candidate; stale candidates are never grouped into a split.
- **Ledger** (`/ledger`, `/ledger/<id>`): what has been decided. Current decisions (`dadr/ledger/`) by default; a "Show archived" toggle (`?archived=1`) adds the archived records (`dadr/archive/`: superseded and rejected). One search box, a tag facet, and the record detail with **Document** and **History** tabs (`?tab=history` opens the second).
  - **History** (`RecordHistory`, laid out by `$lib/history/layout.ts`): the versions of one decision newest first, the opened one highlighted, current ones labeled; lanes join at a split and fork at a merge; each change shows its reason and an expandable diff; neighbors are one line ("Also became …", "Merged with …"). Under each version, "Edited 2 times since accepted" (or "since rejected") expands to the text edits Git recorded since then, newest first: date, author, and commit subject, or "Not committed yet" for the working tree, each with its own expandable `RecordDiffView` (`$lib/history/edits.ts`). Nothing is shown when `edits` is empty or null (outside Git).
  - **Edit** works in place on every record with the raw Markdown editor: candidates freely; accepted, superseded, and rejected records for corrections, with a note that corrections stay in the record and show in History from Git, and that changing the decision means **Supersede**. Tags are edited with the rest of the front matter; there is no separate tag editor. When the opened version has text edits, an "Edited since acceptance" (or "since rejection") marker next to the file name opens History.
  - **Supersede** (decisions in force only) opens the Draft page's successor form; **Revise with agent** runs `supersede-record` on a decision in force and `edit-record` on a candidate (`detailActions` and `successorDraft` in `$lib/records/detail-state.ts`). Archived records have Edit but no Supersede or Revise; a superseded one says "Superseded by …".
- **Draft** (`/draft`): a full page, reached from the header or from a record. One text area plus attachments (files, folders, records). Where it was opened from and what is attached decide which built-in task runs (`$lib/draft/intent.ts`). Results land in the Inbox. Pages open it with `composer.show(prefill)`, which stores the prefill and navigates; `prefill.view` picks the agent prompt or the form written by hand.
  - Supersede mode takes one or more decisions in force (several make a merge) and sends them all as `record_ids` to `supersede-record`. Written by hand (`DraftSuccessorForm`, where **Supersede** lands), the successor starts from the predecessor's exact title, tags, and body (a merge starts from every predecessor's sections) and asks "Why does this decision change?", which is optional; saving calls `createCandidate` with `supersedes` and, when given, `supersedes_reason`, and lands on the candidate's Changes tab. Its title reads "Supersede “X”".
- **Project gear**: the project name in the header opens a popover with the root path, the agent, and the few project-level actions (Validate now, Clear agent runs). There is no settings page.

The places follow the project folders: candidates in the Inbox live in `dadr/inbox/`, and the Ledger lists `dadr/ledger/` and, on request, `dadr/archive/`.

Legacy routes (`/records`, `/candidates/*`, `/tasks`, `/validation`, `/tags`, `/settings`, `/audit`, `/links`) redirect through `$lib/navigation/legacy.ts`.

## Words

- **Supersede** is the only verb for one record taking the place of another: "Supersede “X”", "Supersedes “X”", "Superseded by". Never "replace" or "replacement".
- **Successor** and **predecessor** name the newer and older record of a supersession, including a successor that is still a candidate.
- **Archived** means rejected or superseded: kept, but not in force.
- **History** means the versions of one decision and the text edits to each. Do not use it for the archive or for agent runs, which are "runs".
- **Edit** always changes a record in place: a candidate freely, an accepted, superseded, or rejected record for corrections. **Supersede** changes a decision by drafting its successor. Archived records are corrected, never superseded or revised.
- **Edited since acceptance** marks a record whose text changed after it was accepted (or rejected); **Not committed yet** is an edit in the working tree.
- **Changes** is the diff a successor makes against its predecessor; **Retarget** points a stale successor at the decision now in force.

## Page Structure

- `WorkbenchPage`: top-level workspace layout. Use this for primary app pages that arrange panes, lists, detail views, or tools.
- `WorkbenchPane`: bordered, scrollable pane inside a workbench layout. Use it for major regions, not for every small subsection.
- `ResizableWorkbenchSplit`: the list-plus-detail split used by both the Inbox and the Ledger. It stacks below the `md` breakpoint.
- `PaneHeader`: pane title and subtitle. Keep it terse and factual.
- `PaneToolbar`: compact controls attached to a pane header or list.
- `DocumentPane`: rendered document or markdown content.
- `InspectorPane`: secondary read-only details or supporting metadata.
- `ActionBar`: clustered record or item actions.
- `MetadataStrip`: small metadata rows under titles, filenames, and record identifiers.

## UI Primitives

- `Button`: commands and actions. Use icons from `lucide-svelte` when the action is familiar.
- `Badge`: status, source, kind, count, or compact metadata.
- `InlineLinkBadge`: badge-like links, especially tags and other compact metadata links.
- `SegmentedTabs`: a small set of mutually exclusive views inside one detail, such as "Changes" and "Document", or "Document" and "History".
- `StatusPanel`: live task, validation, or operation status with optional expandable detail.
- `PanelSection`: repeated section panels inside a pane when the content is not large enough to be its own `WorkbenchPane`.
- `KeyValueGrid`: compact settings, environment, and diagnostic facts.
- `ListRow`: selectable rows in lists.
- `StackedCard`: repeated result, diagnostic, or issue cards. Use the openable form for folded result stacks.
- `DisclosureRow`: expandable rows in stacked result output.
- `Alert`: errors, warnings, and important neutral notices.
- `EmptyState`: empty lists and no-result states.
- `Field`: labeled form controls when local layout does not need a custom label stack.
- `TextInput`, `SelectInput`, `TextareaInput`: standard form controls for the composer and short fields.
- `MarkdownEditor` (`$lib/components/editor`): the CodeMirror editor for every Markdown body (record source, candidate drafts, successor drafts, revisions). It binds `value`, takes an `ariaLabel`, and exposes the view on its host as `cmView` for browser tests.
- `RecordDiffView` (`$lib/components/records`): one record diff from the API: title change, tags added and removed, and the body as a `LineDiff`.

## Patterns

- One visible button per verb. The redesign removed the dropdown menus; do not reintroduce `ActionMenu` for primary actions.
- Every item in the Inbox has at most three actions, named by what they do: Accept, Reject, Edit, Revise with agent; Accept selected (a split, with Edit and Reject per candidate); Save as candidate, Discard; Apply, Discard; Merge, Supersede, Dismiss; Open record, Fix with agent, Dismiss.
- Agent work is a verb on a thing, never a destination. Opening the composer with a record attached is how "ask the agent" works; a run's log is a detail of the running item in the Inbox.
- Dismissed and consumed Inbox items are remembered per browser in `$lib/stores/resolved.svelte.ts`; Clear agent runs (project gear) deletes `dadr/runs/` and forgets them.
- Record and candidate detail share `RecordDetailPanel` with a `surface` prop, because candidates are records filtered by status. Which verbs it shows, what Revise with agent runs, and whether the editor shows the note on corrections comes from `detailActions` in `$lib/records/detail-state.ts`.
- Queries keyed by a record id read the id from their `queryKey`, not from a prop: a refetch can run after the Inbox has moved on to the next item.
- The mock API (`$lib/api/mock.ts`, records in `mock-records.ts`) follows Dadrock's rules: history, diffs, set acceptance, and corrections in place (`id`, `status`, and `successors` fixed; accepted records keep their four sections). Each record's text at HEAD and its commits stand in for Git, so histories list text edits, and a save shows as "Not committed yet". Browser tests opt into a project with a split, a merge, a stale successor, a rejected record, and an accepted record with a committed and an uncommitted edit with `localStorage['dadrock:mock-scenario'] = 'history'`; `localStorage['dadrock:mock-git'] = 'false'` simulates a project outside Git, where `edits` is null.
