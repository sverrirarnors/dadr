use std::{
    env,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use serde::{Deserialize, Serialize};

use crate::{error::DadrError, paths::expand_tilde};

pub const ACP_AGENT_ENV: &str = "DADROCK_ACP_AGENT";
pub const ACP_AGENT_ARGS_ENV: &str = "DADROCK_ACP_AGENT_ARGS";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownAcpProvider {
    pub id: &'static str,
    pub name: &'static str,
    pub program: &'static str,
    pub args: &'static [&'static str],
}

impl KnownAcpProvider {
    pub fn command_line(self) -> String {
        if self.args.is_empty() {
            self.program.to_owned()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

const KNOWN_ACP_PROVIDERS: &[KnownAcpProvider] = &[
    KnownAcpProvider {
        id: "claude-agent-acp",
        name: "Claude Agent ACP",
        program: "claude-agent-acp",
        args: &[],
    },
    KnownAcpProvider {
        id: "claude-acp",
        name: "Claude ACP",
        program: "claude-code-acp",
        args: &[],
    },
    KnownAcpProvider {
        id: "gemini-acp",
        name: "Gemini ACP",
        program: "gemini",
        args: &["--acp"],
    },
    KnownAcpProvider {
        id: "codex-acp",
        name: "Codex ACP",
        program: "codex-acp",
        args: &[],
    },
    KnownAcpProvider {
        id: "opencode-acp",
        name: "OpenCode ACP",
        program: "opencode",
        args: &["acp"],
    },
    KnownAcpProvider {
        id: "cursor-acp",
        name: "Cursor Agent ACP",
        program: "cursor-agent",
        args: &["acp"],
    },
    KnownAcpProvider {
        id: "copilot-acp",
        name: "Copilot ACP",
        program: "copilot",
        args: &["--acp", "--stdio"],
    },
    KnownAcpProvider {
        id: "auggie-acp",
        name: "Auggie ACP",
        program: "auggie",
        args: &["--acp"],
    },
    KnownAcpProvider {
        id: "mistral-vibe-acp",
        name: "Mistral Vibe ACP",
        program: "vibe-acp",
        args: &[],
    },
    KnownAcpProvider {
        id: "cline-acp",
        name: "Cline ACP",
        program: "cline",
        args: &["--acp"],
    },
    KnownAcpProvider {
        id: "goose-acp",
        name: "Goose ACP",
        program: "goose",
        args: &["acp"],
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalAcpAgentCommand {
    pub program: String,
    pub args: Vec<String>,
}

impl LocalAcpAgentCommand {
    pub fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
        }
    }

    pub fn command_line(&self) -> String {
        if self.args.is_empty() {
            self.program.clone()
        } else {
            format!("{} {}", self.program, self.args.join(" "))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedAcpAgent {
    pub provider: &'static KnownAcpProvider,
    pub command: LocalAcpAgentCommand,
}

pub fn known_acp_providers() -> &'static [KnownAcpProvider] {
    KNOWN_ACP_PROVIDERS
}

pub fn resolve_local_acp_agent_command(
    program: impl Into<String>,
    args: Vec<String>,
    source: &str,
) -> Result<LocalAcpAgentCommand, DadrError> {
    let path_var = env::var_os("PATH");
    resolve_local_acp_agent_command_in_path(program.into(), args, source, path_var.as_deref())
}

pub fn configured_local_acp_agent() -> Result<LocalAcpAgentCommand, DadrError> {
    let extra_args = acp_agent_args_from_env()?;
    match env::var(ACP_AGENT_ENV) {
        Ok(program) => return resolve_local_acp_agent_command(program, extra_args, ACP_AGENT_ENV),
        Err(env::VarError::NotPresent) => {}
        Err(env::VarError::NotUnicode(_)) => {
            return Err(DadrError::Acp(format!(
                "{ACP_AGENT_ENV} must be valid UTF-8"
            )));
        }
    }

    if let Some(mut detected) = detect_local_acp_agent() {
        detected.command.args.extend(extra_args);
        return Ok(detected.command);
    }

    Err(DadrError::Acp(format!(
        "no local ACP agent found; set {ACP_AGENT_ENV}=<program> or install one of: {}",
        known_acp_provider_command_lines().join(", ")
    )))
}

pub fn detect_local_acp_agent() -> Option<DetectedAcpAgent> {
    detect_local_acp_agent_in_path(env::var_os("PATH").as_deref())
}

fn acp_agent_args_from_env() -> Result<Vec<String>, DadrError> {
    match env::var(ACP_AGENT_ARGS_ENV) {
        Ok(raw) => parse_acp_agent_args(&raw),
        Err(env::VarError::NotPresent) => Ok(Vec::new()),
        Err(env::VarError::NotUnicode(_)) => Err(DadrError::Acp(format!(
            "{ACP_AGENT_ARGS_ENV} must be valid UTF-8"
        ))),
    }
}

pub(super) fn parse_acp_agent_args(raw: &str) -> Result<Vec<String>, DadrError> {
    shell_words::split(raw).map_err(|err| {
        DadrError::Acp(format!(
            "{ACP_AGENT_ARGS_ENV} could not be parsed as shell words: {err}"
        ))
    })
}

pub(super) fn detect_local_acp_agent_in_path(path_var: Option<&OsStr>) -> Option<DetectedAcpAgent> {
    KNOWN_ACP_PROVIDERS.iter().find_map(|provider| {
        resolve_command_in_path(provider.program, path_var).map(|program| DetectedAcpAgent {
            provider,
            command: LocalAcpAgentCommand::new(
                program.to_string_lossy().into_owned(),
                provider.args.iter().map(|arg| (*arg).to_owned()).collect(),
            ),
        })
    })
}

pub(super) fn resolve_local_acp_agent_command_in_path(
    program: String,
    args: Vec<String>,
    source: &str,
    path_var: Option<&OsStr>,
) -> Result<LocalAcpAgentCommand, DadrError> {
    let program = program.trim();
    if program.is_empty() {
        return Err(DadrError::Acp(format!("{source} must not be empty")));
    }

    if !command_has_path_separator(program) && program.split_whitespace().count() > 1 {
        return Err(DadrError::Acp(format!(
            "{source} must contain only the ACP executable, got `{program}`. Put arguments in {ACP_AGENT_ARGS_ENV}; for example: {ACP_AGENT_ENV}=gemini {ACP_AGENT_ARGS_ENV}=\"--acp\""
        )));
    }

    let Some(resolved) = resolve_command_in_path(program, path_var) else {
        return Err(DadrError::Acp(unresolved_acp_agent_message(
            source, program,
        )));
    };

    Ok(LocalAcpAgentCommand::new(
        resolved.to_string_lossy().into_owned(),
        args,
    ))
}

fn unresolved_acp_agent_message(source: &str, program: &str) -> String {
    let known = known_acp_provider_command_lines().join(", ");
    if source == ACP_AGENT_ENV {
        format!(
            "{ACP_AGENT_ENV} points to `{program}`, but that command is not executable or not on PATH. Set {ACP_AGENT_ENV} to an executable ACP provider, move arguments into {ACP_AGENT_ARGS_ENV}, or unset {ACP_AGENT_ENV} to allow auto-detection. Known provider commands: {known}"
        )
    } else {
        format!(
            "{source} `{program}` is not executable or not on PATH. Use an executable ACP provider command. Known provider commands: {known}"
        )
    }
}

fn known_acp_provider_command_lines() -> Vec<String> {
    KNOWN_ACP_PROVIDERS
        .iter()
        .map(|provider| provider.command_line())
        .collect()
}

fn resolve_command_in_path(command: &str, path_var: Option<&OsStr>) -> Option<PathBuf> {
    if command.trim().is_empty() {
        return None;
    }

    if command_has_path_separator(command) {
        return first_executable_candidate(&expand_tilde(command));
    }

    path_var
        .into_iter()
        .flat_map(env::split_paths)
        .find_map(|dir| first_executable_candidate(&dir.join(command)))
}

fn command_has_path_separator(command: &str) -> bool {
    command.contains('/') || command.contains('\\')
}

fn first_executable_candidate(path: &Path) -> Option<PathBuf> {
    executable_candidates(path)
        .into_iter()
        .find(|candidate| is_executable_file(candidate))
}

#[cfg(windows)]
fn executable_candidates(path: &Path) -> Vec<PathBuf> {
    if path.extension().is_some() {
        return vec![path.to_path_buf()];
    }
    env::var_os("PATHEXT")
        .map(|raw| {
            env::split_paths(&raw)
                .map(|ext| {
                    let ext = ext.to_string_lossy();
                    path.with_extension(ext.trim_start_matches('.'))
                })
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                path.with_extension("exe"),
                path.with_extension("cmd"),
                path.with_extension("bat"),
            ]
        })
}

#[cfg(not(windows))]
fn executable_candidates(path: &Path) -> Vec<PathBuf> {
    vec![path.to_path_buf()]
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(windows)]
fn is_executable_file(path: &Path) -> bool {
    fs::metadata(path)
        .map(|metadata| metadata.is_file() && has_windows_executable_extension(path))
        .unwrap_or(false)
}

#[cfg(windows)]
fn has_windows_executable_extension(path: &Path) -> bool {
    let Some(extension) = path.extension().and_then(OsStr::to_str) else {
        return false;
    };
    let extension = format!(".{}", extension.trim_start_matches('.')).to_ascii_lowercase();
    windows_executable_extensions()
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(&extension))
}

#[cfg(windows)]
fn windows_executable_extensions() -> Vec<String> {
    env::var_os("PATHEXT")
        .map(|raw| {
            env::split_paths(&raw)
                .filter_map(|path| path.to_str().map(str::to_owned))
                .map(|ext| {
                    if ext.starts_with('.') {
                        ext
                    } else {
                        format!(".{ext}")
                    }
                })
                .collect()
        })
        .filter(|extensions: &Vec<String>| !extensions.is_empty())
        .unwrap_or_else(|| {
            vec![
                ".exe".to_owned(),
                ".cmd".to_owned(),
                ".bat".to_owned(),
                ".com".to_owned(),
            ]
        })
}
