use std::{fs, path::PathBuf};

use dadrock::{
    commands::{
        demo_project, demo_project_with_example, reject_record, save_record_source, CommandContext,
        SaveRecordSourceRequest,
    },
    demo::DemoExample,
    index::ProjectIndex,
    links::{check_project_links, LinkIssueCode},
    project::{discover_from, init_project, ProjectContext},
    records::{
        accept_candidate, create_candidate, create_candidate_with_body, reject_candidate,
        scan_records, status_record, supersede_record, supersede_records, FindResolution,
        MatchKind,
    },
    tags::{build_tag_report, normalize_tag_filter, record_ids_for_tag, TagIssueCode},
    tasks::{list_task_runs, read_task_result},
    templates::{resolve_template, TemplateName, TemplateSource},
    validation::{validate_project_index, ProjectValidationIssueCode},
    DiagnosticCode, RecordId, RecordStatus,
};

fn temp_project(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("dadrock-{label}-{}", RecordId::new_v7()));
    if path.exists() {
        fs::remove_dir_all(&path).unwrap();
    }
    fs::create_dir_all(&path).unwrap();
    path
}

fn project_index(project: &ProjectContext) -> ProjectIndex {
    ProjectIndex::scan(project).unwrap()
}

fn dadr_dir(root: &std::path::Path) -> PathBuf {
    root.join("dadr")
}

fn records_dir(root: &std::path::Path) -> PathBuf {
    dadr_dir(root).join("ledger")
}

fn inbox_dir(root: &std::path::Path) -> PathBuf {
    dadr_dir(root).join("inbox")
}

fn archive_dir(root: &std::path::Path) -> PathBuf {
    dadr_dir(root).join("archive")
}

fn md_files(dir: &std::path::Path) -> Vec<String> {
    let mut names: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".md"))
        .collect();
    names.sort();
    names
}

const FILLED_SECTIONS: &str = "## Context\n\nWhy.\n\n## Decision\n\nWhat.\n\n\
                               ## Alternatives\n\nNone considered.\n\n## Consequences\n\nEffects.\n";

fn write_record(root: &std::path::Path, prefix: &str, title: &str, status: &str, id: &str) {
    write_record_with_successors(root, prefix, title, status, id, &[]);
}

fn write_record_with_successors(
    root: &std::path::Path,
    prefix: &str,
    title: &str,
    status: &str,
    id: &str,
    successors: &[&str],
) {
    let mut front_matter = format!("---\ntitle: {title}\nstatus: {status}\nid: {id}\n");
    if !successors.is_empty() {
        front_matter.push_str("successors:\n");
        for successor in successors {
            front_matter.push_str("  - ");
            front_matter.push_str(successor);
            front_matter.push('\n');
        }
    }
    front_matter.push_str("---\n\n");
    front_matter.push_str(FILLED_SECTIONS);
    fs::write(
        records_dir(root).join(format!(
            "{prefix}-{}.md",
            title.to_lowercase().replace(' ', "-")
        )),
        front_matter,
    )
    .unwrap();
}

fn write_record_with_body(
    root: &std::path::Path,
    prefix: &str,
    title: &str,
    status: &str,
    id: &str,
    body: &str,
) {
    fs::write(
        records_dir(root).join(format!(
            "{prefix}-{}.md",
            title.to_lowercase().replace(' ', "-")
        )),
        format!("---\ntitle: {title}\nstatus: {status}\nid: {id}\n---\n\n{body}"),
    )
    .unwrap();
}

fn issue_codes(
    report: &dadrock::validation::ProjectValidationReport,
) -> Vec<ProjectValidationIssueCode> {
    report.issues.iter().map(|issue| issue.code).collect()
}

fn link_issue_codes(report: &dadrock::links::LinkCheckReport) -> Vec<LinkIssueCode> {
    report.issues.iter().map(|issue| issue.code).collect()
}

fn tag_issue_codes(report: &dadrock::tags::TagReport) -> Vec<TagIssueCode> {
    report.issues.iter().map(|issue| issue.code).collect()
}

#[test]
fn init_is_idempotent_and_creates_layout() {
    let root = temp_project("init");

    let first = init_project(&root).unwrap();
    assert_eq!(first.project_dir, dadr_dir(&root));
    assert!(inbox_dir(&root).is_dir());
    assert!(records_dir(&root).is_dir());
    assert!(archive_dir(&root).is_dir());
    for old_folder in ["accepted", "candidates", "rejected"] {
        assert!(!dadr_dir(&root).join(old_folder).exists());
    }
    assert!(dadr_dir(&root).join("templates").is_dir());
    assert!(dadr_dir(&root).join("tasks").is_dir());
    assert!(dadr_dir(&root).join("runs/.gitignore").is_file());
    assert!(!dadr_dir(&root).join("dadr.toml").exists());
    assert!(!root.join(".dadr-dir").exists());
    assert!(!root.join(".dadrock").exists());
    assert!(!first.created.is_empty());

    let second = init_project(&root).unwrap();
    assert_eq!(second.created.len(), 0);
    assert!(second
        .existing
        .iter()
        .any(|path| path.as_str().ends_with("dadr/ledger")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn template_resolution_uses_built_in_until_project_template_exists() {
    let root = temp_project("template-resolution");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();

    let built_in = resolve_template(&project, TemplateName::Candidate).unwrap();
    assert_eq!(built_in.source, TemplateSource::BuiltIn);
    assert!(built_in.path.is_none());

    fs::write(
        dadr_dir(&root).join("templates/candidate.md"),
        "## Custom\n",
    )
    .unwrap();
    let project_template = resolve_template(&project, TemplateName::Candidate).unwrap();
    assert_eq!(project_template.source, TemplateSource::Project);
    let expected_path =
        camino::Utf8PathBuf::from_path_buf(dadr_dir(&root).join("templates/candidate.md")).unwrap();
    assert_eq!(project_template.path, Some(expected_path));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_validation_reports_candidate_template_that_is_not_readable_file() {
    let root = temp_project("validation-config-template-read");
    init_project(&root).unwrap();
    fs::remove_file(dadr_dir(&root).join("templates/candidate.md")).ok();
    fs::create_dir_all(dadr_dir(&root).join("templates/candidate.md")).unwrap();

    let project = discover_from(&root).unwrap();
    let report = validate_project_index(&project_index(&project));
    let codes = issue_codes(&report);
    assert!(!report.ok);
    assert!(codes.contains(&ProjectValidationIssueCode::TemplateReadError));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_validation_allows_missing_or_readable_candidate_template() {
    let root = temp_project("validation-config-template-ok");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();

    let missing_report = validate_project_index(&project_index(&project));
    assert!(missing_report.ok);
    assert!(missing_report.issues.is_empty());

    fs::write(
        dadr_dir(&root).join("templates/candidate.md"),
        "## Context\n\n## Decision\n\n## Alternatives\n\n## Consequences\n",
    )
    .unwrap();
    let readable_report = validate_project_index(&project_index(&project));
    assert!(readable_report.ok);
    assert!(readable_report.issues.is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn discovery_works_from_nested_directory() {
    let root = temp_project("discover");
    init_project(&root).unwrap();
    let nested = root.join("analysis/subdir");
    fs::create_dir_all(&nested).unwrap();

    let project = discover_from(&nested).unwrap();
    assert_eq!(project.project_dir, dadr_dir(&root));
    assert_eq!(project.inbox_dir(), inbox_dir(&root));
    assert_eq!(project.ledger_dir(), records_dir(&root));
    assert_eq!(project.archive_dir(), archive_dir(&root));
    assert!(project.initialized);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn discovery_does_not_cross_git_root_to_sibling_dadr_directory() {
    let workspace = temp_project("discover-sibling");
    let sibling_dadr = workspace.join("dadr");
    init_project(&sibling_dadr).unwrap();

    let project_root = workspace.join("analysis-project");
    fs::create_dir_all(project_root.join(".git")).unwrap();

    let project = discover_from(&project_root).unwrap();
    assert!(!project.initialized);
    assert_eq!(project.project_dir, project_root.join("dadr"));

    fs::remove_dir_all(workspace).unwrap();
}

#[test]
fn demo_seeds_clean_spec_showcase() {
    let root = temp_project("demo");
    let context = CommandContext::from_path(&root).unwrap();

    let target = camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap();
    let seeded = demo_project(&context, Some(target)).unwrap();
    assert!(seeded.ok);
    assert_eq!(seeded.command, "demo");
    assert_eq!(seeded.data.records.len(), 12);
    assert!(seeded
        .data
        .support_paths
        .iter()
        .any(|path| path.as_str().ends_with("examples/code-tags.py")));
    assert_eq!(seeded.changed_paths, seeded.data.created_paths);

    let project = discover_from(&root).unwrap();
    let index = project_index(&project);
    let summaries = index.summaries();
    assert_eq!(summaries.len(), 12);
    assert_eq!(
        summaries
            .iter()
            .filter(|record| record.status == RecordStatus::Accepted && !record.superseded)
            .count(),
        5
    );
    assert_eq!(
        summaries
            .iter()
            .filter(|record| record.status == RecordStatus::Accepted && record.superseded)
            .count(),
        3
    );
    assert_eq!(
        summaries
            .iter()
            .filter(|record| record.status == RecordStatus::Candidate)
            .count(),
        1
    );
    assert_eq!(
        summaries
            .iter()
            .filter(|record| record.status == RecordStatus::Rejected)
            .count(),
        3
    );

    let split = status_record(&project_index(&project), "Complete-case analysis sample").unwrap();
    assert_eq!(split.record.status, RecordStatus::Accepted);
    assert!(split.record.superseded);
    assert_eq!(split.successors.len(), 2);

    let consolidation = status_record(&project_index(&project), "Shared cleaning rules").unwrap();
    let predecessor_titles: Vec<_> = consolidation
        .predecessors
        .iter()
        .map(|record| record.title.as_str())
        .collect();
    assert_eq!(predecessor_titles.len(), 2);
    assert!(predecessor_titles.contains(&"Outcome scale v1"));
    assert!(predecessor_titles.contains(&"Impute missing covariates"));

    let merged_candidate =
        status_record(&project_index(&project), "Early complete-case candidate").unwrap();
    assert_eq!(merged_candidate.record.status, RecordStatus::Rejected);
    assert!(!merged_candidate.record.superseded);
    assert!(merged_candidate.successors.is_empty());

    for summary in summaries {
        let folder = match summary.status {
            RecordStatus::Candidate => project.inbox_dir(),
            RecordStatus::Accepted if !summary.superseded => project.ledger_dir(),
            RecordStatus::Accepted | RecordStatus::Rejected => project.archive_dir(),
        };
        assert!(summary.path.starts_with(&folder), "{}", summary.path);
    }
    let merge_body = fs::read_to_string(&consolidation.record.path).unwrap();
    let merge = dadrock::records::supersedes_section(&merge_body)
        .unwrap()
        .unwrap();
    assert_eq!(merge.ids.len(), 2);
    assert!(merge.reason.is_some());

    let report = validate_project_index(&project_index(&project));
    assert!(report.ok, "demo validation issues: {:?}", report.issues);
    assert!(location_warnings(&report).is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn demo_refuses_non_empty_decisions_directory() {
    let root = temp_project("demo-non-empty");
    init_project(&root).unwrap();
    let context = CommandContext::from_path(&root).unwrap();
    let target = camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap();

    let error = demo_project(&context, Some(target))
        .unwrap_err()
        .to_string();
    assert!(error.contains("new or empty target directory"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn penguins_demo_seeds_realistic_python_analysis_without_running_it() {
    let root = temp_project("demo-penguins");
    let context = CommandContext::from_path(&root).unwrap();
    let target = camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap();

    let seeded = demo_project_with_example(&context, DemoExample::Penguins, Some(target)).unwrap();
    assert!(seeded.ok);
    assert_eq!(seeded.data.example, DemoExample::Penguins);
    assert_eq!(seeded.data.records.len(), 0);

    let script = root.join("analysis/penguins_ols.py");
    assert!(script.is_file());
    let source = fs::read_to_string(script).unwrap();
    assert!(source.contains("# /// script"));
    assert!(source.contains("Rdatasets/csv/palmerpenguins/penguins.csv"));
    assert!(source.contains(
        "species_subset = penguins[penguins[\"species\"].isin([\"Adelie\", \"Gentoo\"])]"
    ));
    assert!(source.contains("is_adelie"));
    assert!(source.contains("statsmodels.formula.api as smf"));
    assert!(source.contains("body_mass_g ~ bill_length_mm + flipper_length_mm + is_adelie"));

    let project = discover_from(&root).unwrap();
    assert!(project.initialized);
    assert_eq!(project_index(&project).summaries().len(), 0);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn candidate_review_demo_seeds_focused_candidate_queue() {
    let root = temp_project("demo-candidate-review");
    let context = CommandContext::from_path(&root).unwrap();
    let target = camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap();

    let seeded =
        demo_project_with_example(&context, DemoExample::CandidateReview, Some(target)).unwrap();
    assert!(seeded.ok);
    assert_eq!(seeded.data.example, DemoExample::CandidateReview);

    let project = discover_from(&root).unwrap();
    let index = project_index(&project);
    let summaries = index.summaries();
    let candidate_titles: Vec<_> = summaries
        .iter()
        .filter(|record| record.status == RecordStatus::Candidate)
        .map(|record| record.title.as_str())
        .collect();
    assert_eq!(candidate_titles.len(), 5);
    assert!(candidate_titles.contains(&"Use Adelie and Gentoo only"));
    assert!(candidate_titles.contains(&"Drop rows with missing model variables"));
    assert!(candidate_titles.contains(&"Use body mass as the outcome"));
    assert!(candidate_titles.contains(&"Clarify species dummy coding"));
    assert!(candidate_titles.contains(&"Add island fixed effects"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn broken_demo_seeds_validation_issues() {
    let root = temp_project("demo-broken");
    let context = CommandContext::from_path(&root).unwrap();
    let target = camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap();

    let seeded = demo_project_with_example(&context, DemoExample::Broken, Some(target)).unwrap();
    assert!(seeded.ok);
    assert_eq!(seeded.data.example, DemoExample::Broken);

    let project = discover_from(&root).unwrap();
    let report = validate_project_index(&project_index(&project));
    assert!(!report.ok);
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.message.contains("missing")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn conflicts_demo_seeds_overlapping_candidates() {
    let root = temp_project("demo-conflicts");
    let context = CommandContext::from_path(&root).unwrap();
    let target = camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap();

    let seeded = demo_project_with_example(&context, DemoExample::Conflicts, Some(target)).unwrap();
    assert!(seeded.ok);
    assert_eq!(seeded.data.example, DemoExample::Conflicts);

    let project = discover_from(&root).unwrap();
    let index = project_index(&project);
    let summaries = index.summaries();
    assert!(summaries
        .iter()
        .any(|record| record.status == RecordStatus::Candidate
            && record.title == "Drop missing body-mass rows"));
    assert!(summaries
        .iter()
        .any(|record| record.status == RecordStatus::Candidate
            && record.title == "Use complete cases for body-mass model"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn task_history_demo_seeds_persisted_successful_task_runs() {
    let root = temp_project("demo-task-history");
    let context = CommandContext::from_path(&root).unwrap();
    let target = camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap();

    let seeded =
        demo_project_with_example(&context, DemoExample::TaskHistory, Some(target)).unwrap();
    assert!(seeded.ok);
    assert_eq!(seeded.data.example, DemoExample::TaskHistory);

    let project_root = camino::Utf8PathBuf::from_path_buf(root.clone()).unwrap();
    let runs = list_task_runs(&project_root).unwrap();
    let task_ids: Vec<_> = runs.iter().map(|run| run.task_id.as_str()).collect();
    assert_eq!(runs.len(), 7);
    assert!(task_ids.contains(&"context-to-record"));
    assert!(task_ids.contains(&"code-to-record"));
    assert!(task_ids.contains(&"supersede-record"));
    assert!(task_ids.contains(&"find-conflicts"));
    assert!(task_ids.contains(&"merge-records"));
    assert!(task_ids.contains(&"edit-record"));
    assert!(task_ids.contains(&"custom"));
    assert!(runs
        .iter()
        .all(|run| run.status == dadrock::tasks::TaskRunStatus::Succeeded));

    let code_run = runs
        .iter()
        .find(|run| run.task_id == "code-to-record")
        .unwrap();
    let result = read_task_result(&code_run.run_dir).unwrap();
    assert_eq!(
        result.get("kind").and_then(|kind| kind.as_str()),
        Some("candidate_batch.v1")
    );
    assert!(result
        .get("candidates")
        .and_then(|candidates| candidates.as_array())
        .is_some_and(|candidates| candidates.len() >= 2));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_candidate_writes_canonical_candidate_without_lifecycle_dates() {
    let root = temp_project("new");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();

    let created = create_candidate(
        &project,
        "Drop records with missing treatment",
        &["#kind/sample".to_owned(), "scope/preprocessing".to_owned()],
    )
    .unwrap();

    assert!(created.path.is_file());
    assert_eq!(created.path.parent(), Some(project.inbox_dir().as_path()));
    assert!(created
        .path
        .file_name()
        .unwrap()
        .ends_with("-drop-records-with-missing-treatment.md"));
    let content = fs::read_to_string(&created.path).unwrap();
    assert!(content.contains("status: candidate"));
    assert!(content.contains("kind/sample"));
    assert!(!content.contains("created:"));
    assert!(!content.contains("accepted_at:"));
    assert!(!content.contains("date:"));

    let listed = project_index(&project).list_records(Some(RecordStatus::Candidate));
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, created.id);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_candidate_quotes_yaml_resolved_scalars() {
    let root = temp_project("new-yaml-scalars");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();

    let created = create_candidate(&project, "2025-01-01", &["on".to_owned()]).unwrap();
    let content = fs::read_to_string(&created.path).unwrap();

    assert!(!content.contains("title: 2025-01-01\n"));
    assert!(!content.contains("  - on\n"));
    assert_eq!(
        project_index(&project).list_records(None)[0].title,
        "2025-01-01"
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_candidate_uses_project_candidate_template_body() {
    let root = temp_project("new-template");
    init_project(&root).unwrap();
    fs::write(
        dadr_dir(&root).join("templates/candidate.md"),
        "---\n\
         title: Ignored template title\n\
         status: accepted\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         ---\n\
         ## Context\n\n\
         Project-specific context prompt.\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n\n\
         ## Review Notes\n",
    )
    .unwrap();
    let project = discover_from(&root).unwrap();

    let created = create_candidate(&project, "Templated candidate", &[]).unwrap();
    let content = fs::read_to_string(&created.path).unwrap();
    assert!(content.contains("title: Templated candidate"));
    assert!(content.contains("status: candidate"));
    assert!(content.contains(&created.id.to_string()));
    assert!(content.contains("Project-specific context prompt."));
    assert!(content.contains("## Review Notes"));
    assert!(!content.contains("Ignored template title"));
    assert!(!content.contains("status: accepted"));
    assert!(!content.contains("01938b8e-4d70-7abc-8def-0123456789ab"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scanner_does_not_skip_markdown_files_named_like_tool_dirs() {
    let root = temp_project("record-file-named-templates");
    init_project(&root).unwrap();
    fs::write(
        records_dir(&root).join("templates.md"),
        "---\n\
         title: Templates\n\
         status: accepted\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let listed = project_index(&project).list_records(Some(RecordStatus::Accepted));

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].title, "Templates");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tags_list_merges_yaml_and_inline_tags_case_insensitively() {
    let root = temp_project("tags-list");
    init_project(&root).unwrap();
    fs::write(
        records_dir(&root).join("01938b8e4d70-tagged.md"),
        "---\n\
         title: Tagged decision\n\
         status: accepted\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         tags:\n\
           - Kind/Sample\n\
           - scope/preprocessing\n\
           - emoji-🚀\n\
         ---\n\n\
         ## Context\n\
         Inline tags include #kind/sample, #scope/preprocessing, #日本, and #emoji-🚀.\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let report = build_tag_report(&project_index(&project));
    assert!(report.ok);

    let kind = report
        .tags
        .iter()
        .find(|tag| tag.tag == "kind/sample")
        .unwrap();
    assert_eq!(kind.display, "Kind/Sample");
    assert_eq!(kind.record_count, 1);
    assert_eq!(kind.occurrence_count, 2);

    assert!(report.tags.iter().any(|tag| tag.tag == "日本"));
    assert!(report.tags.iter().any(|tag| tag.tag == "emoji-🚀"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tags_validate_reports_malformed_yaml_and_inline_tags() {
    let root = temp_project("tags-validate");
    init_project(&root).unwrap();
    fs::write(
        records_dir(&root).join("01938b8e4d70-bad-tags.md"),
        "---\n\
         title: Bad tags\n\
         status: accepted\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         tags:\n\
           - \"#kind/sample\"\n\
           - kind//bad\n\
           - \"kind sample\"\n\
           - \"1984\"\n\
         ---\n\n\
         ## Context\n\
         Inline bad tag: #bad//inline.\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let report = build_tag_report(&project_index(&project));
    let codes = tag_issue_codes(&report);
    assert!(!report.ok);
    assert!(codes.contains(&TagIssueCode::LeadingHashInYamlTag));
    assert!(codes.contains(&TagIssueCode::EmptyTagSegment));
    assert!(codes.contains(&TagIssueCode::WhitespaceInTag));
    assert!(codes.contains(&TagIssueCode::NumericOnlyTag));

    let project_validation_report = validate_project_index(&project_index(&project));
    let project_validation_codes = issue_codes(&project_validation_report);
    assert!(project_validation_codes.contains(&ProjectValidationIssueCode::InvalidTag));
    assert!(project_validation_report
        .issues
        .iter()
        .any(|issue| issue.code == ProjectValidationIssueCode::InvalidTag
            && issue.sub_code.as_deref() == Some("leading_hash_in_yaml_tag")));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tags_validate_ignores_numeric_hash_references_in_prose() {
    let root = temp_project("tags-numeric-reference");
    init_project(&root).unwrap();
    fs::write(
        records_dir(&root).join("01938b8e4d70-numeric-reference.md"),
        "---\n\
         title: Numeric reference\n\
         status: accepted\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         tags:\n\
           - kind/model\n\
         ---\n\n\
         ## Context\n\
         Per-coder rankings compare RA #4 vs. Kimi K2.\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let report = build_tag_report(&project_index(&project));

    assert!(report.ok);
    assert!(report.issues.is_empty());
    assert!(!report.tags.iter().any(|tag| tag.tag == "4"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn tags_validate_reports_legacy_yaml_tag_object() {
    let root = temp_project("tags-object");
    init_project(&root).unwrap();
    fs::write(
        records_dir(&root).join("01938b8e4d70-tag-object.md"),
        "---\n\
         title: Tag object\n\
         status: accepted\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         tags:\n\
           kind: sample\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let report = build_tag_report(&project_index(&project));
    assert!(!report.ok);
    assert!(tag_issue_codes(&report).contains(&TagIssueCode::InvalidYamlTagField));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn record_ids_for_tag_accepts_hash_or_plain_filter() {
    let root = temp_project("tags-filter");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let model = create_candidate(&project, "Model candidate", &["#kind/model".to_owned()]).unwrap();
    let sample =
        create_candidate(&project, "Sample candidate", &["kind/sample".to_owned()]).unwrap();

    let model_ids = record_ids_for_tag(&project_index(&project), "#kind/model").unwrap();
    assert!(model_ids.contains(&model.id));
    assert!(!model_ids.contains(&sample.id));

    let sample_ids = record_ids_for_tag(&project_index(&project), "kind/sample").unwrap();
    assert!(sample_ids.contains(&sample.id));
    assert!(!sample_ids.contains(&model.id));

    assert_eq!(normalize_tag_filter("#Kind/Model").unwrap(), "kind/model");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn scanner_skips_tool_managed_directories() {
    let fixture = camino::Utf8Path::new("tests/fixtures/scan-project/decisions");
    let scanned = scan_records(fixture).unwrap();
    let paths: Vec<_> = scanned
        .into_iter()
        .map(|record| record.path.to_string())
        .collect();

    assert!(paths.iter().any(|path| path.contains("candidate.md")));
    assert!(!paths.iter().any(|path| path.contains(".dadr")));
    assert!(!paths.iter().any(|path| path.contains("_site")));
}

#[test]
fn scanner_reports_unreadable_records_without_aborting() {
    let root = temp_project("scan-unreadable");
    init_project(&root).unwrap();
    let bad_path = records_dir(&root).join("01938b8e4d70-bad.md");
    fs::write(&bad_path, [0xff, 0xfe, 0xfd]).unwrap();

    let records = scan_records(camino::Utf8Path::from_path(&records_dir(&root)).unwrap())
        .expect("scan should not abort on unreadable record content");

    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].path,
        camino::Utf8PathBuf::from_path_buf(bad_path).unwrap()
    );
    assert!(records[0].record.is_none());
    assert!(records[0]
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == DiagnosticCode::UnreadableRecord));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn find_resolves_exact_id_and_prefix() {
    let root = temp_project("find-id");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let created = create_candidate(&project, "Country exclusions", &[]).unwrap();

    let exact = project_index(&project).find_records(&created.id.to_string());
    assert_eq!(exact.resolution, FindResolution::One);
    assert_eq!(exact.matches[0].match_kind, MatchKind::ExactId);
    assert_eq!(exact.matches[0].record.id, created.id);

    let prefix = project_index(&project).find_records(&created.id.short_id());
    assert_eq!(prefix.resolution, FindResolution::One);
    assert_eq!(prefix.matches[0].match_kind, MatchKind::IdPrefix);
    assert_eq!(prefix.matches[0].record.id, created.id);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn find_reports_ambiguous_title_or_filename_matches() {
    let root = temp_project("find-many");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    create_candidate(&project, "Missing treatment rule", &[]).unwrap();
    create_candidate(&project, "Missing outcome rule", &[]).unwrap();

    let result = project_index(&project).find_records("missing");
    assert_eq!(result.resolution, FindResolution::Many);
    assert_eq!(result.matches.len(), 2);
    assert!(result
        .matches
        .iter()
        .all(|matched| matched.match_kind == MatchKind::Filename));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn find_reports_none() {
    let root = temp_project("find-none");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();

    let result = project_index(&project).find_records("not present");
    assert_eq!(result.resolution, FindResolution::None);
    assert!(result.matches.is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn status_reports_successors_and_predecessors() {
    let root = temp_project("status");
    init_project(&root).unwrap();
    let decisions = records_dir(&root);

    fs::write(
        decisions.join("01938b8e4d70-original.md"),
        "---\n\
         title: Original decision\n\
         status: accepted\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         successors:\n\
           - 01938b8f-9d71-7456-8000-fedcba987654\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();
    fs::write(
        decisions.join("01938b8f9d71-successor.md"),
        "---\n\
         title: Successor decision\n\
         status: accepted\n\
         id: 01938b8f-9d71-7456-8000-fedcba987654\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let original = status_record(&project_index(&project), "Original").unwrap();
    assert_eq!(original.successors.len(), 1);
    assert_eq!(original.successors[0].title, "Successor decision");
    assert!(original.predecessors.is_empty());

    let successor = status_record(&project_index(&project), "Successor").unwrap();
    assert!(successor.successors.is_empty());
    assert_eq!(successor.predecessors.len(), 1);
    assert_eq!(successor.predecessors[0].title, "Original decision");

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reject_moves_candidate_to_archive_and_rewrites_status() {
    let root = temp_project("reject");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let created = create_candidate(&project, "Rejectable candidate", &[]).unwrap();

    let rejected = reject_candidate(&project_index(&project), &created.id.short_id()).unwrap();
    assert_eq!(rejected.id, created.id);
    assert_eq!(rejected.status, RecordStatus::Rejected);
    assert!(!created.path.exists());
    assert!(rejected.to.is_file());
    assert_eq!(rejected.to.parent(), Some(project.archive_dir().as_path()));

    let content = fs::read_to_string(&rejected.to).unwrap();
    assert!(content.contains("status: rejected"));
    assert!(!content.contains("status: candidate"));
    assert!(!content.contains("created:"));
    assert!(!content.contains("accepted_at:"));

    let rejected_records = project_index(&project).list_records(Some(RecordStatus::Rejected));
    assert_eq!(rejected_records.len(), 1);
    assert_eq!(rejected_records[0].id, created.id);

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn reject_rolls_back_created_target_when_source_remove_fails() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_project("reject-rollback");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let created = create_candidate(&project, "Rollback candidate", &[]).unwrap();
    let target = archive_dir(&root).join(created.path.file_name().unwrap());
    let inbox = inbox_dir(&root);
    let original_permissions = fs::metadata(&inbox).unwrap().permissions();

    fs::set_permissions(&inbox, fs::Permissions::from_mode(0o500)).unwrap();
    let error = reject_candidate(&project_index(&project), &created.id.short_id())
        .expect_err("reject should fail when the source cannot be removed");
    fs::set_permissions(&inbox, original_permissions).unwrap();

    assert!(error.to_string().contains("I/O error"));
    assert!(created.path.exists());
    assert!(!target.exists());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reject_refuses_non_candidate() {
    let root = temp_project("reject-non-candidate");
    init_project(&root).unwrap();
    let decisions = records_dir(&root);
    fs::write(
        decisions.join("01938b8e4d70-accepted.md"),
        "---\n\
         title: Accepted decision\n\
         status: accepted\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let error = reject_candidate(&project_index(&project), "Accepted decision").unwrap_err();
    assert!(error
        .to_string()
        .contains("reject requires status `candidate`"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn save_record_source_rewrites_source_through_command() {
    let root = temp_project("save-source");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let created = create_candidate(&project, "Original title", &[]).unwrap();
    let context = CommandContext::from_path(&root).unwrap();
    let edited = format!(
        "---\n\
         title: Updated title\n\
         status: candidate\n\
         id: {}\n\
         ---\n\n\
         ## Context\n\n\
         Updated context.\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
        created.id
    );

    let envelope = save_record_source(
        &context,
        SaveRecordSourceRequest {
            query: created.id.short_id(),
            content: edited.clone(),
        },
    )
    .unwrap();

    assert_eq!(envelope.data.id, created.id);
    assert_eq!(envelope.changed_paths, vec![created.path.clone()]);
    assert_eq!(fs::read_to_string(&created.path).unwrap(), edited);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn save_record_source_rejects_id_changes_without_overwriting_file() {
    let root = temp_project("save-source-id");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let created = create_candidate(&project, "Stable id", &[]).unwrap();
    let original = fs::read_to_string(&created.path).unwrap();
    let context = CommandContext::from_path(&root).unwrap();
    let edited = "\
        ---\n\
        title: Stable id\n\
        status: candidate\n\
        id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
        ---\n\n\
        ## Context\n\n\
        ## Decision\n\n\
        ## Alternatives\n\n\
        ## Consequences\n"
        .to_owned();

    let error = save_record_source(
        &context,
        SaveRecordSourceRequest {
            query: created.id.short_id(),
            content: edited,
        },
    )
    .unwrap_err();

    assert!(error.to_string().contains("record id cannot be changed"));
    assert_eq!(fs::read_to_string(&created.path).unwrap(), original);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn save_record_source_rejects_status_changes_without_overwriting_file() {
    let root = temp_project("save-source-status");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let created = create_candidate(&project, "Stable status", &[]).unwrap();
    let original = fs::read_to_string(&created.path).unwrap();
    let context = CommandContext::from_path(&root).unwrap();
    let edited = format!(
        "---\n\
         title: Stable status\n\
         status: accepted\n\
         id: {}\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
        created.id
    );

    let error = save_record_source(
        &context,
        SaveRecordSourceRequest {
            query: created.id.short_id(),
            content: edited,
        },
    )
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("record status cannot be changed"));
    assert_eq!(fs::read_to_string(&created.path).unwrap(), original);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn save_record_source_refuses_to_drop_required_sections_of_accepted_record() {
    let root = temp_project("save-source-accepted-sections");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let candidate =
        create_candidate_with_body(&project, "Complete accepted", &[], FILLED_SECTIONS).unwrap();
    let accepted = accept_candidate(&project_index(&project), &candidate.id.to_string())
        .unwrap()
        .accepted
        .remove(0);
    let original = fs::read_to_string(&accepted.to).unwrap();
    let context = CommandContext::from_path(&root).unwrap();
    let edited = format!(
        "---\n\
         title: Complete accepted\n\
         status: accepted\n\
         id: {}\n\
         ---\n\n\
         ## Context\n\n\
         Missing the required decision sections.\n",
        accepted.id
    );

    let error = save_record_source(
        &context,
        SaveRecordSourceRequest {
            query: accepted.id.short_id(),
            content: edited,
        },
    )
    .unwrap_err();

    assert!(
        error
            .to_string()
            .contains("keeps each standard section exactly once and with text: missing Decision, Alternatives, Consequences"),
        "{error}"
    );
    assert!(error.to_string().contains("draft a successor"), "{error}");
    assert_eq!(fs::read_to_string(&accepted.to).unwrap(), original);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn reject_record_reports_removed_candidate_and_created_rejected_paths() {
    let root = temp_project("reject-command-changed-paths");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let candidate = create_candidate(&project, "Reject through command", &[]).unwrap();
    let context = CommandContext::from_path(&root).unwrap();

    let envelope = reject_record(&context, &candidate.id.to_string()).unwrap();

    assert_eq!(
        envelope.changed_paths,
        vec![candidate.path, envelope.data.to]
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn accept_converts_candidate_into_accepted_decision() {
    let root = temp_project("accept");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let candidate = create_candidate_with_body(
        &project,
        "Acceptable candidate",
        &["#kind/model".to_owned()],
        FILLED_SECTIONS,
    )
    .unwrap();

    let result = accept_candidate(&project_index(&project), &candidate.id.short_id()).unwrap();
    assert!(result.superseded.is_empty());
    let [accepted] = result.accepted.as_slice() else {
        panic!("expected one accepted record: {result:?}");
    };
    assert_eq!(accepted.id, candidate.id);
    assert!(!candidate.path.exists());
    assert_eq!(accepted.from, candidate.path);
    assert!(accepted.to.is_file());
    assert_eq!(accepted.to.parent(), Some(project.ledger_dir().as_path()));

    let accepted_content = fs::read_to_string(&accepted.to).unwrap();
    assert!(accepted_content.contains("status: accepted"));
    assert!(accepted_content.contains(&candidate.id.to_string()));
    assert!(!accepted_content.contains("successors:"));
    assert!(!accepted_content.contains("date:"));
    assert!(!accepted_content.contains("created:"));
    assert!(!accepted_content.contains("accepted_at:"));

    let accepted_records = project_index(&project).list_records(Some(RecordStatus::Accepted));
    assert_eq!(accepted_records.len(), 1);

    let decision_status =
        status_record(&project_index(&project), &candidate.id.short_id()).unwrap();
    assert_eq!(decision_status.record.status, RecordStatus::Accepted);
    assert!(decision_status.successors.is_empty());
    assert!(decision_status.predecessors.is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn accept_refuses_non_candidate() {
    let root = temp_project("accept-non-candidate");
    init_project(&root).unwrap();
    let decisions = records_dir(&root);
    fs::write(
        decisions.join("01938b8e4d70-accepted.md"),
        "---\n\
         title: Accepted decision\n\
         status: accepted\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let error = accept_candidate(&project_index(&project), "Accepted decision").unwrap_err();
    assert!(error
        .to_string()
        .contains("accept requires status `candidate`"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn accept_requires_required_sections() {
    let root = temp_project("accept-sections");
    init_project(&root).unwrap();
    let candidate_path = inbox_dir(&root).join("01938b8e4d70-incomplete.md");
    fs::write(
        &candidate_path,
        "---\n\
         title: Incomplete candidate\n\
         status: candidate\n\
         id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
         ---\n\n\
         ## Context\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let error = accept_candidate(&project_index(&project), "Incomplete").unwrap_err();
    assert!(error
        .to_string()
        .contains("missing Decision, Alternatives, Consequences"));
    assert!(candidate_path.exists());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn accept_requires_non_empty_sections() {
    let root = temp_project("accept-empty-sections");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let candidate = create_candidate(&project, "Template only", &[]).unwrap();

    let error = accept_candidate(&project_index(&project), &candidate.id.to_string()).unwrap_err();
    assert!(error
        .to_string()
        .contains("empty Context, Decision, Alternatives, Consequences"));
    assert!(candidate.path.exists());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supersede_links_accepted_records() {
    let root = temp_project("supersede");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70",
        "Old decision",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8f9d71",
        "New decision",
        "accepted",
        "01938b8f-9d71-7456-8000-fedcba987654",
    );

    let project = discover_from(&root).unwrap();
    let result =
        supersede_record(&project_index(&project), "Old decision", "New decision").unwrap();
    assert_eq!(result.status, RecordStatus::Accepted);

    let old_status = status_record(&project_index(&project), "Old decision").unwrap();
    assert_eq!(old_status.record.status, RecordStatus::Accepted);
    assert!(old_status.record.superseded);
    assert_eq!(old_status.successors.len(), 1);
    assert_eq!(old_status.successors[0].title, "New decision");

    let new_status = status_record(&project_index(&project), "New decision").unwrap();
    assert_eq!(new_status.predecessors.len(), 1);
    assert_eq!(new_status.predecessors[0].title, "Old decision");

    let old_content = fs::read_to_string(result.old_path).unwrap();
    assert!(old_content.contains("status: accepted"));
    assert!(old_content.contains("01938b8f-9d71-7456-8000-fedcba987654"));
    assert!(!old_content.contains("date:"));
    assert!(!old_content.contains("created:"));
    assert!(!old_content.contains("accepted_at:"));
    assert!(!old_content.contains("status: superseded"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supersede_supports_many_to_one_successors() {
    let root = temp_project("supersede-many-one");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70",
        "Old A",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8f9d71",
        "Old B",
        "accepted",
        "01938b8f-9d71-7456-8000-fedcba987654",
    );
    write_record(
        &root,
        "01938b91bcd0",
        "New X",
        "accepted",
        "01938b91-bcd0-7e00-8b00-cafebabe0002",
    );

    let project = discover_from(&root).unwrap();
    supersede_record(&project_index(&project), "Old A", "New X").unwrap();
    supersede_record(&project_index(&project), "Old B", "New X").unwrap();

    let new_status = status_record(&project_index(&project), "New X").unwrap();
    let predecessor_titles: Vec<_> = new_status
        .predecessors
        .iter()
        .map(|record| record.title.as_str())
        .collect();
    assert_eq!(predecessor_titles.len(), 2);
    assert!(predecessor_titles.contains(&"Old A"));
    assert!(predecessor_titles.contains(&"Old B"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supersede_supports_one_to_many_successors() {
    let root = temp_project("supersede-one-many");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70",
        "Old A",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8f9d71",
        "New X",
        "accepted",
        "01938b8f-9d71-7456-8000-fedcba987654",
    );
    write_record(
        &root,
        "01938b91bcd0",
        "New Y",
        "accepted",
        "01938b91-bcd0-7e00-8b00-cafebabe0002",
    );

    let project = discover_from(&root).unwrap();
    supersede_record(&project_index(&project), "Old A", "New X").unwrap();
    supersede_record(&project_index(&project), "Old A", "New Y").unwrap();

    let old_status = status_record(&project_index(&project), "Old A").unwrap();
    let successor_titles: Vec<_> = old_status
        .successors
        .iter()
        .map(|record| record.title.as_str())
        .collect();
    assert_eq!(old_status.record.status, RecordStatus::Accepted);
    assert!(old_status.record.superseded);
    assert_eq!(successor_titles.len(), 2);
    assert!(successor_titles.contains(&"New X"));
    assert!(successor_titles.contains(&"New Y"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supersede_supports_many_to_many_successors() {
    let root = temp_project("supersede-many-many");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70",
        "Old A",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8f9d71",
        "Old B",
        "accepted",
        "01938b8f-9d71-7456-8000-fedcba987654",
    );
    write_record(
        &root,
        "01938b91bcd0",
        "New X",
        "accepted",
        "01938b91-bcd0-7e00-8b00-cafebabe0002",
    );
    write_record(
        &root,
        "01938b92bcd0",
        "New Y",
        "accepted",
        "01938b92-bcd0-7e00-8b00-cafebabe0003",
    );

    let project = discover_from(&root).unwrap();
    supersede_record(&project_index(&project), "Old A", "New X").unwrap();
    supersede_record(&project_index(&project), "Old A", "New Y").unwrap();
    supersede_record(&project_index(&project), "Old B", "New X").unwrap();
    supersede_record(&project_index(&project), "Old B", "New Y").unwrap();

    assert_eq!(
        status_record(&project_index(&project), "Old A")
            .unwrap()
            .successors
            .len(),
        2
    );
    assert_eq!(
        status_record(&project_index(&project), "Old B")
            .unwrap()
            .successors
            .len(),
        2
    );
    assert_eq!(
        status_record(&project_index(&project), "New X")
            .unwrap()
            .predecessors
            .len(),
        2
    );
    assert_eq!(
        status_record(&project_index(&project), "New Y")
            .unwrap()
            .predecessors
            .len(),
        2
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supersede_records_supports_many_to_many_in_one_operation() {
    let root = temp_project("supersede-bulk-many-many");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70",
        "Old A",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8f9d71",
        "Old B",
        "accepted",
        "01938b8f-9d71-7456-8000-fedcba987654",
    );
    write_record(
        &root,
        "01938b91bcd0",
        "New X",
        "accepted",
        "01938b91-bcd0-7e00-8b00-cafebabe0002",
    );
    write_record(
        &root,
        "01938b92bcd0",
        "New Y",
        "accepted",
        "01938b92-bcd0-7e00-8b00-cafebabe0003",
    );

    let project = discover_from(&root).unwrap();
    let result = supersede_records(
        &project_index(&project),
        &["Old A".to_owned(), "Old B".to_owned()],
        &["New X".to_owned(), "New Y".to_owned()],
    )
    .unwrap();

    assert_eq!(result.from.len(), 2);
    assert_eq!(result.to.len(), 2);
    assert_eq!(result.edges.len(), 4);
    assert_eq!(
        status_record(&project_index(&project), "Old A")
            .unwrap()
            .successors
            .len(),
        2
    );
    assert_eq!(
        status_record(&project_index(&project), "Old B")
            .unwrap()
            .successors
            .len(),
        2
    );

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn supersede_records_restores_every_record_when_a_move_fails() {
    use std::os::unix::fs::PermissionsExt;

    let root = temp_project("supersede-staged-failure");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70",
        "Old A",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    let nested_dir = records_dir(&root).join("nested");
    fs::create_dir_all(&nested_dir).unwrap();
    let old_b_path = nested_dir.join("01938b8f9d71-old-b.md");
    fs::write(
        &old_b_path,
        "---\n\
         title: Old B\n\
         status: accepted\n\
         id: 01938b8f-9d71-7456-8000-fedcba987654\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();
    write_record(
        &root,
        "01938b91bcd0",
        "New X",
        "accepted",
        "01938b91-bcd0-7e00-8b00-cafebabe0002",
    );
    let old_a_path = records_dir(&root).join("01938b8e4d70-old-a.md");
    let old_a_original = fs::read_to_string(&old_a_path).unwrap();
    let old_b_original = fs::read_to_string(&old_b_path).unwrap();
    let original_permissions = fs::metadata(&nested_dir).unwrap().permissions();
    fs::set_permissions(&nested_dir, fs::Permissions::from_mode(0o500)).unwrap();

    let project = discover_from(&root).unwrap();
    let error = supersede_records(
        &project_index(&project),
        &["Old A".to_owned(), "Old B".to_owned()],
        &["New X".to_owned()],
    )
    .unwrap_err();
    fs::set_permissions(&nested_dir, original_permissions).unwrap();

    assert!(error.to_string().contains("I/O error"));
    assert_eq!(fs::read_to_string(&old_a_path).unwrap(), old_a_original);
    assert_eq!(fs::read_to_string(&old_b_path).unwrap(), old_b_original);
    assert!(md_files(&archive_dir(&root)).is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supersede_refuses_invalid_statuses_duplicate_and_cycle() {
    let root = temp_project("supersede-refuse");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70",
        "Old accepted",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8f9d71",
        "New accepted",
        "accepted",
        "01938b8f-9d71-7456-8000-fedcba987654",
    );
    let project = discover_from(&root).unwrap();
    let candidate = create_candidate(&project, "Candidate record", &[]).unwrap();

    let old_candidate_error = supersede_record(
        &project_index(&project),
        &candidate.id.to_string(),
        "New accepted",
    )
    .unwrap_err()
    .to_string();
    assert!(old_candidate_error.contains("old record status"));

    let new_candidate_error = supersede_record(
        &project_index(&project),
        "Old accepted",
        &candidate.id.to_string(),
    )
    .unwrap_err()
    .to_string();
    assert!(new_candidate_error.contains("new record status"));

    supersede_record(&project_index(&project), "Old accepted", "New accepted").unwrap();
    let duplicate_error =
        supersede_record(&project_index(&project), "Old accepted", "New accepted")
            .unwrap_err()
            .to_string();
    assert!(duplicate_error.contains("already lists"));

    fs::remove_dir_all(root).unwrap();

    let root = temp_project("supersede-cycle");
    init_project(&root).unwrap();
    write_record_with_successors(
        &root,
        "01938b8e4d70",
        "Cycle A",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
        &[],
    );
    write_record_with_successors(
        &root,
        "01938b8f9d71",
        "Cycle B",
        "accepted",
        "01938b8f-9d71-7456-8000-fedcba987654",
        &["01938b8e-4d70-7abc-8def-0123456789ab"],
    );
    let project = discover_from(&root).unwrap();
    let cycle_error = supersede_record(&project_index(&project), "Cycle A", "Cycle B")
        .unwrap_err()
        .to_string();
    assert!(cycle_error.contains("would create a cycle"));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_validation_passes_clean_accepted_workflow() {
    let root = temp_project("validation-clean");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let candidate =
        create_candidate_with_body(&project, "Documented decision", &[], FILLED_SECTIONS).unwrap();
    accept_candidate(&project_index(&project), &candidate.id.to_string()).unwrap();

    let report = validate_project_index(&project_index(&project));
    assert!(report.ok);
    assert!(report.issues.is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_validation_reports_project_wide_graph_and_filename_errors() {
    let root = temp_project("validation-graph");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70",
        "Duplicate A",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8e4d70",
        "Duplicate B",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8e4d70",
        "Same short id",
        "accepted",
        "01938b8e-4d70-7abc-8def-0000000000ff",
    );
    write_record_with_successors(
        &root,
        "01938b8f9d71",
        "Dangling successor",
        "accepted",
        "01938b8f-9d71-7456-8000-fedcba987654",
        &["01938b90-9d71-7456-8000-fedcba987654"],
    );
    write_record_with_successors(
        &root,
        "01938b91bcd0",
        "Cycle A",
        "accepted",
        "01938b91-bcd0-7e00-8b00-cafebabe0002",
        &["01938b92-bcd0-7e00-8b00-cafebabe0003"],
    );
    write_record_with_successors(
        &root,
        "01938b92bcd0",
        "Cycle B",
        "accepted",
        "01938b92-bcd0-7e00-8b00-cafebabe0003",
        &["01938b91-bcd0-7e00-8b00-cafebabe0002"],
    );
    fs::write(
        inbox_dir(&root).join("01938b93bcd0-candidate-with-successor.md"),
        "---\n\
         title: Candidate with successor\n\
         status: candidate\n\
         id: 01938b93-bcd0-7e00-8b00-cafebabe0004\n\
         successors:\n\
           - 01938b90-4d70-7abc-8def-0123456789ab\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();
    let project = discover_from(&root).unwrap();
    let report = validate_project_index(&project_index(&project));
    let codes = issue_codes(&report);

    assert!(!report.ok);
    assert!(codes.contains(&ProjectValidationIssueCode::DuplicateRecordId));
    assert!(codes.contains(&ProjectValidationIssueCode::ShortIdCollision));
    assert!(codes.contains(&ProjectValidationIssueCode::UnresolvedSuccessor));
    assert!(codes.contains(&ProjectValidationIssueCode::InvalidSuccessor));
    assert!(codes.contains(&ProjectValidationIssueCode::SuccessorCycle));

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn project_validation_reports_structural_and_location_issues() {
    let root = temp_project("validation-structure");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70",
        "Root candidate",
        "candidate",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8f9d71",
        "Root rejected",
        "rejected",
        "01938b8f-9d71-7456-8000-fedcba987654",
    );

    fs::write(
        inbox_dir(&root).join("01938b91bcd0-active-accepted.md"),
        "---\n\
         title: Active accepted\n\
         status: accepted\n\
         id: 01938b91-bcd0-7e00-8b00-cafebabe0002\n\
         ---\n\n\
         ## Context\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    )
    .unwrap();
    fs::write(
        records_dir(&root).join("01938b92bcd0-incomplete-accepted.md"),
        "---\n\
         title: Incomplete accepted\n\
         status: accepted\n\
         id: 01938b92-bcd0-7e00-8b00-cafebabe0003\n\
         ---\n\n\
         ## Context\n",
    )
    .unwrap();

    let project = discover_from(&root).unwrap();
    let report = validate_project_index(&project_index(&project));
    let codes = issue_codes(&report);

    assert!(!report.ok);
    assert!(codes.contains(&ProjectValidationIssueCode::MissingRequiredSection));
    assert!(codes.contains(&ProjectValidationIssueCode::StatusLocationMismatch));
    assert_eq!(
        report
            .issues
            .iter()
            .filter(|issue| issue.code == ProjectValidationIssueCode::StatusLocationMismatch)
            .count(),
        3
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn validate_links_passes_resolved_wikilinks_and_local_markdown_links() {
    let root = temp_project("links-clean");
    init_project(&root).unwrap();
    fs::create_dir_all(root.join("data")).unwrap();
    fs::write(root.join("data/input.csv"), "x\n1\n").unwrap();
    write_record(
        &root,
        "01938b8f9d71",
        "Target decision",
        "accepted",
        "01938b8f-9d71-7456-8000-fedcba987654",
    );
    write_record_with_body(
        &root,
        "01938b8e4d70",
        "Source decision",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
        "## Context\n\
         See [[01938b8f9d71|target decision]], [[01938b8f-9d71-7456-8000-fedcba987654]], \
         and [data](../../data/input.csv). Notes like [[Reading list]] and [[cafe]] are \
         not record links.\n\n\
         ## Decision\n\n\
         ## Alternatives\n\n\
         ## Consequences\n",
    );

    let project = discover_from(&root).unwrap();
    let report = check_project_links(&project_index(&project));
    assert!(report.ok);
    assert!(report.issues.is_empty());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn new_candidates_get_unique_short_ids() {
    let root = temp_project("unique-short-ids");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();

    let mut short_ids = std::collections::BTreeSet::new();
    for index in 0..5 {
        let created = create_candidate(&project, &format!("Candidate {index}"), &[]).unwrap();
        assert!(short_ids.insert(created.id.short_id()));
        assert!(created
            .path
            .file_name()
            .unwrap()
            .starts_with(&format!("{}-", created.id.short_id())));
    }

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn validate_links_reports_wikilink_and_markdown_link_issues() {
    let root = temp_project("links-issues");
    init_project(&root).unwrap();
    write_record(
        &root,
        "01938b8e4d70-a",
        "Ambiguous A",
        "accepted",
        "01938b8e-4d70-7abc-8def-0123456789ab",
    );
    write_record(
        &root,
        "01938b8e4d70-b",
        "Ambiguous B",
        "accepted",
        "01938b8e-4d70-7def-8000-0123456789ac",
    );
    let project = discover_from(&root).unwrap();
    let candidate = create_candidate(&project, "Rejected candidate target", &[]).unwrap();
    reject_candidate(&project_index(&project), &candidate.id.to_string()).unwrap();

    write_record_with_body(
        &root,
        "01938b92bcd0",
        "Source with bad links",
        "accepted",
        "01938b92-bcd0-7e00-8b00-cafebabe0003",
        &format!(
            "## Context\n\
             Broken [[01938b90abcd]], ambiguous [[01938b8e4d70]], rejected [[{}|draft]], and [missing](../data/missing.csv).\n\n\
             ## Decision\n\n\
             ## Alternatives\n\n\
             ## Consequences\n",
            candidate.id.short_id()
        ),
    );

    let report = check_project_links(&project_index(&project));
    let codes = link_issue_codes(&report);
    assert!(!report.ok);
    assert!(codes.contains(&LinkIssueCode::BrokenWikilink));
    assert!(codes.contains(&LinkIssueCode::AmbiguousWikilink));
    assert!(codes.contains(&LinkIssueCode::RejectedRecordWikilink));
    assert!(codes.contains(&LinkIssueCode::BrokenMarkdownLink));

    let project_validation_report = validate_project_index(&project_index(&project));
    let project_validation_codes = issue_codes(&project_validation_report);
    assert!(project_validation_codes.contains(&ProjectValidationIssueCode::BrokenWikilink));
    assert!(project_validation_codes.contains(&ProjectValidationIssueCode::AmbiguousWikilink));
    assert!(project_validation_codes.contains(&ProjectValidationIssueCode::RejectedRecordWikilink));
    assert!(project_validation_codes.contains(&ProjectValidationIssueCode::BrokenMarkdownLink));
    assert!(project_validation_report
        .issues
        .iter()
        .any(
            |issue| issue.code == ProjectValidationIssueCode::BrokenMarkdownLink
                && issue.sub_code.as_deref() == Some("broken_markdown_link")
        ));

    fs::remove_dir_all(root).unwrap();
}

const SUPERSEDES_A: &str = "0193fb1f-a3a0-7000-8000-000000000001";
const SUPERSEDES_B: &str = "0193fb1f-a3a0-7000-8000-000000000002";

fn accepted_record(project: &ProjectContext, title: &str) -> dadrock::records::RecordMove {
    let candidate = create_candidate_with_body(project, title, &[], FILLED_SECTIONS).unwrap();
    accept_candidate(&project_index(project), &candidate.id.to_string())
        .unwrap()
        .accepted
        .remove(0)
}

fn successor_body(predecessors: &[RecordId]) -> String {
    let links: String = predecessors
        .iter()
        .map(|id| format!("[[{id}|Previous decision]]\n"))
        .collect();
    format!("## Supersedes\n\n{links}\nThe old rule no longer fits.\n\n{FILLED_SECTIONS}")
}

fn location_warnings(report: &dadrock::validation::ProjectValidationReport) -> Vec<String> {
    report
        .issues
        .iter()
        .filter(|issue| issue.code == ProjectValidationIssueCode::StatusLocationMismatch)
        .map(|issue| issue.message.clone())
        .collect()
}

#[test]
fn supersedes_acceptance_links_moves_and_preserves_predecessor_body() {
    let root = temp_project("supersedes-acceptance");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let old = accepted_record(&project, "Previous choice");
    assert_eq!(old.to.parent(), Some(project.ledger_dir().as_path()));
    let original = fs::read_to_string(&old.to).unwrap();

    let candidate =
        create_candidate_with_body(&project, "New choice", &[], &successor_body(&[old.id]))
            .unwrap();
    let index = project_index(&project);
    let pending = dadrock::records::pending_supersedes(&index, &candidate.id.to_string()).unwrap();
    assert_eq!(
        pending
            .predecessors
            .iter()
            .map(|record| record.id)
            .collect::<Vec<_>>(),
        vec![old.id]
    );
    assert!(pending.stale.is_empty());

    let accepted = accept_candidate(&index, &candidate.id.to_string()).unwrap();
    let archived = project.archive_dir().join(old.to.file_name().unwrap());
    assert_eq!(
        accepted.accepted[0].to.parent(),
        Some(project.ledger_dir().as_path())
    );
    assert!(!candidate.path.exists());
    assert!(!old.to.exists());
    assert_eq!(
        accepted.superseded,
        vec![dadrock::records::SupersededRecordMove {
            id: old.id,
            from: old.to.clone(),
            to: archived.clone(),
            successors: vec![candidate.id],
        }]
    );

    let index = project_index(&project);
    let previous = status_record(&index, &old.id.to_string()).unwrap();
    assert_eq!(previous.record.path, archived);
    assert!(previous.record.superseded);
    assert_eq!(previous.successors[0].id, candidate.id);
    let updated = fs::read_to_string(&archived).unwrap();
    assert_eq!(
        original.splitn(3, "---").nth(2),
        updated.splitn(3, "---").nth(2)
    );
    assert!(
        dadrock::records::pending_supersedes(&index, &candidate.id.to_string())
            .unwrap()
            .predecessors
            .is_empty()
    );
    let report = validate_project_index(&index);
    assert!(report.ok, "{:?}", report.issues);
    assert!(location_warnings(&report).is_empty());

    // A later candidate that names the archived predecessor is refused.
    let stale =
        create_candidate_with_body(&project, "Stale choice", &[], &successor_body(&[old.id]))
            .unwrap();
    let error = accept_candidate(&project_index(&project), &stale.id.to_string()).unwrap_err();
    assert!(error.to_string().contains("already superseded"), "{error}");
    assert!(stale.path.exists());

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn accept_record_command_reports_both_moves() {
    let root = temp_project("supersedes-command");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let old = accepted_record(&project, "Previous choice");
    let candidate =
        create_candidate_with_body(&project, "New choice", &[], &successor_body(&[old.id]))
            .unwrap();

    let context = CommandContext::from_path(&root).unwrap();
    let envelope = dadrock::commands::accept_record(&context, &candidate.id.to_string()).unwrap();
    let archived = project.archive_dir().join(old.to.file_name().unwrap());
    assert_eq!(
        envelope.changed_paths,
        vec![
            candidate.path.clone(),
            envelope.data.accepted[0].to.clone(),
            old.to,
            archived
        ]
    );

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_supersedes_does_not_accept_or_modify_either_record() {
    let root = temp_project("supersedes-invalid");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let old = create_candidate(&project, "Still a candidate", &[]).unwrap();
    let candidate =
        create_candidate_with_body(&project, "Successor", &[], &successor_body(&[old.id])).unwrap();
    let original = fs::read_to_string(&old.path).unwrap();
    let proposed = fs::read_to_string(&candidate.path).unwrap();
    assert!(accept_candidate(&project_index(&project), &candidate.id.to_string()).is_err());
    assert_eq!(fs::read_to_string(&old.path).unwrap(), original);
    assert_eq!(fs::read_to_string(&candidate.path).unwrap(), proposed);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn supersedes_section_reads_one_link_without_a_reason() {
    use dadrock::records::{supersedes_section, Supersedes};
    let body = format!("## Supersedes\n\n[[{SUPERSEDES_A}|Old rule]]\n\n## Context\nWhy.\n");
    assert_eq!(
        supersedes_section(&body).unwrap(),
        Some(Supersedes {
            ids: vec![RecordId::parse(SUPERSEDES_A).unwrap()],
            reason: None,
        })
    );
}

#[test]
fn supersedes_section_reads_several_links_with_optional_labels() {
    use dadrock::records::supersedes_section;
    let body = format!("## Supersedes\n[[{SUPERSEDES_A}]]\n  [[{SUPERSEDES_B}|Second rule]]\n");
    let section = supersedes_section(&body).unwrap().unwrap();
    assert_eq!(
        section.ids,
        vec![
            RecordId::parse(SUPERSEDES_A).unwrap(),
            RecordId::parse(SUPERSEDES_B).unwrap()
        ]
    );
    assert_eq!(section.reason, None);
}

#[test]
fn supersedes_section_keeps_the_reason_markdown() {
    use dadrock::records::supersedes_section;
    let body = format!(
        "Intro.\n\n## Supersedes\r\n\r\n[[{SUPERSEDES_A}|Old rule]]\r\n\r\n\
         The two rules **disagreed**:\r\n\r\n- at p99\r\n\r\n```text\r\n## Not a heading\r\n```\r\n\r\n\r\n\
         ## Context\r\nWhy.\r\n"
    );
    let section = supersedes_section(&body).unwrap().unwrap();
    assert_eq!(section.ids, vec![RecordId::parse(SUPERSEDES_A).unwrap()]);
    assert_eq!(
        section.reason.as_deref(),
        Some("The two rules **disagreed**:\n\n- at p99\n\n```text\n## Not a heading\n```")
    );

    // A link after the reason starts is part of the reason.
    let body = format!("## Supersedes\n[[{SUPERSEDES_A}]]\nBecause.\n[[{SUPERSEDES_B}]]\n");
    let section = supersedes_section(&body).unwrap().unwrap();
    assert_eq!(section.ids.len(), 1);
    assert_eq!(
        section.reason.as_deref(),
        Some(format!("Because.\n[[{SUPERSEDES_B}]]").as_str())
    );
}

#[test]
fn supersedes_section_requires_a_full_id_link_first() {
    use dadrock::records::supersedes_section;
    assert!(supersedes_section("## Supersedes\n").is_err());
    assert!(supersedes_section("## Supersedes\n\nOnly a reason.\n").is_err());
    assert!(supersedes_section(&format!("## Supersedes\n- [[{SUPERSEDES_A}]]\n")).is_err());
    let short = supersedes_section("## Supersedes\n[[0193fb1fa3a0]]\n").unwrap_err();
    assert!(short.to_string().contains("full id"), "{short}");
    assert!(supersedes_section(&format!(
        "## Supersedes\n[[{SUPERSEDES_A}]]\n[[{SUPERSEDES_A}|Again]]\n"
    ))
    .is_err());
}

#[test]
fn supersedes_section_allows_at_most_one_section() {
    use dadrock::records::supersedes_section;
    let error = supersedes_section(&format!(
        "## Supersedes\n[[{SUPERSEDES_A}]]\n## Context\nWhy.\n## Supersedes\n[[{SUPERSEDES_B}]]\n"
    ))
    .unwrap_err();
    assert!(error.to_string().contains("only one"), "{error}");
}

#[test]
fn supersedes_examples_in_code_are_not_instructions() {
    use dadrock::records::supersedes_section;
    assert!(
        supersedes_section(&format!("## Context\nSee [[{SUPERSEDES_A}]]"))
            .unwrap()
            .is_none()
    );
    assert!(supersedes_section(&format!(
        "```markdown\n## Supersedes\n[[{SUPERSEDES_A}]]\n```"
    ))
    .unwrap()
    .is_none());
    let nested = format!("````markdown\n```\n## Supersedes\n[[{SUPERSEDES_A}]]\n```\n````");
    assert!(supersedes_section(&nested).unwrap().is_none());
    assert!(
        supersedes_section(&format!("    ## Supersedes\n    [[{SUPERSEDES_A}]]"))
            .unwrap()
            .is_none()
    );
    // A fenced link inside the section is not a link line.
    assert!(supersedes_section(&format!("## Supersedes\n```\n[[{SUPERSEDES_A}]]\n```\n")).is_err());
}

#[test]
fn body_without_supersedes_removes_only_the_section() {
    use dadrock::records::body_without_supersedes;
    let rest = "## Context\nWhy.\n\n## Decision\nWhat.\n";
    let first = format!("## Supersedes\n\n[[{SUPERSEDES_A}|Old]]\n\nBecause.\n\n{rest}");
    assert_eq!(body_without_supersedes(&first), rest);

    let last = format!("{rest}\n## Supersedes\n\n[[{SUPERSEDES_A}]]\n\n");
    assert_eq!(body_without_supersedes(&last), rest);

    assert_eq!(body_without_supersedes(rest), rest);
    let fenced = format!("{rest}```\n## Supersedes\n[[{SUPERSEDES_A}]]\n```\n");
    assert_eq!(body_without_supersedes(&fenced), fenced);
}

#[test]
fn with_supersedes_section_puts_the_section_first() {
    use dadrock::records::{body_without_supersedes, supersedes_section, with_supersedes_section};
    let id = RecordId::parse(SUPERSEDES_A).unwrap();
    let body = with_supersedes_section("\n## Context\nWhy.\n", &[(id, "Odd [[title]]")], None);
    assert_eq!(
        body,
        format!("## Supersedes\n\n[[{SUPERSEDES_A}|Odd [title]]]\n\n## Context\nWhy.\n")
    );
    assert_eq!(supersedes_section(&body).unwrap().unwrap().ids, vec![id]);
    assert_eq!(body_without_supersedes(&body), "## Context\nWhy.\n");
}

#[test]
fn supersede_command_moves_predecessors_to_archive() {
    let root = temp_project("supersede-moves");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let old = accepted_record(&project, "Old rule");
    let new = accepted_record(&project, "New rule");
    let newer = accepted_record(&project, "Newer rule");

    let context = CommandContext::from_path(&root).unwrap();
    let envelope = dadrock::commands::supersede_records(
        &context,
        dadrock::commands::SupersedeRecordsRequest {
            from: vec![old.id.to_string()],
            to: vec![new.id.to_string()],
        },
    )
    .unwrap();
    let archived = project.archive_dir().join(old.to.file_name().unwrap());
    assert!(!old.to.exists());
    assert!(archived.is_file());
    assert!(new.to.is_file());
    assert_eq!(
        envelope.changed_paths,
        vec![old.to.clone(), archived.clone()]
    );
    assert_eq!(envelope.data.from[0].path, archived);
    assert!(envelope.data.from[0].superseded);
    assert_eq!(envelope.data.edges[0].old_path, archived);
    assert_eq!(envelope.data.superseded[0].from, old.to);

    // A record already in the archive gains another successor in place.
    let envelope = dadrock::commands::supersede_records(
        &context,
        dadrock::commands::SupersedeRecordsRequest {
            from: vec![old.id.to_string()],
            to: vec![newer.id.to_string()],
        },
    )
    .unwrap();
    assert_eq!(envelope.changed_paths, vec![archived.clone()]);
    let index = project_index(&project);
    assert_eq!(
        status_record(&index, &old.id.to_string())
            .unwrap()
            .successors
            .len(),
        2
    );
    let report = validate_project_index(&index);
    assert!(location_warnings(&report).is_empty(), "{:?}", report.issues);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn location_check_follows_status_and_successors() {
    let root = temp_project("location-check");
    init_project(&root).unwrap();
    let write = |dir: PathBuf, prefix: &str, title: &str, status: &str, id: &str, extra: &str| {
        fs::write(
            dir.join(format!(
                "{prefix}-{}.md",
                title.to_lowercase().replace(' ', "-")
            )),
            format!(
                "---\ntitle: {title}\nstatus: {status}\nid: {id}\n{extra}---\n\n{FILLED_SECTIONS}"
            ),
        )
        .unwrap();
    };
    let successors = "successors:\n  - 01938b91-bcd0-7e00-8b00-cafebabe0002\n";
    write(
        inbox_dir(&root),
        "01938b8e4d70",
        "Placed candidate",
        "candidate",
        "01938b8e-4d70-7abc-8def-0123456789ab",
        "",
    );
    write(
        records_dir(&root),
        "01938b91bcd0",
        "Placed decision",
        "accepted",
        "01938b91-bcd0-7e00-8b00-cafebabe0002",
        "",
    );
    write(
        archive_dir(&root),
        "01938b92bcd0",
        "Placed superseded",
        "accepted",
        "01938b92-bcd0-7e00-8b00-cafebabe0003",
        successors,
    );
    write(
        archive_dir(&root),
        "01938b93bcd0",
        "Placed rejected",
        "rejected",
        "01938b93-bcd0-7e00-8b00-cafebabe0004",
        "",
    );
    let project = discover_from(&root).unwrap();
    let report = validate_project_index(&project_index(&project));
    assert!(location_warnings(&report).is_empty(), "{:?}", report.issues);

    write(
        records_dir(&root),
        "01938b94bcd0",
        "Superseded in ledger",
        "accepted",
        "01938b94-bcd0-7e00-8b00-cafebabe0005",
        successors,
    );
    write(
        archive_dir(&root),
        "01938b95bcd0",
        "Current in archive",
        "accepted",
        "01938b95-bcd0-7e00-8b00-cafebabe0006",
        "",
    );
    write(
        archive_dir(&root),
        "01938b96bcd0",
        "Candidate in archive",
        "candidate",
        "01938b96-bcd0-7e00-8b00-cafebabe0007",
        "",
    );
    write(
        inbox_dir(&root),
        "01938b97bcd0",
        "Rejected in inbox",
        "rejected",
        "01938b97-bcd0-7e00-8b00-cafebabe0008",
        "",
    );
    let report = validate_project_index(&project_index(&project));
    assert!(
        report.ok,
        "location mismatches are warnings: {:?}",
        report.issues
    );
    let mut warnings: Vec<_> = report
        .issues
        .iter()
        .filter(|issue| issue.code == ProjectValidationIssueCode::StatusLocationMismatch)
        .map(|issue| {
            (
                issue.path.as_ref().unwrap().file_name().unwrap().to_owned(),
                issue.message.clone(),
            )
        })
        .collect();
    warnings.sort();
    assert_eq!(warnings.len(), 4, "{warnings:?}");
    assert!(
        warnings[0].0.contains("superseded-in-ledger") && warnings[0].1.contains("`dadr/archive/`")
    );
    assert!(
        warnings[1].0.contains("current-in-archive") && warnings[1].1.contains("`dadr/ledger/`")
    );
    assert!(
        warnings[2].0.contains("candidate-in-archive") && warnings[2].1.contains("`dadr/inbox/`")
    );
    assert!(
        warnings[3].0.contains("rejected-in-inbox") && warnings[3].1.contains("`dadr/archive/`")
    );

    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn supersedes_move_failure_leaves_candidate_and_predecessor_unchanged() {
    use std::os::unix::fs::PermissionsExt;
    let root = temp_project("supersedes-move-failure");
    init_project(&root).unwrap();
    let project = discover_from(&root).unwrap();
    let old = accepted_record(&project, "Previous choice");
    let nested = project.ledger_dir().join("nested");
    fs::create_dir_all(&nested).unwrap();
    let old_path = nested.join(old.to.file_name().unwrap());
    fs::rename(&old.to, &old_path).unwrap();
    let candidate =
        create_candidate_with_body(&project, "New choice", &[], &successor_body(&[old.id]))
            .unwrap();
    let original = fs::read_to_string(&old_path).unwrap();
    let proposed = fs::read_to_string(&candidate.path).unwrap();
    let permissions = fs::metadata(&nested).unwrap().permissions();
    fs::set_permissions(&nested, fs::Permissions::from_mode(0o500)).unwrap();
    let result = accept_candidate(&project_index(&project), &candidate.id.to_string());
    fs::set_permissions(&nested, permissions).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read_to_string(&old_path).unwrap(), original);
    assert_eq!(fs::read_to_string(&candidate.path).unwrap(), proposed);
    assert!(md_files(&archive_dir(&root)).is_empty());
    assert_eq!(md_files(&records_dir(&root)), Vec::<String>::new());
    assert_eq!(project_index(&project).list_records(None).len(), 2);
    fs::remove_dir_all(root).unwrap();
}
