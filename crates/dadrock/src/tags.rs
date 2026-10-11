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
    model::Record,
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
    /// A character other than a letter, number, emoji, `_`, `-`, or `/`
    /// (spec §7).
    InvalidTagCharacter,
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

        for occurrence in tag_occurrences(record) {
            add_observed_tag(
                &mut tags,
                &mut issues,
                scanned_record,
                &summary,
                &occurrence,
            );
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

/// The tag problems in one record, front matter and inline, as
/// `dadrock validate tags` reports them. Acceptance refuses a candidate that
/// has any (spec §7).
pub fn record_tag_issues(scanned_record: &ScannedRecord) -> Vec<TagIssue> {
    let mut issues = Vec::new();
    add_tag_field_diagnostics(scanned_record, &mut issues);
    if let (Some(record), Some(summary)) = (&scanned_record.record, scanned_record.summary()) {
        for occurrence in tag_occurrences(record) {
            issues.extend(tag_shape_issues(scanned_record, &summary, &occurrence));
        }
    }
    issues
}

/// A tag as written in a record.
struct TagOccurrence {
    raw: String,
    source: TagSource,
    /// The file line of an inline tag.
    line: Option<usize>,
}

/// A record's front-matter tags, then its inline tags outside code.
fn tag_occurrences(record: &Record) -> Vec<TagOccurrence> {
    let mut occurrences: Vec<_> = record
        .front_matter
        .tags
        .iter()
        .map(|tag| TagOccurrence {
            raw: tag.as_str().to_owned(),
            source: TagSource::FrontMatter,
            line: None,
        })
        .collect();

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
            occurrences.push(TagOccurrence {
                raw: inline_tag,
                source: TagSource::Inline,
                line: Some(record.body.line_offset + line_index + 1),
            });
        }
    }
    occurrences
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

fn tag_shape_issues(
    scanned_record: &ScannedRecord,
    summary: &ScannedRecordSummary,
    occurrence: &TagOccurrence,
) -> Vec<TagIssue> {
    let TagOccurrence { raw, source, line } = occurrence;
    validate_tag_shape(raw, *source)
        .into_iter()
        .map(|code| {
            let mut issue = TagIssue::error(
                code,
                scanned_record.path.clone(),
                *source,
                issue_message(code, raw, *source),
            )
            .with_record_id(summary.id)
            .with_tag(raw.as_str());
            if let Some(line) = line {
                issue = issue.with_line(*line);
            }
            issue
        })
        .collect()
}

fn add_observed_tag(
    tags: &mut BTreeMap<String, TagAccumulator>,
    issues: &mut Vec<TagIssue>,
    scanned_record: &ScannedRecord,
    summary: &ScannedRecordSummary,
    occurrence: &TagOccurrence,
) {
    let raw_tag = occurrence.raw.as_str();
    let normalized_value = normalized_tag_value(raw_tag);
    let shape_issues = tag_shape_issues(scanned_record, summary, occurrence);
    let only_repairable_yaml_hash = shape_issues
        .iter()
        .all(|issue| issue.code == TagIssueCode::LeadingHashInYamlTag);
    let skip = !shape_issues.is_empty() && !only_repairable_yaml_hash;
    issues.extend(shape_issues);
    if skip {
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
    if !invalid_tag_characters(&normalized).is_empty() {
        issues.push(TagIssueCode::InvalidTagCharacter);
    }
    issues
}

/// The characters of a tag that a tag cannot contain, each once, in order.
/// Whitespace is reported separately.
fn invalid_tag_characters(tag: &str) -> Vec<char> {
    let mut invalid = Vec::new();
    for ch in tag.chars() {
        if !ch.is_whitespace() && !is_tag_char(ch) && !invalid.contains(&ch) {
            invalid.push(ch);
        }
    }
    invalid
}

/// Whether a tag can contain `ch`. Spec §7: a tag consists of Unicode
/// letters, numbers, emoji, `_`, `-`, and `/`.
///
/// In ASCII that is exact. Outside ASCII, the standard library has no table
/// of Unicode general categories, so, like Obsidian, whose tag syntax DADR
/// uses, this rules out whitespace, control characters, and the punctuation
/// blocks, and accepts the rest: letters with their combining marks, numbers,
/// and emoji with their joiners, variation selectors, and modifiers.
pub(crate) fn is_tag_char(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/');
    }
    if ch.is_alphanumeric() {
        return true;
    }
    !(ch.is_whitespace() || ch.is_control() || is_punctuation_or_format(ch))
}

/// Non-ASCII punctuation, symbols next to it, and invisible format
/// characters, which a tag cannot contain. Letters and numbers in these
/// blocks never get here, and the emoji among them are allowed.
fn is_punctuation_or_format(ch: char) -> bool {
    match ch {
        // Emoji: ©, ®, ‼, ⁉, 〰, 〽; and the zero-width joiner and non-joiner
        // that join emoji sequences and some scripts.
        '\u{a9}' | '\u{ae}' | '\u{203c}' | '\u{2049}' | '\u{3030}' | '\u{303d}' => false,
        '\u{200c}' | '\u{200d}' => false,
        // Latin-1 punctuation and symbols, ×, ÷.
        '\u{a0}'..='\u{bf}' | '\u{d7}' | '\u{f7}' => true,
        // General Punctuation (dashes, quotes, ellipsis, invisible
        // operators, bidirectional controls) and Supplemental Punctuation.
        '\u{2000}'..='\u{206f}' | '\u{2e00}'..='\u{2e7f}' => true,
        // CJK Symbols and Punctuation.
        '\u{3000}'..='\u{303f}' => true,
        // Vertical forms, CJK compatibility forms, small form variants.
        '\u{fe10}'..='\u{fe1f}' | '\u{fe30}'..='\u{fe6f}' => true,
        // Fullwidth and halfwidth punctuation.
        '\u{ff01}'..='\u{ff0f}'
        | '\u{ff1a}'..='\u{ff20}'
        | '\u{ff3b}'..='\u{ff40}'
        | '\u{ff5b}'..='\u{ff65}' => true,
        // Byte-order mark and other format characters.
        '\u{61c}' | '\u{180e}' | '\u{feff}' | '\u{fff9}'..='\u{fffb}' => true,
        _ => false,
    }
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
        TagIssueCode::InvalidTagCharacter => {
            let characters: Vec<_> = invalid_tag_characters(&normalized_tag_value(tag))
                .into_iter()
                .map(describe_char)
                .collect();
            format!(
                "tag `{tag}` contains {}; a tag consists of letters, numbers, emoji, `_`, `-`, and `/`",
                characters.join(", ")
            )
        }
    }
}

/// A character for a message: in backticks when it is visible, otherwise by
/// its code point.
fn describe_char(ch: char) -> String {
    let invisible = ch.is_control()
        || matches!(
            ch,
            '\u{a0}'
                | '\u{ad}'
                | '\u{61c}'
                | '\u{180e}'
                | '\u{2000}'..='\u{200f}'
                | '\u{2028}'..='\u{202f}'
                | '\u{205f}'..='\u{206f}'
                | '\u{3000}'
                | '\u{feff}'
                | '\u{fff9}'..='\u{fffb}'
        );
    if invisible {
        format!("U+{:04X}", u32::from(ch))
    } else {
        format!("`{ch}`")
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

/// An inline tag runs to the first character a tag cannot contain, as in
/// Obsidian, so `#kind/sample.` and `#kind/sample*` carry `kind/sample`.
fn is_inline_tag_terminator(ch: char) -> bool {
    !is_tag_char(ch)
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
