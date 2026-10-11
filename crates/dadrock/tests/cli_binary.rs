//! The `dadrock` binary's argument parsing and stdout handling, run as a
//! subprocess.

use std::{
    fs,
    io::{BufRead, BufReader, Read},
    net::TcpStream,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
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

/// The one record `dadrock list <filters>` finds.
fn unique_record(root: &Path, filters: &[&str]) -> serde_json::Value {
    let mut args = vec!["list", "--unique"];
    args.extend(filters);
    let output = run(root, &args);
    assert_success(&output);
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    envelope["data"][0].clone()
}

/// The one record `dadrock list` finds for `--tag` and `--query`.
fn listed_record(root: &Path, tag: &str, query: &str) -> serde_json::Value {
    unique_record(root, &["--tag", tag, "--query", query])
}

#[test]
fn help_after_an_unquoted_query_shows_help_instead_of_running() {
    let root = temp_project("reject-help");
    assert_success(&run(&root, &["new", "Drop", "missing", "rows"]));

    let output = run(&root, &["reject", "Drop", "missing", "--help"]);
    assert_success(&output);
    assert!(
        stdout(&output).contains("Usage: dadrock reject"),
        "{}",
        stdout(&output)
    );
    let record = unique_record(&root, &["--query", "Drop missing rows"]);
    assert_eq!(record["status"], "candidate");

    assert_success(&run(&root, &["reject", "Drop", "missing", "rows"]));
    let record = unique_record(&root, &["--query", "Drop missing rows"]);
    assert_eq!(record["status"], "rejected");

    fs::remove_dir_all(root).unwrap();
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

/// Accepted records with no sections: each lists in `dadrock list` and fails
/// `dadrock validate`, so both print far more than a pipe buffers (64 KiB).
fn write_many_records(root: &Path, count: u32) {
    let ledger = root.join("dadr").join("ledger");
    for i in 0..count {
        let id = format!("0193{i:04x}-0000-7000-8000-{i:012x}");
        let content = format!(
            "---\ntitle: Padding decision number {i} with a long enough title\n\
             status: accepted\nid: {id}\n---\n"
        );
        fs::write(ledger.join(format!("padding-{i:04}.md")), content).unwrap();
    }
}

/// Run `dadrock <args>` with stdout read by a reader that stops after one
/// line, the way `dadrock <args> | head -1` does.
fn run_with_early_closed_stdout(root: &Path, args: &[&str]) -> (Output, String) {
    let mut child = dadrock(root)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut first_line = String::new();
    {
        let mut reader = BufReader::new(child.stdout.take().unwrap());
        reader.read_line(&mut first_line).unwrap();
    }
    (child.wait_with_output().unwrap(), first_line)
}

fn assert_quiet(args: &[&str], output: &Output, exit_code: i32) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.is_empty(),
        "dadrock {args:?} wrote to stderr:\n{stderr}"
    );
    // A closed stdout keeps the command's own outcome: `list` succeeds,
    // `validate` still fails on the records' errors.
    assert_eq!(
        output.status.code(),
        Some(exit_code),
        "dadrock {args:?} exited with {:?}",
        output.status
    );
}

#[test]
fn reader_that_stops_after_one_line_ends_commands_quietly() {
    let root = temp_project("head");
    write_many_records(&root, 800);

    for (args, first_line, exit_code) in [
        (&["list"][..], "{", 0),
        (&["validate"][..], "dadrock validate: ", 1),
    ] {
        let (output, line) = run_with_early_closed_stdout(&root, args);
        assert!(
            line.starts_with(first_line),
            "dadrock {args:?} printed {line:?} first"
        );
        assert_quiet(args, &output, exit_code);
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stdout_closed_before_any_output_ends_commands_quietly() {
    let root = temp_project("no-reader");
    write_many_records(&root, 3);

    for (args, exit_code) in [
        (&[][..], 0),
        (&["help", "validate", "changes"][..], 0),
        (&["commands"][..], 0),
        (&["commands", "--json"][..], 0),
        (&["completions", "zsh"][..], 0),
        (&["gen-cli-docs"][..], 0),
        (&["dir"][..], 0),
        (&["config"][..], 0),
        (&["list"][..], 0),
        (&["validate"][..], 1),
        (&["validate", "links"][..], 0),
        (&["tags", "list", "--json"][..], 0),
        (&["new", "--quiet", "Closed", "stdout"][..], 0),
    ] {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let output = dadrock(&root)
            .args(args)
            .stdout(writer)
            .stderr(Stdio::piped())
            .output()
            .unwrap();
        assert_quiet(args, &output, exit_code);
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn preview_keeps_serving_when_stdout_is_closed() {
    let root = temp_project("preview");
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let mut child = dadrock(&root)
        .args(["preview", "--port", "0"])
        .env_remove("DADROCK_PREVIEW_EXIT_ON_STDIN_EOF")
        .stdin(Stdio::null())
        .stdout(writer)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stderr = BufReader::new(child.stderr.take().unwrap());
    let mut line = String::new();
    stderr.read_line(&mut line).unwrap();
    let Some(address) = line
        .trim_end()
        .strip_prefix("dadrock-preview: listening on http://")
    else {
        child.kill().ok();
        let mut rest = String::new();
        stderr.read_to_string(&mut rest).ok();
        panic!("dadrock preview did not start:\n{line}{rest}");
    };
    let connected = TcpStream::connect(address);
    let running = child.try_wait().unwrap().is_none();
    child.kill().unwrap();
    child.wait().unwrap();

    connected.unwrap();
    assert!(running, "dadrock preview exited");
    fs::remove_dir_all(root).unwrap();
}
