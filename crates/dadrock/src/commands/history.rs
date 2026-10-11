use crate::{
    error::DadrError,
    history::{diff_records, record_history, RecordDiffResult, RecordHistory},
};

use super::{CommandContext, CommandEnvelope};

/// `dadrock history`: the versions of the decision that contains the record
/// matching `query`, the links between them with their reasons, and the
/// neighbors. With `patch`, each link carries its diff.
pub fn history(
    context: &CommandContext,
    query: &str,
    patch: bool,
) -> Result<CommandEnvelope<RecordHistory>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let (history, diagnostics) = record_history(&index, query, patch)?;
    Ok(CommandEnvelope::new("history", history).with_diagnostics(diagnostics))
}

/// `dadrock diff`: compare any two records, from `old_query` to `new_query`.
pub fn diff(
    context: &CommandContext,
    old_query: &str,
    new_query: &str,
) -> Result<CommandEnvelope<RecordDiffResult>, DadrError> {
    let (_, index) = context.initialized_index()?;
    let result = diff_records(&index, old_query, new_query)?;
    Ok(CommandEnvelope::new("diff", result))
}

#[cfg(test)]
mod tests {
    use crate::commands::{find_command_info, CommandMutation};

    #[test]
    fn catalog_lists_history_and_diff_as_read_only_json_commands() {
        for path in [&["history"][..], &["diff"]] {
            let command = find_command_info(path)
                .unwrap_or_else(|| panic!("missing command metadata for {path:?}"));
            assert_eq!(command.mutation, CommandMutation::ReadOnly);
            assert!(command.supports_json);
        }
    }
}
