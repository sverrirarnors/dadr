use agent_client_protocol::schema::{
    AvailableCommand, SessionConfigKind, SessionConfigOption, SessionConfigOptionCategory,
    SessionConfigSelectOptions,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcpConfigCategory {
    Mode,
    Model,
    ThoughtLevel,
    Other,
}

impl AcpConfigCategory {
    pub fn label(self) -> &'static str {
        match self {
            Self::Mode => "mode",
            Self::Model => "model",
            Self::ThoughtLevel => "effort",
            Self::Other => "setting",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpConfigValue {
    pub value: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpConfigOption {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: AcpConfigCategory,
    pub current_value: String,
    pub values: Vec<AcpConfigValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcpAvailableCommand {
    pub name: String,
    pub description: String,
}

pub(super) fn acp_config_options_from_schema(
    config_options: Vec<SessionConfigOption>,
) -> Vec<AcpConfigOption> {
    config_options
        .into_iter()
        .filter_map(acp_config_option_from_schema)
        .collect()
}

fn acp_config_option_from_schema(option: SessionConfigOption) -> Option<AcpConfigOption> {
    let id = option.id.to_string();
    let category = acp_config_category(option.category, &id, &option.name);
    let (current_value, values) = match option.kind {
        SessionConfigKind::Select(select) => (
            select.current_value.to_string(),
            acp_config_select_values(select.options),
        ),
        _ => return None,
    };
    Some(AcpConfigOption {
        id,
        name: option.name,
        description: option.description,
        category,
        current_value,
        values,
    })
}

fn acp_config_category(
    category: Option<SessionConfigOptionCategory>,
    id: &str,
    name: &str,
) -> AcpConfigCategory {
    match category {
        Some(SessionConfigOptionCategory::Mode) => AcpConfigCategory::Mode,
        Some(SessionConfigOptionCategory::Model) => AcpConfigCategory::Model,
        Some(SessionConfigOptionCategory::ThoughtLevel) => AcpConfigCategory::ThoughtLevel,
        Some(SessionConfigOptionCategory::Other(raw)) => {
            acp_config_category_from_text(&raw).unwrap_or(AcpConfigCategory::Other)
        }
        None => acp_config_category_from_text(&format!("{id} {name}"))
            .unwrap_or(AcpConfigCategory::Other),
        _ => AcpConfigCategory::Other,
    }
}

fn acp_config_category_from_text(input: &str) -> Option<AcpConfigCategory> {
    let lower = input.to_ascii_lowercase();
    if lower.contains("model") {
        Some(AcpConfigCategory::Model)
    } else if lower.contains("mode") {
        Some(AcpConfigCategory::Mode)
    } else if lower.contains("thought")
        || lower.contains("effort")
        || lower.contains("reasoning")
        || lower.contains("reason")
    {
        Some(AcpConfigCategory::ThoughtLevel)
    } else {
        None
    }
}

fn acp_config_select_values(options: SessionConfigSelectOptions) -> Vec<AcpConfigValue> {
    match options {
        SessionConfigSelectOptions::Ungrouped(options) => options
            .into_iter()
            .map(|option| AcpConfigValue {
                value: option.value.to_string(),
                name: option.name,
                description: option.description,
            })
            .collect(),
        SessionConfigSelectOptions::Grouped(groups) => groups
            .into_iter()
            .flat_map(|group| {
                let group_name = group.name;
                group.options.into_iter().map(move |option| AcpConfigValue {
                    value: option.value.to_string(),
                    name: if group_name.trim().is_empty() {
                        option.name
                    } else {
                        format!("{group_name} / {}", option.name)
                    },
                    description: option.description,
                })
            })
            .collect(),
        _ => Vec::new(),
    }
}

pub(super) fn acp_available_command_from_schema(command: AvailableCommand) -> AcpAvailableCommand {
    AcpAvailableCommand {
        name: command.name,
        description: command.description,
    }
}
