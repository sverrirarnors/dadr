use std::{fmt, fs};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{
    error::{DadrError, Diagnostic},
    file_store,
    links::{check_project_links, LinkCheckReport},
    records::{
        accept_candidates, check_record_rewrite, create_candidate, create_candidate_with_body,
        parse_record_str, reject_candidate, scan_input, status_record, successor_candidate_body,
        supersede_record, supersede_records as supersede_records_impl, AcceptRecordResult,
        NewRecordResult, RecordMove, RecordStatusResult, RejectRecordResult, ScannedRecordSummary,
        SupersedeRecordResult, SupersedeRecordsResult,
    },
    tags::{build_tag_report, record_ids_for_tag, TagReport},
    templates::candidate_template_body,
    validation::{validate_project_index, ProjectValidationReport},
    RecordId, RecordStatus,
};

use super::{CommandContext, CommandDiagnostic, CommandEnvelope};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordViewFilter {
    Candidate,
    Accepted,
    Superseded,
    Rejected,
}

impl RecordViewFilter {
    pub fn matches(self, status: RecordStatus, superseded: bool) -> bool {
        match self {
            Self::Candidate => status == RecordStatus::Candidate,
            Self::Accepted => status == RecordStatus::Accepted && !superseded,
            Self::Superseded => status == RecordStatus::Accepted && superseded,
            Self::Rejected => status == RecordStatus::Rejected,
        }
    }

    pub fn matches_summary(self, summary: &ScannedRecordSummary) -> bool {
        self.matches(summary.status, summary.superseded)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
            Self::Accepted => "accepted",
            Self::Superseded => "superseded",
            Self::Rejected => "rejected",
        }
    }
}

impl fmt::Display for RecordViewFilter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListRecordsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<RecordViewFilter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default)]
    pub unique: bool,
}

pub fn list_records_command(
    context: &CommandContext,
    request: ListRecordsRequest,
) -> Result<CommandEnvelope<Vec<ScannedRecordSummary>>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let mut records = if let Some(query) = request.query {
        if query.trim().is_empty() {
            return Err(DadrError::InvalidInput(
                "list query cannot be empty".to_owned(),
            ));
        }
        index
            .find_records(&query)
            .matches
            .into_iter()
            .map(|found| found.record)
            .collect()
    } else {
        index.list_records(None)
    };

    if let Some(filter) = request.status {
        records.retain(|record| filter.matches_summary(record));
    }

    if let Some(tag) = request.tag {
        let matching_ids = record_ids_for_tag(&index, &tag)?;
        records.retain(|record| matching_ids.contains(&record.id));
    }

    let unique_ok = !request.unique || records.len() == 1;
    Ok(CommandEnvelope::new("list", records).with_ok(unique_ok))
}

pub fn show_status(
    context: &CommandContext,
    query: &str,
) -> Result<CommandEnvelope<RecordStatusResult>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let result = status_record(&index, query)?;
    let diagnostics = result
        .diagnostics
        .iter()
        .map(CommandDiagnostic::from)
        .collect();
    Ok(CommandEnvelope::new("status", result).with_diagnostics(diagnostics))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveRecordSourceRequest {
    pub query: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveRecordSourceResult {
    pub id: RecordId,
    pub path: Utf8PathBuf,
    pub status: RecordStatus,
    pub superseded: bool,
}

pub fn save_record_source(
    context: &CommandContext,
    request: SaveRecordSourceRequest,
) -> Result<CommandEnvelope<SaveRecordSourceResult>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let found = index.resolve_one(&request.query)?;
    let parsed = parse_record_str(&request.content);
    if let Some(diagnostic) = parsed
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.is_error())
    {
        return Err(DadrError::InvalidInput(format!(
            "record markdown is invalid: {}",
            diagnostic.message
        )));
    }
    let Some(record) = parsed.record else {
        return Err(DadrError::InvalidInput(
            "record markdown is invalid: missing record front matter".to_owned(),
        ));
    };
    // Accepted and rejected records keep their id, status, and successors,
    // and accepted records their required sections; corrections to the title,
    // body, and tags are saved in place.
    let current = parse_record_str(&fs::read_to_string(found.record.path.as_std_path())?)
        .record
        .ok_or_else(|| {
            DadrError::ProjectInvariant(format!("failed to parse `{}`", found.record.path))
        })?;
    check_record_rewrite(&current, &record)?;
    if record.front_matter.id != found.record.id {
        return Err(DadrError::InvalidInput(
            "record id cannot be changed in the web editor".to_owned(),
        ));
    }
    if record.front_matter.status != found.record.status {
        return Err(DadrError::InvalidInput(
            "record status cannot be changed in the web editor".to_owned(),
        ));
    }

    file_store::write_replace(&found.record.path, &request.content)?;
    let diagnostics = parsed
        .diagnostics
        .iter()
        .map(CommandDiagnostic::from)
        .collect();
    Ok(CommandEnvelope::new(
        "record source save",
        SaveRecordSourceResult {
            id: record.front_matter.id,
            path: found.record.path.clone(),
            status: record.front_matter.status,
            superseded: record.front_matter.is_superseded(),
        },
    )
    .with_diagnostics(diagnostics)
    .with_changed_paths(vec![found.record.path]))
}

pub fn accept_record(
    context: &CommandContext,
    query: &str,
) -> Result<CommandEnvelope<AcceptRecordResult>, DadrError> {
    accept_records(context, &[query.to_owned()])
}

/// Accept a set of candidates in one operation: a single candidate, a merge,
/// or the halves of a split. See [`crate::records::accept_candidates`].
pub fn accept_records(
    context: &CommandContext,
    queries: &[String],
) -> Result<CommandEnvelope<AcceptRecordResult>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let result = accept_candidates(&index, queries)?;
    let mut changed_paths = Vec::new();
    for moved in &result.accepted {
        changed_paths.extend(moved_paths(&moved.from, &moved.to));
    }
    for moved in &result.superseded {
        changed_paths.extend(moved_paths(&moved.from, &moved.to));
    }
    Ok(CommandEnvelope::new("status accept", result).with_changed_paths(changed_paths))
}

pub fn reject_record(
    context: &CommandContext,
    query: &str,
) -> Result<CommandEnvelope<RejectRecordResult>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let result = reject_candidate(&index, query)?;
    let changed_paths = moved_paths(&result.from, &result.to);
    Ok(CommandEnvelope::new("status reject", result).with_changed_paths(changed_paths))
}

pub fn supersede(
    context: &CommandContext,
    old_query: &str,
    new_query: &str,
) -> Result<CommandEnvelope<SupersedeRecordResult>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let result = supersede_record(&index, old_query, new_query)?;
    let old_path = index
        .summary_by_id(result.old_id)
        .map_or_else(|| result.old_path.clone(), |old| old.path.clone());
    let changed_paths = moved_paths(&old_path, &result.old_path);
    Ok(CommandEnvelope::new("status supersede", result).with_changed_paths(changed_paths))
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupersedeRecordsRequest {
    pub from: Vec<String>,
    pub to: Vec<String>,
}

pub fn supersede_records(
    context: &CommandContext,
    request: SupersedeRecordsRequest,
) -> Result<CommandEnvelope<SupersedeRecordsResult>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let result = supersede_records_impl(&index, &request.from, &request.to)?;
    let changed_paths = result
        .superseded
        .iter()
        .flat_map(|RecordMove { from, to, .. }| moved_paths(from, to))
        .collect();
    Ok(CommandEnvelope::new("status supersede", result).with_changed_paths(changed_paths))
}

/// The paths a rewrite touched: one path in place, or both ends of a move.
fn moved_paths(from: &Utf8PathBuf, to: &Utf8PathBuf) -> Vec<Utf8PathBuf> {
    if from == to {
        vec![to.clone()]
    } else {
        vec![from.clone(), to.clone()]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidateRecordsResult {
    pub records: Vec<ValidatedRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatedRecord {
    pub path: Utf8PathBuf,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record: Option<ScannedRecordSummary>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn validate_records(
    context: &CommandContext,
    paths: Vec<Utf8PathBuf>,
) -> Result<CommandEnvelope<ValidateRecordsResult>, DadrError> {
    let scanned_records = if paths.is_empty() {
        context.initialized_index()?.1.scanned_records().to_vec()
    } else {
        let mut scanned_records = Vec::new();
        for path in paths {
            scanned_records.extend(scan_input(&path)?);
        }
        scanned_records
    };
    let mut has_errors = false;
    let mut reports = Vec::new();
    let mut command_diagnostics = Vec::new();

    for parsed in scanned_records {
        let valid = parsed.is_valid();
        has_errors |= !valid;
        command_diagnostics.extend(parsed.diagnostics.iter().map(CommandDiagnostic::from));
        let record = parsed.summary();
        reports.push(ValidatedRecord {
            path: parsed.path,
            valid,
            record,
            diagnostics: parsed.diagnostics,
        });
    }

    Ok(CommandEnvelope::new(
        "validate records",
        ValidateRecordsResult { records: reports },
    )
    .with_ok(!has_errors)
    .with_diagnostics(command_diagnostics))
}

pub fn validate_project(
    context: &CommandContext,
) -> Result<CommandEnvelope<ProjectValidationReport>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let report = validate_project_index(&index);
    let ok = !report.has_errors();
    Ok(CommandEnvelope::new("validate", report).with_ok(ok))
}

pub fn list_tags(context: &CommandContext) -> Result<CommandEnvelope<TagReport>, DadrError> {
    let (_, index) = context.initialized_index()?;
    Ok(CommandEnvelope::new("tags list", build_tag_report(&index)))
}

pub fn validate_tags(context: &CommandContext) -> Result<CommandEnvelope<TagReport>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let report = build_tag_report(&index);
    let ok = !report.has_errors();
    Ok(CommandEnvelope::new("validate tags", report).with_ok(ok))
}

pub fn validate_links(
    context: &CommandContext,
) -> Result<CommandEnvelope<LinkCheckReport>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let report = check_project_links(&index);
    let ok = !report.has_errors();
    Ok(CommandEnvelope::new("validate links", report).with_ok(ok))
}
pub fn new_record(
    context: &CommandContext,
    title: &str,
    tags: &[String],
) -> Result<CommandEnvelope<NewRecordResult>, DadrError> {
    let project = context.discover_project()?;
    let result = create_candidate(&project, title, tags)?;
    let changed_paths = vec![result.path.clone()];
    Ok(CommandEnvelope::new("new", result).with_changed_paths(changed_paths))
}

pub fn new_record_with_body(
    context: &CommandContext,
    title: &str,
    tags: &[String],
    body_markdown: &str,
) -> Result<CommandEnvelope<NewRecordResult>, DadrError> {
    let project = context.discover_project()?;
    let result = create_candidate_with_body(&project, title, tags, body_markdown)?;
    let changed_paths = vec![result.path.clone()];
    Ok(CommandEnvelope::new("new", result).with_changed_paths(changed_paths))
}

/// A new candidate that supersedes one or more current decisions.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewSuccessorRequest {
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    /// The body without `## Supersedes`; the candidate template when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body_markdown: Option<String>,
    /// Queries for the records the candidate supersedes.
    pub supersedes: Vec<String>,
    /// Why the decision changed, written after the links in `## Supersedes`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Create a successor candidate. Dadrock writes its `## Supersedes` section
/// first in the body: one link per predecessor, labelled with its title, then
/// the reason. Accepting the candidate supersedes the predecessors.
pub fn new_successor_record(
    context: &CommandContext,
    request: NewSuccessorRequest,
) -> Result<CommandEnvelope<NewRecordResult>, DadrError> {
    let (project, index) = context.initialized_index()?;
    let body = match request.body_markdown {
        Some(body) => body,
        None => candidate_template_body(&project)?.body,
    };
    let body = successor_candidate_body(
        &index,
        &request.supersedes,
        request.reason.as_deref(),
        &body,
    )?;
    let result = create_candidate_with_body(&project, &request.title, &request.tags, &body)?;
    let changed_paths = vec![result.path.clone()];
    Ok(CommandEnvelope::new("new", result).with_changed_paths(changed_paths))
}
