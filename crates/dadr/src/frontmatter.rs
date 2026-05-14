use crate::error::{Diagnostic, DiagnosticCode};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrontMatterSplit {
    pub raw_yaml: Option<String>,
    pub body: String,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn split_front_matter(input: &str) -> FrontMatterSplit {
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let Some(first_line) = input.split_inclusive('\n').next() else {
        return FrontMatterSplit {
            raw_yaml: None,
            body: String::new(),
            diagnostics: vec![Diagnostic::error(
                DiagnosticCode::MissingFrontMatter,
                "record is empty and has no YAML front matter",
            )],
        };
    };

    if trim_line_ending(first_line) != "---" {
        return FrontMatterSplit {
            raw_yaml: None,
            body: input.to_owned(),
            diagnostics: vec![Diagnostic::error(
                DiagnosticCode::MissingFrontMatter,
                "record must start with a YAML front-matter fence",
            )],
        };
    }

    let mut yaml = String::new();
    let mut consumed = first_line.len();

    for line in input[consumed..].split_inclusive('\n') {
        consumed += line.len();
        if is_closing_fence(line) {
            return FrontMatterSplit {
                raw_yaml: Some(yaml),
                body: input[consumed..].to_owned(),
                diagnostics: Vec::new(),
            };
        }
        yaml.push_str(line);
    }

    FrontMatterSplit {
        raw_yaml: Some(yaml.clone()),
        body: yaml,
        diagnostics: vec![Diagnostic::error(
            DiagnosticCode::MalformedFrontMatter,
            "YAML front matter is missing a closing fence",
        )],
    }
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

fn is_closing_fence(line: &str) -> bool {
    matches!(trim_line_ending(line), "---" | "...")
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
    }

    #[test]
    fn accepts_yaml_document_end_as_closing_fence() {
        let parsed = split_front_matter("---\ntitle: Test\n...\n# Body\n");
        assert!(parsed.diagnostics.is_empty());
        assert_eq!(parsed.raw_yaml.as_deref(), Some("title: Test\n"));
        assert_eq!(parsed.body, "# Body\n");
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
