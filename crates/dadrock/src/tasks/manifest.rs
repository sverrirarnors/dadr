use dadr::{frontmatter::split_front_matter, DiagnosticCode};
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use crate::{mcp::DadrockMcpAccess, DadrError};

use super::{ids::is_valid_task_id, output::TaskOutputKind};

/// A task's manifest: the YAML front matter of its task document. The
/// document body, after the front matter, is the task prompt.
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
    /// Dadrock MCP access granted to the agent while it runs this task.
    #[serde(default)]
    pub mcp_access: DadrockMcpAccess,
    #[serde(default)]
    pub tools: Vec<TaskTool>,
}

/// A parsed task document: its manifest plus the trimmed prompt body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskDocument {
    pub manifest: TaskManifest,
    pub prompt: String,
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
    #[serde(
        default,
        deserialize_with = "deserialize_scalar_default",
        skip_serializing_if = "Option::is_none"
    )]
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

/// Parse a task document: YAML front matter (the manifest) followed by the
/// Markdown task prompt. The returned prompt is trimmed and may be empty;
/// callers decide how to report a task without a prompt.
pub fn parse_task_document(source: &str, content: &str) -> Result<TaskDocument, DadrError> {
    let split = split_front_matter(content);
    if let Some(diagnostic) = split.diagnostics.first() {
        let problem = match diagnostic.code {
            DiagnosticCode::MissingFrontMatter => {
                "it must start with YAML front matter between `---` lines"
            }
            _ => "its YAML front matter is missing a closing `---` line",
        };
        return Err(DadrError::InvalidInput(format!(
            "failed to parse task document `{source}`: {problem}"
        )));
    }
    let yaml = split.raw_yaml.unwrap_or_default();
    let manifest: TaskManifest = serde_yaml::from_str(&yaml).map_err(|err| {
        DadrError::InvalidInput(format!(
            "failed to parse task front matter `{source}`: {err}"
        ))
    })?;
    validate_manifest(source, &manifest)?;
    Ok(TaskDocument {
        manifest,
        prompt: split.body.trim().to_owned(),
    })
}

/// YAML reads `default: 5` or `default: true` as a number or boolean; input
/// defaults are strings, so accept any scalar and keep its text.
fn deserialize_scalar_default<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    match Option::<serde_yaml::Value>::deserialize(deserializer)? {
        None | Some(serde_yaml::Value::Null) => Ok(None),
        Some(serde_yaml::Value::String(value)) => Ok(Some(value)),
        Some(serde_yaml::Value::Number(value)) => Ok(Some(value.to_string())),
        Some(serde_yaml::Value::Bool(value)) => Ok(Some(value.to_string())),
        Some(_) => Err(D::Error::custom(
            "input default must be a string, number, or boolean",
        )),
    }
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
