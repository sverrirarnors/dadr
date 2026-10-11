use crate::error::{Diagnostic, DiagnosticCode};

const BYTE_ORDER_MARK: char = '\u{feff}';

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatterSplit {
    pub raw_yaml: Option<String>,
    pub body: String,
    /// How many lines of the input precede the body: the front matter and
    /// its fences. Line `n` of the body (1-based) is line
    /// `body_line_offset + n` of the input.
    pub body_line_offset: usize,
    pub diagnostics: Vec<Diagnostic>,
}

/// Split a Markdown document with YAML front matter, such as a task document
/// or a template, into its YAML and its body.
///
/// This is the lenient split for documents that are not DADR records: a
/// leading byte-order mark is skipped, and a YAML document-end line (`...`)
/// closes the front matter like `---`. Records use
/// [`split_record_front_matter`], which reports both.
pub fn split_front_matter(input: &str) -> FrontMatterSplit {
    split(input).0
}

/// Split a DADR record into its YAML front matter and its body.
///
/// A record is UTF-8 without a byte-order mark (spec §2) and its front matter
/// is delimited by `---` lines (spec §3). A byte-order mark or a `...` closing
/// line is reported as an error rather than repaired; the split still
/// proceeds, so the rest of the record can be checked.
pub fn split_record_front_matter(input: &str) -> FrontMatterSplit {
    let (mut split, closing) = split(input);
    if input.starts_with(BYTE_ORDER_MARK) {
        split.diagnostics.insert(
            0,
            Diagnostic::error(
                DiagnosticCode::ByteOrderMark,
                "record starts with a UTF-8 byte-order mark; save it as UTF-8 without a BOM",
            ),
        );
    }
    if closing == Some(ClosingFence::DocumentEnd) {
        split.diagnostics.push(Diagnostic::error(
            DiagnosticCode::MalformedFrontMatter,
            "front matter must close with a `---` line, not `...`",
        ));
    }
    split
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClosingFence {
    /// `---`
    Dashes,
    /// `...`, the YAML document-end marker.
    DocumentEnd,
}

fn split(input: &str) -> (FrontMatterSplit, Option<ClosingFence>) {
    let input = input.strip_prefix(BYTE_ORDER_MARK).unwrap_or(input);
    let Some(first_line) = input.split_inclusive('\n').next() else {
        return (
            FrontMatterSplit {
                raw_yaml: None,
                body: String::new(),
                body_line_offset: 0,
                diagnostics: vec![Diagnostic::error(
                    DiagnosticCode::MissingFrontMatter,
                    "record is empty and has no YAML front matter",
                )],
            },
            None,
        );
    };

    if trim_line_ending(first_line) != "---" {
        return (
            FrontMatterSplit {
                raw_yaml: None,
                body: input.to_owned(),
                body_line_offset: 0,
                diagnostics: vec![Diagnostic::error(
                    DiagnosticCode::MissingFrontMatter,
                    "record must start with a YAML front-matter fence",
                )],
            },
            None,
        );
    }

    let mut yaml = String::new();
    let mut consumed = first_line.len();

    for (index, line) in input[consumed..].split_inclusive('\n').enumerate() {
        consumed += line.len();
        if let Some(fence) = closing_fence(line) {
            return (
                FrontMatterSplit {
                    raw_yaml: Some(yaml),
                    body: input[consumed..].to_owned(),
                    // The opening fence, the YAML lines, and this fence.
                    body_line_offset: index + 2,
                    diagnostics: Vec::new(),
                },
                Some(fence),
            );
        }
        yaml.push_str(line);
    }

    (
        FrontMatterSplit {
            raw_yaml: Some(yaml.clone()),
            body: yaml,
            body_line_offset: 1,
            diagnostics: vec![Diagnostic::error(
                DiagnosticCode::MalformedFrontMatter,
                "YAML front matter is missing a closing fence",
            )],
        },
        None,
    )
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn closing_fence(line: &str) -> Option<ClosingFence> {
    match trim_line_ending(line) {
        "---" => Some(ClosingFence::Dashes),
        "..." => Some(ClosingFence::DocumentEnd),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_front_matter_and_preserves_body() {
        let parsed = split_front_matter("---\ntitle: Test\n---\n# Body\n\nText\n");
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.raw_yaml.as_deref(), Some("title: Test\n"));
        assert_eq!(parsed.body, "# Body\n\nText\n");
        assert_eq!(parsed.body_line_offset, 3);
    }

    #[test]
    fn body_line_offset_maps_body_lines_to_file_lines() {
        let content = "\u{feff}---\r\ntitle: Test\r\nid: x\r\n---\r\n\r\nSee [[x]].\r\n";
        let parsed = split_record_front_matter(content);
        assert_eq!(parsed.body_line_offset, 4);
        let body_line = parsed
            .body
            .lines()
            .position(|line| line.contains("[[x]]"))
            .unwrap()
            + 1;
        let file_line = content
            .lines()
            .position(|line| line.contains("[[x]]"))
            .unwrap()
            + 1;
        assert_eq!(parsed.body_line_offset + body_line, file_line);
    }

    // Spec §3: front matter is delimited by `---` lines. A record closed by
    // the YAML document-end marker is reported, not silently accepted, so
    // rewriting it never changes the fence behind the author's back.
    #[test]
    fn reports_yaml_document_end_as_record_closing_fence() {
        let parsed = split_record_front_matter("---\ntitle: Test\n...\n# Body\n");
        assert_eq!(parsed.raw_yaml.as_deref(), Some("title: Test\n"));
        assert_eq!(parsed.body, "# Body\n");
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(
            parsed.diagnostics[0].code,
            DiagnosticCode::MalformedFrontMatter
        );
        assert!(parsed.diagnostics[0].is_error());

        // Documents that are not records, such as task documents, keep the
        // lenient split.
        assert!(split_front_matter("---\ntitle: Test\n...\n# Body\n")
            .diagnostics
            .is_empty());
    }

    // Spec §2: a record is UTF-8 without a byte-order mark.
    #[test]
    fn reports_byte_order_mark_in_records_only() {
        let content = "\u{feff}---\ntitle: Test\n---\n# Body\n";
        let parsed = split_record_front_matter(content);
        assert_eq!(parsed.diagnostics.len(), 1);
        assert_eq!(parsed.diagnostics[0].code, DiagnosticCode::ByteOrderMark);
        assert!(parsed.diagnostics[0].is_error());
        assert_eq!(parsed.raw_yaml.as_deref(), Some("title: Test\n"));
        assert_eq!(parsed.body, "# Body\n");

        assert!(split_front_matter(content).diagnostics.is_empty());
    }

    #[test]
    fn reports_missing_front_matter() {
        let parsed = split_front_matter("# Body\n");
        assert!(parsed.raw_yaml.is_none());
        assert_eq!(parsed.body, "# Body\n");
        assert_eq!(
            parsed.diagnostics[0].code,
            DiagnosticCode::MissingFrontMatter
        );
    }

    #[test]
    fn reports_missing_closing_fence() {
        let parsed = split_front_matter("---\ntitle: Test\n");
        assert_eq!(
            parsed.diagnostics[0].code,
            DiagnosticCode::MalformedFrontMatter
        );
        assert_eq!(parsed.body, "title: Test\n");
    }
}
