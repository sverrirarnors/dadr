pub mod agent;
pub mod commands;
pub mod demo;
pub mod error;
pub(crate) mod file_store;
pub mod ids {
    pub use dadr::{RecordId, RecordIdPrefix};
}
pub mod index;
pub mod links;
pub mod markdown;
pub mod mcp;
pub mod model {
    pub use dadr::{
        BibliographyRef, Record, RecordBody, RecordFrontMatter, RecordStatus, RecordSummary, Tag,
    };
}
pub(crate) mod paths;
pub mod project;
pub mod records;
pub mod tags;
pub mod tasks;
pub mod templates;
pub mod text;
pub mod validation;

pub use error::{DadrError, Diagnostic, DiagnosticCode, IssueCounts, Severity};
pub use ids::{RecordId, RecordIdPrefix};
pub use model::{Record, RecordBody, RecordFrontMatter, RecordStatus, RecordSummary, Tag};
