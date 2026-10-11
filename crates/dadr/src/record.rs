use serde_yaml::{Mapping, Value};

use crate::{
    error::{Diagnostic, DiagnosticCode},
    frontmatter::split_front_matter,
    model::{Record, RecordBody, RecordFrontMatter, RecordStatus, RecordSummary, Tag},
    RecordId,
};

const REQUIRED_SECTIONS: [&str; 4] = ["Context", "Decision", "Alternatives", "Consequences"];

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

    add_date_diagnostics(&mapping, &mut diagnostics);

    let title = required_title(&mapping, &mut diagnostics);
    let status = required_status(&mapping, &mut diagnostics);
    let id = required_record_id(&mapping, &mut diagnostics);
    let successors = optional_record_id_array(&mapping, "successors", &mut diagnostics);
    let tags = optional_string_array(&mapping, "tags", &mut diagnostics)
        .into_iter()
        .map(Tag::raw)
        .collect();

    let record = match (title, status, id) {
        (Some(title), Some(status), Some(id)) => Some(Record {
            front_matter: RecordFrontMatter {
                title,
                status,
                id,
                successors,
                tags,
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

/// How a body measures up against the four standard sections.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SectionCheck {
    pub missing: Vec<&'static str>,
    pub duplicated: Vec<&'static str>,
    pub empty: Vec<&'static str>,
}

impl SectionCheck {
    /// True when the body meets the requirements for an accepted record.
    pub fn is_complete(&self) -> bool {
        self.missing.is_empty() && self.duplicated.is_empty() && self.empty.is_empty()
    }

    pub fn problems(&self) -> Vec<String> {
        let mut problems = Vec::new();
        if !self.missing.is_empty() {
            problems.push(format!("missing {}", self.missing.join(", ")));
        }
        if !self.duplicated.is_empty() {
            problems.push(format!("repeated {}", self.duplicated.join(", ")));
        }
        if !self.empty.is_empty() {
            problems.push(format!("empty {}", self.empty.join(", ")));
        }
        problems
    }
}

/// Checks the `##` standard sections: each must appear exactly once and
/// contain text other than whitespace and HTML comments.
pub fn check_required_sections(markdown: &str) -> SectionCheck {
    let mut occurrences = [0usize; REQUIRED_SECTIONS.len()];
    let mut contents: [String; REQUIRED_SECTIONS.len()] = std::array::from_fn(|_| String::new());
    let mut current: Option<usize> = None;
    let mut fence: Option<(u8, usize)> = None;

    for line in markdown.lines() {
        if let Some((ch, len, bare)) = code_fence_marker(line) {
            fence = match fence {
                None => Some((ch, len)),
                Some((open_ch, open_len)) if bare && ch == open_ch && len >= open_len => None,
                open => open,
            };
        } else if fence.is_none() {
            if let Some((level, text)) = atx_heading(line) {
                if level <= 2 {
                    current = (level == 2)
                        .then(|| {
                            REQUIRED_SECTIONS
                                .iter()
                                .position(|name| name.eq_ignore_ascii_case(text))
                        })
                        .flatten();
                    if let Some(index) = current {
                        occurrences[index] += 1;
                    }
                    continue;
                }
            }
        }
        if let Some(index) = current {
            contents[index].push_str(line);
            contents[index].push('\n');
        }
    }

    let mut check = SectionCheck::default();
    for (index, name) in REQUIRED_SECTIONS.iter().enumerate() {
        match occurrences[index] {
            0 => check.missing.push(name),
            1 => {}
            _ => check.duplicated.push(name),
        }
        if occurrences[index] > 0 && strip_html_comments(&contents[index]).trim().is_empty() {
            check.empty.push(name);
        }
    }
    check
}

/// The standard sections with no `##` heading at all.
pub fn missing_required_sections(markdown: &str) -> Vec<&'static str> {
    check_required_sections(markdown).missing
}

fn atx_heading(line: &str) -> Option<(usize, &str)> {
    let trimmed = strip_block_indent(line)?;
    let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.is_empty() && !rest.starts_with([' ', '\t']) {
        return None;
    }
    let text = rest.trim();
    let text = match text.trim_end_matches('#') {
        stripped if stripped.is_empty() || stripped.ends_with([' ', '\t']) => stripped.trim_end(),
        _ => text,
    };
    Some((level, text))
}

/// Returns the fence character, its run length, and whether the line is a bare
/// fence (no info string), which is required to close an open fence.
fn code_fence_marker(line: &str) -> Option<(u8, usize, bool)> {
    let trimmed = strip_block_indent(line)?;
    let ch = *trimmed.as_bytes().first()?;
    if ch != b'`' && ch != b'~' {
        return None;
    }
    let len = trimmed.bytes().take_while(|byte| *byte == ch).count();
    let bare = trimmed[len..].trim().is_empty();
    (len >= 3).then_some((ch, len, bare))
}

/// Strips up to three leading spaces; four or more make an indented code block.
fn strip_block_indent(line: &str) -> Option<&str> {
    let count = line.bytes().take_while(|byte| *byte == b' ').count();
    (count <= 3).then(|| &line[count..])
}

fn strip_html_comments(text: &str) -> String {
    let mut output = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("<!--") {
        output.push_str(&rest[..start]);
        match rest[start + 4..].find("-->") {
            Some(end) => rest = &rest[start + 4 + end + 3..],
            None => return output,
        }
    }
    output.push_str(rest);
    output
}

/// Front matter never records dates or times: a record's time comes from its
/// UUIDv7 id. Flags a `date` key and any ISO 8601 date or date-time value.
fn add_date_diagnostics(mapping: &Mapping, diagnostics: &mut Vec<Diagnostic>) {
    for (key, value) in mapping {
        let field = match key {
            Value::String(key) => key.clone(),
            other => serde_yaml::to_string(other)
                .unwrap_or_default()
                .trim()
                .to_owned(),
        };
        if field == "date" {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::DateInFrontMatter,
                    "front matter must not contain a `date` key; a record's time comes from its UUIDv7 id",
                )
                .with_field(field),
            );
        } else if contains_iso_date(value) {
            diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::DateInFrontMatter,
                    format!(
                        "field `{field}` contains a date; a record's time comes from its UUIDv7 id"
                    ),
                )
                .with_field(field),
            );
        }
    }
}

fn contains_iso_date(value: &Value) -> bool {
    match value {
        Value::String(text) => is_iso_date(text),
        Value::Sequence(values) => values.iter().any(contains_iso_date),
        Value::Mapping(mapping) => mapping.values().any(contains_iso_date),
        Value::Tagged(tagged) => contains_iso_date(&tagged.value),
        _ => false,
    }
}

/// Matches an ISO 8601 date (`2026-05-07`) or date-time (`2026-05-07T14:30`,
/// `2026-05-07 14:30:00Z`, ...).
fn is_iso_date(text: &str) -> bool {
    let bytes = text.trim().as_bytes();
    let digits = |range: std::ops::Range<usize>| {
        bytes
            .get(range)
            .is_some_and(|part| part.iter().all(u8::is_ascii_digit))
    };
    let is_date = digits(0..4)
        && bytes.get(4) == Some(&b'-')
        && digits(5..7)
        && bytes.get(7) == Some(&b'-')
        && digits(8..10);
    if !is_date {
        return false;
    }
    match bytes.get(10) {
        None => true,
        Some(b'T' | b't' | b' ') => {
            digits(11..13) && bytes.get(13) == Some(&b':') && digits(14..16)
        }
        Some(_) => false,
    }
}

fn required_title(mapping: &Mapping, diagnostics: &mut Vec<Diagnostic>) -> Option<String> {
    let title = required_string(mapping, "title", diagnostics)?;
    if title.contains(['\n', '\r']) {
        diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::InvalidFieldType,
                "field `title` must be a single line",
            )
            .with_field("title"),
        );
        return None;
    }
    Some(title)
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
    fn accepts_keys_outside_dadr() {
        let parsed = parse_record_str(
            "---\n\
             title: Extra keys\n\
             status: candidate\n\
             id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
             bibliography: [refs.bib]\n\
             author: Someone\n\
             plan: sap-2026-05\n\
             ---\n\
             ## Context\n",
        );

        assert!(parsed.record.is_some());
        assert!(parsed.diagnostics.is_empty());
    }

    #[test]
    fn rejects_dates_in_front_matter() {
        for line in [
            "date: today",
            "created: 2026-01-01",
            "updated: \"2026-01-01T10:15:00Z\"",
            "history:\n  - accepted: 2026-01-01 10:15",
        ] {
            let parsed = parse_record_str(&format!(
                "---\n\
                 title: Dated\n\
                 status: candidate\n\
                 id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
                 {line}\n\
                 ---\n"
            ));
            assert!(
                parsed
                    .diagnostics
                    .iter()
                    .any(|diag| diag.code == DiagnosticCode::DateInFrontMatter),
                "{line}"
            );
        }
    }

    #[test]
    fn iso_date_detection_is_exact() {
        assert!(is_iso_date("2026-05-07"));
        assert!(is_iso_date("2026-05-07T14:30:00Z"));
        assert!(is_iso_date("2026-05-07 14:30"));
        assert!(!is_iso_date("sap-2026-05-07"));
        assert!(!is_iso_date("2026-05"));
        assert!(!is_iso_date("2026-05-07-draft"));
        assert!(!is_iso_date("01938b8e-4d70-7abc-8def-0123456789ab"));
    }

    #[test]
    fn rejects_multiline_title() {
        let parsed = parse_record_str(
            "---\n\
             title: \"First line\\nsecond line\"\n\
             status: candidate\n\
             id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
             ---\n",
        );

        assert!(parsed.record.is_none());
        assert!(parsed
            .diagnostics
            .iter()
            .any(|diag| diag.field.as_deref() == Some("title")));
    }

    #[test]
    fn complete_sections_pass() {
        let check = check_required_sections(
            "# Title\n\n## Context\nWhy.\n\n## decision ##\nWhat.\n\n\
             ## Alternatives\nNone considered.\n### Detail\n\n## Consequences\nEffects.\n",
        );
        assert!(check.is_complete(), "{check:?}");
    }

    #[test]
    fn reports_missing_repeated_and_empty_sections() {
        let check = check_required_sections(
            "## Context\nWhy.\n\n## Decision\n<!-- fill in -->\n\n\
             ## Context\nAgain.\n\n### Alternatives\nNot a level-2 heading.\n",
        );
        assert_eq!(check.missing, vec!["Alternatives", "Consequences"]);
        assert_eq!(check.duplicated, vec!["Context"]);
        assert_eq!(check.empty, vec!["Decision"]);
    }

    #[test]
    fn ignores_headings_inside_code_fences() {
        let check = check_required_sections(
            "## Context\nWhy.\n\n```markdown\n## Decision\n```\n\n\
             ## Decision\nWhat.\n\n## Alternatives\nNone.\n\n## Consequences\nEffects.\n",
        );
        assert!(check.is_complete(), "{check:?}");
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
}
