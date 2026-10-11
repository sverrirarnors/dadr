use crate::{
    agent::{
        skills::render_prompt_packs,
        types::{AgentDraftRequirements, AgentIntent, AgentRequestEnvelope},
    },
    error::DadrError,
    tasks::TaskOutputKind,
};

pub fn agent_prompt(request: &AgentRequestEnvelope) -> Result<String, DadrError> {
    match request.intent {
        AgentIntent::DraftCandidate => draft_candidate_prompt(request),
        AgentIntent::DiscussCandidate => discuss_candidate_prompt(request),
        AgentIntent::ReviseCandidate => revise_candidate_prompt(request),
        AgentIntent::RunTask => run_task_prompt(request),
        other => Err(DadrError::Acp(format!(
            "agent intent `{}` is not supported by the ACP prompt runner",
            other.as_str()
        ))),
    }
}

pub fn draft_candidate_prompt(request: &AgentRequestEnvelope) -> Result<String, DadrError> {
    let request_json = serde_json::to_string_pretty(request)?;
    let contract = candidate_contract_text();
    let schema = candidate_response_schema("draft_candidate", "one sentence summary");
    let prompt_packs = render_prompt_packs(&request.prompt_packs);
    Ok(format!(
        r###"You are helping draft a DADR candidate decision record.

Use the user's prompt and project snapshot below. Return exactly one JSON object and no prose outside it.

{prompt_packs}

{contract}

{schema}

Dadrock draft request:
```json
{request_json}
```
"###
    ))
}

pub fn discuss_candidate_prompt(request: &AgentRequestEnvelope) -> Result<String, DadrError> {
    let request_json = serde_json::to_string_pretty(request)?;
    let prompt_packs = render_prompt_packs(&request.prompt_packs);
    Ok(format!(
        r###"You are helping discuss a DADR candidate decision record.

Answer the user's question or request in plain text. Do not return JSON. Do not return a revised candidate body. Do not edit files or ask to edit files. If the user asks for a textual change, explain what you would change and tell them to request a revision proposal from Dadrock.

{prompt_packs}

Use the current candidate markdown, chat history, and project snapshot below for context.

Dadrock discussion request:
```json
{request_json}
```
"###
    ))
}

pub fn revise_candidate_prompt(request: &AgentRequestEnvelope) -> Result<String, DadrError> {
    let request_json = serde_json::to_string_pretty(request)?;
    let contract = candidate_contract_text();
    let schema =
        candidate_response_schema("revise_candidate", "one sentence summary of the revision");
    let prompt_packs = render_prompt_packs(&request.prompt_packs);
    Ok(format!(
        r###"You are helping revise an existing DADR candidate decision record.

Use the user's requested change, the current candidate markdown, any chat history, and the project snapshot below. Return exactly one JSON object and no prose outside it.

Return the complete revised candidate body in `body_markdown`, not a patch. Preserve the decision-record shape and keep the body concise enough for human review.

{prompt_packs}

{contract}

Do not revise accepted records.

{schema}

Dadrock revision request:
```json
{request_json}
```
"###
    ))
}

/// The `dadr` skill's shared record anatomy reference. Task guides link to it
/// as `../record.md`.
const SKILL_RECORD_REFERENCE: &str = include_str!("../../../../skills/dadr/record.md");

/// The `dadr` skill's shared tag reference. Task guides link to it as
/// `../tags.md`.
const SKILL_TAG_REFERENCE: &str = include_str!("../../../../skills/dadr/tags.md");

pub fn run_task_prompt(request: &AgentRequestEnvelope) -> Result<String, DadrError> {
    let task = request.task.as_ref().ok_or_else(|| {
        DadrError::Acp("run_task agent request is missing task payload".to_owned())
    })?;
    let output_kind = task.output_kind.as_str();
    let output_contract = task.output_kind.output_contract();
    let task_id = task.id.as_str();
    let record_reference = SKILL_RECORD_REFERENCE.trim();
    let tag_reference = SKILL_TAG_REFERENCE.trim();
    let prompt_packs = render_prompt_packs(&request.prompt_packs);
    let request_json = serde_json::to_string_pretty(request)?;
    Ok(format!(
        r###"You are running a Dadrock task.

The task guide for task id `{task_id}` is the `task.prompt` field of the Dadrock task request at the end. Use it with the task input, context, and project snapshot in that request.

{prompt_packs}

Task mode rules:
- The task guide says WHAT to produce; dadrock decides how it is delivered.
- Do not create, edit, move, accept, reject, or supersede records or other files. Return everything as the result object; dadrock turns it into proposals that a human reviews.
- Where the task guide mentions writing or editing records, put that content in the result instead.

Links in the task guide to `../record.md` and `../tags.md` refer to the "Record anatomy" and "Tags" sections below.

## Record anatomy

{record_reference}

## Tags

{tag_reference}

## Result

The `dadrock.submit_task_result` tool may be used with the task `run_id` when available. After a successful submission, your final message may be brief plain text. If you do not submit through the tool, your final message must be the result JSON object.

Any result JSON object must include `"kind": "{output_kind}"`. Do not return a JSON array, string, null, markdown fence, or explanatory text outside the JSON object.

The JSON object must match this output contract:
```json
{output_contract}
```

Dadrock task request:
```json
{request_json}
```
"###
    ))
}

pub fn task_output_repair_prompt(
    output_kind: &TaskOutputKind,
    validation_error: &str,
    raw_output: &serde_json::Value,
) -> Result<String, DadrError> {
    let output_contract = output_kind.output_contract();
    let raw_output = serde_json::to_string_pretty(raw_output)?;
    Ok(format!(
        r###"Your previous Dadrock task response did not match the required output schema.

Return exactly one corrected JSON object and no prose outside it. Preserve the useful content from the previous response, but change field types and missing fields so validation passes.

Validation error:
{validation_error}

The corrected JSON object must match this output contract:
```json
{output_contract}
```

Previous invalid JSON:
```json
{raw_output}
```
"###
    ))
}

fn candidate_contract_text() -> String {
    let requirements = AgentDraftRequirements::default();
    format!(
        "Dadrock owns frontmatter fields {} when writing the file. Never put dates or times in frontmatter (no keys such as {}, and no values like 2026-05-07); a record's time comes only from its UUIDv7 id. The record status must remain `{}` and the body must include these sections: {}. Do not mark the decision accepted.",
        requirements.dadrock_owned_frontmatter.join(", "),
        requirements.forbidden_frontmatter.join(", "),
        requirements.status,
        requirements.required_sections.join(", "),
    )
}

fn candidate_response_schema(intent: &str, summary: &str) -> String {
    format!(
        r###"If your ACP runtime can emit a completed tool call with structured `rawOutput`, put this JSON object in that `rawOutput` and use a short human-facing text summary. Otherwise return this JSON object as the final text response.

The JSON object must have this shape:
{{
  "kind": "dadrock.draft_candidate.v1",
  "intent": "{intent}",
  "confidence": "low|medium|high",
  "summary": "{summary}",
  "frontmatter": {{
    "title": "candidate title",
    "tags": ["kind/governance"]
  }},
  "body_markdown": "## Context\n...\n\n## Decision\n...\n\n## Alternatives\n...\n\n## Consequences\n...",
  "source_references": [],
  "activity_observations": [],
  "warnings": [],
  "provenance": {{
    "model": "model or agent name if known",
    "tool_runs": []
  }}
}}"###
    )
}
