use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::DadrError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskOutputKind {
    CandidateDraftV1,
    CandidateBatchV1,
    ConflictSetV1,
    DiagnosisReportV1,
    SupersessionProposalV1,
    RecordRevisionV1,
    MarkdownReportV1,
    Vendor(String),
}

struct TaskOutputKindMeta {
    kind: TaskOutputKind,
    name: &'static str,
    contract: &'static str,
}

static BUILTIN_OUTPUT_KINDS: &[TaskOutputKindMeta] = &[
    TaskOutputKindMeta {
        kind: TaskOutputKind::CandidateDraftV1,
        name: "candidate_draft.v1",
        contract: r#"{
  "kind": "candidate_draft.v1",
  "title": "string",
  "tags": ["string"],
  "body_markdown": "markdown string with Context, Decision, Alternatives, Consequences sections",
  "source_references": [],
  "warnings": ["string"]
}"#,
    },
    TaskOutputKindMeta {
        kind: TaskOutputKind::CandidateBatchV1,
        name: "candidate_batch.v1",
        contract: r#"{
  "kind": "candidate_batch.v1",
  "summary": "string",
  "candidates": [
    {
      "id": "string",
      "title": "string",
      "tags": ["string"],
      "body_markdown": "markdown string with Context, Decision, Alternatives, Consequences sections",
      "source_references": [],
      "confidence": "low|medium|high",
      "warnings": ["string"]
    }
  ]
}"#,
    },
    TaskOutputKindMeta {
        kind: TaskOutputKind::ConflictSetV1,
        name: "conflict_set.v1",
        contract: r#"{
  "kind": "conflict_set.v1",
  "summary": "string",
  "findings": [
    {
      "id": "string",
      "title": "string",
      "left_record_id": "string",
      "right_record_id": "string",
      "relationship": "string",
      "conflict_explanation": "string",
      "why_might_not_be_conflict": "optional string",
      "suggested_actions": ["string"]
    }
  ]
}"#,
    },
    TaskOutputKindMeta {
        kind: TaskOutputKind::DiagnosisReportV1,
        name: "diagnosis_report.v1",
        contract: r#"{
  "kind": "diagnosis_report.v1",
  "summary": "string or JSON object with concise scalar/list fields",
  "issues": [
    {
      "id": "string",
      "severity": "low|medium|high|critical",
      "category": "string",
      "record_id": "optional string or null",
      "title": "string",
      "explanation": "string",
      "suggested_actions": ["string"]
    }
  ]
}"#,
    },
    TaskOutputKindMeta {
        kind: TaskOutputKind::SupersessionProposalV1,
        name: "supersession_proposal.v1",
        contract: r#"{
  "kind": "supersession_proposal.v1",
  "summary": "string",
  "superseded_record_id": "string",
  "superseding_title": "string",
  "tags": ["string"],
  "superseding_body_markdown": "markdown string with Context, Decision, Alternatives, Consequences sections",
  "rationale": "string",
  "suggested_actions": ["string"]
}"#,
    },
    TaskOutputKindMeta {
        kind: TaskOutputKind::RecordRevisionV1,
        name: "record_revision.v1",
        contract: r#"{
  "kind": "record_revision.v1",
  "summary": "string",
  "record_id": "string",
  "title": "optional string",
  "tags": ["optional string"],
  "body_markdown": "complete revised markdown body",
  "change_explanation": "string",
  "warnings": ["string"]
}"#,
    },
    TaskOutputKindMeta {
        kind: TaskOutputKind::MarkdownReportV1,
        name: "markdown_report.v1",
        contract: r#"{
  "kind": "markdown_report.v1",
  "title": "optional string",
  "summary": "optional string",
  "body_markdown": "markdown string",
  "raw_json": {}
}"#,
    },
];

impl TaskOutputKind {
    fn builtin_meta(&self) -> Option<&'static TaskOutputKindMeta> {
        BUILTIN_OUTPUT_KINDS.iter().find(|meta| &meta.kind == self)
    }

    pub fn as_str(&self) -> &str {
        self.builtin_meta()
            .map_or_else(|| self.vendor_kind(), |meta| meta.name)
    }

    fn from_kind_str(kind: &str) -> Self {
        BUILTIN_OUTPUT_KINDS
            .iter()
            .find(|meta| meta.name == kind)
            .map_or_else(|| Self::Vendor(kind.to_owned()), |meta| meta.kind.clone())
    }

    pub fn is_reserved_builtin_like(kind: &str) -> bool {
        let normalized = kind.replace('-', "_");
        BUILTIN_OUTPUT_KINDS
            .iter()
            .any(|meta| meta.name == normalized)
    }

    pub fn output_contract(&self) -> String {
        if let Some(meta) = self.builtin_meta() {
            return meta.contract.to_owned();
        }
        let kind = self.vendor_kind();
        format!(
            r#"{{
  "kind": "{kind}",
  "...": "vendor-defined fields for this task"
}}"#
        )
    }

    fn vendor_kind(&self) -> &str {
        match self {
            Self::Vendor(kind) => kind.as_str(),
            _ => "",
        }
    }
}

impl From<&TaskOutputKind> for TaskOutputKind {
    fn from(kind: &TaskOutputKind) -> Self {
        kind.clone()
    }
}

impl Serialize for TaskOutputKind {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TaskOutputKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let kind = String::deserialize(deserializer)?;
        if kind.trim().is_empty() {
            return Err(serde::de::Error::custom(
                "task output kind must not be empty",
            ));
        }
        Ok(Self::from_kind_str(&kind))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TaskOutputValue {
    #[serde(rename = "candidate_draft.v1")]
    CandidateDraft(CandidateDraftOutput),
    #[serde(rename = "candidate_batch.v1")]
    CandidateBatch(CandidateBatchOutput),
    #[serde(rename = "conflict_set.v1")]
    ConflictSet(ConflictSetOutput),
    #[serde(rename = "diagnosis_report.v1")]
    DiagnosisReport(DiagnosisReportOutput),
    #[serde(rename = "supersession_proposal.v1")]
    SupersessionProposal(SupersessionProposalOutput),
    #[serde(rename = "record_revision.v1")]
    RecordRevision(RecordRevisionOutput),
    #[serde(rename = "markdown_report.v1")]
    MarkdownReport(MarkdownReportOutput),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateDraftOutput {
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub body_markdown: String,
    #[serde(default)]
    pub source_references: Vec<Value>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateBatchOutput {
    pub summary: String,
    #[serde(default)]
    pub candidates: Vec<CandidateBatchCandidate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CandidateBatchCandidate {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub body_markdown: String,
    #[serde(default)]
    pub source_references: Vec<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflictSetOutput {
    pub summary: String,
    #[serde(default)]
    pub findings: Vec<ConflictFinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConflictFinding {
    pub id: String,
    pub title: String,
    pub left_record_id: String,
    pub right_record_id: String,
    pub relationship: String,
    pub conflict_explanation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub why_might_not_be_conflict: Option<String>,
    #[serde(default)]
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosisReportOutput {
    pub summary: Value,
    #[serde(default)]
    pub issues: Vec<DiagnosisIssue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiagnosisIssue {
    pub id: String,
    pub severity: String,
    pub category: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record_id: Option<String>,
    pub title: String,
    pub explanation: String,
    #[serde(default)]
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupersessionProposalOutput {
    pub summary: String,
    pub superseded_record_id: String,
    pub superseding_title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub superseding_body_markdown: String,
    pub rationale: String,
    #[serde(default)]
    pub suggested_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordRevisionOutput {
    pub summary: String,
    pub record_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub body_markdown: String,
    pub change_explanation: String,
    #[serde(default)]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkdownReportOutput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default)]
    pub body_markdown: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_json: Option<Value>,
}

pub trait TaskOutputInput {
    fn into_task_output_value(self) -> Result<Value, DadrError>;
}

impl TaskOutputInput for Value {
    fn into_task_output_value(self) -> Result<Value, DadrError> {
        Ok(self)
    }
}

impl TaskOutputInput for &Value {
    fn into_task_output_value(self) -> Result<Value, DadrError> {
        Ok(self.clone())
    }
}

impl TaskOutputInput for &str {
    fn into_task_output_value(self) -> Result<Value, DadrError> {
        serde_json::from_str(self).map_err(|err| {
            DadrError::InvalidInput(format!("failed to parse task output JSON: {err}"))
        })
    }
}

impl TaskOutputInput for String {
    fn into_task_output_value(self) -> Result<Value, DadrError> {
        serde_json::from_str(&self).map_err(|err| {
            DadrError::InvalidInput(format!("failed to parse task output JSON: {err}"))
        })
    }
}

pub fn validate_task_output(
    expected_kind: impl Into<TaskOutputKind>,
    output: impl TaskOutputInput,
) -> Result<Value, DadrError> {
    let expected_kind = expected_kind.into();
    let value = output.into_task_output_value()?;
    let actual_kind = output_kind_from_value(&value)?;

    if actual_kind != expected_kind {
        return Err(DadrError::InvalidInput(format!(
            "task output kind `{}` did not match expected kind `{}`",
            actual_kind.as_str(),
            expected_kind.as_str()
        )));
    }

    match expected_kind {
        TaskOutputKind::Vendor(_) => Ok(value),
        _ => {
            serde_json::from_value::<TaskOutputValue>(value.clone()).map_err(|err| {
                DadrError::InvalidInput(format!(
                    "failed to validate `{}` task output: {err}",
                    expected_kind.as_str()
                ))
            })?;
            Ok(value)
        }
    }
}

fn output_kind_from_value(value: &Value) -> Result<TaskOutputKind, DadrError> {
    let kind = value
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| DadrError::InvalidInput("task output must include string `kind`"))?;
    if kind.trim().is_empty() {
        return Err(DadrError::InvalidInput(
            "task output kind must not be empty",
        ));
    }
    Ok(TaskOutputKind::from_kind_str(kind))
}

impl fmt::Display for TaskOutputKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}
