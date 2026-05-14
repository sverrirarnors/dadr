use std::fs;

use serde::{Deserialize, Serialize};

use crate::{
    agent::{
        acp::{run_acp_agent, AcpPermissionPolicy, AcpPromptOptions},
        providers::LocalAcpAgentCommand,
        response::{
            candidate_metadata_from_response, validate_draft_response, validate_revision_response,
        },
        skills::{
            combined_prompt_pack_sha256, list_prompt_packs, resolve_prompt_packs,
            AgentPromptPackInfo,
        },
        types::{
            AgentCandidateSnapshot, AgentChatMessage, AgentDraftResult, AgentIntent,
            AgentProjectSnapshot, AgentProvenance, AgentRequestEnvelope, AgentResponseEnvelope,
            AgentRevisionResult, Confidence,
        },
    },
    error::DadrError,
    index::ProjectIndex,
    mcp::DadrockMcpAccess,
    records::{
        create_candidate_with_body, preview_candidate_revision, update_candidate_with_body,
        ScannedRecord, ScannedRecordSummary,
    },
    RecordStatus,
};

use super::{diff::text_diff_lines, CommandContext, CommandEnvelope, TextDiffLine};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDraftCommandRequest {
    pub prompt: String,
    pub local_agent: LocalAcpAgentCommand,
    pub permission_policy: AcpPermissionPolicy,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prompt_packs: Vec<String>,
    #[serde(default = "default_enable_dadrock_mcp")]
    pub enable_dadrock_mcp: bool,
    pub write: bool,
}

fn default_enable_dadrock_mcp() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillsResult {
    pub skills: Vec<AgentPromptPackInfo>,
}

pub fn list_agent_skills(
    context: &CommandContext,
) -> Result<CommandEnvelope<AgentSkillsResult>, DadrError> {
    let skills = list_prompt_packs(&context.cwd)?;
    Ok(CommandEnvelope::new(
        "agent skills",
        AgentSkillsResult { skills },
    ))
}

pub fn draft_with_acp_agent(
    context: &CommandContext,
    request: AgentDraftCommandRequest,
) -> Result<CommandEnvelope<AgentDraftResult>, DadrError> {
    if request.prompt.trim().is_empty() {
        return Err(DadrError::InvalidInput(
            "agent draft requires a prompt".to_owned(),
        ));
    }

    let (project, index) = context.initialized_index()?;
    let records = index.list_records(None);
    let prompt_packs = resolve_prompt_packs(&request.prompt_packs, &project.cwd)?;
    let prompt_pack_sha256 = combined_prompt_pack_sha256(&prompt_packs);
    let agent_request = AgentRequestEnvelope::draft_candidate(
        request.prompt,
        AgentProjectSnapshot::new(&project, records),
    )?
    .with_prompt_packs(prompt_packs);
    let mut response = run_acp_agent(
        &request.local_agent,
        &agent_request,
        AcpPromptOptions {
            permission_policy: request.permission_policy,
            enable_dadrock_mcp: request.enable_dadrock_mcp,
            dadrock_mcp_access: DadrockMcpAccess::ReadOnly,
            ..AcpPromptOptions::default()
        },
    )?;
    if let Some(prompt_pack_sha256) = prompt_pack_sha256 {
        response.provenance.prompt_pack_sha256 = Some(prompt_pack_sha256);
    }
    validate_draft_response(&response)?;

    let mut changed_paths = Vec::new();
    let candidate = if request.write {
        let metadata = candidate_metadata_from_response(&mut response)?;
        let result = create_candidate_with_body(
            &project,
            &metadata.title,
            &metadata.tags,
            &response.body_markdown,
        )?;
        changed_paths.push(result.path.clone());
        Some(result)
    } else {
        None
    };

    Ok(CommandEnvelope::new(
        "agent draft",
        AgentDraftResult {
            response,
            candidate,
        },
    )
    .with_changed_paths(changed_paths))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentReviseCommandRequest {
    pub query: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chat_history: Vec<AgentChatMessage>,
    pub local_agent: LocalAcpAgentCommand,
    pub permission_policy: AcpPermissionPolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRevisionProposal {
    pub response: AgentResponseEnvelope,
    pub candidate: ScannedRecordSummary,
    pub proposed_title: String,
    pub proposed_tags: Vec<String>,
    pub proposed_body_markdown: String,
    pub diff: Vec<TextDiffLine>,
    pub changed: bool,
}

pub fn revise_candidate_with_acp_agent(
    context: &CommandContext,
    request: AgentReviseCommandRequest,
) -> Result<CommandEnvelope<AgentRevisionProposal>, DadrError> {
    if request.prompt.trim().is_empty() {
        return Err(DadrError::InvalidInput(
            "agent revise requires a prompt".to_owned(),
        ));
    }

    let (project, index) = context.initialized_index()?;
    let found = index.resolve_one(&request.query)?.record;
    let scanned = indexed_scanned_record(&index, &found)?;
    if !scanned.is_valid() {
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "`{}` has validation errors and cannot be revised",
            found.path
        )));
    }

    let record = scanned
        .record
        .as_ref()
        .ok_or_else(|| DadrError::ProjectInvariant(format!("failed to parse `{}`", found.path)))?;
    if record.front_matter.status != RecordStatus::Candidate {
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "agent revise requires status `candidate`, found `{}`",
            record.front_matter.status
        )));
    }

    let markdown = fs::read_to_string(found.path.as_std_path())?;
    let records = index.list_records(None);
    let agent_request = AgentRequestEnvelope::revise_candidate(
        request.prompt,
        AgentProjectSnapshot::new(&project, records),
        AgentCandidateSnapshot::new(
            found.id.to_string(),
            found.title.clone(),
            found.status.as_str().to_owned(),
            found.path.clone(),
            markdown,
        )?,
        request.chat_history,
    )?;
    let response = run_acp_agent(
        &request.local_agent,
        &agent_request,
        AcpPromptOptions {
            permission_policy: request.permission_policy,
            dadrock_mcp_access: DadrockMcpAccess::ReadOnly,
            ..AcpPromptOptions::default()
        },
    )?;
    revision_proposal_from_response(&index, &found.id.to_string(), response)
}

pub fn revision_proposal_from_response(
    index: &ProjectIndex,
    query: &str,
    mut response: AgentResponseEnvelope,
) -> Result<CommandEnvelope<AgentRevisionProposal>, DadrError> {
    validate_revision_response(&response)?;
    let found = index.resolve_one(query)?.record;
    let scanned = indexed_scanned_record(index, &found)?;
    if !scanned.is_valid() {
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "`{}` has validation errors and cannot be revised",
            found.path
        )));
    }
    let record = scanned
        .record
        .as_ref()
        .ok_or_else(|| DadrError::ProjectInvariant(format!("failed to parse `{}`", found.path)))?;
    if record.front_matter.status != RecordStatus::Candidate {
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "agent revise requires status `candidate`, found `{}`",
            record.front_matter.status
        )));
    }

    let metadata = candidate_metadata_from_response(&mut response)?;
    let preview = preview_candidate_revision(
        index,
        query,
        &metadata.title,
        &metadata.tags,
        &response.body_markdown,
    )?;
    let diff = text_diff_lines(&preview.current_content, &preview.proposed_content);
    let changed = preview.current_content != preview.proposed_content;

    Ok(CommandEnvelope::new(
        "agent revise",
        AgentRevisionProposal {
            response: response.clone(),
            candidate: found,
            proposed_title: metadata.title,
            proposed_tags: metadata.tags,
            proposed_body_markdown: response.body_markdown,
            diff,
            changed,
        },
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyCandidateRevisionRequest {
    pub query: String,
    pub title: String,
    pub tags: Vec<String>,
    pub body_markdown: String,
}

pub fn apply_candidate_revision(
    context: &CommandContext,
    request: ApplyCandidateRevisionRequest,
) -> Result<CommandEnvelope<AgentRevisionResult>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let candidate = update_candidate_with_body(
        &index,
        &request.query,
        &request.title,
        &request.tags,
        &request.body_markdown,
    )?;
    let changed_paths = vec![candidate.path.clone()];
    Ok(CommandEnvelope::new(
        "agent revise apply",
        AgentRevisionResult {
            response: AgentResponseEnvelope {
                intent: AgentIntent::ReviseCandidate,
                confidence: Confidence::Medium,
                summary: "Applied agent revision.".to_owned(),
                frontmatter: serde_json::json!({
                    "title": request.title,
                    "tags": request.tags,
                }),
                body_markdown: request.body_markdown,
                source_references: Vec::new(),
                activity_observations: Vec::new(),
                warnings: Vec::new(),
                provenance: AgentProvenance::default(),
            },
            candidate,
        },
    )
    .with_changed_paths(changed_paths))
}

fn indexed_scanned_record<'a>(
    index: &'a ProjectIndex,
    summary: &ScannedRecordSummary,
) -> Result<&'a ScannedRecord, DadrError> {
    index
        .scanned_records()
        .iter()
        .find(|scanned| scanned.path == summary.path)
        .ok_or_else(|| {
            DadrError::ProjectInvariant(format!("indexed record `{}` is missing", summary.path))
        })
}
