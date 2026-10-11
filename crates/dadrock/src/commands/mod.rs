use std::path::{Path, PathBuf};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{
    error::{DadrError, Diagnostic, Severity},
    index::ProjectIndex,
    project::{discover_from, ProjectContext},
};

mod agent;
mod catalog;
mod diff;
mod project;
mod records;
mod task;

pub use agent::{
    apply_candidate_revision, draft_with_acp_agent, list_agent_skills,
    revise_candidate_with_acp_agent, revision_proposal_from_response, AgentDraftCommandRequest,
    AgentReviseCommandRequest, AgentRevisionProposal, AgentSkillsResult,
    ApplyCandidateRevisionRequest,
};
pub use catalog::{
    command_catalog, find_command_info, CommandCatalogResult, CommandInfo, CommandInfoRecord,
    CommandMutation, COMMANDS,
};
pub use diff::{TextDiffKind, TextDiffLine};
pub use project::{
    demo_project, demo_project_with_example, initialize_project, show_config, show_dir,
};
pub use records::{
    accept_record, accept_records, list_records_command, list_tags, new_record,
    new_record_with_body, new_successor_record, reject_record, save_record_source, show_status,
    supersede, supersede_records, validate_links, validate_project, validate_records,
    validate_tags, ListRecordsRequest, NewSuccessorRequest, RecordViewFilter,
    SaveRecordSourceRequest, SaveRecordSourceResult, SupersedeRecordsRequest,
    ValidateRecordsResult, ValidatedRecord,
};
pub use task::{
    cancel_task_run_command, list_task_runs_command, list_tasks_command, run_task_command,
    show_task_command, show_task_result_command, submit_task_result_command,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandContext {
    pub cwd: Utf8PathBuf,
}

impl CommandContext {
    pub fn current_dir() -> Result<Self, DadrError> {
        Self::from_path(std::env::current_dir()?)
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, DadrError> {
        Ok(Self {
            cwd: path_to_utf8(path.as_ref().to_path_buf())?,
        })
    }

    pub fn discover_project(&self) -> Result<ProjectContext, DadrError> {
        discover_from(self.cwd.as_std_path())
    }

    pub fn initialized_index(&self) -> Result<(ProjectContext, ProjectIndex), DadrError> {
        let project = self.discover_project()?;
        project.require_initialized()?;
        let index = ProjectIndex::scan(&project)?;
        Ok((project, index))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandEnvelope<T> {
    pub command: String,
    pub ok: bool,
    pub data: T,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<CommandDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_paths: Vec<Utf8PathBuf>,
}

impl<T> CommandEnvelope<T> {
    pub fn new(command: impl Into<String>, data: T) -> Self {
        Self {
            command: command.into(),
            ok: true,
            data,
            diagnostics: Vec::new(),
            changed_paths: Vec::new(),
        }
    }

    pub fn with_ok(mut self, ok: bool) -> Self {
        self.ok = ok;
        self
    }

    pub fn with_diagnostics(mut self, diagnostics: Vec<CommandDiagnostic>) -> Self {
        self.diagnostics = diagnostics;
        self
    }

    pub fn with_changed_paths(mut self, changed_paths: Vec<Utf8PathBuf>) -> Self {
        self.changed_paths = changed_paths;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDiagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl From<&Diagnostic> for CommandDiagnostic {
    fn from(diagnostic: &Diagnostic) -> Self {
        Self {
            severity: diagnostic.severity,
            code: diagnostic.code.as_str().to_owned(),
            message: diagnostic.message.clone(),
            path: diagnostic.path.clone(),
            field: diagnostic.field.clone(),
        }
    }
}

pub trait ProgressSink {
    fn progress(&mut self, progress: CommandProgress);
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandProgress {
    pub command: String,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct NoopProgressSink;

impl ProgressSink for NoopProgressSink {
    fn progress(&mut self, _progress: CommandProgress) {}
}

fn path_to_utf8(path: PathBuf) -> Result<Utf8PathBuf, DadrError> {
    Utf8PathBuf::from_path_buf(path).map_err(|path| DadrError::PathNotUtf8 {
        path: path.display().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::{collections::HashSet, fs, path::PathBuf};

    use super::*;
    use crate::{
        agent::response::{
            candidate_metadata_from_response, parse_agent_response_text, validate_draft_response,
        },
        index::ProjectIndex,
        project::init_project,
        records::create_candidate_with_body,
        RecordId,
    };

    fn command(path: &[&str]) -> &'static CommandInfo {
        find_command_info(path).unwrap_or_else(|| panic!("missing command metadata for {path:?}"))
    }

    fn temp_project(label: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("dadrock-commands-{label}-{}", RecordId::new_v7()));
        if path.exists() {
            fs::remove_dir_all(&path).unwrap();
        }
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn command_catalog_has_no_duplicate_paths() {
        let mut seen = HashSet::new();
        for command in COMMANDS {
            assert!(
                seen.insert(command.path),
                "duplicate command metadata path: {:?}",
                command.path
            );
        }
    }

    #[test]
    fn command_catalog_covers_current_surface() {
        for path in [
            &["help"][..],
            &["commands"],
            &["completions"],
            &["dir"],
            &["config"],
            &["init"],
            &["demo"],
            &["new"],
            &["list"],
            &["status"],
            &["status", "accept"],
            &["status", "reject"],
            &["status", "supersede"],
            &["validate"],
            &["validate", "records"],
            &["validate", "links"],
            &["validate", "tags"],
            &["tags"],
            &["tags", "list"],
            &["mcp"],
            &["mcp", "serve"],
            &["agent"],
            &["agent", "check"],
            &["agent", "draft"],
            &["agent", "skills"],
            &["task"],
            &["task", "list"],
            &["task", "show"],
            &["task", "run"],
            &["task", "runs"],
            &["task", "result"],
        ] {
            assert!(
                find_command_info(path).is_some(),
                "missing command metadata for {path:?}"
            );
        }
    }

    #[test]
    fn command_catalog_marks_project_writes() {
        for path in [
            &["init"][..],
            &["demo"],
            &["new"],
            &["status", "accept"],
            &["status", "reject"],
            &["status", "supersede"],
            &["agent"],
            &["agent", "draft"],
            &["task"],
            &["task", "run"],
        ] {
            assert!(
                command(path).mutation.mutates_project(),
                "expected {path:?} to be marked as project-mutating"
            );
        }

        for path in [
            &["help"][..],
            &["commands"],
            &["completions"],
            &["dir"],
            &["config"],
            &["list"],
            &["status"],
            &["validate"],
            &["validate", "records"],
            &["validate", "links"],
            &["validate", "tags"],
            &["tags"],
            &["tags", "list"],
            &["task", "list"],
            &["task", "show"],
            &["task", "runs"],
            &["task", "result"],
        ] {
            assert!(
                !command(path).mutation.mutates_project(),
                "expected {path:?} to be marked read-only"
            );
        }

        assert_eq!(
            command(&["agent", "draft"]).mutation,
            CommandMutation::WritesWithWrite
        );
    }

    #[test]
    fn command_catalog_marks_json_output_support() {
        for path in [
            &["commands"][..],
            &["config"],
            &["init"],
            &["demo"],
            &["new"],
            &["list"],
            &["status"],
            &["status", "accept"],
            &["status", "reject"],
            &["status", "supersede"],
            &["validate"],
            &["validate", "records"],
            &["validate", "links"],
            &["validate", "tags"],
            &["tags", "list"],
            &["agent", "draft"],
            &["task", "list"],
            &["task", "show"],
            &["task", "run"],
            &["task", "runs"],
            &["task", "result"],
        ] {
            assert!(
                command(path).supports_json,
                "expected {path:?} to support JSON output"
            );
        }

        for path in [
            &["help"][..],
            &["dir"],
            &["completions"],
            &["tags"],
            &["agent"],
            &["task"],
        ] {
            assert!(
                !command(path).supports_json,
                "expected {path:?} to be text-only"
            );
        }
    }

    #[test]
    fn command_catalog_envelope_has_stable_metadata_shape() {
        let envelope = command_catalog();
        assert!(envelope.ok);
        assert_eq!(envelope.command, "commands");
        let validate = envelope
            .data
            .commands
            .iter()
            .find(|command| command.name == "validate")
            .unwrap();

        assert_eq!(
            validate.usage,
            "dadrock validate [--json] [records|links|tags] [options]"
        );
        assert_eq!(validate.mutation, "read-only");
        assert!(!validate.mutates_project);
        assert!(validate.supports_json);
        assert_eq!(validate.path, vec!["validate"]);
        assert!(validate.exit_behavior.contains("diagnostics"));
    }

    #[test]
    fn command_facade_wraps_project_discovery_and_list_filters() {
        let root = temp_project("list");
        init_project(&root).unwrap();
        let context = CommandContext::from_path(&root).unwrap();

        let created =
            new_record(&context, "Tagged candidate", &["#kind/sample".to_owned()]).unwrap();
        assert!(created.ok);
        assert_eq!(created.command, "new");
        assert_eq!(created.changed_paths, vec![created.data.path.clone()]);

        let listed = list_records_command(
            &context,
            ListRecordsRequest {
                status: Some(RecordViewFilter::Candidate),
                tag: Some("#kind/sample".to_owned()),
                ..ListRecordsRequest::default()
            },
        )
        .unwrap();
        assert!(listed.ok);
        assert_eq!(listed.data.len(), 1);
        assert_eq!(listed.data[0].id, created.data.id);

        let unique = list_records_command(
            &context,
            ListRecordsRequest {
                query: Some(created.data.id.short_id()),
                unique: true,
                ..ListRecordsRequest::default()
            },
        )
        .unwrap();
        assert!(unique.ok);
        assert_eq!(unique.data.len(), 1);
        assert_eq!(unique.data[0].id, created.data.id);

        let not_unique = list_records_command(
            &context,
            ListRecordsRequest {
                query: Some("not present".to_owned()),
                unique: true,
                ..ListRecordsRequest::default()
            },
        )
        .unwrap();
        assert!(!not_unique.ok);

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn agent_draft_response_can_create_candidate_without_accepting() {
        let root = temp_project("agent-draft");
        init_project(&root).unwrap();
        let project = discover_from(&root).unwrap();
        let mut response = parse_agent_response_text(r###"```json
{"intent":"draft_candidate","confidence":"medium","summary":"Drafted a candidate.","frontmatter":{"title":"Use local ACP agent drafts","tags":["kind/governance"]},"body_markdown":"## Context\nA local ACP agent can help draft DADR candidates.\n\n## Decision\nUse ACP to prompt the local agent for a structured Dadrock draft envelope.\n\n## Alternatives\nContinue with manual-only candidate creation.\n\n## Consequences\nHumans still review and accept candidates.","source_references":[],"activity_observations":[],"warnings":[],"provenance":{"model":"local-test","tool_runs":[]}}
```"###).unwrap();
        validate_draft_response(&response).unwrap();
        let metadata = candidate_metadata_from_response(&mut response).unwrap();
        let candidate = create_candidate_with_body(
            &project,
            &metadata.title,
            &metadata.tags,
            &response.body_markdown,
        )
        .unwrap();

        assert!(candidate.path.as_str().contains("/dadr/inbox/"));
        let content = fs::read_to_string(&candidate.path).unwrap();
        assert!(content.contains("status: candidate"));
        assert!(content.contains("Use local ACP agent drafts"));
        assert!(content.contains("writer/agent"));
        assert!(content.contains("confidence/medium"));
        assert!(content.contains("writer/agent/local-test"));
        assert!(content.contains("Use ACP to prompt"));
        assert!(!content.contains("accepted_at:"));
        assert!(!content.contains("created:"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn agent_revision_response_previews_diff_before_apply() {
        let root = temp_project("agent-revise");
        init_project(&root).unwrap();
        let project = discover_from(&root).unwrap();
        let context = CommandContext::from_path(&root).unwrap();
        let candidate = create_candidate_with_body(
            &project,
            "Use Markdown records",
            &["kind/design".to_owned()],
            "## Context\nInitial context.\n\n## Decision\nInitial decision.\n\n## Alternatives\nInitial alternative.\n\n## Consequences\nInitial consequence.\n",
        )
        .unwrap();
        let original_id = candidate.id;
        let original_content = fs::read_to_string(&candidate.path).unwrap();

        let response = parse_agent_response_text(r###"```json
{"intent":"revise_candidate","confidence":"high","summary":"Tightened the candidate.","frontmatter":{"title":"Use Markdown with TOML frontmatter","tags":["kind/format"]},"body_markdown":"## Context\nDADR records need readable local storage.\n\n## Decision\nUse Markdown files with TOML frontmatter.\n\n## Alternatives\nUse a database-first record store.\n\n## Consequences\nRecords stay reviewable in ordinary editors.","source_references":[],"activity_observations":[],"warnings":[],"provenance":{"model":"local-test","tool_runs":[]}}
```"###).unwrap();
        let index = ProjectIndex::scan(&project).unwrap();
        let proposal =
            revision_proposal_from_response(&index, &original_id.to_string(), response).unwrap();

        assert!(proposal.data.changed);
        assert_eq!(proposal.data.candidate.id, original_id);
        assert!(proposal
            .data
            .diff
            .iter()
            .any(|line| line.kind == TextDiffKind::Removed && line.text.contains("Initial")));
        assert!(proposal.data.diff.iter().any(|line| {
            line.kind == TextDiffKind::Added
                && line
                    .text
                    .contains("Use Markdown files with TOML frontmatter.")
        }));
        assert_eq!(
            fs::read_to_string(&candidate.path).unwrap(),
            original_content
        );

        let updated = apply_candidate_revision(
            &context,
            ApplyCandidateRevisionRequest {
                query: original_id.to_string(),
                title: proposal.data.proposed_title,
                tags: proposal.data.proposed_tags,
                body_markdown: proposal.data.proposed_body_markdown,
            },
        )
        .unwrap()
        .data
        .candidate;

        assert_eq!(updated.id, original_id);
        assert_eq!(updated.path, candidate.path);
        let content = fs::read_to_string(&updated.path).unwrap();
        assert!(content.contains("status: candidate"));
        assert!(content.contains(&format!("id: {original_id}")));
        assert!(content.contains("Use Markdown with TOML frontmatter"));
        assert!(content.contains("kind/format"));
        assert!(content.contains("writer/agent"));
        assert!(content.contains("confidence/high"));
        assert!(content.contains("Use Markdown files with TOML frontmatter."));
        assert!(!content.contains("accepted_at:"));
        assert!(!content.contains("created:"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn validate_records_envelope_collects_diagnostics() {
        let root = temp_project("validate");
        init_project(&root).unwrap();
        let bad_path = root.join("dadr/ledger/bad.md");
        fs::write(&bad_path, "---\ntitle: Bad\nstatus: candidate\n---\n").unwrap();
        let context = CommandContext::from_path(&root).unwrap();

        let report = validate_records(&context, Vec::new()).unwrap();
        assert!(!report.ok);
        assert_eq!(report.command, "validate records");
        assert_eq!(report.data.records.len(), 1);
        assert!(!report.diagnostics.is_empty());

        fs::remove_dir_all(root).unwrap();
    }
}
