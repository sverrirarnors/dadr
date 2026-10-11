//! What a rewrite may change in a record (specification §6.3).

use crate::{
    error::DadrError,
    model::{Record, RecordStatus},
    records::check_required_sections,
};

/// Refuse a rewrite of an accepted or rejected record that breaks the rules
/// that hold once a record is decided: its `id`, `status`, and `successors`
/// stay as they are (successors are added only by superseding the record), and
/// an accepted record keeps each required section exactly once and with text.
/// The title, body, `tags`, and keys outside DADR may change: corrections stay
/// in the record, and Git shows them. Candidates may change freely.
pub fn check_record_rewrite(current: &Record, proposed: &Record) -> Result<(), DadrError> {
    let status = current.front_matter.status;
    if status == RecordStatus::Candidate {
        return Ok(());
    }
    let (now, then) = (&proposed.front_matter, &current.front_matter);
    let state = if then.is_superseded() {
        "superseded"
    } else {
        status.as_str()
    };
    let advice = if status == RecordStatus::Rejected {
        "To revisit it, draft a new candidate"
    } else {
        "To change the decision, draft a successor that supersedes it"
    };

    let changed: Vec<&str> = [
        ("`id`", now.id != then.id),
        ("`status`", now.status != then.status),
        ("`successors`", now.successors != then.successors),
    ]
    .into_iter()
    .filter_map(|(field, changed)| changed.then_some(field))
    .collect();
    if !changed.is_empty() {
        let successors = if now.successors != then.successors {
            " Successors are added only by superseding a record."
        } else {
            ""
        };
        return Err(DadrError::InvalidLifecycleTransition(format!(
            "`{}` is {state}, so its {} cannot change.{successors} {advice}; corrections to the title, body, and tags are fine.",
            then.title,
            join_fields(&changed)
        )));
    }

    if status == RecordStatus::Accepted {
        let sections = check_required_sections(&proposed.body.raw_markdown);
        if !sections.is_complete() {
            return Err(DadrError::InvalidLifecycleTransition(format!(
                "`{}` is {state}, so it keeps each standard section exactly once and with text: {}. {advice}.",
                then.title,
                sections.problems().join("; ")
            )));
        }
    }
    Ok(())
}

fn join_fields(fields: &[&str]) -> String {
    match fields {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}
