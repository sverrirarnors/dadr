use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::{error::DadrError, ids::RecordId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordStatus {
    Candidate,
    Accepted,
    Rejected,
}

impl RecordStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }
}

impl fmt::Display for RecordStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RecordStatus {
    type Err = DadrError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "candidate" => Ok(Self::Candidate),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            other => Err(DadrError::InvalidStatus(other.to_owned())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Tag(String);

impl Tag {
    pub fn raw(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordFrontMatter {
    pub title: String,
    pub status: RecordStatus,
    pub id: RecordId,
    pub successors: Vec<RecordId>,
    pub tags: Vec<Tag>,
}

impl RecordFrontMatter {
    pub fn is_superseded(&self) -> bool {
        self.status == RecordStatus::Accepted && !self.successors.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordBody {
    pub raw_markdown: String,
    /// How many lines of the record file precede the body: the front matter
    /// and its fences. Line `n` of the body (1-based) is line
    /// `line_offset + n` of the file, the line diagnostics report.
    #[serde(default)]
    pub line_offset: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub front_matter: RecordFrontMatter,
    pub body: RecordBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSummary {
    pub id: RecordId,
    pub title: String,
    pub status: RecordStatus,
}

impl From<&Record> for RecordSummary {
    fn from(record: &Record) -> Self {
        Self {
            id: record.front_matter.id,
            title: record.front_matter.title.clone(),
            status: record.front_matter.status,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_accepts_only_canonical_values() {
        assert_eq!(
            RecordStatus::from_str("candidate").unwrap(),
            RecordStatus::Candidate
        );
        assert!(RecordStatus::from_str("proposed").is_err());
        assert!(RecordStatus::from_str("under_review").is_err());
        assert!(RecordStatus::from_str("withdrawn").is_err());
    }

    #[test]
    fn status_serializes_as_snake_case() {
        let json = serde_json::to_string(&RecordStatus::Accepted).unwrap();
        assert_eq!(json, "\"accepted\"");
    }
}
