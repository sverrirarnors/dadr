//! Supersession: merges, splits, stale successors, consistency, and the rules
//! for rewriting accepted and rejected records.

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Once,
};

use camino::{Utf8Path, Utf8PathBuf};
use dadrock::{
    commands::{
        accept_records, apply_candidate_revision, new_successor_record, save_record_source,
        supersede_records, ApplyCandidateRevisionRequest, CommandContext, NewSuccessorRequest,
        SaveRecordSourceRequest, SupersedeRecordsRequest,
    },
    index::ProjectIndex,
    project::{discover_from, init_project, ProjectContext},
    records::{
        accept_candidate, accept_candidates, create_candidate_with_body, current_successors,
        pending_supersedes, reject_candidate, status_record, RecordMove, SupersededRecordMove,
    },
    tasks::{create_task_run_for_task, validate_task_output, TaskContext, TaskOutputKind},
    validation::{validate_project_index, ProjectValidationIssueCode},
    RecordId,
};
use serde_json::json;

const SECTIONS: &str = "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n\
                        ## Alternatives\n\nNone considered.\n\n## Consequences\n\nEffects.\n";

struct Project {
    root: PathBuf,
    project: ProjectContext,
}

impl Project {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "dadrock-supersession-{label}-{}",
            RecordId::new_v7()
        ));
        fs::create_dir_all(&root).unwrap();
        init_project(&root).unwrap();
        let project = discover_from(&root).unwrap();
        Self { root, project }
    }

    fn index(&self) -> ProjectIndex {
        ProjectIndex::scan(&self.project).unwrap()
    }

    fn context(&self) -> CommandContext {
        CommandContext::from_path(&self.root).unwrap()
    }

    fn candidate(&self, title: &str, body: &str) -> (RecordId, Utf8PathBuf) {
        let created = create_candidate_with_body(&self.project, title, &[], body).unwrap();
        (created.id, created.path)
    }

    fn accepted(&self, title: &str) -> RecordMove {
        let (id, _) = self.candidate(title, SECTIONS);
        accept_candidate(&self.index(), &id.to_string())
            .unwrap()
            .accepted
            .remove(0)
    }

    fn successors_of(&self, id: RecordId) -> Vec<RecordId> {
        status_record(&self.index(), &id.to_string())
            .unwrap()
            .successors
            .into_iter()
            .map(|record| record.id)
            .collect()
    }

    fn files(&self, dir: &Utf8Path) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".md"))
            .collect();
        names.sort();
        names
    }

    fn issues(&self, code: ProjectValidationIssueCode) -> Vec<String> {
        validate_project_index(&self.index())
            .issues
            .into_iter()
            .filter(|issue| issue.code == code)
            .map(|issue| issue.message)
            .collect()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn successor_body(predecessors: &[RecordId]) -> String {
    let links: String = predecessors
        .iter()
        .map(|id| format!("[[{id}|Previous decision]]\n"))
        .collect();
    format!("## Supersedes\n\n{links}\nThe old rule no longer fits.\n\n{SECTIONS}")
}

fn ids(queries: &[RecordId]) -> Vec<String> {
    queries.iter().map(ToString::to_string).collect()
}

fn isolate_user_tasks() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::env::set_var(
            "DADROCK_TASKS_DIR",
            std::env::temp_dir().join("dadrock-supersession-no-user-tasks"),
        );
    });
}

#[test]
fn merge_acceptance_supersedes_every_named_predecessor() {
    let p = Project::new("merge");
    let first = p.accepted("Winsorize income");
    let second = p.accepted("Drop implausible ages");
    let (merged, merged_path) =
        p.candidate("One outlier rule", &successor_body(&[first.id, second.id]));

    let pending = pending_supersedes(&p.index(), &merged.to_string()).unwrap();
    assert_eq!(
        pending
            .predecessors
            .iter()
            .map(|r| r.id)
            .collect::<Vec<_>>(),
        vec![first.id, second.id]
    );

    let envelope = accept_records(&p.context(), &ids(&[merged])).unwrap();
    let archive = p.project.archive_dir();
    let first_archived = archive.join(first.to.file_name().unwrap());
    let second_archived = archive.join(second.to.file_name().unwrap());
    let accepted_path = envelope.data.accepted[0].to.clone();
    assert_eq!(
        envelope.data.superseded,
        vec![
            SupersededRecordMove {
                id: first.id,
                from: first.to.clone(),
                to: first_archived.clone(),
                successors: vec![merged],
            },
            SupersededRecordMove {
                id: second.id,
                from: second.to.clone(),
                to: second_archived.clone(),
                successors: vec![merged],
            },
        ]
    );
    // Both ends of every move.
    assert_eq!(
        envelope.changed_paths,
        vec![
            merged_path,
            accepted_path,
            first.to.clone(),
            first_archived,
            second.to.clone(),
            second_archived
        ]
    );
    assert_eq!(p.successors_of(first.id), vec![merged]);
    assert_eq!(p.successors_of(second.id), vec![merged]);
    let report = validate_project_index(&p.index());
    assert!(
        report.ok && report.counts.warnings == 0,
        "{:?}",
        report.issues
    );
}

#[test]
fn split_acceptance_gives_the_predecessor_every_candidate_in_the_set() {
    let p = Project::new("split");
    let old = p.accepted("Listwise deletion");
    let (left, _) = p.candidate("Impute income", &successor_body(&[old.id]));
    let (right, _) = p.candidate("Complete cases for the outcome", &successor_body(&[old.id]));
    let (plain, _) = p.candidate("Report the sample size", SECTIONS);

    let result = accept_candidates(&p.index(), &ids(&[left, right, plain])).unwrap();
    assert_eq!(
        result
            .accepted
            .iter()
            .map(|moved| moved.id)
            .collect::<Vec<_>>(),
        vec![left, right, plain]
    );
    assert_eq!(result.superseded.len(), 1);
    assert_eq!(result.superseded[0].successors, vec![left, right]);
    assert_eq!(p.successors_of(old.id), vec![left, right]);
    assert!(p.files(&p.project.inbox_dir()).is_empty());
    assert_eq!(p.files(&p.project.archive_dir()).len(), 1);
    let report = validate_project_index(&p.index());
    assert!(
        report.ok && report.counts.warnings == 0,
        "{:?}",
        report.issues
    );
}

#[test]
fn accept_with_flag_accepts_a_split_from_the_command_line() {
    let p = Project::new("split-cli");
    let old = p.accepted("Listwise deletion");
    let (left, _) = p.candidate("Impute income", &successor_body(&[old.id]));
    let (right, _) = p.candidate("Complete cases for the outcome", &successor_body(&[old.id]));

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_dadrock"))
        .current_dir(&p.root)
        .args(["accept", "Impute", "income", "--with", &right.short_id()])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(envelope["command"], "status accept");
    assert_eq!(envelope["data"]["accepted"][0]["id"], left.to_string());
    assert_eq!(envelope["data"]["accepted"][1]["id"], right.to_string());
    assert_eq!(
        envelope["data"]["superseded"][0]["successors"],
        json!([left.to_string(), right.to_string()])
    );
    assert_eq!(envelope["changed_paths"].as_array().unwrap().len(), 6);
    assert_eq!(p.successors_of(old.id), vec![left, right]);
}

#[test]
fn competing_candidates_leave_the_other_one_stale() {
    let p = Project::new("competing");
    let old = p.accepted("Trim income at p95");
    let (first, _) = p.candidate("Winsorize at p99", &successor_body(&[old.id]));
    let (second, second_path) = p.candidate("Winsorize at p97", &successor_body(&[old.id]));
    assert!(p
        .issues(ProjectValidationIssueCode::StaleSupersedes)
        .is_empty());

    accept_candidate(&p.index(), &first.to_string()).unwrap();

    let pending = pending_supersedes(&p.index(), &second.to_string()).unwrap();
    assert_eq!(pending.predecessors[0].id, old.id);
    assert_eq!(pending.stale.len(), 1);
    assert_eq!(pending.stale[0].predecessor.id, old.id);
    assert_eq!(
        pending.stale[0]
            .current_successors
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![first]
    );
    let warnings = p.issues(ProjectValidationIssueCode::StaleSupersedes);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("Winsorize at p99"), "{}", warnings[0]);
    assert!(validate_project_index(&p.index()).ok);

    let before = fs::read_to_string(&second_path).unwrap();
    let error = accept_candidate(&p.index(), &second.to_string()).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("already superseded"), "{message}");
    assert!(message.contains("`Winsorize at p99`"), "{message}");
    assert!(message.contains(&first.short_id()), "{message}");
    assert_eq!(fs::read_to_string(&second_path).unwrap(), before);

    // Rejecting the stale candidate clears the warning.
    reject_candidate(&p.index(), &second.to_string()).unwrap();
    assert!(p
        .issues(ProjectValidationIssueCode::StaleSupersedes)
        .is_empty());
}

#[test]
fn current_successors_follow_the_chain_to_the_decisions_in_force() {
    let p = Project::new("chain");
    let first = p.accepted("Pairwise deletion");
    let (second, _) = p.candidate("Listwise deletion", &successor_body(&[first.id]));
    accept_candidate(&p.index(), &second.to_string()).unwrap();
    let (left, _) = p.candidate("Impute income", &successor_body(&[second]));
    let (right, _) = p.candidate("Complete cases for the outcome", &successor_body(&[second]));
    accept_candidates(&p.index(), &ids(&[left, right])).unwrap();

    let current: Vec<_> = current_successors(&p.index(), first.id)
        .into_iter()
        .map(|record| record.id)
        .collect();
    assert_eq!(current, vec![left, right]);
    assert!(current_successors(&p.index(), left).is_empty());
}

#[test]
fn accepted_supersedes_must_match_the_predecessor_successors() {
    let p = Project::new("consistency");
    let old = p.accepted("Old rule");
    // Accepted by hand, without linking the predecessor.
    let (new, new_path) = p.candidate("New rule", &successor_body(&[old.id]));
    let content = fs::read_to_string(&new_path)
        .unwrap()
        .replace("status: candidate", "status: accepted");
    let ledger_path = p.project.ledger_dir().join(new_path.file_name().unwrap());
    fs::write(&ledger_path, content).unwrap();
    fs::remove_file(&new_path).unwrap();

    let warnings = p.issues(ProjectValidationIssueCode::UnlinkedSupersedes);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("`Old rule`"), "{}", warnings[0]);

    supersede_records(
        &p.context(),
        SupersedeRecordsRequest {
            from: vec![old.id.to_string()],
            to: vec![new.to_string()],
        },
    )
    .unwrap();
    assert!(p
        .issues(ProjectValidationIssueCode::UnlinkedSupersedes)
        .is_empty());
}

#[test]
fn group_acceptance_refusals_change_nothing() {
    let p = Project::new("refusals");
    let old = p.accepted("Listwise deletion");
    let (good, good_path) = p.candidate("Impute income", &successor_body(&[old.id]));
    let (incomplete, _) = p.candidate("Half a decision", "## Context\n\nWhy.\n");
    let snapshot = |paths: &[&Utf8PathBuf]| -> Vec<String> {
        paths
            .iter()
            .map(|path| fs::read_to_string(path).unwrap())
            .collect()
    };
    let before = snapshot(&[&good_path, &old.to]);

    // A candidate that fails the section checks.
    let error = accept_candidates(&p.index(), &ids(&[good, incomplete])).unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("`Half a decision` cannot be accepted"),
        "{message}"
    );
    assert!(
        message.contains("missing Decision, Alternatives, Consequences"),
        "{message}"
    );

    // A predecessor named inside the set.
    let (names_candidate, _) = p.candidate("Names a candidate", &successor_body(&[good]));
    let error = accept_candidates(&p.index(), &ids(&[good, names_candidate])).unwrap_err();
    let message = error.to_string();
    assert!(message.contains("accepted in the same set"), "{message}");

    // The same candidate twice.
    let error = accept_candidates(&p.index(), &[good.to_string(), good.short_id()]).unwrap_err();
    assert!(error.to_string().contains("duplicate"), "{error}");

    assert_eq!(snapshot(&[&good_path, &old.to]), before);
    assert!(p.files(&p.project.archive_dir()).is_empty());
    assert_eq!(p.files(&p.project.ledger_dir()).len(), 1);
}

#[cfg(unix)]
#[test]
fn failed_write_during_group_acceptance_restores_every_record() {
    use std::os::unix::fs::PermissionsExt;

    let p = Project::new("group-rollback");
    let old = p.accepted("Listwise deletion");
    let nested = p.project.ledger_dir().join("nested");
    fs::create_dir_all(&nested).unwrap();
    let old_path = nested.join(old.to.file_name().unwrap());
    fs::rename(&old.to, &old_path).unwrap();
    let (left, left_path) = p.candidate("Impute income", &successor_body(&[old.id]));
    let (right, right_path) = p.candidate("Complete cases", &successor_body(&[old.id]));
    let before = [
        fs::read_to_string(&old_path).unwrap(),
        fs::read_to_string(&left_path).unwrap(),
        fs::read_to_string(&right_path).unwrap(),
    ];

    // The predecessor cannot leave its folder, after both candidates moved.
    let permissions = fs::metadata(&nested).unwrap().permissions();
    fs::set_permissions(&nested, fs::Permissions::from_mode(0o500)).unwrap();
    let result = accept_candidates(&p.index(), &ids(&[left, right]));
    fs::set_permissions(&nested, permissions).unwrap();

    assert!(result.is_err());
    let after = [
        fs::read_to_string(&old_path).unwrap(),
        fs::read_to_string(&left_path).unwrap(),
        fs::read_to_string(&right_path).unwrap(),
    ];
    assert_eq!(before, after);
    assert!(p.files(&p.project.archive_dir()).is_empty());
    assert!(p.files(&p.project.ledger_dir()).is_empty());
    assert_eq!(p.files(&p.project.inbox_dir()).len(), 2);
}

fn source(path: &Utf8Path) -> String {
    fs::read_to_string(path).unwrap()
}

/// The id and successors in a record file's front matter.
fn identity(path: &Utf8Path) -> (RecordId, Vec<RecordId>) {
    let record = dadrock::records::parse_record_file(path)
        .unwrap()
        .record
        .unwrap();
    (record.front_matter.id, record.front_matter.successors)
}

#[test]
fn a_successor_with_its_predecessors_title_takes_its_filename() {
    let p = Project::new("same-title");
    let old = p.accepted("Listwise deletion");
    let ledger_path = p.project.ledger_dir().join("listwise-deletion.md");
    assert_eq!(old.to, ledger_path);
    let (successor, successor_path) = p.candidate("Listwise deletion", &successor_body(&[old.id]));
    assert_eq!(
        successor_path,
        p.project.inbox_dir().join("listwise-deletion.md")
    );

    let envelope = accept_records(&p.context(), &ids(&[successor])).unwrap();
    let archived_path = p.project.archive_dir().join("listwise-deletion.md");
    assert_eq!(envelope.data.accepted[0].to, ledger_path);
    assert_eq!(envelope.data.superseded[0].to, archived_path);
    assert_eq!(
        envelope.changed_paths,
        vec![successor_path, ledger_path.clone(), archived_path.clone()]
    );

    // The successor's text is in the ledger and the predecessor's, now with
    // its successor, in the archive.
    assert_eq!(identity(&ledger_path), (successor, vec![]));
    assert!(source(&ledger_path).contains("The old rule no longer fits."));
    assert_eq!(identity(&archived_path), (old.id, vec![successor]));
    assert!(p.files(&p.project.inbox_dir()).is_empty());
    // The same name in two folders is normal.
    let report = validate_project_index(&p.index());
    assert!(
        report.ok && report.counts.warnings == 0,
        "{:?}",
        report.issues
    );
}

#[cfg(unix)]
#[test]
fn a_failed_same_title_acceptance_restores_both_records() {
    use std::os::unix::fs::PermissionsExt;

    let p = Project::new("same-title-rollback");
    let old = p.accepted("Listwise deletion");
    let (successor, successor_path) = p.candidate("Listwise deletion", &successor_body(&[old.id]));
    let before = (source(&old.to), source(&successor_path));

    // Both records reach their new paths, then the candidate cannot leave the
    // inbox.
    let inbox = p.project.inbox_dir();
    let permissions = fs::metadata(&inbox).unwrap().permissions();
    fs::set_permissions(&inbox, fs::Permissions::from_mode(0o500)).unwrap();
    let result = accept_candidates(&p.index(), &ids(&[successor]));
    fs::set_permissions(&inbox, permissions).unwrap();

    assert!(result.is_err());
    assert_eq!((source(&old.to), source(&successor_path)), before);
    assert_eq!(identity(&old.to), (old.id, vec![]));
    assert!(p.files(&p.project.archive_dir()).is_empty());
    assert_eq!(
        p.files(&p.project.ledger_dir()),
        vec!["listwise-deletion.md"]
    );
}

#[test]
fn splits_and_merges_with_repeated_titles_keep_every_record() {
    let p = Project::new("repeated-titles");
    let title = "Outcome rule";
    let name = "outcome-rule.md";
    let suffixed = |id: RecordId| format!("outcome-rule-{}.md", id.short_id());
    // An earlier rejected draft already holds the name in the archive.
    let (rejected, _) = p.candidate(title, SECTIONS);
    reject_candidate(&p.index(), &rejected.to_string()).unwrap();
    let old = p.accepted(title);
    assert_eq!(old.to, p.project.ledger_dir().join(name));

    // A split into two successors with the same title.
    let (left, left_path) = p.candidate(title, &successor_body(&[old.id]));
    let (right, right_path) = p.candidate(title, &successor_body(&[old.id]));
    assert_eq!(left_path, p.project.inbox_dir().join(name));
    assert_eq!(right_path, p.project.inbox_dir().join(suffixed(right)));
    accept_candidates(&p.index(), &ids(&[left, right])).unwrap();
    let ledger = p.project.ledger_dir();
    let archive = p.project.archive_dir();
    assert_eq!(identity(&ledger.join(name)), (left, vec![]));
    assert_eq!(identity(&ledger.join(suffixed(right))), (right, vec![]));
    assert_eq!(
        identity(&archive.join(suffixed(old.id))),
        (old.id, vec![left, right])
    );
    assert_eq!(identity(&archive.join(name)).0, rejected);

    // A merge of both back into one record with the same title.
    let (merged, merged_path) = p.candidate(title, &successor_body(&[left, right]));
    assert_eq!(merged_path, p.project.inbox_dir().join(name));
    accept_candidates(&p.index(), &ids(&[merged])).unwrap();
    assert_eq!(p.files(&ledger), vec![name]);
    assert_eq!(identity(&ledger.join(name)), (merged, vec![]));
    assert_eq!(
        identity(&archive.join(suffixed(left))),
        (left, vec![merged])
    );
    assert_eq!(
        identity(&archive.join(suffixed(right))),
        (right, vec![merged])
    );
    assert_eq!(p.files(&archive).len(), 4);
    assert!(p.files(&p.project.inbox_dir()).is_empty());
    let report = validate_project_index(&p.index());
    assert!(
        report.ok && report.counts.warnings == 0,
        "{:?}",
        report.issues
    );
}

#[test]
fn lifecycle_moves_keep_the_filename() {
    let p = Project::new("keep-filename");
    // A hand-named candidate keeps its name when accepted, whatever its title.
    let id = RecordId::new_v7();
    let hand_named = p.project.inbox_dir().join("01938b8e4d70-legacy-name.md");
    fs::write(
        &hand_named,
        format!("---\ntitle: Something else\nstatus: candidate\nid: {id}\n---\n\n{SECTIONS}"),
    )
    .unwrap();
    let accepted = accept_candidate(&p.index(), &id.to_string()).unwrap();
    assert_eq!(
        accepted.accepted[0].to,
        p.project.ledger_dir().join("01938b8e4d70-legacy-name.md")
    );

    // Rejecting keeps the name, with the short id only on a collision.
    let (first, _) = p.candidate("Drop site seven", SECTIONS);
    let rejected = reject_candidate(&p.index(), &first.to_string()).unwrap();
    assert_eq!(
        rejected.to,
        p.project.archive_dir().join("drop-site-seven.md")
    );
    let (second, _) = p.candidate("Drop site seven", SECTIONS);
    let rejected = reject_candidate(&p.index(), &second.to_string()).unwrap();
    assert_eq!(
        rejected.to,
        p.project
            .archive_dir()
            .join(format!("drop-site-seven-{}.md", second.short_id()))
    );

    // So does `dadrock supersede`.
    let old = p.accepted("Cluster by site");
    let new = p.accepted("Cluster by site and wave");
    let envelope = supersede_records(
        &p.context(),
        SupersedeRecordsRequest {
            from: vec![old.id.to_string()],
            to: vec![new.id.to_string()],
        },
    )
    .unwrap();
    assert_eq!(
        envelope.data.superseded[0].to,
        p.project.archive_dir().join("cluster-by-site.md")
    );
}

fn save(p: &Project, id: RecordId, content: String) -> Result<(), String> {
    save_record_source(
        &p.context(),
        SaveRecordSourceRequest {
            query: id.to_string(),
            content,
        },
    )
    .map(|_| ())
    .map_err(|error| error.to_string())
}

#[test]
fn accepted_records_take_corrections_but_keep_id_status_successors_and_sections() {
    let p = Project::new("rewrite-accepted");
    let record = p.accepted("Use cluster-robust errors");
    let original = source(&record.to);

    let refusals = [
        (
            original.replace("status: accepted", "status: rejected"),
            "`status` cannot change",
        ),
        (
            original.replace(
                "status: accepted\n",
                &format!("status: accepted\nsuccessors:\n- {}\n", RecordId::new_v7()),
            ),
            "`successors` cannot change",
        ),
        (
            original.replace("## Decision\n\nWhat.\n\n", ""),
            "missing Decision",
        ),
        (original.replace("Why.", ""), "empty Context"),
    ];
    for (content, expected) in refusals {
        let error = save(&p, record.id, content).unwrap_err();
        assert!(error.contains(expected), "{error}");
        assert!(error.contains("draft a successor"), "{error}");
        assert_eq!(source(&record.to), original);
    }
    let error = save(
        &p,
        record.id,
        original.replace(&record.id.to_string(), &RecordId::new_v7().to_string()),
    )
    .unwrap_err();
    assert!(error.contains("`id` cannot change"), "{error}");
    assert_eq!(source(&record.to), original);

    // Corrections to the title and body stay in the record, with the same id.
    let corrected = original
        .replace(
            "title: Use cluster-robust errors",
            "title: Use cluster-robust standard errors",
        )
        .replace("What.", "What, spelled out.");
    save(&p, record.id, corrected.clone()).unwrap();
    assert_eq!(source(&record.to), corrected);

    // Tags, keys outside DADR, and line endings may change.
    let retagged = corrected.replace(
        "status: accepted\n",
        "status: accepted\ntags:\n- kind/inference\nreviewer: vab\n",
    );
    save(&p, record.id, retagged.clone()).unwrap();
    assert_eq!(source(&record.to), retagged);
    let crlf = retagged.replace('\n', "\r\n");
    save(&p, record.id, crlf.clone()).unwrap();
    assert_eq!(source(&record.to), crlf);

    // Agent revisions apply to candidates only.
    let error = apply_candidate_revision(
        &p.context(),
        ApplyCandidateRevisionRequest {
            query: record.id.to_string(),
            title: "Use cluster-robust errors".to_owned(),
            tags: Vec::new(),
            body_markdown: SECTIONS.replace("What.", "Changed."),
        },
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("requires status `candidate`"),
        "{error}"
    );
    assert_eq!(source(&record.to), crlf);
}

#[test]
fn superseded_records_keep_their_successors() {
    let p = Project::new("rewrite-superseded");
    let old = p.accepted("Winsorize income");
    let (successor, _) = p.candidate("Winsorize at p97", &successor_body(&[old.id]));
    let accepted = accept_candidate(&p.index(), &successor.to_string()).unwrap();
    let archived = &accepted.superseded[0].to;
    let original = source(archived);

    let without_successors: String = original
        .lines()
        .filter(|line| !line.contains(&successor.to_string()))
        .map(|line| match line.trim_end() {
            "successors:" => "successors: []\n".to_owned(),
            line => format!("{line}\n"),
        })
        .collect();
    let error = save(&p, old.id, without_successors).unwrap_err();
    assert!(
        error.contains("is superseded, so its `successors` cannot change"),
        "{error}"
    );
    assert!(error.contains("added only by superseding"), "{error}");
    assert_eq!(source(archived), original);

    let corrected = original.replace("Why.", "Why, with a citation.");
    save(&p, old.id, corrected.clone()).unwrap();
    assert_eq!(source(archived), corrected);
}

#[test]
fn rejected_records_take_corrections_but_keep_id_and_status() {
    let p = Project::new("rewrite-rejected");
    let (id, _) = p.candidate("Drop site seven", SECTIONS);
    let rejected = reject_candidate(&p.index(), &id.to_string()).unwrap();
    let original = source(&rejected.to);

    let error = save(
        &p,
        id,
        original.replace("status: rejected", "status: accepted"),
    )
    .unwrap_err();
    assert!(
        error.contains("is rejected, so its `status` cannot change"),
        "{error}"
    );
    assert!(error.contains("draft a new candidate"), "{error}");
    assert_eq!(source(&rejected.to), original);

    // A rejected record needs no required sections.
    let corrected = original
        .replace("title: Drop site seven", "title: Drop site 7")
        .replace("## Decision\n\nWhat.\n\n", "");
    save(&p, id, corrected.clone()).unwrap();
    assert_eq!(source(&rejected.to), corrected);

    let tagged = corrected.replace(
        "status: rejected\n",
        "status: rejected\ntags:\n- rejected/no-longer-relevant\n",
    );
    save(&p, id, tagged.clone()).unwrap();
    assert_eq!(source(&rejected.to), tagged);
}

#[test]
fn candidates_stay_editable() {
    let p = Project::new("candidate-editable");
    let (id, path) = p.candidate("Draft", SECTIONS);
    let edited = source(&path)
        .replace("title: Draft", "title: Better draft")
        .replace("What.", "Something else.");
    save(&p, id, edited.clone()).unwrap();
    assert_eq!(source(&path), edited);
}

#[test]
fn edit_record_refuses_accepted_and_rejected_records() {
    isolate_user_tasks();
    let p = Project::new("edit-record");
    let accepted = p.accepted("Accepted rule");
    let (rejected, _) = p.candidate("Rejected rule", SECTIONS);
    reject_candidate(&p.index(), &rejected.to_string()).unwrap();
    let (candidate, _) = p.candidate("Candidate rule", SECTIONS);

    let run = |id: RecordId| {
        create_task_run_for_task(
            &p.project,
            "edit-record",
            json!({"record_id": id.to_string(), "requested_changes": "Tighten the wording."}),
            &TaskContext::default(),
        )
    };
    let error = run(accepted.id).unwrap_err().to_string();
    assert!(error.contains("`Accepted rule` is accepted"), "{error}");
    assert!(error.contains("supersede-record"), "{error}");
    let error = run(rejected).unwrap_err().to_string();
    assert!(error.contains("`Rejected rule` is rejected"), "{error}");
    assert!(error.contains("new candidate"), "{error}");
    run(candidate).unwrap();
}

#[test]
fn supersede_record_takes_several_current_predecessors() {
    isolate_user_tasks();
    let p = Project::new("supersede-record-input");
    let first = p.accepted("Winsorize income");
    let second = p.accepted("Drop implausible ages");
    let run = |ids: String| {
        create_task_run_for_task(
            &p.project,
            "supersede-record",
            json!({"record_ids": ids, "requested_changes": "One rule for all covariates."}),
            &TaskContext::default(),
        )
    };
    run(format!("{}\n{}", first.id, second.id)).unwrap();
    run(first.id.to_string()).unwrap();

    let (successor, _) = p.candidate("Winsorize at p97", &successor_body(&[first.id]));
    accept_candidate(&p.index(), &successor.to_string()).unwrap();
    let error = run(first.id.to_string()).unwrap_err().to_string();
    assert!(
        error.contains("already superseded by `Winsorize at p97`"),
        "{error}"
    );
}

#[test]
fn supersession_proposal_lists_predecessors_and_omits_supersedes() {
    let proposal = |ids: serde_json::Value, body: &str| {
        validate_task_output(
            TaskOutputKind::SupersessionProposalV1,
            json!({
                "kind": "supersession_proposal.v1",
                "summary": "Merge two outlier rules.",
                "superseded_record_ids": ids,
                "superseding_title": "One outlier rule",
                "superseding_body_markdown": body,
                "rationale": "The rules disagreed."
            }),
        )
    };
    let a = RecordId::new_v7().to_string();
    let b = RecordId::new_v7().to_string();
    proposal(json!([a, b]), SECTIONS).unwrap();
    assert!(proposal(json!([]), SECTIONS).is_err());
    assert!(proposal(json!(a), SECTIONS).is_err());
    let error = proposal(json!([a]), &successor_body(&[RecordId::parse(&a).unwrap()]))
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("must not contain a `## Supersedes` section"),
        "{error}"
    );
}

#[test]
fn saving_a_supersession_proposal_writes_supersedes_first() {
    let p = Project::new("proposal-to-candidate");
    let first = p.accepted("Winsorize income");
    let second = p.accepted("Drop implausible ages");

    // The fields of a supersession_proposal.v1 result.
    let envelope = new_successor_record(
        &p.context(),
        NewSuccessorRequest {
            title: "One outlier rule".to_owned(),
            tags: vec!["kind/transformation".to_owned()],
            body_markdown: Some(SECTIONS.to_owned()),
            supersedes: ids(&[first.id, second.id]),
            reason: Some("The two rules disagreed at the 99th percentile.".to_owned()),
        },
    )
    .unwrap();
    let content = source(&envelope.data.path);
    let body = content.splitn(3, "---").nth(2).unwrap().trim_start();
    assert_eq!(
        body,
        format!(
            "## Supersedes\n\n[[{}|Winsorize income]]\n[[{}|Drop implausible ages]]\n\n\
             The two rules disagreed at the 99th percentile.\n\n{SECTIONS}",
            first.id, second.id
        )
    );
    assert!(content.contains("kind/transformation"));

    // Accepting it merges the predecessors.
    accept_candidate(&p.index(), &envelope.data.id.to_string()).unwrap();
    assert_eq!(p.successors_of(first.id), vec![envelope.data.id]);
    assert_eq!(p.successors_of(second.id), vec![envelope.data.id]);

    // A predecessor must be current, and the reason cannot end the section.
    let error = new_successor_record(
        &p.context(),
        NewSuccessorRequest {
            title: "Late".to_owned(),
            supersedes: ids(&[first.id]),
            body_markdown: Some(SECTIONS.to_owned()),
            ..NewSuccessorRequest::default()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("already superseded"), "{error}");
    let current = envelope.data.id;
    let error = new_successor_record(
        &p.context(),
        NewSuccessorRequest {
            title: "Heading in reason".to_owned(),
            supersedes: ids(&[current]),
            body_markdown: Some(SECTIONS.to_owned()),
            reason: Some("Because.\n\n## Context\nOops.".to_owned()),
            ..NewSuccessorRequest::default()
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("headings"), "{error}");
}

#[test]
fn demo_examples_validate_without_warnings() {
    use dadrock::{commands::demo_project_with_example, demo::DemoExample};

    for example in [
        DemoExample::Lifecycle,
        DemoExample::CandidateReview,
        DemoExample::Penguins,
        DemoExample::Conflicts,
        DemoExample::TaskHistory,
    ] {
        let root = std::env::temp_dir().join(format!(
            "dadrock-supersession-demo-{}-{}",
            example.as_str(),
            RecordId::new_v7()
        ));
        let context = CommandContext::from_path(Path::new(&std::env::temp_dir())).unwrap();
        demo_project_with_example(
            &context,
            example,
            Some(Utf8PathBuf::from_path_buf(root.clone()).unwrap()),
        )
        .unwrap();
        let project = discover_from(&root).unwrap();
        let report = validate_project_index(&ProjectIndex::scan(&project).unwrap());
        let _ = fs::remove_dir_all(&root);
        assert!(
            report.ok && report.counts.warnings == 0,
            "{}: {:?}",
            example.as_str(),
            report.issues
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn mcp_tools_create_successors_for_a_split_that_a_human_accepts() {
    use rmcp::{model::CallToolRequestParams, transport::TokioChildProcess, ServiceExt};

    let p = Project::new("mcp");
    let old = p.accepted("Listwise deletion");
    let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_dadrock"));
    command.arg("mcp").arg("serve").arg(&p.root);
    let transport = TokioChildProcess::new(command).unwrap();
    let client = ().serve(transport).await.unwrap();
    let call = |name: &'static str, arguments: serde_json::Value| {
        let peer = client.peer().clone();
        async move {
            let serde_json::Value::Object(arguments) = arguments else {
                panic!("arguments must be an object");
            };
            peer.call_tool(CallToolRequestParams::new(name).with_arguments(arguments))
                .await
                .unwrap()
                .structured_content
                .unwrap()
        }
    };

    let mut halves = Vec::new();
    for title in ["Impute income", "Complete cases for the outcome"] {
        let created = call(
            "dadrock.create_candidate",
            json!({
                "title": title,
                "body_markdown": SECTIONS,
                "supersedes": [old.id.to_string()],
                "supersedes_reason": "Outcome and covariates need different treatment."
            }),
        )
        .await;
        let path = created["data"]["path"].as_str().unwrap().to_owned();
        assert!(source(Utf8Path::new(&path)).contains(&format!(
            "## Supersedes\n\n[[{}|Listwise deletion]]\n\nOutcome and covariates need different treatment.\n\n## Context",
            old.id
        )));
        halves.push(created["data"]["id"].as_str().unwrap().to_owned());
    }

    // Accepting is a human decision: MCP cannot do it, the command layer can.
    let refused = client
        .peer()
        .call_tool(
            CallToolRequestParams::new("dadrock.accept_record").with_arguments(
                json!({"query": halves[0], "with": [halves[1]]})
                    .as_object()
                    .cloned()
                    .unwrap(),
            ),
        )
        .await;
    assert!(refused.is_err(), "MCP accepted a candidate: {refused:?}");
    client.cancel().await.unwrap();
    assert!(p.successors_of(old.id).is_empty());

    let accepted = serde_json::to_value(accept_records(&p.context(), &halves).unwrap()).unwrap();
    assert_eq!(accepted["ok"], true);
    assert_eq!(
        accepted["data"]["superseded"][0]["successors"],
        json!(halves)
    );
    assert_eq!(p.successors_of(old.id).len(), 2);
}
