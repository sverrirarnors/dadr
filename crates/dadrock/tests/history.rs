use std::{
    fs,
    path::{Path, PathBuf},
};

use dadrock::{
    commands::{accept_record, diff, history, new_record_with_body, CommandContext, TextDiffKind},
    history::{render_diff_text, render_history_text, RecordHistory},
    index::ProjectIndex,
    project::{discover_from, init_project},
    records::supersede_records,
    RecordId,
};

fn temp_project(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("dadrock-history-{label}-{}", RecordId::new_v7()));
    fs::create_dir_all(&path).unwrap();
    init_project(&path).unwrap();
    path
}

fn context(root: &Path) -> CommandContext {
    CommandContext::from_path(root).unwrap()
}

fn body(decision: &str) -> String {
    format!(
        "## Context\n\nSome rows are missing.\n\n## Decision\n\n{decision}\n\n## Alternatives\n\nOther rules.\n\n## Consequences\n\nResults change.\n"
    )
}

fn supersedes(predecessors: &[&str], reason: Option<&str>) -> String {
    let mut section = String::from("## Supersedes\n\n");
    for predecessor in predecessors {
        section.push_str(&format!("[[{predecessor}]]\n"));
    }
    if let Some(reason) = reason {
        section.push_str(&format!("\n{reason}\n"));
    }
    section.push('\n');
    section
}

/// Write an accepted record: in the archive when it has successors, else in
/// the ledger.
fn write_record(
    root: &Path,
    id: &str,
    title: &str,
    successors: &[&str],
    tags: &[&str],
    body: &str,
) {
    let mut content = format!("---\ntitle: {title}\nstatus: accepted\nid: {id}\n");
    if !successors.is_empty() {
        content.push_str("successors:\n");
        for successor in successors {
            content.push_str(&format!("  - {successor}\n"));
        }
    }
    if !tags.is_empty() {
        content.push_str("tags:\n");
        for tag in tags {
            content.push_str(&format!("  - {tag}\n"));
        }
    }
    content.push_str("---\n\n");
    content.push_str(body);
    let folder = if successors.is_empty() {
        "ledger"
    } else {
        "archive"
    };
    let short_id: String = id.replace('-', "").chars().take(12).collect();
    fs::write(
        root.join("dadr")
            .join(folder)
            .join(format!("{short_id}-record.md")),
        content,
    )
    .unwrap();
}

fn open(root: &Path, query: &str) -> RecordHistory {
    let envelope = history(&context(root), query, false).unwrap();
    assert!(envelope.ok);
    envelope.data
}

fn version_titles(history: &RecordHistory) -> Vec<&str> {
    history
        .versions
        .iter()
        .map(|version| version.record.title.as_str())
        .collect()
}

fn link_pairs(history: &RecordHistory) -> Vec<(String, String)> {
    history
        .links
        .iter()
        .map(|link| (link.predecessor.short_id(), link.successor.short_id()))
        .collect()
}

// The RFC's split: a predecessor with an earlier version, superseded by two
// records accepted together.
const PAIRWISE: &str = "01938a77-e310-7000-8000-000000000001";
const LISTWISE: &str = "01938c20-a3b4-7000-8000-000000000002";
const COMPLETE: &str = "0193b2a1-c0f4-7000-8000-000000000003";
const IMPUTATION: &str = "0193b2a1-d117-7000-8000-000000000004";

fn split_project() -> PathBuf {
    let root = temp_project("split");
    write_record(
        &root,
        PAIRWISE,
        "Pairwise deletion",
        &[LISTWISE],
        &["topic/attrition"],
        &body("Use pairwise deletion."),
    );
    write_record(
        &root,
        LISTWISE,
        "Listwise deletion",
        &[IMPUTATION, COMPLETE],
        &["topic/attrition"],
        &format!(
            "{}{}",
            supersedes(
                &[PAIRWISE],
                Some("Complete cases lose a third of the sample.")
            ),
            body("Use listwise deletion.")
        ),
    );
    for (id, title, tag, decision) in [
        (
            IMPUTATION,
            "Multiple imputation for income",
            "topic/missing-data",
            "Impute income.",
        ),
        (
            COMPLETE,
            "Complete cases for the outcome",
            "topic/attrition",
            "Drop rows without the outcome.",
        ),
    ] {
        write_record(
            &root,
            id,
            title,
            &[],
            &[tag],
            &format!(
                "{}{}",
                supersedes(
                    &[LISTWISE],
                    Some("Outcome and covariates need different treatment.")
                ),
                body(decision)
            ),
        );
    }
    root
}

// The RFC's merge: two rules, one of which had an earlier version, become one.
const TRIM: &str = "01938f11-aa02-7000-8000-000000000001";
const AGES: &str = "01939f2c-4d07-7000-8000-000000000002";
const WINSORIZE: &str = "0193a0b7-e911-7000-8000-000000000003";
const ONE_RULE: &str = "0193c4e1-a2b0-7000-8000-000000000004";

fn merge_project() -> PathBuf {
    let root = temp_project("merge");
    write_record(
        &root,
        TRIM,
        "Trim income at p95",
        &[WINSORIZE],
        &[],
        &body("Trim income."),
    );
    write_record(
        &root,
        WINSORIZE,
        "Winsorize income at p99",
        &[ONE_RULE],
        &[],
        &body("Winsorize income."),
    );
    write_record(
        &root,
        AGES,
        "Drop implausible ages",
        &[ONE_RULE],
        &[],
        &body("Drop ages above 110."),
    );
    write_record(
        &root,
        ONE_RULE,
        "One outlier rule for all covariates",
        &[],
        &[],
        &format!(
            "{}{}",
            supersedes(
                &[WINSORIZE, AGES],
                Some("The two rules disagreed at the 99th percentile.")
            ),
            body("Winsorize every continuous covariate at p99.")
        ),
    );
    root
}

#[test]
fn single_record_is_its_own_current_version() {
    let root = temp_project("single");
    let context = context(&root);
    let created = new_record_with_body(&context, "Only version", &[], &body("Decide."))
        .unwrap()
        .data;

    let history = open(&root, &created.id.to_string());
    assert_eq!(history.record, created.id);
    assert_eq!(version_titles(&history), ["Only version"]);
    assert!(history.versions[0].record.current);
    assert!(history.links.is_empty());
    assert!(history.neighbors.is_empty());
    assert_eq!(
        render_history_text(&history),
        format!(
            "◉   {}  Only version  candidate  current\n",
            created.id.short_id()
        )
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn linear_chain_built_by_acceptance_and_supersede_has_reasons_where_written() {
    let root = temp_project("linear");
    let context = context(&root);
    let first = new_record_with_body(&context, "Drop missing rows", &[], &body("Drop them."))
        .unwrap()
        .data
        .id;
    accept_record(&context, &first.to_string()).unwrap();
    let second_body = format!(
        "## Supersedes\n\n[[{first}|Drop missing rows]]\n\nDropping lost a third of the sample.\n\n{}",
        body("Impute them.")
    );
    let second = new_record_with_body(&context, "Impute missing rows", &[], &second_body)
        .unwrap()
        .data
        .id;
    accept_record(&context, &second.to_string()).unwrap();
    let third = new_record_with_body(
        &context,
        "Impute with chained equations",
        &[],
        &body("Use MICE."),
    )
    .unwrap()
    .data
    .id;
    accept_record(&context, &third.to_string()).unwrap();
    let project = discover_from(&root).unwrap();
    supersede_records(
        &ProjectIndex::scan(&project).unwrap(),
        &[second.to_string()],
        &[third.to_string()],
    )
    .unwrap();

    let histories: Vec<RecordHistory> = [first, second, third]
        .iter()
        .map(|id| open(&root, &id.to_string()))
        .collect();
    for history in &histories {
        assert_eq!(
            version_titles(history),
            [
                "Impute with chained equations",
                "Impute missing rows",
                "Drop missing rows"
            ]
        );
        assert_eq!(history.links, histories[0].links);
        assert!(history.neighbors.is_empty());
    }
    assert_eq!(histories[1].record, second);

    let history = &histories[0];
    let current: Vec<bool> = history
        .versions
        .iter()
        .map(|version| version.record.current)
        .collect();
    assert_eq!(current, [true, false, false]);
    assert!(history.versions[1].record.superseded);
    assert_eq!(history.links.len(), 2);
    // Linked by `dadrock supersede`: no `## Supersedes` section, no reason.
    assert_eq!(history.links[0].predecessor, second);
    assert_eq!(history.links[0].successor, third);
    assert_eq!(history.links[0].reason, None);
    // Linked by accepting a successor: the reason comes from its section.
    assert_eq!(history.links[1].predecessor, first);
    assert_eq!(history.links[1].successor, second);
    assert_eq!(
        history.links[1].reason.as_deref(),
        Some("Dropping lost a third of the sample.")
    );
    assert!(!history.versions[1].body.contains("## Supersedes"));
    assert!(history.versions[1].body.starts_with("## Context\n"));

    assert_eq!(
        render_history_text(history),
        format!(
            "●   {}  Impute with chained equations  current\n\
             ●   {}  Impute missing rows\n\
             │   Dropping lost a third of the sample.\n\
             ◉   {}  Drop missing rows\n",
            third.short_id(),
            second.short_id(),
            first.short_id()
        )
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn split_shows_both_successors_and_joins_them_at_the_predecessor() {
    let root = split_project();

    let history = open(&root, PAIRWISE);
    assert_eq!(
        version_titles(&history),
        [
            "Multiple imputation for income",
            "Complete cases for the outcome",
            "Listwise deletion",
            "Pairwise deletion"
        ]
    );
    assert!(history.neighbors.is_empty());
    assert_eq!(
        render_history_text(&history),
        "\
●   0193b2a1d117  Multiple imputation for income  current
│ ● 0193b2a1c0f4  Complete cases for the outcome  current
├─╯ Outcome and covariates need different treatment.
●   01938c20a3b4  Listwise deletion
│   Complete cases lose a third of the sample.
◉   01938a77e310  Pairwise deletion
"
    );
    assert_eq!(open(&root, LISTWISE).versions, history.versions);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn split_opened_at_one_half_lists_the_other_half_as_a_neighbor() {
    let root = split_project();

    let history = open(&root, COMPLETE);
    assert_eq!(
        version_titles(&history),
        [
            "Complete cases for the outcome",
            "Listwise deletion",
            "Pairwise deletion"
        ]
    );
    let neighbors: Vec<&str> = history
        .neighbors
        .iter()
        .map(|neighbor| neighbor.title.as_str())
        .collect();
    assert_eq!(neighbors, ["Multiple imputation for income"]);
    assert!(history.neighbors[0].current);
    // The neighbor's link touches a version, so it is listed with its reason.
    assert!(history.links.iter().any(|link| {
        link.predecessor.to_string() == LISTWISE
            && link.successor.to_string() == IMPUTATION
            && link.reason.as_deref() == Some("Outcome and covariates need different treatment.")
    }));
    assert_eq!(
        render_history_text(&history),
        "\
◉   0193b2a1c0f4  Complete cases for the outcome  current
│   Outcome and covariates need different treatment.
●   01938c20a3b4  Listwise deletion
│   also became 0193b2a1d117  Multiple imputation for income
│   Complete cases lose a third of the sample.
●   01938a77e310  Pairwise deletion
"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn merge_opens_its_predecessors_with_the_reason_on_the_fork() {
    let root = merge_project();

    let history = open(&root, ONE_RULE);
    assert_eq!(
        version_titles(&history),
        [
            "One outlier rule for all covariates",
            "Winsorize income at p99",
            "Drop implausible ages",
            "Trim income at p95"
        ]
    );
    let current: Vec<bool> = history
        .versions
        .iter()
        .map(|version| version.record.current)
        .collect();
    assert_eq!(current, [true, false, false, false]);
    assert_eq!(
        link_pairs(&history),
        [
            ("0193a0b7e911".to_owned(), "0193c4e1a2b0".to_owned()),
            ("01939f2c4d07".to_owned(), "0193c4e1a2b0".to_owned()),
            ("01938f11aa02".to_owned(), "0193a0b7e911".to_owned()),
        ]
    );
    for link in &history.links[..2] {
        assert_eq!(
            link.reason.as_deref(),
            Some("The two rules disagreed at the 99th percentile.")
        );
    }
    assert_eq!(history.links[2].reason, None);
    assert_eq!(
        render_history_text(&history),
        "\
◉   0193c4e1a2b0  One outlier rule for all covariates  current
├─╮ The two rules disagreed at the 99th percentile.
● │ 0193a0b7e911  Winsorize income at p99
│ ● 01939f2c4d07  Drop implausible ages
●   01938f11aa02  Trim income at p95
"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn merge_opened_at_one_input_lists_the_other_input_as_a_neighbor() {
    let root = merge_project();

    let history = open(&root, WINSORIZE);
    assert_eq!(
        version_titles(&history),
        [
            "One outlier rule for all covariates",
            "Winsorize income at p99",
            "Trim income at p95"
        ]
    );
    assert_eq!(history.neighbors.len(), 1);
    assert_eq!(history.neighbors[0].id.to_string(), AGES);
    assert!(!history.neighbors[0].current);
    assert_eq!(
        render_history_text(&history),
        "\
●   0193c4e1a2b0  One outlier rule for all covariates  current
│   merged with 01939f2c4d07  Drop implausible ages
│   The two rules disagreed at the 99th percentile.
◉   0193a0b7e911  Winsorize income at p99
●   01938f11aa02  Trim income at p95
"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn diamond_forks_and_joins_and_reads_the_same_from_either_end() {
    let root = temp_project("diamond");
    let root_rule = "01930000-0001-7000-8000-000000000001";
    let left = "01930000-0002-7000-8000-000000000002";
    let right = "01930000-0003-7000-8000-000000000003";
    let rejoined = "01930000-0004-7000-8000-000000000004";
    write_record(
        &root,
        root_rule,
        "Root rule",
        &[left, right],
        &[],
        &body("Root."),
    );
    write_record(
        &root,
        left,
        "Left branch",
        &[rejoined],
        &[],
        &format!(
            "{}{}",
            supersedes(&[root_rule], Some("Left needs it.")),
            body("Left.")
        ),
    );
    write_record(
        &root,
        right,
        "Right branch",
        &[rejoined],
        &[],
        &format!(
            "{}{}",
            supersedes(&[root_rule], Some("Right needs it.")),
            body("Right.")
        ),
    );
    write_record(
        &root,
        rejoined,
        "Rejoined rule",
        &[],
        &[],
        &format!(
            "{}{}",
            supersedes(&[left, right], Some("The branches converged.")),
            body("Rejoined.")
        ),
    );

    let from_root = open(&root, root_rule);
    let from_tip = open(&root, rejoined);
    assert_eq!(from_root.versions, from_tip.versions);
    assert_eq!(from_root.links, from_tip.links);
    assert_eq!(
        version_titles(&from_tip),
        ["Rejoined rule", "Right branch", "Left branch", "Root rule"]
    );
    assert_eq!(
        render_history_text(&from_tip),
        "\
◉   019300000004  Rejoined rule  current
├─╮ The branches converged.
● │ 019300000003  Right branch
│ ● 019300000002  Left branch
├─╯ 019300000003: Right needs it.
│   019300000002: Left needs it.
●   019300000001  Root rule
"
    );

    // From one branch, the other branch is a neighbor at both ends.
    let from_left = open(&root, left);
    assert_eq!(
        version_titles(&from_left),
        ["Rejoined rule", "Left branch", "Root rule"]
    );
    assert_eq!(from_left.neighbors.len(), 1);
    assert_eq!(from_left.neighbors[0].id.to_string(), right);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn successor_created_before_its_predecessor_is_listed_above_it() {
    let root = temp_project("older-successor");
    let older = "01930000-0001-7000-8000-000000000001";
    let newer = "01930000-0009-7000-8000-000000000009";
    write_record(&root, older, "Written first", &[], &[], &body("First."));
    write_record(
        &root,
        newer,
        "Written later",
        &[older],
        &[],
        &body("Later."),
    );

    let history = open(&root, newer);
    assert_eq!(version_titles(&history), ["Written first", "Written later"]);
    assert!(history.versions[0].record.current);
    assert_eq!(
        render_history_text(&history),
        "\
●   019300000001  Written first  current
◉   019300000009  Written later
"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn several_tips_and_roots_get_their_own_lanes() {
    let root = temp_project("lanes");
    // Root splits into X and Y; X splits into T1 and T3, Y becomes T2. Joining
    // T1 and T3 at X crosses Y's lane.
    let root_id = "01930000-0001-7000-8000-000000000001";
    let y = "01930000-0002-7000-8000-000000000002";
    let x = "01930000-0003-7000-8000-000000000003";
    let t3 = "01930000-0004-7000-8000-000000000004";
    let t2 = "01930000-0005-7000-8000-000000000005";
    let t1 = "01930000-0006-7000-8000-000000000006";
    write_record(&root, root_id, "Root", &[x, y], &[], &body("Root."));
    write_record(&root, x, "X", &[t1, t3], &[], &body("X."));
    write_record(&root, y, "Y", &[t2], &[], &body("Y."));
    for (id, title) in [(t1, "T1"), (t2, "T2"), (t3, "T3")] {
        write_record(&root, id, title, &[], &[], &body(title));
    }

    let history = open(&root, root_id);
    assert_eq!(
        version_titles(&history),
        ["T1", "T2", "T3", "X", "Y", "Root"]
    );
    assert_eq!(
        render_history_text(&history),
        "\
●     019300000006  T1    current
│ ●   019300000005  T2    current
│ │ ● 019300000004  T3    current
├─┼─╯
● │   019300000003  X
│ ●   019300000002  Y
├─╯
◉     019300000001  Root
"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unresolved_successors_and_cycles_are_reported_not_fatal() {
    let root = temp_project("bad-graph");
    let a = "01930000-0001-7000-8000-000000000001";
    let b = "01930000-0002-7000-8000-000000000002";
    let c = "01930000-0003-7000-8000-000000000003";
    let missing = "01930000-0009-7000-8000-000000000009";
    write_record(&root, a, "A", &[b], &[], &body("A."));
    write_record(&root, b, "B", &[c, missing], &[], &body("B."));
    write_record(&root, c, "C", &[a], &[], &body("C."));

    let envelope = history(&context(&root), b, false).unwrap();
    assert!(envelope.ok);
    assert_eq!(envelope.data.versions.len(), 3);
    assert_eq!(envelope.data.links.len(), 3);
    let codes: Vec<&str> = envelope
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect();
    assert!(codes.contains(&"unresolved_successor"));
    assert!(codes.contains(&"successor_cycle"));
    assert!(!render_history_text(&envelope.data).is_empty());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn history_json_has_the_documented_shape() {
    let root = split_project();

    let plain = serde_json::to_value(history(&context(&root), COMPLETE, false).unwrap()).unwrap();
    assert_eq!(plain["command"], "history");
    assert_eq!(plain["ok"], true);
    let data = &plain["data"];
    assert_eq!(data["record"], COMPLETE);
    let mut keys: Vec<&str> = data
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    assert_eq!(keys, ["links", "neighbors", "record", "versions"]);

    let mut version_keys: Vec<&str> = data["versions"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    version_keys.sort_unstable();
    assert_eq!(
        version_keys,
        [
            "body",
            "current",
            "id",
            "path",
            "short_id",
            "status",
            "superseded",
            "tags",
            "title"
        ]
    );
    assert_eq!(data["versions"][0]["short_id"], "0193b2a1c0f4");
    assert_eq!(data["versions"][0]["status"], "accepted");
    assert_eq!(
        data["versions"][0]["tags"],
        serde_json::json!(["topic/attrition"])
    );

    let mut neighbor_keys: Vec<&str> = data["neighbors"][0]
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    neighbor_keys.sort_unstable();
    assert_eq!(
        neighbor_keys,
        [
            "current",
            "id",
            "path",
            "short_id",
            "status",
            "superseded",
            "tags",
            "title"
        ]
    );

    for link in data["links"].as_array().unwrap() {
        let mut link_keys: Vec<&str> = link
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        link_keys.sort_unstable();
        assert_eq!(link_keys, ["predecessor", "reason", "successor"]);
    }

    let patched = serde_json::to_value(history(&context(&root), COMPLETE, true).unwrap()).unwrap();
    for link in patched["data"]["links"].as_array().unwrap() {
        let mut diff_keys: Vec<&str> = link["diff"]
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        diff_keys.sort_unstable();
        assert_eq!(
            diff_keys,
            ["changed", "lines", "tags_added", "tags_removed", "title"]
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn patch_prints_each_links_diff_under_it() {
    let root = split_project();

    let history = history(&context(&root), LISTWISE, true).unwrap().data;
    let text = render_history_text(&history);
    assert!(text.contains(
        "├─╯ Outcome and covariates need different treatment.\n\
         │   diff 01938c20a3b4 → 0193b2a1d117\n\
         │   title: Listwise deletion → Multiple imputation for income\n\
         │   tags: +topic/missing-data −topic/attrition\n"
    ));
    assert!(text.contains("│   - Use listwise deletion.\n│   + Impute income.\n"));
    assert!(text.contains("│   diff 01938c20a3b4 → 0193b2a1c0f4\n"));
    assert!(text.contains(
        "│   Complete cases lose a third of the sample.\n\
         │   diff 01938a77e310 → 01938c20a3b4\n\
         │   title: Pairwise deletion → Listwise deletion\n"
    ));
    assert!(!text.contains("## Supersedes"));
    assert!(text.ends_with("●   01938a77e310  Pairwise deletion\n"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn diff_compares_title_tags_and_body_without_supersedes() {
    let root = split_project();

    let envelope = diff(&context(&root), LISTWISE, IMPUTATION).unwrap();
    assert_eq!(envelope.command, "diff");
    let result = envelope.data;
    assert_eq!(result.old.id.to_string(), LISTWISE);
    assert_eq!(result.new.id.to_string(), IMPUTATION);
    assert!(result.diff.changed);
    let title = result.diff.title.as_ref().unwrap();
    assert_eq!(title.old, "Listwise deletion");
    assert_eq!(title.new, "Multiple imputation for income");
    assert_eq!(result.diff.tags_added, ["topic/missing-data"]);
    assert_eq!(result.diff.tags_removed, ["topic/attrition"]);
    let changed: Vec<(TextDiffKind, &str)> = result
        .diff
        .lines
        .iter()
        .filter(|line| line.kind != TextDiffKind::Context)
        .map(|line| (line.kind, line.text.as_str()))
        .collect();
    // Both bodies start with a `## Supersedes` section naming different
    // records; neither appears in the diff.
    assert_eq!(
        changed,
        [
            (TextDiffKind::Removed, "Use listwise deletion."),
            (TextDiffKind::Added, "Impute income.")
        ]
    );
    assert_eq!(
        render_diff_text(&result),
        "\
--- 01938c20a3b4  Listwise deletion
+++ 0193b2a1d117  Multiple imputation for income
title: Listwise deletion → Multiple imputation for income
tags: +topic/missing-data −topic/attrition

  ## Decision

- Use listwise deletion.
+ Impute income.

  ## Alternatives

"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn diff_normalizes_line_endings() {
    let root = temp_project("crlf");
    let unix = "01930000-0001-7000-8000-000000000001";
    let windows = "01930000-0002-7000-8000-000000000002";
    write_record(
        &root,
        unix,
        "Same rule",
        &[],
        &["kind/sample"],
        &body("Keep it."),
    );
    let content = format!(
        "---\ntitle: Same rule\nstatus: accepted\nid: {windows}\ntags:\n  - kind/sample\n---\n\n{}{}",
        supersedes(&[unix], Some("Saved on Windows.")),
        body("Keep it.")
    )
    .replace('\n', "\r\n");
    fs::write(root.join("dadr/ledger/019300000002-windows.md"), content).unwrap();

    let result = diff(&context(&root), unix, windows).unwrap().data;
    assert!(!result.diff.changed, "{:?}", result.diff);
    assert!(result.diff.title.is_none());
    assert!(result
        .diff
        .lines
        .iter()
        .all(|line| line.kind == TextDiffKind::Context && !line.text.contains('\r')));
    assert_eq!(
        render_diff_text(&result),
        "--- 019300000001  Same rule\n+++ 019300000002  Same rule\n(no changes)\n"
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn history_and_diff_report_unknown_records() {
    let root = temp_project("unknown");
    assert!(history(&context(&root), "no such record", false).is_err());
    assert!(diff(&context(&root), "no such record", "another").is_err());
    fs::remove_dir_all(root).unwrap();
}
