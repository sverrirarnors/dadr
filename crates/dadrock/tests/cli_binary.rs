//! The `dadrock` binary's argument parsing and stdout handling, run as a
//! subprocess.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use dadrock::{project::init_project, RecordId};

/// An initialized project in a fresh temporary directory.
fn temp_project(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("dadrock-cli-{label}-{}", RecordId::new_v7()));
    fs::create_dir_all(&root).unwrap();
    init_project(&root).unwrap();
    root
}

fn dadrock(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_dadrock"));
    command.current_dir(root);
    command
}

fn run(root: &Path, args: &[&str]) -> Output {
    dadrock(root).args(args).output().unwrap()
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        stdout(output),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The one record `dadrock list` finds for `--tag` and `--query`.
fn listed_record(root: &Path, tag: &str, query: &str) -> serde_json::Value {
    let output = run(root, &["list", "--tag", tag, "--query", query, "--unique"]);
    assert_success(&output);
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    envelope["data"][0].clone()
}

#[test]
fn new_accepts_options_after_an_unquoted_title() {
    let root = temp_project("new-after");

    let output = run(
        &root,
        &[
            "new",
            "Drop",
            "missing",
            "rows",
            "--tag",
            "kind/sample",
            "--quiet",
        ],
    );
    assert_success(&output);
    let id = stdout(&output).trim().to_owned();
    RecordId::parse(&id).unwrap();

    let record = listed_record(&root, "kind/sample", "Drop missing rows");
    assert_eq!(record["title"], "Drop missing rows");
    assert_eq!(record["id"], id.as_str());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_accepts_options_before_the_title() {
    let root = temp_project("new-before");

    let output = run(
        &root,
        &[
            "new",
            "--tag",
            "kind/estimand",
            "Report the ATE",
            "--tag",
            "writer/agent",
        ],
    );
    assert_success(&output);
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["ok"], true);

    for tag in ["kind/estimand", "writer/agent"] {
        let record = listed_record(&root, tag, "Report the ATE");
        assert_eq!(record["title"], "Report the ATE");
        assert_eq!(record["id"], envelope["data"]["id"]);
    }

    fs::remove_dir_all(root).unwrap();
}
