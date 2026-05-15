use camino::Utf8PathBuf;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use crate::{
    agent::skills::AgentPromptPack,
    project::ProjectContext,
    records::{NewRecordResult, ScannedRecordSummary, UpdateCandidateResult},
    tasks::{TaskContext, TaskOutputKind},
};

pub const DADROCK_DRAFT_PROTOCOL: &str = "dadrock-agent-draft";
pub const DADROCK_DRAFT_VERSION: u32 = 1;
pub const AGENT_PROJECT_RECORD_LIMIT: usize = 200;
pub const AGENT_PROJECT_RECORD_BYTE_LIMIT: usize = 128 * 1024;
pub const AGENT_USER_PROMPT_BYTE_LIMIT: usize = 32 * 1024;
pub const AGENT_CANDIDATE_MARKDOWN_BYTE_LIMIT: usize = 256 * 1024;
pub const AGENT_CHAT_HISTORY_LIMIT: usize = 24;
pub const AGENT_CHAT_MESSAGE_BYTE_LIMIT: usize = 8 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentIntent {
    ScanDiff,
    BootstrapRepo,
    DraftCandidate,
    DiscussCandidate,
    ReviseCandidate,
    ClassifyActivities,
    InferDecisions,
    AnalyzeFaithfulness,
    CaptureState,
    ExtractCodeTags,
    RefreshLinks,
    ProposeSplit,
    ProposeMerge,
    ProposeSupersession,
    RunTask,
}

impl AgentIntent {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ScanDiff => "scan_diff",
            Self::BootstrapRepo => "bootstrap_repo",
            Self::DraftCandidate => "draft_candidate",
            Self::DiscussCandidate => "discuss_candidate",
            Self::ReviseCandidate => "revise_candidate",
            Self::ClassifyActivities => "classify_activities",
            Self::InferDecisions => "infer_decisions",
            Self::AnalyzeFaithfulness => "analyze_faithfulness",
            Self::CaptureState => "capture_state",
            Self::ExtractCodeTags => "extract_code_tags",
            Self::RefreshLinks => "refresh_links",
            Self::ProposeSplit => "propose_split",
            Self::ProposeMerge => "propose_merge",
            Self::ProposeSupersession => "propose_supersession",
            Self::RunTask => "run_task",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn as_tag(self) -> &'static str {
        match self {
            Self::Low => "confidence/low",
            Self::Medium => "confidence/medium",
            Self::High => "confidence/high",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReference {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excerpt: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityObservation {
    pub activity: String,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

impl ActivityObservation {
    fn from_summary(summary: String) -> Self {
        Self {
            activity: "observation".to_owned(),
            summary,
            path: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProvenance {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_pack_sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "deserialize_agent_tool_runs"
    )]
    pub tool_runs: Vec<AgentToolRun>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentToolRun {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

impl AgentToolRun {
    fn from_name(name: String) -> Self {
        Self {
            name,
            summary: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentResponseEnvelope {
    pub intent: AgentIntent,
    pub confidence: Confidence,
    pub summary: String,
    #[serde(default)]
    pub frontmatter: Value,
    pub body_markdown: String,
    #[serde(default)]
    pub source_references: Vec<SourceReference>,
    #[serde(default, deserialize_with = "deserialize_activity_observations")]
    pub activity_observations: Vec<ActivityObservation>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub provenance: AgentProvenance,
}

fn deserialize_activity_observations<'de, D>(
    deserializer: D,
) -> Result<Vec<ActivityObservation>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ActivityObservationValue {
        Structured(ActivityObservation),
        Summary(String),
    }

    Vec::<ActivityObservationValue>::deserialize(deserializer).map(|observations| {
        observations
            .into_iter()
            .map(|observation| match observation {
                ActivityObservationValue::Structured(observation) => observation,
                ActivityObservationValue::Summary(summary) => {
                    ActivityObservation::from_summary(summary)
                }
            })
            .collect()
    })
}

fn deserialize_agent_tool_runs<'de, D>(deserializer: D) -> Result<Vec<AgentToolRun>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AgentToolRunValue {
        Structured(AgentToolRun),
        Name(String),
    }

    Vec::<AgentToolRunValue>::deserialize(deserializer).map(|tool_runs| {
        tool_runs
            .into_iter()
            .map(|tool_run| match tool_run {
                AgentToolRunValue::Structured(tool_run) => tool_run,
                AgentToolRunValue::Name(name) => AgentToolRun::from_name(name),
            })
            .collect()
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRecordSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub path: Utf8PathBuf,
}

impl From<ScannedRecordSummary> for AgentRecordSummary {
    fn from(record: ScannedRecordSummary) -> Self {
        Self {
            id: record.id.to_string(),
            title: record.title,
            status: record.status.as_str().to_owned(),
            path: record.path,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProjectSnapshot {
    pub cwd: Utf8PathBuf,
    pub decisions_dir: Utf8PathBuf,
    pub initialized: bool,
    pub total_records: usize,
    pub omitted_records: usize,
    pub records: Vec<AgentRecordSummary>,
}

impl AgentProjectSnapshot {
    pub fn new(project: &ProjectContext, records: Vec<ScannedRecordSummary>) -> Self {
        let total_records = records.len();
        let records = bounded_project_records(records);
        let omitted_records = total_records.saturating_sub(records.len());
        Self {
            cwd: project.cwd.clone(),
            decisions_dir: project.decisions_dir.clone(),
            initialized: project.initialized,
            total_records,
            omitted_records,
            records: records.into_iter().map(AgentRecordSummary::from).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDraftRequirements {
    pub status: String,
    pub required_sections: Vec<String>,
    pub dadrock_owned_frontmatter: Vec<String>,
    pub forbidden_frontmatter: Vec<String>,
}

impl Default for AgentDraftRequirements {
    fn default() -> Self {
        Self {
            status: "candidate".to_owned(),
            required_sections: vec![
                "Context".to_owned(),
                "Decision".to_owned(),
                "Alternatives".to_owned(),
                "Consequences".to_owned(),
            ],
            dadrock_owned_frontmatter: vec![
                "id".to_owned(),
                "status".to_owned(),
                "title".to_owned(),
                "tags".to_owned(),
            ],
            forbidden_frontmatter: vec![
                "date".to_owned(),
                "created".to_owned(),
                "accepted_at".to_owned(),
                "superseded_at".to_owned(),
            ],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentChatRole {
    User,
    Agent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentChatMessage {
    pub role: AgentChatRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCandidateSnapshot {
    pub id: String,
    pub title: String,
    pub status: String,
    pub path: Utf8PathBuf,
    pub markdown: String,
    pub markdown_bytes: usize,
}

impl AgentCandidateSnapshot {
    pub fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        status: impl Into<String>,
        path: Utf8PathBuf,
        markdown: String,
    ) -> Result<Self, crate::error::DadrError> {
        let markdown_bytes = markdown.len();
        if markdown_bytes > AGENT_CANDIDATE_MARKDOWN_BYTE_LIMIT {
            return Err(crate::error::DadrError::Acp(format!(
                "candidate markdown is {markdown_bytes} bytes, exceeding the ACP prompt limit of {AGENT_CANDIDATE_MARKDOWN_BYTE_LIMIT} bytes"
            )));
        }
        Ok(Self {
            id: id.into(),
            title: title.into(),
            status: status.into(),
            path,
            markdown,
            markdown_bytes,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentTaskRequest {
    pub id: String,
    pub run_id: String,
    pub title: String,
    pub description: String,
    pub output_kind: TaskOutputKind,
    pub input: Value,
    #[serde(default, skip_serializing_if = "TaskContext::is_empty")]
    pub context: TaskContext,
    pub prompt: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRequestEnvelope {
    pub protocol: String,
    pub version: u32,
    pub intent: AgentIntent,
    pub prompt: String,
    pub project: AgentProjectSnapshot,
    #[serde(default, skip_serializing, skip_deserializing)]
    pub prompt_packs: Vec<AgentPromptPack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<AgentCandidateSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task: Option<AgentTaskRequest>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub chat_history: Vec<AgentChatMessage>,
}

impl AgentRequestEnvelope {
    pub fn draft_candidate(
        prompt: String,
        project: AgentProjectSnapshot,
    ) -> Result<Self, crate::error::DadrError> {
        Ok(Self {
            protocol: DADROCK_DRAFT_PROTOCOL.to_owned(),
            version: DADROCK_DRAFT_VERSION,
            intent: AgentIntent::DraftCandidate,
            prompt: bounded_user_prompt(prompt)?,
            project,
            prompt_packs: Vec::new(),
            candidate: None,
            task: None,
            chat_history: Vec::new(),
        })
    }

    pub fn revise_candidate(
        prompt: String,
        project: AgentProjectSnapshot,
        candidate: AgentCandidateSnapshot,
        chat_history: Vec<AgentChatMessage>,
    ) -> Result<Self, crate::error::DadrError> {
        Ok(Self {
            protocol: DADROCK_DRAFT_PROTOCOL.to_owned(),
            version: DADROCK_DRAFT_VERSION,
            intent: AgentIntent::ReviseCandidate,
            prompt: bounded_user_prompt(prompt)?,
            project,
            prompt_packs: Vec::new(),
            candidate: Some(candidate),
            task: None,
            chat_history: bounded_chat_history(chat_history),
        })
    }

    pub fn discuss_candidate(
        prompt: String,
        project: AgentProjectSnapshot,
        candidate: AgentCandidateSnapshot,
        chat_history: Vec<AgentChatMessage>,
    ) -> Result<Self, crate::error::DadrError> {
        Ok(Self {
            protocol: DADROCK_DRAFT_PROTOCOL.to_owned(),
            version: DADROCK_DRAFT_VERSION,
            intent: AgentIntent::DiscussCandidate,
            prompt: bounded_user_prompt(prompt)?,
            project,
            prompt_packs: Vec::new(),
            candidate: Some(candidate),
            task: None,
            chat_history: bounded_chat_history(chat_history),
        })
    }

    pub fn run_task(
        prompt: String,
        project: AgentProjectSnapshot,
        task: AgentTaskRequest,
    ) -> Result<Self, crate::error::DadrError> {
        Ok(Self {
            protocol: DADROCK_DRAFT_PROTOCOL.to_owned(),
            version: DADROCK_DRAFT_VERSION,
            intent: AgentIntent::RunTask,
            prompt: bounded_user_prompt(prompt)?,
            project,
            prompt_packs: Vec::new(),
            candidate: None,
            task: Some(task),
            chat_history: Vec::new(),
        })
    }

    pub fn with_prompt_packs(mut self, prompt_packs: Vec<AgentPromptPack>) -> Self {
        self.prompt_packs = prompt_packs;
        self
    }
}

fn bounded_project_records(records: Vec<ScannedRecordSummary>) -> Vec<ScannedRecordSummary> {
    let mut kept = Vec::new();
    let mut bytes = 0usize;

    for record in records.into_iter().rev() {
        let record_bytes = record_prompt_bytes(&record);
        if kept.len() >= AGENT_PROJECT_RECORD_LIMIT {
            break;
        }
        if !kept.is_empty() && bytes.saturating_add(record_bytes) > AGENT_PROJECT_RECORD_BYTE_LIMIT
        {
            break;
        }
        bytes = bytes.saturating_add(record_bytes);
        kept.push(record);
    }

    kept.reverse();
    kept
}

fn record_prompt_bytes(record: &ScannedRecordSummary) -> usize {
    record.id.to_string().len()
        + record.title.len()
        + record.status.as_str().len()
        + record.path.as_str().len()
}

fn bounded_user_prompt(prompt: String) -> Result<String, crate::error::DadrError> {
    let bytes = prompt.len();
    if bytes > AGENT_USER_PROMPT_BYTE_LIMIT {
        Err(crate::error::DadrError::Acp(format!(
            "agent prompt is {bytes} bytes, exceeding the ACP prompt limit of {AGENT_USER_PROMPT_BYTE_LIMIT} bytes"
        )))
    } else {
        Ok(prompt)
    }
}

fn bounded_chat_history(chat_history: Vec<AgentChatMessage>) -> Vec<AgentChatMessage> {
    let mut kept = chat_history
        .into_iter()
        .rev()
        .take(AGENT_CHAT_HISTORY_LIMIT)
        .map(|mut message| {
            message.content = truncate_utf8_with_marker(
                &message.content,
                AGENT_CHAT_MESSAGE_BYTE_LIMIT,
                "\n\n[Dadrock omitted older chat text to keep the ACP prompt bounded.]",
            );
            message
        })
        .collect::<Vec<_>>();
    kept.reverse();
    kept
}

fn truncate_utf8_with_marker(input: &str, max_bytes: usize, marker: &str) -> String {
    if input.len() <= max_bytes {
        return input.to_owned();
    }
    let mut keep = max_bytes.saturating_sub(marker.len());
    while keep > 0 && !input.is_char_boundary(keep) {
        keep -= 1;
    }
    let mut truncated = input[..keep].to_owned();
    truncated.push_str(marker);
    truncated
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentDraftResult {
    pub response: AgentResponseEnvelope,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<NewRecordResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentRevisionResult {
    pub response: AgentResponseEnvelope,
    pub candidate: UpdateCandidateResult,
}
