# Web UI Vocabulary

Dadrock's web UI should feel dense, quiet, and utilitarian: closer to an editor or knowledge-base workbench than a marketing app. Prefer a small set of shared primitives before adding page-local visual patterns.

## Page Structure

- `WorkbenchPage`: top-level workspace layout. Use this for primary app pages that arrange panes, lists, detail views, or tools.
- `WorkbenchPane`: bordered, scrollable pane inside a workbench layout. Use it for major regions, not for every small subsection.
- `PaneHeader`: pane title and subtitle. Keep it terse and factual.
- `PaneToolbar`: compact controls attached to a pane header or list.
- `DocumentPane`: rendered document or markdown content.
- `InspectorPane`: secondary read-only details or supporting metadata.
- `ActionBar`: clustered record or task actions.
- `MetadataStrip`: small metadata rows under titles, filenames, and record identifiers.

## UI Primitives

- `Button`: commands and actions. Use icons from `lucide-svelte` when the action is familiar.
- `Badge`: status, source, kind, count, or compact metadata.
- `InlineLinkBadge`: badge-like links, especially tags and other compact metadata links.
- `SegmentedTabs`: a small set of mutually exclusive page sections or modes.
- `StatusPanel`: live task, validation, or operation status with optional expandable detail.
- `PanelSection`: repeated section panels inside a pane when the content is not large enough to be its own `WorkbenchPane`.
- `KeyValueGrid`: compact settings, environment, and diagnostic facts.
- `ListRow`: selectable rows in lists and history views.
- `StackedCard`: repeated result, diagnostic, or issue cards. Use the openable form for folded result stacks.
- `DisclosureRow`: expandable rows in stacked result output.
- `Alert`: errors, warnings, and important neutral notices.
- `EmptyState`: empty lists and no-result states.
- `Field`: labeled form controls when local layout does not need a custom label stack.
- `TextInput`, `SelectInput`, `TextareaInput`: standard form controls for route-level forms and task inputs.

## Patterns

Use panes for page-level structure, panel sections for repeated subsections, rows for lists, badges for metadata, tabs for mode changes, and status panels for running work. Avoid inventing new rounded-card recipes when one of these primitives already describes the interaction.

Task results should stay as folded stacked cards. Agent output belongs inside the running task status panel, so it can expand while the task is still active. Large task route regions should live in task-owned components while the route keeps state, mutations, and navigation.

Record and candidate detail should share the same visual language because candidates are records filtered by status. Record-scoped task actions should route through `/tasks` with query context rather than running inline inside the record detail view.
