pub mod error;
pub mod frontmatter;
pub mod ids;
pub mod model;
pub mod record;

pub use error::{DadrError, Diagnostic, DiagnosticCode, IssueCounts, Severity};
pub use ids::{RecordId, RecordIdPrefix};
pub use model::{
    BibliographyRef, Record, RecordBody, RecordFrontMatter, RecordStatus, RecordSummary, Tag,
};
pub use record::{missing_required_sections, parse_record_str, ParsedRecord};
