//! Spec rules that `dadrock validate` reports as errors and acceptance
//! refuses, and the `## Supersedes` convention checked the same way.

use std::{
    fs,
    path::{Path, PathBuf},
};

use dadrock::{
    index::ProjectIndex,
    links::{check_project_links, LinkIssueCode},
    project::{discover_from, init_project, ProjectContext},
    records::{accept_candidate, pending_supersedes, status_record},
    tags::{build_tag_report, TagIssueCode},
    validation::{
        validate_project_index, ProjectValidationIssue, ProjectValidationIssueCode,
        ProjectValidationReport,
    },
    DiagnosticCode, RecordId, Severity,
};

const SECTIONS: &str = "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n\
                        ## Alternatives\n\nNone considered.\n\n## Consequences\n\nEffects.\n";

const PREDECESSOR: &str = "0193fb1f-a3a0-7000-8000-000000000001";

struct Project {
    root: PathBuf,
    context: ProjectContext,
}

impl Project {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!("dadrock-{label}-{}", RecordId::new_v7()));
        fs::create_dir_all(&root).unwrap();
        let root = fs::canonicalize(root).unwrap();
        init_project(&root).unwrap();
        let context = discover_from(&root).unwrap();
        Self { root, context }
    }

    fn index(&self) -> ProjectIndex {
        ProjectIndex::scan(&self.context).unwrap()
    }

    fn validate(&self) -> ProjectValidationReport {
        validate_project_index(&self.index())
    }

    /// Write `content` as `dadr/<folder>/<name>.md`.
    fn write(&self, folder: &str, name: &str, content: &str) -> PathBuf {
        let path = self
            .root
            .join("dadr")
            .join(folder)
            .join(format!("{name}.md"));
        fs::write(&path, content).unwrap();
        path
    }

    fn candidate(&self, name: &str, id: &str, body: &str) -> PathBuf {
        self.write(
            "inbox",
            name,
            &format!("---\ntitle: {name}\nstatus: candidate\nid: {id}\n---\n\n{body}"),
        )
    }

    fn accepted(&self, name: &str, id: &str) -> PathBuf {
        self.write(
            "ledger",
            name,
            &format!("---\ntitle: {name}\nstatus: accepted\nid: {id}\n---\n\n{SECTIONS}"),
        )
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn issues_for<'a>(
    report: &'a ProjectValidationReport,
    path: &Path,
) -> Vec<&'a ProjectValidationIssue> {
    report
        .issues
        .iter()
        .filter(|issue| {
            issue
                .path
                .as_ref()
                .is_some_and(|issue_path| issue_path.as_std_path() == path)
        })
        .collect()
}

/// Accepting the candidate at `path` fails with a message that contains
/// `expected`, and leaves the file as it was.
fn assert_accept_refused(project: &Project, path: &Path, query: &str, expected: &str) {
    let before = fs::read(path).unwrap();
    let error = accept_candidate(&project.index(), query).unwrap_err();
    assert!(
        error.to_string().contains(expected),
        "`{error}` does not mention `{expected}`"
    );
    assert_eq!(fs::read(path).unwrap(), before, "{}", path.display());
}

// `## Supersedes` is a DADRock convention (docs-src/changing.md): at most one
// section, starting with record links by full id. Validation and `status`
// report what acceptance refuses, with the same message.
#[test]
fn validate_and_status_report_malformed_supersedes_that_acceptance_refuses() {
    let project = Project::new("supersedes-malformed");
    project.accepted("Predecessor", PREDECESSOR);
    let predecessor = RecordId::parse(PREDECESSOR).unwrap();
    let simple = PREDECESSOR.replace('-', "");
    let cases = [
        ("No link", "## Supersedes\n\nOnly a reason.\n\n".to_owned()),
        (
            "Two sections",
            format!(
                "## Supersedes\n[[{PREDECESSOR}]]\n\n## Note\n\nText.\n\n## Supersedes\n[[{PREDECESSOR}]]\n\n"
            ),
        ),
        (
            "Short id link",
            format!("## Supersedes\n[[{}]]\n\n", predecessor.short_id()),
        ),
        ("Unhyphenated link", format!("## Supersedes\n[[{simple}]]\n\n")),
        ("Braced link", format!("## Supersedes\n[[{{{PREDECESSOR}}}]]\n\n")),
        ("URN link", format!("## Supersedes\n[[urn:uuid:{PREDECESSOR}]]\n\n")),
        (
            "Repeated link",
            format!("## Supersedes\n[[{PREDECESSOR}]]\n[[{PREDECESSOR}|Again]]\n\n"),
        ),
    ];

    let mut candidates = Vec::new();
    for (index, (name, supersedes)) in cases.iter().enumerate() {
        let id = format!("0193fb{:02x}-0000-7000-8000-000000000001", 0x20 + index);
        let path = project.candidate(name, &id, &format!("{supersedes}{SECTIONS}"));
        candidates.push((name, id, path));
    }

    let report = project.validate();
    assert!(!report.ok);
    for (name, id, path) in &candidates {
        let issues = issues_for(&report, path);
        let invalid: Vec<_> = issues
            .iter()
            .filter(|issue| issue.code == ProjectValidationIssueCode::InvalidSupersedes)
            .collect();
        assert_eq!(invalid.len(), 1, "{name}: {issues:#?}");
        assert_eq!(invalid[0].severity, Severity::Error, "{name}");

        let status = status_record(&project.index(), id).unwrap();
        let diagnostic = status
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagnosticCode::InvalidSupersedes)
            .unwrap_or_else(|| panic!("{name}: {:?}", status.diagnostics));
        assert!(diagnostic.is_error());
        assert!(
            invalid[0].message.ends_with(&diagnostic.message),
            "{name}: {} / {}",
            invalid[0].message,
            diagnostic.message
        );

        // The web lifecycle shows the record's diagnostics; the pending
        // supersession does not report the same problem again.
        let pending = pending_supersedes(&project.index(), id).unwrap();
        assert!(pending.predecessors.is_empty(), "{name}");

        assert_accept_refused(&project, path, id, &diagnostic.message);
    }

    // Record links match hex digits case-insensitively (spec §8.1), so an
    // uppercase full id is a valid `## Supersedes` link.
    let upper = PREDECESSOR.to_uppercase();
    let id = "0193fbff-0000-7000-8000-000000000001";
    project.candidate(
        "Uppercase link",
        id,
        &format!("## Supersedes\n[[{upper}|Predecessor]]\n\n{SECTIONS}"),
    );
    let pending = pending_supersedes(&project.index(), id).unwrap();
    assert_eq!(pending.predecessors.len(), 1);
    assert_eq!(pending.predecessors[0].id, predecessor);
    let status = status_record(&project.index(), id).unwrap();
    assert!(status.diagnostics.is_empty(), "{:?}", status.diagnostics);
}

#[test]
fn validate_reports_candidates_that_supersede_themselves_or_a_non_accepted_record() {
    let project = Project::new("supersedes-targets");
    let own = "0193fb21-0000-7000-8000-000000000001";
    let other = "0193fb22-0000-7000-8000-000000000001";
    let names_other = "0193fb23-0000-7000-8000-000000000001";
    let own_path = project.candidate(
        "Supersedes itself",
        own,
        &format!("## Supersedes\n[[{own}]]\n\n{SECTIONS}"),
    );
    project.candidate("Still a candidate", other, SECTIONS);
    let names_other_path = project.candidate(
        "Supersedes a candidate",
        names_other,
        &format!("## Supersedes\n[[{other}]]\n\n{SECTIONS}"),
    );

    let report = project.validate();
    for (path, expected) in [
        (&own_path, "cannot supersede itself"),
        (&names_other_path, "is candidate"),
    ] {
        let issue = issues_for(&report, path)
            .into_iter()
            .find(|issue| issue.code == ProjectValidationIssueCode::InvalidSupersedes)
            .unwrap_or_else(|| panic!("{}: {:#?}", path.display(), report.issues));
        assert_eq!(issue.severity, Severity::Error);
        assert!(issue.message.contains(expected), "{}", issue.message);
    }
    assert_accept_refused(&project, &own_path, own, "cannot supersede itself");
    assert_accept_refused(&project, &names_other_path, names_other, "`accepted`");
}

// Spec §2: a record is UTF-8 without a byte-order mark.
#[test]
fn byte_order_mark_is_reported_and_acceptance_refuses_it() {
    let project = Project::new("byte-order-mark");
    let id = "0193fb20-0000-7000-8000-000000000001";
    let path = project.write(
        "inbox",
        "bom",
        &format!("\u{feff}---\ntitle: With BOM\nstatus: candidate\nid: {id}\n---\n\n{SECTIONS}"),
    );

    let report = project.validate();
    assert!(!report.ok);
    let issues = issues_for(&report, &path);
    assert!(
        issues.iter().any(
            |issue| issue.code == ProjectValidationIssueCode::RecordDiagnostic
                && issue.record_code == Some(DiagnosticCode::ByteOrderMark)
                && issue.severity == Severity::Error
        ),
        "{issues:#?}"
    );
    let status = status_record(&project.index(), id).unwrap();
    assert!(status
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::ByteOrderMark));

    assert_accept_refused(&project, &path, id, "byte-order mark");
    assert!(fs::read(&path).unwrap().starts_with("\u{feff}".as_bytes()));
}

// Spec §3: front matter is delimited by `---` lines. Acceptance refuses a
// `...` closing line instead of rewriting it to `---`.
#[test]
fn document_end_fence_is_reported_and_acceptance_refuses_rather_than_rewrites() {
    let project = Project::new("document-end-fence");
    let id = "0193fb20-0000-7000-8000-000000000001";
    let path = project.write(
        "inbox",
        "dots",
        &format!("---\ntitle: Dots\nstatus: candidate\nid: {id}\n...\n\n{SECTIONS}"),
    );

    let report = project.validate();
    assert!(!report.ok);
    assert!(
        issues_for(&report, &path)
            .iter()
            .any(
                |issue| issue.record_code == Some(DiagnosticCode::MalformedFrontMatter)
                    && issue.message.contains("`...`")
            ),
        "{:#?}",
        report.issues
    );

    assert_accept_refused(&project, &path, id, "`...`");
    assert!(fs::read_to_string(&path).unwrap().contains("\n...\n"));
}

// Spec §3 and §5: `id` and `successors` hold lowercase hyphenated UUIDv7s.
// Queries stay lenient: people may type any spelling of an id.
#[test]
fn non_canonical_ids_are_reported_and_acceptance_refuses_them() {
    let project = Project::new("non-canonical-ids");
    let upper = "0193FB20-0000-7000-8000-00000000000A";
    let path = project.candidate("Uppercase id", upper, SECTIONS);
    let successor = "0193fb21-0000-7000-8000-000000000002";
    project.accepted("Successor", successor);
    let predecessor = project.write(
        "archive",
        "predecessor",
        &format!(
            "---\ntitle: Predecessor\nstatus: accepted\nid: {PREDECESSOR}\nsuccessors:\n  - \"{{{successor}}}\"\n---\n\n{SECTIONS}"
        ),
    );

    let report = project.validate();
    assert!(!report.ok);
    for (path, field) in [(&path, "id"), (&predecessor, "successors")] {
        let issue = issues_for(&report, path)
            .into_iter()
            .find(|issue| issue.record_code == Some(DiagnosticCode::NonCanonicalRecordId))
            .unwrap_or_else(|| panic!("{}: {:#?}", path.display(), report.issues));
        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.field.as_deref(), Some(field));
    }
    // The successor link itself still resolves, so nothing else is reported.
    assert!(!report
        .issues
        .iter()
        .any(|issue| issue.code == ProjectValidationIssueCode::UnresolvedSuccessor));

    // Any spelling finds the record.
    let lowercase = upper.to_lowercase();
    for query in [upper, lowercase.as_str(), "0193FB200000"] {
        let status = status_record(&project.index(), query).unwrap();
        assert_eq!(status.record.id.to_string(), lowercase);
    }
    assert_accept_refused(&project, &path, upper, "lowercase hyphenated form");
}

// Spec §7: a tag consists of Unicode letters, numbers, emoji, `_`, `-`, and
// `/`.
#[test]
fn tags_outside_the_character_set_are_reported_and_acceptance_refuses_them() {
    let project = Project::new("tag-characters");
    let id = "0193fb20-0000-7000-8000-000000000001";
    let path = project.write(
        "inbox",
        "bad-tags",
        &format!(
            "---\ntitle: Bad tags\nstatus: candidate\nid: {id}\ntags:\n  - kind.sample\n  - foo:bar\n  - a,b\n  - x!y\n  - a–b\n---\n\n{SECTIONS}"
        ),
    );

    let report = build_tag_report(&project.index());
    assert!(!report.ok);
    let mut flagged: Vec<_> = report
        .issues
        .iter()
        .filter(|issue| issue.code == TagIssueCode::InvalidTagCharacter)
        .map(|issue| issue.tag.as_deref().unwrap())
        .collect();
    flagged.sort_unstable();
    assert_eq!(flagged, ["a,b", "a–b", "foo:bar", "kind.sample", "x!y"]);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.message.contains("`–`")));
    assert!(!report.tags.iter().any(|tag| tag.tag == "kind.sample"));

    let validation = project.validate();
    assert!(validation
        .issues
        .iter()
        .any(|issue| issue.code == ProjectValidationIssueCode::InvalidTag
            && issue.sub_code.as_deref() == Some("invalid_tag_character")
            && issue.severity == Severity::Error));

    assert_accept_refused(&project, &path, id, "tag `kind.sample` contains `.`");
}

#[test]
fn letters_numbers_and_emoji_are_valid_tag_characters() {
    let project = Project::new("tag-characters-valid");
    let id = "0193fb20-0000-7000-8000-000000000001";
    project.write(
        "inbox",
        "good-tags",
        &format!(
            "---\ntitle: Good tags\nstatus: candidate\nid: {id}\ntags:\n  - étoile\n  - nai\u{308}ve\n  - 日本/東京\n  - हिन्दी\n  - emoji-🚀\n  - 👩‍🔬\n  - y1984\n---\n\n\
             ## Context\n\nInline #kind/sample*, #topic/data. and *#x*.\n\n## Decision\n\nWhat.\n\n\
             ## Alternatives\n\nNone considered.\n\n## Consequences\n\nEffects.\n"
        ),
    );

    let report = build_tag_report(&project.index());
    assert!(report.issues.is_empty(), "{:#?}", report.issues);
    for tag in ["kind/sample", "topic/data", "👩‍🔬", "हिन्दी"] {
        assert!(
            report.tags.iter().any(|summary| summary.tag == tag),
            "{tag}"
        );
    }
    assert!(!report
        .tags
        .iter()
        .any(|summary| summary.tag == "kind/sample*"));

    let accepted = accept_candidate(&project.index(), id).unwrap();
    assert_eq!(accepted.accepted.len(), 1);
}

#[test]
fn acceptance_refuses_invalid_inline_tags() {
    let project = Project::new("inline-tag");
    let id = "0193fb20-0000-7000-8000-000000000001";
    let path = project.candidate("Inline tag", id, &format!("> #bad//inline\n\n{SECTIONS}"));
    assert_accept_refused(&project, &path, id, "empty `/` path segments");
}

// Link and inline tag issues give the line in the file, after the front
// matter, so `path:line` opens at the problem.
#[test]
fn link_and_tag_issues_report_file_lines() {
    let project = Project::new("file-lines");
    let id = "0193fb20-0000-7000-8000-000000000001";
    let content = format!(
        "---\ntitle: Lines\nstatus: accepted\nid: {id}\ntags:\n  - kind/sample\n---\n\n\
         ## Context\n\nSee [[0193fb2fffff]].\n\n## Decision\n\nWhat #bad//tag.\n\n\
         ## Alternatives\n\nNone considered.\n\n## Consequences\n\nEffects.\n"
    );
    let path = project.write("ledger", "lines", &content);
    let line_of = |needle: &str| {
        content
            .lines()
            .position(|line| line.contains(needle))
            .unwrap()
            + 1
    };
    let link_line = line_of("[[0193fb2fffff]]");
    let tag_line = line_of("#bad//tag");
    assert_eq!((link_line, tag_line), (11, 15));

    let links = check_project_links(&project.index());
    let broken = links
        .issues
        .iter()
        .find(|issue| issue.code == LinkIssueCode::BrokenWikilink)
        .unwrap();
    assert_eq!(broken.line, link_line);

    let tags = build_tag_report(&project.index());
    let bad_tag = tags
        .issues
        .iter()
        .find(|issue| issue.code == TagIssueCode::EmptyTagSegment)
        .unwrap();
    assert_eq!(bad_tag.line, Some(tag_line));

    let report = project.validate();
    let lines: Vec<_> = issues_for(&report, &path)
        .into_iter()
        .filter_map(|issue| issue.line.map(|line| (issue.code, line)))
        .collect();
    assert!(lines.contains(&(ProjectValidationIssueCode::BrokenWikilink, link_line)));
    assert!(lines.contains(&(ProjectValidationIssueCode::InvalidTag, tag_line)));
}
