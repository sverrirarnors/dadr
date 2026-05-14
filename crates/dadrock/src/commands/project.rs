use camino::Utf8PathBuf;

use crate::{
    demo::{seed_demo_project_with_example, DemoExample, DemoProjectResult},
    error::DadrError,
    project::{init_project, project_layout, InitResult, ProjectContext, ProjectLayout},
};

use super::{CommandContext, CommandEnvelope};

pub fn show_dir(context: &CommandContext) -> Result<CommandEnvelope<ProjectContext>, DadrError> {
    let project = context.discover_project()?;
    Ok(CommandEnvelope::new("dir", project))
}

pub fn show_config(context: &CommandContext) -> Result<CommandEnvelope<ProjectLayout>, DadrError> {
    let project = context.discover_project()?;
    Ok(CommandEnvelope::new("config", project_layout(&project)))
}

pub fn initialize_project(
    context: &CommandContext,
    target: Option<Utf8PathBuf>,
) -> Result<CommandEnvelope<InitResult>, DadrError> {
    let target = target.unwrap_or_else(|| context.cwd.clone());
    let result = init_project(target.as_std_path())?;
    let changed_paths = result.created.clone();
    Ok(CommandEnvelope::new("init", result).with_changed_paths(changed_paths))
}

pub fn demo_project(
    context: &CommandContext,
    target: Option<Utf8PathBuf>,
) -> Result<CommandEnvelope<DemoProjectResult>, DadrError> {
    demo_project_with_example(context, DemoExample::Lifecycle, target)
}

pub fn demo_project_with_example(
    context: &CommandContext,
    example: DemoExample,
    target: Option<Utf8PathBuf>,
) -> Result<CommandEnvelope<DemoProjectResult>, DadrError> {
    let target = target.unwrap_or_else(|| context.cwd.clone());
    let target = if target.is_absolute() {
        target
    } else {
        context.cwd.join(target)
    };
    let result = seed_demo_project_with_example(&target, example)?;
    let changed_paths = result.created_paths.clone();
    Ok(CommandEnvelope::new("demo", result).with_changed_paths(changed_paths))
}
