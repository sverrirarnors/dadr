//! `dadrock validate changes`: accepted and rejected records against a Git
//! base revision.
//!
//! Once a record is accepted or rejected, its `id` and `status` never change
//! and entries are never removed from its `successors`: breaking one of these
//! rules is an error. Its title and body may take corrections, so an edit to
//! either is a warning that carries the diff, for a reviewer to tell a
//! correction from a change of decision; `--strict` makes those edits errors
//! too. `tags` and keys outside DADR may change. This module compares every
//! record file that changed since a base revision with its version at that
//! revision, following renames, by shelling out to `git`.

use std::{fmt, io::ErrorKind};

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::{
    commands::text_diff_lines,
    error::{DadrError, IssueCounts, Severity},
    git::{DiffEntry, GitRepo, GrepScope},
    history::{RecordDiff, TitleChange},
    ids::RecordId,
    model::{Record, RecordStatus},
    project::{ProjectContext, ARCHIVE_DIR, INBOX_DIR, LEDGER_DIR},
    records::parse_record_str,
    text::write_serde_label,
};

/// The revision compared when no base is given.
pub const DEFAULT_BASE: &str = "HEAD";

/// What the base revision is compared with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeTarget {
    /// Files on disk, staged or not.
    WorkingTree,
    /// The Git index: what the next commit would contain.
    Index,
}

impl ChangeTarget {
    pub fn label(self) -> &'static str {
        match self {
            Self::WorkingTree => "working tree",
            Self::Index => "staged changes",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeValidationReport {
    pub project_dir: Utf8PathBuf,
    /// The base revision as given, such as `HEAD` or `origin/main`.
    pub base: String,
    /// The commit `base` resolved to; `null` when the repository has no
    /// commits yet, so there is nothing to compare.
    pub base_commit: Option<String>,
    pub target: ChangeTarget,
    /// Whether title and body edits are reported as errors.
    pub strict: bool,
    pub ok: bool,
    pub counts: IssueCounts,
    /// Changed record files that were accepted or rejected at the base.
    pub checked: Vec<CheckedRecordChange>,
    pub issues: Vec<ChangeIssue>,
}

impl ChangeValidationReport {
    pub fn has_errors(&self) -> bool {
        self.counts.has_errors()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordChangeKind {
    Modified,
    Moved,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckedRecordChange {
    pub kind: RecordChangeKind,
    pub record_id: RecordId,
    pub title: String,
    pub base_status: RecordStatus,
    pub base_path: Utf8PathBuf,
    /// Where the record is now; absent when it was deleted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeIssueCode {
    RecordIdChanged,
    RecordStatusChanged,
    SuccessorRemoved,
    InvalidRecord,
    RecordTitleEdited,
    RecordBodyEdited,
    RecordDeleted,
}

impl fmt::Display for ChangeIssueCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_serde_label(self, formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeIssue {
    pub severity: Severity,
    pub code: ChangeIssueCode,
    pub message: String,
    /// The record's current path, or its path at the base when it was deleted.
    pub path: Utf8PathBuf,
    /// The record's path at the base, when it moved.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_path: Option<Utf8PathBuf>,
    pub record_id: RecordId,
    /// The front-matter key or `body` that changed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    /// For a title or body edit, what changed: `title` for
    /// `record_title_edited`, the body's `lines` for `record_body_edited`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<RecordDiff>,
}

/// Compare the project's record files with their versions at `base`. With
/// `strict`, title and body edits are errors instead of warnings.
///
/// Fails when Git is missing, the project is not inside a Git repository, or
/// `base` does not resolve to a commit. A repository without commits has
/// nothing to compare and passes.
pub fn check_record_changes(
    project: &ProjectContext,
    base: &str,
    target: ChangeTarget,
    strict: bool,
) -> Result<ChangeValidationReport, DadrError> {
    let base = base.trim();
    if base.is_empty() || base.starts_with('-') {
        return Err(DadrError::InvalidInput(format!(
            "`{base}` is not a Git revision; pass a branch, tag, or commit such as `HEAD` or `origin/main`"
        )));
    }
    let repo = GitRepo::open(&project.project_dir)?;
    let mut report = ChangeValidationReport {
        project_dir: project.project_dir.clone(),
        base: base.to_owned(),
        base_commit: None,
        target,
        strict,
        ok: true,
        counts: IssueCounts::default(),
        checked: Vec::new(),
        issues: Vec::new(),
    };

    let Some(commit) = repo.resolve_commit(base)? else {
        if base == DEFAULT_BASE {
            // An unborn HEAD: no commit has recorded any decision yet.
            return Ok(report);
        }
        return Err(DadrError::Git(format!(
            "base revision `{base}` does not resolve to a commit; fetch it first or pass another `--base`"
        )));
    };

    let checker = ChangeChecker {
        repo: &repo,
        project_dir: &project.project_dir,
        base,
        commit: &commit,
        target,
        strict,
    };
    for entry in repo.changed_files(&commit, target == ChangeTarget::Index)? {
        checker.check_entry(&entry, &mut report)?;
    }

    report.base_commit = Some(commit);
    report.issues.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.message.cmp(&right.message))
    });
    report
        .checked
        .sort_by(|left, right| left.base_path.cmp(&right.base_path));
    report.counts = IssueCounts::from_severities(report.issues.iter().map(|issue| issue.severity));
    report.ok = !report.counts.has_errors();
    Ok(report)
}

/// A record that was accepted or rejected at the base.
struct DecidedRecord {
    record: Record,
    /// Path relative to `dadr/`.
    rel_path: Utf8PathBuf,
}

struct ChangeChecker<'a> {
    repo: &'a GitRepo,
    project_dir: &'a Utf8Path,
    base: &'a str,
    commit: &'a str,
    target: ChangeTarget,
    strict: bool,
}

impl ChangeChecker<'_> {
    fn check_entry(
        &self,
        entry: &DiffEntry,
        report: &mut ChangeValidationReport,
    ) -> Result<(), DadrError> {
        let Some(base) = self.decided_base_record(entry.base_path())? else {
            return Ok(());
        };
        let id = base.record.front_matter.id;

        let current = match entry {
            DiffEntry::Modified { path } => match self.read_current(path)? {
                Some(content) => Some((path.clone(), content)),
                None => self.find_current_by_id(id)?,
            },
            DiffEntry::Renamed { to, .. } => {
                let renamed = match self.repo.rel_path(to) {
                    Some(rel) if is_record_path(&rel) => self.read_current(to)?,
                    _ => None,
                };
                match renamed {
                    // Rename detection pairs files by similarity, so a
                    // successor drafted from this record can look like its
                    // rename. Keep the pair only when the id still matches.
                    Some(content)
                        if parse_record_str(&content)
                            .record
                            .is_some_and(|record| record.front_matter.id != id) =>
                    {
                        self.find_current_by_id(id)?
                    }
                    Some(content) => Some((to.clone(), content)),
                    None => self.find_current_by_id(id)?,
                }
            }
            // A record moved with a plain `mv` shows up as a deletion until the
            // new file is staged; look for the same id elsewhere.
            DiffEntry::Deleted { .. } => self.find_current_by_id(id)?,
        };

        let base_path = self.project_dir.join(&base.rel_path);
        let Some((current_git_path, content)) = current else {
            report.checked.push(CheckedRecordChange {
                kind: RecordChangeKind::Deleted,
                record_id: id,
                title: base.record.front_matter.title.clone(),
                base_status: base.record.front_matter.status,
                base_path: base_path.clone(),
                path: None,
            });
            report.issues.push(ChangeIssue {
                severity: Severity::Warning,
                code: ChangeIssueCode::RecordDeleted,
                message: format!(
                    "{} was deleted since {}. {}",
                    describe(&base.record),
                    self.base,
                    deletion_advice(&base.record)
                ),
                path: base_path,
                base_path: None,
                record_id: id,
                field: None,
                diff: None,
            });
            return Ok(());
        };

        let current_rel = self
            .repo
            .rel_path(&current_git_path)
            .unwrap_or_else(|| Utf8PathBuf::from(&current_git_path));
        let path = self.project_dir.join(&current_rel);
        let moved = current_rel != base.rel_path;
        report.checked.push(CheckedRecordChange {
            kind: if moved {
                RecordChangeKind::Moved
            } else {
                RecordChangeKind::Modified
            },
            record_id: id,
            title: base.record.front_matter.title.clone(),
            base_status: base.record.front_matter.status,
            base_path: base_path.clone(),
            path: Some(path.clone()),
        });
        let issue = |finding: Finding| ChangeIssue {
            severity: finding.severity,
            code: finding.code,
            message: finding.message,
            path: path.clone(),
            base_path: moved.then(|| base_path.clone()),
            record_id: id,
            field: finding.field.map(str::to_owned),
            diff: finding.diff,
        };

        let parsed = parse_record_str(&content);
        let Some(record) = parsed.record else {
            let reason = parsed
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.is_error())
                .map_or_else(
                    || "its front matter is incomplete".to_owned(),
                    |diagnostic| diagnostic.message.clone(),
                );
            report.issues.push(issue(Finding::error(
                ChangeIssueCode::InvalidRecord,
                None,
                format!(
                    "{} no longer parses as a record: {reason}.",
                    describe(&base.record)
                ),
            )));
            return Ok(());
        };

        for finding in self.findings(&base.record, &record) {
            report.issues.push(issue(finding));
        }
        Ok(())
    }

    /// Broken rules (errors) and title or body edits (warnings, or errors
    /// with `--strict`) from `base` to `current`.
    fn findings(&self, base: &Record, current: &Record) -> Vec<Finding> {
        let before = &base.front_matter;
        let after = &current.front_matter;
        let record = describe(base);
        let since = self.base;
        let advice = rule_advice(base);
        let mut findings = Vec::new();

        if after.id != before.id {
            findings.push(Finding::error(
                ChangeIssueCode::RecordIdChanged,
                Some("id"),
                format!(
                    "the id of {record} changed to `{}` since {since}. {advice}",
                    after.id
                ),
            ));
        }
        if after.status != before.status {
            findings.push(Finding::error(
                ChangeIssueCode::RecordStatusChanged,
                Some("status"),
                format!(
                    "the status of {record} changed to `{}` since {since}. {advice}",
                    after.status
                ),
            ));
        }
        let removed = before
            .successors
            .iter()
            .filter(|successor| !after.successors.contains(successor))
            .map(|successor| format!("`{successor}`"))
            .collect::<Vec<_>>();
        if !removed.is_empty() {
            findings.push(Finding::error(
                ChangeIssueCode::SuccessorRemoved,
                Some("successors"),
                format!(
                    "{record} lost successor(s) {} since {since}. Entries can be added to `successors` but never removed.",
                    removed.join(", ")
                ),
            ));
        }

        let edit_severity = if self.strict {
            Severity::Error
        } else {
            Severity::Warning
        };
        let edit_advice = edit_advice(base, self.strict);
        if after.title != before.title {
            findings.push(Finding {
                severity: edit_severity,
                code: ChangeIssueCode::RecordTitleEdited,
                field: Some("title"),
                message: format!("the title of {record} was edited since {since}. {edit_advice}"),
                diff: Some(RecordDiff {
                    changed: true,
                    title: Some(TitleChange {
                        old: before.title.clone(),
                        new: after.title.clone(),
                    }),
                    tags_added: Vec::new(),
                    tags_removed: Vec::new(),
                    lines: Vec::new(),
                }),
            });
        }
        let old_body = normalize_line_endings(&base.body.raw_markdown);
        let new_body = normalize_line_endings(&current.body.raw_markdown);
        if new_body != old_body {
            findings.push(Finding {
                severity: edit_severity,
                code: ChangeIssueCode::RecordBodyEdited,
                field: Some("body"),
                message: format!("the body of {record} was edited since {since}. {edit_advice}"),
                diff: Some(RecordDiff {
                    changed: true,
                    title: None,
                    tags_added: Vec::new(),
                    tags_removed: Vec::new(),
                    lines: text_diff_lines(&old_body, &new_body),
                }),
            });
        }
        findings
    }

    /// The record at `git_path` in the base commit, when it was accepted or
    /// rejected there. Other files, and candidates, are free to change.
    fn decided_base_record(&self, git_path: &str) -> Result<Option<DecidedRecord>, DadrError> {
        let Some(rel_path) = self.repo.rel_path(git_path) else {
            return Ok(None);
        };
        if !is_record_path(&rel_path) {
            return Ok(None);
        }
        let Some(content) = self.repo.blob(&format!("{}:{git_path}", self.commit))? else {
            return Ok(None);
        };
        let Some(record) = parse_record_str(&content).record else {
            return Ok(None);
        };
        if record.front_matter.status == RecordStatus::Candidate {
            return Ok(None);
        }
        Ok(Some(DecidedRecord { record, rel_path }))
    }

    /// The current content of a file: from disk, or from the index with
    /// `--staged`. `None` when it is absent; lossy text when it is not UTF-8,
    /// so that it fails to parse rather than fail the command.
    fn read_current(&self, git_path: &str) -> Result<Option<String>, DadrError> {
        match self.target {
            ChangeTarget::Index => self.repo.blob(&format!(":0:{git_path}")),
            ChangeTarget::WorkingTree => {
                let Some(rel) = self.repo.rel_path(git_path) else {
                    return Ok(None);
                };
                match std::fs::read(self.project_dir.join(rel)) {
                    Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
                    Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
                    Err(err) => Err(err.into()),
                }
            }
        }
    }

    /// The current record file, other than its old path, whose front matter
    /// has `id`: how a record moved between folders is found when Git did not
    /// pair the move as a rename.
    fn find_current_by_id(&self, id: RecordId) -> Result<Option<(String, String)>, DadrError> {
        let scope = match self.target {
            ChangeTarget::Index => GrepScope::Index,
            ChangeTarget::WorkingTree => GrepScope::WorkingTree,
        };
        for git_path in self.repo.files_containing(&id.to_string(), scope)? {
            let Some(rel) = self.repo.rel_path(&git_path) else {
                continue;
            };
            if !is_record_path(&rel) {
                continue;
            }
            let Some(content) = self.read_current(&git_path)? else {
                continue;
            };
            if parse_record_str(&content)
                .record
                .is_some_and(|record| record.front_matter.id == id)
            {
                return Ok(Some((git_path, content)));
            }
        }
        Ok(None)
    }
}

/// "accepted record `Title` (0193a0b7e911)".
fn describe(record: &Record) -> String {
    format!(
        "{} record `{}` ({})",
        state_label(record),
        record.front_matter.title,
        record.front_matter.id.short_id()
    )
}

fn state_label(record: &Record) -> &'static str {
    if record.front_matter.is_superseded() {
        "superseded"
    } else {
        record.front_matter.status.as_str()
    }
}

/// One issue for a changed record, before its paths are attached.
struct Finding {
    severity: Severity,
    code: ChangeIssueCode,
    field: Option<&'static str>,
    message: String,
    diff: Option<RecordDiff>,
}

impl Finding {
    fn error(code: ChangeIssueCode, field: Option<&'static str>, message: String) -> Self {
        Self {
            severity: Severity::Error,
            code,
            field,
            message,
            diff: None,
        }
    }
}

/// What to do instead of changing a record's `id` or `status`.
fn rule_advice(record: &Record) -> &'static str {
    if record.front_matter.is_superseded() {
        "A superseded record keeps its id and status: to change the decision, draft a successor to its current version instead."
    } else if record.front_matter.status == RecordStatus::Rejected {
        "A rejected record keeps its id and status: to revisit the decision, draft a new candidate instead."
    } else {
        "An accepted record keeps its id and status: to change the decision, draft a successor candidate instead."
    }
}

/// What a title or body edit asks of the reviewer.
fn edit_advice(record: &Record, strict: bool) -> String {
    let instead = if record.front_matter.is_superseded() {
        "If this changes the decision, supersede its current version instead."
    } else if record.front_matter.status == RecordStatus::Rejected {
        "If this changes what was rejected, draft a new candidate instead."
    } else {
        "If this changes the decision, supersede the record instead."
    };
    if strict {
        format!("With `--strict`, accepted and rejected text must not change. {instead}")
    } else {
        instead.to_owned()
    }
}

fn deletion_advice(record: &Record) -> &'static str {
    if record.front_matter.status == RecordStatus::Rejected {
        "Rejected records stay in `dadr/archive/` as the record of what was decided against."
    } else {
        "Accepted records stay in the project: when superseded, they move to `dadr/archive/`."
    }
}

fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Whether a path relative to `dadr/` is a Markdown file that project scanning
/// reads as a record: under `inbox/`, `ledger/`, or `archive/`, outside the
/// folders the scanner skips.
pub(crate) fn is_record_path(rel: &Utf8Path) -> bool {
    if rel.extension() != Some("md") {
        return false;
    }
    let components = rel.components().collect::<Vec<_>>();
    let Some((_, dirs)) = components.split_last() else {
        return false;
    };
    let Some((Utf8Component::Normal(top), nested)) = dirs.split_first() else {
        return false;
    };
    [INBOX_DIR, LEDGER_DIR, ARCHIVE_DIR].contains(top)
        && nested.iter().all(|component| match component {
            Utf8Component::Normal(name) => {
                !name.starts_with('.') && !matches!(*name, "_reports" | "_site" | "templates")
            }
            _ => false,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_paths_are_markdown_files_in_the_record_folders() {
        for path in [
            "inbox/0193-a.md",
            "ledger/0193-a.md",
            "archive/0193-a.md",
            "ledger/nested/0193-a.md",
        ] {
            assert!(is_record_path(Utf8Path::new(path)), "{path}");
        }
        for path in [
            "templates/candidate.md",
            "tasks/x/prompt.md",
            "runs/r/result.md",
            "README.md",
            "ledger/notes.txt",
            "ledger/.hidden/0193-a.md",
            "ledger/templates/0193-a.md",
            "accepted/0193-a.md",
        ] {
            assert!(!is_record_path(Utf8Path::new(path)), "{path}");
        }
    }

    #[test]
    fn line_endings_are_normalized() {
        assert_eq!(normalize_line_endings("a\r\nb\rc\n"), "a\nb\nc\n");
    }

    #[test]
    fn issue_codes_use_snake_case_labels() {
        assert_eq!(
            ChangeIssueCode::RecordBodyEdited.to_string(),
            "record_body_edited"
        );
        assert_eq!(
            serde_json::to_value(ChangeTarget::WorkingTree).unwrap(),
            "working_tree"
        );
    }
}
