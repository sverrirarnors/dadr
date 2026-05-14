use std::fs;

use dadr::{error::DiagnosticCode, parse_record_str, RecordStatus};

#[test]
fn parses_valid_candidate_fixture() {
    let content = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/valid-candidate/01938b8e4d70-drop-missing-treatment.md"
    ))
    .unwrap();
    let parsed = parse_record_str(&content);

    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let record = parsed.record.unwrap();
    assert_eq!(record.front_matter.status, RecordStatus::Candidate);
    assert_eq!(
        record.front_matter.id.to_string(),
        "01938b8e-4d70-7abc-8def-0123456789ab"
    );
    assert!(record.body.raw_markdown.contains("## Context"));
}

#[test]
fn parses_utf8_bom_before_frontmatter() {
    let content = "\u{feff}---\n\
        title: BOM candidate\n\
        status: candidate\n\
        id: 01938b8e-4d70-7abc-8def-0123456789ab\n\
        ---\n\n\
        ## Context\n\n\
        ## Decision\n\n\
        ## Alternatives\n\n\
        ## Consequences\n";
    let parsed = parse_record_str(content);

    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    assert_eq!(parsed.record.unwrap().front_matter.title, "BOM candidate");
}

#[test]
fn malformed_frontmatter_fixture_reports_error() {
    let content = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/malformed-frontmatter/missing-closing-fence.md"
    ))
    .unwrap();
    let parsed = parse_record_str(&content);

    assert!(parsed.record.is_none());
    assert!(parsed
        .diagnostics
        .iter()
        .any(|diag| diag.code == DiagnosticCode::MalformedFrontMatter));
}

#[test]
fn manual_missing_id_fixture_stays_repairable() {
    let content = fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/manual-missing-id/manual-candidate.md"
    ))
    .unwrap();
    let parsed = parse_record_str(&content);

    assert!(parsed.record.is_none());
    assert!(parsed
        .diagnostics
        .iter()
        .any(|diag| diag.code == DiagnosticCode::MissingRequiredField
            && diag.field.as_deref() == Some("id")));
}
