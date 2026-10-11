use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, fs,
};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{
    error::{DiagnosticCode, IssueCounts, Severity},
    ids::RecordId,
    index::ProjectIndex,
    links::{check_project_links, LinkIssue, LinkIssueCode},
    model::{Record, RecordStatus},
    project::{utf8_to_path, ProjectContext},
    records::{
        check_required_sections, current_successors, record_list, supersedes_diagnostic,
        supersedes_section, ScannedRecord,
    },
    tags::{build_tag_report, TagIssue},
    text::write_serde_label,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectValidationReport {
    pub project_dir: Utf8PathBuf,
    pub ok: bool,
    pub counts: IssueCounts,
    pub issues: Vec<ProjectValidationIssue>,
}

impl ProjectValidationReport {
    pub fn has_errors(&self) -> bool {
        self.counts.has_errors()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectValidationIssueCode {
    RecordDiagnostic,
    MissingRequiredSection,
    DuplicateRequiredSection,
    EmptyRequiredSection,
    DuplicateRecordId,
    ShortIdCollision,
    UnresolvedSuccessor,
    InvalidSuccessor,
    SuccessorCycle,
    /// A `## Supersedes` section that acceptance refuses: repeated, not
    /// starting with a link, or with a link that is not a full id; or, in a
    /// candidate, naming the candidate itself or a record that is not
    /// accepted.
    InvalidSupersedes,
    /// A candidate's `## Supersedes` names a record superseded since.
    StaleSupersedes,
    /// An accepted record's `## Supersedes` names a record that does not list
    /// it in `successors`.
    UnlinkedSupersedes,
    StatusLocationMismatch,
    BrokenWikilink,
    AmbiguousWikilink,
    RejectedRecordWikilink,
    BrokenMarkdownLink,
    TemplateReadError,
    InvalidTag,
}

impl fmt::Display for ProjectValidationIssueCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_serde_label(self, formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectValidationIssue {
    pub severity: Severity,
    pub code: ProjectValidationIssueCode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_code: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<RecordId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_code: Option<DiagnosticCode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_paths: Vec<Utf8PathBuf>,
}

impl ProjectValidationIssue {
    fn new(
        severity: Severity,
        code: ProjectValidationIssueCode,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            sub_code: None,
            message: message.into(),
            path: None,
            record_id: None,
            record_code: None,
            field: None,
            line: None,
            related_paths: Vec::new(),
        }
    }

    fn error(code: ProjectValidationIssueCode, message: impl Into<String>) -> Self {
        Self::new(Severity::Error, code, message)
    }

    fn warning(code: ProjectValidationIssueCode, message: impl Into<String>) -> Self {
        Self::new(Severity::Warning, code, message)
    }

    fn with_path(mut self, path: impl Into<Utf8PathBuf>) -> Self {
        self.path = Some(path.into());
        self
    }

    fn with_record_id(mut self, record_id: RecordId) -> Self {
        self.record_id = Some(record_id);
        self
    }

    fn with_record_code(mut self, record_code: DiagnosticCode) -> Self {
        self.record_code = Some(record_code);
        self
    }

    fn with_sub_code(mut self, code: impl Into<String>) -> Self {
        self.sub_code = Some(code.into());
        self
    }

    fn with_field(mut self, field: Option<String>) -> Self {
        self.field = field;
        self
    }

    fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    fn with_related_paths(mut self, paths: Vec<Utf8PathBuf>) -> Self {
        self.related_paths = paths;
        self
    }
}

pub fn validate_project_index(index: &ProjectIndex) -> ProjectValidationReport {
    let project = index.project();
    let scanned = index.scanned_records();
    let mut issues = Vec::new();

    add_template_issues(project, &mut issues);
    add_record_diagnostics(scanned, &mut issues);
    add_tag_issues(index, &mut issues);
    add_record_structure_issues(scanned, project, &mut issues);
    add_duplicate_id_issues(scanned, &mut issues);
    add_short_id_collision_issues(scanned, &mut issues);
    add_successor_graph_issues(scanned, &mut issues);
    add_supersedes_issues(index, &mut issues);
    add_link_issues(index, &mut issues);

    sort_issues(&mut issues);
    let counts = IssueCounts::from_severities(issues.iter().map(|issue| issue.severity));
    let ok = counts.errors == 0;

    ProjectValidationReport {
        project_dir: project.project_dir.clone(),
        ok,
        counts,
        issues,
    }
}

fn add_tag_issues(index: &ProjectIndex, issues: &mut Vec<ProjectValidationIssue>) {
    let report = build_tag_report(index);
    issues.extend(
        report
            .issues
            .into_iter()
            .map(project_validation_issue_from_tag_issue),
    );
}

fn project_validation_issue_from_tag_issue(issue: TagIssue) -> ProjectValidationIssue {
    let mut project_validation_issue = ProjectValidationIssue::new(
        issue.severity,
        ProjectValidationIssueCode::InvalidTag,
        issue.message,
    )
    .with_sub_code(issue.code.to_string())
    .with_path(issue.path)
    .with_field(Some("tags".to_owned()));
    if let Some(line) = issue.line {
        project_validation_issue = project_validation_issue.with_line(line);
    }
    if let Some(record_id) = issue.record_id {
        project_validation_issue = project_validation_issue.with_record_id(record_id);
    }
    project_validation_issue
}

fn add_template_issues(project: &ProjectContext, issues: &mut Vec<ProjectValidationIssue>) {
    let candidate_path = project.templates_dir().join("candidate.md");
    if candidate_path.exists() {
        if !candidate_path.is_file() {
            issues.push(
                ProjectValidationIssue::error(
                    ProjectValidationIssueCode::TemplateReadError,
                    "`dadr/templates/candidate.md` is not a file",
                )
                .with_path(candidate_path)
                .with_field(Some("templates/candidate.md".to_owned())),
            );
        } else if let Err(err) = fs::read_to_string(utf8_to_path(&candidate_path)) {
            issues.push(
                ProjectValidationIssue::error(
                    ProjectValidationIssueCode::TemplateReadError,
                    format!("failed to read candidate template: {err}"),
                )
                .with_path(candidate_path)
                .with_field(Some("templates/candidate.md".to_owned())),
            );
        }
    }
}

fn add_link_issues(index: &ProjectIndex, issues: &mut Vec<ProjectValidationIssue>) {
    let report = check_project_links(index);
    issues.extend(
        report
            .issues
            .into_iter()
            .map(project_validation_issue_from_link_issue),
    );
}

fn project_validation_issue_from_link_issue(issue: LinkIssue) -> ProjectValidationIssue {
    let mut project_validation_issue = ProjectValidationIssue::new(
        issue.severity,
        project_validation_code_from_link_code(issue.code),
        issue.message,
    )
    .with_sub_code(issue.code.to_string())
    .with_path(issue.path)
    .with_line(issue.line)
    .with_field(Some("body".to_owned()))
    .with_related_paths(issue.related_paths);
    if let Some(record_id) = issue.record_id {
        project_validation_issue = project_validation_issue.with_record_id(record_id);
    }
    project_validation_issue
}

fn project_validation_code_from_link_code(code: LinkIssueCode) -> ProjectValidationIssueCode {
    match code {
        LinkIssueCode::BrokenWikilink => ProjectValidationIssueCode::BrokenWikilink,
        LinkIssueCode::AmbiguousWikilink => ProjectValidationIssueCode::AmbiguousWikilink,
        LinkIssueCode::RejectedRecordWikilink => ProjectValidationIssueCode::RejectedRecordWikilink,
        LinkIssueCode::BrokenMarkdownLink => ProjectValidationIssueCode::BrokenMarkdownLink,
    }
}

fn add_record_diagnostics(scanned: &[ScannedRecord], issues: &mut Vec<ProjectValidationIssue>) {
    for record in scanned {
        for diagnostic in &record.diagnostics {
            issues.push(
                ProjectValidationIssue::new(
                    diagnostic.severity,
                    ProjectValidationIssueCode::RecordDiagnostic,
                    diagnostic.message.clone(),
                )
                .with_path(
                    diagnostic
                        .path
                        .clone()
                        .unwrap_or_else(|| record.path.clone()),
                )
                .with_record_code(diagnostic.code)
                .with_field(diagnostic.field.clone()),
            );
        }
    }
}

fn add_record_structure_issues(
    scanned: &[ScannedRecord],
    project: &ProjectContext,
    issues: &mut Vec<ProjectValidationIssue>,
) {
    for scanned_record in scanned {
        let Some(record) = &scanned_record.record else {
            continue;
        };
        let id = record.front_matter.id;

        if record.front_matter.status == RecordStatus::Accepted {
            let sections = check_required_sections(&record.body.raw_markdown);
            let problems = [
                (
                    ProjectValidationIssueCode::MissingRequiredSection,
                    "is missing required section(s)",
                    sections.missing,
                ),
                (
                    ProjectValidationIssueCode::DuplicateRequiredSection,
                    "repeats required section(s)",
                    sections.duplicated,
                ),
                (
                    ProjectValidationIssueCode::EmptyRequiredSection,
                    "has empty required section(s)",
                    sections.empty,
                ),
            ];
            for (code, description, names) in problems {
                if names.is_empty() {
                    continue;
                }
                issues.push(
                    ProjectValidationIssue::error(
                        code,
                        format!(
                            "`{}` {description}: {}",
                            record.front_matter.title,
                            names.join(", ")
                        ),
                    )
                    .with_path(scanned_record.path.clone())
                    .with_record_id(id),
                );
            }
        }

        if let Some(issue) = status_location_issue(project, scanned_record, record, id) {
            issues.push(issue);
        }
    }
}

fn status_location_issue(
    project: &ProjectContext,
    scanned: &ScannedRecord,
    record: &Record,
    id: RecordId,
) -> Option<ProjectValidationIssue> {
    let (expected_dir, message) = match record.front_matter.status {
        RecordStatus::Candidate => (
            project.inbox_dir(),
            "candidate records belong in the inbox, `dadr/inbox/`",
        ),
        RecordStatus::Accepted if record.front_matter.is_superseded() => (
            project.archive_dir(),
            "superseded records (accepted, with successors) belong in the archive, `dadr/archive/`",
        ),
        RecordStatus::Accepted => (
            project.ledger_dir(),
            "accepted records without successors belong in the ledger, `dadr/ledger/`",
        ),
        RecordStatus::Rejected => (
            project.archive_dir(),
            "rejected records belong in the archive, `dadr/archive/`",
        ),
    };
    if scanned.path.starts_with(&expected_dir) {
        return None;
    }

    Some(
        ProjectValidationIssue::warning(
            ProjectValidationIssueCode::StatusLocationMismatch,
            message,
        )
        .with_path(scanned.path.clone())
        .with_record_id(id),
    )
}

fn add_duplicate_id_issues(scanned: &[ScannedRecord], issues: &mut Vec<ProjectValidationIssue>) {
    let by_id = records_by_id(scanned);
    for (id, paths) in by_id {
        if paths.len() > 1 {
            issues.push(
                ProjectValidationIssue::error(
                    ProjectValidationIssueCode::DuplicateRecordId,
                    format!("record id `{id}` appears in multiple files"),
                )
                .with_record_id(id)
                .with_related_paths(paths),
            );
        }
    }
}

fn add_short_id_collision_issues(
    scanned: &[ScannedRecord],
    issues: &mut Vec<ProjectValidationIssue>,
) {
    let mut by_short_id = BTreeMap::<String, BTreeMap<RecordId, Vec<Utf8PathBuf>>>::new();
    for scanned_record in scanned {
        let Some(record) = &scanned_record.record else {
            continue;
        };
        let id = record.front_matter.id;
        by_short_id
            .entry(id.short_id())
            .or_default()
            .entry(id)
            .or_default()
            .push(scanned_record.path.clone());
    }

    // Records sharing a full id are reported as duplicate ids instead.
    for (short_id, ids) in by_short_id {
        if ids.len() < 2 {
            continue;
        }
        issues.push(
            ProjectValidationIssue::error(
                ProjectValidationIssueCode::ShortIdCollision,
                format!("multiple records share the short id `{short_id}`"),
            )
            .with_related_paths(ids.into_values().flatten().collect()),
        );
    }
}

fn add_successor_graph_issues(scanned: &[ScannedRecord], issues: &mut Vec<ProjectValidationIssue>) {
    let by_id = records_by_id(scanned);
    let status_by_id = record_statuses_by_id(scanned);
    let mut graph = BTreeMap::<RecordId, Vec<RecordId>>::new();
    let mut path_by_id = BTreeMap::<RecordId, Utf8PathBuf>::new();
    let mut title_by_id = BTreeMap::<RecordId, String>::new();

    for scanned_record in scanned {
        let Some(record) = &scanned_record.record else {
            continue;
        };
        let id = record.front_matter.id;
        graph
            .entry(id)
            .or_default()
            .extend(record.front_matter.successors.iter().copied());
        path_by_id
            .entry(id)
            .or_insert_with(|| scanned_record.path.clone());
        title_by_id
            .entry(id)
            .or_insert_with(|| record.front_matter.title.clone());

        if record.front_matter.status != RecordStatus::Accepted
            && !record.front_matter.successors.is_empty()
        {
            issues.push(
                ProjectValidationIssue::error(
                    ProjectValidationIssueCode::InvalidSuccessor,
                    format!(
                        "`{}` has successors, but only accepted records may supersede other decisions",
                        record.front_matter.title
                    ),
                )
                .with_path(scanned_record.path.clone())
                .with_record_id(id),
            );
        }

        for successor in &record.front_matter.successors {
            if !by_id.contains_key(successor) {
                issues.push(
                    ProjectValidationIssue::error(
                        ProjectValidationIssueCode::UnresolvedSuccessor,
                        format!(
                            "`{}` lists missing successor `{successor}`",
                            record.front_matter.title
                        ),
                    )
                    .with_path(scanned_record.path.clone())
                    .with_record_id(id),
                );
            } else if status_by_id.get(successor) != Some(&RecordStatus::Accepted) {
                let status = status_by_id
                    .get(successor)
                    .map(RecordStatus::as_str)
                    .unwrap_or("unknown");
                issues.push(
                    ProjectValidationIssue::error(
                        ProjectValidationIssueCode::InvalidSuccessor,
                        format!(
                            "`{}` lists successor `{successor}`, but successor status is `{status}`",
                            record.front_matter.title
                        ),
                    )
                    .with_path(scanned_record.path.clone())
                    .with_record_id(id),
                );
            }
        }
    }

    let mut cycle_ids = BTreeSet::new();
    for (id, successors) in &graph {
        for successor in successors {
            if reaches_record(&graph, *successor, *id, &mut BTreeSet::new()) {
                cycle_ids.insert(*id);
                break;
            }
        }
    }

    for id in cycle_ids {
        let title = title_by_id
            .get(&id)
            .map(String::as_str)
            .unwrap_or("<untitled>");
        let mut issue = ProjectValidationIssue::error(
            ProjectValidationIssueCode::SuccessorCycle,
            format!("`{title}` appears in its own transitive successor chain"),
        )
        .with_record_id(id);
        if let Some(path) = path_by_id.get(&id) {
            issue = issue.with_path(path.clone());
        }
        issues.push(issue);
    }
}

/// Check `## Supersedes` sections against the successor graph. A section
/// that cannot be read is an error, with the message acceptance refuses it
/// with. A candidate's section must name accepted records other than itself;
/// a predecessor that is not a record is reported by the link check, as a
/// broken wikilink.
fn add_supersedes_issues(index: &ProjectIndex, issues: &mut Vec<ProjectValidationIssue>) {
    for scanned_record in index.scanned_records() {
        let Some(record) = &scanned_record.record else {
            continue;
        };
        let id = record.front_matter.id;
        let title = &record.front_matter.title;
        let body = &record.body.raw_markdown;
        let section = match supersedes_section(body) {
            Ok(Some(section)) => section,
            Ok(None) => continue,
            Err(_) => {
                if let Some(diagnostic) = supersedes_diagnostic(body) {
                    issues.push(
                        ProjectValidationIssue::error(
                            ProjectValidationIssueCode::InvalidSupersedes,
                            format!("`{title}`: {}", diagnostic.message),
                        )
                        .with_path(scanned_record.path.clone())
                        .with_record_id(id)
                        .with_record_code(diagnostic.code)
                        .with_field(Some("body".to_owned())),
                    );
                }
                continue;
            }
        };
        for predecessor_id in section.ids {
            let candidate = record.front_matter.status == RecordStatus::Candidate;
            if candidate && predecessor_id == id {
                issues.push(
                    ProjectValidationIssue::error(
                        ProjectValidationIssueCode::InvalidSupersedes,
                        format!(
                            "`{title}` names itself in `## Supersedes`; a record cannot supersede itself"
                        ),
                    )
                    .with_path(scanned_record.path.clone())
                    .with_record_id(id)
                    .with_field(Some("body".to_owned())),
                );
                continue;
            }
            let Some(predecessor) = index.summary_by_id(predecessor_id) else {
                continue;
            };
            let issue = match record.front_matter.status {
                RecordStatus::Candidate if predecessor.status != RecordStatus::Accepted => {
                    ProjectValidationIssue::error(
                        ProjectValidationIssueCode::InvalidSupersedes,
                        format!(
                            "`{title}` names `{}` in `## Supersedes`, but `{}` is {}; only an accepted decision can be superseded",
                            predecessor.title, predecessor.title, predecessor.status
                        ),
                    )
                    .with_related_paths(vec![predecessor.path.clone()])
                }
                RecordStatus::Candidate if predecessor.superseded => {
                    let current = current_successors(index, predecessor_id);
                    let mut related = vec![predecessor.path.clone()];
                    related.extend(current.iter().map(|record| record.path.clone()));
                    ProjectValidationIssue::warning(
                        ProjectValidationIssueCode::StaleSupersedes,
                        format!(
                            "`{title}` supersedes `{}`, which is already superseded by {}; retarget `## Supersedes` at the current decision, or reject the candidate",
                            predecessor.title,
                            record_list(&current)
                        ),
                    )
                    .with_related_paths(related)
                }
                RecordStatus::Accepted if !lists_successor(index, predecessor_id, id) => {
                    ProjectValidationIssue::warning(
                        ProjectValidationIssueCode::UnlinkedSupersedes,
                        format!(
                            "`{title}` names `{}` in `## Supersedes`, but `{}` does not list it in `successors`",
                            predecessor.title, predecessor.title
                        ),
                    )
                    .with_related_paths(vec![predecessor.path.clone()])
                }
                _ => continue,
            };
            issues.push(
                issue
                    .with_path(scanned_record.path.clone())
                    .with_record_id(id)
                    .with_field(Some("body".to_owned())),
            );
        }
    }
}

fn lists_successor(index: &ProjectIndex, predecessor: RecordId, successor: RecordId) -> bool {
    index
        .scanned_by_id(predecessor)
        .and_then(|scanned| scanned.record.as_ref())
        .is_some_and(|record| record.front_matter.successors.contains(&successor))
}

fn records_by_id(scanned: &[ScannedRecord]) -> BTreeMap<RecordId, Vec<Utf8PathBuf>> {
    let mut by_id = BTreeMap::<RecordId, Vec<Utf8PathBuf>>::new();
    for scanned_record in scanned {
        let Some(record) = &scanned_record.record else {
            continue;
        };
        by_id
            .entry(record.front_matter.id)
            .or_default()
            .push(scanned_record.path.clone());
    }
    by_id
}

fn record_statuses_by_id(scanned: &[ScannedRecord]) -> BTreeMap<RecordId, RecordStatus> {
    let mut by_id = BTreeMap::<RecordId, RecordStatus>::new();
    for scanned_record in scanned {
        let Some(record) = &scanned_record.record else {
            continue;
        };
        by_id.insert(record.front_matter.id, record.front_matter.status);
    }
    by_id
}

fn reaches_record(
    graph: &BTreeMap<RecordId, Vec<RecordId>>,
    current_id: RecordId,
    target_id: RecordId,
    visited: &mut BTreeSet<RecordId>,
) -> bool {
    if current_id == target_id {
        return true;
    }
    if !visited.insert(current_id) {
        return false;
    }

    graph
        .get(&current_id)
        .into_iter()
        .flatten()
        .copied()
        .any(|successor| reaches_record(graph, successor, target_id, visited))
}

fn sort_issues(issues: &mut [ProjectValidationIssue]) {
    issues.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.record_id.cmp(&right.record_id))
            .then_with(|| left.message.cmp(&right.message))
    });
}
