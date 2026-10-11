use camino::Utf8PathBuf;
use std::error::Error as StdError;
use thiserror::Error;

pub use dadr::{Diagnostic, DiagnosticCode, IssueCounts, Severity};

#[derive(Debug, Error)]
pub enum DadrError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid DADR record id: {0}")]
    InvalidRecordId(String),

    #[error("invalid DADR status: {0}")]
    InvalidStatus(String),

    #[error("record not found for query: {0}")]
    RecordNotFound(String),

    #[error("record query is ambiguous: {query} matched {matches} records")]
    AmbiguousRecordQuery { query: String, matches: usize },

    #[error("no Dadrock project found for `{project_dir}`; run `dadrock init` first")]
    ProjectNotInitialized { project_dir: Utf8PathBuf },

    #[error("path is not valid UTF-8: {path}")]
    PathNotUtf8 { path: String },

    #[error("refusing to overwrite existing path `{path}`")]
    OverwriteCollision { path: Utf8PathBuf },

    #[error(transparent)]
    Domain(DomainError),
}

impl From<dadr::DadrError> for DadrError {
    fn from(error: dadr::DadrError) -> Self {
        match error {
            dadr::DadrError::InvalidRecordId(input) => Self::InvalidRecordId(input),
            dadr::DadrError::InvalidStatus(input) => Self::InvalidStatus(input),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainErrorKind {
    InvalidInput,
    InvalidLifecycleTransition,
    ProjectInvariant,
    Config,
    Template,
    Acp,
    Mcp,
    Git,
}

impl DomainErrorKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid input",
            Self::InvalidLifecycleTransition => "invalid lifecycle transition",
            Self::ProjectInvariant => "project invariant failed",
            Self::Config => "configuration error",
            Self::Template => "template error",
            Self::Acp => "ACP error",
            Self::Mcp => "MCP error",
            Self::Git => "Git error",
        }
    }
}

#[derive(Debug)]
pub struct DomainError {
    kind: DomainErrorKind,
    message: String,
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
}

impl DomainError {
    pub fn new(kind: DomainErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            source: None,
        }
    }

    pub fn with_source(
        kind: DomainErrorKind,
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn kind(&self) -> DomainErrorKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.kind.label(), self.message)
    }
}

impl StdError for DomainError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}

impl DadrError {
    pub fn domain(
        kind: DomainErrorKind,
        message: impl Into<String>,
        source: Option<Box<dyn StdError + Send + Sync + 'static>>,
    ) -> Self {
        Self::Domain(DomainError {
            kind,
            message: message.into(),
            source,
        })
    }

    pub fn domain_with_source(
        kind: DomainErrorKind,
        message: impl Into<String>,
        source: impl StdError + Send + Sync + 'static,
    ) -> Self {
        Self::Domain(DomainError::with_source(kind, message, source))
    }

    pub fn is_domain_kind(&self, kind: DomainErrorKind) -> bool {
        matches!(self, Self::Domain(error) if error.kind() == kind)
    }

    pub fn domain_message(&self, kind: DomainErrorKind) -> Option<&str> {
        match self {
            Self::Domain(error) if error.kind() == kind => Some(error.message()),
            _ => None,
        }
    }

    #[allow(non_snake_case)]
    pub fn InvalidInput(message: impl Into<String>) -> Self {
        Self::Domain(DomainError::new(DomainErrorKind::InvalidInput, message))
    }

    #[allow(non_snake_case)]
    pub fn InvalidLifecycleTransition(message: impl Into<String>) -> Self {
        Self::Domain(DomainError::new(
            DomainErrorKind::InvalidLifecycleTransition,
            message,
        ))
    }

    #[allow(non_snake_case)]
    pub fn ProjectInvariant(message: impl Into<String>) -> Self {
        Self::Domain(DomainError::new(DomainErrorKind::ProjectInvariant, message))
    }

    #[allow(non_snake_case)]
    pub fn Config(message: impl Into<String>) -> Self {
        Self::Domain(DomainError::new(DomainErrorKind::Config, message))
    }

    #[allow(non_snake_case)]
    pub fn Template(message: impl Into<String>) -> Self {
        Self::Domain(DomainError::new(DomainErrorKind::Template, message))
    }

    #[allow(non_snake_case)]
    pub fn Acp(message: impl Into<String>) -> Self {
        Self::Domain(DomainError::new(DomainErrorKind::Acp, message))
    }

    #[allow(non_snake_case)]
    pub fn Mcp(message: impl Into<String>) -> Self {
        Self::Domain(DomainError::new(DomainErrorKind::Mcp, message))
    }

    #[allow(non_snake_case)]
    pub fn Git(message: impl Into<String>) -> Self {
        Self::Domain(DomainError::new(DomainErrorKind::Git, message))
    }
}
