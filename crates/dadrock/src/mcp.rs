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
        RecordViewFilter, SupersedeRecordsRequest, ValidateRecordsResult,
    },
    error::{DadrError, DomainErrorKind},
    index::ProjectIndex,
    links::LinkCheckReport,
    records::{
        AcceptRecordResult, FindRecordsResult, FoundRecord, NewRecordResult, RecordStatusResult,
        RejectRecordResult, ScannedRecordSummary, SupersedeRecordsResult,
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
const TOOL_ACCEPT_RECORD: &str = "dadrock.accept_record";
const TOOL_REJECT_RECORD: &str = "dadrock.reject_record";
const TOOL_SUPERSEDE_RECORDS: &str = "dadrock.supersede_records";
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DadrockMcpAccess {
    Full,
    ReadOnly,
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
        static TOOLS: OnceLock<Vec<Tool>> = OnceLock::new();
        TOOLS.get_or_init(Self::build_tools).clone()
    }

    pub fn tools_for_access(access: DadrockMcpAccess) -> Vec<Tool> {
        let tools = Self::tools();
        match access {
            DadrockMcpAccess::Full => tools,
            DadrockMcpAccess::ReadOnly => tools
                .into_iter()
                .filter(|tool| tool_is_read_only(tool))
                .collect(),
        }
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
                "Run `dadrock history` and return a command envelope with the versions of the decision containing one record (newest first, each with its body without `## Supersedes`), the supersession links between them with the reason from each successor's `## Supersedes` section, and neighbors (the other half of a split, the other input of a merge). With `patch`, each link carries its diff.",
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
                "Run `dadrock new` to create a candidate DADR from a title, tags, and optional body Markdown. With `supersedes`, the candidate is a successor: Dadrock writes its `## Supersedes` section first in the body, linking each named current decision, then `supersedes_reason`. Accepting it supersedes those decisions.",
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
            write_tool(
                TOOL_ACCEPT_RECORD,
                "Run `dadrock status accept` to convert a valid candidate into an accepted decision in the ledger. `with` accepts more candidates in the same operation. Each record named in a `## Supersedes` section of the set gains every candidate in the set that names it as a successor and moves to the archive; nothing changes if any check fails.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "with": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "More candidates to accept together with `query`, such as the other halves of a split."
                        }
                    },
                    "required": ["query"],
                    "additionalProperties": false
                })),
                true,
            ),
            write_tool(
                TOOL_REJECT_RECORD,
                "Run `dadrock status reject` to reject a candidate and move it to the archive.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"}
                    },
                    "required": ["query"],
                    "additionalProperties": false
                })),
                true,
            ),
            write_tool(
                TOOL_SUPERSEDE_RECORDS,
                "Run `dadrock status supersede` to link accepted successors to the accepted records they supersede and move those records to the archive.",
                object_schema(json!({
                    "type": "object",
                    "properties": {
                        "from": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "to": {
                            "type": "array",
                            "items": {"type": "string"}
                        }
                    },
                    "required": ["from", "to"],
                    "additionalProperties": false
                })),
                true,
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
        if self.access == DadrockMcpAccess::ReadOnly && !Self::tool_name_is_read_only(name) {
            return Err(DadrError::InvalidInput(format!(
                "Dadrock MCP tool `{name}` is not available in read-only mode"
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
            TOOL_ACCEPT_RECORD => {
                serde_json::to_value(self.accept_record(arguments)?).map_err(Into::into)
            }
            TOOL_REJECT_RECORD => {
                serde_json::to_value(self.reject_record(arguments)?).map_err(Into::into)
            }
            TOOL_SUPERSEDE_RECORDS => {
                serde_json::to_value(self.supersede_records(arguments)?).map_err(Into::into)
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
            other => Err(DadrError::InvalidInput(format!(
                "unknown Dadrock MCP tool `{other}`"
            ))),
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

    fn accept_record(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<AcceptRecordResult>, DadrError> {
        let mut queries = vec![required_string(&arguments, "query")?];
        queries.extend(optional_string_array(&arguments, "with")?);
        commands::accept_records(&self.context, &queries)
    }

    fn reject_record(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<RejectRecordResult>, DadrError> {
        let query = required_string(&arguments, "query")?;
        commands::reject_record(&self.context, &query)
    }

    fn supersede_records(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<CommandEnvelope<SupersedeRecordsResult>, DadrError> {
        let from = required_string_array(&arguments, "from")?;
        let to = required_string_array(&arguments, "to")?;
        commands::supersede_records(&self.context, SupersedeRecordsRequest { from, to })
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
                    .with_description("Dadrock project and lifecycle tools"),
            )
            .with_instructions(
                "Use these tools as the structured Dadrock API. Read tools inspect DADR records, tags, project snapshots, and validation reports. Write tools call the same Dadrock command layer as the CLI and return command envelopes with changed paths and diagnostics.",
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
    if access == DadrockMcpAccess::ReadOnly {
        args.push("--read-only".to_owned());
    }
    args.push(cwd.to_string());
    Ok(agent_client_protocol::schema::McpServer::Stdio(
        agent_client_protocol::schema::McpServerStdio::new("dadrock", executable).args(args),
    ))
}

impl DadrockMcpServer {
    fn tool_name_is_read_only(name: &str) -> bool {
        Self::tools()
            .iter()
            .find(|tool| tool.name == name)
            .is_some_and(tool_is_read_only)
    }
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

fn required_string_array(
    arguments: &Option<JsonObject>,
    key: &str,
) -> Result<Vec<String>, DadrError> {
    let values = optional_string_array(arguments, key)?;
    if values.is_empty() {
        return Err(DadrError::InvalidInput(format!(
            "Dadrock MCP tool argument `{key}` must contain at least one value"
        )));
    }
    Ok(values)
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
    use super::{DadrockMcpAccess, DadrockMcpServer};
    use crate::commands::CommandContext;

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

    #[test]
    fn read_only_dadrock_mcp_omits_write_tools() {
        let tools = DadrockMcpServer::tools_for_access(DadrockMcpAccess::ReadOnly);

        assert!(tools.iter().any(|tool| tool.name == "dadrock.list_records"));
        assert!(!tools
            .iter()
            .any(|tool| tool.name == "dadrock.create_candidate"));
        assert!(!tools
            .iter()
            .any(|tool| tool.name == "dadrock.accept_record"));
    }

    #[test]
    fn read_only_dadrock_mcp_serves_history_and_diff() {
        let root =
            std::env::temp_dir().join(format!("dadrock-mcp-history-{}", crate::RecordId::new_v7()));
        std::fs::create_dir_all(&root).unwrap();
        crate::project::init_project(&root).unwrap();
        let context = CommandContext::from_path(&root).unwrap();
        let created = crate::commands::new_record(&context, "Only version", &[]).unwrap();
        let id = created.data.id.to_string();
        let server = DadrockMcpServer::new_with_access(context, DadrockMcpAccess::ReadOnly);

        let history = server
            .call_tool_value(
                "dadrock.history",
                super::object_schema(serde_json::json!({"query": id, "patch": true})).into(),
            )
            .unwrap();
        assert_eq!(history["command"], "history");
        assert_eq!(history["data"]["record"], id.as_str());
        assert_eq!(history["data"]["versions"][0]["current"], true);

        let diff = server
            .call_tool_value(
                "dadrock.diff",
                super::object_schema(serde_json::json!({"old": id, "new": id})).into(),
            )
            .unwrap();
        assert_eq!(diff["command"], "diff");
        assert_eq!(diff["data"]["diff"]["changed"], false);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn read_only_dadrock_mcp_rejects_write_tool_calls() {
        let server = DadrockMcpServer::new_with_access(
            CommandContext {
                cwd: camino::Utf8PathBuf::from("/tmp"),
            },
            DadrockMcpAccess::ReadOnly,
        );

        let error = server
            .call_tool_value("dadrock.create_candidate", None)
            .expect_err("read-only MCP must reject write tools");

        assert!(error.to_string().contains("not available in read-only"));
    }
}
