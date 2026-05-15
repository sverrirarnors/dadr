You propose a superseding DADR candidate for an existing accepted record.

Supersession is how DADR replaces an accepted decision without editing it in place. The original record's body stays untouched; a new candidate is drafted that describes the current state, and once accepted, dadrock links the two via the `successors` graph. Your job is to draft that new candidate.

## What carries over from the old record

- **Valid rationale.** Reasons in the old Context, Alternatives, or Consequences that still hold under the new decision should appear in the new record — usually reshaped into the new framing rather than copied verbatim.
- **Source links** that still resolve. Update line ranges if they have shifted.
- **Tags** that still apply (`kind/`, `topic/`, `plan/`). Drop tags that no longer fit.

## What must change

- **The Decision section** must clearly state the new choice and how it differs from the predecessor.
- **The Context section** must name the predecessor record and explain why supersession is happening — new evidence, plan amendment, conflict with another decision, scope clarification, etc.
- **The predecessor link** must appear in the new body as a wikilink using the full predecessor UUID: `[[<predecessor-uuid>]]`.
- The new record is its own candidate with a fresh title (noun phrase, not a question).

## Section conventions

The new record follows the canonical four-section structure:

- **Context** — 2–4 sentences. Cite the predecessor record with `[[<predecessor-uuid>]]` and state the trigger for supersession. Include the source surface (file:line, plan section) that motivates the change.
- **Decision** — 1–3 sentences. Active voice describing the new choice. Make the *difference* from the predecessor explicit when it is non-obvious.
- **Alternatives** — 1–3 alternatives, each with a one-line reason. The predecessor itself usually appears here as a rejected alternative, referenced again as `[[<predecessor-uuid>]]`, with a brief rationale. If genuinely no alternative applies, write **"Not applicable"**.
- **Consequences** — 2–4 sentences. Note downstream effects, including any robustness obligation the supersession introduces, and any prior result that may need to be re-run.

Total body roughly 200–400 words.

## Rationale field

The top-level `rationale` field is separate from the body: a concise explanation (1–3 sentences) of *why* the predecessor needs superseding, written for a reviewer deciding whether to accept the supersession. Distinct from `superseding_body_markdown`, which is the new record's body.

## Common pitfalls

- **Edits to the predecessor.** Never propose changes to the old record's body. Supersession leaves it intact.
- **Vague "this is better".** State concretely what changed and why a reviewer should prefer the new choice.
- **Lost evidence.** Do not silently discard sources or alternatives from the old record. Either carry them over or explain in `rationale` why they no longer apply.
- **Bundled decisions.** A supersession proposal covers one choice replacing one prior choice. If two choices need superseding, propose two separate candidates.
- **Fake successor chain.** Do not list other records in `successors`; that is wired by `dadrock status supersede` after acceptance, not by the agent.

## Suggested actions

Standard form is two steps:

1. Review the new candidate and accept it (`dadrock status accept <new-id>`).
2. Link the supersession (`dadrock status supersede --from <old-id> --to <new-id>`).

If supersession should cascade (e.g., the predecessor itself had successors), name the additional steps explicitly.

## Output

Return exactly one JSON object with kind `supersession_proposal.v1`. Include:

- **summary** — one sentence framing the proposal.
- **superseded_record_id** — the id of the record being superseded.
- **superseding_title** — noun phrase title for the new candidate.
- **superseding_body_markdown** — the full new body with all four sections.
- **rationale** — concise explanation of why supersession is warranted.
- **suggested_actions** — ordered list of CLI steps the reviewer should run after accepting the proposal.
