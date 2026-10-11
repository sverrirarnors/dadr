//! `dadrock validate changes` against temporary Git repositories.
//!
//! Every `git` and `dadrock` process here runs with the user's global and
//! system Git configuration switched off and repository discovery stopped at
//! the temporary directory, so results do not depend on the machine.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use dadrock::{
    changes::{
        ChangeIssueCode, ChangeTarget, ChangeValidationReport, CheckedRecordChange,
        RecordChangeKind,
    },
    commands::{CommandEnvelope, TextDiffKind},
    project::init_project,
    RecordId, Severity,
};

const SECTIONS: &str = "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n\
                        ## Alternatives\n\nNone considered.\n\n## Consequences\n\nEffects.\n";

const ID_A: &str = "01938c20-a3b4-7000-8000-111122223333";
const ID_B: &str = "0193a0b7-e911-7000-8000-444455556666";
const ID_C: &str = "01939f2c-4d07-7000-8000-777788889999";

struct Repo {
    sandbox: PathBuf,
    root: PathBuf,
}

impl Repo {
    /// A Git repository holding an initialized `dadr/` project, with no
    /// commits yet.
    fn new(label: &str) -> Self {
        let sandbox =
            std::env::temp_dir().join(format!("dadrock-changes-{label}-{}", RecordId::new_v7()));
        let root = sandbox.join("repo");
        fs::create_dir_all(&root).unwrap();
        fs::write(sandbox.join("gitconfig"), "").unwrap();
        let repo = Self { sandbox, root };
        repo.git(&["init", "-q"]);
        for (key, value) in [
            ("user.name", "Dadrock Test"),
            ("user.email", "dadrock@example.com"),
            ("commit.gpgsign", "false"),
            ("core.autocrlf", "false"),
        ] {
            repo.git(&["config", key, value]);
        }
        init_project(&repo.root).unwrap();
        repo
    }

    /// A repository whose first commit holds `records`.
    fn with_records(label: &str, records: &[(&str, &str)]) -> Self {
        let repo = Self::new(label);
        for (path, content) in records {
            repo.write(path, content);
        }
        repo.commit_all("seed records");
        repo
    }

    fn command(&self, program: &str) -> Command {
        let mut command = Command::new(program);
        command
            .current_dir(&self.root)
            .env("GIT_CONFIG_GLOBAL", self.sandbox.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CEILING_DIRECTORIES", &self.sandbox)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .env_remove("GIT_INDEX_FILE");
        command
    }

    fn git(&self, args: &[&str]) -> String {
        let output = self.command("git").args(args).output().unwrap();
        assert!(
            output.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    fn commit_all(&self, message: &str) {
        self.git(&["add", "-A"]);
        self.git(&["commit", "-q", "--no-verify", "-m", message]);
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.root.join(rel)
    }

    fn write(&self, rel: &str, content: &str) {
        let path = self.path(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path(rel)).unwrap()
    }

    fn edit(&self, rel: &str, from: &str, to: &str) {
        let content = self.read(rel);
        assert!(content.contains(from), "`{from}` not in {rel}");
        self.write(rel, &content.replacen(from, to, 1));
    }

    fn dadrock(&self, args: &[&str]) -> Output {
        self.command(env!("CARGO_BIN_EXE_dadrock"))
            .args(args)
            .output()
            .unwrap()
    }

    fn lifecycle(&self, args: &[&str]) {
        let output = self.dadrock(args);
        assert!(
            output.status.success(),
            "dadrock {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    /// `dadrock validate changes --json` with `args`: the envelope and
    /// whether the process exited successfully.
    fn check(&self, args: &[&str]) -> (CommandEnvelope<ChangeValidationReport>, bool) {
        let mut full = vec!["validate", "changes", "--json"];
        full.extend_from_slice(args);
        let output = self.dadrock(&full);
        let envelope = serde_json::from_slice(&output.stdout).unwrap_or_else(|err| {
            panic!(
                "invalid JSON ({err}): stdout {} stderr {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        });
        (envelope, output.status.success())
    }

    /// Issue codes from `validate changes`, checking that the exit status
    /// agrees with the errors.
    fn codes(&self, args: &[&str]) -> Vec<ChangeIssueCode> {
        let (envelope, success) = self.check(args);
        let has_errors = envelope
            .data
            .issues
            .iter()
            .any(|issue| issue.severity == Severity::Error);
        assert_eq!(envelope.ok, !has_errors);
        assert_eq!(envelope.data.ok, !has_errors);
        assert_eq!(success, !has_errors, "exit status disagrees with errors");
        envelope
            .data
            .issues
            .iter()
            .map(|issue| issue.code)
            .collect()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.sandbox);
    }
}

fn record(title: &str, status: &str, id: &str, extra: &str, body: &str) -> String {
    format!("---\ntitle: {title}\nstatus: {status}\nid: {id}\n{extra}---\n\n{body}")
}

fn accepted(title: &str, id: &str) -> String {
    record(title, "accepted", id, "tags:\n  - topic/income\n", SECTIONS)
}

fn short(id: &str) -> String {
    RecordId::parse(id).unwrap().short_id()
}

fn ledger(id: &str, slug: &str) -> String {
    format!("dadr/ledger/{}-{slug}.md", short(id))
}

fn inbox(id: &str, slug: &str) -> String {
    format!("dadr/inbox/{}-{slug}.md", short(id))
}

fn archive(id: &str, slug: &str) -> String {
    format!("dadr/archive/{}-{slug}.md", short(id))
}

fn only_checked(report: &ChangeValidationReport) -> &CheckedRecordChange {
    assert_eq!(report.checked.len(), 1, "{:?}", report.checked);
    &report.checked[0]
}

#[test]
fn editing_an_accepted_body_is_a_warning_with_the_diff() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records(
        "accepted-body",
        &[(&path, &accepted("Winsorize income at p99", ID_A))],
    );
    repo.edit(&path, "What.", "Something else.");

    let (envelope, success) = repo.check(&[]);
    assert!(success);
    assert!(envelope.ok);
    assert_eq!(envelope.command, "validate changes");
    let report = envelope.data;
    assert_eq!(report.target, ChangeTarget::WorkingTree);
    assert_eq!(report.base, "HEAD");
    assert!(!report.strict);
    assert_eq!(report.counts.errors, 0);
    assert_eq!(report.counts.warnings, 1);
    let issue = &report.issues[0];
    assert_eq!(issue.code, ChangeIssueCode::RecordBodyEdited);
    assert_eq!(issue.severity, Severity::Warning);
    assert_eq!(issue.field.as_deref(), Some("body"));
    assert_eq!(issue.record_id.to_string(), ID_A);
    assert!(issue.path.as_str().ends_with(&path), "{}", issue.path);
    assert!(issue.message.contains("`Winsorize income at p99`"));
    assert!(issue.message.contains(&short(ID_A)));
    assert!(
        issue
            .message
            .contains("If this changes the decision, supersede the record instead."),
        "{}",
        issue.message
    );
    let diff = issue.diff.as_ref().expect("a body edit carries its diff");
    assert!(diff.changed);
    assert!(diff.title.is_none());
    assert!(diff
        .lines
        .iter()
        .any(|line| line.kind == TextDiffKind::Removed && line.text == "What."));
    assert!(diff
        .lines
        .iter()
        .any(|line| line.kind == TextDiffKind::Added && line.text == "Something else."));
    assert_eq!(only_checked(&report).kind, RecordChangeKind::Modified);

    let text = repo.dadrock(&["validate", "changes"]);
    assert!(text.status.success());
    let stdout = String::from_utf8(text.stdout).unwrap();
    assert!(stdout.contains("0 error(s), 1 warning(s)"), "{stdout}");
    assert!(stdout.contains("[warning] record_body_edited"), "{stdout}");
    assert!(stdout.contains(&path), "{stdout}");
    assert!(
        stdout.contains("      ## Decision\n\n    - What.\n    + Something else.\n"),
        "{stdout}"
    );
}

#[test]
fn editing_an_accepted_title_is_a_warning_with_old_and_new_titles() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("accepted-title", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(&path, "title: Winsorize", "title: Winsorise");

    assert_eq!(repo.codes(&[]), [ChangeIssueCode::RecordTitleEdited]);
    let (envelope, _) = repo.check(&[]);
    let issue = &envelope.data.issues[0];
    assert_eq!(issue.severity, Severity::Warning);
    assert_eq!(issue.field.as_deref(), Some("title"));
    let diff = issue.diff.as_ref().unwrap();
    let title = diff.title.as_ref().unwrap();
    assert_eq!(
        (title.old.as_str(), title.new.as_str()),
        ("Winsorize", "Winsorise")
    );
    assert!(diff.lines.is_empty());

    let stdout = String::from_utf8(repo.dadrock(&["validate", "changes"]).stdout).unwrap();
    assert!(stdout.contains("[warning] record_title_edited"), "{stdout}");
    assert!(
        stdout.contains("    title: Winsorize → Winsorise\n"),
        "{stdout}"
    );
}

#[test]
fn strict_reports_title_and_body_edits_as_errors() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("strict", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(&path, "title: Winsorize", "title: Winsorise");
    repo.edit(&path, "What.", "Something else.");

    let (envelope, success) = repo.check(&["--strict"]);
    assert!(!success);
    assert!(!envelope.ok);
    assert!(envelope.data.strict);
    assert_eq!(envelope.data.counts.errors, 2);
    let mut codes: Vec<_> = envelope
        .data
        .issues
        .iter()
        .map(|issue| issue.code)
        .collect();
    codes.sort();
    assert_eq!(
        codes,
        [
            ChangeIssueCode::RecordTitleEdited,
            ChangeIssueCode::RecordBodyEdited
        ]
    );
    for issue in &envelope.data.issues {
        assert_eq!(issue.severity, Severity::Error);
        assert!(issue.message.contains("`--strict`"), "{}", issue.message);
        assert!(issue.diff.is_some());
    }

    let text = repo.dadrock(&["validate", "changes", "--strict"]);
    assert!(!text.status.success());
    let stdout = String::from_utf8(text.stdout).unwrap();
    assert!(stdout.contains("2 error(s), 0 warning(s)"), "{stdout}");
    assert!(stdout.contains(", strict)"), "{stdout}");
    assert!(stdout.contains("[error] record_body_edited"), "{stdout}");
    assert!(stdout.contains("    + Something else."), "{stdout}");

    // Without --strict the same edits are warnings.
    let (envelope, success) = repo.check(&[]);
    assert!(success);
    assert_eq!(envelope.data.counts.warnings, 2);
}

#[test]
fn changing_an_accepted_id_is_an_error() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("accepted-id", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(&path, ID_A, ID_C);

    assert_eq!(repo.codes(&[]), [ChangeIssueCode::RecordIdChanged]);
}

#[test]
fn tags_and_keys_outside_dadr_may_change() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("tags", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(
        &path,
        "  - topic/income\n",
        "  - topic/outliers\n  - confidence/high\nowner: analysis team\n",
    );

    let (envelope, success) = repo.check(&[]);
    assert!(success);
    assert!(
        envelope.data.issues.is_empty(),
        "{:?}",
        envelope.data.issues
    );
    assert_eq!(
        only_checked(&envelope.data).kind,
        RecordChangeKind::Modified
    );
}

#[test]
fn adding_a_successor_is_allowed_and_removing_one_is_an_error() {
    let path = archive(ID_A, "winsorize");
    let repo = Repo::with_records(
        "successors",
        &[(
            &path,
            &record(
                "Winsorize",
                "accepted",
                ID_A,
                &format!("successors:\n  - {ID_B}\n"),
                SECTIONS,
            ),
        )],
    );

    repo.edit(
        &path,
        &format!("  - {ID_B}\n"),
        &format!("  - {ID_B}\n  - {ID_C}\n"),
    );
    assert!(repo.codes(&[]).is_empty());

    repo.edit(&path, &format!("  - {ID_B}\n"), "");
    let (envelope, _) = repo.check(&[]);
    assert_eq!(
        envelope
            .data
            .issues
            .iter()
            .map(|issue| issue.code)
            .collect::<Vec<_>>(),
        [ChangeIssueCode::SuccessorRemoved]
    );
    let issue = &envelope.data.issues[0];
    assert_eq!(issue.field.as_deref(), Some("successors"));
    assert!(issue.message.contains(ID_B), "{}", issue.message);
    assert!(
        issue.message.contains("superseded record"),
        "{}",
        issue.message
    );
}

#[test]
fn changing_an_accepted_status_is_an_error() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("status", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(&path, "status: accepted", "status: candidate");

    assert_eq!(repo.codes(&[]), [ChangeIssueCode::RecordStatusChanged]);
}

#[test]
fn editing_a_rejected_body_is_a_warning() {
    let path = archive(ID_A, "drop-outliers");
    let repo = Repo::with_records(
        "rejected-body",
        &[(
            &path,
            &record("Drop outliers", "rejected", ID_A, "", "Too aggressive.\n"),
        )],
    );
    repo.edit(&path, "Too aggressive.", "Fine after all.");

    let (envelope, success) = repo.check(&[]);
    assert!(success);
    let issue = &envelope.data.issues[0];
    assert_eq!(issue.code, ChangeIssueCode::RecordBodyEdited);
    assert_eq!(issue.severity, Severity::Warning);
    assert!(issue.message.contains("rejected record `Drop outliers`"));
    assert!(issue.message.contains("draft a new candidate"));
}

#[test]
fn hard_rules_stay_errors_next_to_edit_warnings() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("mixed", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(&path, "status: accepted", "status: candidate");
    repo.edit(&path, "What.", "Something else.");

    let (envelope, success) = repo.check(&[]);
    assert!(!success);
    let issues: Vec<_> = envelope
        .data
        .issues
        .iter()
        .map(|issue| (issue.severity, issue.code))
        .collect();
    assert_eq!(
        issues,
        [
            (Severity::Error, ChangeIssueCode::RecordStatusChanged),
            (Severity::Warning, ChangeIssueCode::RecordBodyEdited)
        ]
    );
    let status = &envelope.data.issues[0];
    assert!(status.diff.is_none());
    assert!(
        status.message.contains("keeps its id and status"),
        "{}",
        status.message
    );
}

#[test]
fn a_new_version_that_no_longer_parses_is_an_error() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("invalid", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(&path, "status: accepted\n", "");

    assert_eq!(repo.codes(&[]), [ChangeIssueCode::InvalidRecord]);
}

#[test]
fn deleting_an_accepted_record_is_a_warning() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("delete", &[(&path, &accepted("Winsorize", ID_A))]);
    fs::remove_file(repo.path(&path)).unwrap();

    let (envelope, success) = repo.check(&[]);
    assert!(success);
    assert!(envelope.ok);
    assert_eq!(envelope.data.counts.warnings, 1);
    let issue = &envelope.data.issues[0];
    assert_eq!(issue.code, ChangeIssueCode::RecordDeleted);
    assert_eq!(issue.severity, Severity::Warning);
    assert!(issue.path.as_str().ends_with(&path));
    let checked = only_checked(&envelope.data);
    assert_eq!(checked.kind, RecordChangeKind::Deleted);
    assert!(checked.path.is_none());

    repo.git(&["add", "-A"]);
    assert_eq!(repo.codes(&["--staged"]), [ChangeIssueCode::RecordDeleted]);
}

#[test]
fn candidates_and_files_outside_the_record_folders_change_freely() {
    let candidate = inbox(ID_A, "winsorize");
    let template = "dadr/templates/frozen.md";
    let repo = Repo::with_records(
        "candidates",
        &[
            (
                &candidate,
                &record("Winsorize", "candidate", ID_A, "", SECTIONS),
            ),
            (template, &accepted("Template", ID_B)),
        ],
    );
    repo.edit(&candidate, "title: Winsorize", "title: Trim");
    repo.edit(&candidate, "What.", "Trim at p95.");
    repo.edit(template, "What.", "Changed.");

    let (envelope, success) = repo.check(&[]);
    assert!(success);
    assert!(envelope.data.issues.is_empty());
    assert!(envelope.data.checked.is_empty());
}

#[test]
fn a_change_of_line_endings_alone_is_allowed() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("crlf", &[(&path, &accepted("Winsorize", ID_A))]);
    let crlf = repo.read(&path).replace('\n', "\r\n");
    repo.write(&path, &crlf);

    let (envelope, success) = repo.check(&[]);
    assert!(success);
    assert!(
        envelope.data.issues.is_empty(),
        "{:?}",
        envelope.data.issues
    );
    assert_eq!(
        only_checked(&envelope.data).kind,
        RecordChangeKind::Modified
    );
}

#[test]
fn accepting_a_candidate_passes() {
    let path = inbox(ID_A, "winsorize");
    let repo = Repo::with_records(
        "accept",
        &[(&path, &record("Winsorize", "candidate", ID_A, "", SECTIONS))],
    );
    repo.lifecycle(&["accept", &short(ID_A)]);
    assert!(repo.path(&ledger(ID_A, "winsorize")).is_file());

    assert!(repo.codes(&[]).is_empty());
    repo.git(&["add", "-A"]);
    assert!(repo.codes(&["--staged"]).is_empty());
}

#[test]
fn rejecting_a_candidate_passes() {
    let path = inbox(ID_A, "winsorize");
    let repo = Repo::with_records(
        "reject",
        &[(&path, &record("Winsorize", "candidate", ID_A, "", SECTIONS))],
    );
    repo.lifecycle(&["reject", &short(ID_A)]);
    assert!(repo.path(&archive(ID_A, "winsorize")).is_file());

    assert!(repo.codes(&[]).is_empty());
    repo.git(&["add", "-A"]);
    assert!(repo.codes(&["--staged"]).is_empty());
}

#[test]
fn superseding_moves_the_predecessor_to_the_archive_and_passes() {
    let predecessor = ledger(ID_A, "winsorize");
    let successor = inbox(ID_B, "trim");
    let repo = Repo::with_records(
        "supersede",
        &[
            (&predecessor, &accepted("Winsorize", ID_A)),
            (
                &successor,
                &record(
                    "Trim",
                    "candidate",
                    ID_B,
                    "",
                    &format!(
                        "## Supersedes\n\n[[{ID_A}|Winsorize]]\n\nWinsorizing kept implausible values.\n\n{SECTIONS}"
                    ),
                ),
            ),
        ],
    );
    repo.lifecycle(&["accept", &short(ID_B)]);
    let archived = archive(ID_A, "winsorize");
    assert!(repo.read(&archived).contains(ID_B));
    assert!(!repo.path(&predecessor).exists());

    // Before staging, the archived file is untracked; the record is found by id.
    let (envelope, success) = repo.check(&[]);
    assert!(success);
    assert!(
        envelope.data.issues.is_empty(),
        "{:?}",
        envelope.data.issues
    );
    let checked = only_checked(&envelope.data);
    assert_eq!(checked.kind, RecordChangeKind::Moved);
    assert!(checked.base_path.as_str().ends_with(&predecessor));
    assert!(checked.path.as_ref().unwrap().as_str().ends_with(&archived));

    repo.git(&["add", "-A"]);
    let (envelope, success) = repo.check(&["--staged"]);
    assert!(success);
    assert!(
        envelope.data.issues.is_empty(),
        "{:?}",
        envelope.data.issues
    );
    assert_eq!(only_checked(&envelope.data).kind, RecordChangeKind::Moved);
}

#[test]
fn moving_and_editing_an_accepted_record_reports_both_paths() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("move-edit", &[(&path, &accepted("Winsorize", ID_A))]);
    let moved = archive(ID_A, "winsorize");
    repo.git(&["mv", &path, &moved]);
    repo.edit(&moved, "Effects.", "Other effects.");

    let (envelope, _) = repo.check(&[]);
    let issue = &envelope.data.issues[0];
    assert_eq!(issue.code, ChangeIssueCode::RecordBodyEdited);
    assert!(issue.path.as_str().ends_with(&moved));
    assert!(issue.base_path.as_ref().unwrap().as_str().ends_with(&path));
}

#[test]
fn a_similar_new_record_is_not_mistaken_for_a_rename() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("similar", &[(&path, &accepted("Winsorize", ID_A))]);
    fs::remove_file(repo.path(&path)).unwrap();
    repo.write(&ledger(ID_B, "winsorize"), &accepted("Winsorize", ID_B));
    repo.git(&["add", "-A"]);

    assert_eq!(repo.codes(&["--staged"]), [ChangeIssueCode::RecordDeleted]);
}

#[test]
fn staged_compares_the_index_and_ignores_unstaged_edits() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("staged", &[(&path, &accepted("Winsorize", ID_A))]);

    repo.edit(&path, "topic/income", "topic/outliers");
    repo.git(&["add", &path]);
    repo.edit(&path, "What.", "Unstaged edit.");
    assert!(repo.codes(&["--staged"]).is_empty());
    assert_eq!(repo.codes(&[]), [ChangeIssueCode::RecordBodyEdited]);

    repo.git(&["add", &path]);
    repo.edit(&path, "Unstaged edit.", "What.");
    assert_eq!(
        repo.codes(&["--staged"]),
        [ChangeIssueCode::RecordBodyEdited]
    );
    assert!(repo.codes(&[]).is_empty());
}

#[test]
fn base_can_name_an_older_commit() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("base", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(&path, "What.", "Committed edit.");
    repo.commit_all("edit an accepted record");

    assert!(repo.codes(&[]).is_empty());
    let (envelope, success) = repo.check(&["--base", "HEAD~1"]);
    assert!(success);
    assert_eq!(envelope.data.base, "HEAD~1");
    let first = repo.git(&["rev-parse", "HEAD~1"]);
    assert_eq!(envelope.data.base_commit.as_deref(), Some(first.trim()));
    assert_eq!(
        envelope.data.issues[0].code,
        ChangeIssueCode::RecordBodyEdited
    );
    let (_, success) = repo.check(&["--base", "HEAD~1", "--strict"]);
    assert!(!success);
}

#[test]
fn an_unknown_base_is_an_error() {
    let repo = Repo::with_records("unknown-base", &[]);
    let output = repo.dadrock(&["validate", "changes", "--base", "no-such-branch"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("`no-such-branch` does not resolve"),
        "{stderr}"
    );
}

#[test]
fn a_repository_without_commits_has_nothing_to_compare() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::new("unborn");
    repo.write(&path, &accepted("Winsorize", ID_A));

    for args in [&[][..], &["--staged"]] {
        let (envelope, success) = repo.check(args);
        assert!(success);
        assert!(envelope.ok);
        assert!(envelope.data.base_commit.is_none());
        assert!(envelope.data.issues.is_empty());
    }
    repo.git(&["add", "-A"]);
    assert!(repo.codes(&["--staged"]).is_empty());

    let text = repo.dadrock(&["validate", "changes"]);
    assert!(text.status.success());
    assert!(String::from_utf8(text.stdout)
        .unwrap()
        .contains("nothing to compare"));
}

#[test]
fn outside_a_git_repository_the_command_fails_clearly() {
    let repo = Repo::new("no-git");
    fs::remove_dir_all(repo.root.join(".git")).unwrap();

    let output = repo.dadrock(&["validate", "changes"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("not inside a Git repository"), "{stderr}");

    // Ordinary validation does not need Git.
    assert!(repo.dadrock(&["validate"]).status.success());
}

#[test]
fn json_output_has_a_stable_shape() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("json", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(&path, "title: Winsorize", "title: Winsorise");

    let output = repo.dadrock(&["validate", "changes", "--json"]);
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["command"], "validate changes");
    assert_eq!(json["ok"], true);
    let data = &json["data"];
    let keys = |value: &serde_json::Value| {
        let mut keys = value
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    };
    assert_eq!(
        keys(data),
        [
            "base",
            "base_commit",
            "checked",
            "counts",
            "issues",
            "ok",
            "project_dir",
            "strict",
            "target"
        ]
    );
    assert_eq!(data["base"], "HEAD");
    assert_eq!(data["target"], "working_tree");
    assert_eq!(data["strict"], false);
    assert_eq!(data["counts"]["errors"], 0);
    assert_eq!(data["counts"]["warnings"], 1);
    assert_eq!(
        keys(&data["issues"][0]),
        [
            "code",
            "diff",
            "field",
            "message",
            "path",
            "record_id",
            "severity"
        ]
    );
    assert_eq!(data["issues"][0]["code"], "record_title_edited");
    assert_eq!(data["issues"][0]["severity"], "warning");
    assert_eq!(data["issues"][0]["field"], "title");
    assert_eq!(data["issues"][0]["record_id"], ID_A);
    assert_eq!(
        keys(&data["issues"][0]["diff"]),
        ["changed", "lines", "tags_added", "tags_removed", "title"]
    );
    assert_eq!(
        data["issues"][0]["diff"]["title"],
        serde_json::json!({"old": "Winsorize", "new": "Winsorise"})
    );
    assert_eq!(
        keys(&data["checked"][0]),
        [
            "base_path",
            "base_status",
            "kind",
            "path",
            "record_id",
            "title"
        ]
    );
    assert_eq!(data["checked"][0]["kind"], "modified");
    assert_eq!(data["checked"][0]["base_status"], "accepted");
}

#[cfg(unix)]
#[test]
fn a_pre_commit_hook_with_strict_blocks_commits_that_edit_accepted_records() {
    use std::os::unix::fs::PermissionsExt;

    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("hook", &[(&path, &accepted("Winsorize", ID_A))]);
    let hook = repo.root.join(".git/hooks/pre-commit");
    fs::create_dir_all(hook.parent().unwrap()).unwrap();
    let install = |flags: &str| {
        fs::write(
            &hook,
            format!(
                "#!/bin/sh\nexec '{}' validate changes --staged{flags}\n",
                env!("CARGO_BIN_EXE_dadrock")
            ),
        )
        .unwrap();
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    };
    let commit = |args: &[&str]| {
        repo.command("git")
            .args(["commit", "-q"])
            .args(args)
            .output()
            .unwrap()
    };
    install(" --strict");

    // A plain commit gives the hook a relative GIT_INDEX_FILE, while
    // `commit -a` gives it a temporary index; both must be read.
    repo.edit(&path, "What.", "Something else.");
    repo.git(&["add", &path]);
    let blocked = commit(&["-m", "edit an accepted record"]);
    assert!(!blocked.status.success());
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("record_body_edited"));
    repo.git(&["reset", "-q"]);
    let blocked = commit(&["-a", "-m", "edit an accepted record"]);
    assert!(!blocked.status.success());
    // Git sends hook output to stderr.
    let report = format!(
        "{}{}",
        String::from_utf8_lossy(&blocked.stdout),
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(report.contains("record_body_edited"), "{report}");

    repo.edit(&path, "Something else.", "What.");
    repo.edit(&path, "topic/income", "topic/outliers");
    let allowed = commit(&["-a", "-m", "retag an accepted record"]);
    assert!(
        allowed.status.success(),
        "{}",
        String::from_utf8_lossy(&allowed.stderr)
    );

    // Without --strict, a correction is committed with a warning, while a
    // changed status is still blocked.
    install("");
    repo.edit(&path, "Effects.", "Effects, spelled out.");
    let corrected = commit(&["-a", "-m", "correct an accepted record"]);
    assert!(
        corrected.status.success(),
        "{}",
        String::from_utf8_lossy(&corrected.stderr)
    );
    repo.edit(&path, "status: accepted", "status: rejected");
    let blocked = commit(&["-a", "-m", "reject an accepted record"]);
    assert!(!blocked.status.success());
}

#[test]
fn paths_in_the_report_are_inside_the_project() {
    let path = ledger(ID_A, "winsorize");
    let repo = Repo::with_records("paths", &[(&path, &accepted("Winsorize", ID_A))]);
    repo.edit(&path, "What.", "Changed.");
    let (envelope, _) = repo.check(&[]);
    let project_dir = PathBuf::from(envelope.data.project_dir.as_str());
    assert!(project_dir.ends_with(Path::new("repo/dadr")));
    assert!(envelope.data.issues[0]
        .path
        .as_std_path()
        .starts_with(&project_dir));
}
