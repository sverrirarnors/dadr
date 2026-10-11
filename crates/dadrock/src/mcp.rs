use std::{
    borrow::Cow,
    future::Future,
    sync::{Arc, OnceLock},
};

use camino::Utf8PathBuf;
use rmcp::{
    model::{
        CallToolRequestParams, CallToolResult, ErrorCode, ErrorData, Implementation, JsonObject,
        ListToolsResult, PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
        ToolAnnotations,
    },
    transport::io::stdio,
    ServerHandler, ServiceExt,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::history::{RecordDiffResult, RecordHistory};
use crate::{
    agent::types::AgentProjectSnapshot,
    commands::{
        self, CommandContext, CommandEnvelope, ListRecordsRequest, NewSuccessorRequest,
        RecordViewFilter, ValidateRecordsResult,
    },
    error::{DadrError, DomainErrorKind},
    index::ProjectIndex,
    links::LinkCheckReport,
    records::{
        FindRecordsResult, FoundRecord, NewRecordResult, RecordStatusResult, ScannedRecordSummary,
    },
    tags::TagReport,
    tasks::{PersistedTaskRun, TaskListing, TaskPackage},
    validation::ProjectValidationReport,
};

const TOOL_COMMAND_CATALOG: &str = "dadrock.command_catalog";
const TOOL_LIST_RECORDS: &str = "dadrock.list_records";
const TOOL_GET_RECORD: &str = "dadrock.get_record";
const TOOL_SEARCH_RECORDS: &str = "dadrock.search_records";
const TOOL_RESOLVE_RECORD: &str = "dadrock.resolve_record";
const TOOL_STATUS: &str = "dadrock.status";
const TOOL_HISTORY: &str = "dadrock.history";
const TOOL_DIFF: &str = "dadrock.diff";
const TOOL_LIST_TAGS: &str = "dadrock.list_tags";
const TOOL_LIST_TASKS: &str = "dadrock.list_tasks";
const TOOL_GET_TASK: &str = "dadrock.get_task";
const TOOL_LIST_TASK_RUNS: &str = "dadrock.list_task_runs";
const TOOL_GET_TASK_RUN: &str = "dadrock.get_task_run";
const TOOL_SUBMIT_TASK_RESULT: &str = "dadrock.submit_task_result";
const TOOL_CREATE_CANDIDATE: &str = "dadrock.create_candidate";
const TOOL_VALIDATE_PROJECT: &str = "dadrock.validate_project";
const TOOL_VALIDATE_RECORDS: &str = "dadrock.validate_records";
const TOOL_VALIDATE_LINKS: &str = "dadrock.validate_links";
const TOOL_VALIDATE_TAGS: &str = "dadrock.validate_tags";
const TOOL_PROJECT_SNAPSHOT: &str = "dadrock.project_snapshot";

#[derive(Debug, Clone)]
pub struct DadrockMcpServer {
    context: CommandContext,
    access: DadrockMcpAccess,
}

/// How much of the Dadrock MCP server a session may use. No level exposes
/// accepting, rejecting, or superseding records: those lifecycle actions are
/// human decisions taken in the web UI, the CLI, or an editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DadrockMcpAccess {
    /// Read tools, plus creating candidates and submitting task results.
    Full,
    /// Read tools only.
    ReadOnly,
    /// Read tools plus `dadrock.submit_task_result`. Task runs only propose:
    /// a human reviews their result before anything changes.
    TaskRun,
}

impl DadrockMcpAccess {
    fn allows(self, tool: &Tool) -> bool {
        match self {
            Self::Full => true,
            Self::ReadOnly => tool_is_read_only(tool),
            Self::TaskRun => tool_is_read_only(tool) || tool.name == TOOL_SUBMIT_TASK_RESULT,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpRecordDetails {
    pub record: ScannedRecordSummary,
    pub markdown: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpProjectSnapshot {
    pub project: AgentProjectSnapshot,
}

impl DadrockMcpServer {
    pub fn new(context: CommandContext) -> Self {
        Self::new_with_access(context, DadrockMcpAccess::Full)
    }

    pub fn new_with_access(context: CommandContext, access: DadrockMcpAccess) -> Self {
        Self { context, access }
    }

    pub fn tools() -> Vec<Tool> {
        Self::tool_definitions().to_vec()
    }

    pub fn tools_for_access(access: DadrockMcpAccess) -> Vec<Tool> {
        Self::tool_definitions()
            .iter()
            .filter(|tool| access.allows(tool))
            .cloned()
            .collect()
    }

    fn tool_definitions() -> &'static [Tool] {
        static TOOLS: OnceLock<Vec<Tool>> = OnceLock::new();
        TOOLS.get_or_init(Self::build_tools)
    }

    fn build_tools() -> Vec<Tool> {
        vec![
            read_tool(
                TOOL_COMMAND_CATALOG,
                "List Dadrock commands, descriptions, mutation behavior, JSON support, and exit behavior.",
                empty_schema(),
            ),
            read_tool(
                TOOL_LIST_RECORDS,
                "Run `dadrock list` and return a command envelope with DADR record summaries, optionally filtered by status, tag, query, or unique-result requirement.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "status": {
                            "type": "string",
                            "enum": ["candidate", "accepted", "superseded", "rejected"]
                        },
                        "tag": {"type": "string"},
                        "query": {"type": "string"},
                        "unique": {"type": "boolean"}
                    },
                    "additionalProperties": false
                })),
            ),
            read_tool(
                TOOL_GET_RECORD,
                "Resolve one DADR record and return its summary plus full Markdown text.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                })),
            ),
            read_tool(
                TOOL_SEARCH_RECORDS,
                "Search DADR records by UUID, UUID prefix, filename, or title.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                })),
            ),
            read_tool(
                TOOL_RESOLVE_RECORD,
                "Resolve exactly one DADR record by UUID, UUID prefix, filename, or title.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                })),
            ),
            read_tool(
                TOOL_STATUS,
                "Run `dadrock status` and return one record's lifecycle status, successors, predecessors, and diagnostics.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                })),
            ),
            read_tool(
                TOOL_HISTORY,
                "Run `dadrock history` and return a command envelope with the versions of the decision containing one record (newest first, each with its body without `## Supersedes`), the supersession links between them with the reason from each successor's `## Supersedes` section, neighbors (the other half of a split, the other input of a merge), and each version's `edits`: the title and body edits Git recorded since the version was accepted or rejected, newest first (null outside a Git repository). With `patch`, each link and each edit carries its diff.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "patch": {"type": "boolean"}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                })),
            ),
            read_tool(
                TOOL_DIFF,
                "Run `dadrock diff` and return a command envelope comparing two records from `old` to `new`: title change, tags added and removed, and a line diff of the bodies without `## Supersedes`.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "old": {"type": "string"},
                        "new": {"type": "string"}
                    },
                    "required": ["old", "new"],
                    "additionalProperties": false
                })),
            ),
            read_tool(
                TOOL_LIST_TAGS,
                "Run `dadrock tags list` and return a command envelope with normalized YAML and inline DADR tags.",
                empty_schema(),
            ),
            read_tool(
                TOOL_LIST_TASKS,
                "Run `dadrock task list` and return a command envelope with discovered built-in, user, and project task packages.",
                empty_schema(),
            ),
            read_tool(
                TOOL_GET_TASK,
                "Run `dadrock task show` and return a command envelope with one resolved task package.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "task_id": {"type": "string"}
                    },
                    "required": ["task_id"],
                    "additionalProperties": false
                })),
            ),
            read_tool(
                TOOL_LIST_TASK_RUNS,
                "Run `dadrock task runs` and return a command envelope with persisted local task runs.",
                empty_schema(),
            ),
            read_tool(
                TOOL_GET_TASK_RUN,
                "Run `dadrock task result` and return a command envelope with one persisted task run result.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "run_id": {"type": "string"}
                    },
                    "required": ["run_id"],
                    "additionalProperties": false
                })),
            ),
            write_tool(
                TOOL_SUBMIT_TASK_RESULT,
                "Submit a completed Dadrock task result for a local task run. The result is validated against the task output kind, persisted as result.json, and used by Dadrock result renderers.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "run_id": {"type": "string"},
                        "result": {"type": "object"}
                    },
                    "required": ["run_id", "result"],
                    "additionalProperties": false
                })),
                false,
            ),
            write_tool(
                TOOL_CREATE_CANDIDATE,
                "Run `dadrock new` to create a candidate DADR from a title, tags, and optional body Markdown. With `supersedes`, the candidate is a successor: Dadrock writes its `## Supersedes` section first in the body, linking each named current decision, then `supersedes_reason`. When a human accepts it, it supersedes those decisions.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "title": {"type": "string"},
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "body_markdown": {"type": "string"},
                        "supersedes": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Accepted, current records the candidate supersedes (several for a merge)."
                        },
                        "supersedes_reason": {
                            "type": "string",
                            "description": "Why the decision changed, written in `## Supersedes` after the links."
                        }
                    },
                    "required": ["title"],
                    "additionalProperties": false
                })),
                false,
            ),
            read_tool(
                TOOL_VALIDATE_PROJECT,
                "Run `dadrock validate` and return project-level validation diagnostics.",
                empty_schema(),
            ),
            read_tool(
                TOOL_VALIDATE_RECORDS,
                "Run `dadrock validate records` and return record front matter and structure diagnostics.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "paths": {
                            "type": "array",
                            "items": {"type": "string"}
                        }
                    },
                    "additionalProperties": false
                })),
            ),
            read_tool(
                TOOL_VALIDATE_LINKS,
                "Run `dadrock validate links` and return DADR wikilink and local Markdown link diagnostics.",
                empty_schema(),
            ),
            read_tool(
                TOOL_VALIDATE_TAGS,
                "Run `dadrock validate tags` and return normalized tag usage plus tag diagnostics.",
                empty_schema(),
            ),
            read_tool(
                TOOL_PROJECT_SNAPSHOT,
                "Return the bounded Dadrock project snapshot used for agent prompting.",
                empty_schema(),
            ),
        ]
    }

    fn call_tool_value(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
    ) -> Result<Value, DadrError> {
        let Some(tool) = Self::tool_definitions()
            .iter()
            .find(|tool| tool.name == name)
        else {
            return Err(unknown_tool(name));
        };
        if !self.access.allows(tool) {
            let scope = match self.access {
                DadrockMcpAccess::ReadOnly => "in read-only mode",
                DadrockMcpAccess::TaskRun => {
                    "to task runs, which propose changes only through `dadrock.submit_task_result`"
                }
                DadrockMcpAccess::Full => "in this session",
            };
            return Err(DadrError::InvalidInput(format!(
                "Dadrock MCP tool `{name}` is not available {scope}"
            )));
        }
        match name {
            TOOL_COMMAND_CATALOG => {
                serde_json::to_value(commands::command_catalog()).map_err(Into::into)
            }
            TOOL_LIST_RECORDS => {
                serde_json::to_value(self.list_records(arguments)?).map_err(Into::into)
            }
            TOOL_GET_RECORD => {
                serde_json::to_value(self.get_record(arguments)?).map_err(Into::into)
            }
            TOOL_SEARCH_RECORDS => {
                serde_json::to_value(self.search_records(arguments)?).map_err(Into::into)
            }
            TOOL_RESOLVE_RECORD => {
                serde_json::to_value(self.resolve_record(arguments)?).map_err(Into::into)
            }
            TOOL_STATUS => serde_json::to_value(self.status(arguments)?).map_err(Into::into),
            TOOL_HISTORY => serde_json::to_value(self.history(arguments)?).map_err(Into::into),
            TOOL_DIFF => serde_json::to_value(self.diff(arguments)?).map_err(Into::into),
            TOOL_LIST_TAGS => serde_json::to_value(self.list_tags()?).map_err(Into::into),
            TOOL_LIST_TASKS => serde_json::to_value(self.list_tasks()).map_err(Into::into),
            TOOL_GET_TASK => serde_json::to_value(self.get_task(arguments)?).map_err(Into::into),
            TOOL_LIST_TASK_RUNS => serde_json::to_value(self.list_task_runs()?).map_err(Into::into),
            TOOL_GET_TASK_RUN => {
                serde_json::to_value(self.get_task_run(arguments)?).map_err(Into::into)
            }
            TOOL_SUBMIT_TASK_RESULT => {
                serde_json::to_value(self.submit_task_result(arguments)?).map_err(Into::into)
            }
            TOOL_CREATE_CANDIDATE => {
                serde_json::to_value(self.create_candidate(arguments)?).map_err(Into::into)
            }
            TOOL_VALIDATE_PROJECT => {
                serde_json::to_value(self.validate_project()?).map_err(Into::into)
            }
            TOOL_VALIDATE_RECORDS => {
                serde_json::to_value(self.validate_records(arguments)?).map_err(Into::into)
            }
            TOOL_VALIDATE_LINKS => serde_json::to_value(self.validate_links()?).map_err(Into::into),
            TOOL_VALIDATE_TAGS => serde_json::to_value(self.validate_tags()?).map_err(Into::into),
            TOOL_PROJECT_SNAPSHOT => {
                serde_json::to_value(self.project_snapshot()?).map_err(Into::into)
            }
            other => Err(unknown_tool(other)),
        }
    }

    fn list_records(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<Vec<ScannedRecordSummary>>, DadrError> {
        let request = ListRecordsRequest {
            status: optional_status(&arguments)?,
            tag: optional_string(&arguments, "tag"),
            query: optional_string(&arguments, "query"),
            unique: optional_bool(&arguments, "unique").unwrap_or(false),
        };
        commands::list_records_command(&self.context, request)
    }

    fn get_record(&self, arguments: Option<JsonObject>) -> Result<McpRecordDetails, DadrError> {
        let query = required_string(&arguments, "query")?;
        let project = self.context.discover_project()?;
        project.require_initialized()?;
        let index = ProjectIndex::scan(&project)?;
        let found = index.resolve_one(&query)?;
        let markdown = std::fs::read_to_string(found.record.path.as_std_path())?;
        Ok(McpRecordDetails {
            record: found.record,
            markdown,
        })
    }

    fn search_records(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<FindRecordsResult, DadrError> {
        let query = required_string(&arguments, "query")?;
        let project = self.context.discover_project()?;
        project.require_initialized()?;
        Ok(ProjectIndex::scan(&project)?.find_records(&query))
    }

    fn resolve_record(&self, arguments: Option<JsonObject>) -> Result<FoundRecord, DadrError> {
        let query = required_string(&arguments, "query")?;
        let project = self.context.discover_project()?;
        project.require_initialized()?;
        ProjectIndex::scan(&project)?.resolve_one(&query)
    }

    fn status(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<RecordStatusResult>, DadrError> {
        let query = required_string(&arguments, "query")?;
        commands::show_status(&self.context, &query)
    }

    fn history(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<RecordHistory>, DadrError> {
        let query = required_string(&arguments, "query")?;
        let patch = optional_bool(&arguments, "patch").unwrap_or(false);
        commands::history(&self.context, &query, patch)
    }

    fn diff(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<RecordDiffResult>, DadrError> {
        let old = required_string(&arguments, "old")?;
        let new = required_string(&arguments, "new")?;
        commands::diff(&self.context, &old, &new)
    }

    fn list_tags(&self) -> Result<CommandEnvelope<TagReport>, DadrError> {
        commands::list_tags(&self.context)
    }

    fn list_tasks(&self) -> CommandEnvelope<TaskListing> {
        commands::list_tasks_command(&self.context)
    }

    fn get_task(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<TaskPackage>, DadrError> {
        let task_id = required_string(&arguments, "task_id")?;
        commands::show_task_command(&self.context, &task_id)
    }

    fn list_task_runs(&self) -> Result<CommandEnvelope<Vec<PersistedTaskRun>>, DadrError> {
        commands::list_task_runs_command(&self.context)
    }

    fn get_task_run(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<Value>, DadrError> {
        let run_id = required_string(&arguments, "run_id")?;
        commands::show_task_result_command(&self.context, &run_id)
    }

    fn submit_task_result(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<PersistedTaskRun>, DadrError> {
        let run_id = required_string(&arguments, "run_id")?;
        let result = required_value(&arguments, "result")?;
        commands::submit_task_result_command(&self.context, &run_id, result)
    }

    fn create_candidate(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<NewRecordResult>, DadrError> {
        let title = required_string(&arguments, "title")?;
        let tags = optional_string_array(&arguments, "tags")?;
        let supersedes = optional_string_array(&arguments, "supersedes")?;
        if !supersedes.is_empty() {
            return commands::new_successor_record(
                &self.context,
                NewSuccessorRequest {
                    title,
                    tags,
                    body_markdown: optional_string(&arguments, "body_markdown"),
                    supersedes,
                    reason: optional_string(&arguments, "supersedes_reason"),
                },
            );
        }
        if let Some(body_markdown) = optional_string(&arguments, "body_markdown") {
            commands::new_record_with_body(&self.context, &title, &tags, &body_markdown)
        } else {
            commands::new_record(&self.context, &title, &tags)
        }
    }

    fn validate_project(&self) -> Result<CommandEnvelope<ProjectValidationReport>, DadrError> {
        commands::validate_project(&self.context)
    }

    fn validate_records(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<ValidateRecordsResult>, DadrError> {
        commands::validate_records(&self.context, optional_path_array(&arguments, "paths")?)
    }

    fn validate_links(&self) -> Result<CommandEnvelope<LinkCheckReport>, DadrError> {
        commands::validate_links(&self.context)
    }

    fn validate_tags(&self) -> Result<CommandEnvelope<TagReport>, DadrError> {
        commands::validate_tags(&self.context)
    }

    fn project_snapshot(&self) -> Result<McpProjectSnapshot, DadrError> {
        let project = self.context.discover_project()?;
        project.require_initialized()?;
        let index = ProjectIndex::scan(&project)?;
        let records = index.list_records(None);
        Ok(McpProjectSnapshot {
            project: AgentProjectSnapshot::new(&project, records),
        })
    }
}

impl ServerHandler for DadrockMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(
                Implementation::new("dadrock-mcp", env!("CARGO_PKG_VERSION"))
                    .with_title("Dadrock MCP")
                    .with_description("Dadrock project tools"),
            )
            .with_instructions(
                "Use these tools as the structured Dadrock API. Read tools inspect DADR records, decision history and diffs, tags, project snapshots, and validation reports. Write tools call the same Dadrock command layer as the CLI and return command envelopes with changed paths and diagnostics. Accepting, rejecting, and superseding records are human decisions and are not available over MCP.",
            )
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> impl Future<Output = Result<ListToolsResult, ErrorData>> + rmcp::service::MaybeSendFuture + '_
    {
        std::future::ready(Ok(ListToolsResult::with_all_items(Self::tools_for_access(
            self.access,
        ))))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        Self::tools_for_access(self.access)
            .into_iter()
            .find(|tool| tool.name == name)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: rmcp::service::RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        self.call_tool_value(&request.name, request.arguments)
            .map(CallToolResult::structured)
            .map_err(dadr_error_to_mcp)
    }
}

pub fn serve_stdio(context: CommandContext) -> Result<(), DadrError> {
    serve_stdio_with_access(context, DadrockMcpAccess::Full)
}

pub fn serve_stdio_with_access(
    context: CommandContext,
    access: DadrockMcpAccess,
) -> Result<(), DadrError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| DadrError::Mcp(format!("failed to start MCP runtime: {err}")))?;
    runtime.block_on(async move {
        let service = DadrockMcpServer::new_with_access(context, access);
        let running = service
            .serve(stdio())
            .await
            .map_err(|err| DadrError::Mcp(format!("MCP server initialization failed: {err}")))?;
        running
            .waiting()
            .await
            .map_err(|err| DadrError::Mcp(format!("MCP server task failed: {err}")))?;
        Ok(())
    })
}

pub fn dadrock_mcp_server_config(
    cwd: &Utf8PathBuf,
) -> Result<agent_client_protocol::schema::McpServer, DadrError> {
    dadrock_mcp_server_config_with_access(cwd, DadrockMcpAccess::Full)
}

pub fn dadrock_mcp_server_config_with_access(
    cwd: &Utf8PathBuf,
    access: DadrockMcpAccess,
) -> Result<agent_client_protocol::schema::McpServer, DadrError> {
    let executable = std::env::current_exe()
        .map_err(|err| DadrError::Mcp(format!("failed to resolve current executable: {err}")))?;
    let mut args = vec!["mcp".to_owned(), "serve".to_owned()];
    match access {
        DadrockMcpAccess::Full => {}
        DadrockMcpAccess::ReadOnly => args.push("--read-only".to_owned()),
        DadrockMcpAccess::TaskRun => args.push("--task-run".to_owned()),
    }
    args.push(cwd.to_string());
    Ok(agent_client_protocol::schema::McpServer::Stdio(
        agent_client_protocol::schema::McpServerStdio::new("dadrock", executable).args(args),
    ))
}

fn unknown_tool(name: &str) -> DadrError {
    DadrError::InvalidInput(format!("unknown Dadrock MCP tool `{name}`"))
}

fn tool_is_read_only(tool: &Tool) -> bool {
    tool.annotations
        .as_ref()
        .and_then(|annotations| annotations.read_only_hint)
        .unwrap_or(false)
}

fn read_tool(name: &'static str, description: &'static str, input_schema: JsonObject) -> Tool {
    tool(
        name,
        description,
        input_schema,
        ToolKind::Read,
        Idempotency::Yes,
    )
}

fn write_tool(
    name: &'static str,
    description: &'static str,
    input_schema: JsonObject,
    destructive: bool,
) -> Tool {
    tool(
        name,
        description,
        input_schema,
        ToolKind::Write { destructive },
        Idempotency::No,
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolKind {
    Read,
    Write { destructive: bool },
}

impl ToolKind {
    fn read_only(self) -> bool {
        matches!(self, Self::Read)
    }

    fn destructive(self) -> bool {
        match self {
            Self::Read => false,
            Self::Write { destructive } => destructive,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Idempotency {
    Yes,
    No,
}

impl Idempotency {
    fn as_bool(self) -> bool {
        matches!(self, Self::Yes)
    }
}

fn tool(
    name: &'static str,
    description: &'static str,
    input_schema: JsonObject,
    kind: ToolKind,
    idempotency: Idempotency,
) -> Tool {
    Tool::new(name, description, Arc::new(input_schema)).with_annotations(
        ToolAnnotations::new()
            .read_only(kind.read_only())
            .destructive(kind.destructive())
            .idempotent(idempotency.as_bool())
            .open_world(false),
    )
}

fn empty_schema() -> JsonObject {
    object_schema(json!({
        "type": "object",
        "properties": {},
        "additionalProperties": false
    }))
}

fn object_schema(value: Value) -> JsonObject {
    match value {
        Value::Object(object) => object,
        _ => JsonObject::new(),
    }
}

fn required_string(arguments: &Option<JsonObject>, key: &str) -> Result<String, DadrError> {
    optional_string(arguments, key).ok_or_else(|| {
        DadrError::InvalidInput(format!("Dadrock MCP tool argument `{key}` is required"))
    })
}

fn required_value(arguments: &Option<JsonObject>, key: &str) -> Result<Value, DadrError> {
    arguments
        .as_ref()
        .and_then(|args| args.get(key))
        .cloned()
        .ok_or_else(|| {
            DadrError::InvalidInput(format!("Dadrock MCP tool argument `{key}` is required"))
        })
}

fn optional_string(arguments: &Option<JsonObject>, key: &str) -> Option<String> {
    arguments
        .as_ref()
        .and_then(|args| args.get(key))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn optional_string_array(
    arguments: &Option<JsonObject>,
    key: &str,
) -> Result<Vec<String>, DadrError> {
    let Some(value) = arguments.as_ref().and_then(|args| args.get(key)) else {
        return Ok(Vec::new());
    };
    let Some(values) = value.as_array() else {
        return Err(DadrError::InvalidInput(format!(
            "Dadrock MCP tool argument `{key}` must be an array of strings"
        )));
    };
    values
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                DadrError::InvalidInput(format!(
                    "Dadrock MCP tool argument `{key}` must be an array of strings"
                ))
            })
        })
        .collect()
}

fn optional_path_array(
    arguments: &Option<JsonObject>,
    key: &str,
) -> Result<Vec<Utf8PathBuf>, DadrError> {
    Ok(optional_string_array(arguments, key)?
        .into_iter()
        .map(Utf8PathBuf::from)
        .collect())
}

fn optional_bool(arguments: &Option<JsonObject>, key: &str) -> Option<bool> {
    arguments
        .as_ref()
        .and_then(|args| args.get(key))
        .and_then(Value::as_bool)
}

fn optional_status(arguments: &Option<JsonObject>) -> Result<Option<RecordViewFilter>, DadrError> {
    let Some(status) = optional_string(arguments, "status") else {
        return Ok(None);
    };
    serde_json::from_value(Value::String(status.clone()))
        .map(Some)
        .map_err(|_| {
            DadrError::InvalidInput(format!(
                "invalid status `{status}`; expected candidate, accepted, superseded, or rejected"
            ))
        })
}

fn dadr_error_to_mcp(error: DadrError) -> ErrorData {
    match error {
        DadrError::Domain(domain) if domain.kind() == DomainErrorKind::InvalidInput => {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                Cow::Owned(domain.message().to_owned()),
                None,
            )
        }
        DadrError::RecordNotFound(message)
        | DadrError::AmbiguousRecordQuery { query: message, .. } => {
            ErrorData::new(ErrorCode::INVALID_PARAMS, Cow::Owned(message), None)
        }
        other => ErrorData::new(
            ErrorCode::INTERNAL_ERROR,
            Cow::Owned(other.to_string()),
            None,
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use rmcp::{model::ErrorCode, ServerHandler};
    use serde_json::json;

    use super::{
        dadr_error_to_mcp, dadrock_mcp_server_config_with_access, DadrockMcpAccess,
        DadrockMcpServer,
    };
    use crate::{
        commands::CommandContext,
        error::DomainErrorKind,
        project::init_project,
        tasks::{create_task_run, discover_tasks, read_task_run, TaskRunStatus},
        RecordId,
    };

    #[test]
    fn dadrock_mcp_tools_mark_read_and_write_capabilities() {
        let tools = DadrockMcpServer::tools();

        assert!(tools.iter().any(|tool| tool.name == "dadrock.list_records"));
        let list_records = tools
            .iter()
            .find(|tool| tool.name == "dadrock.list_records")
            .expect("list records tool");
        assert!(list_records
            .annotations
            .as_ref()
            .and_then(|annotations| annotations.read_only_hint)
            .unwrap_or(false));

        let list_tasks = tools
            .iter()
            .find(|tool| tool.name == "dadrock.list_tasks")
            .expect("list tasks tool");
        let list_tasks_annotations = list_tasks
            .annotations
            .as_ref()
            .expect("list tasks annotations");
        assert_eq!(list_tasks_annotations.read_only_hint, Some(true));
        assert_eq!(list_tasks_annotations.idempotent_hint, Some(true));
        assert_eq!(list_tasks_annotations.open_world_hint, Some(false));

        let create_candidate = tools
            .iter()
            .find(|tool| tool.name == "dadrock.create_candidate")
            .expect("create candidate tool");
        assert_eq!(
            create_candidate
                .annotations
                .as_ref()
                .and_then(|annotations| annotations.read_only_hint),
            Some(false)
        );
    }

    const READ_TOOLS: &[&str] = &[
        "dadrock.command_catalog",
        "dadrock.list_records",
        "dadrock.get_record",
        "dadrock.search_records",
        "dadrock.resolve_record",
        "dadrock.status",
        "dadrock.history",
        "dadrock.diff",
        "dadrock.list_tags",
        "dadrock.list_tasks",
        "dadrock.get_task",
        "dadrock.list_task_runs",
        "dadrock.get_task_run",
        "dadrock.validate_project",
        "dadrock.validate_records",
        "dadrock.validate_links",
        "dadrock.validate_tags",
        "dadrock.project_snapshot",
    ];
    const LIFECYCLE_TOOLS: &[&str] = &[
        "dadrock.accept_record",
        "dadrock.reject_record",
        "dadrock.supersede_records",
    ];
    const ACCESS_LEVELS: [DadrockMcpAccess; 3] = [
        DadrockMcpAccess::Full,
        DadrockMcpAccess::ReadOnly,
        DadrockMcpAccess::TaskRun,
    ];

    fn tool_names(access: DadrockMcpAccess) -> BTreeSet<String> {
        DadrockMcpServer::tools_for_access(access)
            .into_iter()
            .map(|tool| tool.name.into_owned())
            .collect()
    }

    fn names(groups: &[&[&str]]) -> BTreeSet<String> {
        groups
            .iter()
            .flat_map(|group| group.iter().map(|name| (*name).to_owned()))
            .collect()
    }

    fn temp_project(label: &str) -> (std::path::PathBuf, CommandContext) {
        let root = std::env::temp_dir().join(format!("dadrock-mcp-{label}-{}", RecordId::new_v7()));
        init_project(&root).expect("initialize project");
        let context = CommandContext::from_path(&root).expect("command context");
        (root, context)
    }

    #[test]
    fn each_access_level_lists_exactly_its_tools() {
        assert_eq!(
            tool_names(DadrockMcpAccess::Full),
            names(&[
                READ_TOOLS,
                &["dadrock.create_candidate", "dadrock.submit_task_result"]
            ])
        );
        assert_eq!(tool_names(DadrockMcpAccess::ReadOnly), names(&[READ_TOOLS]));
        assert_eq!(
            tool_names(DadrockMcpAccess::TaskRun),
            names(&[READ_TOOLS, &["dadrock.submit_task_result"]])
        );
        assert_eq!(
            tool_names(DadrockMcpAccess::Full),
            DadrockMcpServer::tools()
                .into_iter()
                .map(|tool| tool.name.into_owned())
                .collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn no_access_level_exposes_lifecycle_tools() {
        for access in ACCESS_LEVELS {
            let listed = tool_names(access);
            let server = DadrockMcpServer::new_with_access(
                CommandContext {
                    cwd: camino::Utf8PathBuf::from("/tmp"),
                },
                access,
            );
            for name in LIFECYCLE_TOOLS {
                assert!(!listed.contains(*name), "{access:?} lists {name}");
                assert!(
                    server.get_tool(name).is_none(),
                    "{access:?} resolves {name}"
                );
                let error = dadr_error_to_mcp(
                    server
                        .call_tool_value(name, None)
                        .expect_err("lifecycle tools are not served"),
                );
                assert_eq!(error.code, ErrorCode::INVALID_PARAMS, "{access:?} {name}");
                assert_eq!(
                    error.message,
                    format!("unknown Dadrock MCP tool `{name}`"),
                    "{access:?} {name}"
                );
            }
        }
    }

    #[test]
    fn restricted_access_refuses_unlisted_write_tools_before_running_them() {
        let (root, context) = temp_project("refuse-create");
        let arguments = json!({"title": "Created by an agent"}).as_object().cloned();
        for access in [DadrockMcpAccess::ReadOnly, DadrockMcpAccess::TaskRun] {
            let server = DadrockMcpServer::new_with_access(context.clone(), access);
            let error = server
                .call_tool_value("dadrock.create_candidate", arguments.clone())
                .expect_err("restricted access must refuse create_candidate");
            assert!(
                error.is_domain_kind(DomainErrorKind::InvalidInput),
                "{access:?}: {error}"
            );
            let (_, index) = context.initialized_index().expect("project index");
            assert!(
                index.list_records(None).is_empty(),
                "{access:?} created a record"
            );
        }

        let server = DadrockMcpServer::new_with_access(context.clone(), DadrockMcpAccess::Full);
        server
            .call_tool_value("dadrock.create_candidate", arguments)
            .expect("full access creates candidates");
        let (_, index) = context.initialized_index().expect("project index");
        assert_eq!(index.list_records(None).len(), 1);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn task_run_access_submits_task_results() {
        let (root, context) = temp_project("task-run-submit");
        let user_tasks = context.cwd.join("user-tasks");
        let tasks = discover_tasks(&context.cwd, Some(&user_tasks));
        let task = tasks
            .tasks
            .iter()
            .find(|task| task.manifest.id == "find-conflicts")
            .expect("built-in find-conflicts task");
        let run = create_task_run(&context.cwd, task, json!({})).expect("create task run");
        let server = DadrockMcpServer::new_with_access(context.clone(), DadrockMcpAccess::TaskRun);

        let submitted = server
            .call_tool_value(
                "dadrock.submit_task_result",
                json!({
                    "run_id": run.id.to_string(),
                    "result": {"kind": "conflict_set.v1", "summary": "None.", "findings": []}
                })
                .as_object()
                .cloned(),
            )
            .expect("task runs may submit their result");

        assert_eq!(submitted["data"]["status"], json!("succeeded"));
        assert_eq!(
            read_task_run(&run.run_dir).expect("read run").status,
            TaskRunStatus::Succeeded
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn every_access_level_serves_history_and_diff() {
        let (root, context) = temp_project("history");
        let created =
            crate::commands::new_record(&context, "Only version", &[]).expect("create record");
        let id = created.data.id.to_string();
        for access in ACCESS_LEVELS {
            let server = DadrockMcpServer::new_with_access(context.clone(), access);

            let history = server
                .call_tool_value(
                    "dadrock.history",
                    json!({"query": id, "patch": true}).as_object().cloned(),
                )
                .expect("history");
            assert_eq!(history["command"], "history", "{access:?}");
            assert_eq!(history["data"]["record"], id.as_str(), "{access:?}");
            assert_eq!(
                history["data"]["versions"][0]["current"], true,
                "{access:?}"
            );

            let diff = server
                .call_tool_value(
                    "dadrock.diff",
                    json!({"old": id, "new": id}).as_object().cloned(),
                )
                .expect("diff");
            assert_eq!(diff["command"], "diff", "{access:?}");
            assert_eq!(diff["data"]["diff"]["changed"], false, "{access:?}");
        }
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn server_config_passes_access_level_to_mcp_serve() {
        let cwd = camino::Utf8PathBuf::from("/tmp/dadrock-project");
        for (access, flag) in [
            (DadrockMcpAccess::Full, None),
            (DadrockMcpAccess::ReadOnly, Some("--read-only")),
            (DadrockMcpAccess::TaskRun, Some("--task-run")),
        ] {
            let config = dadrock_mcp_server_config_with_access(&cwd, access).expect("config");
            let args = serde_json::to_value(config).expect("serialize config")["args"].clone();
            let mut expected = vec!["mcp", "serve"];
            expected.extend(flag);
            expected.push(cwd.as_str());
            assert_eq!(args, json!(expected), "{access:?}");
        }
    }
}
