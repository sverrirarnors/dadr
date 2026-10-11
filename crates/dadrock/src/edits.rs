//! Text edits to a record since it was accepted or rejected, read from Git.
//!
//! Decision history lives in the record set: supersession. Text history lives
//! in Git and is only read here, never copied into records. A record's edits
//! come from `git log --follow` on its file, which follows the moves between
//! `inbox/`, `ledger/`, and `archive/`. The baseline is the earliest commit in
//! which the record is accepted or rejected; every later commit whose title or
//! body (without `## Supersedes`, line endings normalized) differs from the
//! previous commit's is an edit. Changes to tags, `successors`, or the folder
//! alone are not edits. An uncommitted change in the working tree, against
//! `HEAD`, comes first.

use std::cell::OnceCell;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::{
    changes::is_record_path,
    error::DadrError,
    git::{FileCommit, GitRepo, GrepScope},
    history::{comparable_body, record_diff, RecordDiff},
    ids::RecordId,
    model::{Record, RecordStatus},
    records::parse_record_str,
};

/// Hex digits of a commit hash kept in `short_commit`.
const SHORT_COMMIT_LEN: usize = 12;

/// One edit to a record's title or body after it was accepted or rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordEdit {
    /// The commit that made the edit; `null` for an uncommitted edit in the
    /// working tree, as are `short_commit`, `author`, `date`, and `subject`.
    pub commit: Option<String>,
    pub short_commit: Option<String>,
    pub author: Option<String>,
    /// The author date, ISO 8601, from Git.
    pub date: Option<String>,
    /// The first line of the commit message.
    pub subject: Option<String>,
    pub title_changed: bool,
    pub body_changed: bool,
    /// The change from the previous version, when asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diff: Option<RecordDiff>,
}

/// Reads the text edits of the records of one project.
pub(crate) struct EditReader {
    project_dir: Utf8PathBuf,
    /// The repository and its `HEAD` commit (`None` before the first commit);
    /// `None` outside a repository or without `git`. Opened on first use.
    git: OnceCell<Option<(GitRepo, Option<String>)>>,
}

impl EditReader {
    pub(crate) fn new(project_dir: &Utf8Path) -> Self {
        Self {
            project_dir: project_dir.to_owned(),
            git: OnceCell::new(),
        }
    }

    /// The edits to `record`, stored at `path`, newest first; with `patch`,
    /// each carries its diff. Candidates have none. `None` when Git cannot be
    /// read: outside a repository, without `git`, or when a `git` command
    /// fails.
    pub(crate) fn edits(
        &self,
        record: &Record,
        path: &Utf8Path,
        patch: bool,
    ) -> Option<Vec<RecordEdit>> {
        if !is_decided(record) {
            return Some(Vec::new());
        }
        let (repo, head) = self
            .git
            .get_or_init(|| {
                let repo = GitRepo::open(&self.project_dir).ok()?;
                let head = repo.resolve_commit("HEAD").ok()?;
                Some((repo, head))
            })
            .as_ref()?;
        let Some(head) = head else {
            // No commits yet: nothing was accepted in Git.
            return Some(Vec::new());
        };
        self.read(repo, head, record, path, patch).ok()
    }

    fn read(
        &self,
        repo: &GitRepo,
        head: &str,
        record: &Record,
        path: &Utf8Path,
        patch: bool,
    ) -> Result<Vec<RecordEdit>, DadrError> {
        let id = record.front_matter.id;
        let Some((head_path, head_record)) = self.head_version(repo, head, id, path)? else {
            // Never committed.
            return Ok(Vec::new());
        };

        let log = repo.log_follow(head, &head_path)?;
        let objects: Vec<String> = log
            .iter()
            .filter_map(|commit| commit.object.clone())
            .collect();
        let mut contents = repo.blobs(&objects)?.into_iter();
        // Newest first: each commit with the record as it stood there.
        let mut versions: Vec<(&FileCommit, Option<Record>)> = Vec::with_capacity(log.len());
        for commit in &log {
            let content = match commit.object {
                Some(_) => contents.next().flatten(),
                None => None,
            };
            let version = content.and_then(|content| parse_record_str(&content).record);
            if version
                .as_ref()
                .is_some_and(|version| version.front_matter.id != id)
            {
                // Rename detection followed the file into another record.
                break;
            }
            versions.push((commit, version));
        }
        versions.reverse();

        let mut edits = Vec::new();
        let baseline = versions
            .iter()
            .position(|(_, version)| version.as_ref().is_some_and(is_decided));
        if let Some(baseline) = baseline {
            let mut previous = versions[baseline].1.as_ref();
            for (commit, version) in &versions[baseline + 1..] {
                let Some(version) = version else {
                    continue;
                };
                if let Some(edit) = previous.and_then(|old| edit(old, version, Some(commit), patch))
                {
                    edits.push(edit);
                }
                previous = Some(version);
            }
        }
        edits.reverse();
        if is_decided(&head_record) {
            if let Some(edit) = edit(&head_record, record, None, patch) {
                edits.insert(0, edit);
            }
        }
        Ok(edits)
    }

    /// The record's path from the repository root at `HEAD` and its version
    /// there: at its current path, or, when it moved since, wherever `HEAD`
    /// holds a record file with its id. `None` when `HEAD` has no such file.
    fn head_version(
        &self,
        repo: &GitRepo,
        head: &str,
        id: RecordId,
        path: &Utf8Path,
    ) -> Result<Option<(String, Record)>, DadrError> {
        let rel = path.strip_prefix(&self.project_dir).map_err(|_| {
            DadrError::ProjectInvariant(format!("`{path}` is outside `{}`", self.project_dir))
        })?;
        let git_path = repo.git_path(rel);
        let read = |git_path: &str| -> Result<Option<Record>, DadrError> {
            Ok(repo
                .blob(&format!("{head}:{git_path}"))?
                .and_then(|content| parse_record_str(&content).record)
                .filter(|record| record.front_matter.id == id))
        };
        if let Some(record) = read(&git_path)? {
            return Ok(Some((git_path, record)));
        }
        for other in repo.files_containing(&id.to_string(), GrepScope::Commit(head))? {
            if other == git_path
                || !repo
                    .rel_path(&other)
                    .is_some_and(|rel| is_record_path(&rel))
            {
                continue;
            }
            if let Some(record) = read(&other)? {
                return Ok(Some((other, record)));
            }
        }
        Ok(None)
    }
}

fn is_decided(record: &Record) -> bool {
    matches!(
        record.front_matter.status,
        RecordStatus::Accepted | RecordStatus::Rejected
    )
}

/// The edit from `old` to `new`, made by `commit` (`None` for the working
/// tree), or `None` when neither the title nor the body changed.
fn edit(
    old: &Record,
    new: &Record,
    commit: Option<&FileCommit>,
    patch: bool,
) -> Option<RecordEdit> {
    let title_changed = old.front_matter.title != new.front_matter.title;
    let body_changed =
        comparable_body(&old.body.raw_markdown) != comparable_body(&new.body.raw_markdown);
    if !title_changed && !body_changed {
        return None;
    }
    Some(RecordEdit {
        commit: commit.map(|commit| commit.commit.clone()),
        short_commit: commit.map(|commit| commit.commit.chars().take(SHORT_COMMIT_LEN).collect()),
        author: commit.map(|commit| commit.author.clone()),
        date: commit.map(|commit| commit.date.clone()),
        subject: commit.map(|commit| commit.subject.clone()),
        title_changed,
        body_changed,
        diff: patch.then(|| record_diff(old, new)),
    })
}
