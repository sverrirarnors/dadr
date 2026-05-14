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
    #[command(about = "Print the discovered accepted records directory.")]
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
    #[command(about = "Inspect one record's lifecycle status or run lifecycle transitions.")]
    Status(StatusArgs),
    #[command(about = "Run project validation, or target records, links, or tags.")]
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
    #[arg(num_args = 0.., trailing_var_arg = true)]
    pub topic: Vec<String>,
}

#[derive(Debug, Args)]
pub struct JsonArg {
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct CompletionsArgs {
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
    pub directory: Option<Utf8PathBuf>,
}

#[derive(Debug, Args)]
pub struct DemoArgs {
    #[arg(long)]
    pub json: bool,
    #[arg(long, value_enum, default_value_t = DemoExampleArg::Lifecycle)]
    pub example: DemoExampleArg,
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
    #[arg(long = "tag")]
    pub tags: Vec<String>,
    #[arg(long)]
    pub quiet: bool,
    #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
    pub title: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long, value_enum)]
    pub status: Option<RecordStatusArg>,
    #[arg(long)]
    pub tag: Option<String>,
    #[arg(long)]
    pub query: Option<String>,
    #[arg(long)]
    pub unique: bool,
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
    #[arg(num_args = 0.., trailing_var_arg = true)]
    pub query: Vec<String>,
}

#[derive(Debug, Subcommand)]
pub enum StatusCommand {
    #[command(about = "Accept a candidate by converting it into an accepted decision.")]
    Accept(QueryArgs),
    #[command(about = "Reject a candidate and move it into the rejected records directory.")]
    Reject(QueryArgs),
    #[command(about = "Link accepted successors to the records they supersede.")]
    Supersede(StatusSupersedeArgs),
}

#[derive(Debug, Args)]
pub struct QueryArgs {
    #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
    pub query: Vec<String>,
}

impl QueryArgs {
    pub fn joined(&self) -> String {
        self.query.join(" ")
    }
}

#[derive(Debug, Args)]
pub struct StatusSupersedeArgs {
    #[arg(long = "from")]
    pub from: Vec<String>,
    #[arg(long = "to")]
    pub to: Vec<String>,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
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
}

#[derive(Debug, Args)]
pub struct ValidateRecordsArgs {
    #[arg(long)]
    pub json: bool,
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
    #[arg(long)]
    pub read_only: bool,
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
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long = "agent-arg", allow_hyphen_values = true)]
    pub agent_args: Vec<String>,
    #[arg(long = "no-mcp")]
    pub no_mcp: bool,
}

#[derive(Debug, Args)]
pub struct AgentDraftArgs {
    #[arg(long)]
    pub write: bool,
    #[arg(long)]
    pub json: bool,
    #[arg(long, value_enum, default_value_t = PermissionPolicyArg::Deny)]
    pub permission: PermissionPolicyArg,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long = "agent-arg", allow_hyphen_values = true)]
    pub agent_args: Vec<String>,
    #[arg(long = "skill", visible_alias = "prompt-pack")]
    pub skills: Vec<String>,
    #[arg(long = "no-mcp")]
    pub no_mcp: bool,
    #[arg(required = true, num_args = 1.., trailing_var_arg = true)]
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
    Run(TaskRunArgs),
    #[command(about = "List persisted local task runs.")]
    Runs(JsonArg),
    #[command(about = "Read a persisted task run result.")]
    Result(TaskResultArgs),
}

#[derive(Debug, Args)]
pub struct TaskShowArgs {
    pub task_id: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct TaskRunArgs {
    pub task_id: String,
    #[arg(long = "input-json")]
    pub input_json: String,
    #[arg(long)]
    pub json: bool,
    #[arg(long, value_enum, default_value_t = PermissionPolicyArg::Deny)]
    pub permission: PermissionPolicyArg,
    #[arg(long)]
    pub agent: Option<String>,
    #[arg(long = "agent-arg", allow_hyphen_values = true)]
    pub agent_args: Vec<String>,
    #[arg(long = "no-mcp")]
    pub no_mcp: bool,
}

#[derive(Debug, Args)]
pub struct TaskResultArgs {
    pub run_id: String,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct PreviewArgs {
    #[arg(long, default_value_t = 4004)]
    pub port: u16,
    #[arg(long, default_value = "127.0.0.1")]
    pub host: String,
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
    Deny,
    AllowOnce,
    AllowAlways,
}

impl From<PermissionPolicyArg> for AcpPermissionPolicy {
    fn from(policy: PermissionPolicyArg) -> Self {
        match policy {
            PermissionPolicyArg::Deny => Self::Deny,
            PermissionPolicyArg::AllowOnce => Self::AllowOnce,
            PermissionPolicyArg::AllowAlways => Self::AllowAlways,
        }
    }
}

#[cfg(test)]
mod tests {
    use clap::{error::ErrorKind, Parser};

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
        assert_eq!(args.joined(), "candidate record");
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
        assert_eq!(args.directory, Some(Utf8PathBuf::from("example")));
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
        assert_eq!(args.input_json, r#"{"status_scope":"live"}"#);
        assert!(args.json);
        assert!(matches!(args.permission, PermissionPolicyArg::AllowOnce));
        assert_eq!(args.agent.as_deref(), Some("codex"));
        assert_eq!(args.agent_args, ["--model=gpt-5"]);
        assert!(args.no_mcp);
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
