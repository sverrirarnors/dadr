# Web UI Vocabulary

Dadrock's web UI should feel dense, quiet, and utilitarian: closer to an editor or knowledge-base workbench than a marketing app. Prefer a small set of shared primitives before adding page-local visual patterns.

## Places

There are two places and one button. Everything else is a popover, a sheet, or a redirect.

- **Inbox** (`/inbox`, the home route): everything that needs a human decision. Candidates, agent proposals, proposed revisions, conflicts, reports, and validation issues are one grouped list; the selected item's detail sits beside it. Empty is the success state ("Inbox zero").
- **Ledger** (`/ledger`, `/ledger/<id>`): what has been decided. Accepted records by default; a "Show history" toggle reveals superseded and rejected records. One search box, a tag facet, and the record detail with its supersession chain.
- **Draft**: a header button that opens the composer sheet from anywhere. One text area plus attachments (files, folders, records). Where it was opened from and what is attached decide which built-in task runs (`$lib/draft/intent.ts`). Results land in the Inbox.
- **Project gear**: the project name in the header opens a popover with the root path, the agent, and the few project-level actions. There is no settings page.

Legacy routes (`/records`, `/candidates/*`, `/tasks`, `/validation`, `/tags`, `/settings`, `/audit`, `/links`) redirect through `$lib/navigation/legacy.ts`.

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
- `SegmentedTabs`: a small set of mutually exclusive views inside one detail, such as "Changes" and "Edit".
- `StatusPanel`: live task, validation, or operation status with optional expandable detail.
- `PanelSection`: repeated section panels inside a pane when the content is not large enough to be its own `WorkbenchPane`.
- `KeyValueGrid`: compact settings, environment, and diagnostic facts.
- `ListRow`: selectable rows in lists and history views.
- `StackedCard`: repeated result, diagnostic, or issue cards. Use the openable form for folded result stacks.
- `DisclosureRow`: expandable rows in stacked result output.
- `Alert`: errors, warnings, and important neutral notices.
- `EmptyState`: empty lists and no-result states.
- `Field`: labeled form controls when local layout does not need a custom label stack.
- `TextInput`, `SelectInput`, `TextareaInput`: standard form controls for the composer and inline editors.

## Patterns

- One visible button per verb. The redesign removed the dropdown menus; do not reintroduce `ActionMenu` for primary actions.
- Every item in the Inbox has at most three actions, named by what they do: Accept, Reject, Edit, Revise with agent; Save as candidate, Discard; Apply, Discard; Merge, Replace, Dismiss; Open record, Fix with agent, Dismiss.
- Agent work is a verb on a thing, never a destination. Opening the composer with a record attached is how "ask the agent" works; a run's log is a detail of the running item in the Inbox.
- Dismissed and consumed Inbox items are remembered per browser in `$lib/stores/resolved.svelte.ts`; clearing the agent history (project gear) forgets them.
- Record and candidate detail share `RecordDetailPanel` with a `surface` prop, because candidates are records filtered by status.
