use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DadrError {
    #[error("invalid DADR record id: {0}")]
    InvalidRecordId(String),

    #[error("invalid DADR status: {0}")]
    InvalidStatus(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueCounts {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
}

impl IssueCounts {
    pub fn from_severities(severities: impl IntoIterator<Item = Severity>) -> Self {
        let mut counts = Self::default();
        for severity in severities {
            counts.push(severity);
        }
        counts
    }

    pub fn push(&mut self, severity: Severity) {
        match severity {
            Severity::Error => self.errors += 1,
            Severity::Warning => self.warnings += 1,
            Severity::Info => self.infos += 1,
        }
    }

    pub fn has_errors(self) -> bool {
        self.errors > 0
    }

    pub fn label(self) -> String {
        format!(
            "{} errors, {} warnings, {} infos",
            self.errors, self.warnings, self.infos
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    MissingFrontMatter,
    MalformedFrontMatter,
    MalformedYaml,
    FrontMatterNotMapping,
    MissingRequiredField,
    InvalidFieldType,
    InvalidStatus,
    InvalidRecordId,
    /// An `id` or `successors` entry that is a valid UUIDv7 but not in the
    /// lowercase hyphenated form records must use (spec §3, §5).
    NonCanonicalRecordId,
    DateInFrontMatter,
    /// The file starts with a UTF-8 byte-order mark (spec §2).
    ByteOrderMark,
    UnreadableRecord,
    /// A `## Supersedes` section that DADRock cannot read: repeated, not
    /// starting with a link, or with a link that is not a full id. This is a
    /// DADRock convention, not part of the DADR format; `dadr` never emits it.
    InvalidSupersedes,
}

impl DiagnosticCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingFrontMatter => "missing_front_matter",
            Self::MalformedFrontMatter => "malformed_front_matter",
            Self::MalformedYaml => "malformed_yaml",
            Self::FrontMatterNotMapping => "front_matter_not_mapping",
            Self::MissingRequiredField => "missing_required_field",
            Self::InvalidFieldType => "invalid_field_type",
            Self::InvalidStatus => "invalid_status",
            Self::InvalidRecordId => "invalid_record_id",
            Self::NonCanonicalRecordId => "non_canonical_record_id",
            Self::DateInFrontMatter => "date_in_front_matter",
            Self::ByteOrderMark => "byte_order_mark",
            Self::UnreadableRecord => "unreadable_record",
            Self::InvalidSupersedes => "invalid_supersedes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl Diagnostic {
    pub fn new(severity: Severity, code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            path: None,
            field: None,
        }
    }

    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }

    pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    pub fn info(code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self::new(Severity::Info, code, message)
    }

    pub fn with_field(mut self, field: impl Into<String>) -> Self {
        self.field = Some(field.into());
        self
    }

    pub fn with_path(mut self, path: impl Into<Utf8PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn is_error(&self) -> bool {
        self.severity == Severity::Error
    }
}
