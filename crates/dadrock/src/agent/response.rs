use std::collections::BTreeSet;

use serde_json::Value;

use crate::{
    agent::{
        json::extract_json_object,
        types::{AgentDraftRequirements, AgentIntent, AgentResponseEnvelope},
    },
    error::DadrError,
    records::missing_required_sections,
    text::{ascii_slug, normalized_tag_key},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentCandidateMetadata {
    pub title: String,
    pub tags: Vec<String>,
}

pub fn parse_agent_response_text(text: &str) -> Result<AgentResponseEnvelope, DadrError> {
    let json = extract_json_object(text, looks_like_agent_response_json).ok_or_else(|| {
        DadrError::Acp("ACP agent response did not contain a JSON object".to_owned())
    })?;
    serde_json::from_str::<AgentResponseEnvelope>(json).map_err(|err| {
        DadrError::Acp(format!(
            "ACP agent response did not match the Dadrock draft envelope: {err}"
        ))
    })
}

fn looks_like_agent_response_json(input: &str) -> bool {
    let Ok(Value::Object(object)) = serde_json::from_str::<Value>(input) else {
        return false;
    };
    object.contains_key("intent") && object.contains_key("body_markdown")
}

pub fn candidate_metadata_from_response(
    response: &mut AgentResponseEnvelope,
) -> Result<AgentCandidateMetadata, DadrError> {
    let title = candidate_title_from_response(response)?;
    let tags = tags_from_response(response);
    Ok(AgentCandidateMetadata { title, tags })
}

fn candidate_title_from_response(
    response: &mut AgentResponseEnvelope,
) -> Result<String, DadrError> {
    let source = response
        .frontmatter
        .get("title")
        .and_then(Value::as_str)
        .map(AgentTitleSource::Frontmatter)
        .or_else(|| first_heading(&response.body_markdown).map(AgentTitleSource::Heading))
        .unwrap_or(AgentTitleSource::Summary(response.summary.as_str()));
    let title = source.title().trim();

    if title.is_empty() {
        return Err(DadrError::Acp(
            "agent response did not include a candidate title".to_owned(),
        ));
    }

    if matches!(source, AgentTitleSource::Summary(_)) {
        push_response_warning(
            &mut response.warnings,
            "agent response did not include frontmatter title or H1; using summary as candidate title",
        );
    }

    Ok(title.to_owned())
}

fn tags_from_response(response: &AgentResponseEnvelope) -> Vec<String> {
    let mut tags = Vec::new();
    if let Some(raw_tags) = response.frontmatter.get("tags").and_then(Value::as_array) {
        for tag in raw_tags {
            if let Some(tag) = tag.as_str() {
                tags.push(tag.to_owned());
            }
        }
    }
    tags.push("origin/agent".to_owned());
    tags.push(response.confidence.as_tag().to_owned());
    if let Some(model) = &response.provenance.model {
        let model_tag = ascii_slug(model, 80, "");
        if !model_tag.is_empty() {
            tags.push(format!("origin/agent/{model_tag}"));
        }
    }
    dedup_tags(&mut tags);
    tags
}

enum AgentTitleSource<'a> {
    Frontmatter(&'a str),
    Heading(&'a str),
    Summary(&'a str),
}

impl AgentTitleSource<'_> {
    fn title(&self) -> &str {
        match self {
            Self::Frontmatter(title) | Self::Heading(title) | Self::Summary(title) => title,
        }
    }
}

fn push_response_warning(warnings: &mut Vec<String>, warning: &str) {
    if !warnings.iter().any(|existing| existing == warning) {
        warnings.push(warning.to_owned());
    }
}

fn dedup_tags(tags: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    tags.retain(|tag| {
        let key = normalized_tag_key(tag);
        !key.is_empty() && seen.insert(key)
    });
}

pub fn validate_draft_response(response: &AgentResponseEnvelope) -> Result<(), DadrError> {
    validate_candidate_response(response, AgentIntent::DraftCandidate)
}

pub fn validate_revision_response(response: &AgentResponseEnvelope) -> Result<(), DadrError> {
    validate_candidate_response(response, AgentIntent::ReviseCandidate)
}

fn validate_candidate_response(
    response: &AgentResponseEnvelope,
    expected: AgentIntent,
) -> Result<(), DadrError> {
    if response.intent != expected {
        return Err(DadrError::Acp(format!(
            "ACP agent returned intent `{}`; expected `{}`",
            response.intent.as_str(),
            expected.as_str()
        )));
    }
    if response.body_markdown.trim().is_empty() {
        return Err(DadrError::Acp(
            "ACP agent returned an empty candidate body".to_owned(),
        ));
    }
    if !response.frontmatter.is_null() && !response.frontmatter.is_object() {
        return Err(DadrError::Acp(
            "ACP agent response frontmatter must be a JSON object".to_owned(),
        ));
    }
    validate_candidate_frontmatter(&response.frontmatter)?;
    let missing = missing_required_sections(&response.body_markdown);
    if !missing.is_empty() {
        return Err(DadrError::Acp(format!(
            "ACP agent candidate body is missing required section(s): {}",
            missing.join(", ")
        )));
    }
    Ok(())
}

fn validate_candidate_frontmatter(frontmatter: &Value) -> Result<(), DadrError> {
    let Some(object) = frontmatter.as_object() else {
        return Ok(());
    };
    let requirements = AgentDraftRequirements::default();
    if let Some(status) = object.get("status").and_then(Value::as_str) {
        if status != requirements.status {
            return Err(DadrError::Acp(format!(
                "ACP agent response frontmatter status must be `{}`",
                requirements.status
            )));
        }
    }
    if object.contains_key("id") {
        return Err(DadrError::Acp(
            "ACP agent response frontmatter must not include `id`; Dadrock assigns record ids"
                .to_owned(),
        ));
    }
    for field in requirements.forbidden_frontmatter {
        if object.contains_key(&field) {
            return Err(DadrError::Acp(format!(
                "ACP agent response frontmatter must not include `{field}`"
            )));
        }
    }
    Ok(())
}

fn first_heading(markdown: &str) -> Option<&str> {
    markdown
        .lines()
        .find_map(|line| line.trim().strip_prefix("# "))
}
