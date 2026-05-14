use serde_yaml::{Mapping, Value};

use crate::{
    error::{Diagnostic, DiagnosticCode},
    frontmatter::split_front_matter,
    model::{
        BibliographyRef, Record, RecordBody, RecordFrontMatter, RecordStatus, RecordSummary, Tag,
    },
    RecordId,
};

const UNSUPPORTED_LIFECYCLE_DATE_FIELDS: &[&str] = &["date", "created", "accepted_at"];

#[derive(Debug, Clone)]
pub struct ParsedRecord {
    pub record: Option<Record>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ParsedRecord {
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }

    pub fn summary(&self) -> Option<RecordSummary> {
        self.record.as_ref().map(RecordSummary::from)
    }
}

pub fn parse_record_str(input: &str) -> ParsedRecord {
    let split = split_front_matter(input);
    let mut diagnostics = split.diagnostics;

    let Some(raw_yaml) = split.raw_yaml else {
        return ParsedRecord {
            record: None,
            diagnostics,
        };
    };

    let value = match serde_yaml::from_str::<Value>(&raw_yaml) {
        Ok(value) => value,
        Err(err) => {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::MalformedYaml,
                format!("front matter is not valid YAML: {err}"),
            ));
            return ParsedRecord {
                record: None,
                diagnostics,
            };
        }
    };

    let Value::Mapping(mapping) = value else {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::FrontMatterNotMapping,
            "front matter must be a YAML mapping",
        ));
        return ParsedRecord {
            record: None,
            diagnostics,
        };
    };

    for &field in UNSUPPORTED_LIFECYCLE_DATE_FIELDS {
        if mapping.contains_key(Value::String(field.to_owned())) {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::UnsupportedLifecycleDateField,
                    format!("`{field}` is not a supported DADR lifecycle field; use the UUIDv7 timestamp"),
                )
                .with_field(field),
            );
        }
    }

    let title = required_string(&mapping, "title", &mut diagnostics);
    let status = required_status(&mapping, &mut diagnostics);
    let id = required_record_id(&mapping, &mut diagnostics);
    let successors = optional_record_id_array(&mapping, "successors", &mut diagnostics);
    let tags = optional_string_array(&mapping, "tags", &mut diagnostics)
        .into_iter()
        .map(Tag::raw)
        .collect();
    let bibliography = optional_bibliography(&mapping, &mut diagnostics);

    let record = match (title, status, id) {
        (Some(title), Some(status), Some(id)) => Some(Record {
            front_matter: RecordFrontMatter {
                title,
                status,
                id,
                successors,
                tags,
                bibliography,
            },
            body: RecordBody {
                raw_markdown: split.body,
            },
        }),
        _ => None,
    };

    ParsedRecord {
        record,
        diagnostics,
    }
}

pub fn missing_required_sections(markdown: &str) -> Vec<&'static str> {
    let mut has_context = false;
    let mut has_decision = false;
    let mut has_alternatives = false;
    let mut has_consequences = false;

    for line in markdown.lines() {
        let Some(heading) = markdown_heading_text(line) else {
            continue;
        };
        let heading = heading.to_ascii_lowercase();
        match heading.as_str() {
            "context" => has_context = true,
            "decision" => has_decision = true,
            "alternatives" => has_alternatives = true,
            "consequences" => has_consequences = true,
            _ => {}
        }
    }

    let mut missing = Vec::new();
    if !has_context {
        missing.push("Context");
    }
    if !has_decision {
        missing.push("Decision");
    }
    if !has_alternatives {
        missing.push("Alternatives");
    }
    if !has_consequences {
        missing.push("Consequences");
    }
    missing
}

fn markdown_heading_text(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if level == 0 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.chars().next().is_some_and(char::is_whitespace) {
        return None;
    }
    Some(rest.trim())
}

fn required_string(
    mapping: &Mapping,
    field: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    match mapping_get(mapping, field) {
        None => {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::MissingRequiredField,
                    format!("missing required field `{field}`"),
                )
                .with_field(field),
            );
            None
        }
        Some(Value::String(value)) if !value.trim().is_empty() => Some(value.clone()),
        Some(_) => {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidFieldType,
                    format!("field `{field}` must be a non-empty string"),
                )
                .with_field(field),
            );
            None
        }
    }
}

fn required_status(mapping: &Mapping, diagnostics: &mut Vec<Diagnostic>) -> Option<RecordStatus> {
    let value = required_string(mapping, "status", diagnostics)?;
    match value.parse() {
        Ok(status) => Some(status),
        Err(_) => {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidStatus,
                    format!("`{value}` is not a canonical DADR status"),
                )
                .with_field("status"),
            );
            None
        }
    }
}

fn required_record_id(mapping: &Mapping, diagnostics: &mut Vec<Diagnostic>) -> Option<RecordId> {
    let value = required_string(mapping, "id", diagnostics)?;
    match RecordId::parse(&value) {
        Ok(id) => Some(id),
        Err(_) => {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidRecordId,
                    format!("`{value}` is not a valid UUIDv7 DADR id"),
                )
                .with_field("id"),
            );
            None
        }
    }
}

fn optional_record_id_array(
    mapping: &Mapping,
    field: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<RecordId> {
    let Some(value) = mapping_get(mapping, field) else {
        return Vec::new();
    };
    let Value::Sequence(values) = value else {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::InvalidFieldType,
                format!("field `{field}` must be an array of UUIDv7 strings"),
            )
            .with_field(field),
        );
        return Vec::new();
    };

    values
        .iter()
        .filter_map(|value| match value {
            Value::String(raw) => match RecordId::parse(raw) {
                Ok(id) => Some(id),
                Err(_) => {
                    diagnostics.push(
                        Diagnostic::error(
                            DiagnosticCode::InvalidRecordId,
                            format!("`{raw}` is not a valid UUIDv7 DADR id"),
                        )
                        .with_field(field),
                    );
                    None
                }
            },
            _ => {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidFieldType,
                        format!("field `{field}` must contain only UUIDv7 strings"),
                    )
                    .with_field(field),
                );
                None
            }
        })
        .collect()
}

fn optional_string_array(
    mapping: &Mapping,
    field: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<String> {
    let Some(value) = mapping_get(mapping, field) else {
        return Vec::new();
    };
    let Value::Sequence(values) = value else {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::InvalidFieldType,
                format!("field `{field}` must be an array of strings"),
            )
            .with_field(field),
        );
        return Vec::new();
    };

    values
        .iter()
        .filter_map(|value| match value {
            Value::String(raw) => Some(raw.clone()),
            _ => {
                diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::InvalidFieldType,
                        format!("field `{field}` must contain only strings"),
                    )
                    .with_field(field),
                );
                None
            }
        })
        .collect()
}

fn optional_bibliography(
    mapping: &Mapping,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<BibliographyRef> {
    let value = mapping_get(mapping, "bibliography")?;
    match value {
        Value::Null => None,
        Value::String(raw) => Some(BibliographyRef::One(raw.clone())),
        Value::Sequence(_) => {
            let values = optional_string_array(mapping, "bibliography", diagnostics);
            Some(BibliographyRef::Many(values))
        }
        _ => {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::InvalidFieldType,
                    "field `bibliography` must be a string or an array of strings",
                )
                .with_field("bibliography"),
            );
            None
        }
    }
}

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DiagnosticCode;

    #[test]
    fn parses_valid_candidate() {
        let parsed = parse_record_str(
            "---\n\
             title: Drop records with missing treatment\n\
             status: candidate\n\
             id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
             tags:\n\
               - kind/sample\n\
             ---\n\
             ## Context\n\
             Body text.\n",
        );

        assert!(parsed.record.is_some());
        assert!(parsed.diagnostics.is_empty());
        let summary = parsed.summary().unwrap();
        assert_eq!(summary.status, RecordStatus::Candidate);
        assert_eq!(summary.title, "Drop records with missing treatment");
    }

    #[test]
    fn rejects_lifecycle_date_fields() {
        let parsed = parse_record_str(
            "---\n\
             title: Bad date\n\
             status: candidate\n\
             id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
             created: 2026-01-01\n\
             ---\n\
             ## Context\n",
        );

        assert!(parsed
            .diagnostics
            .iter()
            .any(|diag| diag.code == DiagnosticCode::UnsupportedLifecycleDateField));
    }

    #[test]
    fn missing_id_is_partial_parse() {
        let parsed = parse_record_str(
            "---\n\
             title: Manual draft\n\
             status: candidate\n\
             ---\n\
             ## Context\n",
        );

        assert!(parsed.record.is_none());
        assert!(parsed
            .diagnostics
            .iter()
            .any(|diag| diag.code == DiagnosticCode::MissingRequiredField
                && diag.field.as_deref() == Some("id")));
    }

    #[test]
    fn bibliography_null_is_treated_as_absent() {
        let parsed = parse_record_str(
            "---\n\
             title: Null bibliography\n\
             status: candidate\n\
             id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
             bibliography: null\n\
             ---\n\
             ## Context\n",
        );

        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.record.unwrap().front_matter.bibliography, None);
    }
}
