use std::{
    collections::BTreeMap,
    sync::{mpsc as std_mpsc, Arc, Mutex},
    time::Duration,
};

use agent_client_protocol::schema::{
    PermissionOption, PermissionOptionKind, RequestPermissionOutcome, RequestPermissionRequest,
    SelectedPermissionOutcome, ToolKind,
};

use super::{AcpPermissionOption, AcpPermissionOptionKind, AcpPermissionPolicy};
use crate::error::DadrError;

const ACP_PERMISSION_DECISION_TIMEOUT: Duration = Duration::from_secs(2 * 60);

pub(super) type AcpPermissionBridge = Arc<Mutex<AcpPermissionBridgeState>>;

#[derive(Debug, Default)]
pub(super) struct AcpPermissionBridgeState {
    next_request_id: u64,
    pending: BTreeMap<u64, std_mpsc::Sender<Option<String>>>,
}

pub(super) fn new_permission_bridge() -> AcpPermissionBridge {
    Arc::new(Mutex::new(AcpPermissionBridgeState::default()))
}

pub(super) fn select_permission_option(
    permissions: &AcpPermissionBridge,
    request_id: u64,
    option_id: impl Into<String>,
) -> Result<(), DadrError> {
    let sender = permissions
        .lock()
        .map_err(|_| DadrError::Acp("ACP permission state is unavailable".to_owned()))?
        .pending
        .remove(&request_id)
        .ok_or_else(|| DadrError::Acp("ACP permission request is no longer pending".to_owned()))?;
    sender
        .send(Some(option_id.into()))
        .map_err(|_| DadrError::Acp("ACP permission request is no longer pending".to_owned()))
}

pub(super) struct NonInteractivePermissionDecision {
    pub(super) outcome: RequestPermissionOutcome,
    /// What was decided, for the run log.
    pub(super) message: String,
    pub(super) post_run_error: Option<String>,
}

pub(super) fn non_interactive_permission_decision(
    policy: AcpPermissionPolicy,
    request: &RequestPermissionRequest,
) -> NonInteractivePermissionDecision {
    let tool = permission_tool_label(request);
    let kind = tool_kind_label(request.tool_call.fields.kind);
    if let Some(option) = non_interactive_permission_option(policy, request) {
        let verdict = match option.kind {
            PermissionOptionKind::AllowOnce => "Approved once",
            PermissionOptionKind::AllowAlways => "Approved for the session",
            _ => "Denied",
        };
        return NonInteractivePermissionDecision {
            outcome: RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                option.option_id.to_string(),
            )),
            message: format!("{verdict}: {tool} ({kind})."),
            post_run_error: None,
        };
    }

    let message = match policy {
        AcpPermissionPolicy::Deny => {
            format!(
                "ACP agent requested permission for {tool}, but did not offer a deny option; request was cancelled"
            )
        }
        AcpPermissionPolicy::AllowReads => {
            format!(
                "ACP agent requested permission for {tool} ({kind}), but did not offer the option the `allow-reads` policy needs; request was cancelled"
            )
        }
        AcpPermissionPolicy::AllowOnce | AcpPermissionPolicy::AllowAlways => {
            format!(
                "ACP agent requested permission for {tool}, but did not offer the configured `{}` option; request was cancelled",
                policy.as_str()
            )
        }
    };
    NonInteractivePermissionDecision {
        outcome: RequestPermissionOutcome::Cancelled,
        message: format!("Cancelled: {tool} ({kind})."),
        post_run_error: Some(message),
    }
}

/// Tool kinds the `allow-reads` policy approves: they look at the project or
/// reason about it, but change nothing and reach nothing outside it.
fn is_read_only_tool_kind(kind: Option<ToolKind>) -> bool {
    matches!(
        kind,
        Some(ToolKind::Read | ToolKind::Search | ToolKind::Think)
    )
}

fn tool_kind_label(kind: Option<ToolKind>) -> &'static str {
    match kind {
        None => "no tool kind",
        Some(ToolKind::Read) => "read",
        Some(ToolKind::Edit) => "edit",
        Some(ToolKind::Delete) => "delete",
        Some(ToolKind::Move) => "move",
        Some(ToolKind::Search) => "search",
        Some(ToolKind::Execute) => "execute",
        Some(ToolKind::Think) => "think",
        Some(ToolKind::Fetch) => "fetch",
        Some(ToolKind::SwitchMode) => "switch_mode",
        Some(_) => "other",
    }
}

const DENY_OPTIONS: &[PermissionOptionKind] = &[
    PermissionOptionKind::RejectOnce,
    PermissionOptionKind::RejectAlways,
];

fn non_interactive_permission_option(
    policy: AcpPermissionPolicy,
    request: &RequestPermissionRequest,
) -> Option<&PermissionOption> {
    let wanted = match policy {
        AcpPermissionPolicy::Deny => DENY_OPTIONS,
        // A read that cannot be approved once is denied rather than approved
        // for the whole session.
        AcpPermissionPolicy::AllowReads
            if is_read_only_tool_kind(request.tool_call.fields.kind) =>
        {
            &[
                PermissionOptionKind::AllowOnce,
                PermissionOptionKind::RejectOnce,
                PermissionOptionKind::RejectAlways,
            ][..]
        }
        AcpPermissionPolicy::AllowReads => DENY_OPTIONS,
        AcpPermissionPolicy::AllowOnce => &[PermissionOptionKind::AllowOnce][..],
        AcpPermissionPolicy::AllowAlways => &[PermissionOptionKind::AllowAlways][..],
    };
    wanted.iter().find_map(|wanted| {
        request
            .options
            .iter()
            .find(|option| permission_option_kind_matches(&option.kind, wanted))
    })
}

fn permission_option_kind_matches(
    left: &PermissionOptionKind,
    right: &PermissionOptionKind,
) -> bool {
    matches!(
        (left, right),
        (
            PermissionOptionKind::AllowOnce,
            PermissionOptionKind::AllowOnce
        ) | (
            PermissionOptionKind::AllowAlways,
            PermissionOptionKind::AllowAlways
        ) | (
            PermissionOptionKind::RejectOnce,
            PermissionOptionKind::RejectOnce
        ) | (
            PermissionOptionKind::RejectAlways,
            PermissionOptionKind::RejectAlways
        )
    )
}

pub(super) fn register_permission_request(
    permissions: &AcpPermissionBridge,
) -> Result<(u64, std_mpsc::Receiver<Option<String>>), ()> {
    let (sender, receiver) = std_mpsc::channel();
    let mut permissions = permissions.lock().map_err(|_| ())?;
    permissions.next_request_id = permissions.next_request_id.checked_add(1).ok_or(())?;
    let request_id = permissions.next_request_id;
    permissions.pending.insert(request_id, sender);
    Ok((request_id, receiver))
}

pub(super) async fn wait_for_permission_decision(
    request_id: u64,
    receiver: std_mpsc::Receiver<Option<String>>,
    permissions: AcpPermissionBridge,
) -> Result<Option<String>, agent_client_protocol::Error> {
    let decision = tokio::time::timeout(
        ACP_PERMISSION_DECISION_TIMEOUT,
        tokio::task::spawn_blocking(move || receiver.recv()),
    )
    .await;

    match decision {
        Ok(Ok(Ok(decision))) => Ok(decision),
        Ok(Ok(Err(_))) => Err(agent_client_protocol::Error::internal_error()
            .data("failed to read ACP permission response")),
        Ok(Err(_)) => Err(agent_client_protocol::Error::internal_error()
            .data("ACP permission response task failed")),
        Err(_) => {
            if let Ok(mut permissions) = permissions.lock() {
                permissions.pending.remove(&request_id);
            }
            Ok(None)
        }
    }
}

pub(super) fn cancel_pending_permissions(permissions: &AcpPermissionBridge) {
    let pending = permissions
        .lock()
        .map(|mut permissions| std::mem::take(&mut permissions.pending))
        .unwrap_or_default();
    for (_, sender) in pending {
        let _ = sender.send(None);
    }
}

pub(super) fn acp_permission_options_from_schema(
    request: &RequestPermissionRequest,
) -> Vec<AcpPermissionOption> {
    request
        .options
        .iter()
        .map(|option| AcpPermissionOption {
            id: option.option_id.to_string(),
            name: option.name.clone(),
            kind: match option.kind {
                PermissionOptionKind::AllowOnce => AcpPermissionOptionKind::AllowOnce,
                PermissionOptionKind::AllowAlways => AcpPermissionOptionKind::AllowAlways,
                PermissionOptionKind::RejectOnce => AcpPermissionOptionKind::RejectOnce,
                PermissionOptionKind::RejectAlways => AcpPermissionOptionKind::RejectAlways,
                _ => AcpPermissionOptionKind::Other,
            },
        })
        .collect()
}

pub(super) fn permission_request_message(request: &RequestPermissionRequest) -> String {
    let tool = permission_tool_label(request);
    let options = request
        .options
        .iter()
        .map(|option| option.name.as_str())
        .filter(|name| !name.trim().is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    if options.is_empty() {
        format!("ACP agent requests permission for {tool}.")
    } else {
        format!("ACP agent requests permission for {tool}. Options offered: {options}.")
    }
}

pub(super) fn permission_denied_message(request: &RequestPermissionRequest) -> String {
    let tool = permission_tool_label(request);
    let options = request
        .options
        .iter()
        .map(|option| option.name.as_str())
        .filter(|name| !name.trim().is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    if options.is_empty() {
        format!("Denied ACP tool permission request for {tool}.")
    } else {
        format!("Denied ACP tool permission request for {tool}. Options offered: {options}.")
    }
}

fn permission_tool_label(request: &RequestPermissionRequest) -> String {
    request
        .tool_call
        .fields
        .title
        .as_deref()
        .filter(|title| !title.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| "an unnamed tool".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(kind: Option<&str>, options: &[(&str, &str)]) -> RequestPermissionRequest {
        let mut tool_call = serde_json::json!({"toolCallId": "tool-1", "title": "Some tool"});
        if let Some(kind) = kind {
            tool_call["kind"] = serde_json::json!(kind);
        }
        let options = options
            .iter()
            .map(|(id, kind)| serde_json::json!({"optionId": id, "name": id, "kind": kind}))
            .collect::<Vec<_>>();
        serde_json::from_value(serde_json::json!({
            "sessionId": "session-1",
            "toolCall": tool_call,
            "options": options,
        }))
        .expect("permission request")
    }

    fn selected_option(decision: &NonInteractivePermissionDecision) -> Option<String> {
        match &decision.outcome {
            RequestPermissionOutcome::Selected(selected) => Some(selected.option_id.to_string()),
            _ => None,
        }
    }

    const ALL_OPTIONS: &[(&str, &str)] = &[
        ("allow-once", "allow_once"),
        ("allow-always", "allow_always"),
        ("reject-once", "reject_once"),
        ("reject-always", "reject_always"),
    ];

    #[test]
    fn allow_reads_approves_read_search_and_think_once() {
        for kind in ["read", "search", "think"] {
            let decision = non_interactive_permission_decision(
                AcpPermissionPolicy::AllowReads,
                &request(Some(kind), ALL_OPTIONS),
            );
            assert_eq!(selected_option(&decision).as_deref(), Some("allow-once"));
            assert!(decision.post_run_error.is_none());
        }
    }

    #[test]
    fn allow_reads_denies_every_other_kind_and_a_missing_kind() {
        for kind in [
            Some("edit"),
            Some("delete"),
            Some("move"),
            Some("execute"),
            Some("fetch"),
            Some("switch_mode"),
            Some("other"),
            Some("not-a-kind"),
            None,
        ] {
            let decision = non_interactive_permission_decision(
                AcpPermissionPolicy::AllowReads,
                &request(kind, ALL_OPTIONS),
            );
            assert_eq!(
                selected_option(&decision).as_deref(),
                Some("reject-once"),
                "{kind:?}"
            );
            assert!(decision.post_run_error.is_none());
        }
    }

    #[test]
    fn allow_reads_denies_a_read_it_cannot_approve_once() {
        let decision = non_interactive_permission_decision(
            AcpPermissionPolicy::AllowReads,
            &request(
                Some("read"),
                &[
                    ("allow-always", "allow_always"),
                    ("reject-always", "reject_always"),
                ],
            ),
        );
        assert_eq!(selected_option(&decision).as_deref(), Some("reject-always"));
        assert_eq!(decision.message, "Denied: Some tool (read).");
    }

    #[test]
    fn allow_reads_cancels_and_fails_when_no_deny_option_is_offered() {
        let decision = non_interactive_permission_decision(
            AcpPermissionPolicy::AllowReads,
            &request(Some("execute"), &[("allow-once", "allow_once")]),
        );
        assert!(matches!(
            decision.outcome,
            RequestPermissionOutcome::Cancelled
        ));
        let error = decision.post_run_error.expect("post-run error");
        assert!(error.contains("Some tool (execute)"), "{error}");
        assert!(error.contains("allow-reads"), "{error}");
    }
}
