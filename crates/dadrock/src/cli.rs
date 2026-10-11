use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use dadrock::{agent::acp::AcpPermissionPolicy, commands::RecordViewFilter, demo::DemoExample};

#[derive(Debug, Parser)]
#[command(
    name = "dadrock",
    version,
    about = "DADR decision records and web review UI",
    long_about = "DADR decision records and web review UI.\n\nWith no command, prints this help menu."
)]
#[command(disable_help_subcommand = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    pub fn into_command_or_default(self) -> Command {
        self.command.unwrap_or_default()
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    #[command(about = "Show top-level help or detailed metadata for one command.")]
    Help(HelpArgs),
    #[command(about = "List command metadata for automation and command discovery.")]
    Commands(JsonArg),
    #[command(about = "Generate a shell completion script.")]
    Completions(CompletionsArgs),
    #[command(
        hide = true,
        about = "Print the CLI reference as Markdown (used to regenerate docs-src/dadrock/cli.md)."
    )]
    GenCliDocs,
    #[command(about = "Print the project's dadr/ directory, even before it exists.")]
    Dir,
    #[command(about = "Print the resolved project layout.")]
    Config(JsonArg),
    #[command(about = "Create a Dadrock dadr/ directory.")]
    Init(InitArgs),
    #[command(about = "Seed a new project with a named DADR example.")]
    Demo(DemoArgs),
    #[command(about = "Create a candidate decision record from the active candidate template.")]
    New(NewArgs),
    #[command(about = "List records, optionally filtered by lifecycle status, tag, or query.")]
    List(ListArgs),
    #[command(
        about = "Show a decision's versions as a graph, newest first, with the reason for each change and each version's text edits from Git."
    )]
    History(HistoryArgs),
    #[command(about = "Compare two records: title, tags, and body without `## Supersedes`.")]
    Diff(DiffArgs),
    #[command(about = "Inspect one record's lifecycle status or run lifecycle transitions.")]
    Status(StatusArgs),
    #[command(about = "Accept a candidate decision, or several together with --with.")]
    Accept(AcceptArgs),
    #[command(about = "Reject a candidate decision.")]
    Reject(QueryArgs),
    #[command(
        about = "Link accepted successors to the records they supersede and move those records to the archive."
    )]
    Supersede(StatusSupersedeArgs),
    #[command(about = "Run project validation, or target records, links, tags, or changes.")]
    Validate(ValidateArgs),
    #[command(about = "Inspect DADR tag usage.")]
    Tags(TagsArgs),
    #[command(about = "Serve Dadrock MCP tools over stdio.")]
    Mcp(McpArgs),
    #[command(about = "Call a configured local ACP agent for candidate drafting.")]
    Agent(AgentArgs),
    #[command(about = "Discover, inspect, and run Dadrock task packages.")]
    Task(TaskArgs),
    #[command(about = "Serve the local web review UI.")]
    Preview(PreviewArgs),
}

impl Default for Command {
    fn default() -> Self {
        Self::Help(HelpArgs { topic: Vec::new() })
    }
}

#[derive(Debug, Args)]
pub struct HelpArgs {
    /// The command to describe, such as `validate changes`.
    #[arg(num_args = 0..)]
    pub topic: Vec<String>,
}

#[derive(Debug, Args)]
pub struct JsonArg {
    /// Print the JSON envelope instead of text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
    /// The shell to generate the script for.
    #[arg(value_enum)]
    pub shell: ShellArg,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ShellArg {
    Bash,
    Zsh,
    Fish,
}

impl From<ShellArg> for Shell {
    fn from(shell: ShellArg) -> Self {
        match shell {
            ShellArg::Bash => Self::Bash,
            ShellArg::Zsh => Self::Zsh,
            ShellArg::Fish => Self::Fish,
        }
    }
}

#[derive(Debug, Args)]
pub struct InitArgs {
    /// Where to create `dadr/`: its parent directory, or a path ending in
    /// `dadr` (defaults to the current directory).
    pub directory: Option<Utf8PathBuf>,
}

#[derive(Debug, Args)]
pub struct DemoArgs {
    /// Print the JSON envelope instead of text.
    #[arg(long)]
    pub json: bool,
    /// The example project to seed.
    #[arg(long, value_enum, default_value_t = DemoExampleArg::Lifecycle)]
    pub example: DemoExampleArg,
    /// The directory to seed, which must be new or empty (defaults to the
    /// current directory).
    pub directory: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum DemoExampleArg {
    Lifecycle,
    CandidateReview,
    Penguins,
    Broken,
    Conflicts,
    TaskHistory,
}

impl From<DemoExampleArg> for DemoExample {
    fn from(example: DemoExampleArg) -> Self {
        match example {
            DemoExampleArg::Lifecycle => Self::Lifecycle,
            DemoExampleArg::CandidateReview => Self::CandidateReview,
            DemoExampleArg::Penguins => Self::Penguins,
            DemoExampleArg::Broken => Self::Broken,
            DemoExampleArg::Conflicts => Self::Conflicts,
            DemoExampleArg::TaskHistory => Self::TaskHistory,
        }
    }
}

#[derive(Debug, Args)]
pub struct NewArgs {
    /// A tag for the record, without `#`; repeat for more.
    #[arg(long = "tag", value_name = "TAG")]
    pub tags: Vec<String>,
    /// Print only the new record's id instead of the JSON envelope.
    #[arg(long)]
    pub quiet: bool,
    /// The record's title; the words join into one title. Options can come
    /// before or after it; put `--` before a title that starts with `-`.
    #[arg(required = true, num_args = 1..)]
    pub title: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// Only records with this status; `accepted` leaves out superseded
    /// records, and `superseded` lists only them.
    #[arg(long, value_enum)]
    pub status: Option<RecordStatusArg>,
    /// Only records with this tag, in front matter or inline (the `#` is
    /// optional).
    #[arg(long)]
    pub tag: Option<String>,
    /// Only records matching a full id, an id prefix, or part of a filename
    /// or title.
    #[arg(long)]
    pub query: Option<String>,
    /// Exit non-zero unless exactly one record matches.
    #[arg(long)]
    pub unique: bool,
}

#[derive(Debug, Args)]
pub struct HistoryArgs {
    /// Print each change's diff under its link, and each text edit's diff under its version.
    #[arg(long)]
    pub patch: bool,
    /// Print the JSON envelope instead of text.
    #[arg(long)]
    pub json: bool,
    /// Any version of the decision: full id, id prefix, filename, or title.
    #[arg(required = true, num_args = 1..)]
    pub query: Vec<String>,
}

#[derive(Debug, Args)]
pub struct DiffArgs {
    /// Print the JSON envelope instead of text.
    #[arg(long)]
    pub json: bool,
    /// The record to compare from: full id, id prefix, filename, or title.
    pub old: String,
    /// The record to compare to.
    pub new: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "snake_case")]
pub enum RecordStatusArg {
    Candidate,
    Accepted,
    Rejected,
    Superseded,
}

impl From<RecordStatusArg> for RecordViewFilter {
    fn from(status: RecordStatusArg) -> Self {
        match status {
            RecordStatusArg::Candidate => Self::Candidate,
            RecordStatusArg::Accepted => Self::Accepted,
            RecordStatusArg::Rejected => Self::Rejected,
            RecordStatusArg::Superseded => Self::Superseded,
        }
    }
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    #[command(subcommand)]
    pub command: Option<StatusCommand>,
    /// The record to inspect: full id, id prefix, filename, or title; the
    /// words join into one query.
    #[arg(num_args = 0..)]
    pub query: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum StatusCommand {
    #[command(
        about = "Accept a candidate and move it to the ledger, or several together with --with."
    )]
    Accept(AcceptArgs),
    #[command(about = "Reject a candidate and move it to the archive.")]
    Reject(QueryArgs),
    #[command(
        about = "Link accepted successors to the records they supersede and move those records to the archive."
    )]
    Supersede(StatusSupersedeArgs),
}

#[derive(Debug, Args)]
pub struct QueryArgs {
    /// The candidate: full id, id prefix, filename, or title; the words join
    /// into one query.
    #[arg(required = true, num_args = 1..)]
    pub query: Vec<String>,
}

impl QueryArgs {
    pub fn joined(&self) -> String {
        self.query.join(" ")
    }
}

#[derive(Debug, Args)]
pub struct AcceptArgs {
    /// The candidate to accept; the words join into one query.
    #[arg(required = true, num_args = 1..)]
    pub query: Vec<String>,
    /// Another candidate to accept in the same operation, such as the other
    /// half of a split. Repeat for more.
    #[arg(long = "with", value_name = "QUERY")]
    pub with: Vec<String>,
}

impl AcceptArgs {
    /// Every candidate query: the positional query first, then each --with.
    pub fn queries(&self) -> Vec<String> {
        let mut queries = vec![self.query.join(" ")];
        queries.extend(self.with.iter().cloned());
        queries
    }
}

#[derive(Debug, Args)]
pub struct StatusSupersedeArgs {
    /// An accepted record to supersede: full id, id prefix, filename, or
    /// title. Repeat for more.
    #[arg(long = "from")]
    pub from: Vec<String>,
    /// An accepted record that replaces it. Repeat for more; every `--from`
    /// record gets every `--to` record as a successor.
    #[arg(long = "to")]
    pub to: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Print the project validation's JSON envelope instead of text.
    #[arg(long)]
    pub json: bool,
    #[command(subcommand)]
    pub command: Option<ValidateCommand>,
}

#[derive(Debug, Subcommand)]
pub enum ValidateCommand {
    #[command(about = "Validate record front matter and structure.")]
    Records(ValidateRecordsArgs),
    #[command(about = "Report broken or ambiguous wikilinks and local Markdown links.")]
    Links(JsonArg),
    #[command(about = "Report malformed YAML and inline tags.")]
    Tags(JsonArg),
    #[command(
        about = "Check accepted and rejected records against a Git revision: a changed id or status or a removed successor is an error; a title or body edit is a warning with the diff."
    )]
    Changes(ValidateChangesArgs),
}

#[derive(Debug, Args)]
pub struct ValidateChangesArgs {
    /// Git revision to compare with.
    #[arg(long, default_value = "HEAD")]
    pub base: String,
    /// Compare the index (staged changes) instead of the working tree.
    #[arg(long)]
    pub staged: bool,
    /// Report title and body edits to accepted and rejected records as errors.
    #[arg(long)]
    pub strict: bool,
    /// Print the JSON envelope instead of text.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct ValidateRecordsArgs {
    /// No effect: this command always prints the JSON envelope.
    #[arg(long)]
    pub json: bool,
    /// Record files or directories to validate (defaults to every record in
    /// the project).
    pub paths: Vec<Utf8PathBuf>,
}

#[derive(Debug, Args)]
pub struct TagsArgs {
    #[command(subcommand)]
    pub command: TagsCommand,
}

#[derive(Debug, Subcommand)]
pub enum TagsCommand {
    #[command(about = "List normalized YAML and inline tags.")]
    List(JsonArg),
}

#[derive(Debug, Args)]
pub struct McpArgs {
    #[command(subcommand)]
    pub command: McpCommand,
}

#[derive(Debug, Subcommand)]
pub enum McpCommand {
    #[command(about = "Serve Dadrock MCP tools over stdio.")]
    Serve(McpServeArgs),
}

#[derive(Debug, Args)]
pub struct McpServeArgs {
    /// Serve only the read tools, so no tool can create or change a record.
    #[arg(long)]
    pub read_only: bool,
    /// Serve the access level Dadrock gives its own task runs: read tools
    /// plus task-result submission. Internal; set by the task runner.
    #[arg(long, hide = true, conflicts_with = "read_only")]
    pub task_run: bool,
    /// The directory to find the project from (defaults to the current
    /// directory).
    pub directory: Option<Utf8PathBuf>,
}

#[derive(Debug, Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    #[command(about = "Check that a local ACP agent can start a Dadrock session.")]
    Check(AgentCheckArgs),
    #[command(about = "Ask a local ACP agent to draft a candidate decision.")]
    Draft(AgentDraftArgs),
    #[command(about = "List available Dadrock agent skills and prompt packs.")]
    Skills(JsonArg),
}

#[derive(Debug, Args)]
pub struct AgentCheckArgs {
    /// The ACP agent program to start (defaults to `DADROCK_ACP_AGENT`, then
    /// an installed provider).
    #[arg(long)]
    pub agent: Option<String>,
    /// An extra argument for the agent program; repeat for more.
    #[arg(long = "agent-arg", allow_hyphen_values = true)]
    pub agent_args: Vec<String>,
    /// Start the agent without Dadrock's MCP tools.
    #[arg(long = "no-mcp")]
    pub no_mcp: bool,
}

#[derive(Debug, Args)]
pub struct AgentDraftArgs {
    /// Save the draft as a new candidate record; without it, the draft is
    /// only printed.
    #[arg(long)]
    pub write: bool,
    /// Print the JSON envelope instead of text.
    #[arg(long)]
    pub json: bool,
    /// How to answer the agent's tool permission requests.
    #[arg(long, value_enum, default_value_t = PermissionPolicyArg::Deny)]
    pub permission: PermissionPolicyArg,
    /// The ACP agent program to start (defaults to `DADROCK_ACP_AGENT`, then
    /// an installed provider).
    #[arg(long)]
    pub agent: Option<String>,
    /// An extra argument for the agent program; repeat for more.
    #[arg(long = "agent-arg", allow_hyphen_values = true)]
    pub agent_args: Vec<String>,
    /// An agent skill or prompt pack to include, by name or path; repeat for
    /// more.
    #[arg(long = "skill", visible_alias = "prompt-pack")]
    pub skills: Vec<String>,
    /// Start the agent without Dadrock's MCP tools.
    #[arg(long = "no-mcp")]
    pub no_mcp: bool,
    /// The decision to draft; the words join into one prompt.
    #[arg(required = true, num_args = 1..)]
    pub prompt: Vec<String>,
}

#[derive(Debug, Args)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub command: TaskCommand,
}

#[derive(Debug, Subcommand)]
pub enum TaskCommand {
    #[command(about = "List built-in, user, and project task packages.")]
    List(JsonArg),
    #[command(about = "Show one resolved task package.")]
    Show(TaskShowArgs),
    #[command(about = "Run one resolved task package with a local ACP agent.")]
    Run(Box<TaskRunArgs>),
    #[command(about = "List persisted local task runs.")]
    Runs(JsonArg),
    #[command(about = "Read a persisted task run result.")]
    Result(TaskResultArgs),
}

#[derive(Debug, Args)]
pub struct TaskShowArgs {
    /// The task id, as `dadrock task list` prints it.
    pub task_id: String,
    /// Print the JSON envelope instead of text.
    #[arg(long)]
    pub json: bool,
    /// Print the task file exactly as written, front matter included.
    #[arg(long, conflicts_with = "json")]
    pub raw: bool,
}

#[derive(Debug, Args)]
pub struct TaskRunArgs {
    /// Task to run, by id (see `dadrock task list`).
    pub task_id: String,
    /// Task inputs as a JSON object, for tasks without friendly input flags.
    #[arg(long = "input-json", value_name = "JSON", conflicts_with_all = ["path", "context", "focus", "limit"])]
    pub input_json: Option<String>,
    /// File or directory to inspect (code-to-record).
    #[arg(long)]
    pub path: Option<String>,
    /// Describe the decision to capture (context-to-record).
    #[arg(long)]
    pub context: Option<String>,
    /// Topic to concentrate on (code-to-record, decisions-to-spec).
    #[arg(long)]
    pub focus: Option<String>,
    /// Maximum initial candidates (code-to-record defaults to 5).
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub limit: Option<u32>,
    /// What the agent should do, as task context (required by custom).
    #[arg(long)]
    pub instructions: Option<String>,
    /// Record for the agent to work on, by id, short id, or title; repeatable.
    #[arg(long = "record", value_name = "QUERY")]
    pub records: Vec<String>,
    /// File or folder for the agent to read, as task context; repeatable.
    #[arg(long, value_name = "PATH")]
    pub attach: Vec<String>,
    /// Print the run as a JSON command envelope.
    #[arg(long)]
    pub json: bool,
    /// How to answer the agent's tool permission requests.
    #[arg(long, value_enum, default_value_t = PermissionPolicyArg::Deny)]
    pub permission: PermissionPolicyArg,
    /// ACP agent executable to run instead of the configured or detected one.
    #[arg(long, value_name = "PROGRAM")]
    pub agent: Option<String>,
    /// Extra argument for the ACP agent; repeatable.
    #[arg(long = "agent-arg", value_name = "ARG", allow_hyphen_values = true)]
    pub agent_args: Vec<String>,
    /// Do not attach Dadrock's MCP server to the agent session.
    #[arg(long = "no-mcp")]
    pub no_mcp: bool,
}

impl TaskRunArgs {
    pub fn task_input(&self) -> anyhow::Result<serde_json::Value> {
        if let Some(json) = &self.input_json {
            return Ok(serde_json::from_str(json)?);
        }
        let mut input = serde_json::Map::new();
        for (name, value) in [
            ("path", &self.path),
            ("context", &self.context),
            ("focus", &self.focus),
        ] {
            if let Some(value) = value {
                input.insert(name.to_owned(), serde_json::json!(value));
            }
        }
        if let Some(limit) = self.limit {
            input.insert("limit".to_owned(), serde_json::json!(limit));
        }
        match self.task_id.as_str() {
            "code-to-record" => {
                anyhow::ensure!(self.context.is_none(), "code-to-record accepts --path, --focus and --limit; use context-to-record for --context");
                input.entry("path").or_insert(serde_json::json!("."));
                input.entry("limit").or_insert(serde_json::json!(5));
            }
            "context-to-record" => {
                anyhow::ensure!(
                    self.path.is_none() && self.focus.is_none() && self.limit.is_none(),
                    "context-to-record accepts --context"
                );
            }
            "decisions-to-spec" => {
                anyhow::ensure!(
                    self.path.is_none() && self.context.is_none() && self.limit.is_none(),
                    "decisions-to-spec accepts --focus"
                );
            }
            task_id => anyhow::ensure!(
                input.is_empty(),
                "`{task_id}` does not take --path, --context, --focus or --limit; give its inputs with --input-json and its instructions with --instructions"
            ),
        }
        Ok(serde_json::Value::Object(input))
    }

    /// The task context: `--instructions`, `--record` and `--attach`, plus a
    /// `path` input, which is also a file the agent reads. Each flag needs a
    /// task whose manifest has a use for it.
    pub fn task_context(
        &self,
        input: &serde_json::Value,
        requirements: &dadrock::tasks::TaskRequirements,
    ) -> anyhow::Result<dadrock::tasks::TaskContext> {
        use dadrock::tasks::{TaskNoteRequirement, TaskPathRequirement, TaskScopeRequirement};

        let task_id = &self.task_id;
        anyhow::ensure!(
            self.instructions.is_none() || requirements.notes != TaskNoteRequirement::None,
            "`{task_id}` does not take --instructions: its manifest has `notes: none`"
        );
        anyhow::ensure!(
            self.attach.is_empty() || requirements.paths != TaskPathRequirement::None,
            "`{task_id}` does not read files or folders, so it does not take --attach: its manifest has `paths: none`"
        );
        anyhow::ensure!(
            self.records.is_empty() || requirements.scope == TaskScopeRequirement::None,
            "`{task_id}` chooses the records it reads by scope, so it does not take --record"
        );

        let mut paths = Vec::new();
        let path_input = input.get("path").and_then(serde_json::Value::as_str);
        for path in path_input
            .into_iter()
            .chain(self.attach.iter().map(String::as_str))
        {
            if !paths.iter().any(|known| known == path) {
                paths.push(path.to_owned());
            }
        }
        Ok(dadrock::tasks::TaskContext {
            instructions: self.instructions.clone(),
            records: self.records.clone(),
            paths,
            ..Default::default()
        })
    }
}

#[derive(Debug, Args)]
pub struct TaskResultArgs {
    /// The run id, as `dadrock task runs` prints it.
    pub run_id: String,
    /// Print the whole JSON envelope instead of the result alone.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct PreviewArgs {
    /// The port to listen on; 0 picks a free one.
    #[arg(long, default_value_t = 4004)]
    pub port: u16,
    /// The address to listen on: only `localhost`, `127.0.0.1`, and `::1`
    /// are accepted.
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
    /// The directory to find the project from (defaults to the current
    /// directory).
    pub directory: Option<Utf8PathBuf>,
}

impl Default for PreviewArgs {
    fn default() -> Self {
        Self {
            port: 4004,
            host: "127.0.0.1".to_owned(),
            directory: None,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "kebab-case")]
pub enum PermissionPolicyArg {
    /// Deny every tool permission request.
    Deny,
    /// Approve read, search, and think tool requests once; deny the rest.
    AllowReads,
    /// Approve every tool permission request once.
    AllowOnce,
    /// Approve every tool permission request for the session.
    AllowAlways,
}

impl From<PermissionPolicyArg> for AcpPermissionPolicy {
    fn from(policy: PermissionPolicyArg) -> Self {
        match policy {
            PermissionPolicyArg::Deny => Self::Deny,
            PermissionPolicyArg::AllowReads => Self::AllowReads,
            PermissionPolicyArg::AllowOnce => Self::AllowOnce,
            PermissionPolicyArg::AllowAlways => Self::AllowAlways,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{error::ErrorKind, CommandFactory, Parser};

    use super::*;

    #[test]
    fn no_args_defaults_to_help() {
        let cli = Cli::try_parse_from(["dadrock"]).unwrap();

        let Command::Help(args) = cli.into_command_or_default() else {
            panic!("expected help command");
        };
        assert!(args.topic.is_empty());
    }

    #[test]
    fn parses_new_record_options_and_title() {
        let cli =
            Cli::try_parse_from(["dadrock", "new", "--tag", "cli", "--quiet", "Move to clap"])
                .unwrap();

        let Some(Command::New(args)) = cli.command else {
            panic!("expected new command");
        };
        assert_eq!(args.tags, ["cli"]);
        assert!(args.quiet);
        assert_eq!(args.title, ["Move to clap"]);
    }

    fn parse_new(args: &[&str]) -> NewArgs {
        let cli = Cli::try_parse_from(["dadrock", "new"].into_iter().chain(args.iter().copied()))
            .unwrap();
        let Some(Command::New(args)) = cli.command else {
            panic!("expected new command");
        };
        args
    }

    #[test]
    fn parses_new_options_after_the_title() {
        let args = parse_new(&["Move to clap", "--tag", "cli", "--quiet"]);
        assert_eq!(args.title, ["Move to clap"]);
        assert_eq!(args.tags, ["cli"]);
        assert!(args.quiet);

        let args = parse_new(&["Drop", "missing", "rows", "--tag", "kind/sample"]);
        assert_eq!(args.title, ["Drop", "missing", "rows"]);
        assert_eq!(args.tags, ["kind/sample"]);
        assert!(!args.quiet);
    }

    #[test]
    fn parses_new_options_on_both_sides_of_an_unquoted_title() {
        let args = parse_new(&[
            "--tag",
            "kind/sample",
            "Drop",
            "missing",
            "rows",
            "--tag",
            "writer/agent",
        ]);
        assert_eq!(args.title, ["Drop", "missing", "rows"]);
        assert_eq!(args.tags, ["kind/sample", "writer/agent"]);
    }

    #[test]
    fn new_title_starting_with_a_hyphen_needs_a_separator() {
        let args = parse_new(&["--tag", "kind/sample", "--", "-1", "codes", "missing"]);
        assert_eq!(args.title, ["-1", "codes", "missing"]);
        assert_eq!(args.tags, ["kind/sample"]);

        assert!(Cli::try_parse_from(["dadrock", "new", "-1", "codes", "missing"]).is_err());
        assert!(Cli::try_parse_from(["dadrock", "new", "--quiet"]).is_err());
    }

    #[test]
    fn parses_agent_draft_options_after_the_prompt() {
        let cli = Cli::try_parse_from([
            "dadrock", "agent", "draft", "Drop", "missing", "rows", "--write", "--json",
        ])
        .unwrap();
        let Some(Command::Agent(AgentArgs {
            command: AgentCommand::Draft(args),
        })) = cli.command
        else {
            panic!("expected agent draft command");
        };
        assert_eq!(args.prompt, ["Drop", "missing", "rows"]);
        assert!(args.write);
        assert!(args.json);

        let cli = Cli::try_parse_from([
            "dadrock",
            "agent",
            "draft",
            "--permission",
            "allow-once",
            "x",
            "--write",
        ])
        .unwrap();
        let Some(Command::Agent(AgentArgs {
            command: AgentCommand::Draft(args),
        })) = cli.command
        else {
            panic!("expected agent draft command");
        };
        assert_eq!(args.prompt, ["x"]);
        assert!(args.write);
        assert!(matches!(args.permission, PermissionPolicyArg::AllowOnce));
    }

    #[test]
    fn free_text_arguments_stop_at_help() {
        for args in [
            &["dadrock", "reject", "Use", "imputation", "--help"][..],
            &["dadrock", "status", "reject", "Use", "imputation", "--help"][..],
            &["dadrock", "status", "Use", "imputation", "--help"][..],
            &["dadrock", "agent", "draft", "Drop", "rows", "--help"][..],
            &["dadrock", "help", "validate", "--help"][..],
        ] {
            let error = Cli::try_parse_from(args).unwrap_err();
            assert_eq!(error.kind(), ErrorKind::DisplayHelp, "{args:?}");
        }
    }

    #[test]
    fn free_text_words_join_into_one_query() {
        let cli = Cli::try_parse_from(["dadrock", "reject", "Use", "imputation"]).unwrap();
        let Some(Command::Reject(args)) = cli.command else {
            panic!("expected reject command");
        };
        assert_eq!(args.joined(), "Use imputation");

        let cli =
            Cli::try_parse_from(["dadrock", "status", "reject", "Use", "imputation"]).unwrap();
        let Some(Command::Status(StatusArgs {
            command: Some(StatusCommand::Reject(args)),
            ..
        })) = cli.command
        else {
            panic!("expected status reject command");
        };
        assert_eq!(args.joined(), "Use imputation");

        let cli = Cli::try_parse_from(["dadrock", "status", "Use", "imputation"]).unwrap();
        let Some(Command::Status(StatusArgs {
            command: None,
            query,
        })) = cli.command
        else {
            panic!("expected status query");
        };
        assert_eq!(query, ["Use", "imputation"]);

        let cli = Cli::try_parse_from(["dadrock", "help", "validate", "changes"]).unwrap();
        let Some(Command::Help(args)) = cli.command else {
            panic!("expected help command");
        };
        assert_eq!(args.topic, ["validate", "changes"]);

        assert!(Cli::try_parse_from(["dadrock", "reject", "--", "-1", "codes"]).is_ok());
    }

    #[test]
    fn every_visible_argument_has_help() {
        fn collect_missing(command: &clap::Command, path: &str, missing: &mut Vec<String>) {
            for arg in command.get_arguments() {
                if !arg.is_hide_set() && arg.get_help().is_none() {
                    missing.push(format!("{path} {}", arg.get_id()));
                }
            }
            for subcommand in command.get_subcommands() {
                let path = format!("{path} {}", subcommand.get_name());
                // `task run` is skipped until its options get help text.
                if path != "dadrock task run" {
                    collect_missing(subcommand, &path, missing);
                }
            }
        }

        let mut missing = Vec::new();
        collect_missing(&Cli::command(), "dadrock", &mut missing);
        assert!(missing.is_empty(), "arguments without help: {missing:#?}");
    }

    #[test]
    fn help_names_the_accepted_preview_hosts() {
        let mut command = Cli::command();
        let preview = command
            .find_subcommand_mut("preview")
            .expect("preview command");
        let help = preview.render_help().to_string();
        assert!(
            help.contains("`localhost`, `127.0.0.1`, and `::1`"),
            "{help}"
        );
    }

    #[test]
    fn parses_nested_status_commands() {
        let cli = Cli::try_parse_from(["dadrock", "status", "accept", "candidate record"]).unwrap();

        let Some(Command::Status(StatusArgs {
            command: Some(StatusCommand::Accept(args)),
            ..
        })) = cli.command
        else {
            panic!("expected status accept command");
        };
        assert_eq!(args.queries(), ["candidate record"]);
    }

    #[test]
    fn accept_takes_more_candidates_with_repeated_with() {
        let cli = Cli::try_parse_from([
            "dadrock",
            "accept",
            "Use",
            "imputation",
            "--with",
            "Complete cases for the outcome",
            "--with",
            "0193b2a1d117",
        ])
        .unwrap();
        let Some(Command::Accept(args)) = cli.command else {
            panic!("expected accept command");
        };
        assert_eq!(
            args.queries(),
            [
                "Use imputation",
                "Complete cases for the outcome",
                "0193b2a1d117"
            ]
        );

        let cli = Cli::try_parse_from(["dadrock", "status", "accept", "B", "--with", "C"]).unwrap();
        let Some(Command::Status(StatusArgs {
            command: Some(StatusCommand::Accept(args)),
            ..
        })) = cli.command
        else {
            panic!("expected status accept command");
        };
        assert_eq!(args.queries(), ["B", "C"]);
    }

    #[test]
    fn parses_demo_example_option() {
        let cli =
            Cli::try_parse_from(["dadrock", "demo", "--example", "penguins", "example"]).unwrap();

        let Some(Command::Demo(args)) = cli.command else {
            panic!("expected demo command");
        };
        assert_eq!(args.example, DemoExampleArg::Penguins);
        assert_eq!(args.directory, Some(Utf8PathBuf::from("example")));
    }

    #[test]
    fn parses_all_demo_example_options() {
        for (raw, expected) in [
            ("lifecycle", DemoExampleArg::Lifecycle),
            ("candidate-review", DemoExampleArg::CandidateReview),
            ("penguins", DemoExampleArg::Penguins),
            ("broken", DemoExampleArg::Broken),
            ("conflicts", DemoExampleArg::Conflicts),
            ("task-history", DemoExampleArg::TaskHistory),
        ] {
            let cli = Cli::try_parse_from(["dadrock", "demo", "--example", raw]).unwrap();

            let Some(Command::Demo(args)) = cli.command else {
                panic!("expected demo command");
            };
            assert_eq!(args.example, expected);
        }
    }

    #[test]
    fn parses_validate_changes_options() {
        let cli = Cli::try_parse_from(["dadrock", "validate", "changes"]).unwrap();
        let Some(Command::Validate(ValidateArgs {
            command: Some(ValidateCommand::Changes(args)),
            ..
        })) = cli.command
        else {
            panic!("expected validate changes command");
        };
        assert_eq!(args.base, "HEAD");
        assert!(!args.staged);
        assert!(!args.strict);
        assert!(!args.json);

        let cli = Cli::try_parse_from([
            "dadrock",
            "validate",
            "changes",
            "--base",
            "origin/main",
            "--staged",
            "--strict",
            "--json",
        ])
        .unwrap();
        let Some(Command::Validate(ValidateArgs {
            command: Some(ValidateCommand::Changes(args)),
            ..
        })) = cli.command
        else {
            panic!("expected validate changes command");
        };
        assert_eq!(args.base, "origin/main");
        assert!(args.staged);
        assert!(args.strict);
        assert!(args.json);
    }

    #[test]
    fn parses_status_filter_values() {
        let cli = Cli::try_parse_from(["dadrock", "list", "--status", "superseded"]).unwrap();

        let Some(Command::List(args)) = cli.command else {
            panic!("expected list command");
        };
        assert_eq!(
            args.status.map(RecordViewFilter::from),
            Some(RecordViewFilter::Superseded)
        );
    }

    #[test]
    fn unimplemented_accepted_creation_is_not_advertised() {
        let error =
            Cli::try_parse_from(["dadrock", "new", "--accepted", "Already accepted"]).unwrap_err();
        assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    }

    #[test]
    fn parses_preview_options() {
        let cli = Cli::try_parse_from(["dadrock", "preview", "--port", "4100", "example"]).unwrap();

        let Some(Command::Preview(args)) = cli.command else {
            panic!("expected preview command");
        };
        assert_eq!(args.port, 4100);
        assert_eq!(args.host, "127.0.0.1");
        assert_eq!(args.directory, Some(Utf8PathBuf::from("example")));
    }

    #[test]
    fn parses_preview_host_override() {
        let cli = Cli::try_parse_from(["dadrock", "preview", "--host", "0.0.0.0"]).unwrap();

        let Some(Command::Preview(args)) = cli.command else {
            panic!("expected preview command");
        };
        assert_eq!(args.host, "0.0.0.0");
    }

    #[test]
    fn parses_history_query_with_flags_in_any_position() {
        let cli =
            Cli::try_parse_from(["dadrock", "history", "Listwise", "deletion", "--patch"]).unwrap();

        let Some(Command::History(args)) = cli.command else {
            panic!("expected history command");
        };
        assert_eq!(args.query, ["Listwise", "deletion"]);
        assert!(args.patch);
        assert!(!args.json);

        let cli = Cli::try_parse_from(["dadrock", "history", "--json", "01938c20a3b4"]).unwrap();
        let Some(Command::History(args)) = cli.command else {
            panic!("expected history command");
        };
        assert_eq!(args.query, ["01938c20a3b4"]);
        assert!(args.json);
        assert!(Cli::try_parse_from(["dadrock", "history"]).is_err());
    }

    #[test]
    fn parses_diff_old_and_new_queries() {
        let cli =
            Cli::try_parse_from(["dadrock", "diff", "old-record", "new-record", "--json"]).unwrap();

        let Some(Command::Diff(args)) = cli.command else {
            panic!("expected diff command");
        };
        assert_eq!(args.old, "old-record");
        assert_eq!(args.new, "new-record");
        assert!(args.json);
        assert!(Cli::try_parse_from(["dadrock", "diff", "only-one"]).is_err());
    }

    #[test]
    fn parses_mcp_serve_directory() {
        let cli = Cli::try_parse_from(["dadrock", "mcp", "serve", "example"]).unwrap();

        let Some(Command::Mcp(McpArgs {
            command: McpCommand::Serve(args),
        })) = cli.command
        else {
            panic!("expected mcp serve command");
        };
        assert!(!args.read_only);
        assert_eq!(args.directory, Some(Utf8PathBuf::from("example")));
    }

    #[test]
    fn parses_mcp_serve_read_only() {
        let cli =
            Cli::try_parse_from(["dadrock", "mcp", "serve", "--read-only", "example"]).unwrap();

        let Some(Command::Mcp(McpArgs {
            command: McpCommand::Serve(args),
        })) = cli.command
        else {
            panic!("expected mcp serve command");
        };
        assert!(args.read_only);
        assert!(!args.task_run);
        assert_eq!(args.directory, Some(Utf8PathBuf::from("example")));
    }

    #[test]
    fn parses_hidden_mcp_serve_task_run_flag() {
        let cli =
            Cli::try_parse_from(["dadrock", "mcp", "serve", "--task-run", "example"]).unwrap();

        let Some(Command::Mcp(McpArgs {
            command: McpCommand::Serve(args),
        })) = cli.command
        else {
            panic!("expected mcp serve command");
        };
        assert!(args.task_run);
        assert!(!args.read_only);
        assert_eq!(args.directory, Some(Utf8PathBuf::from("example")));

        let conflict =
            Cli::try_parse_from(["dadrock", "mcp", "serve", "--read-only", "--task-run"])
                .unwrap_err();
        assert_eq!(conflict.kind(), ErrorKind::ArgumentConflict);

        let mut command = Cli::command();
        let serve = command
            .find_subcommand_mut("mcp")
            .and_then(|mcp| mcp.find_subcommand_mut("serve"))
            .expect("mcp serve command");
        assert!(!serve.render_long_help().to_string().contains("--task-run"));
    }

    #[test]
    fn parses_agent_draft_skill_and_mcp_flags() {
        let cli = Cli::try_parse_from([
            "dadrock",
            "agent",
            "draft",
            "--skill",
            "dadr-style",
            "--prompt-pack",
            "./pack",
            "--no-mcp",
            "Draft this",
        ])
        .unwrap();

        let Some(Command::Agent(AgentArgs {
            command: AgentCommand::Draft(args),
        })) = cli.command
        else {
            panic!("expected agent draft command");
        };
        assert_eq!(args.skills, ["dadr-style", "./pack"]);
        assert!(args.no_mcp);
        assert_eq!(args.prompt, ["Draft this"]);
    }

    #[test]
    fn parses_agent_skills_json() {
        let cli = Cli::try_parse_from(["dadrock", "agent", "skills", "--json"]).unwrap();

        let Some(Command::Agent(AgentArgs {
            command: AgentCommand::Skills(args),
        })) = cli.command
        else {
            panic!("expected agent skills command");
        };
        assert!(args.json);
    }

    #[test]
    fn parses_agent_check_no_mcp() {
        let cli = Cli::try_parse_from(["dadrock", "agent", "check", "--no-mcp"]).unwrap();

        let Some(Command::Agent(AgentArgs {
            command: AgentCommand::Check(args),
        })) = cli.command
        else {
            panic!("expected agent check command");
        };
        assert!(args.no_mcp);
    }

    #[test]
    fn parses_task_list_json() {
        let cli = Cli::try_parse_from(["dadrock", "task", "list", "--json"]).unwrap();

        let Some(Command::Task(TaskArgs {
            command: TaskCommand::List(args),
        })) = cli.command
        else {
            panic!("expected task list command");
        };
        assert!(args.json);
    }

    #[test]
    fn parses_task_show() {
        let cli = Cli::try_parse_from(["dadrock", "task", "show", "find-conflicts"]).unwrap();

        let Some(Command::Task(TaskArgs {
            command: TaskCommand::Show(args),
        })) = cli.command
        else {
            panic!("expected task show command");
        };
        assert_eq!(args.task_id, "find-conflicts");
        assert!(!args.json);
        assert!(!args.raw);
    }

    #[test]
    fn parses_task_show_raw() {
        let cli =
            Cli::try_parse_from(["dadrock", "task", "show", "code-to-record", "--raw"]).unwrap();

        let Some(Command::Task(TaskArgs {
            command: TaskCommand::Show(args),
        })) = cli.command
        else {
            panic!("expected task show command");
        };
        assert_eq!(args.task_id, "code-to-record");
        assert!(args.raw);
        assert!(!args.json);
    }

    #[test]
    fn rejects_task_show_raw_with_json() {
        let error = Cli::try_parse_from([
            "dadrock",
            "task",
            "show",
            "code-to-record",
            "--raw",
            "--json",
        ])
        .unwrap_err();

        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }

    #[test]
    fn parses_task_run_with_input_json() {
        let cli = Cli::try_parse_from([
            "dadrock",
            "task",
            "run",
            "find-conflicts",
            "--input-json",
            r#"{"status_scope":"live"}"#,
            "--json",
            "--permission",
            "allow-once",
            "--agent",
            "codex",
            "--agent-arg",
            "--model=gpt-5",
            "--no-mcp",
        ])
        .unwrap();

        let Some(Command::Task(TaskArgs {
            command: TaskCommand::Run(args),
        })) = cli.command
        else {
            panic!("expected task run command");
        };
        assert_eq!(args.task_id, "find-conflicts");
        assert_eq!(
            args.input_json.as_deref(),
            Some(r#"{"status_scope":"live"}"#)
        );
        assert!(args.json);
        assert!(matches!(args.permission, PermissionPolicyArg::AllowOnce));
        assert_eq!(args.agent.as_deref(), Some("codex"));
        assert_eq!(args.agent_args, ["--model=gpt-5"]);
        assert!(args.no_mcp);
    }

    #[test]
    fn task_run_permission_defaults_to_deny_and_accepts_allow_reads() {
        let task_run = |extra: &[&str]| {
            let cli = Cli::try_parse_from(
                ["dadrock", "task", "run", "custom"]
                    .into_iter()
                    .chain(extra.iter().copied()),
            )
            .unwrap();
            let Some(Command::Task(TaskArgs {
                command: TaskCommand::Run(args),
            })) = cli.command
            else {
                panic!("expected task run command");
            };
            AcpPermissionPolicy::from(args.permission)
        };

        assert_eq!(task_run(&[]), AcpPermissionPolicy::Deny);
        assert_eq!(
            task_run(&["--permission", "allow-reads"]),
            AcpPermissionPolicy::AllowReads
        );
    }

    #[test]
    fn parses_task_runs_json() {
        let cli = Cli::try_parse_from(["dadrock", "task", "runs", "--json"]).unwrap();

        let Some(Command::Task(TaskArgs {
            command: TaskCommand::Runs(args),
        })) = cli.command
        else {
            panic!("expected task runs command");
        };
        assert!(args.json);
    }

    #[test]
    fn parses_task_result() {
        let cli = Cli::try_parse_from([
            "dadrock",
            "task",
            "result",
            "019abf91-8b0d-7000-8000-000000000001",
            "--json",
        ])
        .unwrap();

        let Some(Command::Task(TaskArgs {
            command: TaskCommand::Result(args),
        })) = cli.command
        else {
            panic!("expected task result command");
        };
        assert_eq!(args.run_id, "019abf91-8b0d-7000-8000-000000000001");
        assert!(args.json);
    }
}

#[cfg(test)]
mod simple_workflow_tests {
    use super::*;
    fn input(args: &[&str]) -> serde_json::Value {
        let cli = Cli::try_parse_from(args).unwrap();
        let Some(Command::Task(TaskArgs {
            command: TaskCommand::Run(args),
        })) = cli.command
        else {
            panic!("task run")
        };
        args.task_input().unwrap()
    }
    #[test]
    fn friendly_task_inputs_and_defaults() {
        assert_eq!(
            input(&["dadrock", "task", "run", "code-to-record"]),
            serde_json::json!({"path":".","limit":5})
        );
        assert_eq!(
            input(&[
                "dadrock",
                "task",
                "run",
                "code-to-record",
                "--path",
                "analysis/",
                "--limit",
                "12"
            ]),
            serde_json::json!({"path":"analysis/","limit":12})
        );
        assert_eq!(
            input(&[
                "dadrock",
                "task",
                "run",
                "context-to-record",
                "--context",
                "Exclude missing income"
            ]),
            serde_json::json!({"context":"Exclude missing income"})
        );
        assert_eq!(
            input(&[
                "dadrock",
                "task",
                "run",
                "decisions-to-spec",
                "--focus",
                "inference"
            ]),
            serde_json::json!({"focus":"inference"})
        );
    }
    #[test]
    fn rejects_ambiguous_and_zero_limit() {
        assert!(Cli::try_parse_from([
            "dadrock",
            "task",
            "run",
            "code-to-record",
            "--input-json",
            "{}",
            "--path",
            "."
        ])
        .is_err());
        assert!(
            Cli::try_parse_from(["dadrock", "task", "run", "code-to-record", "--limit", "0"])
                .is_err()
        );
    }
    fn task_run_args(args: &[&str]) -> TaskRunArgs {
        let cli = Cli::try_parse_from(args).unwrap();
        let Some(Command::Task(TaskArgs {
            command: TaskCommand::Run(args),
        })) = cli.command
        else {
            panic!("task run")
        };
        *args
    }

    /// The built-in task's manifest requirements, without user or project
    /// overrides.
    fn builtin_requirements(task_id: &str) -> dadrock::tasks::TaskRequirements {
        dadrock::tasks::discover_tasks(camino::Utf8Path::new("/nonexistent-dadrock-project"), None)
            .tasks
            .into_iter()
            .find(|task| task.manifest.id == task_id)
            .unwrap_or_else(|| panic!("built-in task {task_id}"))
            .manifest
            .requirements
    }

    fn task_context(args: &[&str]) -> anyhow::Result<dadrock::tasks::TaskContext> {
        let args = task_run_args(args);
        let input = args.task_input()?;
        args.task_context(&input, &builtin_requirements(&args.task_id))
    }

    #[test]
    fn custom_takes_instructions_records_and_attached_paths() {
        let context = task_context(&[
            "dadrock",
            "task",
            "run",
            "custom",
            "--instructions",
            "Summarize the sample restrictions",
            "--record",
            "0198abc",
            "--record",
            "Income cutoff",
            "--attach",
            "analysis/",
            "--attach",
            "plan.md",
        ])
        .unwrap();

        assert_eq!(
            context.instructions.as_deref(),
            Some("Summarize the sample restrictions")
        );
        assert_eq!(context.records, ["0198abc", "Income cutoff"]);
        assert_eq!(context.paths, ["analysis/", "plan.md"]);
    }

    #[test]
    fn context_flags_combine_with_input_json_and_friendly_flags() {
        let args = task_run_args(&[
            "dadrock",
            "task",
            "run",
            "edit-record",
            "--input-json",
            r#"{"record_id":"0198abc","requested_changes":"Tighten it"}"#,
            "--record",
            "0198abc",
            "--instructions",
            "Keep the tags",
        ]);
        let input = args.task_input().unwrap();
        let context = args
            .task_context(&input, &builtin_requirements("edit-record"))
            .unwrap();
        assert_eq!(input["record_id"], "0198abc");
        assert_eq!(context.records, ["0198abc"]);
        assert_eq!(context.instructions.as_deref(), Some("Keep the tags"));

        let context = task_context(&[
            "dadrock",
            "task",
            "run",
            "code-to-record",
            "--path",
            "analysis/",
            "--attach",
            "plan.md",
            "--attach",
            "analysis/",
        ])
        .unwrap();
        assert_eq!(context.paths, ["analysis/", "plan.md"]);
    }

    #[test]
    fn context_flags_are_rejected_where_the_manifest_has_no_use_for_them() {
        let attach = task_context(&[
            "dadrock",
            "task",
            "run",
            "merge-records",
            "--attach",
            "plan.md",
        ])
        .unwrap_err();
        assert!(
            attach.to_string().contains("does not take --attach"),
            "{attach}"
        );

        for task_id in ["find-conflicts", "decisions-to-spec"] {
            let record = task_context(&["dadrock", "task", "run", task_id, "--record", "0198abc"])
                .unwrap_err();
            assert!(
                record.to_string().contains("does not take --record"),
                "{record}"
            );
        }

        let args = task_run_args(&[
            "dadrock",
            "task",
            "run",
            "lab-report",
            "--instructions",
            "Go",
        ]);
        let instructions = args
            .task_context(
                &serde_json::json!({}),
                &dadrock::tasks::TaskRequirements::default(),
            )
            .unwrap_err();
        assert!(
            instructions
                .to_string()
                .contains("`lab-report` does not take --instructions"),
            "{instructions}"
        );
    }

    #[test]
    fn friendly_input_flags_point_other_tasks_to_input_json() {
        let error = task_run_args(&["dadrock", "task", "run", "custom", "--focus", "tags"])
            .task_input()
            .unwrap_err();
        assert!(error.to_string().contains("--input-json"), "{error}");
        assert!(error.to_string().contains("--instructions"), "{error}");
    }

    #[test]
    fn direct_lifecycle_commands_preserve_legacy_commands() {
        assert!(matches!(
            Cli::try_parse_from(["dadrock", "accept", "abc"])
                .unwrap()
                .command,
            Some(Command::Accept(_))
        ));
        assert!(matches!(
            Cli::try_parse_from(["dadrock", "reject", "abc"])
                .unwrap()
                .command,
            Some(Command::Reject(_))
        ));
        assert!(matches!(
            Cli::try_parse_from(["dadrock", "supersede", "--from", "abc", "--to", "def"])
                .unwrap()
                .command,
            Some(Command::Supersede(_))
        ));
        assert!(Cli::try_parse_from(["dadrock", "status", "accept", "abc"]).is_ok());
    }
}
