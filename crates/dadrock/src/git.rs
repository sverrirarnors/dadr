//! The Git repository holding a project, driven through the `git` command.
//!
//! Every command runs in the project's `dadr/` directory, with arguments passed
//! as argv (never through a shell), paths after `--` as literal pathspecs,
//! revisions resolved to commit hashes before they are used, and
//! `GIT_OPTIONAL_LOCKS=0`, so reading never takes the index lock.

use std::{
    io::{ErrorKind, Write},
    process::{Command, Output, Stdio},
};

use camino::{Utf8Path, Utf8PathBuf};

use crate::error::DadrError;

/// One changed file under `dadr/`, by paths relative to the repository root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DiffEntry {
    Modified { path: String },
    Renamed { from: String, to: String },
    Deleted { path: String },
}

impl DiffEntry {
    pub(crate) fn base_path(&self) -> &str {
        match self {
            Self::Modified { path } | Self::Deleted { path } => path,
            Self::Renamed { from, .. } => from,
        }
    }
}

/// Where `git grep` looks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GrepScope<'a> {
    /// The index.
    Index,
    /// The working tree, including untracked files.
    WorkingTree,
    /// The tree of a commit, by hash.
    Commit(&'a str),
}

/// A commit that touched a file, from `git log --follow`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileCommit {
    pub commit: String,
    /// The author date, strict ISO 8601.
    pub date: String,
    pub author: String,
    pub subject: String,
    /// The object holding the file's content in this commit, for
    /// [`GitRepo::blobs`]; `None` when the commit deleted the file.
    pub object: Option<String>,
}

/// The Git repository holding a project.
pub(crate) struct GitRepo {
    /// The `dadr/` directory; every command runs there.
    dir: Utf8PathBuf,
    /// `dadr/`'s path from the repository root, with a trailing slash, or
    /// empty when `dadr/` is the root.
    prefix: String,
}

impl GitRepo {
    /// The repository holding `project_dir`. Fails when Git is missing or
    /// `project_dir` is not inside a repository.
    pub(crate) fn open(project_dir: &Utf8Path) -> Result<Self, DadrError> {
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
    pub(crate) fn resolve_commit(&self, revision: &str) -> Result<Option<String>, DadrError> {
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

    /// Files under `dadr/` that differ between `commit` and the working tree,
    /// or the index with `cached`, with rename detection. Added and copied
    /// files have no version at `commit` and are left out.
    pub(crate) fn changed_files(
        &self,
        commit: &str,
        cached: bool,
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
        if cached {
            args.push("--cached");
        }
        args.extend([commit, "--", "."]);
        let output = self.checked_output(&args)?;
        parse_name_status(&stdout_text(&output)?)
    }

    /// The content of `object` (`<commit>:<path>`, `:0:<path>`, or a blob
    /// hash), or `None` when it does not exist.
    pub(crate) fn blob(&self, object: &str) -> Result<Option<String>, DadrError> {
        let output = self.output(&["cat-file", "blob", object])?;
        if !output.status.success() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(&output.stdout).into_owned()))
    }

    /// The contents of `objects`, read by one `git cat-file --batch`: `None`
    /// for each object that does not exist or is not a blob.
    pub(crate) fn blobs(&self, objects: &[String]) -> Result<Vec<Option<String>>, DadrError> {
        // `--batch` reads one object name per line.
        let sendable: Vec<&String> = objects
            .iter()
            .filter(|object| !object.contains(['\n', '\r']))
            .collect();
        if sendable.is_empty() {
            return Ok(vec![None; objects.len()]);
        }
        let mut child = self
            .command(&["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(spawn_error)?;
        let input: String = sendable
            .iter()
            .map(|object| format!("{object}\n"))
            .collect();
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| DadrError::Git("`git cat-file` did not accept its input".to_owned()))?;
        // Write from another thread so a large output cannot block the input.
        let writer = std::thread::spawn(move || stdin.write_all(input.as_bytes()));
        let output = child.wait_with_output()?;
        let written = writer
            .join()
            .map_err(|_| DadrError::Git("`git cat-file` input failed".to_owned()))?;
        if !output.status.success() {
            return Err(git_failure("cat-file", &output));
        }
        written?;
        let mut contents = parse_batch(&output.stdout, sendable.len())?.into_iter();
        Ok(objects
            .iter()
            .map(|object| {
                if object.contains(['\n', '\r']) {
                    None
                } else {
                    contents.next().flatten()
                }
            })
            .collect())
    }

    /// Paths under `dadr/`, from the repository root, of files that contain
    /// `needle`.
    pub(crate) fn files_containing(
        &self,
        needle: &str,
        scope: GrepScope<'_>,
    ) -> Result<Vec<String>, DadrError> {
        let mut args = vec!["grep", "--no-color"];
        match scope {
            GrepScope::Index => args.push("--cached"),
            GrepScope::WorkingTree => args.push("--untracked"),
            GrepScope::Commit(_) => {}
        }
        args.extend([
            "--full-name",
            "--files-with-matches",
            "-z",
            "--fixed-strings",
            "-e",
            needle,
        ]);
        if let GrepScope::Commit(commit) = scope {
            args.push(commit);
        }
        args.extend(["--", "."]);
        let output = self.output(&args)?;
        // `git grep` exits with 1 when nothing matches.
        let paths = match output.status.code() {
            Some(0) => stdout_text(&output)?,
            Some(1) => return Ok(Vec::new()),
            _ => return Err(git_failure("grep", &output)),
        };
        let tree_prefix = match scope {
            GrepScope::Commit(commit) => format!("{commit}:"),
            _ => String::new(),
        };
        Ok(paths
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(|path| path.strip_prefix(&tree_prefix).unwrap_or(path).to_owned())
            .collect())
    }

    /// The commits reachable from `commit` that touched the file at
    /// `git_path` (from the repository root), newest first, following renames
    /// with `git log --follow`.
    pub(crate) fn log_follow(
        &self,
        commit: &str,
        git_path: &str,
    ) -> Result<Vec<FileCommit>, DadrError> {
        let Some(rel) = self.rel_path(git_path) else {
            return Ok(Vec::new());
        };
        let output = self.checked_output(&[
            "-c",
            "diff.relative=false",
            "log",
            "--follow",
            "--no-color",
            "--no-show-signature",
            "--no-abbrev",
            "--raw",
            "-z",
            "--format=%x1e%H%x00%aI%x00%an%x00%s",
            commit,
            "--",
            rel.as_str(),
        ])?;
        parse_follow_log(&stdout_text(&output)?, git_path)
    }

    /// A path relative to `dadr/` for a path from the repository root.
    pub(crate) fn rel_path(&self, git_path: &str) -> Option<Utf8PathBuf> {
        git_path
            .strip_prefix(self.prefix.as_str())
            .filter(|rel| !rel.is_empty())
            .map(Utf8PathBuf::from)
    }

    /// A path from the repository root for a path relative to `dadr/`.
    pub(crate) fn git_path(&self, rel: &Utf8Path) -> String {
        format!("{}{rel}", self.prefix)
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new("git");
        command
            .args(args)
            .current_dir(&self.dir)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_LITERAL_PATHSPECS", "1");
        command
    }

    fn output(&self, args: &[&str]) -> Result<Output, DadrError> {
        self.command(args)
            .stdin(Stdio::null())
            .output()
            .map_err(spawn_error)
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

fn spawn_error(err: std::io::Error) -> DadrError {
    if err.kind() == ErrorKind::NotFound {
        DadrError::Git(
            "`git` was not found on PATH; comparing records with their Git history needs Git"
                .to_owned(),
        )
    } else {
        DadrError::Io(err)
    }
}

pub(crate) fn parse_name_status(output: &str) -> Result<Vec<DiffEntry>, DadrError> {
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

/// Parse `git log --follow --raw -z` with the format
/// `%x1e%H%x00%aI%x00%an%x00%s`. `start_path` is the file's path at the newest
/// commit; walking back, a rename switches to the older path.
fn parse_follow_log(output: &str, start_path: &str) -> Result<Vec<FileCommit>, DadrError> {
    let unexpected = |after: &str| {
        DadrError::Git(format!(
            "unexpected `git log --follow` output after `{after}`"
        ))
    };
    let mut tokens = output.split('\0').peekable();
    let mut path = start_path.to_owned();
    let mut commits = Vec::new();
    while let Some(token) = tokens.next() {
        let token = token.trim_start_matches('\n');
        if token.is_empty() {
            continue;
        }
        let commit = token
            .strip_prefix('\x1e')
            .ok_or_else(|| unexpected(token))?
            .to_owned();
        let mut field = || {
            tokens
                .next()
                .map(str::to_owned)
                .ok_or_else(|| unexpected(&commit))
        };
        let date = field()?;
        let author = field()?;
        let subject = field()?;

        let mut object = None;
        let mut raw_seen = false;
        while let Some(raw) = tokens.peek().map(|raw| raw.trim_start_matches('\n')) {
            let Some(raw) = raw.strip_prefix(':') else {
                break;
            };
            tokens.next();
            // `:<old mode> <new mode> <old blob> <new blob> <status>`
            let fields: Vec<&str> = raw.split(' ').collect();
            let (Some(new_blob), Some(status)) = (fields.get(3), fields.get(4)) else {
                return Err(unexpected(&commit));
            };
            let first = tokens.next().ok_or_else(|| unexpected(&commit))?;
            let (old_path, new_path) = if status.starts_with(['R', 'C']) {
                let second = tokens.next().ok_or_else(|| unexpected(&commit))?;
                (first, second)
            } else {
                (first, first)
            };
            if raw_seen {
                continue;
            }
            raw_seen = true;
            object = if status.starts_with('D') || new_blob.bytes().all(|byte| byte == b'0') {
                None
            } else {
                Some((*new_blob).to_owned())
            };
            // Older commits hold the file at its path before a rename.
            path = if status.starts_with(['R', 'C']) {
                old_path.to_owned()
            } else {
                new_path.to_owned()
            };
        }
        if !raw_seen {
            // A merge lists no changes; read the file from its tree.
            object = Some(format!("{commit}:{path}"));
        }
        commits.push(FileCommit {
            commit,
            date,
            author,
            subject,
            object,
        });
    }
    Ok(commits)
}

/// Parse `git cat-file --batch` output for `count` objects.
fn parse_batch(output: &[u8], count: usize) -> Result<Vec<Option<String>>, DadrError> {
    let unexpected = || DadrError::Git("unexpected `git cat-file --batch` output".to_owned());
    let mut contents = Vec::with_capacity(count);
    let mut rest = output;
    for _ in 0..count {
        let end = rest
            .iter()
            .position(|byte| *byte == b'\n')
            .ok_or_else(unexpected)?;
        let header = String::from_utf8_lossy(&rest[..end]).into_owned();
        rest = &rest[end + 1..];
        let fields: Vec<&str> = header.split(' ').collect();
        let size = match fields.as_slice() {
            [_, kind, size] if kind.chars().all(|ch| ch.is_ascii_lowercase()) => size
                .parse::<usize>()
                .ok()
                .map(|size| (size, *kind == "blob")),
            _ => None,
        };
        let Some((size, is_blob)) = size else {
            // `<object> missing` or `<object> ambiguous`.
            contents.push(None);
            continue;
        };
        if rest.len() < size {
            return Err(unexpected());
        }
        let (content, after) = rest.split_at(size);
        contents.push(is_blob.then(|| String::from_utf8_lossy(content).into_owned()));
        rest = after.strip_prefix(b"\n").unwrap_or(after);
    }
    Ok(contents)
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
    fn follow_log_tracks_blobs_and_renames() {
        let zeros = "0".repeat(40);
        let (a, b, c, d) = (
            "a".repeat(40),
            "b".repeat(40),
            "c".repeat(40),
            "d".repeat(40),
        );
        let output = format!(
            "\x1e{c}\x002026-10-02T10:00:00+02:00\0Alice\0supersede\0\n:100644 100644 {b} {b} R100\0dadr/ledger/x.md\0dadr/archive/x.md\0\
             \x1e{b}\x002026-10-01T10:00:00+02:00\0Bob\0\0\n:100644 100644 {a} {b} M\0dadr/ledger/x.md\0\
             \x1e{a}\x002026-09-30T10:00:00+02:00\0Alice\0merge\0\
             \x1e{d}\x002026-09-29T10:00:00+02:00\0Alice\0draft\0\n:000000 100644 {zeros} {a} A\0dadr/ledger/x.md\0"
        );
        let commits = parse_follow_log(&output, "dadr/archive/x.md").unwrap();
        assert_eq!(commits.len(), 4);
        assert_eq!(commits[0].commit, c);
        assert_eq!(commits[0].author, "Alice");
        assert_eq!(commits[0].object.as_deref(), Some(b.as_str()));
        assert_eq!(commits[1].subject, "");
        assert_eq!(commits[1].date, "2026-10-01T10:00:00+02:00");
        assert_eq!(
            commits[2].object.as_deref(),
            Some(format!("{a}:dadr/ledger/x.md").as_str())
        );
        assert_eq!(commits[3].object.as_deref(), Some(a.as_str()));
    }

    #[test]
    fn batch_output_parses_blobs_and_missing_objects() {
        let output = b"abc blob 5\nhello\nHEAD:nope missing\ndef blob 0\n\n";
        assert_eq!(
            parse_batch(output, 3).unwrap(),
            vec![Some("hello".to_owned()), None, Some(String::new())]
        );
    }
}
