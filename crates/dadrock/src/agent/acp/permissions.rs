use std::{
    collections::BTreeMap,
    sync::{mpsc as std_mpsc, Arc, Mutex},
    time::Duration,
};

use agent_client_protocol::schema::{
    PermissionOptionKind, RequestPermissionOutcome, RequestPermissionRequest,
    SelectedPermissionOutcome,
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
    pub(super) post_run_error: Option<String>,
}

pub(super) fn non_interactive_permission_decision(
    policy: AcpPermissionPolicy,
    request: &RequestPermissionRequest,
) -> NonInteractivePermissionDecision {
    if let Some(option_id) = non_interactive_permission_option(policy, request) {
        return NonInteractivePermissionDecision {
            outcome: RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id)),
            post_run_error: None,
        };
    }

    let tool = permission_tool_label(request);
    let message = match policy {
        AcpPermissionPolicy::Deny => {
            format!(
                "ACP agent requested permission for {tool}, but did not offer a deny option; request was cancelled"
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
        post_run_error: Some(message),
    }
}

fn non_interactive_permission_option(
    policy: AcpPermissionPolicy,
    request: &RequestPermissionRequest,
) -> Option<String> {
    let wanted = match policy {
        AcpPermissionPolicy::Deny => &[
            PermissionOptionKind::RejectOnce,
            PermissionOptionKind::RejectAlways,
        ][..],
        AcpPermissionPolicy::AllowOnce => &[PermissionOptionKind::AllowOnce][..],
        AcpPermissionPolicy::AllowAlways => &[PermissionOptionKind::AllowAlways][..],
    };
    wanted.iter().find_map(|wanted| {
        request
            .options
            .iter()
            .find(|option| permission_option_kind_matches(&option.kind, wanted))
            .map(|option| option.option_id.to_string())
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
