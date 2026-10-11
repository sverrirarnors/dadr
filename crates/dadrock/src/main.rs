use std::process::ExitCode;

use anyhow::{bail, Context, Result};
use clap::{CommandFactory, Parser};
use dadrock::{
    agent::acp::{check_local_acp_agent_with_options, AcpHealthOptions},
    agent::providers::{
        configured_local_acp_agent, resolve_local_acp_agent_command, LocalAcpAgentCommand,
    },
    commands as core_commands,
    links::LinkCheckReport,
    mcp as core_mcp,
    project::ProjectLayout,
    tags::TagReport,
    tasks::{PersistedTaskRun, TaskContext, TaskListing, TaskPackage, TaskRunCommandRequest},
    validation::ProjectValidationReport,
    Severity,
};
use serde::Serialize;

mod cli;
mod web;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("dadrock: {err:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = cli::Cli::parse();
    dispatch_command(cli.into_command_or_default())
}

fn dispatch_command(command: cli::Command) -> Result<ExitCode> {
    match command {
        cli::Command::Help(args) => help(args),
        cli::Command::Commands(args) => commands(args),
        cli::Command::Completions(args) => completions(args),
        cli::Command::GenCliDocs => gen_cli_docs(),
        cli::Command::Dir => show_dir(),
        cli::Command::Config(args) => config(args),
        cli::Command::Init(args) => init(args),
        cli::Command::Demo(args) => demo(args),
        cli::Command::New(args) => new_record(args),
        cli::Command::List(args) => list(args),
        cli::Command::Status(args) => status(args),
        cli::Command::Accept(args) => status_accept(args),
        cli::Command::Reject(args) => status_reject(args),
        cli::Command::Supersede(args) => status_supersede(args),
        cli::Command::Validate(args) => validate(args),
        cli::Command::Tags(args) => tags(args),
        cli::Command::Mcp(args) => mcp(args),
        cli::Command::Agent(args) => agent(args),
        cli::Command::Task(args) => task(args),
        cli::Command::Preview(args) => preview(args),
    }
}

fn print_help() {
    println!("Dadrock\n\nUSAGE:\n    dadrock [COMMAND]\n\nWith no command, prints this help menu.");
    for command in core_commands::COMMANDS {
        println!("    dadrock {}", command.usage);
    }
    println!("\nDadrock stores project artifacts under dadr/.");
    println!("Use `dadrock help <command>` for Dadrock's command catalog metadata.");
}

fn help(args: cli::HelpArgs) -> Result<ExitCode> {
    if args.topic.is_empty() {
        print_help();
        return Ok(ExitCode::SUCCESS);
    }

    let path = args.topic.iter().map(String::as_str).collect::<Vec<_>>();
    let Some(command) = core_commands::find_command_info(&path) else {
        bail!("unknown help topic `{}`", args.topic.join(" "));
    };
    print_command_help(command);
    Ok(ExitCode::SUCCESS)
}

fn commands(args: cli::JsonArg) -> Result<ExitCode> {
    if args.json {
        let envelope = core_commands::command_catalog();
        print_json(&envelope)?;
    } else {
        print_command_catalog();
    }
    Ok(ExitCode::SUCCESS)
}

fn completions(args: cli::CompletionsArgs) -> Result<ExitCode> {
    let mut command = cli::Cli::command();
    let shell: clap_complete::Shell = args.shell.into();
    clap_complete::generate(shell, &mut command, "dadrock", &mut std::io::stdout());
    Ok(ExitCode::SUCCESS)
}

fn gen_cli_docs() -> Result<ExitCode> {
    print!("{}", render_cli_docs());
    Ok(ExitCode::SUCCESS)
}

fn render_cli_docs() -> String {
    clap_markdown::help_markdown::<cli::Cli>()
        .replace("[alias: `prompt-pack`]", r"\[alias: `prompt-pack`\]")
}

fn print_command_catalog() {
    let width = core_commands::COMMANDS
        .iter()
        .map(|command| format!("dadrock {}", command.usage).len())
        .max()
        .unwrap_or(0);

    println!("Dadrock commands\n");
    for command in core_commands::COMMANDS {
        let usage = format!("dadrock {}", command.usage);
        let mutation = command.mutation.label();
        let json = if command.supports_json {
            "json"
        } else {
            "text"
        };
        println!(
            "  {usage:<width$}  {mutation:<19}  {json:<4}  {}",
            command.description
        );
    }
    println!("\nUse `dadrock help <command>` for command details.");
}

fn print_command_help(command: &core_commands::CommandInfo) {
    println!("Dadrock command: dadrock {}\n", command.name());
    println!("USAGE:\n    dadrock {}\n", command.usage);
    println!("{}\n", command.description);
    println!("METADATA:");
    println!("    Mutation: {}", command.mutation.label());
    println!(
        "    JSON output: {}",
        if command.supports_json { "yes" } else { "no" }
    );
    println!("    Exit behavior: {}", command.exit_behavior);
    if !command.subcommands.is_empty() {
        println!("\nSUBCOMMANDS:");
        for subcommand in command.subcommands {
            println!("    dadrock {subcommand}");
        }
    }
}

fn command_context() -> Result<core_commands::CommandContext> {
    core_commands::CommandContext::current_dir().context("failed to create command context")
}

fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn envelope_exit_code<T>(envelope: &core_commands::CommandEnvelope<T>) -> ExitCode {
    if envelope.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn print_json_envelope<T: Serialize>(
    envelope: &core_commands::CommandEnvelope<T>,
) -> Result<ExitCode> {
    print_json(envelope)?;
    Ok(envelope_exit_code(envelope))
}

fn print_envelope<T: Serialize>(
    envelope: &core_commands::CommandEnvelope<T>,
    json: bool,
    print_text: impl FnOnce(&T) -> Result<()>,
) -> Result<ExitCode> {
    if json {
        print_json(envelope)?;
    } else {
        print_text(&envelope.data)?;
    }
    Ok(envelope_exit_code(envelope))
}

fn show_dir() -> Result<ExitCode> {
    let envelope = core_commands::show_dir(&command_context()?)?;
    println!("{}", envelope.data.decisions_dir);
    Ok(envelope_exit_code(&envelope))
}

fn config(args: cli::JsonArg) -> Result<ExitCode> {
    let envelope = core_commands::show_config(&command_context()?)?;
    print_envelope(&envelope, args.json, |layout| {
        print_project_layout(layout);
        Ok(())
    })
}

fn init(args: cli::InitArgs) -> Result<ExitCode> {
    let envelope = core_commands::initialize_project(&command_context()?, args.directory)?;
    print_json_envelope(&envelope)
}

fn demo(args: cli::DemoArgs) -> Result<ExitCode> {
    let envelope = core_commands::demo_project_with_example(
        &command_context()?,
        args.example.into(),
        args.directory,
    )?;
    print_envelope(&envelope, args.json, |result| {
        println!(
            "seeded {} DADR demo at {}",
            result.example.as_str(),
            result.decisions_dir
        );
        println!("{} record(s)", result.records.len());
        Ok(())
    })
}

fn new_record(args: cli::NewArgs) -> Result<ExitCode> {
    let title = args.title.join(" ");

    let envelope = core_commands::new_record(&command_context()?, &title, &args.tags)?;
    if args.quiet {
        println!("{}", envelope.data.id);
        return Ok(envelope_exit_code(&envelope));
    }
    print_json_envelope(&envelope)
}

fn list(args: cli::ListArgs) -> Result<ExitCode> {
    let envelope = core_commands::list_records_command(
        &command_context()?,
        core_commands::ListRecordsRequest {
            status: args.status.map(Into::into),
            tag: args.tag,
            query: args.query,
            unique: args.unique,
        },
    )?;
    print_json_envelope(&envelope)
}

fn status(args: cli::StatusArgs) -> Result<ExitCode> {
    match args.command {
        Some(cli::StatusCommand::Accept(query)) => return status_accept(query),
        Some(cli::StatusCommand::Reject(query)) => return status_reject(query),
        Some(cli::StatusCommand::Supersede(args)) => return status_supersede(args),
        None => {}
    }
    if args.query.is_empty() {
        bail!("status requires a query");
    }
    let query = args.query.join(" ");
    let envelope = core_commands::show_status(&command_context()?, &query)?;
    print_json_envelope(&envelope)
}

fn status_reject(args: cli::QueryArgs) -> Result<ExitCode> {
    let query = args.joined();
    let envelope = core_commands::reject_record(&command_context()?, &query)?;
    print_json_envelope(&envelope)
}

fn status_accept(args: cli::QueryArgs) -> Result<ExitCode> {
    let query = args.joined();
    let envelope = core_commands::accept_record(&command_context()?, &query)?;
    print_json_envelope(&envelope)
}

fn status_supersede(args: cli::StatusSupersedeArgs) -> Result<ExitCode> {
    let envelope = core_commands::supersede_records(
        &command_context()?,
        core_commands::SupersedeRecordsRequest {
            from: args.from,
            to: args.to,
        },
    )?;
    print_json_envelope(&envelope)
}

fn validate(args: cli::ValidateArgs) -> Result<ExitCode> {
    match args.command {
        Some(cli::ValidateCommand::Records(args)) => validate_records_cli(args),
        Some(cli::ValidateCommand::Links(args)) => validate_links_cli(args),
        Some(cli::ValidateCommand::Tags(args)) => validate_tags_cli(args),
        None => validate_project(args.json),
    }
}

fn validate_project(json: bool) -> Result<ExitCode> {
    let envelope = core_commands::validate_project(&command_context()?)?;
    print_envelope(&envelope, json, |report| {
        print_project_validation_report(report);
        Ok(())
    })
}

fn validate_records_cli(args: cli::ValidateRecordsArgs) -> Result<ExitCode> {
    let envelope = core_commands::validate_records(&command_context()?, args.paths)?;
    print_json_envelope(&envelope)
}

fn validate_links_cli(args: cli::JsonArg) -> Result<ExitCode> {
    let envelope = core_commands::validate_links(&command_context()?)?;
    print_envelope(&envelope, args.json, |report| {
        print_link_report(report);
        Ok(())
    })
}

fn validate_tags_cli(args: cli::JsonArg) -> Result<ExitCode> {
    let envelope = core_commands::validate_tags(&command_context()?)?;
    print_envelope(&envelope, args.json, |report| {
        print_tag_validation(report);
        Ok(())
    })
}

fn agent(args: cli::AgentArgs) -> Result<ExitCode> {
    match args.command {
        cli::AgentCommand::Check(args) => agent_check(args),
        cli::AgentCommand::Draft(args) => agent_draft(args),
        cli::AgentCommand::Skills(args) => agent_skills(args),
    }
}

fn agent_check(args: cli::AgentCheckArgs) -> Result<ExitCode> {
    let local_agent = local_acp_agent_command("agent check", args.agent, args.agent_args)?;
    let report = check_local_acp_agent_with_options(
        &local_agent,
        command_context()?.cwd.into_std_path_buf(),
        AcpHealthOptions {
            enable_dadrock_mcp: !args.no_mcp,
            ..AcpHealthOptions::default()
        },
    )?;
    println!("{}", report.message);
    Ok(ExitCode::SUCCESS)
}

fn agent_skills(args: cli::JsonArg) -> Result<ExitCode> {
    let envelope = core_commands::list_agent_skills(&command_context()?)?;
    print_envelope(&envelope, args.json, |result| {
        if result.skills.is_empty() {
            println!("No Dadrock agent skills found.");
        } else {
            for skill in &result.skills {
                println!("{}\t{}\t{}", skill.name, skill.sha256, skill.source);
            }
        }
        Ok(())
    })
}

fn agent_draft(args: cli::AgentDraftArgs) -> Result<ExitCode> {
    let prompt = args.prompt.join(" ");
    let local_agent = local_acp_agent_command("agent draft", args.agent, args.agent_args)?;
    let request = core_commands::AgentDraftCommandRequest {
        prompt,
        local_agent,
        permission_policy: args.permission.into(),
        prompt_packs: args.skills,
        enable_dadrock_mcp: !args.no_mcp,
        write: args.write,
    };
    let envelope = core_commands::draft_with_acp_agent(&command_context()?, request)?;

    print_envelope(&envelope, args.json, |result| {
        if let Some(candidate) = &result.candidate {
            println!("drafted candidate {}", candidate.id);
            println!("{}", candidate.path);
            if !result.response.summary.trim().is_empty() {
                println!("\n{}", result.response.summary.trim());
            }
        } else {
            println!("{}", result.response.summary.trim());
            println!("\n{}", result.response.body_markdown.trim());
        }
        Ok(())
    })
}

fn local_acp_agent_command(
    command_name: &str,
    agent_program: Option<String>,
    agent_args: Vec<String>,
) -> Result<LocalAcpAgentCommand> {
    if let Some(program) = agent_program {
        return resolve_local_acp_agent_command(program, agent_args, "--agent").with_context(
            || format!("{command_name} --agent must point to an executable ACP provider"),
        );
    }

    let mut command = configured_local_acp_agent().with_context(|| {
        format!(
            "{command_name} requires --agent <program>, DADROCK_ACP_AGENT, or an installed ACP provider"
        )
    })?;
    command.args.extend(agent_args);
    Ok(command)
}

fn task(args: cli::TaskArgs) -> Result<ExitCode> {
    match args.command {
        cli::TaskCommand::List(args) => task_list(args),
        cli::TaskCommand::Show(args) => task_show(args),
        cli::TaskCommand::Run(args) => task_run(args),
        cli::TaskCommand::Runs(args) => task_runs(args),
        cli::TaskCommand::Result(args) => task_result(args),
    }
}

fn task_list(args: cli::JsonArg) -> Result<ExitCode> {
    let envelope = core_commands::list_tasks_command(&command_context()?);
    print_envelope(&envelope, args.json, |listing| {
        print_task_list(listing);
        Ok(())
    })
}

fn task_show(args: cli::TaskShowArgs) -> Result<ExitCode> {
    let envelope = core_commands::show_task_command(&command_context()?, &args.task_id)?;
    print_envelope(&envelope, args.json, |task| {
        print_task_package(task);
        Ok(())
    })
}

fn task_run(args: cli::TaskRunArgs) -> Result<ExitCode> {
    let input = args.task_input()?;
    let mut context = TaskContext::default();
    if let Some(path) = input.get("path").and_then(serde_json::Value::as_str) {
        context.paths.push(path.to_owned());
    }
    let local_agent = if args.agent.is_some() || !args.agent_args.is_empty() {
        Some(local_acp_agent_command(
            "task run",
            args.agent,
            args.agent_args,
        )?)
    } else {
        None
    };
    let envelope = core_commands::run_task_command(
        &command_context()?,
        TaskRunCommandRequest {
            task_id: args.task_id,
            input,
            context,
            local_agent,
            permission_policy: args.permission.into(),
            enable_dadrock_mcp: !args.no_mcp,
        },
    )?;

    print_envelope(&envelope, args.json, |run| {
        print_task_run(run);
        Ok(())
    })
}

fn task_runs(args: cli::JsonArg) -> Result<ExitCode> {
    let envelope = core_commands::list_task_runs_command(&command_context()?)?;
    print_envelope(&envelope, args.json, |runs| {
        print_task_runs(runs);
        Ok(())
    })
}

fn task_result(args: cli::TaskResultArgs) -> Result<ExitCode> {
    let envelope = core_commands::show_task_result_command(&command_context()?, &args.run_id)?;
    print_envelope(&envelope, args.json, print_json)
}

fn tags(args: cli::TagsArgs) -> Result<ExitCode> {
    match args.command {
        cli::TagsCommand::List(args) => tags_list(args),
    }
}

fn tags_list(args: cli::JsonArg) -> Result<ExitCode> {
    let envelope = core_commands::list_tags(&command_context()?)?;
    print_envelope(&envelope, args.json, |report| {
        print_tag_list(report);
        Ok(())
    })
}

fn mcp(args: cli::McpArgs) -> Result<ExitCode> {
    match args.command {
        cli::McpCommand::Serve(args) => mcp_serve(args),
    }
}

fn mcp_serve(args: cli::McpServeArgs) -> Result<ExitCode> {
    let access = if args.read_only {
        core_mcp::DadrockMcpAccess::ReadOnly
    } else {
        core_mcp::DadrockMcpAccess::Full
    };
    let context = match args.directory {
        Some(directory) => core_commands::CommandContext::from_path(directory.as_std_path())
            .context("failed to create command context")?,
        None => command_context()?,
    };
    core_mcp::serve_stdio_with_access(context, access)?;
    Ok(ExitCode::SUCCESS)
}

fn preview(args: cli::PreviewArgs) -> Result<ExitCode> {
    web::serve(web::PreviewOptions {
        directory: args.directory,
        host: args.host,
        port: args.port,
    })?;
    Ok(ExitCode::SUCCESS)
}

fn print_project_layout(layout: &ProjectLayout) {
    println!("project_dir\t{}", layout.project_dir);
    println!("accepted_dir\t{}", layout.accepted_dir);
    println!("candidates_dir\t{}", layout.candidates_dir);
    println!("rejected_dir\t{}", layout.rejected_dir);
    println!("templates_dir\t{}", layout.templates_dir);
    println!("tasks_dir\t{}", layout.tasks_dir);
    println!("runs_dir\t{}", layout.runs_dir);
}

fn print_project_validation_report(report: &ProjectValidationReport) {
    if report.issues.is_empty() {
        println!("dadrock validate: ok");
        return;
    }

    println!(
        "dadrock validate: {} error(s), {} warning(s), {} info",
        report.counts.errors, report.counts.warnings, report.counts.infos
    );
    for issue in &report.issues {
        let location = issue
            .path
            .as_ref()
            .map_or_else(|| "project".to_owned(), ToString::to_string);
        let location = if let Some(line) = issue.line {
            format!("{location}:{line}")
        } else {
            location
        };
        println!(
            "[{}] {} {}: {}",
            severity_label(issue.severity),
            issue.code,
            location,
            issue.message
        );
        for related_path in &issue.related_paths {
            println!("  - {related_path}");
        }
    }
}

fn print_link_report(report: &LinkCheckReport) {
    if report.issues.is_empty() {
        println!("dadrock validate links: ok");
        return;
    }

    println!(
        "dadrock validate links: {} error(s), {} warning(s), {} info",
        report.counts.errors, report.counts.warnings, report.counts.infos
    );
    for issue in &report.issues {
        println!(
            "[{}] {} {}:{}: {}",
            severity_label(issue.severity),
            issue.code,
            issue.path,
            issue.line,
            issue.message
        );
        for related_path in &issue.related_paths {
            println!("  - {related_path}");
        }
    }
}

fn print_tag_list(report: &TagReport) {
    for tag in &report.tags {
        println!(
            "#{}\t{}\t{}",
            tag.display, tag.record_count, tag.occurrence_count
        );
    }
}

fn print_tag_validation(report: &TagReport) {
    if report.issues.is_empty() {
        println!("dadrock validate tags: ok");
        return;
    }

    println!(
        "dadrock validate tags: {} error(s), {} warning(s), {} info",
        report.counts.errors, report.counts.warnings, report.counts.infos
    );
    for issue in &report.issues {
        let location = issue.line.map_or_else(
            || issue.path.to_string(),
            |line| format!("{}:{line}", issue.path),
        );
        let tag = issue
            .tag
            .as_ref()
            .map_or_else(String::new, |tag| format!(" `{tag}`"));
        println!(
            "[{}] {} {}{}: {}",
            severity_label(issue.severity),
            issue.code,
            location,
            tag,
            issue.message
        );
    }
}

fn print_task_list(listing: &TaskListing) {
    for task in &listing.tasks {
        println!(
            "{}\tv{}\t{}\t{}",
            task.manifest.id,
            task.manifest.version,
            task.source.kind.as_str(),
            task.manifest.title
        );
    }
    for diagnostic in &listing.diagnostics {
        let path = diagnostic
            .path
            .as_ref()
            .map_or_else(|| "task source".to_owned(), ToString::to_string);
        eprintln!(
            "[{}] {}: {}",
            diagnostic.code.as_str(),
            path,
            diagnostic.message
        );
    }
}

fn print_task_package(task: &TaskPackage) {
    println!("{} v{}", task.manifest.id, task.manifest.version);
    println!("{}", task.manifest.title);
    if !task.manifest.description.trim().is_empty() {
        println!("{}", task.manifest.description.trim());
    }
    println!("source\t{}", task.source.id);
    println!("output\t{}", task.manifest.output.kind.as_str());
    println!("\n{}", task.prompt.trim());
}

fn print_task_run(run: &PersistedTaskRun) {
    println!("task run {}", run.id);
    println!("task\t{}", run.task_id);
    println!("status\t{}", run.status.as_str());
    println!("run_dir\t{}", run.run_dir);
}

fn print_task_runs(runs: &[PersistedTaskRun]) {
    if runs.is_empty() {
        println!("No task runs found.");
        return;
    }

    for run in runs {
        println!(
            "{}\t{}\t{}\t{}\t{}",
            run.id,
            run.status.as_str(),
            run.task_id,
            run.created_at.to_rfc3339(),
            run.run_dir
        );
    }
}

fn severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_cli_docs_escape_alias_brackets() {
        let docs = render_cli_docs();

        assert!(!docs.contains(" [alias:"));
        assert!(docs.contains(r"\[alias: `prompt-pack`\]"));
    }
}
