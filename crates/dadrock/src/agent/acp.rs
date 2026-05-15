use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc as std_mpsc, Arc, Mutex,
    },
    thread,
    time::Duration,
};

use agent_client_protocol::schema::{
    CancelNotification, InitializeRequest, NewSessionRequest, ProtocolVersion,
    RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionNotification, SessionUpdate, SetSessionConfigOptionRequest,
    SetSessionConfigOptionResponse, ToolCall, ToolCallId, ToolCallStatus, ToolCallUpdate,
};
use agent_client_protocol::util::MatchDispatch;
use agent_client_protocol::{
    ActiveSession, Agent, Client, Dispatch, SessionMessage, UntypedMessage,
};
use agent_client_protocol_tokio::AcpAgent;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc as tokio_mpsc;

use crate::{
    agent::{
        json::extract_json_object,
        prompts::agent_prompt,
        providers::LocalAcpAgentCommand,
        response::{
            looks_like_agent_response_value, parse_agent_response_text, parse_agent_response_value,
        },
        types::{AgentRequestEnvelope, AgentResponseEnvelope},
    },
    error::DadrError,
    mcp::{dadrock_mcp_server_config_with_access, DadrockMcpAccess},
};

mod config;
mod output;
mod permissions;

pub use self::config::{AcpAvailableCommand, AcpConfigCategory, AcpConfigOption, AcpConfigValue};

use self::{
    config::{acp_available_command_from_schema, acp_config_options_from_schema},
    output::{acp_output_limit_error, content_block_to_string, AcpOutputBuffer},
    permissions::{
        acp_permission_options_from_schema, cancel_pending_permissions, new_permission_bridge,
        non_interactive_permission_decision, permission_denied_message, permission_request_message,
        register_permission_request, select_permission_option, wait_for_permission_decision,
        AcpPermissionBridge,
    },
};

const ACP_STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const ACP_PROMPT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const ACP_CONFIG_RESPONSE_TIMEOUT: Duration = Duration::from_secs(30);
const ACP_ACTIVITY_TIMEOUT: Duration = Duration::from_secs(5 * 60);
const ACP_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
pub const ACP_OUTPUT_LIMIT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AcpPermissionPolicy {
    Deny,
    AllowOnce,
    AllowAlways,
}

impl AcpPermissionPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Deny => "deny",
            Self::AllowOnce => "allow-once",
            Self::AllowAlways => "allow-always",
        }
    }
}

impl Default for AcpPermissionPolicy {
    fn default() -> Self {
        Self::Deny
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpPromptOptions {
    pub permission_policy: AcpPermissionPolicy,
    pub output_limit_bytes: usize,
    pub enable_dadrock_mcp: bool,
    pub dadrock_mcp_access: DadrockMcpAccess,
}

impl Default for AcpPromptOptions {
    fn default() -> Self {
        Self {
            permission_policy: AcpPermissionPolicy::default(),
            output_limit_bytes: ACP_OUTPUT_LIMIT_BYTES,
            enable_dadrock_mcp: true,
            dadrock_mcp_access: DadrockMcpAccess::Full,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpConversationOptions {
    pub enable_dadrock_mcp: bool,
    pub dadrock_mcp_access: DadrockMcpAccess,
}

impl Default for AcpConversationOptions {
    fn default() -> Self {
        Self {
            enable_dadrock_mcp: true,
            dadrock_mcp_access: DadrockMcpAccess::Full,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpHealthReport {
    pub command_line: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpPromptEvent {
    Status { message: String },
    AgentMessageChunk { text: String },
    Permission { message: String },
}

pub type AcpPromptEventSink = Arc<dyn Fn(AcpPromptEvent) + Send + Sync>;

#[derive(Debug, Clone, Default)]
pub struct AcpCancellationToken {
    cancelled: Arc<AtomicBool>,
}

impl AcpCancellationToken {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpHealthOptions {
    pub enable_dadrock_mcp: bool,
    pub dadrock_mcp_access: DadrockMcpAccess,
}

impl Default for AcpHealthOptions {
    fn default() -> Self {
        Self {
            enable_dadrock_mcp: true,
            dadrock_mcp_access: DadrockMcpAccess::Full,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpSessionEvent {
    Ready { command_line: String },
    ConfigOptions(Vec<AcpConfigOption>),
    AvailableCommands(Vec<AcpAvailableCommand>),
    PermissionRequested(AcpPermissionRequest),
    PermissionDenied { message: String },
    Failed { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpPermissionRequest {
    pub request_id: u64,
    pub message: String,
    pub options: Vec<AcpPermissionOption>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpPermissionOption {
    pub id: String,
    pub name: String,
    pub kind: AcpPermissionOptionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpPermissionOptionKind {
    AllowOnce,
    AllowAlways,
    RejectOnce,
    RejectAlways,
    Other,
}

#[derive(Debug)]
pub struct AcpConversation {
    sender: Option<tokio_mpsc::UnboundedSender<AcpConversationCommand>>,
    permissions: AcpPermissionBridge,
    state: AcpConversationState,
    done: Option<std_mpsc::Receiver<()>>,
    handle: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct AcpConversationHandle {
    sender: tokio_mpsc::UnboundedSender<AcpConversationCommand>,
    permissions: AcpPermissionBridge,
    state: AcpConversationState,
}

type AcpConversationState = Arc<Mutex<AcpConversationStateData>>;

#[derive(Debug, Clone, PartialEq, Eq)]
enum AcpConversationLifecycle {
    Starting,
    Ready,
    Stopping,
    Closed,
    Failed(String),
}

#[derive(Debug)]
struct AcpConversationStateData {
    lifecycle: AcpConversationLifecycle,
}

impl Default for AcpConversationStateData {
    fn default() -> Self {
        Self {
            lifecycle: AcpConversationLifecycle::Starting,
        }
    }
}

#[derive(Debug)]
enum AcpConversationCommand {
    Prompt {
        prompt: String,
        response: std_mpsc::Sender<Result<String, String>>,
    },
    SetConfig {
        config_id: String,
        value: String,
        response: std_mpsc::Sender<Result<Vec<AcpConfigOption>, String>>,
    },
    Cancel,
    Shutdown,
}

#[derive(Debug, Default)]
struct AcpPromptOutput {
    text: String,
    raw_outputs: Vec<Value>,
}

impl AcpPromptOutput {
    fn parse_agent_response(self) -> Result<AgentResponseEnvelope, DadrError> {
        for raw_output in self.raw_outputs.into_iter().rev() {
            if looks_like_agent_response_value(&raw_output) {
                return parse_agent_response_value(raw_output);
            }
        }
        parse_agent_response_text(&self.text)
    }

    fn parse_json_response(self) -> Result<Value, DadrError> {
        for raw_output in self.raw_outputs.into_iter().rev() {
            if looks_like_structured_json_output(&raw_output) {
                return Ok(raw_output);
            }
        }
        parse_acp_agent_json_response(&self.text)
    }
}

fn looks_like_structured_json_output(value: &Value) -> bool {
    value
        .as_object()
        .and_then(|object| object.get("kind"))
        .and_then(Value::as_str)
        .is_some()
}

#[derive(Debug, Default)]
struct AcpToolCallState {
    calls: HashMap<ToolCallId, ToolCall>,
    completed_raw_outputs: Vec<Value>,
}

impl AcpToolCallState {
    fn apply_call(&mut self, call: ToolCall) {
        if call.status == ToolCallStatus::Completed {
            if let Some(raw_output) = call.raw_output.clone() {
                self.completed_raw_outputs.push(raw_output);
            }
        }
        self.calls.insert(call.tool_call_id.clone(), call);
    }

    fn apply_update(&mut self, update: ToolCallUpdate) {
        let tool_call_id = update.tool_call_id.clone();
        if let Some(call) = self.calls.get_mut(&tool_call_id) {
            call.update(update.fields);
            if call.status == ToolCallStatus::Completed {
                if let Some(raw_output) = call.raw_output.clone() {
                    self.completed_raw_outputs.push(raw_output);
                }
            }
            return;
        }

        if update.fields.status == Some(ToolCallStatus::Completed) {
            if let Some(raw_output) = update.fields.raw_output.clone() {
                self.completed_raw_outputs.push(raw_output);
            }
        }

        if let Ok(call) = ToolCall::try_from(update) {
            self.apply_call(call);
        }
    }
}

fn set_conversation_state(state: &AcpConversationState, lifecycle: AcpConversationLifecycle) {
    if let Ok(mut state) = state.lock() {
        state.lifecycle = lifecycle;
    }
}

fn conversation_state_error(state: &AcpConversationState) -> Result<(), DadrError> {
    let lifecycle = state
        .lock()
        .map_err(|_| DadrError::Acp("ACP conversation state is unavailable".to_owned()))?
        .lifecycle
        .clone();
    match lifecycle {
        AcpConversationLifecycle::Starting | AcpConversationLifecycle::Ready => Ok(()),
        AcpConversationLifecycle::Stopping => Err(DadrError::Acp(
            "ACP conversation is shutting down".to_owned(),
        )),
        AcpConversationLifecycle::Closed => {
            Err(DadrError::Acp("ACP conversation is closed".to_owned()))
        }
        AcpConversationLifecycle::Failed(message) => Err(DadrError::Acp(message)),
    }
}

impl AcpConversation {
    pub fn start(command: LocalAcpAgentCommand, cwd: PathBuf) -> Result<Self, DadrError> {
        Self::start_with_events(command, cwd, None)
    }

    pub fn start_with_events(
        command: LocalAcpAgentCommand,
        cwd: PathBuf,
        events: Option<std_mpsc::Sender<AcpSessionEvent>>,
    ) -> Result<Self, DadrError> {
        Self::start_with_events_and_options(command, cwd, events, AcpConversationOptions::default())
    }

    pub fn start_with_events_and_options(
        command: LocalAcpAgentCommand,
        cwd: PathBuf,
        events: Option<std_mpsc::Sender<AcpSessionEvent>>,
        options: AcpConversationOptions,
    ) -> Result<Self, DadrError> {
        if command.program.trim().is_empty() {
            return Err(DadrError::Acp(
                "local ACP agent command cannot be empty".to_owned(),
            ));
        }

        let (sender, receiver) = tokio_mpsc::unbounded_channel();
        let permissions = new_permission_bridge();
        let permissions_for_runtime = Arc::clone(&permissions);
        let state = Arc::new(Mutex::new(AcpConversationStateData::default()));
        let state_for_runtime = Arc::clone(&state);
        let state_for_thread = Arc::clone(&state);
        let (done_sender, done_receiver) = std_mpsc::channel();
        let handle = thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    let message = format!("failed to start ACP runtime: {err}");
                    set_conversation_state(
                        &state_for_thread,
                        AcpConversationLifecycle::Failed(message.clone()),
                    );
                    send_acp_event(events.as_ref(), AcpSessionEvent::Failed { message });
                    let _ = done_sender.send(());
                    return;
                }
            };
            if let Err(err) = runtime.block_on(run_acp_conversation_async(
                command,
                cwd,
                receiver,
                events.clone(),
                permissions_for_runtime,
                state_for_runtime,
                options,
            )) {
                let message = err.to_string();
                set_conversation_state(
                    &state_for_thread,
                    AcpConversationLifecycle::Failed(message.clone()),
                );
                send_acp_event(events.as_ref(), AcpSessionEvent::Failed { message });
            } else {
                set_conversation_state(&state_for_thread, AcpConversationLifecycle::Closed);
            }
            let _ = done_sender.send(());
        });

        Ok(Self {
            sender: Some(sender),
            permissions,
            state,
            done: Some(done_receiver),
            handle: Some(handle),
        })
    }

    pub fn prompt(
        &self,
        request: &AgentRequestEnvelope,
    ) -> Result<AgentResponseEnvelope, DadrError> {
        let sender = self
            .sender
            .as_ref()
            .ok_or_else(|| DadrError::Acp("ACP conversation is not running".to_owned()))?;
        AcpConversationHandle {
            sender: sender.clone(),
            permissions: Arc::clone(&self.permissions),
            state: Arc::clone(&self.state),
        }
        .prompt(request)
    }

    pub fn cancel(&self) -> Result<(), DadrError> {
        let sender = self
            .sender
            .as_ref()
            .ok_or_else(|| DadrError::Acp("ACP conversation is not running".to_owned()))?;
        AcpConversationHandle {
            sender: sender.clone(),
            permissions: Arc::clone(&self.permissions),
            state: Arc::clone(&self.state),
        }
        .cancel()
    }

    pub fn handle(&self) -> Result<AcpConversationHandle, DadrError> {
        let sender = self
            .sender
            .as_ref()
            .ok_or_else(|| DadrError::Acp("ACP conversation is not running".to_owned()))?;
        Ok(AcpConversationHandle {
            sender: sender.clone(),
            permissions: Arc::clone(&self.permissions),
            state: Arc::clone(&self.state),
        })
    }
}

impl AcpConversationHandle {
    pub fn prompt(
        &self,
        request: &AgentRequestEnvelope,
    ) -> Result<AgentResponseEnvelope, DadrError> {
        let prompt = agent_prompt(request)?;
        let text = self.prompt_text(prompt)?;
        parse_agent_response_text(&text)
    }

    pub fn prompt_text(&self, prompt: String) -> Result<String, DadrError> {
        conversation_state_error(&self.state)?;
        let (response_sender, response_receiver) = std_mpsc::channel();
        self.sender
            .send(AcpConversationCommand::Prompt {
                prompt,
                response: response_sender,
            })
            .map_err(|_| {
                conversation_state_error(&self.state)
                    .err()
                    .unwrap_or_else(|| DadrError::Acp("ACP conversation is not running".to_owned()))
            })?;
        let text = match response_receiver.recv_timeout(ACP_PROMPT_RESPONSE_TIMEOUT) {
            Ok(text) => text,
            Err(std_mpsc::RecvTimeoutError::Timeout) => {
                let _ = self.cancel();
                return Err(DadrError::Acp(format!(
                    "timed out after {} seconds waiting for ACP prompt response",
                    ACP_PROMPT_RESPONSE_TIMEOUT.as_secs()
                )));
            }
            Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                conversation_state_error(&self.state)?;
                return Err(DadrError::Acp(
                    "ACP conversation closed before responding".to_owned(),
                ));
            }
        };
        text.map_err(DadrError::Acp)
    }

    pub fn cancel(&self) -> Result<(), DadrError> {
        cancel_pending_permissions(&self.permissions);
        self.sender
            .send(AcpConversationCommand::Cancel)
            .map_err(|_| DadrError::Acp("ACP conversation is not running".to_owned()))
    }

    pub fn select_permission_option(
        &self,
        request_id: u64,
        option_id: impl Into<String>,
    ) -> Result<(), DadrError> {
        conversation_state_error(&self.state)?;
        select_permission_option(&self.permissions, request_id, option_id)
    }

    pub fn set_config_option(
        &self,
        config_id: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Vec<AcpConfigOption>, DadrError> {
        conversation_state_error(&self.state)?;
        let (response_sender, response_receiver) = std_mpsc::channel();
        self.sender
            .send(AcpConversationCommand::SetConfig {
                config_id: config_id.into(),
                value: value.into(),
                response: response_sender,
            })
            .map_err(|_| {
                conversation_state_error(&self.state)
                    .err()
                    .unwrap_or_else(|| DadrError::Acp("ACP conversation is not running".to_owned()))
            })?;
        let options = match response_receiver.recv_timeout(ACP_CONFIG_RESPONSE_TIMEOUT) {
            Ok(options) => options,
            Err(std_mpsc::RecvTimeoutError::Timeout) => {
                return Err(DadrError::Acp(format!(
                    "timed out after {} seconds waiting for ACP settings update",
                    ACP_CONFIG_RESPONSE_TIMEOUT.as_secs()
                )));
            }
            Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                conversation_state_error(&self.state)?;
                return Err(DadrError::Acp(
                    "ACP conversation closed before changing settings".to_owned(),
                ));
            }
        };
        options.map_err(DadrError::Acp)
    }
}

impl Drop for AcpConversation {
    fn drop(&mut self) {
        set_conversation_state(&self.state, AcpConversationLifecycle::Stopping);
        cancel_pending_permissions(&self.permissions);
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(AcpConversationCommand::Shutdown);
        }
        let shutdown_finished = self
            .done
            .take()
            .is_none_or(|done| done.recv_timeout(ACP_SHUTDOWN_TIMEOUT).is_ok());
        if let Some(handle) = self.handle.take() {
            finish_acp_thread(handle, shutdown_finished);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AcpThreadFinish {
    Joined,
    Reaping,
}

fn finish_acp_thread(handle: thread::JoinHandle<()>, shutdown_finished: bool) -> AcpThreadFinish {
    if shutdown_finished {
        let _ = handle.join();
        AcpThreadFinish::Joined
    } else {
        let _ = thread::Builder::new()
            .name("dadrock-acp-shutdown-reaper".to_owned())
            .spawn(move || {
                let _ = handle.join();
            });
        AcpThreadFinish::Reaping
    }
}

pub fn run_acp_agent(
    command: &LocalAcpAgentCommand,
    request: &AgentRequestEnvelope,
    options: AcpPromptOptions,
) -> Result<AgentResponseEnvelope, DadrError> {
    let prompt = agent_prompt(request)?;
    let output = run_acp_prompt(
        command,
        request.project.cwd.clone().into_std_path_buf(),
        &prompt,
        options,
        None,
        None,
    )?;
    output.parse_agent_response()
}

pub fn run_acp_agent_json(
    command: &LocalAcpAgentCommand,
    request: &AgentRequestEnvelope,
    options: AcpPromptOptions,
) -> Result<Value, DadrError> {
    run_acp_agent_json_with_events(command, request, options, None)
}

pub fn run_acp_agent_json_with_events(
    command: &LocalAcpAgentCommand,
    request: &AgentRequestEnvelope,
    options: AcpPromptOptions,
    events: Option<AcpPromptEventSink>,
) -> Result<Value, DadrError> {
    run_acp_agent_json_with_events_and_cancellation(command, request, options, events, None)
}

pub fn run_acp_agent_json_with_events_and_cancellation(
    command: &LocalAcpAgentCommand,
    request: &AgentRequestEnvelope,
    options: AcpPromptOptions,
    events: Option<AcpPromptEventSink>,
    cancellation: Option<AcpCancellationToken>,
) -> Result<Value, DadrError> {
    let prompt = agent_prompt(request)?;
    let output = run_acp_prompt(
        command,
        request.project.cwd.clone().into_std_path_buf(),
        &prompt,
        options,
        events,
        cancellation,
    )?;
    output.parse_json_response()
}

pub fn run_acp_prompt_json_with_events(
    command: &LocalAcpAgentCommand,
    cwd: PathBuf,
    prompt: String,
    options: AcpPromptOptions,
    events: Option<AcpPromptEventSink>,
) -> Result<Value, DadrError> {
    run_acp_prompt(command, cwd, &prompt, options, events, None)?.parse_json_response()
}

pub fn run_acp_prompt_text_with_events(
    command: &LocalAcpAgentCommand,
    cwd: PathBuf,
    prompt: String,
    options: AcpPromptOptions,
    events: Option<AcpPromptEventSink>,
) -> Result<String, DadrError> {
    Ok(run_acp_prompt(command, cwd, &prompt, options, events, None)?.text)
}

pub fn run_acp_agent_text_with_events(
    command: &LocalAcpAgentCommand,
    request: &AgentRequestEnvelope,
    options: AcpPromptOptions,
    events: Option<AcpPromptEventSink>,
) -> Result<String, DadrError> {
    run_acp_agent_text_with_events_and_cancellation(command, request, options, events, None)
}

pub fn run_acp_agent_text_with_events_and_cancellation(
    command: &LocalAcpAgentCommand,
    request: &AgentRequestEnvelope,
    options: AcpPromptOptions,
    events: Option<AcpPromptEventSink>,
    cancellation: Option<AcpCancellationToken>,
) -> Result<String, DadrError> {
    let prompt = agent_prompt(request)?;
    Ok(run_acp_prompt(
        command,
        request.project.cwd.clone().into_std_path_buf(),
        &prompt,
        options,
        events,
        cancellation,
    )?
    .text)
}

pub fn parse_acp_agent_json_response(text: &str) -> Result<Value, DadrError> {
    let value = match serde_json::from_str::<Value>(text) {
        Ok(value) => value,
        Err(strict_error) => {
            let Some(json) = extract_json_object(text, |_| true) else {
                return Err(DadrError::Acp(format!(
                    "ACP agent returned invalid JSON object response: {strict_error}"
                )));
            };
            serde_json::from_str::<Value>(json).map_err(|err| {
                DadrError::Acp(format!(
                    "ACP agent returned invalid JSON object response: {err}"
                ))
            })?
        }
    };
    match value {
        Value::Object(_) => Ok(value),
        Value::Array(_) => Err(DadrError::Acp(
            "ACP agent returned a JSON array; expected a JSON object".to_owned(),
        )),
        Value::String(_) => Err(DadrError::Acp(
            "ACP agent returned a JSON string; expected a JSON object".to_owned(),
        )),
        Value::Number(_) => Err(DadrError::Acp(
            "ACP agent returned a JSON number; expected a JSON object".to_owned(),
        )),
        Value::Bool(_) => Err(DadrError::Acp(
            "ACP agent returned a JSON boolean; expected a JSON object".to_owned(),
        )),
        Value::Null => Err(DadrError::Acp(
            "ACP agent returned JSON null; expected a JSON object".to_owned(),
        )),
    }
}

pub fn check_local_acp_agent(
    command: &LocalAcpAgentCommand,
    cwd: PathBuf,
) -> Result<AcpHealthReport, DadrError> {
    check_local_acp_agent_with_options(command, cwd, AcpHealthOptions::default())
}

pub fn check_local_acp_agent_with_options(
    command: &LocalAcpAgentCommand,
    cwd: PathBuf,
    options: AcpHealthOptions,
) -> Result<AcpHealthReport, DadrError> {
    if command.program.trim().is_empty() {
        return Err(DadrError::Acp(
            "local ACP agent command cannot be empty".to_owned(),
        ));
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| DadrError::Acp(format!("failed to start ACP runtime: {err}")))?;
    runtime.block_on(check_local_acp_agent_async(command.clone(), cwd, options))
}

fn run_acp_prompt(
    command: &LocalAcpAgentCommand,
    cwd: PathBuf,
    prompt: &str,
    options: AcpPromptOptions,
    events: Option<AcpPromptEventSink>,
    cancellation: Option<AcpCancellationToken>,
) -> Result<AcpPromptOutput, DadrError> {
    if command.program.trim().is_empty() {
        return Err(DadrError::Acp(
            "local ACP agent command cannot be empty".to_owned(),
        ));
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| DadrError::Acp(format!("failed to start ACP runtime: {err}")))?;
    runtime.block_on(run_acp_prompt_async(
        command.clone(),
        cwd,
        prompt.to_owned(),
        options,
        events,
        cancellation,
    ))
}

fn new_session_request(
    cwd: PathBuf,
    dadrock_mcp_access: Option<DadrockMcpAccess>,
) -> Result<NewSessionRequest, DadrError> {
    let mut request = NewSessionRequest::new(cwd.clone());
    if let Some(access) = dadrock_mcp_access {
        let utf8_cwd = camino::Utf8PathBuf::from_path_buf(cwd).map_err(|path| {
            DadrError::Mcp(format!(
                "failed to attach Dadrock MCP server for non-UTF-8 cwd `{}`",
                path.display()
            ))
        })?;
        request = request.mcp_servers(vec![dadrock_mcp_server_config_with_access(
            &utf8_cwd, access,
        )?]);
    }
    Ok(request)
}

fn dadrock_mcp_access_option(
    enable_dadrock_mcp: bool,
    access: DadrockMcpAccess,
) -> Option<DadrockMcpAccess> {
    enable_dadrock_mcp.then_some(access)
}

fn dadr_to_acp(error: DadrError) -> agent_client_protocol::Error {
    agent_client_protocol::Error::internal_error().data(error.to_string())
}

fn acp_agent_from_command(command: LocalAcpAgentCommand) -> Result<AcpAgent, DadrError> {
    let mut args = Vec::with_capacity(command.args.len() + 1);
    args.push(command.program);
    args.extend(command.args);
    AcpAgent::from_args(args)
        .map_err(|err| DadrError::Acp(format!("invalid ACP agent command: {err}")))
}

async fn check_local_acp_agent_async(
    command: LocalAcpAgentCommand,
    cwd: PathBuf,
    options: AcpHealthOptions,
) -> Result<AcpHealthReport, DadrError> {
    let command_line = command.command_line();
    let agent = acp_agent_from_command(command)?;

    let health_check = Client.connect_with(
        agent,
        |cx: agent_client_protocol::ConnectionTo<Agent>| async move {
            let initialize = cx
                .send_request(InitializeRequest::new(ProtocolVersion::V1))
                .block_task()
                .await?;
            let session_request = new_session_request(
                cwd,
                dadrock_mcp_access_option(options.enable_dadrock_mcp, options.dadrock_mcp_access),
            )
            .map_err(dadr_to_acp)?;
            let session = cx
                .build_session_from(session_request)
                .block_task()
                .start_session()
                .await?;
            let agent_label = initialize
                .agent_info
                .as_ref()
                .map(|info| {
                    info.title
                        .clone()
                        .unwrap_or_else(|| info.name.clone())
                        .trim()
                        .to_owned()
                })
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| "ACP agent".to_owned());
            let message = format!(
                "{agent_label} responded; session {} started with {}",
                session.session_id(),
                command_line
            );
            Ok(AcpHealthReport {
                command_line,
                message,
            })
        },
    );

    tokio::time::timeout(ACP_STARTUP_TIMEOUT, health_check)
        .await
        .map_err(|_| {
            DadrError::Acp(format!(
                "ACP health check timed out after {} seconds",
                ACP_STARTUP_TIMEOUT.as_secs()
            ))
        })?
        .map_err(|err| DadrError::Acp(format!("ACP health check failed: {err}")))
}

async fn run_acp_prompt_async(
    command: LocalAcpAgentCommand,
    cwd: PathBuf,
    prompt: String,
    options: AcpPromptOptions,
    events: Option<AcpPromptEventSink>,
    cancellation: Option<AcpCancellationToken>,
) -> Result<AcpPromptOutput, DadrError> {
    validate_prompt_options(options)?;
    let command_line = command.command_line();
    send_acp_prompt_event(
        events.as_ref(),
        AcpPromptEvent::Status {
            message: format!("Starting local ACP agent: {command_line}"),
        },
    );
    let agent = acp_agent_from_command(command)?;
    let output = Arc::new(Mutex::new(AcpOutputBuffer::new(options.output_limit_bytes)));
    let output_for_handler = Arc::clone(&output);
    let tool_calls = Arc::new(Mutex::new(AcpToolCallState::default()));
    let tool_calls_for_handler = Arc::clone(&tool_calls);
    let permission_error = Arc::new(Mutex::new(None::<String>));
    let permission_error_for_handler = Arc::clone(&permission_error);
    let events_for_handler = events.clone();

    let prompt_run =
        Client.connect_with(agent, |cx: agent_client_protocol::ConnectionTo<Agent>| {
            let output = Arc::clone(&output_for_handler);
            let tool_calls = Arc::clone(&tool_calls_for_handler);
            let permission_error = Arc::clone(&permission_error_for_handler);
            let events = events_for_handler.clone();
            async move {
                cx.send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task()
                    .await?;

                let session_request = new_session_request(
                    cwd,
                    dadrock_mcp_access_option(
                        options.enable_dadrock_mcp,
                        options.dadrock_mcp_access,
                    ),
                )
                .map_err(dadr_to_acp)?;
                let mut session = cx
                    .build_session_from(session_request)
                    .block_task()
                    .start_session()
                    .await?;
                send_acp_prompt_event(
                    events.as_ref(),
                    AcpPromptEvent::Status {
                        message: "ACP session started.".to_owned(),
                    },
                );

                session.send_prompt(prompt)?;
                send_acp_prompt_event(
                    events.as_ref(),
                    AcpPromptEvent::Status {
                        message: "Prompt sent; waiting for agent output.".to_owned(),
                    },
                );

                loop {
                    if cancellation
                        .as_ref()
                        .is_some_and(|token| token.is_cancelled())
                    {
                        send_acp_prompt_event(
                            events.as_ref(),
                            AcpPromptEvent::Status {
                                message: "Cancellation requested; stopping ACP session.".to_owned(),
                            },
                        );
                        cancel_session(&session)?;
                        return Err(agent_client_protocol::Error::internal_error()
                            .data("ACP prompt cancelled"));
                    }
                    let update = tokio::time::timeout(ACP_ACTIVITY_TIMEOUT, session.read_update())
                        .await
                        .map_err(|_| {
                            agent_client_protocol::Error::internal_error()
                                .data("timed out waiting for ACP session update")
                        })?;
                    match update? {
                        SessionMessage::SessionMessage(message) => {
                            let permission_error_for_request = Arc::clone(&permission_error);
                            MatchDispatch::new(message)
                                .if_notification(async |notification: SessionNotification| {
                                    match notification.update {
                                        SessionUpdate::AgentMessageChunk(content_chunk) => {
                                            let text =
                                                content_block_to_string(&content_chunk.content);
                                            let mut output = output.lock().map_err(|_| {
                                                agent_client_protocol::Error::internal_error()
                                            })?;
                                            output
                                                .push_str(&text)
                                                .map_err(acp_output_limit_error)?;
                                            if !text.is_empty() {
                                                send_acp_prompt_event(
                                                    events.as_ref(),
                                                    AcpPromptEvent::AgentMessageChunk { text },
                                                );
                                            }
                                        }
                                        SessionUpdate::ToolCall(call) => {
                                            let mut tool_calls =
                                                tool_calls.lock().map_err(|_| {
                                                    agent_client_protocol::Error::internal_error()
                                                })?;
                                            tool_calls.apply_call(call);
                                        }
                                        SessionUpdate::ToolCallUpdate(update) => {
                                            let mut tool_calls =
                                                tool_calls.lock().map_err(|_| {
                                                    agent_client_protocol::Error::internal_error()
                                                })?;
                                            tool_calls.apply_update(update);
                                        }
                                        SessionUpdate::UserMessageChunk(_)
                                        | SessionUpdate::AgentThoughtChunk(_)
                                        | SessionUpdate::Plan(_)
                                        | SessionUpdate::AvailableCommandsUpdate(_)
                                        | SessionUpdate::CurrentModeUpdate(_)
                                        | SessionUpdate::ConfigOptionUpdate(_)
                                        | SessionUpdate::SessionInfoUpdate(_)
                                        | SessionUpdate::UsageUpdate(_) => {}
                                        _ => {}
                                    }
                                    Ok(())
                                })
                                .await
                                .if_request(
                                    async move |request: RequestPermissionRequest, responder| {
                                        let decision = non_interactive_permission_decision(
                                            options.permission_policy,
                                            &request,
                                        );
                                        if let Some(message) = decision.post_run_error {
                                            if let Ok(mut permission_error) =
                                                permission_error_for_request.lock()
                                            {
                                                *permission_error = Some(message);
                                            }
                                        }
                                        responder.respond(RequestPermissionResponse::new(
                                            decision.outcome,
                                        ))?;
                                        Ok(())
                                    },
                                )
                                .await
                                .otherwise(async |_msg| Ok(()))
                                .await?;
                        }
                        SessionMessage::StopReason(_) => {
                            send_acp_prompt_event(
                                events.as_ref(),
                                AcpPromptEvent::Status {
                                    message: "Agent finished.".to_owned(),
                                },
                            );
                            break;
                        }
                        _ => {}
                    }
                }

                Ok(())
            }
        });

    tokio::time::timeout(ACP_PROMPT_RESPONSE_TIMEOUT, prompt_run)
        .await
        .map_err(|_| {
            DadrError::Acp(format!(
                "ACP agent prompt timed out after {} seconds",
                ACP_PROMPT_RESPONSE_TIMEOUT.as_secs()
            ))
        })?
        .map_err(|err| DadrError::Acp(format!("ACP agent prompt failed: {err}")))?;

    let permission_error = permission_error
        .lock()
        .map_err(|_| DadrError::Acp("failed to read ACP permission state".to_owned()))?
        .clone();
    if let Some(message) = permission_error {
        send_acp_prompt_event(
            events.as_ref(),
            AcpPromptEvent::Permission {
                message: message.clone(),
            },
        );
        return Err(DadrError::Acp(message));
    }

    let output = output
        .lock()
        .map_err(|_| DadrError::Acp("failed to read ACP response buffer".to_owned()))?;
    let tool_calls = tool_calls
        .lock()
        .map_err(|_| DadrError::Acp("failed to read ACP tool call state".to_owned()))?;
    Ok(AcpPromptOutput {
        text: output.as_str().to_owned(),
        raw_outputs: tool_calls.completed_raw_outputs.clone(),
    })
}

fn send_acp_prompt_event(events: Option<&AcpPromptEventSink>, event: AcpPromptEvent) {
    if let Some(events) = events {
        events(event);
    }
}

async fn run_acp_conversation_async(
    command: LocalAcpAgentCommand,
    cwd: PathBuf,
    mut receiver: tokio_mpsc::UnboundedReceiver<AcpConversationCommand>,
    events: Option<std_mpsc::Sender<AcpSessionEvent>>,
    permissions: AcpPermissionBridge,
    state: AcpConversationState,
    options: AcpConversationOptions,
) -> Result<(), DadrError> {
    let command_line = command.command_line();
    let agent = acp_agent_from_command(command)?;

    let conversation = Client.connect_with(
        agent,
        |cx: agent_client_protocol::ConnectionTo<Agent>| async move {
            cx.send_request(InitializeRequest::new(ProtocolVersion::V1))
                .block_task()
                .await?;

            let session_request = new_session_request(
                cwd,
                dadrock_mcp_access_option(options.enable_dadrock_mcp, options.dadrock_mcp_access),
            )
            .map_err(dadr_to_acp)?;
            let response = cx
                .send_request_to(Agent, session_request)
                .block_task()
                .await?;
            send_acp_event(
                events.as_ref(),
                AcpSessionEvent::ConfigOptions(acp_config_options_from_schema(
                    response.config_options.clone().unwrap_or_default(),
                )),
            );
            let mut session = cx.attach_session(response, Vec::new())?;
            set_conversation_state(&state, AcpConversationLifecycle::Ready);
            send_acp_event(events.as_ref(), AcpSessionEvent::Ready { command_line });

            while let Some(command) = receiver.recv().await {
                match command {
                    AcpConversationCommand::Prompt { prompt, response } => {
                        let result = send_prompt_and_collect(
                            &mut session,
                            prompt,
                            &mut receiver,
                            events.as_ref(),
                            &permissions,
                        )
                        .await;
                        let _ = response.send(result.map_err(|err| err.to_string()));
                    }
                    AcpConversationCommand::SetConfig {
                        config_id,
                        value,
                        response,
                    } => {
                        let result = tokio::time::timeout(
                            ACP_CONFIG_RESPONSE_TIMEOUT,
                            set_session_config_option(&session, config_id, value, events.as_ref()),
                        )
                        .await
                        .map_err(|_| {
                            agent_client_protocol::Error::internal_error()
                                .data("timed out waiting for ACP settings update")
                        })
                        .and_then(|result| result);
                        let _ = response.send(result.map_err(|err| err.to_string()));
                    }
                    AcpConversationCommand::Cancel => {
                        cancel_session(&session)?;
                    }
                    AcpConversationCommand::Shutdown => {
                        set_conversation_state(&state, AcpConversationLifecycle::Stopping);
                        break;
                    }
                }
            }

            Ok(())
        },
    );

    conversation
        .await
        .map_err(|err| DadrError::Acp(format!("ACP conversation failed: {err}")))?;

    Ok(())
}

async fn send_prompt_and_collect(
    session: &mut ActiveSession<'static, Agent>,
    prompt: String,
    receiver: &mut tokio_mpsc::UnboundedReceiver<AcpConversationCommand>,
    events: Option<&std_mpsc::Sender<AcpSessionEvent>>,
    permissions: &AcpPermissionBridge,
) -> Result<String, agent_client_protocol::Error> {
    let mut output = AcpOutputBuffer::new(ACP_OUTPUT_LIMIT_BYTES);
    session.send_prompt(prompt)?;

    loop {
        tokio::select! {
            _ = tokio::time::sleep(ACP_ACTIVITY_TIMEOUT) => {
                cancel_session(session)?;
                return Err(agent_client_protocol::Error::internal_error()
                    .data("timed out waiting for ACP session update"));
            }
            Some(command) = receiver.recv() => {
                match command {
                    AcpConversationCommand::Cancel => {
                        cancel_session(session)?;
                    }
                    AcpConversationCommand::Shutdown => {
                        cancel_session(session)?;
                        return Err(agent_client_protocol::Error::internal_error()
                            .data("ACP conversation shut down while a prompt was running"));
                    }
                    AcpConversationCommand::Prompt { response, .. } => {
                        let _ = response.send(Err("another ACP prompt is already running".to_owned()));
                    }
                    AcpConversationCommand::SetConfig { response, .. } => {
                        let _ = response.send(Err(
                            "ACP settings cannot be changed while a prompt is running".to_owned(),
                        ));
                    }
                }
            }
            message = session.read_update() => {
                match message? {
                    SessionMessage::SessionMessage(message) => {
                        if let Err(err) =
                            handle_session_dispatch(message, &mut output, events, permissions).await
                        {
                            let _ = cancel_session(session);
                            return Err(err);
                        }
                    }
                    SessionMessage::StopReason(_) => break,
                    _ => {}
                }
            }
        }
    }

    Ok(output.into_string())
}

async fn set_session_config_option(
    session: &ActiveSession<'static, Agent>,
    config_id: String,
    value: String,
    events: Option<&std_mpsc::Sender<AcpSessionEvent>>,
) -> Result<Vec<AcpConfigOption>, agent_client_protocol::Error> {
    let response: SetSessionConfigOptionResponse = session
        .connection()
        .send_request_to(
            Agent,
            SetSessionConfigOptionRequest::new(session.session_id().clone(), config_id, value),
        )
        .block_task()
        .await?;
    let options = acp_config_options_from_schema(response.config_options);
    send_acp_event(events, AcpSessionEvent::ConfigOptions(options.clone()));
    Ok(options)
}

fn cancel_session(
    session: &ActiveSession<'static, Agent>,
) -> Result<(), agent_client_protocol::Error> {
    session
        .connection()
        .send_notification_to(Agent, CancelNotification::new(session.session_id().clone()))
}

async fn handle_session_dispatch(
    message: Dispatch<UntypedMessage, UntypedMessage>,
    output: &mut AcpOutputBuffer,
    events: Option<&std_mpsc::Sender<AcpSessionEvent>>,
    permissions: &AcpPermissionBridge,
) -> Result<(), agent_client_protocol::Error> {
    let collected = Arc::new(Mutex::new(String::new()));
    let collected_for_handler = Arc::clone(&collected);
    let events_for_notification = events.cloned();
    let events_for_permission = events.cloned();
    let permissions_for_request = Arc::clone(permissions);
    MatchDispatch::new(message)
        .if_notification(async |notification: SessionNotification| {
            match notification.update {
                SessionUpdate::AgentMessageChunk(content_chunk) => {
                    let mut collected = collected_for_handler
                        .lock()
                        .map_err(|_| agent_client_protocol::Error::internal_error())?;
                    collected.push_str(&content_block_to_string(&content_chunk.content));
                }
                SessionUpdate::ConfigOptionUpdate(update) => {
                    send_acp_event(
                        events_for_notification.as_ref(),
                        AcpSessionEvent::ConfigOptions(acp_config_options_from_schema(
                            update.config_options,
                        )),
                    );
                }
                SessionUpdate::AvailableCommandsUpdate(update) => {
                    send_acp_event(
                        events_for_notification.as_ref(),
                        AcpSessionEvent::AvailableCommands(
                            update
                                .available_commands
                                .into_iter()
                                .map(acp_available_command_from_schema)
                                .collect(),
                        ),
                    );
                }
                SessionUpdate::UserMessageChunk(_)
                | SessionUpdate::AgentThoughtChunk(_)
                | SessionUpdate::ToolCall(_)
                | SessionUpdate::ToolCallUpdate(_)
                | SessionUpdate::Plan(_)
                | SessionUpdate::CurrentModeUpdate(_)
                | SessionUpdate::SessionInfoUpdate(_)
                | SessionUpdate::UsageUpdate(_) => {}
                _ => {}
            }
            Ok(())
        })
        .await
        .if_request(async |request: RequestPermissionRequest, responder| {
            let options = acp_permission_options_from_schema(&request);
            if options.is_empty() {
                send_acp_event(
                    events_for_permission.as_ref(),
                    AcpSessionEvent::PermissionDenied {
                        message: permission_denied_message(&request),
                    },
                );
                responder.respond(RequestPermissionResponse::new(
                    RequestPermissionOutcome::Cancelled,
                ))?;
                return Ok(());
            }
            let (request_id, decision_receiver) =
                register_permission_request(&permissions_for_request).map_err(|_| {
                    agent_client_protocol::Error::internal_error()
                        .data("failed to register ACP permission request")
                })?;
            let message = permission_request_message(&request);
            send_acp_event(
                events_for_permission.as_ref(),
                AcpSessionEvent::PermissionRequested(AcpPermissionRequest {
                    request_id,
                    message,
                    options,
                }),
            );
            let decision = wait_for_permission_decision(
                request_id,
                decision_receiver,
                Arc::clone(&permissions_for_request),
            )
            .await?;
            let outcome = match decision {
                Some(option_id) => {
                    RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(option_id))
                }
                None => RequestPermissionOutcome::Cancelled,
            };
            responder.respond(RequestPermissionResponse::new(outcome))?;
            Ok(())
        })
        .await
        .otherwise(async |_msg| Ok(()))
        .await?;

    let collected = collected
        .lock()
        .map_err(|_| agent_client_protocol::Error::internal_error())?;
    output
        .push_str(&collected)
        .map_err(acp_output_limit_error)?;
    Ok(())
}

fn send_acp_event(events: Option<&std_mpsc::Sender<AcpSessionEvent>>, event: AcpSessionEvent) {
    if let Some(events) = events {
        let _ = events.send(event);
    }
}

fn validate_prompt_options(options: AcpPromptOptions) -> Result<(), DadrError> {
    if options.output_limit_bytes == 0 {
        return Err(DadrError::Acp(
            "ACP output limit must be greater than zero".to_owned(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn unfinished_shutdown_reaps_join_handle_after_worker_exits() {
        let (release_sender, release_receiver) = std_mpsc::channel();
        let (joined_sender, joined_receiver) = std_mpsc::channel();
        let handle = thread::spawn(move || {
            release_receiver.recv().unwrap();
            joined_sender.send(()).unwrap();
        });

        assert_eq!(finish_acp_thread(handle, false), AcpThreadFinish::Reaping);
        release_sender.send(()).unwrap();

        assert!(joined_receiver.recv_timeout(Duration::from_secs(1)).is_ok());
    }

    #[test]
    fn read_only_dadrock_mcp_session_uses_read_only_server_arg() {
        let request = new_session_request(
            PathBuf::from("/tmp/dadrock-project"),
            Some(DadrockMcpAccess::ReadOnly),
        )
        .expect("session request");
        let json = serde_json::to_value(request).expect("serialize session request");

        assert!(json.to_string().contains("--read-only"));
    }
}
