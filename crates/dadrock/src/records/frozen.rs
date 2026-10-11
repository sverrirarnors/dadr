//! Accepted and rejected records are frozen (specification §6.3).

use crate::{
    error::DadrError,
    model::{Record, RecordStatus},
};

/// Refuse a rewrite of an accepted or rejected record that changes its `id`,
/// `status`, `title`, body, or `successors`. Successors are added only by
/// superseding the record, never by rewriting it. `tags` and keys outside DADR
/// may change. Bodies are compared with normalized line endings. Candidates
/// may change freely.
pub fn check_frozen_rewrite(current: &Record, proposed: &Record) -> Result<(), DadrError> {
    let status = current.front_matter.status;
    if status == RecordStatus::Candidate {
        return Ok(());
    }
    let (now, then) = (&proposed.front_matter, &current.front_matter);
    let changed: Vec<&str> = [
        ("`id`", now.id != then.id),
        ("`status`", now.status != then.status),
        ("title", now.title != then.title),
        (
            "body",
            normalize_line_endings(&proposed.body.raw_markdown)
                != normalize_line_endings(&current.body.raw_markdown),
        ),
        ("`successors`", now.successors != then.successors),
    ]
    .into_iter()
    .filter_map(|(field, changed)| changed.then_some(field))
    .collect();
    if changed.is_empty() {
        return Ok(());
    }

    let advice = if status == RecordStatus::Accepted {
        "To change the decision, draft a successor that supersedes it"
    } else {
        "To revisit it, draft a new candidate"
    };
    Err(DadrError::InvalidLifecycleTransition(format!(
        "`{}` is {status}, so its {} cannot change. {advice}; tags and keys outside DADR can still change.",
        then.title,
        join_fields(&changed)
    )))
}

fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

fn join_fields(fields: &[&str]) -> String {
    match fields {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
