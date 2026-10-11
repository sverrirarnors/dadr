//! `dadrock history`: text edits read from Git, in temporary repositories.
//!
//! Every `git` and `dadrock` process here runs with the user's global and
//! system Git configuration switched off and repository discovery stopped at
//! the temporary directory, so results do not depend on the machine.

use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

use dadrock::{project::init_project, RecordId};
use serde_json::{json, Value};

const SECTIONS: &str = "## Context\n\nIncome has a lomg right tail.\n\n## Decision\n\nWinsorize income at p99.\n\n\
                        ## Alternatives\n\nTrim at p95.\n\n## Consequences\n\nEstimates are less sensitive.\n";

const ID_A: &str = "01938c20-a3b4-7000-8000-111122223333";
const ID_B: &str = "0193a0b7-e911-7000-8000-444455556666";

struct Repo {
    sandbox: PathBuf,
    root: PathBuf,
}

impl Repo {
    /// A Git repository holding an initialized `dadr/` project, with no
    /// commits yet.
    fn new(label: &str) -> Self {
        let sandbox =
            std::env::temp_dir().join(format!("dadrock-edits-{label}-{}", RecordId::new_v7()));
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

    /// A project that is not in a Git repository.
    fn without_git(label: &str) -> Self {
        let repo = Self::new(label);
        fs::remove_dir_all(repo.root.join(".git")).unwrap();
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

    /// Commit everything with `message`, dated `date` (ISO 8601).
    fn commit(&self, message: &str, date: &str) -> String {
        self.git(&["add", "-A"]);
        let output = self
            .command("git")
            .args(["commit", "-q", "--no-verify", "-m", message])
            .env("GIT_AUTHOR_DATE", date)
            .env("GIT_COMMITTER_DATE", date)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        self.git(&["rev-parse", "HEAD"]).trim().to_owned()
    }

    fn write(&self, rel: &str, content: &str) {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn edit(&self, rel: &str, from: &str, to: &str) {
        let content = fs::read_to_string(self.root.join(rel)).unwrap();
        assert!(content.contains(from), "`{from}` not in {rel}");
        self.write(rel, &content.replacen(from, to, 1));
    }

    fn dadrock(&self, args: &[&str]) -> Output {
        let output = self
            .command(env!("CARGO_BIN_EXE_dadrock"))
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "dadrock {args:?} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    /// `dadrock history --json`: the history's data.
    fn history(&self, query: &str, patch: bool) -> Value {
        let mut args = vec!["history", "--json", query];
        if patch {
            args.push("--patch");
        }
        let output = self.dadrock(&args);
        let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(envelope["ok"], true, "{envelope}");
        envelope["data"].clone()
    }

    /// The edits of the version with `id`.
    fn edits(&self, id: &str, patch: bool) -> Value {
        let history = self.history(id, patch);
        history["versions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|version| version["id"] == id)
            .unwrap_or_else(|| panic!("no version {id} in {history}"))["edits"]
            .clone()
    }

    fn text(&self, args: &[&str]) -> String {
        String::from_utf8(self.dadrock(args).stdout).unwrap()
    }
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.sandbox);
    }
}

fn short(id: &str) -> String {
    RecordId::parse(id).unwrap().short_id()
}

fn candidate(title: &str, id: &str, body: &str) -> String {
    format!("---\ntitle: {title}\nstatus: candidate\nid: {id}\n---\n\n{body}")
}

fn inbox(id: &str) -> String {
    format!("dadr/inbox/{}-winsorize-income.md", short(id))
}

fn ledger(id: &str) -> String {
    format!("dadr/ledger/{}-winsorize-income.md", short(id))
}

fn archive(id: &str) -> String {
    format!("dadr/archive/{}-winsorize-income.md", short(id))
}

/// A repository where record A was drafted, edited as a candidate, and
/// accepted, each in its own commit.
fn accepted_repo(label: &str) -> Repo {
    let repo = Repo::new(label);
    repo.write(&inbox(ID_A), &candidate("Winsorize income", ID_A, SECTIONS));
    repo.commit("Draft winsorizing", "2026-09-01T09:00:00+02:00");
    repo.edit(&inbox(ID_A), "at p99", "at the 99th percentile");
    repo.commit("Reword the draft", "2026-09-02T09:00:00+02:00");
    repo.dadrock(&["accept", &short(ID_A)]);
    repo.commit("Accept winsorizing", "2026-09-03T09:00:00+02:00");
    repo
}

#[test]
fn committed_edits_after_acceptance_are_listed_newest_first() {
    let repo = accepted_repo("committed");
    assert_eq!(repo.edits(ID_A, false), json!([]));

    repo.edit(&ledger(ID_A), "lomg", "long");
    let typo = repo.commit("Fix typo in context", "2026-10-01T10:00:00+02:00");
    repo.edit(
        &ledger(ID_A),
        "status: accepted\n",
        "status: accepted\ntags:\n  - topic/income\n",
    );
    repo.commit("Tag the decision", "2026-10-02T10:00:00+02:00");
    repo.edit(
        &ledger(ID_A),
        "title: Winsorize income",
        "title: Winsorize household income",
    );
    let retitle = repo.commit("Name the income measure", "2026-10-03T10:00:00+02:00");

    let edits = repo.edits(ID_A, false);
    assert_eq!(
        edits,
        json!([
            {
                "commit": retitle,
                "short_commit": &retitle[..12],
                "author": "Dadrock Test",
                "date": "2026-10-03T10:00:00+02:00",
                "subject": "Name the income measure",
                "title_changed": true,
                "body_changed": false
            },
            {
                "commit": typo,
                "short_commit": &typo[..12],
                "author": "Dadrock Test",
                "date": "2026-10-01T10:00:00+02:00",
                "subject": "Fix typo in context",
                "title_changed": false,
                "body_changed": true
            }
        ])
    );

    let text = repo.text(&["history", &short(ID_A)]);
    assert!(
        text.contains("\n    edited 2 times since accepted (latest 2026-10-03, Dadrock Test)\n"),
        "{text}"
    );
}

#[test]
fn patch_carries_each_edits_diff() {
    let repo = accepted_repo("patch");
    repo.edit(&ledger(ID_A), "lomg", "long");
    let typo = repo.commit("Fix typo in context", "2026-10-01T10:00:00+02:00");

    let edits = repo.edits(ID_A, true);
    let diff = &edits[0]["diff"];
    assert_eq!(diff["changed"], true);
    assert_eq!(diff["title"], Value::Null);
    let lines = diff["lines"].as_array().unwrap();
    assert!(lines.contains(&json!({"kind": "removed", "text": "Income has a lomg right tail."})));
    assert!(lines.contains(&json!({"kind": "added", "text": "Income has a long right tail."})));
    // Without --patch, edits carry no diff.
    assert!(repo.edits(ID_A, false)[0].get("diff").is_none());

    let text = repo.text(&["history", "--patch", &short(ID_A)]);
    assert!(
        text.contains(&format!(
            "    edited 1 time since accepted (latest 2026-10-01, Dadrock Test)\n    edit {}  Fix typo in context\n",
            &typo[..12]
        )),
        "{text}"
    );
    assert!(
        text.contains("    - Income has a lomg right tail.\n    + Income has a long right tail.\n"),
        "{text}"
    );
}

#[test]
fn uncommitted_edits_come_first_without_a_commit() {
    let repo = accepted_repo("uncommitted");
    repo.edit(&ledger(ID_A), "lomg", "long");
    repo.commit("Fix typo in context", "2026-10-01T10:00:00+02:00");
    repo.edit(&ledger(ID_A), "less sensitive", "much less sensitive");

    let edits = repo.edits(ID_A, true);
    assert_eq!(edits.as_array().unwrap().len(), 2);
    for key in ["commit", "short_commit", "author", "date", "subject"] {
        assert_eq!(edits[0][key], Value::Null, "{key}");
    }
    assert_eq!(edits[0]["body_changed"], true);
    assert!(edits[0]["diff"]["lines"]
        .as_array()
        .unwrap()
        .contains(&json!({"kind": "added", "text": "Estimates are much less sensitive."})));
    assert_eq!(edits[1]["subject"], "Fix typo in context");

    let text = repo.text(&["history", "--patch", &short(ID_A)]);
    assert!(
        text.contains(
            "    edited 2 times since accepted (latest not committed yet)\n    edit (not committed yet)\n"
        ),
        "{text}"
    );
}

#[test]
fn tag_and_successor_changes_are_not_edits() {
    let repo = accepted_repo("tags");
    repo.edit(
        &ledger(ID_A),
        "status: accepted\n",
        "status: accepted\ntags:\n  - topic/income\nowner: team\n",
    );
    repo.commit("Tag the decision", "2026-10-01T10:00:00+02:00");
    // Line endings alone are not an edit either.
    let crlf = fs::read_to_string(repo.root.join(ledger(ID_A)))
        .unwrap()
        .replace('\n', "\r\n");
    repo.write(&ledger(ID_A), &crlf);
    repo.commit("Use CRLF", "2026-10-02T10:00:00+02:00");

    assert_eq!(repo.edits(ID_A, false), json!([]));
    assert!(!repo.text(&["history", &short(ID_A)]).contains("edited"));
}

#[test]
fn edits_follow_the_record_into_the_archive() {
    let repo = accepted_repo("archive");
    repo.edit(&ledger(ID_A), "lomg", "long");
    repo.commit("Fix typo in context", "2026-10-01T10:00:00+02:00");
    repo.write(
        &inbox(ID_B),
        &candidate(
            "Trim income",
            ID_B,
            &format!(
                "## Supersedes\n\n[[{ID_A}|Winsorize income]]\n\nTrimming is simpler.\n\n{}",
                SECTIONS.replace("Winsorize income at p99.", "Trim income at p95.")
            ),
        ),
    );
    repo.commit("Draft trimming", "2026-10-02T10:00:00+02:00");
    repo.dadrock(&["accept", &short(ID_B)]);
    assert!(repo.root.join(archive(ID_A)).is_file());

    // Before the move is committed, the record is found at its old path.
    let uncommitted = repo.edits(ID_A, false);
    assert_eq!(uncommitted.as_array().unwrap().len(), 1, "{uncommitted}");
    assert_eq!(uncommitted[0]["subject"], "Fix typo in context");

    repo.commit("Supersede winsorizing", "2026-10-03T10:00:00+02:00");
    let history = repo.history(ID_A, false);
    let versions = history["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 2);
    let old = versions
        .iter()
        .find(|version| version["id"] == ID_A)
        .unwrap();
    assert!(old["path"].as_str().unwrap().ends_with(&archive(ID_A)));
    assert_eq!(old["edits"].as_array().unwrap().len(), 1);
    assert_eq!(old["edits"][0]["subject"], "Fix typo in context");
    let new = versions
        .iter()
        .find(|version| version["id"] == ID_B)
        .unwrap();
    assert_eq!(new["edits"], json!([]));
}

#[test]
fn rejected_records_list_edits_since_rejection() {
    let repo = Repo::new("rejected");
    repo.write(&inbox(ID_A), &candidate("Winsorize income", ID_A, SECTIONS));
    repo.commit("Draft winsorizing", "2026-09-01T09:00:00+02:00");
    repo.dadrock(&["reject", &short(ID_A)]);
    repo.commit("Reject winsorizing", "2026-09-02T09:00:00+02:00");
    repo.edit(&archive(ID_A), "lomg", "long");
    repo.commit("Fix typo", "2026-09-03T09:00:00+02:00");

    assert_eq!(repo.edits(ID_A, false).as_array().unwrap().len(), 1);
    assert!(repo
        .text(&["history", &short(ID_A)])
        .contains("edited 1 time since rejected (latest 2026-09-03, Dadrock Test)"));
}

#[test]
fn candidates_have_no_edits_and_outside_git_edits_are_null() {
    let repo = Repo::new("candidate");
    repo.write(&inbox(ID_A), &candidate("Winsorize income", ID_A, SECTIONS));
    repo.commit("Draft winsorizing", "2026-09-01T09:00:00+02:00");
    repo.edit(&inbox(ID_A), "lomg", "long");
    repo.commit("Fix typo", "2026-09-02T09:00:00+02:00");
    assert_eq!(repo.edits(ID_A, false), json!([]));

    let repo = Repo::without_git("no-git");
    repo.write(&inbox(ID_A), &candidate("Winsorize income", ID_A, SECTIONS));
    repo.dadrock(&["accept", &short(ID_A)]);
    let history = repo.history(ID_A, true);
    assert_eq!(history["versions"][0]["edits"], Value::Null);
    assert_eq!(history["versions"][0]["title"], "Winsorize income");
    assert!(!repo.text(&["history", &short(ID_A)]).contains("edited"));
}

#[test]
fn a_repository_without_commits_has_no_edits() {
    let repo = Repo::new("unborn");
    repo.write(&inbox(ID_A), &candidate("Winsorize income", ID_A, SECTIONS));
    repo.dadrock(&["accept", &short(ID_A)]);
    assert_eq!(repo.edits(ID_A, false), json!([]));
}
