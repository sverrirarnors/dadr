use serde::{Deserialize, Serialize};

use crate::DadrError;

use super::{ids::is_valid_task_id, output::TaskOutputKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskManifest {
    pub id: String,
    pub title: String,
    pub description: String,
    pub version: u32,
    #[serde(default)]
    pub execution: TaskExecutionMode,
    #[serde(default)]
    pub inputs: Vec<TaskInput>,
    #[serde(default)]
    pub requirements: TaskRequirements,
    pub output: TaskOutput,
    pub prompt: TaskPrompt,
    #[serde(default)]
    pub tools: Vec<TaskTool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskExecutionMode {
    #[default]
    Agent,
    Local,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskInput {
    pub name: String,
    pub label: String,
    #[serde(rename = "type")]
    pub field_type: TaskInputType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<String>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskRequirements {
    #[serde(default)]
    pub records: TaskRecordRequirement,
    #[serde(default)]
    pub record_status: TaskRecordStatusRequirement,
    #[serde(default)]
    pub paths: TaskPathRequirement,
    #[serde(default)]
    pub notes: TaskNoteRequirement,
    #[serde(default)]
    pub scope: TaskScopeRequirement,
}

impl Default for TaskRequirements {
    fn default() -> Self {
        Self {
            records: TaskRecordRequirement::None,
            record_status: TaskRecordStatusRequirement::All,
            paths: TaskPathRequirement::None,
            notes: TaskNoteRequirement::None,
            scope: TaskScopeRequirement::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskRecordRequirement {
    #[default]
    None,
    One,
    Many,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskRecordStatusRequirement {
    #[default]
    All,
    Candidate,
    Accepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskPathRequirement {
    #[default]
    None,
    Optional,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskNoteRequirement {
    #[default]
    None,
    Optional,
    Required,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskScopeRequirement {
    #[default]
    None,
    Live,
    Candidates,
    Accepted,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskInputType {
    Text,
    Textarea,
    Path,
    Select,
    Multiselect,
    Checkbox,
    Number,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskOutput {
    pub kind: TaskOutputKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskPrompt {
    pub system: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskTool {
    pub name: String,
    pub mode: TaskToolMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskToolMode {
    Read,
    Write,
}

pub fn parse_task_manifest(source: &str, content: &str) -> Result<TaskManifest, DadrError> {
    let manifest: TaskManifest = toml::from_str(content).map_err(|err| {
        DadrError::InvalidInput(format!("failed to parse task manifest `{source}`: {err}"))
    })?;
    validate_manifest(source, &manifest)?;
    Ok(manifest)
}

fn validate_manifest(source: &str, manifest: &TaskManifest) -> Result<(), DadrError> {
    if !is_valid_task_id(&manifest.id) {
        return Err(DadrError::InvalidInput(format!(
            "task id `{}` in `{source}` must use lowercase letters, digits, and hyphens",
            manifest.id
        )));
    }
    if manifest.title.trim().is_empty() {
        return Err(DadrError::InvalidInput(format!(
            "task `{}` must have a non-empty title",
            manifest.id
        )));
    }
    if manifest.description.trim().is_empty() {
        return Err(DadrError::InvalidInput(format!(
            "task `{}` must have a non-empty description",
            manifest.id
        )));
    }
    if manifest.prompt.system.trim().is_empty() {
        return Err(DadrError::InvalidInput(format!(
            "task `{}` must name a system prompt file",
            manifest.id
        )));
    }
    if manifest.execution == TaskExecutionMode::Local {
        return Err(DadrError::InvalidInput(format!(
            "task `{}` declares local task execution, but local task execution is not implemented",
            manifest.id
        )));
    }
    if let TaskOutputKind::Vendor(kind) = &manifest.output.kind {
        if TaskOutputKind::is_reserved_builtin_like(kind) {
            return Err(DadrError::InvalidInput(format!(
                "task `{}` declares unknown built-in task output kind `{kind}`",
                manifest.id
            )));
        }
    }
    for input in &manifest.inputs {
        if !is_valid_input_name(&input.name) {
            return Err(DadrError::InvalidInput(format!(
                "task `{}` input `{}` must use lowercase letters, digits, and underscores",
                manifest.id, input.name
            )));
        }
        if matches!(
            input.field_type,
            TaskInputType::Select | TaskInputType::Multiselect
        ) && input.options.is_empty()
        {
            return Err(DadrError::InvalidInput(format!(
                "task `{}` input `{}` must declare options",
                manifest.id, input.name
            )));
        }
    }
    Ok(())
}

fn is_valid_input_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}
