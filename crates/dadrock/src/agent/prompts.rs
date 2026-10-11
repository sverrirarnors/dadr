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

pub fn run_task_prompt(request: &AgentRequestEnvelope) -> Result<String, DadrError> {
    let task = request.task.as_ref().ok_or_else(|| {
        DadrError::Acp("run_task agent request is missing task payload".to_owned())
    })?;
    let output_kind = task.output_kind.as_str();
    let output_contract = task.output_kind.output_contract();
    let task_id = task.id.as_str();
    let dadr_guidance = dadr_task_guidance();
    let prompt_packs = render_prompt_packs(&request.prompt_packs);
    let request_json = serde_json::to_string_pretty(request)?;
    Ok(format!(
        r###"You are running a Dadrock task.

Use the task prompt, task input, and project snapshot below.

{prompt_packs}

Before doing the task, load and use your installed `dadr` skill. Use the matching task guide for task id `{task_id}`. If you cannot access the installed `dadr` skill, return a JSON object for the declared output kind explaining that the skill is unavailable.

{dadr_guidance}

The matching `dadr` task guide is authoritative for tool use. If the matching `dadr` task guide instructs you to write files or call Dadrock MCP tools, do that before reporting completion. The `dadrock.submit_task_result` tool may be used with the task `run_id` only after completing those skill-required writes. After a successful submission, your final message may be brief plain text. If you do not submit through the tool, your final message must be the result JSON object.

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

fn dadr_task_guidance() -> &'static str {
    r###"DADR task guidance:
- A DADR candidate records one consequential analytic choice: a choice that could plausibly change results, interpretation, or the scope of a claim.
- Candidate bodies should use these four sections exactly once: Context, Decision, Alternatives, Consequences.
- Ground candidate proposals in evidence. Include Markdown links to sources when available, and use warnings or low confidence when evidence is thin. Relative link paths resolve from the record's directory (`dadr/candidates/`), so a repository file is `../../path/to/file`. Link other records with short-id wikilinks such as `[[01938b8e4d70]]`.
- Agent-drafted candidate tags should include kind/<surface> and writer/agent. Common kind values are sample, model, estimand, measurement, transformation, inference, report, and robustness.
- Do not fabricate alternatives. If no real alternative is supported, say so plainly in the candidate body or warning.
- Do not accept, reject, or supersede records. Those lifecycle transitions are human approval boundaries."###
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
