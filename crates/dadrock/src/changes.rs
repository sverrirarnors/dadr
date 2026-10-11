//! `dadrock validate changes`: the immutability check against Git history.
//!
//! Once a record is accepted or rejected, its `id`, `status`, `title`, and
//! body never change, and entries are never removed from its `successors`.
//! `tags` and keys outside DADR may change. This module compares every record
//! file that changed since a base revision with its version at that revision,
//! following renames, by shelling out to `git`. Checking each change against
//! the previous revision is enough: the invariant carries forward.

use std::{
    fmt,
    io::ErrorKind,
    process::{Command, Output, Stdio},
};

use camino::{Utf8Component, Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::{
    error::{DadrError, IssueCounts, Severity},
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
    RecordTitleChanged,
    RecordBodyChanged,
    SuccessorRemoved,
    InvalidRecord,
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
}

/// Compare the project's record files with their versions at `base`.
///
/// Fails when Git is missing, the project is not inside a Git repository, or
/// `base` does not resolve to a commit. A repository without commits has
/// nothing to compare and passes.
pub fn check_record_changes(
    project: &ProjectContext,
    base: &str,
    target: ChangeTarget,
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
    };
    for entry in repo.changed_files(&commit, target)? {
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

/// One changed file under `dadr/`, by paths relative to the repository root.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DiffEntry {
    Modified { path: String },
    Renamed { from: String, to: String },
    Deleted { path: String },
}

impl DiffEntry {
    fn base_path(&self) -> &str {
        match self {
            Self::Modified { path } | Self::Deleted { path } => path,
            Self::Renamed { from, .. } => from,
        }
    }
}

/// A record frozen at the base: accepted or rejected.
struct FrozenRecord {
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
}

impl ChangeChecker<'_> {
    fn check_entry(
        &self,
        entry: &DiffEntry,
        report: &mut ChangeValidationReport,
    ) -> Result<(), DadrError> {
        let Some(base) = self.frozen_base_record(entry.base_path())? else {
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
        let issue = |code: ChangeIssueCode, field: Option<&str>, message: String| ChangeIssue {
            severity: Severity::Error,
            code,
            message,
            path: path.clone(),
            base_path: moved.then(|| base_path.clone()),
            record_id: id,
            field: field.map(str::to_owned),
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
            report.issues.push(issue(
                ChangeIssueCode::InvalidRecord,
                None,
                format!(
                    "{} no longer parses as a record: {reason}.",
                    describe(&base.record)
                ),
            ));
            return Ok(());
        };

        for (code, field, message) in self.violations(&base.record, &record) {
            report.issues.push(issue(code, Some(field), message));
        }
        Ok(())
    }

    fn violations(
        &self,
        base: &Record,
        current: &Record,
    ) -> Vec<(ChangeIssueCode, &'static str, String)> {
        let before = &base.front_matter;
        let after = &current.front_matter;
        let record = describe(base);
        let since = self.base;
        let advice = advice(base);
        let mut violations = Vec::new();

        if after.id != before.id {
            violations.push((
                ChangeIssueCode::RecordIdChanged,
                "id",
                format!(
                    "the id of {record} changed to `{}` since {since}. {advice}",
                    after.id
                ),
            ));
        }
        if after.status != before.status {
            violations.push((
                ChangeIssueCode::RecordStatusChanged,
                "status",
                format!(
                    "the status of {record} changed to `{}` since {since}. {advice}",
                    after.status
                ),
            ));
        }
        if after.title != before.title {
            violations.push((
                ChangeIssueCode::RecordTitleChanged,
                "title",
                format!(
                    "the title of {record} changed to `{}` since {since}. {advice}",
                    after.title
                ),
            ));
        }
        if normalize_line_endings(&current.body.raw_markdown)
            != normalize_line_endings(&base.body.raw_markdown)
        {
            violations.push((
                ChangeIssueCode::RecordBodyChanged,
                "body",
                format!("the body of {record} changed since {since}. {advice}"),
            ));
        }
        let removed = before
            .successors
            .iter()
            .filter(|successor| !after.successors.contains(successor))
            .map(|successor| format!("`{successor}`"))
            .collect::<Vec<_>>();
        if !removed.is_empty() {
            violations.push((
                ChangeIssueCode::SuccessorRemoved,
                "successors",
                format!(
                    "{record} lost successor(s) {} since {since}. Entries can be added to `successors` but never removed.",
                    removed.join(", ")
                ),
            ));
        }
        violations
    }

    /// The record at `git_path` in the base commit, when it was accepted or
    /// rejected there. Other files, and candidates, are free to change.
    fn frozen_base_record(&self, git_path: &str) -> Result<Option<FrozenRecord>, DadrError> {
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
        Ok(Some(FrozenRecord { record, rel_path }))
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
        for git_path in self.repo.files_containing(&id.to_string(), self.target)? {
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

fn advice(record: &Record) -> &'static str {
    if record.front_matter.is_superseded() {
        "Superseded records do not change: to change the decision, draft a successor to its current version instead."
    } else if record.front_matter.status == RecordStatus::Rejected {
        "Rejected records do not change: to revisit the decision, draft a new candidate instead."
    } else {
        "Accepted records do not change: to change the decision, draft a successor candidate instead."
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
fn is_record_path(rel: &Utf8Path) -> bool {
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

/// The Git repository holding a project, driven through the `git` command.
struct GitRepo {
    /// The `dadr/` directory; every command runs there.
    dir: Utf8PathBuf,
    /// `dadr/`'s path from the repository root, with a trailing slash, or
    /// empty when `dadr/` is the root.
    prefix: String,
}

impl GitRepo {
    fn open(project_dir: &Utf8Path) -> Result<Self, DadrError> {
        let mut repo = Self {
            dir: project_dir.to_owned(),
            prefix: String::new(),
        };
        let output = repo.output(&["rev-parse", "--show-prefix"])?;
        if !output.status.success() {
            return Err(DadrError::Git(format!(
                "`{project_dir}` is not inside a Git repository, so there is no history to compare records with ({})",
                stderr_line(&output)
            )));
        }
        repo.prefix = stdout_text(&output)?
            .trim_end_matches(['\n', '\r'])
            .to_owned();
        Ok(repo)
    }

    /// The commit `revision` names, or `None` when it names none.
    fn resolve_commit(&self, revision: &str) -> Result<Option<String>, DadrError> {
        let output = self.output(&[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{revision}^{{commit}}"),
        ])?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(Some(stdout_text(&output)?.trim().to_owned()))
    }

    /// Files under `dadr/` that differ between `commit` and the target, with
    /// rename detection. Added and copied files have no frozen base version
    /// and are left out.
    fn changed_files(
        &self,
        commit: &str,
        target: ChangeTarget,
    ) -> Result<Vec<DiffEntry>, DadrError> {
        let mut args = vec![
            "-c",
            "diff.relative=false",
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--name-status",
            "-z",
            "-M",
        ];
        if target == ChangeTarget::Index {
            args.push("--cached");
        }
        args.extend([commit, "--", "."]);
        let output = self.checked_output(&args)?;
        parse_name_status(&stdout_text(&output)?)
    }

    /// The content of `object` (`<commit>:<path>` or `:0:<path>`), or `None`
    /// when it does not exist.
    fn blob(&self, object: &str) -> Result<Option<String>, DadrError> {
        let output = self.output(&["cat-file", "blob", object])?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
    }

    /// Paths under `dadr/`, from the repository root, of files that contain
    /// `needle`: in the index, or in the working tree including untracked
    /// files.
    fn files_containing(
        &self,
        needle: &str,
        target: ChangeTarget,
    ) -> Result<Vec<String>, DadrError> {
        let scope = match target {
            ChangeTarget::Index => "--cached",
            ChangeTarget::WorkingTree => "--untracked",
        };
        let output = self.output(&[
            "grep",
            "--no-color",
            scope,
            "--full-name",
            "--files-with-matches",
            "-z",
            "--fixed-strings",
            "-e",
            needle,
            "--",
            ".",
        ])?;
        // `git grep` exits with 1 when nothing matches.
        match output.status.code() {
            Some(0) => Ok(stdout_text(&output)?
                .split('\0')
                .filter(|path| !path.is_empty())
                .map(str::to_owned)
                .collect()),
            Some(1) => Ok(Vec::new()),
            _ => Err(git_failure("grep", &output)),
        }
    }

    /// A path relative to `dadr/` for a path from the repository root.
    fn rel_path(&self, git_path: &str) -> Option<Utf8PathBuf> {
        git_path
            .strip_prefix(self.prefix.as_str())
            .filter(|rel| !rel.is_empty())
            .map(Utf8PathBuf::from)
    }

    fn output(&self, args: &[&str]) -> Result<Output, DadrError> {
        Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null())
            .output()
            .map_err(|err| {
                if err.kind() == ErrorKind::NotFound {
                    DadrError::Git(
                        "`git` was not found on PATH; `dadrock validate changes` needs Git"
                            .to_owned(),
                    )
                } else {
                    DadrError::Io(err)
                }
            })
    }

    fn checked_output(&self, args: &[&str]) -> Result<Output, DadrError> {
        let output = self.output(args)?;
        if output.status.success() {
            Ok(output)
        } else {
            let command = args
                .iter()
                .find(|arg| !arg.starts_with('-') && !arg.contains('='))
                .copied()
                .unwrap_or("git");
            Err(git_failure(command, &output))
        }
    }
}

fn parse_name_status(output: &str) -> Result<Vec<DiffEntry>, DadrError> {
    let mut fields = output.split('\0').filter(|field| !field.is_empty());
    let mut entries = Vec::new();
    while let Some(status) = fields.next() {
        let mut path = || {
            fields.next().map(str::to_owned).ok_or_else(|| {
                DadrError::Git(format!(
                    "unexpected `git diff --name-status` output after `{status}`"
                ))
            })
        };
        match status.chars().next() {
            Some('R') => {
                let from = path()?;
                let to = path()?;
                entries.push(DiffEntry::Renamed { from, to });
            }
            // A copy leaves its source in place; any change to the source is
            // listed on its own.
            Some('C') => {
                path()?;
                path()?;
            }
            Some('D') => entries.push(DiffEntry::Deleted { path: path()? }),
            Some('M' | 'T' | 'U') => entries.push(DiffEntry::Modified { path: path()? }),
            _ => {
                path()?;
            }
        }
    }
    Ok(entries)
}

fn stdout_text(output: &Output) -> Result<String, DadrError> {
    String::from_utf8(output.stdout.clone()).map_err(|_| DadrError::PathNotUtf8 {
        path: String::from_utf8_lossy(&output.stdout).into_owned(),
    })
}

fn stderr_line(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let line = stderr.trim();
    let line = line.strip_prefix("fatal: ").unwrap_or(line);
    if line.is_empty() {
        format!("git exited with {}", output.status)
    } else {
        line.to_owned()
    }
}

fn git_failure(command: &str, output: &Output) -> DadrError {
    DadrError::Git(format!("`git {command}` failed: {}", stderr_line(output)))
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
    fn name_status_output_parses_renames_deletions_and_modifications() {
        let output = "M\0dadr/ledger/a.md\0R087\0dadr/ledger/b.md\0dadr/archive/b.md\0D\0dadr/ledger/c.md\0A\0dadr/inbox/d.md\0C100\0dadr/ledger/e.md\0dadr/ledger/f.md\0";
        assert_eq!(
            parse_name_status(output).unwrap(),
            vec![
                DiffEntry::Modified {
                    path: "dadr/ledger/a.md".to_owned()
                },
                DiffEntry::Renamed {
                    from: "dadr/ledger/b.md".to_owned(),
                    to: "dadr/archive/b.md".to_owned()
                },
                DiffEntry::Deleted {
                    path: "dadr/ledger/c.md".to_owned()
                },
            ]
        );
    }

    #[test]
    fn line_endings_are_normalized() {
        assert_eq!(normalize_line_endings("a\r\nb\rc\n"), "a\nb\nc\n");
    }

    #[test]
    fn issue_codes_use_snake_case_labels() {
        assert_eq!(
            ChangeIssueCode::RecordBodyChanged.to_string(),
            "record_body_changed"
        );
        assert_eq!(
            serde_json::to_value(ChangeTarget::WorkingTree).unwrap(),
            "working_tree"
        );
    }
}
