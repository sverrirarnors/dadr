use serde::{Deserialize, Serialize};

use super::CommandEnvelope;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CommandMutation {
    ReadOnly,
    Writes,
    WritesWithApply,
    WritesWithWrite,
}

impl CommandMutation {
    pub fn label(self) -> &'static str {
        match self {
            CommandMutation::ReadOnly => "read-only",
            CommandMutation::Writes => "writes",
            CommandMutation::WritesWithApply => "writes with --apply",
            CommandMutation::WritesWithWrite => "writes with --write",
        }
    }

    pub fn mutates_project(self) -> bool {
        !matches!(self, CommandMutation::ReadOnly)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CommandInfo {
    pub path: &'static [&'static str],
    pub usage: &'static str,
    pub description: &'static str,
    pub mutation: CommandMutation,
    pub supports_json: bool,
    pub exit_behavior: &'static str,
    pub subcommands: &'static [&'static str],
}

impl CommandInfo {
    pub fn name(self) -> String {
        self.path.join(" ")
    }

    pub fn to_record(self) -> CommandInfoRecord {
        CommandInfoRecord {
            path: self.path.iter().map(|part| (*part).to_owned()).collect(),
            name: self.name(),
            usage: format!("dadrock {}", self.usage),
            description: self.description.to_owned(),
            mutation: self.mutation.label().to_owned(),
            mutates_project: self.mutation.mutates_project(),
            supports_json: self.supports_json,
            exit_behavior: self.exit_behavior.to_owned(),
            subcommands: self
                .subcommands
                .iter()
                .map(|subcommand| (*subcommand).to_owned())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandInfoRecord {
    pub path: Vec<String>,
    pub name: String,
    pub usage: String,
    pub description: String,
    pub mutation: String,
    pub mutates_project: bool,
    pub supports_json: bool,
    pub exit_behavior: String,
    pub subcommands: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandCatalogResult {
    pub version: String,
    pub commands: Vec<CommandInfoRecord>,
}

pub const COMMANDS: &[CommandInfo] = &[
    CommandInfo {
        path: &["help"],
        usage: "help [command]",
        description: "Show top-level help or detailed metadata for one command.",
        mutation: CommandMutation::ReadOnly,
        supports_json: false,
        exit_behavior: "Exits non-zero when a requested command topic does not exist.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["commands"],
        usage: "commands [--json]",
        description: "List command metadata for automation and command discovery.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when an unknown option is provided.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["completions"],
        usage: "completions <zsh|bash|fish>",
        description: "Generate a shell completion script.",
        mutation: CommandMutation::ReadOnly,
        supports_json: false,
        exit_behavior: "Exits non-zero when the shell is missing or unsupported.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["dir"],
        usage: "dir",
        description: "Print the discovered Dadrock project directory (dadr/).",
        mutation: CommandMutation::ReadOnly,
        supports_json: false,
        exit_behavior: "Exits non-zero when project discovery fails.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["config"],
        usage: "config [--json]",
        description: "Print the resolved project layout.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when project discovery fails.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["init"],
        usage: "init [directory]",
        description: "Create a Dadrock dadr/ directory.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when options are invalid or initialization cannot write files.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["demo"],
        usage: "demo [--json] [--example <example>] <directory>",
        description: "Seed a new project with a named DADR example.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when the target dadr directory already contains files or cannot be written.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["new"],
        usage: "new [--tag <tag>] [--quiet] <title>",
        description: "Create a candidate decision record from the active candidate template.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when the title, tags, template, project, or write target is invalid.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["list"],
        usage: "list [--status <status>] [--tag <tag>] [--query <query>] [--unique]",
        description: "List records, optionally filtered by lifecycle status, tag, or query.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when filters are invalid, project scanning fails, or --unique is set and the filtered result count is not one.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["history"],
        usage: "history [--patch] [--json] <query>",
        description: "Show a decision's versions as a graph, newest first, with the reason for each change and each version's text edits from Git.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when the query is missing, ambiguous, unresolved, or scanning fails.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["diff"],
        usage: "diff [--json] <old-query> <new-query>",
        description: "Compare two records: title, tags, and body without `## Supersedes`.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when either query is missing, ambiguous, unresolved, or scanning fails.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["status"],
        usage: "status <query>",
        description: "Inspect one record's lifecycle status or run lifecycle transitions.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when the query is missing, ambiguous, unresolved, or scanning fails.",
        subcommands: &["status accept", "status reject", "status supersede"],
    },
    CommandInfo {
        path: &["accept"],
        usage: "accept <candidate-query> [--with <candidate-query>]...",
        description: "Accept a candidate and move it to the ledger; --with accepts more candidates in the same operation. Records named in `## Supersedes` move to the archive.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when any candidate is unresolved, ambiguous, invalid, or cannot be moved, or a record named in `## Supersedes` cannot be superseded; nothing changes then.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["reject"],
        usage: "reject <candidate-query>",
        description: "Reject a candidate and move it to the archive.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when the candidate is unresolved, ambiguous, invalid, or cannot be moved.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["supersede"],
        usage: "supersede --from <old-query> --to <new-query> [--from <old-query>] [--to <new-query>]",
        description: "Link accepted successors to the records they supersede and move those records to the archive.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when any query is invalid, statuses are incompatible, or the requested edges would duplicate or cycle.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["status", "accept"],
        usage: "status accept <candidate-query> [--with <candidate-query>]...",
        description: "Accept a candidate and move it to the ledger; --with accepts more candidates in the same operation. Records named in `## Supersedes` move to the archive.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when any candidate is unresolved, ambiguous, invalid, or cannot be moved, or a record named in `## Supersedes` cannot be superseded; nothing changes then.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["status", "reject"],
        usage: "status reject <candidate-query>",
        description: "Reject a candidate and move it to the archive.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when the candidate is unresolved, ambiguous, invalid, or cannot be moved.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["status", "supersede"],
        usage: "status supersede --from <old-query> --to <new-query> [--from <old-query>] [--to <new-query>]",
        description: "Link accepted successors to the records they supersede and move those records to the archive.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when any query is invalid, statuses are incompatible, or the requested edges would duplicate or cycle.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["validate"],
        usage: "validate [--json] [records|links|tags|changes] [options]",
        description: "Run project validation, or target records, links, tags, or changes since a Git revision.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when validation diagnostics contain one or more errors.",
        subcommands: &[
            "validate records",
            "validate links",
            "validate tags",
            "validate changes",
        ],
    },
    CommandInfo {
        path: &["validate", "records"],
        usage: "validate records [--json] [record-or-directory]...",
        description: "Validate record front matter and structure for paths or the discovered project.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when scanning fails or any parsed record has an error diagnostic.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["validate", "links"],
        usage: "validate links [--json]",
        description: "Report broken or ambiguous wikilinks and local Markdown links.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when link diagnostics contain one or more errors.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["validate", "tags"],
        usage: "validate tags [--json]",
        description: "Report malformed YAML and inline tags.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when tag diagnostics contain one or more errors.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["validate", "changes"],
        usage: "validate changes [--base <rev>] [--staged] [--strict] [--json]",
        description: "Check accepted and rejected records against a Git revision: a changed id or status, a removed successor, or an unparsable file is an error; a title or body edit is a warning with the diff (an error with --strict).",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when any error is reported (with --strict, including title and body edits), or when Git is missing, the project is not in a Git repository, or the base revision does not resolve. Warnings alone exit zero.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["tags"],
        usage: "tags <list> [options]",
        description: "Inspect DADR tag usage.",
        mutation: CommandMutation::ReadOnly,
        supports_json: false,
        exit_behavior: "Exits non-zero when the subcommand is missing or unknown.",
        subcommands: &["tags list"],
    },
    CommandInfo {
        path: &["tags", "list"],
        usage: "tags list [--json]",
        description: "List normalized YAML and inline tags with record and occurrence counts.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when project discovery or tag indexing fails.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["mcp"],
        usage: "mcp <serve> [options]",
        description: "Serve Dadrock MCP tools over stdio.",
        mutation: CommandMutation::ReadOnly,
        supports_json: false,
        exit_behavior: "Exits non-zero when the subcommand is missing or unknown.",
        subcommands: &["mcp serve"],
    },
    CommandInfo {
        path: &["mcp", "serve"],
        usage: "mcp serve [directory]",
        description: "Serve Dadrock MCP tools over stdio for a project.",
        mutation: CommandMutation::ReadOnly,
        supports_json: false,
        exit_behavior: "Exits non-zero when project discovery fails or the MCP transport cannot start.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["agent"],
        usage: "agent <check|draft|skills> [options]",
        description: "Call or inspect configured local agent support.",
        mutation: CommandMutation::WritesWithWrite,
        supports_json: false,
        exit_behavior: "Exits non-zero when the subcommand is missing or unknown.",
        subcommands: &["agent check", "agent draft", "agent skills"],
    },
    CommandInfo {
        path: &["agent", "check"],
        usage: "agent check [--agent <program>] [--agent-arg <arg>] [--no-mcp]",
        description: "Check that a local ACP agent can start a Dadrock session.",
        mutation: CommandMutation::ReadOnly,
        supports_json: false,
        exit_behavior: "Exits non-zero when the ACP agent is unavailable or cannot start a session.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["agent", "draft"],
        usage: "agent draft [--write] [--json] [--permission <deny|allow-once|allow-always>] [--agent <program>] [--agent-arg <arg>] [--skill <name-or-path>] [--no-mcp] <prompt>",
        description: "Ask a local ACP agent to draft a candidate decision.",
        mutation: CommandMutation::WritesWithWrite,
        supports_json: true,
        exit_behavior: "Exits non-zero when the ACP agent is unavailable, permission policy cannot be satisfied, output is too large, JSON is invalid, or candidate creation fails.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["agent", "skills"],
        usage: "agent skills [--json]",
        description: "List available Dadrock agent skills and prompt packs.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when project discovery fails or an installed skill cannot be read.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["task"],
        usage: "task <list|show|run|runs|result> [options]",
        description: "Discover, inspect, and run Dadrock task packages.",
        mutation: CommandMutation::Writes,
        supports_json: false,
        exit_behavior: "Exits non-zero when the subcommand is missing or unknown.",
        subcommands: &[
            "task list",
            "task show",
            "task run",
            "task runs",
            "task result",
        ],
    },
    CommandInfo {
        path: &["task", "list"],
        usage: "task list [--json]",
        description: "List built-in, user, and project task packages visible from the current project.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Returns task diagnostics in the response without failing command execution.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["task", "show"],
        usage: "task show <task-id> [--json]",
        description: "Show one resolved task package, including manifest, source, and prompt.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when the task id is empty or no task resolves.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["task", "run"],
        usage: "task run <task-id> [--path <path>] [--context <text>] [--focus <text>] [--limit <n>] [--input-json <json>] [--json] [--permission <deny|allow-once|allow-always>] [--agent <program>] [--agent-arg <arg>] [--no-mcp]",
        description: "Run a resolved task package with a local ACP agent.",
        mutation: CommandMutation::Writes,
        supports_json: true,
        exit_behavior: "Exits non-zero when the task id, input JSON, ACP agent, task output, or run directory is invalid.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["task", "runs"],
        usage: "task runs [--json]",
        description: "List persisted local task runs.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when run state cannot be read.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["task", "result"],
        usage: "task result <run-id> [--json]",
        description: "Read a persisted task run result.",
        mutation: CommandMutation::ReadOnly,
        supports_json: true,
        exit_behavior: "Exits non-zero when the run cannot be resolved or has no readable result.",
        subcommands: &[],
    },
    CommandInfo {
        path: &["preview"],
        usage: "preview [--port <port>] [directory]",
        description: "Serve the local web review UI on localhost.",
        mutation: CommandMutation::ReadOnly,
        supports_json: false,
        exit_behavior: "Exits non-zero when the preview directory is invalid or the localhost port cannot be bound.",
        subcommands: &[],
    },
];

pub fn find_command_info(path: &[&str]) -> Option<&'static CommandInfo> {
    COMMANDS
        .iter()
        .find(|command| command.path.iter().copied().eq(path.iter().copied()))
}

pub fn command_catalog() -> CommandEnvelope<CommandCatalogResult> {
    CommandEnvelope::new(
        "commands",
        CommandCatalogResult {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            commands: COMMANDS.iter().map(|command| command.to_record()).collect(),
        },
    )
}
