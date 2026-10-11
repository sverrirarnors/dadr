use std::fmt;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::{
    error::{IssueCounts, Severity},
    index::ProjectIndex,
    markdown::scan::{byte_in_spans, inline_code_spans, is_fenced_code_fence},
    model::RecordStatus,
    records::{FindResolution, ScannedRecord},
    text::write_serde_label,
    RecordId, RecordRef,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkCheckReport {
    pub project_dir: Utf8PathBuf,
    pub ok: bool,
    pub counts: IssueCounts,
    pub issues: Vec<LinkIssue>,
}

impl LinkCheckReport {
    pub fn has_errors(&self) -> bool {
        self.counts.has_errors()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkIssueCode {
    BrokenWikilink,
    AmbiguousWikilink,
    RejectedRecordWikilink,
    BrokenMarkdownLink,
}

impl fmt::Display for LinkIssueCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_serde_label(self, formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkIssue {
    pub severity: Severity,
    pub code: LinkIssueCode,
    pub message: String,
    pub path: Utf8PathBuf,
    pub line: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_id: Option<RecordId>,
    pub target: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_paths: Vec<Utf8PathBuf>,
}

impl LinkIssue {
    fn error(
        code: LinkIssueCode,
        path: Utf8PathBuf,
        line: usize,
        target: String,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Error, code, path, line, target, message)
    }

    fn warning(
        code: LinkIssueCode,
        path: Utf8PathBuf,
        line: usize,
        target: String,
        message: impl Into<String>,
    ) -> Self {
        Self::new(Severity::Warning, code, path, line, target, message)
    }

    fn new(
        severity: Severity,
        code: LinkIssueCode,
        path: Utf8PathBuf,
        line: usize,
        target: String,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            path,
            line,
            record_id: None,
            target,
            related_paths: Vec::new(),
        }
    }

    fn with_optional_record_id(mut self, record_id: Option<RecordId>) -> Self {
        self.record_id = record_id;
        self
    }

    fn with_related_paths(mut self, paths: Vec<Utf8PathBuf>) -> Self {
        self.related_paths = paths;
        self
    }
}

pub fn check_project_links(index: &ProjectIndex) -> LinkCheckReport {
    let project = index.project();
    let scanned = index.scanned_records();
    let mut issues = Vec::new();

    for scanned_record in scanned {
        let Some(record) = &scanned_record.record else {
            continue;
        };
        let mut in_fenced_code = false;
        for (line_index, line) in record.body.raw_markdown.lines().enumerate() {
            let line_number = line_index + 1;
            if is_fenced_code_fence(line) {
                in_fenced_code = !in_fenced_code;
                continue;
            }
            if in_fenced_code {
                continue;
            }
            add_wikilink_issues(index, scanned_record, line, line_number, &mut issues);
            add_markdown_link_issues(scanned_record, line, line_number, &mut issues);
        }
    }

    sort_issues(&mut issues);
    let counts = IssueCounts::from_severities(issues.iter().map(|issue| issue.severity));
    let ok = counts.errors == 0;
    LinkCheckReport {
        project_dir: project.project_dir.clone(),
        ok,
        counts,
        issues,
    }
}

fn add_wikilink_issues(
    index: &ProjectIndex,
    scanned: &ScannedRecord,
    line: &str,
    line_number: usize,
    issues: &mut Vec<LinkIssue>,
) {
    for wikilink in wikilinks_in_line(line) {
        // Only short-id and full-id targets are record links; leave other
        // wikilinks alone.
        let Some(target) = RecordRef::parse(&wikilink.target) else {
            continue;
        };
        let query = wikilink.target.trim();
        let result = index.find_record_ref(&target);
        match result.resolution {
            FindResolution::None => {
                issues.push(
                    LinkIssue::error(
                        LinkIssueCode::BrokenWikilink,
                        scanned.path.clone(),
                        line_number,
                        query.to_owned(),
                        format!("wikilink target `{query}` does not resolve to a DADR record"),
                    )
                    .with_optional_record_id(source_record_id(scanned)),
                );
            }
            FindResolution::Many => {
                let paths = result
                    .matches
                    .into_iter()
                    .map(|matched| matched.record.path)
                    .collect();
                issues.push(
                    LinkIssue::error(
                        LinkIssueCode::AmbiguousWikilink,
                        scanned.path.clone(),
                        line_number,
                        query.to_owned(),
                        format!("wikilink target `{query}` resolves to multiple DADR records"),
                    )
                    .with_optional_record_id(source_record_id(scanned))
                    .with_related_paths(paths),
                );
            }
            FindResolution::One => {
                let found = result
                    .matches
                    .into_iter()
                    .next()
                    .expect("one resolution contains one match");
                // The archive also holds superseded decisions, which are fine
                // to link, so the check reads the status, not the folder.
                if found.record.status == RecordStatus::Rejected {
                    issues.push(
                        LinkIssue::warning(
                            LinkIssueCode::RejectedRecordWikilink,
                            scanned.path.clone(),
                            line_number,
                            query.to_owned(),
                            format!(
                                "wikilink target `{query}` resolves to rejected record `{}` rather than an accepted decision",
                                found.record.title
                            ),
                        )
                        .with_optional_record_id(source_record_id(scanned))
                        .with_related_paths(vec![found.record.path]),
                    );
                }
            }
        }
    }
}

fn add_markdown_link_issues(
    scanned: &ScannedRecord,
    line: &str,
    line_number: usize,
    issues: &mut Vec<LinkIssue>,
) {
    for markdown_link in markdown_links_in_line(line) {
        let Some(local_target) = local_markdown_target(&markdown_link.target) else {
            continue;
        };
        let resolved = resolve_markdown_target(&scanned.path, &local_target);
        if !resolved.exists() {
            issues.push(
                LinkIssue::error(
                    LinkIssueCode::BrokenMarkdownLink,
                    scanned.path.clone(),
                    line_number,
                    markdown_link.target,
                    format!("Markdown link target `{local_target}` does not exist"),
                )
                .with_optional_record_id(source_record_id(scanned)),
            );
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wikilink {
    pub raw: String,
    pub target: String,
    pub display_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikilinkSpan {
    pub start: usize,
    pub end: usize,
    pub link: Wikilink,
}

pub fn wikilinks_in_line(line: &str) -> Vec<Wikilink> {
    wikilink_spans_in_line(line)
        .into_iter()
        .map(|span| span.link)
        .collect()
}

pub fn wikilink_spans_in_line(line: &str) -> Vec<WikilinkSpan> {
    let mut links = Vec::new();
    let mut offset = 0;
    while let Some(start) = line[offset..].find("[[") {
        let content_start = offset + start + 2;
        let Some(end) = line[content_start..].find("]]") else {
            break;
        };
        let content_end = content_start + end;
        let raw = line[offset + start..content_end + 2].to_owned();
        let content = &line[content_start..content_end];
        let (target, display_text) = content
            .split_once('|')
            .map_or((content, None), |(target, display)| {
                (target, Some(display.trim().to_owned()))
            });
        links.push(WikilinkSpan {
            start: offset + start,
            end: content_end + 2,
            link: Wikilink {
                raw,
                target: target.trim().to_owned(),
                display_text,
            },
        });
        offset = content_end + 2;
    }
    links
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MarkdownLink {
    target: String,
}

fn markdown_links_in_line(line: &str) -> Vec<MarkdownLink> {
    let mut links = Vec::new();
    let mut offset = 0;
    let code_spans = inline_code_spans(line);
    while let Some(open_bracket) = line[offset..].find('[') {
        let label_start = offset + open_bracket;
        if byte_in_spans(label_start, &code_spans) {
            offset = label_start + 1;
            continue;
        }
        if is_escaped(line, label_start) {
            offset = label_start + 1;
            continue;
        }
        if label_start > 0 && line.as_bytes()[label_start - 1] == b'!' {
            offset = label_start + 1;
            continue;
        }
        if line[label_start..].starts_with("[[") {
            offset = label_start + 2;
            continue;
        }

        let Some(close_bracket_rel) = line[label_start + 1..].find(']') else {
            break;
        };
        let close_bracket = label_start + 1 + close_bracket_rel;
        if !line[close_bracket + 1..].starts_with('(') {
            offset = close_bracket + 1;
            continue;
        }

        let target_start = close_bracket + 2;
        let Some(target_end_rel) = line[target_start..].find(')') else {
            break;
        };
        let target_end = target_start + target_end_rel;
        links.push(MarkdownLink {
            target: line[target_start..target_end].trim().to_owned(),
        });
        offset = target_end + 1;
    }
    links
}

fn is_escaped(line: &str, index: usize) -> bool {
    let mut slash_count = 0;
    for byte in line[..index].bytes().rev() {
        if byte != b'\\' {
            break;
        }
        slash_count += 1;
    }
    slash_count % 2 == 1
}

fn local_markdown_target(raw_target: &str) -> Option<String> {
    let raw_target = raw_target.trim();
    if raw_target.is_empty() {
        return None;
    }

    let without_title = if let Some(stripped) = raw_target.strip_prefix('<') {
        let end = stripped.find('>')?;
        stripped[..end].trim()
    } else {
        raw_target.split_whitespace().next()?
    };
    let without_fragment = without_title
        .split_once('#')
        .map_or(without_title, |(target, _fragment)| target);
    let without_query = without_fragment
        .split_once('?')
        .map_or(without_fragment, |(target, _query)| target);
    let target = without_query.trim();

    if target.is_empty()
        || target.starts_with('#')
        || target.contains("://")
        || target.starts_with("mailto:")
        || target.starts_with("tel:")
        || target.starts_with("data:")
        || target.starts_with("urn:")
    {
        return None;
    }

    Some(target.to_owned())
}

fn resolve_markdown_target(source_path: &camino::Utf8Path, target: &str) -> Utf8PathBuf {
    let target_path = Utf8PathBuf::from(target);
    if target_path.is_absolute() {
        target_path
    } else if let Some(parent) = source_path.parent() {
        parent.join(target)
    } else {
        target_path
    }
}

fn source_record_id(scanned: &ScannedRecord) -> Option<RecordId> {
    scanned.record.as_ref().map(|record| record.front_matter.id)
}

fn sort_issues(issues: &mut [LinkIssue]) {
    issues.sort_by(|left, right| {
        left.severity
            .cmp(&right.severity)
            .then_with(|| left.code.cmp(&right.code))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.target.cmp(&right.target))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wikilinks_with_display_text() {
        let links = wikilinks_in_line("See [[01938b8e4d70]] and [[01938b8f9d71|new one]].");
        assert_eq!(
            links,
            vec![
                Wikilink {
                    raw: "[[01938b8e4d70]]".to_owned(),
                    target: "01938b8e4d70".to_owned(),
                    display_text: None,
                },
                Wikilink {
                    raw: "[[01938b8f9d71|new one]]".to_owned(),
                    target: "01938b8f9d71".to_owned(),
                    display_text: Some("new one".to_owned()),
                },
            ]
        );
    }

    #[test]
    fn parses_inline_markdown_links_without_wikilinks_or_images() {
        let links = markdown_links_in_line(
            "See [data](../data/input.csv), ![plot](plot.png), \\[literal](skip.md), and [[01938b8e4d70]].",
        );
        assert_eq!(
            links,
            vec![MarkdownLink {
                target: "../data/input.csv".to_owned(),
            }]
        );
    }
}
