# Agent and Application Architecture

## Status

Future RFC. Existing Dadrock CLI, preview UI, MCP, ACP, and task-system behavior
belongs in `docs-src/` or generated references. This spec only records future
architecture constraints and product directions that are not yet core behavior.

## Purpose

Define boundaries for future application and agent surfaces without duplicating
existing command, route, or task behavior. Future work must preserve the
basic DADR model: Markdown records on disk are the source of truth, and
application layers provide workflows over those files rather than replacing
them.

## Invariants

Future application surfaces should preserve these constraints:

- Record files remain canonical; derived indexes, rendered views, task outputs,
  and agent transcripts are operational state.
- Lifecycle changes remain explicit human actions. Agents may propose them, but
  no agent tool accepts, rejects, or supersedes a record, and agents must not
  rewrite accepted records.
- Local-first operation remains the default. Networked, team, or hosted features
  must be opt-in and should not be required for ordinary project use.
- Task packages remain the preferred extension point for new agent-assisted
  workflows unless a dedicated command or API has a clear product reason.
- Future APIs should route mutations through the same command-layer semantics as
  CLI, MCP, editor integrations, and the preview UI.
- Operational state such as `dadr/runs/` may contain pasted context, source
  excerpts, and model transcripts. Future sharing, sync, retention, or export
  features must treat it as potentially sensitive.

## Future Surfaces

Future application work may include:

- a team review surface with reviewer assignment, notifications, and decision
  queues;
- hosted or shared-server operation for projects that opt in;
- role-based permissions for record creation, acceptance, rejection,
  supersession, and task execution;
- scheduled validation, link checks, inference tasks, and faithfulness reports;
- persistent dismissals or review state for recurring conflict and drift
  findings;
- richer editor integrations that open source references, surface diagnostics,
  and run Dadrock tasks without bypassing the local project model;
- code-tag extraction, link refresh, and source-range binding built on the
  documented code-tag convention;
- webhook, SSE, or similar notification channels for long-running tasks and team
  review events.

## Future API Constraints

If Dadrock grows beyond the localhost preview API, the external API should:

- expose command and task metadata rather than hard-coding UI assumptions;
- distinguish read-only, write, and destructive operations;
- make permission checks explicit for lifecycle mutations and agent/tool calls;
- carry enough provenance to audit which user, tool, or agent proposed a change;
- avoid storing raw source excerpts, model transcripts, or run artifacts in
  shared services unless the project explicitly enables that behavior;
- provide stable identifiers for records, task runs, diagnostics, and review
  events;
- keep generated or derived state rebuildable from records plus declared
  operational logs.

## Out of Scope

This RFC does not define existing CLI commands, web routes, or MCP tool names.
Those are implementation details and should be documented in `docs-src/` or
generated from the code.

This RFC also does not propose replacing task packages with a parallel plugin or
workflow system. New agent-assisted capabilities should start as tasks unless a
future design shows that tasks cannot support the required behavior.
