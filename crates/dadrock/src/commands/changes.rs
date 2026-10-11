use serde::{Deserialize, Serialize};

use crate::{
    changes::{check_record_changes, ChangeTarget, ChangeValidationReport, DEFAULT_BASE},
    error::DadrError,
};

use super::{CommandContext, CommandEnvelope};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidateChangesRequest {
    /// The Git revision to compare with, `HEAD` by default.
    #[serde(default = "default_base")]
    pub base: String,
    /// Compare the index (what the next commit holds) instead of the working
    /// tree.
    #[serde(default)]
    pub staged: bool,
}

impl Default for ValidateChangesRequest {
    fn default() -> Self {
        Self {
            base: default_base(),
            staged: false,
        }
    }
}

fn default_base() -> String {
    DEFAULT_BASE.to_owned()
}

/// Check that accepted and rejected records changed since `base` only where
/// the format allows: `tags`, keys outside DADR, and added `successors`.
pub fn validate_changes(
    context: &CommandContext,
    request: ValidateChangesRequest,
) -> Result<CommandEnvelope<ChangeValidationReport>, DadrError> {
    let project = context.discover_project()?;
    project.require_initialized()?;
    let target = if request.staged {
        ChangeTarget::Index
    } else {
        ChangeTarget::WorkingTree
    };
    let report = check_record_changes(&project, &request.base, target)?;
    let ok = !report.has_errors();
    Ok(CommandEnvelope::new("validate changes", report).with_ok(ok))
}

#[cfg(test)]
mod tests {
    use crate::commands::{find_command_info, CommandMutation};

    #[test]
    fn validate_changes_is_a_read_only_json_command() {
        let command = find_command_info(&["validate", "changes"]).unwrap();
        assert_eq!(command.mutation, CommandMutation::ReadOnly);
        assert!(command.supports_json);
        assert!(find_command_info(&["validate"])
            .unwrap()
            .subcommands
            .contains(&"validate changes"));
    }
}
