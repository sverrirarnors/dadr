use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{
    error::{DadrError, DiagnosticCode, IssueCounts, Severity},
    ids::RecordId,
    index::ProjectIndex,
    markdown::scan::{byte_in_spans, inline_code_spans, is_fenced_code_fence},
    records::{ScannedRecord, ScannedRecordSummary},
    text::{normalized_tag_key, write_serde_label},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagReport {
    pub ok: bool,
    pub tags: Vec<TagSummary>,
    pub counts: IssueCounts,
    pub issues: Vec<TagIssue>,
}

impl TagReport {
    pub fn has_errors(&self) -> bool {
        self.counts.has_errors()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagSummary {
    pub tag: String,
    pub display: String,
    pub record_count: usize,
    pub occurrence_count: usize,
    pub records: Vec<ScannedRecordSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagIssueCode {
    InvalidYamlTagField,
    EmptyTag,
    LeadingHashInYamlTag,
    WhitespaceInTag,
    EmptyTagSegment,
    NumericOnlyTag,
}

impl fmt::Display for TagIssueCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_serde_label(self, formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagSource {
    FrontMatter,
    Inline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagIssue {
    pub severity: Severity,
    pub code: TagIssueCode,
    pub message: String,
    pub path: Utf8PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<RecordId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub source: TagSource,
}

impl TagIssue {
    fn error(
        code: TagIssueCode,
        path: Utf8PathBuf,
        source: TagSource,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code,
            message: message.into(),
            path,
            line: None,
            record_id: None,
            tag: None,
            source,
        }
    }

    fn with_line(mut self, line: usize) -> Self {
        self.line = Some(line);
        self
    }

    fn with_record_id(mut self, record_id: RecordId) -> Self {
        self.record_id = Some(record_id);
        self
    }

    fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = Some(tag.into());
        self
    }
}

#[derive(Debug, Clone)]
struct TagAccumulator {
    display: String,
    occurrence_count: usize,
    records: BTreeMap<RecordId, ScannedRecordSummary>,
}

pub fn build_tag_report(index: &ProjectIndex) -> TagReport {
    let mut tags = BTreeMap::<String, TagAccumulator>::new();
    let mut issues = Vec::new();

    for scanned_record in index.scanned_records() {
        add_tag_field_diagnostics(scanned_record, &mut issues);
        let Some(record) = &scanned_record.record else {
            continue;
        };
        let Some(summary) = scanned_record.summary() else {
            continue;
        };

        for tag in &record.front_matter.tags {
            add_observed_tag(
                &mut tags,
                &mut issues,
                scanned_record,
                &summary,
                tag.as_str(),
                TagSource::FrontMatter,
                None,
            );
        }

        let mut in_fenced_code = false;
        for (line_index, line) in record.body.raw_markdown.lines().enumerate() {
            if is_fenced_code_fence(line) {
                in_fenced_code = !in_fenced_code;
                continue;
            }
            if in_fenced_code {
                continue;
            }
            for inline_tag in inline_tags_in_line(line) {
                add_observed_tag(
                    &mut tags,
                    &mut issues,
                    scanned_record,
                    &summary,
                    &inline_tag,
                    TagSource::Inline,
                    Some(line_index + 1),
                );
            }
        }
    }

    let mut tag_summaries: Vec<_> = tags
        .into_iter()
        .map(|(tag, accumulator)| {
            let records: Vec<_> = accumulator.records.into_values().collect();
            TagSummary {
                tag,
                display: accumulator.display,
                record_count: records.len(),
                occurrence_count: accumulator.occurrence_count,
                records,
            }
        })
        .collect();
    tag_summaries.sort_by(|left, right| left.tag.cmp(&right.tag));
    sort_issues(&mut issues);
    let counts = IssueCounts::from_severities(issues.iter().map(|issue| issue.severity));
    let ok = counts.errors == 0;

    TagReport {
        ok,
        tags: tag_summaries,
        counts,
        issues,
    }
}

pub fn record_ids_for_tag(
    index: &ProjectIndex,
    tag_filter: &str,
) -> Result<BTreeSet<RecordId>, DadrError> {
    let key = normalize_tag_filter(tag_filter)?;
    let report = build_tag_report(index);
    Ok(report
        .tags
        .into_iter()
        .find(|tag| tag.tag == key)
        .map(|tag| tag.records.into_iter().map(|record| record.id).collect())
        .unwrap_or_default())
}

pub fn normalize_tag_filter(input: &str) -> Result<String, DadrError> {
    let tag = input.trim().trim_start_matches('#');
    let issues = validate_tag_shape(tag, TagSource::Inline);
    if issues.is_empty() {
        Ok(normalized_tag_key(tag))
    } else {
        Err(DadrError::InvalidInput(format!(
            "invalid tag filter `{input}`: {}",
            issue_message(issues[0], tag, TagSource::Inline)
        )))
    }
}

fn add_tag_field_diagnostics(scanned_record: &ScannedRecord, issues: &mut Vec<TagIssue>) {
    for diagnostic in &scanned_record.diagnostics {
        if diagnostic.field.as_deref() == Some("tags")
            && matches!(
                diagnostic.code,
                DiagnosticCode::InvalidFieldType | DiagnosticCode::MissingRequiredField
            )
        {
            issues.push(
                TagIssue::error(
                    TagIssueCode::InvalidYamlTagField,
                    diagnostic
                        .path
                        .clone()
                        .unwrap_or_else(|| scanned_record.path.clone()),
                    TagSource::FrontMatter,
                    diagnostic.message.clone(),
                )
                .with_tag("tags"),
            );
        }
    }
}

fn add_observed_tag(
    tags: &mut BTreeMap<String, TagAccumulator>,
    issues: &mut Vec<TagIssue>,
    scanned_record: &ScannedRecord,
    summary: &ScannedRecordSummary,
    raw_tag: &str,
    source: TagSource,
    line: Option<usize>,
) {
    let normalized_value = normalized_tag_value(raw_tag);
    let issue_codes = validate_tag_shape(raw_tag, source);
    for code in &issue_codes {
        let mut issue = TagIssue::error(
            *code,
            scanned_record.path.clone(),
            source,
            issue_message(*code, raw_tag, source),
        )
        .with_record_id(summary.id)
        .with_tag(raw_tag);
        if let Some(line) = line {
            issue = issue.with_line(line);
        }
        issues.push(issue);
    }

    let only_repairable_yaml_hash = issue_codes
        .iter()
        .all(|code| *code == TagIssueCode::LeadingHashInYamlTag);
    if !issue_codes.is_empty() && !only_repairable_yaml_hash {
        return;
    }

    let display = normalized_value.trim();
    if display.is_empty() {
        return;
    }
    let key = normalized_tag_key(display);
    let accumulator = tags.entry(key).or_insert_with(|| TagAccumulator {
        display: display.to_owned(),
        occurrence_count: 0,
        records: BTreeMap::new(),
    });
    accumulator.occurrence_count += 1;
    accumulator
        .records
        .entry(summary.id)
        .or_insert_with(|| summary.clone());
}

fn normalized_tag_value(raw_tag: &str) -> String {
    let tag = raw_tag.trim();
    tag.trim_start_matches('#').to_owned()
}

fn validate_tag_shape(tag: &str, source: TagSource) -> Vec<TagIssueCode> {
    let mut issues = Vec::new();
    if tag.trim().is_empty() {
        issues.push(TagIssueCode::EmptyTag);
        return issues;
    }
    if source == TagSource::FrontMatter && tag.trim_start().starts_with('#') {
        issues.push(TagIssueCode::LeadingHashInYamlTag);
    }
    let normalized = normalized_tag_value(tag);
    if normalized.chars().any(char::is_whitespace) || normalized.trim() != normalized {
        issues.push(TagIssueCode::WhitespaceInTag);
    }
    if normalized.split('/').any(str::is_empty) {
        issues.push(TagIssueCode::EmptyTagSegment);
    }
    if !normalized
        .chars()
        .any(|ch| !ch.is_numeric() && !matches!(ch, '/' | '-' | '_'))
    {
        issues.push(TagIssueCode::NumericOnlyTag);
    }
    issues
}

fn issue_message(code: TagIssueCode, tag: &str, source: TagSource) -> String {
    match code {
        TagIssueCode::InvalidYamlTagField => "`tags` must be an array of strings".to_owned(),
        TagIssueCode::EmptyTag => "tag must not be empty".to_owned(),
        TagIssueCode::LeadingHashInYamlTag => {
            format!("YAML tag `{tag}` must be stored without a leading `#`")
        }
        TagIssueCode::WhitespaceInTag => format!("tag `{tag}` must not contain whitespace"),
        TagIssueCode::EmptyTagSegment => {
            format!("tag `{tag}` must not contain empty `/` path segments")
        }
        TagIssueCode::NumericOnlyTag => {
            let prefix = match source {
                TagSource::FrontMatter => "YAML tag",
                TagSource::Inline => "inline tag",
            };
            format!("{prefix} `{tag}` must contain at least one non-numeric character")
        }
    }
}

fn inline_tags_in_line(line: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let code_spans = inline_code_spans(line);
    let indices = line.char_indices();
    for (index, ch) in indices {
        if byte_in_spans(index, &code_spans) {
            continue;
        }
        if ch != '#' || !is_inline_tag_start(line, index) {
            continue;
        }

        let start = index + ch.len_utf8();
        let mut end = start;
        for (next_index, next_ch) in line[start..].char_indices() {
            if is_inline_tag_terminator(next_ch) {
                break;
            }
            end = start + next_index + next_ch.len_utf8();
        }

        if end > start {
            let tag = &line[index..end];
            if !validate_tag_shape(tag, TagSource::Inline).contains(&TagIssueCode::NumericOnlyTag) {
                tags.push(tag.to_owned());
            }
        }
    }
    tags
}

fn is_inline_tag_start(line: &str, index: usize) -> bool {
    let next = line[index + 1..].chars().next();
    if next.is_none_or(|ch| ch.is_whitespace() || ch == '#') {
        return false;
    }
    let previous = line[..index].chars().next_back();
    previous.is_none_or(|ch| ch.is_whitespace() || matches!(ch, '>' | '[' | '{'))
}

fn is_inline_tag_terminator(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '#' | '<'
                | '>'
                | '('
                | ')'
                | '['
                | ']'
                | '{'
                | '}'
                | '"'
                | '\''
                | '`'
                | ','
                | ';'
                | ':'
                | '!'
                | '?'
                | '.'
        )
}

fn sort_issues(issues: &mut [TagIssue]) {
    issues.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.tag.cmp(&right.tag))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_inline_tags_on_boundaries() {
        assert_eq!(
            inline_tags_in_line("See #kind/sample, C# code, [x](#anchor), and #emoji-🚀."),
            vec!["#kind/sample".to_owned(), "#emoji-🚀".to_owned()]
        );
    }

    #[test]
    fn ignores_inline_tags_inside_inline_code() {
        assert_eq!(
            inline_tags_in_line("Use `#kind/example` but keep #kind/real."),
            vec!["#kind/real".to_owned()]
        );
    }

    #[test]
    fn ignores_numeric_hash_references_in_prose() {
        assert_eq!(
            inline_tags_in_line("Per-coder rankings (e.g. RA #4 vs. Kimi K2) use shrinkage."),
            Vec::<String>::new()
        );
        assert_eq!(
            inline_tags_in_line("Keep #kind/modeling, but ignore issue #42 and rank #4."),
            vec!["#kind/modeling".to_owned()]
        );
    }

    #[test]
    fn validates_unicode_and_emoji_tags() {
        assert!(validate_tag_shape("étoile", TagSource::FrontMatter).is_empty());
        assert!(validate_tag_shape("日本", TagSource::FrontMatter).is_empty());
        assert!(validate_tag_shape("emoji-🚀", TagSource::FrontMatter).is_empty());
    }

    #[test]
    fn rejects_punctuation_only_tags_as_numeric_only() {
        assert!(validate_tag_shape("--", TagSource::FrontMatter)
            .contains(&TagIssueCode::NumericOnlyTag));
        assert!(validate_tag_shape("///", TagSource::FrontMatter)
            .contains(&TagIssueCode::NumericOnlyTag));
        assert!(validate_tag_shape("-_/_", TagSource::FrontMatter)
            .contains(&TagIssueCode::NumericOnlyTag));
    }
}
