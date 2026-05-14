use std::{borrow::Cow, fmt, fs};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{
    error::DadrError,
    markdown::frontmatter::split_front_matter,
    project::{utf8_to_path, ProjectContext},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateName {
    Candidate,
}

impl TemplateName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Candidate => "candidate",
        }
    }
}

impl fmt::Display for TemplateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateSource {
    Project,
    BuiltIn,
}

const BUILT_IN_CANDIDATE_TEMPLATE: &str =
    "## Context\n\n\n## Decision\n\n\n## Alternatives\n\n\n## Consequences\n\n";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateResolution {
    pub name: TemplateName,
    pub source: TemplateSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Utf8PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateContent {
    pub resolution: TemplateResolution,
    pub body: String,
}

pub fn resolve_template(
    project: &ProjectContext,
    name: TemplateName,
) -> Result<TemplateResolution, DadrError> {
    let configured_path = match name {
        TemplateName::Candidate => project.templates_dir().join("candidate.md"),
    };

    if configured_path.exists() && !configured_path.is_file() {
        return Err(DadrError::Template(format!(
            "configured {name} template `{configured_path}` is not a file"
        )));
    }

    if configured_path.is_file() {
        Ok(TemplateResolution {
            name,
            source: TemplateSource::Project,
            path: Some(configured_path),
        })
    } else {
        Ok(TemplateResolution {
            name,
            source: TemplateSource::BuiltIn,
            path: None,
        })
    }
}

pub fn candidate_template_body(project: &ProjectContext) -> Result<TemplateContent, DadrError> {
    read_template(project, TemplateName::Candidate)
}

pub fn read_template(
    project: &ProjectContext,
    name: TemplateName,
) -> Result<TemplateContent, DadrError> {
    let resolution = resolve_template(project, name)?;
    let body = match (&resolution.source, &resolution.path, name) {
        (TemplateSource::Project, Some(path), _) => {
            let content = fs::read_to_string(utf8_to_path(path))?;
            validate_template_front_matter(name, &content)?;
            template_body_from_content(&content).into_owned()
        }
        _ => built_in_template_body(name).to_owned(),
    };

    Ok(TemplateContent { resolution, body })
}

fn template_body_from_content(content: &str) -> Cow<'_, str> {
    let split = split_front_matter(content);
    if split.raw_yaml.is_some() && split.diagnostics.is_empty() {
        Cow::Owned(split.body)
    } else {
        Cow::Borrowed(content)
    }
}

fn validate_template_front_matter(name: TemplateName, content: &str) -> Result<(), DadrError> {
    let split = split_front_matter(content);
    if split.raw_yaml.is_some() && split.diagnostics.iter().any(|diag| diag.is_error()) {
        return Err(DadrError::Template(format!(
            "{name} template front matter is malformed"
        )));
    }
    Ok(())
}

fn built_in_template_body(name: TemplateName) -> &'static str {
    match name {
        TemplateName::Candidate => BUILT_IN_CANDIDATE_TEMPLATE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_optional_template_front_matter() {
        let body = template_body_from_content("---\nnotes: ignored\n---\n## Context\n");
        assert_eq!(body, "## Context\n");
    }

    #[test]
    fn malformed_template_front_matter_is_rejected() {
        let error =
            validate_template_front_matter(TemplateName::Candidate, "---\nnotes: ignored\n")
                .unwrap_err();
        assert!(error.to_string().contains("front matter is malformed"));
    }
}
